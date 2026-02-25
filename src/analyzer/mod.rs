use std::path::Path;

use anyhow::Result;

use crate::models::Dependency;

pub mod dotnet;
pub mod java;
pub mod node;
pub mod python;
pub mod rust;

/// Common interface for all ecosystem-specific dependency analyzers.
///
/// Each implementation parses one or more manifest files found under `path`
/// and returns a deduplicated list of [`Dependency`] values with name, version,
/// and any license information that can be extracted from the manifest itself.
///
/// License classification and policy evaluation happen in the main orchestration
/// layer, not inside the analyzer.
pub trait Analyzer {
    /// Parse manifests under `path` and return the discovered dependencies.
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>>;
}
