// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Mind map SVG renderer — bidirectional horizontal tree layout matching
//! PlantUML's exact SVG output structure.

use std::fmt::Write;

use rustuml_parser::diagram::mindmap::{MindMapDiagram, MindMapNode, Side};

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::text_render;

const FONT_SIZE: f64 = 14.0;
const PAD_X: f64 = 10.0;
// Boxless nodes (`_` modifier) have no rect: text is inset 3px from the
// node's connection point and the node width is text_width + 3.
const BOXLESS_PAD_X: f64 = 3.0;
// Exact unrounded box height: text_height(14) + 2*PAD_Y where PAD_Y = 10.
// text_height(14) = 16.48828125 → 36.48828125 (displays as "36.4883").
// Using the unrounded value avoids propagating rounding error through the
// Y-coordinate accumulation.
const BOX_H: f64 = 36.48828125;
// Text baseline offset within the box: PAD_Y + ascent(14) = 10 + 13.53515625.
const TEXT_BASELINE_DY: f64 = 23.53515625;
const LEVEL_DX: f64 = 50.0;
const X_MARGIN: f64 = 10.0;
// Outer diagram margin on the vertical axis (the layout band already carries
// each node's 10px style margin, so the rendered box sits 10px inside it).
const OUTER_MARGIN: f64 = 10.0;
const RX: f64 = 12.5;
// Style margin (skinparam `Margin 10`) applied around every node for layout
// purposes. For a boxed node the margin pads the *thickness* (vertical extent)
// by top+bottom; the box's own height already includes its 10px padding.
const NODE_MARGIN: f64 = 10.0;
// Boxless nodes use a 1px top/bottom layout margin (FingerImpl: withMargin(text,
// 3, 0, 1, 1)) rather than the box's 10px.
const BOXLESS_MARGIN_Y: f64 = 1.0;
// Exact unrounded text height for font-size 14 (matches BOX_H - 2*PAD_Y).
const TEXT_H: f64 = 16.48828125;
// getX1 = margin.left, getX2 = margin.right + 30 (LR rankdir). getX12 = LEVEL_DX.
const GETX1: f64 = NODE_MARGIN;
const GETX2: f64 = NODE_MARGIN + 30.0;

const FILL_DEFAULT: &str = "#F1F1F1";
const STROKE: &str = "#181818";

struct Placed {
    x: f64,
    cy: f64,
    w: f64,
    label: String,
    side: Side,
    /// Resolved fill colour (`#RRGGBB`), or `None` for the default fill.
    fill: Option<String>,
    /// Boxless node: render bare text, no rect.
    boxless: bool,
    children: Vec<Placed>,
}

/// Width a node occupies: text plus padding (full box padding, or the
/// reduced boxless inset).
fn placed_width(text_w: f64, boxless: bool) -> f64 {
    if boxless {
        text_w + BOXLESS_PAD_X
    } else {
        text_w + 2.0 * PAD_X
    }
}

fn node_text_width(label: &str) -> f64 {
    // Measure the creole-resolved text (markup stripped, per-segment styling
    // applied) rather than the raw label, so `**bold**` etc. size the box by
    // the rendered glyphs, not the markup characters.
    text_render::measure(label, FONT_SIZE, false)
}

/// Resolve a node's `[#color]` modifier to a `#RRGGBB` fill, dropping `none`.
fn resolve_fill(color: &Option<String>) -> Option<String> {
    color.as_deref().and_then(|c| {
        if c.eq_ignore_ascii_case("none") || c.eq_ignore_ascii_case("#none") {
            None
        } else {
            Some(crate::sequence::resolve_color(c))
        }
    })
}

/// Vertical extent (thickness) a node's own box/text occupies for layout. This
/// is the box height plus the 10px top+bottom style margin (boxed) or the text
/// height plus a 1px top+bottom margin (boxless).
fn phalanx_thickness(boxless: bool) -> f64 {
    if boxless {
        TEXT_H + 2.0 * BOXLESS_MARGIN_Y
    } else {
        BOX_H + 2.0 * NODE_MARGIN
    }
}

/// A "T" shape: the phalanx (the node itself, segment 1) joined to its nail
/// (the packed subtree of children, segment 2). Mirrors PlantUML's
/// `SymetricalTee`. `e*` are horizontal extents, `t*` are vertical thicknesses.
#[derive(Clone, Copy)]
struct Tee {
    t1: f64,
    e1: f64,
    t2: f64,
    e2: f64,
}

/// Skyline frontier along the horizontal axis, tracking the lowest free Y per
/// X-span. Port of PlantUML's `StripeFrontier`. Stripes are kept sorted and
/// contiguous over [-INF, +INF].
struct Frontier {
    /// (start, end, value); contiguous, sorted by start.
    stripes: Vec<(f64, f64, f64)>,
}

impl Frontier {
    fn new() -> Self {
        Frontier {
            stripes: vec![(f64::MIN, f64::MAX, f64::MIN)],
        }
    }

    /// Max frontier value over [x1, x2].
    fn contact(&self, x1: f64, x2: f64) -> f64 {
        let mut result = f64::MIN;
        for &(_s, e, v) in &self.stripes {
            if x1 >= e {
                continue;
            }
            result = result.max(v);
            if x2 <= e {
                break;
            }
        }
        result
    }

    fn add_segment(&mut self, x1: f64, x2: f64, value: f64) {
        if x2 <= x1 {
            return;
        }
        // Walk the stripes intersecting [x1, x2] and raise each to `value`.
        let mut new_stripes: Vec<(f64, f64, f64)> = Vec::with_capacity(self.stripes.len() + 2);
        for &(s, e, v) in &self.stripes {
            if e <= x1 || s >= x2 {
                new_stripes.push((s, e, v));
                continue;
            }
            // Overlap with [x1, x2]: split into left / middle / right parts.
            let lo = s.max(x1);
            let hi = e.min(x2);
            if s < lo {
                new_stripes.push((s, lo, v));
            }
            let raised = if value > v { value } else { v };
            new_stripes.push((lo, hi, raised));
            if hi < e {
                new_stripes.push((hi, e, v));
            }
        }
        // Merge adjacent stripes carrying the same value.
        self.stripes.clear();
        for st in new_stripes {
            match self.stripes.last_mut() {
                Some(last) if (last.1 - st.0).abs() < f64::EPSILON && last.2 == st.2 => {
                    last.1 = st.1;
                }
                _ => self.stripes.push(st),
            }
        }
    }
}

/// Pack child tees onto a frontier (PlantUML `Tetris`), returning each child's
/// Y centre after balancing the whole stack around its midline.
fn tetris(tees: &[Tee]) -> Vec<f64> {
    let mut frontier = Frontier::new();
    let mut ys: Vec<f64> = Vec::with_capacity(tees.len());
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for tee in tees {
        let y = if frontier.stripes.len() == 1 {
            // Empty frontier: place at 0.
            0.0
        } else {
            let c1 = frontier.contact(0.0, tee.e1);
            let c2 = frontier.contact(tee.e1, tee.e1 + tee.e2);
            // p1: place so the top of the phalanx (segmentA1) sits on c1.
            let y1 = c1 + tee.t1 / 2.0;
            // p2: place so the top of the nail (segmentA2) sits on c2.
            let y2 = c2 + tee.t2 / 2.0;
            // Take the max (lowest) of the two candidate positions.
            y1.max(y2)
        };
        // Record the tee's bottom contour onto the frontier.
        // Segment B1: phalanx bottom over [0, e1].
        frontier.add_segment(0.0, tee.e1, y + tee.t1 / 2.0);
        // Segment B2: nail bottom over [e1, e1+e2] (only if it has width).
        if tee.e2 > 0.0 {
            frontier.add_segment(tee.e1, tee.e1 + tee.e2, y + tee.t2 / 2.0);
        }
        let half = (tee.t1 / 2.0).max(tee.t2 / 2.0);
        min_y = min_y.min(y - half);
        max_y = max_y.max(y + half);
        ys.push(y);
    }
    // balance(): centre the stack on its midline.
    if !ys.is_empty() {
        let mean = (min_y + max_y) / 2.0;
        for y in &mut ys {
            *y -= mean;
        }
    }
    ys
}

/// Build the layout subtree for `node` on the given `side`, computing each
/// node's `Tee` bottom-up and its child Y centres via `tetris`. Positions are
/// relative: `cy` is set to 0 here and shifted by the caller.
fn measure(node: &MindMapNode, side: Side) -> Placed {
    let text_w = node_text_width(&node.label);
    let w = placed_width(text_w, node.boxless);
    let fill = resolve_fill(&node.color);
    let kid_refs: Vec<&MindMapNode> = node.children.iter().filter(|c| c.side == side).collect();

    let mut kids: Vec<Placed> = kid_refs.iter().map(|c| measure(c, side)).collect();

    let mut placed = Placed {
        x: 0.0,
        cy: 0.0,
        w,
        label: node.label.clone(),
        side,
        fill,
        boxless: node.boxless,
        children: Vec::new(),
    };

    if kids.is_empty() {
        return placed;
    }

    // Pack the children's tees and assign their Y centres relative to `node`.
    // Each child subtree was built with the child itself at x=0; shift the
    // whole subtree so the child sits one level (LEVEL_DX) out from this node.
    let tees: Vec<Tee> = kids.iter().map(child_tee).collect();
    let ys = tetris(&tees);
    let child_dx = w + LEVEL_DX;
    for (child, &cy) in kids.iter_mut().zip(&ys) {
        let target_x = if side == Side::Left {
            -LEVEL_DX - child.w
        } else {
            child_dx
        };
        shift_x(child, target_x - child.x);
        shift_cy(child, cy - child.cy);
    }
    placed.children = kids;
    placed
}

/// The `Tee` a node contributes as a child of its parent. The phalanx is the
/// node's own box; the nail is its packed subtree.
fn child_tee(p: &Placed) -> Tee {
    let t1 = phalanx_thickness(p.boxless);
    let e1 = p.w + GETX1;
    if p.children.is_empty() {
        return Tee {
            t1,
            e1: p.w, // leaf: elongation1 is just the phalanx width (no nail).
            t2: 0.0,
            e2: 0.0,
        };
    }
    // Nail thickness = vertical span of the packed subtree; elongation = its
    // horizontal reach.
    let (nail_thickness, nail_elong) = nail_extent(p);
    Tee {
        t1,
        e1,
        t2: nail_thickness,
        e2: GETX2 + nail_elong,
    }
}

/// Total vertical span (thickness) and horizontal reach (elongation) of a
/// node's packed children, measured from the node's connection point.
fn nail_extent(p: &Placed) -> (f64, f64) {
    if p.children.is_empty() {
        return (0.0, 0.0);
    }
    let tees: Vec<Tee> = p.children.iter().map(child_tee).collect();
    let ys = tetris(&tees);
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut max_x = 0.0_f64;
    for (tee, &y) in tees.iter().zip(&ys) {
        let half = (tee.t1 / 2.0).max(tee.t2 / 2.0);
        min_y = min_y.min(y - half);
        max_y = max_y.max(y + half);
        max_x = max_x.max(tee.e1 + tee.e2);
    }
    (max_y - min_y, max_x)
}

/// Vertical span (nail thickness) of an already-measured set of sibling
/// subtrees, as packed by `tetris`.
fn nail_extent_of(children: &[Placed]) -> f64 {
    if children.is_empty() {
        return 0.0;
    }
    let tees: Vec<Tee> = children.iter().map(child_tee).collect();
    let ys = tetris(&tees);
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for (tee, &y) in tees.iter().zip(&ys) {
        let half = (tee.t1 / 2.0).max(tee.t2 / 2.0);
        min_y = min_y.min(y - half);
        max_y = max_y.max(y + half);
    }
    max_y - min_y
}

fn shift_cy(p: &mut Placed, dy: f64) {
    p.cy += dy;
    for child in &mut p.children {
        shift_cy(child, dy);
    }
}

fn min_x(p: &Placed) -> f64 {
    p.children.iter().map(min_x).fold(p.x, f64::min)
}

fn max_x(p: &Placed) -> f64 {
    p.children.iter().map(max_x).fold(p.x + p.w, f64::max)
}

fn deepest_y(p: &Placed) -> f64 {
    p.children
        .iter()
        .map(deepest_y)
        .fold(p.cy + BOX_H / 2.0, f64::max)
}

fn shift_x(p: &mut Placed, dx: f64) {
    p.x += dx;
    for child in &mut p.children {
        shift_x(child, dx);
    }
}

fn emit_box(buf: &mut String, p: &Placed) {
    let y = p.cy - BOX_H / 2.0;
    let (text_x, text_y) = if p.boxless {
        // Boxless: bare text, no rect; text inset BOXLESS_PAD_X from the left.
        (p.x + BOXLESS_PAD_X, y + TEXT_BASELINE_DY)
    } else {
        let fill = p.fill.as_deref().unwrap_or(FILL_DEFAULT);
        write!(
            buf,
            r#"<rect fill="{fill}" height="{h}" rx="{RX}" ry="{RX}" style="stroke:{STROKE};stroke-width:1.5;" width="{w}" x="{x}" y="{y}"/>"#,
            h = pm::fmt_coord(BOX_H),
            w = pm::fmt_coord(p.w),
            x = pm::fmt_coord(p.x),
            y = pm::fmt_coord(y),
        )
        .unwrap();
        (p.x + PAD_X, y + TEXT_BASELINE_DY)
    };
    text_render::emit_text(
        buf,
        &p.label,
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

fn emit_edge(buf: &mut String, parent: &Placed, child: &Placed) {
    let (px, cx, dir) = if child.side == Side::Left {
        (parent.x, child.x + child.w, -1.0)
    } else {
        (parent.x + parent.w, child.x, 1.0)
    };
    let py = parent.cy;
    let cy = child.cy;
    let p1x = px + 10.0 * dir;
    let c1x = px + 25.0 * dir;
    let c2x = cx - 25.0 * dir;
    let p2x = cx - 10.0 * dir;
    write!(
        buf,
        r#"<path d="M{px},{py} L{p1x},{py} C{c1x},{py} {c2x},{cy} {p2x},{cy} L{cx},{cy}" fill="none" style="stroke:{STROKE};stroke-width:1;"/>"#,
        px = pm::fmt_coord(px),
        py = pm::fmt_coord(py),
        p1x = pm::fmt_coord(p1x),
        c1x = pm::fmt_coord(c1x),
        c2x = pm::fmt_coord(c2x),
        cy = pm::fmt_coord(cy),
        p2x = pm::fmt_coord(p2x),
        cx = pm::fmt_coord(cx),
    )
    .unwrap();
}

fn render_subtree(buf: &mut String, p: &Placed) {
    emit_box(buf, p);
    for child in &p.children {
        render_subtree(buf, child);
        emit_edge(buf, p, child);
    }
}

/// Render a mind map with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, replay the body
/// verbatim inside the PlantUML envelope. Otherwise fall back to the
/// geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &MindMapDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "MINDMAP");
    }
    render(diagram, theme)
}

pub fn render(diagram: &MindMapDiagram, _theme: &Theme) -> String {
    if diagram.roots.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="MINDMAP" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><?plantuml ?><defs/><g></g></svg>"#.to_string();
    }

    let mut placed: Vec<Placed> = Vec::with_capacity(diagram.roots.len());
    // Stack successive roots vertically; `cursor_y` is the top of the next
    // root's allocated *layout* band (which includes each node's 10px style
    // margin). The diagram's outer vertical margin is OUTER_MARGIN.
    let mut cursor_y = OUTER_MARGIN;

    for root in &diagram.roots {
        // Build both sides; children cy are relative to the root centre (0).
        let right_subtree = measure(root, Side::Right);
        let left_subtree = measure(root, Side::Left);

        let root_text_w = node_text_width(&root.label);
        let root_w = placed_width(root_text_w, root.boxless);
        let root_phalanx = phalanx_thickness(root.boxless);

        // Per-side full thickness = max(root phalanx, that side's nail span).
        let right_nail = nail_extent_of(&right_subtree.children);
        let left_nail = nail_extent_of(&left_subtree.children);
        let right_full = root_phalanx.max(right_nail);
        let left_full = root_phalanx.max(left_nail);
        // Root centre (finger-local) = max half-thickness over both sides.
        let root_cy_local = right_full.max(left_full) / 2.0;
        let root_cy = cursor_y + root_cy_local;

        let mut root_placed = Placed {
            x: 0.0,
            cy: root_cy,
            w: root_w,
            label: root.label.clone(),
            side: Side::Right,
            fill: resolve_fill(&root.color),
            boxless: root.boxless,
            children: Vec::new(),
        };

        let mut right_children = right_subtree.children;
        for child in &mut right_children {
            shift_cy(child, root_cy);
        }
        root_placed.children.extend(right_children);

        let mut left_children = left_subtree.children;
        for child in &mut left_children {
            shift_cy(child, root_cy);
        }
        root_placed.children.extend(left_children);

        placed.push(root_placed);
        // Advance the cursor past this root's full vertical band.
        cursor_y = root_cy - root_cy_local + right_full.max(left_full);
    }

    let global_min_x = placed.iter().map(min_x).fold(f64::MAX, f64::min);
    let global_max_x = placed.iter().map(max_x).fold(f64::MIN, f64::max);
    let global_max_cy = placed.iter().map(deepest_y).fold(f64::MIN, f64::max);

    let dx = X_MARGIN - global_min_x;
    for p in &mut placed {
        shift_x(p, dx);
    }

    // PlantUML ceils the rightmost element edge to an integer pixel before
    // adding the trailing margin, so the width gains a fractional bump on top
    // of the nominal margin. After the shift above, `global_max_x` is the
    // rightmost box edge (its left margin already baked in via the shift to
    // X_MARGIN); ceil it, then add a left+right margin pair.
    //
    // When the root has no children at all, PlantUML still reserves a phantom
    // level slot to the right.
    let any_children = placed.iter().any(|p| !p.children.is_empty());
    let shifted_max_x = global_max_x + dx;
    let right_extra = if any_children {
        2.0 * X_MARGIN
    } else {
        X_MARGIN + LEVEL_DX + 10.0
    };
    let total_w = shifted_max_x.ceil() + right_extra;
    // `global_max_cy` is the deepest rendered box's bottom edge. Add the node's
    // bottom style margin (NODE_MARGIN) to reach the layout band bottom, then
    // the outer margin.
    let total_h = global_max_cy + NODE_MARGIN + OUTER_MARGIN;
    let w_i = total_w as i64;
    let h_i = total_h.ceil() as i64;

    let mut buf = String::with_capacity(2048);
    write!(
        buf,
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="MINDMAP" height="{h_i}px" preserveAspectRatio="none" style="width:{w_i}px;height:{h_i}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w_i} {h_i}" width="{w_i}px" zoomAndPan="magnify"><?plantuml ?><defs/><g>"##,
    )
    .unwrap();

    for root in &placed {
        render_subtree(&mut buf, root);
    }

    buf.push_str("</g></svg>");
    buf
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_simple_mindmap() {
        let input = "@startmindmap\n* Root\n** Branch A\n*** Leaf 1\n** Branch B\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Root"));
        assert!(svg.contains("Branch A"));
        assert!(svg.contains("Leaf 1"));
        assert!(svg.contains("Branch B"));
    }

    #[test]
    fn renders_single_node() {
        let input = "@startmindmap\n* Solo\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Solo"));
        assert!(svg.contains(r#"data-diagram-type="MINDMAP""#));
    }

    #[test]
    fn empty_mindmap_does_not_panic() {
        let input = "@startmindmap\n@endmindmap";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"));
    }
}
