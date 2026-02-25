use crate::license::spdx::{classify_spdx_id, normalize};
use crate::models::LicenseRisk;

/// Classify a license string (raw or SPDX) into a risk level.
///
/// Handles:
/// - SPDX identifiers (MIT, Apache-2.0, etc.)
/// - SPDX OR expressions (MIT OR Apache-2.0)  → most permissive wins
/// - SPDX AND expressions (MIT AND GPL-3.0)  → most restrictive wins
/// - Proprietary/commercial strings
/// - Empty / unknown
pub fn classify(license: &str) -> LicenseRisk {
    let trimmed = license.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
        return LicenseRisk::Unknown;
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("proprietary") || lower.contains("commercial") {
        return LicenseRisk::Proprietary;
    }

    // Normalize common non-SPDX strings first
    // Also normalize slash separator to OR (e.g. "MIT/Apache-2.0" → "MIT OR Apache-2.0")
    let normalized = normalize(trimmed).replace('/', " OR ");

    // Handle SPDX OR expressions — take the most permissive component
    if normalized.contains(" OR ") {
        let risks: Vec<LicenseRisk> = normalized
            .split(" OR ")
            .map(|p| classify_single(p.trim()))
            .collect();
        return most_permissive(risks);
    }

    // Handle SPDX AND expressions — take the most restrictive component
    if normalized.contains(" AND ") {
        let risks: Vec<LicenseRisk> = normalized
            .split(" AND ")
            .map(|p| classify_single(p.trim()))
            .collect();
        return most_restrictive(risks);
    }

    classify_single(&normalized)
}

fn classify_single(id: &str) -> LicenseRisk {
    // Strip WITH exception clauses (e.g. "GPL-2.0 WITH Classpath-exception-2.0")
    let base = id.split(" WITH ").next().unwrap_or(id).trim();
    classify_spdx_id(base)
}

fn most_permissive(risks: Vec<LicenseRisk>) -> LicenseRisk {
    if risks.contains(&LicenseRisk::Permissive) {
        return LicenseRisk::Permissive;
    }
    if risks.contains(&LicenseRisk::WeakCopyleft) {
        return LicenseRisk::WeakCopyleft;
    }
    if risks.contains(&LicenseRisk::StrongCopyleft) {
        return LicenseRisk::StrongCopyleft;
    }
    if risks.contains(&LicenseRisk::Proprietary) {
        return LicenseRisk::Proprietary;
    }
    LicenseRisk::Unknown
}

fn most_restrictive(risks: Vec<LicenseRisk>) -> LicenseRisk {
    if risks.contains(&LicenseRisk::Proprietary) {
        return LicenseRisk::Proprietary;
    }
    if risks.contains(&LicenseRisk::StrongCopyleft) {
        return LicenseRisk::StrongCopyleft;
    }
    if risks.contains(&LicenseRisk::WeakCopyleft) {
        return LicenseRisk::WeakCopyleft;
    }
    if risks.contains(&LicenseRisk::Permissive) {
        return LicenseRisk::Permissive;
    }
    LicenseRisk::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_or_expression() {
        assert_eq!(classify("MIT OR GPL-3.0"), LicenseRisk::Permissive);
    }

    #[test]
    fn test_slash_separator() {
        assert_eq!(classify("MIT/Apache-2.0"), LicenseRisk::Permissive);
        assert_eq!(classify("MIT/GPL-3.0"), LicenseRisk::Permissive);
        assert_eq!(classify("GPL-3.0/LGPL-3.0"), LicenseRisk::WeakCopyleft);
    }

    #[test]
    fn test_and_expression() {
        assert_eq!(classify("MIT AND GPL-3.0"), LicenseRisk::StrongCopyleft);
    }

    #[test]
    fn test_proprietary() {
        assert_eq!(classify("Proprietary"), LicenseRisk::Proprietary);
        assert_eq!(classify("commercial license"), LicenseRisk::Proprietary);
    }

    #[test]
    fn test_unknown() {
        assert_eq!(classify(""), LicenseRisk::Unknown);
        assert_eq!(classify("unknown"), LicenseRisk::Unknown);
        assert_eq!(classify("CUSTOM-LICENSE-42"), LicenseRisk::Unknown);
    }

    #[test]
    fn test_with_exception() {
        assert_eq!(
            classify("GPL-2.0 WITH Classpath-exception-2.0"),
            LicenseRisk::StrongCopyleft
        );
    }
}
