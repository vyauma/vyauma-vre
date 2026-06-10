use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashMap;
use std::process::Command;
use std::net::{TcpStream, TcpListener, UdpSocket, ToSocketAddrs, IpAddr};
use std::sync::{Arc, Mutex};

// ─── HTTP Types ──────────────────────────────────────────────────────────────

/// A simplified HTTP response returned by the PAL HTTP methods
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// A simplified HTTP request builder for PAL
#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

// ─── WebSocket Types ─────────────────────────────────────────────────────────

/// Opaque handle for an open WebSocket connection
pub type WsHandle = usize;

/// A WebSocket message (text or binary)
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

// ─── Timer Types ─────────────────────────────────────────────────────────────

/// Opaque timer ID returned by set_timer
pub type TimerId = usize;

// ─── Signal Types ────────────────────────────────────────────────────────────

/// Well-known OS signals supported by the PAL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Interrupt, // SIGINT / Ctrl+C
    Terminate, // SIGTERM
    Hangup,    // SIGHUP (Unix only)
    UserDef1,  // SIGUSR1 (Unix only)
    UserDef2,  // SIGUSR2 (Unix only)
}

// ─── PAL Trait ───────────────────────────────────────────────────────────────

pub trait PlatformAbstractionLayer: Send + Sync {
    // ── Filesystem ───────────────────────────────────────────────────────────
    fn read_to_string(&self, path: &Path) -> Result<String, String>;
    fn write(&self, path: &Path, content: &str) -> Result<(), String>;
    fn append(&self, path: &Path, content: &str) -> Result<(), String>;
    fn remove_file(&self, path: &Path) -> Result<(), String>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), String>;
    fn create_dir_all(&self, path: &Path) -> Result<(), String>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn metadata_len(&self, path: &Path) -> Result<u64, String>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String>;
    fn open_file(&self, path: &Path) -> Result<fs::File, String>;
    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), String>;
    fn copy_file(&self, from: &Path, to: &Path) -> Result<u64, String>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, String>;
    fn watch_file(&self, path: &Path) -> Result<usize, String>;

    // ── I/O ──────────────────────────────────────────────────────────────────
    fn print(&self, msg: &str);
    fn println(&self, msg: &str);
    fn eprintln(&self, msg: &str);

    // ── Timing ───────────────────────────────────────────────────────────────
    fn current_time_millis(&self) -> u64;
    fn sleep_ms(&self, ms: u64);
    /// Schedule a one-shot timer that fires after `ms` milliseconds.
    /// Calls `callback` on a background thread. Returns a TimerId.
    fn set_timer(&self, ms: u64, callback: Box<dyn Fn() + Send + 'static>) -> TimerId;
    /// Cancel a previously set timer (best-effort; may already have fired).
    fn cancel_timer(&self, id: TimerId);

    // ── Environment ──────────────────────────────────────────────────────────
    fn get_env_var(&self, key: &str) -> Option<String>;
    fn set_env_var(&self, key: &str, value: &str);
    fn get_all_env_vars(&self) -> HashMap<String, String>;
    fn get_system_info(&self) -> HashMap<String, String>;

    // ── Process ──────────────────────────────────────────────────────────────
    fn spawn_process(&self, command: &str, args: &[&str]) -> Result<u32, String>;
    fn kill_process(&self, pid: u32) -> Result<(), String>;
    /// Send a signal to a process (Unix: kill(pid, sig); Windows: TerminateProcess for Terminate)
    fn send_signal(&self, pid: u32, signal: Signal) -> Result<(), String>;
    /// Register a handler for a signal; returns Err if unsupported on this platform.
    fn handle_signal(&self, signal: Signal, callback: Box<dyn Fn() + Send + 'static>) -> Result<(), String>;
    /// Register Ctrl+C (SIGINT) handler (convenience wrapper).
    fn handle_interrupt(&self) -> Result<(), String>;

    // ── TCP / UDP / DNS ──────────────────────────────────────────────────────
    fn tcp_connect(&self, addr: &str) -> Result<TcpStream, String>;
    fn tcp_bind(&self, addr: &str) -> Result<TcpListener, String>;
    fn udp_bind(&self, addr: &str) -> Result<UdpSocket, String>;
    fn resolve_dns(&self, hostname: &str) -> Result<Vec<IpAddr>, String>;

    // ── HTTP / HTTPS ─────────────────────────────────────────────────────────
    fn http_get(&self, url: &str, headers: &HashMap<String, String>) -> Result<HttpResponse, String>;
    fn http_post(&self, url: &str, headers: &HashMap<String, String>, body: &str) -> Result<HttpResponse, String>;
    fn http_put(&self, url: &str, headers: &HashMap<String, String>, body: &str) -> Result<HttpResponse, String>;
    fn http_delete(&self, url: &str, headers: &HashMap<String, String>) -> Result<HttpResponse, String>;
    fn http_request(&self, req: HttpRequest) -> Result<HttpResponse, String>;

    // ── WebSocket ────────────────────────────────────────────────────────────
    fn ws_connect(&self, url: &str) -> Result<WsHandle, String>;
    fn ws_send(&self, handle: WsHandle, msg: WsMessage) -> Result<(), String>;
    fn ws_recv(&self, handle: WsHandle) -> Result<WsMessage, String>;
    fn ws_close(&self, handle: WsHandle) -> Result<(), String>;

    // ── Dynamic Libraries ────────────────────────────────────────────────────
    fn load_library(&self, path: &str) -> Result<usize, String>;
    fn resolve_symbol(&self, lib: usize, sym: &str) -> Result<usize, String>;
    fn unload_library(&self, lib: usize) -> Result<(), String>;
}

// ─── OsPal Implementation ────────────────────────────────────────────────────

use tungstenite::{connect, WebSocket, stream::MaybeTlsStream};

pub struct OsPal {
    libraries:       std::sync::Mutex<HashMap<usize, libloading::Library>>,
    next_lib_id:     std::sync::Mutex<usize>,
    watchers:        std::sync::Mutex<HashMap<usize, notify::RecommendedWatcher>>,
    next_watcher_id: std::sync::Mutex<usize>,
    /// Active timers: id → cancel flag
    timers:          Arc<Mutex<HashMap<usize, Arc<std::sync::atomic::AtomicBool>>>>,
    next_timer_id:   std::sync::Mutex<usize>,
    /// Active WebSocket connections
    websockets:      std::sync::Mutex<HashMap<usize, WebSocket<MaybeTlsStream<TcpStream>>>>,
    next_ws_id:      std::sync::Mutex<usize>,
}

impl Default for OsPal {
    fn default() -> Self {
        Self {
            libraries:       std::sync::Mutex::new(HashMap::new()),
            next_lib_id:     std::sync::Mutex::new(1),
            watchers:        std::sync::Mutex::new(HashMap::new()),
            next_watcher_id: std::sync::Mutex::new(1),
            timers:          Arc::new(Mutex::new(HashMap::new())),
            next_timer_id:   std::sync::Mutex::new(1),
            websockets:      std::sync::Mutex::new(HashMap::new()),
            next_ws_id:      std::sync::Mutex::new(1),
        }
    }
}

impl PlatformAbstractionLayer for OsPal {
    // ── Filesystem ───────────────────────────────────────────────────────────

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

    fn create_dir_all(&self, path: &Path) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    fn exists(&self, path: &Path) -> bool { path.exists() }
    fn is_file(&self, path: &Path) -> bool { path.is_file() }
    fn is_dir(&self, path: &Path) -> bool { path.is_dir() }

    fn metadata_len(&self, path: &Path) -> Result<u64, String> {
        fs::metadata(path).map(|m| m.len()).map_err(|e| e.to_string())
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String> {
        fs::canonicalize(path).map_err(|e| e.to_string())
    }

    fn open_file(&self, path: &Path) -> Result<fs::File, String> {
        fs::OpenOptions::new().read(true).write(true).create(true)
            .open(path).map_err(|e| e.to_string())
    }

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), String> {
        fs::rename(from, to).map_err(|e| e.to_string())
    }

    fn copy_file(&self, from: &Path, to: &Path) -> Result<u64, String> {
        fs::copy(from, to).map_err(|e| e.to_string())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string()))
            .collect()
    }

    fn watch_file(&self, path: &Path) -> Result<usize, String> {
        use notify::{Watcher, RecursiveMode};
        let mut watcher = notify::recommended_watcher(|res| {
            match res {
                Ok(event) => println!("FS Watch Event: {:?}", event),
                Err(e)    => println!("FS Watch Error: {:?}", e),
            }
        }).map_err(|e| e.to_string())?;
        watcher.watch(path, RecursiveMode::NonRecursive).map_err(|e| e.to_string())?;
        let mut id_lock = self.next_watcher_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        self.watchers.lock().unwrap().insert(id, watcher);
        Ok(id)
    }

    // ── I/O ──────────────────────────────────────────────────────────────────

    fn print(&self, msg: &str)   { print!("{}", msg); }
    fn println(&self, msg: &str) { println!("{}", msg); }
    fn eprintln(&self, msg: &str) { eprintln!("{}", msg); }

    // ── Timing ───────────────────────────────────────────────────────────────

    fn current_time_millis(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
    }

    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    fn set_timer(&self, ms: u64, callback: Box<dyn Fn() + Send + 'static>) -> TimerId {
        let mut id_lock = self.next_timer_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        drop(id_lock);

        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = Arc::clone(&cancelled);

        self.timers.lock().unwrap().insert(id, Arc::clone(&cancelled));

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(ms));
            if !cancelled_clone.load(std::sync::atomic::Ordering::SeqCst) {
                callback();
            }
        });

        id
    }

    fn cancel_timer(&self, id: TimerId) {
        if let Some(flag) = self.timers.lock().unwrap().remove(&id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    // ── Environment ──────────────────────────────────────────────────────────

    fn get_env_var(&self, key: &str) -> Option<String> { std::env::var(key).ok() }
    fn set_env_var(&self, key: &str, value: &str) { std::env::set_var(key, value); }
    fn get_all_env_vars(&self) -> HashMap<String, String> { std::env::vars().collect() }

    fn get_system_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("os".to_string(),     std::env::consts::OS.to_string());
        info.insert("arch".to_string(),   std::env::consts::ARCH.to_string());
        info.insert("family".to_string(), std::env::consts::FAMILY.to_string());
        info
    }

    // ── Process ──────────────────────────────────────────────────────────────

    fn spawn_process(&self, command: &str, args: &[&str]) -> Result<u32, String> {
        let child = Command::new(command).args(args).spawn().map_err(|e| e.to_string())?;
        Ok(child.id())
    }

    fn kill_process(&self, pid: u32) -> Result<(), String> {
        if cfg!(windows) {
            Command::new("taskkill").args(&["/F", "/PID", &pid.to_string()])
                .output().map_err(|e| e.to_string())?;
        } else {
            Command::new("kill").args(&["-9", &pid.to_string()])
                .output().map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn send_signal(&self, pid: u32, signal: Signal) -> Result<(), String> {
        #[cfg(unix)]
        {
            let signum = match signal {
                Signal::Interrupt  => libc::SIGINT,
                Signal::Terminate  => libc::SIGTERM,
                Signal::Hangup     => libc::SIGHUP,
                Signal::UserDef1   => libc::SIGUSR1,
                Signal::UserDef2   => libc::SIGUSR2,
            };
            unsafe {
                if libc::kill(pid as i32, signum) != 0 {
                    return Err(format!("kill({}, {}) failed: {}", pid, signum, std::io::Error::last_os_error()));
                }
            }
            Ok(())
        }
        #[cfg(windows)]
        {
            match signal {
                Signal::Terminate | Signal::Interrupt => self.kill_process(pid),
                _ => Err("Signal not supported on Windows".to_string()),
            }
        }
    }

    fn handle_signal(&self, signal: Signal, callback: Box<dyn Fn() + Send + 'static>) -> Result<(), String> {
        #[cfg(unix)]
        {
            use signal_hook::consts::signal as sig;
            use signal_hook::iterator::Signals;

            let signum = match signal {
                Signal::Interrupt  => sig::SIGINT,
                Signal::Terminate  => sig::SIGTERM,
                Signal::Hangup     => sig::SIGHUP,
                Signal::UserDef1   => sig::SIGUSR1,
                Signal::UserDef2   => sig::SIGUSR2,
            };
            let mut signals = Signals::new(&[signum]).map_err(|e| e.to_string())?;
            std::thread::spawn(move || {
                for _ in signals.forever() {
                    callback();
                }
            });
            Ok(())
        }
        #[cfg(windows)]
        {
            if signal == Signal::Interrupt {
                ctrlc::set_handler(move || callback()).map_err(|e| e.to_string())
            } else {
                Err("Only SIGINT (Ctrl+C) supported on Windows".to_string())
            }
        }
    }

    fn handle_interrupt(&self) -> Result<(), String> {
        ctrlc::set_handler(move || {
            println!("Received Interrupt Signal! Halting gracefully...");
            std::process::exit(0);
        }).map_err(|e| e.to_string())
    }

    // ── TCP / UDP / DNS ──────────────────────────────────────────────────────

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

    // ── HTTP / HTTPS ─────────────────────────────────────────────────────────

    fn http_get(&self, url: &str, headers: &HashMap<String, String>) -> Result<HttpResponse, String> {
        self.http_request(HttpRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: headers.clone(),
            body: None,
        })
    }

    fn http_post(&self, url: &str, headers: &HashMap<String, String>, body: &str) -> Result<HttpResponse, String> {
        self.http_request(HttpRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: headers.clone(),
            body: Some(body.to_string()),
        })
    }

    fn http_put(&self, url: &str, headers: &HashMap<String, String>, body: &str) -> Result<HttpResponse, String> {
        self.http_request(HttpRequest {
            method: "PUT".to_string(),
            url: url.to_string(),
            headers: headers.clone(),
            body: Some(body.to_string()),
        })
    }

    fn http_delete(&self, url: &str, headers: &HashMap<String, String>) -> Result<HttpResponse, String> {
        self.http_request(HttpRequest {
            method: "DELETE".to_string(),
            url: url.to_string(),
            headers: headers.clone(),
            body: None,
        })
    }

    fn http_request(&self, req: HttpRequest) -> Result<HttpResponse, String> {
        let mut builder = ureq::request(&req.method, &req.url);
        for (key, val) in &req.headers {
            builder = builder.set(key, val);
        }

        let response = if let Some(body) = &req.body {
            builder.send_string(body).map_err(|e| e.to_string())?
        } else {
            builder.call().map_err(|e| e.to_string())?
        };

        let status = response.status();
        let mut resp_headers = HashMap::new();
        for name in response.headers_names() {
            if let Some(val) = response.header(&name) {
                resp_headers.insert(name, val.to_string());
            }
        }
        let body = response.into_string().map_err(|e| e.to_string())?;

        Ok(HttpResponse { status, body, headers: resp_headers })
    }

    // ── WebSocket ────────────────────────────────────────────────────────────

    fn ws_connect(&self, url: &str) -> Result<WsHandle, String> {
        let (socket, _response) = connect(url).map_err(|e| e.to_string())?;
        let mut id_lock = self.next_ws_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        self.websockets.lock().unwrap().insert(id, socket);
        Ok(id)
    }

    fn ws_send(&self, handle: WsHandle, msg: WsMessage) -> Result<(), String> {
        let mut sockets = self.websockets.lock().unwrap();
        let socket = sockets.get_mut(&handle).ok_or_else(|| "WebSocket handle not found".to_string())?;
        let ws_msg = match msg {
            WsMessage::Text(t)   => tungstenite::Message::Text(t),
            WsMessage::Binary(b) => tungstenite::Message::Binary(b),
            WsMessage::Close     => tungstenite::Message::Close(None),
        };
        socket.send(ws_msg).map_err(|e| e.to_string())
    }

    fn ws_recv(&self, handle: WsHandle) -> Result<WsMessage, String> {
        let mut sockets = self.websockets.lock().unwrap();
        let socket = sockets.get_mut(&handle).ok_or_else(|| "WebSocket handle not found".to_string())?;
        let msg = socket.read().map_err(|e| e.to_string())?;
        Ok(match msg {
            tungstenite::Message::Text(t)   => WsMessage::Text(t),
            tungstenite::Message::Binary(b) => WsMessage::Binary(b),
            tungstenite::Message::Close(_)  => WsMessage::Close,
            tungstenite::Message::Ping(_)   => WsMessage::Text("[ping]".to_string()),
            tungstenite::Message::Pong(_)   => WsMessage::Text("[pong]".to_string()),
            tungstenite::Message::Frame(_)  => WsMessage::Text("[frame]".to_string()),
        })
    }

    fn ws_close(&self, handle: WsHandle) -> Result<(), String> {
        let mut sockets = self.websockets.lock().unwrap();
        if let Some(mut socket) = sockets.remove(&handle) {
            socket.close(None).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── Dynamic Libraries ────────────────────────────────────────────────────

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
                Ok(*symbol as usize)
            }
        } else {
            Err("Library not loaded".to_string())
        }
    }

    fn unload_library(&self, lib_id: usize) -> Result<(), String> {
        let mut libs = self.libraries.lock().unwrap();
        if libs.remove(&lib_id).is_some() { Ok(()) } else { Err("Library not found".to_string()) }
    }
}

// ─── Global PAL Instance ─────────────────────────────────────────────────────

use std::sync::OnceLock;

static PAL_INSTANCE: OnceLock<Box<dyn PlatformAbstractionLayer>> = OnceLock::new();

pub fn get_pal() -> &'static dyn PlatformAbstractionLayer {
    PAL_INSTANCE.get_or_init(|| Box::new(OsPal::default())).as_ref()
}

pub fn set_pal(pal: Box<dyn PlatformAbstractionLayer>) {
    let _ = PAL_INSTANCE.set(pal);
}
