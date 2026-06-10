use crate::jit::memory::JitMemory;
use crate::bytecode::opcode::OpCode;
use crate::vm::vm::VirtualMachine;

type HandlerFn = extern "C" fn(*mut VirtualMachine);

/// A lightweight Call-Threaded Just-In-Time Compiler for x86_64.
/// It translates Vyauma bytecodes into native machine code `call` instructions.
pub struct JitCompiler {
    code: Vec<u8>,
}

impl JitCompiler {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    /// Emits the JIT prologue.
    /// Preserves the non-volatile register `r12` and moves the `VirtualMachine` pointer into it.
    #[cfg(target_arch = "x86_64")]
    pub fn emit_prologue(&mut self) {
        self.code.extend_from_slice(&[0x41, 0x54]); // push r12
        if cfg!(target_os = "windows") {
            self.code.extend_from_slice(&[0x49, 0x89, 0xCC]); // mov r12, rcx
        } else {
            self.code.extend_from_slice(&[0x49, 0x89, 0xFC]); // mov r12, rdi
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn emit_prologue(&mut self) {
        // sub sp, sp, #16
        // str x19, [sp]
        self.code.extend_from_slice(&[0xFF, 0x43, 0x00, 0xD1]);
        self.code.extend_from_slice(&[0xF3, 0x03, 0x00, 0xF9]);
        // mov x19, x0 (save VM ptr in x19)
        self.code.extend_from_slice(&[0xF3, 0x03, 0x00, 0xAA]);
    }

    /// Emits a call to a Rust native handler function.
    #[cfg(target_arch = "x86_64")]
    pub fn emit_call(&mut self, func: HandlerFn) {
        if cfg!(target_os = "windows") {
            self.code.extend_from_slice(&[0x4C, 0x89, 0xE1]); // mov rcx, r12
            self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]); // sub rsp, 40
        } else {
            self.code.extend_from_slice(&[0x4C, 0x89, 0xE7]); // mov rdi, r12
            self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x08]); // sub rsp, 8
        }

        let addr = func as usize;
        self.code.extend_from_slice(&[0x48, 0xB8]); // mov rax, addr
        self.code.extend_from_slice(&addr.to_ne_bytes());
        self.code.extend_from_slice(&[0xFF, 0xD0]); // call rax

        if cfg!(target_os = "windows") {
            self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]); // add rsp, 40
        } else {
            self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]); // add rsp, 8
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn emit_call(&mut self, func: HandlerFn) {
        // mov x0, x19
        self.code.extend_from_slice(&[0xE0, 0x03, 0x13, 0xAA]);
        let addr = func as usize;
        
        // ldr x8, [pc, #8]
        // blr x8
        // b #12 (skip literal)
        // literal: 8 bytes
        self.code.extend_from_slice(&[0x48, 0x00, 0x00, 0x58]); // ldr x8, PC+8
        self.code.extend_from_slice(&[0x00, 0x01, 0x3F, 0xD6]); // blr x8
        self.code.extend_from_slice(&[0x03, 0x00, 0x00, 0x14]); // b PC+12
        self.code.extend_from_slice(&addr.to_ne_bytes());
    }

    /// Emits a call to a Rust handler that takes an extra u32 argument.
    #[cfg(target_arch = "x86_64")]
    pub fn emit_call_with_arg(&mut self, func: extern "C" fn(*mut VirtualMachine, u32), arg: u32) {
        if cfg!(target_os = "windows") {
            self.code.extend_from_slice(&[0x4C, 0x89, 0xE1]); // mov rcx, r12
            self.code.extend_from_slice(&[0xBA]); // mov edx, imm32
            self.code.extend_from_slice(&arg.to_le_bytes());
            self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]); // sub rsp, 40
        } else {
            self.code.extend_from_slice(&[0x4C, 0x89, 0xE7]); // mov rdi, r12
            self.code.extend_from_slice(&[0xBE]); // mov esi, imm32
            self.code.extend_from_slice(&arg.to_le_bytes());
            self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x08]); // sub rsp, 8
        }

        let addr = func as usize;
        self.code.extend_from_slice(&[0x48, 0xB8]); // mov rax, addr
        self.code.extend_from_slice(&addr.to_ne_bytes());
        self.code.extend_from_slice(&[0xFF, 0xD0]); // call rax

        if cfg!(target_os = "windows") {
            self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]); // add rsp, 40
        } else {
            self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]); // add rsp, 8
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn emit_call_with_arg(&mut self, func: extern "C" fn(*mut VirtualMachine, u32), arg: u32) {
        // mov x0, x19
        self.code.extend_from_slice(&[0xE0, 0x03, 0x13, 0xAA]);
        
        // load arg into w1 using ldr w1, [pc, #8] ? No, let's just inline it
        // Or simpler, same literal pool trick:
        // ldr x8, [pc, #16] ; func addr
        // ldr w1, [pc, #20] ; arg
        // blr x8
        // b #16
        // .quad func addr
        // .word arg
        // .word 0 (padding)
        self.code.extend_from_slice(&[0x88, 0x00, 0x00, 0x58]); // ldr x8, PC+16
        self.code.extend_from_slice(&[0xA1, 0x00, 0x00, 0x18]); // ldr w1, PC+20
        self.code.extend_from_slice(&[0x00, 0x01, 0x3F, 0xD6]); // blr x8
        self.code.extend_from_slice(&[0x04, 0x00, 0x00, 0x14]); // b PC+16
        
        let addr = func as usize;
        self.code.extend_from_slice(&addr.to_ne_bytes());
        self.code.extend_from_slice(&arg.to_ne_bytes());
        self.code.extend_from_slice(&[0,0,0,0]); // padding
    }

    /// Emits the JIT epilogue.
    #[cfg(target_arch = "x86_64")]
    pub fn emit_epilogue(&mut self) {
        self.code.extend_from_slice(&[0x41, 0x5C]); // pop r12
        self.code.push(0xC3); // ret
    }

    #[cfg(target_arch = "aarch64")]
    pub fn emit_epilogue(&mut self) {
        // ldr x19, [sp]
        // add sp, sp, #16
        self.code.extend_from_slice(&[0xF3, 0x03, 0x40, 0xF9]);
        self.code.extend_from_slice(&[0xFF, 0x43, 0x00, 0x91]);
        self.code.extend_from_slice(&[0xC0, 0x03, 0x5F, 0xD6]); // ret
    }

    /// Compiles an array of opcodes into executable memory.
    pub fn compile(&mut self, body: &[u8]) -> JitMemory {
        self.emit_prologue();

        let mut ip = 0;
        while ip < body.len() {
            let opcode = body[ip];
            ip += 1;

            if opcode == OpCode::AddF64 as u8 { self.emit_call(jit_handler_add); }
            else if opcode == OpCode::SubF64 as u8 { self.emit_call(jit_handler_sub); }
            else if opcode == OpCode::MulF64 as u8 { self.emit_call(jit_handler_mul); }
            else if opcode == OpCode::DivF64 as u8 { self.emit_call(jit_handler_div); }
            else if opcode == OpCode::Push as u8 { 
                let idx = u16::from_be_bytes([body[ip], body[ip+1]]) as u32;
                ip += 2; 
                self.emit_call_with_arg(jit_handler_push, idx);
            }
            else if opcode == OpCode::LoadLocal as u8 { 
                let idx = body[ip] as u32;
                ip += 1; 
                self.emit_call_with_arg(jit_handler_load_local, idx);
            }
            else if opcode == OpCode::StoreLocal as u8 { 
                let idx = body[ip] as u32;
                ip += 1; 
                self.emit_call_with_arg(jit_handler_store_local, idx);
            }
            else if opcode == OpCode::Jump as u8 { ip += 4; }
            else if opcode == OpCode::JumpIf as u8 { ip += 4; }
            else if opcode == OpCode::Call as u8 { ip += 6; }
            else if opcode == OpCode::CallNative as u8 { ip += 4; }
            else if opcode == OpCode::Return as u8 { break; }
        }

        // To satisfy Vyauma's calling convention in this PoC JIT,
        // we must push a dummy return value onto the stack before exiting!
        self.emit_call(jit_handler_return);

        self.emit_epilogue();
        JitMemory::new(&self.code)
    }
}

// --- JIT Native Handlers ---
// These are called directly from our generated machine code!

extern "C" fn jit_handler_return(vm_ptr: *mut VirtualMachine) {
    let vm = unsafe { &mut *vm_ptr };
    use crate::vm::value::Value;
    // We don't push a dummy return value here if the function expects to return normally, 
    // but the original PoC pushed 0.0. Let's just leave it since it's a PoC calling convention workaround.
    let _ = vm.stack.push(Value::Float64(0.0));
}

extern "C" fn jit_handler_push(vm_ptr: *mut VirtualMachine, idx: u32) {
    let vm = unsafe { &mut *vm_ptr };
    let c = vm.constants().get(idx as usize).unwrap();
    let _ = vm.stack.push(c);
}

extern "C" fn jit_handler_load_local(vm_ptr: *mut VirtualMachine, idx: u32) {
    let vm = unsafe { &mut *vm_ptr };
    if let Ok(frame) = vm.current_frame() {
        let val = frame.locals.load(idx as usize).unwrap();
        let _ = vm.stack.push(val);
    }
}

extern "C" fn jit_handler_store_local(vm_ptr: *mut VirtualMachine, idx: u32) {
    let vm = unsafe { &mut *vm_ptr };
    let val = vm.stack.pop().unwrap();
    if let Ok(frame) = vm.current_frame_mut() {
        let _ = frame.locals.store(idx as usize, val);
    }
}

macro_rules! jit_math {
    ($name:ident, $op:tt) => {
        extern "C" fn $name(vm_ptr: *mut VirtualMachine) {
            let vm = unsafe { &mut *vm_ptr };
            let b = vm.stack.pop().unwrap();
            let a = vm.stack.pop().unwrap();
            use crate::vm::value::Value;
            let result = match (a, b) {
                (Value::Int32(na), Value::Int32(nb)) => Value::Int32(na $op nb),
                (Value::Int64(na), Value::Int64(nb)) => Value::Int64(na $op nb),
                (Value::Float32(na), Value::Float32(nb)) => Value::Float32(na $op nb),
                (Value::Float64(na), Value::Float64(nb)) => Value::Float64(na $op nb),
                _ => panic!("JIT Error: Math operation on non-matching or invalid types!"),
            };
            let _ = vm.stack.push(result);
        }
    };
}

jit_math!(jit_handler_add, +);
jit_math!(jit_handler_sub, -);
jit_math!(jit_handler_mul, *);
jit_math!(jit_handler_div, /);
