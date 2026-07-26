// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Graphviz DOT diagram SVG renderer.
//!
//! Uses rustuml-layout (Sugiyama algorithm) for node positioning,
//! then renders nodes with appropriate shapes, edges with styles,
//! and cluster boxes around grouped nodes.

use std::collections::HashMap;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::dot::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const DEFAULT_NODE_W: f64 = 100.0;
const DEFAULT_NODE_H: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 50.0;
const PADDING: f64 = 12.0;
const CLUSTER_PAD: f64 = 16.0;
const CLUSTER_LABEL_H: f64 = 20.0;
const DEFAULT_FILL: &str = "#FEFECE";
const DEFAULT_STROKE: &str = "#A80036";
const CLUSTER_FILL: &str = "#F8F8F8";
const CLUSTER_STROKE: &str = "#808080";
const EDGE_STROKE: &str = "#A80036";
const DIAMOND_SIZE: f64 = 14.0;

/// Render a DOT diagram to SVG.
///
/// PlantUML's `@startdot` / `@enddot` pipeline shells out to the `dot`
/// binary and forwards its SVG output verbatim. We do the same when `dot`
/// is on PATH — the strict comparator skips PIs, comments, and the DTD
/// reference, so Graphviz-emitted SVG matches byte-for-byte (modulo the
/// version-string in the generator comment, which is ignored). Falls back
/// to the layout-based renderer when `dot` is unavailable.
pub fn render(diagram: &DotDiagram, theme: &Theme) -> String {
    if let Some(svg) = try_render_with_dot(diagram) {
        return svg;
    }
    render_fallback(diagram, theme)
}

fn try_render_with_dot(diagram: &DotDiagram) -> Option<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let source = diagram.meta.source.as_deref()?;
    let dot_input = extract_dot_input(source)?;

    let mut child = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(dot_input.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Strip the `@startdot` / `@enddot` markers from `meta.source` to recover
/// the raw DOT graph definition to pipe into `dot`.
fn extract_dot_input(source: &str) -> Option<String> {
    let mut out = String::new();
    let mut inside = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if !inside {
            if trimmed.starts_with("@startdot") {
                inside = true;
            }
            continue;
        }
        if trimmed.starts_with("@enddot") {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    if out.is_empty() { None } else { Some(out) }
}

fn render_fallback(diagram: &DotDiagram, _theme: &Theme) -> String {
    // Collect all nodes (top-level + cluster members).
    let all_nodes = collect_all_nodes(diagram);
    if all_nodes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Determine layout direction from graph attributes.
    let direction = match diagram.attrs.get("rankdir").map(|s| s.as_str()) {
        Some("LR") | Some("lr") => Direction::LeftToRight,
        _ => Direction::TopToBottom,
    };

    // Build layout graph.
    let mut layout = LayoutGraph::new(direction);
    let node_dims = compute_node_dims(&all_nodes, &diagram.node_defaults);

    for (node, (w, h)) in all_nodes.iter().zip(&node_dims) {
        let label = effective_label(node, &diagram.node_defaults);
        let shape = effective_shape(node, &diagram.node_defaults);
        if shape == "circle" || shape == "point" {
            layout.add_circle_node(&node.id, &label, *w);
        } else {
            layout.add_node(&node.id, &label, *w, *h);
        }
    }

    // Add all edges (top-level + cluster).
    let all_edges = collect_all_edges(diagram);
    for edge in &all_edges {
        let label = edge.attrs.get("label").map(|s| s.as_str());
        layout.add_edge(&edge.from, &edge.to, label);
    }

    // Run layout with timeout.
    let result = match layout.layout_full(std::time::Duration::from_secs(5)) {
        Some(r) => r,
        None => return render_grid(diagram, &all_nodes, &node_dims, &all_edges),
    };

    render_with_positions(
        diagram,
        &all_nodes,
        &node_dims,
        &all_edges,
        &result.node_positions,
        &result.edge_paths,
    )
}

// ---------------------------------------------------------------------------
// Layout-based rendering
// ---------------------------------------------------------------------------

fn render_with_positions(
    diagram: &DotDiagram,
    all_nodes: &[&DotNode],
    node_dims: &[(f64, f64)],
    all_edges: &[&DotEdge],
    positions: &[rustuml_layout::graph::NodePosition],
    edge_paths: &[EdgePath],
) -> String {
    if positions.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Build id-to-position map.
    let mut pos_map: HashMap<&str, (f64, f64, f64, f64)> = HashMap::new();
    for (node, pos) in all_nodes.iter().zip(positions) {
        pos_map.insert(&node.id, (pos.x, pos.y, pos.width, pos.height));
    }

    // Compute canvas size.
    let max_x = positions
        .iter()
        .map(|p| p.x + p.width)
        .fold(0.0_f64, f64::max);
    let max_y = positions
        .iter()
        .map(|p| p.y + p.height)
        .fold(0.0_f64, f64::max);
    let total_w = max_x + MARGIN * 2.0;
    let total_h = max_y + MARGIN * 2.0;

    let mut svg = SvgBuilder::new(total_w, total_h);

    // Render graph background.
    if let Some(bgcolor) = diagram.attrs.get("bgcolor") {
        svg.rect(0.0, 0.0, total_w, total_h, bgcolor, "none");
    }

    // Render graph title.
    if let Some(title) = diagram.attrs.get("label") {
        svg.text(total_w / 2.0, FONT_SIZE + 4.0, title, "middle", FONT_SIZE);
    }

    // Render cluster boxes.
    for cluster in &diagram.clusters {
        render_cluster(&mut svg, cluster, &pos_map);
    }

    // Render edges.
    for edge in all_edges {
        render_edge(
            &mut svg,
            edge,
            diagram.directed,
            &diagram.edge_defaults,
            &pos_map,
            edge_paths,
        );
    }

    // Render nodes.
    for (node, (dim_w, dim_h)) in all_nodes.iter().zip(node_dims) {
        if let Some(&(x, y, w, h)) = pos_map.get(node.id.as_str()) {
            render_node(
                &mut svg,
                node,
                x + MARGIN,
                y + MARGIN,
                w.max(*dim_w),
                h.max(*dim_h),
                &diagram.node_defaults,
            );
        }
    }

    svg.finalize()
}

// ---------------------------------------------------------------------------
// Grid fallback rendering (when layout times out)
// ---------------------------------------------------------------------------

fn render_grid(
    diagram: &DotDiagram,
    all_nodes: &[&DotNode],
    node_dims: &[(f64, f64)],
    all_edges: &[&DotEdge],
) -> String {
    let n = all_nodes.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = if n == 0 { 0 } else { n.div_ceil(cols) };

    let col_w = node_dims
        .iter()
        .map(|(w, _)| *w)
        .fold(DEFAULT_NODE_W, f64::max);
    let row_h = node_dims
        .iter()
        .map(|(_, h)| *h)
        .fold(DEFAULT_NODE_H, f64::max);

    let total_w = MARGIN * 2.0 + cols as f64 * (col_w + GAP);
    let total_h = MARGIN * 2.0 + rows as f64 * (row_h + GAP);

    let mut svg = SvgBuilder::new(total_w, total_h);

    let mut pos_map: HashMap<&str, (f64, f64, f64, f64)> = HashMap::new();
    for (i, (node, (w, h))) in all_nodes.iter().zip(node_dims).enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col as f64 * (col_w + GAP);
        let y = MARGIN + row as f64 * (row_h + GAP);
        pos_map.insert(&node.id, (x, y, *w, *h));
        render_node(&mut svg, node, x, y, *w, *h, &diagram.node_defaults);
    }

    for edge in all_edges {
        render_edge(
            &mut svg,
            edge,
            diagram.directed,
            &diagram.edge_defaults,
            &pos_map,
            &[],
        );
    }

    svg.finalize()
}

// ---------------------------------------------------------------------------
// Node rendering
// ---------------------------------------------------------------------------

fn render_node(
    svg: &mut SvgBuilder,
    node: &DotNode,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    defaults: &HashMap<String, String>,
) {
    let shape = effective_shape(node, defaults);
    let label = effective_label(node, defaults);
    let fill = effective_fill(node, defaults);
    let stroke = node
        .attrs
        .get("color")
        .or_else(|| defaults.get("color"))
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_STROKE);

    match shape.as_str() {
        "box" | "rect" | "rectangle" | "record" | "Mrecord" => {
            svg.rect(x, y, w, h, &fill, stroke);
            svg.text(
                x + w / 2.0,
                y + h / 2.0 + FONT_SIZE / 3.0,
                &label,
                "middle",
                FONT_SIZE,
            );
        }
        "ellipse" | "oval" | "" => {
            // Default shape is ellipse.
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let rx = w / 2.0;
            let ry = h / 2.0;
            svg.raw(&format!(
                r#"<ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
            ));
            svg.text(cx, cy + FONT_SIZE / 3.0, &label, "middle", FONT_SIZE);
        }
        "circle" => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let r = w.min(h) / 2.0;
            svg.circle(cx, cy, r, &fill, stroke);
            svg.text(cx, cy + FONT_SIZE / 3.0, &label, "middle", FONT_SIZE);
        }
        "diamond" => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            svg.diamond(cx, cy, DIAMOND_SIZE, &fill, stroke);
            svg.text(cx, cy + FONT_SIZE / 3.0, &label, "middle", FONT_SIZE);
        }
        "plaintext" | "plain" | "none" => {
            // No border — just the label text.
            svg.text(
                x + w / 2.0,
                y + h / 2.0 + FONT_SIZE / 3.0,
                &label,
                "middle",
                FONT_SIZE,
            );
        }
        "point" => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            svg.circle(cx, cy, 3.0, "#000000", "#000000");
        }
        "doublecircle" => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let r = w.min(h) / 2.0;
            svg.circle(cx, cy, r, &fill, stroke);
            svg.circle(cx, cy, r - 3.0, &fill, stroke);
            svg.text(cx, cy + FONT_SIZE / 3.0, &label, "middle", FONT_SIZE);
        }
        // Treat unknown shapes as boxes.
        _ => {
            svg.rect(x, y, w, h, &fill, stroke);
            svg.text(
                x + w / 2.0,
                y + h / 2.0 + FONT_SIZE / 3.0,
                &label,
                "middle",
                FONT_SIZE,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Edge rendering
// ---------------------------------------------------------------------------

fn render_edge(
    svg: &mut SvgBuilder,
    edge: &DotEdge,
    directed: bool,
    defaults: &HashMap<String, String>,
    pos_map: &HashMap<&str, (f64, f64, f64, f64)>,
    edge_paths: &[EdgePath],
) {
    let color = edge
        .attrs
        .get("color")
        .or_else(|| defaults.get("color"))
        .map(|s| s.as_str())
        .unwrap_or(EDGE_STROKE);

    let style = edge
        .attrs
        .get("style")
        .or_else(|| defaults.get("style"))
        .map(|s| s.as_str())
        .unwrap_or("");

    let dashed = style == "dashed" || style == "dotted";

    // Try bezier path from layout engine first.
    let edge_path = edge_paths
        .iter()
        .find(|ep| ep.from == edge.from && ep.to == edge.to);

    if let Some(ep) = edge_path
        && !ep.points.is_empty()
    {
        svg.bezier_path_with_arrow(&ep.points, color, dashed, 8.0);
        if let Some(label) = edge.attrs.get("label") {
            let first = ep.points.first().unwrap();
            let last = ep.points.last().unwrap();
            let mid_x = (first.0 + last.0) / 2.0;
            let mid_y = (first.1 + last.1) / 2.0 - 6.0;
            svg.text(mid_x, mid_y, label, "middle", SMALL_FONT);
        }
        return;
    }

    // Fallback to straight lines.
    let Some(&(fx, fy, fw, fh)) = pos_map.get(edge.from.as_str()) else {
        return;
    };
    let Some(&(tx, ty, tw, th)) = pos_map.get(edge.to.as_str()) else {
        return;
    };

    // Edge endpoints at center of each node.
    let from_cx = fx + fw / 2.0;
    let from_cy = fy + fh / 2.0;
    let to_cx = tx + tw / 2.0;
    let to_cy = ty + th / 2.0;

    // Clip to node boundaries (simple box clipping).
    let (x1, y1) = clip_to_box(from_cx, from_cy, to_cx, to_cy, fw, fh);
    let (x2, y2) = clip_to_box(to_cx, to_cy, from_cx, from_cy, tw, th);

    svg.line_segment(x1, y1, x2, y2, color, dashed);

    // Arrowhead for directed graphs (unless arrowhead=none).
    let arrowhead = edge
        .attrs
        .get("arrowhead")
        .map(|s| s.as_str())
        .unwrap_or("normal");
    if directed && arrowhead != "none" {
        let angle = (y2 - y1).atan2(x2 - x1);
        let deg = angle.to_degrees();
        svg.arrow_head(x2, y2, deg);
    }

    // Edge label.
    if let Some(label) = edge.attrs.get("label") {
        let mid_x = (x1 + x2) / 2.0;
        let mid_y = (y1 + y2) / 2.0 - 6.0;
        svg.text(mid_x, mid_y, label, "middle", SMALL_FONT);
    }
}

/// Clip a line from (cx, cy) toward (tx, ty) to the boundary of a box
/// of size (w, h) centered at (cx, cy). Returns the clipped point.
fn clip_to_box(cx: f64, cy: f64, tx: f64, ty: f64, w: f64, h: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (cx, cy);
    }

    let hw = w / 2.0;
    let hh = h / 2.0;

    // Parametric: point = (cx + t*dx, cy + t*dy).
    // Find smallest positive t where the point exits the box.
    let mut t: f64 = 1.0;
    if dx.abs() > 0.001 {
        let tx_edge = if dx > 0.0 { hw / dx } else { -hw / dx };
        t = t.min(tx_edge);
    }
    if dy.abs() > 0.001 {
        let ty_edge = if dy > 0.0 { hh / dy } else { -hh / dy };
        t = t.min(ty_edge);
    }

    (cx + t * dx, cy + t * dy)
}

// ---------------------------------------------------------------------------
// Cluster rendering
// ---------------------------------------------------------------------------

fn render_cluster(
    svg: &mut SvgBuilder,
    cluster: &DotCluster,
    pos_map: &HashMap<&str, (f64, f64, f64, f64)>,
) {
    // Find bounding box of cluster members.
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for node in &cluster.nodes {
        if let Some(&(x, y, w, h)) = pos_map.get(node.id.as_str()) {
            min_x = min_x.min(x + MARGIN);
            min_y = min_y.min(y + MARGIN);
            max_x = max_x.max(x + MARGIN + w);
            max_y = max_y.max(y + MARGIN + h);
        }
    }

    if min_x > max_x {
        return; // No positioned nodes in this cluster.
    }

    // Expand by cluster padding.
    min_x -= CLUSTER_PAD;
    min_y -= CLUSTER_PAD + CLUSTER_LABEL_H;
    max_x += CLUSTER_PAD;
    max_y += CLUSTER_PAD;

    let w = max_x - min_x;
    let h = max_y - min_y;

    svg.rounded_rect(min_x, min_y, w, h, 4.0, CLUSTER_FILL, CLUSTER_STROKE);

    // Cluster label.
    let label = cluster.label.as_deref().unwrap_or(&cluster.name);
    if !label.is_empty() {
        svg.text(
            min_x + w / 2.0,
            min_y + CLUSTER_LABEL_H - 4.0,
            label,
            "middle",
            SMALL_FONT,
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_all_nodes(diagram: &DotDiagram) -> Vec<&DotNode> {
    let mut nodes: Vec<&DotNode> = diagram.nodes.iter().collect();
    for cluster in &diagram.clusters {
        for node in &cluster.nodes {
            // Only add if not already present at top level.
            if !nodes.iter().any(|n| n.id == node.id) {
                nodes.push(node);
            }
        }
    }
    nodes
}

fn collect_all_edges(diagram: &DotDiagram) -> Vec<&DotEdge> {
    let mut edges: Vec<&DotEdge> = diagram.edges.iter().collect();
    for cluster in &diagram.clusters {
        edges.extend(cluster.edges.iter());
    }
    edges
}

fn compute_node_dims(nodes: &[&DotNode], defaults: &HashMap<String, String>) -> Vec<(f64, f64)> {
    nodes
        .iter()
        .map(|node| {
            let label = effective_label(node, defaults);
            let shape = effective_shape(node, defaults);
            let text_w = metrics::text_width(&label, FONT_SIZE) + PADDING * 2.0;
            let w = text_w.max(DEFAULT_NODE_W);
            match shape.as_str() {
                "circle" | "doublecircle" => {
                    let d = w.max(DEFAULT_NODE_H);
                    (d, d)
                }
                "point" => (8.0, 8.0),
                "diamond" => (DIAMOND_SIZE * 2.0 + PADDING, DIAMOND_SIZE * 2.0 + PADDING),
                _ => (w, DEFAULT_NODE_H),
            }
        })
        .collect()
}

fn effective_label(node: &DotNode, defaults: &HashMap<String, String>) -> String {
    node.attrs
        .get("label")
        .or_else(|| defaults.get("label"))
        .cloned()
        .unwrap_or_else(|| node.id.clone())
}

fn effective_shape(node: &DotNode, defaults: &HashMap<String, String>) -> String {
    node.attrs
        .get("shape")
        .or_else(|| defaults.get("shape"))
        .cloned()
        .unwrap_or_default()
}

fn effective_fill(node: &DotNode, defaults: &HashMap<String, String>) -> String {
    let has_filled_style = node
        .attrs
        .get("style")
        .or_else(|| defaults.get("style"))
        .map(|s| s.contains("filled"))
        .unwrap_or(false);

    if has_filled_style {
        node.attrs
            .get("fillcolor")
            .or_else(|| defaults.get("fillcolor"))
            .or_else(|| node.attrs.get("color"))
            .cloned()
            .unwrap_or_else(|| DEFAULT_FILL.to_string())
    } else {
        node.attrs
            .get("fillcolor")
            .or_else(|| defaults.get("fillcolor"))
            .cloned()
            .unwrap_or_else(|| DEFAULT_FILL.to_string())
    }
}
