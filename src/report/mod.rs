//! Report renderers for license scan results.
//!
//! - [`terminal`] — colored, tabular output with summary box; respects `--verbose` / `--quiet`.
//! - [`pdf`] — multi-page PDF with cover, risk summary, and full dependency table.
//! - [`sbom_pdf`] — SBOM-specific PDF with ecosystem summary and component catalog.

pub mod pdf;
pub mod sbom_pdf;
pub mod terminal;
