//! `license-checkr` — scan dependency manifests, classify licenses, and enforce policy.
//!
//! # Flow
//! 1. Parse CLI arguments ([`cli`]).
//! 2. Load policy config ([`config::load_config`]).
//! 3. Auto-detect ecosystems ([`detector::detect_ecosystems`]).
//! 4. Analyze each ecosystem's manifests ([`analyzer`]).
//! 5. Optionally enrich from package registries (`--online`, [`registry`]).
//! 6. Classify licenses and apply policy ([`license`], [`config::apply_policy`]).
//! 7. Render the requested report ([`report`]).
//! 8. Exit `0` (clean) or `1` (at least one [`models::PolicyVerdict::Error`]).

mod analyzer;
mod cli;
mod config;
mod detector;
mod license;
mod models;
mod registry;
mod report;

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use analyzer::Analyzer;
use cli::{Cli, ReportFormat};
use config::{apply_policy, load_config};
use detector::detect_ecosystems;
use license::classifier::classify;
use models::{Ecosystem, LicenseSource, PolicyVerdict, ProjectScan};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let path = cli
        .path
        .canonicalize()
        .unwrap_or_else(|_| cli.path.clone());

    let excluded: Vec<Ecosystem> = cli.exclude_lang.iter().map(Into::into).collect();

    let report_format = match &cli.pdf {
        Some(_) => ReportFormat::Pdf,
        None => cli.report.clone(),
    };
    let pdf_path = cli
        .pdf
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("license-report.pdf"));

    let has_errors = if cli.recursive {
        run_workspace(&cli, &path, &excluded, &report_format, &pdf_path).await?
    } else {
        run_single(&cli, &path, &excluded, &report_format, &pdf_path).await?
    };

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

// ── Single-project mode ───────────────────────────────────────────────────────

async fn run_single(
    cli: &Cli,
    path: &Path,
    excluded: &[Ecosystem],
    report_format: &ReportFormat,
    pdf_path: &Path,
) -> Result<bool> {
    let config = load_config(path, cli.config.as_deref())?;

    let ecosystems: Vec<Ecosystem> = detect_ecosystems(path)
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

    let mut all_deps = scan_project(path, &config, excluded, cli.online, cli.quiet).await?;

    // Classify + apply policy
    for dep in &mut all_deps {
        let license = dep
            .license_spdx
            .as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown");
        dep.risk = classify(license);
        dep.verdict = apply_policy(&config, Some(license));
    }

    match report_format {
        ReportFormat::Terminal => {
            report::terminal::render(&all_deps, path, cli.verbose, cli.quiet)?;
        }
        ReportFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&all_deps)?);
        }
        ReportFormat::Pdf => {
            report::pdf::render(&all_deps, path, pdf_path)?;
        }
    }

    Ok(all_deps.iter().any(|d| d.verdict == PolicyVerdict::Error))
}

// ── Workspace mode ────────────────────────────────────────────────────────────

async fn run_workspace(
    cli: &Cli,
    root: &Path,
    excluded: &[Ecosystem],
    report_format: &ReportFormat,
    pdf_path: &Path,
) -> Result<bool> {
    let project_paths = detector::find_workspace_projects(root);

    if project_paths.is_empty() {
        eprintln!("No sub-projects found under {}", root.display());
        std::process::exit(1);
    }

    if !cli.quiet {
        println!(
            "\n {} v{}  —  workspace mode",
            "license-checkr".bold(),
            env!("CARGO_PKG_VERSION")
        );
        println!(
            " Root:  {}\n Found: {} sub-project{}\n",
            root.display(),
            project_paths.len(),
            if project_paths.len() == 1 { "" } else { "s" }
        );
    }

    let tasks: Vec<_> = project_paths
        .into_iter()
        .map(|proj_path| {
            let excluded = excluded.to_vec();
            let online = cli.online;
            let config_override = cli.config.clone();

            tokio::spawn(async move {
                let name = proj_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let proj_config = load_config(&proj_path, config_override.as_deref())?;
                // Always suppress inline prints — output is flushed in order after join_all.
                let mut deps =
                    scan_project(&proj_path, &proj_config, &excluded, online, true).await?;

                for dep in &mut deps {
                    let license = dep
                        .license_spdx
                        .as_deref()
                        .or(dep.license_raw.as_deref())
                        .unwrap_or("unknown");
                    dep.risk = classify(license);
                    dep.verdict = apply_policy(&proj_config, Some(license));
                }

                Ok::<ProjectScan, anyhow::Error>(ProjectScan {
                    name,
                    path: proj_path,
                    deps,
                })
            })
        })
        .collect();

    let mut projects: Vec<ProjectScan> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|join_result| join_result.expect("project scan task panicked"))
        .collect::<Result<Vec<_>>>()?;

    // Drop projects with zero dependencies (empty / unsupported ecosystems)
    projects.retain(|p| !p.deps.is_empty());

    if projects.is_empty() {
        eprintln!("No dependencies found in any sub-project.");
        return Ok(false);
    }

    // Print scan summaries in deterministic order now that all tasks have finished.
    if !cli.quiet {
        for project in &projects {
            println!(
                " {} scanning {}  ({})",
                "→".cyan(),
                project.name.bold(),
                project.path.display()
            );
            // Group dep counts by ecosystem.
            let mut eco_counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for dep in &project.deps {
                *eco_counts.entry(dep.ecosystem.to_string()).or_insert(0) += 1;
            }
            for (eco, count) in &eco_counts {
                eprintln!("    {} {} {} dependencies", "·".dimmed(), eco, count);
            }
        }
        println!();
    }

    match report_format {
        ReportFormat::Terminal => {
            report::terminal::render_workspace(&projects, cli.verbose, cli.quiet)?;
        }
        ReportFormat::Json => {
            #[derive(Serialize)]
            struct ProjectScanJson<'a> {
                project: &'a str,
                path: String,
                dependencies: &'a [models::Dependency],
            }
            let out: Vec<ProjectScanJson<'_>> = projects
                .iter()
                .map(|p| ProjectScanJson {
                    project: &p.name,
                    path: p.path.display().to_string(),
                    dependencies: &p.deps,
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        ReportFormat::Pdf => {
            report::pdf::render_workspace(&projects, pdf_path)?;
        }
    }

    let has_errors = projects
        .iter()
        .flat_map(|p| &p.deps)
        .any(|d| d.verdict == PolicyVerdict::Error);

    Ok(has_errors)
}

// ── Shared scan logic ─────────────────────────────────────────────────────────

/// Detect ecosystems, analyze manifests, and optionally enrich online.
/// Returns an empty `Vec` (not an error) when no ecosystems are detected.
async fn scan_project(
    path: &Path,
    _config: &config::Config,
    excluded: &[Ecosystem],
    online: bool,
    quiet: bool,
) -> Result<Vec<models::Dependency>> {
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
            Ecosystem::Rust => analyzer::rust::RustAnalyzer::new().analyze(path)?,
            Ecosystem::Python => analyzer::python::PythonAnalyzer::new().analyze(path)?,
            Ecosystem::Java => analyzer::java::JavaAnalyzer::new().analyze(path)?,
            Ecosystem::Node => analyzer::node::NodeAnalyzer::new().analyze(path)?,
            Ecosystem::DotNet => analyzer::dotnet::DotNetAnalyzer::new().analyze(path)?,
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

// ── Online enrichment ─────────────────────────────────────────────────────────

async fn enrich_online(deps: &mut [models::Dependency], quiet: bool) -> Result<()> {
    use futures::future::join_all;

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
