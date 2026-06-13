use super::NativeModule;
use crate::value::Value;
use crate::heap::ObjectType;
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::time::Duration;

pub fn create_module() -> NativeModule {
    let mut module = NativeModule::new("time");

    module.define_function("now", 0, |_heap, _args| {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Ok(Value::Int(since_the_epoch.as_millis() as i64))
    });

    module.define_function("timestamp", 0, |_heap, _args| {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Ok(Value::Int(since_the_epoch.as_secs() as i64))
    });

    module.define_function("sleep", 1, |_heap, args| {
        if let Value::Int(ms) = args[0] {
            thread::sleep(Duration::from_millis(ms as u64));
            Ok(Value::Null)
        } else {
            Err("Expected integer milliseconds for sleep".into())
        }
    });

    module.define_function("format", 1, |heap, args| {
        if let Value::Int(ms) = args[0] {
            // Simplified format since standard Rust requires chrono for real ISO strings
            let s = format!("EPOCH_MS:{}", ms);
            let handle = heap.allocate(ObjectType::String(s));
            Ok(Value::HeapRef(handle))
        } else {
            Err("Expected integer timestamp".into())
        }
    });

    module
}
