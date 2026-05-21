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
/// Vertical extent of a connector that carries a label (sans-serif 11).
/// Reverse-engineered from PlantUML goldens: 20 (normal) + 21.275 extra to
/// fit the label beside the line.
const LABELED_ARROW_LEN: f64 = 41.2754;
const ACTION_PADDING: f64 = 20.0; // total vertical padding in action box
const ACTION_H_PADDING: f64 = 10.0; // horizontal padding each side
const ACTION_RX: f64 = 12.5;
const DIAMOND_HALF: f64 = 12.0; // half-size of decision diamond
/// PlantUML enforces a minimum width on the inner (top/bottom) edge of
/// decision diamonds: 24 px regardless of how short the condition text is.
/// Reverse-engineered from goldens with one- and two-character conditions
/// ("A?", "B?", "c?") which all produce a 24-px inner span while their text
/// `textLength` stays at the measured value.
const DIAMOND_MIN_INNER_W: f64 = 24.0;

/// Vertical gap between an if/else condition diamond's bottom point and
/// the top of each branch's first action box. PlantUML uses 10 px here,
/// not the generic 20 px `ARROW_LEN` used for sequential arrows.
const IF_BRANCH_DOWN: f64 = 10.0;
/// Vertical gap between the last action of an if/else branch and the
/// top of the merge diamond below. PlantUML uses 6 px here.
const IF_BRANCH_UP: f64 = 6.0;
const FORK_BAR_HEIGHT: f64 = 6.0;
const FORK_BAR_RX: f64 = 2.5;
const FORK_BAR_MARGIN: f64 = 14.0; // margin on each side of fork bar

const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 11.0;
const TITLE_FONT_SIZE: f64 = 14.0;

const START_FILL: &str = "#222222";
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

/// Per-diagram color palette, derived from the PlantUML default plus any
/// inline `skinparam` overrides. Mirrors the constants above but allows
/// skinparams to mutate individual fields without rebuilding the theme
/// machinery in `style.rs` (which uses the `slate` defaults).
#[derive(Debug, Clone)]
struct Palette {
    action_fill: String,
    action_stroke: String,
    action_stroke_width: String,
    diamond_fill: String,
    diamond_stroke: String,
    arrow_color: String,
    text_color: String,
    start_fill: String,
    stop_fill: String,
    bar_color: String,
}

impl Palette {
    fn default_puml() -> Self {
        Self {
            action_fill: ACTION_FILL.into(),
            action_stroke: ACTION_STROKE.into(),
            action_stroke_width: ACTION_STROKE_WIDTH.into(),
            diamond_fill: DIAMOND_FILL.into(),
            diamond_stroke: ACTION_STROKE.into(),
            arrow_color: ARROW_COLOR.into(),
            text_color: TEXT_COLOR.into(),
            start_fill: START_FILL.into(),
            stop_fill: STOP_FILL.into(),
            bar_color: FORK_BAR_COLOR.into(),
        }
    }

    /// Apply the supplied skinparams (key-insensitive) onto the default
    /// PlantUML palette. Unrecognised or empty values are ignored.
    fn from_skinparams(skinparams: &[rustuml_parser::diagram::SkinParam]) -> Self {
        let mut p = Self::default_puml();
        for sp in skinparams {
            let key = sp.key.to_ascii_lowercase();
            let val = sp.value.trim();
            if val.is_empty() {
                continue;
            }
            let resolved = crate::sequence::resolve_color(val);
            match key.as_str() {
                "activitybackgroundcolor" => p.action_fill = resolved,
                "activitybordercolor" => p.action_stroke = resolved,
                "activityborderthickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        // Format like PlantUML: integer when whole, otherwise raw float.
                        p.action_stroke_width = pm::fmt_coord(v);
                    }
                }
                "activitydiamondbackgroundcolor" => p.diamond_fill = resolved,
                "activitydiamondbordercolor" => p.diamond_stroke = resolved,
                "activityarrowcolor" | "arrowcolor" => p.arrow_color = resolved,
                "activitystartcolor" => p.start_fill = resolved,
                "activityendcolor" | "activitystopcolor" => p.stop_fill = resolved,
                "activitybarcolor" => p.bar_color = resolved,
                "activityfontcolor" => p.text_color = resolved,
                _ => {}
            }
        }
        p
    }
}

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
    dotted: bool,
    bold: bool,
    hidden: bool,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        ArrowStyle {
            color: ARROW_COLOR.to_string(),
            dashed: false,
            dotted: false,
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
            "dotted" => style.dotted = true,
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
    Partition {
        name: String,
        color: Option<String>,
        body: Vec<LayoutNode>,
    },
}

#[derive(Debug)]
struct ElseBranch {
    label: Option<String>,
    body: Vec<LayoutNode>,
}

/// Returns true if a branch ends with a control-flow terminator (Stop, End,
/// Detach, or Kill). PlantUML omits the merge diamond and post-merge
/// connectors entirely when every branch of an if/else terminates this way.
fn branch_terminates(body: &[LayoutNode]) -> bool {
    matches!(
        body.last(),
        Some(LayoutNode::Stop)
            | Some(LayoutNode::End)
            | Some(LayoutNode::Detach)
            | Some(LayoutNode::Kill)
    )
}

/// Width of an if/while/repeat condition diamond's inner (top/bottom) edge.
/// PlantUML clamps this to a minimum of 24 px so very short conditions still
/// produce a diamond wider than their text. The text inside stays at its
/// measured length — the polygon and the text are sized independently.
fn diamond_inner_w(condition: &str) -> f64 {
    text_render::measure(condition, SMALL_FONT, false).max(DIAMOND_MIN_INNER_W)
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
            ActivityStep::Partition(p) => {
                let name = p.name.clone();
                let color = p.color.clone();
                i += 1;
                let body =
                    collect_until(steps, &mut i, |s| matches!(s, ActivityStep::EndPartition));
                if i < steps.len() {
                    i += 1; // skip EndPartition
                }
                nodes.push(LayoutNode::Partition { name, color, body });
            }
            ActivityStep::EndPartition => {
                i += 1;
            }
            ActivityStep::Backward(_)
            | ActivityStep::Swimlane(_)
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
            ActivityStep::Partition(_) => depth += 1,
            ActivityStep::EndPartition => depth -= 1,
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

/// Compute the asymmetric (left, right) extents of a single node from its
/// vertical centreline. For most nodes this is symmetric (width/2, width/2);
/// for if/else with unequal branches, the left extent (then-side) and right
/// extent (else-side) can differ, which shifts the diagram's cx so both
/// branches remain symmetric around the diamond.
fn node_extents(node: &LayoutNode) -> (f64, f64) {
    match node {
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            ..
        } => {
            let diamond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
            let then_w = sequence_width(then_branch);
            let else_w: f64 = else_branches.iter().map(|b| sequence_width(&b.body)).sum();
            // Branch centrelines are at least `diamond_w + 20` apart, but
            // also at least `(then_w + else_w)/2 + 20` so the branch boxes
            // don't crowd each other. PlantUML takes the max of these two.
            let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0);
            (
                branch_dist / 2.0 + then_w / 2.0,
                branch_dist / 2.0 + else_w / 2.0,
            )
        }
        LayoutNode::Repeat {
            body, condition, ..
        } => {
            // Every `repeatwhile` runs a loop-back arrow up the right side
            // (with or without an `is (...)` label). PlantUML places the
            // condition diamond's left vertex 9 px inside the content area
            // (so left extent is cond_half + 9 regardless of body width),
            // and the loop-back arrow extends 12 px past max(diamond_right,
            // body_right) with another 15 px of right margin past that.
            // Reverse-engineered from goldens with varying body/condition
            // widths.
            let body_w = sequence_width(body);
            let cond_half = diamond_inner_w(condition) / 2.0 + DIAMOND_HALF;
            let body_half = body_w / 2.0;
            let left_extent = cond_half + 9.0;
            let right_extent = cond_half.max(body_half) + 12.0 + 15.0;
            (left_extent, right_extent)
        }
        // Title contributes 3 px of asymmetric padding on each side beyond
        // tw/2 (reverse-engineered against multiple title goldens). This
        // shifts cx 3 px right of action's natural midline when the title
        // is the widest element.
        LayoutNode::Title(t) => {
            let tw = text_render::measure(t, TITLE_FONT_SIZE, true);
            (tw / 2.0 + 3.0, tw / 2.0 + 3.0)
        }
        // Partition wraps a body with a title bar. Left extent is
        // max(title_w, body_w)/2 + 10; right extent is max(title_w/2 + 5,
        // body_w/2 + 10) — the title's notch corner extends 5 px right of
        // the title text, while body content needs 10 px padding either
        // side inside the partition rect.
        LayoutNode::Partition { name, body, .. } => {
            let title_w = text_render::measure(name, TITLE_FONT_SIZE, false);
            let body_w = sequence_width(body);
            let left = title_w.max(body_w) / 2.0 + 10.0;
            let right = (title_w / 2.0 + 5.0).max(body_w / 2.0 + 10.0);
            (left, right)
        }
        _ => {
            let w = node_width(node);
            (w / 2.0, w / 2.0)
        }
    }
}

/// Compute the (left, right) extents of a sequence, taking the max of each
/// dimension independently so an asymmetric node anywhere in the sequence
/// shifts cx as needed.
fn sequence_extents(nodes: &[LayoutNode]) -> (f64, f64) {
    nodes
        .iter()
        .map(node_extents)
        .fold((0.0f64, 0.0f64), |(l, r), (nl, nr)| (l.max(nl), r.max(nr)))
}

fn node_width(node: &LayoutNode) -> f64 {
    match node {
        // Bare start/stop circles: PlantUML lays them out at minimum width
        // without padding (margins are added once at the SVG level). The
        // `+ ACTION_MIN_X * 2.0` previously here forced ~52px of empty
        // space whenever the longest action was narrower than the circle.
        LayoutNode::Start => START_R * 2.0,
        LayoutNode::Stop => STOP_OUTER_R * 2.0,
        LayoutNode::End => 20.0, // `end` uses rx=10 outer circle
        LayoutNode::Action { text_width, .. } => {
            // Box content width only. The outer ACTION_MIN_X margin is added
            // once at the SVG level (margin_x in render_diagram).
            *text_width + ACTION_H_PADDING * 2.0
        }
        LayoutNode::DeprecatedAction { text_width, .. } => {
            // The deprecated-action box is itself just a normal action box.
            // The warning banner lives in its own horizontal band above the
            // diagram and is sized independently in `render`.
            *text_width + ACTION_H_PADDING * 2.0
        }
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            ..
        } => {
            let diamond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
            let then_w = sequence_width(then_branch);
            let else_w: f64 = else_branches.iter().map(|b| sequence_width(&b.body)).sum();
            // Branch centrelines are at least `diamond_w + 20` apart, but
            // also at least `(then_w + else_w)/2 + 20` so the branch boxes
            // don't crowd each other when the branches are wider than the
            // diamond. content_w = branch_dist + (then_w + else_w) / 2.
            let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0);
            branch_dist + (then_w + else_w) / 2.0
        }
        LayoutNode::Fork { branches } => {
            // Mirror emit_fork's bar-width formula: 12 px inner pad each side,
            // 10 px gap between adjacent branches, +18 in the middle gap when
            // the branch count is even, with a minimum bar width when all
            // branches are narrow.
            let branch_widths: Vec<f64> = branches.iter().map(|b| sequence_width(b)).collect();
            let n = branch_widths.len();
            let total_branch_w: f64 = branch_widths.iter().sum();
            let inter_gaps = if n > 1 { (n - 1) as f64 } else { 0.0 };
            let even_extra = if n >= 2 && n.is_multiple_of(2) {
                18.0
            } else {
                0.0
            };
            let bar_w = 12.0 * 2.0 + total_branch_w + inter_gaps * 10.0 + even_extra;
            let min_bar_w = FORK_BAR_MARGIN * 2.0 + 80.0;
            bar_w.max(min_bar_w)
        }
        LayoutNode::While {
            body, condition, ..
        } => {
            let body_w = sequence_width(body);
            let cond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
            body_w.max(cond_w + 40.0) // extra space for loop-back arrow
        }
        LayoutNode::Repeat {
            body, condition, ..
        } => {
            // Every `repeatwhile` produces a loop-back arrow on the right;
            // see node_extents for the formula derivation.
            let body_w = sequence_width(body);
            let cond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
            let cond_half = cond_w / 2.0;
            let body_half = body_w / 2.0;
            let left = cond_half + 9.0;
            let right = cond_half.max(body_half) + 12.0 + 15.0;
            left + right
        }
        // Partition wraps a body with a title bar; width = max(title+15, body+34).
        LayoutNode::Partition { name, body, .. } => {
            let title_w = text_render::measure(name, TITLE_FONT_SIZE, false);
            let body_w = sequence_width(body);
            (title_w + 15.0).max(body_w + 34.0)
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
    // Pending arrow style — modifiers from an explicit `-[…]->` preceding the
    // next flow node change the gap length (10 for hidden, 41.275 for
    // labelled, default 20).
    let mut pending_gap: Option<f64> = None;
    for node in nodes {
        // Notes contribute nothing themselves.
        if matches!(node, LayoutNode::Note { .. }) {
            continue;
        }
        // Title contributes its own height but never has a connector arrow
        // before or after it — the emit loop also skips arrows around titles.
        // Don't toggle prior_flow so the following node (typically `start`)
        // doesn't get an unwanted ARROW_LEN gap.
        if matches!(node, LayoutNode::Title(_)) {
            h += node_height(node);
            pending_gap = None;
            continue;
        }
        // Partition: its top-gap (10 px) already absorbs the would-be arrow.
        // The inbound flow line is drawn by the partition's first inner
        // node, extended back to the cursor's y_in. Same approach as Title.
        if matches!(node, LayoutNode::Partition { .. }) {
            h += node_height(node);
            pending_gap = None;
            // prior_flow remains true so the next node after the partition
            // does get a connector arrow back to the partition's bottom.
            prior_flow = true;
            continue;
        }
        // Track explicit arrow style for the next flow connector.
        if let LayoutNode::Arrow {
            color,
            dashed,
            label,
        } = node
        {
            let style = match color {
                Some(c) => arrow_style_from_brackets(c, *dashed),
                None => ArrowStyle {
                    color: ARROW_COLOR.to_string(),
                    dashed: *dashed,
                    dotted: false,
                    bold: false,
                    hidden: false,
                },
            };
            let gap = if style.hidden {
                10.0
            } else if label.is_some() {
                LABELED_ARROW_LEN
            } else {
                ARROW_LEN
            };
            pending_gap = Some(gap);
            continue;
        }
        // Detach/Kill/Break terminate the flow but produce no visual height.
        // They also suppress the arrow that would precede them.
        if matches!(
            node,
            LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break
        ) {
            prior_flow = false;
            pending_gap = None;
            continue;
        }
        if prior_flow {
            h += pending_gap.unwrap_or(ARROW_LEN);
        }
        pending_gap = None;
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
        LayoutNode::Stop => STOP_OUTER_R * 2.0,
        // `end` uses smaller geometry: rx=10 outer circle, no extra ring.
        LayoutNode::End => 20.0,
        LayoutNode::Action { text, .. } => action_height(text),
        LayoutNode::DeprecatedAction { text, .. } => {
            // Warning banner is accounted for separately by warning_band_h
            // in render; this node's own height is just the action box.
            action_height(text)
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
            // diamond + IF_BRANCH_DOWN + branch_h + IF_BRANCH_UP + merge_diamond.
            // When every branch terminates, the merge diamond and its leading
            // IF_BRANCH_UP gap are skipped (see emit_if).
            let then_terminates = branch_terminates(then_branch);
            let else_terminates = !else_branches.is_empty()
                && else_branches.iter().all(|b| branch_terminates(&b.body));
            let all_terminate = then_terminates && else_terminates;
            if all_terminate {
                diamond_h + IF_BRANCH_DOWN + branch_h
            } else {
                diamond_h + IF_BRANCH_DOWN + branch_h + IF_BRANCH_UP + DIAMOND_HALF * 2.0
            }
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
        // Title region: text_height + 30 of vertical padding so the cursor
        // lands at the cy of the following Start ellipse (composed of 4 px
        // text-top offset + text_height + 16 px gap below text + START_R).
        // Reverse-engineered from golden SVGs.
        LayoutNode::Title(_) => pm::text_height(TITLE_FONT_SIZE) + 30.0,
        // Partition: top gap (10 or 10.4531 if the partition has a fill
        // colour) + 36.49 (title bar) + body height + 12 (bottom margin).
        // The top gap absorbs the would-be inbound arrow.
        LayoutNode::Partition { color, name, body } => {
            let top_gap = if color.is_some()
                || name
                    .chars()
                    .any(|c| matches!(c, 'g' | 'j' | 'p' | 'q' | 'y'))
            {
                10.4531
            } else {
                10.0
            };
            top_gap + 36.4883 + sequence_height(body) + 12.0
        }
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

/// Walk a layout tree and collect every connector label so they can be
/// used to size the SVG.
fn collect_arrow_labels(nodes: &[LayoutNode]) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            LayoutNode::Arrow { label: Some(l), .. } => out.push(l.clone()),
            LayoutNode::If {
                then_branch,
                else_branches,
                ..
            } => {
                out.extend(collect_arrow_labels(then_branch));
                for b in else_branches {
                    out.extend(collect_arrow_labels(&b.body));
                }
            }
            LayoutNode::While { body, .. } | LayoutNode::Repeat { body, .. } => {
                out.extend(collect_arrow_labels(body));
            }
            LayoutNode::Fork { branches } => {
                for b in branches {
                    out.extend(collect_arrow_labels(b));
                }
            }
            _ => {}
        }
    }
    out
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
    /// Resolved color palette for this render (PlantUML defaults +
    /// inline skinparam overrides).
    palette: Palette,
}

#[allow(clippy::too_many_arguments)]
impl SvgEmitter {
    fn with_palette(palette: Palette) -> Self {
        SvgEmitter {
            shapes: String::new(),
            connectors: String::new(),
            palette,
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

    /// Emit a partition's outer rectangle (no rounded corners).
    fn partition_rect(&mut self, fill: &str, height: f64, width: f64, x: f64, y: f64) {
        write!(
            self.shapes,
            r#"<rect fill="{}" height="{}" style="stroke:#000000;stroke-width:1.5;" width="{}" x="{}" y="{}"/>"#,
            fill,
            f(height),
            f(width),
            f(x),
            f(y)
        )
        .unwrap();
    }

    /// Emit the title-corner notch path on a partition.
    fn partition_path(&mut self, r: f64, y: f64, partition_x: f64) {
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="none" style="stroke:#000000;stroke-width:1.5;"/>"#,
            f(r),
            f(y),
            f(r),
            f(y + 9.4883),
            f(r - 10.0),
            f(y + 19.4883),
            f(partition_x),
            f(y + 19.4883)
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

    /// A line that belongs with the SHAPE group (e.g. the X inside an
    /// `end` node — visually part of the node, not a connector).
    fn shape_line(&mut self, stroke: &str, stroke_width: &str, x1: f64, x2: f64, y1: f64, y2: f64) {
        write!(
            self.shapes,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke,
            stroke_width,
            f(x1),
            f(x2),
            f(y1),
            f(y2)
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
        // PlantUML closes filled shape polygons by repeating the first point
        // as the last entry. Arrowhead/connector polygons (polygon_connector)
        // do not.
        let mut closed: Vec<(f64, f64)> = points.to_vec();
        if let (Some(first), Some(last)) = (points.first(), points.last())
            && first != last
        {
            closed.push(*first);
        }
        let pts = polygon_points(&closed);
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

    /// Emit a styled downward arrow (handles colour, dashed/dotted, bold).
    fn down_arrow_full(&mut self, cx: f64, y1: f64, y2: f64, style: &ArrowStyle) {
        let sw = if style.bold {
            "2"
        } else if style.dotted {
            "1.5"
        } else {
            "1"
        };
        let dash = if style.dotted {
            Some("1,3")
        } else if style.dashed {
            Some("2,2")
        } else {
            None
        };
        self.line_with_dash(&style.color, sw, cx, cx, y1, y2, dash);
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

    /// Emit a text element into the connectors buffer (used for arrow labels
    /// which PlantUML interleaves with the connector group rather than the
    /// shape group).
    fn connector_text(
        &mut self,
        fill: &str,
        font_family: &str,
        font_size: f64,
        _text_length: f64,
        x: f64,
        y: f64,
        content: &str,
    ) {
        let base = TextBase {
            x,
            y,
            font_size: font_size as u32,
            font_family,
            fill,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        };
        text_render::emit_text(&mut self.connectors, content, &base);
    }

    /// Emit a line with an explicit dasharray pattern (or none).
    fn line_with_dash(
        &mut self,
        stroke: &str,
        stroke_width: &str,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        dash: Option<&str>,
    ) {
        let dash_str = match dash {
            Some(d) => format!("stroke-dasharray:{d};"),
            None => String::new(),
        };
        write!(
            self.connectors,
            r#"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke,
            stroke_width,
            dash_str,
            f(x1),
            f(x2),
            f(y1),
            f(y2)
        )
        .unwrap();
    }

    /// Emit an upward arrow (arrowhead pointing up).
    #[allow(dead_code)]
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
        // Title is a free-standing label; never gets an inbound connector.
        if let LayoutNode::Title(_) = node {
            y = emit_node(svg, node, cx, y);
            continue;
        }
        // Compute the inbound down-arrow's style + gap (if any). PlantUML
        // emits inbound connectors AFTER the destination node's internal
        // connectors, so we defer the actual svg writes until after
        // emit_node returns. We still advance `y` upfront so the node lands
        // at the right position.
        let mut pending_arrow: Option<(f64, ArrowStyle, Option<String>, f64)> = None;
        if i > 0 {
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
                    LayoutNode::Title(_) => {}
                    LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break => {
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
                        color: svg.palette.arrow_color.clone(),
                        dashed: *dashed,
                        dotted: false,
                        bold: false,
                        hidden: false,
                    },
                    _ => ArrowStyle {
                        color: svg.palette.arrow_color.clone(),
                        ..ArrowStyle::default()
                    },
                };
                let label = match explicit_arrow {
                    Some(LayoutNode::Arrow { label: Some(l), .. }) => Some(l.clone()),
                    _ => None,
                };
                let gap = if style.hidden {
                    10.0
                } else if label.is_some() {
                    LABELED_ARROW_LEN
                } else {
                    ARROW_LEN
                };
                // Partition entry: stretch the inbound arrow so it spans the
                // full distance from prev cursor through the title bar to
                // the first inner action's top (no separate arrow to the
                // partition rect).
                let partition_top_gap = match node {
                    LayoutNode::Partition { color, name, .. } => Some(
                        if color.is_some()
                            || name
                                .chars()
                                .any(|c| matches!(c, 'g' | 'j' | 'p' | 'q' | 'y'))
                        {
                            10.4531
                        } else {
                            10.0
                        },
                    ),
                    _ => None,
                };
                let prev_was_partition = matches!(
                    prev_idx.and_then(|j| nodes.get(j)),
                    Some(LayoutNode::Partition { .. })
                );
                let is_partition = partition_top_gap.is_some();
                // When the previous flow node was a partition, the inbound
                // arrow to the current node extends back 12 px into the
                // partition's bottom margin (overlaying the partition rect).
                let arrow_top_y = if prev_was_partition { y - 12.0 } else { y };
                let arrow_gap = {
                    let base = if let Some(tg) = partition_top_gap {
                        // y_in is `y` (no advance yet); first inner action
                        // sits at y + tg + 36.4883.
                        tg + 36.4883
                    } else {
                        gap
                    };
                    if prev_was_partition {
                        base + 12.0
                    } else {
                        base
                    }
                };
                if !style.hidden {
                    pending_arrow = Some((arrow_top_y, style, label, arrow_gap));
                }
                // Don't advance y past the partition's outer top — the
                // partition's emit handles its own top positioning at y + 10.
                if !is_partition {
                    y += gap;
                }
            }
        }
        let node_y = emit_node(svg, node, cx, y);
        // Inbound connector goes AFTER the node's own emit so it lands
        // after the node's internal connectors in the connectors buffer
        // (matches PlantUML's emission order: internal first, then inbound).
        if let Some((arrow_top, style, label, arrow_gap)) = pending_arrow {
            svg.down_arrow_full(cx, arrow_top, arrow_top + arrow_gap, &style);
            if let Some(l) = label {
                let lw = text_render::measure(&l, SMALL_FONT, false);
                svg.connector_text(
                    TEXT_COLOR,
                    "sans-serif",
                    SMALL_FONT,
                    lw,
                    cx + 4.0,
                    arrow_top + 21.455078125,
                    &l,
                );
            }
        }
        y = node_y;
    }
    y
}

/// Emit a single node at the given center-x and y position.
/// Returns the y position after this node (bottom edge).
fn emit_node(svg: &mut SvgEmitter, node: &LayoutNode, cx: f64, y: f64) -> f64 {
    match node {
        LayoutNode::Start => {
            // The cursor (`y`) represents the centreline at which the next
            // node should sit. PlantUML enforces a minimum of START_CY (25)
            // so the ellipse's top is at least 15 px from the SVG top edge;
            // for first-thing layouts the cursor sits at MARGIN_LEAD (16)
            // and gets clamped up. After a warnings band or title the
            // cursor is already past START_CY and is used as-is.
            let cy = y.max(START_CY);
            let fill = svg.palette.start_fill.clone();
            svg.ellipse(cx, cy, START_R, START_R, &fill, &fill, "1");
            cy + START_R
        }
        LayoutNode::Stop => {
            let cy = y + STOP_OUTER_R;
            let fill = svg.palette.stop_fill.clone();
            svg.ellipse(cx, cy, STOP_OUTER_R, STOP_OUTER_R, "none", &fill, "1");
            svg.ellipse(cx, cy, STOP_INNER_R, STOP_INNER_R, &fill, &fill, "1");
            y + STOP_OUTER_R * 2.0
        }
        LayoutNode::End => {
            // PlantUML's `end` node is a circle with an X inside (not the
            // filled-bullseye that `stop` uses).
            //  - Outer circle: rx=10, fill=none, stroke-width=1.5
            //  - Two diagonal lines forming an X, stroke-width=2.5
            // The X spans from (cx-6.1872, cy-6.1872) to (cx+6.1872, cy+6.1872).
            const END_R: f64 = 10.0;
            const X_HALF: f64 = 6.1872; // empirical from goldens
            let cy = y + END_R;
            let stroke = svg.palette.stop_fill.clone();
            svg.ellipse(cx, cy, END_R, END_R, "none", &stroke, "1.5");
            // X lines belong with shapes (between ellipse and any following
            // text) — they are the visual content of the end node.
            svg.shape_line(
                &stroke,
                "2.5",
                cx - X_HALF,
                cx + X_HALF,
                cy - X_HALF,
                cy + X_HALF,
            );
            svg.shape_line(
                &stroke,
                "2.5",
                cx + X_HALF,
                cx - X_HALF,
                cy - X_HALF,
                cy + X_HALF,
            );
            y + END_R * 2.0
        }
        LayoutNode::Action { text, text_width } => {
            let ah = action_height(text);
            let rect_w = *text_width + ACTION_H_PADDING * 2.0;
            let rect_x = cx - rect_w / 2.0;
            let fill = svg.palette.action_fill.clone();
            let stroke = svg.palette.action_stroke.clone();
            let sw = svg.palette.action_stroke_width.clone();
            let text_col = svg.palette.text_color.clone();
            svg.rect_styled(
                &fill, ah, ACTION_RX, ACTION_RX, &stroke, &sw, rect_w, rect_x, y,
            );
            // Text baseline: padding_top + ascent, both derived from the
            // label's actual font so monospace labels position correctly.
            let lh = text_render::label_height(text, FONT_SIZE);
            let padding_top = (ah - lh) / 2.0;
            let text_y = y + padding_top + text_render::label_ascent(text, FONT_SIZE);
            svg.text_element(
                &text_col,
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
            let fill = svg.palette.action_fill.clone();
            let stroke = svg.palette.action_stroke.clone();
            let sw = svg.palette.action_stroke_width.clone();
            let text_col = svg.palette.text_color.clone();
            svg.rect_styled(
                &fill, ah, ACTION_RX, ACTION_RX, &stroke, &sw, rect_w, rect_x, y,
            );
            let lh = text_render::label_height(text, FONT_SIZE);
            let padding_top = (ah - lh) / 2.0;
            let text_y = y + padding_top + text_render::label_ascent(text, FONT_SIZE);
            svg.text_element(
                &text_col,
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
            // PlantUML wraps the title in `<g class="title" data-source-line="1">`.
            // Title text is centred within an x-extent padded by 4px on the
            // left compared to the action content cx. Baseline is at
            // y + ascent + 4.
            let tw = text_render::measure(text, TITLE_FONT_SIZE, true);
            let text_y = y + pm::ascent(TITLE_FONT_SIZE) + 4.0;
            svg.shapes
                .push_str(r#"<g class="title" data-source-line="1">"#);
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                TITLE_FONT_SIZE,
                tw,
                cx - tw / 2.0 + 1.0,
                text_y,
                text,
                true,
            );
            svg.shapes.push_str("</g>");
            y + pm::text_height(TITLE_FONT_SIZE) + 30.0
        }
        LayoutNode::Partition { name, color, body } => {
            // Partition's outer rect spans from y_in + 10 (top) to y_in +
            // 10 + 36.49 + body_h + 12 (bottom). The title path corner
            // notches the top-right of the title band; the title text sits
            // at partition_x + 3, baseline = partition_top + ascent(14) + 1.
            //
            // PlantUML adds an extra 0.4531 px to the top gap when the
            // partition has a fill colour (the visual offset that makes
            // coloured partitions land slightly lower than uncoloured ones).
            let title_w = text_render::measure(name, TITLE_FONT_SIZE, false);
            let body_w = sequence_width(body);
            let partition_w = (title_w + 15.0).max(body_w + 20.0);
            let partition_x = 16.0; // always MARGIN_LEAD-aligned in goldens
            let top_gap = if color.is_some()
                || name
                    .chars()
                    .any(|c| matches!(c, 'g' | 'j' | 'p' | 'q' | 'y'))
            {
                10.4531
            } else {
                10.0
            };
            let partition_top = y + top_gap;
            let body_h = sequence_height(body);
            let title_band_h = 36.4883; // title bar height (matches goldens)
            let partition_h = title_band_h + body_h + 12.0;
            let partition_right = partition_x + partition_w;

            // Outer rect: fill = color (default none), stroke #000000 width 1.5
            let fill = color.as_deref().unwrap_or("none");
            let resolved_fill = if fill == "none" {
                "none".to_string()
            } else {
                let stripped = fill.strip_prefix('#').unwrap_or(fill);
                crate::sequence::resolve_color(stripped)
            };
            svg.partition_rect(
                &resolved_fill,
                partition_h,
                partition_w,
                partition_x,
                partition_top,
            );

            // Title bar path: M{R},{Y} L{R},{Y+9.49} L{R-10},{Y+19.49} L{X},{Y+19.49}.
            // R is anchored to the title text: partition_x + title_w + 10
            // (the notch sits just past the title's right edge).
            let path_r = partition_x + title_w + 10.0;
            svg.partition_path(path_r, partition_top, partition_x);
            let _ = partition_right;

            // Title text
            let title_y = partition_top + pm::ascent(TITLE_FONT_SIZE) + 1.0;
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                TITLE_FONT_SIZE,
                title_w,
                partition_x + 3.0,
                title_y,
                name,
                false,
            );

            // Emit body inside, at the diagram's cx, starting at partition_top + 36.49.
            // For uncoloured / descender-less partitions a 0.00005 px nudge
            // accounts for Java's intermediate-rounding quirk: the displayed
            // rect_y matches golden (HALF_UP rounding kicks 81.48825 →
            // 81.4883) while inner text_y baselines compute from the
            // un-rounded 81.48825 value. Coloured / descender-titled
            // partitions already have the 0.4531 top-gap shift absorb this.
            let needs_nudge = top_gap == 10.0;
            let body_top = partition_top + title_band_h - if needs_nudge { 0.00005 } else { 0.0 };
            emit_sequence(svg, body, cx, body_top);

            partition_top + partition_h
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
    // Cache the per-diagram colours up front so the many line/polygon emit
    // calls below can borrow them as &str without re-borrowing svg.palette.
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();

    // The diamond's inner edge is clamped to DIAMOND_MIN_INNER_W; the
    // condition's `textLength` is the measured width (no clamp). Track both
    // separately so the polygon and the text are sized independently.
    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);

    // Diamond: centered at (cx, y + DIAMOND_HALF)
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;

    // Diamond polygon (hexagonal for conditions with text)
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, ACTION_STROKE_WIDTH);

    // Condition text (textLength = measured, centred under cx).
    let text_y = y + DIAMOND_HALF + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        TEXT_COLOR,
        "sans-serif",
        SMALL_FONT,
        cond_text_w,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
    );

    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // Then label (to the left of diamond). PlantUML places the label
    // flush against the diamond's left vertex (no horizontal gap), with
    // the baseline at `diamond_cy - descent(11)` (= 64.68 for cy=67).
    if let Some(label) = then_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_left - lw,
            diamond_cy - pm::descent(SMALL_FONT),
            label,
            false,
        );
    }

    // Compute branch positions: PlantUML places the then/else branches
    // with their centrelines `branch_dist` apart, where
    // `branch_dist = max(diamond_w + 20, (then_w + else_w)/2 + 20)` so
    // wider branches don't crowd each other.
    let diamond_w = cond_inner_w + DIAMOND_HALF * 2.0;
    let then_w = sequence_width(then_branch);
    let else_w: f64 = else_branches.iter().map(|b| sequence_width(&b.body)).sum();
    let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0);
    let _else_count = else_branches.len().max(1);
    let then_cx = cx - branch_dist / 2.0;
    let else_cx = cx + branch_dist / 2.0;

    // Else label: text shape, must land in shapes buffer before branch
    // shapes (matches golden order: yes label, no label, then branch boxes).
    if let Some(label) = else_branches.first().and_then(|b| b.label.as_ref()) {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_right,
            diamond_cy - pm::descent(SMALL_FONT),
            label,
            false,
        );
    }

    // Render branches first — this puts the branch shapes into the shapes
    // buffer (after the diamond/condition/labels) and any branch-internal
    // connectors into the connectors buffer FIRST. PlantUML emits branch-
    // internal connectors before the diamond→branch outbound connectors.
    let branch_y = diamond_bottom + IF_BRANCH_DOWN;
    let then_bottom = emit_sequence(svg, then_branch, then_cx, branch_y);
    let else_bottom = if !else_branches.is_empty() {
        emit_sequence(svg, &else_branches[0].body, else_cx, branch_y)
    } else {
        branch_y
    };

    // If every branch ends with a terminator (Stop/End/Detach/Kill), PlantUML
    // skips the merge diamond and post-merge connectors entirely. The two
    // branches stand on their own; the if-block's bottom is the deeper one.
    let then_terminates = branch_terminates(then_branch);
    let else_terminates =
        !else_branches.is_empty() && else_branches.iter().all(|b| branch_terminates(&b.body));
    let all_terminate = then_terminates && else_terminates;

    // Merge diamond at bottom — sits IF_BRANCH_UP px below the deepest branch.
    let merge_y = then_bottom.max(else_bottom) + IF_BRANCH_UP;
    let merge_diamond_top = merge_y;
    let merge_cy = merge_diamond_top + DIAMOND_HALF;

    if !all_terminate {
        // Small merge diamond shape (lands in shapes buffer after branch shapes).
        svg.polygon_shape(
            &diamond_fill,
            &[
                (cx, merge_diamond_top),
                (cx + DIAMOND_HALF, merge_cy),
                (cx, merge_diamond_top + DIAMOND_HALF * 2.0),
                (cx - DIAMOND_HALF, merge_cy),
            ],
            &diamond_stroke,
            ACTION_STROKE_WIDTH,
        );
    }

    // Now emit the if/else-frame connectors AFTER the branch-internal ones.
    // Order: diamond→then, diamond→else, then→merge, else→merge.

    // Diamond → then: horizontal from diamond left to then_cx, then down to
    // branch top, with an arrowhead overlay.
    svg.line_styled(
        &arrow_color,
        "1",
        diamond_left,
        then_cx,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.line_styled(
        &arrow_color,
        "1",
        then_cx,
        then_cx,
        diamond_cy,
        diamond_bottom + IF_BRANCH_DOWN,
        false,
    );
    svg.polygon_connector(
        &arrow_color,
        &[
            (then_cx - 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (then_cx, diamond_bottom + IF_BRANCH_DOWN),
            (then_cx + 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (then_cx, diamond_bottom + IF_BRANCH_DOWN - 6.0),
        ],
        &arrow_color,
        "1",
    );

    // Diamond → else: mirror of the then side.
    svg.line_styled(
        &arrow_color,
        "1",
        diamond_right,
        else_cx,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.line_styled(
        &arrow_color,
        "1",
        else_cx,
        else_cx,
        diamond_cy,
        diamond_bottom + IF_BRANCH_DOWN,
        false,
    );
    svg.polygon_connector(
        &arrow_color,
        &[
            (else_cx - 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (else_cx, diamond_bottom + IF_BRANCH_DOWN),
            (else_cx + 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (else_cx, diamond_bottom + IF_BRANCH_DOWN - 6.0),
        ],
        &arrow_color,
        "1",
    );

    // Then branch → merge — skipped if the branch terminates.
    if !then_terminates {
        svg.line_styled(
            &arrow_color,
            "1",
            then_cx,
            then_cx,
            then_bottom,
            merge_cy,
            false,
        );
        svg.line_styled(
            &arrow_color,
            "1",
            then_cx,
            cx - DIAMOND_HALF,
            merge_cy,
            merge_cy,
            false,
        );
        svg.right_arrow(cx - DIAMOND_HALF, merge_cy, &arrow_color);
    }

    // Else branch → merge — skipped if every else branch terminates.
    if !else_terminates {
        svg.line_styled(
            &arrow_color,
            "1",
            else_cx,
            else_cx,
            else_bottom,
            merge_cy,
            false,
        );
        svg.line_styled(
            &arrow_color,
            "1",
            else_cx,
            cx + DIAMOND_HALF,
            merge_cy,
            merge_cy,
            false,
        );
        svg.left_arrow(cx + DIAMOND_HALF, merge_cy, &arrow_color);
    }

    if all_terminate {
        // No merge diamond was emitted — block height ends at the deeper branch.
        then_bottom.max(else_bottom)
    } else {
        merge_diamond_top + DIAMOND_HALF * 2.0
    }
}

fn emit_fork(svg: &mut SvgEmitter, cx: f64, y: f64, branches: &[Vec<LayoutNode>]) -> f64 {
    if branches.is_empty() {
        return y;
    }

    // Compute branch widths. PlantUML's fork-bar layout:
    //   bar_w = 24 (inner pad each side) + sum(branch_widths) + (n-1)*10 +
    //           (18 if n is even else 0)
    // The extra 18 px goes into the middle gap for even branch counts,
    // pushing the centre branches apart (so the fork has a visual midpoint
    // on the bar rather than landing on a branch).
    let branch_widths: Vec<f64> = branches.iter().map(|b| sequence_width(b)).collect();
    let n = branch_widths.len();
    const FORK_INNER_PAD: f64 = 12.0;
    const FORK_BRANCH_GAP: f64 = 10.0;
    let total_branch_w: f64 = branch_widths.iter().sum();
    let inter_gaps = if n > 1 { (n - 1) as f64 } else { 0.0 };
    let even_extra = if n >= 2 && n.is_multiple_of(2) {
        18.0
    } else {
        0.0
    };
    let mut bar_w =
        FORK_INNER_PAD * 2.0 + total_branch_w + inter_gaps * FORK_BRANCH_GAP + even_extra;
    // Empirical floor: when branch action widths are very small (≲30px),
    // PlantUML still gives each branch enough room for a centred arrowhead
    // and a 14 px outer margin around the bar. Bump the bar width up to
    // satisfy max(bar_w_computed, FORK_BAR_MARGIN*2 + 80) so narrow forks
    // don't collapse.
    let min_bar_w = FORK_BAR_MARGIN * 2.0 + 80.0;
    if bar_w < min_bar_w {
        bar_w = min_bar_w;
    }

    // Top bar
    let bar_x = cx - bar_w / 2.0;
    let bar_color = svg.palette.bar_color.clone();
    let arrow_color = svg.palette.arrow_color.clone();
    svg.rect_styled(
        &bar_color,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        &bar_color,
        "1",
        bar_w,
        bar_x,
        y,
    );

    let bar_bottom = y + FORK_BAR_HEIGHT;

    // Compute branch center-x positions. Branches sit FORK_INNER_PAD from
    // the bar edges with FORK_BRANCH_GAP between adjacent branches. When the
    // branch count is even, an extra 18 px goes into the middle gap.
    let mut branch_centers = Vec::new();
    let mut bx = bar_x + FORK_INNER_PAD;
    if branch_widths.len() == 1 {
        branch_centers.push(bar_x + bar_w / 2.0);
    } else {
        // Distribute extra slack: when the bar was widened past the natural
        // sum (e.g. by min_bar_w), spread across all gaps. Otherwise the
        // 18 px even-count bonus lands solely in the middle gap.
        let natural_w =
            FORK_INNER_PAD * 2.0 + total_branch_w + inter_gaps * FORK_BRANCH_GAP + even_extra;
        let slack = (bar_w - natural_w).max(0.0);
        let slack_per_gap = if inter_gaps > 0.0 {
            slack / inter_gaps
        } else {
            0.0
        };
        // The middle gap index for even n is between branches n/2-1 and n/2.
        let middle_gap_idx = if even_extra > 0.0 {
            Some(n / 2 - 1)
        } else {
            None
        };
        for (i, w) in branch_widths.iter().enumerate() {
            branch_centers.push(bx + w / 2.0);
            bx += w;
            if i + 1 < branch_widths.len() {
                let extra = if Some(i) == middle_gap_idx {
                    even_extra
                } else {
                    0.0
                };
                bx += FORK_BRANCH_GAP + slack_per_gap + extra;
            }
        }
    }

    // Render branches FIRST so their internal arrow connectors land in the
    // connectors buffer before the top/bottom-bar arrows below. Java
    // emits in this order: branch internal connectors, then all top arrows,
    // then all bottom arrows. Reverse-engineered from goldens.
    let mut branch_bottoms = Vec::new();
    for (branch, &bcx) in branches.iter().zip(branch_centers.iter()) {
        let bottom = emit_sequence(svg, branch, bcx, bar_bottom + ARROW_LEN);
        branch_bottoms.push(bottom);
    }

    // Find the maximum bottom
    let max_bottom = branch_bottoms.iter().cloned().fold(0.0f64, f64::max);

    // Top arrows from bar to each branch (all together, after internals).
    for &bcx in &branch_centers {
        svg.down_arrow(bcx, bar_bottom, bar_bottom + ARROW_LEN, &arrow_color);
    }

    // Bottom arrows from each branch to bottom bar.
    for (i, bottom) in branch_bottoms.iter().enumerate() {
        let bcx = branch_centers[i];
        svg.down_arrow(bcx, *bottom, max_bottom + ARROW_LEN, &arrow_color);
    }

    // Bottom bar
    let bottom_bar_y = max_bottom + ARROW_LEN;
    svg.rect_styled(
        &bar_color,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        &bar_color,
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
    // PlantUML emits the while body's shapes BEFORE the condition diamond's
    // shape in the SVG (the diamond visually sits above the body, but in
    // emission order the body comes first). Compute geometry first, emit
    // body, then the diamond + texts, then the inbound connector.
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let text_color = svg.palette.text_color.clone();
    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // Body below diamond — emit it first.
    let body_bottom = emit_sequence(svg, body, cx, diamond_bottom + IF_BRANCH_DOWN);

    // Now the diamond + its texts (lands after body shapes in `shapes`).
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0 + DIAMOND_HALF, diamond_cy),
        (cx + cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0 - DIAMOND_HALF, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, ACTION_STROKE_WIDTH);

    // PlantUML emits the "is (label)" text BEFORE the condition text, so
    // the body-path "yes" appears in the SVG before the inside-diamond
    // condition label.
    if let Some(label) = is_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        // "is (label)" sits just below the diamond on the body's down-path,
        // at cx + 4 horizontally and one ascent below the diamond's bottom.
        svg.text_element(
            &text_color,
            "sans-serif",
            SMALL_FONT,
            lw,
            cx + 4.0,
            diamond_bottom + pm::ascent(SMALL_FONT),
            label,
            false,
        );
    }

    let text_y = y + DIAMOND_HALF + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        &text_color,
        "sans-serif",
        SMALL_FONT,
        cond_text_w,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
    );

    // Inbound connector from diamond down to body (after body shapes and
    // diamond shape are in place).
    svg.down_arrow(
        cx,
        diamond_bottom,
        diamond_bottom + IF_BRANCH_DOWN,
        &arrow_color,
    );

    body_bottom
}

fn emit_repeat(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    body: &[LayoutNode],
    condition: &str,
    is_label: &Option<String>,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let text_color = svg.palette.text_color.clone();

    // PlantUML emits repeat in this order: body shapes → top entry diamond
    // → condition diamond → labels → connectors. Compute positions
    // up-front so we can defer the diamond emits until after the body.
    let top_diamond_size = DIAMOND_HALF;
    let top_bottom = y + top_diamond_size * 2.0;
    let body_y = top_bottom + ARROW_LEN;

    // Body first — its rects/texts land in `shapes` before either diamond.
    let body_bottom = emit_sequence(svg, body, cx, body_y);
    let cond_y = body_bottom + ARROW_LEN;

    // Top entry diamond (small rhombus at y).
    svg.polygon_shape(
        &diamond_fill,
        &[
            (cx, y),
            (cx + top_diamond_size, y + top_diamond_size),
            (cx, y + top_diamond_size * 2.0),
            (cx - top_diamond_size, y + top_diamond_size),
        ],
        &diamond_stroke,
        ACTION_STROKE_WIDTH,
    );

    // Condition diamond (hexagon below body).
    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let cond_diamond_cy = cond_y + DIAMOND_HALF;
    let pts = vec![
        (cx - cond_inner_w / 2.0, cond_y),
        (cx + cond_inner_w / 2.0, cond_y),
        (cx + cond_inner_w / 2.0 + DIAMOND_HALF, cond_diamond_cy),
        (cx + cond_inner_w / 2.0, cond_y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, cond_y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0 - DIAMOND_HALF, cond_diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, ACTION_STROKE_WIDTH);

    let text_y = cond_diamond_cy + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        &text_color,
        "sans-serif",
        SMALL_FONT,
        cond_text_w,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
    );

    // "is" label (optional). Sits with its baseline at cond_cy - descent(11)
    // so the text aligns vertically slightly above the diamond's mid-line.
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    if let Some(label) = is_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            &text_color,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_right,
            cond_diamond_cy - pm::descent(SMALL_FONT),
            label,
            false,
        );
    }

    // Top-diamond → body inbound connector — PlantUML emits this BEFORE
    // the loop-back path in the connector stream.
    svg.down_arrow(cx, top_bottom, body_y, &arrow_color);

    // Loop-back arrow runs up the right side regardless of whether `is`
    // has a label — every `repeatwhile` produces it. The arrow's x sits
    // 12 px past max(diamond_right, body_right).
    let body_w = sequence_width(body);
    let body_right = cx + body_w / 2.0;
    let loop_x = diamond_right.max(body_right) + 12.0;
    svg.line_styled(
        &arrow_color,
        "1",
        diamond_right,
        loop_x,
        cond_diamond_cy,
        cond_diamond_cy,
        false,
    );
    let top_cy = y + top_diamond_size;
    // Vertical loop-back: PlantUML emits the arrowhead polygon BEFORE the
    // line in the SVG, and places the arrowhead at the midpoint of the
    // long vertical run (not at the top) so the direction is clear when
    // the loop spans many actions.
    let mid_y = (top_cy + cond_diamond_cy) / 2.0;
    svg.polygon_connector(
        &arrow_color,
        &[
            (loop_x - 4.0, mid_y + 10.0),
            (loop_x, mid_y),
            (loop_x + 4.0, mid_y + 10.0),
            (loop_x, mid_y + 6.0),
        ],
        &arrow_color,
        "1",
    );
    svg.line_styled(
        &arrow_color,
        "1",
        loop_x,
        loop_x,
        top_cy,
        cond_diamond_cy,
        false,
    );
    svg.line_styled(
        &arrow_color,
        "1",
        loop_x,
        cx + top_diamond_size,
        top_cy,
        top_cy,
        false,
    );
    svg.left_arrow(cx + top_diamond_size, top_cy, &arrow_color);

    // Body → condition diamond connector (after loop-back path).
    svg.down_arrow(cx, body_bottom, cond_y, &arrow_color);

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

    // Collect deprecated color action warnings, deduplicated by color
    // (PlantUML emits one banner per unique color, not one per usage).
    let deprecated_warnings: Vec<(String, f64)> = {
        let mut seen = std::collections::HashSet::new();
        let mut order: Vec<String> = Vec::new();
        for s in &diagram.steps {
            if let ActivityStep::DeprecatedColorAction(dca) = s
                && seen.insert(dca.color.clone())
            {
                order.push(dca.color.clone());
            }
        }
        order
            .into_iter()
            .map(|color| {
                let warning = deprecated_warning(&color);
                let ww = pm::mono_text_width(&warning, 10.0);
                (warning, ww)
            })
            .collect()
    };
    let has_deprecated = !deprecated_warnings.is_empty();

    // Compute overall dimensions. `extents` is asymmetric (left, right) from
    // the diagram's centreline — for if/else with unequal branches, the
    // centreline shifts so both branches stay symmetric around the diamond.
    let (content_left, content_right) = sequence_extents(&tree);
    let content_w = content_left + content_right;
    let content_h = sequence_height(&tree);

    // Total SVG dimensions: PlantUML uses asymmetric margins on both axes —
    // 16px left/top (the ACTION_MIN_X start position) and 19px right/bottom.
    // Verified against single-action goldens of varying widths.
    const MARGIN_LEAD: f64 = 16.0; // left and top
    const MARGIN_TRAIL: f64 = 19.0; // right and bottom
    let margin_top = MARGIN_LEAD;

    // Title contributes its own 3 px asymmetric padding through node_extents
    // so it doesn't need additional SVG-level padding here.

    // Warnings live in their own horizontal band at x=13 — independent of
    // the action layout. SVG width must cover the wider of the action band
    // and the warning band.
    let warn_h_each = pm::mono_text_height(10.0) + 5.0; // = 16.6406
    let max_warning_w = deprecated_warnings
        .iter()
        .map(|(_, w)| *w + 10.0) // warning rect = text + 7 left + 3 right
        .fold(0.0f64, f64::max);
    let warning_total_w = if has_deprecated {
        13.0 + max_warning_w + 17.0
    } else {
        0.0
    };

    // Vertical extent of the warning band: starts at y=13, contains one
    // rect spanning all warnings ((n-1) * 21.6406 + 16.6406), then 17 px
    // gap before the first flow node.
    let num_warnings = deprecated_warnings.len() as f64;
    let warn_band_h = if has_deprecated {
        (num_warnings - 1.0) * (warn_h_each + 5.0) + warn_h_each
    } else {
        0.0
    };
    let start_y = if has_deprecated {
        13.0 + warn_band_h + 17.0
    } else {
        margin_top
    };

    let action_total_w = content_w + MARGIN_LEAD + MARGIN_TRAIL;
    // PlantUML enforces a minimum SVG width of 65 px (= 30 px content
    // breathing room + 35 px margins), so very narrow diagrams (single
    // letter actions) don't collapse to bare lines.
    let min_action_w = if has_deprecated { 0.0 } else { 65.0 };

    // Labelled arrows extend the diagram to the right of cx — for each
    // labelled arrow, the text sits at x = cx + 4 with width label_w and
    // requires ~20 px right margin.
    let label_extent = collect_arrow_labels(&tree)
        .iter()
        .map(|label| text_render::measure(label, SMALL_FONT, false))
        .fold(0.0f64, f64::max);
    let cx_preview = MARGIN_LEAD + content_left;
    let label_total_w = if label_extent > 0.0 {
        cx_preview + 4.0 + label_extent + 20.0
    } else {
        0.0
    };

    let svg_w = action_total_w
        .ceil()
        .max(min_action_w)
        .max(warning_total_w.ceil())
        .max(label_total_w.ceil()) as u32;

    // content_h was computed by sequence_height assuming Start contributes
    // 19 px (cy=25 - MARGIN_LEAD=16 + START_R=10). When start_y > START_CY
    // the actual Start contribution is only START_R (cy = start_y).
    // Subtract the 9 px discrepancy in that case. The same applies when a
    // Title precedes Start — the title's height contribution already places
    // the cursor at the Start ellipse's cy, so Start only adds START_R.
    let title_precedes_start = matches!(tree.first(), Some(LayoutNode::Title(_)))
        && tree
            .iter()
            .skip(1)
            .find_map(|n| match n {
                LayoutNode::Title(_) | LayoutNode::Note { .. } | LayoutNode::Arrow { .. } => None,
                other => Some(other),
            })
            .map(|n| matches!(n, LayoutNode::Start))
            .unwrap_or(false);
    let start_h_delta = if (start_y > START_CY && matches!(tree.first(), Some(LayoutNode::Start)))
        || title_precedes_start
    {
        START_CY + START_R - MARGIN_LEAD - START_R
    } else {
        0.0
    };
    let svg_h = (start_y + content_h - start_h_delta + MARGIN_TRAIL).ceil() as u32;
    // cx aligns the diagram's vertical centreline to MARGIN_LEAD + content_left
    // (the asymmetric left extent). For symmetric layouts this equals
    // MARGIN_LEAD + content_w/2; for if/else with unequal branches it shifts
    // so the branches stay symmetric around the diamond.
    let cx = MARGIN_LEAD + content_left;

    // Build a per-render palette from the diagram's skinparams. Activity
    // diagrams have a substantial set of `skinparam activity*` keys that
    // change individual element colors without affecting the broader
    // theme; resolving them here keeps activity.rs decoupled from the
    // theme machinery in `style.rs`.
    let palette = Palette::from_skinparams(&diagram.meta.skinparams);
    let mut svg = SvgEmitter::with_palette(palette);

    // Emit deprecated warning banners at the top. Warnings live at fixed
    // x=13, y=13, independent of the action layout. PlantUML emits a single
    // rect enclosing all warnings and one text element per warning stacked
    // inside (21.6406 px between baselines, first baseline at line_pitch-6
    // = 10.6406 below the rect top).
    if has_deprecated {
        let line_pitch = warn_h_each; // = 16.6406
        let baseline_pitch = line_pitch + 5.0; // = 21.6406
        let rect_w = max_warning_w;
        svg.rect_styled(
            DEPRECATED_FILL,
            warn_band_h,
            2.5,
            2.5,
            DEPRECATED_STROKE,
            "3",
            rect_w,
            13.0,
            13.0,
        );
        let mut warn_text_y = 13.0 + line_pitch - 6.0;
        for (warning, ww) in &deprecated_warnings {
            svg.monospace_text_element(TEXT_COLOR, 10.0, *ww, 13.0 + 7.0, warn_text_y, warning);
            warn_text_y += baseline_pitch;
        }
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
