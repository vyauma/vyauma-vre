//! Vyauma Bytecode Assembler Implementation
//!
//! Parses .vasm assembly text and serializes it to Vyauma v0.1 bytecode.

use std::collections::HashMap;
use vre_core::bytecode::opcode::OpCode;

/// Types of constants supported by Vyauma
#[derive(Debug, Clone)]
enum AsmConstant {
    Null,
    Bool(bool),
    Number(f64),
    Ref(u32),
}

#[derive(Debug, Clone)]
struct AsmInstruction {
    opcode: OpCode,
    operands: Vec<String>,
}

pub struct Assembler {
    constants: Vec<AsmConstant>,
    labels: HashMap<String, usize>, // Label name -> byte offset
    entry_label: Option<String>,
    instructions_to_encode: Vec<(AsmInstruction, usize)>, // Instruction and its byte offset
}

impl Assembler {
    pub fn new() -> Self {
        Assembler {
            constants: Vec::new(),
            labels: HashMap::new(),
            entry_label: None,
            instructions_to_encode: Vec::new(),
        }
    }

    /// Assemble source text into a binary bytecode buffer
    pub fn assemble(&mut self, source: &str) -> Result<Vec<u8>, String> {
        self.parse(source)?;
        self.codegen()
    }

    /// Pass 1: Parse structure, collect constants, determine label targets
    fn parse(&mut self, source: &str) -> Result<(), String> {
        let mut byte_offset = 0;

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;
            let cleaned = match line.split(';').next() {
                Some(s) => s.trim(),
                None => continue,
            };

            if cleaned.is_empty() {
                continue;
            }

            // Check if label
            if cleaned.ends_with(':') {
                let label_name = cleaned[..cleaned.len() - 1].trim().to_string();
                if label_name.is_empty() {
                    return Err(format!("Line {}: empty label name", line_num));
                }
                if self.labels.contains_key(&label_name) {
                    return Err(format!("Line {}: duplicate label '{}'", line_num, label_name));
                }
                self.labels.insert(label_name, byte_offset);
                continue;
            }

            // Check if directive
            if cleaned.starts_with('.') {
                let parts: Vec<&str> = cleaned.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                match parts[0] {
                    ".entry" => {
                        if parts.len() != 2 {
                            return Err(format!("Line {}: .entry requires exactly one label", line_num));
                        }
                        self.entry_label = Some(parts[1].to_string());
                    }
                    ".const" => {
                        let constant = parse_const_directive(&parts[1..])
                            .map_err(|e| format!("Line {}: invalid constant: {}", line_num, e))?;
                        self.constants.push(constant);
                    }
                    _ => return Err(format!("Line {}: unknown directive '{}'", line_num, parts[0])),
                }
                continue;
            }

            // Parse instruction
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let op_str = parts[0];
            let opcode = parse_opcode(op_str)
                .ok_or_else(|| format!("Line {}: unknown opcode '{}'", line_num, op_str))?;

            let operands: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            let instr = AsmInstruction { opcode, operands };

            // Compute size of instruction
            let size = instruction_size(&instr)
                .map_err(|e| format!("Line {}: {}", line_num, e))?;

            self.instructions_to_encode.push((instr, byte_offset));
            byte_offset += size;
        }

        Ok(())
    }

    /// Pass 2: Resolve labels/operands and emit binary bytes
    fn codegen(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();

        // 1. Magic: "VYMA"
        out.extend_from_slice(&0x5659_4D41u32.to_be_bytes());

        // 2. Version: 1.0.1 (Major 1, Minor 0, Patch 1, Reserved 0)
        out.push(1);
        out.push(0);
        out.push(1);
        out.push(0);

        // 3. Entry point offset
        let entry_point = match &self.entry_label {
            Some(lbl) => {
                *self.labels.get(lbl)
                    .ok_or_else(|| format!("Entry label '{}' is not defined", lbl))?
            }
            None => 0, // default to start of instructions
        };
        out.extend_from_slice(&(entry_point as u32).to_be_bytes());

        // 4. Constant Pool
        out.extend_from_slice(&(self.constants.len() as u32).to_be_bytes());
        for constant in &self.constants {
            match constant {
                AsmConstant::Null => {
                    out.push(0x00);
                }
                AsmConstant::Bool(b) => {
                    out.push(0x01);
                    out.push(if *b { 1 } else { 0 });
                }
                AsmConstant::Number(n) => {
                    out.push(0x02);
                    out.extend_from_slice(&n.to_be_bytes());
                }
                AsmConstant::Ref(r) => {
                    out.push(0xFF);
                    out.extend_from_slice(&r.to_be_bytes());
                }
            }
        }

        // 5. Instruction Bytes Code Gen
        let mut instr_bytes = Vec::new();
        for (instr, offset) in &self.instructions_to_encode {
            instr_bytes.push(instr.opcode as u8);

            match instr.opcode {
                // Stack ops
                OpCode::Push => {
                    if instr.operands.len() != 1 {
                        return Err(format!("push requires exactly 1 operand at offset {}", offset));
                    }
                    let index = parse_u16_operand(&instr.operands[0])?;
                    instr_bytes.extend_from_slice(&index.to_be_bytes());
                }
                OpCode::Pop | OpCode::Dup => {}

                // Locals and Properties
                OpCode::LoadLocal | OpCode::LoadLocalI32 | OpCode::LoadLocalI64 | OpCode::LoadLocalF32 | OpCode::LoadLocalF64 | OpCode::LoadLocalStr | OpCode::StoreLocal | OpCode::LoadProperty | OpCode::StoreProperty => {
                    if instr.operands.len() != 1 {
                        return Err(format!("{:?} requires exactly 1 operand at offset {}", instr.opcode, offset));
                    }
                    let index = parse_u16_operand(&instr.operands[0])?;
                    instr_bytes.extend_from_slice(&index.to_be_bytes());
                }

                // Arithmetic & Comparison
                OpCode::AddI32 | OpCode::SubI32 | OpCode::MulI32 | OpCode::DivI32 | OpCode::ModI32 | OpCode::NegI32 |
                OpCode::AddI64 | OpCode::SubI64 | OpCode::MulI64 | OpCode::DivI64 | OpCode::ModI64 | OpCode::NegI64 |
                OpCode::AddF32 | OpCode::SubF32 | OpCode::MulF32 | OpCode::DivF32 | OpCode::ModF32 | OpCode::NegF32 |
                OpCode::AddF64 | OpCode::SubF64 | OpCode::MulF64 | OpCode::DivF64 | OpCode::ModF64 | OpCode::NegF64 |
                OpCode::EqualI32 | OpCode::NotEqualI32 | OpCode::LessI32 | OpCode::LessEqualI32 | OpCode::GreaterI32 | OpCode::GreaterEqualI32 |
                OpCode::EqualI64 | OpCode::NotEqualI64 | OpCode::LessI64 | OpCode::LessEqualI64 | OpCode::GreaterI64 | OpCode::GreaterEqualI64 |
                OpCode::EqualF32 | OpCode::NotEqualF32 | OpCode::LessF32 | OpCode::LessEqualF32 | OpCode::GreaterF32 | OpCode::GreaterEqualF32 |
                OpCode::EqualF64 | OpCode::NotEqualF64 | OpCode::LessF64 | OpCode::LessEqualF64 | OpCode::GreaterF64 | OpCode::GreaterEqualF64 |
                OpCode::EqualStr | OpCode::NotEqualStr | OpCode::AddStr |
                OpCode::NewArray | OpCode::LoadElement | OpCode::StoreElement | OpCode::NewStruct => {}

                // Jumps
                OpCode::Jump | OpCode::JumpIf => {
                    if instr.operands.len() != 1 {
                        return Err(format!("{:?} requires exactly 1 target at offset {}", instr.opcode, offset));
                    }
                    let target_offset = self.resolve_label_or_u32(&instr.operands[0])?;
                    instr_bytes.extend_from_slice(&(target_offset as u32).to_be_bytes());
                }

                // Call
                OpCode::Call => {
                    if instr.operands.len() != 2 {
                        return Err(format!("call requires exactly 2 operands (target, locals) at offset {}", offset));
                    }
                    let target_offset = self.resolve_label_or_u32(&instr.operands[0])?;
                    let locals = parse_u16_operand(&instr.operands[1])?;
                    instr_bytes.extend_from_slice(&(target_offset as u32).to_be_bytes());
                    instr_bytes.extend_from_slice(&locals.to_be_bytes());
                }

                OpCode::Return | OpCode::Nop | OpCode::Halt | OpCode::TryEnd | OpCode::Throw => {}

                OpCode::TryStart => {
                    if instr.operands.len() != 1 {
                        return Err(format!("trystart requires exactly 1 target at offset {}", offset));
                    }
                    let target_offset = self.resolve_label_or_u32(&instr.operands[0])?;
                    instr_bytes.extend_from_slice(&(target_offset as u32).to_be_bytes());
                }

                OpCode::CallNative => {
                    if instr.operands.len() != 2 {
                        return Err(format!("callnative requires exactly 2 operands (native_idx, args_count) at offset {}", offset));
                    }
                    let native_idx = parse_u16_operand(&instr.operands[0])?;
                    let args_count = parse_u8_operand(&instr.operands[1])?;
                    instr_bytes.extend_from_slice(&native_idx.to_be_bytes());
                    instr_bytes.push(args_count);
                    // Add 3 bytes padding to match 6-byte operand space of standard Call
                    instr_bytes.push(0);
                    instr_bytes.push(0);
                    instr_bytes.push(0);
                }

                OpCode::Syscall => {
                    if instr.operands.len() != 1 {
                        return Err(format!("syscall requires exactly 1 operand at offset {}", offset));
                    }
                    let id = parse_u8_operand(&instr.operands[0])?;
                    instr_bytes.push(id);
                }

                OpCode::Spawn => {
                    if instr.operands.len() != 1 {
                        return Err(format!("spawn requires exactly 1 operand at offset {}", offset));
                    }
                    let target_offset = self.resolve_label_or_u32(&instr.operands[0])?;
                    instr_bytes.extend_from_slice(&(target_offset as u32).to_be_bytes());
                }

                OpCode::Yield | OpCode::Await | OpCode::SpawnDynamic => {}
                OpCode::AndBool | OpCode::OrBool => {}

                OpCode::NewClosure => {
                    // u32 func addr + u16 upvalue count
                    let target_offset = self.resolve_label_or_u32(&instr.operands[0])?;
                    let upcount = parse_u16_operand(&instr.operands[1])?;
                    instr_bytes.extend_from_slice(&(target_offset as u32).to_be_bytes());
                    instr_bytes.extend_from_slice(&upcount.to_be_bytes());
                }
                OpCode::CallDynamic => {
                    // u16 arg_count + u16 local_count
                    let ac = parse_u16_operand(&instr.operands[0])?;
                    let lc = parse_u16_operand(&instr.operands[1])?;
                    instr_bytes.extend_from_slice(&ac.to_be_bytes());
                    instr_bytes.extend_from_slice(&lc.to_be_bytes());
                }
                OpCode::LoadUpvalue | OpCode::StoreUpvalue | OpCode::BoxValue | OpCode::LoadBox | OpCode::StoreBox => {
                    // These take 0 operands in assembly (they pop/push from stack)
                }
                OpCode::ImportModule | OpCode::ExportValue => {
                    // Single u16 operand: constant-pool index of the path/name string
                    if !instr.operands.is_empty() {
                        let idx = parse_u16_operand(&instr.operands[0])?;
                        instr_bytes.extend_from_slice(&idx.to_be_bytes());
                    }
                }
            }
        }

        // 6. Write Instruction Length and Bytes
        out.extend_from_slice(&(instr_bytes.len() as u32).to_be_bytes());
        out.extend(instr_bytes);

        Ok(out)
    }

    fn resolve_label_or_u32(&self, operand: &str) -> Result<usize, String> {
        if let Some(&offset) = self.labels.get(operand) {
            Ok(offset)
        } else {
            operand.parse::<usize>()
                .map_err(|_| format!("unknown label or invalid numeric offset '{}'", operand))
        }
    }
}

fn parse_const_directive(args: &[&str]) -> Result<AsmConstant, String> {
    if args.is_empty() {
        return Err("missing constant type".to_string());
    }

    match args[0] {
        "null" => Ok(AsmConstant::Null),
        "bool" => {
            if args.len() < 2 {
                return Err("missing value for bool constant".to_string());
            }
            let b = args[1].parse::<bool>()
                .map_err(|e| format!("invalid bool value: {}", e))?;
            Ok(AsmConstant::Bool(b))
        }
        "number" => {
            if args.len() < 2 {
                return Err("missing value for number constant".to_string());
            }
            let n = args[1].parse::<f64>()
                .map_err(|e| format!("invalid number value: {}", e))?;
            Ok(AsmConstant::Number(n))
        }
        "ref" => {
            if args.len() < 2 {
                return Err("missing value for ref constant".to_string());
            }
            let r = args[1].parse::<u32>()
                .map_err(|e| format!("invalid ref value: {}", e))?;
            Ok(AsmConstant::Ref(r))
        }
        _ => Err(format!("unknown constant type '{}'", args[0])),
    }
}

fn parse_opcode(name: &str) -> Option<OpCode> {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "push" => Some(OpCode::Push),
        "pop" => Some(OpCode::Pop),
        "dup" => Some(OpCode::Dup),
        "loadlocal" | "load_local" => Some(OpCode::LoadLocal),
        "loadlocali32" | "load_locali32" => Some(OpCode::LoadLocalI32),
        "loadlocali64" | "load_locali64" => Some(OpCode::LoadLocalI64),
        "loadlocalf32" | "load_localf32" => Some(OpCode::LoadLocalF32),
        "loadlocalf64" | "load_localf64" => Some(OpCode::LoadLocalF64),
        "loadlocalstr" | "load_localstr" => Some(OpCode::LoadLocalStr),
        "storelocal" | "store_local" => Some(OpCode::StoreLocal),
        "addi32" => Some(OpCode::AddI32), "subi32" => Some(OpCode::SubI32), "muli32" => Some(OpCode::MulI32), "divi32" => Some(OpCode::DivI32), "modi32" => Some(OpCode::ModI32), "negi32" => Some(OpCode::NegI32),
        "addi64" => Some(OpCode::AddI64), "subi64" => Some(OpCode::SubI64), "muli64" => Some(OpCode::MulI64), "divi64" => Some(OpCode::DivI64), "modi64" => Some(OpCode::ModI64), "negi64" => Some(OpCode::NegI64),
        "addf32" => Some(OpCode::AddF32), "subf32" => Some(OpCode::SubF32), "mulf32" => Some(OpCode::MulF32), "divf32" => Some(OpCode::DivF32), "modf32" => Some(OpCode::ModF32), "negf32" => Some(OpCode::NegF32),
        "addf64" => Some(OpCode::AddF64), "subf64" => Some(OpCode::SubF64), "mulf64" => Some(OpCode::MulF64), "divf64" => Some(OpCode::DivF64), "modf64" => Some(OpCode::ModF64), "negf64" => Some(OpCode::NegF64),
        "equali32" => Some(OpCode::EqualI32), "notequali32" => Some(OpCode::NotEqualI32), "lessi32" => Some(OpCode::LessI32), "lessequali32" => Some(OpCode::LessEqualI32), "greateri32" => Some(OpCode::GreaterI32), "greaterequali32" => Some(OpCode::GreaterEqualI32),
        "equali64" => Some(OpCode::EqualI64), "notequali64" => Some(OpCode::NotEqualI64), "lessi64" => Some(OpCode::LessI64), "lessequali64" => Some(OpCode::LessEqualI64), "greateri64" => Some(OpCode::GreaterI64), "greaterequali64" => Some(OpCode::GreaterEqualI64),
        "equalf32" => Some(OpCode::EqualF32), "notequalf32" => Some(OpCode::NotEqualF32), "lessf32" => Some(OpCode::LessF32), "lessequalf32" => Some(OpCode::LessEqualF32), "greaterf32" => Some(OpCode::GreaterF32), "greaterequalf32" => Some(OpCode::GreaterEqualF32),
        "equalf64" => Some(OpCode::EqualF64), "notequalf64" => Some(OpCode::NotEqualF64), "lessf64" => Some(OpCode::LessF64), "lessequalf64" => Some(OpCode::LessEqualF64), "greaterf64" => Some(OpCode::GreaterF64), "greaterequalf64" => Some(OpCode::GreaterEqualF64),
        "equalstr" => Some(OpCode::EqualStr), "notequalstr" => Some(OpCode::NotEqualStr), "addstr" => Some(OpCode::AddStr),
        "jump" | "jmp" => Some(OpCode::Jump),
        "jumpif" | "jmpif" => Some(OpCode::JumpIf),
        "call" => Some(OpCode::Call),
        "return" | "ret" => Some(OpCode::Return),
        "nop" => Some(OpCode::Nop),
        "syscall" => Some(OpCode::Syscall),
        "halt" => Some(OpCode::Halt),
        "spawn" => Some(OpCode::Spawn),
        "spawndynamic" => Some(OpCode::SpawnDynamic),
        "yield" => Some(OpCode::Yield),
        "and_bool" => Some(OpCode::AndBool),
        "or_bool" => Some(OpCode::OrBool),
        "await" => Some(OpCode::Await),
        "newarray" => Some(OpCode::NewArray),
        "loadelement" => Some(OpCode::LoadElement),
        "storeelement" => Some(OpCode::StoreElement),
        "new_struct" => Some(OpCode::NewStruct),
        "load_property" => Some(OpCode::LoadProperty),
        "store_property" => Some(OpCode::StoreProperty),
        "callnative" => Some(OpCode::CallNative),
        "calldynamic" => Some(OpCode::CallDynamic),
        "newclosure" => Some(OpCode::NewClosure),
        "loadupvalue" => Some(OpCode::LoadUpvalue),
        "storeupvalue" => Some(OpCode::StoreUpvalue),
        "boxvalue" => Some(OpCode::BoxValue),
        "loadbox" => Some(OpCode::LoadBox),
        "storebox" => Some(OpCode::StoreBox),
        "trystart" => Some(OpCode::TryStart),
        "tryend" => Some(OpCode::TryEnd),
        "throw" => Some(OpCode::Throw),
        _ => None,
    }
}

fn instruction_size(instr: &AsmInstruction) -> Result<usize, String> {
    match instr.opcode {
        // Opcode only
        OpCode::AddI32 | OpCode::SubI32 | OpCode::MulI32 | OpCode::DivI32 | OpCode::ModI32 | OpCode::NegI32 |
                OpCode::AddI64 | OpCode::SubI64 | OpCode::MulI64 | OpCode::DivI64 | OpCode::ModI64 | OpCode::NegI64 |
                OpCode::AddF32 | OpCode::SubF32 | OpCode::MulF32 | OpCode::DivF32 | OpCode::ModF32 | OpCode::NegF32 |
                OpCode::AddF64 | OpCode::SubF64 | OpCode::MulF64 | OpCode::DivF64 | OpCode::ModF64 | OpCode::NegF64 |
                OpCode::EqualI32 | OpCode::NotEqualI32 | OpCode::LessI32 | OpCode::LessEqualI32 | OpCode::GreaterI32 | OpCode::GreaterEqualI32 |
                OpCode::EqualI64 | OpCode::NotEqualI64 | OpCode::LessI64 | OpCode::LessEqualI64 | OpCode::GreaterI64 | OpCode::GreaterEqualI64 |
                OpCode::EqualF32 | OpCode::NotEqualF32 | OpCode::LessF32 | OpCode::LessEqualF32 | OpCode::GreaterF32 | OpCode::GreaterEqualF32 |
                OpCode::EqualF64 | OpCode::NotEqualF64 | OpCode::LessF64 | OpCode::LessEqualF64 | OpCode::GreaterF64 | OpCode::GreaterEqualF64 |
                OpCode::EqualStr | OpCode::NotEqualStr | OpCode::AddStr |
        OpCode::Return | OpCode::Nop | OpCode::Halt | OpCode::Pop | OpCode::Dup |
        OpCode::NewArray | OpCode::LoadElement | OpCode::StoreElement |
        OpCode::NewStruct | OpCode::TryEnd | OpCode::Throw |
        OpCode::Yield | OpCode::Await |
        OpCode::LoadUpvalue | OpCode::StoreUpvalue |
        OpCode::BoxValue | OpCode::LoadBox | OpCode::StoreBox => Ok(1),
        OpCode::AndBool | OpCode::OrBool => Ok(1),

        // Opcode + u8
        OpCode::Syscall => Ok(2),

        // Opcode + u16
        OpCode::Push | OpCode::LoadLocal | OpCode::LoadLocalI32 | OpCode::LoadLocalI64 | OpCode::LoadLocalF32 | OpCode::LoadLocalF64 | OpCode::LoadLocalStr | OpCode::StoreLocal | OpCode::LoadProperty | OpCode::StoreProperty => Ok(3),

        // Opcode + u32
        OpCode::Jump | OpCode::JumpIf | OpCode::TryStart | OpCode::Spawn => Ok(5),

        // Opcode + u32 + u16
        OpCode::Call | OpCode::NewClosure => Ok(7),

        // Opcode + u16 + u16
        OpCode::CallDynamic => Ok(5),
        OpCode::SpawnDynamic => Ok(1),

        // Opcode + u16 + u8 + 3 bytes padding
        OpCode::CallNative => Ok(7),

        // Module system: opcode + u16 constant-pool index
        OpCode::ImportModule | OpCode::ExportValue => Ok(3),
    }
}

fn parse_u16_operand(op: &str) -> Result<u16, String> {
    op.parse::<u16>()
        .map_err(|_| format!("invalid u16 operand: '{}'", op))
}

fn parse_u8_operand(op: &str) -> Result<u8, String> {
    op.parse::<u8>()
        .map_err(|_| format!("invalid u8 operand: '{}'", op))
}
