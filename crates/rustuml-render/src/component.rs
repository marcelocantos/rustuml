// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's component diagram rendering.
//! PlantUML renders component diagrams as diagram type "DESCRIPTION".

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::component::*;

use crate::layout_oracle::{EntityRect, OracleLayout};
use crate::metrics;
use crate::plantuml_metrics::{ascent, fmt_coord};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ---------------------------------------------------------------------------
// PlantUML constants (extracted from golden SVGs)
// ---------------------------------------------------------------------------

/// Font size for component labels and cluster titles (PlantUML default).
const FONT_SIZE: f64 = 14.0;
/// Font size for stereotype text.
const SMALL_FONT: f64 = 14.0;
/// Font size for arrow/link labels.
const LINK_FONT: f64 = 13.0;
/// Line height per text line in a component box (= text_height at 14pt).
const LINE_HEIGHT: f64 = 16.4883;
/// Vertical padding above the first text line inside a component.
const COMPONENT_PAD_TOP: f64 = 20.0;
/// Vertical padding below the last text line inside a component.
const COMPONENT_PAD_BOTTOM: f64 = 10.0;
/// Left padding for text inside a component (accounts for icon space on right).
const TEXT_PAD_LEFT: f64 = 15.0;
/// Right padding inside component (icon area).
const TEXT_PAD_RIGHT: f64 = 25.0;
/// Margin around the entire diagram.
const MARGIN: f64 = 7.0;
/// Gap between entities when laid out by Sugiyama.
const GAP: f64 = 60.0;
/// Fill color for component bodies.
const COMP_FILL: &str = "#F1F1F1";
/// Stroke color for component borders and arrows.
const STROKE: &str = "#181818";
/// Text fill color.
const TEXT_COLOR: &str = "#000000";
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// Radius for rounded corners on component rects.
const ROUND_R: f64 = 2.5;
/// Interface circle radius.
const IFACE_R: f64 = 8.0;
/// Note fold (dog-ear) size.
const NOTE_FOLD: f64 = 10.0;
/// Note padding.
const NOTE_PAD: f64 = 6.0;
/// Note line height.
const NOTE_LINE_H: f64 = 18.0;
/// Note gap from attached element.
const NOTE_GAP: f64 = 10.0;

/// Title font size.
const TITLE_FONT_SIZE: f64 = 14.0;
/// Title height including padding.
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;

/// Container (package) label height.
const CONTAINER_LABEL_H: f64 = 22.0;
/// Container internal padding.
const CONTAINER_PAD: f64 = 16.0;

/// Minimum component width.
const COMPONENT_MIN_W: f64 = 40.0;

// ---------------------------------------------------------------------------
// Component icon geometry (the "tab" icon at top-right of each component)
// ---------------------------------------------------------------------------

/// Width of the component tab icon.
const ICON_TAB_W: f64 = 15.0;
/// Height of the component tab icon.
const ICON_TAB_H: f64 = 10.0;
/// Width of each bar in the component icon.
const ICON_BAR_W: f64 = 4.0;
/// Height of each bar in the component icon.
const ICON_BAR_H: f64 = 2.0;
/// Offset from right edge of component rect to left edge of tab.
const ICON_TAB_RIGHT_OFFSET: f64 = 20.0;
/// Offset from top of component rect to top of tab.
const ICON_TAB_TOP_OFFSET: f64 = 5.0;
/// Offset from left edge of tab to left edge of bars.
const ICON_BAR_LEFT_OFFSET: f64 = 2.0;
/// Vertical offset from top of tab to first bar.
const ICON_BAR_TOP_OFFSET_1: f64 = 2.0;
/// Vertical offset from top of tab to second bar.
const ICON_BAR_TOP_OFFSET_2: f64 = 6.0;

// ---------------------------------------------------------------------------
// Text-width helpers (PlantUML-exact Java AWT metrics)
// ---------------------------------------------------------------------------

/// PlantUML guillemet width at 14pt (scaled from the 12pt value, matching
/// PlantUML's JVM output exactly; metrics.rs `GUILLEMET_LEFT_WIDTH_14`
/// constant is incorrect at the time of writing).
const GUILLEMET_W_14: f64 = 7.33496_09375;

/// PlantUML text width at 14pt (the most common size for component labels).
/// Overrides guillemet handling because `metrics::plantuml_text_width_14`
/// uses a fallback width of 8.0 for non-ASCII chars.
fn tw14(s: &str) -> f64 {
    let mut total = 0.0_f64;
    for c in s.chars() {
        total += match c {
            '\u{00AB}' | '\u{00BB}' => GUILLEMET_W_14,
            c if (c as u32) < 128 => metrics::plantuml_text_width_14(&c.to_string()),
            _ => metrics::plantuml_text_width_14(&c.to_string()),
        };
    }
    total
}

/// PlantUML text width at arbitrary font size (scaled from 14pt baseline).
fn tw(s: &str, font_size: f64) -> f64 {
    tw14(s) * font_size / 14.0
}

/// Format a numeric SVG coordinate value (4 decimals, trailing zeros stripped).
fn n(v: f64) -> String {
    fmt_coord(v)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn render(diagram: &ComponentDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a component diagram to SVG, optionally using pre-computed layout from an oracle.
pub fn render_with_oracle(
    diagram: &ComponentDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if diagram.components.is_empty() && diagram.packages.is_empty() && diagram.interfaces.is_empty()
    {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="DESCRIPTION" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><defs/><g></g></svg>"#.to_string();
    }

    // Build a lookup mapping component id → qualified name (e.g. "G1.AA")
    // by walking the package tree.
    let qualified_names = build_qualified_names(diagram);

    // Compute dimensions for each component (used as fallback when oracle has no rect).
    let comp_dims: Vec<CompDim> = diagram.components.iter().map(calc_component_dim).collect();

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    let use_oracle = oracle.is_some();

    // Try Sugiyama layout (skip when oracle is available).
    let layout_result = if use_oracle {
        None
    } else if !diagram.components.is_empty() || !diagram.interfaces.is_empty() {
        let mut layout = LayoutGraph::new(Direction::TopToBottom);
        for (comp, dim) in diagram.components.iter().zip(&comp_dims) {
            layout.add_node(&comp.id, &comp.label, dim.width, dim.height);
        }
        for iface in &diagram.interfaces {
            layout.add_node(
                &iface.id,
                &iface.label,
                IFACE_R * 2.0 + 20.0,
                IFACE_R * 2.0 + 20.0,
            );
        }
        for conn in &diagram.connections {
            layout.add_edge(&conn.from, &conn.to, conn.label.as_deref());
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    } else {
        None
    };

    let n_comp = diagram.components.len();

    // Compute positions from oracle, layout engine, or grid fallback.
    let (positions, iface_positions, content_w, content_h) = if let Some(orc) = oracle {
        compute_positions_from_oracle(diagram, &comp_dims, &qualified_names, orc, title_h)
    } else if let Some(ref result) = layout_result
        && result.node_positions.len() >= n_comp + diagram.interfaces.len()
    {
        compute_positions_from_layout(diagram, &comp_dims, &result.node_positions, title_h)
    } else {
        compute_positions_grid(diagram, &comp_dims, title_h)
    };

    let empty_edge_paths: Vec<EdgePath> = Vec::new();
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };

    // Estimate package bounding box.
    let pkg_total_w = estimate_packages_width(&diagram.packages);
    let pkg_total_h = estimate_packages_height(&diagram.packages);

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else {
        (
            content_w.max(pkg_total_w).max(100.0),
            (content_h + pkg_total_h + title_h).max(50.0),
        )
    };

    let mut svg = SvgBuilder::new_plantuml(total_w, total_h, "DESCRIPTION");

    // Title.
    if let Some(title) = &diagram.meta.title {
        for (i, tline) in title.lines().enumerate() {
            let ty = TITLE_HEIGHT - 4.0 + i as f64 * (TITLE_FONT_SIZE + 2.0);
            svg.text(total_w / 2.0, ty, tline, "middle", TITLE_FONT_SIZE);
        }
    }

    // Header.
    if let Some(header) = &diagram.meta.header {
        svg.text(
            total_w / 2.0,
            SMALL_FONT + 2.0,
            header,
            "middle",
            SMALL_FONT,
        );
    }

    // Render packages (clusters).
    let mut pkg_y = title_h + MARGIN;
    render_packages(&diagram.packages, &mut svg, MARGIN, &mut pkg_y, theme);

    // Determine starting entity counter.  PlantUML's entity IDs start at
    // ent0002.  When clusters are present, they consume ent IDs ahead of
    // their contained components — but our parser doesn't track cluster
    // declaration order vs component declaration order well enough to
    // reproduce this perfectly.  When the oracle is available, we prefer
    // its qualified-name → rect mapping for positions; for now, components
    // get sequential IDs starting at 2.
    let mut entity_counter: usize = 2;

    // PlantUML's `componentStyle rectangle` skinparam suppresses the UML2
    // tab+bars icon and uses a smaller text padding (10 instead of 15/25).
    let rectangle_style = theme.component.style.eq_ignore_ascii_case("rectangle");

    // Render each component entity.
    for (i, comp) in diagram.components.iter().enumerate() {
        let (x, y) = positions[i];

        // Prefer the oracle's exact rect width/height if available.
        let qual = qualified_names
            .get(&comp.id)
            .cloned()
            .unwrap_or_else(|| comp.id.clone());
        let oracle_rect = oracle.and_then(|orc| {
            orc.entities
                .get(&qual)
                .or_else(|| orc.entities.get(&comp.id))
                .or_else(|| orc.entities.get(&comp.label))
        });

        let dim = &comp_dims[i];
        let comp_w = oracle_rect.map(|r| r.width).unwrap_or(dim.width);
        let comp_h = oracle_rect.map(|r| r.height).unwrap_or(dim.height);

        let ent_id = format!("ent{entity_counter:04}");
        entity_counter += 1;

        emit_component(
            &mut svg,
            comp,
            &qual,
            &ent_id,
            x,
            y,
            comp_w,
            comp_h,
            oracle_rect,
            rectangle_style,
        );
    }

    // Render interfaces.
    for (ii, iface) in diagram.interfaces.iter().enumerate() {
        let (ix, iy) = iface_positions[ii];
        let ent_id = format!("ent{entity_counter:04}");
        entity_counter += 1;

        svg.raw(&format!("<!--entity {}-->", iface.id));
        svg.raw(&format!(
            r#"<g class="entity" data-qualified-name="{}" id="{ent_id}">"#,
            iface.id
        ));

        // Circle.
        svg.raw(&format!(
            r#"<ellipse cx="{}" cy="{}" fill="{COMP_FILL}" rx="{IFACE_R}" ry="{IFACE_R}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
            n(ix),
            n(iy),
        ));

        // Label below.
        let label_y = iy + IFACE_R + LINE_HEIGHT + 4.0;
        let tl = tw14(&iface.label);
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            n(tl),
            n(ix - tl / 2.0),
            n(label_y),
            escape_xml(&iface.label),
        ));

        svg.raw("</g>");
    }

    // Render connections (links).
    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc);
    } else {
        for (link_counter, conn) in (entity_counter..).zip(diagram.connections.iter()) {
            let link_id = format!("lnk{link_counter}");
            render_connection_fallback(
                &mut svg,
                conn,
                diagram,
                &positions,
                &iface_positions,
                &comp_dims,
                edge_paths,
                n_comp,
                &link_id,
            );
        }
    }

    // Render notes.
    for note in &diagram.notes {
        render_note(
            note,
            &positions,
            &comp_dims,
            &diagram.components,
            &mut svg,
            total_w,
            total_h,
        );
    }

    // Footer.
    if let Some(footer) = &diagram.meta.footer {
        svg.text(total_w / 2.0, total_h - 4.0, footer, "middle", SMALL_FONT);
    }
    // Legend.
    if let Some(legend) = &diagram.meta.legend {
        svg.render_legend(MARGIN, total_h / 2.0, legend, SMALL_FONT);
    }

    svg.finalize_plantuml()
}

// ---------------------------------------------------------------------------
// Qualified-name resolution (component id → "Package.id" or "id")
// ---------------------------------------------------------------------------

fn build_qualified_names(diagram: &ComponentDiagram) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for comp in &diagram.components {
        // Default: bare id (top-level component).
        map.insert(comp.id.clone(), comp.id.clone());
    }
    for pkg in &diagram.packages {
        walk_pkg(pkg, "", &mut map);
    }
    map
}

fn walk_pkg(
    pkg: &ComponentPackage,
    parent_prefix: &str,
    map: &mut std::collections::HashMap<String, String>,
) {
    let prefix = if parent_prefix.is_empty() {
        pkg.name.clone()
    } else {
        format!("{parent_prefix}.{}", pkg.name)
    };
    for comp_id in &pkg.components {
        map.insert(comp_id.clone(), format!("{prefix}.{comp_id}"));
    }
    for sub in &pkg.packages {
        walk_pkg(sub, &prefix, map);
    }
}

// ---------------------------------------------------------------------------
// Component rendering
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn emit_component(
    svg: &mut SvgBuilder,
    comp: &Component,
    qualified: &str,
    ent_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    oracle_rect: Option<&EntityRect>,
    rectangle_style: bool,
) {
    let fill = COMP_FILL;

    // HTML comment.
    svg.raw(&format!("<!--entity {}-->", comp.id));

    // Open entity group with data-source-line.
    let source_attr = if comp.source_line > 0 {
        format!(r#" data-source-line="{}""#, comp.source_line)
    } else {
        String::new()
    };
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qualified}"{source_attr} id="{ent_id}">"#
    ));

    if let Some(ref url) = comp.url {
        svg.open_link(url);
    }

    // Main body rectangle.
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{ROUND_R}" ry="{ROUND_R}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{xs}" y="{ys}"/>"#,
        h = n(height),
        w = n(width),
        xs = n(x),
        ys = n(y),
    ));

    // Component UML2 icon (tab + 2 bars) at top-right.  Suppressed under
    // `skinparam componentStyle rectangle`.
    if !rectangle_style {
        let tab_x = x + width - ICON_TAB_RIGHT_OFFSET;
        let tab_y = y + ICON_TAB_TOP_OFFSET;
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{ICON_TAB_H}" style="stroke:{STROKE};stroke-width:0.5;" width="{ICON_TAB_W}" x="{}" y="{}"/>"#,
            n(tab_x),
            n(tab_y),
        ));

        let bar_x = tab_x - ICON_BAR_LEFT_OFFSET;
        let bar_y1 = tab_y + ICON_BAR_TOP_OFFSET_1;
        let bar_y2 = tab_y + ICON_BAR_TOP_OFFSET_2;
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{ICON_BAR_H}" style="stroke:{STROKE};stroke-width:0.5;" width="{ICON_BAR_W}" x="{}" y="{}"/>"#,
            n(bar_x),
            n(bar_y1),
        ));
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{ICON_BAR_H}" style="stroke:{STROKE};stroke-width:0.5;" width="{ICON_BAR_W}" x="{}" y="{}"/>"#,
            n(bar_x),
            n(bar_y2),
        ));
    }

    // Text lines: stereotypes (italic) above the label.
    // Each text line is centered horizontally in the available text area.
    // In rectangle style the padding is symmetric (10 each side); with the
    // UML2 icon the right padding includes the icon (15 left + 25 right).
    let (text_pad_left, text_pad_right) = if rectangle_style {
        (10.0, 10.0)
    } else {
        (TEXT_PAD_LEFT, TEXT_PAD_RIGHT)
    };
    let available_w = width - text_pad_left - text_pad_right;
    let text_area_left = x + text_pad_left;

    // First text baseline: pad_top above + ascent of font.
    // For 14pt: 20 + 13.5352 = 33.5352 → y_first = comp_y + 33.5352.
    // Rectangle style has smaller padding: pad_top = 10 → y = comp_y + 23.5352.
    let pad_top = if rectangle_style {
        10.0
    } else {
        COMPONENT_PAD_TOP
    };
    let first_text_y = y + pad_top + ascent(FONT_SIZE);

    // Oracle override: if oracle has text_y_values, prefer those.
    let oracle_text_ys: Option<&[f64]> = oracle_rect.map(|r| r.text_y_values.as_slice());

    // Emit stereotypes (each italic with «...» guillemets).
    for (si, stereo) in comp.stereotypes.iter().enumerate() {
        let ty = oracle_text_ys
            .and_then(|ys| ys.get(si).copied())
            .unwrap_or(first_text_y + si as f64 * LINE_HEIGHT);
        let label = format!("\u{00AB}{stereo}\u{00BB}");
        let tl = tw14(&label);
        let tx = text_area_left + (available_w - tl) / 2.0;
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" font-style="italic" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            n(tl),
            n(tx),
            n(ty),
            escape_xml(&label),
        ));
    }

    // Label (last line).
    let label_idx = comp.stereotypes.len();
    let label_y = oracle_text_ys
        .and_then(|ys| ys.get(label_idx).copied())
        .unwrap_or(first_text_y + label_idx as f64 * LINE_HEIGHT);
    let label_tl = tw14(&comp.label);
    let label_tx = text_area_left + (available_w - label_tl) / 2.0;
    svg.raw(&format!(
        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
        n(label_tl),
        n(label_tx),
        n(label_y),
        escape_xml(&comp.label),
    ));

    if comp.url.is_some() {
        svg.close_link();
    }

    svg.raw("</g>");
}

// ---------------------------------------------------------------------------
// Fallback connection rendering (no-oracle path)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_connection_fallback(
    svg: &mut SvgBuilder,
    conn: &Connection,
    diagram: &ComponentDiagram,
    positions: &[(f64, f64)],
    iface_positions: &[(f64, f64)],
    comp_dims: &[CompDim],
    edge_paths: &[EdgePath],
    n_comp: usize,
    link_id: &str,
) {
    // Find source and target positions.
    let from_comp = diagram
        .components
        .iter()
        .enumerate()
        .find(|(_, c)| c.id == conn.from);
    let to_comp = diagram
        .components
        .iter()
        .enumerate()
        .find(|(_, c)| c.id == conn.to);
    let from_iface = diagram
        .interfaces
        .iter()
        .enumerate()
        .find(|(_, i)| i.id == conn.from);
    let to_iface = diagram
        .interfaces
        .iter()
        .enumerate()
        .find(|(_, i)| i.id == conn.to);

    let (from_cx, from_cy, from_bottom) = if let Some((i, _)) = from_comp {
        let (x, y) = positions[i];
        let dim = &comp_dims[i];
        (x + dim.width / 2.0, y + dim.height, y + dim.height)
    } else if let Some((i, _)) = from_iface {
        let (ix, iy) = iface_positions[i];
        (ix, iy, iy + IFACE_R)
    } else {
        return;
    };

    let (to_cx, to_cy, _to_top) = if let Some((i, _)) = to_comp {
        let (x, y) = positions[i];
        let dim = &comp_dims[i];
        (x + dim.width / 2.0, y, y)
    } else if let Some((i, _)) = to_iface {
        let (ix, iy) = iface_positions[i];
        (ix, iy, iy - IFACE_R)
    } else {
        return;
    };

    let link_type = "dependency";
    let dash_attr = if conn.dashed {
        "stroke-dasharray:7,7;"
    } else {
        ""
    };

    // Try bezier path from layout engine first.
    let edge_path = edge_paths
        .iter()
        .find(|ep| ep.from == conn.from && ep.to == conn.to);

    svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));

    let from_ent_idx = diagram
        .components
        .iter()
        .position(|c| c.id == conn.from)
        .map(|i| i + 2)
        .or_else(|| {
            diagram
                .interfaces
                .iter()
                .position(|i| i.id == conn.from)
                .map(|i| i + 2 + n_comp)
        });
    let to_ent_idx = diagram
        .components
        .iter()
        .position(|c| c.id == conn.to)
        .map(|i| i + 2)
        .or_else(|| {
            diagram
                .interfaces
                .iter()
                .position(|i| i.id == conn.to)
                .map(|i| i + 2 + n_comp)
        });

    let from_ent_id = from_ent_idx
        .map(|i| format!("ent{i:04}"))
        .unwrap_or_default();
    let to_ent_id = to_ent_idx.map(|i| format!("ent{i:04}")).unwrap_or_default();

    svg.raw(&format!(
        r#"<g class="link" data-entity-1="{from_ent_id}" data-entity-2="{to_ent_id}" data-link-type="{link_type}" id="{link_id}">"#,
    ));

    if let Some(ep) = edge_path
        && !ep.points.is_empty()
    {
        let path_d = build_path_d(&ep.points);
        let path_id = format!("{}-to-{}", conn.from, conn.to);
        svg.raw(&format!(
            r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
        ));

        let last = ep.points.last().unwrap();
        let prev = if ep.points.len() >= 2 {
            &ep.points[ep.points.len() - 2]
        } else {
            last
        };
        render_arrowhead(svg, prev, last);

        let first = ep.points.first().unwrap();
        if let Some(label) = &conn.label {
            let mx = (first.0 + last.0) / 2.0;
            let my = (first.1 + last.1) / 2.0;
            let tl = tw(label, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(mx + 1.0),
                n(my - 4.0),
                escape_xml(label),
            ));
        }
        if let Some(from_mult) = &conn.from_mult {
            let tl = tw(from_mult, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(first.0 - tl - 1.0),
                n(first.1 + LINK_FONT + 2.0),
                escape_xml(from_mult),
            ));
        }
        if let Some(to_mult) = &conn.to_mult {
            let tl = tw(to_mult, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(last.0 - tl - 1.0),
                n(last.1 - 4.0),
                escape_xml(to_mult),
            ));
        }
    } else {
        // Straight line fallback.
        let path_d = format!(
            "M {},{} C {},{} {},{} {},{}",
            n(from_cx),
            n(from_cy),
            n(from_cx),
            n(from_cy + (to_cy - from_cy) * 0.3),
            n(to_cx),
            n(from_cy + (to_cy - from_cy) * 0.7),
            n(to_cx),
            n(to_cy),
        );
        let path_id = format!("{}-to-{}", conn.from, conn.to);
        svg.raw(&format!(
            r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
        ));

        render_arrowhead_from_coords(svg, from_cx, from_bottom, to_cx, to_cy);

        if let Some(label) = &conn.label {
            let mx = (from_cx + to_cx) / 2.0;
            let my = (from_cy + to_cy) / 2.0;
            let tl = tw(label, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(mx + 1.0),
                n(my - 4.0),
                escape_xml(label),
            ));
        }
        if let Some(from_mult) = &conn.from_mult {
            let tl = tw(from_mult, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(from_cx - tl - 1.0),
                n(from_cy + LINK_FONT + 2.0),
                escape_xml(from_mult),
            ));
        }
        if let Some(to_mult) = &conn.to_mult {
            let tl = tw(to_mult, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(to_cx - tl - 1.0),
                n(to_cy - 4.0),
                escape_xml(to_mult),
            ));
        }
    }

    svg.raw("</g>");
}

// ---------------------------------------------------------------------------
// Component dimension calculation (fallback when no oracle)
// ---------------------------------------------------------------------------

struct CompDim {
    width: f64,
    height: f64,
}

fn calc_component_dim(comp: &Component) -> CompDim {
    let n_lines = 1 + comp.stereotypes.len();
    let height = COMPONENT_PAD_TOP + COMPONENT_PAD_BOTTOM + n_lines as f64 * LINE_HEIGHT;

    let label_w = tw14(&comp.label);
    let max_stereo_w = comp
        .stereotypes
        .iter()
        .map(|s| tw14(&format!("\u{00AB}{s}\u{00BB}")))
        .fold(0.0_f64, f64::max);
    let text_w = label_w.max(max_stereo_w);
    let width = (text_w + TEXT_PAD_LEFT + TEXT_PAD_RIGHT).max(COMPONENT_MIN_W);

    CompDim { width, height }
}

// ---------------------------------------------------------------------------
// Layout positioning
// ---------------------------------------------------------------------------

/// Positions of components and interfaces, plus content width and height.
type LayoutResult = (Vec<(f64, f64)>, Vec<(f64, f64)>, f64, f64);

#[allow(clippy::type_complexity)]
fn compute_positions_from_layout(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    node_positions: &[rustuml_layout::graph::NodePosition],
    title_h: f64,
) -> LayoutResult {
    let n_comp = diagram.components.len();
    let mut positions = Vec::with_capacity(n_comp);
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    for (i, _comp) in diagram.components.iter().enumerate() {
        let p = &node_positions[i];
        positions.push((p.x + MARGIN, p.y + MARGIN + title_h));
    }
    for (i, _iface) in diagram.interfaces.iter().enumerate() {
        let p = &node_positions[n_comp + i];
        iface_positions.push((p.x + MARGIN + IFACE_R, p.y + MARGIN + title_h + IFACE_R));
    }

    let max_x = node_positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.x + if i < n_comp {
                comp_dims[i].width
            } else {
                IFACE_R * 2.0 + 20.0
            }
        })
        .fold(0.0_f64, f64::max);
    let max_y = node_positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.y + if i < n_comp {
                comp_dims[i].height
            } else {
                IFACE_R * 2.0 + 20.0
            }
        })
        .fold(0.0_f64, f64::max);

    let content_w = max_x + MARGIN * 2.0;
    let content_h = max_y + MARGIN * 2.0 + title_h;

    (positions, iface_positions, content_w, content_h)
}

fn compute_positions_from_oracle(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    qualified_names: &std::collections::HashMap<String, String>,
    oracle: &OracleLayout,
    title_h: f64,
) -> LayoutResult {
    let mut positions = Vec::with_capacity(diagram.components.len());
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    for (i, comp) in diagram.components.iter().enumerate() {
        let qual = qualified_names
            .get(&comp.id)
            .cloned()
            .unwrap_or_else(|| comp.id.clone());
        let rect = oracle
            .entities
            .get(&qual)
            .or_else(|| oracle.entities.get(&comp.id))
            .or_else(|| oracle.entities.get(&comp.label));
        if let Some(rect) = rect {
            positions.push((rect.x, rect.y));
        } else {
            let dim = &comp_dims[i];
            positions.push((MARGIN + (i as f64) * (dim.width + GAP), MARGIN + title_h));
        }
    }

    for iface in &diagram.interfaces {
        let rect = oracle.entities.get(&iface.id);
        if let Some(rect) = rect {
            iface_positions.push((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0));
        } else {
            iface_positions.push((MARGIN + 50.0, MARGIN + title_h + 50.0));
        }
    }

    let content_w = if oracle.canvas_width > 0.0 {
        oracle.canvas_width
    } else {
        positions
            .iter()
            .enumerate()
            .map(|(i, (x, _))| x + comp_dims[i].width + MARGIN)
            .fold(100.0_f64, f64::max)
    };
    let content_h = if oracle.canvas_height > 0.0 {
        oracle.canvas_height
    } else {
        positions
            .iter()
            .enumerate()
            .map(|(i, (_, y))| y + comp_dims[i].height + MARGIN)
            .fold(50.0_f64, f64::max)
    };

    (positions, iface_positions, content_w, content_h)
}

fn compute_positions_grid(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    title_h: f64,
) -> LayoutResult {
    let n = diagram.components.len();
    let cols = if n == 0 {
        1
    } else {
        (n as f64).sqrt().ceil() as usize
    };

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, dim) in comp_dims.iter().enumerate() {
            cw[i % cols] = cw[i % cols].max(dim.width);
        }
        cw
    };
    let rows = if n == 0 { 0 } else { n.div_ceil(cols) };

    let single_h = COMPONENT_PAD_TOP + COMPONENT_PAD_BOTTOM + LINE_HEIGHT;

    let mut positions = Vec::with_capacity(n);
    let y_start = title_h + MARGIN;
    for (i, _comp) in diagram.components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (single_h + GAP);
        positions.push((x, y));
    }

    let comp_total_w = if n > 0 {
        MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64
    } else {
        0.0
    };
    let comp_total_h = if n > 0 {
        rows as f64 * (single_h + GAP)
    } else {
        0.0
    };

    let iface_y_start = y_start + comp_total_h;
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());
    for (ii, _iface) in diagram.interfaces.iter().enumerate() {
        let ix = MARGIN + ii as f64 * (IFACE_R * 2.0 + GAP) + IFACE_R;
        let iy = iface_y_start + IFACE_R;
        iface_positions.push((ix, iy));
    }

    let iface_total_h = if !diagram.interfaces.is_empty() {
        IFACE_R * 2.0 + 20.0 + GAP
    } else {
        0.0
    };
    let iface_total_w = if !diagram.interfaces.is_empty() {
        MARGIN * 2.0 + diagram.interfaces.len() as f64 * (IFACE_R * 2.0 + GAP)
    } else {
        0.0
    };

    let content_w = comp_total_w.max(iface_total_w).max(100.0);
    let content_h = comp_total_h + iface_total_h + title_h;

    (positions, iface_positions, content_w, content_h)
}

// ---------------------------------------------------------------------------
// Connection rendering (oracle path)
// ---------------------------------------------------------------------------

fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &ComponentDiagram,
    oracle: &OracleLayout,
) {
    for conn in &diagram.connections {
        // PlantUML uses different edge id patterns depending on direction
        // and arrow style:
        //   - `A-to-B`     : dependency arrow A→B (down/right direction)
        //   - `B-backto-A` : dependency arrow A→B reversed for up/left
        //   - `A-B` / `B-A`: association (no arrow) — direction varies
        let candidates = [
            format!("{}-to-{}", conn.from, conn.to),
            format!("{}-backto-{}", conn.to, conn.from),
            format!("{}-{}", conn.from, conn.to),
            format!("{}-{}", conn.to, conn.from),
        ];
        let (matched_id, oracle_edge) = match candidates
            .iter()
            .find_map(|id| oracle.edges.iter().find(|e| &e.id == id).map(|e| (id, e)))
        {
            Some((id, e)) => (id.clone(), e),
            None => continue,
        };

        svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));

        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("dependency");
        let source_line = oracle_edge.source_line.as_deref();
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");

        let source_attr = source_line
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();

        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}"{source_attr} id="{link_id}">"#,
        ));

        let path_style = oracle_edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let code_line_attr = oracle_edge
            .code_line
            .as_ref()
            .map(|c| format!(r#" codeLine="{c}""#))
            .unwrap_or_default();
        svg.raw(&format!(
            r#"<path{code_line_attr} d="{}" fill="none" id="{matched_id}" style="{path_style}"/>"#,
            oracle_edge.d,
        ));

        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }

        // Render the connection label, if any.  Position it at the midpoint
        // of the path's first and last endpoints, biased slightly to match
        // PlantUML output (+1 px right of midpoint, +4 px above midpoint).
        if let Some(label) = &conn.label
            && let Some((first, last)) = path_endpoints(&oracle_edge.d)
        {
            let mx = (first.0 + last.0) / 2.0;
            let my = (first.1 + last.1) / 2.0;
            let tl = tw(label, LINK_FONT);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(tl),
                n(mx + 1.0),
                n(my + 5.0),
                escape_xml(label),
            ));
        }

        svg.raw("</g>");
    }
}

/// Extract first and last (x, y) points from an SVG path `d` attribute.
/// Supports the simple "M x,y ... x,y" patterns PlantUML emits.
fn path_endpoints(d: &str) -> Option<((f64, f64), (f64, f64))> {
    // Replace command letters with spaces and collect numeric tokens.
    let cleaned: String = d
        .chars()
        .map(|c| if c.is_ascii_alphabetic() { ' ' } else { c })
        .collect();
    let nums: Vec<f64> = cleaned
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() >= 4 {
        Some((
            (nums[0], nums[1]),
            (nums[nums.len() - 2], nums[nums.len() - 1]),
        ))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Arrowhead rendering (non-oracle path)
// ---------------------------------------------------------------------------

fn render_arrowhead(svg: &mut SvgBuilder, prev: &(f64, f64), tip: &(f64, f64)) {
    let dx = tip.0 - prev.0;
    let dy = tip.1 - prev.1;
    let angle = dy.atan2(dx);
    render_arrow_at(svg, tip.0, tip.1, angle);
}

fn render_arrowhead_from_coords(svg: &mut SvgBuilder, _fx: f64, _fy: f64, tx: f64, ty: f64) {
    let size = 5.0;
    let pts = format!(
        "{},{},{},{},{},{},{},{},{},{}",
        n(tx),
        n(ty),
        n(tx + size),
        n(ty - size * 2.0),
        n(tx),
        n(ty - size * 1.5),
        n(tx - size),
        n(ty - size * 2.0),
        n(tx),
        n(ty),
    );
    svg.raw(&format!(
        r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
    ));
}

fn render_arrow_at(svg: &mut SvgBuilder, x: f64, y: f64, angle: f64) {
    let size = 5.0;
    let spread = 0.5;
    let x1 = x - size * 2.0 * (angle - spread).cos();
    let y1 = y - size * 2.0 * (angle - spread).sin();
    let x2 = x - size * 1.5 * angle.cos();
    let y2 = y - size * 1.5 * angle.sin();
    let x3 = x - size * 2.0 * (angle + spread).cos();
    let y3 = y - size * 2.0 * (angle + spread).sin();
    let pts = format!(
        "{},{},{},{},{},{},{},{},{},{}",
        n(x),
        n(y),
        n(x1),
        n(y1),
        n(x2),
        n(y2),
        n(x3),
        n(y3),
        n(x),
        n(y),
    );
    svg.raw(&format!(
        r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
    ));
}

// ---------------------------------------------------------------------------
// Path building
// ---------------------------------------------------------------------------

fn build_path_d(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut d = String::new();
    let (x0, y0) = points[0];
    write!(d, "M {},{}", n(x0), n(y0)).unwrap();
    if points.len() >= 4 {
        let mut i = 1;
        while i + 2 < points.len() {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];
            let (x3, y3) = points[i + 2];
            write!(
                d,
                " C {},{} {},{} {},{}",
                n(x1),
                n(y1),
                n(x2),
                n(y2),
                n(x3),
                n(y3)
            )
            .unwrap();
            i += 3;
        }
    } else {
        for &(x, y) in &points[1..] {
            write!(d, " L {},{}", n(x), n(y)).unwrap();
        }
    }
    d
}

// ---------------------------------------------------------------------------
// Note rendering
// ---------------------------------------------------------------------------

fn render_note(
    note: &ComponentNote,
    positions: &[(f64, f64)],
    comp_dims: &[CompDim],
    components: &[Component],
    svg: &mut SvgBuilder,
    canvas_w: f64,
    canvas_h: f64,
) {
    let lines: Vec<&str> = note.text.lines().collect();
    let note_w = lines
        .iter()
        .map(|l| tw(l, LINK_FONT) + NOTE_PAD * 2.0)
        .fold(60.0_f64, f64::max);
    let note_h = (lines.len() as f64).max(1.0) * NOTE_LINE_H + NOTE_PAD * 2.0;

    let (nx, ny) = if let Some(target) = &note.target {
        if let Some(idx) = components.iter().position(|c| c.id == *target) {
            let (ox, oy) = positions[idx];
            let ow = comp_dims[idx].width;
            let oh = comp_dims[idx].height;
            (ox + ow + NOTE_GAP, oy + oh / 2.0 - note_h / 2.0)
        } else {
            (
                canvas_w - note_w - NOTE_GAP * 2.0,
                canvas_h / 2.0 - note_h / 2.0,
            )
        }
    } else {
        (NOTE_GAP, canvas_h - note_h - NOTE_GAP)
    };

    let nx = nx.max(NOTE_GAP);
    let ny = ny.max(NOTE_GAP);

    svg.note_box(nx, ny, note_w, note_h, NOTE_FOLD, NOTE_FILL, STROKE);

    for (i, line) in lines.iter().enumerate() {
        let ty = ny + NOTE_PAD + (i as f64 + 1.0) * NOTE_LINE_H - 2.0;
        let tl = tw(line, LINK_FONT);
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            n(tl),
            n(nx + NOTE_PAD),
            n(ty),
            escape_xml(line),
        ));
    }
}

// ---------------------------------------------------------------------------
// Package / container rendering
// ---------------------------------------------------------------------------

#[allow(clippy::only_used_in_recursion)]
fn render_packages(
    packages: &[ComponentPackage],
    svg: &mut SvgBuilder,
    x: f64,
    y: &mut f64,
    theme: &Theme,
) {
    let single_h = COMPONENT_PAD_TOP + COMPONENT_PAD_BOTTOM + LINE_HEIGHT;

    for pkg in packages {
        let name_w = tw14(&pkg.label) + 20.0;
        let stereo_w = pkg
            .stereotype
            .as_deref()
            .map(|s| tw14(&format!("\u{00AB}{s}\u{00BB}")) + 20.0)
            .unwrap_or(0.0);
        let pkg_label_w = name_w.max(stereo_w).max(COMPONENT_MIN_W);
        let inner_w = estimate_package_inner_width(pkg).max(pkg_label_w);
        let pkg_w = inner_w + CONTAINER_PAD * 2.0;

        let pkg_y_start = *y;
        let label_y = pkg_y_start + CONTAINER_LABEL_H;
        *y = label_y + CONTAINER_PAD;

        if !pkg.packages.is_empty() {
            render_packages(&pkg.packages, svg, x + CONTAINER_PAD, y, theme);
        }

        let leaf_count = pkg.components.len();
        if leaf_count > 0 {
            *y += leaf_count as f64 * (single_h + GAP);
        }

        let pkg_inner_h = (*y - label_y - CONTAINER_PAD).max(single_h);
        let pkg_h = pkg_inner_h + CONTAINER_PAD * 2.0 + CONTAINER_LABEL_H;

        // Package container rectangle.
        svg.raw(&format!(
            r#"<rect fill="none" height="{}" style="stroke:{STROKE};stroke-width:1.5;" width="{}" x="{}" y="{}"/>"#,
            n(pkg_h), n(pkg_w), n(x), n(pkg_y_start),
        ));

        // Label.
        let label_tl = tw14(&pkg.label);
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            n(label_tl),
            n(x + CONTAINER_PAD),
            n(pkg_y_start + CONTAINER_LABEL_H - 4.0),
            escape_xml(&pkg.label),
        ));

        if let Some(stereo) = &pkg.stereotype {
            let label = format!("\u{00AB}{stereo}\u{00BB}");
            let stereo_tl = tw14(&label);
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" font-style="italic" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                n(stereo_tl),
                n(x + CONTAINER_PAD),
                n(pkg_y_start + CONTAINER_LABEL_H + 12.0),
                escape_xml(&label),
            ));
        }

        *y = pkg_y_start + pkg_h + GAP;
    }
}

fn estimate_packages_width(packages: &[ComponentPackage]) -> f64 {
    if packages.is_empty() {
        return 0.0;
    }
    packages
        .iter()
        .map(estimate_package_width)
        .fold(0.0_f64, f64::max)
        + MARGIN * 2.0
}

fn estimate_packages_height(packages: &[ComponentPackage]) -> f64 {
    if packages.is_empty() {
        return 0.0;
    }
    packages.iter().map(estimate_package_height).sum::<f64>()
        + MARGIN * 2.0
        + GAP * packages.len().saturating_sub(1) as f64
}

fn estimate_package_width(pkg: &ComponentPackage) -> f64 {
    let name_w = tw14(&pkg.label) + 20.0;
    let stereo_w = pkg
        .stereotype
        .as_deref()
        .map(|s| tw14(&format!("\u{00AB}{s}\u{00BB}")) + 20.0)
        .unwrap_or(0.0);
    let label_w = name_w.max(stereo_w).max(COMPONENT_MIN_W);
    let inner_w = estimate_package_inner_width(pkg);
    (label_w.max(inner_w) + CONTAINER_PAD * 2.0).max(COMPONENT_MIN_W)
}

fn estimate_package_inner_width(pkg: &ComponentPackage) -> f64 {
    let nested_w = pkg
        .packages
        .iter()
        .map(estimate_package_width)
        .fold(0.0_f64, f64::max);
    let leaf_w = if pkg.components.is_empty() {
        0.0
    } else {
        COMPONENT_MIN_W
    };
    nested_w.max(leaf_w)
}

fn estimate_package_height(pkg: &ComponentPackage) -> f64 {
    let single_h = COMPONENT_PAD_TOP + COMPONENT_PAD_BOTTOM + LINE_HEIGHT;
    let nested_h: f64 = pkg.packages.iter().map(estimate_package_height).sum();
    let leaf_h = pkg.components.len() as f64 * (single_h + GAP);
    CONTAINER_LABEL_H + CONTAINER_PAD * 2.0 + nested_h + leaf_h
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('\u{00AB}', "&#171;")
        .replace('\u{00BB}', "&#187;")
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\ncomponent \"Web\" as WS\ncomponent \"DB\" as DB\nWS --> DB : query\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Web"));
        assert!(svg.contains("DB"));
        assert!(svg.contains("query"));
    }

    #[test]
    fn nested_container_labels_rendered() {
        let input = "@startuml\ncloud Outer #LightBlue {\n  folder Inner {\n    component X\n    component Y\n    X --> Y\n  }\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Outer"), "expected 'Outer' in SVG, got: {svg}");
        assert!(svg.contains("Inner"), "expected 'Inner' in SVG, got: {svg}");
    }

    #[test]
    fn note_text_rendered() {
        let input = "@startuml\ncomponent MyComp <<facade>> #LightBlue\nnote right of MyComp : Tagged component\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("Tagged component"),
            "note text missing in SVG: {svg}"
        );
        assert!(
            svg.contains("MyComp"),
            "component label missing in SVG: {svg}"
        );
    }

    #[test]
    fn interface_label_rendered() {
        let input =
            "@startuml\ncomponent Hub\ninterface IA\ninterface IB\nHub - IA\nHub - IB\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Hub"), "Hub missing in SVG: {svg}");
        assert!(svg.contains("IA"), "IA missing in SVG: {svg}");
        assert!(svg.contains("IB"), "IB missing in SVG: {svg}");
    }

    #[test]
    fn multiple_stereotypes_rendered() {
        let input = "@startuml\ncomponent Auth <<service>> <<secured>>\nAuth --> Backend\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("service"), "first stereotype missing: {svg}");
        assert!(svg.contains("secured"), "second stereotype missing: {svg}");
    }

    #[test]
    fn plantuml_envelope() {
        let input = "@startuml\ncomponent Alpha\ncomponent Beta\nAlpha --> Beta\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains(r#"data-diagram-type="DESCRIPTION""#),
            "missing DESCRIPTION diagram type: {svg}"
        );
        assert!(
            svg.contains(r#"class="entity""#),
            "missing entity groups: {svg}"
        );
    }

    #[test]
    fn debug_stereo_width() {
        let s = "\u{00AB}service\u{00BB}";
        let w = crate::metrics::plantuml_text_width_14(s);
        eprintln!("«service» width = {w}");
        let w2 = crate::metrics::plantuml_text_width_14("service");
        eprintln!("service width = {w2}");
    }

    #[test]
    fn component_icon_rects() {
        let input = "@startuml\ncomponent Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let rect_count = svg.matches("<rect ").count();
        assert!(
            rect_count >= 4,
            "expected at least 4 rects for component icon, got {rect_count}: {svg}"
        );
        assert!(
            svg.contains(r##"fill="#F1F1F1""##),
            "missing #F1F1F1 fill: {svg}"
        );
    }

    // ─── Debug helper for golden-pair iteration ─────────────────────────
    // Set RUSTUML_DEBUG_COMPONENT=1 to dump oracle-driven render output for
    // every flat (no-package) test in the component bucket; failures are
    // written to /tmp/component_diffs.txt for fast iteration during fixes.
    #[test]
    fn dump_component_oracle_diffs() {
        if std::env::var("RUSTUML_DEBUG_COMPONENT").is_err() {
            return;
        }
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-diagrams/golden/component");
        if !root.exists() {
            eprintln!("no golden dir: {}", root.display());
            return;
        }
        let mut report = String::new();
        let mut pass = 0;
        let mut fail = 0;
        for entry in std::fs::read_dir(&root).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "puml") {
                continue;
            }
            let svg_path = path.with_extension("svg");
            if !svg_path.exists() {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            // Skip clustered (with `{`) for now.
            if source.contains('{') {
                continue;
            }
            // Skip error goldens.
            let golden = std::fs::read_to_string(&svg_path).unwrap();
            if golden.contains("Syntax Error") {
                continue;
            }
            let oracle = test_extract_oracle(&golden);
            let diagram = match rustuml_parser::parse::parse(&source) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let rust_svg = crate::render_svg_with_oracle(&diagram, Some(&oracle));
            let cmp = test_structural_compare(&golden, &rust_svg);
            if cmp.is_empty() {
                pass += 1;
            } else {
                fail += 1;
                if report.len() < 50_000 {
                    let name = path.file_stem().unwrap().to_string_lossy();
                    report.push_str(&format!("=== {name} ===\n"));
                    report.push_str(&cmp);
                    report.push('\n');
                }
            }
        }
        std::fs::write(
            "/tmp/component_diffs.txt",
            format!("pass={pass} fail={fail}\n\n{report}"),
        )
        .unwrap();
        eprintln!("flat: pass={pass} fail={fail}");
    }

    fn test_extract_oracle(svg: &str) -> crate::layout_oracle::OracleLayout {
        use crate::layout_oracle::{EntityRect, OracleEdgePath, OracleLayout};
        let mut layout = OracleLayout::default();

        // Canvas via viewBox.
        if let Some(vb_start) = svg.find("viewBox=\"") {
            let after = &svg[vb_start + 9..];
            if let Some(end) = after.find('"') {
                let parts: Vec<f64> = after[..end]
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if parts.len() == 4 {
                    layout.canvas_width = parts[2];
                    layout.canvas_height = parts[3];
                }
            }
        }

        // Extract entity groups: <g class="entity" data-qualified-name="X" ...>
        // followed by their first <rect> attributes.
        let entity_re = regex::Regex::new(
            r#"<g class="(?:entity|cluster)" data-qualified-name="([^"]+)"[^>]*>"#,
        )
        .unwrap();
        let attr_re = regex::Regex::new(r#"([A-Za-z_:][A-Za-z0-9_:\-]*)="([^"]*)""#).unwrap();
        for m in entity_re.find_iter(svg) {
            let caps = entity_re.captures(m.as_str()).unwrap();
            let name = caps.get(1).unwrap().as_str().to_string();
            let after_open = &svg[m.end()..];
            // Find the close </g> at this level.  Track nesting.
            let mut close_idx = None;
            let mut depth = 1;
            let mut pos = 0;
            while pos < after_open.len() {
                let next_open = after_open[pos..]
                    .find("<g")
                    .map(|i| pos + i)
                    .unwrap_or(usize::MAX);
                let next_close = after_open[pos..]
                    .find("</g>")
                    .map(|i| pos + i)
                    .unwrap_or(usize::MAX);
                if next_close == usize::MAX {
                    break;
                }
                if next_open < next_close {
                    depth += 1;
                    pos = next_open + 2;
                } else {
                    depth -= 1;
                    if depth == 0 {
                        close_idx = Some(next_close);
                        break;
                    }
                    pos = next_close + 4;
                }
            }
            let inner = &after_open[..close_idx.unwrap_or(after_open.len())];
            // Find first <rect ... />.
            let mut rect = None;
            if let Some(rect_pos) = inner.find("<rect ") {
                let rect_end = inner[rect_pos..]
                    .find("/>")
                    .map(|i| rect_pos + i)
                    .unwrap_or(inner.len());
                let mut ax = 0.0_f64;
                let mut ay = 0.0_f64;
                let mut aw = 0.0_f64;
                let mut ah = 0.0_f64;
                for attr_m in attr_re.captures_iter(&inner[rect_pos..rect_end]) {
                    let k = attr_m.get(1).unwrap().as_str();
                    let v = attr_m.get(2).unwrap().as_str();
                    let f = v.parse::<f64>().unwrap_or(0.0);
                    match k {
                        "x" => ax = f,
                        "y" => ay = f,
                        "width" => aw = f,
                        "height" => ah = f,
                        _ => {}
                    }
                }
                rect = Some((ax, ay, aw, ah));
            }
            if let Some((x, y, w, h)) = rect {
                // Collect all text y-values inside.
                let text_y_re = regex::Regex::new(r#"<text[^>]* y="([0-9.\-]+)""#).unwrap();
                let text_ys: Vec<f64> = text_y_re
                    .captures_iter(inner)
                    .filter_map(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
                    .collect();
                layout.entities.insert(
                    name,
                    EntityRect {
                        x,
                        y,
                        width: w,
                        height: h,
                        icon_cx: None,
                        glyph_path_d: None,
                        name_text_x: None,
                        text_y_values: text_ys,
                        sep_y_values: Vec::new(),
                        vis_icon_y_values: Vec::new(),
                    },
                );
            }
        }

        // Extract link groups.
        let link_re =
            regex::Regex::new(r#"<g class="link"([^>]*)>(.*?)</g>"#).unwrap();
        for caps in link_re.captures_iter(svg) {
            let header = caps.get(1).unwrap().as_str();
            let body = caps.get(2).unwrap().as_str();
            let attrs: std::collections::HashMap<String, String> = attr_re
                .captures_iter(header)
                .map(|c| {
                    (
                        c.get(1).unwrap().as_str().to_string(),
                        c.get(2).unwrap().as_str().to_string(),
                    )
                })
                .collect();
            // Parse <path>
            let path_re = regex::Regex::new(r#"<path([^>]*)/>"#).unwrap();
            let Some(path_caps) = path_re.captures(body) else {
                continue;
            };
            let path_attrs: std::collections::HashMap<String, String> = attr_re
                .captures_iter(path_caps.get(1).unwrap().as_str())
                .map(|c| {
                    (
                        c.get(1).unwrap().as_str().to_string(),
                        c.get(2).unwrap().as_str().to_string(),
                    )
                })
                .collect();
            let id = path_attrs.get("id").cloned().unwrap_or_default();
            let d = path_attrs.get("d").cloned().unwrap_or_default();
            // Polygon if present.
            let poly_re = regex::Regex::new(r#"<polygon([^>]*)/>"#).unwrap();
            let mut arrow_points = None;
            let mut arrow_fill = None;
            let mut polygon_style = None;
            if let Some(p) = poly_re.captures(body) {
                let pa: std::collections::HashMap<String, String> = attr_re
                    .captures_iter(p.get(1).unwrap().as_str())
                    .map(|c| {
                        (
                            c.get(1).unwrap().as_str().to_string(),
                            c.get(2).unwrap().as_str().to_string(),
                        )
                    })
                    .collect();
                arrow_points = pa.get("points").cloned();
                arrow_fill = pa.get("fill").cloned();
                polygon_style = pa.get("style").cloned();
            }
            layout.edges.push(OracleEdgePath {
                id,
                d,
                arrow_points,
                arrow_fill,
                link_type: attrs.get("data-link-type").cloned(),
                entity_1: attrs.get("data-entity-1").cloned(),
                entity_2: attrs.get("data-entity-2").cloned(),
                source_line: attrs.get("data-source-line").cloned(),
                link_id: attrs.get("id").cloned(),
                path_style: path_attrs.get("style").cloned(),
                code_line: path_attrs.get("codeLine").cloned(),
                polygon_style,
            });
        }

        layout
    }

    fn test_structural_compare(expected: &str, actual: &str) -> String {
        // Mirror the strict-XML comparator's contract using a tiny regex-based
        // tokenizer: collect (tag, depth, sorted_attrs, text) tuples,
        // ignoring processing instructions, comments, and whitespace-only text.
        let exp = test_collect_elements(expected);
        let act = test_collect_elements(actual);
        let mut diffs = String::new();
        if exp.len() != act.len() {
            diffs.push_str(&format!("len(exp)={} len(act)={}\n", exp.len(), act.len()));
        }
        let n = exp.len().min(act.len());
        let mut shown = 0;
        for i in 0..n {
            if exp[i] != act[i] {
                if shown < 6 {
                    diffs.push_str(&format!(
                        "@{i}: exp={:?}\n      act={:?}\n",
                        exp[i], act[i]
                    ));
                    shown += 1;
                }
            }
        }
        diffs
    }

    fn test_collect_elements(svg: &str) -> Vec<(String, usize, Vec<(String, String)>, String)> {
        // Strip PIs and comments.
        let re_pi = regex::Regex::new(r"<\?[^?]*\?>").unwrap();
        let re_cm = regex::Regex::new(r"<!--[^>]*-->").unwrap();
        let s = re_pi.replace_all(svg, "");
        let s = re_cm.replace_all(&s, "");
        let mut out = Vec::new();
        let mut depth = 0_usize;
        // Tag tokenizer.
        let tag_re = regex::Regex::new(r"<(/?)([A-Za-z][A-Za-z0-9_-]*)([^>]*)(/?)>").unwrap();
        let attr_re = regex::Regex::new(r#"([A-Za-z_:][A-Za-z0-9_:\-]*)="([^"]*)""#).unwrap();
        let mut last_end = 0;
        for m in tag_re.find_iter(&s) {
            let caps = tag_re.captures(m.as_str()).unwrap();
            let closing = !caps.get(1).unwrap().as_str().is_empty();
            let tag = caps.get(2).unwrap().as_str().to_string();
            let attr_part = caps.get(3).unwrap().as_str();
            let self_closing = !caps.get(4).unwrap().as_str().is_empty();
            // Text between last_end and m.start() (only for opening tags).
            let text_between = s[last_end..m.start()].trim().to_string();
            if closing {
                if depth > 0 {
                    depth -= 1;
                }
            } else if tag == "title" {
                // skip title and its content
            } else {
                let mut attrs: Vec<(String, String)> = attr_re
                    .captures_iter(attr_part)
                    .filter(|c| !c.get(1).unwrap().as_str().starts_with("xmlns"))
                    .map(|c| {
                        (
                            c.get(1).unwrap().as_str().to_string(),
                            c.get(2).unwrap().as_str().to_string(),
                        )
                    })
                    .collect();
                attrs.sort();
                let _ = text_between;
                // Text is associated with the *most recent* opened element;
                // we approximate by reading until the next tag start.
                // For our purposes element-only comparison is enough; gather text
                // from immediately after this opening tag until next '<'.
                let after = &s[m.end()..];
                let text = after
                    .split_once('<')
                    .map(|(t, _)| t.trim().to_string())
                    .unwrap_or_default();
                out.push((tag, depth, attrs, text));
                if !self_closing {
                    depth += 1;
                }
            }
            last_end = m.end();
        }
        out
    }
}
