//! Report renderers for license scan results.
//!
//! - [`terminal`] — colored, tabular output with summary box; respects `--verbose` / `--quiet`.
//! - [`pdf`] — multi-page PDF with cover, risk summary, and full dependency table.
//! - [`sbom_pdf`] — SBOM-specific PDF with ecosystem summary and component catalog.
//! - [`gitlab`] — GitLab Code Quality JSON array for CI artifact integration.

pub mod gitlab;
pub mod pdf;
pub mod sbom_pdf;
pub mod terminal;
