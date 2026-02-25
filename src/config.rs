use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::models::PolicyVerdict;

/// Root configuration structure, deserialized from `license-checkr.toml`.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// License policy rules.
    pub policy: PolicyConfig,
}

/// Defines how licenses are evaluated.
#[derive(Debug, Deserialize)]
pub struct PolicyConfig {
    /// Verdict applied to any license not explicitly listed in `licenses`.
    /// Defaults to `warn`.
    #[serde(default = "default_policy_action")]
    pub default: PolicyAction,
    /// Per-license overrides keyed by SPDX identifier (e.g. `"MIT"`, `"GPL-3.0"`).
    #[serde(default)]
    pub licenses: HashMap<String, PolicyAction>,
}

fn default_policy_action() -> PolicyAction {
    PolicyAction::Warn
}

/// The action to take when a dependency's license matches a policy rule.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    /// Dependency is compliant; no action needed.
    Pass,
    /// Dependency warrants review but does not fail the scan.
    Warn,
    /// Dependency violates policy; the CLI exits with code 1.
    Error,
}

impl PolicyAction {
    /// Convert to the corresponding [`PolicyVerdict`].
    pub fn to_verdict(&self) -> PolicyVerdict {
        match self {
            PolicyAction::Pass => PolicyVerdict::Pass,
            PolicyAction::Warn => PolicyVerdict::Warn,
            PolicyAction::Error => PolicyVerdict::Error,
        }
    }
}

impl Default for Config {
    /// Built-in default policy used when no config file is found.
    ///
    /// Permissive licenses pass, weak-copyleft licenses warn, and strong-copyleft
    /// licenses (GPL, AGPL) produce an error.
    fn default() -> Self {
        let mut licenses = HashMap::new();
        licenses.insert("MIT".to_string(), PolicyAction::Pass);
        licenses.insert("Apache-2.0".to_string(), PolicyAction::Pass);
        licenses.insert("BSD-2-Clause".to_string(), PolicyAction::Pass);
        licenses.insert("BSD-3-Clause".to_string(), PolicyAction::Pass);
        licenses.insert("ISC".to_string(), PolicyAction::Pass);
        licenses.insert("LGPL-2.1".to_string(), PolicyAction::Warn);
        licenses.insert("GPL-2.0".to_string(), PolicyAction::Error);
        licenses.insert("GPL-3.0".to_string(), PolicyAction::Error);
        licenses.insert("AGPL-3.0".to_string(), PolicyAction::Error);
        licenses.insert("unknown".to_string(), PolicyAction::Warn);

        Config {
            policy: PolicyConfig {
                default: PolicyAction::Warn,
                licenses,
            },
        }
    }
}

/// Load the policy configuration, searching in order:
///
/// 1. `config_override` — path passed via `--config`
/// 2. `<project_path>/license-checkr.toml`
/// 3. `~/.config/license-checkr/config.toml`
/// 4. Built-in [`Config::default`]
pub fn load_config(project_path: &Path, config_override: Option<&Path>) -> Result<Config> {
    if let Some(path) = config_override {
        let content = std::fs::read_to_string(path)?;
        return Ok(toml::from_str(&content)?);
    }

    let project_config = project_path.join("license-checkr.toml");
    if project_config.exists() {
        let content = std::fs::read_to_string(&project_config)?;
        return Ok(toml::from_str(&content)?);
    }

    if let Some(home) = dirs::home_dir() {
        let home_config = home
            .join(".config")
            .join("license-checkr")
            .join("config.toml");
        if home_config.exists() {
            let content = std::fs::read_to_string(&home_config)?;
            return Ok(toml::from_str(&content)?);
        }
    }

    Ok(Config::default())
}

/// Determine the policy verdict for a given SPDX license identifier.
///
/// Lookup order:
/// 1. Exact match in `config.policy.licenses`
/// 2. `"unknown"` key if `license_spdx` is `None` or `"unknown"`
/// 3. `config.policy.default`
pub fn apply_policy(config: &Config, license_spdx: Option<&str>) -> PolicyVerdict {
    let license = license_spdx.unwrap_or("unknown");

    if let Some(action) = config.policy.licenses.get(license) {
        return action.to_verdict();
    }

    if license == "unknown" {
        if let Some(action) = config.policy.licenses.get("unknown") {
            return action.to_verdict();
        }
    }

    config.policy.default.to_verdict()
}
