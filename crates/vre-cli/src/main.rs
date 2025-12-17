//! Vyauma Runtime Engine - CLI
//!
//! Minimal command-line interface to execute Vyauma bytecode.

use std::env;
use std::fs;
use std::process;

use vre_core::config::VreConfig;
use vre_core::loader::loader::BytecodeLoader;
use vre_core::vm::vm::VirtualMachine;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let bytecode_path = &args[1];

    // Read bytecode file
    let bytes = match fs::read(bytecode_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: failed to read bytecode file: {}", e);
            process::exit(1);
        }
    };

    // Load bytecode
    let loaded = match BytecodeLoader::load(&bytes) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Error: invalid bytecode: {}", e);
            process::exit(1);
        }
    };

    // Create VM
    let config = VreConfig::default();
    let mut vm = VirtualMachine::new(
        config,
        loaded.constants,
        loaded.instructions,
        0, // global variable count (v0.1)
    );

    // NOTE: entry_point wiring will be added when VM supports it

    // Execute
    if let Err(e) = vm.execute() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn print_usage(program: &str) {
    eprintln!("Vyauma Runtime Engine (VRE)");
    eprintln!("Usage: {} <bytecode_file>", program);
}
