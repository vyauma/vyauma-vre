//! VRE Configuration
//!
//! Defines runtime limits for the Vyauma Virtual Machine.
//! Configuration specifies constraints only; enforcement is handled by the VM.

use std::collections::HashMap;
use crate::vm::vm::NativeFunction;

use crate::capability::capability::Capability;

#[derive(Clone)]
pub struct FfiBinding {
    pub func: NativeFunction,
    pub caps: Vec<Capability>,
}

/// VM Configuration
#[derive(Clone)]
pub struct VreConfig {
    /// Maximum stack depth
    pub max_stack_size: usize,

    /// Maximum number of local variables per function
    pub max_locals: usize,

    /// Maximum call depth (recursion limit)
    pub max_call_depth: usize,

    /// Foreign Function Interface definitions
    pub ffi_functions: HashMap<String, FfiBinding>,
}

impl std::fmt::Debug for VreConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VreConfig")
            .field("max_stack_size", &self.max_stack_size)
            .field("max_locals", &self.max_locals)
            .field("max_call_depth", &self.max_call_depth)
            .field("ffi_functions", &format!("<{} native functions>", self.ffi_functions.len()))
            .finish()
    }
}

impl Default for VreConfig {
    fn default() -> Self {
        VreConfig {
            max_stack_size: 1024,
            max_locals: 256,
            max_call_depth: 256,
            ffi_functions: HashMap::new(),
        }
    }
}

impl VreConfig {
    /// Create a new configuration with default limits
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a native FFI function with capability constraints
    pub fn register_ffi(&mut self, name: &str, func: NativeFunction, caps: Vec<Capability>) {
        self.ffi_functions.insert(name.to_string(), FfiBinding { func, caps });
    }

    /// Register a native FFI function with no capability constraints (legacy wrapper)
    pub fn insert_ffi(&mut self, name: String, func: NativeFunction) {
        self.ffi_functions.insert(name, FfiBinding { func, caps: vec![] });
    }
}
