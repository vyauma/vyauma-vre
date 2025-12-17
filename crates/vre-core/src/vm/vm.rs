//! Virtual Machine Core
//!
//! Defines the Vyauma Virtual Machine structure and execution loop.
//! Instruction semantics are intentionally minimal in v0.1.

use crate::config::VreConfig;
use crate::error::{VreError, VreResult};
use crate::bytecode::opcode::OpCode;

use super::stack::Stack;
use super::memory::{Globals, Locals, ConstantPool};
use super::value::Value;

/// Call frame representing a single function invocation
#[derive(Debug)]
struct CallFrame {
    return_ip: usize,
    locals: Locals,
}

/// Vyauma Virtual Machine
#[derive(Debug)]
pub struct VirtualMachine {
    config: VreConfig,
    stack: Stack,
    globals: Globals,
    constants: ConstantPool,

    instructions: Vec<u8>,
    ip: usize,

    call_stack: Vec<CallFrame>,
    halted: bool,
}

impl VirtualMachine {
    /// Create a new VM instance
    pub fn new(
        config: VreConfig,
        constants: Vec<Value>,
        instructions: Vec<u8>,
        global_count: usize,
    ) -> Self {
        VirtualMachine {
            stack: Stack::new(config.max_stack_size),
            globals: Globals::new(global_count),
            constants: ConstantPool::new(constants),
            instructions,
            ip: 0,
            call_stack: Vec::new(),
            halted: false,
            config,
        }
    }

    /// Execute bytecode until halt or error
    pub fn execute(&mut self) -> VreResult<()> {
        while !self.halted && self.ip < self.instructions.len() {
            self.step()?;
        }
        Ok(())
    }

    /// Execute a single instruction (dispatch only)
    fn step(&mut self) -> VreResult<()> {
        let opcode_byte = self.read_u8()?;
        let opcode = OpCode::from_u8(opcode_byte)
            .ok_or(VreError::InvalidOpcode(opcode_byte))?;

        match opcode {
            OpCode::Nop => Ok(()),
            OpCode::Halt => {
                self.halted = true;
                Ok(())
            }

            // Stack
            OpCode::Push
            | OpCode::Pop
            | OpCode::Dup

            // Locals
            | OpCode::LoadLocal
            | OpCode::StoreLocal

            // Arithmetic
            | OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Mod
            | OpCode::Neg

            // Comparison
            | OpCode::Equal
            | OpCode::NotEqual
            | OpCode::Less
            | OpCode::LessEqual
            | OpCode::Greater
            | OpCode::GreaterEqual

            // Control flow
            | OpCode::Jump
            | OpCode::JumpIf
            | OpCode::Call
            | OpCode::Return
            => Err(VreError::RuntimeFault),
        }
    }

    /// Read next byte from instruction stream
    fn read_u8(&mut self) -> VreResult<u8> {
        if self.ip >= self.instructions.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let byte = self.instructions[self.ip];
        self.ip += 1;
        Ok(byte)
    }

    /// Read a big-endian u16 operand
    fn read_u16(&mut self) -> VreResult<u16> {
        let high = self.read_u8()? as u16;
        let low = self.read_u8()? as u16;
        Ok((high << 8) | low)
    }
}
