//! `vre uninstall` — Remove a locally installed package.

use std::path::Path;
use std::process;
use crate::cli::UninstallArgs;
use crate::diagnostics::{codes, Diagnostic};

pub fn run(args: UninstallArgs) {
    let pkg_dir = Path::new("vym_modules").join(&args.package);

    println!();

    if !pkg_dir.exists() {
        Diagnostic::error(codes::E001, format!("Package '{}' is not installed.", args.package))
            .with_hint("Check vym_modules/ for installed packages.")
            .emit();
        process::exit(1);
    }

    match std::fs::remove_dir_all(&pkg_dir) {
        Ok(()) => {
            println!("  ✓ Uninstalled '{}'", args.package);
            println!();
        }
        Err(e) => {
            Diagnostic::error(codes::E010, format!("Failed to remove '{}': {}", args.package, e)).emit();
            process::exit(1);
        }
    }
}
