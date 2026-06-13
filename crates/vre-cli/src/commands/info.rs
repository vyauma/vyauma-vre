//! `vre info` — Show detailed information about a registry package.

use crate::cli::InfoArgs;
use crate::registry::RegistryClient;
use crate::diagnostics::{codes, Diagnostic};

pub fn run(args: InfoArgs) {
    println!();
    println!("  Fetching info for '{}'...", args.package);
    println!();

    let client = match &args.registry {
        Some(url) => RegistryClient::with_url(url),
        None => RegistryClient::new(),
    };

    match client.info(&args.package) {
        Some(info) => {
            println!("  Name:         {}", info.name);
            println!("  Version:      {}", info.version);
            println!("  Description:  {}", info.description);
            if !info.authors.is_empty() {
                println!("  Authors:      {}", info.authors.join(", "));
            }
            if let Some(license) = &info.license {
                println!("  License:      {}", license);
            }
            if let Some(repo) = &info.repository {
                println!("  Repository:   {}", repo);
            }
            println!("  Downloads:    {}", info.downloads);
            println!("  Published:    {}", info.published_at);
            if !info.dependencies.is_empty() {
                println!();
                println!("  Dependencies:");
                for (dep, ver) in &info.dependencies {
                    println!("    {} = \"{}\"", dep, ver);
                }
            }
            println!();
            println!("  Install:");
            println!("    vre install {}", info.name);
            println!();
        }
        None => {
            Diagnostic::error(codes::E001, format!("Package '{}' not found.", args.package))
                .with_suggestion(format!("Run: vre search {}", args.package))
                .emit();
            std::process::exit(1);
        }
    }
}
