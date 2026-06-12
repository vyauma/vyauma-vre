use esp_idf_sys as _; // If using the `bin` feature of `esp-idf-sys`, always keep this module imported
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
