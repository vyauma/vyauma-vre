use super::NativeModule;
use crate::value::Value;
use crate::heap::ObjectType;
use std::env;
use std::process;

fn get_string_arg(heap: &crate::heap::Heap, val: &Value) -> Result<String, String> {
    if let Value::HeapRef(handle) = val {
        let obj = heap.get(*handle);
        if let ObjectType::String(s) = &obj.obj_type {
            return Ok(s.clone());
        }
    }
    Err("Expected string argument".into())
}

pub fn create_module() -> NativeModule {
    let mut module = NativeModule::new("sys");

    module.define_function("args", 0, |heap, _args| {
        let mut results = Vec::new();
        for arg in env::args() {
            let s_handle = heap.allocate(ObjectType::String(arg));
            results.push(Value::HeapRef(s_handle));
        }
        let handle = heap.allocate(ObjectType::Array(results));
        Ok(Value::HeapRef(handle))
    });

    module.define_function("exit", 1, |_heap, args| {
        if let Value::Int(code) = args[0] {
            process::exit(code as i32);
        } else {
            process::exit(1);
        }
    });

    module.define_function("env", 1, |heap, args| {
        let name = get_string_arg(heap, &args[0])?;
        match env::var(&name) {
            Ok(val) => {
                let handle = heap.allocate(ObjectType::String(val));
                Ok(Value::HeapRef(handle))
            }
            Err(_) => Ok(Value::Null), // Return null if missing
        }
    });

    module.define_function("cwd", 0, |heap, _args| {
        match env::current_dir() {
            Ok(path) => {
                let s = path.to_string_lossy().to_string();
                let handle = heap.allocate(ObjectType::String(s));
                Ok(Value::HeapRef(handle))
            }
            Err(e) => Err(format!("Failed to get cwd: {}", e)),
        }
    });

    module.define_function("platform", 0, |heap, _args| {
        let s = env::consts::OS.to_string();
        let handle = heap.allocate(ObjectType::String(s));
        Ok(Value::HeapRef(handle))
    });

    module
}
