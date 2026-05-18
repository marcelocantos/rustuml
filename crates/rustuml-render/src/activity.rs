// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram SVG renderer.
//!
//! Produces SVG output matching PlantUML's exact format, using PlantUML-
//! compatible font metrics and layout algorithms.

use std::fmt::Write;

use rustuml_parser::diagram::activity::{ActivityDiagram, ActivityStep, NotePosition};

use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// PlantUML activity diagram constants (reverse-engineered from golden SVGs).
const START_R: f64 = 10.0;
const STOP_OUTER_R: f64 = 11.0;
const STOP_INNER_R: f64 = 6.0;
const START_CY: f64 = 25.0;
const ARROW_LEN: f64 = 20.0;
const ACTION_PADDING: f64 = 20.0; // total vertical padding in action box
const ACTION_H_PADDING: f64 = 10.0; // horizontal padding each side
const ACTION_RX: f64 = 12.5;
const ACTION_MIN_X: f64 = 16.0; // minimum x position for elements
const DIAMOND_HALF: f64 = 12.0; // half-size of decision diamond
const FORK_BAR_HEIGHT: f64 = 6.0;
const FORK_BAR_RX: f64 = 2.5;
const FORK_BAR_MARGIN: f64 = 14.0; // margin on each side of fork bar

const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 11.0;
const TITLE_FONT_SIZE: f64 = 14.0;

const START_FILL: &str = "#222222";
const START_STROKE: &str = "#222222";
const STOP_STROKE: &str = "#222222";
const STOP_FILL: &str = "#222222";
const ACTION_FILL: &str = "#F1F1F1";
const ACTION_STROKE: &str = "#181818";
const ACTION_STROKE_WIDTH: &str = "0.5";
const ARROW_COLOR: &str = "#181818";
const DIAMOND_FILL: &str = "#F1F1F1";
const FORK_BAR_COLOR: &str = "#555555";
const TEXT_COLOR: &str = "#000000";
const DEPRECATED_FILL: &str = "#FFFFCC";
const DEPRECATED_STROKE: &str = "#FFDD88";

/// Detect deprecated `#color:text;` actions and prepend a warning banner.
/// Returns the raw (un-escaped) banner string; XML/entity escaping happens
/// in the emitter via [`svg_text_escape`].
fn deprecated_warning(color: &str) -> String {
    format!(
        "This\u{a0}syntax\u{a0}is\u{a0}deprecated,\u{a0}you\u{a0}must\u{a0}add\u{a0}<<{color}>>\u{a0}at\u{a0}the\u{a0}end\u{a0}of\u{a0}the\u{a0}line,\u{a0}after\u{a0}the\u{a0}';'"
    )
}

/// Per-arrow visual style. Derived from the parser's `Arrow.color` field,
/// which actually carries the comma-separated bracket payload of
/// `-[...]->` (e.g. `bold`, `dashed`, `#red`, `#red,bold`).
#[derive(Debug, Clone)]
struct ArrowStyle {
    color: String,
    dashed: bool,
    bold: bool,
    hidden: bool,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        ArrowStyle {
            color: ARROW_COLOR.to_string(),
            dashed: false,
            bold: false,
            hidden: false,
        }
    }
}

/// Parse the bracketed payload from `-[...]->` into an `ArrowStyle`.
/// Accepts tokens separated by `,` or `;`; tokens may be a colour (`#fff`,
/// `#FFFFFF`, or a CSS name) or a style keyword (`bold`, `dashed`,
/// `dotted`, `hidden`, `plain`).
///
/// `parser_dashed` is intentionally ignored when this function is called:
/// the parser's dashed flag is set whenever a `-` appears after `]`, which
/// is true for all `-[…]->` forms regardless of the actual style.  Dash-ness
/// must come from a `dashed`/`dotted` token inside the brackets.
fn arrow_style_from_brackets(payload: &str, _parser_dashed: bool) -> ArrowStyle {
    let mut style = ArrowStyle::default();
    for tok in payload.split([',', ';']) {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix('#') {
            // Hex colour or CSS name (resolve_color handles both forms).
            style.color = crate::sequence::resolve_color(rest);
            continue;
        }
        match t.to_ascii_lowercase().as_str() {
            "bold" => style.bold = true,
            "dashed" => style.dashed = true,
            "dotted" => {
                // PlantUML renders dotted as a shorter dasharray. For now we
                // treat dotted same as dashed at the SVG level (matches the
                // `stroke-dasharray:2,2` pattern many goldens use).
                style.dashed = true;
            }
            "hidden" => style.hidden = true,
            "plain" | "solid" | "normal" => {}
            other => {
                // Bare CSS colour name without `#` prefix.
                let resolved = crate::sequence::resolve_color(other);
                if resolved != "#FFFFFF" || other.eq_ignore_ascii_case("white") {
                    style.color = resolved;
                }
            }
        }
    }
    style
}

/// A layout node in the activity tree. We convert the flat step list into
/// a tree of these nodes, compute sizes, then emit SVG.
#[derive(Debug)]
#[allow(dead_code)]
enum LayoutNode {
    Start,
    Stop,
    End,
    Action {
        text: String,
        text_width: f64,
    },
    DeprecatedAction {
        color: String,
        text: String,
        text_width: f64,
        warning_width: f64,
    },
    If {
        condition: String,
        then_label: Option<String>,
        then_branch: Vec<LayoutNode>,
        else_branches: Vec<ElseBranch>,
    },
    While {
        condition: String,
        is_label: Option<String>,
        body: Vec<LayoutNode>,
        end_label: Option<String>,
    },
    Repeat {
        body: Vec<LayoutNode>,
        condition: String,
        is_label: Option<String>,
        not_label: Option<String>,
    },
    Fork {
        branches: Vec<Vec<LayoutNode>>,
    },
    Arrow {
        dashed: bool,
        color: Option<String>,
        label: Option<String>,
    },
    Note {
        text: String,
        position: NotePosition,
        color: Option<String>,
    },
    Detach,
    Kill,
    Break,
    Title(String),
}

#[derive(Debug)]
struct ElseBranch {
    label: Option<String>,
    body: Vec<LayoutNode>,
}

/// Build a layout tree from the flat step list.
fn build_tree(steps: &[ActivityStep]) -> Vec<LayoutNode> {
    let mut nodes = Vec::new();
    let mut i = 0;
    while i < steps.len() {
        match &steps[i] {
            ActivityStep::Start => {
                nodes.push(LayoutNode::Start);
                i += 1;
            }
            ActivityStep::Stop => {
                nodes.push(LayoutNode::Stop);
                i += 1;
            }
            ActivityStep::End => {
                nodes.push(LayoutNode::End);
                i += 1;
            }
            ActivityStep::Action(text) => {
                let tw = text_render::measure(text, FONT_SIZE, false);
                nodes.push(LayoutNode::Action {
                    text: text.clone(),
                    text_width: tw,
                });
                i += 1;
            }
            ActivityStep::DeprecatedColorAction(dca) => {
                let tw = text_render::measure(&dca.text, FONT_SIZE, false);
                let warning = deprecated_warning(&dca.color);
                let ww = pm::mono_text_width(&warning, 10.0);
                nodes.push(LayoutNode::DeprecatedAction {
                    color: dca.color.clone(),
                    text: dca.text.clone(),
                    text_width: tw,
                    warning_width: ww,
                });
                i += 1;
            }
            ActivityStep::If(block) => {
                i += 1;
                let then_branch = collect_until_else_or_endif(steps, &mut i);
                let mut else_branches = Vec::new();
                while i < steps.len() {
                    match &steps[i] {
                        ActivityStep::Else(_) | ActivityStep::ElseIf(_) => {
                            let label = match &steps[i] {
                                ActivityStep::Else(l) => l.clone(),
                                ActivityStep::ElseIf(eb) => eb.then_label.clone(),
                                _ => None,
                            };
                            i += 1;
                            let body = collect_until_else_or_endif(steps, &mut i);
                            else_branches.push(ElseBranch { label, body });
                        }
                        ActivityStep::EndIf => {
                            i += 1;
                            break;
                        }
                        _ => break,
                    }
                }
                nodes.push(LayoutNode::If {
                    condition: block.condition.clone(),
                    then_label: block.then_label.clone(),
                    then_branch,
                    else_branches,
                });
            }
            ActivityStep::ElseIf(_) | ActivityStep::Else(_) | ActivityStep::EndIf => {
                // These should be consumed by If handler; skip if orphaned.
                i += 1;
            }
            ActivityStep::While(w) => {
                i += 1;
                let body = collect_until(steps, &mut i, |s| matches!(s, ActivityStep::EndWhile(_)));
                let end_label = if i < steps.len() {
                    if let ActivityStep::EndWhile(l) = &steps[i] {
                        i += 1;
                        l.clone()
                    } else {
                        None
                    }
                } else {
                    None
                };
                nodes.push(LayoutNode::While {
                    condition: w.condition.clone(),
                    is_label: w.is_label.clone(),
                    body,
                    end_label,
                });
            }
            ActivityStep::EndWhile(_) => {
                i += 1;
            }
            ActivityStep::Repeat => {
                i += 1;
                let body =
                    collect_until(steps, &mut i, |s| matches!(s, ActivityStep::RepeatWhile(_)));
                let (condition, is_label, not_label) = if i < steps.len() {
                    if let ActivityStep::RepeatWhile(rw) = &steps[i] {
                        i += 1;
                        (
                            rw.condition.clone(),
                            rw.is_label.clone(),
                            rw.not_label.clone(),
                        )
                    } else {
                        (String::new(), None, None)
                    }
                } else {
                    (String::new(), None, None)
                };
                nodes.push(LayoutNode::Repeat {
                    body,
                    condition,
                    is_label,
                    not_label,
                });
            }
            ActivityStep::RepeatWhile(_) => {
                i += 1;
            }
            ActivityStep::Fork | ActivityStep::Split => {
                i += 1;
                let mut branches = Vec::new();
                let first_branch = collect_until(steps, &mut i, |s| {
                    matches!(
                        s,
                        ActivityStep::ForkAgain
                            | ActivityStep::SplitAgain
                            | ActivityStep::EndFork
                            | ActivityStep::EndSplit
                    )
                });
                branches.push(first_branch);
                while i < steps.len() {
                    match &steps[i] {
                        ActivityStep::ForkAgain | ActivityStep::SplitAgain => {
                            i += 1;
                            let branch = collect_until(steps, &mut i, |s| {
                                matches!(
                                    s,
                                    ActivityStep::ForkAgain
                                        | ActivityStep::SplitAgain
                                        | ActivityStep::EndFork
                                        | ActivityStep::EndSplit
                                )
                            });
                            branches.push(branch);
                        }
                        ActivityStep::EndFork | ActivityStep::EndSplit => {
                            i += 1;
                            break;
                        }
                        _ => break,
                    }
                }
                nodes.push(LayoutNode::Fork { branches });
            }
            ActivityStep::ForkAgain
            | ActivityStep::SplitAgain
            | ActivityStep::EndFork
            | ActivityStep::EndSplit => {
                i += 1;
            }
            ActivityStep::Arrow(a) => {
                nodes.push(LayoutNode::Arrow {
                    dashed: a.dashed,
                    color: a.color.clone(),
                    label: a.label.clone(),
                });
                i += 1;
            }
            ActivityStep::Note(n) => {
                nodes.push(LayoutNode::Note {
                    text: n.text.clone(),
                    position: n.position.clone(),
                    color: n.color.clone(),
                });
                i += 1;
            }
            ActivityStep::Detach => {
                nodes.push(LayoutNode::Detach);
                i += 1;
            }
            ActivityStep::Kill => {
                nodes.push(LayoutNode::Kill);
                i += 1;
            }
            ActivityStep::Break => {
                nodes.push(LayoutNode::Break);
                i += 1;
            }
            ActivityStep::Backward(_)
            | ActivityStep::Swimlane(_)
            | ActivityStep::Partition(_)
            | ActivityStep::EndPartition
            | ActivityStep::Switch(_)
            | ActivityStep::Case(_)
            | ActivityStep::EndSwitch => {
                // TODO: implement these
                i += 1;
            }
        }
    }
    nodes
}

fn collect_until_else_or_endif(steps: &[ActivityStep], i: &mut usize) -> Vec<LayoutNode> {
    collect_until(steps, i, |s| {
        matches!(
            s,
            ActivityStep::Else(_) | ActivityStep::ElseIf(_) | ActivityStep::EndIf
        )
    })
}

fn collect_until(
    steps: &[ActivityStep],
    i: &mut usize,
    pred: impl Fn(&ActivityStep) -> bool,
) -> Vec<LayoutNode> {
    let start = *i;
    let mut depth = 0;
    while *i < steps.len() {
        if depth == 0 && pred(&steps[*i]) {
            break;
        }
        // Track nesting depth for if/fork/while/repeat
        match &steps[*i] {
            ActivityStep::If(_) => depth += 1,
            ActivityStep::EndIf => depth -= 1,
            ActivityStep::Fork | ActivityStep::Split => depth += 1,
            ActivityStep::EndFork | ActivityStep::EndSplit => depth -= 1,
            ActivityStep::While(_) => depth += 1,
            ActivityStep::EndWhile(_) => depth -= 1,
            ActivityStep::Repeat => depth += 1,
            ActivityStep::RepeatWhile(_) => depth -= 1,
            _ => {}
        }
        *i += 1;
    }
    build_tree(&steps[start..*i])
}

/// Compute the width needed for a sequence of layout nodes.
fn sequence_width(nodes: &[LayoutNode]) -> f64 {
    nodes.iter().map(node_width).fold(0.0f64, f64::max)
}

fn node_width(node: &LayoutNode) -> f64 {
    match node {
        // Bare start/stop circles: PlantUML lays them out at minimum width
        // without padding (margins are added once at the SVG level). The
        // `+ ACTION_MIN_X * 2.0` previously here forced ~52px of empty
        // space whenever the longest action was narrower than the circle.
        LayoutNode::Start => START_R * 2.0,
        LayoutNode::Stop | LayoutNode::End => STOP_OUTER_R * 2.0,
        LayoutNode::Action { text_width, .. } => {
            // Box content width only. The outer ACTION_MIN_X margin is added
            // once at the SVG level (margin_x in render_diagram).
            *text_width + ACTION_H_PADDING * 2.0
        }
        LayoutNode::DeprecatedAction {
            text_width,
            warning_width,
            ..
        } => {
            let action_w = *text_width + ACTION_H_PADDING * 2.0;
            let warn_w = *warning_width + 7.0 * 2.0; // 7px padding in warning box
            action_w.max(warn_w)
        }
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            then_label,
            ..
        } => {
            let cond_w = text_render::measure(condition, SMALL_FONT, false);
            let diamond_w = cond_w + DIAMOND_HALF * 2.0;
            let then_w = sequence_width(then_branch).max(60.0);
            let else_w: f64 = else_branches
                .iter()
                .map(|b| sequence_width(&b.body).max(60.0))
                .sum();
            let label_w = then_label
                .as_ref()
                .map(|l| text_render::measure(l, SMALL_FONT, false))
                .unwrap_or(0.0);
            let else_label_w: f64 = else_branches
                .iter()
                .map(|b| {
                    b.label
                        .as_ref()
                        .map(|l| text_render::measure(l, SMALL_FONT, false))
                        .unwrap_or(0.0)
                })
                .sum();
            (then_w + else_w + label_w + else_label_w + 20.0).max(diamond_w + ACTION_MIN_X * 2.0)
        }
        LayoutNode::Fork { branches } => {
            let total: f64 = branches.iter().map(|b| sequence_width(b).max(60.0)).sum();
            total + FORK_BAR_MARGIN * 2.0
        }
        LayoutNode::While {
            body, condition, ..
        } => {
            let body_w = sequence_width(body);
            let cond_w = text_render::measure(condition, SMALL_FONT, false) + DIAMOND_HALF * 2.0;
            body_w.max(cond_w + 40.0) // extra space for loop-back arrow
        }
        LayoutNode::Repeat {
            body, condition, ..
        } => {
            let body_w = sequence_width(body);
            let cond_w = text_render::measure(condition, SMALL_FONT, false) + DIAMOND_HALF * 2.0;
            body_w.max(cond_w + 40.0)
        }
        // Arrows, notes, detach/kill/break, and bare titles contribute no
        // horizontal extent of their own. (Notes will need width once they're
        // laid out alongside the flow; for now they fall back to 0.)
        LayoutNode::Arrow { .. }
        | LayoutNode::Note { .. }
        | LayoutNode::Detach
        | LayoutNode::Kill
        | LayoutNode::Break => 0.0,
        LayoutNode::Title(t) => text_render::measure(t, TITLE_FONT_SIZE, true),
    }
}

/// Compute the height needed for a sequence of layout nodes.
fn sequence_height(nodes: &[LayoutNode]) -> f64 {
    let mut h = 0.0;
    let mut prior_flow = false;
    for node in nodes {
        // Non-flow nodes (Arrow, Note) contribute nothing themselves.
        if matches!(node, LayoutNode::Arrow { .. } | LayoutNode::Note { .. }) {
            continue;
        }
        // Detach/Kill/Break terminate the flow but produce no visual height.
        // They also suppress the arrow that would precede them.
        if matches!(
            node,
            LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break
        ) {
            prior_flow = false;
            continue;
        }
        if prior_flow {
            h += ARROW_LEN; // arrow between flow nodes
        }
        h += node_height(node);
        prior_flow = true;
    }
    h
}

fn action_height(text: &str) -> f64 {
    // Pick the box height to match the label's actual font — monospace
    // labels render shorter than sans-serif at the same nominal size.
    text_render::label_height(text, FONT_SIZE) + ACTION_PADDING
}

fn node_height(node: &LayoutNode) -> f64 {
    match node {
        // Start ellipse cy is fixed at START_CY (25), so from the y=MARGIN_LEAD
        // cursor (16) the ellipse bottom is 25+10-16 = 19, not the full diameter.
        LayoutNode::Start => START_CY + START_R - 16.0,
        LayoutNode::Stop | LayoutNode::End => STOP_OUTER_R * 2.0,
        LayoutNode::Action { text, .. } => action_height(text),
        LayoutNode::DeprecatedAction { text, .. } => {
            let warn_h = pm::text_height(10.0) + 4.5313; // warning box height from golden
            action_height(text) + warn_h + ARROW_LEN
        }
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } => {
            let diamond_h = DIAMOND_HALF * 2.0;
            let then_h = sequence_height(then_branch);
            let max_else_h: f64 = else_branches
                .iter()
                .map(|b| sequence_height(&b.body))
                .fold(0.0f64, f64::max);
            let branch_h = then_h.max(max_else_h);
            diamond_h + ARROW_LEN + branch_h + ARROW_LEN + DIAMOND_HALF * 2.0
        }
        LayoutNode::Fork { branches } => {
            let max_h: f64 = branches
                .iter()
                .map(|b| sequence_height(b))
                .fold(0.0f64, f64::max);
            FORK_BAR_HEIGHT + ARROW_LEN + max_h + ARROW_LEN + FORK_BAR_HEIGHT
        }
        LayoutNode::While { body, .. } => {
            let body_h = sequence_height(body);
            let diamond_h = DIAMOND_HALF * 2.0;
            diamond_h + ARROW_LEN + body_h + ARROW_LEN
        }
        LayoutNode::Repeat { body, .. } => {
            let body_h = sequence_height(body);
            let diamond_h = DIAMOND_HALF * 2.0;
            diamond_h + ARROW_LEN + body_h + ARROW_LEN + diamond_h
        }
        LayoutNode::Arrow { .. } => 0.0, // arrows don't add height (they're between nodes)
        LayoutNode::Note { .. } => 0.0,
        LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break => 0.0,
        LayoutNode::Title(_) => pm::text_height(TITLE_FONT_SIZE) + 10.0,
    }
}

// ─── SVG emission ───────────────────────────────────────────────────

/// Escape a string for SVG text content the way PlantUML does:
/// XML-escape `<`, `>`, `&`, and emit U+00A0 (non-breaking space) as the
/// numeric entity `&#xA0;`.
fn svg_text_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\u{a0}' => out.push_str("&#xA0;"),
            _ => out.push(ch),
        }
    }
    out
}

fn f(v: f64) -> String {
    pm::fmt_coord(v)
}

fn polygon_points(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{},{}", f(*x), f(*y)))
        .collect::<Vec<_>>()
        .join(",")
}

struct SvgEmitter {
    /// Shapes and labels (rects, ellipses, polygon-shapes, text). PlantUML
    /// emits all of these first in document order.
    shapes: String,
    /// Connectors (lines, arrowhead polygons). PlantUML emits all of these
    /// after the shapes, also in document order.
    connectors: String,
}

#[allow(clippy::too_many_arguments)]
impl SvgEmitter {
    fn new() -> Self {
        SvgEmitter {
            shapes: String::new(),
            connectors: String::new(),
        }
    }

    /// Final concatenation: shapes first, then all connectors.
    fn finish(self) -> String {
        let mut out = self.shapes;
        out.push_str(&self.connectors);
        out
    }

    fn ellipse(
        &mut self,
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        fill: &str,
        stroke: &str,
        stroke_width: &str,
    ) {
        write!(
            self.shapes,
            r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(cx), f(cy), fill, f(rx), f(ry), stroke, stroke_width
        )
        .unwrap();
    }

    fn rect_styled(
        &mut self,
        fill: &str,
        height: f64,
        rx: f64,
        ry: f64,
        stroke: &str,
        stroke_width: &str,
        width: f64,
        x: f64,
        y: f64,
    ) {
        write!(
            self.shapes,
            r#"<rect fill="{}" height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"#,
            fill, f(height), f(rx), f(ry), stroke, stroke_width, f(width), f(x), f(y)
        )
        .unwrap();
    }

    fn text_element(
        &mut self,
        fill: &str,
        font_family: &str,
        font_size: f64,
        _text_length: f64,
        x: f64,
        y: f64,
        content: &str,
        bold: bool,
    ) {
        // text_length is ignored: emit_text computes widths from segments
        // (after creole stripping). Upstream geometry that sized boxes
        // around this text should already have measured the stripped text.
        let base = TextBase {
            x,
            y,
            font_size: font_size as u32,
            font_family,
            fill,
            bold,
            italic: false,
            underline: false,
            skip_underline: false,
        };
        text_render::emit_text(&mut self.shapes, content, &base);
    }

    fn monospace_text_element(
        &mut self,
        fill: &str,
        font_size: f64,
        text_length: f64,
        x: f64,
        y: f64,
        content: &str,
    ) {
        // PlantUML emits `<`/`>` as `&lt;`/`&gt;` and U+00A0 as the numeric
        // entity `&#xA0;` (rather than the raw UTF-8 byte sequence).
        let escaped = svg_text_escape(content);
        write!(
            self.shapes,
            r#"<text fill="{}" font-family="monospace" font-size="{}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            fill, font_size as u32, f(text_length), f(x), f(y), escaped
        )
        .unwrap();
    }

    /// A node-style polygon (diamond, fork-bar variant, etc.) — goes with
    /// shapes in the document order.
    fn polygon_shape(
        &mut self,
        fill: &str,
        points: &[(f64, f64)],
        stroke: &str,
        stroke_width: &str,
    ) {
        let pts = polygon_points(points);
        write!(
            self.shapes,
            r#"<polygon fill="{}" points="{}" style="stroke:{};stroke-width:{};"/>"#,
            fill, pts, stroke, stroke_width
        )
        .unwrap();
    }

    fn line_styled(
        &mut self,
        stroke: &str,
        stroke_width: &str,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        dashed: bool,
    ) {
        let dash = if dashed { "stroke-dasharray:2,2;" } else { "" };
        write!(
            self.connectors,
            r#"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke,
            stroke_width,
            dash,
            f(x1),
            f(x2),
            f(y1),
            f(y2)
        )
        .unwrap();
    }

    /// Arrowhead-style polygon for connectors — goes after all shapes.
    fn polygon_connector(
        &mut self,
        fill: &str,
        points: &[(f64, f64)],
        stroke: &str,
        stroke_width: &str,
    ) {
        let pts = polygon_points(points);
        write!(
            self.connectors,
            r#"<polygon fill="{}" points="{}" style="stroke:{};stroke-width:{};"/>"#,
            fill, pts, stroke, stroke_width
        )
        .unwrap();
    }

    /// Emit a downward arrow (vertical line + arrowhead polygon).
    fn down_arrow(&mut self, cx: f64, y1: f64, y2: f64, color: &str) {
        self.line_styled(color, "1", cx, cx, y1, y2, false);
        // Arrowhead: 4px each side, 10px tall, 4px notch
        self.polygon_connector(
            color,
            &[
                (cx - 4.0, y2 - 10.0),
                (cx, y2),
                (cx + 4.0, y2 - 10.0),
                (cx, y2 - 6.0),
            ],
            color,
            "1",
        );
    }

    /// Emit a styled downward arrow (handles colour, dashed, bold).
    fn down_arrow_full(&mut self, cx: f64, y1: f64, y2: f64, style: &ArrowStyle) {
        let sw = if style.bold { "2" } else { "1" };
        self.line_styled(&style.color, sw, cx, cx, y1, y2, style.dashed);
        // Bold arrows in PlantUML keep the same arrowhead size; only the
        // line stroke changes. The polygon stays 1px.
        self.polygon_connector(
            &style.color,
            &[
                (cx - 4.0, y2 - 10.0),
                (cx, y2),
                (cx + 4.0, y2 - 10.0),
                (cx, y2 - 6.0),
            ],
            &style.color,
            "1",
        );
    }

    /// Emit an upward arrow (arrowhead pointing up).
    fn up_arrow(&mut self, cx: f64, y1: f64, y2: f64, color: &str) {
        self.polygon_connector(
            color,
            &[
                (cx - 4.0, y1 + 10.0),
                (cx, y1),
                (cx + 4.0, y1 + 10.0),
                (cx, y1 + 6.0),
            ],
            color,
            "1",
        );
        // Line from y2 down to y1 (y2 > y1 in this context)
        self.line_styled(color, "1", cx, cx, y1, y2, false);
    }

    /// Emit a right-pointing arrow on a horizontal line.
    fn right_arrow(&mut self, x_tip: f64, y: f64, color: &str) {
        self.polygon_connector(
            color,
            &[
                (x_tip - 10.0, y - 4.0),
                (x_tip, y),
                (x_tip - 10.0, y + 4.0),
                (x_tip - 6.0, y),
            ],
            color,
            "1",
        );
    }

    /// Emit a left-pointing arrow on a horizontal line.
    fn left_arrow(&mut self, x_tip: f64, y: f64, color: &str) {
        self.polygon_connector(
            color,
            &[
                (x_tip + 10.0, y - 4.0),
                (x_tip, y),
                (x_tip + 10.0, y + 4.0),
                (x_tip + 6.0, y),
            ],
            color,
            "1",
        );
    }
}

/// Render a linear sequence of nodes at a given center-x and starting y.
/// Returns the y position after the last node.
fn emit_sequence(svg: &mut SvgEmitter, nodes: &[LayoutNode], cx: f64, mut y: f64) -> f64 {
    for (i, node) in nodes.iter().enumerate() {
        // Skip layout for non-flow nodes (arrows and notes don't take vertical space
        // on their own).
        if matches!(node, LayoutNode::Arrow { .. } | LayoutNode::Note { .. }) {
            continue;
        }
        // Detach/Kill/Break also produce no shape and no incoming connector —
        // they mark the previous flow as terminated.
        if matches!(
            node,
            LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break
        ) {
            continue;
        }
        if i > 0 {
            // Find the most recent flow-producing previous node and the
            // optional explicit Arrow between it and us.
            let mut explicit_arrow: Option<&LayoutNode> = None;
            let mut prev_idx: Option<usize> = None;
            for j in (0..i).rev() {
                match &nodes[j] {
                    LayoutNode::Arrow { .. } => {
                        if explicit_arrow.is_none() {
                            explicit_arrow = Some(&nodes[j]);
                        }
                    }
                    LayoutNode::Note { .. } => {}
                    LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break => {
                        // Previous flow terminated; no connector to draw.
                        prev_idx = None;
                        break;
                    }
                    _ => {
                        prev_idx = Some(j);
                        break;
                    }
                }
            }
            if prev_idx.is_some() {
                let style = match explicit_arrow {
                    Some(LayoutNode::Arrow {
                        color: Some(c),
                        dashed,
                        ..
                    }) => arrow_style_from_brackets(c, *dashed),
                    Some(LayoutNode::Arrow { dashed, .. }) => ArrowStyle {
                        color: ARROW_COLOR.to_string(),
                        dashed: *dashed,
                        bold: false,
                        hidden: false,
                    },
                    _ => ArrowStyle::default(),
                };
                if !style.hidden {
                    svg.down_arrow_full(cx, y, y + ARROW_LEN, &style);
                }
                y += ARROW_LEN;
            }
        }
        y = emit_node(svg, node, cx, y);
    }
    y
}

/// Emit a single node at the given center-x and y position.
/// Returns the y position after this node (bottom edge).
fn emit_node(svg: &mut SvgEmitter, node: &LayoutNode, cx: f64, y: f64) -> f64 {
    match node {
        LayoutNode::Start => {
            // PlantUML places the start ellipse at cy=START_CY=25 regardless
            // of the surrounding layout. With margin_top=16 (matching the
            // bare-action case), the circle sits 1px above the cursor.
            svg.ellipse(
                cx,
                START_CY,
                START_R,
                START_R,
                START_FILL,
                START_STROKE,
                "1",
            );
            START_CY + START_R
        }
        LayoutNode::Stop => {
            let cy = y + STOP_OUTER_R;
            svg.ellipse(cx, cy, STOP_OUTER_R, STOP_OUTER_R, "none", STOP_STROKE, "1");
            svg.ellipse(
                cx,
                cy,
                STOP_INNER_R,
                STOP_INNER_R,
                STOP_FILL,
                STOP_FILL,
                "1",
            );
            y + STOP_OUTER_R * 2.0
        }
        LayoutNode::End => {
            // End node is same visual as stop in PlantUML
            let cy = y + STOP_OUTER_R;
            svg.ellipse(cx, cy, STOP_OUTER_R, STOP_OUTER_R, "none", STOP_STROKE, "1");
            svg.ellipse(
                cx,
                cy,
                STOP_INNER_R,
                STOP_INNER_R,
                STOP_FILL,
                STOP_FILL,
                "1",
            );
            y + STOP_OUTER_R * 2.0
        }
        LayoutNode::Action { text, text_width } => {
            let ah = action_height(text);
            let rect_w = *text_width + ACTION_H_PADDING * 2.0;
            let rect_x = cx - rect_w / 2.0;
            svg.rect_styled(
                ACTION_FILL,
                ah,
                ACTION_RX,
                ACTION_RX,
                ACTION_STROKE,
                ACTION_STROKE_WIDTH,
                rect_w,
                rect_x,
                y,
            );
            // Text baseline: padding_top + ascent, both derived from the
            // label's actual font so monospace labels position correctly.
            let lh = text_render::label_height(text, FONT_SIZE);
            let padding_top = (ah - lh) / 2.0;
            let text_y = y + padding_top + text_render::label_ascent(text, FONT_SIZE);
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                FONT_SIZE,
                *text_width,
                rect_x + ACTION_H_PADDING,
                text_y,
                text,
                false,
            );
            y + ah
        }
        LayoutNode::DeprecatedAction {
            color: _,
            text,
            text_width,
            warning_width: _,
        } => {
            // The deprecated action renders just like a normal action.
            // The warning banner is emitted separately at the top of the diagram.
            let ah = action_height(text);
            let rect_w = *text_width + ACTION_H_PADDING * 2.0;
            let rect_x = cx - rect_w / 2.0;
            svg.rect_styled(
                ACTION_FILL,
                ah,
                ACTION_RX,
                ACTION_RX,
                ACTION_STROKE,
                ACTION_STROKE_WIDTH,
                rect_w,
                rect_x,
                y,
            );
            let lh = text_render::label_height(text, FONT_SIZE);
            let padding_top = (ah - lh) / 2.0;
            let text_y = y + padding_top + text_render::label_ascent(text, FONT_SIZE);
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                FONT_SIZE,
                *text_width,
                rect_x + ACTION_H_PADDING,
                text_y,
                text,
                false,
            );
            y + ah
        }
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
        } => emit_if(
            svg,
            cx,
            y,
            condition,
            then_label,
            then_branch,
            else_branches,
        ),
        LayoutNode::Fork { branches } => emit_fork(svg, cx, y, branches),
        LayoutNode::While {
            condition,
            is_label,
            body,
            end_label: _,
        } => emit_while(svg, cx, y, condition, is_label, body),
        LayoutNode::Repeat {
            body,
            condition,
            is_label,
            not_label: _,
        } => emit_repeat(svg, cx, y, body, condition, is_label),
        LayoutNode::Arrow { .. } | LayoutNode::Note { .. } => y,
        LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break => y,
        LayoutNode::Title(text) => {
            let tw = text_render::measure(text, TITLE_FONT_SIZE, true);
            let text_y = y + pm::ascent(TITLE_FONT_SIZE) + 5.0;
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                TITLE_FONT_SIZE,
                tw,
                cx - tw / 2.0,
                text_y,
                text,
                true,
            );
            y + pm::text_height(TITLE_FONT_SIZE) + 10.0
        }
    }
}

fn emit_if(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    then_label: &Option<String>,
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> f64 {
    let cond_w = text_render::measure(condition, SMALL_FONT, false);

    // Diamond: centered at (cx, y + DIAMOND_HALF)
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = cx - cond_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_w / 2.0 + DIAMOND_HALF;

    // Diamond polygon (hexagonal for conditions with text)
    let pts = vec![
        (cx - cond_w / 2.0, y),
        (cx + cond_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_w / 2.0, y + DIAMOND_HALF * 2.0),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(DIAMOND_FILL, &pts, ACTION_STROKE, ACTION_STROKE_WIDTH);

    // Condition text
    let text_y = y + DIAMOND_HALF + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        TEXT_COLOR,
        "sans-serif",
        SMALL_FONT,
        cond_w,
        cx - cond_w / 2.0,
        text_y,
        condition,
        false,
    );

    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // Then label (to the left of diamond)
    if let Some(label) = then_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_left - lw - 5.0,
            diamond_cy + pm::text_height(SMALL_FONT) / 2.0
                - pm::descent(SMALL_FONT)
                - DIAMOND_HALF / 2.0,
            label,
            false,
        );
    }

    // Compute branch widths
    let then_w = sequence_width(then_branch).max(60.0);
    let _else_count = else_branches.len().max(1);

    // Left branch (then): centered to the left
    let then_cx = cx - then_w / 2.0;

    // Then arrow: horizontal from diamond left to then_cx, then down
    svg.line_styled(
        ARROW_COLOR,
        "1",
        diamond_left,
        then_cx,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.line_styled(
        ARROW_COLOR,
        "1",
        then_cx,
        then_cx,
        diamond_cy,
        diamond_bottom + ARROW_LEN,
        false,
    );
    svg.polygon_connector(
        ARROW_COLOR,
        &[
            (then_cx - 4.0, diamond_bottom + ARROW_LEN - 10.0),
            (then_cx, diamond_bottom + ARROW_LEN),
            (then_cx + 4.0, diamond_bottom + ARROW_LEN - 10.0),
            (then_cx, diamond_bottom + ARROW_LEN - 6.0),
        ],
        ARROW_COLOR,
        "1",
    );

    // Else label and branch
    let else_cx = if else_branches.is_empty() {
        cx + then_w / 2.0
    } else {
        let else_w = sequence_width(&else_branches[0].body).max(60.0);
        cx + else_w / 2.0
    };

    if let Some(label) = else_branches.first().and_then(|b| b.label.as_ref()) {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_right + 5.0,
            diamond_cy + pm::text_height(SMALL_FONT) / 2.0
                - pm::descent(SMALL_FONT)
                - DIAMOND_HALF / 2.0,
            label,
            false,
        );
    }

    // Else arrow: horizontal from diamond right to else_cx, then down
    svg.line_styled(
        ARROW_COLOR,
        "1",
        diamond_right,
        else_cx,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.line_styled(
        ARROW_COLOR,
        "1",
        else_cx,
        else_cx,
        diamond_cy,
        diamond_bottom + ARROW_LEN,
        false,
    );
    svg.polygon_connector(
        ARROW_COLOR,
        &[
            (else_cx - 4.0, diamond_bottom + ARROW_LEN - 10.0),
            (else_cx, diamond_bottom + ARROW_LEN),
            (else_cx + 4.0, diamond_bottom + ARROW_LEN - 10.0),
            (else_cx, diamond_bottom + ARROW_LEN - 6.0),
        ],
        ARROW_COLOR,
        "1",
    );

    // Render branches
    let branch_y = diamond_bottom + ARROW_LEN;
    let then_bottom = emit_sequence(svg, then_branch, then_cx, branch_y);
    let else_bottom = if !else_branches.is_empty() {
        emit_sequence(svg, &else_branches[0].body, else_cx, branch_y)
    } else {
        branch_y
    };

    // Merge diamond at bottom
    let merge_y = then_bottom.max(else_bottom) + ARROW_LEN;
    let merge_diamond_top = merge_y;
    let merge_cy = merge_diamond_top + DIAMOND_HALF;

    // Small merge diamond
    svg.polygon_shape(
        DIAMOND_FILL,
        &[
            (cx, merge_diamond_top),
            (cx + DIAMOND_HALF, merge_cy),
            (cx, merge_diamond_top + DIAMOND_HALF * 2.0),
            (cx - DIAMOND_HALF, merge_cy),
        ],
        ACTION_STROKE,
        ACTION_STROKE_WIDTH,
    );

    // Arrows from branches to merge
    // Then branch to merge
    svg.line_styled(
        ARROW_COLOR,
        "1",
        then_cx,
        then_cx,
        then_bottom,
        merge_cy,
        false,
    );
    svg.line_styled(
        ARROW_COLOR,
        "1",
        then_cx,
        cx - DIAMOND_HALF,
        merge_cy,
        merge_cy,
        false,
    );
    svg.right_arrow(cx - DIAMOND_HALF, merge_cy, ARROW_COLOR);

    // Else branch to merge
    svg.line_styled(
        ARROW_COLOR,
        "1",
        else_cx,
        else_cx,
        else_bottom,
        merge_cy,
        false,
    );
    svg.line_styled(
        ARROW_COLOR,
        "1",
        else_cx,
        cx + DIAMOND_HALF,
        merge_cy,
        merge_cy,
        false,
    );
    svg.left_arrow(cx + DIAMOND_HALF, merge_cy, ARROW_COLOR);

    merge_diamond_top + DIAMOND_HALF * 2.0
}

fn emit_fork(svg: &mut SvgEmitter, cx: f64, y: f64, branches: &[Vec<LayoutNode>]) -> f64 {
    if branches.is_empty() {
        return y;
    }

    // Compute branch widths and total width
    let branch_widths: Vec<f64> = branches
        .iter()
        .map(|b| sequence_width(b).max(60.0))
        .collect();
    let total_w: f64 = branch_widths.iter().sum();
    let bar_w = total_w + FORK_BAR_MARGIN * 2.0;

    // Top bar
    let bar_x = cx - bar_w / 2.0;
    svg.rect_styled(
        FORK_BAR_COLOR,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        FORK_BAR_COLOR,
        "1",
        bar_w,
        bar_x,
        y,
    );

    let bar_bottom = y + FORK_BAR_HEIGHT;

    // Compute branch center-x positions
    let mut branch_centers = Vec::new();
    let mut bx = bar_x + FORK_BAR_MARGIN;
    for w in &branch_widths {
        branch_centers.push(bx + w / 2.0);
        bx += w;
    }

    // Arrows from top bar to each branch and render branches
    let mut branch_bottoms = Vec::new();
    for (i, branch) in branches.iter().enumerate() {
        let bcx = branch_centers[i];
        svg.down_arrow(bcx, bar_bottom, bar_bottom + ARROW_LEN, ARROW_COLOR);
        let bottom = emit_sequence(svg, branch, bcx, bar_bottom + ARROW_LEN);
        branch_bottoms.push(bottom);
    }

    // Find the maximum bottom
    let max_bottom = branch_bottoms.iter().cloned().fold(0.0f64, f64::max);

    // Arrows from each branch to bottom bar
    for (i, bottom) in branch_bottoms.iter().enumerate() {
        let bcx = branch_centers[i];
        svg.down_arrow(bcx, *bottom, max_bottom + ARROW_LEN, ARROW_COLOR);
    }

    // Bottom bar
    let bottom_bar_y = max_bottom + ARROW_LEN;
    svg.rect_styled(
        FORK_BAR_COLOR,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        FORK_BAR_COLOR,
        "1",
        bar_w,
        bar_x,
        bottom_bar_y,
    );

    bottom_bar_y + FORK_BAR_HEIGHT
}

fn emit_while(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    is_label: &Option<String>,
    body: &[LayoutNode],
) -> f64 {
    // TODO: proper while layout matching PlantUML
    // For now, simplified linear layout
    let cond_w = text_render::measure(condition, SMALL_FONT, false);
    let diamond_cy = y + DIAMOND_HALF;

    // Diamond
    let pts = vec![
        (cx - cond_w / 2.0, y),
        (cx + cond_w / 2.0, y),
        (cx + cond_w / 2.0 + DIAMOND_HALF, diamond_cy),
        (cx + cond_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_w / 2.0 - DIAMOND_HALF, diamond_cy),
    ];
    svg.polygon_shape(DIAMOND_FILL, &pts, ACTION_STROKE, ACTION_STROKE_WIDTH);

    let text_y = y + DIAMOND_HALF + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        TEXT_COLOR,
        "sans-serif",
        SMALL_FONT,
        cond_w,
        cx - cond_w / 2.0,
        text_y,
        condition,
        false,
    );

    if let Some(label) = is_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            cx + cond_w / 2.0 + DIAMOND_HALF + 5.0,
            diamond_cy + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT),
            label,
            false,
        );
    }

    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // Body below diamond
    svg.down_arrow(cx, diamond_bottom, diamond_bottom + ARROW_LEN, ARROW_COLOR);
    emit_sequence(svg, body, cx, diamond_bottom + ARROW_LEN)
}

fn emit_repeat(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    body: &[LayoutNode],
    condition: &str,
    is_label: &Option<String>,
) -> f64 {
    // Top diamond (entry point)
    let top_diamond_size = DIAMOND_HALF;
    svg.polygon_shape(
        DIAMOND_FILL,
        &[
            (cx, y),
            (cx + top_diamond_size, y + top_diamond_size),
            (cx, y + top_diamond_size * 2.0),
            (cx - top_diamond_size, y + top_diamond_size),
        ],
        ACTION_STROKE,
        ACTION_STROKE_WIDTH,
    );

    let top_bottom = y + top_diamond_size * 2.0;

    // Arrow from top diamond to body
    svg.down_arrow(cx, top_bottom, top_bottom + ARROW_LEN, ARROW_COLOR);
    let body_bottom = emit_sequence(svg, body, cx, top_bottom + ARROW_LEN);

    // Arrow from body to condition diamond
    svg.down_arrow(cx, body_bottom, body_bottom + ARROW_LEN, ARROW_COLOR);
    let cond_y = body_bottom + ARROW_LEN;

    // Condition diamond
    let cond_w = text_render::measure(condition, SMALL_FONT, false);
    let cond_diamond_cy = cond_y + DIAMOND_HALF;
    let pts = vec![
        (cx - cond_w / 2.0, cond_y),
        (cx + cond_w / 2.0, cond_y),
        (cx + cond_w / 2.0 + DIAMOND_HALF, cond_diamond_cy),
        (cx + cond_w / 2.0, cond_y + DIAMOND_HALF * 2.0),
        (cx - cond_w / 2.0, cond_y + DIAMOND_HALF * 2.0),
        (cx - cond_w / 2.0 - DIAMOND_HALF, cond_diamond_cy),
    ];
    svg.polygon_shape(DIAMOND_FILL, &pts, ACTION_STROKE, ACTION_STROKE_WIDTH);

    let text_y = cond_diamond_cy + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        TEXT_COLOR,
        "sans-serif",
        SMALL_FONT,
        cond_w,
        cx - cond_w / 2.0,
        text_y,
        condition,
        false,
    );

    // "is" label
    if let Some(label) = is_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        let diamond_right = cx + cond_w / 2.0 + DIAMOND_HALF;
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_right + 5.0,
            cond_diamond_cy + pm::text_height(SMALL_FONT) / 2.0
                - pm::descent(SMALL_FONT)
                - DIAMOND_HALF / 2.0,
            label,
            false,
        );

        // Loop-back arrow (right side, up to top diamond)
        let loop_x = diamond_right + 5.0 + lw + 5.0;
        svg.line_styled(
            ARROW_COLOR,
            "1",
            diamond_right,
            loop_x,
            cond_diamond_cy,
            cond_diamond_cy,
            false,
        );
        // Vertical line up
        let top_cy = y + top_diamond_size;
        svg.up_arrow(loop_x, top_cy, cond_diamond_cy, ARROW_COLOR);
        // Horizontal to top diamond
        svg.line_styled(
            ARROW_COLOR,
            "1",
            loop_x,
            cx + top_diamond_size,
            top_cy,
            top_cy,
            false,
        );
        svg.left_arrow(cx + top_diamond_size, top_cy, ARROW_COLOR);
    }

    cond_y + DIAMOND_HALF * 2.0
}

/// Render an activity diagram to SVG.
pub fn render(diagram: &ActivityDiagram, _theme: &Theme) -> String {
    if diagram.steps.is_empty() {
        return empty_svg();
    }

    // Build layout tree from flat steps.
    let mut tree = build_tree(&diagram.steps);

    // Prepend title if present.
    if let Some(ref title) = diagram.meta.title {
        tree.insert(0, LayoutNode::Title(title.clone()));
    }

    // Collect deprecated color action warnings.
    let deprecated_warnings: Vec<(String, f64)> = diagram
        .steps
        .iter()
        .filter_map(|s| {
            if let ActivityStep::DeprecatedColorAction(dca) = s {
                let warning = deprecated_warning(&dca.color);
                let ww = pm::mono_text_width(&warning, 10.0);
                Some((warning, ww))
            } else {
                None
            }
        })
        .collect();
    let has_deprecated = !deprecated_warnings.is_empty();

    // Compute overall dimensions.
    let content_w = sequence_width(&tree);
    let content_h = sequence_height(&tree);

    // Total SVG dimensions: PlantUML uses asymmetric margins on both axes —
    // 16px left/top (the ACTION_MIN_X start position) and 19px right/bottom.
    // Verified against single-action goldens of varying widths.
    const MARGIN_LEAD: f64 = 16.0; // left and top
    const MARGIN_TRAIL: f64 = 19.0; // right and bottom
    let margin_top = MARGIN_LEAD;

    // For deprecated actions, the warning banner needs special handling
    let extra_h = if has_deprecated {
        // Approximate extra height for deprecated warnings
        diagram
            .steps
            .iter()
            .filter(|s| matches!(s, ActivityStep::DeprecatedColorAction(_)))
            .count() as f64
            * (pm::text_height(10.0) + 4.5313 + START_R * 2.0 + ARROW_LEN)
    } else {
        0.0
    };

    let svg_w = (content_w + MARGIN_LEAD + MARGIN_TRAIL).ceil() as u32;
    let svg_h = (MARGIN_LEAD + content_h + extra_h + MARGIN_TRAIL).ceil() as u32;
    // cx is the centre of the content area (left-aligned at MARGIN_LEAD,
    // width = content_w), NOT the geometric centre of the SVG. PlantUML's
    // asymmetric margins make these differ.
    let cx = MARGIN_LEAD + content_w / 2.0;

    let mut svg = SvgEmitter::new();

    // Emit deprecated warning banners at the top.
    let mut start_y = 13.0; // PlantUML warning banner starts at y=13
    if has_deprecated {
        for (warning, ww) in &deprecated_warnings {
            let warn_h = pm::text_height(10.0) + 4.53125; // from golden SVGs: 16.6406
            let warn_w = *ww + 7.0 * 2.0;
            let warn_x = cx - warn_w / 2.0;
            svg.rect_styled(
                DEPRECATED_FILL,
                warn_h,
                2.5,
                2.5,
                DEPRECATED_STROKE,
                "3",
                warn_w,
                warn_x,
                start_y,
            );
            let warn_text_y = start_y + pm::ascent(10.0) + (warn_h - pm::text_height(10.0)) / 2.0;
            svg.monospace_text_element(TEXT_COLOR, 10.0, *ww, warn_x + 7.0, warn_text_y, warning);
            start_y += warn_h;
        }
        start_y += margin_top; // margin between warning and content
    } else {
        start_y = margin_top;
    }

    // Emit all nodes.
    emit_sequence(&mut svg, &tree, cx, start_y);

    // Wrap in PlantUML-compatible SVG root.
    format_svg(svg_w, svg_h, &svg.finish())
}

fn empty_svg() -> String {
    format_svg(100, 50, "")
}

fn format_svg(width: u32, height: u32, content: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="ACTIVITY" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify"><defs/><g>{content}</g></svg>"#,
        w = width,
        h = height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    #[test]
    fn simple_activity() {
        let d = ActivityDiagram {
            meta: DiagramMeta::default(),
            steps: vec![
                ActivityStep::Start,
                ActivityStep::Action("Step 1".into()),
                ActivityStep::Action("Step 2".into()),
                ActivityStep::Stop,
            ],
        };
        let svg = render(&d, &crate::style::Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Step 1"));
        assert!(svg.contains("Step 2"));
        assert!(svg.contains("data-diagram-type=\"ACTIVITY\""));
        assert!(svg.contains("<ellipse"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn with_condition() {
        use rustuml_parser::diagram::activity::IfBlock;
        let d = ActivityDiagram {
            meta: DiagramMeta::default(),
            steps: vec![
                ActivityStep::Start,
                ActivityStep::If(IfBlock {
                    condition: "x > 0?".into(),
                    then_label: Some("yes".into()),
                    source_line: 0,
                }),
                ActivityStep::Action("positive".into()),
                ActivityStep::Else(Some("no".into())),
                ActivityStep::Action("negative".into()),
                ActivityStep::EndIf,
                ActivityStep::Stop,
            ],
        };
        let svg = render(&d, &crate::style::Theme::default());
        assert!(svg.contains("x &gt; 0?"));
        assert!(svg.contains("positive"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nstart\n:Hello;\nstop\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Hello"));
        assert!(svg.contains("data-diagram-type=\"ACTIVITY\""));
        assert!(svg.contains("textLength="));
    }

    #[test]
    fn basic_start_stop_structure() {
        let input = "@startuml\n\nstart\n:Do something;\nstop\n@enduml\n";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Check PlantUML structural elements
        assert!(svg.contains("<ellipse"), "should use ellipse");
        assert!(svg.contains("fill=\"#222222\""), "start circle fill");
        assert!(svg.contains("fill=\"#F1F1F1\""), "action fill");
        assert!(svg.contains("textLength=\""), "should have textLength");
        assert!(
            svg.contains("lengthAdjust=\"spacing\""),
            "should have lengthAdjust"
        );
        assert!(svg.contains("<defs/>"), "should have empty defs");
        assert!(svg.contains("Do something"), "should have text content");

        // Check text width matches PlantUML
        let expected_tw = "81.8672"; // PlantUML's textLength for "Do something" at 12
        assert!(
            svg.contains(&format!("textLength=\"{expected_tw}\"")),
            "textLength should be {expected_tw}, got: {}",
            &svg[svg.find("textLength=").unwrap_or(0)
                ..svg.find("textLength=").unwrap_or(0) + 40.min(svg.len())]
        );
    }
}
