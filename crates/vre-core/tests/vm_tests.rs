use vre_core::config::VreConfig;
use vre_core::error::{VreError, VreResult};
use vre_core::vm::vm::VirtualMachine;
use vre_core::vm::value::Value;
use vre_core::bytecode::opcode::OpCode;
use vre_core::loader::loader::BytecodeLoader;
use vre_core::{Capability, CapabilityRegistry};

// Helper to construct a minimal VM with default/empty capabilities
fn run_vm(constants: Vec<Value>, instructions: Vec<u8>) -> VreResult<Value> {
    run_vm_with_config(VreConfig::default(), constants, instructions, 0)
}

fn run_vm_with_config(
    config: VreConfig,
    constants: Vec<Value>,
    instructions: Vec<u8>,
    _entry_point: usize,
) -> VreResult<Value> {
    let mut capabilities = CapabilityRegistry::new();
    capabilities.grant(Capability::new("io.read"));
    capabilities.grant(Capability::new("io.write"));

    let mut vm = VirtualMachine::new(config, instructions, constants, vec![], capabilities).unwrap();
    vm.execute()?;
    vm.peek_stack().cloned()
}

// Helper to assemble a valid binary file payload programmatically for loader testing
fn build_bytecode_binary(constants: Vec<Value>, instructions: Vec<u8>, entry_point: u32) -> Vec<u8> {
    let mut out = Vec::new();
    // Magic "VYMA"
    out.extend_from_slice(&0x5659_4D41u32.to_be_bytes());
    // Version 1.0.1.0
    out.push(1);
    out.push(0);
    out.push(1);
    out.push(0);
    // Entry Point
    out.extend_from_slice(&entry_point.to_be_bytes());
    // Constants count
    out.extend_from_slice(&(constants.len() as u32).to_be_bytes());
    for constant in constants {
        match constant {
            Value::Null => out.push(0x00),
            Value::Bool(b) => {
                out.push(0x01);
                out.push(if b { 1 } else { 0 });
            }
            Value::Number(n) => {
                out.push(0x02);
                out.extend_from_slice(&n.to_be_bytes());
            }
            Value::Ref(r) => {
                out.push(0xFF);
                out.extend_from_slice(&r.to_be_bytes());
            }
            Value::String(s) => {
                out.push(0x03);
                out.extend_from_slice(&(s.len() as u32).to_be_bytes());
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
    // Instructions length
    out.extend_from_slice(&(instructions.len() as u32).to_be_bytes());
    out.extend(instructions);
    out
}

#[test]
fn test_stack_push_pop_dup() {
    let constants = vec![Value::Number(42.0), Value::Number(100.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0, // push constant 0 (42.0)
        OpCode::Push as u8, 0, 1, // push constant 1 (100.0)
        OpCode::Pop as u8,        // pop 100.0
        OpCode::Dup as u8,        // duplicate 42.0
        OpCode::Halt as u8,
    ];

    let result = run_vm(constants, instructions).unwrap();
    assert_eq!(result, Value::Number(42.0));
}

#[test]
fn test_arithmetic() {
    // 10 + 5 = 15
    let constants = vec![Value::Number(10.0), Value::Number(5.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Add as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Number(15.0));

    // 10 - 5 = 5
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Sub as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Number(5.0));

    // 10 * 5 = 50
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Mul as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Number(50.0));

    // 10 / 5 = 2
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Div as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Number(2.0));

    // 10 % 3 = 1
    let constants = vec![Value::Number(10.0), Value::Number(3.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Mod as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Number(1.0));

    // -10
    let constants = vec![Value::Number(10.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Neg as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Number(-10.0));
}

#[test]
fn test_comparisons() {
    let constants = vec![Value::Number(10.0), Value::Number(5.0)];
    
    // 10 == 5 -> false
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Equal as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Bool(false));

    // 10 != 5 -> true
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::NotEqual as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Bool(true));

    // 10 < 5 -> false
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Less as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Bool(false));

    // 10 <= 10 -> true
    let constants = vec![Value::Number(10.0), Value::Number(10.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::LessEqual as u8,
        OpCode::Halt as u8,
    ];
    assert_eq!(run_vm(constants.clone(), instructions).unwrap(), Value::Bool(true));
}

#[test]
fn test_unconditional_jump() {
    let constants = vec![Value::Number(99.0)];
    let instructions = vec![
        OpCode::Jump as u8, 0, 0, 0, 8, // Jump past push
        OpCode::Push as u8, 0, 0,
        OpCode::Halt as u8,             // offset 8
    ];

    let capabilities = CapabilityRegistry::new();
    let mut vm = VirtualMachine::new(VreConfig::default(), instructions, constants, vec![], capabilities).unwrap();
    vm.execute().unwrap();
    // Stack should be empty because we jumped past the push
    assert!(vm.peek_stack().is_err());
}

#[test]
fn test_conditional_jump() {
    let constants = vec![Value::Bool(true), Value::Number(42.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,       // push true
        OpCode::JumpIf as u8, 0, 0, 0, 12, // jump to offset 12 (push 42.0)
        OpCode::Push as u8, 0, 0,       // (skipped)
        OpCode::Halt as u8,             // (skipped)
        OpCode::Push as u8, 0, 1,       // offset 12: push 42.0
        OpCode::Halt as u8,
    ];

    let result = run_vm(constants, instructions).unwrap();
    assert_eq!(result, Value::Number(42.0));
}

#[test]
fn test_function_call_and_return() {
    let constants = vec![Value::Number(10.0), Value::Number(20.0)];
    // Main calls function at offset 8.
    // Function loads locals, adds them, and returns.
    let instructions = vec![
        // Offset 0 (main)
        OpCode::Call as u8, 0, 0, 0, 8, 0, 2, // call target=8, locals=2
        OpCode::Halt as u8,

        // Offset 8 (function)
        OpCode::Push as u8, 0, 0,             // push 10.0
        OpCode::StoreLocal as u8, 0, 0,       // store in local 0
        OpCode::Push as u8, 0, 1,             // push 20.0
        OpCode::StoreLocal as u8, 0, 1,       // store in local 1
        OpCode::LoadLocal as u8, 0, 0,        // load local 0
        OpCode::LoadLocal as u8, 0, 1,        // load local 1
        OpCode::Add as u8,                    // add
        OpCode::Return as u8,
    ];

    let result = run_vm(constants, instructions).unwrap();
    assert_eq!(result, Value::Number(30.0));
}

#[test]
fn test_division_by_zero() {
    let constants = vec![Value::Number(5.0), Value::Number(0.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 1,
        OpCode::Div as u8,
        OpCode::Halt as u8,
    ];

    let err = run_vm(constants, instructions).unwrap_err();
    assert!(matches!(err, VreError::DivisionByZero));
}

#[test]
fn test_stack_overflow() {
    let config = VreConfig {
        max_stack_size: 2,
        max_locals: 256,
        max_call_depth: 256,
        ffi_functions: std::collections::HashMap::new(),
    };
    let constants = vec![Value::Number(1.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 0,
        OpCode::Push as u8, 0, 0, // Should trigger overflow
        OpCode::Halt as u8,
    ];

    let err = run_vm_with_config(config, constants, instructions, 0).unwrap_err();
    assert!(matches!(err, VreError::StackOverflow));
}

#[test]
fn test_call_depth_overflow() {
    let config = VreConfig {
        max_stack_size: 1024,
        max_locals: 256,
        max_call_depth: 1, // limit call stack to 1 deep
        ffi_functions: std::collections::HashMap::new(),
    };
    let constants = vec![];
    let instructions = vec![
        OpCode::Call as u8, 0, 0, 0, 0, 0, 0, // recursive call targeting self
    ];

    let err = run_vm_with_config(config, constants, instructions, 0).unwrap_err();
    assert!(matches!(err, VreError::StackOverflow));
}

#[test]
fn test_bytecode_loader_validation() {
    let constants = vec![Value::Number(77.0)];
    let instructions = vec![OpCode::Push as u8, 0, 0, OpCode::Halt as u8];
    let binary = build_bytecode_binary(constants, instructions, 0);

    let loaded = BytecodeLoader::load(&binary).unwrap();
    assert_eq!(loaded.entry_point, 0);
    assert_eq!(loaded.constants.len(), 1);
    assert_eq!(loaded.constants[0], Value::Number(77.0));
    assert_eq!(loaded.instructions.len(), 4);
}

#[test]
fn test_syscall_print_capability_enforced() {
    let constants = vec![Value::Number(88.0)];
    let instructions = vec![
        OpCode::Push as u8, 0, 0,
        OpCode::Syscall as u8, 0x01, // print
        OpCode::Halt as u8,
    ];

    // Case 1: Run with capabilities granted -> should succeed
    let mut caps_granted = CapabilityRegistry::new();
    caps_granted.grant(Capability::new("io.write"));
    let mut vm = VirtualMachine::new(VreConfig::default(), instructions.clone(), constants.clone(), vec![], caps_granted).unwrap();
    assert!(vm.execute().is_ok());

    // Case 2: Run without capability granted -> should fail with CapabilityNotGranted
    let caps_denied = CapabilityRegistry::new();
    let mut vm = VirtualMachine::new(VreConfig::default(), instructions, constants, vec![], caps_denied).unwrap();
    let err = vm.execute().unwrap_err();
    assert!(matches!(err, VreError::CapabilityNotGranted));
}
