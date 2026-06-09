//! Vyauma Bytecode Assembler - CLI Entry Point
//!
//! Driver for assembling text source files into Vyauma bytecode files.

use std::env;
use std::fs;
use std::process;

mod assembler;
use assembler::Assembler;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let source_path = &args[1];
    let output_path = &args[2];

    // Read source code
    let source = match fs::read_to_string(source_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: failed to read assembly source file: {}", e);
            process::exit(1);
        }
    };

    // Assemble
    let mut assembler = Assembler::new();
    let bytecode = match assembler.assemble(&source) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Assembly Error: {}", e);
            process::exit(1);
        }
    };

    // Write binary bytecode
    if let Err(e) = fs::write(output_path, bytecode) {
        eprintln!("Error: failed to write output bytecode file: {}", e);
        process::exit(1);
    }

    println!("Assembled successfully: {} -> {}", source_path, output_path);
}

fn print_usage(program: &str) {
    eprintln!("Vyauma Bytecode Assembler v0.1");
    eprintln!("Usage: {} <source_file.vasm> <output_file.vyma>", program);
}
