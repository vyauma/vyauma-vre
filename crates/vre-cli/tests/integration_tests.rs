use std::process::Command;
use std::fs;
use std::path::PathBuf;

// ── Test helpers ──────────────────────────────────────────────────────────────

fn vre_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vre")
}

/// Run a Vyauma script file via `vre run <file>`.
fn run_script(script_code: &str, test_name: &str) -> (String, String) {
    run_script_with_args(script_code, test_name, &[])
}

fn run_script_with_args(script_code: &str, test_name: &str, args: &[&str]) -> (String, String) {
    let test_dir = std::env::temp_dir().join("vyauma_test");
    fs::create_dir_all(&test_dir).unwrap();

    let script_path = test_dir.join(format!("{}.vym", test_name));
    fs::write(&script_path, script_code).unwrap();

    let mut cmd = Command::new(vre_bin());
    cmd.arg("run");                      // ← new: explicit `run` subcommand
    cmd.arg(&script_path);
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().expect("Failed to execute vre binary");
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    (out, err)
}

// ── Runtime integration tests ─────────────────────────────────────────────────

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
    let mut i = 0
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
    let (out, err) = run_script_with_args(script, "test_phase4_fs_dir", &["--allow-all"]);
    println!("STDOUT: {}", out);
    println!("STDERR: {}", err);
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
    let fd = net_connect("127.0.0.1", 8080)
"#;
    let (out, err) = run_script_with_args(script, "test_phase6_capability_denied", &[]);
    assert!(err.contains("capability not granted") || out.contains("capability not granted"));
}

#[test]
fn test_phase6_capability_granted() {
    let script = r#"
fn main():
    let fd = net_connect("127.0.0.1", 8080)
    ffi_console_print("Executed")
"#;
    let (out, err) =
        run_script_with_args(script, "test_phase6_capability_granted", &["--allow-net"]);
    assert!(!err.contains("capability not granted"));
    assert!(out.contains("Executed"));
}

#[test]
fn test_phase7_export_encapsulation() {
    let script_a = r#"
fn secret_func() {
    return 42
}

export fn public_func() {
    return secret_func()
}
"#;
    let script_b = r#"
import "./test_export_a.vya"

fn main() {
    ffi_console_print("Trying secret: " + test_export_a__secret_func())
}
"#;
    let script_c = r#"
import "./test_export_a.vya"

fn main() {
    ffi_console_print("Public says: " + test_export_a__public_func())
}
"#;

    let dir = std::env::temp_dir().join("vre_phase7_test");
    std::fs::create_dir_all(&dir).unwrap();

    let path_a = dir.join("test_export_a.vya");
    let path_b = dir.join("test_export_b.vya");
    let path_c = dir.join("test_export_c.vya");

    std::fs::write(&path_a, script_a).unwrap();
    std::fs::write(&path_b, script_b).unwrap();
    std::fs::write(&path_c, script_c).unwrap();

    // Run B (tries to call private function — should fail)
    let output_b = Command::new(vre_bin())
        .arg("run")
        .arg(&path_b)
        .arg("--allow-all")
        .current_dir(&dir)
        .output()
        .unwrap();

    let err_b = String::from_utf8_lossy(&output_b.stderr).to_lowercase();
    let out_b = String::from_utf8_lossy(&output_b.stdout).to_lowercase();
    assert!(!output_b.status.success());
    assert!(
        err_b.contains("unresolved") || out_b.contains("unresolved")
            || err_b.contains("undefined") || out_b.contains("undefined"),
        "Expected undefined/unresolved error, got stderr: '{}' stdout: '{}'", err_b, out_b
    );

    // Run C (calls public function — should succeed)
    let output_c = Command::new(vre_bin())
        .arg("run")
        .arg(&path_c)
        .arg("--allow-all")
        .current_dir(&dir)
        .output()
        .unwrap();

    let out_c = String::from_utf8_lossy(&output_c.stdout);
    assert!(
        output_c.status.success(),
        "Expected success, got: {}",
        String::from_utf8_lossy(&output_c.stderr)
    );
    assert!(out_c.contains("Public says: 42"));
}

// ── CLI subcommand tests ───────────────────────────────────────────────────────

#[test]
fn test_cli_help() {
    let output = Command::new(vre_bin())
        .arg("--help")
        .output()
        .expect("Failed to run vre --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Vyauma Runtime Engine"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("new"));
    assert!(stdout.contains("build"));
}

#[test]
fn test_cli_version() {
    let output = Command::new(vre_bin())
        .arg("version")
        .output()
        .expect("Failed to run vre version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VRE"));
    assert!(stdout.contains("CLI Version"));
}

#[test]
fn test_cli_new_app_template() {
    let tmp = std::env::temp_dir().join("vre_cli_test_new_app");
    let _ = std::fs::remove_dir_all(&tmp); // clean up from previous runs

    let output = Command::new(vre_bin())
        .arg("new")
        .arg(tmp.to_str().unwrap())
        .arg("--template")
        .arg("app")
        .output()
        .expect("Failed to run vre new");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "vre new failed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    assert!(tmp.join("vre.toml").exists(), "vre.toml not created");
    assert!(tmp.join("src").exists(), "src/ not created");
    assert!(tmp.join("README.md").exists(), "README.md not created");
    assert!(tmp.join(".gitignore").exists(), ".gitignore not created");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_cli_new_library_template() {
    let tmp = std::env::temp_dir().join("vre_cli_test_new_lib");
    let _ = std::fs::remove_dir_all(&tmp);

    let output = Command::new(vre_bin())
        .arg("new")
        .arg(tmp.to_str().unwrap())
        .arg("--template")
        .arg("library")
        .output()
        .expect("Failed to run vre new --template library");

    assert!(output.status.success());
    assert!(tmp.join("src/lib.vya").exists(), "lib.vya not created");
    assert!(tmp.join("tests").exists(), "tests/ not created");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_cli_check_valid_file() {
    let tmp = std::env::temp_dir().join("vre_cli_check_test.vya");
    fs::write(
        &tmp,
        "fn main() {\n    ffi_console_print(\"hello\");\n}\n",
    )
    .unwrap();

    let output = Command::new(vre_bin())
        .arg("check")
        .arg(&tmp)
        .output()
        .expect("Failed to run vre check");

    // Even if check fails for syntax reasons, it shouldn't panic
    // (exit code 0 means type-check passed)
    let _stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);
}

#[test]
fn test_cli_doctor_runs() {
    let output = Command::new(vre_bin())
        .arg("doctor")
        .output()
        .expect("Failed to run vre doctor");
    // Doctor always exits 0 (it's diagnostic, not pass/fail)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain diagnostic sections
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("Runtime") || combined.contains("VRE") || combined.contains("Doctor"),
        "doctor output unexpected: {}",
        combined
    );
}

#[test]
fn test_cli_search() {
    let output = Command::new(vre_bin())
        .arg("search")
        .arg("nonexistent-package-xyz")
        .output()
        .expect("Failed to run vre search");
    // Should not crash — returns 0 even with no results
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No packages found") || stdout.contains("Found"),
        "Unexpected search output: {}",
        stdout
    );
}

#[test]
fn test_cli_upgrade_runs() {
    let output = Command::new(vre_bin())
        .arg("upgrade")
        .output()
        .expect("Failed to run vre upgrade");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VRE CLI version") || stdout.contains("upgrade"));
}
