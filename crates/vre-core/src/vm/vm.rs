//! Virtual Machine Core
//!
//! Vyauma Virtual Machine execution engine.
//! Implements instruction semantics as per bytecode spec v0.1.

use crate::config::VreConfig;
use crate::error::{VreError, VreResult};
use crate::bytecode::opcode::OpCode;
use crate::capability::registry::CapabilityRegistry;
use crate::capability::CapabilityId;

use super::stack::Stack;
use super::memory::{Globals, Locals, ConstantPool};
use super::value::Value;

/// State change events produced by the VM for deterministic observers (Samskāra)
#[derive(Debug, Clone)]
pub enum StateChange {
    /// A local variable was written
    LocalStore { index: usize, value: Value },

    /// External call requested by bytecode (capability-checked). The VM does not execute this.
    ExternalCallRequest { cap_id: u8, args: Vec<Value> },
}

/// Call frame representing a single function invocation
#[derive(Debug)]
struct CallFrame {
    _return_ip: usize,
    locals: Locals,
}

/// Vyauma Virtual Machine
#[derive(Debug)]
pub struct VirtualMachine {
    _config: VreConfig,
    stack: Stack,
    _globals: Globals,
    constants: ConstantPool,

    instructions: Vec<u8>,
    ip: usize,

    call_stack: Vec<CallFrame>,
    halted: bool,
    /// Collected state-change events produced during execution
    state_changes: Vec<StateChange>,
    /// Capability registry (Suraksha). Defaults to deny-all.
    capabilities: CapabilityRegistry,
}

impl VirtualMachine {
    pub fn new(
        config: VreConfig,
        constants: Vec<Value>,
        instructions: Vec<u8>,
        global_count: usize,
    ) -> Self {
        VirtualMachine {
            stack: Stack::new(config.max_stack_size),
            _globals: Globals::new(global_count),
            constants: ConstantPool::new(constants),
            instructions,
            ip: 0,
            call_stack: vec![CallFrame { _return_ip: 0, locals: Locals::new(config.max_locals) }],
            halted: false,
            _config: config,
            state_changes: Vec::new(),
            capabilities: CapabilityRegistry::new(),
        }
    }

    /// Drain and return any state changes recorded by the VM. This clears the internal buffer.
    pub fn drain_state_changes(&mut self) -> Vec<StateChange> {
        std::mem::take(&mut self.state_changes)
    }

    /// Grant a capability to the VM (host-managed). This is explicit and minimal.
    pub fn grant_capability(&mut self, id: u8) {
        self.capabilities.grant(CapabilityId(id));
    }

    /// Resume execution after an external call has been handled by the host.
    pub fn resume(&mut self) {
        self.halted = false;
    }

    /// Apply values produced by the host for an external call. Values are pushed
    /// onto the VM stack in order.
    pub fn apply_external_results(&mut self, results: Vec<Value>) -> VreResult<()> {
        for v in results {
            self.stack.push(v)?;
        }
        Ok(())
    }

    /// Peek top of stack without removing (host/test helper)
    pub fn peek_top(&self) -> VreResult<Value> {
        self.stack.peek().map(|v| v.clone())
    }

    /// Pop the top value from the VM stack (host/test helper).
    pub fn pop_top(&mut self) -> VreResult<Value> {
        self.stack.pop()
    }

    pub fn execute(&mut self) -> VreResult<()> {
        while !self.halted && self.ip < self.instructions.len() {
            self.step()?;
        }
        Ok(())
    }

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

            OpCode::Dup => self.stack.dup(),

            OpCode::LoadLocal => {
                let index = self.read_u8()? as usize;
                let frame = self
                    .call_stack
                    .last()
                    .ok_or(VreError::InvalidLocalAccess(index))?;
                let value = frame.locals.load(index)?;
                self.stack.push(value)
            }

            OpCode::StoreLocal => {
                let index = self.read_u8()? as usize;
                let value = self.stack.pop()?;
                let frame = self
                    .call_stack
                    .last_mut()
                    .ok_or(VreError::InvalidLocalAccess(index))?;
                frame.locals.store(index, value.clone())?;
                // Record a state change for deterministic observers
                self.state_changes.push(StateChange::LocalStore { index, value });
                Ok(())
            }

            OpCode::Add => self.binary_numeric(|a, b| a + b),
            OpCode::Sub => self.binary_numeric(|a, b| a - b),
            OpCode::Mul => self.binary_numeric(|a, b| a * b),
            OpCode::Div => {
                let b = self.pop_number()?;
                if b == 0.0 {
                    return Err(VreError::DivisionByZero);
                }
                let a = self.pop_number()?;
                self.stack.push(Value::Number(a / b))
            }

            OpCode::Equal => self.compare(|a, b| a == b),
            OpCode::NotEqual => self.compare(|a, b| a != b),
            OpCode::Less => self.compare(|a, b| a < b),
            OpCode::Greater => self.compare(|a, b| a > b),

            OpCode::ExternalCall => {
                // External calls are gated: read capability id and argument count
                let cap_id = self.read_u8()?;
                let argc = self.read_u8()? as usize;
                // collect args (last pushed is last arg)
                let mut args = Vec::with_capacity(argc);
                for _ in 0..argc {
                    args.push(self.stack.pop()?);
                }
                args.reverse();

                // Suraksha check: fail-closed if capability not granted
                self.capabilities.check(cap_id)?;

                // Emit an ExternalCall request and halt execution so host can handle it.
                self.state_changes.push(StateChange::ExternalCallRequest { cap_id, args });
                self.halted = true;
                Ok(())
            }

            OpCode::Call => {
                // Call <target:u32>
                let target = self.read_u32()? as usize;
                // push a new call frame with return_ip set to current ip
                let frame = CallFrame { _return_ip: self.ip, locals: Locals::new(self._config.max_locals) };
                self.call_stack.push(frame);
                // jump to target
                if target >= self.instructions.len() {
                    return Err(VreError::InvalidJumpTarget(target));
                }
                self.ip = target;
                Ok(())
            }

            OpCode::Return => {
                // Pop current frame and resume at return_ip
                if self.call_stack.len() <= 1 {
                    // returning from root frame is a runtime fault
                    return Err(VreError::RuntimeFault);
                }
                let frame = self.call_stack.pop().unwrap();
                let ret = frame._return_ip;
                if ret > self.instructions.len() {
                    return Err(VreError::InvalidJumpTarget(ret));
                }
                self.ip = ret;
                Ok(())
            }

            _ => Err(VreError::RuntimeFault),
        }
    }

    fn pop_number(&mut self) -> VreResult<f64> {
        match self.stack.pop()? {
            Value::Number(n) => Ok(n),
            _ => Err(VreError::TypeMismatch),
        }
    }

    fn binary_numeric<F>(&mut self, op: F) -> VreResult<()>
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let b = self.pop_number()?;
        let a = self.pop_number()?;
        self.stack.push(Value::Number(op(a, b)))
    }

    fn compare<F>(&mut self, cmp: F) -> VreResult<()>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let b = self.pop_number()?;
        let a = self.pop_number()?;
        self.stack.push(Value::Bool(cmp(a, b)))
    }

    fn read_u8(&mut self) -> VreResult<u8> {
        if self.ip >= self.instructions.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let byte = self.instructions[self.ip];
        self.ip += 1;
        Ok(byte)
    }

    fn read_u32(&mut self) -> VreResult<u32> {
        if self.ip + 4 > self.instructions.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let v = u32::from_be_bytes([
            self.instructions[self.ip],
            self.instructions[self.ip + 1],
            self.instructions[self.ip + 2],
            self.instructions[self.ip + 3],
        ]);
        self.ip += 4;
        Ok(v)
    }
}
