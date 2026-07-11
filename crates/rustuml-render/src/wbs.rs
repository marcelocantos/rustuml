// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Work Breakdown Structure (WBS) SVG renderer.
//!
//! This is a faithful port of PlantUML's `Fork` / `ITFComposed` / `ITFLeaf`
//! layout (package `net.sourceforge.plantuml.wbs`).  Each node measures its own
//! subtree dimension bottom-up; the draw pass then walks the tree emitting
//! boxes and connector lines in PlantUML's exact order so the SVG element
//! sequence matches.
//!
//! Coordinate model: PlantUML draws the `Fork` in a local frame and translates
//! the whole diagram by a `MARGIN` on each side.  The root box sits at the top,
//! a short vertical drop reaches a horizontal spine, and each top-level child
//! subtree hangs below.  Within a subtree, left-side (`--`) children grow
//! leftward and right-side (`**`) children grow rightward, each as a vertical
//! stack indented `DELTA_X` past the parent's trunk.

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
const MARGIN: f64 = 20.0;
/// `ITFComposed.delta1x` / `Fork.delta1x` between sibling vertical stacks; also
/// the length of an elbow stub line from a parent trunk to a child box edge.
const DELTA_X: f64 = 10.0;
/// `Fork.delta1x`: horizontal gap between top-level child subtrees.
const FORK_DELTA_X: f64 = 20.0;
/// `Fork.deltay`: vertical room between the root box and the spine of children.
const FORK_DELTA_Y: f64 = 40.0;
/// `ITFComposed.marginBottom`: vertical gap between stacked children.
const MARGIN_BOTTOM: f64 = 15.0;

const FILL_DEFAULT: &str = "#F1F1F1";
const STROKE: &str = "#181818";

/// A measured WBS node: PlantUML's `ITF` (either `ITFLeaf` or `ITFComposed`).
struct Itf {
    label: String,
    fill: String,
    text_w: f64,
    main_w: f64,
    /// Children that grow leftward (`--` prefix), in source order.
    left: Vec<Itf>,
    /// Children that grow rightward (`**` prefix), in source order.
    right: Vec<Itf>,
    /// Cached `calculateDimension` width of the whole subtree.
    width: f64,
    /// Cached `calculateDimension` height of the whole subtree.
    height: f64,
}

impl Itf {
    /// `getw1`: x of this node's own trunk within its subtree frame.
    fn w1(&self) -> f64 {
        (self.main_w / 2.0).max(DELTA_X + coll_width(&self.left))
    }
}

/// `getCollWidth`: max child width on one side (children stack vertically).
fn coll_width(children: &[Itf]) -> f64 {
    children.iter().map(|c| c.width).fold(0.0, f64::max)
}

/// `getCollHeight`: sum of `marginBottom + childHeight` over one side.
fn coll_height(children: &[Itf]) -> f64 {
    children
        .iter()
        .map(|c| MARGIN_BOTTOM + c.height)
        .sum::<f64>()
}

fn measure(node: &WbsNode, is_root: bool) -> Itf {
    let text_w = pm::text_width(&node.label, FONT_SIZE, false);
    let main_w = text_w + PAD_X * 2.0;
    let fill = node
        .color
        .as_deref()
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| FILL_DEFAULT.to_string());

    let mut left = Vec::new();
    let mut right = Vec::new();
    for child in &node.children {
        let m = measure(child, false);
        match child.side {
            // PlantUML's WElement.createElement re-homes a left child of the
            // root to the FRONT of the right list (newLevel == 1), so the root
            // has no left side; deeper nodes keep left/right as written.
            WbsSide::Left if is_root => right.insert(0, m),
            WbsSide::Left => left.push(m),
            WbsSide::Right => right.push(m),
        }
    }

    // ITFComposed.calculateDimension / ITFLeaf.calculateDimension.
    let (width, height) = if left.is_empty() && right.is_empty() {
        (main_w, BOX_H)
    } else {
        let w = (main_w / 2.0).max(DELTA_X + coll_width(&left))
            + (main_w / 2.0).max(DELTA_X + coll_width(&right));
        let h = BOX_H + coll_height(&left).max(coll_height(&right));
        (w, h)
    };

    Itf {
        label: node.label.clone(),
        fill,
        text_w,
        main_w,
        left,
        right,
        width,
        height,
    }
}

fn emit_line(buf: &mut String, x1: f64, y1: f64, x2: f64, y2: f64) {
    // PlantUML's drawLine normalises so the smaller x is x1.
    let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
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

fn emit_box(buf: &mut String, x: f64, y: f64, node: &Itf) {
    write!(
        buf,
        r#"<rect fill="{fill}" height="{h}" style="stroke:{STROKE};stroke-width:1.5;" width="{w}" x="{x}" y="{y}"/>"#,
        fill = node.fill,
        h = pm::fmt_coord(BOX_H),
        w = pm::fmt_coord(node.main_w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    )
    .unwrap();
    let _ = node.text_w; // width is recomputed inside emit_text via the segmenter
    text_render::emit_text(
        buf,
        &node.label,
        &text_render::TextBase {
            x: x + PAD_X,
            y: y + TEXT_BASELINE,
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

/// `ITFComposed.drawU` (and `ITFLeaf.drawU`): draw this subtree with its frame
/// origin translated to (`ox`, `oy`).  Mirrors PlantUML's emit order: main box,
/// then left children (each preceded by its elbow line), then right children,
/// then the trunk vertical.
fn draw_itf(buf: &mut String, node: &Itf, ox: f64, oy: f64) {
    let x = node.w1(); // trunk x within this frame
    emit_box(buf, ox + x - node.main_w / 2.0, oy, node);

    if node.left.is_empty() && node.right.is_empty() {
        return;
    }

    let mut last_y1 = BOX_H;
    let mut y = BOX_H;
    for child in &node.left {
        y += MARGIN_BOTTOM;
        // child.getF2 = (childW, childH/2) for leaf, or (w1+mainW/2, mainH/2).
        let f2x = child.f2_x();
        let f2y = child.f_y();
        last_y1 = y + f2y;
        emit_line(
            buf,
            ox + x - child.width - DELTA_X + f2x,
            oy + last_y1,
            ox + x,
            oy + last_y1,
        );
        draw_itf(buf, child, ox + x - child.width - DELTA_X, oy + y);
        y += child.height;
    }

    let mut last_y2 = BOX_H;
    y = BOX_H;
    for child in &node.right {
        y += MARGIN_BOTTOM;
        let f1x = child.f1_x();
        let f1y = child.f_y();
        last_y2 = y + f1y;
        emit_line(
            buf,
            ox + x,
            oy + last_y2,
            ox + x + DELTA_X + f1x,
            oy + last_y2,
        );
        draw_itf(buf, child, ox + x + DELTA_X, oy + y);
        y += child.height;
    }

    emit_line(buf, ox + x, oy + BOX_H, ox + x, oy + last_y1.max(last_y2));
}

impl Itf {
    fn is_leaf(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
    /// `getF1.x`: attach x on the left face, relative to the subtree frame.
    fn f1_x(&self) -> f64 {
        if self.is_leaf() {
            0.0
        } else {
            self.w1() - self.main_w / 2.0
        }
    }
    /// `getF2.x`: attach x on the right face.
    fn f2_x(&self) -> f64 {
        if self.is_leaf() {
            self.width
        } else {
            self.w1() + self.main_w / 2.0
        }
    }
    /// `getF1.y` / `getF2.y`: vertical mid of this node's own box.
    fn f_y(&self) -> f64 {
        BOX_H / 2.0
    }
}

/// `Fork.drawU`: lay out the root and its top-level children.  Returns the
/// rendered subtree's total width and height (excluding outer margins) so the
/// caller can size the canvas and stack multiple roots.
fn draw_fork(buf: &mut String, root: &Itf, ox: f64, oy: f64) -> (f64, f64) {
    let main_w = root.main_w;
    let y0 = BOX_H;
    let y1 = y0 + FORK_DELTA_Y / 2.0;
    let y2 = y0 + FORK_DELTA_Y;

    // Top-level children are all on the right in Fork (left-at-root is
    // re-homed to the right at parse time in PlantUML).
    let children = &root.right;

    if children.is_empty() {
        emit_box(buf, ox, oy, root);
        emit_line(buf, ox + main_w / 2.0, oy + y0, ox + main_w / 2.0, oy + y1);
        return (main_w, y1);
    }

    let mut x = 0.0;
    let first_x = children[0].t1_x();
    let mut last_x = first_x;
    for child in children {
        last_x = x + child.t1_x();
        emit_line(buf, ox + last_x, oy + y1, ox + last_x, oy + y2);
        draw_itf(buf, child, ox + x, oy + y2);
        x += child.width + FORK_DELTA_X;
    }

    let full_w = fork_width(root);
    let pos_main = if last_x > first_x {
        emit_line(buf, ox + first_x, oy + y1, ox + last_x, oy + y1);
        first_x + (last_x - first_x - main_w) / 2.0
    } else {
        let pm = (full_w - main_w) / 2.0;
        emit_line(buf, ox + first_x, oy + y1, ox + pm + main_w / 2.0, oy + y1);
        pm
    };
    emit_box(buf, ox + pos_main, oy, root);
    emit_line(
        buf,
        ox + pos_main + main_w / 2.0,
        oy + y0,
        ox + pos_main + main_w / 2.0,
        oy + y1,
    );

    let mut h = 0.0_f64;
    for child in children {
        h = h.max(child.height);
    }
    (full_w, y0 + FORK_DELTA_Y + h)
}

impl Itf {
    /// `getT1.x`: trunk x used by `Fork` to attach top-level drop lines.
    fn t1_x(&self) -> f64 {
        if self.is_leaf() {
            self.width / 2.0
        } else {
            self.w1()
        }
    }
}

/// `Fork.calculateDimension` width.
fn fork_width(root: &Itf) -> f64 {
    let children = &root.right;
    let mut w: f64 = children.iter().map(|c| c.width).sum();
    if children.len() > 1 {
        w += (children.len() - 1) as f64 * FORK_DELTA_X;
    }
    w.max(root.main_w)
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

    let roots: Vec<Itf> = diagram.nodes.iter().map(|n| measure(n, true)).collect();

    // First pass: lay out into a scratch buffer to learn each root's extent.
    let mut body = String::with_capacity(2048);
    let mut canvas_w = 0.0_f64;
    let mut y_cursor = MARGIN;
    for root in &roots {
        let (w, h) = draw_fork(&mut body, root, MARGIN, y_cursor);
        canvas_w = canvas_w.max(w);
        y_cursor += h + MARGIN;
    }

    let total_w_i = (canvas_w + 2.0 * MARGIN).ceil() as i64;
    let total_h_i = y_cursor.ceil() as i64;

    let mut buf = String::with_capacity(body.len() + 512);
    write!(
        buf,
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="WBS" height="{total_h_i}px" preserveAspectRatio="none" style="width:{total_w_i}px;height:{total_h_i}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {total_w_i} {total_h_i}" width="{total_w_i}px" zoomAndPan="magnify"><?plantuml ?><defs/><g>"##,
    )
    .unwrap();
    buf.push_str(&body);
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
