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
/// Exact box height: text height (14.1328125 at size 12) + 20px vertical
/// padding.  Kept un-rounded so accumulated row offsets match PlantUML's
/// per-coordinate HALF_UP rounding.
const BOX_H: f64 = 34.1328125;
/// Baseline offset inside a box: (box_h − text_height)/2 + ascent.
const TEXT_BASELINE: f64 = 21.6015625;
const SPINE_DROP: f64 = 20.0;
const H_GAP: f64 = 20.0;
const MARGIN: f64 = 20.0;
/// Vertical gap between stacked boxes; stride = `BOX_H + V_GAP`.
const V_GAP: f64 = 15.0;
const V_STRIDE: f64 = BOX_H + V_GAP;
/// Horizontal indent of a child box past its parent's centre (also the
/// length of the elbow stub line).
const INDENT: f64 = 10.0;

const FILL_DEFAULT: &str = "#F1F1F1";
const STROKE: &str = "#181818";

struct Subtree {
    label: String,
    fill: String,
    text_w: f64,
    box_w: f64,
    /// Horizontal extent of the vertical-stack subtree rooted here, measured
    /// from this box's left edge.  Children indent `box_w/2 + INDENT` past the
    /// left edge, so the extent is the wider of this box and the deepest child.
    hwidth: f64,
    /// Number of boxes in the subtree (this node plus all descendants), i.e.
    /// the row count a vertical stack occupies.
    rows: usize,
    /// Whether this depth-2 branch grows from the left (`--` prefix).  Only
    /// meaningful for the root's direct children.
    is_left: bool,
    children: Vec<Subtree>,
}

fn measure_subtree(node: &WbsNode) -> Subtree {
    let text_w = pm::text_width(&node.label, FONT_SIZE, false);
    let box_w = text_w + PAD_X * 2.0;
    let fill = node
        .color
        .as_deref()
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| FILL_DEFAULT.to_string());
    let children: Vec<Subtree> = node.children.iter().map(measure_subtree).collect();
    let child_indent = box_w / 2.0 + INDENT;
    let hwidth = children
        .iter()
        .map(|c| child_indent + c.hwidth)
        .fold(box_w, f64::max);
    let rows = 1 + children.iter().map(|c| c.rows).sum::<usize>();
    Subtree {
        label: node.label.clone(),
        fill,
        text_w,
        box_w,
        hwidth,
        rows,
        is_left: node.side == WbsSide::Left,
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

fn emit_box(buf: &mut String, x: f64, y: f64, w: f64, label: &str, text_w: f64, fill: &str) {
    write!(
        buf,
        r#"<rect fill="{fill}" height="{h}" style="stroke:{STROKE};stroke-width:1.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(BOX_H),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    )
    .unwrap();
    let text_x = x + PAD_X;
    let text_y = y + TEXT_BASELINE;
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

/// Place a depth-≥2 subtree as a vertical stack growing rightward.
///
/// `box_left`/`box_top` position this node's own box; descendants stack below
/// in DFS pre-order at a constant `V_STRIDE`, indented `INDENT` past this
/// node's centre.  Returns the next free box-top below the whole subtree.
fn render_vstack(buf: &mut String, node: &Subtree, box_left: f64, box_top: f64) -> f64 {
    let center_x = box_left + node.box_w / 2.0;
    emit_box(
        buf,
        box_left,
        box_top,
        node.box_w,
        &node.label,
        node.text_w,
        &node.fill,
    );
    if node.children.is_empty() {
        return box_top + V_STRIDE;
    }
    let child_left = center_x + INDENT;
    let mut cursor = box_top + V_STRIDE;
    let mut last_center_y = box_top + BOX_H / 2.0;
    for child in &node.children {
        let child_center_y = cursor + BOX_H / 2.0;
        // Elbow stub from this node's centre out to the child box's left edge.
        emit_line(buf, center_x, child_center_y, child_left, child_center_y);
        last_center_y = child_center_y;
        cursor = render_vstack(buf, child, child_left, cursor);
    }
    // Vertical spine from this box's bottom down to the last child's centre.
    emit_line(buf, center_x, box_top + BOX_H, center_x, last_center_y);
    cursor
}

/// Place one depth-≥2 subtree as a mirror-image vertical stack growing
/// leftward (left-side `--` branches).  `box_right` is the right edge of this
/// node's own box.
fn render_vstack_left(buf: &mut String, node: &Subtree, box_right: f64, box_top: f64) -> f64 {
    let box_left = box_right - node.box_w;
    let center_x = box_right - node.box_w / 2.0;
    emit_box(
        buf,
        box_left,
        box_top,
        node.box_w,
        &node.label,
        node.text_w,
        &node.fill,
    );
    if node.children.is_empty() {
        return box_top + V_STRIDE;
    }
    let child_right = center_x - INDENT;
    let mut cursor = box_top + V_STRIDE;
    let mut last_center_y = box_top + BOX_H / 2.0;
    for child in &node.children {
        let child_center_y = cursor + BOX_H / 2.0;
        emit_line(buf, center_x, child_center_y, child_right, child_center_y);
        last_center_y = child_center_y;
        cursor = render_vstack_left(buf, child, child_right, cursor);
    }
    emit_line(buf, center_x, box_top + BOX_H, center_x, last_center_y);
    cursor
}

struct RootLayout {
    tree: Subtree,
    offset_y: f64,
    /// Total vertical extent of this root's diagram (root box + spine + the
    /// tallest depth-2 stack).
    canvas_h: f64,
    /// Total width (excluding the page margins applied by the caller).
    span_w: f64,
}

fn compute_root_layout(root_node: &WbsNode, offset_y: f64) -> (RootLayout, f64) {
    let tree = measure_subtree(root_node);
    let (right_w, left_w) = side_widths(root_node, &tree);
    let lr_gap = if right_w > 0.0 && left_w > 0.0 {
        H_GAP
    } else {
        0.0
    };
    let span_w = (right_w + lr_gap + left_w).max(tree.box_w);
    let canvas_w = span_w + 2.0 * MARGIN;
    let canvas_h = if root_node.children.is_empty() {
        BOX_H
    } else {
        // Header (root box, two drops) plus the tallest depth-2 stack.
        let max_rows = root_node
            .children
            .iter()
            .zip(tree.children.iter())
            .map(|(_, c)| c.rows)
            .max()
            .unwrap_or(0);
        BOX_H + SPINE_DROP * 2.0 + max_rows as f64 * V_STRIDE - V_GAP
    };
    (
        RootLayout {
            tree,
            offset_y,
            canvas_h,
            span_w,
        },
        canvas_w,
    )
}

/// Sum of the horizontal widths of the right-side and left-side depth-2
/// columns (each separated internally by `H_GAP`).
fn side_widths(root_node: &WbsNode, tree: &Subtree) -> (f64, f64) {
    let mut right = 0.0;
    let mut right_n = 0usize;
    let mut left = 0.0;
    let mut left_n = 0usize;
    for (n, c) in root_node.children.iter().zip(tree.children.iter()) {
        match n.side {
            WbsSide::Right => {
                right += c.hwidth;
                right_n += 1;
            }
            WbsSide::Left => {
                left += c.hwidth;
                left_n += 1;
            }
        }
    }
    if right_n > 1 {
        right += H_GAP * (right_n - 1) as f64;
    }
    if left_n > 1 {
        left += H_GAP * (left_n - 1) as f64;
    }
    (right, left)
}

fn render_root(buf: &mut String, rl: &RootLayout, canvas_w: f64) {
    let tree = &rl.tree;
    let root_top_y = rl.offset_y;
    let root_box_w = tree.box_w;
    let span_left = (canvas_w - rl.span_w) / 2.0;

    if tree.children.is_empty() {
        let root_box_x = canvas_w / 2.0 - root_box_w / 2.0;
        emit_box(
            buf,
            root_box_x,
            root_top_y,
            root_box_w,
            &tree.label,
            tree.text_w,
            &tree.fill,
        );
        return;
    }

    let spine_y = root_top_y + BOX_H + SPINE_DROP;
    let depth2_top = spine_y + SPINE_DROP;

    // Partition depth-2 columns into left- and right-side branches.
    let left_cols: Vec<usize> = (0..tree.children.len())
        .filter(|&i| tree.children[i].is_left)
        .collect();
    let right_cols: Vec<usize> = (0..tree.children.len())
        .filter(|&i| !tree.children[i].is_left)
        .collect();

    let left_block_w = block_w(tree, &left_cols);
    let lr_gap = if !left_cols.is_empty() && !right_cols.is_empty() {
        H_GAP
    } else {
        0.0
    };
    let right_block_start = span_left + left_block_w + lr_gap;

    // Centre x of each depth-2 box.  Left columns occupy the left part of the
    // span (boxes anchored to the right edge of their column, growing left);
    // right columns occupy the right part (boxes anchored to the left edge).
    let mut centers = vec![0.0_f64; tree.children.len()];
    {
        let mut x = span_left;
        for &i in &left_cols {
            let c = &tree.children[i];
            let col_right = x + c.hwidth;
            centers[i] = col_right - c.box_w / 2.0;
            x += c.hwidth + H_GAP;
        }
    }
    {
        let mut x = right_block_start;
        for &i in &right_cols {
            let c = &tree.children[i];
            centers[i] = x + c.box_w / 2.0;
            x += c.hwidth + H_GAP;
        }
    }

    // Emit each depth-2 subtree in source order: drop line then the stack.
    for (i, c) in tree.children.iter().enumerate() {
        let cx = centers[i];
        emit_line(buf, cx, spine_y, cx, depth2_top);
        if c.is_left {
            render_vstack_left(buf, c, cx + c.box_w / 2.0, depth2_top);
        } else {
            render_vstack(buf, c, cx - c.box_w / 2.0, depth2_top);
        }
    }

    let leftmost = centers.iter().cloned().fold(f64::INFINITY, f64::min);
    let rightmost = centers.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Horizontal spine across all depth-2 centres.  PlantUML emits this even
    // for a single child, as a degenerate zero-length line at that centre.
    emit_line(buf, leftmost, spine_y, rightmost, spine_y);

    // Root box centred over the span of depth-2 centres.
    let root_cx = (leftmost + rightmost) / 2.0;
    let root_box_x = root_cx - root_box_w / 2.0;
    emit_box(
        buf,
        root_box_x,
        root_top_y,
        root_box_w,
        &tree.label,
        tree.text_w,
        &tree.fill,
    );
    emit_line(buf, root_cx, root_top_y + BOX_H, root_cx, spine_y);
}

/// Total horizontal width of a set of depth-2 columns (internal `H_GAP`s).
fn block_w(tree: &Subtree, cols: &[usize]) -> f64 {
    if cols.is_empty() {
        return 0.0;
    }
    cols.iter().map(|&i| tree.children[i].hwidth).sum::<f64>() + H_GAP * (cols.len() - 1) as f64
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
