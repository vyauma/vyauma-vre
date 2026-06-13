//! `vre.toml` — VRE project manifest parser.
//!
//! Canonical project configuration file format for the Vyauma Runtime Engine.
//!
//! ```toml
//! [project]
//! name = "my-app"
//! version = "0.1.0"
//! authors = ["Your Name <you@example.com>"]
//! description = "A Vyauma application"
//!
//! [target]
//! default = "windows-x64"
//!
//! [dependencies]
//! # std = "1.0.0"
//!
//! [capabilities]
//! filesystem = true
//! network = true
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ── Top-level manifest ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VreToml {
    pub project: ProjectSection,

    #[serde(default)]
    pub target: TargetSection,

    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    #[serde(default)]
    pub capabilities: CapabilitiesSection,

    #[serde(default)]
    pub build: BuildSection,
}

// ── [project] ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectSection {
    pub name: String,
    pub version: String,

    #[serde(default)]
    pub authors: Vec<String>,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub license: Option<String>,

    #[serde(default)]
    pub repository: Option<String>,

    /// Entry point source file, relative to project root.
    /// Defaults to `src/main.vya`.
    #[serde(default = "default_entry")]
    pub entry: String,
}

fn default_entry() -> String {
    "src/main.vya".to_string()
}

// ── [target] ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TargetSection {
    /// Default build target (e.g. "windows-x64").
    #[serde(default = "default_target")]
    pub default: String,
}

fn default_target() -> String {
    #[cfg(target_os = "windows")]
    return "windows-x64".to_string();
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64".to_string();
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64".to_string();
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x64".to_string();
    #[allow(unreachable_code)]
    "linux-x64".to_string()
}

// ── [capabilities] ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CapabilitiesSection {
    #[serde(default)]
    pub filesystem: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub environment: bool,
    #[serde(default)]
    pub process: bool,
    #[serde(default)]
    pub database: bool,
}

// ── [build] ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BuildSection {
    /// Output directory for compiled artifacts. Defaults to `dist/`.
    #[serde(default = "default_dist")]
    pub out_dir: String,
}

fn default_dist() -> String {
    "dist".to_string()
}

// ── Implementation ───────────────────────────────────────────────────────────

impl VreToml {
    /// Load `vre.toml` from the given path.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Invalid vre.toml: {}", e))
    }

    /// Search for `vre.toml` starting from `start_dir`, walking up to the
    /// filesystem root. Falls back to `vyauma.toml` for backward compatibility.
    pub fn find_and_load(start_dir: &Path) -> Result<(Self, PathBuf), String> {
        let mut dir = start_dir.to_path_buf();
        loop {
            // Prefer vre.toml
            let candidate = dir.join("vre.toml");
            if candidate.exists() {
                let manifest = Self::load(&candidate)?;
                return Ok((manifest, candidate));
            }
            // Backward-compat fallback: vyauma.toml
            let legacy = dir.join("vyauma.toml");
            if legacy.exists() {
                let manifest = Self::load_legacy(&legacy)?;
                return Ok((manifest, legacy));
            }
            match dir.parent() {
                Some(p) => dir = p.to_path_buf(),
                None => break,
            }
        }
        Err("No vre.toml found. Run `vre init` to create one.".to_string())
    }

    /// Load a legacy `vyauma.toml` (older format) and convert it into a `VreToml`.
    fn load_legacy(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

        // Legacy format uses [package] instead of [project]
        #[derive(Deserialize)]
        struct Legacy {
            package: LegacyPackage,
            #[serde(default)]
            dependencies: HashMap<String, String>,
        }
        #[derive(Deserialize)]
        struct LegacyPackage {
            name: String,
            version: String,
            #[serde(default)]
            authors: Vec<String>,
        }

        let legacy: Legacy = toml::from_str(&content)
            .map_err(|e| format!("Invalid vyauma.toml: {}", e))?;

        Ok(VreToml {
            project: ProjectSection {
                name: legacy.package.name,
                version: legacy.package.version,
                authors: legacy.package.authors,
                description: String::new(),
                license: None,
                repository: None,
                entry: default_entry(),
            },
            target: TargetSection::default(),
            dependencies: legacy.dependencies,
            capabilities: CapabilitiesSection::default(),
            build: BuildSection::default(),
        })
    }

    /// Write a default `vre.toml` into `dir` with the given project name.
    pub fn init(dir: &Path, name: &str) -> Result<(), String> {
        let path = dir.join("vre.toml");
        if path.exists() {
            return Err("vre.toml already exists in this directory.".to_string());
        }
        let content = format!(
            r#"[project]
name = "{}"
version = "0.1.0"
authors = []
description = ""

[target]
default = "{}"

[dependencies]
# std = "1.0.0"

[capabilities]
filesystem = false
network = false
"#,
            name,
            default_target()
        );
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write vre.toml: {}", e))
    }

    /// Serialize this manifest back to TOML and write it to `path`.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
    }
}
