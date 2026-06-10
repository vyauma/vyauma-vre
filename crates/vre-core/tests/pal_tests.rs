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
        if fs.remove(&path_str).is_some() {
            Ok(())
        } else {
            Err("File not found".to_string())
        }
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), String> {
        self.remove_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let fs = self.fs.lock().unwrap();
        fs.contains_key(&path_str)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }

    fn metadata_len(&self, path: &Path) -> Result<u64, String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        let fs = self.fs.lock().unwrap();
        if let Some(content) = fs.get(&path_str) {
            Ok(content.len() as u64)
        } else {
            Err("File not found".to_string())
        }
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String> {
        let path_str = path.to_string_lossy().replace("\\", "/");
        Ok(PathBuf::from(path_str))
    }

    fn print(&self, _msg: &str) {}
    fn println(&self, _msg: &str) {}
    fn eprintln(&self, _msg: &str) {}

    fn current_time_millis(&self) -> u64 {
        1000
    }
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
