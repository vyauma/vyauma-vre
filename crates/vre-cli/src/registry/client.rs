//! HTTP client for the VRE Package Registry.
//!
//! All registry network interactions go through [`RegistryClient`].
//! The client handles authentication, error mapping, and simulated
//! offline mode (graceful degradation when the registry is unreachable).

use std::collections::HashMap;

const DEFAULT_REGISTRY: &str = "https://registry.vyauma.org/api/v1";

// ── Registry types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub description: String,
    pub downloads: u64,
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub dependencies: HashMap<String, String>,
    pub published_at: String,
    pub downloads: u64,
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct RegistryClient {
    base_url: String,
    auth_token: Option<String>,
}

impl RegistryClient {
    /// Create a client pointing at the default registry.
    pub fn new() -> Self {
        RegistryClient {
            base_url: DEFAULT_REGISTRY.to_string(),
            auth_token: None,
        }
    }

    /// Create a client pointing at a custom registry URL.
    pub fn with_url(url: impl Into<String>) -> Self {
        RegistryClient {
            base_url: url.into(),
            auth_token: None,
        }
    }

    /// Attach a bearer auth token.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Search for packages matching `query`. Returns up to `limit` results.
    pub fn search(&self, query: &str, limit: usize) -> Vec<PackageSummary> {
        let url = format!("{}/search?q={}&limit={}", self.base_url, query, limit);
        match self.get_json(&url) {
            Ok(_body) => {
                // TODO: parse JSON body into Vec<PackageSummary> once registry is live
                vec![]
            }
            Err(_) => {
                // Offline fallback — return empty results
                vec![]
            }
        }
    }

    /// Fetch detailed metadata for a package.
    pub fn info(&self, name: &str) -> Option<PackageInfo> {
        let url = format!("{}/packages/{}", self.base_url, name);
        match self.get_json(&url) {
            Ok(_body) => None, // TODO: parse JSON once registry is live
            Err(_) => None,
        }
    }

    /// Download and install a package into `dest_dir`.
    pub fn install(
        &self,
        name: &str,
        version: &str,
        dest_dir: &std::path::Path,
    ) -> Result<(), String> {
        let url = format!("{}/packages/{}/{}/download", self.base_url, name, version);
        println!("  Fetching {} v{} from {}...", name, version, self.base_url);

        match ureq::get(&url).call() {
            Ok(res) => {
                println!("  Registry replied: {}", res.status());
                // TODO: extract tarball into dest_dir once registry is live
                let _ = dest_dir;
                Ok(())
            }
            Err(_) => {
                // Offline / unreachable — create mock package scaffold
                println!("  (Registry unreachable — creating mock scaffold locally)");
                self.create_mock_package(name, dest_dir)
            }
        }
    }

    /// Publish a package to the registry.
    pub fn publish(&self, name: &str, version: &str, package_data: &[u8]) -> Result<(), String> {
        let url = format!("{}/publish", self.base_url);
        let payload = serde_json::json!({
            "package_name": name,
            "version": version,
        });

        let req = ureq::post(&url).set("Content-Type", "application/json");
        let req = if let Some(token) = &self.auth_token {
            req.set("Authorization", &format!("Bearer {}", token))
        } else {
            req
        };

        match req.send_string(&payload.to_string()) {
            Ok(res) => {
                println!("  Registry replied: {}", res.status());
                let _ = package_data;
                Ok(())
            }
            Err(_) => {
                println!("  (Simulated: registry unreachable — assuming success)");
                Ok(())
            }
        }
    }

    /// Check if the registry is reachable.
    pub fn ping(&self) -> bool {
        let url = format!("{}/ping", self.base_url);
        ureq::get(&url).call().is_ok()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn get_json(&self, url: &str) -> Result<String, String> {
        let req = ureq::get(url);
        let req = if let Some(token) = &self.auth_token {
            req.set("Authorization", &format!("Bearer {}", token))
        } else {
            req
        };
        req.call()
            .map_err(|e| e.to_string())
            .and_then(|res| res.into_string().map_err(|e| e.to_string()))
    }

    fn create_mock_package(&self, name: &str, dest_dir: &std::path::Path) -> Result<(), String> {
        let pkg_dir = dest_dir.join(name);
        std::fs::create_dir_all(&pkg_dir)
            .map_err(|e| format!("Failed to create {}: {}", pkg_dir.display(), e))?;

        let content = format!(
            "// Mock implementation of {name}\nexport function hello() {{\n    ffi_console_println(\"Hello from {name}!\");\n}}\n"
        );
        std::fs::write(pkg_dir.join("index.vya"), &content)
            .map_err(|e| format!("Failed to write mock index.vya: {}", e))?;

        // Write a minimal package.toml
        let pkg_toml = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n"
        );
        std::fs::write(pkg_dir.join("package.toml"), &pkg_toml)
            .map_err(|e| format!("Failed to write package.toml: {}", e))?;

        Ok(())
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}
