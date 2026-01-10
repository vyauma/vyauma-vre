//! Bytecode Loader
//!
//! Loads and validates Vyauma bytecode.
//! This layer performs structural validation only.

use crate::error::{VreError, VreResult};
use crate::vm::value::Value;
use std::collections::HashSet;

/// Bytecode magic: "VYMA"
const BYTECODE_MAGIC: u32 = 0x5659_4D41;

/// Supported bytecode version
const VERSION_MAJOR: u8 = 1;
/// Minimum bytecode header size
const MIN_FILE_SIZE: usize = 16;

/// Loaded bytecode bundle
#[derive(Debug)]
pub struct LoadedBytecode {
    pub constants: Vec<Value>,
    pub instructions: Vec<u8>,
    pub entry_point: usize,
    /// External capability ids referenced by the instructions (if determinable)
    pub caps: Vec<u8>,
}

/// Bytecode loader
pub struct BytecodeLoader;

impl BytecodeLoader {
    /// Load bytecode from raw bytes
    pub fn load(bytes: &[u8]) -> VreResult<LoadedBytecode> {
        if bytes.len() < MIN_FILE_SIZE {
            return Err(VreError::BytecodeTooShort);
        }
        
        let mut cursor = 0;

        // Magic
        let magic = Self::read_u32(bytes, &mut cursor)?;
        if magic != BYTECODE_MAGIC {
            return Err(VreError::InvalidMagicNumber);
        }

        // Version
        let major = Self::read_u8(bytes, &mut cursor)?;
        let _minor = Self::read_u8(bytes, &mut cursor)?;
        let _patch = Self::read_u8(bytes, &mut cursor)?;

        if major != VERSION_MAJOR {
            return Err(VreError::InvalidBytecodeVersion);
        }

        // Reserved
        Self::read_u8(bytes, &mut cursor)?;

        // Entry point
        let entry_point = Self::read_u32(bytes, &mut cursor)? as usize;

        // Constants
        let constant_count = Self::read_u32(bytes, &mut cursor)? as usize;
        let mut constants = Vec::with_capacity(constant_count);

        for _ in 0..constant_count {
            constants.push(Self::read_constant(bytes, &mut cursor)?);
        }

        // Instructions
        let instruction_len = Self::read_u32(bytes, &mut cursor)? as usize;
        if cursor + instruction_len > bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }

        let instructions = bytes[cursor..cursor + instruction_len].to_vec();

        // Validate instruction stream using a conservative CFG-based stack-height
        // analysis and collect referenced external capability ids.
        let caps_set = Self::analyze_instructions_cfg(&instructions)?;
        let caps = caps_set.into_iter().collect();

        Ok(LoadedBytecode {
            constants,
            instructions,
            entry_point,
            caps,
        })
    }

    /// Validate instructions and collect ExternalCall capability ids. This performs
    /// a conservative, linear simulation of stack depth and will return
    /// `VreError::MalformedBytecode` for mismatches or truncation.
    fn analyze_instructions_cfg(instructions: &[u8]) -> VreResult<HashSet<u8>> {
        use crate::bytecode::opcode::OpCode;

        // First pass: parse instructions and collect metadata (offsets, opcode,
        // immediate lengths, jump targets when applicable).
        #[derive(Debug)]
        struct InstrMeta {
            offset: usize,
            opcode: OpCode,
            _imm_len: usize,
            // For jump-like instructions, absolute target offset when known.
            target: Option<usize>,
        }

        let mut metas: Vec<InstrMeta> = Vec::new();
        let mut idx = 0usize;
        while idx < instructions.len() {
            let offset = idx;
            let byte = instructions[idx];
            idx += 1;
            let op = OpCode::from_u8(byte).ok_or(VreError::MalformedBytecode)?;

            match op {
                OpCode::Push => {
                    if idx < instructions.len() { idx += 1; metas.push(InstrMeta{offset, opcode:op, _imm_len:1, target:None}); }
                    else { return Err(VreError::MalformedBytecode); }
                }
                OpCode::LoadLocal | OpCode::StoreLocal => {
                    if idx < instructions.len() { idx += 1; metas.push(InstrMeta{offset, opcode:op, _imm_len:1, target:None}); }
                    else { return Err(VreError::MalformedBytecode); }
                }
                OpCode::ExternalCall => {
                    if idx + 1 < instructions.len() {
                        // cap_id (u8) and argc (u8)
                        let _cap_id = instructions[idx];
                        let _argc = instructions[idx + 1];
                        idx += 2;
                        metas.push(InstrMeta{offset, opcode:op, _imm_len:2, target:None});
                    } else { return Err(VreError::MalformedBytecode); }
                }
                OpCode::Jump | OpCode::JumpIf | OpCode::Call => {
                    // Assume 4-byte u32 BE absolute instruction target
                    if idx + 4 <= instructions.len() {
                        let t = u32::from_be_bytes([
                            instructions[idx], instructions[idx+1], instructions[idx+2], instructions[idx+3]
                        ]) as usize;
                        idx += 4;
                        metas.push(InstrMeta{offset, opcode:op, _imm_len:4, target:Some(t)});
                    } else { return Err(VreError::MalformedBytecode); }
                }
                // For Call/Return we stop conservative analysis; still record them
                OpCode::Return => { metas.push(InstrMeta{offset, opcode:op, _imm_len:0, target:None}); }
                // All other opcodes have no immediates
                _ => { metas.push(InstrMeta{offset, opcode:op, _imm_len:0, target:None}); }
            }
        }

        // Build a mapping from offset to index in metas for quick lookup
        use std::collections::{HashMap, VecDeque};
        let mut offset_to_idx: HashMap<usize, usize> = HashMap::new();
        for (i, m) in metas.iter().enumerate() { offset_to_idx.insert(m.offset, i); }

        // Build successor map
        let mut succs: Vec<Vec<usize>> = vec![Vec::new(); metas.len()];
        for i in 0..metas.len() {
            let m = &metas[i];
            // fall-through successor: the next instruction offset, if any
            let fallthrough = metas.get(i+1).map(|n| n.offset);

            match m.opcode {
                OpCode::Halt | OpCode::Nop => { /* no successors */ }
                OpCode::Jump => {
                    if let Some(t) = m.target {
                        if !offset_to_idx.contains_key(&t) { return Err(VreError::InvalidJumpTarget(t)); }
                        succs[i].push(*offset_to_idx.get(&t).unwrap());
                    }
                }
                OpCode::Call => {
                    if let Some(t) = m.target {
                        if !offset_to_idx.contains_key(&t) { return Err(VreError::InvalidJumpTarget(t)); }
                        // Call transfers control to callee entry; also allow fallthrough
                        // to the next instruction (return site) so that intra-callee
                        // control-flow can reach Return nodes during analysis.
                        succs[i].push(*offset_to_idx.get(&t).unwrap());
                        if let Some(ft) = fallthrough { succs[i].push(*offset_to_idx.get(&ft).unwrap()); }
                    }
                }
                OpCode::JumpIf => {
                    if let Some(t) = m.target {
                        if !offset_to_idx.contains_key(&t) { return Err(VreError::InvalidJumpTarget(t)); }
                        succs[i].push(*offset_to_idx.get(&t).unwrap());
                    }
                    if let Some(ft) = fallthrough { succs[i].push(*offset_to_idx.get(&ft).unwrap()); }
                }
                OpCode::Return => {
                    // Conservative: stop analysis at return boundaries by not adding succs
                }
                _ => {
                    if let Some(ft) = fallthrough { succs[i].push(*offset_to_idx.get(&ft).unwrap()); }
                }
            }
        }

        // Collect call targets and caller return-sites
        let mut call_targets: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for i in 0..metas.len() {
            if metas[i].opcode == OpCode::Call {
                if let Some(t) = metas[i].target {
                    if let Some(next) = metas.get(i+1) {
                        call_targets.entry(t).or_default().push(next.offset);
                    }
                }
            }
        }

        // Build composable callee summaries for targets that can be analyzed.
        // We iteratively compute summaries so that callees that call other
        // callees may be summarized once their callees' summaries are known.
        let mut summaries: std::collections::HashMap<usize, isize> = std::collections::HashMap::new();
        let mut pending: std::collections::HashSet<usize> = call_targets.keys().cloned().collect();

        // Iterate until no new summaries can be discovered (fixed-point)
        let mut progress = true;
        while progress {
            progress = false;

            // collect to avoid borrow issues while mutating summaries/pending
            let current_pending: Vec<usize> = pending.iter().cloned().collect();

                // Fallback: attempt to summarize remaining pending targets by allowing
                // calls between pending targets (treat intra-pending calls as no-op).
                if !pending.is_empty() {
                    let mut progress2 = true;
                    while progress2 {
                        progress2 = false;
                        let current_pending: Vec<usize> = pending.iter().cloned().collect();
                        for &target_offset in &current_pending {
                            if summaries.contains_key(&target_offset) { pending.remove(&target_offset); continue; }
                            let &t_idx = match offset_to_idx.get(&target_offset) { Some(i) => i, None => { pending.remove(&target_offset); continue; } };

                            let mut local_heights: Vec<Option<isize>> = vec![None; metas.len()];
                            let mut q: VecDeque<usize> = VecDeque::new();
                            local_heights[t_idx] = Some(0);
                            q.push_back(t_idx);
                            let mut complex = false;
                            let mut return_heights: Vec<isize> = Vec::new();

                            while let Some(j) = q.pop_front() {
                                let h = local_heights[j].unwrap();
                                let mm = &metas[j];
                                let mut nh = h;
                                match mm.opcode {
                                    OpCode::Push => nh += 1,
                                    OpCode::Pop => { if nh <= 0 { complex = true; break; } nh -= 1; }
                                    OpCode::Dup => { if nh <= 0 { complex = true; break; } nh += 1; }
                                    OpCode::LoadLocal => nh += 1,
                                    OpCode::StoreLocal => { if nh <= 0 { complex = true; break; } nh -= 1; }
                                    OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod
                                    | OpCode::Equal | OpCode::NotEqual | OpCode::Less | OpCode::LessEqual
                                    | OpCode::Greater | OpCode::GreaterEqual => { if nh < 2 { complex = true; break; } nh -= 1; }
                                    OpCode::ExternalCall => { complex = true; break; }
                                    OpCode::Call => {
                                        if let Some(nested_t) = mm.target {
                                            if summaries.contains_key(&nested_t) {
                                                nh += *summaries.get(&nested_t).unwrap();
                                            } else if current_pending.contains(&nested_t) {
                                                // intra-pending call: treat as no net change for now
                                            } else {
                                                complex = true; break;
                                            }
                                        } else { complex = true; break; }
                                    }
                                    OpCode::Jump | OpCode::JumpIf => { /* no change */ }
                                    OpCode::Return => { return_heights.push(nh); }
                                    _ => {}
                                }

                                for &s in &succs[j] {
                                    if local_heights[s].is_none() {
                                        local_heights[s] = Some(nh);
                                        q.push_back(s);
                                    } else {
                                        let existing = local_heights[s].unwrap();
                                        let merged = std::cmp::min(existing, nh);
                                        if merged != existing {
                                            local_heights[s] = Some(merged);
                                            q.push_back(s);
                                        }
                                    }
                                }
                            }

                            if !complex && !return_heights.is_empty() {
                                let first = return_heights[0];
                                if return_heights.iter().all(|&r| r == first) {
                                    summaries.insert(target_offset, first);
                                    pending.remove(&target_offset);
                                    progress2 = true;
                                }
                            }
                        }
                    }
                }

            for target_offset in current_pending {
                // skip if already summarized
                if summaries.contains_key(&target_offset) { pending.remove(&target_offset); continue; }

                // find meta index for target
                let &t_idx = match offset_to_idx.get(&target_offset) { Some(i) => i, None => { pending.remove(&target_offset); continue; } };

                // Local dataflow within callee region starting at t_idx, entry height=0
                let mut local_heights: Vec<Option<isize>> = vec![None; metas.len()];
                let mut q: VecDeque<usize> = VecDeque::new();
                local_heights[t_idx] = Some(0);
                q.push_back(t_idx);
                let mut complex = false;
                let mut return_heights: Vec<isize> = Vec::new();

                while let Some(j) = q.pop_front() {
                    let h = local_heights[j].unwrap();
                    let mm = &metas[j];

                    let mut nh = h;
                    match mm.opcode {
                        OpCode::Push => nh += 1,
                        OpCode::Pop => { if nh <= 0 { complex = true; break; } nh -= 1; }
                        OpCode::Dup => { if nh <= 0 { complex = true; break; } nh += 1; }
                        OpCode::LoadLocal => nh += 1,
                        OpCode::StoreLocal => { if nh <= 0 { complex = true; break; } nh -= 1; }
                        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod
                        | OpCode::Equal | OpCode::NotEqual | OpCode::Less | OpCode::LessEqual
                        | OpCode::Greater | OpCode::GreaterEqual => { if nh < 2 { complex = true; break; } nh -= 1; }
                        OpCode::ExternalCall => {
                            // conservatively abort if external call present in callee
                            complex = true;
                            break;
                        }
                        OpCode::Call => {
                            // If nested call has a known summary, apply it; otherwise defer
                            if let Some(nested_t) = mm.target {
                                if let Some(nested_delta) = summaries.get(&nested_t) {
                                    nh += *nested_delta;
                                } else {
                                    complex = true;
                                    break;
                                }
                            } else {
                                complex = true;
                                break;
                            }
                        }
                        OpCode::Jump | OpCode::JumpIf => { /* no change */ }
                        OpCode::Return => {
                            return_heights.push(nh);
                        }
                        _ => {}
                    }

                    // propagate to successors within the callee
                    for &s in &succs[j] {
                        if local_heights[s].is_none() {
                            local_heights[s] = Some(nh);
                            q.push_back(s);
                        } else {
                            let existing = local_heights[s].unwrap();
                            let merged = std::cmp::min(existing, nh);
                            if merged != existing {
                                local_heights[s] = Some(merged);
                                q.push_back(s);
                            }
                        }
                    }
                }

                if !complex && !return_heights.is_empty() {
                    // require all return heights to be equal for a simple summary
                    let first = return_heights[0];
                    if return_heights.iter().all(|&r| r == first) {
                        summaries.insert(target_offset, first);
                        pending.remove(&target_offset);
                        progress = true;
                    }
                }
            }
        }

        // Dataflow: simulate stack height (isize) at each instruction index.
        let mut heights: Vec<Option<isize>> = vec![None; metas.len()];
        let mut work: VecDeque<usize> = VecDeque::new();

        // entry is first instruction
        if metas.is_empty() { return Ok(HashSet::new()); }
        heights[0] = Some(0);
        work.push_back(0);

        let mut caps = HashSet::new();

        while let Some(i) = work.pop_front() {
            let current_height = heights[i].unwrap();
            let m = &metas[i];

            // simulate effect
            let mut new_height = current_height;
            match m.opcode {
                OpCode::Push => { new_height += 1; }
                OpCode::Pop => { if new_height <= 0 { return Err(VreError::MalformedBytecode); } new_height -= 1; }
                OpCode::Dup => { if new_height <= 0 { return Err(VreError::MalformedBytecode); } new_height += 1; }
                OpCode::LoadLocal => { new_height += 1; }
                OpCode::StoreLocal => { if new_height <= 0 { return Err(VreError::MalformedBytecode); } new_height -= 1; }
                OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod
                | OpCode::Equal | OpCode::NotEqual | OpCode::Less | OpCode::LessEqual
                | OpCode::Greater | OpCode::GreaterEqual => { if new_height < 2 { return Err(VreError::MalformedBytecode); } new_height -= 1; }
                OpCode::ExternalCall => {
                    // immediates stored in instruction stream; find them
                    let imm_off = m.offset + 1; // opcode byte consumed
                    if imm_off + 1 >= instructions.len() { return Err(VreError::MalformedBytecode); }
                    let cap_id = instructions[imm_off];
                    let argc = instructions[imm_off + 1] as isize;
                    if new_height < argc { return Err(VreError::MalformedBytecode); }
                    caps.insert(cap_id);
                    new_height -= argc;
                }
                OpCode::Jump | OpCode::JumpIf => { /* no stack change */ }
                OpCode::Call => {
                    // If we have a summary for this call target, apply its net stack delta.
                    if let Some(t) = m.target {
                        if let Some(delta) = summaries.get(&t) {
                            new_height += *delta;
                        } else {
                            // Conservative: if no summary available, reject (avoid propagation)
                            return Err(VreError::MalformedBytecode);
                        }
                    } else {
                        return Err(VreError::MalformedBytecode);
                    }
                }
                OpCode::Return => { /* stop; handled via CFG */ }
                _ => { /* no-op */ }
            }

            // propagate to successors
            for &s in &succs[i] {
                match heights[s] {
                    Some(h) => {
                        if h != new_height {
                            // If heights differ, merge by taking the min (conservative)
                            let merged = std::cmp::min(h, new_height);
                            if merged != h {
                                heights[s] = Some(merged);
                                work.push_back(s);
                            }
                        }
                    }
                    None => {
                        heights[s] = Some(new_height);
                        work.push_back(s);
                    }
                }
            }
        }

        Ok(caps)
    }

    /// Public convenience: parse the provided raw bytecode and return the list
    /// of ExternalCall capability ids referenced by the instruction stream.
    /// This will perform the same structural validation as `load`.
    pub fn collect_caps(bytes: &[u8]) -> VreResult<Vec<u8>> {
        let loaded = Self::load(bytes)?;
        Ok(loaded.caps)
    }

    /// Load with optional lenient fallback. If `allow_opt_in` is true and the
    /// strict loader validation fails, a best-effort lenient parse is attempted
    /// which validates instruction encoding but skips CFG/stack validation.
    /// Returns a tuple `(LoadedBytecode, used_lenient_fallback)` where the
    /// boolean indicates whether the lenient path was used.
    pub fn load_with_opt_in(bytes: &[u8], allow_opt_in: bool) -> VreResult<(LoadedBytecode, bool)> {
        match Self::load(bytes) {
            Ok(lb) => Ok((lb, false)),
            Err(_) if allow_opt_in => {
                // Perform lenient parse: header, constants, and weak instruction
                // scan that only checks immediate lengths and collects cap ids.
                let mut cursor = 0;

                // Magic and header
                let magic = Self::read_u32(bytes, &mut cursor)?;
                if magic != BYTECODE_MAGIC { return Err(crate::error::VreError::InvalidMagicNumber); }
                let major = Self::read_u8(bytes, &mut cursor)?;
                let _minor = Self::read_u8(bytes, &mut cursor)?;
                let _patch = Self::read_u8(bytes, &mut cursor)?;
                if major != VERSION_MAJOR { return Err(crate::error::VreError::InvalidBytecodeVersion); }
                Self::read_u8(bytes, &mut cursor)?; // reserved

                let entry_point = Self::read_u32(bytes, &mut cursor)? as usize;

                // constants
                let constant_count = Self::read_u32(bytes, &mut cursor)? as usize;
                let mut constants = Vec::with_capacity(constant_count);
                for _ in 0..constant_count { constants.push(Self::read_constant(bytes, &mut cursor)?); }

                // instructions
                let instruction_len = Self::read_u32(bytes, &mut cursor)? as usize;
                if cursor + instruction_len > bytes.len() { return Err(crate::error::VreError::BytecodeTooShort); }
                let instructions = bytes[cursor..cursor + instruction_len].to_vec();

                // weak scan for caps
                let caps_set = Self::weak_scan_for_caps(&instructions)?;
                let caps = caps_set.into_iter().collect();

                Ok((LoadedBytecode { constants, instructions, entry_point, caps }, true))
            }
            Err(e) => Err(e),
        }
    }

    /// Weak instruction scanner: checks immediate lengths, opcode encoding,
    /// and collects `ExternalCall` capability ids, but does not perform CFG or
    /// stack-height validation. Useful as a lenient fallback when strict
    /// validation is rejected but host opts in.
    fn weak_scan_for_caps(instructions: &[u8]) -> VreResult<HashSet<u8>> {
        use crate::bytecode::opcode::OpCode;
        let mut caps = HashSet::new();
        let mut idx = 0usize;
        while idx < instructions.len() {
            let b = instructions[idx];
            idx += 1;
            let op = OpCode::from_u8(b).ok_or(crate::error::VreError::MalformedBytecode)?;
            match op {
                OpCode::Push => { if idx < instructions.len() { idx += 1 } else { return Err(crate::error::VreError::MalformedBytecode); } }
                OpCode::LoadLocal | OpCode::StoreLocal => { if idx < instructions.len() { idx += 1 } else { return Err(crate::error::VreError::MalformedBytecode); } }
                OpCode::ExternalCall => {
                    if idx + 1 < instructions.len() {
                        let cap_id = instructions[idx];
                        // let argc = instructions[idx+1];
                        caps.insert(cap_id);
                        idx += 2;
                    } else { return Err(crate::error::VreError::MalformedBytecode); }
                }
                OpCode::Jump | OpCode::JumpIf | OpCode::Call => {
                    if idx + 4 <= instructions.len() { idx += 4 } else { return Err(crate::error::VreError::MalformedBytecode); }
                }
                _ => { /* no immediates */ }
            }
        }
        Ok(caps)
    }

    /// Read a constant value (minimal runtime types only)
    fn read_constant(bytes: &[u8], cursor: &mut usize) -> VreResult<Value> {
        let tag = Self::read_u8(bytes, cursor)?;

        match tag {
            0x00 => Ok(Value::Null),
            0x01 => {
                let b = Self::read_u8(bytes, cursor)?;
                Ok(Value::Bool(b != 0))
            }
            0x02 => {
                let n = Self::read_f64(bytes, cursor)?;
                Ok(Value::Number(n))
            }
            0xFF => {
                let id = Self::read_u32(bytes, cursor)?;
                Ok(Value::Ref(id))
            }
            _ => Err(VreError::MalformedBytecode),
        }
    }

    fn read_u8(bytes: &[u8], cursor: &mut usize) -> VreResult<u8> {
        if *cursor >= bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = bytes[*cursor];
        *cursor += 1;
        Ok(v)
    }

    fn read_u32(bytes: &[u8], cursor: &mut usize) -> VreResult<u32> {
        if *cursor + 4 > bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = u32::from_be_bytes([
            bytes[*cursor],
            bytes[*cursor + 1],
            bytes[*cursor + 2],
            bytes[*cursor + 3],
        ]);
        *cursor += 4;
        Ok(v)
    }

    fn read_f64(bytes: &[u8], cursor: &mut usize) -> VreResult<f64> {
        if *cursor + 8 > bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = f64::from_be_bytes([
            bytes[*cursor],
            bytes[*cursor + 1],
            bytes[*cursor + 2],
            bytes[*cursor + 3],
            bytes[*cursor + 4],
            bytes[*cursor + 5],
            bytes[*cursor + 6],
            bytes[*cursor + 7],
        ]);
        *cursor += 8;
        Ok(v)
    }
}
