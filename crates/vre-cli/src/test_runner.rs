use std::path::Path;
use vre_core::config::VreConfig;
use vre_core::vm::vm::VirtualMachine;
use vre_compiler::compile;

pub fn run_tests(path_str: &str) {
    let path = Path::new(path_str);
    
    // In a real framework, we'd recursively find files ending in `_test.vya`
    // For Phase 25, we just run the specified file as a test script.
    
    if !path.exists() {
        println!("Test file or directory does not exist: {}", path_str);
        return;
    }

    println!("Running tests in {}...", path_str);

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to read test file: {}", e);
            return;
        }
    };

    let base_path = path.parent().unwrap_or(Path::new("."));
    
    let compiled = match compile(&source, path_str, Some(base_path)) {
        Ok(c) => c,
        Err(e) => {
            println!("Compilation failed:\n{}", e);
            std::process::exit(1);
        }
    };

    let mut config = VreConfig::default();
    crate::native::register_ffi(&mut config);
    vre_core::vm::api::register_apis(&mut config);

    let mut capabilities = vre_core::CapabilityRegistry::new();
    // Tests get full capabilities by default
    capabilities.grant(vre_core::Capability::new("fs.read"));
    capabilities.grant(vre_core::Capability::new("fs.write"));
    capabilities.grant(vre_core::Capability::new("net.connect"));
    capabilities.grant(vre_core::Capability::new("io.read"));
    capabilities.grant(vre_core::Capability::new("io.write"));
    capabilities.grant(vre_core::Capability::new("db.read"));
    capabilities.grant(vre_core::Capability::new("db.write"));

    let mut vm = match VirtualMachine::new(
        config,
        compiled.instructions,
        compiled.constants,
        compiled.native_imports,
        capabilities,
        compiled.function_table,
    ) {
        Ok(v) => v,
        Err(e) => {
            println!("VM Initialization failed: {}", e);
            std::process::exit(1);
        }
    };

    // Tests run the top-level script which should contain assertions.
    // If any assertion fails, it triggers a VRE runtime error.
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(vm.execute()) {
        Ok(_) => {
            println!("\n✅ All tests passed in {}.", path_str);
        }
        Err(e) => {
            println!("\n❌ Test failure in {}:", path_str);
            println!("  {}", e);
            std::process::exit(1);
        }
    }
}
