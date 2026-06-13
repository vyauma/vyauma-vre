//! `vre version` — Print version information for all VRE components.

pub fn run() {
    println!();
    println!("  VRE (Vyauma Runtime Engine)");
    println!();
    println!("  CLI Version:      {}", env!("CARGO_PKG_VERSION"));
    println!("  Runtime Version:  {}", vre_core_version());
    println!("  Compiler Version: {}", vre_compiler_version());
    println!("  VIR Version:      1.0 (SSA, CFG, Optimization Passes)");
    println!();
    println!("  Build Target:     {}", build_target());
    println!("  Build Profile:    {}", build_profile());
    println!();
}

fn vre_core_version() -> &'static str {
    // Mirrors the version from vre-core's Cargo.toml
    "0.2.0"
}

fn vre_compiler_version() -> &'static str {
    "0.2.0"
}

fn build_target() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "x86_64-pc-windows-msvc";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "x86_64-unknown-linux-gnu";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "aarch64-apple-darwin";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "x86_64-apple-darwin";
    #[allow(unreachable_code)]
    "unknown"
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) { "debug" } else { "release" }
}
