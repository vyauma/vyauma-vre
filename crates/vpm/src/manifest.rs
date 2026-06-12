//! vyauma.toml — Package Manifest Model
//!
//! Represents the parsed contents of a Vyauma project manifest file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level manifest structure (`vyauma.toml`)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Manifest {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
}

/// `[package]` table
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    /// Entry point, defaults to `src/main.vya`
    #[serde(default)]
    pub entry: Option<String>,
}

impl Manifest {
    /// Load manifest from the current directory or a given path.
    pub fn load(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Invalid {}: {}", path, e))
    }

    /// Write manifest to disk.
    pub fn save(&self, path: &str) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write {}: {}", path, e))
    }

    /// Return entry point path (defaults to `src/main.vya`).
    pub fn entry_point(&self) -> &str {
        self.package.entry.as_deref().unwrap_or("src/main.vya")
    }
}
