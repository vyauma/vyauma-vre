//! Vyauma Runtime Engine - CLI
//!
//! Minimal command-line interface to execute Vyauma bytecode.

use std::env;
use std::fs;
use std::process;

use vre_core::config::VreConfig;
use vre_core::loader::loader::BytecodeLoader;
use vre_core::vm::vm::VirtualMachine;
use vre_core::{Capability, CapabilityRegistry};

mod native;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let input_path = &args[1];

    let (instructions, constants, native_imports) = if input_path.ends_with(".vym") {
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
            Ok(compiled) => (compiled.instructions, compiled.constants, compiled.native_imports),
            Err(e) => {
                eprintln!("Compile Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        let bytes = match fs::read(input_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error: failed to read bytecode file: {}", e);
                process::exit(1);
            }
        };

        let loaded = match BytecodeLoader::load(&bytes) {
            Ok(bc) => bc,
            Err(e) => {
                eprintln!("Error: invalid bytecode: {}", e);
                process::exit(1);
            }
        };
        // Bytecode loader doesn't support native_imports yet
        (loaded.instructions, loaded.constants, Vec::new())
    };

    // Set up Configuration and FFI
    let mut config = VreConfig::default();
    native::register_ffi(&mut config);

    let mut capabilities = CapabilityRegistry::new();
    capabilities.grant(Capability::new("io.read"));
    capabilities.grant(Capability::new("io.write"));
    capabilities.grant(Capability::new("fs.read"));
    capabilities.grant(Capability::new("fs.write"));
    capabilities.grant(Capability::new("net.listen"));
    capabilities.grant(Capability::new("net.accept"));
    capabilities.grant(Capability::new("net.connect"));

    // Run VM
    let mut vm = match VirtualMachine::new(config, instructions, constants, native_imports, capabilities) {
        Ok(vm) => vm,
        Err(e) => {
            eprintln!("VM Init Error: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = vm.execute() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn print_usage(program_name: &str) {
    println!("Vyauma Runtime Engine (VRE)");
    println!("Usage:");
    println!("  {} <file.vbc>   - Execute compiled bytecode", program_name);
    println!("  {} <file.vym>    - Compile and execute Vyauma source", program_name);
}
