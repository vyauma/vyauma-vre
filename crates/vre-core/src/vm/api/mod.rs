pub mod fs;
pub mod timers;
pub mod tests;

use std::collections::HashMap;
use crate::vm::memory::Heap;
use crate::vm::value::Value;

pub type NativeFunction = fn(&mut Heap, Vec<Value>) -> Result<Value, String>;

use crate::config::VreConfig;

use crate::capability::capability::Capability;

pub fn register_apis(config: &mut VreConfig) {
    // Register fs APIs
    config.register_ffi("fs.readFile", fs::read_file, vec![Capability::new("fs.read")]);
    config.register_ffi("fs.writeFile", fs::write_file, vec![Capability::new("fs.write")]);

    // Register timer APIs
    config.insert_ffi("timers.setTimeout".to_string(), timers::set_timeout);
}
