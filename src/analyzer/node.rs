use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;

use crate::models::{Dependency, Ecosystem, LicenseRisk, LicenseSource, PolicyVerdict};

pub struct NodeAnalyzer;

impl NodeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl super::Analyzer for NodeAnalyzer {
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>> {
        let mut deps: Vec<Dependency> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // package-lock.json (most precise — pinned versions with optional license field)
        let lock = path.join("package-lock.json");
        if lock.exists() {
            if let Ok(parsed) = parse_package_lock_json(&lock, path) {
                for d in parsed {
                    let key = format!("{}@{}", d.name, d.version);
                    if seen.insert(key) {
                        deps.push(d);
                    }
                }
            }
        }

        // yarn.lock
        let yarn = path.join("yarn.lock");
        if yarn.exists() {
            if let Ok(parsed) = parse_yarn_lock(&yarn) {
                for d in parsed {
                    let key = format!("{}@{}", d.name, d.version);
                    if seen.insert(key) {
                        deps.push(d);
                    }
                }
            }
        }

        // package.json (no pinned versions, fall back to declared range)
        let pkg = path.join("package.json");
        if pkg.exists() && deps.is_empty() {
            if let Ok(parsed) = parse_package_json(&pkg) {
                for d in parsed {
                    let key = format!("{}@{}", d.name, d.version);
                    if seen.insert(key) {
                        deps.push(d);
                    }
                }
            }
        }

        Ok(deps)
    }
}

fn make_dep(name: String, version: String, license: Option<String>) -> Dependency {
    let source = if license.is_some() {
        LicenseSource::Manifest
    } else {
        LicenseSource::Unknown
    };
    let license_spdx = license.clone();
    Dependency {
        name,
        version,
        ecosystem: Ecosystem::Node,
        license_raw: license,
        license_spdx,
        risk: LicenseRisk::Unknown,
        verdict: PolicyVerdict::Warn,
        source,
    }
}

/// Parse `package-lock.json` v2/v3 (the `packages` map).
/// Also tries to read `node_modules/{pkg}/package.json` for offline license data.
fn parse_package_lock_json(lock_path: &Path, project_root: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(lock_path)?;
    let json: Value = serde_json::from_str(&content)?;
    let mut deps = Vec::new();

    if let Some(packages) = json.get("packages").and_then(|v| v.as_object()) {
        for (pkg_path, info) in packages {
            // Skip the root entry (empty string key)
            if pkg_path.is_empty() {
                continue;
            }

            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("*")
                .to_string();

            // Derive package name from path: "node_modules/foo" → "foo"
            // "node_modules/@scope/foo" → "@scope/foo"
            let name = pkg_path
                .strip_prefix("node_modules/")
                .unwrap_or(pkg_path)
                .to_string();

            // License may be present in lock entry
            let license_in_lock = info
                .get("license")
                .and_then(|v| v.as_str())
                .map(str::to_string);

            // Try reading from node_modules for more complete info
            let license = license_in_lock.or_else(|| {
                let nm_pkg_json = project_root
                    .join(pkg_path)
                    .join("package.json");
                read_license_from_package_json(&nm_pkg_json)
            });

            deps.push(make_dep(name, version, license));
        }
    }

    Ok(deps)
}

fn read_license_from_package_json(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;
    json.get("license").and_then(|v| v.as_str()).map(str::to_string)
}

/// Parse `yarn.lock` — custom line-based format.
fn parse_yarn_lock(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let mut deps = Vec::new();
    let mut lines = content.lines().peekable();

    // Regex to extract the package name from header like: "foo@^1.0.0:" or "@scope/foo@^1.0.0:"
    let header_re = Regex::new(r#"^"?(@?[^@"]+)@[^:"]+"?:$"#)?;
    let version_re = Regex::new(r#"^\s+version\s+"([^"]+)""#)?;

    while let Some(line) = lines.next() {
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Package header (not indented, ends with ":")
        if !line.starts_with(' ') && !line.starts_with('\t') {
            let trimmed = line.trim_end_matches(':').trim_matches('"');

            // Handle comma-separated specs: take the first name
            let first_spec = trimmed.split(", ").next().unwrap_or(trimmed);

            if let Some(caps) = header_re.captures(&format!("{}:", first_spec.trim_end_matches(':'))) {
                let pkg_name = caps[1].to_string();
                let mut version = String::new();

                // Look ahead for `version "x.y.z"`
                while let Some(next) = lines.peek() {
                    if next.is_empty() {
                        break;
                    }
                    if let Some(vcaps) = version_re.captures(next) {
                        version = vcaps[1].to_string();
                        lines.next();
                        break;
                    }
                    lines.next();
                }

                if !version.is_empty() {
                    deps.push(make_dep(pkg_name, version, None));
                }
            }
        }
    }

    Ok(deps)
}

/// Parse `package.json` — extract `dependencies` and `devDependencies`.
fn parse_package_json(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&content)?;
    let mut deps = Vec::new();

    for section in &["dependencies", "devDependencies"] {
        if let Some(pkgs) = json.get(section).and_then(|v| v.as_object()) {
            for (name, version_range) in pkgs {
                let version = version_range
                    .as_str()
                    .unwrap_or("*")
                    .trim_start_matches(|c: char| !c.is_ascii_digit() && c != '*')
                    .to_string();
                deps.push(make_dep(name.clone(), version, None));
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
    fn test_parse_package_json() {
        let json = r#"{
  "name": "my-app",
  "dependencies": {
    "express": "^4.18.2",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let deps = parse_package_json(f.path()).unwrap();
        assert_eq!(deps.len(), 3);
    }

    #[test]
    fn test_parse_package_lock_json() {
        let json = r#"{
  "name": "my-app",
  "lockfileVersion": 3,
  "packages": {
    "": { "name": "my-app", "version": "1.0.0" },
    "node_modules/express": {
      "version": "4.18.2",
      "license": "MIT"
    },
    "node_modules/lodash": {
      "version": "4.17.21",
      "license": "MIT"
    }
  }
}"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let deps = parse_package_lock_json(f.path(), Path::new("/tmp")).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "express");
        assert_eq!(deps[0].license_raw, Some("MIT".to_string()));
    }
}
