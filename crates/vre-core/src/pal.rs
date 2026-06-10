use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

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
    
    fn print(&self, msg: &str);
    fn println(&self, msg: &str);
    fn eprintln(&self, msg: &str);
    
    fn current_time_millis(&self) -> u64;
}

pub struct OsPal;

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
}

use std::sync::OnceLock;

static PAL_INSTANCE: OnceLock<Box<dyn PlatformAbstractionLayer>> = OnceLock::new();

pub fn get_pal() -> &'static dyn PlatformAbstractionLayer {
    PAL_INSTANCE.get_or_init(|| Box::new(OsPal)).as_ref()
}

pub fn set_pal(pal: Box<dyn PlatformAbstractionLayer>) {
    let _ = PAL_INSTANCE.set(pal);
}
