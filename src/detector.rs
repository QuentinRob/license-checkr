use std::path::Path;

use crate::models::Ecosystem;

/// Auto-detect supported ecosystems by scanning for known manifest files.
///
/// Detection is based purely on the presence of well-known files in `path`.
/// Multiple ecosystems can be detected for polyglot repositories.
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

    if path.join("packages.config").exists()
        || path.join("paket.dependencies").exists()
        || has_dotnet_project_file(path)
    {
        ecosystems.push(Ecosystem::DotNet);
    }

    ecosystems
}

/// Returns `true` if any `.csproj` or `.fsproj` file exists directly under `path`.
fn has_dotnet_project_file(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    entries.flatten().any(|e| {
        let p = e.path();
        matches!(
            p.extension().and_then(|s| s.to_str()),
            Some("csproj" | "fsproj")
        )
    })
}
