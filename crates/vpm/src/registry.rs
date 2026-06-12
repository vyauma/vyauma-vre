//! Vyauma Registry Client
//!
//! HTTP client for the Vyauma Package Registry (`https://registry.vyauma.org`).
//! All operations gracefully degrade to an error message when the registry is offline.

use std::io::Read;
use serde::{Deserialize, Serialize};

pub const REGISTRY_URL: &str = "https://registry.vyauma.org";

/// Package metadata returned by the registry API.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    /// Direct download URL for the tarball (`.tar.gz`)
    pub download_url: String,
    /// SHA-256 hex digest of the tarball
    pub checksum: String,
    /// Names of direct dependencies
    #[serde(default)]
    pub dependencies: Vec<RegistryDependency>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryDependency {
    pub name: String,
    pub version_req: String,
}

/// Fetch all versions of a package from the registry.
pub fn fetch_versions(name: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/api/v1/packages/{}", REGISTRY_URL, name);
    match ureq::get(&url).call() {
        Ok(resp) => {
            let body = resp.into_string()
                .map_err(|e| format!("Failed to read registry response: {}", e))?;
            let versions: Vec<String> = serde_json::from_str(&body)
                .map_err(|e| format!("Failed to parse version list: {}", e))?;
            Ok(versions)
        }
        Err(e) => Err(format!("Registry unreachable: {}. Is https://registry.vyauma.org online?", e)),
    }
}

/// Fetch metadata for a specific version of a package.
pub fn fetch_package_metadata(name: &str, version: &str) -> Result<RegistryPackage, String> {
    let url = format!("{}/api/v1/packages/{}/{}", REGISTRY_URL, name, version);
    match ureq::get(&url).call() {
        Ok(resp) => {
            let body = resp.into_string()
                .map_err(|e| format!("Failed to read registry response: {}", e))?;
            serde_json::from_str(&body)
                .map_err(|e| format!("Failed to parse package metadata: {}", e))
        }
        Err(e) => Err(format!("Failed to fetch {}@{}: {}", name, version, e)),
    }
}

/// Download a package tarball and extract it to `dest_dir/<package_name>/`.
/// Returns the SHA-256 hex digest of the downloaded bytes.
pub fn download_package(pkg: &RegistryPackage, dest_dir: &std::path::Path) -> Result<String, String> {
    use sha2::{Sha256, Digest};

    let resp = ureq::get(&pkg.download_url)
        .call()
        .map_err(|e| format!("Failed to download {}: {}", pkg.name, e))?;

    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read download stream: {}", e))?;

    // Verify checksum
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = format!("{:x}", hasher.finalize());

    if !pkg.checksum.is_empty() && digest != pkg.checksum {
        return Err(format!(
            "Checksum mismatch for {}@{}: expected {}, got {}",
            pkg.name, pkg.version, pkg.checksum, digest
        ));
    }

    // Extract tarball
    let pkg_dir = dest_dir.join(&pkg.name);
    std::fs::create_dir_all(&pkg_dir)
        .map_err(|e| format!("Failed to create package dir: {}", e))?;

    let cursor = std::io::Cursor::new(&bytes);
    let gz = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&pkg_dir)
        .map_err(|e| format!("Failed to extract {}: {}", pkg.name, e))?;

    Ok(digest)
}

/// Publish a package to the registry.
pub fn publish_package(
    manifest_json: &str,
    tarball: &[u8],
    token: &str,
) -> Result<(), String> {
    let boundary = "VPM_BOUNDARY_12345";
    let mut body = Vec::new();

    // Manifest part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"manifest\"\r\n\r\n");
    body.extend_from_slice(manifest_json.as_bytes());
    body.extend_from_slice(b"\r\n");

    // Tarball part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"tarball\"; filename=\"package.tar.gz\"\r\n");
    body.extend_from_slice(b"Content-Type: application/gzip\r\n\r\n");
    body.extend_from_slice(tarball);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let url = format!("{}/api/v1/publish", REGISTRY_URL);
    let content_type = format!("multipart/form-data; boundary={}", boundary);

    match ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Content-Type", &content_type)
        .send_bytes(&body)
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to publish: {}", e)),
    }
}
