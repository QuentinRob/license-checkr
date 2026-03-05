//! SBOM PDF report renderer.
//!
//! Follows the same Light Liquid Glass design system as [`super::pdf`]:
//! blue-to-purple gradient header, stat cards, rounded panels, and alternating
//! row tints.  The dependency table columns are adapted for SBOM data:
//! NAME | VERSION | ECOSYSTEM | PURL | LICENSE

use std::path::Path;

use anyhow::{Context, Result};
use printpdf::{
    BuiltinFont, Color, IndirectFontRef, Line, Mm, PdfDocument, PdfDocumentReference,
    PdfLayerIndex, PdfLayerReference, PdfPageIndex, Point, Polygon, Rgb,
};
use printpdf::path::{PaintMode, WindingOrder};

use crate::models::{Dependency, LicenseRisk, ProjectScan};

// ── Identical palette to pdf.rs ───────────────────────────────────────────────
const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 18.0;
const COVER_HDR_H: f32 = 72.0;

const BG:           (f32, f32, f32) = (1.00, 1.00, 1.00);
const PANEL:        (f32, f32, f32) = (1.00, 1.00, 1.00);
const PANEL_ALT:    (f32, f32, f32) = (0.95, 0.96, 0.99);
const PANEL_BORDER: (f32, f32, f32) = (0.85, 0.87, 0.92);
const ACCENT_BLU:   (f32, f32, f32) = (0.20, 0.46, 0.95);
const ACCENT_PUR:   (f32, f32, f32) = (0.52, 0.30, 0.95);
const TEXT_PRI:     (f32, f32, f32) = (0.07, 0.08, 0.14);
const TEXT_SEC:     (f32, f32, f32) = (0.36, 0.40, 0.52);
const TEXT_MUT:     (f32, f32, f32) = (0.58, 0.63, 0.72);
const WHITE:        (f32, f32, f32) = (1.00, 1.00, 1.00);
const WHITE_DIM:    (f32, f32, f32) = (0.82, 0.89, 1.00);

// Risk badge colours
const PASS_BG: (f32, f32, f32) = (0.90, 0.98, 0.92);
const PASS_FG: (f32, f32, f32) = (0.07, 0.52, 0.22);
const WARN_BG: (f32, f32, f32) = (1.00, 0.95, 0.87);
const WARN_FG: (f32, f32, f32) = (0.70, 0.40, 0.02);
const ERR_BG:  (f32, f32, f32) = (1.00, 0.91, 0.91);
const ERR_FG:  (f32, f32, f32) = (0.76, 0.09, 0.13);
const UNK_BG:  (f32, f32, f32) = (0.93, 0.93, 0.97);
const UNK_FG:  (f32, f32, f32) = (0.40, 0.40, 0.60);

const R_PANEL: f32 = 2.5;
const R_BADGE: f32 = 1.5;

const T_END: f32 = PAGE_W - MARGIN;

// ── Public entry points ───────────────────────────────────────────────────────

/// Render a single-project SBOM PDF report.
pub fn render(deps: &[Dependency], project_path: &Path, sbom_format: &str, output_path: &Path) -> Result<()> {
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Project");

    let doc = PdfDocument::empty("SBOM Report");

    add_cover_page(&doc, deps, project_name, sbom_format)?;
    add_ecosystem_summary_page(&doc, deps, None)?;
    add_component_table_pages(&doc, deps, None)?;

    let bytes = doc.save_to_bytes()?;
    std::fs::write(output_path, &bytes)
        .with_context(|| format!("Failed to write SBOM PDF to {}", output_path.display()))?;

    println!("SBOM PDF report written to: {}", output_path.display());
    Ok(())
}

/// Render a workspace SBOM PDF report.
pub fn render_workspace(projects: &[ProjectScan], sbom_format: &str, output_path: &Path) -> Result<()> {
    let doc = PdfDocument::empty("SBOM Report — Workspace");

    add_workspace_cover_page(&doc, projects, sbom_format)?;

    for proj in projects {
        add_ecosystem_summary_page(&doc, &proj.deps, Some(&proj.name))?;
        add_component_table_pages(&doc, &proj.deps, Some(&proj.name))?;
    }

    let bytes = doc.save_to_bytes()?;
    std::fs::write(output_path, &bytes)
        .with_context(|| format!("Failed to write SBOM PDF to {}", output_path.display()))?;

    println!("SBOM PDF workspace report written to: {}", output_path.display());
    Ok(())
}

// ── Cover page ────────────────────────────────────────────────────────────────

fn add_cover_page(
    doc: &PdfDocumentReference,
    deps: &[Dependency],
    project_name: &str,
    sbom_format: &str,
) -> Result<()> {
    let (page_idx, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Cover");
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);

    let hdr_bot = PAGE_H - COVER_HDR_H;
    fill_gradient_h(&layer, 0.0, hdr_bot, PAGE_W, COVER_HDR_H, ACCENT_BLU, ACCENT_PUR, 28);

    set_color(&layer, WHITE_DIM);
    layer.use_text(
        format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(PAGE_W - MARGIN - 44.0), Mm(PAGE_H - 10.5), &font_r,
    );

    set_color(&layer, WHITE);
    layer.use_text("Software Bill of Materials", 22.0, Mm(MARGIN), Mm(PAGE_H - 26.0), &font_b);
    set_color(&layer, WHITE_DIM);
    layer.use_text("SBOM Report", 22.0, Mm(MARGIN), Mm(PAGE_H - 39.0), &font_b);

    // Format badge in header
    let fmt_badge_x = PAGE_W - MARGIN - 50.0;
    fill_rounded_rect(&layer, fmt_badge_x, PAGE_H - 50.0, 48.0, 8.0, R_BADGE, (0.60, 0.72, 0.98));
    set_color(&layer, WHITE);
    layer.use_text(sbom_format.to_uppercase(), 7.0, Mm(fmt_badge_x + 3.0), Mm(PAGE_H - 46.5), &font_b);

    // Project chip
    let chip_y = hdr_bot - 18.0;
    let chip_h = 12.0f32;
    let chip_w = 106.0f32;
    fill_rounded_rect(&layer, MARGIN, chip_y, chip_w, chip_h, R_BADGE, PANEL);
    stroke_rounded_rect(&layer, MARGIN, chip_y, chip_w, chip_h, R_BADGE, PANEL_BORDER);
    fill_rect(&layer, MARGIN, chip_y, 2.5, chip_h, ACCENT_BLU);

    set_color(&layer, TEXT_MUT);
    layer.use_text("PROJECT", 6.0, Mm(MARGIN + 5.0), Mm(chip_y + chip_h - 3.8), &font_b);
    set_color(&layer, TEXT_PRI);
    layer.use_text(truncate(project_name, 34), 9.5, Mm(MARGIN + 5.0), Mm(chip_y + 2.8), &font_b);

    set_color(&layer, TEXT_SEC);
    layer.use_text(
        format!("Generated  {}", chrono_now()),
        9.0, Mm(MARGIN), Mm(chip_y - 8.0), &font_r,
    );

    // Divider + OVERVIEW
    let rule_y = chip_y - 16.5;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, rule_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("OVERVIEW", 6.5, Mm(MARGIN), Mm(rule_y - 7.0), &font_b);

    // Stat cards — Total, Permissive, Weak Copyleft, Unknown/Other
    let permissive = deps.iter().filter(|d| d.risk == LicenseRisk::Permissive).count();
    let copyleft   = deps.iter().filter(|d| matches!(d.risk, LicenseRisk::WeakCopyleft | LicenseRisk::StrongCopyleft)).count();
    let unknown    = deps.iter().filter(|d| d.risk == LicenseRisk::Unknown).count();

    let card_y  = rule_y - 42.0;
    let card_h  = 26.0f32;
    let gap     = 4.0f32;
    let total_w = T_END - MARGIN;
    let card_w  = (total_w - gap * 3.0) / 4.0;

    let cards: [(&str, String, (f32, f32, f32)); 4] = [
        ("TOTAL",       deps.len().to_string(),    ACCENT_BLU),
        ("PERMISSIVE",  permissive.to_string(),    PASS_FG),
        ("COPYLEFT",    copyleft.to_string(),      WARN_FG),
        ("UNKNOWN",     unknown.to_string(),       UNK_FG),
    ];

    for (i, (label, value, accent)) in cards.iter().enumerate() {
        let cx = MARGIN + (card_w + gap) * i as f32;
        draw_stat_card(&layer, cx, card_y, card_w, card_h, label, value, *accent, &font_r, &font_b);
    }

    // What's in this report
    let section_y = card_y - 13.0;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, section_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("WHAT'S IN THIS REPORT", 6.5, Mm(MARGIN), Mm(section_y - 7.5), &font_b);

    let items = [
        ("Ecosystem Summary",  "Components grouped by package ecosystem and license risk"),
        ("Component Catalog",  "Full SBOM inventory with PURL, license, and risk classification"),
    ];
    for (j, (title, desc)) in items.iter().enumerate() {
        let iy = section_y - 15.0 - j as f32 * 10.0;
        fill_rounded_rect(&layer, MARGIN, iy + 2.0, 2.0, 2.0, 1.0, ACCENT_BLU);
        set_color(&layer, TEXT_PRI);
        layer.use_text(*title, 8.5, Mm(MARGIN + 5.0), Mm(iy + 2.0), &font_b);
        set_color(&layer, TEXT_SEC);
        layer.use_text(*desc, 8.0, Mm(MARGIN + 5.0), Mm(iy - 3.5), &font_r);
    }

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

fn add_workspace_cover_page(
    doc: &PdfDocumentReference,
    projects: &[ProjectScan],
    sbom_format: &str,
) -> Result<()> {
    let (page_idx, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Cover");
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    let all_deps: Vec<&Dependency> = projects.iter().flat_map(|p| &p.deps).collect();
    let permissive = all_deps.iter().filter(|d| d.risk == LicenseRisk::Permissive).count();
    let copyleft   = all_deps.iter().filter(|d| matches!(d.risk, LicenseRisk::WeakCopyleft | LicenseRisk::StrongCopyleft)).count();
    let unknown    = all_deps.iter().filter(|d| d.risk == LicenseRisk::Unknown).count();

    fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);
    let hdr_bot = PAGE_H - COVER_HDR_H;
    fill_gradient_h(&layer, 0.0, hdr_bot, PAGE_W, COVER_HDR_H, ACCENT_BLU, ACCENT_PUR, 28);

    set_color(&layer, WHITE_DIM);
    layer.use_text(
        format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(PAGE_W - MARGIN - 44.0), Mm(PAGE_H - 10.5), &font_r,
    );

    set_color(&layer, WHITE);
    layer.use_text("Software Bill of Materials", 22.0, Mm(MARGIN), Mm(PAGE_H - 26.0), &font_b);
    set_color(&layer, WHITE_DIM);
    layer.use_text("Workspace SBOM Report", 22.0, Mm(MARGIN), Mm(PAGE_H - 39.0), &font_b);

    // Format badge
    let fmt_badge_x = PAGE_W - MARGIN - 50.0;
    fill_rounded_rect(&layer, fmt_badge_x, PAGE_H - 50.0, 48.0, 8.0, R_BADGE, (0.60, 0.72, 0.98));
    set_color(&layer, WHITE);
    layer.use_text(sbom_format.to_uppercase(), 7.0, Mm(fmt_badge_x + 3.0), Mm(PAGE_H - 46.5), &font_b);

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

    set_color(&layer, TEXT_SEC);
    layer.use_text(
        format!("Generated  {}", chrono_now()),
        9.0, Mm(MARGIN), Mm(chip_y - 8.0), &font_r,
    );

    let rule_y = chip_y - 16.5;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, rule_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("OVERVIEW", 6.5, Mm(MARGIN), Mm(rule_y - 7.0), &font_b);

    let card_y  = rule_y - 42.0;
    let card_h  = 26.0f32;
    let gap     = 4.0f32;
    let total_w = T_END - MARGIN;
    let card_w  = (total_w - gap * 3.0) / 4.0;

    let cards: [(&str, String, (f32, f32, f32)); 4] = [
        ("TOTAL",      all_deps.len().to_string(), ACCENT_BLU),
        ("PERMISSIVE", permissive.to_string(),     PASS_FG),
        ("COPYLEFT",   copyleft.to_string(),       WARN_FG),
        ("UNKNOWN",    unknown.to_string(),        UNK_FG),
    ];

    for (i, (label, value, accent)) in cards.iter().enumerate() {
        let cx = MARGIN + (card_w + gap) * i as f32;
        draw_stat_card(&layer, cx, card_y, card_w, card_h, label, value, *accent, &font_r, &font_b);
    }

    // Projects table
    let section_y = card_y - 13.0;
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, section_y, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text("PROJECTS SCANNED", 6.5, Mm(MARGIN), Mm(section_y - 7.5), &font_b);

    let tbl_hdr_y = section_y - 14.0;
    let col_proj = MARGIN + 2.0;
    let col_tot  = MARGIN + 88.0;
    let col_perm = MARGIN + 106.0;
    let col_copy = MARGIN + 124.0;
    let col_unk  = MARGIN + 143.0;

    set_color(&layer, TEXT_MUT);
    layer.use_text("PROJECT",    6.5, Mm(col_proj), Mm(tbl_hdr_y), &font_b);
    layer.use_text("TOTAL",      6.5, Mm(col_tot),  Mm(tbl_hdr_y), &font_b);
    layer.use_text("PERMISSIVE", 6.5, Mm(col_perm), Mm(tbl_hdr_y), &font_b);
    layer.use_text("COPYLEFT",   6.5, Mm(col_copy), Mm(tbl_hdr_y), &font_b);
    layer.use_text("UNKNOWN",    6.5, Mm(col_unk),  Mm(tbl_hdr_y), &font_b);
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, tbl_hdr_y - 2.0, PANEL_BORDER);

    const MAX_ROWS: usize = 12;
    let show = projects.len().min(MAX_ROWS);

    for (i, proj) in projects.iter().take(show).enumerate() {
        let row_y = tbl_hdr_y - 7.5 - i as f32 * 6.5;
        let p_total = proj.deps.len();
        let p_perm  = proj.deps.iter().filter(|d| d.risk == LicenseRisk::Permissive).count();
        let p_copy  = proj.deps.iter().filter(|d| matches!(d.risk, LicenseRisk::WeakCopyleft | LicenseRisk::StrongCopyleft)).count();
        let p_unk   = proj.deps.iter().filter(|d| d.risk == LicenseRisk::Unknown).count();

        if i % 2 == 0 {
            fill_rect(&layer, MARGIN, row_y - 1.5, T_END - MARGIN, 6.5, PANEL_ALT);
        }

        set_color(&layer, TEXT_PRI);
        layer.use_text(truncate(&proj.name, 32), 8.0, Mm(col_proj), Mm(row_y), &font_r);
        set_color(&layer, TEXT_SEC);
        layer.use_text(p_total.to_string(), 8.0, Mm(col_tot),  Mm(row_y), &font_r);
        layer.use_text(p_perm.to_string(),  8.0, Mm(col_perm), Mm(row_y), &font_r);
        layer.use_text(p_copy.to_string(),  8.0, Mm(col_copy), Mm(row_y), &font_r);
        layer.use_text(p_unk.to_string(),   8.0, Mm(col_unk),  Mm(row_y), &font_r);
    }

    if projects.len() > MAX_ROWS {
        let more_y = tbl_hdr_y - 7.5 - show as f32 * 6.5;
        set_color(&layer, TEXT_MUT);
        layer.use_text(
            format!("+ {} more…", projects.len() - MAX_ROWS),
            7.5, Mm(col_proj), Mm(more_y), &font_r,
        );
    }

    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 22.0, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text(
        format!("Generated by license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(MARGIN), Mm(15.0), &font_r,
    );
    layer.use_text(chrono_now(), 7.5, Mm(PAGE_W - MARGIN - 22.0), Mm(15.0), &font_r);

    Ok(())
}

// ── Ecosystem summary page ────────────────────────────────────────────────────

fn add_ecosystem_summary_page(
    doc: &PdfDocumentReference,
    deps: &[Dependency],
    project_label: Option<&str>,
) -> Result<()> {
    use std::collections::BTreeMap;

    let (page_idx, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Ecosystem Summary");
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);
    fill_gradient_h(&layer, 0.0, PAGE_H - 2.5, PAGE_W, 2.5, ACCENT_BLU, ACCENT_PUR, 21);

    let heading = match project_label {
        Some(name) => format!("Ecosystem Summary — {}", name),
        None => "Ecosystem Summary".to_string(),
    };
    set_color(&layer, TEXT_PRI);
    layer.use_text(truncate(&heading, 44), 20.0, Mm(MARGIN), Mm(278.5), &font_b);
    set_color(&layer, TEXT_SEC);
    layer.use_text(
        "Components grouped by ecosystem and license risk",
        9.0, Mm(MARGIN), Mm(271.5), &font_r,
    );
    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 267.5, PANEL_BORDER);

    // Group deps by ecosystem
    let mut by_eco: BTreeMap<String, Vec<&Dependency>> = BTreeMap::new();
    for dep in deps {
        by_eco.entry(dep.ecosystem.to_string()).or_default().push(dep);
    }

    const TABLE_TOP: f32 = 258.0;
    const HDR_H: f32 = 9.0;
    const ROW_H: f32 = 14.0;

    let col1 = MARGIN;
    let col2 = MARGIN + 40.0;
    let col3 = MARGIN + 70.0;
    let col4 = MARGIN + 100.0;
    let col5 = MARGIN + 130.0;
    let col6 = MARGIN + 155.0;

    let table_w = T_END - col1;
    let total_h = HDR_H + by_eco.len() as f32 * ROW_H;
    let table_bot = TABLE_TOP - total_h;

    fill_rounded_rect(&layer, col1, table_bot, table_w, total_h, R_PANEL, PANEL);
    stroke_rounded_rect(&layer, col1, table_bot, table_w, total_h, R_PANEL, PANEL_BORDER);

    set_color(&layer, TEXT_SEC);
    layer.use_text("ECOSYSTEM",  7.0, Mm(col1 + 4.0), Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("TOTAL",      7.0, Mm(col2 + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("PERMISSIVE", 7.0, Mm(col3 + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("WK. COPY.",  7.0, Mm(col4 + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("ST. COPY.",  7.0, Mm(col5 + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    layer.use_text("UNKNOWN",    7.0, Mm(col6 + 2.0), Mm(TABLE_TOP - 6.2), &font_b);
    draw_hline(&layer, col1 + R_PANEL, T_END - R_PANEL, TABLE_TOP - HDR_H, PANEL_BORDER);

    let mut y = TABLE_TOP - HDR_H;

    for (i, (eco_name, eco_deps)) in by_eco.iter().enumerate() {
        let row_bot = y - ROW_H;

        if i % 2 == 1 {
            fill_rect(&layer, col1, row_bot, table_w, ROW_H, PANEL_ALT);
        }

        let total = eco_deps.len();
        let perm  = eco_deps.iter().filter(|d| d.risk == LicenseRisk::Permissive).count();
        let wk    = eco_deps.iter().filter(|d| d.risk == LicenseRisk::WeakCopyleft).count();
        let st    = eco_deps.iter().filter(|d| d.risk == LicenseRisk::StrongCopyleft).count();
        let unk   = eco_deps.iter().filter(|d| d.risk == LicenseRisk::Unknown).count();

        let text_y = y - ROW_H / 2.0 - 1.5;

        set_color(&layer, TEXT_PRI);
        layer.use_text(eco_name.as_str(), 9.0, Mm(col1 + 4.0), Mm(text_y), &font_b);
        set_color(&layer, TEXT_SEC);
        layer.use_text(total.to_string(), 9.0, Mm(col2 + 2.0), Mm(text_y), &font_r);

        // Permissive count with green dot
        if perm > 0 {
            fill_rounded_rect(&layer, col3 + 2.0, text_y - 0.5, 2.5, 2.5, 1.25, PASS_FG);
            set_color(&layer, PASS_FG);
            layer.use_text(perm.to_string(), 9.0, Mm(col3 + 7.0), Mm(text_y), &font_r);
        } else {
            set_color(&layer, TEXT_MUT);
            layer.use_text("—", 9.0, Mm(col3 + 2.0), Mm(text_y), &font_r);
        }

        // Weak copyleft with amber dot
        if wk > 0 {
            fill_rounded_rect(&layer, col4 + 2.0, text_y - 0.5, 2.5, 2.5, 1.25, WARN_FG);
            set_color(&layer, WARN_FG);
            layer.use_text(wk.to_string(), 9.0, Mm(col4 + 7.0), Mm(text_y), &font_r);
        } else {
            set_color(&layer, TEXT_MUT);
            layer.use_text("—", 9.0, Mm(col4 + 2.0), Mm(text_y), &font_r);
        }

        // Strong copyleft with red dot
        if st > 0 {
            fill_rounded_rect(&layer, col5 + 2.0, text_y - 0.5, 2.5, 2.5, 1.25, ERR_FG);
            set_color(&layer, ERR_FG);
            layer.use_text(st.to_string(), 9.0, Mm(col5 + 7.0), Mm(text_y), &font_r);
        } else {
            set_color(&layer, TEXT_MUT);
            layer.use_text("—", 9.0, Mm(col5 + 2.0), Mm(text_y), &font_r);
        }

        // Unknown
        if unk > 0 {
            fill_rounded_rect(&layer, col6 + 2.0, text_y - 0.5, 2.5, 2.5, 1.25, UNK_FG);
            set_color(&layer, UNK_FG);
            layer.use_text(unk.to_string(), 9.0, Mm(col6 + 7.0), Mm(text_y), &font_r);
        } else {
            set_color(&layer, TEXT_MUT);
            layer.use_text("—", 9.0, Mm(col6 + 2.0), Mm(text_y), &font_r);
        }

        if i < by_eco.len() - 1 {
            draw_hline(&layer, col1 + R_PANEL, T_END - R_PANEL, row_bot, PANEL_BORDER);
        }
        y = row_bot;
    }

    draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 22.0, PANEL_BORDER);
    set_color(&layer, TEXT_MUT);
    layer.use_text(
        format!("license-checkr v{}", env!("CARGO_PKG_VERSION")),
        7.5, Mm(MARGIN), Mm(15.0), &font_r,
    );

    Ok(())
}

// ── Component table pages ─────────────────────────────────────────────────────

fn add_component_table_pages(
    doc: &PdfDocumentReference,
    deps: &[Dependency],
    project_label: Option<&str>,
) -> Result<()> {
    let font_b = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_r = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    const BASE_ROW_H: f32 = 7.0;
    const EXTRA_LINE_H: f32 = 3.5;
    const HDR_Y: f32 = 268.5;
    const FIRST_Y: f32 = 261.0;
    const BOT_MARGIN: f32 = 25.0;
    const LICENSE_WRAP: usize = 22;
    // PURL has no spaces so wrap_text never splits it — truncate instead
    const PURL_TRUNCATE: usize = 26;

    // NAME | VERSION | ECOSYSTEM | PURL | LICENSE | RISK
    // Total available: T_END - MARGIN = 192 - 18 = 174mm
    // NAME:36 + VERSION:16 + ECOSYSTEM:20 + PURL:44 + LICENSE:32 + RISK:26 = 174mm
    let col_x = [MARGIN, MARGIN + 36.0, MARGIN + 52.0, MARGIN + 72.0, MARGIN + 116.0, MARGIN + 148.0];
    let headers = ["NAME", "VERSION", "ECOSYSTEM", "PURL", "LICENSE", "RISK"];

    // Pre-compute purl (truncated) + license lines and row heights
    let dep_data: Vec<(String, Vec<String>, f32)> = deps.iter().map(|dep| {
        let purl = purl_for_dep(dep);
        let license = dep.license_spdx.as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown");
        let purl_trunc    = truncate(&purl, PURL_TRUNCATE);
        let license_lines = wrap_text(license, LICENSE_WRAP);
        let extra = license_lines.len().saturating_sub(1);
        let h = BASE_ROW_H + extra as f32 * EXTRA_LINE_H;
        (purl_trunc, license_lines, h)
    }).collect();

    let mut cur_y = FIRST_Y;
    let mut page_state: Option<(PdfPageIndex, PdfLayerIndex)> = None;
    let mut page_num: u32 = 0;

    for (row_idx, dep) in deps.iter().enumerate() {
        let (purl_trunc, license_lines, row_h) = &dep_data[row_idx];
        let row_h = *row_h;

        let needs_new_page = page_state.is_none() || cur_y - row_h < BOT_MARGIN;

        if needs_new_page {
            page_num += 1;
            let (pi, li) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Components");
            let layer = doc.get_page(pi).get_layer(li);

            fill_rect(&layer, 0.0, 0.0, PAGE_W, PAGE_H, BG);
            fill_gradient_h(&layer, 0.0, PAGE_H - 2.5, PAGE_W, 2.5, ACCENT_BLU, ACCENT_PUR, 21);

            set_color(&layer, TEXT_PRI);
            let heading = match project_label {
                Some(name) => format!("Component Catalog — {}", name),
                None => "Component Catalog".to_string(),
            };
            layer.use_text(truncate(&heading, 46), 14.0, Mm(MARGIN), Mm(282.5), &font_b);
            set_color(&layer, TEXT_MUT);
            layer.use_text(
                format!("Page {}", page_num),
                8.0, Mm(PAGE_W - MARGIN - 14.0), Mm(283.0), &font_r,
            );
            draw_hline(&layer, MARGIN, PAGE_W - MARGIN, 277.5, PANEL_BORDER);

            fill_rect(&layer, MARGIN, HDR_Y - 7.5, PAGE_W - 2.0 * MARGIN, 9.5, PANEL_ALT);
            draw_hline(&layer, MARGIN, T_END, HDR_Y - 7.5, PANEL_BORDER);
            set_color(&layer, TEXT_SEC);
            for (i, h) in headers.iter().enumerate() {
                layer.use_text(*h, 6.5, Mm(col_x[i] + 1.5), Mm(HDR_Y - 4.0), &font_b);
            }

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

        let (risk_str, risk_fg, risk_bg) = risk_badge(&dep.risk);

        if row_idx % 2 == 0 {
            fill_rect(&layer, MARGIN, cur_y - row_h + 1.5, PAGE_W - 2.0 * MARGIN, row_h, PANEL_ALT);
        }

        let text_y = cur_y - 4.0;

        set_color(&layer, TEXT_PRI);
        layer.use_text(truncate(&dep.name, 18), 8.0, Mm(col_x[0] + 1.5), Mm(text_y), &font_r);
        set_color(&layer, TEXT_SEC);
        layer.use_text(truncate(&dep.version, 10), 8.0, Mm(col_x[1] + 1.5), Mm(text_y), &font_r);
        layer.use_text(dep.ecosystem.to_string(), 8.0, Mm(col_x[2] + 1.5), Mm(text_y), &font_r);

        // PURL — truncated (PURLs have no spaces so wrap_text would not split them)
        set_color(&layer, TEXT_MUT);
        layer.use_text(purl_trunc.as_str(), 6.5, Mm(col_x[3] + 1.5), Mm(text_y), &font_r);

        // LICENSE — wrapped
        set_color(&layer, TEXT_SEC);
        for (j, line) in license_lines.iter().enumerate() {
            let line_y = text_y - j as f32 * EXTRA_LINE_H;
            layer.use_text(line.as_str(), 8.0, Mm(col_x[4] + 1.5), Mm(line_y), &font_r);
        }

        // Risk badge
        let badge_x = col_x[5] + 1.5;
        let badge_y = cur_y - row_h + 2.2;
        fill_rounded_rect(&layer, badge_x, badge_y, 24.0, 4.8, R_BADGE, risk_bg);
        set_color(&layer, risk_fg);
        layer.use_text(risk_str, 6.5, Mm(badge_x + 2.0), Mm(badge_y + 1.1), &font_b);

        draw_hline(&layer, MARGIN, T_END, cur_y - row_h + 1.5, PANEL_BORDER);
        cur_y -= row_h;
    }

    Ok(())
}

fn risk_badge(risk: &LicenseRisk) -> (&'static str, (f32, f32, f32), (f32, f32, f32)) {
    match risk {
        LicenseRisk::Permissive    => ("PERMISSIVE", PASS_FG, PASS_BG),
        LicenseRisk::WeakCopyleft  => ("WK COPYLEFT", WARN_FG, WARN_BG),
        LicenseRisk::StrongCopyleft => ("ST COPYLEFT", ERR_FG, ERR_BG),
        LicenseRisk::Proprietary   => ("PROPRIETARY", (0.20, 0.34, 0.82), (0.91, 0.93, 1.00)),
        LicenseRisk::Unknown       => ("UNKNOWN", UNK_FG, UNK_BG),
    }
}

fn purl_for_dep(dep: &Dependency) -> String {
    use crate::models::Ecosystem;
    let pkg_type = match dep.ecosystem {
        Ecosystem::Rust   => "cargo",
        Ecosystem::Python => "pypi",
        Ecosystem::Java   => "maven",
        Ecosystem::Node   => "npm",
        Ecosystem::DotNet => "nuget",
    };
    format!("pkg:{}/{}@{}", pkg_type, dep.name, dep.version)
}

// ── Drawing helpers (mirrors pdf.rs) ─────────────────────────────────────────

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

fn rounded_rect_ring(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<(Point, bool)> {
    let r = r.min(w / 2.0).min(h / 2.0);
    const SEGS: usize = 8;
    let mut pts = Vec::with_capacity(4 * (SEGS + 1));
    let corners = [
        (x + w - r, y + r,     270.0f32, 360.0f32),
        (x + w - r, y + h - r, 0.0f32,   90.0f32),
        (x + r,     y + h - r, 90.0f32,  180.0f32),
        (x + r,     y + r,     180.0f32, 270.0f32),
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
        fill_rect(layer, x + i as f32 * step_w, y, step_w + 0.6, h, color);
    }
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
    fill_rect(layer, x, y + h - 2.0, w, 2.0, accent);
    set_color(layer, accent);
    layer.use_text(value, 20.0, Mm(x + 5.0), Mm(y + h * 0.38), font_b);
    set_color(layer, TEXT_MUT);
    layer.use_text(label, 6.5, Mm(x + 5.0), Mm(y + 3.5), font_r);
}

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

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days  = secs / 86400;
    let year  = 1970 + days / 365;
    let doy   = days % 365;
    let month = (doy / 30 + 1).min(12);
    let day   = (doy % 30 + 1).min(31);
    format!("{:04}-{:02}-{:02}", year, month, day)
}
