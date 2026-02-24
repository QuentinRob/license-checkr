mod analyzer;
mod cli;
mod config;
mod detector;
mod license;
mod models;
mod registry;
mod report;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use analyzer::Analyzer;
use cli::{Cli, ReportFormat};
use config::{apply_policy, load_config};
use detector::detect_ecosystems;
use license::classifier::classify;
use models::{Ecosystem, LicenseSource, PolicyVerdict};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve project path
    let path = cli
        .path
        .canonicalize()
        .unwrap_or_else(|_| cli.path.clone());

    // Load policy config
    let config = load_config(&path, cli.config.as_deref())?;

    // Detect ecosystems (always automatic; --exclude-lang opts out)
    let excluded: Vec<Ecosystem> = cli.exclude_lang.iter().map(Into::into).collect();

    let ecosystems: Vec<Ecosystem> = detect_ecosystems(&path)
        .into_iter()
        .filter(|e| !excluded.contains(e))
        .collect();

    if ecosystems.is_empty() {
        eprintln!(
            "No supported project manifests found in {}",
            path.display()
        );
        std::process::exit(1);
    }

    // Analyze each detected ecosystem
    let mut all_deps = Vec::new();

    for ecosystem in &ecosystems {
        let deps = match ecosystem {
            Ecosystem::Rust => analyzer::rust::RustAnalyzer::new().analyze(&path)?,
            Ecosystem::Python => analyzer::python::PythonAnalyzer::new().analyze(&path)?,
            Ecosystem::Java => analyzer::java::JavaAnalyzer::new().analyze(&path)?,
            Ecosystem::Node => analyzer::node::NodeAnalyzer::new().analyze(&path)?,
        };

        if !cli.quiet {
            eprintln!(
                "  {} {} {} dependencies",
                "→".cyan(),
                ecosystem,
                deps.len()
            );
        }

        all_deps.extend(deps);
    }

    // Online enrichment: fetch license data from package registries
    if cli.online {
        enrich_online(&mut all_deps, cli.quiet).await?;
    }

    // Classify licenses and apply policy verdicts
    for dep in &mut all_deps {
        let license = dep
            .license_spdx
            .as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown");

        dep.risk = classify(license);
        dep.verdict = apply_policy(&config, Some(license));
    }

    // Render report
    match cli.report {
        ReportFormat::Terminal => {
            report::terminal::render(&all_deps, &path, cli.verbose, cli.quiet)?;
        }
        ReportFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&all_deps)?);
        }
        ReportFormat::Pdf => {
            report::pdf::render(&all_deps, &path, &cli.pdf)?;
        }
    }

    // Exit code: 1 if any error verdict found
    let has_errors = all_deps
        .iter()
        .any(|d| d.verdict == PolicyVerdict::Error);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

async fn enrich_online(deps: &mut Vec<models::Dependency>, quiet: bool) -> Result<()> {
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

    for dep in deps.iter_mut() {
        if let Some(pb) = &pb {
            pb.set_message(dep.name.clone());
        }

        let result = match dep.ecosystem {
            Ecosystem::Rust => {
                registry::crates_io::fetch_license(&client, &dep.name, &dep.version).await
            }
            Ecosystem::Python => {
                registry::pypi::fetch_license(&client, &dep.name, &dep.version).await
            }
            Ecosystem::Java => {
                registry::maven::fetch_license(&client, &dep.name, &dep.version).await
            }
            Ecosystem::Node => {
                registry::npm::fetch_license(&client, &dep.name, &dep.version).await
            }
        };

        if let Ok(Some(license)) = result {
            dep.license_raw = Some(license.clone());
            dep.license_spdx = Some(license);
            dep.source = LicenseSource::Registry;
        }

        if let Some(pb) = &pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Done");
    }

    Ok(())
}
