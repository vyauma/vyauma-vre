use vre_core::pal::{PlatformAbstractionLayer, set_pal};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::collections::HashMap;

struct MockPal {
    fs: Mutex<HashMap<String, String>>,
}

impl MockPal {
    fn new() -> Self {
        let mut fs = HashMap::new();
        fs.insert("main.vym".to_string(), "print(\"hello\");".to_string());
        Self { fs: Mutex::new(fs) }
    }
}

impl PlatformAbstractionLayer for MockPal {
    fn read_to_string(&self, path: &Path) -> Result<String, String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let fs = self.fs.lock().unwrap();
        if let Some(content) = fs.get(path_str.as_str()) {
            Ok(content.clone())
        } else {
            Err("File not found".to_string())
        }
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let mut fs = self.fs.lock().unwrap();
        fs.insert(path_str, content.to_string());
        Ok(())
    }

    fn append(&self, path: &Path, content: &str) -> Result<(), String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let mut fs = self.fs.lock().unwrap();
        let existing = fs.entry(path_str).or_insert_with(String::new);
        existing.push_str(content);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let mut fs = self.fs.lock().unwrap();
        if fs.remove(&path_str).is_some() { Ok(()) } else { Err("File not found".to_string()) }
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), String> { self.remove_file(path) }
    fn create_dir_all(&self, _path: &Path) -> Result<(), String> { Ok(()) }

    fn exists(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().replace("\\", "/");
        self.fs.lock().unwrap().contains_key(&path_str)
    }

    fn is_file(&self, path: &Path) -> bool { self.exists(path) }
    fn is_dir(&self, _path: &Path) -> bool { false }

    fn metadata_len(&self, path: &Path) -> Result<u64, String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let fs = self.fs.lock().unwrap();
        fs.get(&path_str).map(|c| c.len() as u64).ok_or("File not found".to_string())
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String> {
        Ok(PathBuf::from(path.to_string_lossy().replace("\\", "/")))
    }

    fn open_file(&self, _path: &Path) -> Result<std::fs::File, String> { Err("Mock open_file".to_string()) }
    fn rename_file(&self, _from: &Path, _to: &Path) -> Result<(), String> { Ok(()) }
    fn copy_file(&self, _from: &Path, _to: &Path) -> Result<u64, String> { Ok(0) }
    fn read_dir(&self, _path: &Path) -> Result<Vec<PathBuf>, String> { Ok(vec![]) }
    fn watch_file(&self, _path: &Path) -> Result<usize, String> { Ok(0) }

    fn print(&self, _msg: &str) {}
    fn println(&self, _msg: &str) {}
    fn eprintln(&self, _msg: &str) {}

    fn current_time_millis(&self) -> u64 { 1000 }
    fn sleep_ms(&self, _ms: u64) {}
    fn set_timer(&self, _ms: u64, _cb: Box<dyn Fn() + Send + 'static>) -> vre_core::pal::TimerId { 0 }
    fn cancel_timer(&self, _id: vre_core::pal::TimerId) {}

    fn get_env_var(&self, _key: &str) -> Option<String> { None }
    fn set_env_var(&self, _key: &str, _value: &str) {}
    fn get_all_env_vars(&self) -> HashMap<String, String> { HashMap::new() }
    fn get_system_info(&self) -> HashMap<String, String> { HashMap::new() }

    fn spawn_process(&self, _cmd: &str, _args: &[&str]) -> Result<u32, String> { Ok(0) }
    fn kill_process(&self, _pid: u32) -> Result<(), String> { Ok(()) }
    fn send_signal(&self, _pid: u32, _sig: vre_core::pal::Signal) -> Result<(), String> { Ok(()) }
    fn handle_signal(&self, _sig: vre_core::pal::Signal, _cb: Box<dyn Fn() + Send + 'static>) -> Result<(), String> { Ok(()) }
    fn handle_interrupt(&self) -> Result<(), String> { Ok(()) }

    fn tcp_connect(&self, _addr: &str) -> Result<std::net::TcpStream, String> { Err("Mock".to_string()) }
    fn tcp_bind(&self, _addr: &str) -> Result<std::net::TcpListener, String> { Err("Mock".to_string()) }
    fn udp_bind(&self, _addr: &str) -> Result<std::net::UdpSocket, String> { Err("Mock".to_string()) }
    fn resolve_dns(&self, _hostname: &str) -> Result<Vec<std::net::IpAddr>, String> { Err("Mock".to_string()) }

    fn http_get(&self, _url: &str, _h: &HashMap<String, String>) -> Result<vre_core::pal::HttpResponse, String> {
        Ok(vre_core::pal::HttpResponse { status: 200, body: "mock".to_string(), headers: HashMap::new() })
    }
    fn http_post(&self, _url: &str, _h: &HashMap<String, String>, _b: &str) -> Result<vre_core::pal::HttpResponse, String> {
        Ok(vre_core::pal::HttpResponse { status: 200, body: "mock".to_string(), headers: HashMap::new() })
    }
    fn http_put(&self, _url: &str, _h: &HashMap<String, String>, _b: &str) -> Result<vre_core::pal::HttpResponse, String> {
        Ok(vre_core::pal::HttpResponse { status: 200, body: "mock".to_string(), headers: HashMap::new() })
    }
    fn http_delete(&self, _url: &str, _h: &HashMap<String, String>) -> Result<vre_core::pal::HttpResponse, String> {
        Ok(vre_core::pal::HttpResponse { status: 200, body: "mock".to_string(), headers: HashMap::new() })
    }
    fn http_request(&self, _req: vre_core::pal::HttpRequest) -> Result<vre_core::pal::HttpResponse, String> {
        Ok(vre_core::pal::HttpResponse { status: 200, body: "mock".to_string(), headers: HashMap::new() })
    }

    fn ws_connect(&self, _url: &str) -> Result<vre_core::pal::WsHandle, String> { Err("Mock".to_string()) }
    fn ws_send(&self, _h: vre_core::pal::WsHandle, _m: vre_core::pal::WsMessage) -> Result<(), String> { Ok(()) }
    fn ws_recv(&self, _h: vre_core::pal::WsHandle) -> Result<vre_core::pal::WsMessage, String> { Err("Mock".to_string()) }
    fn ws_close(&self, _h: vre_core::pal::WsHandle) -> Result<(), String> { Ok(()) }

    fn load_library(&self, _path: &str) -> Result<usize, String> { Err("Mock".to_string()) }
    fn resolve_symbol(&self, _lib: usize, _sym: &str) -> Result<usize, String> { Err("Mock".to_string()) }
    fn unload_library(&self, _lib: usize) -> Result<(), String> { Ok(()) }
}


#[test]
fn test_mock_pal_cross_platform_paths() {
    let mock = Box::new(MockPal::new());
    
    // Testing normalization of Windows style paths
    let windows_path = Path::new("test\\path.txt");
    mock.write(windows_path, "test content").unwrap();
    
    // Testing unix style path access for the same file
    let unix_path = Path::new("test/path.txt");
    let content = mock.read_to_string(unix_path).unwrap();
    
    assert_eq!(content, "test content");
}
