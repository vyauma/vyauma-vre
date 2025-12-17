//! VRE Configuration
//!
//! Defines runtime limits for the Vyauma Virtual Machine.
//! Configuration specifies constraints only; enforcement is handled by the VM.

/// VM Configuration
#[derive(Debug, Clone)]
pub struct VreConfig {
    /// Maximum stack depth
    pub max_stack_size: usize,

    /// Maximum number of local variables per function
    pub max_locals: usize,

    /// Maximum call depth (recursion limit)
    pub max_call_depth: usize,
}

impl Default for VreConfig {
    fn default() -> Self {
        VreConfig {
            max_stack_size: 1024,
            max_locals: 256,
            max_call_depth: 256,
        }
    }
}

impl VreConfig {
    /// Create a new configuration with default limits
    pub fn new() -> Self {
        Self::default()
    }
}
