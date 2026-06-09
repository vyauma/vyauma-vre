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
    let bytecode = compile(&source_code, base_path).expect("Compilation failed");

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
                println!("  s, step      Step one instruction");
                println!("  c, cont      Continue execution");
                println!("  b, break <ip> Set breakpoint at instruction pointer");
                println!("  is           Info stack");
                println!("  il           Info locals");
                println!("  q, quit      Quit debugger");
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
            "q" | "quit" => {
                break;
            }
            _ => {
                println!("Unknown command: {}. Type 'h' for help.", parts[0]);
            }
        }
    }
}

fn disassemble(vm: &VirtualMachine, ip: usize) -> (String, usize) {
    let insts = vm.instructions();
    if ip >= insts.len() {
        return ("EOF".to_string(), ip);
    }
    
    let opcode_byte = insts[ip];
    let opcode = match OpCode::from_u8(opcode_byte) {
        Some(op) => op,
        None => return (format!("Unknown opcode: 0x{:02X}", opcode_byte), ip + 1),
    };

    let mut string = format!("{:?}", opcode);
    let mut next_ip = ip + 1;

    match opcode {
        OpCode::Push | OpCode::LoadLocal | OpCode::StoreLocal | OpCode::CallNative => {
            if next_ip + 1 < insts.len() {
                let operand = ((insts[next_ip] as u16) << 8) | (insts[next_ip + 1] as u16);
                string.push_str(&format!(" {}", operand));
                next_ip += 2;
                
                if opcode == OpCode::Push {
                    if let Ok(val) = vm.constants().get(operand as usize) {
                        string.push_str(&format!(" ({:?})", val));
                    }
                }
            }
        }
        OpCode::Jump | OpCode::JumpIf => {
            if next_ip + 3 < insts.len() {
                let target = ((insts[next_ip] as u32) << 24)
                    | ((insts[next_ip + 1] as u32) << 16)
                    | ((insts[next_ip + 2] as u32) << 8)
                    | (insts[next_ip + 3] as u32);
                string.push_str(&format!(" 0x{:04X}", target));
                next_ip += 4;
            }
        }
        OpCode::Call => {
            if next_ip + 5 < insts.len() {
                let target = ((insts[next_ip] as u32) << 24)
                    | ((insts[next_ip + 1] as u32) << 16)
                    | ((insts[next_ip + 2] as u32) << 8)
                    | (insts[next_ip + 3] as u32);
                let args = ((insts[next_ip + 4] as u16) << 8) | (insts[next_ip + 5] as u16);
                string.push_str(&format!(" target: 0x{:04X}, locals: {}", target, args));
                next_ip += 6;
            }
        }
        OpCode::Syscall => {
            if next_ip < insts.len() {
                string.push_str(&format!(" 0x{:02X}", insts[next_ip]));
                next_ip += 1;
            }
        }
        _ => {}
    }

    (string, next_ip)
}
