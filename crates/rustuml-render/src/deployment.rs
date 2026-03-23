// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram SVG renderer.

use std::collections::{HashMap, HashSet};

use rustuml_parser::diagram::deployment::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const NODE_MIN_W: f64 = 100.0;
const NODE_H: f64 = 40.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 12.0;
const LABEL_H: f64 = 22.0;
const CONTAINER_PADDING: f64 = 16.0;

fn node_fill(kind: DeploymentNodeKind) -> &'static str {
    match kind {
        DeploymentNodeKind::Cloud => "#E8F4FD",
        DeploymentNodeKind::Database => "#D5F5E3",
        DeploymentNodeKind::Storage => "#FEF9E7",
        _ => "#F8F9FA",
    }
}

fn node_label_w(node: &DeploymentNode) -> f64 {
    let lw = metrics::text_width(&node.label, FONT_SIZE) + PADDING * 2.0;
    let sw = node
        .stereotype
        .as_deref()
        .map(|s| metrics::text_width(&format!("«{s}»"), SMALL_FONT) + PADDING * 2.0)
        .unwrap_or(0.0);
    lw.max(sw).max(NODE_MIN_W)
}

/// Returns (width, height) for this node including all children.
fn node_size(id: &str, nodes: &[DeploymentNode]) -> (f64, f64) {
    let node = match nodes.iter().find(|n| n.id == id) {
        Some(n) => n,
        None => return (NODE_MIN_W, NODE_H),
    };
    if node.children.is_empty() {
        return (node_label_w(node), NODE_H);
    }
    let mut inner_h = 0.0_f64;
    let mut max_child_w = 0.0_f64;
    for (i, child_id) in node.children.iter().enumerate() {
        let (cw, ch) = node_size(child_id, nodes);
        if i > 0 {
            inner_h += GAP;
        }
        inner_h += ch;
        max_child_w = max_child_w.max(cw);
    }
    let w = node_label_w(node).max(max_child_w + CONTAINER_PADDING * 2.0);
    let h = LABEL_H + CONTAINER_PADDING + inner_h + CONTAINER_PADDING;
    (w, h)
}

fn render_leaf_label(node: &DeploymentNode, svg: &mut SvgBuilder, x: f64, y: f64, w: f64) {
    if let Some(stereo) = &node.stereotype {
        svg.text(
            x + w / 2.0,
            y + NODE_H / 2.0 - 4.0,
            &format!("«{stereo}»"),
            "middle",
            SMALL_FONT,
        );
        svg.text(
            x + w / 2.0,
            y + NODE_H / 2.0 + 10.0,
            &node.label,
            "middle",
            FONT_SIZE,
        );
    } else {
        svg.text(
            x + w / 2.0,
            y + NODE_H / 2.0 + 5.0,
            &node.label,
            "middle",
            FONT_SIZE,
        );
    }
}

fn render_container_label(node: &DeploymentNode, svg: &mut SvgBuilder, x: f64, y: f64) {
    if let Some(stereo) = &node.stereotype {
        svg.text(
            x + CONTAINER_PADDING,
            y + LABEL_H - 4.0,
            &format!("«{stereo}»"),
            "start",
            SMALL_FONT,
        );
        svg.text(
            x + CONTAINER_PADDING,
            y + LABEL_H + 9.0,
            &node.label,
            "start",
            FONT_SIZE,
        );
    } else {
        svg.text(
            x + CONTAINER_PADDING,
            y + LABEL_H - 4.0,
            &node.label,
            "start",
            FONT_SIZE,
        );
    }
}

fn render_node(
    id: &str,
    nodes: &[DeploymentNode],
    x: f64,
    y: f64,
    w: f64,
    svg: &mut SvgBuilder,
    theme: &Theme,
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
) {
    let node = match nodes.iter().find(|n| n.id == id) {
        Some(n) => n,
        None => return,
    };
    let (natural_w, h) = node_size(id, nodes);
    let w = w.max(natural_w);
    let fill = node_fill(node.kind);
    let gs = &theme.global;
    positions.insert(id.to_string(), (x, y, w, h));

    if node.children.is_empty() {
        svg.rounded_rect(x, y, w, NODE_H, 5.0, fill, &gs.border_color);
        render_leaf_label(node, svg, x, y, w);
    } else {
        svg.rounded_rect(x, y, w, h, 5.0, fill, &gs.border_color);
        render_container_label(node, svg, x, y);
        let inner_x = x + CONTAINER_PADDING;
        let inner_w = w - CONTAINER_PADDING * 2.0;
        let mut child_y = y + LABEL_H + CONTAINER_PADDING;
        // Clone children to avoid borrow conflict while mutating positions.
        let children: Vec<String> = node.children.clone();
        for child_id in children {
            let (_, ch) = node_size(&child_id, nodes);
            render_node(&child_id, nodes, inner_x, child_y, inner_w, svg, theme, positions);
            child_y += ch + GAP;
        }
    }
}

pub fn render(diagram: &DeploymentDiagram, theme: &Theme) -> String {
    if diagram.nodes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Find root nodes (not listed as children of any other node).
    let all_children: HashSet<&str> = diagram
        .nodes
        .iter()
        .flat_map(|n| n.children.iter().map(|s| s.as_str()))
        .collect();
    let roots: Vec<&DeploymentNode> = diagram
        .nodes
        .iter()
        .filter(|n| !all_children.contains(n.id.as_str()))
        .collect();

    let n = roots.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, root) in roots.iter().enumerate() {
            let (w, _) = node_size(&root.id, &diagram.nodes);
            cw[i % cols] = cw[i % cols].max(w);
        }
        cw
    };
    let row_h: Vec<f64> = {
        let mut rh = vec![0.0_f64; rows];
        for (i, root) in roots.iter().enumerate() {
            let (_, h) = node_size(&root.id, &diagram.nodes);
            rh[i / cols] = rh[i / cols].max(h);
        }
        rh
    };

    let total_w =
        MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * cols.saturating_sub(1) as f64;
    let total_h =
        MARGIN * 2.0 + row_h.iter().sum::<f64>() + GAP * rows.saturating_sub(1) as f64;

    let mut svg = SvgBuilder::new(total_w, total_h);
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    for (i, root) in roots.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = MARGIN + row_h[..row].iter().sum::<f64>() + GAP * row as f64;
        let w = col_w[col];
        render_node(&root.id, &diagram.nodes, x, y, w, &mut svg, theme, &mut positions);
    }

    let gs = &theme.global;
    for conn in &diagram.connections {
        if let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, _))) =
            (positions.get(&conn.from), positions.get(&conn.to))
        {
            svg.line_segment(
                fx + fw / 2.0,
                fy + fh,
                tx + tw / 2.0,
                ty,
                &gs.border_color,
                false,
            );
            svg.arrow_head(tx + tw / 2.0, ty, 90.0);
            if let Some(label) = &conn.label {
                let mx = (fx + fw / 2.0 + tx + tw / 2.0) / 2.0;
                let my = (fy + fh + ty) / 2.0;
                svg.text(mx, my - 4.0, label, "middle", SMALL_FONT);
            }
        }
    }

    svg.finalize()
}
