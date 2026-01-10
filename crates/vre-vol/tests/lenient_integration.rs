use vre_core::BytecodeLoader;

// Build a bytecode bundle whose instruction stream contains a Call whose
// target offset does not exist. The strict loader will reject this as an
// invalid jump/call target, while the lenient opt-in loader only checks
// immediate lengths and should succeed.
#[test]
fn lenient_loader_opt_in_allows_weak_parse() {
    // kept for potential debug prints

    // Magic
    let mut buf = Vec::new();
    buf.extend(&0x5659_4D41u32.to_be_bytes());
    // version major/minor/patch and reserved
    buf.push(1u8);
    buf.push(0u8);
    buf.push(0u8);
    buf.push(0u8);
    // entry_point (u32)
    buf.extend(&(0u32.to_be_bytes()));

    // no constants
    buf.extend(&(0u32.to_be_bytes()));

    // instructions: [Call <big_target>, Halt]
    use vre_core::bytecode::OpCode;
    let big_target: u32 = 0x00FF_FFFF; // large target that doesn't match any instruction offset
    let instr = vec![
        OpCode::Call as u8,
        (big_target >> 24) as u8,
        (big_target >> 16) as u8,
        (big_target >> 8) as u8,
        (big_target) as u8,
        OpCode::Halt as u8,
    ];

    buf.extend(&(instr.len() as u32).to_be_bytes());
    buf.extend(&instr);

    // Strict load must fail due to invalid target
    let strict = BytecodeLoader::load(&buf);
    assert!(strict.is_err());

    // Lenient opt-in must succeed and indicate the lenient path was used
    let optin = BytecodeLoader::load_with_opt_in(&buf, true).expect("lenient load failed");
    assert!(optin.1, "expected lenient path to be used");
    let loaded = optin.0;
    // Basic sanity checks
    assert_eq!(loaded.entry_point, 0);
    assert_eq!(loaded.instructions.len(), instr.len());
}
