use std::path::Path;

use anyhow::Result;

use crate::models::Dependency;

pub mod java;
pub mod node;
pub mod python;
pub mod rust;

pub trait Analyzer {
    fn analyze(&self, path: &Path) -> Result<Vec<Dependency>>;
}
