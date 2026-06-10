pub trait FileSystem {
    fn read_file(&self, path: &str) -> Result<String, String>;
    fn write_file(&self, path: &str, content: &str) -> Result<(), String>;
    fn exists(&self, path: &str) -> bool;
}

pub trait Networking {
    fn connect(&self, address: &str) -> Result<(), String>;
    fn listen(&self, address: &str) -> Result<(), String>;
}

pub trait ProcessManagement {
    fn spawn(&self, command: &str, args: &[&str]) -> Result<u32, String>;
    fn kill(&self, pid: u32) -> Result<(), String>;
}

pub trait Timers {
    fn sleep(&self, ms: u64);
    fn now_ms(&self) -> u64;
}

pub trait Environment {
    fn get_env(&self, key: &str) -> Option<String>;
    fn set_env(&self, key: &str, value: &str);
}

pub trait Console {
    fn print(&self, msg: &str);
    fn eprint(&self, msg: &str);
}

pub trait RuntimeServices {
    fn fs(&self) -> &dyn FileSystem;
    fn net(&self) -> &dyn Networking;
    fn process(&self) -> &dyn ProcessManagement;
    fn timers(&self) -> &dyn Timers;
    fn env(&self) -> &dyn Environment;
    fn console(&self) -> &dyn Console;
}
