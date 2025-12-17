//! Bytecode Opcode Definitions
//!
//! Defines the raw opcode set for Vyauma bytecode.
//! This file contains no execution semantics.
//! Opcode values are an eternal contract.

/// Bytecode opcodes (v0.1)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    // Stack operations
    Push = 0x01,
    Pop  = 0x02,
    Dup  = 0x03,

    // Local access
    LoadLocal  = 0x10,
    StoreLocal = 0x11,

    // Arithmetic
    Add = 0x20,
    Sub = 0x21,
    Mul = 0x22,
    Div = 0x23,
    Mod = 0x24,
    Neg = 0x25,

    // Comparison
    Equal        = 0x30,
    NotEqual     = 0x31,
    Less         = 0x32,
    LessEqual    = 0x33,
    Greater      = 0x34,
    GreaterEqual = 0x35,

    // Control flow
    Jump     = 0x40,
    JumpIf  = 0x41,
    Call    = 0x42,
    Return  = 0x43,

    // System
    Nop  = 0xF0,
    Halt = 0xFF,
}

impl OpCode {
    /// Convert raw byte to opcode
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(OpCode::Push),
            0x02 => Some(OpCode::Pop),
            0x03 => Some(OpCode::Dup),

            0x10 => Some(OpCode::LoadLocal),
            0x11 => Some(OpCode::StoreLocal),

            0x20 => Some(OpCode::Add),
            0x21 => Some(OpCode::Sub),
            0x22 => Some(OpCode::Mul),
            0x23 => Some(OpCode::Div),
            0x24 => Some(OpCode::Mod),
            0x25 => Some(OpCode::Neg),

            0x30 => Some(OpCode::Equal),
            0x31 => Some(OpCode::NotEqual),
            0x32 => Some(OpCode::Less),
            0x33 => Some(OpCode::LessEqual),
            0x34 => Some(OpCode::Greater),
            0x35 => Some(OpCode::GreaterEqual),

            0x40 => Some(OpCode::Jump),
            0x41 => Some(OpCode::JumpIf),
            0x42 => Some(OpCode::Call),
            0x43 => Some(OpCode::Return),

            0xF0 => Some(OpCode::Nop),
            0xFF => Some(OpCode::Halt),

            _ => None,
        }
    }
}
