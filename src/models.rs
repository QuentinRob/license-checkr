use serde::{Deserialize, Serialize};

/// A resolved dependency with its license information and policy verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name as it appears in the manifest (e.g. `serde`, `numpy`, `com.google.guava:guava`).
    pub name: String,
    /// Resolved version string (e.g. `1.0.136`, `3.11.0`).
    pub version: String,
    /// Ecosystem the dependency belongs to.
    pub ecosystem: Ecosystem,
    /// Raw license string as found in the manifest or registry (may be non-SPDX).
    pub license_raw: Option<String>,
    /// Normalized SPDX identifier (e.g. `MIT`, `Apache-2.0`, `GPL-3.0`).
    pub license_spdx: Option<String>,
    /// Risk classification of the license.
    pub risk: LicenseRisk,
    /// Policy verdict after evaluating the license against the active policy.
    pub verdict: PolicyVerdict,
    /// Where the license information was obtained from.
    pub source: LicenseSource,
}

/// Risk level associated with a license type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LicenseRisk {
    /// Minimal restrictions; freely usable in most projects (MIT, Apache-2.0, BSD, ISC, …).
    Permissive,
    /// Share-alike obligations apply only to the library itself (LGPL, MPL-2.0, EPL, …).
    WeakCopyleft,
    /// Any project using this dependency may need to be open-sourced (GPL, AGPL, …).
    StrongCopyleft,
    /// Source code is not publicly available; usage requires a commercial agreement.
    Proprietary,
    /// License could not be determined or is not in the known SPDX table.
    Unknown,
}

impl std::fmt::Display for LicenseRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LicenseRisk::Permissive => write!(f, "Permissive"),
            LicenseRisk::WeakCopyleft => write!(f, "Weak Copyleft"),
            LicenseRisk::StrongCopyleft => write!(f, "Strong Copyleft"),
            LicenseRisk::Proprietary => write!(f, "Proprietary"),
            LicenseRisk::Unknown => write!(f, "Unknown"),
        }
    }
}

/// The result of evaluating a dependency's license against the active policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyVerdict {
    /// License is explicitly allowed by the policy.
    Pass,
    /// License is not blocked but warrants attention.
    Warn,
    /// License violates the policy; the CLI exits with code 1.
    Error,
}

impl std::fmt::Display for PolicyVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyVerdict::Pass => write!(f, "pass"),
            PolicyVerdict::Warn => write!(f, "warn"),
            PolicyVerdict::Error => write!(f, "error"),
        }
    }
}

/// Supported package ecosystems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Ecosystem {
    /// Rust crates managed by Cargo (`Cargo.lock`).
    Rust,
    /// Python packages managed by pip / Poetry / Pipenv.
    Python,
    /// Java/Kotlin artifacts managed by Maven or Gradle.
    Java,
    /// Node.js packages managed by npm, Yarn, or pnpm.
    Node,
    /// .NET NuGet packages (SDK-style projects, `packages.config`, Paket).
    DotNet,
}

impl std::fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ecosystem::Rust => write!(f, "Rust"),
            Ecosystem::Python => write!(f, "Python"),
            Ecosystem::Java => write!(f, "Java"),
            Ecosystem::Node => write!(f, "Node"),
            Ecosystem::DotNet => write!(f, ".NET"),
        }
    }
}

/// Where the license information for a dependency was sourced from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LicenseSource {
    /// Extracted directly from the project manifest (e.g. `package.json`).
    Manifest,
    /// Fetched from the upstream package registry via `--online`.
    Registry,
    /// Read from the local package manager cache (e.g. `~/.cargo/registry/src/…/Cargo.toml`).
    Cache,
    /// Source is undetermined (offline scan with no license in manifest).
    Unknown,
}

impl std::fmt::Display for LicenseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LicenseSource::Manifest => write!(f, "manifest"),
            LicenseSource::Registry => write!(f, "registry"),
            LicenseSource::Cache => write!(f, "cache"),
            LicenseSource::Unknown => write!(f, "unknown"),
        }
    }
}
