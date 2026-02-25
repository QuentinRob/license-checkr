use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

use crate::models::{Dependency, Ecosystem, LicenseRisk, LicenseSource, PolicyVerdict};

/// Analyzer for Java/Kotlin projects managed by Maven or Gradle.
///
/// Parses `pom.xml`, `build.gradle` / `build.gradle.kts`, and `gradle.lockfile`.
/// Dependencies are deduplicated by `group:artifact:version` key.
pub struct JavaAnalyzer;

impl JavaAnalyzer {
    /// Create a new `JavaAnalyzer`.
    pub fn new() -> Self {
        Self
    }
}

impl super::Analyzer for JavaAnalyzer {
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>> {
        let mut deps: Vec<Dependency> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // Parse pom.xml
        let pom = path.join("pom.xml");
        if pom.exists() {
            if let Ok(parsed) = parse_pom_xml(&pom) {
                for d in parsed {
                    let key = format!("{}:{}", d.name, d.version);
                    if seen.insert(key) {
                        deps.push(d);
                    }
                }
            }
        }

        // Parse build.gradle / build.gradle.kts
        for gradle_file in &["build.gradle", "build.gradle.kts"] {
            let gradle = path.join(gradle_file);
            if gradle.exists() {
                if let Ok(parsed) = parse_build_gradle(&gradle) {
                    for d in parsed {
                        let key = format!("{}:{}", d.name, d.version);
                        if seen.insert(key) {
                            deps.push(d);
                        }
                    }
                }
            }
        }

        // Parse gradle.lockfile if present
        let lockfile = path.join("gradle.lockfile");
        if lockfile.exists() {
            if let Ok(parsed) = parse_gradle_lockfile(&lockfile) {
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

fn make_dep(group_id: &str, artifact_id: &str, version: &str) -> Dependency {
    // Use "group:artifact" as the name to retain Maven coordinates
    let name = if group_id.is_empty() {
        artifact_id.to_string()
    } else {
        format!("{}:{}", group_id, artifact_id)
    };
    Dependency {
        name,
        version: version.to_string(),
        ecosystem: Ecosystem::Java,
        license_raw: None,
        license_spdx: None,
        risk: LicenseRisk::Unknown,
        verdict: PolicyVerdict::Warn,
        source: LicenseSource::Unknown,
    }
}

/// Parse `pom.xml` using quick-xml event API.
fn parse_pom_xml(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut deps = Vec::new();
    let mut buf = Vec::new();

    let mut in_dependencies = false;
    let mut depth: u32 = 0;
    let mut dependencies_depth: u32 = 0;

    let mut in_dependency = false;
    let mut current_tag = String::new();
    let mut group_id = String::new();
    let mut artifact_id = String::new();
    let mut version = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name =
                    String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                current_tag = name.clone();

                match name.as_str() {
                    "dependencies" if !in_dependency => {
                        in_dependencies = true;
                        dependencies_depth = depth;
                    }
                    "dependency" if in_dependencies => {
                        in_dependency = true;
                        group_id.clear();
                        artifact_id.clear();
                        version.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name =
                    String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();

                if name == "dependency" && in_dependency {
                    if !artifact_id.is_empty() {
                        deps.push(make_dep(&group_id, &artifact_id, &version));
                    }
                    in_dependency = false;
                } else if name == "dependencies" && depth == dependencies_depth {
                    in_dependencies = false;
                }

                depth = depth.saturating_sub(1);
                current_tag.clear();
            }
            Ok(Event::Text(ref e)) => {
                if in_dependency {
                    let text = e.unescape().unwrap_or_default();
                    match current_tag.as_str() {
                        "groupId" => group_id = text.to_string(),
                        "artifactId" => artifact_id = text.to_string(),
                        "version" => version = text.to_string(),
                        _ => {}
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

/// Parse `build.gradle` or `build.gradle.kts` with regex.
fn parse_build_gradle(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let mut deps = Vec::new();

    // Matches: implementation 'group:artifact:version'
    //          implementation "group:artifact:version"
    let re_shorthand =
        Regex::new(r#"(?:implementation|api|compileOnly|runtimeOnly|testImplementation)\s+['"]([^'"]+):([^'"]+):([^'"]+)['"]"#)?;

    for caps in re_shorthand.captures_iter(&content) {
        let group = &caps[1];
        let artifact = &caps[2];
        let version = caps[3].trim_end_matches('"').trim_end_matches('\'');
        deps.push(make_dep(group, artifact, version));
    }

    // Matches: group: 'com.example', name: 'foo', version: '1.0'
    let re_map = Regex::new(
        r#"(?:implementation|api|compileOnly|runtimeOnly|testImplementation)\s+group:\s*['"]([^'"]+)['"]\s*,\s*name:\s*['"]([^'"]+)['"]\s*,\s*version:\s*['"]([^'"]+)['"]"#,
    )?;

    for caps in re_map.captures_iter(&content) {
        deps.push(make_dep(&caps[1], &caps[2], &caps[3]));
    }

    Ok(deps)
}

/// Parse `gradle.lockfile` — format: `group:artifact:version=...`
fn parse_gradle_lockfile(path: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let re = Regex::new(r"^([^:]+):([^:]+):([^=\s]+)")?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(caps) = re.captures(line) {
            deps.push(make_dep(&caps[1], &caps[2], &caps[3]));
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
    fn test_parse_pom_xml() {
        let xml = r#"<?xml version="1.0"?>
<project>
  <dependencies>
    <dependency>
      <groupId>org.apache.commons</groupId>
      <artifactId>commons-lang3</artifactId>
      <version>3.12.0</version>
    </dependency>
    <dependency>
      <groupId>junit</groupId>
      <artifactId>junit</artifactId>
      <version>4.13.2</version>
    </dependency>
  </dependencies>
</project>"#;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", xml).unwrap();
        let deps = parse_pom_xml(f.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "org.apache.commons:commons-lang3");
        assert_eq!(deps[0].version, "3.12.0");
    }

    #[test]
    fn test_parse_build_gradle() {
        let content = r#"
dependencies {
    implementation 'org.springframework:spring-core:5.3.23'
    implementation "com.google.guava:guava:31.1-jre"
    testImplementation 'junit:junit:4.13.2'
}
"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        let deps = parse_build_gradle(f.path()).unwrap();
        assert_eq!(deps.len(), 3);
    }
}
