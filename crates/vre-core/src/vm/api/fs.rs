use crate::vm::memory::Heap;
use crate::vm::value::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;

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

pub fn append_file(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("fs.appendFile expects exactly 2 arguments (path, content)".to_string());
    }

    let path = if let Value::String(p) = &args[0] { p } else { return Err("fs.appendFile path must be a string".to_string()); };
    let content = if let Value::String(c) = &args[1] { c } else { return Err("fs.appendFile content must be a string".to_string()); };

    let mut file = OpenOptions::new().append(true).create(true).open(path).map_err(|e| format!("fs.appendFile failed: {}", e))?;
    file.write_all(content.as_bytes()).map_err(|e| format!("fs.appendFile failed: {}", e))?;
    Ok(Value::Bool(true))
}

pub fn exists(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("fs.exists expects exactly 1 argument (path)".to_string()); }
    let path = if let Value::String(p) = &args[0] { p } else { return Err("fs.exists path must be a string".to_string()); };
    Ok(Value::Bool(std::path::Path::new(path).exists()))
}

pub fn delete(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("fs.delete expects exactly 1 argument (path)".to_string()); }
    let path = if let Value::String(p) = &args[0] { p } else { return Err("fs.delete path must be a string".to_string()); };
    
    if std::path::Path::new(path).is_dir() {
        fs::remove_dir_all(path).map_err(|e| format!("fs.delete failed: {}", e))?;
    } else {
        fs::remove_file(path).map_err(|e| format!("fs.delete failed: {}", e))?;
    }
    Ok(Value::Bool(true))
}

pub fn size(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("fs.size expects exactly 1 argument (path)".to_string()); }
    let path = if let Value::String(p) = &args[0] { p } else { return Err("fs.size path must be a string".to_string()); };
    match fs::metadata(path) {
        Ok(meta) => Ok(Value::Float64(meta.len() as f64)),
        Err(e) => Err(format!("fs.size failed: {}", e)),
    }
}
