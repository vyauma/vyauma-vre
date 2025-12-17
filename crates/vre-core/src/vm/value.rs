//! Runtime Value Representation
//!
//! Defines the core value types used by the Vyauma Virtual Machine.
//! This layer is intentionally minimal and language-neutral.

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Absence of a value
    Null,

    /// Boolean value
    Bool(bool),

    /// Numeric value (IEEE 754)
    Number(f64),

    /// Opaque reference (reserved for future extensions)
    Ref(u32),
}
