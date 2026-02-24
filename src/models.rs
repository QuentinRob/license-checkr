use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    pub license_raw: Option<String>,
    pub license_spdx: Option<String>,
    pub risk: LicenseRisk,
    pub verdict: PolicyVerdict,
    pub source: LicenseSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LicenseRisk {
    Permissive,
    WeakCopyleft,
    StrongCopyleft,
    Proprietary,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyVerdict {
    Pass,
    Warn,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Ecosystem {
    Rust,
    Python,
    Java,
    Node,
}

impl std::fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ecosystem::Rust => write!(f, "Rust"),
            Ecosystem::Python => write!(f, "Python"),
            Ecosystem::Java => write!(f, "Java"),
            Ecosystem::Node => write!(f, "Node"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LicenseSource {
    Manifest,
    Registry,
    Unknown,
}

impl std::fmt::Display for LicenseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LicenseSource::Manifest => write!(f, "manifest"),
            LicenseSource::Registry => write!(f, "registry"),
            LicenseSource::Unknown => write!(f, "unknown"),
        }
    }
}
