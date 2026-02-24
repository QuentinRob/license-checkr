use anyhow::Result;
use reqwest::Client;

/// Fetch the license for a crate from crates.io.
pub async fn fetch_license(client: &Client, name: &str, version: &str) -> Result<Option<String>> {
    let url = format!("https://crates.io/api/v1/crates/{}/{}", name, version);

    let response = client
        .get(&url)
        .header("User-Agent", "license-checkr/0.1.0 (license compliance tool)")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await?;
    let license = data
        .get("version")
        .and_then(|v| v.get("license"))
        .and_then(|l| l.as_str())
        .map(str::to_string);

    Ok(license)
}
