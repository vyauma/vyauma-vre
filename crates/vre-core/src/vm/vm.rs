//! Virtual Machine Core
//!
//! Vyauma Virtual Machine execution engine.
//! Implements instruction semantics as per bytecode spec v0.1.

use crate::config::VreConfig;
use crate::error::{VreError, VreResult};
use crate::bytecode::opcode::OpCode;

use super::stack::Stack;
use super::memory::{Globals, Locals, ConstantPool, Heap, HeapObject, LeakReport};
use super::value::Value;

use crate::capability::capability::Capability;
use crate::capability::registry::CapabilityRegistry;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::fs::File;
use mio::{Events, Poll, Token, Interest};
use mio::net::{TcpStream, TcpListener};
use std::net::SocketAddr;

/// IO Resource handle
#[derive(Debug)]
pub enum Resource {
    File(File),
    TcpStream(TcpStream),
    TcpListener(TcpListener),
}

/// Call frame representing a single function invocation
pub type NativeFunction = fn(&mut Heap, Vec<Value>) -> Result<Value, String>;

#[derive(Debug)]
pub struct CallFrame {
    pub return_ip: usize,
    pub locals: Locals,
}

#[derive(Debug)]
pub struct ExceptionHandler {
    pub catch_ip: usize,
    pub call_depth: usize,
    pub stack_depth: usize,
}

use crate::scheduler::{Scheduler, Task};

/// Vyauma Virtual Machine
pub struct VirtualMachine {
    config: VreConfig,
    pub stack: Stack,
    #[allow(dead_code)] // reserved for LoadGlobal/StoreGlobal opcodes (post v0.1)
    globals: Globals,
    constants: ConstantPool,
    heap: Heap,

    instructions: Vec<u8>,
    ip: usize,

    call_stack: Vec<CallFrame>,
    halted: bool,

    scheduler: Scheduler,
    current_task_id: u64,
    capabilities: CapabilityRegistry,

    resources: HashMap<usize, Resource>,
    next_fd: usize,
    pub native_functions: Vec<NativeFunction>,
    poll: Poll,
    events: Events,
    
    exception_handlers: Vec<ExceptionHandler>,

    // JIT specific fields
    pub jit_call_counts: HashMap<usize, usize>,
    jit_cache: HashMap<usize, crate::jit::memory::JitMemory>,
}

impl VirtualMachine {
    pub fn heap(&self) -> &Heap { &self.heap }
    pub fn call_stack(&self) -> &[CallFrame] { &self.call_stack }

    /// Create a new VM instance.
    pub fn new(
        config: VreConfig,
        instructions: Vec<u8>,
        constants: Vec<Value>,
        native_imports: Vec<String>,
        capabilities: CapabilityRegistry,
    ) -> Result<Self, String> {
        let mut native_functions = Vec::new();
        for import in native_imports {
            if let Some(func) = config.ffi_functions.get(&import) {
                native_functions.push(*func);
            } else {
                return Err(format!("Unresolved FFI native import: {}", import));
            }
        }

        let poll = Poll::new().map_err(|e| format!("Failed to create mio Poll: {}", e))?;
        let _events = Events::with_capacity(1024);

        let max_stack_size = config.max_stack_size;

        Ok(VirtualMachine {
            config,
            instructions,
            constants: ConstantPool::new(constants),
            ip: 0,
            stack: Stack::new(max_stack_size),
            call_stack: Vec::new(),
            globals: Globals::new(0),
            heap: Heap::new(),
            scheduler: Scheduler::new(),
            current_task_id: 0, // 0 signifies the main synchronous context
            halted: false,
            capabilities,
            resources: HashMap::new(),
            next_fd: 0,
            native_functions,
            poll,
            events: Events::with_capacity(128),
            exception_handlers: Vec::new(),
            jit_call_counts: HashMap::new(),
            jit_cache: HashMap::new(),
        })
    }

    /// Execute bytecode until halt or error
    pub fn execute(&mut self) -> VreResult<()> {
        let mut next_gc_threshold = 1024;
        while !self.halted && self.ip < self.instructions.len() {
            if let Err(err) = self.step() {
                if self.exception_handlers.is_empty() {
                    return Err(err);
                } else {
                    let err_str = format!("{:?}", err);
                    self.execute_throw(Value::String(err_str))?;
                }
            }
            if self.heap.live_objects > next_gc_threshold {
                self.gc()?;
                // Double the threshold if we couldn't free enough to get under it
                if self.heap.live_objects > next_gc_threshold / 2 {
                    next_gc_threshold *= 2;
                }
            }
        }
        Ok(())
    }

    pub fn execute_throw(&mut self, err_val: Value) -> VreResult<()> {
        if let Some(handler) = self.exception_handlers.pop() {
            self.call_stack.truncate(handler.call_depth);
            self.stack.truncate(handler.stack_depth);
            self.stack.push(err_val)?;
            self.ip = handler.catch_ip;
            Ok(())
        } else {
            Err(VreError::RuntimeFault)
        }
    }

    /// Suspend the currently executing task
    fn yield_current_task(&mut self) {
        let task = Task {
            id: self.current_task_id,
            ip: self.ip,
            stack: std::mem::replace(&mut self.stack, Stack::new(0)),
            call_stack: std::mem::take(&mut self.call_stack),
            state: crate::scheduler::TaskState::Ready,
        };
        self.scheduler.yield_task(task);
    }

    /// Resume the next task from the scheduler
    fn resume_next_task(&mut self) -> bool {
        if let Some(mut task) = self.scheduler.pop_next() {
            self.current_task_id = task.id;
            self.ip = task.ip;
            self.stack = std::mem::replace(&mut task.stack, Stack::new(0));
            self.call_stack = std::mem::take(&mut task.call_stack);
            true
        } else {
            self.current_task_id = 0;
            false
        }
    }

    /// Execute a single instruction (public for tests)
    pub fn step(&mut self) -> VreResult<()> {
        let op = self.read_u8()?;
        self.execute_instruction(op)
    }

    fn execute_instruction(&mut self, op: u8) -> VreResult<()> {
        let opcode = OpCode::from_u8(op)
            .ok_or(VreError::InvalidOpcode(op))?;

        match opcode {
            // ── System ─────────────────────────────────────────────────────
            OpCode::Halt => {
                self.halted = true;
                Ok(())
            }

            OpCode::Nop => Ok(()),

            // ── Stack ──────────────────────────────────────────────────────
            OpCode::Push => {
                // operand: u16 constant pool index (big-endian)
                let index = self.read_u16()? as usize;
                let value = self.constants.get(index)?;
                self.stack.push(value)
            }

            OpCode::Pop => {
                self.stack.pop()?;
                Ok(())
            }

            OpCode::Dup => self.stack.dup(),

            // ── Local variables ────────────────────────────────────────────
            OpCode::LoadLocal | OpCode::LoadLocalI32 | OpCode::LoadLocalI64 | OpCode::LoadLocalF32 | OpCode::LoadLocalF64 | OpCode::LoadLocalStr => {
                let index = self.read_u16()? as usize;
                let frame = self.current_frame()?;
                let value = frame.locals.load(index)?;
                self.stack.push(value)
            }

            OpCode::StoreLocal => {
                let index = self.read_u16()? as usize;
                let value = self.stack.pop()?;
                let frame = self.current_frame_mut()?;
                frame.locals.store(index, value)
            }

            
            // ── Arithmetic Int32 ──────────────────────────────────────────
            OpCode::AddI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Int32(a + b)) }
            OpCode::SubI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Int32(a - b)) }
            OpCode::MulI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Int32(a * b)) }
            OpCode::DivI32 => {
                let (a, b) = self.pop_two_i32()?;
                if b == 0 { return Err(VreError::DivisionByZero); }
                self.stack.push(Value::Int32(a / b))
            }
            OpCode::ModI32 => {
                let (a, b) = self.pop_two_i32()?;
                if b == 0 { return Err(VreError::DivisionByZero); }
                self.stack.push(Value::Int32(a % b))
            }
            OpCode::NegI32 => {
                let a = self.pop_i32()?;
                self.stack.push(Value::Int32(-a))
            }

            // ── Arithmetic Int64 ──────────────────────────────────────────
            OpCode::AddI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Int64(a + b)) }
            OpCode::SubI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Int64(a - b)) }
            OpCode::MulI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Int64(a * b)) }
            OpCode::DivI64 => {
                let (a, b) = self.pop_two_i64()?;
                if b == 0 { return Err(VreError::DivisionByZero); }
                self.stack.push(Value::Int64(a / b))
            }
            OpCode::ModI64 => {
                let (a, b) = self.pop_two_i64()?;
                if b == 0 { return Err(VreError::DivisionByZero); }
                self.stack.push(Value::Int64(a % b))
            }
            OpCode::NegI64 => {
                let a = self.pop_i64()?;
                self.stack.push(Value::Int64(-a))
            }

            // ── Arithmetic Float32 ────────────────────────────────────────
            OpCode::AddF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Float32(a + b)) }
            OpCode::SubF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Float32(a - b)) }
            OpCode::MulF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Float32(a * b)) }
            OpCode::DivF32 => { let (a, b) = self.pop_two_f32()?; if b == 0.0 { return Err(VreError::DivisionByZero); } self.stack.push(Value::Float32(a / b)) }
            OpCode::ModF32 => { let (a, b) = self.pop_two_f32()?; if b == 0.0 { return Err(VreError::DivisionByZero); } self.stack.push(Value::Float32(a % b)) }
            OpCode::NegF32 => { let a = self.pop_f32()?; self.stack.push(Value::Float32(-a)) }

            // ── Arithmetic Float64 ────────────────────────────────────────
            OpCode::AddF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Float64(a + b)) }
            OpCode::SubF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Float64(a - b)) }
            OpCode::MulF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Float64(a * b)) }
            OpCode::DivF64 => { let (a, b) = self.pop_two_f64()?; if b == 0.0 { return Err(VreError::DivisionByZero); } self.stack.push(Value::Float64(a / b)) }
            OpCode::ModF64 => { let (a, b) = self.pop_two_f64()?; if b == 0.0 { return Err(VreError::DivisionByZero); } self.stack.push(Value::Float64(a % b)) }
            OpCode::NegF64 => { let a = self.pop_f64()?; self.stack.push(Value::Float64(-a)) }

            // ── Comparison Int32 ──────────────────────────────────────────
            OpCode::EqualI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Bool(a == b)) }
            OpCode::NotEqualI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Bool(a != b)) }
            OpCode::LessI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Bool(a < b)) }
            OpCode::LessEqualI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Bool(a <= b)) }
            OpCode::GreaterI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Bool(a > b)) }
            OpCode::GreaterEqualI32 => { let (a, b) = self.pop_two_i32()?; self.stack.push(Value::Bool(a >= b)) }

            // ── Comparison Int64 ──────────────────────────────────────────
            OpCode::EqualI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Bool(a == b)) }
            OpCode::NotEqualI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Bool(a != b)) }
            OpCode::LessI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Bool(a < b)) }
            OpCode::LessEqualI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Bool(a <= b)) }
            OpCode::GreaterI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Bool(a > b)) }
            OpCode::GreaterEqualI64 => { let (a, b) = self.pop_two_i64()?; self.stack.push(Value::Bool(a >= b)) }

            // ── Comparison Float32 ────────────────────────────────────────
            OpCode::EqualF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Bool(a == b)) }
            OpCode::NotEqualF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Bool(a != b)) }
            OpCode::LessF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Bool(a < b)) }
            OpCode::LessEqualF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Bool(a <= b)) }
            OpCode::GreaterF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Bool(a > b)) }
            OpCode::GreaterEqualF32 => { let (a, b) = self.pop_two_f32()?; self.stack.push(Value::Bool(a >= b)) }

            // ── Comparison Float64 ────────────────────────────────────────
            OpCode::EqualF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Bool(a == b)) }
            OpCode::NotEqualF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Bool(a != b)) }
            OpCode::LessF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Bool(a < b)) }
            OpCode::LessEqualF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Bool(a <= b)) }
            OpCode::GreaterF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Bool(a > b)) }
            OpCode::GreaterEqualF64 => { let (a, b) = self.pop_two_f64()?; self.stack.push(Value::Bool(a >= b)) }

            // ── Comparison String ─────────────────────────────────────────
            OpCode::EqualStr => { let (a, b) = self.pop_two_string()?; self.stack.push(Value::Bool(a == b)) }
            OpCode::NotEqualStr => { let (a, b) = self.pop_two_string()?; self.stack.push(Value::Bool(a != b)) }
            OpCode::AddStr => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                let a_str = match a {
                    Value::String(s) => s,
                    Value::Float64(n) => n.to_string(),
                    Value::Float32(n) => n.to_string(),
                    Value::Int64(n) => n.to_string(),
                    Value::Int32(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => format!("{:?}", a),
                };
                let b_str = match b {
                    Value::String(s) => s,
                    Value::Float64(n) => n.to_string(),
                    Value::Float32(n) => n.to_string(),
                    Value::Int64(n) => n.to_string(),
                    Value::Int32(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => format!("{:?}", b),
                };
                self.stack.push(Value::String(format!("{}{}", a_str, b_str)))
            }
            OpCode::AndBool => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                if let (Value::Bool(ba), Value::Bool(bb)) = (a, b) {
                    self.stack.push(Value::Bool(ba && bb))
                } else {
                    Err(VreError::TypeMismatch)
                }
            }
            OpCode::OrBool => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                if let (Value::Bool(ba), Value::Bool(bb)) = (a, b) {
                    self.stack.push(Value::Bool(ba || bb))
                } else {
                    Err(VreError::TypeMismatch)
                }
            }


            // ── Control flow ───────────────────────────────────────────────
            OpCode::Jump => {
                let target = self.read_u32()? as usize;
                self.jump(target)?;
                Ok(())
            }

            OpCode::JumpIf => {
                let target = self.read_u32()? as usize;
                let condition = self.pop_bool()?;
                if condition {
                    self.jump(target)?;
                }
                Ok(())
            }

            OpCode::Call => {
                let target = self.read_u32()? as usize;
                let local_count = self.read_u16()? as usize;

                if self.call_stack.len() >= self.config.max_call_depth {
                    return Err(VreError::StackOverflow);
                }

                if target >= self.instructions.len() {
                    return Err(VreError::InvalidJumpTarget(target));
                }

                // JIT Tiered Execution: Track calls and compile/execute if hot
                let count = self.jit_call_counts.entry(target).or_insert(0);
                *count += 1;

                if *count > 50 {
                    if !self.jit_cache.contains_key(&target) {
                        let mut end = target;
                        while end < self.instructions.len() && self.instructions[end] != OpCode::Return as u8 {
                            // Advance by instruction length
                            let op = self.instructions[end];
                            if op == OpCode::Push as u8 { end += 3; }
                            else if op == OpCode::LoadLocal as u8 || op == OpCode::StoreLocal as u8 { end += 2; }
                            else if op == OpCode::Jump as u8 || op == OpCode::JumpIf as u8 { end += 5; }
                            else if op == OpCode::Call as u8 { end += 7; } 
                            else if op == OpCode::CallNative as u8 { end += 5; } // u32 native index
                            else { end += 1; }
                        }
                        if end < self.instructions.len() {
                            end += 1; // Include Return
                            let body = &self.instructions[target..end];
                            println!("=> [JIT] Compiling function at IP {} into native x86_64 machine code...", target);
                            let mut compiler = crate::jit::compiler::JitCompiler::new();
                            let mem = compiler.compile(body);
                            self.jit_cache.insert(target, mem);
                        }
                    }

                    if let Some(jit_mem) = self.jit_cache.get(&target) {
                        println!("=> [JIT] Executing machine code natively at {:p}!", jit_mem.get_ptr());
                        let func_ptr: extern "C" fn(*mut VirtualMachine) = unsafe { std::mem::transmute(jit_mem.get_ptr()) };
                        func_ptr(self as *mut _);
                        // The JIT executed the function natively and returned.
                        return Ok(());
                    }
                }

                let frame = CallFrame {
                    return_ip: self.ip,
                    locals: Locals::new(local_count),
                };
                self.call_stack.push(frame);
                self.ip = target;
                Ok(())
            }

            OpCode::Return => {
                match self.call_stack.pop() {
                    Some(frame) => {
                        self.ip = frame.return_ip;
                        Ok(())
                    }
                    None => {
                        // Return from top-level — task complete!
                        if self.resume_next_task() {
                            // Another task took over
                            Ok(())
                        } else {
                            // No more tasks, halt the VM
                            self.halted = true;
                            Ok(())
                        }
                    }
                }
            }

            OpCode::Spawn => {
                let target = self.read_u32()? as usize;
                // Create task and push to ready queue
                let task_id = self.scheduler.spawn(target, self.config.max_stack_size);
                self.stack.push(Value::Int64(task_id as i64))?;
                Ok(())
            }

            OpCode::Yield => {
                self.yield_current_task();
                if !self.resume_next_task() {
                    // No other tasks to resume, just continue the current one
                    self.resume_next_task(); // Pop the task we just yielded!
                }
                Ok(())
            }

            OpCode::Await => {
                // Simplified Await: Just act as a yield for now
                // In a robust implementation, this would track the target task ID 
                // and block the current task until the target completes.
                let _target_id = self.stack.pop()?;
                self.yield_current_task();
                if !self.resume_next_task() {
                    self.resume_next_task();
                }
                // Push placeholder result
                self.stack.push(Value::Null)?;
                Ok(())
            }

            OpCode::CallNative => {
                let native_idx = self.read_u16()? as usize;
                let arg_count = self.read_u8()? as usize;
                // Ignore the 3 bytes of padding from the 6-byte Call operand space
                self.ip += 3;

                let mut args = Vec::new();
                for _ in 0..arg_count {
                    args.push(self.stack.pop()?);
                }
                args.reverse();

                let func = self.native_functions[native_idx];
                let result = match func(&mut self.heap, args) {
                    Ok(v) => v,
                    Err(e) => return Err(VreError::NativeFunctionError(e)),
                };
                self.stack.push(result)
            }

            OpCode::TryStart => {
                let catch_offset = self.read_u32()? as usize;
                let handler = ExceptionHandler {
                    catch_ip: catch_offset,
                    call_depth: self.call_stack.len(),
                    stack_depth: self.stack.size(),
                };
                self.exception_handlers.push(handler);
                Ok(())
            }

            OpCode::TryEnd => {
                self.exception_handlers.pop();
                Ok(())
            }

            OpCode::Throw => {
                let err_val = self.stack.pop()?;
                self.execute_throw(err_val)
            }

            // ── Heap and Objects ───────────────────────────────────────────
            OpCode::NewArray => {
                let size = self.pop_number()? as usize;
                let arr = vec![Value::Null; size];
                let id = self.heap.allocate(HeapObject::Array(arr));
                self.stack.push(Value::Reference(id))
            }

            OpCode::LoadElement => {
                let index_val = self.stack.pop()?;
                let ref_val = self.stack.pop()?;
                if let Value::Reference(id) = ref_val {
                    let obj = self.heap.get(id)?;
                    match obj {
                        HeapObject::Array(arr) => {
                            if let Value::Float64(n) = index_val {
                                let index = n as usize;
                                if index >= arr.len() {
                                    println!("StoreElement array bound fault! Index: {}, len: {}", index, arr.len());
                                    return Err(VreError::RuntimeFault);
                                }
                                self.stack.push(arr[index].clone())
                            } else {
                                return Err(VreError::TypeMismatch);
                            }
                        }
                        HeapObject::Struct(fields) => {
                            if let Value::String(s) = index_val {
                                let val = fields.get(&s).cloned().unwrap_or(Value::Null);
                                self.stack.push(val)
                            } else {
                                return Err(VreError::TypeMismatch);
                            }
                        }
                        _ => return Err(VreError::TypeMismatch),
                    }
                } else {
                    return Err(VreError::TypeMismatch);
                }
            }

            OpCode::StoreElement => {
                let val = self.stack.pop()?;
                let index_val = self.stack.pop()?;
                let ref_val = self.stack.pop()?;
                if let Value::Reference(id) = ref_val {
                    let obj = self.heap.get_mut(id)?;
                    match obj {
                        HeapObject::Array(arr) => {
                            if let Value::Float64(n) = index_val {
                                let index = n as usize;
                                if index >= arr.len() {
                                    println!("StoreElement array bound fault! Index: {}, len: {}", index, arr.len());
                                    return Err(VreError::RuntimeFault);
                                }
                                arr[index] = val;
                                Ok(())
                            } else {
                                return Err(VreError::TypeMismatch);
                            }
                        }
                        HeapObject::Struct(fields) => {
                            if let Value::String(s) = index_val {
                                fields.insert(s, val);
                                Ok(())
                            } else {
                                return Err(VreError::TypeMismatch);
                            }
                        }
                        _ => Err(VreError::TypeMismatch),
                    }
                } else {
                    Err(VreError::TypeMismatch)
                }
            }

            OpCode::NewStruct => {
                let count = self.pop_number()? as usize;
                let mut fields = std::collections::HashMap::new();
                for _ in 0..count {
                    let val = self.stack.pop()?;
                    let key = match self.stack.pop()? {
                        Value::String(s) => s,
                        _ => return Err(VreError::TypeMismatch),
                    };
                    fields.insert(key, val);
                }
                let id = self.heap.allocate(HeapObject::Struct(fields));
                self.stack.push(Value::Reference(id))
            }

            OpCode::LoadProperty => {
                let name_idx = self.read_u16()? as usize;
                let name = match self.constants.get(name_idx)? {
                    Value::String(s) => s,
                    _ => return Err(VreError::TypeMismatch),
                };
                let ref_val = self.stack.pop()?;
                if let Value::Reference(id) = ref_val {
                    let obj = self.heap.get(id)?;
                    match obj {
                        HeapObject::Struct(fields) => {
                            let val = fields.get(&name).cloned().unwrap_or(Value::Null);
                            self.stack.push(val)
                        }
                        _ => return Err(VreError::TypeMismatch),
                    }
                } else {
                    return Err(VreError::TypeMismatch);
                }
            }

            OpCode::StoreProperty => {
                let name_idx = self.read_u16()? as usize;
                let name = match self.constants.get(name_idx)? {
                    Value::String(s) => s,
                    _ => return Err(VreError::TypeMismatch),
                };
                let val = self.stack.pop()?;
                let ref_val = self.stack.pop()?;
                if let Value::Reference(id) = ref_val {
                    let obj = self.heap.get_mut(id)?;
                    match obj {
                        HeapObject::Struct(fields) => {
                            fields.insert(name, val);
                            Ok(())
                        }
                        _ => return Err(VreError::TypeMismatch),
                    }
                } else {
                    return Err(VreError::TypeMismatch);
                }
            }

            OpCode::Syscall => {
                let id = self.read_u8()?;
                match id {
                    0x01 => {
                        // Print
                        let cap = Capability::new("io.write");
                        self.capabilities.require(&cap)?;
                        let val = self.stack.pop()?;
                        println!("{:?}", val);
                        Ok(())
                    }
                    0x02 => {
                        // ReadChar
                        let cap = Capability::new("io.read");
                        self.capabilities.require(&cap)?;
                        let mut buf = [0u8; 1];
                        let bytes_read = std::io::stdin().read(&mut buf)?;
                        if bytes_read == 0 {
                            self.stack.push(Value::Float64(-1.0))
                        } else {
                            self.stack.push(Value::Float64(buf[0] as f64))
                        }
                    }
                    0x03 => {
                        // read(fd, buffer_ref) -> bytes read
                        let buffer_ref = self.stack.pop()?;
                        let fd = self.pop_number()? as usize;
                        
                        if let Value::Reference(id) = buffer_ref {
                            // We need to borrow the resource mutably to read from it.
                            // To avoid borrow checker issues with `self`, we take the resource out.
                            if let Some(mut resource) = self.resources.remove(&fd) {
                                let mut buf = vec![0u8; 1024]; // Temporary buffer
                                let res = match &mut resource {
                                    Resource::File(f) => f.read(&mut buf),
                                    Resource::TcpStream(s) => s.read(&mut buf),
                                    _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid resource for read")),
                                };

                                match res {
                                    Ok(n) => {
                                        // Update array
                                        let obj = self.heap.get_mut(id)?;
                                        if let HeapObject::Array(arr) = obj {
                                            for i in 0..n {
                                                if i < arr.len() {
                                                    arr[i] = Value::Float64(buf[i] as f64);
                                                }
                                            }
                                        }
                                        self.stack.push(Value::Float64(n as f64))?;
                                    }
                                    Err(_) => { self.stack.push(Value::Float64(-1.0))?; }
                                }
                                self.resources.insert(fd, resource);
                            } else {
                                self.stack.push(Value::Float64(-1.0))?;
                            }
                        } else {
                            return Err(VreError::TypeMismatch);
                        }
                        Ok(())
                    }
                    0x04 => {
                        // write(fd, buffer_ref) -> bytes written
                        let buffer_ref = self.stack.pop()?;
                        let fd = self.pop_number()? as usize;

                        if let Value::Reference(id) = buffer_ref {
                            let obj = self.heap.get(id)?;
                            if let HeapObject::Array(arr) = obj {
                                let mut buf = Vec::new();
                                for val in arr {
                                    if let Value::Float64(n) = val {
                                        buf.push(*n as u8);
                                    }
                                }
                                if let Some(mut resource) = self.resources.remove(&fd) {
                                    let res = match &mut resource {
                                        Resource::File(f) => f.write(&buf),
                                        Resource::TcpStream(s) => s.write(&buf),
                                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid resource for write")),
                                    };
                                    match res {
                                        Ok(n) => { self.stack.push(Value::Float64(n as f64))?; }
                                        Err(_) => { self.stack.push(Value::Float64(-1.0))?; }
                                    }
                                    self.resources.insert(fd, resource);
                                } else {
                                    self.stack.push(Value::Float64(-1.0))?;
                                }
                            } else {
                                return Err(VreError::TypeMismatch);
                            }
                        } else {
                            return Err(VreError::TypeMismatch);
                        }
                        Ok(())
                    }
                    0x05 => {
                        // close(fd)
                        let fd = self.pop_number()? as usize;
                        if self.resources.remove(&fd).is_some() {
                            self.stack.push(Value::Float64(0.0))?;
                        } else {
                            self.stack.push(Value::Float64(-1.0))?;
                        }
                        Ok(())
                    }
                    0x06 => {
                        // sleep(ms)
                        let ms = self.pop_number()? as u64;
                        crate::pal::get_pal().sleep_ms(ms);
                        self.stack.push(Value::Float64(0.0))?;
                        Ok(())
                    }
                    0x07 => {
                        // gc()
                        let objects_before = self.heap.live_objects;
                        self.gc()?;
                        let objects_after = self.heap.live_objects;
                        let reclaimed = objects_before.saturating_sub(objects_after);
                        self.stack.push(Value::Float64(reclaimed as f64))?;
                        Ok(())
                    }
                    0x10 => {
                        // file_open(filename_string) -> fd
                        let cap = Capability::new("fs.read");
                        self.capabilities.require(&cap)?;
                        
                        let filename_val = self.stack.pop()?;
                        if let Value::String(filename) = filename_val {
                            match crate::pal::get_pal().open_file(std::path::Path::new(&filename)) {
                                Ok(file) => {
                                    let fd = self.next_fd;
                                    self.next_fd += 1;
                                    self.resources.insert(fd, Resource::File(file));
                                    self.stack.push(Value::Float64(fd as f64))?;
                                }
                                Err(_) => { self.stack.push(Value::Float64(-1.0))?; }
                            }
                        } else {
                            return Err(VreError::TypeMismatch);
                        }
                        Ok(())
                    }
                    0x20 => {
                        // net_connect(host_string, port) -> fd
                        let cap = Capability::new("net.connect");
                        self.capabilities.require(&cap)?;
                        
                        let port = self.pop_number()? as u16;
                        let host_val = self.stack.pop()?;
                        if let Value::String(host) = host_val {
                            let addr = format!("{}:{}", host, port);
                            match crate::pal::get_pal().tcp_connect(&addr) {
                                Ok(std_stream) => {
                                    std_stream.set_nonblocking(true).map_err(|_| VreError::RuntimeFault)?;
                                    let mut stream = mio::net::TcpStream::from_std(std_stream);
                                    let fd = self.next_fd;
                                    self.next_fd += 1;
                                    if let Err(_) = self.poll.registry().register(&mut stream, Token(fd), Interest::READABLE | Interest::WRITABLE) {
                                        return Err(VreError::RuntimeFault);
                                    }
                                    self.resources.insert(fd, Resource::TcpStream(stream));
                                    self.stack.push(Value::Float64(fd as f64))?;
                                }
                                Err(_) => { self.stack.push(Value::Float64(-1.0))?; }
                            }
                        } else {
                            return Err(VreError::TypeMismatch);
                        }
                        Ok(())
                    }
                    0x21 => {
                        // net_listen(port) -> fd
                        let cap = Capability::new("net.listen");
                        self.capabilities.require(&cap)?;

                        let port = self.pop_number()? as u16;
                        let addr = format!("127.0.0.1:{}", port);
                        match crate::pal::get_pal().tcp_bind(&addr) {
                            Ok(std_listener) => {
                                std_listener.set_nonblocking(true).map_err(|_| VreError::RuntimeFault)?;
                                let mut listener = mio::net::TcpListener::from_std(std_listener);
                                let fd = self.next_fd;
                                self.next_fd += 1;
                                if let Err(_) = self.poll.registry().register(&mut listener, Token(fd), Interest::READABLE) {
                                    return Err(VreError::RuntimeFault);
                                }
                                self.resources.insert(fd, Resource::TcpListener(listener));
                                self.stack.push(Value::Float64(fd as f64))?;
                            }
                            Err(_) => { self.stack.push(Value::Float64(-1.0))?; }
                        }
                        Ok(())
                    }
                    0x22 => {
                        // net_accept(server_fd) -> fd
                        let server_fd = self.pop_number()? as usize;
                        if let Some(Resource::TcpListener(listener)) = self.resources.get(&server_fd) {
                            match listener.accept() {
                                Ok((mut stream, _)) => {
                                    let fd = self.next_fd;
                                    self.next_fd += 1;
                                    if let Err(_) = self.poll.registry().register(&mut stream, Token(fd), Interest::READABLE | Interest::WRITABLE) {
                                        return Err(VreError::RuntimeFault);
                                    }
                                    self.resources.insert(fd, Resource::TcpStream(stream));
                                    self.stack.push(Value::Float64(fd as f64))?;
                                }
                                Err(_) => { self.stack.push(Value::Float64(-1.0))?; }
                            }
                        } else {
                            self.stack.push(Value::Float64(-1.0))?;
                        }
                        Ok(())
                    }
                    0x23 => {
                        // net_set_nonblocking(fd, is_nonblocking)
                        let _is_nonblocking = self.pop_number()? != 0.0;
                        let fd = self.pop_number()? as usize;
                        if let Some(resource) = self.resources.get_mut(&fd) {
                            match resource {
                                Resource::TcpListener(_) => {
                                    // mio sockets are always non-blocking
                                }
                                Resource::TcpStream(_) => {
                                    // mio sockets are always non-blocking
                                }
                                _ => {}
                            }
                        }
                        self.stack.push(Value::Float64(0.0))?;
                        Ok(())
                    }
                    0x24 => {
                        // net_poll() -> Array of fds
                        let cap = Capability::new("net.listen"); // Just a rough capability check for networking
                        self.capabilities.require(&cap)?;

                        match self.poll.poll(&mut self.events, None) {
                            Ok(_) => {
                                let mut fds = Vec::new();
                                for event in self.events.iter() {
                                    fds.push(Value::Float64(event.token().0 as f64));
                                }
                                let array_obj = HeapObject::Array(fds);
                                let ref_id = self.heap.allocate(array_obj);
                                self.stack.push(Value::Reference(ref_id))?;
                            }
                            Err(_) => {
                                return Err(VreError::RuntimeFault);
                            }
                        }
                        Ok(())
                    }
                    0x30 => {
                        // string_to_bytes(string) -> array_ref
                        let val = self.stack.pop()?;
                        if let Value::String(s) = val {
                            let bytes = s.into_bytes();
                            let arr = bytes.into_iter().map(|b| Value::Float64(b as f64)).collect();
                            let id = self.heap.allocate(HeapObject::Array(arr));
                            self.stack.push(Value::Reference(id))?;
                        } else {
                            return Err(VreError::TypeMismatch);
                        }
                        Ok(())
                    }
                    0x31 => {
                        // bytes_to_string(array_ref) -> string
                        let val = self.stack.pop()?;
                        if let Value::Reference(id) = val {
                            let obj = self.heap.get(id)?;
                            if let HeapObject::Array(arr) = obj {
                                let mut bytes = Vec::new();
                                for item in arr {
                                    if let Value::Float64(n) = item {
                                        bytes.push(*n as u8);
                                    } else {
                                        return Err(VreError::TypeMismatch);
                                    }
                                }
                                let s = String::from_utf8_lossy(&bytes).into_owned();
                                self.stack.push(Value::String(s))?;
                            } else {
                                return Err(VreError::TypeMismatch);
                            }
                        } else {
                            return Err(VreError::TypeMismatch);
                        }
                        Ok(())
                    }
                    _ => Err(VreError::RuntimeFault),
                }
            }
        }
    }

    pub fn gc(&mut self) -> VreResult<()> {
        let capacity = self.heap.objects.len();
        if capacity == 0 {
            return Ok(());
        }

        let mut marked = vec![false; capacity];
        let mut worklist = Vec::new();

        // 1. Trace roots (Current task)
        for val in self.stack.values() {
            if let Value::Reference(id) = val {
                worklist.push(*id);
            }
        }

        for frame in &self.call_stack {
            for val in frame.locals.values() {
                if let Value::Reference(id) = val {
                    worklist.push(*id);
                }
            }
        }

        // 1.5. Trace roots (Scheduled tasks)
        for task in self.scheduler.iter_tasks() {
            for val in task.stack.values() {
                if let Value::Reference(id) = val {
                    worklist.push(*id);
                }
            }
            for frame in &task.call_stack {
                for val in frame.locals.values() {
                    if let Value::Reference(id) = val {
                        worklist.push(*id);
                    }
                }
            }
        }

        // Trace globals
        for val in self.globals.values() {
            if let Value::Reference(id) = val {
                worklist.push(*id);
            }
        }
        // 2. Mark
        while let Some(id) = worklist.pop() {
            let idx = id as usize;
            if idx < capacity && !marked[idx] {
                marked[idx] = true;
                if let Ok(obj) = self.heap.get(id) {
                    match obj {
                        HeapObject::Array(arr) => {
                            for val in arr {
                                if let Value::Reference(child) = val {
                                    worklist.push(*child);
                                }
                            }
                        }
                        HeapObject::Struct(fields) => {
                            for val in fields.values() {
                                if let Value::Reference(child) = val {
                                    worklist.push(*child);
                                }
                            }
                        }
                        HeapObject::String(_) | HeapObject::Function(_) => {}
                    }
                }
            }
        }

        // 3. Sweep
        self.heap.sweep(&marked);
        Ok(())
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    /// Read next byte from instruction stream
    fn read_u8(&mut self) -> VreResult<u8> {
        if self.ip >= self.instructions.len() {
            return Err(VreError::BytecodeTooShort);
        }
        let byte = self.instructions[self.ip];
        self.ip += 1;
        Ok(byte)
    }

    /// Read next two bytes as a big-endian u16
    fn read_u16(&mut self) -> VreResult<u16> {
        let hi = self.read_u8()? as u16;
        let lo = self.read_u8()? as u16;
        Ok((hi << 8) | lo)
    }

    /// Read next four bytes as a big-endian u32
    fn read_u32(&mut self) -> VreResult<u32> {
        let b0 = self.read_u8()? as u32;
        let b1 = self.read_u8()? as u32;
        let b2 = self.read_u8()? as u32;
        let b3 = self.read_u8()? as u32;
        Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    }

    /// Validate and set instruction pointer
    fn jump(&mut self, target: usize) -> VreResult<()> {
        if target >= self.instructions.len() {
            return Err(VreError::InvalidJumpTarget(target));
        }
        self.ip = target;
        Ok(())
    }

    /// Pop a number from the stack, returning TypeMismatch on wrong type
    fn pop_number(&mut self) -> VreResult<f64> {
        self.stack.pop()?.as_f64()
    }

    /// Pop a bool from the stack, returning TypeMismatch on wrong type
    fn pop_bool(&mut self) -> VreResult<bool> {
        match self.stack.pop()? {
            Value::Bool(b) => Ok(b),
            _ => Err(VreError::TypeMismatch),
        }
    }

    /// Pop two numbers (a, b) where `a` was pushed first, `b` second

    // ── Typed Stack Helpers ───────────────────────────────────────────────

    fn pop_two_i32(&mut self) -> VreResult<(i32, i32)> {
        let b = self.pop_i32()?;
        let a = self.pop_i32()?;
        Ok((a, b))
    }

    fn pop_two_i64(&mut self) -> VreResult<(i64, i64)> {
        let b = self.pop_i64()?;
        let a = self.pop_i64()?;
        Ok((a, b))
    }

    fn pop_two_f32(&mut self) -> VreResult<(f32, f32)> {
        let b = self.pop_f32()?;
        let a = self.pop_f32()?;
        Ok((a, b))
    }

    fn pop_two_f64(&mut self) -> VreResult<(f64, f64)> {
        let b = self.pop_f64()?;
        let a = self.pop_f64()?;
        Ok((a, b))
    }

    fn pop_two_string(&mut self) -> VreResult<(String, String)> {
        let b = self.pop_string()?;
        let a = self.pop_string()?;
        Ok((a, b))
    }

    fn pop_i32(&mut self) -> VreResult<i32> {
        match self.stack.pop()? {
            Value::Int32(v) => Ok(v),
            _ => Err(VreError::TypeMismatch),
        }
    }

    fn pop_i64(&mut self) -> VreResult<i64> {
        match self.stack.pop()? {
            Value::Int64(v) => Ok(v),
            _ => Err(VreError::TypeMismatch),
        }
    }

    fn pop_f32(&mut self) -> VreResult<f32> {
        match self.stack.pop()? {
            Value::Float32(v) => Ok(v),
            _ => Err(VreError::TypeMismatch),
        }
    }

    fn pop_f64(&mut self) -> VreResult<f64> {
        match self.stack.pop()? {
            Value::Float64(v) => Ok(v),
            _ => Err(VreError::TypeMismatch),
        }
    }

    fn pop_string(&mut self) -> VreResult<String> {
        match self.stack.pop()? {
            Value::String(s) => Ok(s),
            _ => Err(VreError::TypeMismatch),
        }
    }

    fn pop_two_numbers(&mut self) -> VreResult<(f64, f64)> {
        let b = self.pop_number()?;
        let a = self.pop_number()?;
        Ok((a, b))
    }

    /// Get a shared reference to the current call frame
    pub fn current_frame(&self) -> VreResult<&CallFrame> {
        self.call_stack.last().ok_or(VreError::InvalidStackAccess)
    }

    pub fn ip(&self) -> usize { self.ip }
    pub fn instructions(&self) -> &[u8] { &self.instructions }
    pub fn stack(&self) -> &Stack { &self.stack }
    pub fn halted(&self) -> bool { self.halted }
    pub fn constants(&self) -> &ConstantPool { &self.constants }

    /// Get a mutable reference to the current call frame
    pub fn current_frame_mut(&mut self) -> VreResult<&mut CallFrame> {
        self.call_stack.last_mut().ok_or(VreError::InvalidStackAccess)
    }

    /// Peek the top value of the stack
    pub fn peek_stack(&self) -> VreResult<&Value> {
        self.stack.peek()
    }

    /// Generate a heap leak report after execution completes.
    /// Returns a structured summary of all live (un-freed) objects.
    pub fn leak_report(&self) -> LeakReport {
        self.heap.leak_report()
    }

    /// Quick heap stats: (live_objects, total_allocations)
    pub fn heap_stats(&self) -> (usize, usize) {
        (self.heap.live_objects, self.heap.total_allocations)
    }
}
