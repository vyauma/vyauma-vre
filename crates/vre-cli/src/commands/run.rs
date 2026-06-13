//! `vre run` — Compile and execute a Vyauma source file or project.
//!
//! This command preserves the full VM execution pipeline from the original
//! monolithic `main.rs`, including capability registration, module loader
//! injection, and heap leak detection.

use std::collections::HashMap;
use std::path::Path;
use std::process;

use vre_core::config::VreConfig;
use vre_core::loader::loader::BytecodeLoader;
use vre_core::vm::vm::VirtualMachine;
use vre_core::{Capability, CapabilityRegistry};

use crate::cli::RunArgs;
use crate::config::VreToml;
use crate::diagnostics::{self, codes, Diagnostic};

pub fn run(args: RunArgs) {
    // Resolve the source file: explicit argument > project entry point
    let input_path: String = args.file.unwrap_or_else(|| {
        // Try to load vre.toml and get the configured entry point
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        match VreToml::find_and_load(&cwd) {
            Ok((manifest, _)) => manifest.project.entry,
            Err(_) => {
                // No manifest — try conventional defaults
                for candidate in &["src/main.vya", "src/main.vym", "src/main.ts", "main.vya"] {
                    if Path::new(candidate).exists() {
                        return candidate.to_string();
                    }
                }
                Diagnostic::error(codes::E003,
                    "No source file specified and no vre.toml found.")
                    .with_suggestion("Run `vre init` to initialize a project, or specify a file:\n  vre run src/main.vya")
                    .emit();
                process::exit(1);
            }
        }
    });

    // Verify the file exists
    if !Path::new(&input_path).exists() {
        Diagnostic::error(codes::E014, format!("File not found: '{}'", input_path))
            .with_hint("Make sure the path is correct and the file exists.")
            .emit();
        process::exit(1);
    }

    // ── Compile or load bytecode ───────────────────────────────────────────────

    let (instructions, constants, native_imports, function_table) =
        if is_source_file(&input_path) {
            compile_source(&input_path)
        } else {
            load_bytecode(&input_path)
        };

    // ── Set up VM configuration and FFI ───────────────────────────────────────

    let mut config = VreConfig::default();
    crate::native::register_ffi(&mut config);
    vre_core::vm::api::register_apis(&mut config);

    // ── Set up capability registry ────────────────────────────────────────────

    let mut capabilities = CapabilityRegistry::new();

    // Console I/O is always granted
    capabilities.grant(Capability::new("io.read"));
    capabilities.grant(Capability::new("io.write"));

    let grant_all = args.allow_all;
    if grant_all || args.allow_read  { capabilities.grant(Capability::new("fs.read")); }
    if grant_all || args.allow_write { capabilities.grant(Capability::new("fs.write")); }
    if grant_all || args.allow_net {
        capabilities.grant(Capability::new("net.listen"));
        capabilities.grant(Capability::new("net.accept"));
        capabilities.grant(Capability::new("net.connect"));
    }
    if grant_all || args.allow_env { capabilities.grant(Capability::new("sys.env")); }
    if grant_all || args.allow_run { capabilities.grant(Capability::new("sys.process")); }
    if grant_all || args.allow_db  { capabilities.grant(Capability::new("db.access")); }

    // ── Distributed cluster mode ──────────────────────────────────────────────

    if let Some(bind_addr) = &args.cluster {
        vre_core::pal::get_pal().eprintln(
            &format!("Starting VRE in distributed cluster mode on {}", bind_addr)
        );
        let _node = vre_core::distributed::ClusterNode::new("node-1", bind_addr);
    }

    // ── Initialise VM ─────────────────────────────────────────────────────────

    let mut vm = match VirtualMachine::new(
        config,
        instructions,
        constants,
        native_imports,
        capabilities,
        function_table,
    ) {
        Ok(vm) => vm,
        Err(e) => {
            Diagnostic::error(codes::E006, format!("VM initialisation error: {}", e.to_string())).emit();
            process::exit(1);
        }
    };

    // Inject compiler-backed module loader
    let base_dir = Path::new(&input_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    vm.set_module_loader(Box::new(
        crate::module_loader::CompilerModuleLoader::new(base_dir)
    ));

    // ── Execute ───────────────────────────────────────────────────────────────

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(vm.execute()) {
        Diagnostic::error(codes::E006, e.to_string())
            .with_hint("Check the stack trace above for more details.")
            .emit();
        process::exit(1);
    }

    // ── Heap leak detection ───────────────────────────────────────────────────

    let report = vm.leak_report();
    if report.has_leaks() {
        let pal = vre_core::pal::get_pal();
        pal.eprintln("");
        pal.eprintln(&report.format());
        if args.check_leaks {
            process::exit(2);
        }
    } else if args.check_leaks {
        vre_core::pal::get_pal().eprintln(&report.format());
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn is_source_file(path: &str) -> bool {
    matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str()),
        Some("vym" | "vya" | "js" | "ts" | "php" | "py")
    )
}

type CompiledOutput = (
    Vec<u8>,
    Vec<vre_core::vm::value::Value>,
    Vec<String>,
    HashMap<String, u32>,
);

fn compile_source(input_path: &str) -> CompiledOutput {
    let source = match vre_core::pal::get_pal().read_to_string(Path::new(input_path)) {
        Ok(s) => s,
        Err(e) => {
            Diagnostic::error(codes::E014, format!("Failed to read source file: {}", e)).emit();
            process::exit(1);
        }
    };

    let path = Path::new(input_path);
    let base_path = path.parent().unwrap_or(Path::new("."));

    match vre_compiler::compile(&source, input_path, Some(base_path)) {
        Ok(compiled) => (
            compiled.instructions,
            compiled.constants,
            compiled.native_imports,
            compiled.function_table,
        ),
        Err(e) => {
            diagnostics::emit_compiler_error(&source, input_path, &e);
            process::exit(1);
        }
    }
}

fn load_bytecode(input_path: &str) -> CompiledOutput {
    let bytes = match std::fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            Diagnostic::error(codes::E014, format!("Failed to read bytecode file: {}", e)).emit();
            process::exit(1);
        }
    };

    let loaded = match BytecodeLoader::load(&bytes) {
        Ok(bc) => bc,
        Err(e) => {
            Diagnostic::error(codes::E006, format!("Invalid bytecode: {}", e))
                .with_hint("Ensure the file was produced by the VRE compiler.")
                .emit();
            process::exit(1);
        }
    };

    (loaded.instructions, loaded.constants, Vec::new(), HashMap::new())
}
