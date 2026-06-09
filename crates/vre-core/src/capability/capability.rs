//! Capability Definition
//!
//! A capability is a named, explicit permission that code must hold
//! before exercising a privileged operation.
//! This module defines the core capability type.

/// A named capability token
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    /// Unique capability name (e.g. "io.read", "net.connect")
    pub name: &'static str,
}

impl Capability {
    /// Define a new capability
    pub const fn new(name: &'static str) -> Self {
        Capability { name }
    }
}
