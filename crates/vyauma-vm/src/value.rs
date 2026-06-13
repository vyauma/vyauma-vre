use std::rc::Rc;
use crate::chunk::Chunk;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    HeapRef(usize),
    Function(Rc<Function>), // Static bytecode functions aren't garbage collected in Phase C
    NativeFunction(Rc<NativeFunction>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

#[derive(Debug, Clone)]
pub struct NativeFunction {
    pub name: String,
    pub arity: usize,
    pub func: fn(&mut crate::heap::Heap, &[Value]) -> Result<Value, String>,
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name 
    }
}

// Display won't be able to deeply format HeapRefs here easily since it doesn't hold a reference to the Heap, 
// so we'll just format the pointer. In a real system, the VM stringifies values with the heap context.
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::HeapRef(h) => write!(f, "<ref {}>", h),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::NativeFunction(func) => write!(f, "<native fn {}>", func.name),
        }
    }
}
