use crate::vm::memory::{Heap, HeapObject};
use crate::vm::value::Value;
use crate::db;

pub fn connect(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("db.connect expects 2 arguments (driver, url)".to_string());
    }
    let driver = if let Value::String(s) = &args[0] { s } else { return Err("driver must be string".into()) };
    let url = if let Value::String(s) = &args[1] { s } else { return Err("url must be string".into()) };
    
    let id = db::db_connect(driver, url)?;
    Ok(Value::Int32(id))
}

pub fn query(heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("db.query expects 3 arguments (pool_id, sql, params)".to_string());
    }
    let pool_id = match &args[0] {
        Value::Int32(i) => *i,
        Value::Float64(f) => *f as i32,
        _ => return Err("pool_id must be number".into())
    };
    let sql = if let Value::String(s) = &args[1] { s } else { return Err("sql must be string".into()) };
    
    let params = match &args[2] {
        Value::Reference(r) => {
            if let Ok(HeapObject::Array(arr)) = heap.get(*r) {
                arr.clone()
            } else {
                return Err("params must be an array".into());
            }
        }
        _ => return Err("params must be an array reference".into()),
    };
    
    db::db_query(pool_id, sql, params, heap)
}

pub fn execute(heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("db.execute expects 3 arguments (pool_id, sql, params)".to_string());
    }
    let pool_id = match &args[0] {
        Value::Int32(i) => *i,
        Value::Float64(f) => *f as i32,
        _ => return Err("pool_id must be number".into())
    };
    let sql = if let Value::String(s) = &args[1] { s } else { return Err("sql must be string".into()) };
    
    let params = match &args[2] {
        Value::Reference(r) => {
            if let Ok(HeapObject::Array(arr)) = heap.get(*r) {
                arr.clone()
            } else {
                return Err("params must be an array".into());
            }
        }
        _ => return Err("params must be an array reference".into()),
    };
    
    db::db_execute(pool_id, sql, params, heap)
}
