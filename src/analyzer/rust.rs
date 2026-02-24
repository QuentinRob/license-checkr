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

pub struct RustAnalyzer;

impl RustAnalyzer {
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
            .map(|p| Dependency {
                name: p.name,
                version: p.version,
                ecosystem: Ecosystem::Rust,
                license_raw: None,
                license_spdx: None,
                risk: LicenseRisk::Unknown,
                verdict: PolicyVerdict::Warn,
                source: LicenseSource::Unknown,
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
