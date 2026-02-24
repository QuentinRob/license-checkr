use anyhow::Result;
use reqwest::Client;

/// Fetch the license for an npm package from the npm registry.
pub async fn fetch_license(client: &Client, name: &str, version: &str) -> Result<Option<String>> {
    // npm registry endpoint: GET /{name}/{version}
    // Scoped packages need URL encoding: @scope/pkg → %40scope%2Fpkg
    let encoded_name = name.replace('@', "%40").replace('/', "%2F");
    let url = if version == "*" {
        format!("https://registry.npmjs.org/{}", encoded_name)
    } else {
        format!("https://registry.npmjs.org/{}/{}", encoded_name, version)
    };

    let response = client
        .get(&url)
        .header("User-Agent", "license-checkr/0.1.0")
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await?;

    // For /{name}/{version} the license is at top level.
    // For /{name} (latest), it's at .dist-tags.latest then versions[latest].license
    let license = if version == "*" {
        let latest = data
            .get("dist-tags")
            .and_then(|d| d.get("latest"))
            .and_then(|v| v.as_str());
        if let Some(ver) = latest {
            data.get("versions")
                .and_then(|vs| vs.get(ver))
                .and_then(|v| v.get("license"))
                .and_then(|l| l.as_str())
                .map(str::to_string)
        } else {
            None
        }
    } else {
        data.get("license")
            .and_then(|l| l.as_str())
            .map(str::to_string)
    };

    Ok(license)
}
