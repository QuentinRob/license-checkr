use std::path::Path;

use anyhow::{Context, Result};
use printpdf::{
    BuiltinFont, Color, IndirectFontRef, Line, Mm, PdfDocument, PdfDocumentReference,
    PdfLayerIndex, PdfLayerReference, PdfPageIndex, Point, Polygon, Rgb,
};
use printpdf::path::{PaintMode, WindingOrder};

use crate::models::{Dependency, LicenseRisk, PolicyVerdict, ProjectScan};

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 18.0;
const COVER_HDR_H: f32 = 72.0; // gradient header height on cover page

// ── Light Liquid Glass colour palette ─────────────────────────────────────────
const BG:           (f32, f32, f32) = (1.00, 1.00, 1.00); // pure white page
const PANEL:        (f32, f32, f32) = (1.00, 1.00, 1.00); // pure white
const PANEL_ALT:    (f32, f32, f32) = (0.95, 0.96, 0.99); // subtle alternating tint
const PANEL_BORDER: (f32, f32, f32) = (0.85, 0.87, 0.92); // subtle border
const ACCENT_BLU:   (f32, f32, f32) = (0.20, 0.46, 0.95); // vivid blue
const ACCENT_PUR:   (f32, f32, f32) = (0.52, 0.30, 0.95); // vivid purple
const TEXT_PRI:     (f32, f32, f32) = (0.07, 0.08, 0.14); // near-black
const TEXT_SEC:     (f32, f32, f32) = (0.36, 0.40, 0.52); // medium grey-blue
const TEXT_MUT:     (f32, f32, f32) = (0.58, 0.63, 0.72); // muted grey
const WHITE:        (f32, f32, f32) = (1.00, 1.00, 1.00);
const WHITE_DIM:    (f32, f32, f32) = (0.82, 0.89, 1.00); // dimmed white for header

const PASS_BG: (f32, f32, f32) = (0.90, 0.98, 0.92);
const PASS_FG: (f32, f32, f32) = (0.07, 0.52, 0.22);
const WARN_BG: (f32, f32, f32) = (1.00, 0.95, 0.87);
const WARN_FG: (f32, f32, f32) = (0.70, 0.40, 0.02);
const ERR_BG:  (f32, f32, f32) = (1.00, 0.91, 0.91);
const ERR_FG:  (f32, f32, f32) = (0.76, 0.09, 0.13);
const PROP_BG: (f32, f32, f32) = (0.91, 0.93, 1.00);
const PROP_FG: (f32, f32, f32) = (0.20, 0.34, 0.82);

// Corner radius constants
const R_PANEL: f32 = 2.5;
const R_BADGE: f32 = 1.5;

// ── Risk summary table layout ─────────────────────────────────────────────────
const C1_X: f32 = MARGIN;
const C2_X: f32 = MARGIN + 44.0;
const C3_X: f32 = MARGIN + 118.0;
const T_END: f32 = PAGE_W - MARGIN;

const HDR_H: f32 = 9.0;
const LINE_H: f32 = 4.8;
const ROW_PAD: f32 = 4.5;

const BADGE_W: f32 = 37.0;
const BADGE_H: f32 = 6.5;
const DOT_SIZE: f32 = 2.5;

const DESC_WRAP: usize = 36;
const DEPS_WRAP: usize = 28;
const DEPS_MAX_LINES: usize = 4;

// ── Public entry point ────────────────────────────────────────────────────────

/// Render a PDF report: cover page → risk summary table → full dependency table.
pub fn render(deps: &[Dependency], project_path: &Path, output_path: &Path) -> Result<()> {
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Project");

    let doc = PdfDocument::empty("License Report");

    add_cover_page(&doc, deps, project_name)?;
    add_risk_summary_page(&doc, deps, None)?;
    add_table_pages(&doc, deps, None)?;

    let bytes = doc.save_to_bytes()?;
    std::fs::write(output_path, &bytes)
        .with_context(|| format!("Failed to write PDF to {}", output_path.display()))?;

    println!("PDF report written to: {}", output_path.display());
    Ok(())
}

/// Render a workspace PDF: workspace cover → per-project Risk Summary + Dependency Table.
pub fn render_workspace(projects: &[ProjectScan], output_path: &Path) -> Result<()> {
    let doc = PdfDocument::empty("License Report — Workspace");

    add_workspace_cover_page(&doc, projects)?;

    for proj in projects {
        add_risk_summary_page(&doc, &proj.deps, Some(&proj.name))?;
        add_table_pages(&doc, &proj.deps, Some(&proj.name))?;
    }

    let bytes = doc.save_to_bytes()?;
    std::fs::write(output_path, &bytes)
        .with_context(|| format!("Failed to write PDF to {}", output_path.display()))?;

    println!("PDF workspace report written to: {}", output_path.display());
    Ok(())
}

// ── Workspace cover page ──────────────────────────────────────────────────────

fn add_workspace_cover_page(doc: &PdfDocumentReference, projects: &[ProjectScan]) -> Result<()> {
    let (page_idx, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Cover");
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    let all_deps: Vec<&Dependency> = projects.iter().flat_map(|p| &p.deps).collect();
    let pass  = all_deps.iter().filter(|d| d.verdict == PolicyVerdict::Pass).count();
    let warn  = all_deps.iter().filter(|d| d.verdict == PolicyVerdict::Warn).count();
    let error = all_deps.iter().filter(|d| d.verdict == PolicyVerdict::Error).count();

    // Background + gradient header
    fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);
    let hdr_bot = PAGE_H - COVER_HDR_H;
    fill_gradient_h(&layer, 0.0, hdr_bot, PAGE_W, COVER_HDR_H, ACCENT_BLU, ACCENT_PUR, 28);

    set_color(&layer, WHITE_DIM);
    layer.use_text(
        format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(PAGE_W - MARGIN - 44.0), Mm(PAGE_H - 10.5), &font_r,
    );

    set_color(&layer, WHITE);
    layer.use_text("License Compliance", 28.0, Mm(MARGIN), Mm(PAGE_H - 26.0), &font_b);
    set_color(&layer, WHITE_DIM);
    layer.use_text("Workspace Report", 28.0, Mm(MARGIN), Mm(PAGE_H - 41.0), &font_b);

    // Workspace chip
    let chip_y = hdr_bot - 18.0;
    let chip_h = 12.0f32;
    let chip_w = 106.0f32;
    fill_rounded_rect(&layer, MARGIN, chip_y, chip_w, chip_h, R_BADGE, PANEL);
    stroke_rounded_rect(&layer, MARGIN, chip_y, chip_w, chip_h, R_BADGE, PANEL_BORDER);
    fill_rect(&layer, MARGIN, chip_y, 2.5, chip_h, ACCENT_PUR);

    set_color(&layer, TEXT_MUT);
    layer.use_text("WORKSPACE", 6.0, Mm(MARGIN + 5.0), Mm(chip_y + chip_h - 3.8), &font_b);
    set_color(&layer, TEXT_PRI);
    layer.use_text(
        format!("{} sub-project{}", projects.len(), if projects.len() == 1 { "" } else { "s" }),
        9.5, Mm(MARGIN + 5.0), Mm(chip_y + 2.8), &font_b,
    );

    // Scan date
    set_color(&layer, TEXT_SEC);
    layer.use_text(
        format!("Scanned  {}", chrono_now()),
        9.0, Mm(MARGIN), Mm(chip_y - 8.0), &font_r,
    );

    // Divider + OVERVIEW
    let rule_y = chip_y - 16.5;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, rule_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("OVERVIEW", 6.5, Mm(MARGIN), Mm(rule_y - 7.0), &font_b);

    // Stat cards
    let card_y  = rule_y - 42.0;
    let card_h  = 26.0f32;
    let gap     = 4.0f32;
    let total_w = T_END - MARGIN;
    let card_w  = (total_w - gap * 3.0) / 4.0;

    let cards: [(&str, String, (f32, f32, f32)); 4] = [
        ("TOTAL",  all_deps.len().to_string(), ACCENT_BLU),
        ("PASS",   pass.to_string(),           PASS_FG),
        ("WARN",   warn.to_string(),           WARN_FG),
        ("ERROR",  error.to_string(),          ERR_FG),
    ];

    for (i, (label, value, accent)) in cards.iter().enumerate() {
        let cx = MARGIN + (card_w + gap) * i as f32;
        draw_stat_card(&layer, cx, card_y, card_w, card_h, label, value, *accent,
                       &font_r, &font_b);
    }

    // Projects scanned table
    let section_y = card_y - 13.0;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, section_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("PROJECTS SCANNED", 6.5, Mm(MARGIN), Mm(section_y - 7.5), &font_b);

    // Table header
    let tbl_hdr_y = section_y - 14.0;
    let col_proj = MARGIN + 2.0;
    let col_tot  = MARGIN + 88.0;
    let col_pass = MARGIN + 106.0;
    let col_warn = MARGIN + 124.0;
    let col_err  = MARGIN + 143.0;

    set_color(&layer, TEXT_MUT);
    layer.use_text("PROJECT", 6.5, Mm(col_proj), Mm(tbl_hdr_y), &font_b);
    layer.use_text("TOTAL",   6.5, Mm(col_tot),  Mm(tbl_hdr_y), &font_b);
    layer.use_text("PASS",    6.5, Mm(col_pass), Mm(tbl_hdr_y), &font_b);
    layer.use_text("WARN",    6.5, Mm(col_warn), Mm(tbl_hdr_y), &font_b);
    layer.use_text("ERROR",   6.5, Mm(col_err),  Mm(tbl_hdr_y), &font_b);
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, tbl_hdr_y - 2.0, PANEL_BORDER);

    const MAX_ROWS: usize = 12;
    let show = projects.len().min(MAX_ROWS);

    for (i, proj) in projects.iter().take(show).enumerate() {
        let row_y = tbl_hdr_y - 7.5 - i as f32 * 6.5;
        let p_total = proj.deps.len();
        let p_pass = proj.deps.iter().filter(|d| d.verdict == PolicyVerdict::Pass).count();
        let p_warn = proj.deps.iter().filter(|d| d.verdict == PolicyVerdict::Warn).count();
        let p_err  = proj.deps.iter().filter(|d| d.verdict == PolicyVerdict::Error).count();

        if i % 2 == 0 {
            fill_rect(&layer, MARGIN, row_y - 1.5, T_END - MARGIN, 6.5, PANEL_ALT);
        }

        set_color(&layer, TEXT_PRI);
        layer.use_text(truncate(&proj.name, 32), 8.0, Mm(col_proj), Mm(row_y), &font_r);
        set_color(&layer, TEXT_SEC);
        layer.use_text(p_total.to_string(), 8.0, Mm(col_tot),  Mm(row_y), &font_r);
        layer.use_text(p_pass.to_string(),  8.0, Mm(col_pass), Mm(row_y), &font_r);
        layer.use_text(p_warn.to_string(),  8.0, Mm(col_warn), Mm(row_y), &font_r);

        if p_err > 0 {
            fill_rounded_rect(&layer, col_err - 0.5, row_y - 1.2, 14.0, 4.5, R_BADGE, ERR_BG);
            set_color(&layer, ERR_FG);
            layer.use_text(p_err.to_string(), 8.0, Mm(col_err + 1.0), Mm(row_y), &font_b);
        } else {
            set_color(&layer, TEXT_MUT);
            layer.use_text("0", 8.0, Mm(col_err), Mm(row_y), &font_r);
        }
    }

    if projects.len() > MAX_ROWS {
        let more_y = tbl_hdr_y - 7.5 - show as f32 * 6.5;
        set_color(&layer, TEXT_MUT);
        layer.use_text(
            format!("+ {} more…", projects.len() - MAX_ROWS),
            7.5, Mm(col_proj), Mm(more_y), &font_r,
        );
    }

    // What's in this report — compact bullet
    let bullet_y = tbl_hdr_y - 7.5 - (show.min(MAX_ROWS) as f32 + 1.0) * 6.5 - 4.0;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, bullet_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("WHAT'S IN THIS REPORT", 6.5, Mm(MARGIN), Mm(bullet_y - 7.5), &font_b);
    fill_rounded_rect(&layer, MARGIN, bullet_y - 14.5, 2.0, 2.0, 1.0, ACCENT_PUR);
    set_color(&layer, TEXT_SEC);
    layer.use_text(
        "For each project: Risk Summary + Dependency Table",
        8.0, Mm(MARGIN + 5.0), Mm(bullet_y - 14.5), &font_r,
    );

    // Footer
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 22.0, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text(
        format!("Generated by license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(MARGIN), Mm(15.0), &font_r,
    );
    layer.use_text(chrono_now(), 7.5, Mm(PAGE_W - MARGIN - 22.0), Mm(15.0), &font_r);

    Ok(())
}

// ── Cover page ────────────────────────────────────────────────────────────────

fn add_cover_page(
    doc: &PdfDocumentReference,
    deps: &[Dependency],
    project_name: &str,
) -> Result<()> {
    let (page_idx, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Cover");
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    let pass  = deps.iter().filter(|d| d.verdict == PolicyVerdict::Pass).count();
    let warn  = deps.iter().filter(|d| d.verdict == PolicyVerdict::Warn).count();
    let error = deps.iter().filter(|d| d.verdict == PolicyVerdict::Error).count();

    // ── Background ────────────────────────────────────────────────────────────
    fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);

    // ── Gradient header zone (top COVER_HDR_H mm) ─────────────────────────────
    let hdr_bot = PAGE_H - COVER_HDR_H;
    fill_gradient_h(&layer, 0.0, hdr_bot, PAGE_W, COVER_HDR_H, ACCENT_BLU, ACCENT_PUR, 28);

    // Tool version — white, small, top-right of header
    set_color(&layer, WHITE_DIM);
    layer.use_text(
        format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(PAGE_W - MARGIN - 44.0), Mm(PAGE_H - 10.5), &font_r,
    );

    // Title
    set_color(&layer, WHITE);
    layer.use_text("License Compliance", 28.0, Mm(MARGIN), Mm(PAGE_H - 26.0), &font_b);
    set_color(&layer, WHITE_DIM);
    layer.use_text("Report", 28.0, Mm(MARGIN), Mm(PAGE_H - 41.0), &font_b);

    // ── Project chip (just below header) ──────────────────────────────────────
    let chip_y = hdr_bot - 18.0;
    let chip_h = 12.0f32;
    let chip_w = 106.0f32;
    fill_rounded_rect(&layer, MARGIN, chip_y, chip_w, chip_h, R_BADGE, PANEL);
    stroke_rounded_rect(&layer, MARGIN, chip_y, chip_w, chip_h, R_BADGE, PANEL_BORDER);
    // Thin accent bar on the left of the chip (not rounded, sits inside)
    fill_rect(&layer, MARGIN, chip_y, 2.5, chip_h, ACCENT_BLU);

    set_color(&layer, TEXT_MUT);
    layer.use_text("PROJECT", 6.0, Mm(MARGIN + 5.0), Mm(chip_y + chip_h - 3.8), &font_b);
    set_color(&layer, TEXT_PRI);
    layer.use_text(
        truncate(project_name, 34),
        9.5, Mm(MARGIN + 5.0), Mm(chip_y + 2.8), &font_b,
    );

    // ── Scan date ─────────────────────────────────────────────────────────────
    set_color(&layer, TEXT_SEC);
    layer.use_text(
        format!("Scanned  {}", chrono_now()),
        9.0, Mm(MARGIN), Mm(chip_y - 8.0), &font_r,
    );

    // ── Divider + OVERVIEW ────────────────────────────────────────────────────
    let rule_y = chip_y - 16.5;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, rule_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("OVERVIEW", 6.5, Mm(MARGIN), Mm(rule_y - 7.0), &font_b);

    // ── Stat cards (4 in a row) ───────────────────────────────────────────────
    let card_y  = rule_y - 42.0;
    let card_h  = 26.0f32;
    let gap     = 4.0f32;
    let total_w = T_END - MARGIN;
    let card_w  = (total_w - gap * 3.0) / 4.0;

    let cards: [(&str, String, (f32, f32, f32)); 4] = [
        ("TOTAL",  deps.len().to_string(), ACCENT_BLU),
        ("PASS",   pass.to_string(),       PASS_FG),
        ("WARN",   warn.to_string(),       WARN_FG),
        ("ERROR",  error.to_string(),      ERR_FG),
    ];

    for (i, (label, value, accent)) in cards.iter().enumerate() {
        let cx = MARGIN + (card_w + gap) * i as f32;
        draw_stat_card(&layer, cx, card_y, card_w, card_h, label, value, *accent,
                       &font_r, &font_b);
    }

    // ── "What's in this report" section ───────────────────────────────────────
    let section_y = card_y - 13.0;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, section_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("WHAT'S IN THIS REPORT", 6.5, Mm(MARGIN), Mm(section_y - 7.5), &font_b);

    let items = [
        ("Risk Summary",       "Dependencies grouped by license risk level"),
        ("All Dependencies",   "Full scan results with license and policy verdict"),
    ];
    for (j, (title, desc)) in items.iter().enumerate() {
        let iy = section_y - 15.0 - j as f32 * 10.0;
        // Small dot
        fill_rounded_rect(&layer, MARGIN, iy + 2.0, 2.0, 2.0, 1.0, ACCENT_BLU);
        set_color(&layer, TEXT_PRI);
        layer.use_text(*title, 8.5, Mm(MARGIN + 5.0), Mm(iy + 2.0), &font_b);
        set_color(&layer, TEXT_SEC);
        layer.use_text(*desc, 8.0, Mm(MARGIN + 5.0), Mm(iy - 3.5), &font_r);
    }

    // ── Footer ────────────────────────────────────────────────────────────────
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 22.0, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text(
        format!("Generated by license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(MARGIN), Mm(15.0), &font_r,
    );
    layer.use_text(chrono_now(), 7.5, Mm(PAGE_W - MARGIN - 22.0), Mm(15.0), &font_r);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_stat_card(
    layer: &PdfLayerReference,
    x: f32, y: f32, w: f32, h: f32,
    label: &str,
    value: &str,
    accent: (f32, f32, f32),
    font_r: &IndirectFontRef,
    font_b: &IndirectFontRef,
) {
    fill_rounded_rect(layer, x, y, w, h, R_BADGE, PANEL);
    stroke_rounded_rect(layer, x, y, w, h, R_BADGE, PANEL_BORDER);

    // Thin accent top strip
    fill_rect(layer, x, y + h - 2.0, w, 2.0, accent);

    set_color(layer, accent);
    layer.use_text(value, 20.0, Mm(x + 5.0), Mm(y + h * 0.38), font_b);

    set_color(layer, TEXT_MUT);
    layer.use_text(label, 6.5, Mm(x + 5.0), Mm(y + 3.5), font_r);
}

// ── Risk summary page ─────────────────────────────────────────────────────────

struct RowDef {
    name: &'static str,
    risk: LicenseRisk,
    description: &'static str,
    bg: (f32, f32, f32),
    fg: (f32, f32, f32),
}

struct RenderedRow {
    name: &'static str,
    bg: (f32, f32, f32),
    fg: (f32, f32, f32),
    desc_lines: Vec<String>,
    dep_lines: Vec<String>,
    height: f32,
}

fn add_risk_summary_page(
    doc: &PdfDocumentReference,
    deps: &[Dependency],
    project_label: Option<&str>,
) -> Result<()> {
    let (page_idx, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Risk Summary");
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);
    fill_gradient_h(&layer, 0.0, PAGE_H - 2.5, PAGE_W, 2.5, ACCENT_BLU, ACCENT_PUR, 21);

    let defs = [
        RowDef {
            name: "Permissive",
            risk: LicenseRisk::Permissive,
            description: "Minimal restrictions — use freely in any project, commercial or otherwise.",
            bg: PASS_BG, fg: PASS_FG,
        },
        RowDef {
            name: "Weak Copyleft",
            risk: LicenseRisk::WeakCopyleft,
            description: "Share-alike applies only to modifications of the library itself.",
            bg: WARN_BG, fg: WARN_FG,
        },
        RowDef {
            name: "Strong Copyleft",
            risk: LicenseRisk::StrongCopyleft,
            description: "Your project may need to be released as open source if you use this.",
            bg: ERR_BG, fg: ERR_FG,
        },
        RowDef {
            name: "Proprietary",
            risk: LicenseRisk::Proprietary,
            description: "Source is closed; a commercial agreement is required for use.",
            bg: PROP_BG, fg: PROP_FG,
        },
        RowDef {
            name: "Unknown",
            risk: LicenseRisk::Unknown,
            description: "License could not be determined. Use --online to resolve it.",
            bg: PANEL_ALT, fg: TEXT_SEC,
        },
    ];

    let rows: Vec<RenderedRow> = defs.iter().map(|d| {
        let names: Vec<String> = deps.iter()
            .filter(|dep| dep.risk == d.risk)
            .map(|dep| dep.name.clone())
            .collect();
        let desc_lines = wrap_text(d.description, DESC_WRAP);
        // All names listed first (capped to DEPS_MAX_LINES), count line at the bottom
        let dep_lines = {
            let mut lines = format_dep_count_list(&names, DEPS_WRAP);
            if lines.len() > DEPS_MAX_LINES {
                let count_line = lines.last().cloned().unwrap_or_default();
                lines.truncate(DEPS_MAX_LINES - 1);
                lines.push(count_line);
            }
            lines
        };
        let n = desc_lines.len().max(dep_lines.len()).max(2) as f32;
        RenderedRow {
            name: d.name, bg: d.bg, fg: d.fg, desc_lines, dep_lines,
            height: n * LINE_H + ROW_PAD * 2.0,
        }
    }).collect();

    const TABLE_TOP: f32 = 258.0;
    let total_h = HDR_H + rows.iter().map(|r| r.height).sum::<f32>();
    let table_bot = TABLE_TOP - total_h;
    let table_w = T_END - C1_X;

    // Page header
    set_color(&layer, TEXT_PRI);
    let heading = match project_label {
        Some(name) => format!("Risk Summary — {}", name),
        None => "Risk Summary".to_string(),
    };
    layer.use_text(truncate(&heading, 44), 20.0, Mm(MARGIN), Mm(278.5), &font_b);
    set_color(&layer, TEXT_SEC);
    layer.use_text(
        "All dependencies grouped by license risk level",
        9.0, Mm(MARGIN), Mm(271.5), &font_r,
    );
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 267.5, PANEL_BORDER);

    // Table panel background (white, rounded)
    fill_rounded_rect(&layer, C1_X, table_bot, table_w, total_h, R_PANEL, PANEL);
    stroke_rounded_rect(&layer, C1_X, table_bot, table_w, total_h, R_PANEL, PANEL_BORDER);

    // Header row labels + bottom separator
    set_color(&layer, TEXT_SEC);
    layer.use_text("RISK LEVEL",    7.0, Mm(C1_X + 4.0),  Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("WHAT IT MEANS", 7.0, Mm(C2_X + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("DEPENDENCIES",  7.0, Mm(C3_X + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    draw_hline(&layer, C1_X + R_PANEL, T_END - R_PANEL, TABLE_TOP - HDR_H, PANEL_BORDER);

    // Data rows
    let mut y_top = TABLE_TOP - HDR_H;

    for (i, row) in rows.iter().enumerate() {
        let y_bot = y_top - row.height;

        if i % 2 == 1 {
            fill_rect(&layer, C1_X, y_bot, table_w, row.height, PANEL_ALT);
        }

        // Risk badge (rounded)
        let badge_x = C1_X + 3.0;
        let badge_y = y_top - ROW_PAD - BADGE_H;
        fill_rounded_rect(&layer, badge_x, badge_y, BADGE_W, BADGE_H, R_BADGE, row.bg);

        // Dot in badge
        fill_rounded_rect(&layer,
            badge_x + 2.5, badge_y + (BADGE_H - DOT_SIZE) / 2.0,
            DOT_SIZE, DOT_SIZE, DOT_SIZE / 2.0, row.fg);

        set_color(&layer, row.fg);
        layer.use_text(row.name, 8.0, Mm(badge_x + 7.5), Mm(badge_y + 1.5), &font_b);

        // Description
        set_color(&layer, TEXT_SEC);
        for (j, line) in row.desc_lines.iter().enumerate() {
            let ly = y_top - ROW_PAD - (j as f32 + 0.9) * LINE_H;
            layer.use_text(line.as_str(), 8.0, Mm(C2_X + 2.0), Mm(ly), &font_r);
        }

        // Dependency names — all names listed first (muted), count line last (bold, prominent)
        let last_dep_idx = row.dep_lines.len().saturating_sub(1);
        for (j, line) in row.dep_lines.iter().enumerate() {
            let ly = y_top - ROW_PAD - (j as f32 + 0.9) * LINE_H;
            if j == last_dep_idx {
                set_color(&layer, TEXT_PRI);
                layer.use_text(line.as_str(), 9.0, Mm(C3_X + 2.0), Mm(ly), &font_b);
            } else {
                set_color(&layer, TEXT_MUT);
                layer.use_text(line.as_str(), 7.0, Mm(C3_X + 2.0), Mm(ly), &font_r);
            }
        }

        if i < rows.len() - 1 {
            draw_hline(&layer, C1_X + R_PANEL, T_END - R_PANEL, y_bot, PANEL_BORDER);
        }
        y_top = y_bot;
    }

    // Footer
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 22.0, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text(
        format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(MARGIN), Mm(15.0), &font_r,
    );

    Ok(())
}

// ── Full dependency table pages ───────────────────────────────────────────────

fn add_table_pages(
    doc: &PdfDocumentReference,
    deps: &[Dependency],
    project_label: Option<&str>,
) -> Result<()> {
    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    const BASE_ROW_H: f32 = 7.0;
    const EXTRA_LINE_H: f32 = 3.5;
    const HDR_Y: f32 = 268.5;
    const FIRST_Y: f32 = 259.5;
    const BOT_MARGIN: f32 = 25.0;
    const LICENSE_WRAP: usize = 38;

    //  NAME       VERSION    ECOSYSTEM  LICENSE    VERDICT
    //  18…68      68…88      88…110     110…150    150…192  (mm)
    let col_x = [MARGIN, MARGIN + 50.0, MARGIN + 70.0, MARGIN + 90.0, MARGIN + 152.0];
    let headers = ["NAME", "VERSION", "ECOSYSTEM", "LICENSE", "VERDICT"];

    // Pre-compute license lines and dynamic row heights
    let dep_data: Vec<(Vec<String>, f32)> = deps.iter().map(|dep| {
        let license = dep.license_spdx.as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown");
        let lines = wrap_text(license, LICENSE_WRAP);
        let extra = lines.len().saturating_sub(1);
        let h = BASE_ROW_H + extra as f32 * EXTRA_LINE_H;
        (lines, h)
    }).collect();

    let mut cur_y = FIRST_Y;
    let mut page_state: Option<(PdfPageIndex, PdfLayerIndex)> = None;
    let mut page_num: u32 = 0;

    for (row_idx, dep) in deps.iter().enumerate() {
        let (license_lines, row_h) = &dep_data[row_idx];
        let row_h = *row_h;

        let needs_new_page = page_state.is_none() || cur_y - row_h < BOT_MARGIN;

        if needs_new_page {
            page_num += 1;
            let (pi, li) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Deps");
            let layer = doc.get_page(pi).get_layer(li);

            fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);
            fill_gradient_h(&layer, 0.0, PAGE_H - 2.5, PAGE_W, 2.5, ACCENT_BLU, ACCENT_PUR, 21);

            set_color(&layer, TEXT_PRI);
            let deps_heading = match project_label {
                Some(name) => format!("All Dependencies — {}", name),
                None => "All Dependencies".to_string(),
            };
            layer.use_text(truncate(&deps_heading, 46), 14.0, Mm(MARGIN), Mm(282.5), &font_b);
            set_color(&layer, TEXT_MUT);
            layer.use_text(
                format!("Page {}", page_num),
                8.0, Mm(PAGE_W - MARGIN - 14.0), Mm(283.0), &font_r,
            );
            draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 277.5, PANEL_BORDER);

            // Header row (white rounded panel)
            fill_rounded_rect(&layer, MARGIN, HDR_Y - 7.5, PAGE_W - 2.0 * MARGIN, 9.5, R_BADGE, PANEL);
            stroke_rounded_rect(&layer, MARGIN, HDR_Y - 7.5, PAGE_W - 2.0 * MARGIN, 9.5, R_BADGE, PANEL_BORDER);
            set_color(&layer, TEXT_MUT);
            for (i, h) in headers.iter().enumerate() {
                layer.use_text(*h, 7.0, Mm(col_x[i] + 1.5), Mm(HDR_Y - 4.0), &font_b);
            }

            // Footer
            draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 22.0, PANEL_BORDER);
            set_color(&layer, TEXT_MUT);
            layer.use_text(
                format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
                7.5, Mm(MARGIN), Mm(15.0), &font_r,
            );

            cur_y = FIRST_Y;
            page_state = Some((pi, li));
        }

        let (pi, li) = page_state.unwrap();
        let layer = doc.get_page(pi).get_layer(li);

        let (verdict_str, verdict_fg, verdict_bg) = match dep.verdict {
            PolicyVerdict::Pass  => ("PASS",  PASS_FG, PASS_BG),
            PolicyVerdict::Warn  => ("WARN",  WARN_FG, WARN_BG),
            PolicyVerdict::Error => ("ERROR", ERR_FG,  ERR_BG),
        };

        // Alternating row background (even rows get a subtle tint)
        if row_idx % 2 == 0 {
            fill_rect(&layer, MARGIN, cur_y - row_h + 1.5, PAGE_W - 2.0 * MARGIN, row_h, PANEL_ALT);
        }

        let text_y = cur_y - 4.0;

        set_color(&layer, TEXT_PRI);
        layer.use_text(truncate(&dep.name, 30), 8.0, Mm(col_x[0] + 1.5), Mm(text_y), &font_r);
        set_color(&layer, TEXT_SEC);
        layer.use_text(&dep.version, 8.0, Mm(col_x[1] + 1.5), Mm(text_y), &font_r);
        layer.use_text(dep.ecosystem.to_string(), 8.0, Mm(col_x[2] + 1.5), Mm(text_y), &font_r);

        // License — wrapped across multiple lines, no truncation
        set_color(&layer, TEXT_SEC);
        for (j, line) in license_lines.iter().enumerate() {
            let line_y = text_y - j as f32 * EXTRA_LINE_H;
            layer.use_text(line.as_str(), 8.0, Mm(col_x[3] + 1.5), Mm(line_y), &font_r);
        }

        // Verdict badge — stays within col[4] to T_END (150..192 = 42mm)
        let badge_x = col_x[4] + 1.5;
        let badge_y = cur_y - row_h + 2.2;
        fill_rounded_rect(&layer, badge_x, badge_y, 20.0, 4.8, R_BADGE, verdict_bg);
        set_color(&layer, verdict_fg);
        layer.use_text(verdict_str, 7.0, Mm(badge_x + 3.0), Mm(badge_y + 1.1), &font_b);

        // Row separator
        draw_hline(&layer, MARGIN, T_END, cur_y - row_h + 1.5, PANEL_BORDER);

        cur_y -= row_h;
    }

    Ok(())
}

// ── Drawing helpers ───────────────────────────────────────────────────────────

fn set_color(layer: &PdfLayerReference, (r, g, b): (f32, f32, f32)) {
    layer.set_fill_color(Color::Rgb(Rgb { r, g, b, icc_profile: None }));
}

fn fill_rect(layer: &PdfLayerReference, x: f32, y: f32, w: f32, h: f32,
             (r, g, b): (f32, f32, f32)) {
    layer.set_fill_color(Color::Rgb(Rgb { r, g, b, icc_profile: None }));
    layer.add_polygon(Polygon {
        rings: vec![vec![
            (Point::new(Mm(x),     Mm(y)),     false),
            (Point::new(Mm(x + w), Mm(y)),     false),
            (Point::new(Mm(x + w), Mm(y + h)), false),
            (Point::new(Mm(x),     Mm(y + h)), false),
        ]],
        mode: PaintMode::Fill,
        winding_order: WindingOrder::NonZero,
    });
    layer.set_fill_color(Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }));
}

/// Build a clockwise polygon ring approximating a rounded rectangle.
/// Uses 8 line segments per quarter-circle arc.
fn rounded_rect_ring(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<(Point, bool)> {
    let r = r.min(w / 2.0).min(h / 2.0);
    const SEGS: usize = 8;
    let mut pts = Vec::with_capacity(4 * (SEGS + 1));

    // (corner_cx, corner_cy, arc_start_deg, arc_end_deg) — clockwise order
    let corners = [
        (x + w - r, y + r,     270.0f32, 360.0f32), // bottom-right
        (x + w - r, y + h - r, 0.0f32,   90.0f32),  // top-right
        (x + r,     y + h - r, 90.0f32,  180.0f32), // top-left
        (x + r,     y + r,     180.0f32, 270.0f32), // bottom-left
    ];

    for (cx, cy, start, end) in &corners {
        for i in 0..=SEGS {
            let t = i as f32 / SEGS as f32;
            let angle = (start + (end - start) * t).to_radians();
            pts.push((
                Point::new(Mm(cx + r * angle.cos()), Mm(cy + r * angle.sin())),
                false,
            ));
        }
    }
    pts
}

fn fill_rounded_rect(layer: &PdfLayerReference, x: f32, y: f32, w: f32, h: f32,
                     r: f32, (cr, cg, cb): (f32, f32, f32)) {
    layer.set_fill_color(Color::Rgb(Rgb { r: cr, g: cg, b: cb, icc_profile: None }));
    layer.add_polygon(Polygon {
        rings: vec![rounded_rect_ring(x, y, w, h, r)],
        mode: PaintMode::Fill,
        winding_order: WindingOrder::NonZero,
    });
    layer.set_fill_color(Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }));
}

fn stroke_rounded_rect(layer: &PdfLayerReference, x: f32, y: f32, w: f32, h: f32,
                       r: f32, (cr, cg, cb): (f32, f32, f32)) {
    layer.set_outline_color(Color::Rgb(Rgb { r: cr, g: cg, b: cb, icc_profile: None }));
    layer.set_outline_thickness(0.4);
    layer.add_polygon(Polygon {
        rings: vec![rounded_rect_ring(x, y, w, h, r)],
        mode: PaintMode::Stroke,
        winding_order: WindingOrder::NonZero,
    });
    layer.set_outline_color(Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }));
    layer.set_outline_thickness(1.0);
}

fn draw_hline(layer: &PdfLayerReference, x1: f32, x2: f32, y: f32,
              (r, g, b): (f32, f32, f32)) {
    layer.set_outline_color(Color::Rgb(Rgb { r, g, b, icc_profile: None }));
    layer.set_outline_thickness(0.3);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(x1), Mm(y)), false),
            (Point::new(Mm(x2), Mm(y)), false),
        ],
        is_closed: false,
    });
    layer.set_outline_color(Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }));
    layer.set_outline_thickness(1.0);
}

/// Fill a left-to-right gradient rectangle using `steps` vertical strips.
#[allow(clippy::too_many_arguments)]
fn fill_gradient_h(
    layer: &PdfLayerReference,
    x: f32, y: f32, w: f32, h: f32,
    from: (f32, f32, f32),
    to: (f32, f32, f32),
    steps: usize,
) {
    let step_w = w / steps as f32;
    for i in 0..steps {
        let t = i as f32 / (steps - 1).max(1) as f32;
        let color = (
            from.0 + (to.0 - from.0) * t,
            from.1 + (to.1 - from.1) * t,
            from.2 + (to.2 - from.2) * t,
        );
        // Overlap strips slightly to prevent rounding gaps
        fill_rect(layer, x + i as f32 * step_w, y, step_w + 0.6, h, color);
    }
}

// ── Text helpers ──────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max {
        format!("{}…", chars[..max - 1].iter().collect::<String>())
    } else {
        s.to_string()
    }
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() > max_chars {
            lines.push(current.clone());
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}


/// All names wrapped into lines first, then "<N> package(s)" as the final line.
fn format_dep_count_list(names: &[String], max_chars: usize) -> Vec<String> {
    if names.is_empty() {
        return vec!["—".to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for name in names {
        let sep = if current.is_empty() { "" } else { ", " };
        let candidate = format!("{}{}", sep, name);
        if !current.is_empty() && current.len() + candidate.len() > max_chars {
            lines.push(current.clone());
            current = name.clone();
        } else {
            current.push_str(&candidate);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    let count_line = format!("{} package{}", names.len(), if names.len() == 1 { "" } else { "s" });
    lines.push(count_line);
    lines
}

// ── Date helper ───────────────────────────────────────────────────────────────

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days  = secs / 86400;
    let year  = 1970 + days / 365;
    let doy   = days % 365;
    let month = (doy / 30) + 1;
    let day   = (doy % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(31))
}
