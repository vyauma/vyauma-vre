//! `vre package` — Package the project into a `.vpkg` archive.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process;

use crate::cli::PackageArgs;
use crate::config::VreToml;
use crate::diagnostics::{codes, Diagnostic};

pub fn run(args: PackageArgs) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());

    let (manifest, manifest_path) = match VreToml::find_and_load(&cwd) {
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

    // Output path: explicit flag > `<name>-<version>.vpkg`
    let output_path = args.output.unwrap_or_else(|| {
        format!("{}-{}.vpkg", name, version)
    });

    println!();
    println!("  Packaging {} v{}...", name, version);
    println!();

    let project_root = manifest_path.parent().unwrap_or(Path::new("."));

    // Create .vpkg (currently a JSON manifest + placeholder binary bundle)
    // A full implementation would use zip/tar to bundle bytecode + assets.
    let pkg_manifest = serde_json::json!({
        "name": name,
        "version": version,
        "authors": manifest.project.authors,
        "description": manifest.project.description,
        "dependencies": manifest.dependencies,
        "capabilities": {
            "filesystem": manifest.capabilities.filesystem,
            "network": manifest.capabilities.network,
        },
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let mut pkg_content = Vec::new();

    // Header magic: VRE Package Format
    pkg_content.extend_from_slice(b"VPKG");            // magic
    pkg_content.extend_from_slice(&[0x01, 0x00]);      // version 1.0
    let manifest_bytes = pkg_manifest.to_string().into_bytes();
    let manifest_len = manifest_bytes.len() as u32;
    pkg_content.extend_from_slice(&manifest_len.to_le_bytes());
    pkg_content.extend_from_slice(&manifest_bytes);

    match fs::write(&output_path, &pkg_content) {
        Ok(()) => {
            let size_kb = pkg_content.len() as f64 / 1024.0;
            println!("  ✓ Created {}", output_path);
            println!("  ✓ Size: {:.1} KB", size_kb);
            println!("  ✓ Package: {} v{}", name, version);
            println!();
            println!("  To publish this package:");
            println!("    vre publish");
            println!();
        }
        Err(e) => {
            Diagnostic::error(codes::E010, format!("Failed to write {}: {}", output_path, e)).emit();
            process::exit(1);
        }
    }
}
