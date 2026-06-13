use crate::vm::memory::{Heap, HeapObject};
use crate::vm::value::Value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

fn json_to_vyauma(heap: &mut Heap, json: &JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int64(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float64(f)
            } else {
                Value::Null
            }
        }
        JsonValue::String(s) => Value::String(s.clone()),
        JsonValue::Array(arr) => {
            let mut v_arr = Vec::new();
            for item in arr {
                v_arr.push(json_to_vyauma(heap, item));
            }
            let id = heap.allocate(HeapObject::Array(v_arr));
            Value::Reference(id)
        }
        JsonValue::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_vyauma(heap, v));
            }
            let id = heap.allocate(HeapObject::Struct(map));
            Value::Reference(id)
        }
    }
}

fn vyauma_to_json(heap: &Heap, val: &Value) -> JsonValue {
    match val {
        Value::Null => JsonValue::Null,
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Int32(n) => JsonValue::Number(serde_json::Number::from(*n)),
        Value::Int64(n) => JsonValue::Number(serde_json::Number::from(*n)),
        Value::Float32(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n as f64) {
                JsonValue::Number(num)
            } else {
                JsonValue::Null
            }
        }
        Value::Float64(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                JsonValue::Number(num)
            } else {
                JsonValue::Null
            }
        }
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Reference(id) => {
            if let Ok(obj) = heap.get(*id) {
                match obj {
                    HeapObject::Array(arr) => {
                        let mut j_arr = Vec::new();
                        for item in arr {
                            j_arr.push(vyauma_to_json(heap, item));
                        }
                        JsonValue::Array(j_arr)
                    }
                    HeapObject::Struct(map) => {
                        let mut j_map = serde_json::Map::new();
                        for (k, v) in map {
                            j_map.insert(k.clone(), vyauma_to_json(heap, v));
                        }
                        JsonValue::Object(j_map)
                    }
                    HeapObject::String(s) => JsonValue::String(s.clone()),
                    _ => JsonValue::Null, // Function, Closure, Box cannot be serialized
                }
            } else {
                JsonValue::Null
            }
        }
        _ => JsonValue::Null,
    }
}

pub fn parse(heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("json.parse expects exactly 1 argument (json_string)".to_string());
    }

    if let Value::String(json_str) = &args[0] {
        match serde_json::from_str::<JsonValue>(json_str) {
            Ok(json_val) => Ok(json_to_vyauma(heap, &json_val)),
            Err(e) => Err(format!("json.parse failed: {}", e)),
        }
    } else {
        Err("json.parse argument must be a string".to_string())
    }
}

pub fn stringify(heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("json.stringify expects exactly 1 argument (object)".to_string());
    }

    let json_val = vyauma_to_json(heap, &args[0]);
    match serde_json::to_string(&json_val) {
        Ok(json_str) => Ok(Value::String(json_str)),
        Err(e) => Err(format!("json.stringify failed: {}", e)),
    }
}
