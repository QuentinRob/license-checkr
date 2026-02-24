use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;

/// Fetch the license for a Maven artifact from Maven Central.
///
/// The `name` is expected in `groupId:artifactId` format (as stored in our models).
pub async fn fetch_license(client: &Client, name: &str, version: &str) -> Result<Option<String>> {
    let parts: Vec<&str> = name.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Ok(None);
    }

    let group_id = parts[0];
    let artifact_id = parts[1];

    // Maven Central POM URL
    let group_path = group_id.replace('.', "/");
    let pom_url = format!(
        "https://repo1.maven.org/maven2/{}/{}/{}/{}-{}.pom",
        group_path, artifact_id, version, artifact_id, version
    );

    let response = client
        .get(&pom_url)
        .header("User-Agent", "license-checkr/0.1.0")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let pom_xml = response.text().await?;
    Ok(extract_license_from_pom(&pom_xml))
}

/// Extract the first `<license><name>` from a POM XML string.
fn extract_license_from_pom(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_licenses = false;
    let mut in_license = false;
    let mut in_name = false;
    let mut depth: u32 = 0;
    let mut licenses_depth: u32 = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let tag =
                    String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                match tag.as_str() {
                    "licenses" => {
                        in_licenses = true;
                        licenses_depth = depth;
                    }
                    "license" if in_licenses => {
                        in_license = true;
                    }
                    "name" if in_license => {
                        in_name = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_name => {
                if let Ok(text) = e.unescape() {
                    return Some(text.to_string());
                }
            }
            Ok(Event::End(ref e)) => {
                let tag =
                    String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                match tag.as_str() {
                    "name" => in_name = false,
                    "license" => in_license = false,
                    "licenses" if depth == licenses_depth => {
                        break;
                    }
                    _ => {}
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_license_from_pom() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <licenses>
    <license>
      <name>Apache License, Version 2.0</name>
      <url>https://www.apache.org/licenses/LICENSE-2.0</url>
    </license>
  </licenses>
</project>"#;
        let license = extract_license_from_pom(pom);
        assert_eq!(license, Some("Apache License, Version 2.0".to_string()));
    }
}
