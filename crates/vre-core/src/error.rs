//! VRE Error Types
//!
//! Defines all core error conditions produced by the Vyauma Runtime Engine.
//! Errors are deterministic, dependency-free, and scoped strictly to runtime concerns.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum VreError {
    // Bytecode errors
    InvalidMagicNumber,
    InvalidBytecodeVersion,
    InvalidOpcode(u8),
    MalformedBytecode,
    BytecodeTooShort,

    // VM execution errors
    StackOverflow,
    StackUnderflow,
    InvalidStackAccess,
    InvalidLocalAccess(usize),
    InvalidConstantAccess(usize),
    DivisionByZero,
    InvalidJumpTarget(usize),
    InvalidFunctionIndex(usize),

    // Capability & security errors
    CapabilityNotGranted,
    CapabilityDenied,
    SecurityViolation,

    // Resource & runtime errors
    OutOfMemory,
    TypeMismatch,
    RuntimeFault,

    // IO boundary
    IoError(String),
}

impl fmt::Display for VreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VreError::InvalidMagicNumber =>
                write!(f, "invalid bytecode magic number"),
            VreError::InvalidBytecodeVersion =>
                write!(f, "incompatible bytecode version"),
            VreError::InvalidOpcode(op) =>
                write!(f, "invalid opcode: 0x{:02X}", op),
            VreError::MalformedBytecode =>
                write!(f, "malformed bytecode"),
            VreError::BytecodeTooShort =>
                write!(f, "bytecode is too short"),

            VreError::StackOverflow =>
                write!(f, "stack overflow"),
            VreError::StackUnderflow =>
                write!(f, "stack underflow"),
            VreError::InvalidStackAccess =>
                write!(f, "invalid stack access"),
            VreError::InvalidLocalAccess(idx) =>
                write!(f, "invalid local access: {}", idx),
            VreError::InvalidConstantAccess(idx) =>
                write!(f, "invalid constant access: {}", idx),
            VreError::DivisionByZero =>
                write!(f, "division by zero"),
            VreError::InvalidJumpTarget(addr) =>
                write!(f, "invalid jump target: {}", addr),
            VreError::InvalidFunctionIndex(idx) =>
                write!(f, "invalid function index: {}", idx),

            VreError::CapabilityNotGranted =>
                write!(f, "capability not granted"),
            VreError::CapabilityDenied =>
                write!(f, "capability denied"),
            VreError::SecurityViolation =>
                write!(f, "security violation"),

            VreError::OutOfMemory =>
                write!(f, "out of memory"),
            VreError::TypeMismatch =>
                write!(f, "type mismatch"),
            VreError::RuntimeFault =>
                write!(f, "runtime fault"),

            VreError::IoError(msg) =>
                write!(f, "io error: {}", msg),
        }
    }
}

impl From<io::Error> for VreError {
    fn from(err: io::Error) -> Self {
        VreError::IoError(err.to_string())
    }
}

pub type VreResult<T> = Result<T, VreError>;
