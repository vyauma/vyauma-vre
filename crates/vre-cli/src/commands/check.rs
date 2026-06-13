//! `vre check` — Type-check a source file without executing it.

use std::path::Path;
use std::process;

use crate::cli::CheckArgs;
use crate::diagnostics::{self, codes, Diagnostic};

pub fn run(args: CheckArgs) {
    let input_path = &args.file;

    if !Path::new(input_path).exists() {
        Diagnostic::error(codes::E014, format!("File not found: '{}'", input_path)).emit();
        process::exit(1);
    }

    let source = match vre_core::pal::get_pal().read_to_string(Path::new(input_path)) {
        Ok(s) => s,
        Err(e) => {
            Diagnostic::error(codes::E014, format!("Failed to read source file: {}", e)).emit();
            process::exit(1);
        }
    };

    let path = Path::new(input_path);
    let base_path = path.parent().unwrap_or(Path::new("."));

    match vre_compiler::compile(&source, input_path, Some(base_path)) {
        Ok(_) => {
            println!();
            println!("  ✓ Type check passed: {}", input_path);
            println!();
        }
        Err(e) => {
            diagnostics::emit_compiler_error(&source, input_path, &e);
            process::exit(1);
        }
    }
}
