use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

pub struct DocumentDatabase {
    base_dir: PathBuf,
}

impl DocumentDatabase {
    pub fn new(base_dir: &str) -> Self {
        let path = PathBuf::from(base_dir);
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        Self { base_dir: path }
    }

    fn collection_path(&self, name: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", name))
    }

    fn read_collection(&self, name: &str) -> Vec<Value> {
        let path = self.collection_path(name);
        if !path.exists() {
            return Vec::new();
        }
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Vec::new()),
            Err(_) => Vec::new(),
        }
    }

    fn write_collection(&self, name: &str, data: &Vec<Value>) -> Result<(), String> {
        let path = self.collection_path(name);
        let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    pub fn insert(&self, collection: &str, document: Value) -> Result<String, String> {
        let mut data = self.read_collection(collection);
        
        let mut doc = document.clone();
        let id = uuid::Uuid::new_v4().to_string();
        
        if let Some(obj) = doc.as_object_mut() {
            obj.insert("_id".to_string(), Value::String(id.clone()));
        }

        data.push(doc);
        self.write_collection(collection, &data)?;
        Ok(id)
    }

    pub fn find(&self, collection: &str, filter_key: &str, filter_value: &str) -> Vec<Value> {
        let data = self.read_collection(collection);
        if filter_key.is_empty() {
            return data;
        }

        data.into_iter().filter(|doc| {
            if let Some(obj) = doc.as_object() {
                if let Some(val) = obj.get(filter_key) {
                    return match val {
                        Value::String(s) => s == filter_value,
                        Value::Number(n) => n.to_string() == filter_value,
                        Value::Bool(b) => b.to_string() == filter_value,
                        _ => false,
                    };
                }
            }
            false
        }).collect()
    }

    pub fn delete(&self, collection: &str, filter_key: &str, filter_value: &str) -> Result<bool, String> {
        let mut data = self.read_collection(collection);
        let initial_len = data.len();

        data.retain(|doc| {
            if let Some(obj) = doc.as_object() {
                if let Some(val) = obj.get(filter_key) {
                    let matches = match val {
                        Value::String(s) => s == filter_value,
                        Value::Number(n) => n.to_string() == filter_value,
                        Value::Bool(b) => b.to_string() == filter_value,
                        _ => false,
                    };
                    return !matches; // retain if NOT matches
                }
            }
            true // retain if filter_key not present
        });

        if data.len() != initial_len {
            self.write_collection(collection, &data)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
