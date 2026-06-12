#!/bin/bash
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
