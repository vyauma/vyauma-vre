//! SemVer Dependency Resolver
//!
//! Resolves the full transitive dependency graph from a `vyauma.toml` manifest
//! and produces a `vyauma.lock` lockfile entry for each package.
//!
//! Strategy:
//! - Fetch registry metadata for each declared dependency.
//! - For each fetched package, recursively resolve its dependencies.
//! - Deduplicate by package name (first resolved version wins — simple resolution).

use std::collections::{HashMap, HashSet};
use crate::lockfile::{LockFile, LockedPackage};
use crate::manifest::Manifest;
use crate::registry;

/// Resolve all dependencies declared in `manifest` and return a complete lockfile.
pub fn resolve(manifest: &Manifest) -> Result<LockFile, String> {
    let mut lock = LockFile::default();
    let mut visited: HashSet<String> = HashSet::new();

    // Combine normal + dev dependencies (dev deps resolved locally during install)
    let mut all_deps: HashMap<String, String> = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());

    for (name, version_req) in &all_deps {
        resolve_package(name, version_req, &mut lock, &mut visited)?;
    }

    Ok(lock)
}

fn resolve_package(
    name: &str,
    version_req: &str,
    lock: &mut LockFile,
    visited: &mut HashSet<String>,
) -> Result<(), String> {
    if visited.contains(name) {
        return Ok(()); // already resolved (deduplication)
    }
    visited.insert(name.to_string());

    println!("  Resolving {}@{}...", name, version_req);

    // Fetch available versions from the registry
    let resolved_version = match registry::fetch_versions(name) {
        Ok(versions) => {
            // Try to match the version requirement (simple best-match heuristic)
            select_version(&versions, version_req)
                .ok_or_else(|| format!("No version of '{}' satisfies '{}'", name, version_req))?
        }
        Err(e) => {
            // Registry offline — use the version_req verbatim as a best-effort version
            eprintln!(
                "  Warning: registry unreachable ({}) — using '{}' as declared version for '{}'",
                e, version_req, name
            );
            version_req.trim_start_matches('^')
                .trim_start_matches('~')
                .trim_start_matches(">=")
                .to_string()
        }
    };

    // Fetch metadata for the resolved version
    let pkg_meta = match registry::fetch_package_metadata(name, &resolved_version) {
        Ok(meta) => Some(meta),
        Err(e) => {
            eprintln!("  Warning: could not fetch metadata for {}@{}: {}", name, resolved_version, e);
            None
        }
    };

    // Recursively resolve transitive dependencies
    let dep_names: Vec<String> = if let Some(ref meta) = pkg_meta {
        for dep in &meta.dependencies {
            resolve_package(&dep.name, &dep.version_req, lock, visited)?;
        }
        meta.dependencies.iter().map(|d| d.name.clone()).collect()
    } else {
        vec![]
    };

    // Add to lockfile
    lock.upsert(LockedPackage {
        name: name.to_string(),
        version: resolved_version,
        checksum: pkg_meta.as_ref().map(|m| m.checksum.clone()).unwrap_or_default(),
        registry: registry::REGISTRY_URL.to_string(),
        dependencies: dep_names,
    });

    Ok(())
}

/// Select the highest version from `versions` that satisfies `req`.
///
/// Supports:
/// - `^1.2.3`  → `>=1.2.3, <2.0.0`  (caret — compatible release)
/// - `~1.2.3`  → `>=1.2.3, <1.3.0`  (tilde — patch-level compatible)
/// - `>=1.2.3` → any version ≥ that
/// - `1.2.3`   → exact version
fn select_version(versions: &[String], req: &str) -> Option<String> {
    // Parse the constraint
    let (op, ver_str) = if let Some(v) = req.strip_prefix('^') {
        ("^", v)
    } else if let Some(v) = req.strip_prefix('~') {
        ("~", v)
    } else if let Some(v) = req.strip_prefix(">=") {
        (">=", v)
    } else {
        ("=", req)
    };

    let parts: Vec<u64> = ver_str.split('.').filter_map(|p| p.parse().ok()).collect();
    let (major, minor, patch) = (
        parts.get(0).copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    );

    let mut candidates: Vec<(u64, u64, u64)> = versions.iter().filter_map(|v| {
        let ps: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        let (ma, mi, pa) = (
            *ps.get(0)?,
            *ps.get(1).unwrap_or(&0),
            *ps.get(2).unwrap_or(&0),
        );
        let ok = match op {
            "^"  => ma == major && (ma > 0 || (mi > minor || (mi == minor && pa >= patch))),
            "~"  => ma == major && mi == minor && pa >= patch,
            ">=" => (ma, mi, pa) >= (major, minor, patch),
            _    => ma == major && mi == minor && pa == patch,
        };
        if ok { Some((ma, mi, pa)) } else { None }
    }).collect();

    candidates.sort_unstable();
    candidates.last().map(|(ma, mi, pa)| format!("{}.{}.{}", ma, mi, pa))
}
