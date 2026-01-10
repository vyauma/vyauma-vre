use vre_core::BytecodeLoader;

// Construct bytecode with two mutually recursive callees that each push one
// value and call the other, then return. The SCC-based grouped analysis in
// the loader should accept this pattern and produce summaries for both
// callees, allowing the load to succeed.
#[test]
fn scc_summary_accepts_mutual_recursion() {
    use vre_core::bytecode::OpCode;

    // Header: magic, version, reserved, entry_point=0
    let mut buf = Vec::new();
    buf.extend(&0x5659_4D41u32.to_be_bytes());
    buf.push(1u8); buf.push(0u8); buf.push(0u8); buf.push(0u8);
    buf.extend(&(0u32.to_be_bytes()));

    // zero constants
    buf.extend(&(0u32.to_be_bytes()));

    // Build instructions: funcA at offset 0, funcB at offset 8
    // funcA: Push const0; Pop; Call -> offset 8; Return
    // funcB: Push const0; Call -> offset 0; Return
    let mut instr: Vec<u8> = Vec::new();
    // funcA
    instr.push(OpCode::Push as u8); instr.push(0u8);
    instr.push(OpCode::Pop as u8);
    instr.push(OpCode::Call as u8);
    instr.extend(&(8u32.to_be_bytes()));
    instr.push(OpCode::Return as u8);
    // funcB
    instr.push(OpCode::Push as u8); instr.push(0u8);
    instr.push(OpCode::Pop as u8);
    instr.push(OpCode::Call as u8);
    instr.extend(&(0u32.to_be_bytes()));
    instr.push(OpCode::Return as u8);
    // final Halt
    instr.push(OpCode::Halt as u8);

    buf.extend(&((instr.len() as u32).to_be_bytes()));
    buf.extend(&instr);

    // Quick pre-scan to help debugging if loader rejects
    let instr_len = u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as usize;
    let instr_bytes = &buf[20..20+instr_len];
    // OpCode already imported above
    let mut i = 0usize;
    while i < instr_bytes.len() {
        let b = instr_bytes[i];
        if OpCode::from_u8(b).is_none() {
            panic!("invalid opcode byte 0x{:02X} at instr offset {}", b, i);
        }
        match OpCode::from_u8(b).unwrap() {
            OpCode::Push | OpCode::LoadLocal | OpCode::StoreLocal => { i += 2; }
            OpCode::ExternalCall => { i += 3; }
            OpCode::Jump | OpCode::JumpIf | OpCode::Call => { i += 5; }
            _ => { i += 1; }
        }
    }

    // Build metas, offset_to_idx and succs similar to loader to inspect
    let instr_len = u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as usize;
    let instr_bytes = &buf[20..20+instr_len];
    #[derive(Debug, Clone, Copy)] struct M { off: usize, op: u8, imm_len: usize, target: Option<usize> }
    let mut metas2: Vec<M> = Vec::new();
    let mut idx2 = 0usize;
    while idx2 < instr_bytes.len() {
        let offset = idx2;
        let byte = instr_bytes[idx2]; idx2 += 1;
        let op = OpCode::from_u8(byte).unwrap();
        match op {
            OpCode::Push => { idx2 += 1; metas2.push(M{off: offset, op: byte, imm_len:1, target: None}); }
            OpCode::LoadLocal | OpCode::StoreLocal => { idx2 += 1; metas2.push(M{off: offset, op: byte, imm_len:1, target: None}); }
            OpCode::ExternalCall => { let _cap = instr_bytes[idx2]; let _argc = instr_bytes[idx2+1]; idx2 += 2; metas2.push(M{off:offset, op:byte, imm_len:2, target: None}); }
            OpCode::Jump | OpCode::JumpIf | OpCode::Call => { let t = u32::from_be_bytes([instr_bytes[idx2], instr_bytes[idx2+1], instr_bytes[idx2+2], instr_bytes[idx2+3]]) as usize; idx2 += 4; metas2.push(M{off:offset, op:byte, imm_len:4, target: Some(t)}); }
            _ => { metas2.push(M{off:offset, op:byte, imm_len:0, target: None}); }
        }
    }

    use std::collections::HashMap;
    use std::collections::VecDeque;
    let mut offset_to_idx: HashMap<usize, usize> = HashMap::new();
    for (i, m) in metas2.iter().enumerate() { offset_to_idx.insert(m.off, i); }
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); metas2.len()];
    for i in 0..metas2.len() {
        let m = metas2[i];
        let fallthrough = metas2.get(i+1).map(|n| n.off);
        match OpCode::from_u8(m.op).unwrap() {
            OpCode::Halt | OpCode::Nop => {}
            OpCode::Jump => {
                if let Some(t) = m.target { if !offset_to_idx.contains_key(&t) { println!("invalid target {}", t); } else { succs[i].push(*offset_to_idx.get(&t).unwrap()); } }
            }
            OpCode::Call => {
                if let Some(t) = m.target { if !offset_to_idx.contains_key(&t) { println!("invalid target {}", t); } else { succs[i].push(*offset_to_idx.get(&t).unwrap()); } }
                if let Some(ft) = fallthrough { succs[i].push(*offset_to_idx.get(&ft).unwrap()); }
            }
            OpCode::JumpIf => { if let Some(t) = m.target { if !offset_to_idx.contains_key(&t) { println!("invalid target {}", t);} else { succs[i].push(*offset_to_idx.get(&t).unwrap()); } } if let Some(ft) = fallthrough { succs[i].push(*offset_to_idx.get(&ft).unwrap()); } }
            OpCode::Return => {}
            _ => { if let Some(ft) = fallthrough { succs[i].push(*offset_to_idx.get(&ft).unwrap()); } }
        }
    }

    // collect call_targets
    let mut call_targets: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for i in 0..metas2.len() { if OpCode::from_u8(metas2[i].op) == Some(OpCode::Call) { if let Some(t) = metas2[i].target { if let Some(next) = metas2.get(i+1) { call_targets.entry(t).or_default().push(next.off); } } } }

    println!("metas2: {:?}", metas2);
    println!("offset_to_idx keys: {:?}", offset_to_idx.keys().cloned().collect::<Vec<_>>());
    println!("call_targets: {:?}", call_targets);
    // print succs mapping (index -> offsets)
    for i in 0..succs.len() {
        let targets: Vec<usize> = succs[i].iter().map(|&si| metas2[si].off).collect();
        println!("succ[{}] (off {}) -> {:?}", i, metas2[i].off, targets);
    }
    // simulate fallback attempt locally for each pending target
    let mut summaries_sim: std::collections::HashMap<usize, isize> = std::collections::HashMap::new();
    let mut pending_sim: std::collections::HashSet<usize> = call_targets.keys().cloned().collect();
    let mut progress_sim = true;
    while progress_sim {
        progress_sim = false;
        let current_pending: Vec<usize> = pending_sim.iter().cloned().collect();
        for &target_offset in &current_pending {
            if summaries_sim.contains_key(&target_offset) { pending_sim.remove(&target_offset); continue; }
            let &t_idx = match offset_to_idx.get(&target_offset) { Some(i) => i, None => { pending_sim.remove(&target_offset); continue; } };
            let mut local_heights: Vec<Option<isize>> = vec![None; metas2.len()];
            let mut q: VecDeque<usize> = VecDeque::new();
            local_heights[t_idx] = Some(0);
            q.push_back(t_idx);
            let mut complex = false;
            let mut return_heights: Vec<isize> = Vec::new();
            while let Some(j) = q.pop_front() {
                let h = local_heights[j].unwrap();
                let mm = metas2[j];
                let mut nh = h;
                match OpCode::from_u8(mm.op).unwrap() {
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
                            if summaries_sim.contains_key(&nested_t) {
                                nh += *summaries_sim.get(&nested_t).unwrap();
                            } else if current_pending.contains(&nested_t) {
                                // intra-pending allowed
                            } else { complex = true; break; }
                        } else { complex = true; break; }
                    }
                    OpCode::Jump | OpCode::JumpIf => {}
                    OpCode::Return => { return_heights.push(nh); }
                    _ => {}
                }
                for &s in &succs[j] {
                    if local_heights[s].is_none() { local_heights[s] = Some(nh); q.push_back(s); }
                    else { let existing = local_heights[s].unwrap(); let merged = std::cmp::min(existing, nh); if merged != existing { local_heights[s] = Some(merged); q.push_back(s); } }
                }
            }
            println!("target {} -> complex={} returns={:?}", target_offset, complex, return_heights);
            if !complex && !return_heights.is_empty() { let first = return_heights[0]; if return_heights.iter().all(|&r| r == first) { summaries_sim.insert(target_offset, first); pending_sim.remove(&target_offset); progress_sim = true; } }
        }
    }
    println!("sim summaries: {:?}", summaries_sim);

    // Now run loader and assert
    match BytecodeLoader::load(&buf) {
        Ok(_) => {}
        Err(e) => panic!("loader rejected SCC bytecode: {}", e),
    }
}
