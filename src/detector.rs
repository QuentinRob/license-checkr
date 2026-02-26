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

/// Well-known manifest filenames used to identify a project root.
const MANIFEST_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "requirements.txt",
    "pyproject.toml",
    "Pipfile.lock",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "package.json",
    "package-lock.json",
    "yarn.lock",
    "packages.config",
    "paket.dependencies",
];

/// Directories that should never be descended into during workspace discovery.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "vendor",
    ".cargo",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "bin",
    "obj",
];

/// Walk `root` recursively and return one path per discovered sub-project.
///
/// A directory is considered a project if it contains at least one known
/// manifest file or a `.csproj`/`.fsproj` file. Descending stops once a
/// project is found (nested manifests are not double-counted). Results are
/// returned in sorted order.
pub fn find_workspace_projects(root: &Path) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    let mut visited = std::collections::HashSet::new();
    walk_for_projects(root, &mut results, &mut visited);
    results.sort();
    results
}

fn walk_for_projects(
    dir: &Path,
    out: &mut Vec<std::path::PathBuf>,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) {
    // Canonicalize to guard against symlink cycles
    let canonical = match dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return,
    };
    if !visited.insert(canonical) {
        return;
    }

    // Is this directory itself a project?
    let is_project = MANIFEST_FILES.iter().any(|f| dir.join(f).exists())
        || has_dotnet_project_file(dir);

    if is_project {
        out.push(dir.to_path_buf());
        return; // stop descending — nested manifests not double-counted
    }

    // Recurse into sorted subdirectories, skipping noise dirs
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut subdirs: Vec<std::path::PathBuf> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            if SKIP_DIRS.contains(&name.as_str()) {
                return None;
            }
            Some(path)
        })
        .collect();

    subdirs.sort();

    for sub in subdirs {
        walk_for_projects(&sub, out, visited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(dir: &std::path::Path, name: &str) {
        fs::write(dir.join(name), "").unwrap();
    }

    #[test]
    fn test_finds_root_project() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "Cargo.toml");
        let projects = find_workspace_projects(tmp.path());
        assert_eq!(projects.len(), 1);
        // Canonicalize both sides so Windows UNC prefix (\\?\) doesn't cause mismatches
        assert_eq!(
            projects[0].canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_finds_sub_projects() {
        let tmp = TempDir::new().unwrap();
        let backend = tmp.path().join("backend");
        let frontend = tmp.path().join("frontend");
        fs::create_dir_all(&backend).unwrap();
        fs::create_dir_all(&frontend).unwrap();
        touch(&backend, "Cargo.toml");
        touch(&frontend, "package.json");

        let projects = find_workspace_projects(tmp.path());
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_does_not_recurse_into_sub_project() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        let nested = sub.join("nested");
        fs::create_dir_all(&nested).unwrap();
        touch(&sub, "Cargo.toml");
        touch(&nested, "package.json"); // should not be found independently

        let projects = find_workspace_projects(tmp.path());
        assert_eq!(projects.len(), 1);
        assert_eq!(
            projects[0].canonicalize().unwrap(),
            sub.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_skips_node_modules() {
        let tmp = TempDir::new().unwrap();
        let nm = tmp.path().join("node_modules").join("some-pkg");
        fs::create_dir_all(&nm).unwrap();
        touch(&nm, "package.json");

        let projects = find_workspace_projects(tmp.path());
        assert!(projects.is_empty());
    }

    #[test]
    fn test_skips_target_dir() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("target").join("debug");
        fs::create_dir_all(&target).unwrap();
        touch(&target, "Cargo.toml");

        let projects = find_workspace_projects(tmp.path());
        assert!(projects.is_empty());
    }

    #[test]
    fn test_empty_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let projects = find_workspace_projects(tmp.path());
        assert!(projects.is_empty());
    }

    #[test]
    fn test_results_are_sorted() {
        let tmp = TempDir::new().unwrap();
        for name in &["zz", "aa", "mm"] {
            let dir = tmp.path().join(name);
            fs::create_dir_all(&dir).unwrap();
            touch(&dir, "Cargo.toml");
        }
        let projects = find_workspace_projects(tmp.path());
        assert_eq!(projects.len(), 3);
        // Sorted by path means "aa" < "mm" < "zz"
        let names: Vec<&str> = projects
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, vec!["aa", "mm", "zz"]);
    }
}
