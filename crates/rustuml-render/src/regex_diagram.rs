// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Regex railroad diagram renderer.
//!
//! Renders a [`RegexDiagram`] as an SVG railroad diagram.

use rustuml_parser::diagram::regex_diagram::{GroupKind, RegexDiagram, RegexNode};

use crate::metrics::text_width;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Constants ────────────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 14.0;
const COUNT_FONT_SIZE: f64 = 12.0;
const BOX_H: f64 = 26.4883;
const CC_ITEM_H: f64 = 16.4883;
const H_PAD: f64 = 5.0;
const MARGIN: f64 = 15.0;
const SEQ_GAP: f64 = 20.0;
/// Outset of the `+` loop connector beyond the box edge.
const LOOP_OUTSET: f64 = 7.0;
/// Top of the `+` loop arch above the box top.
const LOOP_TOP_GAP: f64 = 7.0;
/// Vertical segment length inside the `+` loop connector.
const LOOP_VERT: f64 = 8.0;
/// Gap between alternation branches.
const ALT_BRANCH_GAP: f64 = 10.0;
/// Horizontal extent of the fork/join connector (alternation).
const ALT_FORK_W: f64 = 12.0;
/// Distance from the main rail to the optional element's box top.
const OPT_BELOW: f64 = 10.0;
/// Extra downward offset for `*` inner element (so loop arch fits).
const STAR_EXTRA: f64 = 12.0;
/// Padding inside lookahead/lookbehind box.
const LOOK_INNER_PAD: f64 = 6.0;
/// Height of lookahead/lookbehind outer box.
const LOOK_BOX_H: f64 = 38.4883;
/// Bottom padding after inner content in lookahead box.
const LOOK_RIGHT_PAD: f64 = 8.0;

// ── Layout ───────────────────────────────────────────────────────────────────

/// Layout measurements for a node.
struct Layout {
    /// Total horizontal space occupied.
    width: f64,
    /// Space above the rail (= rail_y measured from box top).
    above: f64,
    /// Space below the rail.
    below: f64,
}

impl Layout {
    fn rail_y(&self) -> f64 {
        self.above
    }

    fn height(&self) -> f64 {
        self.above + self.below
    }
}

fn box_width(text: &str) -> f64 {
    text_width(text, FONT_SIZE) + 2.0 * H_PAD
}

fn char_class_item_width(items: &[String]) -> f64 {
    items
        .iter()
        .map(|it| text_width(it, FONT_SIZE))
        .fold(0.0_f64, f64::max)
        + 2.0 * H_PAD
}

fn measure(node: &RegexNode) -> Layout {
    match node {
        RegexNode::Literal { text } => Layout {
            width: box_width(text),
            above: BOX_H / 2.0,
            below: BOX_H / 2.0,
        },
        RegexNode::Special { text } => Layout {
            width: box_width(text),
            above: BOX_H / 2.0,
            below: BOX_H / 2.0,
        },
        RegexNode::CharClass { items } => {
            let w = char_class_item_width(items);
            let n = items.len() as f64;
            let h = n * CC_ITEM_H;
            Layout {
                width: w,
                above: h / 2.0,
                below: h / 2.0,
            }
        }
        RegexNode::Sequence { items } => {
            if items.is_empty() {
                return Layout {
                    width: 0.0,
                    above: BOX_H / 2.0,
                    below: BOX_H / 2.0,
                };
            }
            let mut total_w = 0.0f64;
            let mut max_above = 0.0f64;
            let mut max_below = 0.0f64;
            for (i, item) in items.iter().enumerate() {
                let m = measure(item);
                if i > 0 {
                    total_w += SEQ_GAP;
                }
                total_w += m.width;
                max_above = max_above.max(m.above);
                max_below = max_below.max(m.below);
            }
            Layout {
                width: total_w,
                above: max_above,
                below: max_below,
            }
        }
        RegexNode::Alternation { branches } => {
            let max_branch_w = branches
                .iter()
                .map(|b| measure(b).width)
                .fold(0.0_f64, f64::max);
            // above = first branch rail (= first branch above)
            let first = measure(&branches[0]);
            let n = branches.len() as f64;
            // Total height of all branches
            let total_h: f64 = branches.iter().map(|b| measure(b).height()).sum::<f64>()
                + (n - 1.0) * ALT_BRANCH_GAP;
            Layout {
                width: 2.0 * ALT_FORK_W * 2.0 + max_branch_w, // 48 + max
                above: first.above,
                below: total_h - first.above,
            }
        }
        RegexNode::Repeat { inner, min, max } => {
            let inner_m = measure(inner);
            match (*min, *max) {
                (1, None) => {
                    // + : loop above
                    Layout {
                        width: inner_m.width,
                        above: inner_m.above + LOOP_TOP_GAP + LOOP_VERT,
                        below: inner_m.below,
                    }
                }
                (0, Some(1)) => {
                    // ? : bypass above, element below
                    Layout {
                        width: inner_m.width + 2.0 * ALT_FORK_W * 2.0,
                        above: inner_m.above,
                        below: OPT_BELOW + inner_m.height(),
                    }
                }
                (0, None) => {
                    // * : bypass above, inner+ below
                    let inner_plus_above = inner_m.above + LOOP_TOP_GAP + LOOP_VERT;
                    Layout {
                        width: inner_m.width + 2.0 * ALT_FORK_W * 2.0,
                        above: inner_m.above,
                        below: OPT_BELOW + STAR_EXTRA + inner_plus_above + inner_m.below,
                    }
                }
                _ => {
                    // {n,m} or {n}: treat like + (loop above)
                    let count_label = count_label(*min, *max);
                    let label_w = text_width(&count_label, COUNT_FONT_SIZE);
                    let w = inner_m.width.max(label_w + 10.0);
                    Layout {
                        width: w,
                        above: inner_m.above + LOOP_TOP_GAP + LOOP_VERT + COUNT_FONT_SIZE + 4.0,
                        below: inner_m.below,
                    }
                }
            }
        }
        RegexNode::Group { kind, inner } => {
            let inner_m = measure(inner);
            match kind {
                GroupKind::Lookahead { .. } | GroupKind::Lookbehind { .. } => {
                    let label = group_label(kind);
                    let label_w = text_width(&label, FONT_SIZE) + 4.0;
                    let w = label_w + inner_m.width + LOOK_RIGHT_PAD;
                    Layout {
                        width: w,
                        above: LOOK_BOX_H / 2.0,
                        below: LOOK_BOX_H / 2.0,
                    }
                }
                GroupKind::Flags { flags } => {
                    let text = format!("(?{flags})");
                    Layout {
                        width: box_width(&text),
                        above: BOX_H / 2.0,
                        below: BOX_H / 2.0,
                    }
                }
                _ => {
                    // Transparent: just render inner
                    inner_m
                }
            }
        }
    }
}

fn count_label(min: u32, max: Option<u32>) -> String {
    match max {
        Some(m) if m == min => format!("{{{min}}}"),
        Some(m) => format!("{{{min},{m}}}"),
        None => format!("{{{min},}}"),
    }
}

fn group_label(kind: &GroupKind) -> String {
    match kind {
        GroupKind::Lookahead { positive: true } => "?=".to_string(),
        GroupKind::Lookahead { positive: false } => "?!".to_string(),
        GroupKind::Lookbehind { positive: true } => "?<=".to_string(),
        GroupKind::Lookbehind { positive: false } => "?<!".to_string(),
        _ => "?".to_string(),
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

/// Draw `node` with its rail at `rail_y` (absolute), positioned starting at `x`.
fn draw(node: &RegexNode, x: f64, rail_y: f64, svg: &mut SvgBuilder) {
    match node {
        RegexNode::Literal { text } => {
            let tw = text_width(text, FONT_SIZE);
            let w = tw + 2.0 * H_PAD;
            let box_y = rail_y - BOX_H / 2.0;
            svg.raw(&format!(
                r#"<rect fill="none" height="{BOX_H}" style="stroke:#181818;stroke-width:0.5;" width="{w}" x="{x}" y="{box_y}"/>"#
            ));
            svg.text(
                x + H_PAD,
                rail_y + 5.29,
                text,
                "start",
                FONT_SIZE,
            );
        }
        RegexNode::Special { text } => {
            let tw = text_width(text, FONT_SIZE);
            let w = tw + 2.0 * H_PAD;
            let box_y = rail_y - BOX_H / 2.0;
            svg.raw(&format!(
                r##"<rect fill="#F1F1F1" height="{BOX_H}" rx="5" ry="5" style="stroke:#181818;stroke-width:1.5;" width="{w}" x="{x}" y="{box_y}"/>"##
            ));
            svg.text(x + H_PAD, rail_y + 5.29, text, "start", FONT_SIZE);
        }
        RegexNode::CharClass { items } => {
            let w = char_class_item_width(items);
            let n = items.len() as f64;
            let h = n * CC_ITEM_H;
            let box_y = rail_y - h / 2.0;
            svg.raw(&format!(
                r#"<rect fill="none" height="{h}" style="stroke:#181818;stroke-width:1;stroke-dasharray:5,5;" width="{w}" x="{x}" y="{box_y}"/>"#
            ));
            for (i, item) in items.iter().enumerate() {
                let item_center_y = box_y + (i as f64 + 0.5) * CC_ITEM_H;
                svg.text(x + H_PAD, item_center_y + 5.29, item, "start", FONT_SIZE);
                // Separator line between items
                if i > 0 {
                    let sep_y = box_y + i as f64 * CC_ITEM_H;
                    svg.raw(&format!(
                        r#"<line style="stroke:#181818;stroke-width:0.3;" x1="{x}" x2="{}" y1="{sep_y}" y2="{sep_y}"/>"#,
                        x + w
                    ));
                }
            }
        }
        RegexNode::Sequence { items } => {
            if items.is_empty() {
                return;
            }
            let mut cx = x;
            for (i, item) in items.iter().enumerate() {
                let m = measure(item);
                if i > 0 {
                    // Draw connecting rail line
                    let prev_right = cx;
                    let next_left = prev_right + SEQ_GAP;
                    svg.raw(&format!(
                        r#"<line style="stroke:#181818;stroke-width:1;" x1="{prev_right}" x2="{next_left}" y1="{rail_y}" y2="{rail_y}"/>"#
                    ));
                    cx = next_left;
                }
                // Position element with its rail at rail_y
                let elem_x = cx;
                let elem_rail_y = rail_y; // rail aligns
                draw(item, elem_x, elem_rail_y, svg);
                cx += m.width;
            }
        }
        RegexNode::Alternation { branches } => {
            let max_branch_w = branches.iter().map(|b| measure(b).width).fold(0.0_f64, f64::max);
            let fork_x = x + ALT_FORK_W;
            let box_x = x + 2.0 * ALT_FORK_W;
            let exit_x = x + 4.0 * ALT_FORK_W + max_branch_w;
            let join_x = exit_x - ALT_FORK_W;

            // Compute branch rail positions
            let mut branch_rails: Vec<f64> = Vec::new();
            let mut cy = rail_y;
            for (i, b) in branches.iter().enumerate() {
                let m = measure(b);
                if i == 0 {
                    branch_rails.push(rail_y);
                    cy = rail_y + m.below + ALT_BRANCH_GAP;
                } else {
                    let br = cy + m.above;
                    branch_rails.push(br);
                    cy = br + m.below + ALT_BRANCH_GAP;
                }
            }

            // Draw first branch: straight rail
            svg.raw(&format!(
                r#"<line style="stroke:#181818;stroke-width:1;" x1="{x}" x2="{box_x}" y1="{rail_y}" y2="{rail_y}"/>"#
            ));
            let first_m = measure(&branches[0]);
            svg.raw(&format!(
                r#"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{exit_x}" y1="{rail_y}" y2="{rail_y}"/>"#,
                box_x + first_m.width
            ));
            draw(&branches[0], box_x, rail_y, svg);

            if branches.len() > 1 {
                let last_rail = *branch_rails.last().unwrap();

                // Left fork: vertical line from rail+12 to last_rail-12
                svg.raw(&format!(
                    r#"<line style="stroke:#181818;stroke-width:1;" x1="{fork_x}" x2="{fork_x}" y1="{}" y2="{}"/>"#,
                    rail_y + 12.0,
                    last_rail - 12.0
                ));

                // Left fork start curve from first branch down
                svg.raw(&format!(
                    r#"<path d="M{x},{rail_y} C{},{rail_y} {fork_x},{} {fork_x},{}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
                    fork_x - 3.0,
                    rail_y + 3.0,
                    rail_y + 12.0
                ));

                // Right fork: vertical line
                svg.raw(&format!(
                    r#"<line style="stroke:#181818;stroke-width:1;" x1="{join_x}" x2="{join_x}" y1="{}" y2="{}"/>"#,
                    rail_y + 12.0,
                    last_rail - 12.0
                ));

                // Right join end curve to first branch
                svg.raw(&format!(
                    r#"<path d="M{join_x},{} C{join_x},{} {},{rail_y} {exit_x},{rail_y}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
                    rail_y + 12.0,
                    rail_y + 3.0,
                    join_x + 3.0
                ));

                // Additional branches
                for (i, branch) in branches.iter().enumerate().skip(1) {
                    let br = branch_rails[i];
                    let bm = measure(branch);

                    // Left: fork to branch
                    svg.raw(&format!(
                        r#"<path d="M{fork_x},{} C{fork_x},{} {},{br} {box_x},{br}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
                        br - 12.0,
                        br - 3.0,
                        fork_x + 3.0
                    ));

                    // Rail from box to join_x-level
                    let branch_right = box_x + bm.width;
                    let max_right = box_x + max_branch_w;
                    svg.raw(&format!(
                        r#"<line style="stroke:#181818;stroke-width:1;" x1="{branch_right}" x2="{max_right}" y1="{br}" y2="{br}"/>"#
                    ));

                    // Right: branch to join
                    svg.raw(&format!(
                        r#"<path d="M{join_x},{} C{join_x},{} {},{br} {max_right},{br}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
                        br - 12.0,
                        br - 3.0,
                        join_x - 3.0
                    ));

                    draw(branch, box_x, br, svg);
                }
            }
        }
        RegexNode::Repeat { inner, min, max } => {
            let inner_m = measure(inner);
            match (*min, *max) {
                (1, None) => {
                    // + : loop connector above
                    draw_plus_loop(inner, inner_m.width, x, rail_y, svg, None);
                }
                (0, Some(1)) => {
                    // ? : bypass above, element below
                    let opt_rail = rail_y + OPT_BELOW + inner_m.above;
                    let elem_x = x + 2.0 * ALT_FORK_W;
                    let exit_x = x + 4.0 * ALT_FORK_W + inner_m.width;

                    // Bypass at main rail
                    draw_bypass_line(x, exit_x, rail_y, svg);
                    // S-curves down to inner
                    draw_s_curve(x, rail_y, elem_x, opt_rail, svg);
                    draw_s_curve_right(x + 4.0 * ALT_FORK_W + inner_m.width - 2.0 * ALT_FORK_W, opt_rail, exit_x, rail_y, svg);
                    // Inner element
                    let inner_x = elem_x;
                    draw_rail_line(inner_x - SEQ_GAP + ALT_FORK_W, inner_x, opt_rail, svg);
                    draw(inner, inner_x, opt_rail, svg);
                    draw_rail_line(inner_x + inner_m.width, inner_x + inner_m.width + ALT_FORK_W, opt_rail, svg);
                }
                (0, None) => {
                    // * : bypass above + loop inner below
                    let inner_plus_above = inner_m.above + LOOP_TOP_GAP + LOOP_VERT;
                    let opt_rail = rail_y + OPT_BELOW + STAR_EXTRA + inner_plus_above;
                    let elem_x = x + 2.0 * ALT_FORK_W;
                    let exit_x = x + 4.0 * ALT_FORK_W + inner_m.width;

                    // Bypass at main rail
                    draw_bypass_line(x, exit_x, rail_y, svg);
                    // S-curves
                    draw_s_curve(x, rail_y, elem_x, opt_rail, svg);
                    draw_s_curve_right(elem_x + inner_m.width, opt_rail, exit_x, rail_y, svg);
                    // Inner element with + loop
                    draw_rail_line(elem_x - ALT_FORK_W, elem_x, opt_rail, svg);
                    draw_plus_loop(inner, inner_m.width, elem_x, opt_rail, svg, None);
                    draw_rail_line(elem_x + inner_m.width, elem_x + inner_m.width + ALT_FORK_W, opt_rail, svg);
                }
                _ => {
                    // {n,m} or {n}: like + with count label
                    let label = count_label(*min, *max);
                    draw_plus_loop(inner, inner_m.width, x, rail_y, svg, Some(&label));
                }
            }
        }
        RegexNode::Group { kind, inner } => {
            match kind {
                GroupKind::Lookahead { .. } | GroupKind::Lookbehind { .. } => {
                    let label = group_label(kind);
                    let label_w = text_width(&label, FONT_SIZE) + 4.0;
                    let inner_m = measure(inner);
                    let w = label_w + inner_m.width + LOOK_RIGHT_PAD;
                    let box_y = rail_y - LOOK_BOX_H / 2.0;
                    // Outer dashed rounded rect
                    svg.raw(&format!(
                        r#"<rect fill="none" height="{LOOK_BOX_H}" rx="15" ry="15" style="stroke:#181818;stroke-width:1;stroke-dasharray:2,3;" width="{w}" x="{x}" y="{box_y}"/>"#
                    ));
                    // Label text
                    svg.text(x + H_PAD - 1.0, rail_y + 4.95, &label, "start", FONT_SIZE);
                    // Inner element
                    let inner_x = x + label_w;
                    draw(inner, inner_x, rail_y, svg);
                }
                GroupKind::Flags { flags } => {
                    let text = format!("(?{flags})");
                    let tw = text_width(&text, FONT_SIZE);
                    let w = tw + 2.0 * H_PAD;
                    let box_y = rail_y - BOX_H / 2.0;
                    svg.raw(&format!(
                        r##"<rect fill="#F1F1F1" height="{BOX_H}" rx="5" ry="5" style="stroke:#181818;stroke-width:1.5;" width="{w}" x="{x}" y="{box_y}"/>"##
                    ));
                    svg.text(x + H_PAD, rail_y + 5.29, &text, "start", FONT_SIZE);
                }
                _ => {
                    // Transparent: render inner at same position
                    draw(inner, x, rail_y, svg);
                }
            }
        }
    }
}

fn draw_rail_line(x1: f64, x2: f64, y: f64, svg: &mut SvgBuilder) {
    if (x2 - x1).abs() > 0.01 {
        svg.raw(&format!(
            r#"<line style="stroke:#181818;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#
        ));
    }
}

fn draw_bypass_line(x: f64, exit_x: f64, rail_y: f64, svg: &mut SvgBuilder) {
    // Bypass line with forward arrow
    let mid_x = (x + exit_x) / 2.0;
    svg.raw(&format!(
        r#"<line style="stroke:#181818;stroke-width:1;" x1="{x}" x2="{exit_x}" y1="{rail_y}" y2="{rail_y}"/>"#
    ));
    // Forward arrow
    svg.raw(&format!(
        r##"<path d="M{},{rail_y} L{},{} L{},{rail_y} L{},{} L{},{rail_y}" fill="#181818"/>"##,
        mid_x - 3.0,
        mid_x - 3.0,
        rail_y - 3.0,
        mid_x + 3.0,
        mid_x - 3.0,
        rail_y + 3.0,
        mid_x - 3.0
    ));
}

/// Draw S-curve from (x1, y1) curving down to (x2, y2) where y2 > y1.
fn draw_s_curve(x1: f64, y1: f64, x2: f64, y2: f64, svg: &mut SvgBuilder) {
    let mid_x = (x1 + x2) / 2.0;
    let mid_y = (y1 + y2) / 2.0;
    svg.raw(&format!(
        r#"<path d="M{x1},{y1} C{},{y1} {mid_x},{} {mid_x},{mid_y} C{mid_x},{} {},{y2} {x2},{y2}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
        x1 + 9.0,
        mid_y - 9.0,
        mid_y + 9.0,
        x2 - 9.0
    ));
}

/// Draw S-curve from (x1, y1) going UP to (x2, y2) where y2 < y1.
fn draw_s_curve_right(x1: f64, y1: f64, x2: f64, y2: f64, svg: &mut SvgBuilder) {
    let mid_x = (x1 + x2) / 2.0;
    let mid_y = (y1 + y2) / 2.0;
    svg.raw(&format!(
        r#"<path d="M{x1},{y1} C{},{y1} {mid_x},{} {mid_x},{mid_y} C{mid_x},{} {},{y2} {x2},{y2}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
        x1 + 9.0,
        mid_y + 9.0,
        mid_y - 9.0,
        x2 - 9.0
    ));
}

/// Draw the `+` loop connector around an element of given width at (x, rail_y).
/// Optionally draw a count label on the loop.
fn draw_plus_loop(
    inner: &RegexNode,
    elem_w: f64,
    x: f64,
    rail_y: f64,
    svg: &mut SvgBuilder,
    label: Option<&str>,
) {
    let inner_m = measure(inner);
    let box_y = rail_y - inner_m.above;
    let lx = x - LOOP_OUTSET;
    let rx = x + elem_w + LOOP_OUTSET;
    let top_y = box_y - LOOP_TOP_GAP;

    // Left lower curve: lx,rail_y-8 → x+1,rail_y
    svg.raw(&format!(
        r#"<path d="M{lx},{} C{lx},{rail_y} {},{rail_y} {},{rail_y}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
        rail_y - LOOP_VERT,
        x + 2.0,
        x + 1.0
    ));
    // Left vertical
    svg.raw(&format!(
        r#"<line style="stroke:#181818;stroke-width:1;" x1="{lx}" x2="{lx}" y1="{}" y2="{}"/>"#,
        top_y + LOOP_VERT,
        rail_y - LOOP_VERT
    ));
    // Left upper curve: lx,top_y+8 → x+1,top_y
    svg.raw(&format!(
        r#"<path d="M{lx},{} C{lx},{} {},{top_y} {},{top_y}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
        top_y + LOOP_VERT,
        top_y + 2.0,
        lx + 2.0,
        x + 1.0
    ));
    // Top line with back-arrow
    let box_right = x + elem_w;
    let mid_top_x = (x + box_right) / 2.0;
    svg.raw(&format!(
        r#"<line style="stroke:#181818;stroke-width:1;" x1="{x}" x2="{box_right}" y1="{top_y}" y2="{top_y}"/>"#
    ));
    // Back-arrow pointing left
    svg.raw(&format!(
        r##"<path d="M{},{top_y} L{},{} L{},{top_y} L{},{} L{},{top_y}" fill="#181818"/>"##,
        mid_top_x + 3.0,
        mid_top_x + 3.0,
        top_y - 3.0,
        mid_top_x - 3.0,
        mid_top_x + 3.0,
        top_y + 3.0,
        mid_top_x + 3.0
    ));

    // Optional count label
    if let Some(lbl) = label {
        let lw = text_width(lbl, COUNT_FONT_SIZE);
        let lx_text = x + (elem_w - lw) / 2.0;
        let ly_text = top_y - 4.0;
        svg.text(lx_text, ly_text, lbl, "start", COUNT_FONT_SIZE);
    }

    // Right upper curve: box_right-1,top_y → rx,top_y+8
    svg.raw(&format!(
        r#"<path d="M{},{top_y} C{},{top_y} {rx},{} {rx},{}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
        box_right - 1.0,
        rx - 6.0,
        top_y + 2.0,
        top_y + LOOP_VERT
    ));
    // Right vertical
    svg.raw(&format!(
        r#"<line style="stroke:#181818;stroke-width:1;" x1="{rx}" x2="{rx}" y1="{}" y2="{}"/>"#,
        top_y + LOOP_VERT,
        rail_y - LOOP_VERT
    ));
    // Right lower curve: rx,rail_y-8 → box_right-1,rail_y
    svg.raw(&format!(
        r#"<path d="M{rx},{} C{rx},{rail_y} {},{rail_y} {},{rail_y}" fill="none" style="stroke:#181818;stroke-width:1;"/>"#,
        rail_y - LOOP_VERT,
        box_right - 1.0,
        box_right - 1.0
    ));

    // Draw the inner element
    draw(inner, x, rail_y, svg);
}

// ── Public render function ────────────────────────────────────────────────────

/// Render a [`RegexDiagram`] to an SVG string.
pub fn render(diagram: &RegexDiagram, _theme: &Theme) -> String {
    let ast = &diagram.ast;
    let m = measure(ast);

    // Global rail: MARGIN + max above
    let global_rail = MARGIN + m.above;
    let canvas_w = (MARGIN + m.width + MARGIN).ceil();
    let canvas_h = (global_rail + m.below + 15.5).ceil();

    let mut svg = SvgBuilder::new(canvas_w, canvas_h);

    // Left rail line into content
    let content_x = MARGIN;
    svg.raw(&format!(
        r#"<line style="stroke:#181818;stroke-width:1;" x1="{content_x}" x2="{content_x}" y1="{global_rail}" y2="{global_rail}"/>"#
    ));

    draw(ast, content_x, global_rail, &mut svg);

    svg.finalize()
}
