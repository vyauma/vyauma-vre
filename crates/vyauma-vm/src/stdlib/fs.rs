use super::NativeModule;
use crate::value::Value;
use crate::heap::ObjectType;
use std::fs;
use std::path::Path;

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
    let mut module = NativeModule::new("fs");

    module.define_function("exists", 1, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        Ok(Value::Bool(Path::new(&path).exists()))
    });

    module.define_function("read", 1, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        match fs::read_to_string(&path) {
            Ok(content) => {
                let handle = heap.allocate(ObjectType::String(content));
                Ok(Value::HeapRef(handle))
            }
            Err(e) => Err(format!("Failed to read file '{}': {}", path, e)),
        }
    });

    module.define_function("write", 2, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        let content = get_string_arg(heap, &args[1])?;
        match fs::write(&path, content) {
            Ok(_) => Ok(Value::Null),
            Err(e) => Err(format!("Failed to write file '{}': {}", path, e)),
        }
    });

    module.define_function("append", 2, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        let content = get_string_arg(heap, &args[1])?;
        use std::io::Write;
        match fs::OpenOptions::new().append(true).create(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = write!(file, "{}", content) {
                    return Err(format!("Failed to append to '{}': {}", path, e));
                }
                Ok(Value::Null)
            }
            Err(e) => Err(format!("Failed to open file '{}': {}", path, e)),
        }
    });

    module.define_function("delete", 1, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        match fs::remove_file(&path) {
            Ok(_) => Ok(Value::Null),
            Err(e) => Err(format!("Failed to delete file '{}': {}", path, e)),
        }
    });

    module.define_function("mkdir", 1, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        match fs::create_dir_all(&path) {
            Ok(_) => Ok(Value::Null),
            Err(e) => Err(format!("Failed to create directory '{}': {}", path, e)),
        }
    });

    module.define_function("listdir", 1, |heap, args| {
        let path = get_string_arg(heap, &args[0])?;
        match fs::read_dir(&path) {
            Ok(entries) => {
                let mut results = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(name) = entry.file_name().to_str() {
                            let s_handle = heap.allocate(ObjectType::String(name.to_string()));
                            results.push(Value::HeapRef(s_handle));
                        }
                    }
                }
                let handle = heap.allocate(ObjectType::Array(results));
                Ok(Value::HeapRef(handle))
            }
            Err(e) => Err(format!("Failed to read directory '{}': {}", path, e)),
        }
    });

    module
}
