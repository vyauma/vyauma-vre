pub mod android;
pub mod ios;

pub fn pack(input_path: &str, target: &str) {
    println!("Packaging {} for {}...", input_path, target);
    match target {
        "android" => android::pack_android(input_path),
        "ios" => ios::pack_ios(input_path),
        _ => {
            println!("Unknown target '{}'. Use 'android' or 'ios'.", target);
        }
    }
}
