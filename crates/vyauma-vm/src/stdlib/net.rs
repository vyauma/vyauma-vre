use super::NativeModule;
use crate::value::Value;
use crate::heap::ObjectType;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref LISTENERS: Mutex<HashMap<i64, TcpListener>> = Mutex::new(HashMap::new());
    static ref STREAMS: Mutex<HashMap<i64, TcpStream>> = Mutex::new(HashMap::new());
    static ref NEXT_ID: Mutex<i64> = Mutex::new(1);
}

pub fn create_module() -> NativeModule {
    let mut module = NativeModule::new("net");

    module.define_function("listen", 1, |_heap, args| {
        if let Value::Int(port) = args[0] {
            let addr = format!("127.0.0.1:{}", port);
            match TcpListener::bind(&addr) {
                Ok(listener) => {
                    let mut id_gen = NEXT_ID.lock().unwrap();
                    let id = *id_gen;
                    *id_gen += 1;
                    
                    let mut listeners = LISTENERS.lock().unwrap();
                    listeners.insert(id, listener);
                    Ok(Value::Int(id))
                }
                Err(e) => Err(format!("Failed to bind to {}: {}", addr, e)),
            }
        } else {
            Err("Expected port number as int".into())
        }
    });

    module.define_function("accept", 1, |_heap, args| {
        if let Value::Int(listener_id) = args[0] {
            let listeners = LISTENERS.lock().unwrap();
            if let Some(listener) = listeners.get(&listener_id) {
                // Blocking accept
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let mut id_gen = NEXT_ID.lock().unwrap();
                        let id = *id_gen;
                        *id_gen += 1;
                        
                        let mut streams = STREAMS.lock().unwrap();
                        streams.insert(id, stream);
                        Ok(Value::Int(id))
                    }
                    Err(e) => Err(format!("Failed to accept connection: {}", e)),
                }
            } else {
                Err("Invalid listener ID".into())
            }
        } else {
            Err("Expected listener ID as int".into())
        }
    });

    module.define_function("read", 1, |heap, args| {
        if let Value::Int(stream_id) = args[0] {
            let mut streams = STREAMS.lock().unwrap();
            if let Some(stream) = streams.get_mut(&stream_id) {
                let mut buffer = [0; 1024];
                match stream.read(&mut buffer) {
                    Ok(size) => {
                        let s = String::from_utf8_lossy(&buffer[..size]).to_string();
                        let handle = heap.allocate(ObjectType::String(s));
                        Ok(Value::HeapRef(handle))
                    }
                    Err(e) => Err(format!("Failed to read stream: {}", e)),
                }
            } else {
                Err("Invalid stream ID".into())
            }
        } else {
            Err("Expected stream ID as int".into())
        }
    });

    module.define_function("write", 2, |heap, args| {
        if let Value::Int(stream_id) = args[0] {
            let mut streams = STREAMS.lock().unwrap();
            if let Some(stream) = streams.get_mut(&stream_id) {
                let data = match &args[1] {
                    Value::HeapRef(handle) => {
                        let obj = heap.get(*handle);
                        if let ObjectType::String(s) = &obj.obj_type {
                            s.clone()
                        } else {
                            return Err("Expected string to write".into());
                        }
                    }
                    _ => return Err("Expected string to write".into()),
                };
                
                match stream.write_all(data.as_bytes()) {
                    Ok(_) => Ok(Value::Null),
                    Err(e) => Err(format!("Failed to write stream: {}", e)),
                }
            } else {
                Err("Invalid stream ID".into())
            }
        } else {
            Err("Expected stream ID as int".into())
        }
    });

    module.define_function("close", 1, |_heap, args| {
        if let Value::Int(stream_id) = args[0] {
            let mut streams = STREAMS.lock().unwrap();
            streams.remove(&stream_id);
            Ok(Value::Null)
        } else {
            Err("Expected stream ID as int".into())
        }
    });

    module
}
