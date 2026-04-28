// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's component diagram rendering.
//! PlantUML renders component diagrams as diagram type "DESCRIPTION".

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::component::*;

use crate::layout_oracle::OracleLayout;
use crate::metrics;
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
/// Line height per text line in a component box.
const LINE_HEIGHT: f64 = 16.4883;
/// Base component box height (padding around one line of text).
const COMPONENT_BASE_H: f64 = 30.0;
/// Single-line component height.
const COMPONENT_H: f64 = COMPONENT_BASE_H + LINE_HEIGHT;
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

    // Compute dimensions for each component.
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
        compute_positions_from_oracle(diagram, &comp_dims, orc, title_h)
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

    // Render each component entity.
    let mut entity_counter = 2; // PlantUML entity IDs start at ent0002
    for (i, comp) in diagram.components.iter().enumerate() {
        let (x, y) = positions[i];
        let dim = &comp_dims[i];
        let ent_id = format!("ent{entity_counter:04}");
        entity_counter += 1;

        // HTML comment.
        svg.raw(&format!("<!--entity {}-->", comp.id));

        // Open entity group.
        let qualified = &comp.id;
        svg.raw(&format!(
            r#"<g class="entity" data-qualified-name="{qualified}" id="{ent_id}">"#
        ));

        // URL link wrapper.
        if let Some(ref url) = comp.url {
            svg.open_link(url);
        }

        // Determine fill: use custom color from theme if set, otherwise default.
        let fill = COMP_FILL;

        // Main body rectangle.
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{h}" rx="{ROUND_R}" ry="{ROUND_R}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
            h = dim.height,
            w = dim.width,
        ));

        // Component icon (tab + bars) at top-right.
        let tab_x = x + dim.width - ICON_TAB_RIGHT_OFFSET;
        let tab_y = y + ICON_TAB_TOP_OFFSET;
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{ICON_TAB_H}" style="stroke:{STROKE};stroke-width:0.5;" width="{ICON_TAB_W}" x="{tab_x}" y="{tab_y}"/>"#,
        ));

        let bar_x = tab_x - ICON_BAR_LEFT_OFFSET;
        let bar_y1 = tab_y + ICON_BAR_TOP_OFFSET_1;
        let bar_y2 = tab_y + ICON_BAR_TOP_OFFSET_2;
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{ICON_BAR_H}" style="stroke:{STROKE};stroke-width:0.5;" width="{ICON_BAR_W}" x="{bar_x}" y="{bar_y1}"/>"#,
        ));
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{ICON_BAR_H}" style="stroke:{STROKE};stroke-width:0.5;" width="{ICON_BAR_W}" x="{bar_x}" y="{bar_y2}"/>"#,
        ));

        // Render text lines.
        let text_x = x + TEXT_PAD_LEFT;
        let n_lines = 1 + comp.stereotypes.len();
        let first_text_y = y + dim.height - LINE_HEIGHT * n_lines as f64 + LINE_HEIGHT
            - (COMPONENT_BASE_H - LINE_HEIGHT) / 2.0;

        // Stereotypes first (italic in PlantUML).
        for (si, stereo) in comp.stereotypes.iter().enumerate() {
            let ty = first_text_y + si as f64 * LINE_HEIGHT;
            let label = format!("\u{00AB}{stereo}\u{00BB}"); // «stereo»
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" font-style="italic" lengthAdjust="spacing" textLength="{tl}" x="{text_x}" y="{ty}">{escaped}</text>"#,
                tl = metrics::text_width(&label, FONT_SIZE),
                escaped = escape_xml(&label),
            ));
        }

        // Label (last line).
        let label_y = first_text_y + comp.stereotypes.len() as f64 * LINE_HEIGHT;
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" lengthAdjust="spacing" textLength="{tl}" x="{text_x}" y="{label_y}">{escaped}</text>"#,
            tl = metrics::text_width(&comp.label, FONT_SIZE),
            escaped = escape_xml(&comp.label),
        ));

        if comp.url.is_some() {
            svg.close_link();
        }

        svg.raw("</g>");
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
            r#"<ellipse cx="{ix}" cy="{iy}" fill="{COMP_FILL}" rx="{IFACE_R}" ry="{IFACE_R}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        ));

        // Label below.
        let label_y = iy + IFACE_R + LINE_HEIGHT + 4.0;
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" lengthAdjust="spacing" textLength="{tl}" x="{lx}" y="{label_y}">{escaped}</text>"#,
            tl = metrics::text_width(&iface.label, FONT_SIZE),
            lx = ix - metrics::text_width(&iface.label, FONT_SIZE) / 2.0,
            escaped = escape_xml(&iface.label),
        ));

        svg.raw("</g>");
    }

    // Render connections (links).
    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc);
    } else {
        for (link_counter, conn) in (entity_counter..).zip(diagram.connections.iter()) {
            let link_id = format!("lnk{link_counter}");

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
                continue;
            };

            let (to_cx, to_cy, _to_top) = if let Some((i, _)) = to_comp {
                let (x, y) = positions[i];
                let dim = &comp_dims[i];
                (x + dim.width / 2.0, y, y)
            } else if let Some((i, _)) = to_iface {
                let (ix, iy) = iface_positions[i];
                (ix, iy, iy - IFACE_R)
            } else {
                continue;
            };

            // Determine link type.
            let _has_arrow = conn.from.contains("-->")
                || conn.to.contains("-->")
                || !conn.dashed && from_comp.is_some() && to_comp.is_some();
            // In PlantUML: --> is dependency, -- is association, ..> is dependency (dashed),
            // .. is association (dashed). We infer from the parser's dashed flag and arrow presence.
            // The parser sets dashed=true for dotted lines. Arrow presence is implied by --> vs --.
            // Since Connection doesn't carry arrow type, we assume:
            // - non-dashed + components => dependency (has arrow)
            // - dashed => dependency (has arrow)
            // For association (no arrow), the link type is "association".
            let link_type = "dependency"; // Simplified - the parser doesn't distinguish fully.
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
                // Render bezier path.
                let path_d = build_path_d(&ep.points);
                let path_id = format!("{}-to-{}", conn.from, conn.to);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
            ));

                // Arrowhead.
                let last = ep.points.last().unwrap();
                let prev = if ep.points.len() >= 2 {
                    &ep.points[ep.points.len() - 2]
                } else {
                    last
                };
                render_arrowhead(&mut svg, prev, last);

                // Labels.
                let first = ep.points.first().unwrap();
                if let Some(label) = &conn.label {
                    let mx = (first.0 + last.0) / 2.0;
                    let my = (first.1 + last.1) / 2.0;
                    svg.raw(&format!(
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                    tl = metrics::text_width(label, LINK_FONT),
                    tx = mx + 1.0,
                    ty = my - 4.0,
                    escaped = escape_xml(label),
                ));
                }
                if let Some(from_mult) = &conn.from_mult {
                    svg.raw(&format!(
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                    tl = metrics::text_width(from_mult, LINK_FONT),
                    tx = first.0 - metrics::text_width(from_mult, LINK_FONT) - 1.0,
                    ty = first.1 + LINK_FONT + 2.0,
                    escaped = escape_xml(from_mult),
                ));
                }
                if let Some(to_mult) = &conn.to_mult {
                    svg.raw(&format!(
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                    tl = metrics::text_width(to_mult, LINK_FONT),
                    tx = last.0 - metrics::text_width(to_mult, LINK_FONT) - 1.0,
                    ty = last.1 - 4.0,
                    escaped = escape_xml(to_mult),
                ));
                }
            } else {
                // Straight line fallback.
                let path_d = format!(
                    "M {from_cx},{from_cy} C {from_cx},{mid_y1} {to_cx},{mid_y2} {to_cx},{to_cy}",
                    mid_y1 = from_cy + (to_cy - from_cy) * 0.3,
                    mid_y2 = from_cy + (to_cy - from_cy) * 0.7,
                );
                let path_id = format!("{}-to-{}", conn.from, conn.to);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
            ));

                // Arrowhead for dependency arrows.
                render_arrowhead_from_coords(&mut svg, from_cx, from_bottom, to_cx, to_cy);

                // Labels.
                if let Some(label) = &conn.label {
                    let mx = (from_cx + to_cx) / 2.0;
                    let my = (from_cy + to_cy) / 2.0;
                    svg.raw(&format!(
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                    tl = metrics::text_width(label, LINK_FONT),
                    tx = mx + 1.0,
                    ty = my - 4.0,
                    escaped = escape_xml(label),
                ));
                }
                if let Some(from_mult) = &conn.from_mult {
                    svg.raw(&format!(
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                    tl = metrics::text_width(from_mult, LINK_FONT),
                    tx = from_cx - metrics::text_width(from_mult, LINK_FONT) - 1.0,
                    ty = from_cy + LINK_FONT + 2.0,
                    escaped = escape_xml(from_mult),
                ));
                }
                if let Some(to_mult) = &conn.to_mult {
                    svg.raw(&format!(
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                    tl = metrics::text_width(to_mult, LINK_FONT),
                    tx = to_cx - metrics::text_width(to_mult, LINK_FONT) - 1.0,
                    ty = to_cy - 4.0,
                    escaped = escape_xml(to_mult),
                ));
                }
            }

            svg.raw("</g>");
        }
    } // end else (non-oracle connections)

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
// Component dimension calculation
// ---------------------------------------------------------------------------

struct CompDim {
    width: f64,
    height: f64,
}

fn calc_component_dim(comp: &Component) -> CompDim {
    let n_lines = 1 + comp.stereotypes.len();
    let height = COMPONENT_BASE_H + n_lines as f64 * LINE_HEIGHT;

    // Width: max of label width and stereotype widths, plus padding.
    let label_w = metrics::text_width(&comp.label, FONT_SIZE);
    let max_stereo_w = comp
        .stereotypes
        .iter()
        .map(|s| metrics::text_width(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE))
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
    oracle: &OracleLayout,
    title_h: f64,
) -> LayoutResult {
    let mut positions = Vec::with_capacity(diagram.components.len());
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    for (i, comp) in diagram.components.iter().enumerate() {
        // Try qualified name (may be package.component) and bare id.
        let rect = oracle
            .entities
            .get(&comp.id)
            .or_else(|| oracle.entities.get(&comp.label));
        if let Some(rect) = rect {
            positions.push((rect.x, rect.y));
        } else {
            // Fallback: use grid position.
            let dim = &comp_dims[i];
            positions.push((MARGIN + (i as f64) * (dim.width + GAP), MARGIN + title_h));
        }
    }

    for iface in &diagram.interfaces {
        let rect = oracle.entities.get(&iface.id);
        if let Some(rect) = rect {
            iface_positions.push((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0));
        } else {
            // Fallback.
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

    let mut positions = Vec::with_capacity(n);
    let y_start = title_h + MARGIN;
    for (i, _comp) in diagram.components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (COMPONENT_H + GAP);
        positions.push((x, y));
    }

    let comp_total_w = if n > 0 {
        MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64
    } else {
        0.0
    };
    let comp_total_h = if n > 0 {
        rows as f64 * (COMPONENT_H + GAP)
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
// Arrowhead rendering
// ---------------------------------------------------------------------------

/// Render connections directly from oracle edge data.
fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &ComponentDiagram,
    oracle: &OracleLayout,
) {
    for conn in &diagram.connections {
        let expected_id = format!("{}-to-{}", conn.from, conn.to);

        let oracle_edge = match oracle.edges.iter().find(|e| e.id == expected_id) {
            Some(e) => e,
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
            r#"<path{code_line_attr} d="{}" fill="none" id="{expected_id}" style="{path_style}"/>"#,
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

        svg.raw("</g>");
    }
}

fn render_arrowhead(svg: &mut SvgBuilder, prev: &(f64, f64), tip: &(f64, f64)) {
    let dx = tip.0 - prev.0;
    let dy = tip.1 - prev.1;
    let angle = dy.atan2(dx);
    render_arrow_at(svg, tip.0, tip.1, angle);
}

fn render_arrowhead_from_coords(svg: &mut SvgBuilder, _fx: f64, _fy: f64, tx: f64, ty: f64) {
    // Downward arrow (most common in top-to-bottom layout).
    let size = 5.0;
    let pts = format!(
        "{tx},{ty},{x1},{y1},{tx2},{ty2},{x3},{y3},{tx},{ty}",
        x1 = tx + size,
        y1 = ty - size * 2.0,
        tx2 = tx,
        ty2 = ty - size * 1.5,
        x3 = tx - size,
        y3 = ty - size * 2.0,
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
    let pts = format!("{x},{y},{x1},{y1},{x2},{y2},{x3},{y3},{x},{y}");
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
    write!(d, "M {x0},{y0}").unwrap();
    if points.len() >= 4 {
        // Cubic bezier.
        let mut i = 1;
        while i + 2 < points.len() {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];
            let (x3, y3) = points[i + 2];
            write!(d, " C {x1},{y1} {x2},{y2} {x3},{y3}").unwrap();
            i += 3;
        }
    } else {
        // Line segments.
        for &(x, y) in &points[1..] {
            write!(d, " L {x},{y}").unwrap();
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
        .map(|l| metrics::text_width(l, LINK_FONT) + NOTE_PAD * 2.0)
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

    // Note box with dog-ear, matching PlantUML's path-based rendering.
    // PlantUML uses a path for the note shape including a connector line.
    svg.note_box(nx, ny, note_w, note_h, NOTE_FOLD, NOTE_FILL, STROKE);

    for (i, line) in lines.iter().enumerate() {
        let ty = ny + NOTE_PAD + (i as f64 + 1.0) * NOTE_LINE_H - 2.0;
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT}" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
            tl = metrics::text_width(line, LINK_FONT),
            tx = nx + NOTE_PAD,
            escaped = escape_xml(line),
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
    for pkg in packages {
        let name_w = metrics::text_width(&pkg.label, FONT_SIZE) + 20.0;
        let stereo_w = pkg
            .stereotype
            .as_deref()
            .map(|s| metrics::text_width(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE) + 20.0)
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
            *y += leaf_count as f64 * (COMPONENT_H + GAP);
        }

        let pkg_inner_h = (*y - label_y - CONTAINER_PAD).max(COMPONENT_H);
        let pkg_h = pkg_inner_h + CONTAINER_PAD * 2.0 + CONTAINER_LABEL_H;

        // Package container rectangle.
        svg.raw(&format!(
            r#"<rect fill="none" height="{pkg_h}" style="stroke:{STROKE};stroke-width:1.5;" width="{pkg_w}" x="{x}" y="{pkg_y_start}"/>"#,
        ));

        // Label.
        svg.raw(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" font-weight="700" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
            tl = metrics::text_width(&pkg.label, FONT_SIZE),
            tx = x + CONTAINER_PAD,
            ty = pkg_y_start + CONTAINER_LABEL_H - 4.0,
            escaped = escape_xml(&pkg.label),
        ));

        // Stereotype.
        if let Some(stereo) = &pkg.stereotype {
            let label = format!("\u{00AB}{stereo}\u{00BB}");
            svg.raw(&format!(
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{FONT_SIZE}" font-style="italic" lengthAdjust="spacing" textLength="{tl}" x="{tx}" y="{ty}">{escaped}</text>"#,
                tl = metrics::text_width(&label, FONT_SIZE),
                tx = x + CONTAINER_PAD,
                ty = pkg_y_start + CONTAINER_LABEL_H + 12.0,
                escaped = escape_xml(&label),
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
    let name_w = metrics::text_width(&pkg.label, FONT_SIZE) + 20.0;
    let stereo_w = pkg
        .stereotype
        .as_deref()
        .map(|s| metrics::text_width(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE) + 20.0)
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
    let nested_h: f64 = pkg.packages.iter().map(estimate_package_height).sum();
    let leaf_h = pkg.components.len() as f64 * (COMPONENT_H + GAP);
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
    fn component_icon_rects() {
        let input = "@startuml\ncomponent Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        // Should have 4 rects: main body + tab + 2 bars.
        let rect_count = svg.matches("<rect ").count();
        assert!(
            rect_count >= 4,
            "expected at least 4 rects for component icon, got {rect_count}: {svg}"
        );
        // Fill should be PlantUML default.
        assert!(
            svg.contains(r##"fill="#F1F1F1""##),
            "missing #F1F1F1 fill: {svg}"
        );
    }
}
