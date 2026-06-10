use std::process::Command;
use std::fs;
use std::path::PathBuf;

fn run_script(script_code: &str, test_name: &str) -> (String, String) {
    let test_dir = std::env::temp_dir().join("vyauma_test");
    fs::create_dir_all(&test_dir).unwrap();
    
    let script_path = test_dir.join(format!("{}.vym", test_name));
    fs::write(&script_path, script_code).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_vre"))
        .arg(script_path)
        .output()
        .expect("Failed to execute vre binary");
        
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    (out, err)
}

#[test]
fn test_basic_arithmetic() {
    let script = r#"
fn main():
    let x = 10
    let y = x + 5 * 2
    if y > 15:
        ffi_console_print("y is greater than 15\n")
    else:
        ffi_console_print("y is less than or equal to 15\n")
"#;
    let (out, err) = run_script(script, "test_basic_arithmetic");
    if out.is_empty() && !err.is_empty() {
        panic!("CLI Error: {}", err);
    }
    assert_eq!(out, "y is greater than 15\n");
}

#[test]
fn test_while_loop() {
    let script = r#"
fn main():
    let i = 0
    while i < 3:
        ffi_console_print("loop\n")
        i = i + 1
"#;
    let (out, err) = run_script(script, "test_while_loop");
    if out.is_empty() && !err.is_empty() {
        panic!("CLI Error: {}", err);
    }
    assert_eq!(out, "loop\nloop\nloop\n");
}

#[test]
fn test_type_error() {
    let script = r#"
fn main():
    let x = "string"
    let y = x + 5
"#;
    let (out, err) = run_script(script, "test_type_error");
    let combined = format!("{}{}", out, err);
    if !combined.contains("Type mismatch") {
        panic!("Did not find expected error string. Combined output was:\n{}", combined);
    }
}
