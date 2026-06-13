use crate::pal::{PlatformAbstractionLayer, HttpRequest, HttpResponse, WsHandle, WsMessage, TimerId, Signal};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::net::{UdpSocket, ToSocketAddrs, IpAddr};
use tokio::net::{TcpStream, TcpListener};

pub struct IosPal {
    // iOS specific state
}

impl Default for IosPal {
    fn default() -> Self {
        Self {}
    }
}

// Minimal stub for iOS PAL
#[async_trait::async_trait]
impl PlatformAbstractionLayer for IosPal {
    fn read_to_string(&self, _path: &Path) -> Result<String, String> { Err("iOS: Not implemented".into()) }
    fn write(&self, _path: &Path, _content: &str) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn append(&self, _path: &Path, _content: &str) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn remove_file(&self, _path: &Path) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn remove_dir_all(&self, _path: &Path) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn create_dir_all(&self, _path: &Path) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn exists(&self, _path: &Path) -> bool { false }
    fn is_file(&self, _path: &Path) -> bool { false }
    fn is_dir(&self, _path: &Path) -> bool { false }
    fn metadata_len(&self, _path: &Path) -> Result<u64, String> { Err("iOS: Not implemented".into()) }
    fn canonicalize(&self, _path: &Path) -> Result<PathBuf, String> { Err("iOS: Not implemented".into()) }
    fn open_file(&self, _path: &Path) -> Result<std::fs::File, String> { Err("iOS: Not implemented".into()) }
    fn rename_file(&self, _from: &Path, _to: &Path) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn copy_file(&self, _from: &Path, _to: &Path) -> Result<u64, String> { Err("iOS: Not implemented".into()) }
    fn read_dir(&self, _path: &Path) -> Result<Vec<PathBuf>, String> { Err("iOS: Not implemented".into()) }
    fn watch_file(&self, _path: &Path) -> Result<usize, String> { Err("iOS: Not implemented".into()) }

    fn print(&self, msg: &str) { print!("[iOS] {}", msg); }
    fn println(&self, msg: &str) { println!("[iOS] {}", msg); }
    fn eprintln(&self, msg: &str) { eprintln!("[iOS Error] {}", msg); }

    fn current_time_millis(&self) -> u64 { 0 }
    fn sleep_ms(&self, _ms: u64) {}
    fn set_timer(&self, _ms: u64, _callback: Box<dyn Fn() + Send + 'static>) -> TimerId { 0 }
    fn cancel_timer(&self, _id: TimerId) {}

    fn get_env_var(&self, _key: &str) -> Option<String> { None }
    fn set_env_var(&self, _key: &str, _value: &str) {}
    fn get_all_env_vars(&self) -> HashMap<String, String> { HashMap::new() }
    fn get_system_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("os".to_string(), "ios".to_string());
        info
    }

    fn spawn_process(&self, _command: &str, _args: &[&str]) -> Result<u32, String> { Err("iOS: Cannot spawn process".into()) }
    fn kill_process(&self, _pid: u32) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn send_signal(&self, _pid: u32, _signal: Signal) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn handle_signal(&self, _signal: Signal, _callback: Box<dyn Fn() + Send + 'static>) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn handle_interrupt(&self) -> Result<(), String> { Err("iOS: Not implemented".into()) }

    async fn tcp_connect(&self, _addr: &str) -> Result<TcpStream, String> { Err("iOS: Not implemented".into()) }
    async fn tcp_bind(&self, _addr: &str) -> Result<TcpListener, String> { Err("iOS: Not implemented".into()) }
    fn udp_bind(&self, _addr: &str) -> Result<UdpSocket, String> { Err("iOS: Not implemented".into()) }
    fn resolve_dns(&self, _hostname: &str) -> Result<Vec<IpAddr>, String> { Err("iOS: Not implemented".into()) }

    fn http_get(&self, _url: &str, _headers: &HashMap<String, String>) -> Result<HttpResponse, String> { Err("iOS: Not implemented".into()) }
    fn http_post(&self, _url: &str, _headers: &HashMap<String, String>, _body: &str) -> Result<HttpResponse, String> { Err("iOS: Not implemented".into()) }
    fn http_put(&self, _url: &str, _headers: &HashMap<String, String>, _body: &str) -> Result<HttpResponse, String> { Err("iOS: Not implemented".into()) }
    fn http_delete(&self, _url: &str, _headers: &HashMap<String, String>) -> Result<HttpResponse, String> { Err("iOS: Not implemented".into()) }
    fn http_request(&self, _req: HttpRequest) -> Result<HttpResponse, String> { Err("iOS: Not implemented".into()) }

    fn ws_connect(&self, _url: &str) -> Result<WsHandle, String> { Err("iOS: Not implemented".into()) }
    fn ws_send(&self, _handle: WsHandle, _msg: WsMessage) -> Result<(), String> { Err("iOS: Not implemented".into()) }
    fn ws_recv(&self, _handle: WsHandle) -> Result<WsMessage, String> { Err("iOS: Not implemented".into()) }
    fn ws_close(&self, _handle: WsHandle) -> Result<(), String> { Err("iOS: Not implemented".into()) }

    fn load_library(&self, _path: &str) -> Result<usize, String> { Err("iOS: Not implemented".into()) }
    fn resolve_symbol(&self, _lib: usize, _sym: &str) -> Result<usize, String> { Err("iOS: Not implemented".into()) }
    fn unload_library(&self, _lib: usize) -> Result<(), String> { Err("iOS: Not implemented".into()) }
}
