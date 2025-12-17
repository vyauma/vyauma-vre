//! Bytecode Instruction Representation
//!
//! Defines the raw instruction format for Vyauma bytecode.
//! This layer contains no execution semantics.

use super::opcode::OpCode;

/// Raw bytecode instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operand: u16,
}

impl Instruction {
    /// Create an instruction with no operand
    pub fn new(opcode: OpCode) -> Self {
        Instruction {
            opcode,
            operand: 0,
        }
    }

    /// Create an instruction with a single operand
    pub fn with_operand(opcode: OpCode, operand: u16) -> Self {
        Instruction {
            opcode,
            operand,
        }
    }
}
