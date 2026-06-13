#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    // Stack manipulation
    Pop,
    Dup,

    // Constants
    PushConstant, // Operand: 1 byte index into constant pool
    PushTrue,
    PushFalse,
    PushNull,

    // Variables (Local scope mapped to Operand Stack offsets)
    LoadLocal,  // Operand: 1 byte slot index
    StoreLocal, // Operand: 1 byte slot index

    // Globals
    LoadGlobal,  // Operand: 1 byte constant pool index containing string name
    StoreGlobal, // Operand: 1 byte constant pool index

    // Objects
    LoadField,  // Operand: 1 byte constant pool index (field name)
    StoreField, // Operand: 1 byte constant pool index (field name)

    // Control flow
    Jump,        // Operand: 2 bytes offset
    JumpIfFalse, // Operand: 2 bytes offset

    // Functions
    Call,   // Operand: 1 byte arg count
    Return,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,

    // Logic
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,

    // System
    Halt,
}

impl From<u8> for OpCode {
    fn from(val: u8) -> Self {
        unsafe { std::mem::transmute(val) }
    }
}
