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
    LoadLocalI32 = 0x12,
    LoadLocalI64 = 0x13,
    LoadLocalF32 = 0x14,
    LoadLocalF64 = 0x15,
    LoadLocalStr = 0x16,

    // Arithmetic Int32
    AddI32 = 0x20, SubI32 = 0x21, MulI32 = 0x22, DivI32 = 0x23, ModI32 = 0x24, NegI32 = 0x25,
    // Arithmetic Int64
    AddI64 = 0x26, SubI64 = 0x27, MulI64 = 0x28, DivI64 = 0x29, ModI64 = 0x2A, NegI64 = 0x2B,
    // Arithmetic Float32
    AddF32 = 0x2C, SubF32 = 0x2D, MulF32 = 0x2E, DivF32 = 0x2F, ModF32 = 0x30, NegF32 = 0x31,
    // Arithmetic Float64
    AddF64 = 0x32, SubF64 = 0x33, MulF64 = 0x34, DivF64 = 0x35, ModF64 = 0x36, NegF64 = 0x37,

    // Comparison Int32
    EqualI32 = 0x38, NotEqualI32 = 0x39, LessI32 = 0x3A, LessEqualI32 = 0x3B, GreaterI32 = 0x3C, GreaterEqualI32 = 0x3D,
    // Comparison Int64
    EqualI64 = 0x3E, NotEqualI64 = 0x3F, LessI64 = 0x40, LessEqualI64 = 0x41, GreaterI64 = 0x42, GreaterEqualI64 = 0x43,
    // Comparison Float32
    EqualF32 = 0x44, NotEqualF32 = 0x45, LessF32 = 0x46, LessEqualF32 = 0x47, GreaterF32 = 0x48, GreaterEqualF32 = 0x49,
    // Comparison Float64
    EqualF64 = 0x4A, NotEqualF64 = 0x4B, LessF64 = 0x4C, LessEqualF64 = 0x4D, GreaterF64 = 0x4E, GreaterEqualF64 = 0x4F,
    // Comparison String
    EqualStr = 0x50, NotEqualStr = 0x51,
    // Logical
    AndBool = 0x52, OrBool = 0x53,

    // Control flow
    Jump     = 0x60,
    JumpIf  = 0x61,
    Call    = 0x62,
    Return  = 0x63,
    Spawn   = 0x64,
    Yield   = 0x65,
    Await   = 0x66,

    // Heap and Objects
    NewArray     = 0x70,
    LoadElement  = 0x71,
    StoreElement = 0x72,
    NewStruct     = 0x73,
    LoadProperty  = 0x74,
    StoreProperty = 0x75,

    // FFI Native Call
    CallNative = 0x76,

    // Exception Handling
    TryStart = 0x80,
    TryEnd   = 0x81,
    Throw    = 0x82,

    // System
    Nop     = 0xF0,
    Syscall = 0xF1,
    Halt    = 0xFF,
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
            0x12 => Some(OpCode::LoadLocalI32),
            0x13 => Some(OpCode::LoadLocalI64),
            0x14 => Some(OpCode::LoadLocalF32),
            0x15 => Some(OpCode::LoadLocalF64),
            0x16 => Some(OpCode::LoadLocalStr),

            0x20 => Some(OpCode::AddI32), 0x21 => Some(OpCode::SubI32), 0x22 => Some(OpCode::MulI32), 0x23 => Some(OpCode::DivI32), 0x24 => Some(OpCode::ModI32), 0x25 => Some(OpCode::NegI32),
            0x26 => Some(OpCode::AddI64), 0x27 => Some(OpCode::SubI64), 0x28 => Some(OpCode::MulI64), 0x29 => Some(OpCode::DivI64), 0x2A => Some(OpCode::ModI64), 0x2B => Some(OpCode::NegI64),
            0x2C => Some(OpCode::AddF32), 0x2D => Some(OpCode::SubF32), 0x2E => Some(OpCode::MulF32), 0x2F => Some(OpCode::DivF32), 0x30 => Some(OpCode::ModF32), 0x31 => Some(OpCode::NegF32),
            0x32 => Some(OpCode::AddF64), 0x33 => Some(OpCode::SubF64), 0x34 => Some(OpCode::MulF64), 0x35 => Some(OpCode::DivF64), 0x36 => Some(OpCode::ModF64), 0x37 => Some(OpCode::NegF64),

            0x38 => Some(OpCode::EqualI32), 0x39 => Some(OpCode::NotEqualI32), 0x3A => Some(OpCode::LessI32), 0x3B => Some(OpCode::LessEqualI32), 0x3C => Some(OpCode::GreaterI32), 0x3D => Some(OpCode::GreaterEqualI32),
            0x3E => Some(OpCode::EqualI64), 0x3F => Some(OpCode::NotEqualI64), 0x40 => Some(OpCode::LessI64), 0x41 => Some(OpCode::LessEqualI64), 0x42 => Some(OpCode::GreaterI64), 0x43 => Some(OpCode::GreaterEqualI64),
            0x44 => Some(OpCode::EqualF32), 0x45 => Some(OpCode::NotEqualF32), 0x46 => Some(OpCode::LessF32), 0x47 => Some(OpCode::LessEqualF32), 0x48 => Some(OpCode::GreaterF32), 0x49 => Some(OpCode::GreaterEqualF32),
            0x4A => Some(OpCode::EqualF64), 0x4B => Some(OpCode::NotEqualF64), 0x4C => Some(OpCode::LessF64), 0x4D => Some(OpCode::LessEqualF64), 0x4E => Some(OpCode::GreaterF64), 0x4F => Some(OpCode::GreaterEqualF64),
            0x50 => Some(OpCode::EqualStr), 0x51 => Some(OpCode::NotEqualStr),
            0x52 => Some(OpCode::AndBool), 0x53 => Some(OpCode::OrBool),

            0x60 => Some(OpCode::Jump),
            0x61 => Some(OpCode::JumpIf),
            0x62 => Some(OpCode::Call),
            0x63 => Some(OpCode::Return),
            0x64 => Some(OpCode::Spawn),
            0x65 => Some(OpCode::Yield),
            0x66 => Some(OpCode::Await),

            0x70 => Some(OpCode::NewArray),
            0x71 => Some(OpCode::LoadElement),
            0x72 => Some(OpCode::StoreElement),
            0x73 => Some(OpCode::NewStruct),
            0x74 => Some(OpCode::LoadProperty),
            0x75 => Some(OpCode::StoreProperty),
            0x76 => Some(OpCode::CallNative),

            0x80 => Some(OpCode::TryStart),
            0x81 => Some(OpCode::TryEnd),
            0x82 => Some(OpCode::Throw),

            0xF0 => Some(OpCode::Nop),
            0xF1 => Some(OpCode::Syscall),
            0xFF => Some(OpCode::Halt),

            _ => None,
        }
    }
}
