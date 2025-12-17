//! VM Memory Model
//!
//! Defines memory structures used during VM execution.
//! This layer is index-based and language-neutral.

use crate::error::{VreError, VreResult};
use super::value::Value;

/// Global variable storage (index-based)
#[derive(Debug)]
pub struct Globals {
    values: Vec<Value>,
}

impl Globals {
    pub fn new(size: usize) -> Self {
        Globals {
            values: vec![Value::Null; size],
        }
    }

    pub fn load(&self, index: usize) -> VreResult<Value> {
        self.values
            .get(index)
            .cloned()
            .ok_or(VreError::InvalidStackAccess)
    }

    pub fn store(&mut self, index: usize, value: Value) -> VreResult<()> {
        if index >= self.values.len() {
            return Err(VreError::InvalidStackAccess);
        }
        self.values[index] = value;
        Ok(())
    }
}

/// Local variables for a single call frame
#[derive(Debug)]
pub struct Locals {
    values: Vec<Value>,
}

impl Locals {
    pub fn new(size: usize) -> Self {
        Locals {
            values: vec![Value::Null; size],
        }
    }

    pub fn load(&self, index: usize) -> VreResult<Value> {
        self.values
            .get(index)
            .cloned()
            .ok_or(VreError::InvalidLocalAccess(index))
    }

    pub fn store(&mut self, index: usize, value: Value) -> VreResult<()> {
        if index >= self.values.len() {
            return Err(VreError::InvalidLocalAccess(index));
        }
        self.values[index] = value;
        Ok(())
    }
}

/// Constant pool (read-only)
#[derive(Debug)]
pub struct ConstantPool {
    values: Vec<Value>,
}

impl ConstantPool {
    pub fn new(values: Vec<Value>) -> Self {
        ConstantPool { values }
    }

    pub fn get(&self, index: usize) -> VreResult<Value> {
        self.values
            .get(index)
            .cloned()
            .ok_or(VreError::InvalidConstantAccess(index))
    }
}
