// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! EBNF railroad diagram renderer.
//!
//! Each production rule is rendered as a labelled section with a railroad-style
//! track.  Terminals are rectangular boxes, nonterminals are rounded-rectangle
//! boxes.  Alternation branches vertically, repetition draws a loop-back arc,
//! and optional draws a bypass path.

use rustuml_parser::diagram::ebnf::{EbnfDiagram, EbnfExpr, EbnfRule};

use crate::metrics::text_width;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ──────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 14.0;
const LABEL_FONT_SIZE: f64 = 16.0;
const BOX_H: f64 = 28.0;
const H_PAD: f64 = 8.0;
const MARGIN: f64 = 20.0;
const SEQ_GAP: f64 = 16.0;
const ALT_BRANCH_GAP: f64 = 12.0;
const ALT_FORK_W: f64 = 16.0;
const LOOP_OUTSET: f64 = 10.0;
const LOOP_TOP_GAP: f64 = 10.0;
const LOOP_VERT: f64 = 10.0;
const OPT_BELOW: f64 = 12.0;
const RULE_GAP: f64 = 30.0;
const LABEL_GAP: f64 = 8.0;
const RAIL_EXTEND: f64 = 20.0;
const START_END_R: f64 = 5.0;

// ── Colours ────────────────────────────────────────────────────────────────────

const TERMINAL_FILL: &str = "#FEFECE";
const TERMINAL_STROKE: &str = "#A80036";
const NONTERM_FILL: &str = "#F1F1F1";
const NONTERM_STROKE: &str = "#181818";
const RAIL_STROKE: &str = "#181818";

// ── Layout measurement ────────────────────────────────────────────────────────

struct Layout {
    width: f64,
    above: f64,
    below: f64,
}

impl Layout {
    fn height(&self) -> f64 {
        self.above + self.below
    }
}

fn box_width(text: &str) -> f64 {
    text_width(text, FONT_SIZE) + 2.0 * H_PAD
}

fn measure(expr: &EbnfExpr) -> Layout {
    match expr {
        EbnfExpr::Terminal(s) => Layout {
            width: box_width(s),
            above: BOX_H / 2.0,
            below: BOX_H / 2.0,
        },
        EbnfExpr::Nonterminal(s) => Layout {
            width: box_width(s),
            above: BOX_H / 2.0,
            below: BOX_H / 2.0,
        },
        EbnfExpr::Sequence(items) => {
            if items.is_empty() {
                return Layout {
                    width: 0.0,
                    above: BOX_H / 2.0,
                    below: BOX_H / 2.0,
                };
            }
            let mut total_w = 0.0;
            let mut max_above = 0.0_f64;
            let mut max_below = 0.0_f64;
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
        EbnfExpr::Alternation(branches) => {
            if branches.is_empty() {
                return Layout {
                    width: 0.0,
                    above: BOX_H / 2.0,
                    below: BOX_H / 2.0,
                };
            }
            let max_branch_w = branches
                .iter()
                .map(|b| measure(b).width)
                .fold(0.0_f64, f64::max);
            let first = measure(&branches[0]);
            let total_h: f64 = branches.iter().map(|b| measure(b).height()).sum::<f64>()
                + (branches.len() as f64 - 1.0) * ALT_BRANCH_GAP;
            Layout {
                width: 2.0 * ALT_FORK_W + max_branch_w + 2.0 * ALT_FORK_W,
                above: first.above,
                below: total_h - first.above,
            }
        }
        EbnfExpr::Repetition(inner) => {
            let inner_m = measure(inner);
            Layout {
                width: inner_m.width,
                above: inner_m.above + LOOP_TOP_GAP + LOOP_VERT,
                below: inner_m.below,
            }
        }
        EbnfExpr::Optional(inner) => {
            let inner_m = measure(inner);
            Layout {
                width: inner_m.width + 2.0 * ALT_FORK_W,
                above: inner_m.above,
                below: (OPT_BELOW + inner_m.height()).max(inner_m.below),
            }
        }
        EbnfExpr::Group(inner) => measure(inner),
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(expr: &EbnfExpr, x: f64, rail_y: f64, svg: &mut SvgBuilder) {
    match expr {
        EbnfExpr::Terminal(text) => {
            let w = box_width(text);
            let box_y = rail_y - BOX_H / 2.0;
            // Rectangular box for terminals.
            svg.raw(&format!(
                r#"<rect x="{x}" y="{box_y}" width="{w}" height="{BOX_H}" fill="{TERMINAL_FILL}" stroke="{TERMINAL_STROKE}" stroke-width="1"/>"#
            ));
            svg.plain_text(
                x + w / 2.0,
                rail_y + FONT_SIZE / 2.0 - 2.0,
                text,
                "middle",
                FONT_SIZE,
            );
        }
        EbnfExpr::Nonterminal(text) => {
            let w = box_width(text);
            let box_y = rail_y - BOX_H / 2.0;
            // Rounded-rectangle box for nonterminals.
            svg.raw(&format!(
                r#"<rect x="{x}" y="{box_y}" width="{w}" height="{BOX_H}" rx="6" ry="6" fill="{NONTERM_FILL}" stroke="{NONTERM_STROKE}" stroke-width="1"/>"#
            ));
            svg.plain_text(
                x + w / 2.0,
                rail_y + FONT_SIZE / 2.0 - 2.0,
                text,
                "middle",
                FONT_SIZE,
            );
        }
        EbnfExpr::Sequence(items) => {
            let mut cx = x;
            for (i, item) in items.iter().enumerate() {
                let m = measure(item);
                if i > 0 {
                    // Draw connecting rail line.
                    let next_x = cx + SEQ_GAP;
                    draw_rail(cx, next_x, rail_y, svg);
                    cx = next_x;
                }
                draw(item, cx, rail_y, svg);
                cx += m.width;
            }
        }
        EbnfExpr::Alternation(branches) => {
            if branches.is_empty() {
                return;
            }
            let max_branch_w = branches
                .iter()
                .map(|b| measure(b).width)
                .fold(0.0_f64, f64::max);
            let box_x = x + ALT_FORK_W;
            let total_w = 2.0 * ALT_FORK_W + max_branch_w + 2.0 * ALT_FORK_W;
            let exit_x = x + total_w;
            let content_right = box_x + max_branch_w;

            // Compute branch rail Y positions.
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

            // Draw first branch: straight rail through.
            draw_rail(x, box_x, rail_y, svg);
            let first_m = measure(&branches[0]);
            draw(&branches[0], box_x, rail_y, svg);
            draw_rail(box_x + first_m.width, exit_x, rail_y, svg);

            // Additional branches.
            if branches.len() > 1 {
                let last_rail = *branch_rails.last().unwrap();

                // Left fork vertical.
                let fork_x = x + ALT_FORK_W / 2.0;
                svg.raw(&format!(
                    r#"<line x1="{fork_x}" y1="{}" x2="{fork_x}" y2="{}" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                    rail_y + 10.0,
                    last_rail - 10.0
                ));
                // Left fork curve from main rail.
                svg.raw(&format!(
                    r#"<path d="M{x},{rail_y} Q{fork_x},{rail_y} {fork_x},{}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                    rail_y + 10.0
                ));

                // Right join vertical.
                let join_x = exit_x - ALT_FORK_W / 2.0;
                svg.raw(&format!(
                    r#"<line x1="{join_x}" y1="{}" x2="{join_x}" y2="{}" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                    rail_y + 10.0,
                    last_rail - 10.0
                ));
                // Right join curve to main rail.
                svg.raw(&format!(
                    r#"<path d="M{join_x},{} Q{join_x},{rail_y} {exit_x},{rail_y}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                    rail_y + 10.0
                ));

                for (i, branch) in branches.iter().enumerate().skip(1) {
                    let br = branch_rails[i];
                    let bm = measure(branch);

                    // Left curve to this branch.
                    svg.raw(&format!(
                        r#"<path d="M{fork_x},{} Q{fork_x},{br} {box_x},{br}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                        br - 10.0
                    ));

                    draw(branch, box_x, br, svg);

                    // Rail from branch end to right side.
                    draw_rail(box_x + bm.width, content_right, br, svg);

                    // Right curve from this branch.
                    svg.raw(&format!(
                        r#"<path d="M{content_right},{br} Q{join_x},{br} {join_x},{}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                        br - 10.0
                    ));
                }
            }
        }
        EbnfExpr::Repetition(inner) => {
            let inner_m = measure(inner);
            let elem_w = inner_m.width;

            // Draw the inner element on the rail.
            draw(inner, x, rail_y, svg);

            // Loop-back arc above.
            let lx = x - LOOP_OUTSET;
            let rx = x + elem_w + LOOP_OUTSET;
            let top_y = rail_y - inner_m.above - LOOP_TOP_GAP;

            // Left curve up.
            svg.raw(&format!(
                r#"<path d="M{},{rail_y} Q{lx},{rail_y} {lx},{}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                x.max(lx),
                rail_y - LOOP_VERT
            ));
            // Left vertical.
            svg.raw(&format!(
                r#"<line x1="{lx}" y1="{}" x2="{lx}" y2="{}" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                top_y + LOOP_VERT,
                rail_y - LOOP_VERT
            ));
            // Left curve to top.
            svg.raw(&format!(
                r#"<path d="M{lx},{} Q{lx},{top_y} {},{top_y}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                top_y + LOOP_VERT,
                x + 1.0
            ));
            // Top rail.
            let box_right = x + elem_w;
            svg.raw(&format!(
                r#"<line x1="{x}" y1="{top_y}" x2="{box_right}" y2="{top_y}" stroke="{RAIL_STROKE}" stroke-width="1"/>"#
            ));
            // Back-arrow on top rail.
            let mid_x = (x + box_right) / 2.0;
            svg.raw(&format!(
                r##"<path d="M{} {} L{} {} L{} {} Z" fill="{RAIL_STROKE}"/>"##,
                mid_x + 3.0,
                top_y,
                mid_x - 3.0,
                top_y - 3.0,
                mid_x - 3.0,
                top_y + 3.0
            ));
            // Right curve from top.
            svg.raw(&format!(
                r#"<path d="M{},{top_y} Q{rx},{top_y} {rx},{}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                box_right - 1.0,
                top_y + LOOP_VERT
            ));
            // Right vertical.
            svg.raw(&format!(
                r#"<line x1="{rx}" y1="{}" x2="{rx}" y2="{}" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                top_y + LOOP_VERT,
                rail_y - LOOP_VERT
            ));
            // Right curve down.
            svg.raw(&format!(
                r#"<path d="M{rx},{} Q{rx},{rail_y} {},{rail_y}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#,
                rail_y - LOOP_VERT,
                box_right.min(rx)
            ));
        }
        EbnfExpr::Optional(inner) => {
            let inner_m = measure(inner);
            let elem_x = x + ALT_FORK_W;
            let total_w = inner_m.width + 2.0 * ALT_FORK_W;
            let exit_x = x + total_w;
            let opt_rail = rail_y + OPT_BELOW + inner_m.above;

            // Bypass at main rail (straight through).
            draw_rail(x, exit_x, rail_y, svg);

            // S-curve down to optional element.
            svg.raw(&format!(
                r#"<path d="M{x},{rail_y} Q{x},{opt_rail} {elem_x},{opt_rail}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#
            ));

            // Draw optional element.
            draw(inner, elem_x, opt_rail, svg);

            // S-curve back up from optional element.
            let elem_right = elem_x + inner_m.width;
            svg.raw(&format!(
                r#"<path d="M{elem_right},{opt_rail} Q{exit_x},{opt_rail} {exit_x},{rail_y}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1"/>"#
            ));
        }
        EbnfExpr::Group(inner) => {
            // Transparent: render inner directly.
            draw(inner, x, rail_y, svg);
        }
    }
}

fn draw_rail(x1: f64, x2: f64, y: f64, svg: &mut SvgBuilder) {
    if (x2 - x1).abs() > 0.01 {
        svg.raw(&format!(
            r#"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{RAIL_STROKE}" stroke-width="1"/>"#
        ));
    }
}

// ── Rule rendering ────────────────────────────────────────────────────────────

fn render_rule(rule: &EbnfRule, y_offset: f64, svg: &mut SvgBuilder) -> f64 {
    let m = measure(&rule.body);

    // Label for the rule name.
    let label_y = y_offset + LABEL_FONT_SIZE;
    svg.plain_text(MARGIN, label_y, &rule.name, "start", LABEL_FONT_SIZE);

    let content_y = label_y + LABEL_GAP;
    let rail_y = content_y + m.above;

    // Start circle (open).
    let start_cx = MARGIN;
    svg.raw(&format!(
        r#"<circle cx="{start_cx}" cy="{rail_y}" r="{START_END_R}" fill="none" stroke="{RAIL_STROKE}" stroke-width="1.5"/>"#
    ));

    // Rail from start to content.
    let content_x = start_cx + START_END_R + RAIL_EXTEND;
    draw_rail(start_cx + START_END_R, content_x, rail_y, svg);

    // Draw the expression.
    draw(&rule.body, content_x, rail_y, svg);

    // Rail from content to end.
    let content_right = content_x + m.width;
    let end_cx = content_right + RAIL_EXTEND;
    draw_rail(content_right, end_cx, rail_y, svg);

    // End circle (filled).
    svg.raw(&format!(
        r#"<circle cx="{end_cx}" cy="{rail_y}" r="{START_END_R}" fill="{RAIL_STROKE}" stroke="{RAIL_STROKE}" stroke-width="1.5"/>"#
    ));

    // Return total height consumed by this rule.

    LABEL_FONT_SIZE + LABEL_GAP + m.height()
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn render(diagram: &EbnfDiagram, _theme: &Theme) -> String {
    if diagram.rules.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // First pass: measure all rules to determine canvas size.
    let mut total_h = MARGIN;
    let mut max_w = 0.0_f64;

    let mut rule_heights: Vec<f64> = Vec::new();
    for rule in &diagram.rules {
        let m = measure(&rule.body);
        let label_h = LABEL_FONT_SIZE + LABEL_GAP;
        let rule_h = label_h + m.height();
        let rule_w =
            MARGIN + START_END_R + RAIL_EXTEND + m.width + RAIL_EXTEND + START_END_R + MARGIN;
        max_w = max_w.max(rule_w);
        rule_heights.push(rule_h);
        total_h += rule_h + RULE_GAP;
    }
    total_h -= RULE_GAP; // no gap after last rule
    total_h += MARGIN;

    let mut svg = SvgBuilder::new_plantuml(max_w, total_h, "EBNF");

    // Second pass: render each rule.
    let mut y = MARGIN;
    for (i, rule) in diagram.rules.iter().enumerate() {
        let h = render_rule(rule, y, &mut svg);
        y += h + RULE_GAP;
        let _ = rule_heights[i]; // already computed
    }

    svg.finalize_plantuml()
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_simple_ebnf() {
        let input = "@startebnf\nop = \"+\" | \"-\";\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"), "should produce SVG");
        assert!(svg.contains("+"), "svg should contain terminal '+'");
        assert!(svg.contains("-"), "svg should contain terminal '-'");
        assert!(svg.contains("op"), "svg should contain rule name 'op'");
    }

    #[test]
    fn renders_multiple_rules() {
        let input = "@startebnf\noperator = add_op | mul_op;\nadd_op = \"+\" | \"-\";\nmul_op = \"*\" | \"/\";\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("operator"));
        assert!(svg.contains("add_op"));
        assert!(svg.contains("mul_op"));
    }

    #[test]
    fn renders_repetition() {
        let input = "@startebnf\nlist = { item };\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("item"));
        assert!(svg.contains("list"));
    }

    #[test]
    fn renders_optional() {
        let input = "@startebnf\nmaybe = [ thing ];\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("thing"));
    }

    #[test]
    fn empty_ebnf_does_not_panic() {
        let input = "@startebnf\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"));
    }
}
