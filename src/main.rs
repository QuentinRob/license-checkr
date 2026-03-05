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
mod mcp;
mod models;
mod registry;
mod report;
mod sbom;
mod scanner;
mod updater;

use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use serde::Serialize;

use cli::{Cli, Commands, McpAction, ReportFormat, SbomAction, SbomFormat};
use config::{apply_policy, load_config};
use detector::detect_ecosystems;
use license::classifier::classify;
use models::{Dependency, Ecosystem, PolicyVerdict, ProjectScan};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Dispatch MCP subcommand before anything else — its stdio transport must
    // not be polluted by update notices or other stderr output.
    if let Some(Commands::Mcp { action: McpAction::Serve }) = &cli.command {
        mcp::serve().await?;
        return Ok(());
    }

    // Dispatch SBOM subcommand.
    if let Some(Commands::Sbom { action }) = &cli.command {
        updater::check_for_update().await;
        return run_sbom(action).await;
    }

    // Check for a newer release on GitHub (skipped in quiet/scripting mode).
    if !cli.quiet {
        updater::check_for_update().await;
    }

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

    let mut all_deps = scanner::scan_project(path, &config, excluded, cli.online, cli.quiet).await?;

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
                    scanner::scan_project(&proj_path, &proj_config, &excluded, online, true).await?;

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

// ── SBOM subcommand ───────────────────────────────────────────────────────────

async fn run_sbom(action: &SbomAction) -> Result<()> {
    let SbomAction::Generate {
        path,
        format,
        output,
        pdf,
        online,
        recursive,
        config: config_override,
        exclude_lang,
    } = action;

    let path = path.canonicalize().unwrap_or_else(|_| path.clone());
    let excluded: Vec<Ecosystem> = exclude_lang.iter().map(Into::into).collect();

    let format_str = match format {
        SbomFormat::CycloneDxJson => "cyclonedx-json",
        SbomFormat::CycloneDxXml  => "cyclonedx-xml",
        SbomFormat::SpdxJson      => "spdx-json",
    };

    let default_ext = match format {
        SbomFormat::CycloneDxXml => "sbom.xml",
        _                        => "sbom.json",
    };
    let output_path = output.clone().unwrap_or_else(|| std::path::PathBuf::from(default_ext));
    let pdf_path = pdf.clone().unwrap_or_else(|| std::path::PathBuf::from("sbom-report.pdf"));

    if *recursive {
        run_sbom_workspace(&path, &excluded, *online, config_override.as_deref(), format, format_str, &output_path, pdf).await
    } else {
        run_sbom_single(&path, &excluded, *online, config_override.as_deref(), format, format_str, &output_path, pdf, &pdf_path).await
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_sbom_single(
    path: &std::path::Path,
    excluded: &[Ecosystem],
    online: bool,
    config_override: Option<&std::path::Path>,
    format: &SbomFormat,
    format_str: &str,
    output_path: &std::path::Path,
    pdf_flag: &Option<std::path::PathBuf>,
    pdf_path: &std::path::Path,
) -> Result<()> {
    let config = load_config(path, config_override)?;

    let ecosystems: Vec<Ecosystem> = detect_ecosystems(path)
        .into_iter()
        .filter(|e| !excluded.contains(e))
        .collect();

    if ecosystems.is_empty() {
        eprintln!("No supported project manifests found in {}", path.display());
        std::process::exit(1);
    }

    let mut deps = scanner::scan_project(path, &config, excluded, online, false).await?;
    for dep in &mut deps {
        let license = dep.license_spdx.as_deref().or(dep.license_raw.as_deref()).unwrap_or("unknown");
        dep.risk = classify(license);
        dep.verdict = apply_policy(&config, Some(license));
    }

    let project_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("project");

    let sbom_bytes = match format {
        SbomFormat::CycloneDxJson => {
            let bom = sbom::build_cyclonedx(project_name, &deps);
            sbom::cyclonedx_to_json(&bom)?
        }
        SbomFormat::CycloneDxXml => {
            let bom = sbom::build_cyclonedx(project_name, &deps);
            sbom::cyclonedx_to_xml(&bom)?
        }
        SbomFormat::SpdxJson => {
            let doc = sbom::build_spdx(project_name, &deps);
            sbom::spdx_to_json(&doc)?
        }
    };

    std::fs::write(output_path, &sbom_bytes)
        .with_context(|| format!("Failed to write SBOM to {}", output_path.display()))?;
    println!("SBOM ({}) written to: {}", format_str, output_path.display());

    if pdf_flag.is_some() {
        report::sbom_pdf::render(&deps, path, format_str, pdf_path)?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_sbom_workspace(
    root: &std::path::Path,
    excluded: &[Ecosystem],
    online: bool,
    config_override: Option<&std::path::Path>,
    format: &SbomFormat,
    format_str: &str,
    output_path: &std::path::Path,
    pdf_flag: &Option<std::path::PathBuf>,
) -> Result<()> {
    let project_paths = detector::find_workspace_projects(root);
    if project_paths.is_empty() {
        eprintln!("No sub-projects found under {}", root.display());
        std::process::exit(1);
    }

    let tasks: Vec<_> = project_paths
        .into_iter()
        .map(|proj_path| {
            let excluded = excluded.to_vec();
            let config_override = config_override.map(|p| p.to_path_buf());

            tokio::spawn(async move {
                let name = proj_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
                let proj_config = load_config(&proj_path, config_override.as_deref())?;
                let mut deps = scanner::scan_project(&proj_path, &proj_config, &excluded, online, true).await?;
                for dep in &mut deps {
                    let license = dep.license_spdx.as_deref().or(dep.license_raw.as_deref()).unwrap_or("unknown");
                    dep.risk = classify(license);
                    dep.verdict = apply_policy(&proj_config, Some(license));
                }
                Ok::<ProjectScan, anyhow::Error>(ProjectScan { name, path: proj_path, deps })
            })
        })
        .collect();

    let mut projects: Vec<ProjectScan> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.expect("project scan task panicked"))
        .collect::<Result<Vec<_>>>()?;
    projects.retain(|p| !p.deps.is_empty());

    if projects.is_empty() {
        eprintln!("No dependencies found in any sub-project.");
        return Ok(());
    }

    let all_deps: Vec<Dependency> = projects.iter().flat_map(|p| p.deps.iter().cloned()).collect();
    let workspace_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("workspace");

    let sbom_bytes = match format {
        SbomFormat::CycloneDxJson => {
            let bom = sbom::build_cyclonedx(workspace_name, &all_deps);
            sbom::cyclonedx_to_json(&bom)?
        }
        SbomFormat::CycloneDxXml => {
            let bom = sbom::build_cyclonedx(workspace_name, &all_deps);
            sbom::cyclonedx_to_xml(&bom)?
        }
        SbomFormat::SpdxJson => {
            let doc = sbom::build_spdx(workspace_name, &all_deps);
            sbom::spdx_to_json(&doc)?
        }
    };

    std::fs::write(output_path, &sbom_bytes)
        .with_context(|| format!("Failed to write SBOM to {}", output_path.display()))?;
    println!("SBOM ({}) written to: {}", format_str, output_path.display());

    if let Some(ref pdf_path) = pdf_flag {
        report::sbom_pdf::render_workspace(&projects, format_str, pdf_path)?;
    }

    Ok(())
}
