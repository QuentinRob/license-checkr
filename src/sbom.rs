//! SBOM (Software Bill of Materials) generation.
//!
//! Supports three industry-standard formats:
//! - **CycloneDX JSON** v1.5 — widely used in security tooling and CI/CD
//! - **CycloneDX XML** v1.5 — XML variant of the above
//! - **SPDX JSON** v2.3 — ISO/IEC 5962:2021 standard

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::models::{Dependency, Ecosystem};

// ── CycloneDX JSON ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CycloneDxBom {
    pub bom_format: String,
    pub spec_version: String,
    pub serial_number: String,
    pub version: u32,
    pub metadata: CycloneDxMetadata,
    pub components: Vec<CycloneDxComponent>,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxMetadata {
    pub timestamp: String,
    pub tools: Vec<CycloneDxTool>,
    pub component: CycloneDxMetaComponent,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxTool {
    pub vendor: String,
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxMetaComponent {
    #[serde(rename = "type")]
    pub component_type: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxComponent {
    #[serde(rename = "type")]
    pub component_type: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<CycloneDxLicenseWrapper>,
    pub properties: Vec<CycloneDxProperty>,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxLicenseWrapper {
    pub license: CycloneDxLicense,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxLicense {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CycloneDxProperty {
    pub name: String,
    pub value: String,
}

/// Build a CycloneDX BOM from a flat dependency list.
pub fn build_cyclonedx(project_name: &str, deps: &[Dependency]) -> CycloneDxBom {
    CycloneDxBom {
        bom_format: "CycloneDX".to_string(),
        spec_version: "1.5".to_string(),
        serial_number: format!("urn:uuid:{}", pseudo_uuid()),
        version: 1,
        metadata: CycloneDxMetadata {
            timestamp: iso8601_now(),
            tools: vec![CycloneDxTool {
                vendor: "license-checkr".to_string(),
                name: "license-checkr".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }],
            component: CycloneDxMetaComponent {
                component_type: "application".to_string(),
                name: project_name.to_string(),
            },
        },
        components: deps.iter().map(dep_to_cyclonedx).collect(),
    }
}

fn dep_to_cyclonedx(dep: &Dependency) -> CycloneDxComponent {
    let license_id = dep.license_spdx.clone().or_else(|| dep.license_raw.clone());
    let licenses = match license_id {
        Some(ref id) if !id.is_empty() && id != "unknown" => vec![CycloneDxLicenseWrapper {
            license: CycloneDxLicense {
                id: dep.license_spdx.clone(),
                name: if dep.license_spdx.is_none() { dep.license_raw.clone() } else { None },
            },
        }],
        _ => vec![],
    };

    CycloneDxComponent {
        component_type: "library".to_string(),
        name: dep.name.clone(),
        version: dep.version.clone(),
        purl: Some(purl(&dep.ecosystem, &dep.name, &dep.version)),
        licenses,
        properties: vec![
            CycloneDxProperty {
                name: "license-checkr:ecosystem".to_string(),
                value: dep.ecosystem.to_string(),
            },
            CycloneDxProperty {
                name: "license-checkr:verdict".to_string(),
                value: dep.verdict.to_string(),
            },
            CycloneDxProperty {
                name: "license-checkr:risk".to_string(),
                value: dep.risk.to_string(),
            },
            CycloneDxProperty {
                name: "license-checkr:source".to_string(),
                value: dep.source.to_string(),
            },
        ],
    }
}

/// Serialize a CycloneDX BOM to JSON bytes.
pub fn cyclonedx_to_json(bom: &CycloneDxBom) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(bom)?)
}

/// Serialize a CycloneDX BOM to XML bytes.
pub fn cyclonedx_to_xml(bom: &CycloneDxBom) -> Result<Vec<u8>> {
    let mut buf = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<bom xmlns="http://cyclonedx.org/schema/bom/1.5" version="1">
"#,
    );

    buf.push_str("  <metadata>\n");
    buf.push_str(&format!("    <timestamp>{}</timestamp>\n", bom.metadata.timestamp));
    buf.push_str("    <tools>\n");
    for t in &bom.metadata.tools {
        buf.push_str("      <tool>\n");
        buf.push_str(&format!("        <vendor>{}</vendor>\n", xml_escape(&t.vendor)));
        buf.push_str(&format!("        <name>{}</name>\n", xml_escape(&t.name)));
        buf.push_str(&format!("        <version>{}</version>\n", xml_escape(&t.version)));
        buf.push_str("      </tool>\n");
    }
    buf.push_str("    </tools>\n");
    buf.push_str("    <component type=\"application\">\n");
    buf.push_str(&format!("      <name>{}</name>\n", xml_escape(&bom.metadata.component.name)));
    buf.push_str("    </component>\n");
    buf.push_str("  </metadata>\n");

    buf.push_str("  <components>\n");
    for c in &bom.components {
        buf.push_str("    <component type=\"library\">\n");
        buf.push_str(&format!("      <name>{}</name>\n", xml_escape(&c.name)));
        buf.push_str(&format!("      <version>{}</version>\n", xml_escape(&c.version)));
        if let Some(ref p) = c.purl {
            buf.push_str(&format!("      <purl>{}</purl>\n", xml_escape(p)));
        }
        if !c.licenses.is_empty() {
            buf.push_str("      <licenses>\n");
            for lw in &c.licenses {
                buf.push_str("        <license>\n");
                if let Some(ref id) = lw.license.id {
                    buf.push_str(&format!("          <id>{}</id>\n", xml_escape(id)));
                } else if let Some(ref name) = lw.license.name {
                    buf.push_str(&format!("          <name>{}</name>\n", xml_escape(name)));
                }
                buf.push_str("        </license>\n");
            }
            buf.push_str("      </licenses>\n");
        }
        if !c.properties.is_empty() {
            buf.push_str("      <properties>\n");
            for prop in &c.properties {
                buf.push_str(&format!(
                    "        <property name=\"{}\">{}</property>\n",
                    xml_escape(&prop.name),
                    xml_escape(&prop.value),
                ));
            }
            buf.push_str("      </properties>\n");
        }
        buf.push_str("    </component>\n");
    }
    buf.push_str("  </components>\n");
    buf.push_str("</bom>\n");

    Ok(buf.into_bytes())
}

// ── SPDX JSON ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpdxDocument {
    #[serde(rename = "SPDXID")]
    pub spdx_id: String,
    pub spdx_version: String,
    pub creation_info: SpdxCreationInfo,
    pub name: String,
    pub data_license: String,
    pub document_namespace: String,
    pub packages: Vec<SpdxPackage>,
    pub relationships: Vec<SpdxRelationship>,
}

#[derive(Serialize)]
pub struct SpdxCreationInfo {
    pub created: String,
    pub creators: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpdxPackage {
    #[serde(rename = "SPDXID")]
    pub spdx_id: String,
    pub name: String,
    pub version_info: String,
    pub download_location: String,
    pub files_analyzed: bool,
    pub license_concluded: String,
    pub license_declared: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_refs: Option<Vec<SpdxExternalRef>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpdxExternalRef {
    pub reference_category: String,
    pub reference_type: String,
    pub reference_locator: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpdxRelationship {
    pub spdx_element_id: String,
    pub relationship_type: String,
    pub related_spdx_element: String,
}

/// Build an SPDX document from a flat dependency list.
pub fn build_spdx(project_name: &str, deps: &[Dependency]) -> SpdxDocument {
    let mut packages = Vec::with_capacity(deps.len() + 1);
    let mut relationships = Vec::with_capacity(deps.len());

    // Root package representing the scanned project
    packages.push(SpdxPackage {
        spdx_id: "SPDXRef-DOCUMENT-ROOT".to_string(),
        name: project_name.to_string(),
        version_info: "NOASSERTION".to_string(),
        download_location: "NOASSERTION".to_string(),
        files_analyzed: false,
        license_concluded: "NOASSERTION".to_string(),
        license_declared: "NOASSERTION".to_string(),
        external_refs: None,
    });

    for (i, dep) in deps.iter().enumerate() {
        let spdx_id = format!("SPDXRef-Package-{}", i);
        let license = dep.license_spdx
            .as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("NOASSERTION");
        let license_expr = if license == "unknown" || license.is_empty() {
            "NOASSERTION".to_string()
        } else {
            license.to_string()
        };

        relationships.push(SpdxRelationship {
            spdx_element_id: "SPDXRef-DOCUMENT-ROOT".to_string(),
            relationship_type: "DEPENDS_ON".to_string(),
            related_spdx_element: spdx_id.clone(),
        });

        packages.push(SpdxPackage {
            spdx_id,
            name: dep.name.clone(),
            version_info: dep.version.clone(),
            download_location: "NOASSERTION".to_string(),
            files_analyzed: false,
            license_concluded: license_expr.clone(),
            license_declared: license_expr,
            external_refs: Some(vec![SpdxExternalRef {
                reference_category: "PACKAGE-MANAGER".to_string(),
                reference_type: "purl".to_string(),
                reference_locator: purl(&dep.ecosystem, &dep.name, &dep.version),
            }]),
        });
    }

    SpdxDocument {
        spdx_id: "SPDXRef-DOCUMENT".to_string(),
        spdx_version: "SPDX-2.3".to_string(),
        creation_info: SpdxCreationInfo {
            created: iso8601_now(),
            creators: vec![
                format!("Tool: license-checkr-{}", env!("CARGO_PKG_VERSION")),
            ],
        },
        name: project_name.to_string(),
        data_license: "CC0-1.0".to_string(),
        document_namespace: format!(
            "https://spdx.org/spdxdocs/{}-{}",
            project_name.replace(' ', "-").to_lowercase(),
            pseudo_uuid(),
        ),
        packages,
        relationships,
    }
}

/// Serialize an SPDX document to JSON bytes.
pub fn spdx_to_json(doc: &SpdxDocument) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(doc)?)
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Generate a Package URL (purl) for a dependency.
fn purl(ecosystem: &Ecosystem, name: &str, version: &str) -> String {
    let pkg_type = match ecosystem {
        Ecosystem::Rust   => "cargo",
        Ecosystem::Python => "pypi",
        Ecosystem::Java   => "maven",
        Ecosystem::Node   => "npm",
        Ecosystem::DotNet => "nuget",
    };
    format!("pkg:{}/{}@{}", pkg_type, name, version)
}

fn iso8601_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days  = secs / 86400;
    let year  = 1970 + days / 365;
    let doy   = days % 365;
    let month = ((doy / 30) + 1).min(12);
    let day   = ((doy % 30) + 1).min(31);
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s)
}

/// Produce a deterministic pseudo-UUID from the current timestamp.
fn pseudo_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (n >> 96) as u32,
        ((n >> 80) as u16) & 0xffff,
        ((n >> 68) as u16) & 0x0fff,
        (((n >> 52) as u16) & 0x3fff) | 0x8000,
        (n & 0xffffffffffff_u128) as u64,
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
