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
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
const NOTE_W: f64 = 100.0;

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
    if diagram.nodes.is_empty() && diagram.notes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Check skinparams.
    let is_handwritten = diagram
        .meta
        .skinparams
        .iter()
        .any(|sp| sp.key.to_lowercase() == "handwritten" && sp.value.to_lowercase() == "true");

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
    let cols = if n == 0 { 1 } else { (n as f64).sqrt().ceil() as usize };
    let rows = if n == 0 { 0 } else { n.div_ceil(cols) };

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

    let title_h = if diagram.meta.title.is_some() { TITLE_HEIGHT } else { 0.0 };
    let header_h = if diagram.meta.header.is_some() { TITLE_HEIGHT } else { 0.0 };
    let footer_h = if diagram.meta.footer.is_some() { TITLE_HEIGHT } else { 0.0 };

    // Estimate note space: each note gets one row per text line plus padding.
    let note_line_h = SMALL_FONT + 4.0;
    let note_extra_h: f64 = diagram.notes.iter().map(|note| {
        let nlines = note.text.lines().count().max(1);
        PADDING * 2.0 + (nlines as f64) * note_line_h + 10.0
    }).sum();

    let nodes_w = if col_w.is_empty() { NODE_MIN_W } else {
        col_w.iter().sum::<f64>() + GAP * cols.saturating_sub(1) as f64
    };
    let nodes_h = if row_h.is_empty() { 0.0 } else {
        row_h.iter().sum::<f64>() + GAP * rows.saturating_sub(1) as f64
    };

    let total_w = MARGIN * 2.0 + nodes_w.max(NOTE_W + 20.0);
    let total_h = MARGIN * 2.0 + nodes_h + title_h + header_h + footer_h + note_extra_h;

    let mut svg = SvgBuilder::new(total_w, total_h);

    // Header.
    if let Some(header) = &diagram.meta.header {
        svg.text(total_w / 2.0, header_h - 4.0, header, "middle", SMALL_FONT);
    }

    // Footer.
    if let Some(footer) = &diagram.meta.footer {
        svg.text(total_w / 2.0, total_h - 4.0, footer, "middle", SMALL_FONT);
    }

    // Handwritten warning.
    if is_handwritten {
        let nbsp = '\u{00a0}';
        let msg = format!(
            "Please{n}use{n}'!option{n}handwritten{n}true'{n}to{n}enable{n}handwritten",
            n = nbsp
        );
        svg.text(total_w / 2.0, header_h + title_h + MARGIN + FONT_SIZE, &msg, "middle", SMALL_FONT);
    }

    // Title.
    if let Some(title) = &diagram.meta.title {
        svg.text(total_w / 2.0, header_h + TITLE_HEIGHT - 4.0, title, "middle", TITLE_FONT_SIZE);
    }

    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let content_top = header_h + title_h + MARGIN;

    for (i, root) in roots.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = content_top + row_h[..row].iter().sum::<f64>() + GAP * row as f64;
        let w = col_w[col];
        render_node(&root.id, &diagram.nodes, x, y, w, &mut svg, theme, &mut positions);
    }

    let gs = &theme.global;

    // Render connections.
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

    // Render notes.
    let note_fill = "#FEFECE";
    let mut note_y = content_top + nodes_h + if nodes_h > 0.0 { GAP } else { 0.0 };

    for note in &diagram.notes {
        let lines: Vec<&str> = note.text.lines().collect();
        let nlines = lines.len().max(1);
        let nh = PADDING * 2.0 + (nlines as f64) * note_line_h;
        let nw = lines
            .iter()
            .map(|l| metrics::text_width(l, SMALL_FONT) + PADDING * 2.0)
            .fold(NOTE_W, |a, b| a.max(b))
            .min(total_w - MARGIN * 2.0);
        let nx = MARGIN;
        let ny = note_y;

        // Determine where to place the note box.
        let (box_x, box_y, has_target, tgt_cx, tgt_top) =
            if let Some(target_id) = &note.target {
                if let Some(&(tgt_x, tgt_y, tgt_w, _)) = positions.get(target_id) {
                    let note_cx = (tgt_x + tgt_w / 2.0).min(total_w - MARGIN - nw);
                    (note_cx, ny, true, tgt_x + tgt_w / 2.0, tgt_y)
                } else {
                    (nx, ny, false, 0.0, 0.0)
                }
            } else {
                (nx, ny, false, 0.0, 0.0)
            };

        // Draw the note box.
        svg.rect(box_x, box_y, nw, nh, note_fill, &gs.border_color);

        // Draw each line of text.
        for (i, line) in lines.iter().enumerate() {
            let ly = box_y + PADDING + (i as f64) * note_line_h + SMALL_FONT;
            svg.text(box_x + PADDING, ly, line, "start", SMALL_FONT);
        }

        // If attached, draw a dashed connector to the target element.
        if has_target {
            svg.line_segment(
                box_x + nw / 2.0,
                box_y,
                tgt_cx,
                tgt_top,
                &gs.border_color,
                true,
            );
        }

        note_y += nh + 10.0;
    }

    // Render legend.
    if let Some(legend) = &diagram.meta.legend {
        let legend_x = MARGIN;
        let mut legend_y = note_y + 10.0;
        for line in legend.lines() {
            let t = line.trim();
            if t.is_empty() { continue; }
            // Strip leading/trailing `|` from table rows.
            let t = t.trim_matches('|').trim();
            if t.is_empty() { continue; }
            // Split by `|` for table cells.
            let cells: Vec<&str> = t.split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
            let mut cell_x = legend_x;
            for cell in cells {
                svg.text(cell_x, legend_y, cell, "start", SMALL_FONT);
                let cell_w = metrics::text_width(cell, SMALL_FONT) + 20.0;
                cell_x += cell_w;
            }
            legend_y += 14.0;
        }
    }

    svg.finalize()
}
