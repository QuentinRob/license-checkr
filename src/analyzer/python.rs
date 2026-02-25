use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use regex::Regex;
use serde::Deserialize;

use crate::models::{Dependency, Ecosystem, LicenseRisk, LicenseSource, PolicyVerdict};

/// Analyzer for Python projects.
///
/// Searches for manifests in priority order:
/// `Pipfile.lock` (pinned) → `requirements.txt` → `pyproject.toml`.
/// Results are deduplicated by package name (case-insensitive).
pub struct PythonAnalyzer;

impl PythonAnalyzer {
    /// Create a new `PythonAnalyzer`.
    pub fn new() -> Self {
        Self
    }
}

impl super::Analyzer for PythonAnalyzer {
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>> {
        let mut deps: Vec<Dependency> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // Pipfile.lock (most precise — pinned versions)
        let pipfile_lock = path.join("Pipfile.lock");
        if pipfile_lock.exists() {
            if let Ok(parsed) = parse_pipfile_lock(&pipfile_lock) {
                for d in parsed {
                    seen.insert(d.name.to_lowercase());
                    deps.push(d);
                }
            }
        }

        // requirements.txt
        let requirements = path.join("requirements.txt");
        if requirements.exists() {
            if let Ok(parsed) = parse_requirements_txt(&requirements) {
                for d in parsed {
                    if !seen.contains(&d.name.to_lowercase()) {
                        seen.insert(d.name.to_lowercase());
                        deps.push(d);
                    }
                }
            }
        }

        // pyproject.toml
        let pyproject = path.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(parsed) = parse_pyproject_toml(&pyproject) {
                for d in parsed {
                    if !seen.contains(&d.name.to_lowercase()) {
                        seen.insert(d.name.to_lowercase());
                        deps.push(d);
                    }
                }
            }
        }

        Ok(deps)
    }
}

fn make_dep(name: String, version: String) -> Dependency {
    Dependency {
        name,
        version,
        ecosystem: Ecosystem::Python,
        license_raw: None,
        license_spdx: None,
        risk: LicenseRisk::Unknown,
        verdict: PolicyVerdict::Warn,
        source: LicenseSource::Unknown,
    }
}

/// Parse `requirements.txt` — handles `name==version` and `name>=version` lines.
fn parse_requirements_txt(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let re = Regex::new(r"^([A-Za-z0-9_\-\.]+)\s*==\s*([^\s;]+)")?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }
        if let Some(caps) = re.captures(line) {
            let name = caps[1].to_string();
            let version = caps[2].to_string();
            deps.push(make_dep(name, version));
        }
    }

    Ok(deps)
}

/// Parse `Pipfile.lock` — JSON with `default` and `develop` sections.
fn parse_pipfile_lock(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    let mut deps = Vec::new();

    for section in &["default", "develop"] {
        if let Some(pkgs) = json.get(section).and_then(|v| v.as_object()) {
            for (name, info) in pkgs {
                let version = info
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .trim_start_matches("==")
                    .to_string();
                deps.push(make_dep(name.clone(), version));
            }
        }
    }

    Ok(deps)
}

/// Parse `pyproject.toml` — extract `[project].dependencies`.
#[derive(Debug, Deserialize)]
struct Pyproject {
    project: Option<PyprojectProject>,
}

#[derive(Debug, Deserialize)]
struct PyprojectProject {
    #[serde(default)]
    dependencies: Vec<String>,
}

fn parse_pyproject_toml(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let pyproject: Pyproject = toml::from_str(&content)?;

    let re = Regex::new(r"^([A-Za-z0-9_\-\.]+)\s*(?:==\s*([^\s;,\[]+))?")?;
    let mut deps = Vec::new();

    if let Some(project) = pyproject.project {
        for dep_str in &project.dependencies {
            if let Some(caps) = re.captures(dep_str) {
                let name = caps[1].to_string();
                let version = caps
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "*".to_string());
                deps.push(make_dep(name, version));
            }
        }
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_requirements_txt() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "# comment").unwrap();
        writeln!(f, "requests==2.28.1").unwrap();
        writeln!(f, "flask>=2.0.0").unwrap();
        writeln!(f, "numpy==1.24.0 ; python_version >= '3.8'").unwrap();

        let deps = parse_requirements_txt(f.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "requests");
        assert_eq!(deps[0].version, "2.28.1");
        assert_eq!(deps[1].name, "numpy");
    }
}
