// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Mind map SVG renderer — bidirectional horizontal tree layout matching
//! PlantUML's exact SVG output structure.

use std::fmt::Write;

use rustuml_parser::diagram::mindmap::{MindMapDiagram, MindMapNode, Side};

use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::text_render;

const FONT_SIZE: f64 = 14.0;
const PAD_X: f64 = 10.0;
const BOX_H: f64 = 36.4883;
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
    #[allow(dead_code)]
    text_w: f64,
    label: String,
    side: Side,
    height: f64,
    children: Vec<Placed>,
}

fn node_text_width(label: &str) -> f64 {
    pm::text_width(label, FONT_SIZE, false)
}

fn node_w(label: &str) -> f64 {
    node_text_width(label) + 2.0 * PAD_X
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
    let w = text_w + 2.0 * PAD_X;
    let kid_refs: Vec<&MindMapNode> = node.children.iter().filter(|c| c.side == side).collect();
    if kid_refs.is_empty() {
        return Placed {
            x: 0.0,
            cy: 0.0,
            w,
            text_w,
            label: node.label.clone(),
            side,
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
        text_w,
        label: node.label.clone(),
        side,
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
    write!(
        buf,
        r#"<rect fill="{FILL_DEFAULT}" height="{h}" rx="{RX}" ry="{RX}" style="stroke:{STROKE};stroke-width:1.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(BOX_H),
        w = pm::fmt_coord(p.w),
        x = pm::fmt_coord(p.x),
        y = pm::fmt_coord(y),
    )
    .unwrap();
    let text_y = y + 23.5352;
    let text_x = p.x + PAD_X;
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

        let root_w = node_w(&root.label);
        // Top of root's allocated band = cursor_y. Root centred vertically.
        let root_cy = cursor_y + total_h / 2.0;

        let mut root_placed = Placed {
            x: 0.0,
            cy: root_cy,
            w: root_w,
            text_w: node_text_width(&root.label),
            label: root.label.clone(),
            side: Side::Right,
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

    // PlantUML reserves a level slot for a phantom child when there are no
    // children at all.
    let any_children = placed.iter().any(|p| !p.children.is_empty());
    let right_pad = if any_children {
        X_MARGIN
    } else {
        X_MARGIN + LEVEL_DX + 10.0
    };
    let total_w = (global_max_x - global_min_x) + X_MARGIN + right_pad;
    let total_h = global_max_cy + Y_MARGIN;
    let w_i = total_w.ceil() as i64;
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
