use vre_core::config::VreConfig;

pub fn register_ffi(config: &mut VreConfig) {
    // Add ffi_sum
    config.insert_ffi("ffi_sum".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_sum expects 2 arguments".to_string()); }
        let b = args.pop().unwrap();
        let a = args.pop().unwrap();
        let a_num = match a { vre_core::vm::value::Value::Float64(n) => n, _ => return Err("Expected number".to_string()) };
        let b_num = match b { vre_core::vm::value::Value::Float64(n) => n, _ => return Err("Expected number".to_string()) };
        Ok(vre_core::vm::value::Value::Float64(a_num + b_num))
    });

    // Array Len FFI
    config.insert_ffi("ffi_array_len".to_string(), |heap, mut args| {
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
    config.insert_ffi("ffi_json_parse".to_string(), |heap, mut args| {
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
    config.insert_ffi("ffi_json_stringify".to_string(), |heap, mut args| {
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
    config.insert_ffi("ffi_string_split".to_string(), |heap, mut args| {
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
    config.insert_ffi("ffi_string_starts_with".to_string(), |_heap, mut args| {
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
    config.insert_ffi("ffi_time_now_ms".to_string(), |_heap, _args| {
        let now = std::time::SystemTime::now();
        let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Time went backwards: {}", e))?;
        Ok(vre_core::vm::value::Value::Float64(since_the_epoch.as_millis() as f64))
    });

    // Math FFIs
    config.insert_ffi("ffi_math_abs".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_abs expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.abs())),
            _ => Err("ffi_math_abs expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_floor".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_floor expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.floor())),
            _ => Err("ffi_math_floor expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_ceil".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_ceil expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.ceil())),
            _ => Err("ffi_math_ceil expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_round".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_round expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.round())),
            _ => Err("ffi_math_round expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_sin".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_sin expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.sin())),
            _ => Err("ffi_math_sin expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_cos".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_cos expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.cos())),
            _ => Err("ffi_math_cos expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_tan".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_tan expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.tan())),
            _ => Err("ffi_math_tan expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_sqrt".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_sqrt expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.sqrt())),
            _ => Err("ffi_math_sqrt expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_pow".to_string(), |_heap, mut args| {
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
    config.insert_ffi("ffi_crypto_random_bytes".to_string(), |heap, mut args| {
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

    config.insert_ffi("ffi_crypto_sha256".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_sha256 expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        let hash = vre_core::crypto::sha256(s.as_bytes());
        Ok(vre_core::vm::value::Value::String(hash))
    });

    config.insert_ffi("ffi_crypto_bcrypt".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_bcrypt expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected password string".to_string()),
        };
        let hash = vre_core::crypto::bcrypt_hash(&s)?;
        Ok(vre_core::vm::value::Value::String(hash))
    });

    config.insert_ffi("ffi_crypto_bcrypt_verify".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_crypto_bcrypt_verify expects 2 arguments".to_string()); }
        let hash = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected hash string".to_string()),
        };
        let password = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected password string".to_string()),
        };
        let valid = vre_core::crypto::bcrypt_verify(&password, &hash)?;
        Ok(vre_core::vm::value::Value::Bool(valid))
    });

    config.insert_ffi("ffi_crypto_hmac_sha256_base64url".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_crypto_hmac_sha256_base64url expects 2 arguments".to_string()); }
        let message = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected message string".to_string()),
        };
        let key = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected key string".to_string()),
        };
        let hmac = vre_core::crypto::hmac_sha256(key.as_bytes(), message.as_bytes());
        use base64::Engine;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hmac);
        Ok(vre_core::vm::value::Value::String(b64))
    });

    config.insert_ffi("ffi_crypto_base64url_encode".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_base64url_encode expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        use base64::Engine;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes());
        Ok(vre_core::vm::value::Value::String(b64))
    });

    config.insert_ffi("ffi_crypto_base64url_decode".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_base64url_decode expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s)
            .map_err(|e| format!("Base64 Decode Error: {}", e))?;
        let decoded = String::from_utf8(bytes).map_err(|e| format!("UTF8 Error: {}", e))?;
        Ok(vre_core::vm::value::Value::String(decoded))
    });

    // HTTP FFIs
    config.register_ffi("ffi_http_get", |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_http_get expects 1 argument".to_string()); }
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected url string".to_string()),
        };
        let res = ureq::get(&url).call().map_err(|e| format!("HTTP GET Error: {}", e))?;
        let body = res.into_string().unwrap_or_else(|_| String::new());
        Ok(vre_core::vm::value::Value::String(body))
    }, vec![vre_core::capability::capability::Capability::new("sys.net")]);

    config.register_ffi("ffi_http_post", |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_http_post expects 2 arguments".to_string()); }
        let body = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected body string".to_string()),
        };
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected url string".to_string()),
        };
        let res = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(&body)
            .map_err(|e| format!("HTTP POST Error: {}", e))?;
        let resp_body = res.into_string().unwrap_or_else(|_| String::new());
        Ok(vre_core::vm::value::Value::String(resp_body))
    }, vec![vre_core::capability::capability::Capability::new("sys.net")]);

    // File I/O FFIs
    config.register_ffi("ffi_fs_exists", |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_exists expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        Ok(vre_core::vm::value::Value::Bool(vre_core::pal::get_pal().exists(std::path::Path::new(&path))))
    }, vec![vre_core::capability::capability::Capability::new("fs.read")]);

    config.register_ffi("ffi_fs_delete", |_heap, mut args| {
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
    }, vec![vre_core::capability::capability::Capability::new("fs.write")]);

    config.register_ffi("ffi_fs_size", |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_size expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let size = vre_core::pal::get_pal().metadata_len(std::path::Path::new(&path)).map(|len| len as f64).unwrap_or(-1.0);
        Ok(vre_core::vm::value::Value::Float64(size))
    }, vec![vre_core::capability::capability::Capability::new("fs.read")]);

    config.register_ffi("ffi_fs_read_file", |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_read_file expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let content = vre_core::pal::get_pal().read_to_string(std::path::Path::new(&path)).unwrap_or_else(|_| String::new());
        Ok(vre_core::vm::value::Value::String(content))
    }, vec![vre_core::capability::capability::Capability::new("fs.read")]);

    config.register_ffi("ffi_fs_write_file", |_heap, mut args| {
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
    }, vec![vre_core::capability::capability::Capability::new("fs.write")]);

    config.insert_ffi("ffi_fs_append_file".to_string(), |_heap, mut args| {
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
    // Network FFIs
    config.insert_ffi("ffi_net_read".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_net_read expects 2 args".to_string()); }
        let max_len = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as usize,
            _ => return Err("Expected length".to_string()),
        };
        let fd = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as usize,
            _ => return Err("Expected fd".to_string()),
        };
        // We can't access VirtualMachine's resources from here because NativeFunction only takes Heap!
        // Wait, NativeFunction does NOT take VirtualMachine? It only takes `&mut Heap, Vec<Value>`!
        // That means we CANNOT access `resources` map from here!!
        Ok(vre_core::vm::value::Value::Null)
    });
    
    // Console I/O FFIs
    config.insert_ffi("ffi_console_print".to_string(), |heap, mut args| {
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

    config.insert_ffi("ffi_console_println".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_console_println expects 1 argument".to_string()); }
        let val = args.pop().unwrap();
        let s = match val {
            vre_core::vm::value::Value::String(s) => s,
            vre_core::vm::value::Value::Float64(n) => n.to_string(),
            vre_core::vm::value::Value::Int64(n) => n.to_string(),
            vre_core::vm::value::Value::Int32(n) => n.to_string(),
            vre_core::vm::value::Value::Bool(b) => b.to_string(),
            vre_core::vm::value::Value::Null => "null".to_string(),
            vre_core::vm::value::Value::Reference(_) => "[Object Reference]".to_string(),
            _ => "[Unknown]".to_string(),
        };
        vre_core::pal::get_pal().println(&s);
        Ok(vre_core::vm::value::Value::Null)
    });

    config.insert_ffi("ffi_console_readline".to_string(), |_heap, _args| {
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            Ok(vre_core::vm::value::Value::String(input.trim_end_matches(&['\r', '\n'][..]).to_string()))
        } else {
            Ok(vre_core::vm::value::Value::Null)
        }
    });

    // Additional String FFIs
    config.insert_ffi("ffi_string_len".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_len expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::Float64(s.len() as f64))
    });

    config.insert_ffi("ffi_string_contains".to_string(), |_heap, mut args| {
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

    config.insert_ffi("ffi_string_replace".to_string(), |_heap, mut args| {
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

    config.insert_ffi("ffi_string_split".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_string_split expects 2 arguments".to_string()); }
        let delim = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            vre_core::vm::value::Value::Reference(id) => {
                if let vre_core::vm::memory::HeapObject::String(s) = heap.get(id).map_err(|e| format!("{:?}", e))? {
                    s.clone()
                } else {
                    return Err("Expected string delimiter".to_string());
                }
            }
            _ => return Err("Expected string delimiter".to_string()),
        };
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            vre_core::vm::value::Value::Reference(id) => {
                if let vre_core::vm::memory::HeapObject::String(s) = heap.get(id).map_err(|e| format!("{:?}", e))? {
                    s.clone()
                } else {
                    return Err("Expected string".to_string());
                }
            }
            _ => return Err("Expected string".to_string()),
        };
        
        let parts: Vec<vre_core::vm::value::Value> = s.split(&delim)
            .map(|part| vre_core::vm::value::Value::String(part.to_string()))
            .collect();
            
        let arr_obj = vre_core::vm::memory::HeapObject::Array(parts);
        let arr_ref = heap.allocate(arr_obj);
        Ok(vre_core::vm::value::Value::Reference(arr_ref))
    });

    config.insert_ffi("ffi_string_to_lower".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_to_lower expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(s.to_lowercase()))
    });

    config.insert_ffi("ffi_string_to_upper".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_to_upper expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(s.to_uppercase()))
    });

    config.insert_ffi("ffi_string_trim".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_string_trim expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        Ok(vre_core::vm::value::Value::String(s.trim().to_string()))
    });

    // Array FFIs
    config.insert_ffi("ffi_array_push".to_string(), |heap, mut args| {
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

    config.insert_ffi("ffi_array_pop".to_string(), |heap, mut args| {
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

    config.insert_ffi("ffi_array_get".to_string(), |heap, mut args| {
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

    config.insert_ffi("ffi_array_set".to_string(), |heap, mut args| {
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
    config.insert_ffi("ffi_os_name".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::String(std::env::consts::OS.to_string()))
    });

    config.register_ffi("ffi_env_get", |_heap, mut args| {
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
    }, vec![vre_core::capability::capability::Capability::new("sys.env")]);

    // Additional Math FFIs
    config.insert_ffi("ffi_math_log".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_math_log expects 1 argument".to_string()); }
        match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => Ok(vre_core::vm::value::Value::Float64(n.ln())),
            _ => Err("Expected number".to_string()),
        }
    });

    config.insert_ffi("ffi_math_pi".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::Float64(std::f64::consts::PI))
    });

    // Additional String FFIs
    config.insert_ffi("ffi_string_ends_with".to_string(), |_heap, mut args| {
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

    config.insert_ffi("ffi_string_index_of".to_string(), |_heap, mut args| {
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
    config.insert_ffi("ffi_path_join".to_string(), |_heap, mut args| {
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

    config.insert_ffi("ffi_path_basename".to_string(), |_heap, mut args| {
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
    config.insert_ffi("ffi_dict_keys".to_string(), |heap, mut args| {
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

    config.insert_ffi("ffi_dict_has".to_string(), |heap, mut args| {
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
    config.register_ffi("ffi_process_exit", |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_process_exit expects 1 argument".to_string()); }
        let code = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as i32,
            _ => return Err("Expected exit code number".to_string()),
        };
        std::process::exit(code);
    }, vec![vre_core::capability::capability::Capability::new("sys.process")]);

    // --- Phase 4 Utilities ---

    // Regex FFIs
    config.insert_ffi("ffi_regex_is_match".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_regex_is_match expects 2 arguments".to_string()); }
        let text = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected text string".to_string()),
        };
        let pattern = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected pattern string".to_string()),
        };
        let re = regex::Regex::new(&pattern).map_err(|e| format!("Regex Error: {}", e))?;
        Ok(vre_core::vm::value::Value::Bool(re.is_match(&text)))
    });

    config.insert_ffi("ffi_regex_replace".to_string(), |_heap, mut args| {
        if args.len() != 3 { return Err("ffi_regex_replace expects 3 arguments".to_string()); }
        let rep = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected replacement string".to_string()),
        };
        let text = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected text string".to_string()),
        };
        let pattern = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected pattern string".to_string()),
        };
        let re = regex::Regex::new(&pattern).map_err(|e| format!("Regex Error: {}", e))?;
        Ok(vre_core::vm::value::Value::String(re.replace_all(&text, rep.as_str()).to_string()))
    });

    // Date / Time FFIs
    config.insert_ffi("ffi_date_now_iso8601".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::String(chrono::Utc::now().to_rfc3339()))
    });

    config.insert_ffi("ffi_date_format".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_date_format expects 2 arguments".to_string()); }
        let format_str = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected format string".to_string()),
        };
        let ts_ms = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as i64,
            _ => return Err("Expected timestamp number".to_string()),
        };
        let dt = chrono::DateTime::from_timestamp_millis(ts_ms)
            .ok_or_else(|| "Invalid timestamp".to_string())?;
        Ok(vre_core::vm::value::Value::String(dt.format(&format_str).to_string()))
    });

    // UUID
    config.insert_ffi("ffi_uuid_v4".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::String(uuid::Uuid::new_v4().to_string()))
    });

    // --- Phase 4 Serialization (YAML, TOML) ---

    // YAML Parser
    config.insert_ffi("ffi_yaml_parse".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_yaml_parse expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        // Parse YAML to serde_json::Value to reuse the json logic
        let json_val: serde_json::Value = serde_yaml::from_str(&s).map_err(|e| format!("YAML Parse Error: {}", e))?;
        
        fn json_to_vyauma(heap: &mut vre_core::vm::memory::Heap, json: &serde_json::Value) -> Result<vre_core::vm::value::Value, String> {
            match json {
                serde_json::Value::Null => Ok(vre_core::vm::value::Value::Null),
                serde_json::Value::Bool(b) => Ok(vre_core::vm::value::Value::Bool(*b)),
                serde_json::Value::Number(n) => Ok(vre_core::vm::value::Value::Float64(n.as_f64().unwrap_or(0.0))),
                serde_json::Value::String(s) => Ok(vre_core::vm::value::Value::String(s.clone())),
                serde_json::Value::Array(arr) => {
                    let mut v_arr = Vec::new();
                    for item in arr { v_arr.push(json_to_vyauma(heap, item)?); }
                    let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
                    Ok(vre_core::vm::value::Value::Reference(heap.allocate(obj)))
                }
                serde_json::Value::Object(obj) => {
                    let mut v_map = std::collections::HashMap::new();
                    for (k, v) in obj { v_map.insert(k.clone(), json_to_vyauma(heap, v)?); }
                    let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                    Ok(vre_core::vm::value::Value::Reference(heap.allocate(h_obj)))
                }
            }
        }
        json_to_vyauma(heap, &json_val)
    });

    config.insert_ffi("ffi_yaml_stringify".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_yaml_stringify expects 1 argument".to_string()); }
        let root = args.pop().unwrap();
        
        fn vyauma_to_json(heap: &vre_core::vm::memory::Heap, value: &vre_core::vm::value::Value) -> Result<serde_json::Value, String> {
            match value {
                vre_core::vm::value::Value::Null => Ok(serde_json::Value::Null),
                vre_core::vm::value::Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
                vre_core::vm::value::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
                vre_core::vm::value::Value::Float64(n) => {
                    if let Some(num) = serde_json::Number::from_f64(*n) { Ok(serde_json::Value::Number(num)) } else { Err("Invalid number".to_string()) }
                }
                vre_core::vm::value::Value::Reference(id) => {
                    let obj = heap.get(*id).map_err(|_| "Invalid heap reference".to_string())?;
                    match obj {
                        vre_core::vm::memory::HeapObject::Array(arr) => {
                            let mut j_arr = Vec::new();
                            for item in arr { j_arr.push(vyauma_to_json(heap, item)?); }
                            Ok(serde_json::Value::Array(j_arr))
                        }
                        vre_core::vm::memory::HeapObject::Struct(map) => {
                            let mut j_map = serde_json::Map::new();
                            for (k, v) in map { j_map.insert(k.clone(), vyauma_to_json(heap, v)?); }
                            Ok(serde_json::Value::Object(j_map))
                        }
                        vre_core::vm::memory::HeapObject::String(s) => Ok(serde_json::Value::String(s.clone())),
                        _ => Err("Unsupported heap object".to_string()),
                    }
                }
                _ => Err("Unsupported Vyauma value".to_string()),
            }
        }
        let json_val = vyauma_to_json(heap, &root)?;
        let s = serde_yaml::to_string(&json_val).map_err(|e| format!("YAML Stringify Error: {}", e))?;
        Ok(vre_core::vm::value::Value::String(s))
    });

    config.insert_ffi("ffi_toml_parse".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_toml_parse expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        // TOML -> JSON -> Vyauma (to reuse logic)
        let json_val: serde_json::Value = toml::from_str(&s).map_err(|e| format!("TOML Parse Error: {}", e))?;
        
        fn json_to_vyauma(heap: &mut vre_core::vm::memory::Heap, json: &serde_json::Value) -> Result<vre_core::vm::value::Value, String> {
            match json {
                serde_json::Value::Null => Ok(vre_core::vm::value::Value::Null),
                serde_json::Value::Bool(b) => Ok(vre_core::vm::value::Value::Bool(*b)),
                serde_json::Value::Number(n) => Ok(vre_core::vm::value::Value::Float64(n.as_f64().unwrap_or(0.0))),
                serde_json::Value::String(s) => Ok(vre_core::vm::value::Value::String(s.clone())),
                serde_json::Value::Array(arr) => {
                    let mut v_arr = Vec::new();
                    for item in arr { v_arr.push(json_to_vyauma(heap, item)?); }
                    let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
                    Ok(vre_core::vm::value::Value::Reference(heap.allocate(obj)))
                }
                serde_json::Value::Object(obj) => {
                    let mut v_map = std::collections::HashMap::new();
                    for (k, v) in obj { v_map.insert(k.clone(), json_to_vyauma(heap, v)?); }
                    let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                    Ok(vre_core::vm::value::Value::Reference(heap.allocate(h_obj)))
                }
            }
        }
        json_to_vyauma(heap, &json_val)
    });

    config.insert_ffi("ffi_toml_stringify".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_toml_stringify expects 1 argument".to_string()); }
        let root = args.pop().unwrap();
        
        fn vyauma_to_json(heap: &vre_core::vm::memory::Heap, value: &vre_core::vm::value::Value) -> Result<serde_json::Value, String> {
            match value {
                vre_core::vm::value::Value::Null => Ok(serde_json::Value::Null),
                vre_core::vm::value::Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
                vre_core::vm::value::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
                vre_core::vm::value::Value::Float64(n) => {
                    if let Some(num) = serde_json::Number::from_f64(*n) { Ok(serde_json::Value::Number(num)) } else { Err("Invalid number".to_string()) }
                }
                vre_core::vm::value::Value::Reference(id) => {
                    let obj = heap.get(*id).map_err(|_| "Invalid heap reference".to_string())?;
                    match obj {
                        vre_core::vm::memory::HeapObject::Array(arr) => {
                            let mut j_arr = Vec::new();
                            for item in arr { j_arr.push(vyauma_to_json(heap, item)?); }
                            Ok(serde_json::Value::Array(j_arr))
                        }
                        vre_core::vm::memory::HeapObject::Struct(map) => {
                            let mut j_map = serde_json::Map::new();
                            for (k, v) in map { j_map.insert(k.clone(), vyauma_to_json(heap, v)?); }
                            Ok(serde_json::Value::Object(j_map))
                        }
                        vre_core::vm::memory::HeapObject::String(s) => Ok(serde_json::Value::String(s.clone())),
                        _ => Err("Unsupported heap object".to_string()),
                    }
                }
                _ => Err("Unsupported Vyauma value".to_string()),
            }
        }
        let json_val = vyauma_to_json(heap, &root)?;
        let s = toml::to_string(&json_val).map_err(|e| format!("TOML Stringify Error: {}", e))?;
        Ok(vre_core::vm::value::Value::String(s))
    });

    // --- Phase 4 File APIs (Directory) ---

    config.insert_ffi("ffi_fs_create_dir".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_create_dir expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        let success = vre_core::pal::get_pal().create_dir_all(std::path::Path::new(&path)).is_ok();
        Ok(vre_core::vm::value::Value::Bool(success))
    });

    config.insert_ffi("ffi_fs_is_dir".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_is_dir expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        Ok(vre_core::vm::value::Value::Bool(vre_core::pal::get_pal().is_dir(std::path::Path::new(&path))))
    });

    config.insert_ffi("ffi_fs_read_dir".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_fs_read_dir expects 1 argument".to_string()); }
        let path = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected path string".to_string()),
        };
        match vre_core::pal::get_pal().read_dir(std::path::Path::new(&path)) {
            Ok(entries) => {
                let mut v_arr = Vec::new();
                for e in entries {
                    v_arr.push(vre_core::vm::value::Value::String(e.to_string_lossy().to_string()));
                }
                let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
                Ok(vre_core::vm::value::Value::Reference(heap.allocate(obj)))
            }
            Err(e) => Err(format!("read_dir error: {}", e)),
        }
    });

    // --- Phase 4 Networking (HTTP) ---

    config.insert_ffi("ffi_http_get".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_http_get expects 1 argument".to_string()); }
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected url string".to_string()),
        };
        
        let empty_headers = std::collections::HashMap::new();
        match vre_core::pal::get_pal().http_get(&url, &empty_headers) {
            Ok(res) => {
                let mut v_map = std::collections::HashMap::new();
                v_map.insert("status".to_string(), vre_core::vm::value::Value::Float64(res.status as f64));
                v_map.insert("body".to_string(), vre_core::vm::value::Value::String(res.body));
                
                let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                Ok(vre_core::vm::value::Value::Reference(heap.allocate(h_obj)))
            }
            Err(e) => Err(format!("http_get error: {}", e)),
        }
    });

    config.insert_ffi("ffi_http_post".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_http_post expects 2 arguments (url, body_str)".to_string()); }
        let body = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected body string".to_string()),
        };
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected url string".to_string()),
        };
        
        let empty_headers = std::collections::HashMap::new();
        match vre_core::pal::get_pal().http_post(&url, &empty_headers, &body) {
            Ok(res) => {
                let mut v_map = std::collections::HashMap::new();
                v_map.insert("status".to_string(), vre_core::vm::value::Value::Float64(res.status as f64));
                v_map.insert("body".to_string(), vre_core::vm::value::Value::String(res.body));
                
                let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                Ok(vre_core::vm::value::Value::Reference(heap.allocate(h_obj)))
            }
            Err(e) => Err(format!("http_post error: {}", e)),
        }
    });

    // --- Phase 4 Networking (WebSocket) ---

    config.insert_ffi("ffi_ws_connect".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_ws_connect expects 1 argument".to_string()); }
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected url string".to_string()),
        };
        match vre_core::pal::get_pal().ws_connect(&url) {
            Ok(handle) => Ok(vre_core::vm::value::Value::Float64(handle as f64)),
            Err(e) => Err(format!("ws_connect error: {}", e)),
        }
    });

    config.insert_ffi("ffi_ws_send".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_ws_send expects 2 arguments".to_string()); }
        let msg = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string message".to_string()),
        };
        let handle = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as usize,
            _ => return Err("Expected handle number".to_string()),
        };
        
        match vre_core::pal::get_pal().ws_send(handle, vre_core::pal::WsMessage::Text(msg)) {
            Ok(_) => Ok(vre_core::vm::value::Value::Bool(true)),
            Err(e) => Err(format!("ws_send error: {}", e)),
        }
    });

    config.insert_ffi("ffi_ws_recv".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_ws_recv expects 1 argument".to_string()); }
        let handle = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as usize,
            _ => return Err("Expected handle number".to_string()),
        };
        
        match vre_core::pal::get_pal().ws_recv(handle) {
            Ok(vre_core::pal::WsMessage::Text(s)) => Ok(vre_core::vm::value::Value::String(s)),
            Ok(vre_core::pal::WsMessage::Binary(_)) => Ok(vre_core::vm::value::Value::String("[Binary Data]".to_string())),
            Ok(vre_core::pal::WsMessage::Close) => Ok(vre_core::vm::value::Value::Null),
            Err(e) => Err(format!("ws_recv error: {}", e)),
        }
    });

    config.insert_ffi("ffi_ws_close".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_ws_close expects 1 argument".to_string()); }
        let handle = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as usize,
            _ => return Err("Expected handle number".to_string()),
        };
        
        match vre_core::pal::get_pal().ws_close(handle) {
            Ok(_) => Ok(vre_core::vm::value::Value::Bool(true)),
            Err(e) => Err(format!("ws_close error: {}", e)),
        }
    });

    // --- Phase 4 Process / Timers ---

    config.insert_ffi("ffi_sleep".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_sleep expects 1 argument".to_string()); }
        let ms = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => n as u64,
            _ => return Err("Expected ms number".to_string()),
        };
        vre_core::pal::get_pal().sleep_ms(ms);
        Ok(vre_core::vm::value::Value::Null)
    });

    config.insert_ffi("ffi_array_remove".to_string(), |heap, mut args| {
        if args.len() != 2 { return Err("ffi_array_remove expects 2 arguments".to_string()); }
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
                Ok(arr.remove(idx))
            } else {
                Ok(vre_core::vm::value::Value::Null)
            }
        } else {
            Err("Reference is not an array".to_string())
        }
    });

    // Helper functions for XML/TOML since we can't easily reuse the inner closures of JSON
    fn build_vyauma(heap: &mut vre_core::vm::memory::Heap, json: &serde_json::Value) -> Result<vre_core::vm::value::Value, String> {
        match json {
            serde_json::Value::Null => Ok(vre_core::vm::value::Value::Null),
            serde_json::Value::Bool(b) => Ok(vre_core::vm::value::Value::Bool(*b)),
            serde_json::Value::Number(n) => Ok(vre_core::vm::value::Value::Float64(n.as_f64().unwrap_or(0.0))),
            serde_json::Value::String(s) => Ok(vre_core::vm::value::Value::String(s.clone())),
            serde_json::Value::Array(arr) => {
                let mut v_arr = Vec::new();
                for item in arr { v_arr.push(build_vyauma(heap, item)?); }
                let obj = vre_core::vm::memory::HeapObject::Array(v_arr);
                Ok(vre_core::vm::value::Value::Reference(heap.allocate(obj)))
            }
            serde_json::Value::Object(obj) => {
                let mut v_map = std::collections::HashMap::new();
                for (k, v) in obj { v_map.insert(k.clone(), build_vyauma(heap, v)?); }
                let h_obj = vre_core::vm::memory::HeapObject::Struct(v_map);
                Ok(vre_core::vm::value::Value::Reference(heap.allocate(h_obj)))
            }
        }
    }

    fn dump_vyauma(heap: &vre_core::vm::memory::Heap, value: &vre_core::vm::value::Value) -> Result<serde_json::Value, String> {
        match value {
            vre_core::vm::value::Value::Null => Ok(serde_json::Value::Null),
            vre_core::vm::value::Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            vre_core::vm::value::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
            vre_core::vm::value::Value::Float64(n) => {
                if let Some(num) = serde_json::Number::from_f64(*n) { Ok(serde_json::Value::Number(num)) } else { Err("Invalid number".to_string()) }
            }
            vre_core::vm::value::Value::Reference(id) => {
                let obj = heap.get(*id).map_err(|_| "Invalid heap reference".to_string())?;
                match obj {
                    vre_core::vm::memory::HeapObject::Array(arr) => {
                        let mut j_arr = Vec::new();
                        for item in arr { j_arr.push(dump_vyauma(heap, item)?); }
                        Ok(serde_json::Value::Array(j_arr))
                    }
                    vre_core::vm::memory::HeapObject::Struct(map) => {
                        let mut j_map = serde_json::Map::new();
                        for (k, v) in map { j_map.insert(k.clone(), dump_vyauma(heap, v)?); }
                        Ok(serde_json::Value::Object(j_map))
                    }
                    vre_core::vm::memory::HeapObject::String(s) => Ok(serde_json::Value::String(s.clone())),
                    _ => Err("Unsupported heap object".to_string()),
                }
            }
            _ => Err("Unsupported type".to_string()),
        }
    }

    config.insert_ffi("ffi_toml_parse".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_toml_parse expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        let toml_val: serde_json::Value = toml::from_str(&s).map_err(|e| e.to_string())?;
        build_vyauma(heap, &toml_val)
    });

    config.insert_ffi("ffi_toml_stringify".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_toml_stringify expects 1 argument".to_string()); }
        let val = args.pop().unwrap();
        let j = dump_vyauma(heap, &val)?;
        let s = toml::to_string(&j).map_err(|e| e.to_string())?;
        Ok(vre_core::vm::value::Value::String(s))
    });

    config.insert_ffi("ffi_xml_parse".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_xml_parse expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        // Use quick-xml de::from_str to map to json value
        // Note: For simple documents this maps cleanly
        let xml_val: serde_json::Value = quick_xml::de::from_str(&s).map_err(|e| e.to_string())?;
        build_vyauma(heap, &xml_val)
    });

    config.insert_ffi("ffi_xml_stringify".to_string(), |heap, mut args| {
        if args.len() != 1 { return Err("ffi_xml_stringify expects 1 argument".to_string()); }
        let val = args.pop().unwrap();
        let j = dump_vyauma(heap, &val)?;
        match quick_xml::se::to_string(&j) {
            Ok(s) => Ok(vre_core::vm::value::Value::String(s)),
            Err(e) => Err(e.to_string()),
        }
    });

    config.insert_ffi("ffi_base64_encode".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_base64_encode expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let b64 = STANDARD.encode(s.as_bytes());
        Ok(vre_core::vm::value::Value::String(b64))
    });

    config.insert_ffi("ffi_base64_decode".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_base64_decode expects 1 argument".to_string()); }
        let s = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let bytes = STANDARD.decode(s).map_err(|e| e.to_string())?;
        let decoded = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        Ok(vre_core::vm::value::Value::String(decoded))
    });

    config.insert_ffi("ffi_http_get".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_http_get expects 1 argument (url)".to_string()); }
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string URL".to_string()),
        };
        
        match ureq::get(&url).call() {
            Ok(res) => {
                let body = res.into_string().unwrap_or_default();
                Ok(vre_core::vm::value::Value::String(body))
            }
            Err(e) => Err(format!("HTTP GET failed: {}", e)),
        }
    });

    config.insert_ffi("ffi_http_post".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_http_post expects 2 arguments (url, body)".to_string()); }
        let body = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string body".to_string()),
        };
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string URL".to_string()),
        };
        
        match ureq::post(&url).set("Content-Type", "application/json").send_string(&body) {
            Ok(res) => {
                let resp_body = res.into_string().unwrap_or_default();
                Ok(vre_core::vm::value::Value::String(resp_body))
            }
            Err(e) => Err(format!("HTTP POST failed: {}", e)),
        }
    });

    // --- Phase 25: Testing Assertions ---
    config.insert_ffi("ffi_assert".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_assert expects 2 arguments (condition, message)".to_string()); }
        let msg = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string message".to_string()),
        };
        let cond = match args.pop().unwrap() {
            vre_core::vm::value::Value::Bool(b) => b,
            _ => return Err("Expected bool condition".to_string()),
        };
        if cond {
            Ok(vre_core::vm::value::Value::Null)
        } else {
            Err(format!("Assertion failed: {}", msg))
        }
    });

    config.insert_ffi("ffi_assert_eq".to_string(), |heap, mut args| {
        if args.len() != 3 { return Err("ffi_assert_eq expects 3 arguments (actual, expected, message)".to_string()); }
        let msg = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string message".to_string()),
        };
        let expected = args.pop().unwrap();
        let actual = args.pop().unwrap();
        
        let are_equal = match (&actual, &expected) {
            (vre_core::vm::value::Value::Null, vre_core::vm::value::Value::Null) => true,
            (vre_core::vm::value::Value::Bool(a), vre_core::vm::value::Value::Bool(b)) => a == b,
            (vre_core::vm::value::Value::Float64(a), vre_core::vm::value::Value::Float64(b)) => (a - b).abs() < f64::EPSILON,
            (vre_core::vm::value::Value::String(a), vre_core::vm::value::Value::String(b)) => a == b,
            // For references we could do a deep compare but for now we'll do reference equality
            (vre_core::vm::value::Value::Reference(a), vre_core::vm::value::Value::Reference(b)) => {
                if a == b {
                    true
                } else {
                    // Try to deep compare strings inside heap
                    let obj_a = heap.get(*a);
                    let obj_b = heap.get(*b);
                    if let (Ok(vre_core::vm::memory::HeapObject::String(s_a)), Ok(vre_core::vm::memory::HeapObject::String(s_b))) = (obj_a, obj_b) {
                        s_a == s_b
                    } else {
                        false
                    }
                }
            },
            _ => false,
        };

        if are_equal {
            Ok(vre_core::vm::value::Value::Null)
        } else {
            Err(format!("Assertion failed: {} (Expected {:?}, got {:?})", msg, expected, actual))
        }
    });

    // --- Phase 26: Standard Library Expansions ---

    // ffi_crypto_sha256
    config.insert_ffi("ffi_crypto_sha256".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_sha256 expects 1 argument".to_string()); }
        let data = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string argument".to_string()),
        };
        let hash = vre_core::crypto::sha256(data.as_bytes());
        Ok(vre_core::vm::value::Value::String(hash))
    });

    // ffi_crypto_random_bytes (returns Base64 encoded string)
    config.insert_ffi("ffi_crypto_random_bytes".to_string(), |_heap, mut args| {
        if args.len() != 1 { return Err("ffi_crypto_random_bytes expects 1 argument (length)".to_string()); }
        let len = match args.pop().unwrap() {
            vre_core::vm::value::Value::Float64(n) => {
                if n < 0.0 || n > 1024.0 * 1024.0 {
                    return Err("Invalid random bytes length".to_string());
                }
                n as usize
            },
            _ => return Err("Expected numeric length".to_string()),
        };
        let bytes = vre_core::crypto::random_bytes(len);
        let b64 = base64::encode(bytes);
        Ok(vre_core::vm::value::Value::String(b64))
    });

    // ffi_regex_is_match
    config.insert_ffi("ffi_regex_is_match".to_string(), |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_regex_is_match expects 2 arguments (pattern, text)".to_string()); }
        let text = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string text".to_string()),
        };
        let pattern = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string pattern".to_string()),
        };
        let re = regex::Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(vre_core::vm::value::Value::Bool(re.is_match(&text)))
    });

    // ffi_regex_replace
    config.insert_ffi("ffi_regex_replace".to_string(), |_heap, mut args| {
        if args.len() != 3 { return Err("ffi_regex_replace expects 3 arguments (pattern, text, replacement)".to_string()); }
        let replacement = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string replacement".to_string()),
        };
        let text = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string text".to_string()),
        };
        let pattern = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string pattern".to_string()),
        };
        let re = regex::Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        let result = re.replace_all(&text, replacement.as_str()).to_string();
        Ok(vre_core::vm::value::Value::String(result))
    });

    // --- Phase 27: Native Document Database API ---

    config.register_ffi("ffi_db_insert", |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_db_insert expects 2 arguments (collection, document_json)".to_string()); }
        let doc_json = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected JSON string for document".to_string()),
        };
        let col = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for collection".to_string()),
        };
        
        let doc_val: serde_json::Value = serde_json::from_str(&doc_json).map_err(|e| format!("Invalid JSON: {}", e))?;
        let db = vre_core::db::DocumentDatabase::new("vre_data");
        match db.insert(&col, doc_val) {
            Ok(id) => Ok(vre_core::vm::value::Value::String(id)),
            Err(e) => Err(format!("DB Insert error: {}", e)),
        }
    }, vec![vre_core::capability::capability::Capability::new("db.write")]);

    config.register_ffi("ffi_string_join", |heap, mut args| {
        let delimiter = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string".to_string()),
        };
        let array = args.pop().unwrap();
        if let vre_core::vm::value::Value::Reference(ptr) = array {
            if let Ok(vre_core::vm::memory::HeapObject::Array(arr)) = heap.get(ptr) {
                let parts: Vec<String> = arr.iter().map(|v| match v {
                    vre_core::vm::value::Value::String(s) => s.clone(),
                    _ => "".to_string(),
                }).collect();
                Ok(vre_core::vm::value::Value::String(parts.join(&delimiter)))
            } else {
                Ok(vre_core::vm::value::Value::String("".to_string()))
            }
        } else {
            Ok(vre_core::vm::value::Value::String("".to_string()))
        }
    }, vec![]);

    config.register_ffi("ffi_db_find", |_heap, mut args| {
        if args.len() != 3 { return Err("ffi_db_find expects 3 arguments (collection, filter_key, filter_value)".to_string()); }
        let filter_val = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for filter value".to_string()),
        };
        let filter_key = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for filter key".to_string()),
        };
        let col = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for collection".to_string()),
        };

        let db = vre_core::db::DocumentDatabase::new("vre_data");
        let results = db.find(&col, &filter_key, &filter_val);
        let res_json = serde_json::to_string(&results).map_err(|e| format!("JSON Serialization error: {}", e))?;
        Ok(vre_core::vm::value::Value::String(res_json))
    }, vec![vre_core::capability::capability::Capability::new("db.read")]);

    config.register_ffi("ffi_db_delete", |_heap, mut args| {
        if args.len() != 3 { return Err("ffi_db_delete expects 3 arguments (collection, filter_key, filter_value)".to_string()); }
        let filter_val = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for filter value".to_string()),
        };
        let filter_key = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for filter key".to_string()),
        };
        let col = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string for collection".to_string()),
        };

        let db = vre_core::db::DocumentDatabase::new("vre_data");
        match db.delete(&col, &filter_key, &filter_val) {
            Ok(b) => Ok(vre_core::vm::value::Value::Bool(b)),
            Err(e) => Err(format!("DB Delete error: {}", e)),
        }
    }, vec![vre_core::capability::capability::Capability::new("db.write")]);

    // --- Phase 28: Task Concurrency APIs ---
    
    // NOTE: ffi_task_sleep is intercepted by the VM in OpCode::CallNative!
    // This closure will NEVER actually be called. We register it merely to satisfy the configuration 
    // registry so the bytecode loader maps the name correctly.
    config.insert_ffi("ffi_task_sleep".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::Null)
    });

    // --- Phase 31: Dynamic Task Spawning ---

    // NOTE: ffi_task_spawn is intercepted by the VM before the closure is invoked.
    // Registered here so it resolves in the native import table.
    config.insert_ffi("ffi_task_spawn".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::Null)
    });

    // NOTE: ffi_task_await is intercepted by the VM before invocation
    config.insert_ffi("ffi_task_await".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::Null)
    });

    // NOTE: ffi_set_timeout is intercepted by the VM before invocation
    config.insert_ffi("ffi_set_timeout".to_string(), |_heap, _args| {
        Ok(vre_core::vm::value::Value::Null)
    });
    // --- Relational DB FFIs ---
    config.register_ffi("ffi_sql_connect", |_heap, mut args| {
        if args.len() != 2 { return Err("ffi_sql_connect expects 2 args (driver, url)".to_string()); }
        let url = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string url".to_string()),
        };
        let driver = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string driver".to_string()),
        };
        match vre_core::db::db_connect(&driver, &url) {
            Ok(id) => Ok(vre_core::vm::value::Value::Int32(id)),
            Err(e) => Err(e),
        }
    }, vec![vre_core::capability::capability::Capability::new("db.write")]);

    config.register_ffi("ffi_sql_query", |heap, mut args| {
        if args.len() != 3 { return Err("ffi_sql_query expects 3 args (pool_id, sql, params)".to_string()); }
        let params_val = args.pop().unwrap();
        let sql = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string sql".to_string()),
        };
        let pool_id = match args.pop().unwrap() {
            vre_core::vm::value::Value::Int32(i) => i,
            _ => return Err("Expected integer pool_id".to_string()),
        };
        
        let mut rust_params = Vec::new();
        if let vre_core::vm::value::Value::Reference(r) = params_val {
            if let Ok(vre_core::vm::memory::HeapObject::Array(arr)) = heap.get(r) {
                rust_params = arr.clone();
            }
        }

        match vre_core::db::db_query(pool_id, &sql, rust_params, heap) {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }, vec![vre_core::capability::capability::Capability::new("db.read")]);

    config.register_ffi("ffi_sql_execute", |heap, mut args| {
        if args.len() != 3 { return Err("ffi_sql_execute expects 3 args (pool_id, sql, params)".to_string()); }
        let params_val = args.pop().unwrap();
        let sql = match args.pop().unwrap() {
            vre_core::vm::value::Value::String(s) => s,
            _ => return Err("Expected string sql".to_string()),
        };
        let pool_id = match args.pop().unwrap() {
            vre_core::vm::value::Value::Int32(i) => i,
            _ => return Err("Expected integer pool_id".to_string()),
        };
        
        let mut rust_params = Vec::new();
        if let vre_core::vm::value::Value::Reference(r) = params_val {
            if let Ok(vre_core::vm::memory::HeapObject::Array(arr)) = heap.get(r) {
                rust_params = arr.clone();
            }
        }

        match vre_core::db::db_execute(pool_id, &sql, rust_params, heap) {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }, vec![vre_core::capability::capability::Capability::new("db.write")]);
}
