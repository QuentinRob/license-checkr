use std::path::Path;

use anyhow::Result;
use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};

use crate::models::{Dependency, LicenseRisk, PolicyVerdict};

/// Render a colored terminal report.
pub fn render(deps: &[Dependency], path: &Path, verbose: bool, quiet: bool) -> Result<()> {
    let total = deps.len();
    let pass_count = deps.iter().filter(|d| d.verdict == PolicyVerdict::Pass).count();
    let warn_count = deps.iter().filter(|d| d.verdict == PolicyVerdict::Warn).count();
    let error_count = deps.iter().filter(|d| d.verdict == PolicyVerdict::Error).count();

    if !quiet {
        println!(
            "\n {} v{}",
            "license-checkr".bold(),
            env!("CARGO_PKG_VERSION")
        );
        println!(" Scanning: {}\n", path.display());
    }

    // Summary box
    let pass_licenses = summarize_licenses(deps, &PolicyVerdict::Pass);
    let warn_licenses = summarize_licenses(deps, &PolicyVerdict::Warn);
    let error_licenses = summarize_licenses(deps, &PolicyVerdict::Error);

    if quiet {
        println!(
            "Total: {}  Pass: {}  Warn: {}  Error: {}",
            total,
            pass_count.to_string().green(),
            warn_count.to_string().yellow(),
            error_count.to_string().red(),
        );
        return Ok(());
    }

    println!(" ┌────────────────────────────────────────────────────┐");
    println!(" │  {:<48} │", "SUMMARY".bold());
    println!(
        " │  {:<48} │",
        format!("Total dependencies : {}", total)
    );
    println!(
        " │  {:<48} │",
        format!(
            "{}  Pass            : {:>4}  {}",
            "✓".green(),
            pass_count,
            pass_licenses
        )
    );
    println!(
        " │  {:<48} │",
        format!(
            "{}  Warn            : {:>4}  {}",
            "⚠".yellow(),
            warn_count,
            warn_licenses
        )
    );
    println!(
        " │  {:<48} │",
        format!(
            "{}  Error           : {:>4}  {}",
            "✗".red(),
            error_count,
            error_licenses
        )
    );
    println!(" └────────────────────────────────────────────────────┘\n");

    // Error table
    if error_count > 0 {
        println!(" {} Dependencies requiring attention:\n", "[ERROR]".red().bold());
        render_table(deps, &PolicyVerdict::Error);
        println!();
    }

    // Warn table
    if warn_count > 0 {
        println!(" {} Dependencies with warnings:\n", "[WARN]".yellow().bold());
        render_table(deps, &PolicyVerdict::Warn);
        println!();
    }

    // Verbose: show all passing
    if verbose && pass_count > 0 {
        println!(" {} All passing dependencies:\n", "[PASS]".green().bold());
        render_table(deps, &PolicyVerdict::Pass);
        println!();
    }

    Ok(())
}

fn render_table(deps: &[Dependency], verdict_filter: &PolicyVerdict) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Name").add_attribute(Attribute::Bold),
            Cell::new("Version").add_attribute(Attribute::Bold),
            Cell::new("Ecosystem").add_attribute(Attribute::Bold),
            Cell::new("License").add_attribute(Attribute::Bold),
            Cell::new("Risk").add_attribute(Attribute::Bold),
            Cell::new("Verdict").add_attribute(Attribute::Bold),
        ]);

    for dep in deps.iter().filter(|d| &d.verdict == verdict_filter) {
        let license = dep
            .license_spdx
            .as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown");

        let (verdict_str, verdict_color) = match dep.verdict {
            PolicyVerdict::Pass => ("✓ pass", Color::Green),
            PolicyVerdict::Warn => ("⚠ warn", Color::Yellow),
            PolicyVerdict::Error => ("✗ error", Color::Red),
        };

        let risk_color = match dep.risk {
            LicenseRisk::Permissive => Color::Green,
            LicenseRisk::WeakCopyleft => Color::Yellow,
            LicenseRisk::StrongCopyleft => Color::Red,
            LicenseRisk::Proprietary => Color::Magenta,
            LicenseRisk::Unknown => Color::DarkGrey,
        };

        table.add_row(vec![
            Cell::new(&dep.name),
            Cell::new(&dep.version),
            Cell::new(dep.ecosystem.to_string()),
            Cell::new(license),
            Cell::new(dep.risk.to_string()).fg(risk_color),
            Cell::new(verdict_str)
                .fg(verdict_color)
                .set_alignment(CellAlignment::Center),
        ]);
    }

    println!("{}", table);
}

fn summarize_licenses(deps: &[Dependency], verdict: &PolicyVerdict) -> String {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for dep in deps.iter().filter(|d| &d.verdict == verdict) {
        let lic = dep
            .license_spdx
            .as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(lic).or_insert(0) += 1;
    }

    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));

    let summary: Vec<String> = pairs
        .iter()
        .take(3)
        .map(|(lic, cnt)| format!("{} ({})", lic, cnt))
        .collect();

    if summary.is_empty() {
        String::new()
    } else {
        format!("[{}]", summary.join(", "))
    }
}
