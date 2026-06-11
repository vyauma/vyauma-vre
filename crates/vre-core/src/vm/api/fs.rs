use crate::vm::memory::Heap;
use crate::vm::value::Value;
use std::fs;

pub fn read_file(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("fs.readFile expects exactly 1 argument (path)".to_string());
    }

    if let Value::String(path) = &args[0] {
        match fs::read_to_string(path) {
            Ok(content) => Ok(Value::String(content)),
            Err(e) => Err(format!("fs.readFile failed: {}", e)),
        }
    } else {
        Err("fs.readFile argument must be a string".to_string())
    }
}

pub fn write_file(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("fs.writeFile expects exactly 2 arguments (path, content)".to_string());
    }

    let path = if let Value::String(p) = &args[0] {
        p
    } else {
        return Err("fs.writeFile path must be a string".to_string());
    };

    let content = if let Value::String(c) = &args[1] {
        c
    } else {
        return Err("fs.writeFile content must be a string".to_string());
    };

    match fs::write(path, content) {
        Ok(_) => Ok(Value::Bool(true)),
        Err(e) => Err(format!("fs.writeFile failed: {}", e)),
    }
}
