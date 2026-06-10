use std::env;
use std::fs;
use std::io::{self, Write};
use std::collections::HashSet;
use vre_core::config::VreConfig;
use vre_core::vm::vm::VirtualMachine;
use vre_compiler::compile;
use vre_core::bytecode::opcode::OpCode;
use std::path::Path;

#[path = "../native.rs"]
mod native;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vre-debug <file.vym>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let source_code = fs::read_to_string(file_path).expect("Failed to read file");

    let base_path = Path::new(file_path).parent();
    let bytecode = compile(&source_code, file_path, base_path).expect("Compilation failed");

    let mut config = VreConfig::default();
    
    // Register native functions
    native::register_ffi(&mut config);

    let mut capabilities = vre_core::CapabilityRegistry::new();
    capabilities.grant(vre_core::Capability::new("io.read"));
    capabilities.grant(vre_core::Capability::new("io.write"));
    capabilities.grant(vre_core::Capability::new("fs.read"));
    capabilities.grant(vre_core::Capability::new("fs.write"));
    capabilities.grant(vre_core::Capability::new("net.listen"));
    capabilities.grant(vre_core::Capability::new("net.accept"));
    capabilities.grant(vre_core::Capability::new("net.connect"));

    let mut vm = VirtualMachine::new(
        config, 
        bytecode.instructions, 
        bytecode.constants, 
        bytecode.native_imports, 
        capabilities
    ).expect("Failed to initialize VM");

    println!("VRE Debugger v0.1");
    println!("Loaded {}", file_path);
    println!("Type 'h' for help.");

    let mut breakpoints: HashSet<usize> = HashSet::new();
    let mut last_cmd = String::new();

    loop {
        if vm.halted() || vm.ip() >= vm.instructions().len() {
            println!("Execution finished.");
            break;
        }

        // Print current instruction
        let (disasm, _next_ip) = disassemble(&vm, vm.ip());
        println!("0x{:04X} | {}", vm.ip(), disasm);

        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        let cmd = if input.is_empty() { last_cmd.clone() } else { input.to_string() };
        last_cmd = cmd.clone();

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "h" | "help" => {
                println!("Commands:");
                println!("  s, step         Step one instruction");
                println!("  c, cont         Continue execution");
                println!("  b, break <ip>   Set breakpoint at instruction pointer");
                println!("  lb              List all breakpoints");
                println!("  db <ip>         Delete breakpoint at ip");
                println!("  disasm          Full disassembly listing");
                println!("  disasm <ip>     Disassemble from given address");
                println!("  is              Info stack");
                println!("  il              Info locals");
                println!("  ic              Info call stack depth");
                println!("  q, quit         Quit debugger");
            }
            "s" | "step" => {
                if let Err(e) = vm.step() {
                    println!("Runtime Error: {:?}", e);
                    break;
                }
            }
            "c" | "cont" => {
                loop {
                    if let Err(e) = vm.step() {
                        println!("Runtime Error: {:?}", e);
                        break;
                    }
                    if vm.halted() || vm.ip() >= vm.instructions().len() {
                        println!("Execution finished.");
                        break;
                    }
                    if breakpoints.contains(&vm.ip()) {
                        println!("Breakpoint hit at 0x{:04X}", vm.ip());
                        break;
                    }
                }
            }
            "b" | "break" => {
                if parts.len() > 1 {
                    let ip_str = parts[1].trim_start_matches("0x");
                    if let Ok(ip) = usize::from_str_radix(ip_str, 16) {
                        breakpoints.insert(ip);
                        println!("Breakpoint set at 0x{:04X}", ip);
                    } else if let Ok(ip) = parts[1].parse::<usize>() {
                        breakpoints.insert(ip);
                        println!("Breakpoint set at 0x{:04X}", ip);
                    } else {
                        println!("Invalid IP format.");
                    }
                } else {
                    println!("Usage: b <ip>");
                }
            }
            "is" => {
                let stack = vm.stack().elements();
                println!("Stack ({} elements):", stack.len());
                for (i, val) in stack.iter().enumerate().rev() {
                    println!("  [{}] {:?}", i, val);
                }
            }
            "il" => {
                if let Ok(frame) = vm.current_frame() {
                    let locals = frame.locals.elements();
                    println!("Locals ({} elements):", locals.len());
                    for (i, val) in locals.iter().enumerate() {
                        println!("  [{}] {:?}", i, val);
                    }
                } else {
                    println!("No active frame.");
                }
            }
            "lb" => {
                if breakpoints.is_empty() {
                    println!("No breakpoints set.");
                } else {
                    println!("Breakpoints:");
                    let mut bps: Vec<usize> = breakpoints.iter().cloned().collect();
                    bps.sort();
                    for bp in bps {
                        println!("  0x{:04X}", bp);
                    }
                }
            }
            "db" => {
                if parts.len() > 1 {
                    let ip_str = parts[1].trim_start_matches("0x");
                    let ip = usize::from_str_radix(ip_str, 16)
                        .or_else(|_| parts[1].parse::<usize>());
                    match ip {
                        Ok(addr) => {
                            if breakpoints.remove(&addr) {
                                println!("Breakpoint at 0x{:04X} removed.", addr);
                            } else {
                                println!("No breakpoint at 0x{:04X}.", addr);
                            }
                        }
                        Err(_) => println!("Invalid address."),
                    }
                } else {
                    println!("Usage: db <ip>");
                }
            }
            "disasm" => {
                let start_ip = if parts.len() > 1 {
                    let ip_str = parts[1].trim_start_matches("0x");
                    usize::from_str_radix(ip_str, 16)
                        .or_else(|_| parts[1].parse::<usize>())
                        .unwrap_or(0)
                } else {
                    0
                };
                print_disassembly(&vm, start_ip);
            }
            "ic" => {
                println!("Call stack depth: {}", vm.call_stack().len());
                println!("Current IP:       0x{:04X}", vm.ip());
            }
            "q" | "quit" => {
                break;
            }
            _ => {
                println!("Unknown command: {}. Type 'h' for help.", parts[0]);
            }
        }
    }
}

/// Print a full disassembly listing starting from start_ip
fn print_disassembly(vm: &VirtualMachine, start_ip: usize) {
    let insts = vm.instructions();
    let total = insts.len();
    if start_ip >= total {
        println!("Address 0x{:04X} is past end of instructions (len={}).", start_ip, total);
        return;
    }

    println!();
    println!("  ┌─────────────────────────────────────────────────────────┐");
    println!("  │  DISASSEMBLY  ({} bytes total)                          ", total);
    println!("  └─────────────────────────────────────────────────────────┘");
    println!("  {:>6}  {:>5}  {}", "ADDR", "HEX", "INSTRUCTION");
    println!("  {}", "─".repeat(55));

    let mut ip = start_ip;
    while ip < total {
        let raw_byte = insts[ip];
        let (disasm, next_ip) = disassemble(vm, ip);

        // Show if this is the current execution point
        let marker = if ip == vm.ip() { "→" } else { " " };

        println!("  {} 0x{:04X}  {:02X}     {}", marker, ip, raw_byte, disasm);
        ip = next_ip;
    }
    println!();
}

fn disassemble(vm: &VirtualMachine, ip: usize) -> (String, usize) {
    let insts = vm.instructions();
    if ip >= insts.len() {
        return ("EOF".to_string(), ip);
    }

    let opcode_byte = insts[ip];
    let opcode = match OpCode::from_u8(opcode_byte) {
        Some(op) => op,
        None => return (format!("??? 0x{:02X}", opcode_byte), ip + 1),
    };

    let mut next_ip = ip + 1;

    let text = match opcode {
        // ── Stack ops ──────────────────────────────────────────────────
        OpCode::Push => {
            let operand = read_u16(insts, next_ip);
            next_ip += 2;
            let const_str = vm.constants().get(operand as usize)
                .map(|v| format!(" ; {:?}", v))
                .unwrap_or_default();
            format!("PUSH #{}{}", operand, const_str)
        }
        OpCode::Pop  => "POP".to_string(),
        OpCode::Dup  => "DUP".to_string(),

        // ── Locals ─────────────────────────────────────────────────────
        OpCode::LoadLocal     => { let i = read_u16(insts, next_ip); next_ip += 2; format!("LOAD_LOCAL  [{}]", i) }
        OpCode::LoadLocalI32  => { let i = read_u16(insts, next_ip); next_ip += 2; format!("LOAD_LOCAL_I32 [{}]", i) }
        OpCode::LoadLocalI64  => { let i = read_u16(insts, next_ip); next_ip += 2; format!("LOAD_LOCAL_I64 [{}]", i) }
        OpCode::LoadLocalF32  => { let i = read_u16(insts, next_ip); next_ip += 2; format!("LOAD_LOCAL_F32 [{}]", i) }
        OpCode::LoadLocalF64  => { let i = read_u16(insts, next_ip); next_ip += 2; format!("LOAD_LOCAL_F64 [{}]", i) }
        OpCode::LoadLocalStr  => { let i = read_u16(insts, next_ip); next_ip += 2; format!("LOAD_LOCAL_STR [{}]", i) }
        OpCode::StoreLocal    => { let i = read_u16(insts, next_ip); next_ip += 2; format!("STORE_LOCAL [{}]", i) }
        OpCode::LoadProperty  => { let i = read_u16(insts, next_ip); next_ip += 2;
            let name = vm.constants().get(i as usize).map(|v| format!("{:?}", v)).unwrap_or_default();
            format!("LOAD_PROPERTY #{} {}", i, name)
        }
        OpCode::StoreProperty => { let i = read_u16(insts, next_ip); next_ip += 2;
            let name = vm.constants().get(i as usize).map(|v| format!("{:?}", v)).unwrap_or_default();
            format!("STORE_PROPERTY #{} {}", i, name)
        }

        // ── Arithmetic ─────────────────────────────────────────────────
        OpCode::AddI32 => "ADD_I32".to_string(),   OpCode::SubI32 => "SUB_I32".to_string(),
        OpCode::MulI32 => "MUL_I32".to_string(),   OpCode::DivI32 => "DIV_I32".to_string(),
        OpCode::ModI32 => "MOD_I32".to_string(),   OpCode::NegI32 => "NEG_I32".to_string(),
        OpCode::AddI64 => "ADD_I64".to_string(),   OpCode::SubI64 => "SUB_I64".to_string(),
        OpCode::MulI64 => "MUL_I64".to_string(),   OpCode::DivI64 => "DIV_I64".to_string(),
        OpCode::ModI64 => "MOD_I64".to_string(),   OpCode::NegI64 => "NEG_I64".to_string(),
        OpCode::AddF32 => "ADD_F32".to_string(),   OpCode::SubF32 => "SUB_F32".to_string(),
        OpCode::MulF32 => "MUL_F32".to_string(),   OpCode::DivF32 => "DIV_F32".to_string(),
        OpCode::ModF32 => "MOD_F32".to_string(),   OpCode::NegF32 => "NEG_F32".to_string(),
        OpCode::AddF64 => "ADD_F64".to_string(),   OpCode::SubF64 => "SUB_F64".to_string(),
        OpCode::MulF64 => "MUL_F64".to_string(),   OpCode::DivF64 => "DIV_F64".to_string(),
        OpCode::ModF64 => "MOD_F64".to_string(),   OpCode::NegF64 => "NEG_F64".to_string(),
        OpCode::AddStr => "ADD_STR".to_string(),

        // ── Comparison ─────────────────────────────────────────────────
        OpCode::EqualI32 => "EQ_I32".to_string(),    OpCode::NotEqualI32 => "NE_I32".to_string(),
        OpCode::LessI32  => "LT_I32".to_string(),    OpCode::LessEqualI32 => "LE_I32".to_string(),
        OpCode::GreaterI32 => "GT_I32".to_string(),  OpCode::GreaterEqualI32 => "GE_I32".to_string(),
        OpCode::EqualI64 => "EQ_I64".to_string(),    OpCode::NotEqualI64 => "NE_I64".to_string(),
        OpCode::LessI64  => "LT_I64".to_string(),    OpCode::LessEqualI64 => "LE_I64".to_string(),
        OpCode::GreaterI64 => "GT_I64".to_string(),  OpCode::GreaterEqualI64 => "GE_I64".to_string(),
        OpCode::EqualF32 => "EQ_F32".to_string(),    OpCode::NotEqualF32 => "NE_F32".to_string(),
        OpCode::LessF32  => "LT_F32".to_string(),    OpCode::LessEqualF32 => "LE_F32".to_string(),
        OpCode::GreaterF32 => "GT_F32".to_string(),  OpCode::GreaterEqualF32 => "GE_F32".to_string(),
        OpCode::EqualF64 => "EQ_F64".to_string(),    OpCode::NotEqualF64 => "NE_F64".to_string(),
        OpCode::LessF64  => "LT_F64".to_string(),    OpCode::LessEqualF64 => "LE_F64".to_string(),
        OpCode::GreaterF64 => "GT_F64".to_string(),  OpCode::GreaterEqualF64 => "GE_F64".to_string(),
        OpCode::EqualStr => "EQ_STR".to_string(),    OpCode::NotEqualStr => "NE_STR".to_string(),
        OpCode::AndBool  => "AND_BOOL".to_string(),  OpCode::OrBool => "OR_BOOL".to_string(),

        // ── Control Flow ───────────────────────────────────────────────
        OpCode::Jump => {
            let target = read_u32(insts, next_ip);
            next_ip += 4;
            format!("JUMP       0x{:04X}", target)
        }
        OpCode::JumpIf => {
            let target = read_u32(insts, next_ip);
            next_ip += 4;
            format!("JUMP_IF    0x{:04X}", target)
        }
        OpCode::Call => {
            let target = read_u32(insts, next_ip);
            let locals = read_u16(insts, next_ip + 4);
            next_ip += 6;
            format!("CALL       0x{:04X}  locals={}", target, locals)
        }
        OpCode::Return => "RETURN".to_string(),

        // ── Async ──────────────────────────────────────────────────────
        OpCode::Spawn => {
            let target = read_u32(insts, next_ip);
            next_ip += 4;
            format!("SPAWN      0x{:04X}", target)
        }
        OpCode::Yield => "YIELD".to_string(),
        OpCode::Await => "AWAIT".to_string(),

        // ── Heap / Objects ─────────────────────────────────────────────
        OpCode::NewArray     => "NEW_ARRAY".to_string(),
        OpCode::LoadElement  => "LOAD_ELEMENT".to_string(),
        OpCode::StoreElement => "STORE_ELEMENT".to_string(),
        OpCode::NewStruct    => "NEW_STRUCT".to_string(),

        // ── FFI / Native ───────────────────────────────────────────────
        OpCode::CallNative => {
            let native_idx = read_u16(insts, next_ip);
            let arg_count = if next_ip + 2 < insts.len() { insts[next_ip + 2] } else { 0 };
            next_ip += 6; // u16 + u8 + 3 padding
            format!("CALL_NATIVE #{} args={}", native_idx, arg_count)
        }
        OpCode::Syscall => {
            let id = if next_ip < insts.len() { insts[next_ip] } else { 0 };
            next_ip += 1;
            format!("SYSCALL    0x{:02X}", id)
        }

        // ── Exceptions ─────────────────────────────────────────────────
        OpCode::TryStart => {
            let catch_offset = read_u32(insts, next_ip);
            next_ip += 4;
            format!("TRY_START  catch→0x{:04X}", catch_offset)
        }
        OpCode::TryEnd => "TRY_END".to_string(),
        OpCode::Throw  => "THROW".to_string(),

        // ── System ─────────────────────────────────────────────────────
        OpCode::Nop  => "NOP".to_string(),
        OpCode::Halt => "HALT".to_string(),
    };

    (text, next_ip)
}

#[inline]
fn read_u16(insts: &[u8], ip: usize) -> u16 {
    if ip + 1 < insts.len() {
        ((insts[ip] as u16) << 8) | (insts[ip + 1] as u16)
    } else {
        0
    }
}

#[inline]
fn read_u32(insts: &[u8], ip: usize) -> u32 {
    if ip + 3 < insts.len() {
        ((insts[ip] as u32) << 24)
            | ((insts[ip + 1] as u32) << 16)
            | ((insts[ip + 2] as u32) << 8)
            | (insts[ip + 3] as u32)
    } else {
        0
    }
}

