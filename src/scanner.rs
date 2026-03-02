use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};

use crate::analyzer::Analyzer;
use crate::config::Config;
use crate::models::{Dependency, Ecosystem, LicenseSource};
use crate::registry;

/// Detect ecosystems, analyze manifests, and optionally enrich online.
/// Returns an empty `Vec` (not an error) when no ecosystems are detected.
pub async fn scan_project(
    path: &Path,
    _config: &Config,
    excluded: &[Ecosystem],
    online: bool,
    quiet: bool,
) -> Result<Vec<Dependency>> {
    use crate::detector::detect_ecosystems;

    let ecosystems: Vec<Ecosystem> = detect_ecosystems(path)
        .into_iter()
        .filter(|e| !excluded.contains(e))
        .collect();

    if ecosystems.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_deps = Vec::new();

    for ecosystem in &ecosystems {
        let deps = match ecosystem {
            Ecosystem::Rust => crate::analyzer::rust::RustAnalyzer::new().analyze(path)?,
            Ecosystem::Python => crate::analyzer::python::PythonAnalyzer::new().analyze(path)?,
            Ecosystem::Java => crate::analyzer::java::JavaAnalyzer::new().analyze(path)?,
            Ecosystem::Node => crate::analyzer::node::NodeAnalyzer::new().analyze(path)?,
            Ecosystem::DotNet => crate::analyzer::dotnet::DotNetAnalyzer::new().analyze(path)?,
        };

        if !quiet {
            eprintln!(
                "    {} {} {} dependencies",
                "·".dimmed(),
                ecosystem,
                deps.len()
            );
        }

        all_deps.extend(deps);
    }

    if online {
        enrich_online(&mut all_deps, quiet).await?;
    }

    Ok(all_deps)
}

/// Fetch license information from upstream registries for each dependency.
pub async fn enrich_online(deps: &mut [Dependency], quiet: bool) -> Result<()> {
    const BATCH_SIZE: usize = 50;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let pb = if !quiet {
        let pb = ProgressBar::new(deps.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )?
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    for batch in deps.chunks_mut(BATCH_SIZE) {
        let handles: Vec<_> = batch
            .iter()
            .map(|dep| {
                let client = client.clone();
                let name = dep.name.clone();
                let version = dep.version.clone();
                let ecosystem = dep.ecosystem.clone();
                tokio::spawn(async move {
                    match ecosystem {
                        Ecosystem::Rust => {
                            registry::crates_io::fetch_license(&client, &name, &version).await
                        }
                        Ecosystem::Python => {
                            registry::pypi::fetch_license(&client, &name, &version).await
                        }
                        Ecosystem::Java => {
                            registry::maven::fetch_license(&client, &name, &version).await
                        }
                        Ecosystem::Node => {
                            registry::npm::fetch_license(&client, &name, &version).await
                        }
                        Ecosystem::DotNet => Ok(None),
                    }
                })
            })
            .collect();

        let results = join_all(handles).await;

        for (dep, join_result) in batch.iter_mut().zip(results) {
            if let Ok(Ok(Some(license))) = join_result {
                dep.license_raw = Some(license.clone());
                dep.license_spdx = Some(license);
                dep.source = LicenseSource::Registry;
            }
            if let Some(pb) = &pb {
                pb.inc(1);
            }
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Done");
    }

    Ok(())
}
