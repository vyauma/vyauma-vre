use super::NativeModule;
use crate::value::Value;
use crate::heap::{Heap, ObjectType, StructInstanceData};
use std::collections::HashMap;
use serde_json::Value as JsonValue;

fn get_string_arg(heap: &Heap, val: &Value) -> Result<String, String> {
    if let Value::HeapRef(handle) = val {
        let obj = heap.get(*handle);
        if let ObjectType::String(s) = &obj.obj_type {
            return Ok(s.clone());
        }
    }
    Err("Expected string argument".into())
}

fn json_to_value(heap: &mut Heap, json: JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Bool(b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        JsonValue::String(s) => {
            let handle = heap.allocate(ObjectType::String(s));
            Value::HeapRef(handle)
        }
        JsonValue::Array(arr) => {
            let mut v_arr = Vec::new();
            for item in arr {
                v_arr.push(json_to_value(heap, item));
            }
            let handle = heap.allocate(ObjectType::Array(v_arr));
            Value::HeapRef(handle)
        }
        JsonValue::Object(obj) => {
            // By default, parse into a Map
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_value(heap, v));
            }
            let handle = heap.allocate(ObjectType::Map(map));
            Value::HeapRef(handle)
        }
    }
}

fn value_to_json(heap: &Heap, value: &Value) -> Result<JsonValue, String> {
    match value {
        Value::Null => Ok(JsonValue::Null),
        Value::Bool(b) => Ok(JsonValue::Bool(*b)),
        Value::Int(i) => Ok(JsonValue::Number(serde_json::Number::from(*i))),
        Value::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                Ok(JsonValue::Number(n))
            } else {
                Err("Invalid float for JSON".into())
            }
        }
        Value::HeapRef(handle) => {
            let obj = heap.get(*handle);
            match &obj.obj_type {
                ObjectType::String(s) => Ok(JsonValue::String(s.clone())),
                ObjectType::Array(arr) => {
                    let mut j_arr = Vec::new();
                    for item in arr {
                        j_arr.push(value_to_json(heap, item)?);
                    }
                    Ok(JsonValue::Array(j_arr))
                }
                ObjectType::Map(map) => {
                    let mut j_obj = serde_json::Map::new();
                    for (k, v) in map {
                        j_obj.insert(k.clone(), value_to_json(heap, v)?);
                    }
                    Ok(JsonValue::Object(j_obj))
                }
                ObjectType::StructInstance(inst) => {
                    let mut j_obj = serde_json::Map::new();
                    for (k, v) in &inst.fields {
                        j_obj.insert(k.clone(), value_to_json(heap, v)?);
                    }
                    Ok(JsonValue::Object(j_obj))
                }
                _ => Err("Cannot serialize function or opaque object to JSON".into()),
            }
        }
        _ => Err("Cannot serialize internal value to JSON".into()),
    }
}

pub fn create_module() -> NativeModule {
    let mut module = NativeModule::new("json");

    module.define_function("parse", 1, |heap, args| {
        let json_str = get_string_arg(heap, &args[0])?;
        match serde_json::from_str::<JsonValue>(&json_str) {
            Ok(parsed) => Ok(json_to_value(heap, parsed)),
            Err(e) => Err(format!("Invalid JSON: {}", e)),
        }
    });

    module.define_function("stringify", 1, |heap, args| {
        let json_val = value_to_json(heap, &args[0])?;
        match serde_json::to_string_pretty(&json_val) {
            Ok(s) => {
                let handle = heap.allocate(ObjectType::String(s));
                Ok(Value::HeapRef(handle))
            }
            Err(e) => Err(format!("Failed to stringify: {}", e)),
        }
    });

    module
}
