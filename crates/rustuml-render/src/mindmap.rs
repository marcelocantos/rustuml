// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Mind map SVG renderer — bidirectional horizontal tree layout.
//!
//! The root is placed in the horizontal centre.  Right-side branches (`**`,
//! `***`, …) extend to the right; left-side branches (`--`, `---`, …) extend
//! to the left.  Each node is drawn as a rounded rectangle containing its
//! label.  Parent–child edges are smooth cubic Bézier curves.

use rustuml_parser::diagram::mindmap::{MindMapDiagram, MindMapNode, Side};

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
    side: Side,
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

/// Lay out a right-side subtree rooted at `node`.
///
/// * `x`           — left edge of the node box.
/// * `y_top`       — top of the vertical space allocated to this subtree.
/// * `branch_index`— which depth-1 branch we are in (for colour selection).
fn layout_right(node: &MindMapNode, x: f64, y_top: f64, branch_index: usize) -> Placed {
    let w = node_w(&node.label);
    let child_x = x + w + LEVEL_GAP;

    let mut children: Vec<Placed> = Vec::with_capacity(node.children.len());
    let mut cursor = y_top;

    for child in node.children.iter().filter(|c| c.side == Side::Right) {
        let placed = layout_right(child, child_x, cursor, branch_index);
        cursor += placed.subtree_h() + SIBLING_GAP;
        children.push(placed);
    }

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
        side: node.side,
        branch_index,
        children,
    }
}

/// Lay out a left-side subtree rooted at `node`.
///
/// * `right_edge`  — right edge of the node box (nodes grow leftward).
/// * `y_top`       — top of the vertical space allocated to this subtree.
/// * `branch_index`— which depth-1 branch we are in (for colour selection).
fn layout_left(node: &MindMapNode, right_edge: f64, y_top: f64, branch_index: usize) -> Placed {
    let w = node_w(&node.label);
    let x = right_edge - w;
    let child_right = x - LEVEL_GAP;

    let mut children: Vec<Placed> = Vec::with_capacity(node.children.len());
    let mut cursor = y_top;

    for child in node.children.iter().filter(|c| c.side == Side::Left) {
        let placed = layout_left(child, child_right, cursor, branch_index);
        cursor += placed.subtree_h() + SIBLING_GAP;
        children.push(placed);
    }

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
        side: node.side,
        branch_index,
        children,
    }
}

/// Compute the leftmost X in a subtree (for left-side layout).
fn subtree_min_x(p: &Placed) -> f64 {
    p.children
        .iter()
        .map(subtree_min_x)
        .fold(p.x, f64::min)
}

fn subtree_max_x(p: &Placed) -> f64 {
    let self_right = p.x + p.w;
    p.children
        .iter()
        .map(subtree_max_x)
        .fold(self_right, f64::max)
}

/// Lay out a full diagram including left and right branches.
///
/// Returns `(placed_roots, total_width, total_height)`.  The placed roots
/// include both the actual roots and the implicit per-side branch groups
/// attached to each root.  Coordinates are in a coordinate system where the
/// diagram's top-left is (0, 0); the root nodes are horizontally centred
/// between the left and right subtrees.
fn layout(roots: &[MindMapNode]) -> (Vec<Placed>, f64, f64) {
    let mut placed_roots: Vec<Placed> = Vec::with_capacity(roots.len());
    let mut cursor_y = MARGIN;
    let mut max_right = 0.0_f64;
    let mut min_left = f64::MAX;

    // First pass: lay out at a temporary X origin, then shift everything.
    // We use a two-stage approach:
    //   1. Lay out right branches starting from x=0 for the root.
    //   2. Lay out left branches ending at x=0 for the root (right edge).
    //   3. After all roots are processed, shift X coords by the leftmost
    //      extent + MARGIN so the diagram starts at MARGIN.

    // We collect raw placed items first (root right_edge = 0).
    struct RawRoot {
        right_branches: Placed, // the root box with right children
        left_h: f64,
        left_children: Vec<Placed>, // depth-2+ left nodes
    }

    let mut raw: Vec<RawRoot> = Vec::new();
    let mut temp_cursor = 0.0_f64;

    for (bi, root) in roots.iter().enumerate() {
        // Collect right and left children separately.
        let right_children: Vec<&MindMapNode> =
            root.children.iter().filter(|c| c.side == Side::Right).collect();
        let left_children: Vec<&MindMapNode> =
            root.children.iter().filter(|c| c.side == Side::Left).collect();

        // Compute heights for right and left sides independently.
        let right_h: f64 = if right_children.is_empty() {
            NODE_H
        } else {
            right_children.iter().map(|c| subtree_height(c, Side::Right)).sum::<f64>()
                + SIBLING_GAP * (right_children.len() - 1) as f64
        };

        let left_h: f64 = if left_children.is_empty() {
            NODE_H
        } else {
            left_children.iter().map(|c| subtree_height(c, Side::Left)).sum::<f64>()
                + SIBLING_GAP * (left_children.len() - 1) as f64
        };

        let total_h = right_h.max(left_h).max(NODE_H);
        let root_y_top = temp_cursor;
        let root_cy = root_y_top + total_h / 2.0;

        // Layout right children starting just to the right of the root.
        let root_w = node_w(&root.label);
        let mut right_placed: Vec<Placed> = Vec::new();
        {
            let right_start_y = root_cy - right_h / 2.0;
            let mut c_cursor = right_start_y;
            for child in &right_children {
                let placed = layout_right(child, root_w + LEVEL_GAP, c_cursor, bi);
                c_cursor += placed.subtree_h() + SIBLING_GAP;
                right_placed.push(placed);
            }
        }

        // Layout left children ending just to the left of the root (right_edge = 0).
        let mut left_placed: Vec<Placed> = Vec::new();
        {
            let left_start_y = root_cy - left_h / 2.0;
            let mut c_cursor = left_start_y;
            for child in &left_children {
                let placed = layout_left(child, -LEVEL_GAP, c_cursor, bi);
                c_cursor += placed.subtree_h() + SIBLING_GAP;
                left_placed.push(placed);
            }
        }

        let root_placed = Placed {
            x: 0.0,
            y: root_cy - NODE_H / 2.0,
            w: root_w,
            cy: root_cy,
            label: root.label.clone(),
            depth: root.depth,
            side: root.side,
            branch_index: bi,
            children: right_placed,
        };

        let _ = root_y_top; // used only for positioning within the raw pass
        raw.push(RawRoot {
            right_branches: root_placed,
            left_h: total_h,
            left_children: left_placed,
        });

        temp_cursor += total_h + SIBLING_GAP;
    }

    // Find the leftmost X across all left subtrees.
    let global_min_x = raw
        .iter()
        .flat_map(|r| r.left_children.iter().map(subtree_min_x))
        .fold(0.0_f64, f64::min); // root itself is at x=0

    // Shift amount: bring global_min_x to MARGIN.
    let shift_x = MARGIN - global_min_x;

    // Second pass: apply shift and collect final placed roots.
    let mut final_cursor_y = MARGIN;
    for mut rr in raw {
        // In the raw layout, the root's subtree starts at y = (root_cy - total_h/2).
        // We want that to land at final_cursor_y.
        let total_h = rr.left_h;
        let shift_y = final_cursor_y - (rr.right_branches.cy - total_h / 2.0);

        shift_placed(&mut rr.right_branches, shift_x, shift_y);
        for lc in &mut rr.left_children {
            shift_placed(lc, shift_x, shift_y);
        }

        max_right = max_right.max(subtree_max_x(&rr.right_branches));
        for lc in &rr.left_children {
            let mx = subtree_max_x(lc);
            max_right = max_right.max(mx);
            min_left = min_left.min(subtree_min_x(lc));
        }

        // Combine: the root's Placed carries right children; left children are
        // attached as a sibling list stored separately.  We encode them by
        // adding them to the root's children list (they already have Side::Left
        // set so the renderer can distinguish them).
        let mut root_placed = rr.right_branches;
        for lc in rr.left_children {
            root_placed.children.push(lc);
        }

        final_cursor_y += total_h + SIBLING_GAP;
        cursor_y = final_cursor_y;
        placed_roots.push(root_placed);
    }

    let total_h = cursor_y - SIBLING_GAP + MARGIN;
    let total_w = max_right + MARGIN;
    (placed_roots, total_w, total_h)
}

/// Compute the total vertical span of a subtree following only children
/// with the given side.
fn subtree_height(node: &MindMapNode, side: Side) -> f64 {
    let side_children: Vec<&MindMapNode> =
        node.children.iter().filter(|c| c.side == side).collect();
    if side_children.is_empty() {
        NODE_H
    } else {
        side_children.iter().map(|c| subtree_height(c, side)).sum::<f64>()
            + SIBLING_GAP * (side_children.len() - 1) as f64
    }
}

/// Translate all coordinates in a `Placed` tree by (dx, dy).
fn shift_placed(p: &mut Placed, dx: f64, dy: f64) {
    p.x += dx;
    p.y += dy;
    p.cy += dy;
    for child in &mut p.children {
        shift_placed(child, dx, dy);
    }
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

/// Emit a cubic Bézier edge from parent to child.
///
/// For right-side children the edge leaves the right-centre of the parent and
/// arrives at the left-centre of the child.  For left-side children it's the
/// mirror: edge leaves the left-centre of the parent and arrives at the
/// right-centre of the child.
fn draw_edge(svg: &mut SvgBuilder, parent: &Placed, child: &Placed) {
    let (x1, x2) = if child.side == Side::Left {
        // Parent left-centre → child right-centre
        (parent.x, child.x + child.w)
    } else {
        // Parent right-centre → child left-centre
        (parent.x + parent.w, child.x)
    };
    let y1 = parent.cy;
    let y2 = child.cy;
    let cp = (x2 - x1).abs() / 2.0;
    let (cx1, cx2) = if child.side == Side::Left {
        (x1 - cp, x2 + cp)
    } else {
        (x1 + cp, x2 - cp)
    };
    let path = format!(
        r#"<path d="M {x1} {y1} C {cx1} {y1} {cx2} {y2} {x2} {y2}" fill="none" stroke="{EDGE_STROKE}" stroke-width="1.5"/>"#,
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

    #[test]
    fn renders_left_branches() {
        let input = "@startmindmap\n* Center\n-- Left A\n--- Left A1\n-- Left B\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Center"), "svg should contain 'Center'");
        assert!(svg.contains("Left A"), "svg should contain 'Left A'");
        assert!(svg.contains("Left A1"), "svg should contain 'Left A1'");
        assert!(svg.contains("Left B"), "svg should contain 'Left B'");
    }

    #[test]
    fn renders_mixed_sides() {
        let input =
            "@startmindmap\n* Root\n** Right\n-- Left\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Root"));
        assert!(svg.contains("Right"));
        assert!(svg.contains("Left"));
    }
}
