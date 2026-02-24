use std::path::Path;

use crate::models::Ecosystem;

/// Auto-detect supported ecosystems by scanning for known manifest files.
pub fn detect_ecosystems(path: &Path) -> Vec<Ecosystem> {
    let mut ecosystems = Vec::new();

    if path.join("Cargo.toml").exists() || path.join("Cargo.lock").exists() {
        ecosystems.push(Ecosystem::Rust);
    }

    if path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("Pipfile.lock").exists()
    {
        ecosystems.push(Ecosystem::Python);
    }

    if path.join("pom.xml").exists()
        || path.join("build.gradle").exists()
        || path.join("build.gradle.kts").exists()
    {
        ecosystems.push(Ecosystem::Java);
    }

    if path.join("package.json").exists()
        || path.join("package-lock.json").exists()
        || path.join("yarn.lock").exists()
    {
        ecosystems.push(Ecosystem::Node);
    }

    ecosystems
}
