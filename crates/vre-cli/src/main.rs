//! Vyauma Runtime Engine - CLI
//!
//! Minimal command-line interface to execute Vyauma bytecode.

use std::env;
use std::process;

use vre_core::config::VreConfig;
use vre_core::loader::loader::BytecodeLoader;
use vre_core::vm::vm::VirtualMachine;
use vre_core::{Capability, CapabilityRegistry};

mod native;
mod mobile;
mod cloud;
mod embedded;
mod init;
mod profiler;
mod doc;
mod registry;
mod web;
mod lsp;
mod dap;
mod test_runner;
mod module_loader;
mod manifest;

fn main() {
    let raw_args: Vec<String> = env::args().collect();

    // Parse flags before the file path
    let check_leaks = raw_args.iter().any(|a| a == "--check-leaks");
    let allow_read = raw_args.iter().any(|a| a == "--allow-read");
    let allow_write = raw_args.iter().any(|a| a == "--allow-write");
    let allow_net = raw_args.iter().any(|a| a == "--allow-net");
    let allow_env = raw_args.iter().any(|a| a == "--allow-env");
    let allow_run = raw_args.iter().any(|a| a == "--allow-run");
    let allow_db = raw_args.iter().any(|a| a == "--allow-db");
    let allow_all = raw_args.iter().any(|a| a == "--allow-all");
    
    let args: Vec<&String> = raw_args.iter()
        .filter(|a| !a.starts_with("--"))
        .collect();

    if args.len() < 2 {
        print_usage(&raw_args[0]);
        process::exit(1);
    }

    let command_or_file = args[1];

    if command_or_file == "pack" {
        if args.len() < 3 {
            println!("Usage: vre pack <file> [--target android|ios]");
            process::exit(1);
        }
        let input_path = args[2];
        let mut target = "android".to_string(); // Default to android for now
        
        // rudimentary parsing for --target
        for i in 0..raw_args.len() {
            if raw_args[i] == "--target" && i + 1 < raw_args.len() {
                target = raw_args[i + 1].clone();
            }
        }
        
        mobile::pack(input_path, &target);
        return;
    }

    if command_or_file == "deploy" {
        if args.len() < 3 {
            println!("Usage: vre deploy <file> [--target docker|k8s|serverless]");
            process::exit(1);
        }
        let input_path = args[2];
        let mut target = "docker".to_string(); // Default to docker for now
        
        // rudimentary parsing for --target
        for i in 0..raw_args.len() {
            if raw_args[i] == "--target" && i + 1 < raw_args.len() {
                target = raw_args[i + 1].clone();
            }
        }
        
        cloud::deploy(input_path, &target);
        return;
    }

    if command_or_file == "flash" {
        if args.len() < 3 {
            println!("Usage: vre flash <file> [--target rpi|esp32]");
            process::exit(1);
        }
        let input_path = args[2];
        let mut target = "esp32".to_string(); // Default to esp32 for now
        
        // rudimentary parsing for --target
        for i in 0..raw_args.len() {
            if raw_args[i] == "--target" && i + 1 < raw_args.len() {
                target = raw_args[i + 1].clone();
            }
        }
        
        embedded::flash(input_path, &target);
        return;
    }

    if command_or_file == "init" {
        if args.len() < 3 {
            println!("Usage: vre init <project_name>");
            process::exit(1);
        }
        init::init_project(&args[2]);
        return;
    }

    if command_or_file == "doc" {
        if args.len() < 3 {
            println!("Usage: vre doc <project_dir>");
            process::exit(1);
        }
        doc::generate_docs(&args[2]);
        return;
    }

    if command_or_file == "publish" {
        registry::publish();
        return;
    }

    if command_or_file == "install" {
        let package_name = if args.len() >= 3 {
            Some(args[2].as_str())
        } else {
            None
        };
        registry::install(package_name);
        return;
    }

    if command_or_file == "profile" {
        if args.len() < 3 {
            println!("Usage: vre profile <file.vya>");
            process::exit(1);
        }
        profiler::run_profiler(&args[2]);
        return;
    }
    if command_or_file == "lsp" {
        lsp::run_lsp_server();
        return;
    }
    
    if command_or_file == "dap" {
        dap::run_dap_server();
        return;
    }
    
    if command_or_file == "build-web" {
        if args.len() < 3 {
            println!("Usage: vre build-web <file.vya>");
            process::exit(1);
        }
        web::build_web(&args[2]);
        return;
    }

    if command_or_file == "test" {
        if args.len() < 3 {
            println!("Usage: vre test <file_or_dir>");
            process::exit(1);
        }
        test_runner::run_tests(&args[2]);
        return;
    }

    let is_check_only = command_or_file == "check";
    let input_path = if is_check_only {
        if args.len() < 3 {
            println!("Usage: vre check <file.vya>");
            process::exit(1);
        }
        &args[2]
    } else {
        command_or_file
    };

    let (instructions, constants, native_imports, function_table) = if input_path.ends_with(".vym") || input_path.ends_with(".vya") || input_path.ends_with(".js") || input_path.ends_with(".ts") || input_path.ends_with(".php") || input_path.ends_with(".py") {
        let source = match vre_core::pal::get_pal().read_to_string(std::path::Path::new(input_path)) {
            Ok(s) => s,
            Err(e) => {
                vre_core::pal::get_pal().eprintln(&format!("Error: failed to read source file: {}", e));
                process::exit(1);
            }
        };
        let path = std::path::Path::new(input_path);
        let base_path = path.parent().unwrap_or(std::path::Path::new("."));
        match vre_compiler::compile(&source, input_path, Some(base_path)) {
            Ok(compiled) => {
                if is_check_only {
                    println!("Static Type Check passed for {}.", input_path);
                    process::exit(0);
                }
                (compiled.instructions, compiled.constants, compiled.native_imports, compiled.function_table)
            },
            Err(e) => {
                render_diagnostic(&source, input_path, &e);
                process::exit(1);
            }
        }
    } else {
        if is_check_only {
            println!("Error: 'check' command only supports source files (.vya, .vym, etc.), not bytecode.");
            process::exit(1);
        }
        let bytes = match std::fs::read(input_path) {
            Ok(b) => b,
            Err(e) => {
                vre_core::pal::get_pal().eprintln(&format!("Error: failed to read bytecode file: {}", e));
                process::exit(1);
            }
        };

        let loaded = match BytecodeLoader::load(&bytes) {
            Ok(bc) => bc,
            Err(e) => {
                vre_core::pal::get_pal().eprintln(&format!("Error: invalid bytecode: {}", e));
                process::exit(1);
            }
        };
        // Bytecode loader doesn't support native_imports yet
        (loaded.instructions, loaded.constants, Vec::new(), std::collections::HashMap::new())
    };

    // Set up Configuration and FFI
    let mut config = VreConfig::default();
    native::register_ffi(&mut config);
    vre_core::vm::api::register_apis(&mut config);

    let mut capabilities = CapabilityRegistry::new();
    
    // Console I/O is unrestricted by default
    capabilities.grant(Capability::new("io.read"));
    capabilities.grant(Capability::new("io.write"));

    if allow_all || allow_read {
        capabilities.grant(Capability::new("fs.read"));
    }
    if allow_all || allow_write {
        capabilities.grant(Capability::new("fs.write"));
    }
    if allow_all || allow_net {
        capabilities.grant(Capability::new("net.listen"));
        capabilities.grant(Capability::new("net.accept"));
        capabilities.grant(Capability::new("net.connect"));
    }
    if allow_all || allow_env {
        capabilities.grant(Capability::new("sys.env"));
    }
    if allow_all || allow_run {
        capabilities.grant(Capability::new("sys.process"));
    }
    if allow_all || allow_db {
        capabilities.grant(Capability::new("db.access"));
    }

    // Check for --cluster flag to start distributed node
    for i in 0..raw_args.len() {
        if raw_args[i] == "--cluster" && i + 1 < raw_args.len() {
            let bind_addr = &raw_args[i + 1];
            vre_core::pal::get_pal().eprintln(&format!("Starting VRE in distributed cluster mode on {}", bind_addr));
            let _node = vre_core::distributed::ClusterNode::new("node-1", bind_addr);
            // Just initializing for now.
            // A real server would bind and start listening.
        }
    }

    // Run VM
    let mut vm = match VirtualMachine::new(config, instructions, constants, native_imports, capabilities, function_table) {
        Ok(vm) => vm,
        Err(e) => {
            vre_core::pal::get_pal().eprintln(&format!("VM Init Error: {}", e));
            process::exit(1);
        }
    };

    // Inject the compiler-backed module loader so `import` statements work at runtime
    let base_dir = std::path::Path::new(input_path)
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    vm.set_module_loader(Box::new(module_loader::CompilerModuleLoader::new(base_dir)));

    if let Err(e) = vm.execute() {
        vre_core::pal::get_pal().eprintln(&format!("Runtime error: {}", e));
        process::exit(1);
    }

    // Leak detection — always run report, print only if --check-leaks or leaks found
    let report = vm.leak_report();
    if report.has_leaks() {
        let pal = vre_core::pal::get_pal();
        pal.eprintln("");
        pal.eprintln(&report.format());
        if check_leaks {
            // Non-zero exit when --check-leaks is used and leaks are found
            process::exit(2);
        }
    } else if check_leaks {
        // Explicit check requested — confirm clean heap
        vre_core::pal::get_pal().eprintln(&report.format());
    }
}

fn print_usage(program_name: &str) {
    println!("Vyauma Runtime Engine (VRE)");
    println!("Usage:");
    println!("  {} <file.vbc> [options]    - Execute compiled bytecode", program_name);
    println!("  {} <file.vym> [options]    - Compile and execute Vyauma source", program_name);
    println!("\nSecurity Options:");
    println!("  --allow-read               - Allow file system read access");
    println!("  --allow-write              - Allow file system write access");
    println!("  --allow-net                - Allow network access");
    println!("  --allow-env                - Allow environment variables access");
    println!("  --allow-run                - Allow spawning subprocesses");
    println!("  --allow-db                 - Allow database access");
    println!("  --allow-all                - Allow all access");
    println!("\nDebug Options:");
    println!("  --check-leaks              - Run and report heap leaks");
}

/// Renders a compiler error with a visual `^` pointer to the exact location.
///
/// Error strings with source location have the format: `[line:col] message`
/// e.g. `[5:12] Expected Colon, got Identifier("x")`
///
/// For type errors (no span prefix), prints a plain error with a hint.
fn render_diagnostic(source: &str, filename: &str, error: &str) {
    let pal = vre_core::pal::get_pal();

    // Try to parse a [line:col] prefix from the error string.
    // Pattern: starts with '[', then digits, ':', digits, ']'
    if error.starts_with('[') {
        if let Some(close) = error.find(']') {
            let span_part = &error[1..close];
            let rest = error[close + 1..].trim();
            let parts: Vec<&str> = span_part.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let (Ok(line_num), Ok(col_num)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    let lines: Vec<&str> = source.lines().collect();
                    let source_line = if line_num > 0 && line_num <= lines.len() {
                        lines[line_num - 1]
                    } else {
                        ""
                    };
                    let padding = " ".repeat(col_num.saturating_sub(1));
                    pal.eprintln(&format!("\nerror[E]: {}", rest));
                    pal.eprintln(&format!("  --> {}:{}:{}", filename, line_num, col_num));
                    pal.eprintln(&format!("   |"));
                    pal.eprintln(&format!("{:>3} | {}", line_num, source_line));
                    pal.eprintln(&format!("   | {}^", padding));
                    pal.eprintln(&format!("   |"));
                    return;
                }
            }
        }
    }

    // Fallback: plain error (type errors, etc.)
    pal.eprintln(&format!("\nerror: {}", error));
    pal.eprintln(&format!("  --> {}", filename));
}
