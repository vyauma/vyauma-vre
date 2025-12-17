//! Bytecode Loader
//!
//! Loads and validates Vyauma bytecode.
//! This layer performs structural validation only.

use crate::error::{VreError, VreResult};
use crate::vm::value::Value;

/// Bytecode magic: "VYMA"
const BYTECODE_MAGIC: u32 = 0x5659_4D41;

/// Supported bytecode version
const VERSION_MAJOR: u8 = 1;

/// Minimum bytecode header size
const MIN_FILE_SIZE: usize = 16;

/// Loaded bytecode bundle
#[derive(Debug)]
pub struct LoadedBytecode {
    pub constants: Vec<Value>,
    pub instructions: Vec<u8>,
    pub entry_point: usize,
}

/// Bytecode loader
pub struct BytecodeLoader;

impl BytecodeLoader {
    /// Load bytecode from raw bytes
    pub fn load(bytes: &[u8]) -> VreResult<LoadedBytecode> {
        if bytes.len() < MIN_FILE_SIZE {
            return Err(VreError::BytecodeTooShort);
        }

        let mut cursor = 0;

        // Magic
        let magic = Self::read_u32(bytes, &mut cursor)?;
        if magic != BYTECODE_MAGIC {
            return Err(VreError::InvalidMagicNumber);
        }

        // Version
        let major = Self::read_u8(bytes, &mut cursor)?;
        let _minor = Self::read_u8(bytes, &mut cursor)?;
        let _patch = Self::read_u8(bytes, &mut cursor)?;

        if major != VERSION_MAJOR {
            return Err(VreError::InvalidBytecodeVersion);
        }

        // Reserved
        Self::read_u8(bytes, &mut cursor)?;

        // Entry point
        let entry_point = Self::read_u32(bytes, &mut cursor)? as usize;

        // Constants
        let constant_count = Self::read_u32(bytes, &mut cursor)? as usize;
        let mut constants = Vec::with_capacity(constant_count);

        for _ in 0..constant_count {
            constants.push(Self::read_constant(bytes, &mut cursor)?);
        }

        // Instructions
        let instruction_len = Self::read_u32(bytes, &mut cursor)? as usize;
        if cursor + instruction_len > bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }

        let instructions = bytes[cursor..cursor + instruction_len].to_vec();

        Ok(LoadedBytecode {
            constants,
            instructions,
            entry_point,
        })
    }

    /// Read a constant value (minimal runtime types only)
    fn read_constant(bytes: &[u8], cursor: &mut usize) -> VreResult<Value> {
        let tag = Self::read_u8(bytes, cursor)?;

        match tag {
            0x00 => Ok(Value::Null),
            0x01 => {
                let b = Self::read_u8(bytes, cursor)?;
                Ok(Value::Bool(b != 0))
            }
            0x02 => {
                let n = Self::read_f64(bytes, cursor)?;
                Ok(Value::Number(n))
            }
            0xFF => {
                let id = Self::read_u32(bytes, cursor)?;
                Ok(Value::Ref(id))
            }
            _ => Err(VreError::MalformedBytecode),
        }
    }

    fn read_u8(bytes: &[u8], cursor: &mut usize) -> VreResult<u8> {
        if *cursor >= bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = bytes[*cursor];
        *cursor += 1;
        Ok(v)
    }

    fn read_u32(bytes: &[u8], cursor: &mut usize) -> VreResult<u32> {
        if *cursor + 4 > bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = u32::from_be_bytes([
            bytes[*cursor],
            bytes[*cursor + 1],
            bytes[*cursor + 2],
            bytes[*cursor + 3],
        ]);
        *cursor += 4;
        Ok(v)
    }

    fn read_f64(bytes: &[u8], cursor: &mut usize) -> VreResult<f64> {
        if *cursor + 8 > bytes.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = f64::from_be_bytes([
            bytes[*cursor],
            bytes[*cursor + 1],
            bytes[*cursor + 2],
            bytes[*cursor + 3],
            bytes[*cursor + 4],
            bytes[*cursor + 5],
            bytes[*cursor + 6],
            bytes[*cursor + 7],
        ]);
        *cursor += 8;
        Ok(v)
    }
}
