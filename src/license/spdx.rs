use crate::models::LicenseRisk;

/// Classify a single canonical SPDX identifier into a risk level.
pub fn classify_spdx_id(id: &str) -> LicenseRisk {
    match id.trim() {
        // Permissive
        "MIT"
        | "Apache-2.0"
        | "BSD-2-Clause"
        | "BSD-3-Clause"
        | "BSD-4-Clause"
        | "ISC"
        | "0BSD"
        | "Unlicense"
        | "Zlib"
        | "CC0-1.0"
        | "WTFPL"
        | "CC-BY-4.0"
        | "CC-BY-3.0"
        | "PSF-2.0"
        | "Python-2.0"
        | "MIT-0"
        | "BlueOak-1.0.0"
        | "Artistic-2.0" => LicenseRisk::Permissive,

        // Weak copyleft
        "LGPL-2.0"
        | "LGPL-2.0-only"
        | "LGPL-2.0-or-later"
        | "LGPL-2.1"
        | "LGPL-2.1-only"
        | "LGPL-2.1-or-later"
        | "LGPL-3.0"
        | "LGPL-3.0-only"
        | "LGPL-3.0-or-later"
        | "MPL-2.0"
        | "EUPL-1.2"
        | "CDDL-1.0"
        | "EPL-1.0"
        | "EPL-2.0"
        | "APSL-2.0"
        | "OSL-3.0" => LicenseRisk::WeakCopyleft,

        // Strong copyleft
        "GPL-2.0"
        | "GPL-2.0-only"
        | "GPL-2.0-or-later"
        | "GPL-3.0"
        | "GPL-3.0-only"
        | "GPL-3.0-or-later"
        | "AGPL-3.0"
        | "AGPL-3.0-only"
        | "AGPL-3.0-or-later"
        | "EUPL-1.1" => LicenseRisk::StrongCopyleft,

        _ => LicenseRisk::Unknown,
    }
}

/// Normalize common non-SPDX strings to their SPDX equivalents.
pub fn normalize(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed {
        "Apache 2.0" | "Apache License 2.0" | "Apache License, Version 2.0" => {
            "Apache-2.0".to_string()
        }
        "MIT License" | "The MIT License" => "MIT".to_string(),
        "BSD" | "BSD License" => "BSD-3-Clause".to_string(),
        "BSD 2-Clause" | "Simplified BSD" => "BSD-2-Clause".to_string(),
        "BSD 3-Clause" | "New BSD" | "Modified BSD" => "BSD-3-Clause".to_string(),
        "GNU GPL v2" | "GNU General Public License v2" | "GPL v2" | "GPLv2" => {
            "GPL-2.0".to_string()
        }
        "GNU GPL v3" | "GNU General Public License v3" | "GPL v3" | "GPLv3" => {
            "GPL-3.0".to_string()
        }
        "GNU LGPL v2.1" | "LGPL v2.1" | "LGPLv2.1" => "LGPL-2.1".to_string(),
        "GNU LGPL v3" | "LGPL v3" | "LGPLv3" => "LGPL-3.0".to_string(),
        "Mozilla Public License 2.0" | "MPL 2.0" | "MPLv2" => "MPL-2.0".to_string(),
        "ISC License" => "ISC".to_string(),
        "CC0" | "Public Domain" => "CC0-1.0".to_string(),
        "AGPL v3" | "AGPLv3" | "GNU AGPL v3" => "AGPL-3.0".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_permissive() {
        assert_eq!(classify_spdx_id("MIT"), LicenseRisk::Permissive);
        assert_eq!(classify_spdx_id("Apache-2.0"), LicenseRisk::Permissive);
        assert_eq!(classify_spdx_id("BSD-3-Clause"), LicenseRisk::Permissive);
    }

    #[test]
    fn test_classify_strong_copyleft() {
        assert_eq!(classify_spdx_id("GPL-3.0"), LicenseRisk::StrongCopyleft);
        assert_eq!(classify_spdx_id("AGPL-3.0"), LicenseRisk::StrongCopyleft);
    }

    #[test]
    fn test_classify_weak_copyleft() {
        assert_eq!(classify_spdx_id("LGPL-2.1"), LicenseRisk::WeakCopyleft);
        assert_eq!(classify_spdx_id("MPL-2.0"), LicenseRisk::WeakCopyleft);
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("MIT License"), "MIT");
        assert_eq!(normalize("Apache License 2.0"), "Apache-2.0");
    }
}
