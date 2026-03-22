// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! WBS (Work Breakdown Structure) diagram SVG renderer.
//!
//! Lays out the tree top-down with boxes connected by vertical/horizontal
//! lines.  Each node is a rounded rectangle with its label centred inside.

use rustuml_parser::diagram::wbs::{WbsDiagram, WbsNode};

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ─── Layout constants ────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 13.0;
const BOX_PAD_X: f64 = 12.0; // horizontal padding inside each box
const BOX_PAD_Y: f64 = 7.0; // vertical padding inside each box
const BOX_RX: f64 = 4.0; // corner radius
const H_GAP: f64 = 20.0; // horizontal gap between sibling boxes
const V_GAP: f64 = 40.0; // vertical gap between parent and children
const MARGIN: f64 = 20.0; // outer margin

// Colours (PlantUML WBS defaults use yellow tones for different levels).
const FILL_ROOT: &str = "#FFEF99";
const FILL_L2: &str = "#D4E6F1";
const FILL_DEEP: &str = "#F0F0F0";
const STROKE: &str = "#181818";

// ─── Public entry point ──────────────────────────────────────────────────────

pub fn render(diagram: &WbsDiagram, _theme: &Theme) -> String {
    if diagram.nodes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Measure every node's natural box size and build a layout tree.
    let layout_roots: Vec<LayoutNode> = diagram.nodes.iter().map(measure).collect();

    // Assign (x, y) positions for every node using a two-pass approach.
    // Pass 1 — bottom-up: compute the total sub-tree width of each node.
    // Pass 2 — top-down: assign x by centring children under their parent.
    let mut positioned: Vec<PlacedNode> = layout_roots
        .iter()
        .map(|lr| position(lr, MARGIN, MARGIN))
        .collect();

    // If multiple roots, stack them horizontally with H_GAP between.
    // Re-position from left margin.
    let mut x_cursor = MARGIN;
    for p in &mut positioned {
        let dx = x_cursor - p.x;
        shift_x(p, dx);
        x_cursor += p.subtree_w + H_GAP;
    }

    // Compute canvas size.
    let total_w = x_cursor - H_GAP + MARGIN;
    let total_h = max_y(&positioned) + MARGIN;

    let mut svg = SvgBuilder::new(total_w, total_h);
    for p in &positioned {
        draw(&mut svg, p);
    }

    svg.finalize()
}

// ─── Measurement pass ────────────────────────────────────────────────────────

struct LayoutNode {
    label: String,
    depth: usize,
    box_w: f64,
    box_h: f64,
    children: Vec<LayoutNode>,
    /// Total width required by this subtree (box + children side-by-side).
    subtree_w: f64,
}

fn measure(node: &WbsNode) -> LayoutNode {
    let text_w = metrics::text_width(&node.label, FONT_SIZE);
    let box_w = text_w + BOX_PAD_X * 2.0;
    let box_h = metrics::text_height(FONT_SIZE) + BOX_PAD_Y * 2.0;

    let children: Vec<LayoutNode> = node.children.iter().map(measure).collect();

    let children_total_w = if children.is_empty() {
        0.0
    } else {
        children.iter().map(|c| c.subtree_w).sum::<f64>() + H_GAP * (children.len() - 1) as f64
    };

    let subtree_w = box_w.max(children_total_w);

    LayoutNode {
        label: node.label.clone(),
        depth: node.depth,
        box_w,
        box_h,
        children,
        subtree_w,
    }
}

// ─── Positioning pass ────────────────────────────────────────────────────────

struct PlacedNode {
    label: String,
    depth: usize,
    /// Top-left corner of the box.
    x: f64,
    y: f64,
    box_w: f64,
    box_h: f64,
    subtree_w: f64,
    children: Vec<PlacedNode>,
}

fn position(node: &LayoutNode, left: f64, top: f64) -> PlacedNode {
    // Centre the box within its subtree_w allocation.
    let box_x = left + (node.subtree_w - node.box_w) / 2.0;
    let box_y = top;

    // Lay children out left-to-right underneath.
    let child_y = top + node.box_h + V_GAP;
    let mut child_x = left;
    let mut placed_children = Vec::new();
    for child in &node.children {
        let pc = position(child, child_x, child_y);
        child_x += child.subtree_w + H_GAP;
        placed_children.push(pc);
    }

    PlacedNode {
        label: node.label.clone(),
        depth: node.depth,
        x: box_x,
        y: box_y,
        box_w: node.box_w,
        box_h: node.box_h,
        subtree_w: node.subtree_w,
        children: placed_children,
    }
}

fn shift_x(node: &mut PlacedNode, dx: f64) {
    node.x += dx;
    for c in &mut node.children {
        shift_x(c, dx);
    }
}

fn max_y(nodes: &[PlacedNode]) -> f64 {
    nodes
        .iter()
        .map(|n| {
            let own = n.y + n.box_h;
            let child_max = max_y(&n.children);
            own.max(child_max)
        })
        .fold(0.0_f64, f64::max)
}

// ─── Drawing pass ────────────────────────────────────────────────────────────

fn node_fill(depth: usize) -> &'static str {
    match depth {
        1 => FILL_ROOT,
        2 => FILL_L2,
        _ => FILL_DEEP,
    }
}

fn draw(svg: &mut SvgBuilder, node: &PlacedNode) {
    let fill = node_fill(node.depth);
    let cx = node.x + node.box_w / 2.0;
    let cy = node.y + node.box_h / 2.0;

    // Box.
    svg.rounded_rect(node.x, node.y, node.box_w, node.box_h, BOX_RX, fill, STROKE);
    // Label — vertically centred.
    svg.text(
        cx,
        cy + metrics::text_height(FONT_SIZE) / 2.0 - 1.0,
        &node.label,
        "middle",
        FONT_SIZE,
    );

    // Connector lines to children.
    for child in &node.children {
        let parent_bottom_x = node.x + node.box_w / 2.0;
        let parent_bottom_y = node.y + node.box_h;
        let child_top_x = child.x + child.box_w / 2.0;
        let child_top_y = child.y;
        let mid_y = (parent_bottom_y + child_top_y) / 2.0;

        // Draw an orthogonal connector: down, horizontal elbow, down.
        svg.line_segment(
            parent_bottom_x,
            parent_bottom_y,
            parent_bottom_x,
            mid_y,
            STROKE,
            false,
        );
        svg.line_segment(parent_bottom_x, mid_y, child_top_x, mid_y, STROKE, false);
        svg.line_segment(child_top_x, mid_y, child_top_x, child_top_y, STROKE, false);

        draw(svg, child);
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
