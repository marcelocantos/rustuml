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
const SIBLING_GAP: f64 = 20.0;
const LEVEL_DX: f64 = 50.0;
const X_MARGIN: f64 = 10.0;
const Y_MARGIN: f64 = 20.0;
const RX: f64 = 12.5;

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
    height: f64,
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
    pm::text_width(label, FONT_SIZE, false)
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

fn sum_with_gaps(kids: &[Placed]) -> f64 {
    let n = kids.len();
    if n == 0 {
        return 0.0;
    }
    kids.iter().map(|k| k.height).sum::<f64>() + (n - 1) as f64 * SIBLING_GAP
}

fn measure(node: &MindMapNode, side: Side) -> Placed {
    let text_w = node_text_width(&node.label);
    let w = placed_width(text_w, node.boxless);
    let fill = resolve_fill(&node.color);
    let kid_refs: Vec<&MindMapNode> = node.children.iter().filter(|c| c.side == side).collect();
    if kid_refs.is_empty() {
        return Placed {
            x: 0.0,
            cy: 0.0,
            w,
            label: node.label.clone(),
            side,
            fill,
            boxless: node.boxless,
            height: BOX_H,
            children: Vec::new(),
        };
    }
    let kids: Vec<Placed> = kid_refs.iter().map(|c| measure(c, side)).collect();
    let h = sum_with_gaps(&kids);
    Placed {
        x: 0.0,
        cy: 0.0,
        w,
        label: node.label.clone(),
        side,
        fill,
        boxless: node.boxless,
        height: h,
        children: kids,
    }
}

fn position(parent: &mut Placed) {
    if parent.children.is_empty() {
        return;
    }
    let n = parent.children.len();
    let total_h = sum_with_gaps(&parent.children);
    let mut y_top = parent.cy - total_h / 2.0;
    for (i, child) in parent.children.iter_mut().enumerate() {
        let h = child.height;
        child.cy = y_top + h / 2.0;
        child.x = if parent.side == Side::Left {
            parent.x - LEVEL_DX - child.w
        } else {
            parent.x + parent.w + LEVEL_DX
        };
        position(child);
        y_top += h;
        if i + 1 < n {
            y_top += SIBLING_GAP;
        }
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
    let mut cursor_y = Y_MARGIN;

    for root in &diagram.roots {
        let right_subtree = measure(root, Side::Right);
        let left_subtree = measure(root, Side::Left);
        let right_h = sum_with_gaps(&right_subtree.children);
        let left_h = sum_with_gaps(&left_subtree.children);
        let kids_h = right_h.max(left_h);
        let total_h = if kids_h > 0.0 { kids_h } else { BOX_H };

        let root_text_w = node_text_width(&root.label);
        let root_w = placed_width(root_text_w, root.boxless);
        // Top of root's allocated band = cursor_y. Root centred vertically.
        let root_cy = cursor_y + total_h / 2.0;

        let mut root_placed = Placed {
            x: 0.0,
            cy: root_cy,
            w: root_w,
            label: root.label.clone(),
            side: Side::Right,
            fill: resolve_fill(&root.color),
            boxless: root.boxless,
            height: total_h,
            children: Vec::new(),
        };

        let mut right_children: Vec<Placed> = right_subtree.children;
        if !right_children.is_empty() {
            let n = right_children.len();
            let mut y_top = root_cy - right_h / 2.0;
            for (i, child) in right_children.iter_mut().enumerate() {
                let h = child.height;
                child.cy = y_top + h / 2.0;
                child.x = root_w + LEVEL_DX;
                position(child);
                y_top += h;
                if i + 1 < n {
                    y_top += SIBLING_GAP;
                }
            }
            root_placed.children.extend(right_children);
        }

        let mut left_children: Vec<Placed> = left_subtree.children;
        if !left_children.is_empty() {
            let n = left_children.len();
            let mut y_top = root_cy - left_h / 2.0;
            for (i, child) in left_children.iter_mut().enumerate() {
                let h = child.height;
                child.cy = y_top + h / 2.0;
                child.x = -LEVEL_DX - child.w;
                position(child);
                y_top += h;
                if i + 1 < n {
                    y_top += SIBLING_GAP;
                }
            }
            root_placed.children.extend(left_children);
        }

        placed.push(root_placed);
        cursor_y += total_h;
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
    let total_h = global_max_cy + Y_MARGIN;
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
