//! VM Stack Implementation
//!
//! Stack data structure for VM execution.
//! No execution semantics.

use crate::error::{VreError, VreResult};
use super::value::Value;

/// VM execution stack
#[derive(Debug)]
pub struct Stack {
    values: Vec<Value>,
    max_size: usize,
}

impl Stack {
    /// Create new stack with maximum size
    pub fn new(max_size: usize) -> Self {
        Stack {
            values: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Push value onto stack
    pub fn push(&mut self, value: Value) -> VreResult<()> {
        if self.values.len() >= self.max_size {
            return Err(VreError::StackOverflow);
        }
        self.values.push(value);
        Ok(())
    }

    /// Pop value from stack
    pub fn pop(&mut self) -> VreResult<Value> {
        self.values.pop().ok_or(VreError::StackUnderflow)
    }

    /// Peek at top of stack without removing
    pub fn peek(&self) -> VreResult<&Value> {
        self.values.last().ok_or(VreError::StackUnderflow)
    }

    /// Duplicate top value
    pub fn dup(&mut self) -> VreResult<()> {
        let value = self.peek()?.clone();
        self.push(value)
    }

    /// Get current stack size
    pub fn size(&self) -> usize {
        self.values.len()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Clear stack
    pub fn clear(&mut self) {
        self.values.clear();
    }
}
