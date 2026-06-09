use vre_core::config::VreConfig;

pub fn register_ffi(config: &mut VreConfig) {
    // Add ffi_sum
    config.ffi_functions.insert("ffi_sum".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_sum expects 2 arguments".to_string()); }
        let b = args.pop().unwrap();
        let a = args.pop().unwrap();
        let a_num = match a { vre_core::vm::value::Value::Number(n) => n, _ => return Err("Expected number".to_string()) };
        let b_num = match b { vre_core::vm::value::Value::Number(n) => n, _ => return Err("Expected number".to_string()) };
        Ok(vre_core::vm::value::Value::Number(a_num + b_num))
    });

    // Array Len FFI
    config.ffi_functions.insert("ffi_array_len".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_array_len expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Ref(id) => {
                let obj = heap.get(id).map_err(|_| "Invalid heap reference".to_string())?;
                match obj {
                    vre_core::vm::memory::HeapObject::Array(arr) => Ok(vre_core::vm::value::Value::Number(arr.len() as f64)),
                    _ => Err("Expected array".to_string()),
                }
            }
            _ => Err("Expected array reference".to_string()),
        }
    });

    // JSON Parser FFI
    config.ffi_functions.insert("ffi_json_parse".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_json_parse expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("ffi_json_parse expected string".to_string()),
        };

        let json_val: serde_json::Value = serde_json::from_str(&s).map_err(|e| format!("JSON Parse Error: {}", e))?;
        
        fn json_to_vyauma(heap: &mut vre_core::vm::memory::Heap, json: &serde_json::Value) -> Result<vre_core::vm::value::Value, String> {
            match json {
                serde_json::Value::Null => Ok(vre_core::vm::value::Value::Null),
                serde_json::Value::Bool(b) => Ok(vre_core::vm::value::Value::Bool(*b)),
                serde_json::Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        Ok(vre_core::vm::value::Value::Number(f))
                    } else {
                        Err("Invalid JSON Number".to_string())
                    }
                }
                serde_json::Value::String(s) => Ok(vre_core::vm::value::Value::String(s.clone())),
                serde_json::Value::Array(arr) => {
                    let mut v_arr = Vec::new();
                    for item in arr {
                        v_arr.push(json_to_vyauma(heap, item)?);
                    }
                    let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
                    let ref_id = heap.allocate(obj);
                    Ok(vre_core::vm::value::Value::Ref(ref_id))
                }
                serde_json::Value::Object(obj) => {
                    let mut v_map = std::collections::HashMap::new();
                    for (k, v) in obj {
                        v_map.insert(k.clone(), json_to_vyauma(heap, v)?);
                    }
                    let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                    let ref_id = heap.allocate(h_obj);
                    Ok(vre_core::vm::value::Value::Ref(ref_id))
                }
            }
        }
        
        json_to_vyauma(heap, &json_val)
    });

    // JSON Stringify FFI
    config.ffi_functions.insert("ffi_json_stringify".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_json_stringify expects 1 argument".to_string()); }
        let root = args.pop().unwrap();

        fn vyauma_to_json(heap: &vre_core::vm::memory::Heap, value: &vre_core::vm::value::Value) -> Result<serde_json::Value, String> {
            match value {
                vre_core::vm::value::Value::Null => Ok(serde_json::Value::Null),
                vre_core::vm::value::Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
                vre_core::vm::value::Value::Number(n) => {
                    if let Some(num) = serde_json::Number::from_f64(*n) {
                        Ok(serde_json::Value::Number(num))
                    } else {
                        Err("Invalid float for JSON".to_string())
                    }
                }
                vre_core::vm::value::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
                vre_core::vm::value::Value::Ref(id) => {
                    let obj = heap.get(*id).map_err(|_| "Invalid heap reference".to_string())?;
                    match obj {
                        vre_core::vm::memory::HeapObject::Array(arr) => {
                            let mut j_arr = Vec::new();
                            for item in arr {
                                j_arr.push(vyauma_to_json(heap, item)?);
                            }
                            Ok(serde_json::Value::Array(j_arr))
                        }
                        vre_core::vm::memory::HeapObject::Struct(map) => {
                            let mut j_map = serde_json::Map::new();
                            for (k, v) in map {
                                j_map.insert(k.clone(), vyauma_to_json(heap, v)?);
                            }
                            Ok(serde_json::Value::Object(j_map))
                        }
                        vre_core::vm::memory::HeapObject::String(s) => Ok(serde_json::Value::String(s.clone())),
                    }
                }
            }
        }

        let json_val = vyauma_to_json(heap, &root)?;
        let s = serde_json::to_string(&json_val).map_err(|e| format!("JSON Stringify Error: {}", e))?;
        Ok(vre_core::vm::value::Value::String(s))
    });

    // String Split FFI
    config.ffi_functions.insert("ffi_string_split".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_string_split expects 2 arguments".to_string()); }
        let delimiter = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string delimiter".to_string()),
        };
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string to split".to_string()),
        };

        let parts: Vec<&str> = s.split(&delimiter).collect();
        let mut v_arr = Vec::new();
        for p in parts {
            v_arr.push(vre_core::vm::value::Value::String(p.to_string()));
        }
        let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
        let ref_id = heap.allocate(obj);
        Ok(vre_core::vm::value::Value::Ref(ref_id))
    });

    // String Starts With FFI
    config.ffi_functions.insert("ffi_string_starts_with".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_string_starts_with expects 2 arguments".to_string()); }
        let prefix = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string prefix".to_string()),
        };
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string to check".to_string()),
        };
        Ok(vre_core::vm::value::Value::Bool(s.starts_with(&prefix)))
    });

    // Time FFI
    config.ffi_functions.insert("ffi_time_now_ms".to_string(), |_heap, _args| {
        let now = std::time::SystemTime::now();
        let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Time went backwards: {}", e))?;
        Ok(vre_core::vm::value::Value::Number(since_the_epoch.as_millis() as f64))
    });

    // Math FFIs
    config.ffi_functions.insert("ffi_math_abs".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_abs expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.abs())),
            _ => Err("ffi_math_abs expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_floor".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_floor expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.floor())),
            _ => Err("ffi_math_floor expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_ceil".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_ceil expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.ceil())),
            _ => Err("ffi_math_ceil expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_round".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_round expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.round())),
            _ => Err("ffi_math_round expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_sin".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_sin expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.sin())),
            _ => Err("ffi_math_sin expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_cos".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_cos expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.cos())),
            _ => Err("ffi_math_cos expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_tan".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_tan expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.tan())),
            _ => Err("ffi_math_tan expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_sqrt".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_sqrt expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => Ok(vre_core::vm::value::Value::Number(n.sqrt())),
            _ => Err("ffi_math_sqrt expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_pow".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_math_pow expects 2 arguments".to_string()); }
        let y = match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => n,
            _ => return Err("Expected number".to_string()),
        };
        let x = match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => n,
            _ => return Err("Expected number".to_string()),
        };
        Ok(vre_core::vm::value::Value::Number(x.powf(y)))
    });

    // Crypto FFIs
    config.ffi_functions.insert("ffi_crypto_random_bytes".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_random_bytes expects 1 argument".to_string()); }
        let len = match args.pop().unwrap() {
            vre_core::vm::value::Value::Number(n) => n as usize,
            _ => return Err("Expected length number".to_string()),
        };
        let bytes = vre_core::crypto::random_bytes(len);
        let mut v_arr = Vec::new();
        for b in bytes {
            v_arr.push(vre_core::vm::value::Value::Number(b as f64));
        }
        let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
        let ref_id = heap.allocate(obj);
        Ok(vre_core::vm::value::Value::Ref(ref_id))
    });

    config.ffi_functions.insert("ffi_crypto_sha256".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_sha256 expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        let hash = vre_core::crypto::sha256(s.as_bytes());
        Ok(vre_core::vm::value::Value::String(hash))
    });

    // File I/O FFIs
    config.ffi_functions.insert("ffi_fs_exists".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_exists expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        Ok(vre_core::vm::value::Value::Bool(std::path::Path::new(&path).exists()))
    });

    config.ffi_functions.insert("ffi_fs_delete".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_delete expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let result = if std::path::Path::new(&path).is_file() {
            std::fs::remove_file(&path).is_ok()
        } else {
            std::fs::remove_dir_all(&path).is_ok()
        };
        Ok(vre_core::vm::value::Value::Bool(result))
    });

    config.ffi_functions.insert("ffi_fs_size".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_size expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let size = std::fs::metadata(&path).map(|m| m.len() as f64).unwrap_or(-1.0);
        Ok(vre_core::vm::value::Value::Number(size))
    });

    config.ffi_functions.insert("ffi_fs_read_file".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_read_file expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let content = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());
        Ok(vre_core::vm::value::Value::String(content))
    });

    config.ffi_functions.insert("ffi_fs_write_file".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_fs_write_file expects 2 arguments".to_string()); }
        let content = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected content string".to_string()),
        };
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let success = std::fs::write(&path, content).is_ok();
        Ok(vre_core::vm::value::Value::Bool(success))
    });

    config.ffi_functions.insert("ffi_fs_append_file".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_fs_append_file expects 2 arguments".to_string()); }
        let content = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected content string".to_string()),
        };
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)
            .map_err(|e| format!("Failed to open file for append: {}", e))?;
        let success = file.write_all(content.as_bytes()).is_ok();
        Ok(vre_core::vm::value::Value::Bool(success))
    });
}
