//! Checks GitHub Releases for a newer version and notifies the user via stderr.

use colored::Colorize;
use serde::Deserialize;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

/// Fetches the latest GitHub release and prints a notice to stderr if a newer
/// version is available. Fails silently on any network or parse error so it
/// never blocks or crashes the main workflow.
pub async fn check_for_update() {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent(concat!("license-checkr/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let resp = match client
        .get("https://api.github.com/repos/QuentinRob/license-checkr/releases/latest")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    let release = match resp.json::<GithubRelease>().await {
        Ok(r) => r,
        Err(_) => return,
    };

    let latest = release.tag_name.trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");

    if is_newer(latest, current) {
        eprintln!(
            "\n  {} {} {} {} {}\n  {}\n",
            "↑".yellow().bold(),
            "license-checkr".bold(),
            format!("v{latest}").green().bold(),
            "is available — you have".dimmed(),
            format!("v{current}").dimmed(),
            "https://github.com/QuentinRob/license-checkr/releases/latest".cyan(),
        );
    }
}

/// Returns true if `latest` is a higher semver than `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut parts = v.splitn(3, '.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    };

    match (parse(latest), parse(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn newer_patch() {
        assert!(is_newer("0.2.2", "0.2.1"));
    }

    #[test]
    fn newer_minor() {
        assert!(is_newer("0.3.0", "0.2.9"));
    }

    #[test]
    fn newer_major() {
        assert!(is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn same_version() {
        assert!(!is_newer("0.2.1", "0.2.1"));
    }

    #[test]
    fn older_version() {
        assert!(!is_newer("0.2.0", "0.2.1"));
    }
}
