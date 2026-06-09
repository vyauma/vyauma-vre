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
    pub fn emit_prologue(&mut self) {
        // push r12
        self.code.extend_from_slice(&[0x41, 0x54]);

        if cfg!(target_os = "windows") {
            // mov r12, rcx
            self.code.extend_from_slice(&[0x49, 0x89, 0xCC]);
        } else {
            // mov r12, rdi
            self.code.extend_from_slice(&[0x49, 0x89, 0xFC]);
        }
    }

    /// Emits a call to a Rust native handler function.
    pub fn emit_call(&mut self, func: HandlerFn) {
        if cfg!(target_os = "windows") {
            // mov rcx, r12 (Setup arg 1)
            self.code.extend_from_slice(&[0x4C, 0x89, 0xE1]);
            // sub rsp, 40 (Allocate shadow space and align stack)
            self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);
        } else {
            // mov rdi, r12 (Setup arg 1)
            self.code.extend_from_slice(&[0x4C, 0x89, 0xE7]);
            // sub rsp, 8 (Align stack)
            self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x08]);
        }

        let addr = func as usize;
        // mov rax, addr
        self.code.extend_from_slice(&[0x48, 0xB8]);
        self.code.extend_from_slice(&addr.to_ne_bytes());
        // call rax
        self.code.extend_from_slice(&[0xFF, 0xD0]);

        if cfg!(target_os = "windows") {
            // add rsp, 40
            self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]);
        } else {
            // add rsp, 8
            self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]);
        }
    }

    /// Emits the JIT epilogue.
    pub fn emit_epilogue(&mut self) {
        // pop r12
        self.code.extend_from_slice(&[0x41, 0x5C]);
        // ret
        self.code.push(0xC3);
    }

    /// Compiles an array of opcodes into executable memory.
    pub fn compile(&mut self, opcodes: &[OpCode]) -> JitMemory {
        self.emit_prologue();

        for &op in opcodes {
            match op {
                OpCode::Add => self.emit_call(jit_handler_add),
                OpCode::Sub => self.emit_call(jit_handler_sub),
                OpCode::Mul => self.emit_call(jit_handler_mul),
                OpCode::Div => self.emit_call(jit_handler_div),
                _ => {}
            }
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
    let _ = vm.stack.push(Value::Number(0.0));
}

extern "C" fn jit_handler_add(vm_ptr: *mut VirtualMachine) {
    let vm = unsafe { &mut *vm_ptr };
    // Let's implement pop and push logic using the VM's public interface or stack.
    // Assuming we'll expose `op_add` or manually manipulate it:
    let b = vm.stack.pop().unwrap();
    let a = vm.stack.pop().unwrap();
    
    use crate::vm::value::Value;
    if let (Value::Number(na), Value::Number(nb)) = (&a, &b) {
        let _ = vm.stack.push(Value::Number(na + nb));
    }
}

extern "C" fn jit_handler_sub(vm_ptr: *mut VirtualMachine) {
    let vm = unsafe { &mut *vm_ptr };
    let b = vm.stack.pop().unwrap();
    let a = vm.stack.pop().unwrap();
    use crate::vm::value::Value;
    if let (Value::Number(na), Value::Number(nb)) = (&a, &b) {
        let _ = vm.stack.push(Value::Number(na - nb));
    }
}

extern "C" fn jit_handler_mul(vm_ptr: *mut VirtualMachine) {
    let vm = unsafe { &mut *vm_ptr };
    let b = vm.stack.pop().unwrap();
    let a = vm.stack.pop().unwrap();
    use crate::vm::value::Value;
    if let (Value::Number(na), Value::Number(nb)) = (&a, &b) {
        let _ = vm.stack.push(Value::Number(na * nb));
    }
}

extern "C" fn jit_handler_div(vm_ptr: *mut VirtualMachine) {
    let vm = unsafe { &mut *vm_ptr };
    let b = vm.stack.pop().unwrap();
    let a = vm.stack.pop().unwrap();
    use crate::vm::value::Value;
    if let (Value::Number(na), Value::Number(nb)) = (&a, &b) {
        let _ = vm.stack.push(Value::Number(na / nb));
    }
}
