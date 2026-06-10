use vre_core::config::VreConfig;

pub fn register_ffi(config: &mut VreConfig) {
    // Add ffi_sum
    config.ffi_functions.insert("ffi_sum".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_sum expects 2 arguments".to_string()); }
        let b = args.pop().unwrap();
        let a = args.pop().unwrap();
        let a_num = match a { vre_core::vm::value::Value::Float64(n) => n, _ => return Err("Expected number".to_string()) };
        let b_num = match b { vre_core::vm::value::Value::Float64(n) => n, _ => return Err("Expected number".to_string()) };
        Ok(vre_core::vm::value::Value::Float64(a_num + b_num))
    });

    // Array Len FFI
    config.ffi_functions.insert("ffi_array_len".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_array_len expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => {
                let obj = heap.get(id).map_err(|_| "Invalid heap reference".to_string())?;
                match obj {
                    vre_core::vm::memory::HeapObject::Array(arr) => Ok(vre_core::vm::value::Value::Float64(arr.len() as f64)),
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
                        Ok(vre_core::vm::value::Value::Float64(f))
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
                    Ok(vre_core::vm::value::Value::Reference(ref_id))
                }
                serde_json::Value::Object(obj) => {
                    let mut v_map = std::collections::HashMap::new();
                    for (k, v) in obj {
                        v_map.insert(k.clone(), json_to_vyauma(heap, v)?);
                    }
                    let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                    let ref_id = heap.allocate(h_obj);
                    Ok(vre_core::vm::value::Value::Reference(ref_id))
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
                vre_core::vm::value::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
                vre_core::vm::value::Value::Reference(id) => {
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
                        _ => Err("Unsupported heap object for JSON".to_string()),
                    }
                }
                _ => {
                    if let Ok(n) = value.as_f64() {
                        if let Some(num) = serde_json::Number::from_f64(n) {
                            return Ok(serde_json::Value::Number(num));
                        }
                    }
                    Err("Unsupported Vyauma value for JSON".to_string())
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
        Ok(vre_core::vm::value::Value::Reference(ref_id))
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
        Ok(vre_core::vm::value::Value::Float64(since_the_epoch.as_millis() as f64))
    });

    // Math FFIs
    config.ffi_functions.insert("ffi_math_abs".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_abs expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.abs())),
            _ => Err("ffi_math_abs expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_floor".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_floor expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.floor())),
            _ => Err("ffi_math_floor expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_ceil".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_ceil expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.ceil())),
            _ => Err("ffi_math_ceil expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_round".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_round expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.round())),
            _ => Err("ffi_math_round expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_sin".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_sin expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.sin())),
            _ => Err("ffi_math_sin expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_cos".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_cos expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.cos())),
            _ => Err("ffi_math_cos expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_tan".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_tan expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.tan())),
            _ => Err("ffi_math_tan expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_sqrt".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_sqrt expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.sqrt())),
            _ => Err("ffi_math_sqrt expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_pow".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_math_pow expects 2 arguments".to_string()); }
        let y = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n,
            _ => return Err("Expected number".to_string()),
        };
        let x = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n,
            _ => return Err("Expected number".to_string()),
        };
        Ok(vre_core::vm::value::Value::Float64(x.powf(y)))
    });

    // Crypto FFIs
    config.ffi_functions.insert("ffi_crypto_random_bytes".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_random_bytes expects 1 argument".to_string()); }
        let len = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as usize,
            _ => return Err("Expected length number".to_string()),
        };
        let bytes = vre_core::crypto::random_bytes(len);
        let mut v_arr = Vec::new();
        for b in bytes {
            v_arr.push(vre_core::vm::value::Value::Float64(b as f64));
        }
        let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
        let ref_id = heap.allocate(obj);
        Ok(vre_core::vm::value::Value::Reference(ref_id))
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
        Ok(vre_core::vm::value::Value::Bool(vre_core::pal::get_pal().exists(std::path::Path::new(&path))))
    });

    config.ffi_functions.insert("ffi_fs_delete".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_delete expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let result = if vre_core::pal::get_pal().is_file(std::path::Path::new(&path)) {
            vre_core::pal::get_pal().remove_file(std::path::Path::new(&path)).is_ok()
        } else {
            vre_core::pal::get_pal().remove_dir_all(std::path::Path::new(&path)).is_ok()
        };
        Ok(vre_core::vm::value::Value::Bool(result))
    });

    config.ffi_functions.insert("ffi_fs_size".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_size expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let size = vre_core::pal::get_pal().metadata_len(std::path::Path::new(&path)).map(|len| len as f64).unwrap_or(-1.0);
        Ok(vre_core::vm::value::Value::Float64(size))
    });

    config.ffi_functions.insert("ffi_fs_read_file".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_read_file expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let content = vre_core::pal::get_pal().read_to_string(std::path::Path::new(&path)).unwrap_or_else(|_| String::new());
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
        let success = vre_core::pal::get_pal().write(std::path::Path::new(&path), &content).is_ok();
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
        
        let success = vre_core::pal::get_pal().append(std::path::Path::new(&path), &content).is_ok();
        Ok(vre_core::vm::value::Value::Bool(success))
    });
    // Console I/O FFIs
    config.ffi_functions.insert("ffi_console_print".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_console_print expects 1 argument".to_string()); }
        let val = args.pop().unwrap();
        // Simple stringification for print
        let s = match val {
            vre_core::vm::value::Value::String(s) => s,
            vre_core::vm::value::Value::Float64(n) => n.to_string(),
            vre_core::vm::value::Value::Bool(b) => b.to_string(),
            vre_core::vm::value::Value::Null => "null".to_string(),
            vre_core::vm::value::Value::Reference(_) => "[Object Reference]".to_string(),
            _ => "[Unknown]".to_string(),
        };
        vre_core::pal::get_pal().print(&s);
        Ok(vre_core::vm::value::Value::Null)
    });

    config.ffi_functions.insert("ffi_console_println".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_console_println expects 1 argument".to_string()); }
        let val = args.pop().unwrap();
        let s = match val {
            vre_core::vm::value::Value::String(s) => s,
            vre_core::vm::value::Value::Float64(n) => n.to_string(),
            vre_core::vm::value::Value::Bool(b) => b.to_string(),
            vre_core::vm::value::Value::Null => "null".to_string(),
            vre_core::vm::value::Value::Reference(_) => "[Object Reference]".to_string(),
            _ => "[Unknown]".to_string(),
        };
        vre_core::pal::get_pal().println(&s);
        Ok(vre_core::vm::value::Value::Null)
    });

    config.ffi_functions.insert("ffi_console_readline".to_string(), |_heap, _args| {
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            Ok(vre_core::vm::value::Value::String(input.trim_end_matches(&['\r', '\n'][..]).to_string()))
        } else {
            Ok(vre_core::vm::value::Value::Null)
        }
    });

    // Additional String FFIs
    config.ffi_functions.insert("ffi_string_len".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_len expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::Float64(s.len() as f64))
    });

    config.ffi_functions.insert("ffi_string_contains".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_string_contains expects 2 arguments".to_string()); }
        let sub = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for substring".to_string()),
        };
        let main_str = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string to search".to_string()),
        };
        Ok(vre_core::vm::value::Value::Bool(main_str.contains(&sub)))
    });

    config.ffi_functions.insert("ffi_string_replace".to_string(), |_heap, mut args| {
        if args.len() != 3 { return Err("ffi_string_replace expects 3 arguments".to_string()); }
        let new_str = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected replacement string".to_string()),
        };
        let old_str = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected target string".to_string()),
        };
        let main_str = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(main_str.replace(&old_str, &new_str)))
    });

    config.ffi_functions.insert("ffi_string_to_lower".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_to_lower expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(s.to_lowercase()))
    });

    config.ffi_functions.insert("ffi_string_to_upper".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_to_upper expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(s.to_uppercase()))
    });

    config.ffi_functions.insert("ffi_string_trim".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_trim expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(s.trim().to_string()))
    });

    // Array FFIs
    config.ffi_functions.insert("ffi_array_push".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_array_push expects 2 arguments".to_string()); }
        let val = args.pop().unwrap();
        let arr_ref = match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => id,
            _ => return Err("Expected array reference".to_string()),
        };
        
        let obj = heap.get_mut(arr_ref).map_err(|_| "Invalid heap reference".to_string())?;
        if let vre_core::vm::memory::HeapObject::Array(arr) = obj {
            arr.push(val);
            Ok(vre_core::vm::value::Value::Null)
        } else {
            Err("Reference is not an array".to_string())
        }
    });

    config.ffi_functions.insert("ffi_array_pop".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_array_pop expects 1 argument".to_string()); }
        let arr_ref = match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => id,
            _ => return Err("Expected array reference".to_string()),
        };
        
        let obj = heap.get_mut(arr_ref).map_err(|_| "Invalid heap reference".to_string())?;
        if let vre_core::vm::memory::HeapObject::Array(arr) = obj {
            if let Some(val) = arr.pop() {
                Ok(val)
            } else {
                Ok(vre_core::vm::value::Value::Null)
            }
        } else {
            Err("Reference is not an array".to_string())
        }
    });

    config.ffi_functions.insert("ffi_array_get".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_array_get expects 2 arguments".to_string()); }
        let idx = match args.pop().unwrap() {
            val @ _ => val.as_f64().map_err(|_| "Expected number index".to_string())? as usize,
        };
        let arr_ref = match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => id,
            _ => return Err("Expected array reference".to_string()),
        };
        
        let obj = heap.get(arr_ref).map_err(|_| "Invalid heap reference".to_string())?;
        if let vre_core::vm::memory::HeapObject::Array(arr) = obj {
            if let Some(val) = arr.get(idx) {
                Ok(val.clone())
            } else {
                Ok(vre_core::vm::value::Value::Null)
            }
        } else {
            Err("Reference is not an array".to_string())
        }
    });

    config.ffi_functions.insert("ffi_array_set".to_string(), |heap, mut args| {
        if args.len() != 3 { return Err("ffi_array_set expects 3 arguments".to_string()); }
        let val = args.pop().unwrap();
        let idx = match args.pop().unwrap() {
            num @ _ => num.as_f64().map_err(|_| "Expected number index".to_string())? as usize,
        };
        let arr_ref = match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => id,
            _ => return Err("Expected array reference".to_string()),
        };
        
        let obj = heap.get_mut(arr_ref).map_err(|_| "Invalid heap reference".to_string())?;
        if let vre_core::vm::memory::HeapObject::Array(arr) = obj {
            if idx < arr.len() {
                arr[idx] = val;
                Ok(vre_core::vm::value::Value::Bool(true))
            } else {
                Ok(vre_core::vm::value::Value::Bool(false))
            }
        } else {
            Err("Reference is not an array".to_string())
        }
    });

    // OS / Environment FFIs
    config.ffi_functions.insert("ffi_os_name".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::String(std::env::consts::OS.to_string()))
    });

    config.ffi_functions.insert("ffi_env_get".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_env_get expects 1 argument".to_string()); }
        let key = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string key".to_string()),
        };
        if let Ok(val) = std::env::var(key) {
            Ok(vre_core::vm::value::Value::String(val))
        } else {
            Ok(vre_core::vm::value::Value::Null)
        }
    });

    // Additional Math FFIs
    config.ffi_functions.insert("ffi_math_log".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_log expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.ln())),
            _ => Err("Expected number".to_string()),
        }
    });

    config.ffi_functions.insert("ffi_math_pi".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::Float64(std::f64::consts::PI))
    });

    // Additional String FFIs
    config.ffi_functions.insert("ffi_string_ends_with".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_string_ends_with expects 2 arguments".to_string()); }
        let suffix = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string suffix".to_string()),
        };
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string to check".to_string()),
        };
        Ok(vre_core::vm::value::Value::Bool(s.ends_with(&suffix)))
    });

    config.ffi_functions.insert("ffi_string_index_of".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_string_index_of expects 2 arguments".to_string()); }
        let sub = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string substring".to_string()),
        };
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string to check".to_string()),
        };
        if let Some(idx) = s.find(&sub) {
            Ok(vre_core::vm::value::Value::Float64(idx as f64))
        } else {
            Ok(vre_core::vm::value::Value::Float64(-1.0))
        }
    });

    // Path FFIs
    config.ffi_functions.insert("ffi_path_join".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_path_join expects 2 arguments".to_string()); }
        let p2 = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string path 2".to_string()),
        };
        let p1 = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string path 1".to_string()),
        };
        let mut path = std::path::PathBuf::from(p1);
        path.push(p2);
        Ok(vre_core::vm::value::Value::String(path.to_string_lossy().to_string()))
    });

    config.ffi_functions.insert("ffi_path_basename".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_path_basename expects 1 argument".to_string()); }
        let p1 = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string path".to_string()),
        };
        let path = std::path::Path::new(&p1);
        if let Some(name) = path.file_name() {
            Ok(vre_core::vm::value::Value::String(name.to_string_lossy().to_string()))
        } else {
            Ok(vre_core::vm::value::Value::String(String::new()))
        }
    });

    // Dictionary (Struct) FFIs
    config.ffi_functions.insert("ffi_dict_keys".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_dict_keys expects 1 argument".to_string()); }
        let dict_ref = match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => id,
            _ => return Err("Expected dictionary reference".to_string()),
        };
        
        let obj = heap.get(dict_ref).map_err(|_| "Invalid heap reference".to_string())?;
        if let vre_core::vm::memory::HeapObject::Struct(map) = obj {
            let mut keys = Vec::new();
            for k in map.keys() {
                keys.push(vre_core::vm::value::Value::String(k.clone()));
            }
            let arr_obj = vre_core::vm::memory::HeapObject::Array(keys);
            let arr_ref = heap.allocate(arr_obj);
            Ok(vre_core::vm::value::Value::Reference(arr_ref))
        } else {
            Err("Reference is not a dictionary/struct".to_string())
        }
    });

    config.ffi_functions.insert("ffi_dict_has".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_dict_has expects 2 arguments".to_string()); }
        let key = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string key".to_string()),
        };
        let dict_ref = match args.pop().unwrap() {
            vre_core::vm::value::Value::Reference(id) => id,
            _ => return Err("Expected dictionary reference".to_string()),
        };
        
        let obj = heap.get(dict_ref).map_err(|_| "Invalid heap reference".to_string())?;
        if let vre_core::vm::memory::HeapObject::Struct(map) = obj {
            Ok(vre_core::vm::value::Value::Bool(map.contains_key(&key)))
        } else {
            Err("Reference is not a dictionary/struct".to_string())
        }
    });

    // Process FFI
    config.ffi_functions.insert("ffi_process_exit".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_process_exit expects 1 argument".to_string()); }
        let code = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as i32,
            _ => return Err("Expected exit code number".to_string()),
        };
        std::process::exit(code);
    });
}
