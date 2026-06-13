pub mod fs;
pub mod timers;
pub mod tests;
pub mod db;
pub mod json;
pub mod http;

use std::collections::HashMap;
use crate::vm::memory::Heap;
use crate::vm::value::Value;

pub type NativeFunction = fn(&mut Heap, Vec<Value>) -> Result<Value, String>;

use crate::config::VreConfig;

use crate::capability::capability::Capability;

pub fn register_apis(config: &mut VreConfig) {
    // Register fs APIs
    config.register_ffi("ffi_fs_read_file", fs::read_file, vec![Capability::new("fs.read")]);
    config.register_ffi("ffi_fs_write_file", fs::write_file, vec![Capability::new("fs.write")]);
    config.register_ffi("ffi_fs_append_file", fs::append_file, vec![Capability::new("fs.write")]);
    config.register_ffi("ffi_fs_exists", fs::exists, vec![Capability::new("fs.read")]);
    config.register_ffi("ffi_fs_delete", fs::delete, vec![Capability::new("fs.write")]);
    config.register_ffi("ffi_fs_size", fs::size, vec![Capability::new("fs.read")]);

    // Register timer APIs
    config.insert_ffi("timers.setTimeout".to_string(), timers::set_timeout);
    
    // Register db APIs
    config.register_ffi("ffi_db_connect", db::connect, vec![Capability::new("db.access")]);
    config.register_ffi("ffi_db_query", db::query, vec![Capability::new("db.access")]);
    config.register_ffi("ffi_db_execute", db::execute, vec![Capability::new("db.access")]);

    // Register json APIs
    config.insert_ffi("ffi_json_parse".to_string(), json::parse);
    config.insert_ffi("ffi_json_stringify".to_string(), json::stringify);

    // Register http APIs
    config.register_ffi("ffi_http_get", http::get, vec![Capability::new("net.request")]);
    config.register_ffi("ffi_http_post", http::post, vec![Capability::new("net.request")]);
}
