//! License string normalization and SPDX-based risk classification.
//!
//! - [`spdx`] — maps canonical SPDX identifiers to [`LicenseRisk`](crate::models::LicenseRisk)
//!   and normalizes common non-SPDX strings.
//! - [`classifier`] — entry point that handles raw license strings including
//!   SPDX OR/AND expressions and proprietary keywords.

pub mod classifier;
pub mod spdx;
