// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Work Breakdown Structure (WBS) SVG renderer.
//!
//! Output matches PlantUML's exact SVG structure: root box at top centre, a
//! short vertical drop to a horizontal spine, then verticals from the spine
//! down to each child box.

use std::fmt::Write;

use rustuml_parser::diagram::wbs::{WbsDiagram, WbsNode, WbsSide};

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::text_render;

const FONT_SIZE: f64 = 12.0;
const PAD_X: f64 = 10.0;
const BOX_H: f64 = 34.1328;
const SPINE_DROP: f64 = 20.0;
const H_GAP: f64 = 20.0;
const MARGIN: f64 = 20.0;

const FILL_DEFAULT: &str = "#F1F1F1";
const STROKE: &str = "#181818";

struct Subtree {
    label: String,
    text_w: f64,
    box_w: f64,
    total_w: f64,
    total_h: f64,
    children: Vec<Subtree>,
}

fn measure_subtree(node: &WbsNode) -> Subtree {
    let text_w = pm::text_width(&node.label, FONT_SIZE, false);
    let box_w = text_w + PAD_X * 2.0;
    let children: Vec<Subtree> = node.children.iter().map(measure_subtree).collect();
    let children_total_w = if children.is_empty() {
        0.0
    } else {
        children.iter().map(|c| c.total_w).sum::<f64>() + H_GAP * (children.len() - 1) as f64
    };
    let total_w = box_w.max(children_total_w);
    let max_child_h = children.iter().map(|c| c.total_h).fold(0.0_f64, f64::max);
    let total_h = if children.is_empty() {
        BOX_H
    } else {
        BOX_H + SPINE_DROP * 2.0 + max_child_h
    };
    Subtree {
        label: node.label.clone(),
        text_w,
        box_w,
        total_w,
        total_h,
        children,
    }
}

fn emit_line(buf: &mut String, x1: f64, y1: f64, x2: f64, y2: f64) {
    write!(
        buf,
        r#"<line style="stroke:{STROKE};stroke-width:1.5;" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"/>"#,
        x1 = pm::fmt_coord(x1),
        x2 = pm::fmt_coord(x2),
        y1 = pm::fmt_coord(y1),
        y2 = pm::fmt_coord(y2),
    )
    .unwrap();
}

fn emit_box(buf: &mut String, x: f64, y: f64, w: f64, label: &str, text_w: f64) {
    write!(
        buf,
        r#"<rect fill="{FILL_DEFAULT}" height="{h}" style="stroke:{STROKE};stroke-width:1.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(BOX_H),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    )
    .unwrap();
    let text_x = x + PAD_X;
    let text_y = y + 21.6016;
    let _ = text_w; // width is recomputed inside emit_text via the creole segmenter
    text_render::emit_text(
        buf,
        label,
        &text_render::TextBase {
            x: text_x,
            y: text_y,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
}

fn render_node(buf: &mut String, node: &Subtree, cx: f64, top_y: f64) {
    let box_x = cx - node.box_w / 2.0;
    if node.children.is_empty() {
        emit_box(buf, box_x, top_y, node.box_w, &node.label, node.text_w);
        return;
    }
    let kids_total_w: f64 = node.children.iter().map(|c| c.total_w).sum::<f64>()
        + H_GAP * (node.children.len() - 1) as f64;
    let kids_left = cx - kids_total_w / 2.0;
    let parent_bottom = top_y + BOX_H;
    let spine_y = parent_bottom + SPINE_DROP;
    let child_top_y = spine_y + SPINE_DROP;
    let mut child_xs: Vec<f64> = Vec::with_capacity(node.children.len());
    let mut cursor = kids_left;
    for child in &node.children {
        let ccx = cursor + child.total_w / 2.0;
        child_xs.push(ccx);
        cursor += child.total_w + H_GAP;
    }
    for (child, ccx) in node.children.iter().zip(child_xs.iter()) {
        emit_line(buf, *ccx, spine_y, *ccx, child_top_y);
        render_node(buf, child, *ccx, child_top_y);
    }
    if child_xs.len() >= 2 {
        let first = *child_xs.first().unwrap();
        let last = *child_xs.last().unwrap();
        emit_line(buf, first, spine_y, last, spine_y);
    }
    emit_box(buf, box_x, top_y, node.box_w, &node.label, node.text_w);
    emit_line(buf, cx, parent_bottom, cx, spine_y);
}

struct RootLayout {
    tree: Subtree,
    right_indices: Vec<usize>,
    left_indices: Vec<usize>,
    canvas_h: f64,
    offset_y: f64,
}

fn group_w(tree: &Subtree, indices: &[usize]) -> f64 {
    if indices.is_empty() {
        0.0
    } else {
        indices
            .iter()
            .map(|&i| tree.children[i].total_w)
            .sum::<f64>()
            + H_GAP * (indices.len() - 1) as f64
    }
}

fn compute_root_layout(root_node: &WbsNode, offset_y: f64) -> (RootLayout, f64) {
    let tree = measure_subtree(root_node);
    let right_indices: Vec<usize> = root_node
        .children
        .iter()
        .enumerate()
        .filter(|(_, n)| n.side == WbsSide::Right)
        .map(|(i, _)| i)
        .collect();
    let left_indices: Vec<usize> = root_node
        .children
        .iter()
        .enumerate()
        .filter(|(_, n)| n.side == WbsSide::Left)
        .map(|(i, _)| i)
        .collect();
    let right_w = group_w(&tree, &right_indices);
    let left_w = group_w(&tree, &left_indices);
    let lr_gap = if right_w > 0.0 && left_w > 0.0 {
        H_GAP
    } else {
        0.0
    };
    let children_total_w = right_w + lr_gap + left_w;
    let total_span = children_total_w.max(tree.box_w);
    let canvas_w = total_span + 2.0 * MARGIN;
    let canvas_h = if root_node.children.is_empty() {
        BOX_H
    } else {
        let max_child_h = tree
            .children
            .iter()
            .map(|c| c.total_h)
            .fold(0.0_f64, f64::max);
        BOX_H + SPINE_DROP * 2.0 + max_child_h
    };
    (
        RootLayout {
            tree,
            right_indices,
            left_indices,
            canvas_h,
            offset_y,
        },
        canvas_w,
    )
}

fn render_root(buf: &mut String, rl: &RootLayout, canvas_w: f64) {
    let tree = &rl.tree;
    let root_top_y = rl.offset_y;
    let root_box_w = tree.box_w;
    let right_w = group_w(tree, &rl.right_indices);
    let left_w = group_w(tree, &rl.left_indices);
    let lr_gap = if right_w > 0.0 && left_w > 0.0 {
        H_GAP
    } else {
        0.0
    };
    let children_total_w = right_w + lr_gap + left_w;
    let canvas_centre = canvas_w / 2.0;
    let children_left = canvas_centre - children_total_w / 2.0;
    let root_cx = if children_total_w > 0.0 {
        children_left + children_total_w / 2.0
    } else {
        canvas_centre
    };
    let root_box_x = root_cx - root_box_w / 2.0;
    if tree.children.is_empty() {
        emit_box(
            buf,
            root_box_x,
            root_top_y,
            root_box_w,
            &tree.label,
            tree.text_w,
        );
        return;
    }
    let mut left_cxs: Vec<f64> = Vec::new();
    {
        let mut x = children_left + left_w;
        for &i in &rl.left_indices {
            let tw = tree.children[i].total_w;
            let ccx = x - tw / 2.0;
            left_cxs.push(ccx);
            x -= tw + H_GAP;
        }
    }
    let right_start = children_left + left_w + lr_gap;
    let mut right_cxs: Vec<f64> = Vec::new();
    {
        let mut x = right_start;
        for &i in &rl.right_indices {
            let tw = tree.children[i].total_w;
            let ccx = x + tw / 2.0;
            right_cxs.push(ccx);
            x += tw + H_GAP;
        }
    }
    let spine_y = root_top_y + BOX_H + SPINE_DROP;
    let child_top_y = spine_y + SPINE_DROP;
    for (idx, &i) in rl.right_indices.iter().enumerate() {
        let ccx = right_cxs[idx];
        emit_line(buf, ccx, spine_y, ccx, child_top_y);
        render_node(buf, &tree.children[i], ccx, child_top_y);
    }
    for (idx, &i) in rl.left_indices.iter().enumerate() {
        let ccx = left_cxs[idx];
        emit_line(buf, ccx, spine_y, ccx, child_top_y);
        render_node(buf, &tree.children[i], ccx, child_top_y);
    }
    let all_cxs: Vec<f64> = left_cxs.iter().chain(right_cxs.iter()).cloned().collect();
    if all_cxs.len() >= 2 {
        let leftmost = all_cxs.iter().cloned().fold(f64::INFINITY, f64::min);
        let rightmost = all_cxs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        emit_line(buf, leftmost, spine_y, rightmost, spine_y);
    }
    emit_box(
        buf,
        root_box_x,
        root_top_y,
        root_box_w,
        &tree.label,
        tree.text_w,
    );
    emit_line(buf, root_cx, root_top_y + BOX_H, root_cx, spine_y);
}

/// Render a WBS diagram with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, replay the body
/// verbatim inside the PlantUML envelope. Otherwise fall back to the
/// geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &WbsDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "WBS");
    }
    render(diagram, theme)
}

pub fn render(diagram: &WbsDiagram, _theme: &Theme) -> String {
    if diagram.nodes.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="WBS" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><?plantuml ?><defs/><g></g></svg>"#.to_string();
    }
    let mut roots_layout: Vec<RootLayout> = Vec::new();
    let mut canvas_w = 0.0_f64;
    let mut y_cursor = MARGIN;
    for root_node in &diagram.nodes {
        let (rl, this_canvas_w) = compute_root_layout(root_node, y_cursor);
        canvas_w = canvas_w.max(this_canvas_w);
        y_cursor += rl.canvas_h + MARGIN;
        roots_layout.push(rl);
    }
    let total_w_i = canvas_w.ceil() as i64;
    let total_h_i = y_cursor.ceil() as i64;
    let mut buf = String::with_capacity(2048);
    write!(
        buf,
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="WBS" height="{total_h_i}px" preserveAspectRatio="none" style="width:{total_w_i}px;height:{total_h_i}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {total_w_i} {total_h_i}" width="{total_w_i}px" zoomAndPan="magnify"><?plantuml ?><defs/><g>"##,
    )
    .unwrap();
    for rl in &roots_layout {
        render_root(&mut buf, rl, canvas_w);
    }
    buf.push_str("</g></svg>");
    buf
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_simple_wbs() {
        let input = "@startwbs\n* Project\n** Phase 1\n*** Task A\n** Phase 2\n@endwbs";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Project"));
        assert!(svg.contains("Phase 1"));
        assert!(svg.contains("Task A"));
        assert!(svg.contains("Phase 2"));
        assert!(svg.contains(r#"data-diagram-type="WBS""#));
    }

    #[test]
    fn renders_left_branches() {
        let input = "@startwbs\n* Central\n** Right 1\n-- Left 1\n@endwbs";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Central"));
        assert!(svg.contains("Right 1"));
        assert!(svg.contains("Left 1"));
    }
}
