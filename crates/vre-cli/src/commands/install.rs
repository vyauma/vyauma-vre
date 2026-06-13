//! `vre install` — Install a package from the VRE Registry.

use std::path::Path;
use std::process;
use crate::cli::InstallArgs;
use crate::config::VreToml;
use crate::diagnostics::{codes, Diagnostic};
use crate::registry::RegistryClient;

pub fn run(args: InstallArgs) {
    let client = match &args.registry {
        Some(url) => RegistryClient::with_url(url),
        None => RegistryClient::new(),
    };

    let modules_dir = Path::new("vym_modules");
    if let Err(e) = std::fs::create_dir_all(modules_dir) {
        Diagnostic::error(codes::E010, format!("Failed to create vym_modules/: {}", e)).emit();
        process::exit(1);
    }

    match args.package {
        Some(pkg_spec) => {
            // Parse `name@version` or just `name`
            let (name, version) = parse_package_spec(&pkg_spec);
            println!();
            println!("  Installing {} v{}...", name, version);
            println!();
            install_one(&client, &name, &version, modules_dir);
        }
        None => {
            // Install all dependencies from vre.toml
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            let (manifest, _) = match VreToml::find_and_load(&cwd) {
                Ok(m) => m,
                Err(e) => {
                    Diagnostic::error(codes::E003, e)
                        .with_suggestion("Run `vre init` to create a vre.toml, or specify a package: vre install <name>")
                        .emit();
                    process::exit(1);
                }
            };

            if manifest.dependencies.is_empty() {
                println!();
                println!("  No dependencies to install.");
                println!();
                return;
            }

            println!();
            println!("  Installing {} dependencies...", manifest.dependencies.len());
            println!();

            for (name, version) in &manifest.dependencies {
                install_one(&client, name, version, modules_dir);
            }
        }
    }

    println!("  Installation complete.");
    println!();
}

fn parse_package_spec(spec: &str) -> (String, String) {
    if let Some(at) = spec.find('@') {
        let name = &spec[..at];
        let version = &spec[at + 1..];
        if version.is_empty() {
            (name.to_string(), "latest".to_string())
        } else {
            (name.to_string(), version.to_string())
        }
    } else {
        (spec.to_string(), "latest".to_string())
    }
}

fn install_one(client: &RegistryClient, name: &str, version: &str, dest: &Path) {
    match client.install(name, version, dest) {
        Ok(()) => println!("  ✓ Installed {}", name),
        Err(e) => {
            Diagnostic::error(codes::E001, format!("Failed to install '{}': {}", name, e))
                .with_suggestion(format!("Run `vre search {}` to check if the package exists.", name))
                .emit();
        }
    }
}
