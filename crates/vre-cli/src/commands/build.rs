//! `vre build` — Build the project for a target platform.

use std::fs;
use std::path::Path;
use std::process;

use crate::cli::{BuildArgs, BuildTarget};
use crate::config::VreToml;
use crate::diagnostics::{codes, Diagnostic};

pub fn run(args: BuildArgs) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let target = args.target.to_string();
    let out_dir = &args.out_dir;

    println!();
    println!("  Building for target: {}", target);
    println!("  Output: {}/", out_dir);
    println!();

    // Load project manifest
    let (manifest, manifest_path) = match VreToml::find_and_load(&cwd) {
        Ok(m) => m,
        Err(e) => {
            Diagnostic::error(codes::E003, e)
                .with_suggestion("Run `vre init` to create a vre.toml")
                .emit();
            process::exit(1);
        }
    };

    let project_root = manifest_path.parent().unwrap_or(Path::new("."));
    let entry = project_root.join(&manifest.project.entry);

    if !entry.exists() {
        Diagnostic::error(codes::E014, format!(
            "Entry point not found: '{}'", entry.display()
        ))
        .with_hint("Check the 'entry' field in your vre.toml.")
        .emit();
        process::exit(1);
    }

    // Create output directory
    let dist = project_root.join(out_dir);
    if let Err(e) = fs::create_dir_all(&dist) {
        Diagnostic::error(codes::E010, format!("Failed to create output dir: {}", e)).emit();
        process::exit(1);
    }

    // Compile source
    println!("  Compiling {}...", entry.display());
    let source = match vre_core::pal::get_pal().read_to_string(&entry) {
        Ok(s) => s,
        Err(e) => {
            Diagnostic::error(codes::E014, format!("Failed to read {}: {}", entry.display(), e)).emit();
            process::exit(1);
        }
    };

    let base_path = entry.parent().unwrap_or(Path::new("."));
    let entry_str = entry.to_string_lossy().to_string();

    match vre_compiler::compile(&source, &entry_str, Some(base_path)) {
        Ok(_compiled) => {
            // In a full implementation, emit target-specific binary/bytecode
            // For now write a build metadata file
            let meta = format!(
                "# VRE Build Output\nname = \"{}\"\nversion = \"{}\"\ntarget = \"{}\"\n",
                manifest.project.name, manifest.project.version, target
            );
            let meta_path = dist.join("build.toml");
            if let Err(e) = fs::write(&meta_path, &meta) {
                Diagnostic::error(codes::E010, format!("Failed to write build metadata: {}", e)).emit();
                process::exit(1);
            }

            println!("  ✓ Compiled {}", manifest.project.name);
            println!("  ✓ Target: {}", target);
            println!("  ✓ Output: {}/", dist.display());
            println!();
            println!("  Build complete: {} v{}", manifest.project.name, manifest.project.version);
            println!();
        }
        Err(e) => {
            crate::diagnostics::emit_compiler_error(&source, &entry_str, &e);
            process::exit(1);
        }
    }
}
