//! Report renderers for license scan results.
//!
//! - [`terminal`] — colored, tabular output with summary box; respects `--verbose` / `--quiet`.
//! - [`pdf`] — multi-page PDF with cover, bar charts (risk + ecosystem distribution),
//!   and a full dependency table.

pub mod pdf;
pub mod terminal;
