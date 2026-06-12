use std::fs;
use std::path::Path;

pub fn flash_rpi(input_path: &str) {
    let build_dir = Path::new("build/rpi");
    if let Err(e) = fs::create_dir_all(build_dir) {
        println!("Failed to create rpi build directory: {}", e);
        return;
    }

    // Copy the VYM file
    let dest_file = build_dir.join("app.vym");
    if let Err(e) = fs::copy(input_path, &dest_file) {
        println!("Could not copy input file to build folder: {}", e);
    }

    // Generate cross-compilation bash script
    let script_path = build_dir.join("build_rpi.sh");
    let script_content = r#"#!/bin/bash
# Cross-compile VRE Core for Raspberry Pi (aarch64-unknown-linux-gnu)
set -e

echo "Ensuring cross-compiler is installed..."
rustup target add aarch64-unknown-linux-gnu

echo "Building VRE Engine for Raspberry Pi..."
cargo build --release --target aarch64-unknown-linux-gnu --bin vre

echo "Deploying to Raspberry Pi..."
# scp target/aarch64-unknown-linux-gnu/release/vre pi@raspberrypi.local:/home/pi/
# scp app.vym pi@raspberrypi.local:/home/pi/

echo "Done. Run 'vre app.vym' on your Raspberry Pi!"
"#;
    fs::write(&script_path, script_content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) = fs::metadata(&script_path).map(|m| m.permissions()) {
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&script_path, perms);
        }
    }

    println!("Raspberry Pi embedded scaffold generated at build/rpi/");
}
