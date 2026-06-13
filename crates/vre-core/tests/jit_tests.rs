use vre_core::config::VreConfig;
use vre_core::error::VreResult;
use vre_core::vm::vm::VirtualMachine;
use vre_core::vm::value::Value;
use vre_core::bytecode::opcode::OpCode;
use vre_core::CapabilityRegistry;

fn run_jit_vm(constants: Vec<Value>, instructions: Vec<u8>) -> VreResult<Option<Value>> {
    let mut vm = VirtualMachine::new(
        VreConfig::default(),
        instructions,
        constants,
        vec![],
        CapabilityRegistry::new(),
        std::collections::HashMap::new(),
    )
    .unwrap();
    vm.execute()?;
    Ok(vm.peek_stack().cloned().ok())
}

#[test]
fn test_jit_compiles_and_executes_hot_loop() {
    // We will call a function 55 times (threshold is 50)
    // Main (offset 0):
    // 0..3: Push 0 (0.0 loop counter)
    // LoopStart (offset 4):
    // 4..10: Call offset 27, 0 locals
    // 11..13: Push 1 (1.0)
    // 14: AddF64 (increment loop counter)
    // 15: Dup
    // 16..18: Push 2 (55.0)
    // 19: LessF64 (is counter < 55.0 ?)
    // 20..24: JumpIf LoopStart (offset 4)
    // 25: Halt
    // 
    // Function (offset 26):
    // 26..28: Push 3 (42.0)
    // 29: Return

    let constants = vec![
        Value::Float64(0.0),  // 0
        Value::Float64(1.0),  // 1
        Value::Float64(55.0), // 2
        Value::Float64(42.0), // 3
    ];

    let loop_start: u32 = 3;
    let target_fn: u32 = 26;

    let instructions = vec![
        // Main
        OpCode::Push as u8, 0, 0, // 0..2
        
        // LoopStart (offset 3):
        OpCode::Call as u8, 
            ((target_fn >> 24) & 0xFF) as u8,
            ((target_fn >> 16) & 0xFF) as u8,
            ((target_fn >>  8) & 0xFF) as u8,
            (target_fn        & 0xFF) as u8,
            0, 0, // 3..9
            
        OpCode::Pop as u8, // 10 - Pop the 42.0 result

        OpCode::Push as u8, 0, 1, // 11..13
        OpCode::AddF64 as u8,     // 14
        OpCode::Dup as u8,        // 15
        OpCode::Push as u8, 0, 2, // 16..18
        OpCode::LessF64 as u8,    // 19
        
        OpCode::JumpIf as u8,     // 20..24
            ((loop_start >> 24) & 0xFF) as u8,
            ((loop_start >> 16) & 0xFF) as u8,
            ((loop_start >>  8) & 0xFF) as u8,
            (loop_start        & 0xFF) as u8,

        OpCode::Halt as u8,       // 25

        // Target function (offset 26)
        OpCode::Push as u8, 0, 3, // 26..28
        OpCode::Return as u8,     // 29
    ];

    let result = run_jit_vm(constants, instructions).unwrap();
    
    // The final loop counter should be 55.0
    assert_eq!(result, Some(Value::Float64(55.0)));
}
