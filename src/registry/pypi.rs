use anyhow::Result;
use reqwest::Client;

/// Fetch the license for a Python package from PyPI.
pub async fn fetch_license(client: &Client, name: &str, version: &str) -> Result<Option<String>> {
    let url = if version == "*" {
        format!("https://pypi.org/pypi/{}/json", name)
    } else {
        format!("https://pypi.org/pypi/{}/{}/json", name, version)
    };

    let response = client
        .get(&url)
        .header("User-Agent", "license-checkr/0.1.0")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await?;
    let license = data
        .get("info")
        .and_then(|i| i.get("license"))
        .and_then(|l| l.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(license)
}
