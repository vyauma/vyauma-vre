use std::fs;
use std::path::Path;

pub fn flash_esp32(input_path: &str) {
    let build_dir = Path::new("build/esp32");
    if let Err(e) = fs::create_dir_all(build_dir) {
        println!("Failed to create esp32 build directory: {}", e);
        return;
    }

    // Copy the VYM file
    // For ESP32, bytecode is usually embedded into the firmware image via `include_bytes!`
    let dest_file = build_dir.join("app.vym");
    if let Err(e) = fs::copy(input_path, &dest_file) {
        println!("Could not copy input file to build folder: {}", e);
    }

    // Generate Cargo.toml for esp-idf project
    let cargo_path = build_dir.join("Cargo.toml");
    let cargo_content = r#"[package]
name = "vre-esp32-app"
version = "0.1.0"
edition = "2021"

[dependencies]
esp-idf-sys = { version = "0.33", features = ["bin"] }
esp-idf-hal = "0.41"
esp-idf-svc = "0.46"

# We would also import a `no_std` compatible or minimal `vre-core` here
# vre-core = { path = "../../crates/vre-core", default-features = false }

[build-dependencies]
embuild = "0.31.3"
"#;
    fs::write(cargo_path, cargo_content).unwrap();

    // Generate main.rs
    let src_dir = build_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let main_path = src_dir.join("main.rs");
    let main_content = r#"use esp_idf_sys as _; // If using the `bin` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_hal::prelude::*;

// Embed VRE bytecode directly into flash
static VYM_BYTECODE: &[u8] = include_bytes!("../app.vym");

fn main() {
    // Temporary. Will add this to ESP32 init:
    // esp_idf_sys::link_patches();

    println!("Starting Vyauma Runtime Engine on ESP32!");

    // Boot VRE with VYM_BYTECODE
    // let mut vm = VirtualMachine::new();
    // vm.execute(VYM_BYTECODE);
}
"#;
    fs::write(main_path, main_content).unwrap();

    println!("ESP32 embedded scaffold generated at build/esp32/");
    println!("To flash: 'cd build/esp32 && cargo espflash flash'");
}
