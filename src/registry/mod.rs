//! Async HTTP clients for fetching license data from upstream package registries.
//!
//! Each module exposes a single `fetch_license(client, name, version)` function
//! that returns `Ok(Some(license_string))` on success, `Ok(None)` when the
//! package is not found or has no license field, and `Err` on network failures.

pub mod crates_io;
pub mod maven;
pub mod npm;
pub mod pypi;

