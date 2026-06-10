//! Vyauma Runtime Engine - CLI
//!
//! Minimal command-line interface to execute Vyauma bytecode.

use std::env;
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
        let source = match vre_core::pal::get_pal().read_to_string(std::path::Path::new(input_path)) {
            Ok(s) => s,
            Err(e) => {
                vre_core::pal::get_pal().eprintln(&format!("Error: failed to read source file: {}", e));
                process::exit(1);
            }
        };
        let path = std::path::Path::new(input_path);
        let base_path = path.parent().unwrap_or(std::path::Path::new("."));
        match vre_compiler::compile(&source, input_path, Some(base_path)) {
            Ok(compiled) => (compiled.instructions, compiled.constants, compiled.native_imports),
            Err(e) => {
                render_diagnostic(&source, input_path, &e);
                process::exit(1);
            }
        }
    } else {
        let bytes = match std::fs::read(input_path) {
            Ok(b) => b,
            Err(e) => {
                vre_core::pal::get_pal().eprintln(&format!("Error: failed to read bytecode file: {}", e));
                process::exit(1);
            }
        };

        let loaded = match BytecodeLoader::load(&bytes) {
            Ok(bc) => bc,
            Err(e) => {
                vre_core::pal::get_pal().eprintln(&format!("Error: invalid bytecode: {}", e));
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
            vre_core::pal::get_pal().eprintln(&format!("VM Init Error: {}", e));
            process::exit(1);
        }
    };

    if let Err(e) = vm.execute() {
        vre_core::pal::get_pal().eprintln(&format!("Runtime error: {}", e));
        process::exit(1);
    }
}

fn print_usage(program_name: &str) {
    println!("Vyauma Runtime Engine (VRE)");
    println!("Usage:");
    println!("  {} <file.vbc>   - Execute compiled bytecode", program_name);
    println!("  {} <file.vym>    - Compile and execute Vyauma source", program_name);
}

/// Renders a compiler error with a visual `^` pointer to the exact location.
///
/// Error strings with source location have the format: `[line:col] message`
/// e.g. `[5:12] Expected Colon, got Identifier("x")`
///
/// For type errors (no span prefix), prints a plain error with a hint.
fn render_diagnostic(source: &str, filename: &str, error: &str) {
    let pal = vre_core::pal::get_pal();

    // Try to parse a [line:col] prefix from the error string.
    // Pattern: starts with '[', then digits, ':', digits, ']'
    if error.starts_with('[') {
        if let Some(close) = error.find(']') {
            let span_part = &error[1..close];
            let rest = error[close + 1..].trim();
            let parts: Vec<&str> = span_part.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let (Ok(line_num), Ok(col_num)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    let lines: Vec<&str> = source.lines().collect();
                    let source_line = if line_num > 0 && line_num <= lines.len() {
                        lines[line_num - 1]
                    } else {
                        ""
                    };
                    let padding = " ".repeat(col_num.saturating_sub(1));
                    pal.eprintln(&format!("\nerror[E]: {}", rest));
                    pal.eprintln(&format!("  --> {}:{}:{}", filename, line_num, col_num));
                    pal.eprintln(&format!("   |"));
                    pal.eprintln(&format!("{:>3} | {}", line_num, source_line));
                    pal.eprintln(&format!("   | {}^", padding));
                    pal.eprintln(&format!("   |"));
                    return;
                }
            }
        }
    }

    // Fallback: plain error (type errors, etc.)
    pal.eprintln(&format!("\nerror: {}", error));
    pal.eprintln(&format!("  --> {}", filename));
}
