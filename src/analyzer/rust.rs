use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::models::{Dependency, Ecosystem, LicenseRisk, LicenseSource, PolicyVerdict};

#[derive(Debug, Deserialize)]
struct CargoLock {
    #[serde(default)]
    package: Vec<CargoLockPackage>,
}

#[derive(Debug, Deserialize)]
struct CargoLockPackage {
    name: String,
    version: String,
    /// Packages without a `source` field are local workspace members.
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrateManifest {
    package: Option<CratePackage>,
}

#[derive(Debug, Deserialize)]
struct CratePackage {
    license: Option<String>,
}

/// Look up the `license` field for a crate from the local Cargo registry cache.
///
/// Cargo stores downloaded crate sources at:
/// `$CARGO_HOME/registry/src/<registry-hash>/<name>-<version>/Cargo.toml`
///
/// Returns `None` if the crate is not cached locally or has no `license` field.
fn license_from_cargo_cache(name: &str, version: &str) -> Option<String> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cargo")))?;

    let registry_src = cargo_home.join("registry").join("src");
    let crate_dir_name = format!("{}-{}", name, version);

    // registry/src contains one subdirectory per registry host
    // (e.g. `index.crates.io-6f17d22bba15001f`).
    for entry in std::fs::read_dir(&registry_src).ok()?.flatten() {
        let cargo_toml = entry.path().join(&crate_dir_name).join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if let Ok(manifest) = toml::from_str::<CrateManifest>(&content) {
                if let Some(license) = manifest.package.and_then(|p| p.license) {
                    return Some(license);
                }
            }
        }
    }

    None
}

/// Analyzer for Rust projects managed by Cargo.
///
/// Parses `Cargo.lock` and returns all external crate dependencies,
/// filtering out local workspace members (entries with no `source` field).
pub struct RustAnalyzer;

impl RustAnalyzer {
    /// Create a new `RustAnalyzer`.
    pub fn new() -> Self {
        Self
    }
}

impl super::Analyzer for RustAnalyzer {
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>> {
        let lock_path = path.join("Cargo.lock");
        if !lock_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&lock_path)?;
        let lock: CargoLock = toml::from_str(&content)?;

        let deps = lock
            .package
            .into_iter()
            // Skip local workspace members (they have no `source`)
            .filter(|p| p.source.is_some())
            .map(|p| {
                let license = license_from_cargo_cache(&p.name, &p.version);
                let source = if license.is_some() {
                    LicenseSource::Cache
                } else {
                    LicenseSource::Unknown
                };
                Dependency {
                    name: p.name,
                    version: p.version,
                    ecosystem: Ecosystem::Rust,
                    license_spdx: license.clone(),
                    license_raw: license,
                    risk: LicenseRisk::Unknown,
                    verdict: PolicyVerdict::Warn,
                    source,
                }
            })
            .collect();

        Ok(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_cargo_lock() {
        let content = r#"
version = 3

[[package]]
name = "my-app"
version = "0.1.0"

[[package]]
name = "serde"
version = "1.0.150"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc123"

[[package]]
name = "tokio"
version = "1.25.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "def456"
"#;

        let lock: CargoLock = toml::from_str(content).unwrap();
        let external: Vec<_> = lock.package.into_iter().filter(|p| p.source.is_some()).collect();
        assert_eq!(external.len(), 2);
        assert_eq!(external[0].name, "serde");
        assert_eq!(external[1].name, "tokio");
    }
}
