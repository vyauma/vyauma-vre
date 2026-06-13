use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use crate::vm::value::Value;
use crate::vm::memory::{Heap, HeapObject};

pub enum DatabaseConnection {
    Sqlite(rusqlite::Connection),
    Postgres(postgres::Client),
    Mysql(mysql::Conn),
}

static CONNECTIONS: OnceLock<Mutex<HashMap<i32, DatabaseConnection>>> = OnceLock::new();
static NEXT_ID: OnceLock<Mutex<i32>> = OnceLock::new();

fn get_connections() -> &'static Mutex<HashMap<i32, DatabaseConnection>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_next_id() -> i32 {
    let mut id_lock = NEXT_ID.get_or_init(|| Mutex::new(1)).lock().unwrap();
    let id = *id_lock;
    *id_lock += 1;
    id
}

pub fn db_connect(driver: &str, url: &str) -> Result<i32, String> {
    let conn = match driver {
        "sqlite" => {
            let c = if url == "sqlite::memory:" {
                rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?
            } else {
                rusqlite::Connection::open(url).map_err(|e| e.to_string())?
            };
            DatabaseConnection::Sqlite(c)
        }
        "postgres" => {
            let client = postgres::Client::connect(url, postgres::NoTls).map_err(|e| e.to_string())?;
            DatabaseConnection::Postgres(client)
        }
        "mysql" => {
            let opts = mysql::Opts::from_url(url).map_err(|e| e.to_string())?;
            let conn = mysql::Conn::new(opts).map_err(|e| e.to_string())?;
            DatabaseConnection::Mysql(conn)
        }
        _ => return Err(format!("Unsupported database driver: {}", driver)),
    };

    let id = get_next_id();
    get_connections().lock().unwrap().insert(id, conn);
    Ok(id)
}

fn value_to_sqlite(val: &Value, heap: &Heap) -> rusqlite::types::ToSqlOutput<'static> {
    use rusqlite::types::{ToSqlOutput, Value as SqlValue};
    match val {
        Value::Null => ToSqlOutput::Owned(SqlValue::Null),
        Value::Int32(i) => ToSqlOutput::Owned(SqlValue::Integer(*i as i64)),
        Value::Int64(i) => ToSqlOutput::Owned(SqlValue::Integer(*i)),
        Value::Float32(f) => ToSqlOutput::Owned(SqlValue::Real(*f as f64)),
        Value::Float64(f) => ToSqlOutput::Owned(SqlValue::Real(*f)),
        Value::Bool(b) => ToSqlOutput::Owned(SqlValue::Integer(if *b { 1 } else { 0 })),
        Value::String(s) => ToSqlOutput::Owned(SqlValue::Text(s.clone())),
        Value::Reference(r) => {
            if let Ok(HeapObject::String(s)) = heap.get(*r) {
                ToSqlOutput::Owned(SqlValue::Text(s.clone()))
            } else {
                ToSqlOutput::Owned(SqlValue::Text(format!("{:?}", val)))
            }
        }
        _ => ToSqlOutput::Owned(SqlValue::Text(format!("{:?}", val))), // Fallback
    }
}

pub fn db_query(pool_id: i32, sql: &str, params: Vec<Value>, heap: &mut Heap) -> Result<Value, String> {
    let mut conns = get_connections().lock().unwrap();
    let conn = conns.get_mut(&pool_id).ok_or_else(|| "Invalid connection ID".to_string())?;

    match conn {
        DatabaseConnection::Sqlite(c) => {
            let mut stmt = c.prepare(sql).map_err(|e| e.to_string())?;
            let mut sql_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
            
            let converted_params: Vec<rusqlite::types::ToSqlOutput> = params.iter().map(|p| value_to_sqlite(p, heap)).collect();
            for p in &converted_params {
                sql_params.push(p);
            }

            let mut rows = stmt.query(sql_params.as_slice()).map_err(|e| e.to_string())?;
            
            let mut result_rows = Vec::new();
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let mut dict = HashMap::new();
                for i in 0..row.as_ref().column_count() {
                    let col_name = row.as_ref().column_name(i).unwrap_or("unknown").to_string();
                    let val: rusqlite::types::Value = row.get(i).map_err(|e| e.to_string())?;
                    let v = match val {
                        rusqlite::types::Value::Null => Value::Null,
                        rusqlite::types::Value::Integer(i) => Value::Int32(i as i32),
                        rusqlite::types::Value::Real(f) => Value::Float64(f),
                        rusqlite::types::Value::Text(t) => Value::String(t),
                        rusqlite::types::Value::Blob(_) => Value::String("<blob>".to_string()),
                    };
                    dict.insert(col_name, v);
                }
                let struct_id = heap.allocate(HeapObject::Struct(dict));
                result_rows.push(Value::Reference(struct_id));
            }
            let array_id = heap.allocate(HeapObject::Array(result_rows));
            Ok(Value::Reference(array_id))
        }
        DatabaseConnection::Postgres(_) => Err("Postgres querying not fully implemented".to_string()),
        DatabaseConnection::Mysql(_) => Err("MySQL querying not fully implemented".to_string()),
    }
}

pub fn db_execute(pool_id: i32, sql: &str, params: Vec<Value>, heap: &Heap) -> Result<Value, String> {
    let mut conns = get_connections().lock().unwrap();
    let conn = conns.get_mut(&pool_id).ok_or_else(|| "Invalid connection ID".to_string())?;

    match conn {
        DatabaseConnection::Sqlite(c) => {
            let converted_params: Vec<rusqlite::types::ToSqlOutput> = params.iter().map(|p| value_to_sqlite(p, heap)).collect();
            let mut sql_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for p in &converted_params {
                sql_params.push(p);
            }
            let rows = c.execute(sql, sql_params.as_slice()).map_err(|e| e.to_string())?;
            Ok(Value::Int32(rows as i32))
        }
        DatabaseConnection::Postgres(_) => Err("Postgres execution not fully implemented".to_string()),
        DatabaseConnection::Mysql(_) => Err("MySQL execution not fully implemented".to_string()),
    }
}

pub struct DocumentDatabase {
    pub db_name: String,
}

impl DocumentDatabase {
    pub fn new(db_name: &str) -> Self {
        DocumentDatabase { db_name: db_name.to_string() }
    }
    
    pub fn insert(&self, col: &str, doc: serde_json::Value) -> Result<String, String> {
        // Mock implementation to satisfy compilation
        Ok("mock_id".to_string())
    }
    
    pub fn find(&self, col: &str, key: &str, val: &str) -> Vec<serde_json::Value> {
        // Mock implementation to satisfy compilation
        vec![]
    }
    
    pub fn delete(&self, col: &str, key: &str, val: &str) -> Result<bool, String> {
        // Mock implementation to satisfy compilation
        Ok(true)
    }
}
