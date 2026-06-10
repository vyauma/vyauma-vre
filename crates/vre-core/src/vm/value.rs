//! Runtime Value Representation
//!
//! Defines the core value types used by the Vyauma Virtual Machine.
//! This layer is intentionally minimal and language-neutral.

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Array(usize), // Heap reference
    Map(usize),   // Heap reference
    Object(usize),// Heap reference
    Function(usize), // Heap reference
    Reference(usize),// Generic Heap reference
}

impl Value {
    pub fn as_f64(&self) -> crate::error::VreResult<f64> {
        match self {
            Value::Int32(n) => Ok(*n as f64),
            Value::Int64(n) => Ok(*n as f64),
            Value::Float32(n) => Ok(*n as f64),
            Value::Float64(n) => Ok(*n),
            _ => Err(crate::error::VreError::TypeMismatch),
        }
    }
}
