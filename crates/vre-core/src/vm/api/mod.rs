pub mod fs;
pub mod timers;
pub mod tests;

use std::collections::HashMap;
use crate::vm::memory::Heap;
use crate::vm::value::Value;

pub type NativeFunction = fn(&mut Heap, Vec<Value>) -> Result<Value, String>;

use crate::config::VreConfig;

pub fn register_apis(config: &mut VreConfig) {
    // Register fs APIs
    config.ffi_functions.insert("fs.readFile".to_string(), fs::read_file);
    config.ffi_functions.insert("fs.writeFile".to_string(), fs::write_file);

    // Register timer APIs
    config.ffi_functions.insert("timers.setTimeout".to_string(), timers::set_timeout);
}
