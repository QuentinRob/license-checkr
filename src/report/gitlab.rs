//! GitLab Code Quality JSON report renderer.
//!
//! Produces a JSON array conforming to the
//! [GitLab Code Quality](https://docs.gitlab.com/ee/ci/testing/code_quality.html)
//! artifact format. Only dependencies with a `Warn` or `Error` verdict are
//! included; `Pass` verdicts are omitted.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use anyhow::Result;
use serde::Serialize;

use crate::models::{Dependency, Ecosystem, PolicyVerdict};

/// A single Code Quality issue as expected by GitLab.
#[derive(Serialize)]
struct CodeQualityIssue {
    description: String,
    check_name: String,
    fingerprint: String,
    severity: &'static str,
    location: Location,
}

#[derive(Serialize)]
struct Location {
    path: String,
    lines: Lines,
}

#[derive(Serialize)]
struct Lines {
    begin: u32,
}

fn manifest_path(ecosystem: &Ecosystem) -> &'static str {
    match ecosystem {
        Ecosystem::Rust => "Cargo.lock",
        Ecosystem::Python => "requirements.txt",
        Ecosystem::Java => "pom.xml",
        Ecosystem::Node => "package-lock.json",
        Ecosystem::DotNet => "packages.config",
    }
}

fn severity(verdict: &PolicyVerdict) -> &'static str {
    match verdict {
        PolicyVerdict::Error => "blocker",
        PolicyVerdict::Warn => "minor",
        PolicyVerdict::Pass => "info",
    }
}

fn fingerprint(check_name: &str, name: &str, version: &str, license: &str) -> String {
    let mut hasher = DefaultHasher::new();
    check_name.hash(&mut hasher);
    name.hash(&mut hasher);
    version.hash(&mut hasher);
    license.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn dep_to_issue(dep: &Dependency) -> CodeQualityIssue {
    let license = dep
        .license_spdx
        .as_deref()
        .or(dep.license_raw.as_deref())
        .unwrap_or("unknown");

    let check_name = match dep.verdict {
        PolicyVerdict::Error => "license-checkr/license-error",
        PolicyVerdict::Warn => "license-checkr/license-warn",
        PolicyVerdict::Pass => "license-checkr/license-pass",
    };

    let description = format!(
        "Dependency '{}@{}' uses license '{}' — policy verdict: {}",
        dep.name, dep.version, license, dep.verdict
    );

    CodeQualityIssue {
        description,
        check_name: check_name.to_string(),
        fingerprint: fingerprint(check_name, &dep.name, &dep.version, license),
        severity: severity(&dep.verdict),
        location: Location {
            path: manifest_path(&dep.ecosystem).to_string(),
            lines: Lines { begin: 1 },
        },
    }
}

/// Render a GitLab Code Quality JSON report for a single project scan.
pub fn render(deps: &[Dependency]) -> Result<String> {
    let issues: Vec<CodeQualityIssue> = deps
        .iter()
        .filter(|d| d.verdict != PolicyVerdict::Pass)
        .map(dep_to_issue)
        .collect();

    Ok(serde_json::to_string_pretty(&issues)?)
}

/// Render a GitLab Code Quality JSON report for a workspace scan.
///
/// All sub-project dependencies are merged into a single flat issue list.
pub fn render_workspace(projects: &[crate::models::ProjectScan]) -> Result<String> {
    let issues: Vec<CodeQualityIssue> = projects
        .iter()
        .flat_map(|p| &p.deps)
        .filter(|d| d.verdict != PolicyVerdict::Pass)
        .map(dep_to_issue)
        .collect();

    Ok(serde_json::to_string_pretty(&issues)?)
}
