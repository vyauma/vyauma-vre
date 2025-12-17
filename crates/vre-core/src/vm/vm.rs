//! Virtual Machine Core
//!
//! Vyauma Virtual Machine execution engine.
//! Implements minimal instruction semantics as per bytecode spec v0.1.

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

    /// Execute a single instruction
    fn step(&mut self) -> VreResult<()> {
        let opcode_byte = self.read_u8()?;
        let opcode = OpCode::from_u8(opcode_byte)
            .ok_or(VreError::InvalidOpcode(opcode_byte))?;

        match opcode {
            OpCode::Halt => {
                self.halted = true;
                Ok(())
            }

            OpCode::Push => {
                let index = self.read_u8()? as usize;
                let value = self.constants.get(index)?;
                self.stack.push(value)
            }

            OpCode::Pop => {
                self.stack.pop()?;
                Ok(())
            }

            OpCode::Add => {
                let b = match self.stack.pop()? {
                    Value::Number(n) => n,
                    _ => return Err(VreError::TypeMismatch),
                };

                let a = match self.stack.pop()? {
                    Value::Number(n) => n,
                    _ => return Err(VreError::TypeMismatch),
                };

                self.stack.push(Value::Number(a + b))
            }

            _ => Err(VreError::RuntimeFault),
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
}
