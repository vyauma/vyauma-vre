pub mod rpi;
pub mod esp32;

pub fn flash(input_path: &str, target: &str) {
    println!("Generating embedded flash scaffold for {} targeting {}...", input_path, target);
    match target {
        "rpi" => rpi::flash_rpi(input_path),
        "esp32" => esp32::flash_esp32(input_path),
        _ => {
            println!("Unknown embedded target '{}'. Use 'rpi' or 'esp32'.", target);
        }
    }
}
