//! `vre search` — Search the VRE Registry for packages.

use crate::cli::SearchArgs;
use crate::registry::RegistryClient;

pub fn run(args: SearchArgs) {
    println!();
    println!("  Searching for '{}'...", args.query);
    println!();

    let client = match &args.registry {
        Some(url) => RegistryClient::with_url(url),
        None => RegistryClient::new(),
    };

    let results = client.search(&args.query, args.limit);

    if results.is_empty() {
        println!("  No packages found for '{}'.", args.query);
        println!();
        println!("  Suggestions:");
        println!("    • Check the spelling of the package name");
        println!("    • Try a broader search term");
        println!("    • Browse packages at https://registry.vyauma.org");
        println!();
    } else {
        println!("  Found {} package(s):", results.len());
        println!();
        println!("  {:<30} {:<12} {}", "NAME", "VERSION", "DESCRIPTION");
        println!("  {}", "-".repeat(70));
        for pkg in &results {
            println!(
                "  {:<30} {:<12} {}",
                pkg.name, pkg.version, pkg.description
            );
        }
        println!();
        println!("  Install a package:");
        println!("    vre install <package-name>");
        println!();
    }
}
