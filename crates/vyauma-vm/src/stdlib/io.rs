use super::NativeModule;
use crate::value::Value;
use crate::heap::ObjectType;
use std::io::{self, Write};

pub fn create_module() -> NativeModule {
    let mut module = NativeModule::new("io");

    module.define_function("print", 1, |heap, args| {
        // We have to extract the string logic depending on Value
        match &args[0] {
            Value::HeapRef(handle) => {
                let obj = heap.get(*handle);
                if let ObjectType::String(s) = &obj.obj_type {
                    print!("{}", s);
                } else {
                    print!("{:?}", args[0]);
                }
            }
            _ => print!("{}", args[0]),
        }
        io::stdout().flush().unwrap();
        Ok(Value::Null)
    });

    module.define_function("println", 1, |heap, args| {
        match &args[0] {
            Value::HeapRef(handle) => {
                let obj = heap.get(*handle);
                if let ObjectType::String(s) = &obj.obj_type {
                    println!("{}", s);
                } else {
                    println!("{:?}", args[0]);
                }
            }
            _ => println!("{}", args[0]),
        }
        Ok(Value::Null)
    });

    module.define_function("input", 1, |heap, args| {
        match &args[0] {
            Value::HeapRef(handle) => {
                let obj = heap.get(*handle);
                if let ObjectType::String(s) = &obj.obj_type {
                    print!("{}", s);
                }
            }
            _ => print!("{}", args[0]),
        }
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_line(&mut buffer) {
            return Err(e.to_string());
        }
        let trimmed = buffer.trim_end().to_string();
        let handle = heap.allocate(ObjectType::String(trimmed));
        Ok(Value::HeapRef(handle))
    });

    module.define_function("stderr", 1, |heap, args| {
        match &args[0] {
            Value::HeapRef(handle) => {
                let obj = heap.get(*handle);
                if let ObjectType::String(s) = &obj.obj_type {
                    eprintln!("{}", s);
                } else {
                    eprintln!("{:?}", args[0]);
                }
            }
            _ => eprintln!("{}", args[0]),
        }
        Ok(Value::Null)
    });

    module
}
