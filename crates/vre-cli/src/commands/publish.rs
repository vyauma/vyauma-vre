//! `vre publish` — Publish a package to the VRE Registry.

use std::process;
use crate::cli::PublishArgs;
use crate::config::VreToml;
use crate::diagnostics::{codes, Diagnostic};
use crate::registry::RegistryClient;

pub fn run(args: PublishArgs) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());

    let (manifest, _) = match VreToml::find_and_load(&cwd) {
        Ok(m) => m,
        Err(e) => {
            Diagnostic::error(codes::E003, e)
                .with_suggestion("Run `vre init` to create a vre.toml")
                .emit();
            process::exit(1);
        }
    };

    let name = &manifest.project.name;
    let version = &manifest.project.version;

    println!();
    println!("  Publishing {} v{} to the VRE Registry...", name, version);
    println!();

    // Confirmation prompt unless --yes
    if !args.yes {
        print!("  Proceed? [y/N] ");
        use std::io::{self, BufRead};
        let _ = io::stdout().flush();
        let mut line = String::new();
        let stdin = io::stdin();
        stdin.lock().read_line(&mut line).ok();
        let answer = line.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            println!("  Aborted.");
            println!();
            return;
        }
        println!();
    }

    let client = match &args.registry {
        Some(url) => RegistryClient::with_url(url),
        None => RegistryClient::new(),
    };

    // TODO: read .vpkg if it exists, otherwise package on the fly
    let pkg_data: &[u8] = &[];

    match client.publish(name, version, pkg_data) {
        Ok(()) => {
            println!("  ✓ Published {} v{}", name, version);
            println!();
        }
        Err(e) => {
            Diagnostic::error(codes::E004, e).emit();
            process::exit(1);
        }
    }
}

// io::Write for flush()
use std::io::Write;
