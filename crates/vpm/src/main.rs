//! Vyauma Package Manager (vpm)
//!
//! Commands:
//!   vpm init      — Create a new vyauma.toml in the current directory
//!   vpm install   — Install all dependencies from vyauma.toml
//!   vpm add <pkg> — Add a dependency and install it
//!   vpm update    — Re-resolve and update all dependencies
//!   vpm publish   — Publish the current package to the registry
//!   vpm info <pkg>— Show package information from the registry

mod manifest;
mod lockfile;
mod registry;
mod resolver;
mod install;

use std::env;
use manifest::{Manifest, PackageMeta};
use lockfile::LockFile;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let result = match args[1].as_str() {
        "init"    => cmd_init(),
        "install" => cmd_install(),
        "add"     => cmd_add(&args[2..]),
        "update"  => cmd_update(),
        "publish" => cmd_publish(&args[2..]),
        "info"    => cmd_info(&args[2..]),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn print_usage() {
    println!("Vyauma Package Manager (vpm) v0.2.0");
    println!();
    println!("USAGE:");
    println!("  vpm <command> [options]");
    println!();
    println!("COMMANDS:");
    println!("  init               Initialize a new Vyauma package in this directory");
    println!("  install            Install all dependencies from vyauma.toml");
    println!("  add <package>      Add a dependency and install it");
    println!("  update             Update all dependencies to their latest compatible versions");
    println!("  publish [--token]  Publish this package to the Vyauma registry");
    println!("  info <package>     Show information about a package from the registry");
    println!();
    println!("ENVIRONMENT:");
    println!("  VYAUMA_TOKEN       Registry auth token (used by `vpm publish`)");
    println!("  VRE_MODULES_PATH   Override the vym_modules/ directory path");
    println!("  VRE_STD_PATH       Override the std library directory path");
}

// ── Commands ─────────────────────────────────────────────────────────────────

fn cmd_init() -> Result<(), String> {
    let path = "vyauma.toml";
    if std::path::Path::new(path).exists() {
        return Err("vyauma.toml already exists in this directory.".to_string());
    }

    let cwd_name = env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "my_package".to_string());

    let manifest = Manifest {
        package: PackageMeta {
            name: cwd_name.clone(),
            version: "0.1.0".to_string(),
            description: Some(format!("A Vyauma package")),
            authors: vec![],
            entry: Some("src/main.vya".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    manifest.save(path)?;

    // Create src/ directory and a starter main.vya
    std::fs::create_dir_all("src")
        .map_err(|e| format!("Failed to create src/: {}", e))?;

    let starter = r#"// Vyauma Language - Entry Point
fn main() {
    print("Welcome to Vyauma!");
}
"#;

    if !std::path::Path::new("src/main.vya").exists() {
        std::fs::write("src/main.vya", starter)
            .map_err(|e| format!("Failed to write src/main.vya: {}", e))?;
    }

    println!("Initialized Vyauma package '{}'.", cwd_name);
    println!("  Run:  vre src/main.vya");
    Ok(())
}

fn cmd_install() -> Result<(), String> {
    if !std::path::Path::new("vyauma.toml").exists() {
        return Err("vyauma.toml not found. Run `vpm init` first.".to_string());
    }
    install::install("vyauma.toml")
}

fn cmd_add(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: vpm add <package_name> [version_req]".to_string());
    }

    let pkg_name = &args[0];
    let version_req = args.get(1).map(|s| s.as_str()).unwrap_or("*");

    let mut manifest = Manifest::load("vyauma.toml")?;
    manifest.dependencies.insert(pkg_name.clone(), version_req.to_string());
    manifest.save("vyauma.toml")?;

    println!("Added '{}' @ '{}' to vyauma.toml.", pkg_name, version_req);
    install::install("vyauma.toml")
}

fn cmd_update() -> Result<(), String> {
    println!("Checking for updates...");
    // Re-run install (resolver will re-fetch latest compatible versions)
    install::install("vyauma.toml")
}

fn cmd_publish(args: &[String]) -> Result<(), String> {
    let manifest = Manifest::load("vyauma.toml")?;

    // Auth token: --token flag > VYAUMA_TOKEN env var
    let token = args.windows(2).find(|w| w[0] == "--token")
        .map(|w| w[1].clone())
        .or_else(|| env::var("VYAUMA_TOKEN").ok())
        .ok_or_else(|| {
            "No auth token provided. Pass --token <token> or set VYAUMA_TOKEN env var.".to_string()
        })?;

    println!("Publishing {} v{}...", manifest.package.name, manifest.package.version);

    // Build a minimal tarball of the src/ directory
    let tarball = build_tarball("src")?;

    let manifest_json = serde_json::to_string(&manifest.package)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    registry::publish_package(&manifest_json, &tarball, &token)
        .map(|_| println!("Successfully published {} v{}!", manifest.package.name, manifest.package.version))
        .unwrap_or_else(|e| {
            eprintln!("Registry error: {}", e);
            eprintln!("(Package prepared locally; retry when registry is online.)");
        });

    Ok(())
}

fn cmd_info(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: vpm info <package_name>".to_string());
    }
    let name = &args[0];
    println!("Fetching info for '{}'...", name);

    match registry::fetch_versions(name) {
        Ok(versions) => {
            println!("Package: {}", name);
            println!("Available versions:");
            for v in &versions {
                println!("  {}", v);
            }
        }
        Err(e) => {
            eprintln!("Could not reach registry: {}", e);
            // Fall back to local lockfile
            let lock = LockFile::load();
            if let Some(pkg) = lock.find(name) {
                println!("(From local lockfile)");
                println!("  {}@{}", pkg.name, pkg.version);
                println!("  Registry: {}", pkg.registry);
            } else {
                return Err(format!("Package '{}' not found locally or in registry.", name));
            }
        }
    }
    Ok(())
}

/// Build a minimal `.tar.gz` from a source directory.
fn build_tarball(src_dir: &str) -> Result<Vec<u8>, String> {
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    let mut buf = Vec::new();
    {
        let enc = GzEncoder::new(&mut buf, Compression::best());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(".", src_dir)
            .map_err(|e| format!("Failed to build tarball: {}", e))?;
        tar.into_inner()
            .map_err(|e| format!("Failed to finalize tar: {}", e))?
            .finish()
            .map_err(|e| format!("Failed to finalize gzip: {}", e))?;
    }
    Ok(buf)
}
