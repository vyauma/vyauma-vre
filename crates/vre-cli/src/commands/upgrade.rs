//! `vre upgrade` — Upgrade the VRE toolchain.

pub fn run() {
    println!();
    println!("  Checking for VRE updates...");
    println!();

    // TODO: query release endpoint once vyauma.org/releases is live
    // For now, display current version and point to update instructions
    println!("  Current VRE CLI version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  To upgrade VRE, re-build from source:");
    println!();
    println!("    git pull origin main");
    println!("    cargo build --release -p vre-cli");
    println!();
    println!("  Or download the latest release from:");
    println!("    https://github.com/vyauma/vre/releases");
    println!();
}
