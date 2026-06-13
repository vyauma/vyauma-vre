pub mod io;
pub mod fs;
pub mod json;
pub mod time;
pub mod sys;
pub mod net;

use crate::value::{Value, NativeFunction};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
pub struct NativeModule {
    pub name: String,
    pub functions: HashMap<String, Value>,
}

impl NativeModule {
    pub fn new(name: &str) -> Self {
        NativeModule {
            name: name.to_string(),
            functions: HashMap::new(),
        }
    }

    pub fn define_function(&mut self, name: &str, arity: usize, func: fn(&mut crate::heap::Heap, &[Value]) -> Result<Value, String>) {
        let native = Value::NativeFunction(Rc::new(NativeFunction {
            name: name.to_string(),
            arity,
            func,
        }));
        self.functions.insert(name.to_string(), native);
    }
}

pub fn register_all(vm: &mut crate::vm::VM) {
    let modules = vec![
        io::create_module(),
        fs::create_module(),
        json::create_module(),
        time::create_module(),
        sys::create_module(),
        net::create_module(),
    ];

    for module in modules {
        vm.register_module(module);
    }
}
