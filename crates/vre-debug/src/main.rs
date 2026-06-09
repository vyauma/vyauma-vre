use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

use vre_core::config::VreConfig;
use vre_core::loader::loader::BytecodeLoader;
use vre_core::vm::vm::VirtualMachine;
use vre_core::{Capability, CapabilityRegistry};
use vre_core::bytecode::opcode::OpCode;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: vre-debug <file.vym | file.vyma>");
        process::exit(1);
    }

    let input_path = &args[1];

    let (instructions, constants) = if input_path.ends_with(".vym") {
        let source = match fs::read_to_string(input_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: failed to read source file: {}", e);
                process::exit(1);
            }
        };
        let path = std::path::Path::new(input_path);
        let base_path = path.parent().unwrap_or(std::path::Path::new("."));
        match vre_compiler::compile(&source, Some(base_path)) {
            Ok(compiled) => (compiled.instructions, compiled.constants),
            Err(e) => {
                eprintln!("Compile Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        let bytes = fs::read(input_path).expect("Failed to read bytecode");
        let loaded = BytecodeLoader::load(&bytes).expect("Invalid bytecode");
        (loaded.instructions, loaded.constants)
    };

    let mut capabilities = CapabilityRegistry::new();
    capabilities.grant(Capability::new("io.read"));
    capabilities.grant(Capability::new("io.write"));
    capabilities.grant(Capability::new("fs.read"));
    capabilities.grant(Capability::new("fs.write"));
    capabilities.grant(Capability::new("net.connect"));
    capabilities.grant(Capability::new("net.listen"));

    let config = VreConfig::default();
    let mut vm = VirtualMachine::new(
        config,
        instructions,
        constants,
        vec![],
        capabilities,
    ).expect("Failed to initialize VM");

    println!("Vyauma Bytecode Debugger");
    println!("Type 'help' for commands.");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut breakpoints: std::collections::HashSet<usize> = std::collections::HashSet::new();

    loop {
        if vm.halted() {
            println!("VM halted.");
            break;
        }

        let ip = vm.ip();
        let instrs = vm.instructions();
        if ip >= instrs.len() {
            println!("Execution reached end of instructions.");
            break;
        }

        let op_byte = instrs[ip];
        if let Some(op) = OpCode::from_u8(op_byte) {
            println!("\n[IP: 0x{:04X}] {:?}", ip, op);
        } else {
            println!("\n[IP: 0x{:04X}] Unknown OpCode: 0x{:02X}", ip, op_byte);
        }

        print!("debug> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "s" | "step" => {
                if let Err(e) = vm.step() {
                    println!("Runtime error during step: {}", e);
                    break;
                }
            }
            "c" | "continue" => {
                println!("Continuing execution...");
                loop {
                    if vm.halted() || vm.ip() >= vm.instructions().len() {
                        break;
                    }
                    if let Err(e) = vm.step() {
                        println!("Runtime error: {}", e);
                        break;
                    }
                    if breakpoints.contains(&vm.ip()) {
                        println!("Hit breakpoint at 0x{:04X}", vm.ip());
                        break;
                    }
                }
            }
            "b" | "break" => {
                if let Some(addr_str) = parts.next() {
                    let addr_res = if addr_str.starts_with("0x") {
                        usize::from_str_radix(&addr_str[2..], 16)
                    } else {
                        addr_str.parse::<usize>()
                    };
                    
                    match addr_res {
                        Ok(addr) => {
                            breakpoints.insert(addr);
                            println!("Breakpoint set at 0x{:04X}", addr);
                        }
                        Err(_) => println!("Invalid address format. Use decimal or 0x hex."),
                    }
                } else {
                    println!("Usage: break <address>");
                }
            }
            "rb" | "rmbreak" => {
                if let Some(addr_str) = parts.next() {
                    let addr_res = if addr_str.starts_with("0x") {
                        usize::from_str_radix(&addr_str[2..], 16)
                    } else {
                        addr_str.parse::<usize>()
                    };
                    
                    match addr_res {
                        Ok(addr) => {
                            if breakpoints.remove(&addr) {
                                println!("Breakpoint removed at 0x{:04X}", addr);
                            } else {
                                println!("No breakpoint found at 0x{:04X}", addr);
                            }
                        }
                        Err(_) => println!("Invalid address format."),
                    }
                } else {
                    println!("Usage: rb <address>");
                }
            }
            "bl" | "breaklist" => {
                println!("Breakpoints:");
                if breakpoints.is_empty() {
                    println!("  (none)");
                } else {
                    for bp in &breakpoints {
                        println!("  0x{:04X}", bp);
                    }
                }
            }
            "st" | "stack" => {
                let stack_vals = vm.stack().values();
                println!("Stack ({} items):", stack_vals.len());
                for (i, val) in stack_vals.iter().enumerate() {
                    println!("  {}: {:?}", i, val);
                }
            }
            "l" | "locals" => {
                if let Ok(frame) = vm.current_frame() {
                    let locals = frame.locals.values();
                    println!("Locals ({} items):", locals.len());
                    for (i, val) in locals.iter().enumerate() {
                        println!("  {}: {:?}", i, val);
                    }
                } else {
                    println!("No active call frame.");
                }
            }
            "q" | "quit" => {
                println!("Exiting debugger.");
                break;
            }
            "h" | "help" => {
                println!("Commands:");
                println!("  s, step      - Execute one instruction");
                println!("  c, continue  - Run until halt or breakpoint");
                println!("  b, break <addr> - Set breakpoint at address (e.g. 0x000B or 11)");
                println!("  rb, rmbreak <addr> - Remove breakpoint");
                println!("  bl, breaklist - List all breakpoints");
                println!("  st, stack    - Print current stack");
                println!("  l, locals    - Print local variables of current frame");
                println!("  q, quit      - Exit debugger");
                println!("  h, help      - Show this help message");
            }
            _ => {
                println!("Unknown command: {}", cmd);
            }
        }
    }

    match vm.stack().values().last() {
        Some(val) => println!("Execution finished. Top of stack: {:?}", val),
        None => println!("Execution finished. Stack is empty."),
    }
}
