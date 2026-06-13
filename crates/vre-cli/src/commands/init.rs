//! `vre init` — Initialize a project in the current directory.

use std::env;
use std::path::Path;
use crate::cli::InitArgs;
use crate::config::VreToml;
use crate::diagnostics::{codes, Diagnostic};

pub fn run(args: InitArgs) {
    let cwd = env::current_dir().unwrap_or_else(|_| ".".into());

    // Determine project name: flag > directory name > "my-app"
    let name = args.name.unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-app")
            .to_string()
    });

    println!();
    println!("  Initializing project '{}'...", name);
    println!();

    // Check if vre.toml already exists
    if cwd.join("vre.toml").exists() {
        Diagnostic::error(codes::E015, "vre.toml already exists in this directory.")
            .with_hint("Delete it or use `vre new` to create a project in a new directory.")
            .emit();
        std::process::exit(1);
    }

    match VreToml::init(&cwd, &name) {
        Ok(()) => {
            println!("  ✓ Created vre.toml");
            println!();
            println!("  Project initialized.");
            println!("  Edit vre.toml to configure your project, then run:");
            println!();
            println!("    vre run");
            println!();
        }
        Err(e) => {
            Diagnostic::error(codes::E003, e).emit();
            std::process::exit(1);
        }
    }
}
