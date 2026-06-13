//! `vre doctor` — Diagnose the VRE installation and environment.
//!
//! Performs a comprehensive check of:
//! - VRE runtime and compiler binaries
//! - Registry connectivity
//! - Mobile SDKs (Android/iOS)
//! - Native toolchains (Clang, GCC, MSVC)
//! - Environment variables
//! - Platform support

use std::process::Command;
use crate::diagnostics::{check_pass, check_fail, check_warn, section};
use crate::registry::RegistryClient;

pub fn run() {
    println!();
    println!("  VRE Doctor — Environment Diagnostics");
    println!("  ─────────────────────────────────────");

    let mut issues = 0usize;
    let mut warnings = 0usize;

    // ── Runtime ───────────────────────────────────────────────────────────────
    section("Runtime");

    // VRE CLI itself is running (trivially true here)
    check_pass(&format!("VRE CLI v{}", env!("CARGO_PKG_VERSION")));

    // Check vre binary on PATH
    if which("vre") {
        check_pass("vre binary found on PATH");
    } else {
        check_warn("vre binary not on PATH",
            "Add the VRE bin directory to your PATH environment variable.");
        warnings += 1;
    }

    // ── Compiler ──────────────────────────────────────────────────────────────
    section("Compiler");

    // The VRE compiler is embedded — always available when CLI is installed
    check_pass("VRE compiler (embedded) — available");
    check_pass("VIR (Vyauma Intermediate Representation) — available");

    // ── Project ───────────────────────────────────────────────────────────────
    section("Project");

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    match crate::config::VreToml::find_and_load(&cwd) {
        Ok((manifest, path)) => {
            check_pass(&format!(
                "vre.toml found: {} v{}",
                manifest.project.name, manifest.project.version
            ));
            check_pass(&format!("Config: {}", path.display()));
        }
        Err(_) => {
            check_warn("No vre.toml found in this directory",
                "Run `vre init` to create a project configuration.");
            warnings += 1;
        }
    }

    // ── Registry ──────────────────────────────────────────────────────────────
    section("Registry");

    let client = RegistryClient::new();
    if client.ping() {
        check_pass("VRE Registry — reachable");
    } else {
        check_warn("VRE Registry — unreachable (registry.vyauma.org)",
            "Check your internet connection. Registry features will work in offline mode.");
        warnings += 1;
    }

    // ── Native Toolchains ─────────────────────────────────────────────────────
    section("Native Toolchains");

    #[cfg(target_os = "windows")]
    {
        if which("cl") || std::env::var("VCINSTALLDIR").is_ok() {
            check_pass("MSVC (Visual C++) — found");
        } else {
            check_warn("MSVC — not found",
                "Install Visual Studio Build Tools for native compilation.");
            warnings += 1;
        }
        if which("clang") {
            check_pass("Clang — found");
        } else {
            check_warn("Clang — not found (optional)", "");
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        if which("cc") || which("gcc") || which("clang") {
            check_pass("C compiler (cc/gcc/clang) — found");
        } else {
            check_fail("C compiler — not found",
                Some("Install gcc or clang for native compilation."));
            issues += 1;
        }
    }

    // Rust/Cargo (needed for rebuilding VRE from source)
    if which("cargo") {
        check_pass("Cargo (Rust toolchain) — found");
    } else {
        check_warn("Cargo — not found",
            "Rust is only needed to rebuild VRE from source.");
        warnings += 1;
    }

    // ── Mobile SDKs ───────────────────────────────────────────────────────────
    section("Mobile SDKs");

    let android_home = std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
        .ok();

    if let Some(ref path) = android_home {
        if std::path::Path::new(path).exists() {
            check_pass(&format!("Android SDK — found ({})", path));

            // Check for NDK
            let ndk = std::env::var("ANDROID_NDK_HOME")
                .or_else(|_| std::env::var("ANDROID_NDK_ROOT"))
                .ok();
            if let Some(ndk_path) = ndk {
                check_pass(&format!("Android NDK — found ({})", ndk_path));
            } else {
                check_warn("Android NDK — not found",
                    "Set ANDROID_NDK_HOME for mobile build support.");
                warnings += 1;
            }

            // adb
            if which("adb") {
                check_pass("ADB (Android Debug Bridge) — found");
            } else {
                check_warn("ADB — not found",
                    "Add platform-tools to your PATH.");
                warnings += 1;
            }
        } else {
            check_fail("Android SDK — path set but directory not found",
                Some(&format!("Check ANDROID_HOME: {}", path)));
            issues += 1;
        }
    } else {
        check_fail("Android SDK — not found (ANDROID_HOME not set)",
            Some("Install Android Studio or the Android command-line tools.\nRun `vre mobile build android` requires this."));
        issues += 1;
    }

    #[cfg(target_os = "macos")]
    {
        if which("xcodebuild") {
            check_pass("Xcode — found");
            // Check for iOS SDK
            let output = Command::new("xcodebuild")
                .arg("-showsdks")
                .output();
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains("iphoneos") {
                    check_pass("iOS SDK — found");
                } else {
                    check_warn("iOS SDK — not found",
                        "Open Xcode and install iOS platform support.");
                    warnings += 1;
                }
            }
        } else {
            check_fail("Xcode — not found",
                Some("Install Xcode from the Mac App Store for iOS build support."));
            issues += 1;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        check_warn("iOS SDK — unavailable", "iOS builds require macOS with Xcode.");
    }

    // ── Environment Variables ─────────────────────────────────────────────────
    section("Environment");

    check_env("PATH");
    check_opt_env("VRE_HOME", "Optional: custom VRE installation directory");
    check_opt_env("VRE_REGISTRY", "Optional: custom registry URL");
    check_opt_env("NO_COLOR", "Optional: disable ANSI color output");

    #[cfg(target_os = "windows")]
    check_opt_env("VCINSTALLDIR", "Optional: MSVC installation path");

    // ── Platform Support ──────────────────────────────────────────────────────
    section("Platform");

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    check_pass(&format!("Operating system: {}", os));
    check_pass(&format!("Architecture: {}", arch));

    let supported = matches!(
        (os, arch),
        ("windows", "x86_64")
        | ("linux", "x86_64")
        | ("macos", "x86_64")
        | ("macos", "aarch64")
    );
    if supported {
        check_pass("Platform — supported");
    } else {
        check_warn(
            &format!("Platform {}/{} — experimental", os, arch),
            "This platform may not be fully supported. Please report any issues."
        );
        warnings += 1;
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    println!();
    println!("  ─────────────────────────────────────");

    if issues == 0 && warnings == 0 {
        println!("  ✓ All checks passed. Your VRE environment is ready!");
    } else if issues == 0 {
        println!("  ⚠ {} warning(s) found. VRE will work but some features may be limited.", warnings);
    } else {
        println!("  ✗ {} issue(s) and {} warning(s) found.", issues, warnings);
        println!("  Some features may not work until issues are resolved.");
    }
    println!();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Check if a binary exists on PATH.
fn which(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    let cmd = Command::new("where").arg(name).output();
    #[cfg(not(target_os = "windows"))]
    let cmd = Command::new("which").arg(name).output();

    cmd.map(|o| o.status.success()).unwrap_or(false)
}

fn check_env(name: &str) {
    match std::env::var(name) {
        Ok(val) if !val.is_empty() => check_pass(&format!("{} — set", name)),
        _ => check_fail(&format!("{} — not set", name), Some(&format!("Set the {} environment variable.", name))),
    }
}

fn check_opt_env(name: &str, description: &str) {
    match std::env::var(name) {
        Ok(val) if !val.is_empty() => check_pass(&format!("{} — set ({})", name, val)),
        _ => check_warn(&format!("{} — not set", name), description),
    }
}
