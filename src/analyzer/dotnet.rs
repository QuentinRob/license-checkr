use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

use crate::models::{Dependency, Ecosystem, LicenseRisk, LicenseSource, PolicyVerdict};

/// Analyzer for .NET projects using NuGet or Paket.
///
/// Supports three manifest formats:
/// - SDK-style `*.csproj` / `*.fsproj` (`<PackageReference>` elements)
/// - Legacy `packages.config` (`<package>` elements)
/// - `paket.lock` (NUGET section entries)
///
/// All `.csproj` / `.fsproj` files directly under the project root are scanned.
pub struct DotNetAnalyzer;

impl DotNetAnalyzer {
    /// Create a new `DotNetAnalyzer`.
    pub fn new() -> Self {
        Self
    }
}

impl super::Analyzer for DotNetAnalyzer {
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>> {
        let mut deps: Vec<Dependency> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // Parse *.csproj and *.fsproj (PackageReference)
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if matches!(
                    p.extension().and_then(|s| s.to_str()),
                    Some("csproj" | "fsproj")
                ) {
                    if let Ok(parsed) = parse_project_file(&p) {
                        for d in parsed {
                            let key = format!("{}:{}", d.name, d.version);
                            if seen.insert(key) {
                                deps.push(d);
                            }
                        }
                    }
                }
            }
        }

        // Parse packages.config (legacy NuGet)
        let packages_config = path.join("packages.config");
        if packages_config.exists() {
            if let Ok(parsed) = parse_packages_config(&packages_config) {
                for d in parsed {
                    let key = format!("{}:{}", d.name, d.version);
                    if seen.insert(key) {
                        deps.push(d);
                    }
                }
            }
        }

        // Parse paket.lock
        let paket_lock = path.join("paket.lock");
        if paket_lock.exists() {
            if let Ok(parsed) = parse_paket_lock(&paket_lock) {
                for d in parsed {
                    let key = format!("{}:{}", d.name, d.version);
                    if seen.insert(key) {
                        deps.push(d);
                    }
                }
            }
        }

        Ok(deps)
    }
}

fn make_dep(name: &str, version: &str) -> Dependency {
    Dependency {
        name: name.to_string(),
        version: version.to_string(),
        ecosystem: Ecosystem::DotNet,
        license_raw: None,
        license_spdx: None,
        risk: LicenseRisk::Unknown,
        verdict: PolicyVerdict::Warn,
        source: LicenseSource::Unknown,
    }
}

/// Parse `<PackageReference Include="..." Version="..." />` from `.csproj` / `.fsproj`.
fn parse_project_file(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut deps = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                if tag == "PackageReference" {
                    let mut name = String::new();
                    let mut version = String::new();
                    for attr in e.attributes().flatten() {
                        let key =
                            String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
                        let val = attr.unescape_value().unwrap_or_default().into_owned();
                        match key.as_str() {
                            "Include" => name = val,
                            "Version" => version = val,
                            _ => {}
                        }
                    }
                    if !name.is_empty() {
                        deps.push(make_dep(&name, &version));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(deps)
}

/// Parse `<package id="..." version="..." />` from `packages.config`.
fn parse_packages_config(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut deps = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                if tag == "package" {
                    let mut id = String::new();
                    let mut version = String::new();
                    for attr in e.attributes().flatten() {
                        let key =
                            String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
                        let val = attr.unescape_value().unwrap_or_default().into_owned();
                        match key.as_str() {
                            "id" => id = val,
                            "version" => version = val,
                            _ => {}
                        }
                    }
                    if !id.is_empty() {
                        deps.push(make_dep(&id, &version));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(deps)
}

/// Parse `paket.lock` — NUGET section entries like `    PackageName (1.2.3)`.
fn parse_paket_lock(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    // Matches lines like:     Newtonsoft.Json (13.0.1)
    let re = Regex::new(r"^\s{4}(\S+)\s+\(([^)]+)\)")?;
    let mut deps = Vec::new();
    let mut in_nuget = false;

    for line in content.lines() {
        if line.trim_end() == "NUGET" {
            in_nuget = true;
            continue;
        }
        // A new top-level section (no leading spaces) ends the NUGET block
        if !line.starts_with(' ') && !line.is_empty() {
            in_nuget = false;
        }
        if in_nuget {
            if let Some(caps) = re.captures(line) {
                deps.push(make_dep(&caps[1], &caps[2]));
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
    fn test_parse_csproj() {
        let xml = r#"<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
    <PackageReference Include="Serilog" Version="2.12.0" />
  </ItemGroup>
</Project>"#;
        let mut f = NamedTempFile::with_suffix(".csproj").unwrap();
        write!(f, "{}", xml).unwrap();
        let deps = parse_project_file(f.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "Newtonsoft.Json");
        assert_eq!(deps[0].version, "13.0.1");
        assert_eq!(deps[1].name, "Serilog");
        assert_eq!(deps[1].version, "2.12.0");
    }

    #[test]
    fn test_parse_packages_config() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<packages>
  <package id="Newtonsoft.Json" version="13.0.1" targetFramework="net452" />
  <package id="NUnit" version="3.13.3" targetFramework="net452" />
</packages>"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", xml).unwrap();
        let deps = parse_packages_config(f.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "Newtonsoft.Json");
        assert_eq!(deps[0].version, "13.0.1");
    }

    #[test]
    fn test_parse_paket_lock() {
        let content = r#"REFERENCES

NUGET
  remote: https://api.nuget.org/v3/index.json
    Newtonsoft.Json (13.0.1)
    Serilog (2.12.0)

GITHUB
  remote: some/repo
    file.fs
"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        let deps = parse_paket_lock(f.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "Newtonsoft.Json");
        assert_eq!(deps[1].name, "Serilog");
    }
}
