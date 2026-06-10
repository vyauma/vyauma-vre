pub mod builder;
pub mod codegen;
pub mod opt;

use crate::ast::Type;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMetadata {
    Int32,
    Int64,
    Float32,
    Float64,
    Bool,
    String,
    Any,
    Array,
    Dict,
    Struct(String),
    Class(String),
}

impl From<&Type> for TypeMetadata {
    fn from(t: &Type) -> Self {
        match t {
            Type::Int32 => TypeMetadata::Int32,
            Type::Int64 => TypeMetadata::Int64,
            Type::Float32 => TypeMetadata::Float32,
            Type::Float64 => TypeMetadata::Float64,
            Type::Bool => TypeMetadata::Bool,
            Type::String => TypeMetadata::String,
            Type::Any => TypeMetadata::Any,
            Type::Array(_) => TypeMetadata::Array,
            Type::Dict(_, _) => TypeMetadata::Dict,
            Type::Struct(name) => TypeMetadata::Struct(name.clone()),
            Type::Class(name) => TypeMetadata::Class(name.clone()),
        }
    }
}

pub type Value = usize; // Represents a virtual register/SSA value
pub type BlockId = usize;

#[derive(Debug, Clone)]
pub enum Instruction {
    LoadConstNumber(f64),
    LoadConstBool(bool),
    LoadConstString(String),
    LoadNull,
    
    Add(Value, Value),
    Sub(Value, Value),
    Mul(Value, Value),
    Div(Value, Value),
    Rem(Value, Value),
    
    Eq(Value, Value),
    NotEq(Value, Value),
    Lt(Value, Value),
    Lte(Value, Value),
    Gt(Value, Value),
    Gte(Value, Value),
    
    Not(Value),
    And(Value, Value),
    Or(Value, Value),
    
    Call(String, Vec<Value>),
    MethodCall(Value, String, Vec<Value>),
    
    ArrayLiteral(Vec<Value>),
    DictLiteral(Vec<(Value, Value)>),
    StructInit(String, Vec<(String, Value)>),
    NewClass(String, Vec<Value>),
    
    IndexAccess(Value, Value),
    PropertyAccess(Value, String),
    
    AssignIndex(Value, Value, Value),
    AssignProperty(Value, String, Value),
    
    LoadVar(String),
    StoreVar(String, Value),
    
    Return(Option<Value>),
    Throw(Value),
    
    SetupTry(BlockId),
    PopTry,
    
    Branch(BlockId),
    CondBranch(Value, BlockId, BlockId),
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<(Value, Instruction)>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
}
