#[cfg(test)]
mod tests {
    use crate::config::VreConfig;
    use crate::vm::api;
    use crate::vm::memory::Heap;
    use crate::vm::value::Value;

    #[test]
    fn test_api_registration() {
        let mut config = VreConfig::default();
        api::register_apis(&mut config);
        
        assert!(config.ffi_functions.contains_key("fs.readFile"));
        assert!(config.ffi_functions.contains_key("fs.writeFile"));
        assert!(config.ffi_functions.contains_key("timers.setTimeout"));
    }

    #[test]
    fn test_fs_read_file_args_validation() {
        let mut heap = Heap::new();
        // Missing args
        let result = api::fs::read_file(&mut heap, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "fs.readFile expects exactly 1 argument (path)");

        // Wrong type
        let result = api::fs::read_file(&mut heap, vec![Value::Float64(42.0)]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "fs.readFile argument must be a string");
    }

    #[test]
    fn test_timers_setTimeout_args_validation() {
        let mut heap = Heap::new();
        // Missing args
        let result = api::timers::set_timeout(&mut heap, vec![Value::Float64(1.0)]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "timers.setTimeout expects exactly 2 arguments (callback_id, ms)");

        // Wrong type
        let result = api::timers::set_timeout(&mut heap, vec![Value::String("cb".to_string()), Value::Float64(100.0)]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "timers.setTimeout callback_id must be a number");
    }
}
