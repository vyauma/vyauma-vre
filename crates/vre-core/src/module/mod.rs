//! Module Cache
//!
//! Runtime cache for compiled and executed Vyauma modules.
//! Each module is identified by its canonical file path.
//! Once a module has been executed, its exports are stored here
//! so subsequent `import` statements return the same namespace object
//! without re-compiling or re-executing the module.

use std::collections::HashMap;
use crate::vm::value::Value;

/// A single module's exported bindings.
pub type ModuleExports = HashMap<String, Value>;

/// Runtime module cache — prevents duplicate compilation and execution
/// of the same module file.
#[derive(Debug, Default)]
pub struct ModuleCache {
    /// canonical resolved path → exported symbol table
    entries: HashMap<String, ModuleExports>,
}

impl ModuleCache {
    /// Create an empty module cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Returns true if the module at `path` has already been loaded.
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }

    /// Retrieve the exports of a previously loaded module.
    pub fn get(&self, path: &str) -> Option<&ModuleExports> {
        self.entries.get(path)
    }

    /// Store the exports of a freshly executed module.
    pub fn insert(&mut self, path: String, exports: ModuleExports) {
        self.entries.insert(path, exports);
    }
}
