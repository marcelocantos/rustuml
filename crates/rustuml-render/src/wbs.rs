// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! WBS (Work Breakdown Structure) diagram SVG renderer.
//!
//! Layout model
//! ============
//! The root node sits at the top centre.  A vertical line descends from its
//! bottom to a horizontal "spine".  Children of the root that carry
//! `WbsSide::Right` hang below the spine to the **right** of the root centre;
//! children with `WbsSide::Left` hang to the **left** (in reverse declaration
//! order, matching PlantUML's behaviour — first declared left child is
//! rightmost / closest to the root).
//!
//! Each non-root subtree is itself laid out top-down with children spread
//! horizontally beneath their parent, connected by orthogonal elbow connectors.

use rustuml_parser::diagram::wbs::{WbsDiagram, WbsNode, WbsSide};

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ─── Layout constants ────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 13.0;
const BOX_PAD_X: f64 = 10.0;
const BOX_PAD_Y: f64 = 6.0;
const BOX_RX: f64 = 4.0;
const H_GAP: f64 = 20.0;
const V_GAP: f64 = 20.0;
const MARGIN: f64 = 20.0;

const FILL_ROOT: &str = "#FFEF99";
const FILL_L2: &str = "#F1F1F1";
const FILL_DEEP: &str = "#F1F1F1";
const STROKE: &str = "#181818";

// ─── Public entry point ──────────────────────────────────────────────────────

pub fn render(diagram: &WbsDiagram, _theme: &Theme) -> String {
    if diagram.nodes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let mut y_cursor = MARGIN;
    let mut roots_layout: Vec<RootLayout> = Vec::new();
    let mut canvas_w = 0.0_f64;

    for root in &diagram.nodes {
        let rl = compute_root_layout(root, y_cursor);
        canvas_w = canvas_w.max(rl.canvas_w);
        y_cursor += rl.canvas_h + MARGIN;
        roots_layout.push(rl);
    }
    let canvas_h = y_cursor; // last root already includes its own bottom margin

    let mut svg = SvgBuilder::new(canvas_w, canvas_h);
    for rl in &roots_layout {
        draw_root_layout(&mut svg, rl);
    }
    svg.finalize()
}

// ─── Sub-tree measurement ────────────────────────────────────────────────────

struct Subtree {
    label: String,
    depth: usize,
    box_w: f64,
    box_h: f64,
    total_w: f64,
    total_h: f64,
    children: Vec<Subtree>,
}

fn measure_subtree(node: &WbsNode) -> Subtree {
    let text_w = metrics::text_width(&node.label, FONT_SIZE);
    let box_w = text_w + BOX_PAD_X * 2.0;
    let box_h = metrics::text_height(FONT_SIZE) + BOX_PAD_Y * 2.0;

    let children: Vec<Subtree> = node.children.iter().map(measure_subtree).collect();

    let children_total_w = if children.is_empty() {
        0.0
    } else {
        children.iter().map(|c| c.total_w).sum::<f64>() + H_GAP * (children.len() - 1) as f64
    };

    let total_w = box_w.max(children_total_w);

    let max_child_h = children.iter().map(|c| c.total_h).fold(0.0_f64, f64::max);

    let total_h = if children.is_empty() {
        box_h
    } else {
        box_h + V_GAP + max_child_h
    };

    Subtree {
        label: node.label.clone(),
        depth: node.depth,
        box_w,
        box_h,
        total_w,
        total_h,
        children,
    }
}

fn group_total_w(subtrees: &[Subtree]) -> f64 {
    if subtrees.is_empty() {
        return 0.0;
    }
    subtrees.iter().map(|s| s.total_w).sum::<f64>() + H_GAP * (subtrees.len() - 1) as f64
}

// ─── Root layout ─────────────────────────────────────────────────────────────

struct RootLayout {
    root_box_x: f64,
    root_box_y: f64,
    root_box_w: f64,
    root_box_h: f64,
    root_label: String,
    spine_y: f64,
    child_y: f64,
    right: Vec<(f64, Subtree)>,
    left: Vec<(f64, Subtree)>,
    canvas_w: f64,
    canvas_h: f64,
}

fn compute_root_layout(root: &WbsNode, offset_y: f64) -> RootLayout {
    let root_box_w = metrics::text_width(&root.label, FONT_SIZE) + BOX_PAD_X * 2.0;
    let root_box_h = metrics::text_height(FONT_SIZE) + BOX_PAD_Y * 2.0;

    let right_subtrees: Vec<Subtree> = root
        .children
        .iter()
        .filter(|n| n.side == WbsSide::Right)
        .map(measure_subtree)
        .collect();

    let left_subtrees: Vec<Subtree> = root
        .children
        .iter()
        .filter(|n| n.side == WbsSide::Left)
        .map(measure_subtree)
        .collect();

    let right_total_w = group_total_w(&right_subtrees);
    let left_total_w = group_total_w(&left_subtrees);

    let lr_gap = if !left_subtrees.is_empty() && !right_subtrees.is_empty() {
        H_GAP
    } else {
        0.0
    };
    let children_total_w = left_total_w + lr_gap + right_total_w;
    let total_span = children_total_w.max(root_box_w);
    let canvas_w = total_span + 2.0 * MARGIN;

    let children_left = MARGIN + (total_span - children_total_w) / 2.0;
    let right_start = children_left + left_total_w + lr_gap;

    let root_box_x = if children_total_w > 0.0 {
        children_left + children_total_w / 2.0 - root_box_w / 2.0
    } else {
        MARGIN
    };
    let root_box_y = offset_y;
    let spine_y = root_box_y + root_box_h + V_GAP;
    let child_y = spine_y + V_GAP;

    // Right children: left to right.
    let mut right: Vec<(f64, Subtree)> = Vec::new();
    let mut rx = right_start;
    for s in right_subtrees {
        let w = s.total_w;
        right.push((rx, s));
        rx += w + H_GAP;
    }

    // Left children: right to left (first declared = rightmost = closest to root).
    let mut left: Vec<(f64, Subtree)> = Vec::new();
    let mut lx = children_left + left_total_w;
    for s in left_subtrees {
        lx -= s.total_w;
        left.push((lx, s));
        lx -= H_GAP;
    }

    let max_child_h = right
        .iter()
        .chain(left.iter())
        .map(|(_, s)| s.total_h)
        .fold(0.0_f64, f64::max);

    let canvas_h = if max_child_h > 0.0 {
        child_y - offset_y + max_child_h + MARGIN
    } else {
        root_box_h + MARGIN
    };

    RootLayout {
        root_box_x,
        root_box_y,
        root_box_w,
        root_box_h,
        root_label: root.label.clone(),
        spine_y,
        child_y,
        right,
        left,
        canvas_w,
        canvas_h,
    }
}

// ─── Drawing ─────────────────────────────────────────────────────────────────

fn node_fill(depth: usize) -> &'static str {
    match depth {
        1 => FILL_ROOT,
        2 => FILL_L2,
        _ => FILL_DEEP,
    }
}

fn draw_root_layout(svg: &mut SvgBuilder, rl: &RootLayout) {
    if !rl.right.is_empty() || !rl.left.is_empty() {
        let all_cx: Vec<f64> = rl
            .left
            .iter()
            .chain(rl.right.iter())
            .map(|(x, s)| x + s.total_w / 2.0)
            .collect();
        let leftmost_cx = all_cx.iter().cloned().fold(f64::INFINITY, f64::min);
        let rightmost_cx = all_cx.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        svg.line_segment(
            leftmost_cx,
            rl.spine_y,
            rightmost_cx,
            rl.spine_y,
            STROKE,
            false,
        );

        let root_cx = rl.root_box_x + rl.root_box_w / 2.0;
        svg.line_segment(
            root_cx,
            rl.root_box_y + rl.root_box_h,
            root_cx,
            rl.spine_y,
            STROKE,
            false,
        );

        for (x, subtree) in rl.right.iter().chain(rl.left.iter()) {
            let cx = x + subtree.total_w / 2.0;
            svg.line_segment(cx, rl.spine_y, cx, rl.child_y, STROKE, false);
            draw_subtree(svg, subtree, *x, rl.child_y);
        }
    }

    let root_cx = rl.root_box_x + rl.root_box_w / 2.0;
    let root_cy = rl.root_box_y + rl.root_box_h / 2.0;
    svg.rounded_rect(
        rl.root_box_x,
        rl.root_box_y,
        rl.root_box_w,
        rl.root_box_h,
        BOX_RX,
        node_fill(1),
        STROKE,
    );
    svg.text(
        root_cx,
        root_cy + metrics::text_height(FONT_SIZE) / 2.0 - 1.0,
        &rl.root_label,
        "middle",
        FONT_SIZE,
    );
}

fn draw_subtree(svg: &mut SvgBuilder, node: &Subtree, x: f64, y: f64) {
    let box_x = x + (node.total_w - node.box_w) / 2.0;
    let box_cx = box_x + node.box_w / 2.0;
    let box_cy = y + node.box_h / 2.0;

    svg.rounded_rect(
        box_x,
        y,
        node.box_w,
        node.box_h,
        BOX_RX,
        node_fill(node.depth),
        STROKE,
    );
    svg.text(
        box_cx,
        box_cy + metrics::text_height(FONT_SIZE) / 2.0 - 1.0,
        &node.label,
        "middle",
        FONT_SIZE,
    );

    if node.children.is_empty() {
        return;
    }

    let child_y = y + node.box_h + V_GAP;

    if node.children.len() == 1 {
        let child = &node.children[0];
        let child_x = x + (node.total_w - child.total_w) / 2.0;
        let child_cx = child_x + child.total_w / 2.0;
        svg.line_segment(box_cx, y + node.box_h, child_cx, child_y, STROKE, false);
        draw_subtree(svg, child, child_x, child_y);
    } else {
        let mut child_xs: Vec<f64> = Vec::new();
        let mut cur = x;
        for child in &node.children {
            child_xs.push(cur);
            cur += child.total_w + H_GAP;
        }

        let first_cx = child_xs[0] + node.children[0].total_w / 2.0;
        let last_i = node.children.len() - 1;
        let last_cx = child_xs[last_i] + node.children[last_i].total_w / 2.0;

        svg.line_segment(box_cx, y + node.box_h, box_cx, child_y, STROKE, false);
        svg.line_segment(first_cx, child_y, last_cx, child_y, STROKE, false);

        for (child, &child_x) in node.children.iter().zip(child_xs.iter()) {
            let ccx = child_x + child.total_w / 2.0;
            svg.line_segment(ccx, child_y, ccx, child_y, STROKE, false);
            draw_subtree(svg, child, child_x, child_y);
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

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

    #[test]
    fn empty_wbs_renders_placeholder() {
        use crate::style::Theme;
        use rustuml_parser::diagram::DiagramMeta;
        use rustuml_parser::diagram::wbs::WbsDiagram;
        let d = WbsDiagram {
            meta: DiagramMeta::default(),
            nodes: vec![],
        };
        let svg = super::render(&d, &Theme::default());
        assert!(svg.contains("<svg"));
    }
}
