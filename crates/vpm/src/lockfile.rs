//! vyauma.lock — Lockfile Model
//!
//! Records the exact resolved versions and checksums of all installed packages.
//! This file is committed to version control to ensure reproducible builds.

use serde::{Deserialize, Serialize};

/// Full lockfile (vyauma.lock)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LockFile {
    pub packages: Vec<LockedPackage>,
}

/// A single pinned package entry in the lockfile.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockedPackage {
    /// Package name
    pub name: String,
    /// Exact resolved version (SemVer)
    pub version: String,
    /// SHA-256 hex digest of the downloaded tarball
    pub checksum: String,
    /// Registry URL the package was fetched from
    pub registry: String,
    /// Names of direct dependencies this package requires
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl LockFile {
    pub const PATH: &'static str = "vyauma.lock";

    /// Load an existing lockfile, returning an empty one if not found.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::PATH) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Write the lockfile to disk.
    pub fn save(&self) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize lockfile: {}", e))?;
        std::fs::write(Self::PATH, content)
            .map_err(|e| format!("Failed to write lockfile: {}", e))
    }

    /// Find a package by name.
    pub fn find(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Upsert a locked package (replace if name already present).
    pub fn upsert(&mut self, pkg: LockedPackage) {
        if let Some(existing) = self.packages.iter_mut().find(|p| p.name == pkg.name) {
            *existing = pkg;
        } else {
            self.packages.push(pkg);
        }
    }
}
