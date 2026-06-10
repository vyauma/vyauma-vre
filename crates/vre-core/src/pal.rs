use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashMap;
use std::process::Command;
use std::net::{TcpStream, TcpListener, UdpSocket, ToSocketAddrs, IpAddr};

pub trait PlatformAbstractionLayer: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String, String>;
    fn write(&self, path: &Path, content: &str) -> Result<(), String>;
    fn append(&self, path: &Path, content: &str) -> Result<(), String>;
    fn remove_file(&self, path: &Path) -> Result<(), String>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), String>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn metadata_len(&self, path: &Path) -> Result<u64, String>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String>;
    fn open_file(&self, path: &Path) -> Result<fs::File, String>;
    
    fn print(&self, msg: &str);
    fn println(&self, msg: &str);
    fn eprintln(&self, msg: &str);
    
    fn current_time_millis(&self) -> u64;
    fn sleep_ms(&self, ms: u64);

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), String>;
    fn copy_file(&self, from: &Path, to: &Path) -> Result<u64, String>;

    fn get_env_var(&self, key: &str) -> Option<String>;
    fn set_env_var(&self, key: &str, value: &str);
    fn get_all_env_vars(&self) -> HashMap<String, String>;
    fn get_system_info(&self) -> HashMap<String, String>;

    fn spawn_process(&self, command: &str, args: &[&str]) -> Result<u32, String>;
    fn kill_process(&self, pid: u32) -> Result<(), String>;

    fn tcp_connect(&self, addr: &str) -> Result<TcpStream, String>;
    fn tcp_bind(&self, addr: &str) -> Result<TcpListener, String>;
    fn udp_bind(&self, addr: &str) -> Result<UdpSocket, String>;
    fn resolve_dns(&self, hostname: &str) -> Result<Vec<IpAddr>, String>;

    fn load_library(&self, path: &str) -> Result<usize, String>;
    fn resolve_symbol(&self, lib: usize, sym: &str) -> Result<usize, String>;
    fn unload_library(&self, lib: usize) -> Result<(), String>;

    fn watch_file(&self, path: &Path) -> Result<usize, String>;
    fn handle_interrupt(&self) -> Result<(), String>;
}

pub struct OsPal {
    libraries: std::sync::Mutex<HashMap<usize, libloading::Library>>,
    next_lib_id: std::sync::Mutex<usize>,
    watchers: std::sync::Mutex<HashMap<usize, notify::RecommendedWatcher>>,
    next_watcher_id: std::sync::Mutex<usize>,
}

impl Default for OsPal {
    fn default() -> Self {
        Self {
            libraries: std::sync::Mutex::new(HashMap::new()),
            next_lib_id: std::sync::Mutex::new(1),
            watchers: std::sync::Mutex::new(HashMap::new()),
            next_watcher_id: std::sync::Mutex::new(1),
        }
    }
}

impl PlatformAbstractionLayer for OsPal {
    fn read_to_string(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| e.to_string())
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), String> {
        fs::write(path, content).map_err(|e| e.to_string())
    }

    fn append(&self, path: &Path, content: &str) -> Result<(), String> {
        use std::io::Write;
        let mut file = fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
        write!(file, "{}", content).map_err(|e| e.to_string())
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        fs::remove_file(path).map_err(|e| e.to_string())
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), String> {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn metadata_len(&self, path: &Path) -> Result<u64, String> {
        fs::metadata(path).map(|m| m.len()).map_err(|e| e.to_string())
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String> {
        fs::canonicalize(path).map_err(|e| e.to_string())
    }

    fn open_file(&self, path: &Path) -> Result<fs::File, String> {
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(|e| e.to_string())
    }

    fn print(&self, msg: &str) {
        print!("{}", msg);
    }

    fn println(&self, msg: &str) {
        println!("{}", msg);
    }

    fn eprintln(&self, msg: &str) {
        eprintln!("{}", msg);
    }

    fn current_time_millis(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
    }

    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), String> {
        fs::rename(from, to).map_err(|e| e.to_string())
    }

    fn copy_file(&self, from: &Path, to: &Path) -> Result<u64, String> {
        fs::copy(from, to).map_err(|e| e.to_string())
    }

    fn get_env_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn set_env_var(&self, key: &str, value: &str) {
        std::env::set_var(key, value);
    }

    fn get_all_env_vars(&self) -> HashMap<String, String> {
        std::env::vars().collect()
    }

    fn get_system_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("os".to_string(), std::env::consts::OS.to_string());
        info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
        info.insert("family".to_string(), std::env::consts::FAMILY.to_string());
        info
    }

    fn spawn_process(&self, command: &str, args: &[&str]) -> Result<u32, String> {
        let child = Command::new(command)
            .args(args)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(child.id())
    }

    fn kill_process(&self, pid: u32) -> Result<(), String> {
        // Simplified kill: we can just use std::process::Command to invoke kill on unix or taskkill on windows
        if cfg!(windows) {
            Command::new("taskkill")
                .args(&["/F", "/PID", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("kill")
                .args(&["-9", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn tcp_connect(&self, addr: &str) -> Result<TcpStream, String> {
        TcpStream::connect(addr).map_err(|e| e.to_string())
    }

    fn tcp_bind(&self, addr: &str) -> Result<TcpListener, String> {
        TcpListener::bind(addr).map_err(|e| e.to_string())
    }

    fn udp_bind(&self, addr: &str) -> Result<UdpSocket, String> {
        UdpSocket::bind(addr).map_err(|e| e.to_string())
    }

    fn resolve_dns(&self, hostname: &str) -> Result<Vec<IpAddr>, String> {
        let addrs = (hostname, 0).to_socket_addrs().map_err(|e| e.to_string())?;
        Ok(addrs.map(|a| a.ip()).collect())
    }

    fn load_library(&self, path: &str) -> Result<usize, String> {
        unsafe {
            let lib = libloading::Library::new(path).map_err(|e| e.to_string())?;
            let mut id_lock = self.next_lib_id.lock().unwrap();
            let id = *id_lock;
            *id_lock += 1;
            
            self.libraries.lock().unwrap().insert(id, lib);
            Ok(id)
        }
    }

    fn resolve_symbol(&self, lib_id: usize, sym: &str) -> Result<usize, String> {
        let libs = self.libraries.lock().unwrap();
        if let Some(lib) = libs.get(&lib_id) {
            unsafe {
                let symbol: libloading::Symbol<*mut std::ffi::c_void> = lib.get(sym.as_bytes()).map_err(|e| e.to_string())?;
                // We return the raw pointer cast to usize
                Ok(*symbol as usize)
            }
        } else {
            Err("Library not loaded".to_string())
        }
    }

    fn unload_library(&self, lib_id: usize) -> Result<(), String> {
        let mut libs = self.libraries.lock().unwrap();
        if libs.remove(&lib_id).is_some() {
            Ok(())
        } else {
            Err("Library not found".to_string())
        }
    }

    fn watch_file(&self, path: &Path) -> Result<usize, String> {
        use notify::{Watcher, RecursiveMode};
        let mut watcher = notify::recommended_watcher(|res| {
            // Note: For a fully integrated VM, this would push an event to the VM event loop
            match res {
                Ok(event) => println!("FS Watch Event: {:?}", event),
                Err(e) => println!("FS Watch Error: {:?}", e),
            }
        }).map_err(|e| e.to_string())?;
        
        watcher.watch(path, RecursiveMode::NonRecursive).map_err(|e| e.to_string())?;

        let mut id_lock = self.next_watcher_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;

        self.watchers.lock().unwrap().insert(id, watcher);
        Ok(id)
    }

    fn handle_interrupt(&self) -> Result<(), String> {
        ctrlc::set_handler(move || {
            println!("Received Interrupt Signal! Halting gracefully...");
            std::process::exit(0);
        }).map_err(|e| e.to_string())
    }
}

use std::sync::OnceLock;

static PAL_INSTANCE: OnceLock<Box<dyn PlatformAbstractionLayer>> = OnceLock::new();

pub fn get_pal() -> &'static dyn PlatformAbstractionLayer {
    PAL_INSTANCE.get_or_init(|| Box::new(OsPal::default())).as_ref()
}

pub fn set_pal(pal: Box<dyn PlatformAbstractionLayer>) {
    let _ = PAL_INSTANCE.set(pal);
}
