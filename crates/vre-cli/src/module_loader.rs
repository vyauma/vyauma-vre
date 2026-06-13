//! Compiler-backed Module Loader
//!
//! Implements `ModuleLoader` for use inside the VRE runtime.
//! Lives in `vre-cli` to avoid a circular dependency between `vre-core` and `vre-compiler`.

use std::path::{Path, PathBuf};
use vre_core::vm::vm::{ModuleLoader, CompiledModule};
use vre_core::vm::value::Value;

/// A `ModuleLoader` that delegates to `vre_compiler::compile`.
pub struct CompilerModuleLoader {
    /// The base directory of the entry-point file (used for relative import resolution).
    base_dir: PathBuf,
}

impl CompilerModuleLoader {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

impl ModuleLoader for CompilerModuleLoader {
    fn load(&self, path: &str, _base_dir_override: Option<&str>) -> Result<CompiledModule, String> {
        // Resolve the module path
        let module_path = resolve_module_path(path, &self.base_dir)?;
        let module_path_str = module_path.to_string_lossy().to_string();

        let source = std::fs::read_to_string(&module_path)
            .map_err(|e| format!("Cannot read module '{}': {}", module_path_str, e))?;

        let base = module_path.parent();
        let compiled = vre_compiler::compile(&source, &module_path_str, base)
            .map_err(|e| format!("Compile error in module '{}': {}", module_path_str, e))?;

        Ok(CompiledModule {
            instructions: compiled.instructions,
            constants: compiled.constants,
            native_imports: compiled.native_imports,
            function_table: compiled.function_table,
        })
    }
}

/// Resolve a module path to an absolute `PathBuf`.
///
/// Resolution order:
/// 1. `std/<name>` → `$VRE_STD_PATH/<name>.vya` (or `.vym`)
/// 2. `./` or `../` relative path → resolved against `base_dir`
/// 3. Bare name → `vym_modules/<name>/index.vym` or `.vya`
fn resolve_module_path(path: &str, base_dir: &Path) -> Result<PathBuf, String> {
    if path.starts_with("std/") {
        let std_root = std::env::var("VRE_STD_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("std"));
        let stripped = path.strip_prefix("std/").unwrap();
        let p = std_root.join(with_extension(stripped));
        return Ok(p);
    }

    if path.starts_with("./") || path.starts_with("../") {
        let p = base_dir.join(with_extension(path));
        return Ok(p);
    }

    // Package name — look in vym_modules/
    let modules_root = std::env::var("VRE_MODULES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("vym_modules")
        });

    // Try index.vym then index.vya then index.js etc.
    for index_name in &[
        "index.vym", "index.vya", "index.js", "index.ts", "index.php", "index.py"
    ] {
        let candidate = modules_root.join(path).join(index_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "Cannot resolve module '{}' — not found in ./vym_modules/{}",
        path, path
    ))
}

fn with_extension(p: &str) -> String {
    if p.ends_with(".vya") || p.ends_with(".vym") || p.ends_with(".js")
        || p.ends_with(".ts") || p.ends_with(".php") || p.ends_with(".py")
    {
        p.to_string()
    } else {
        format!("{}.vya", p)
    }
}
