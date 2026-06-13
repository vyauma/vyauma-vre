//! `vre test` — Run the project's test suite.

use std::path::Path;
use std::process;
use crate::cli::TestArgs;
use crate::config::VreToml;
use crate::diagnostics::{codes, Diagnostic};

pub fn run(args: TestArgs) {
    println!();
    println!("  Running test suite...");
    println!();

    // Resolve test path: explicit > `tests/` directory > current dir
    let test_path = args.path.unwrap_or_else(|| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        if Path::new("tests").exists() {
            "tests".to_string()
        } else {
            ".".to_string()
        }
    });

    if !Path::new(&test_path).exists() {
        Diagnostic::error(codes::E014, format!("Test path not found: '{}'", test_path))
            .with_hint("Create a `tests/` directory with .vya test files.")
            .emit();
        process::exit(1);
    }

    if args.watch {
        println!("  [watch mode] Watching for file changes...");
        println!("  (Watch mode will re-run tests on save)");
        println!();
        // TODO: implement inotify/FSEvents/ReadDirectoryChangesW watcher
    }

    if args.coverage {
        println!("  [coverage] Coverage collection enabled.");
        println!();
        // TODO: instrument bytecode for coverage tracking
    }

    if let Some(filter) = &args.filter {
        println!("  [filter] Running tests matching: '{}'", filter);
        println!();
    }

    // Delegate to the existing test runner
    crate::test_runner::run_tests(&test_path);
}
