use std::process::Command;
use std::fs;
use std::path::PathBuf;

fn run_script(script_code: &str, test_name: &str) -> (String, String) {
    run_script_with_args(script_code, test_name, &[])
}

fn run_script_with_args(script_code: &str, test_name: &str, args: &[&str]) -> (String, String) {
    let test_dir = std::env::temp_dir().join("vyauma_test");
    fs::create_dir_all(&test_dir).unwrap();
    
    let script_path = test_dir.join(format!("{}.vym", test_name));
    fs::write(&script_path, script_code).unwrap();
    
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vre"));
    cmd.arg(script_path);
    for arg in args {
        cmd.arg(arg);
    }
    
    let output = cmd.output().expect("Failed to execute vre binary");
        
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
fn test_string_concat() {
    // String + Number should produce string concatenation, not a type error
    let script = r#"
fn main():
    let x = "value: "
    let y = x + 42
    ffi_console_print(y)
"#;
    let (out, err) = run_script(script, "test_string_concat");
    if !err.is_empty() && out.is_empty() {
        panic!("Unexpected error: {}", err);
    }
    assert!(out.contains("value: 42"), "Expected 'value: 42', got: {}", out);
}

#[test]
fn test_phase4_regex() {
    let script = r#"
fn main():
    let is_match = ffi_regex_is_match("^[a-z]+$", "hello")
    let replace = ffi_regex_replace("world", "hello world", "vyauma")
    ffi_console_print(is_match)
    ffi_console_print("\n")
    ffi_console_print(replace)
"#;
    let (out, _err) = run_script(script, "test_phase4_regex");
    assert!(out.contains("true\nhello vyauma"));
}

#[test]
fn test_phase4_uuid() {
    let script = r#"
fn main():
    let u = ffi_uuid_v4()
    let len = ffi_string_len(u)
    ffi_console_print(len)
"#;
    let (out, _err) = run_script(script, "test_phase4_uuid");
    assert!(out.contains("36"), "OUT was '{}', ERR was '{}'", out, _err);
}

#[test]
fn test_phase4_yaml() {
    let script = r#"
fn main():
    let y = ffi_yaml_parse("name: vyauma")
    let s = ffi_yaml_stringify(y)
    ffi_console_print(s)
"#;
    let (out, _err) = run_script(script, "test_phase4_yaml");
    assert!(out.contains("name: vyauma"));
}

#[test]
fn test_phase4_fs_dir() {
    let script = r#"
fn main():
    ffi_fs_create_dir("test_dir_phase4")
    let is_d = ffi_fs_is_dir("test_dir_phase4")
    ffi_console_print(is_d)
"#;
    let (out, _err) = run_script(script, "test_phase4_fs_dir");
    assert!(out.contains("true"));
    let _ = std::fs::remove_dir("test_dir_phase4");
}

#[test]
fn test_phase5_async_await() {
    let script = r#"
fn worker():
    sleep_async(50)
    ffi_console_print("Worker done\n")
    return 42

fn main():
    let task_id = spawn(worker)
    ffi_console_print("Spawned task\n")
    let res = await(task_id)
    ffi_console_print("Result: ")
    ffi_console_print(res)
    ffi_console_print("\n")
"#;
    let (out, err) = run_script(script, "test_phase5_async_await");
    if !err.is_empty() {
        panic!("Error: {}", err);
    }
    assert!(out.contains("Spawned task\n"));
    assert!(out.contains("Worker done\n"));
    assert!(out.contains("Result: 42\n"));
}

#[test]
fn test_phase6_capability_denied() {
    let script = r#"
fn main():
    // Creating a directory needs fs.write in our implementation, wait, no, fs_create_dir is fs.write maybe?
    // Let's test reading a file or opening a file.
    // We didn't map all PAL to capabilities perfectly, but we mapped `file_open` (fs.read) and `net_connect` (net.connect).
    // Let's do `net_connect`.
    let fd = net_connect("127.0.0.1", 8080)
"#;
    let (out, err) = run_script_with_args(script, "test_phase6_capability_denied", &[]);
    assert!(err.contains("capability not granted") || out.contains("capability not granted"));
}

#[test]
fn test_phase6_capability_granted() {
    let script = r#"
fn main():
    // The connection might fail since nothing is listening, but the capability should be granted, so we won't get CapabilityNotGranted.
    let fd = net_connect("127.0.0.1", 8080)
    ffi_console_print("Executed")
"#;
    let (out, err) = run_script_with_args(script, "test_phase6_capability_granted", &["--allow-net"]);
    assert!(!err.contains("capability not granted"));
    assert!(out.contains("Executed"));
}
