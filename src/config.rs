use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::models::PolicyVerdict;

/// Root configuration structure, deserialized from `.license-checkr/config.toml`.
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
/// 2. `<project_path>/.license-checkr/config.toml`
/// 3. `~/.config/license-checkr/config.toml`
/// 4. Built-in [`Config::default`]
pub fn load_config(project_path: &Path, config_override: Option<&Path>) -> Result<Config> {
    if let Some(path) = config_override {
        let content = std::fs::read_to_string(path)?;
        return Ok(toml::from_str(&content)?);
    }

    let project_config = project_path.join(".license-checkr").join("config.toml");
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

/// Determine the policy verdict for a given SPDX license identifier or expression.
///
/// Supports compound SPDX expressions with proper operator precedence:
/// - `AND` binds tighter than `OR`
/// - Parentheses override precedence
/// - `WITH` exception clauses are recognised but the base license is used for evaluation
///
/// Examples: `MIT`, `Apache-2.0 OR MIT`, `(Apache-2.0 OR MIT) AND BSD-3-Clause`
pub fn apply_policy(config: &Config, license_spdx: Option<&str>) -> PolicyVerdict {
    let license = license_spdx.unwrap_or("unknown");

    // Exact match first (covers simple identifiers and the literal "unknown")
    if let Some(action) = config.policy.licenses.get(license) {
        return action.to_verdict();
    }

    // Normalize "/" separator (some ecosystems use it as an OR shorthand)
    let normalized = license.replace('/', " OR ");

    eval_spdx_expr(config, &normalized)
}

// ---------------------------------------------------------------------------
// SPDX expression parser
// ---------------------------------------------------------------------------

/// Tokens produced by [`tokenize_spdx`].
#[derive(Debug, PartialEq, Clone)]
enum Token {
    Id(String),
    And,
    Or,
    With,
    LParen,
    RParen,
}

/// Tokenize an SPDX license expression into a flat [`Vec<Token>`].
fn tokenize_spdx(expr: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '(' {
            tokens.push(Token::LParen);
            chars.next();
        } else if c == ')' {
            tokens.push(Token::RParen);
            chars.next();
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '(' || c == ')' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            let token = match s.as_str() {
                "AND" => Token::And,
                "OR" => Token::Or,
                "WITH" => Token::With,
                _ => Token::Id(s),
            };
            tokens.push(token);
        }
    }
    tokens
}

/// Recursive descent parser that evaluates an SPDX expression against `config`.
///
/// Grammar (AND binds tighter than OR):
/// ```text
/// expr     := or_expr
/// or_expr  := and_expr ( "OR" and_expr )*
/// and_expr := atom ( "AND" atom )*
/// atom     := "(" expr ")" | id ( "WITH" id )?
/// ```
struct ExprParser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    config: &'a Config,
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Parse an OR-level expression (lowest precedence).
    fn parse_or(&mut self) -> PolicyVerdict {
        let mut result = self.parse_and();
        while matches!(self.peek(), Some(Token::Or)) {
            self.consume();
            let rhs = self.parse_and();
            result = verdict_or(result, rhs);
        }
        result
    }

    /// Parse an AND-level expression (higher precedence than OR).
    fn parse_and(&mut self) -> PolicyVerdict {
        let mut result = self.parse_atom();
        while matches!(self.peek(), Some(Token::And)) {
            self.consume();
            let rhs = self.parse_atom();
            result = verdict_and(result, rhs);
        }
        result
    }

    /// Parse an atom: a parenthesised sub-expression or a single license id.
    fn parse_atom(&mut self) -> PolicyVerdict {
        match self.peek() {
            Some(Token::LParen) => {
                self.consume(); // consume '('
                let result = self.parse_or();
                if matches!(self.peek(), Some(Token::RParen)) {
                    self.consume(); // consume ')'
                }
                result
            }
            Some(Token::Id(_)) => {
                let id = if let Some(Token::Id(s)) = self.consume() {
                    s
                } else {
                    unreachable!()
                };
                // Skip WITH exception clause — base license is used for policy
                if matches!(self.peek(), Some(Token::With)) {
                    self.consume(); // WITH
                    self.consume(); // exception identifier
                }
                apply_policy_single(self.config, &id)
            }
            _ => self.config.policy.default.to_verdict(),
        }
    }
}

/// Evaluate a full SPDX expression string against the policy.
fn eval_spdx_expr(config: &Config, expr: &str) -> PolicyVerdict {
    let tokens = tokenize_spdx(expr);
    ExprParser { tokens, pos: 0, config }.parse_or()
}

/// Look up a single (non-compound) SPDX identifier in the policy map.
fn apply_policy_single(config: &Config, id: &str) -> PolicyVerdict {
    if let Some(action) = config.policy.licenses.get(id) {
        return action.to_verdict();
    }
    config.policy.default.to_verdict()
}

/// Most permissive (least severe) of two verdicts — used for OR semantics.
/// Pass < Warn < Error
fn verdict_or(a: PolicyVerdict, b: PolicyVerdict) -> PolicyVerdict {
    match (a, b) {
        (PolicyVerdict::Pass, _) | (_, PolicyVerdict::Pass) => PolicyVerdict::Pass,
        (PolicyVerdict::Warn, _) | (_, PolicyVerdict::Warn) => PolicyVerdict::Warn,
        _ => PolicyVerdict::Error,
    }
}

/// Most restrictive (most severe) of two verdicts — used for AND semantics.
/// Error > Warn > Pass
fn verdict_and(a: PolicyVerdict, b: PolicyVerdict) -> PolicyVerdict {
    match (a, b) {
        (PolicyVerdict::Error, _) | (_, PolicyVerdict::Error) => PolicyVerdict::Error,
        (PolicyVerdict::Warn, _) | (_, PolicyVerdict::Warn) => PolicyVerdict::Warn,
        _ => PolicyVerdict::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_simple_pass() {
        let cfg = default_config();
        assert_eq!(apply_policy(&cfg, Some("MIT")), PolicyVerdict::Pass);
        assert_eq!(apply_policy(&cfg, Some("Apache-2.0")), PolicyVerdict::Pass);
    }

    #[test]
    fn test_or_both_pass() {
        let cfg = default_config();
        assert_eq!(
            apply_policy(&cfg, Some("MIT OR Apache-2.0")),
            PolicyVerdict::Pass
        );
    }

    #[test]
    fn test_or_one_pass_one_error() {
        let cfg = default_config();
        // OR → most permissive wins
        assert_eq!(
            apply_policy(&cfg, Some("MIT OR GPL-3.0")),
            PolicyVerdict::Pass
        );
    }

    #[test]
    fn test_and_one_error() {
        let cfg = default_config();
        // AND → most restrictive wins
        assert_eq!(
            apply_policy(&cfg, Some("MIT AND GPL-3.0")),
            PolicyVerdict::Error
        );
    }

    #[test]
    fn test_slash_separator() {
        let cfg = default_config();
        assert_eq!(
            apply_policy(&cfg, Some("MIT/Apache-2.0")),
            PolicyVerdict::Pass
        );
    }

    #[test]
    fn test_unknown_falls_back_to_default() {
        let cfg = default_config();
        assert_eq!(
            apply_policy(&cfg, Some("CUSTOM-LICENSE")),
            PolicyVerdict::Warn // default
        );
    }

    #[test]
    fn test_parentheses_or_then_and() {
        let cfg = default_config();
        // (Apache-2.0 OR MIT) AND BSD-3-Clause
        // Inner OR → Pass (both are Pass); AND Pass → Pass
        assert_eq!(
            apply_policy(&cfg, Some("(Apache-2.0 OR MIT) AND BSD-3-Clause")),
            PolicyVerdict::Pass
        );
    }

    #[test]
    fn test_parentheses_or_with_error_then_and() {
        let cfg = default_config();
        // (MIT OR GPL-3.0) AND BSD-3-Clause
        // Inner OR → Pass (MIT wins); AND Pass → Pass
        assert_eq!(
            apply_policy(&cfg, Some("(MIT OR GPL-3.0) AND BSD-3-Clause")),
            PolicyVerdict::Pass
        );
    }

    #[test]
    fn test_and_precedence_over_or_without_parens() {
        let cfg = default_config();
        // MIT OR GPL-3.0 AND BSD-3-Clause
        // AND binds tighter: MIT OR (GPL-3.0 AND BSD-3-Clause) → MIT OR Error → Pass
        assert_eq!(
            apply_policy(&cfg, Some("MIT OR GPL-3.0 AND BSD-3-Clause")),
            PolicyVerdict::Pass
        );
    }

    #[test]
    fn test_parentheses_force_or_before_and() {
        let cfg = default_config();
        // (MIT OR GPL-3.0) AND GPL-3.0
        // Inner OR → Pass; AND Error → Error
        assert_eq!(
            apply_policy(&cfg, Some("(MIT OR GPL-3.0) AND GPL-3.0")),
            PolicyVerdict::Error
        );
    }

    #[test]
    fn test_with_exception_ignored() {
        let cfg = default_config();
        // WITH clause should be stripped; base license evaluated
        assert_eq!(
            apply_policy(&cfg, Some("GPL-2.0 WITH Classpath-exception-2.0")),
            PolicyVerdict::Error
        );
    }
}
