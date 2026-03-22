// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Mind map SVG renderer — horizontal tree layout.
//!
//! The root is placed on the left; branches extend to the right.  Each
//! node is drawn as a rounded rectangle containing its label.  Parent–child
//! edges are smooth cubic Bézier curves.

use rustuml_parser::diagram::mindmap::{MindMapDiagram, MindMapNode};

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ──────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 13.0;
const NODE_PADDING_X: f64 = 10.0;
const NODE_PADDING_Y: f64 = 6.0;
const NODE_H: f64 = FONT_SIZE + NODE_PADDING_Y * 2.0;
const LEVEL_GAP: f64 = 60.0; // horizontal gap between depth levels
const SIBLING_GAP: f64 = 10.0; // vertical gap between sibling nodes
const MARGIN: f64 = 20.0;
const RX: f64 = 6.0; // corner radius

// ── Colour palette (PlantUML mind map style) ─────────────────────────────────

const ROOT_FILL: &str = "#8A9ED4";
const ROOT_STROKE: &str = "#4A5EA0";
const L1_FILLS: &[&str] = &["#A8D5A2", "#F4A460", "#F08080", "#87CEEB", "#DDA0DD"];
const L1_STROKES: &[&str] = &["#4A8A44", "#A0601A", "#A04040", "#3080A0", "#8040A0"];
const DEFAULT_FILL: &str = "#FFFDE7";
const DEFAULT_STROKE: &str = "#888888";
const EDGE_STROKE: &str = "#666666";

// ── Positioned node ───────────────────────────────────────────────────────────

struct Placed {
    /// Top-left corner (layout space — Y increases downward).
    x: f64,
    y: f64,
    w: f64,
    /// Centre-Y of this node's box.
    cy: f64,
    label: String,
    depth: usize,
    /// Index into the depth-1 branch (for colouring).
    branch_index: usize,
    children: Vec<Placed>,
}

impl Placed {
    /// Total vertical span occupied by this subtree.
    fn subtree_h(&self) -> f64 {
        if self.children.is_empty() {
            NODE_H
        } else {
            self.children.iter().map(|c| c.subtree_h()).sum::<f64>()
                + SIBLING_GAP * (self.children.len() - 1) as f64
        }
    }
}

// ── Layout ────────────────────────────────────────────────────────────────────

/// Compute the natural width of a node box.
fn node_w(label: &str) -> f64 {
    metrics::text_width(label, FONT_SIZE) + NODE_PADDING_X * 2.0
}

/// Lay out a subtree rooted at `node`.
///
/// * `x`           — left edge of the node box.
/// * `y_top`       — top of the vertical space allocated to this subtree.
/// * `branch_index`— which depth-1 branch we are in (for colour selection).
fn layout_node(node: &MindMapNode, x: f64, y_top: f64, branch_index: usize) -> Placed {
    let w = node_w(&node.label);
    let child_x = x + w + LEVEL_GAP;

    // First pass: lay out children with their natural sizes to find total height.
    let mut children: Vec<Placed> = Vec::with_capacity(node.children.len());
    let mut cursor = y_top;

    for child in &node.children {
        let placed = layout_node(child, child_x, cursor, branch_index);
        cursor += placed.subtree_h() + SIBLING_GAP;
        children.push(placed);
    }

    // Remove the trailing gap.
    if !children.is_empty() {
        cursor -= SIBLING_GAP;
    }

    let total_h = cursor - y_top;
    let subtree_h = total_h.max(NODE_H);
    let cy = y_top + subtree_h / 2.0;

    Placed {
        x,
        y: cy - NODE_H / 2.0,
        w,
        cy,
        label: node.label.clone(),
        depth: node.depth,
        branch_index,
        children,
    }
}

/// Lay out all roots and return (placed_roots, total_width, total_height).
fn layout(roots: &[MindMapNode]) -> (Vec<Placed>, f64, f64) {
    let mut placed_roots: Vec<Placed> = Vec::with_capacity(roots.len());
    let mut cursor_y = MARGIN;
    let mut max_x = 0.0_f64;

    for (bi, root) in roots.iter().enumerate() {
        let placed = layout_node(root, MARGIN, cursor_y, bi);
        cursor_y += placed.subtree_h() + SIBLING_GAP;
        max_x = max_x.max(subtree_max_x(&placed));
        placed_roots.push(placed);
    }

    let total_h = cursor_y - SIBLING_GAP + MARGIN;
    let total_w = max_x + MARGIN;
    (placed_roots, total_w, total_h)
}

fn subtree_max_x(p: &Placed) -> f64 {
    let self_right = p.x + p.w;
    p.children
        .iter()
        .map(subtree_max_x)
        .fold(self_right, f64::max)
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn fill_for(placed: &Placed) -> (&'static str, &'static str) {
    if placed.depth == 1 {
        (ROOT_FILL, ROOT_STROKE)
    } else if placed.depth == 2 {
        let i = placed.branch_index % L1_FILLS.len();
        (L1_FILLS[i], L1_STROKES[i])
    } else {
        (DEFAULT_FILL, DEFAULT_STROKE)
    }
}

/// Emit a cubic Bézier edge from the right-centre of the parent box to the
/// left-centre of the child box.
fn draw_edge(svg: &mut SvgBuilder, parent: &Placed, child: &Placed) {
    let x1 = parent.x + parent.w;
    let y1 = parent.cy;
    let x2 = child.x;
    let y2 = child.cy;
    let cp = (x2 - x1) / 2.0;
    let path = format!(
        r#"<path d="M {x1} {y1} C {} {y1} {} {y2} {x2} {y2}" fill="none" stroke="{EDGE_STROKE}" stroke-width="1.5"/>"#,
        x1 + cp,
        x2 - cp,
    );
    svg.raw(&path);
}

fn draw_node(svg: &mut SvgBuilder, placed: &Placed, _theme: &Theme) {
    let (fill, stroke) = fill_for(placed);

    // Draw edges to children first (behind boxes).
    for child in &placed.children {
        draw_edge(svg, placed, child);
    }

    // Node box.
    svg.rounded_rect(placed.x, placed.y, placed.w, NODE_H, RX, fill, stroke);

    // Label, centred in the box.
    svg.text(
        placed.x + placed.w / 2.0,
        placed.y + NODE_PADDING_Y + FONT_SIZE - 2.0,
        &placed.label,
        "middle",
        FONT_SIZE,
    );

    // Recurse.
    for child in &placed.children {
        draw_node(svg, child, _theme);
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn render(diagram: &MindMapDiagram, theme: &Theme) -> String {
    if diagram.roots.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let (placed_roots, total_w, total_h) = layout(&diagram.roots);

    let mut svg = SvgBuilder::new(total_w, total_h);

    for root in &placed_roots {
        draw_node(&mut svg, root, theme);
    }

    svg.finalize()
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_simple_mindmap() {
        let input = "@startmindmap\n* Root\n** Branch A\n*** Leaf 1\n** Branch B\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Root"), "svg should contain 'Root'");
        assert!(svg.contains("Branch A"), "svg should contain 'Branch A'");
        assert!(svg.contains("Leaf 1"), "svg should contain 'Leaf 1'");
        assert!(svg.contains("Branch B"), "svg should contain 'Branch B'");
    }

    #[test]
    fn renders_single_node() {
        let input = "@startmindmap\n* Solo\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Solo"));
    }

    #[test]
    fn empty_mindmap_does_not_panic() {
        let input = "@startmindmap\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"));
    }
}
