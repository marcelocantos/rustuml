// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram SVG renderer.
//!
//! Produces SVG output matching PlantUML's exact format, using PlantUML-
//! compatible font metrics and layout algorithms.

use std::fmt::Write;

use rustuml_parser::diagram::activity::{ActivityDiagram, ActivityStep, NotePosition};

use crate::ftile;
use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// PlantUML activity diagram constants (reverse-engineered from golden SVGs).
const START_R: f64 = 10.0;
const STOP_OUTER_R: f64 = 11.0;
const STOP_INNER_R: f64 = 6.0;
const START_CY: f64 = 25.0;
/// Half-width of PlantUML's `FtileCircle*` terminal tile (start/stop/end).
/// The circle glyph (rx 10-11) sits inside a fixed-width tile whose spine is
/// 13 px from each side, so a bare start/stop/end column centres at
/// MARGIN_LEAD + 13 = 29 (verified against act_minimal_just_start_{stop,end},
/// act_empty_diagram, act_start_no_action: all golden cx="29").
const CIRCLE_TILE_HALF: f64 = 13.0;
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

// Switch-specific layout constants (reverse-engineered from golden SVGs).
const SWITCH_CASE_GAP: f64 = 10.0; // horizontal gap between adjacent SMALL-mode case boxes
// Case-label baseline offsets above the case-box top, per connection type.
const SWITCH_LABEL_OUTER_DY: f64 = 19.7979; // outermost branches (via diamond vertex)
const SWITCH_LABEL_INNER_DY: f64 = 24.7979; // inner branches (drop from horizontal line)
const SWITCH_LABEL_CENTER_DY: f64 = 18.7979; // exact-centre branch (drop from diamond bottom)
// Centre-branch vertical-line split offsets (the centre drop is split into
// two collinear segments, matching PlantUML's connector decomposition).
const SWITCH_CENTER_TOP_SPLIT: f64 = 23.9401; // split distance above the case top
const SWITCH_CENTER_BOT_SPLIT: f64 = 15.0; // split distance above the merge top

const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 11.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const LANE_TITLE_FONT: f64 = 18.0;

// Note geometry (attached `note left/right` beside the anchoring flow node).
// Reverse-engineered from activity goldens.
const NOTE_FONT: f64 = 13.0;
const NOTE_TEXT_PAD_X: f64 = 6.0; // text inset from the note box's left edge
const NOTE_FOLD: f64 = 10.0; // folded-corner size (top-right)
// Box width = max line textLength + this. (6 left pad + 5 right pad + 10 fold.)
const NOTE_BOX_EXTRA_W: f64 = 21.0;
const NOTE_LINE_H: f64 = 15.3105; // per-line height inside a note
const NOTE_FIRST_BASELINE_DY: f64 = 17.5684; // box top → first text baseline
const NOTE_BOX_BASE_H: f64 = 10.0001; // height = this + nlines * NOTE_LINE_H
const NOTE_GAP: f64 = 20.0; // horizontal gap between anchor box and note box
const NOTE_FILL: &str = "#FEFFDD";
const NOTE_STROKE: &str = "#181818";
const NOTE_STROKE_WIDTH: &str = "0.5";

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
    diamond_stroke_width: String,
    arrow_color: String,
    /// Stroke-width string used for activity connector lines (the ones that
    /// link nodes top-to-bottom and the if/fork frame). Defaults to "1" and
    /// rises with `skinparam activityBorderThickness` — PlantUML cascades
    /// the border thickness onto the connector strokes too.
    arrow_thickness: String,
    text_color: String,
    start_fill: String,
    /// Stroke colour for the start ellipse. Mirrors `start_fill` by default
    /// but stays at `#222222` when only `activityStartColor` is set —
    /// PlantUML keeps the original border when only the fill changes.
    start_stroke: String,
    stop_fill: String,
    stop_stroke: String,
    bar_color: String,
    /// Corner radius for action boxes. PlantUML's default action box has a
    /// 12.5 px radius (corresponding to a `roundCorner` of 25). The
    /// `roundCorner` / `activityRoundCorner` skinparams set it to half their
    /// value.
    action_rx: f64,
}

impl Palette {
    fn default_puml() -> Self {
        Self {
            action_fill: ACTION_FILL.into(),
            action_stroke: ACTION_STROKE.into(),
            action_stroke_width: ACTION_STROKE_WIDTH.into(),
            diamond_fill: DIAMOND_FILL.into(),
            diamond_stroke: ACTION_STROKE.into(),
            diamond_stroke_width: ACTION_STROKE_WIDTH.into(),
            arrow_color: ARROW_COLOR.into(),
            arrow_thickness: "1".into(),
            text_color: TEXT_COLOR.into(),
            start_fill: START_FILL.into(),
            start_stroke: START_FILL.into(),
            stop_fill: STOP_FILL.into(),
            stop_stroke: STOP_FILL.into(),
            bar_color: FORK_BAR_COLOR.into(),
            action_rx: ACTION_RX,
        }
    }

    /// Apply the supplied skinparams (key-insensitive) onto the default
    /// PlantUML palette. Unrecognised or empty values are ignored.
    ///
    /// Activity skinparams cascade: `activityBackgroundColor` also sets the
    /// diamond fill, `activityBorderColor` also sets diamond stroke, and
    /// `activityBorderThickness` also sets diamond stroke width — unless a
    /// more specific `activityDiamond*` override appears later in the
    /// skinparam list.
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
                "activitybackgroundcolor" => {
                    p.action_fill = resolved.clone();
                    p.diamond_fill = resolved;
                }
                "activitybordercolor" => {
                    p.action_stroke = resolved.clone();
                    p.diamond_stroke = resolved;
                }
                "activityborderthickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        // Format like PlantUML: integer when whole, otherwise raw float.
                        let w = pm::fmt_coord(v);
                        p.action_stroke_width = w.clone();
                        p.diamond_stroke_width = w.clone();
                        p.arrow_thickness = w;
                    }
                }
                "activitydiamondbackgroundcolor" => p.diamond_fill = resolved,
                "activitydiamondbordercolor" => p.diamond_stroke = resolved,
                "activitydiamondborderthickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.diamond_stroke_width = pm::fmt_coord(v);
                    }
                }
                "activityarrowcolor" | "arrowcolor" => p.arrow_color = resolved,
                // `activityStartColor` sets the start ellipse fill (border
                // keeps its `#222222` default unless a border colour is
                // specified). `activityStopColor` and `activityEndColor`
                // affect distinct shapes in PlantUML — the former targets
                // the `stop` (filled bullseye), the latter the `end` (X-in-
                // circle). Each leaves its border colour at the default.
                "activitystartcolor" => p.start_fill = resolved,
                "activitystopcolor" => p.stop_fill = resolved,
                "activitystartbordercolor" => p.start_stroke = resolved,
                "activitystopbordercolor" => p.stop_stroke = resolved,
                "activityendcolor" => {
                    // The `end` node ignores this skinparam in PlantUML —
                    // its rendering is the X-in-circle in the default
                    // border colour. Accept the key without effect so the
                    // skinparam doesn't fall into the unknown bucket.
                }
                "activitybarcolor" => p.bar_color = resolved,
                "activityfontcolor" => p.text_color = resolved,
                // The global `roundCorner` skinparam sets the corner-radius
                // diameter; the SVG rect radius is half that value (PlantUML
                // stores a diameter and halves it when drawing the rounded
                // rect). `activityRoundCorner` does NOT affect these action
                // boxes in PlantUML — only the global key cascades — so it is
                // intentionally not matched here.
                "roundcorner" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.action_rx = v / 2.0;
                    }
                }
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
        /// Stop/End/Detach/Kill absorbed from the parent sequence when it
        /// follows the `endwhile`. Mirrors PlantUML's
        /// `manageSpecialStopEndAfterEndWhile` — the terminator is drawn
        /// INSIDE the while's frame at translateForSpecial position, not
        /// as a sibling below.
        special_out: Option<Box<LayoutNode>>,
    },
    Repeat {
        body: Vec<LayoutNode>,
        condition: String,
        is_label: Option<String>,
        not_label: Option<String>,
        /// Label of a `backward :label;` action drawn on the loop-back arm.
        backward: Option<String>,
    },
    Fork {
        branches: Vec<Vec<LayoutNode>>,
    },
    /// A `switch (cond) / case (x) / ... / endswitch` block. Cases lay out
    /// horizontally below a condition diamond, fanning out via the diamond's
    /// left/right vertices, and reconverging into a merge diamond below.
    Switch {
        condition: String,
        cases: Vec<SwitchCase>,
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
    /// A top-level swimlanes container. Each lane has its own vertical
    /// column with a header label at the top; the activity flow weaves
    /// across lanes via cross-lane arrows. PlantUML's `|Lane|` markers
    /// in the source partition the flat step list into lane bodies.
    Swimlanes {
        lanes: Vec<Lane>,
    },
}

#[derive(Debug)]
struct Lane {
    name: String,
    #[allow(dead_code)]
    color: Option<String>,
    body: Vec<LayoutNode>,
}

#[derive(Debug)]
struct ElseBranch {
    label: Option<String>,
    body: Vec<LayoutNode>,
}

#[derive(Debug)]
struct SwitchCase {
    label: String,
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

/// True for nodes that occupy vertical space and receive inbound connectors —
/// i.e. everything `emit_sequence` treats as a flow step. Mirrors the skip set
/// at the top of `emit_sequence_ex`.
fn node_is_flow(n: &LayoutNode) -> bool {
    !matches!(
        n,
        LayoutNode::Arrow { .. }
            | LayoutNode::Note { .. }
            | LayoutNode::Title(_)
            | LayoutNode::Detach
            | LayoutNode::Kill
            | LayoutNode::Break
    )
}

/// A branch is "empty" (for if-down corridor purposes) if it has no flow nodes
/// — only arrows/notes/titles, which take no vertical space.
fn branch_is_empty(body: &[LayoutNode]) -> bool {
    !body.iter().any(node_is_flow)
}

/// PlantUML's `ConditionalBuilder.create` routes an `if/else` to the asymmetric
/// "down" layout (`FtileIfDown`) when exactly one branch is empty and the other
/// is populated and non-terminating: the populated branch flows down the centre
/// spine while the empty branch becomes a thin side corridor. Returns the
/// populated branch's body (to lay out on the spine), the empty branch's label
/// (drawn at the diamond) and the populated branch's label, when applicable.
struct IfDownPlan<'a> {
    /// Body of the populated branch (flows down the spine).
    populated: &'a [LayoutNode],
    /// True when the *then* branch is the populated one (controls which side
    /// the diamond labels sit on).
    then_populated: bool,
}

fn if_down_plan<'a>(
    then_branch: &'a [LayoutNode],
    else_branches: &'a [ElseBranch],
) -> Option<IfDownPlan<'a>> {
    // Only a single plain then + single else (no elseif cascade).
    if else_branches.len() != 1 {
        return None;
    }
    let else_body = &else_branches[0].body;
    let then_empty = branch_is_empty(then_branch);
    let else_empty = branch_is_empty(else_body);

    // Exactly one branch empty.
    if then_empty == else_empty {
        return None;
    }
    let populated = if then_empty { else_body } else { then_branch };
    // The populated branch must not terminate — a terminating populated branch
    // is the single-stop case PlantUML handles with a different connector set.
    if branch_terminates(populated) {
        return None;
    }
    Some(IfDownPlan {
        populated,
        then_populated: !then_empty,
    })
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
    // Swimlane detection: if any `|Lane|` marker appears (and there's more
    // than one distinct lane, or content exists before the first marker),
    // wrap the whole flow in a Swimlanes node. PlantUML treats a single-
    // lane diagram (only one `|Lane|` marker with no content before it) as
    // a no-op — the lane chrome is suppressed and the output matches a
    // plain activity diagram.
    let swimlane_markers: Vec<&str> = steps
        .iter()
        .filter_map(|s| match s {
            ActivityStep::Swimlane(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();
    let distinct_lanes: std::collections::BTreeSet<&str> =
        swimlane_markers.iter().copied().collect();
    let has_pre_lane_content = steps
        .iter()
        .take_while(|s| !matches!(s, ActivityStep::Swimlane(_)))
        .any(|s| !matches!(s, ActivityStep::Note(_) | ActivityStep::Arrow(_)));
    if distinct_lanes.len() > 1 || (distinct_lanes.len() == 1 && has_pre_lane_content) {
        return build_swimlanes(steps);
    }

    build_tree_inner(steps)
}

/// Strip the lane name and optional `#color` prefix from a Swimlane
/// marker payload (e.g. `#blue|Colored Lane` → ("Colored Lane",
/// Some("#blue")), `Lane1` → ("Lane1", None)).
fn parse_lane_marker(raw: &str) -> (String, Option<String>) {
    if let Some(rest) = raw.strip_prefix('#')
        && let Some((color, name)) = rest.split_once('|')
    {
        return (name.to_string(), Some(format!("#{color}")));
    }
    (raw.to_string(), None)
}

fn build_swimlanes(steps: &[ActivityStep]) -> Vec<LayoutNode> {
    let mut lanes: Vec<Lane> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_color: Option<String> = None;
    let mut current_steps: Vec<ActivityStep> = Vec::new();

    let flush = |lanes: &mut Vec<Lane>,
                 name: &Option<String>,
                 color: &Option<String>,
                 steps: &mut Vec<ActivityStep>| {
        if name.is_none() && steps.is_empty() {
            return;
        }
        let name = name.clone().unwrap_or_default();
        lanes.push(Lane {
            name,
            color: color.clone(),
            body: build_tree_inner(steps),
        });
        steps.clear();
    };

    for step in steps {
        if let ActivityStep::Swimlane(raw) = step {
            flush(
                &mut lanes,
                &current_name,
                &current_color,
                &mut current_steps,
            );
            let (name, color) = parse_lane_marker(raw);
            current_name = Some(name);
            current_color = color;
        } else {
            current_steps.push(step.clone());
        }
    }
    flush(
        &mut lanes,
        &current_name,
        &current_color,
        &mut current_steps,
    );

    vec![LayoutNode::Swimlanes { lanes }]
}

fn build_tree_inner(steps: &[ActivityStep]) -> Vec<LayoutNode> {
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
                // Absorb a trailing Stop/End/Detach/Kill into the while's
                // special_out — PlantUML's manageSpecialStopEndAfterEndWhile
                // pulls these terminators inside the FtileWhile frame.
                let special_out = if i < steps.len() {
                    let term = match &steps[i] {
                        ActivityStep::Stop => Some(LayoutNode::Stop),
                        ActivityStep::End => Some(LayoutNode::End),
                        ActivityStep::Detach => Some(LayoutNode::Detach),
                        ActivityStep::Kill => Some(LayoutNode::Kill),
                        _ => None,
                    };
                    if let Some(t) = term {
                        i += 1;
                        Some(Box::new(t))
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
                    special_out,
                });
            }
            ActivityStep::EndWhile(_) => {
                i += 1;
            }
            ActivityStep::Repeat => {
                i += 1;
                // PlantUML attaches a `backward :label;` action to the loop-back
                // path (drawn on the right return arm), not to the body spine.
                // Pull its label out of the raw step run before building the body
                // tree so the body doesn't try to lay it out as a spine action.
                let mut backward: Option<String> = None;
                {
                    let mut j = i;
                    while j < steps.len() && !matches!(steps[j], ActivityStep::RepeatWhile(_)) {
                        if let ActivityStep::Backward(label) = &steps[j] {
                            backward = Some(label.clone());
                        }
                        j += 1;
                    }
                }
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
                    backward,
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
            ActivityStep::Switch(condition) => {
                i += 1;
                let mut cases = Vec::new();
                while i < steps.len() {
                    match &steps[i] {
                        ActivityStep::Case(label) => {
                            let label = label.clone();
                            i += 1;
                            let body = collect_until(steps, &mut i, |s| {
                                matches!(s, ActivityStep::Case(_) | ActivityStep::EndSwitch)
                            });
                            cases.push(SwitchCase { label, body });
                        }
                        ActivityStep::EndSwitch => {
                            i += 1;
                            break;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                nodes.push(LayoutNode::Switch {
                    condition: condition.clone(),
                    cases,
                });
            }
            ActivityStep::Case(_) | ActivityStep::EndSwitch => {
                i += 1;
            }
            ActivityStep::Backward(_) | ActivityStep::Swimlane(_) => {
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
            ActivityStep::Switch(_) => depth += 1,
            ActivityStep::EndSwitch => depth -= 1,
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

/// Width of one switch case box: the tile's own content width (PlantUML
/// imposes no extra minimum on switch case tiles).
fn switch_case_width(case: &SwitchCase) -> f64 {
    sequence_width(&case.body)
}

/// PlantUML's `SUPP15` margin used by `FtileSwitchWithDiamonds` in
/// BIG_DIAMOND mode (the horizontal padding either side of the diamond
/// column between the first and last case tiles).
const SWITCH_SUPP15: f64 = 15.0;

/// Faithful port of PlantUML's `FtileSwitchWithDiamonds` horizontal layout.
///
/// `centers` holds the per-case spine x relative to the block-left edge,
/// `block_w` the total packed width, and `diamond_dx` the switch/merge
/// diamond centre x relative to the same block-left edge (the diagram spine
/// aligns to this, *not* to the geometric block centre).
///
/// PlantUML picks BIG_DIAMOND vs SMALL_DIAMOND mode by comparing the spare
/// horizontal room either side of the diamond (`w13`) against the combined
/// width of the inner case tiles (`w9`). In BIG mode the diamond is wide
/// enough that the inner cases are spread evenly under it; in SMALL mode the
/// cases are packed tight and the diamond sits over the geometric centre.
struct SwitchXLayout {
    centers: Vec<f64>,
    block_w: f64,
    diamond_dx: f64,
    big_diamond: bool,
}

fn switch_x_layout(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    let n = cases.len();
    let widths: Vec<f64> = cases.iter().map(switch_case_width).collect();
    let diamond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;

    if n == 0 {
        return SwitchXLayout {
            centers: Vec::new(),
            block_w: diamond_w.max(60.0),
            diamond_dx: diamond_w.max(60.0) / 2.0,
            big_diamond: false,
        };
    }
    if n == 1 {
        let w = widths[0];
        let block_w = w.max(diamond_w);
        return SwitchXLayout {
            centers: vec![block_w / 2.0],
            block_w,
            diamond_dx: block_w / 2.0,
            big_diamond: false,
        };
    }

    // Simple action tiles are symmetric, so getLeft == getRight == w/2.
    let w13 = diamond_w - widths[0] / 2.0 - widths[n - 1] / 2.0;
    let w9: f64 = widths[1..n - 1].iter().sum();

    if w13 > w9 {
        // BIG_DIAMOND: cases[0] flush left, cases[last] at a fixed offset,
        // inner cases spread by suppx = (w13 - w9) / (n - 1).
        let suppx = (w13 - w9) / (n - 1) as f64;
        let mut centers = vec![0.0f64; n];
        let mut dx = 0.0;
        for i in 0..n - 1 {
            centers[i] = dx + widths[i] / 2.0;
            dx += widths[i] + suppx;
        }
        let dx_last = widths[0] + w13 + SWITCH_SUPP15 + SWITCH_SUPP15;
        centers[n - 1] = dx_last + widths[n - 1] / 2.0;
        let block_w = widths[0] + SWITCH_SUPP15 + w13 + SWITCH_SUPP15 + widths[n - 1];
        // dimTotal.getLeft = tile0.getLeft + SUPP15 + dim1.getLeft.
        let diamond_dx = widths[0] / 2.0 + SWITCH_SUPP15 + diamond_w / 2.0;
        SwitchXLayout {
            centers,
            block_w,
            diamond_dx,
            big_diamond: true,
        }
    } else {
        // SMALL_DIAMOND: cases packed tight with a 10-px gap, plus an extra
        // 10-px gap straddling the centreline for even case counts (so the
        // diamond/merge column has room). The diamond sits at the geometric
        // block centre.
        let mut centers = vec![0.0f64; n];
        let mut x = 0.0;
        for i in 0..n {
            if n.is_multiple_of(2) && i == n / 2 {
                x += SWITCH_CASE_GAP;
            }
            centers[i] = x + widths[i] / 2.0;
            x += widths[i] + SWITCH_CASE_GAP;
        }
        let block_w = x - SWITCH_CASE_GAP;
        SwitchXLayout {
            centers,
            block_w,
            diamond_dx: block_w / 2.0,
            big_diamond: false,
        }
    }
}

fn switch_case_block_width(cases: &[SwitchCase], condition: &str) -> f64 {
    switch_x_layout(cases, condition).block_w
}

/// Maximum case-label line count across all cases (≥1).
fn switch_label_lines(cases: &[SwitchCase]) -> f64 {
    cases
        .iter()
        .map(|c| c.label.split('\n').count().max(1) as f64)
        .fold(1.0f64, f64::max)
}

/// Distance from the switch diamond's *bottom* to the case-box tops, i.e.
/// PlantUML's `FtileSwitchWithManyLinks.getYdelta1a`. The label band sits
/// here; its height drives the gap, and BIG_DIAMOND mode adds an extra
/// `diamondHeight/2`. The single-line bases (35.91015 SMALL, 42.95508 BIG)
/// are taken from the goldens; each extra label line adds one text line.
fn switch_below_diamond(cases: &[SwitchCase], big_diamond: bool) -> f64 {
    let extra = (switch_label_lines(cases) - 1.0) * pm::text_height(SMALL_FONT);
    let base = if big_diamond { 42.95508 } else { 35.91015 };
    base + extra
}

/// Left extent (centreline → leftmost drawn element) of a `while` tile.
///
/// Faithful port of PlantUML's `FtileWhile.getTranslateForSpecial`: with a
/// trailing terminator (`stop`/`end`/etc., the `specialOut` ftile), the
/// terminator is pulled left of the loop and becomes the leftmost element. Its
/// centre sits `special_offset` left of the spine, where
///
///   special_offset = max(body_left + halfHex, cond_half) + specialOut.width/2
///
/// (`xWhile = bodyLeft - halfHex` further left, `xDiamond = condHalf`; the
/// special is then offset by its own half-width). The diagram adds a fixed
/// 13px lead from that centre to the SVG content-left edge (LEFT_SVG_PAD 25 −
/// MARGIN_LEAD 16 + arrowhead wing 4). Without a terminator the exit arm wraps
/// at `geo_left − halfHex`; we keep the legacy +25 lead for that (untested)
/// path since no golden exercises it.
/// Effective left extent of a while body's leftward corridor. A deprecated
/// `#color:text;` action in the loop body carries a top-band warning banner;
/// PlantUML's FtileWhile then seats the loop-back/exit corridor (and the
/// `manageSpecialStopEndAfterEndWhile` terminator) 2 px tighter on the left
/// than for a normal body. The body box itself stays centred on the spine —
/// only the while's left geometry sees the 2 px reduction.
fn while_body_left(body: &[LayoutNode], body_left: f64) -> f64 {
    if body
        .iter()
        .any(|n| matches!(n, LayoutNode::DeprecatedAction { .. }))
    {
        body_left - 2.0
    } else {
        body_left
    }
}

fn while_left_extent(
    body_left: f64,
    cond_half: f64,
    end_label: Option<&str>,
    special_out: Option<&LayoutNode>,
) -> f64 {
    let special_extent = match special_out {
        Some(special) => {
            let special_half = node_width(special) / 2.0;
            let special_offset = (body_left + DIAMOND_HALF).max(cond_half) + special_half;
            special_offset + 13.0
        }
        None => cond_half.max(body_left) + 25.0,
    };
    // The `endwhile (label)` text is drawn flush left of the diamond's left
    // vertex (its right edge at diamond_left_vertex). When wide enough it
    // becomes the leftmost element and drives the content-left edge: the label
    // sits 1px left of MARGIN_LEAD, so it contributes cond_half + label_w − 1.
    let label_extent = end_label
        .map(|l| cond_half + text_render::measure(l, SMALL_FONT, false) - 1.0)
        .unwrap_or(0.0);
    special_extent.max(label_extent)
}

/// The lines of note text (block notes accumulate `\n`-joined lines; single
/// `note left: text` notes are one line).
fn note_lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

/// Total drawn width of a note box: longest line's textLength + padding/fold.
fn note_box_width(text: &str) -> f64 {
    let max_line = note_lines(text)
        .iter()
        .map(|l| text_render::measure(l, NOTE_FONT, false))
        .fold(0.0f64, f64::max);
    max_line + NOTE_BOX_EXTRA_W
}

/// Total drawn height of a note box.
fn note_box_height(text: &str) -> f64 {
    NOTE_BOX_BASE_H + note_lines(text).len() as f64 * NOTE_LINE_H
}

/// Emit a note attached beside an Action-style anchor. `cx`/`anchor_w` give
/// the anchor box centre and width; `anchor_cy` its vertical centre. The note
/// box is vertically centred on the anchor and offset `NOTE_GAP` to the side.
fn emit_attached_note(
    svg: &mut SvgEmitter,
    text: &str,
    position: &NotePosition,
    color: Option<&str>,
    cx: f64,
    anchor_w: f64,
    anchor_cy: f64,
) {
    let box_w = note_box_width(text);
    let box_h = note_box_height(text);
    let box_top = anchor_cy - box_h / 2.0;
    let anchor_half = anchor_w / 2.0;
    let fill = color
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| NOTE_FILL.to_string());
    let left_side = matches!(position, NotePosition::Left);
    let (box_left, tip_x) = if left_side {
        let box_right = cx - anchor_half - NOTE_GAP;
        (box_right - box_w, cx - anchor_half)
    } else {
        (cx + anchor_half + NOTE_GAP, cx + anchor_half)
    };
    svg.note_opale(
        &fill, box_left, box_top, box_w, box_h, tip_x, anchor_cy, left_side,
    );
    for (i, line) in note_lines(text).iter().enumerate() {
        let baseline = box_top + NOTE_FIRST_BASELINE_DY + i as f64 * NOTE_LINE_H;
        let lw = text_render::measure(line, NOTE_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            NOTE_FONT,
            lw,
            box_left + NOTE_TEXT_PAD_X,
            baseline,
            line,
            false,
        );
    }
}

/// Detect a leading `floating note` anchored to the diagram's start node: the
/// tree begins with `Start` immediately followed by a single `Note`, and the
/// (pre-processed) source declares that note with the `floating` keyword.
/// Returns the note's `(text, position, color)`. PlantUML draws such a note as
/// a tail-less folded box at the top, vertically centred on the start ellipse,
/// pushing the rest of the spine down.
fn leading_floating_note(
    tree: &[LayoutNode],
    source: Option<&str>,
) -> Option<(String, NotePosition, Option<String>)> {
    // We can't distinguish floating from attached notes from the AST alone
    // (the `floating` keyword is discarded during parsing), so gate on the raw
    // source declaring a floating note.
    let src = source?;
    if !src
        .lines()
        .any(|l| l.trim_start().starts_with("floating note "))
    {
        return None;
    }
    // Tree shape: Start, then a Note, with nothing else between them.
    match (tree.first(), tree.get(1)) {
        (
            Some(LayoutNode::Start),
            Some(LayoutNode::Note {
                text,
                position,
                color,
            }),
        ) => Some((text.clone(), position.clone(), color.clone())),
        _ => None,
    }
}

/// Emit a leading floating note (tail-less folded box) anchored to the start
/// ellipse at spine centre `cx`. The note's top edge sits at `MARGIN_LEAD - 1`
/// (15) and it is offset `NOTE_GAP` past the start ellipse's radius on the
/// chosen side — mirroring `emit_attached_note`'s lateral placement but with
/// no connector tail. Returns the note's box height.
fn emit_leading_floating_note(
    svg: &mut SvgEmitter,
    text: &str,
    position: &NotePosition,
    color: Option<&str>,
    cx: f64,
) -> f64 {
    let box_w = note_box_width(text);
    let box_h = note_box_height(text);
    let box_top = 15.0; // MARGIN_LEAD - 1
    let fill = color
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| NOTE_FILL.to_string());
    let box_left = match position {
        NotePosition::Left => cx - START_R - NOTE_GAP - box_w,
        NotePosition::Right => cx + START_R + NOTE_GAP,
    };
    svg.note_folded(&fill, box_left, box_top, box_w, box_h);
    for (i, line) in note_lines(text).iter().enumerate() {
        let baseline = box_top + NOTE_FIRST_BASELINE_DY + i as f64 * NOTE_LINE_H;
        let lw = text_render::measure(line, NOTE_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            NOTE_FONT,
            lw,
            box_left + NOTE_TEXT_PAD_X,
            baseline,
            line,
            false,
        );
    }
    box_h
}

// ── ftile geometry bridge (incr-4 groundwork, PURE / not yet wired) ──────────
//
// Map a `LayoutNode` to its PlantUML `FtileGeometry` via the faithful port in
// `ftile.rs`, so the canvas/spine can eventually be derived BY CONSTRUCTION
// (replacing the reverse-engineered constants in node_extents/sequence_extents).
//
// Status: NOT wired into rendering — zero behavioural change. Only the tiles
// whose port is verified exact against the Java are mapped here; the rest
// return `None` so the eventual caller falls back to the legacy extent model:
//   - Repeat/Fork: deferred — need the loop-back-arm + fork/join BAR geometry
//     (ftile incr 3b) before their dimension is faithful.
//   - Partition/Swimlanes/Note/Title/elseif-chains/empty-branch-IfDown: not
//     ported; bail.
// The returned `height` is the bare tile-stack height (no connection-gap tiles
// inserted between siblings); width/left — the part that drives canvas width and
// the spine — are exact. Heights are finished at wire time.

/// The condition hexagon as an `FtileGeometry`. Width matches the validated
/// `diamond_inner_w(c) + 2*DIAMOND_HALF` used throughout the emitters, which
/// equals `FtileGeometry::diamond_inside(text_w, _, 0)` for a non-empty label.
#[allow(dead_code)] // incr-4 groundwork: wired in a later increment
fn condition_diamond(condition: &str) -> ftile::FtileGeometry {
    let w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
    ftile::FtileGeometry::new(
        w,
        DIAMOND_HALF * 2.0,
        w / 2.0,
        0.0,
        Some(DIAMOND_HALF * 2.0),
    )
}

/// Fold a node sequence into one geometry via `assemble_linear`. Connectors and
/// terminal markers contribute no tile; a not-yet-ported member bails the whole
/// sequence to `None`.
#[allow(dead_code)] // incr-4 groundwork: wired in a later increment
fn sequence_geometry(nodes: &[LayoutNode]) -> Option<ftile::FtileGeometry> {
    let mut geoms: Vec<ftile::FtileGeometry> = Vec::new();
    for n in nodes {
        match n {
            LayoutNode::Arrow { .. }
            | LayoutNode::Detach
            | LayoutNode::Kill
            | LayoutNode::Break => continue,
            _ => geoms.push(node_geometry(n)?),
        }
    }
    ftile::assemble_linear(&geoms)
}

/// `LayoutNode → FtileGeometry` via the ftile port. See the module note above
/// for which tiles are mapped vs. deferred.
#[allow(dead_code)] // incr-4 groundwork: wired in a later increment
fn node_geometry(node: &LayoutNode) -> Option<ftile::FtileGeometry> {
    use ftile::FtileGeometry as G;
    let g = match node {
        LayoutNode::Start => G::circle_start(),
        LayoutNode::Stop => G::circle_stop(),
        LayoutNode::End => G::circle_end(),
        LayoutNode::Action { text, text_width }
        | LayoutNode::DeprecatedAction {
            text, text_width, ..
        } => G::box_tile(
            *text_width,
            text_render::label_height(text, FONT_SIZE),
            ACTION_H_PADDING,
            ACTION_H_PADDING,
            ACTION_H_PADDING,
            ACTION_H_PADDING,
        ),
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            ..
        } => {
            // Only the binary FtileIfWithDiamonds (one then + one populated
            // else) is ported; elseif-chains (FtileIfLong) and the empty-branch
            // FtileIfDown fall back to the legacy model.
            if else_branches.len() != 1 || if_down_plan(then_branch, else_branches).is_some() {
                return None;
            }
            let diamond1 = condition_diamond(condition);
            let diamond2 = G::diamond_empty(0.0); // bare 24×24 merge diamond
            let t1 = sequence_geometry(then_branch)?;
            let t2 = sequence_geometry(&else_branches[0].body)?;
            // Non-swimlane two-branch: Ydelta1a(10) + Ydelta1b(6) + labels(0).
            ftile::if_with_diamonds(&diamond1, &t1, &t2, &diamond2, 16.0, (0.0, 0.0, 0.0))
        }
        LayoutNode::While {
            condition,
            body,
            special_out,
            ..
        } => {
            let diamond1 = condition_diamond(condition);
            let block = sequence_geometry(body)?;
            let special_g = special_out.as_deref().and_then(node_geometry);
            ftile::while_tile(&diamond1, &block, None, special_g.as_ref(), 0.0)
        }
        LayoutNode::Switch { condition, cases } => {
            if cases.is_empty() {
                return None;
            }
            let diamond1 = condition_diamond(condition);
            let diamond2 = G::diamond_empty(0.0);
            let tiles = cases
                .iter()
                .map(|c| sequence_geometry(&c.body))
                .collect::<Option<Vec<_>>>()?;
            ftile::switch_with_diamonds(&diamond1, &diamond2, &tiles, 20.0)
        }
        _ => return None,
    };
    Some(g)
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
            then_label,
        } => {
            let _ = then_label;
            if let Some(plan) = if_down_plan(then_branch, else_branches) {
                // FtileIfDown reserves a fixed corridor on the right (the empty
                // branch routes out the diamond's east vertex) plus a small
                // left lead. Reverse-engineered against the act_if_*yes_*no
                // goldens: left = cond_half + halfHex + 9, right = cond_half +
                // halfHex + 27.2182 (independent of the east label width).
                // A wide populated branch overrides via branch_w/2.
                let cond_half = diamond_inner_w(condition) / 2.0 + DIAMOND_HALF;
                let branch_w = sequence_width(plan.populated);
                let left = (cond_half + IF_DOWN_LEFT_PAD).max(branch_w / 2.0);
                let right = (cond_half + IF_DOWN_RIGHT_PAD).max(branch_w / 2.0);
                return (left, right);
            }
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
            body,
            condition,
            is_label,
            backward,
            ..
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
            // A `backward :label;` action draws a box on the return arm at the
            // far right; FtileRepeat widens the tile by the backward box's full
            // width and centres on `getLeft = max(body_left, diamond_half)`.
            if let Some(label) = backward {
                let left_extent = cond_half.max(body_half);
                let right_extent =
                    repeat_backward_right_extent(cond_half, body_half, is_label, label);
                (left_extent, right_extent)
            } else {
                let left_extent = cond_half + 9.0;
                let right_extent = cond_half.max(body_half) + 12.0 + 15.0;
                (left_extent, right_extent)
            }
        }
        LayoutNode::While {
            body,
            condition,
            end_label,
            special_out,
            ..
        } => {
            let (body_left, body_right) = sequence_extents(body);
            let cond_half = diamond_inner_w(condition) / 2.0 + DIAMOND_HALF;
            let left_extent = while_left_extent(
                while_body_left(body, body_left),
                cond_half,
                end_label.as_deref(),
                special_out.as_deref(),
            );
            // Right side: loop-back arm at max(cond,body) + halfHex with a 4px
            // arrowhead, plus halfHex of trailing reservation from FtileWhile's
            // `dx + halfHex` term (= 2*halfHex + 3 past max). Verified against
            // the width-only while goldens.
            let right_extent = cond_half.max(body_right) + 2.0 * DIAMOND_HALF + 3.0;
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
        // Swimlanes: asymmetric +4 left / +9 right so cx aligns lane_left
        // at PlantUML's fixed x=20 from SVG edge.
        LayoutNode::Swimlanes { lanes } => {
            let total_w: f64 = lanes.iter().map(lane_width).sum();
            (total_w / 2.0 + 4.0, total_w / 2.0 + 9.0)
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
        // The switch spine aligns to the condition/merge diamond, which in
        // BIG_DIAMOND mode is offset from the geometric block centre.
        LayoutNode::Switch { cases, condition } => {
            let layout = switch_x_layout(cases, condition);
            (layout.diamond_dx, layout.block_w - layout.diamond_dx)
        }
        // Terminal circles (start/stop/end) sit inside a fixed-width tile whose
        // spine is 13 px from each side, independent of the glyph radius. When
        // a circle is the widest node on the spine (a bare start→stop column),
        // this pins cx at MARGIN_LEAD + 13 = 29 rather than MARGIN_LEAD + r.
        LayoutNode::Start | LayoutNode::Stop | LayoutNode::End => {
            (CIRCLE_TILE_HALF, CIRCLE_TILE_HALF)
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
    let mut left = 0.0f64;
    let mut right = 0.0f64;
    // The half-width of the most recent flow node — a note attaches to it and
    // sits `NOTE_GAP` to one side, so its lateral reach from the spine is
    // anchor_half + NOTE_GAP + note_box_width.
    let mut anchor_half = 0.0f64;
    for node in nodes {
        match node {
            LayoutNode::Note { text, position, .. } => {
                let box_w = note_box_width(text);
                let reach = anchor_half + NOTE_GAP + box_w;
                match position {
                    // The left note's left edge lands 1px left of MARGIN_LEAD,
                    // so it contributes reach − 1 to the left extent.
                    NotePosition::Left => left = left.max(reach - 1.0),
                    // The right note's right edge sits 1px past `reach` from
                    // the spine (mirrors the left's −1).
                    NotePosition::Right => right = right.max(reach + 1.0),
                }
            }
            _ => {
                let (nl, nr) = node_extents(node);
                left = left.max(nl);
                right = right.max(nr);
                // Only genuine flow nodes (those with width) can anchor a note.
                if node_width(node) > 0.0 {
                    anchor_half = node_width(node) / 2.0;
                }
            }
        }
    }
    (left, right)
}

/// Width of a single swimlane: the wider of the content (with 10 px
/// internal padding) and the title text (with ~10 px each side).
fn lane_width(lane: &Lane) -> f64 {
    let content_w = sequence_width(&lane.body);
    let title_w = text_render::measure(&lane.name, LANE_TITLE_FONT, false);
    (content_w + 10.0).max(title_w + 10.0)
}

/// cx of the content column within a lane, given the lane's left edge x.
/// Content is left-anchored at `lane_left + 6` and centred on its own
/// natural cx, NOT the geometric centre of the lane.
fn lane_content_cx(lane: &Lane, lane_left: f64) -> f64 {
    let (content_left_ext, _content_right_ext) = sequence_extents(&lane.body);
    lane_left + 6.0 + content_left_ext
}

/// Width of a `backward :label;` action box drawn on a repeat's return arm.
/// Same sizing as a normal action box: measured text + 10px padding each side.
fn repeat_backward_box_w(label: &str) -> f64 {
    text_render::measure(label, FONT_SIZE, false) + ACTION_H_PADDING * 2.0
}

/// Right extent (from the repeat spine) when a `backward :label;` box sits on
/// the return arm. The box left edge clears the widest of the body's right
/// half and the condition diamond east vertex plus its `is (...)` label, with
/// a 10 px return-arm gap; the extent then spans the box's full width.
fn repeat_backward_right_extent(
    cond_half: f64,
    body_half: f64,
    is_label: &Option<String>,
    backward_label: &str,
) -> f64 {
    let is_label_w = is_label
        .as_ref()
        .map(|l| text_render::measure(l, SMALL_FONT, false))
        .unwrap_or(0.0);
    body_half.max(cond_half + is_label_w) + 10.0 + repeat_backward_box_w(backward_label)
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
            if if_down_plan(then_branch, else_branches).is_some() {
                let (l, r) = node_extents(node);
                return l + r;
            }
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
            // the branch count is even. No minimum-width floor — PlantUML's
            // bar spans exactly the branch extents plus 24 px outer pad.
            let branch_widths: Vec<f64> = branches.iter().map(|b| sequence_width(b)).collect();
            let n = branch_widths.len();
            let total_branch_w: f64 = branch_widths.iter().sum();
            let inter_gaps = if n > 1 { (n - 1) as f64 } else { 0.0 };
            let even_extra = if n >= 2 && n.is_multiple_of(2) {
                18.0
            } else {
                0.0
            };
            12.0 * 2.0 + total_branch_w + inter_gaps * 10.0 + even_extra
        }
        LayoutNode::Switch { cases, condition } => switch_case_block_width(cases, condition),
        LayoutNode::While { .. } => {
            // Width = left_extent + right_extent. The asymmetric formula lives
            // in node_extents (single source of truth for While geometry).
            let (l, r) = node_extents(node);
            l + r
        }
        LayoutNode::Repeat {
            body,
            condition,
            is_label,
            backward,
            ..
        } => {
            // Every `repeatwhile` produces a loop-back arrow on the right;
            // see node_extents for the formula derivation.
            let body_w = sequence_width(body);
            let cond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
            let cond_half = cond_w / 2.0;
            let body_half = body_w / 2.0;
            if let Some(label) = backward {
                let left = cond_half.max(body_half);
                let right = repeat_backward_right_extent(cond_half, body_half, is_label, label);
                left + right
            } else {
                let left = cond_half + 9.0;
                let right = cond_half.max(body_half) + 12.0 + 15.0;
                left + right
            }
        }
        // Partition wraps a body with a title bar; width = max(title+15, body+34).
        LayoutNode::Partition { name, body, .. } => {
            let title_w = text_render::measure(name, TITLE_FONT_SIZE, false);
            let body_w = sequence_width(body);
            (title_w + 15.0).max(body_w + 34.0)
        }
        // Swimlanes: sum of per-lane widths. Each lane width is the wider
        // of its content_w + 10 (6 left + 4 right padding inside the lane)
        // and its title_w + horizontal padding for the heading text.
        LayoutNode::Swimlanes { lanes } => lanes.iter().map(lane_width).sum(),
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
            if let Some(plan) = if_down_plan(then_branch, else_branches) {
                // diamond + lead + populated branch + ARROW_LEN + merge diamond.
                // An even-action branch stretches its middle gap by 15 px.
                let branch_h = sequence_height(plan.populated);
                let flow_count = plan.populated.iter().filter(|n| node_is_flow(n)).count();
                let stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
                    IF_DOWN_MID_STRETCH
                } else {
                    0.0
                };
                return DIAMOND_HALF * 2.0
                    + IF_DOWN_LEAD
                    + branch_h
                    + stretch
                    + ARROW_LEN
                    + DIAMOND_HALF * 2.0;
            }
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
        LayoutNode::Switch { cases, condition } => {
            let max_h: f64 = cases
                .iter()
                .map(|c| sequence_height(&c.body))
                .fold(0.0f64, f64::max);
            // Odd case counts have a centre branch that drops straight into
            // the merge diamond (a full 20-px arrow); even counts route both
            // halves sideways, halving the gap.
            let merge_gap = if cases.len().is_multiple_of(2) {
                ARROW_LEN / 2.0
            } else {
                ARROW_LEN
            };
            let big = switch_x_layout(cases, condition).big_diamond;
            DIAMOND_HALF * 2.0
                + switch_below_diamond(cases, big)
                + max_h
                + merge_gap
                + DIAMOND_HALF * 2.0
        }
        LayoutNode::While {
            body,
            is_label,
            special_out,
            ..
        } => {
            // PlantUML's FtileWhile height formula:
            //   height = diamond.h + body.h + 4*halfHex + suppLabel
            // where:
            //   diamond.h = hexagon(24) + northHeight(="yes" text_height)
            //             ≈ 24 + 12.95 ≈ 36.95 (with is_label)
            //   suppLabel = back1.height ≈ 12.95 (the loop-back's
            //              "yes"/incoming label height; PlantUML reserves
            //              text_height(11) even when the label is invisible)
            //   4*halfHex = 48 padding above and below body
            // When special_out is present, the terminator sits INSIDE the
            // frame at translateForSpecial.y = max(3*half, 4*halfHex) = 48,
            // contributing terminator.h on top. We must ensure
            // height >= special_y + special.h.
            let body_h = sequence_height(body);
            // The total while-frame height = diamond.h + body_top_offset
            // + body_h + below-body-gap + wrap-back-offset. We derive it
            // from the same compression-aware formula as emit_while.
            let diamond_alone_h = DIAMOND_HALF * 2.0;
            let body_top_offset = if is_label.is_some() {
                pm::text_height(SMALL_FONT) + 2.0 * DIAMOND_HALF
            } else {
                ARROW_LEN
            };
            // Below body: junction at +10 (compressed if non-empty body)
            // or +12 (empty body); wrap-back continues another +12 for
            // no-specialOut, or descends to special_y for specialOut.
            let below_body = if special_out.is_some() {
                // Two competing lower extents, both measured from body_bottom:
                //   (a) the loop-back junction at body_bottom + 10, plus
                //       PlantUML's reserved back-edge label height
                //       (text_height(SMALL_FONT), present even when the
                //       back-label is empty); this is the usual winner.
                //   (b) the special terminator's bottom: it sits at
                //       4*halfHex below the diamond bottom (translateForSpecial.y),
                //       so relative to body_bottom that's
                //       4*halfHex + special.h - body_top_offset - body_h.
                let s = special_out.as_ref().unwrap();
                let special_h = node_height(s);
                let junction_below = 10.0 + pm::text_height(SMALL_FONT);
                let special_below = 4.0 * DIAMOND_HALF + special_h - body_top_offset - body_h;
                junction_below.max(special_below)
            } else if body.is_empty() {
                DIAMOND_HALF + DIAMOND_HALF // empty: +12 to junction, +12 wrap-back
            } else {
                10.0 + DIAMOND_HALF // +10 junction, +12 wrap-back
            };
            diamond_alone_h + body_top_offset + body_h + below_body
        }
        LayoutNode::Repeat { body, backward, .. } => {
            let body_h = sequence_height(body);
            let diamond_h = DIAMOND_HALF * 2.0;
            // A backward box on the return arm drops the condition diamond an
            // extra halfHex below the body (see emit_repeat's cond_y).
            let cond_gap = if backward.is_some() {
                ARROW_LEN + 10.0
            } else {
                ARROW_LEN
            };
            diamond_h + ARROW_LEN + body_h + cond_gap + diamond_h
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
        // Swimlanes: header band + start-gap + cumulative body heights
        // across lanes (with cross-lane transitions between them). Lanes
        // are columns spatially but temporally sequential — the flow
        // exits one lane and enters the next, so their bodies stack
        // vertically not side-by-side.
        //
        // Each lane's body contributes its own sequence_height. Between
        // consecutive lanes, the cross-lane arrow takes ARROW_LEN of
        // vertical extent. Within a lane, if the first node is Start,
        // sequence_height overcounts by 9 (START_CY+START_R-MARGIN_LEAD-
        // START_R = 19 vs the swimlane-context 10) because Start sits at
        // body_top instead of START_CY. We subtract that overcount.
        //
        // The final +1.3 (matches header_top offset of 1.2969 above
        // MARGIN_LEAD) absorbs PlantUML's slightly larger top margin for
        // swimlane diagrams. The +2.54 trailing absorbs the slightly
        // larger bottom margin.
        LayoutNode::Swimlanes { lanes } => {
            let header_h = pm::text_height(LANE_TITLE_FONT);
            let mut body_h = 0.0_f64;
            for (i, lane) in lanes.iter().enumerate() {
                let mut h = sequence_height(&lane.body);
                if matches!(lane.body.first(), Some(LayoutNode::Start)) {
                    h -= START_CY + START_R - 16.0 - START_R; // = 9
                }
                body_h += h;
                if i + 1 < lanes.len() {
                    body_h += ARROW_LEN; // cross-lane transition
                }
            }
            // 1.30 top offset + header + 15 to start cy + bodies + 1.24
            // bottom adjustment (empirical match to golden svg height).
            1.2969 + header_h + 15.0 + body_h + 1.24
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
            LayoutNode::Switch { cases, .. } => {
                for c in cases {
                    out.extend(collect_arrow_labels(&c.body));
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

    /// Emit a note "opale" (folded-corner box with a tail pointing at the
    /// anchoring node). `box_left`/`box_top` are the box's top-left corner;
    /// `box_w`/`box_h` its drawn size; `tip` the absolute coordinates of the
    /// tail's point on the anchor; `anchor_cy` the anchor's vertical centre.
    /// `left_side` is true when the note sits to the LEFT of its anchor (tail
    /// on the right edge, PlantUML's `getPolygonRight`); false when it sits to
    /// the right (tail on the left edge, `getPolygonLeft`). Faithful port of
    /// `Opale.getPolygonLeft`/`getPolygonRight` with `roundCorner == 0`.
    fn note_opale(
        &mut self,
        fill: &str,
        box_left: f64,
        box_top: f64,
        box_w: f64,
        box_h: f64,
        tip_x: f64,
        anchor_cy: f64,
        left_side: bool,
    ) {
        const CS: f64 = NOTE_FOLD; // cornersize
        const DELTA: f64 = 4.0;
        let bl = box_left;
        let bt = box_top;
        let br = box_left + box_w;
        let bb = box_top + box_h;
        let tip_y = anchor_cy;
        let mut d = String::new();
        if left_side {
            // getPolygonRight: tail on the right edge.
            let y1 = (tip_y - bt - DELTA).clamp(CS, box_h - 2.0 * DELTA);
            write!(
                d,
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(bl), f(bt),
                f(bl), f(bb),
                f(bl), f(bb),
                f(br), f(bb),
                f(br), f(bb),
                f(br), f(bt + y1 + 2.0 * DELTA),
                f(tip_x), f(tip_y),
                f(br), f(bt + y1),
                f(br), f(bt + CS),
                f(br - CS), f(bt),
                f(bl), f(bt),
                f(bl), f(bt),
            )
            .unwrap();
        } else {
            // getPolygonLeft: tail on the left edge.
            let y1 = (tip_y - bt - DELTA).clamp(0.0, box_h - 2.0 * DELTA);
            write!(
                d,
                "M{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(bl), f(bt),
                f(bl), f(bt + y1),
                f(tip_x), f(tip_y),
                f(bl), f(bt + y1 + 2.0 * DELTA),
                f(bl), f(bb),
                f(bl), f(bb),
                f(br), f(bb),
                f(br), f(bb),
                f(br), f(bt + CS),
                f(br - CS), f(bt),
                f(bl), f(bt),
                f(bl), f(bt),
            )
            .unwrap();
        }
        write!(
            self.shapes,
            r#"<path d="{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            d, fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
        // Fold corner (top-right triangle).
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(br - CS), f(bt),
            f(br - CS), f(bt + CS),
            f(br), f(bt + CS),
            f(br - CS), f(bt),
            fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
    }

    /// Emit a tail-less folded-corner note box (a `floating note`, which has
    /// no connector tail pointing at an anchor). Two paths — body then the
    /// top-right fold triangle — matching PlantUML's floating-note emission.
    fn note_folded(&mut self, fill: &str, box_left: f64, box_top: f64, box_w: f64, box_h: f64) {
        const CS: f64 = NOTE_FOLD; // cornersize
        let bl = box_left;
        let bt = box_top;
        let br = box_left + box_w;
        let bb = box_top + box_h;
        // Body: top-left → bottom-left → bottom-right → up to the fold base →
        // diagonal across the fold → back to top-left.
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(bl), f(bt),
            f(bl), f(bb),
            f(br), f(bb),
            f(br), f(bt + CS),
            f(br - CS), f(bt),
            f(bl), f(bt),
            fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
        // Fold corner (top-right triangle).
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(br - CS), f(bt),
            f(br - CS), f(bt + CS),
            f(br), f(bt + CS),
            f(br - CS), f(bt),
            fill, NOTE_STROKE, NOTE_STROKE_WIDTH
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

    /// Emit a connector line that follows the palette's arrow thickness
    /// (so `skinparam activityBorderThickness` cascades through the
    /// if/fork/while/repeat connector frames without each callsite having
    /// to thread the value).
    fn connector_line(&mut self, stroke: &str, x1: f64, x2: f64, y1: f64, y2: f64, dashed: bool) {
        let thickness = self.palette.arrow_thickness.clone();
        self.line_styled(stroke, &thickness, x1, x2, y1, y2, dashed);
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
        let thickness = self.palette.arrow_thickness.clone();
        self.line_styled(color, &thickness, cx, cx, y1, y2, false);
        // Arrowhead: 4px each side, 10px tall, 4px notch. The arrowhead
        // keeps stroke-width:1 — PlantUML scales only the line.
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
        // Per-arrow style markers (bold/dotted) override any global
        // `activityBorderThickness` cascade. A plain arrow follows the
        // palette's connector thickness.
        let palette_thickness = self.palette.arrow_thickness.clone();
        let sw: &str = if style.bold {
            "2"
        } else if style.dotted {
            "1.5"
        } else {
            palette_thickness.as_str()
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
fn emit_sequence(svg: &mut SvgEmitter, nodes: &[LayoutNode], cx: f64, y: f64) -> f64 {
    emit_sequence_ex(svg, nodes, cx, y, None, None)
}

/// Like `emit_sequence`, but `mid_stretch = Some((flow_idx, extra))` adds
/// `extra` px to the inbound arrow of the `flow_idx`-th flow node. Used by the
/// FtileIfDown corridor layout, where an even-action populated branch stretches
/// the gap straddling its vertical midpoint by 15 px.
///
/// `lead_note_h = Some(h)` signals that a leading `floating note` of box
/// height `h` is centred on the diagram's start node: the start ellipse sits
/// at cy = 15 + h/2 (top of the note tile pinned at 15) and the spine resumes
/// from the note tile's bottom (15 + h), pushing everything below down.
fn emit_sequence_ex(
    svg: &mut SvgEmitter,
    nodes: &[LayoutNode],
    cx: f64,
    mut y: f64,
    mid_stretch: Option<(usize, f64)>,
    lead_note_h: Option<f64>,
) -> f64 {
    // Map flow-node ordinal → node index so the stretch can target the right
    // inbound arrow.
    let mut flow_ordinal = 0usize;
    // Extra length applied to the single arrow leaving a leading-floating-note
    // start node (consumed by the next flow node's inbound connector).
    let mut lead_stretch = 0.0f64;
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
        // Leading floating note: the start ellipse is vertically centred on
        // the note tile (top pinned at 15) and the spine is pushed down so the
        // next node sits at `tile_bottom + ARROW_LEN`. The connector still
        // departs from the ellipse's bottom (cy + START_R), so the outbound
        // arrow lengthens by `tile_bottom - ellipse_bottom = nh/2 - START_R`,
        // applied below as `lead_stretch`. The note paths/text were already
        // emitted by the caller's prologue.
        if let (0, LayoutNode::Start, Some(nh)) = (flow_ordinal, node, lead_note_h)
            && nh > 2.0 * START_R
        {
            let top = 15.0; // MARGIN_LEAD - 1
            let cy = top + nh / 2.0;
            let fill = svg.palette.start_fill.clone();
            let stroke = svg.palette.start_stroke.clone();
            svg.ellipse(cx, cy, START_R, START_R, &fill, &stroke, "1");
            y = cy + START_R; // connector departs from the ellipse bottom
            lead_stretch = nh / 2.0 - START_R; // extra length for the next arrow
            flow_ordinal += 1;
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
                let stretch = match mid_stretch {
                    Some((idx, extra)) if idx == flow_ordinal => extra,
                    _ => 0.0,
                };
                let lead = std::mem::take(&mut lead_stretch);
                let gap = stretch
                    + lead
                    + if style.hidden {
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
        // Emit any notes attached to this flow node BEFORE the node itself,
        // so the note's box/text precede the anchor's shape in document order
        // (matching PlantUML's emission). Notes that follow the node (until the
        // next flow node) anchor to it.
        if matches!(node, LayoutNode::Action { .. }) {
            let anchor_w = node_width(node);
            let anchor_h = node_height(node);
            let anchor_cy = y + anchor_h / 2.0;
            for follow in nodes[i + 1..].iter() {
                match follow {
                    LayoutNode::Note {
                        text,
                        position,
                        color,
                    } => {
                        // Only the note-fits-in-row case is handled here; a
                        // taller note would shift the anchor down (not yet
                        // wired), so skip it to avoid mis-positioning.
                        if note_box_height(text) > anchor_h {
                            continue;
                        }
                        emit_attached_note(
                            svg,
                            text,
                            position,
                            color.as_deref(),
                            cx,
                            anchor_w,
                            anchor_cy,
                        );
                    }
                    LayoutNode::Arrow { .. } => {}
                    _ => break,
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
        flow_ordinal += 1;
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
            let stroke = svg.palette.start_stroke.clone();
            svg.ellipse(cx, cy, START_R, START_R, &fill, &stroke, "1");
            cy + START_R
        }
        LayoutNode::Stop => {
            let cy = y + STOP_OUTER_R;
            let fill = svg.palette.stop_fill.clone();
            let stroke = svg.palette.stop_stroke.clone();
            svg.ellipse(cx, cy, STOP_OUTER_R, STOP_OUTER_R, "none", &stroke, "1");
            svg.ellipse(cx, cy, STOP_INNER_R, STOP_INNER_R, &fill, &stroke, "1");
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
            let stroke = svg.palette.stop_stroke.clone();
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
                &fill,
                ah,
                svg.palette.action_rx,
                svg.palette.action_rx,
                &stroke,
                &sw,
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
                &fill,
                ah,
                svg.palette.action_rx,
                svg.palette.action_rx,
                &stroke,
                &sw,
                rect_w,
                rect_x,
                y,
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
        LayoutNode::Switch { condition, cases } => emit_switch(svg, cx, y, condition, cases),
        LayoutNode::While {
            condition,
            is_label,
            body,
            end_label,
            special_out,
        } => emit_while(
            svg,
            cx,
            y,
            condition,
            is_label,
            end_label,
            body,
            special_out.as_deref(),
        ),
        LayoutNode::Repeat {
            body,
            condition,
            is_label,
            not_label: _,
            backward,
        } => emit_repeat(svg, cx, y, body, condition, is_label, backward.as_deref()),
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
        LayoutNode::Swimlanes { lanes } => emit_swimlanes(svg, cx, y, lanes),
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
    // Empty-branch corridor: when one branch is empty and the other populated
    // and non-terminating, PlantUML's FtileIfDown routes the populated branch
    // down the centre spine and the empty branch as a thin side corridor.
    if let Some(plan) = if_down_plan(then_branch, else_branches) {
        let then_label = then_label.as_deref();
        let else_label = else_branches[0].label.as_deref();
        return emit_if_down(svg, cx, y, condition, then_label, else_label, &plan);
    }

    // Cache the per-diagram colours up front so the many line/polygon emit
    // calls below can borrow them as &str without re-borrowing svg.palette.
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

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
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // Condition text (textLength = measured, centred under cx). The condition
    // honours `skinparam activityFontColor`; the then/else branch labels below
    // keep the default black.
    let cond_text_color = svg.palette.text_color.clone();
    let text_y = y + DIAMOND_HALF + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        &cond_text_color,
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
            &diamond_stroke_width,
        );
    }

    // Now emit the if/else-frame connectors AFTER the branch-internal ones.
    // Order: diamond→then, diamond→else, then→merge, else→merge.

    // Diamond → then: horizontal from diamond left to then_cx, then down to
    // branch top, with an arrowhead overlay.
    svg.connector_line(
        &arrow_color,
        diamond_left,
        then_cx,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.connector_line(
        &arrow_color,
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
    svg.connector_line(
        &arrow_color,
        diamond_right,
        else_cx,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.connector_line(
        &arrow_color,
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
        svg.connector_line(&arrow_color, then_cx, then_cx, then_bottom, merge_cy, false);
        svg.connector_line(
            &arrow_color,
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
        svg.connector_line(&arrow_color, else_cx, else_cx, else_bottom, merge_cy, false);
        svg.connector_line(
            &arrow_color,
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

/// Gap from the condition diamond's bottom to the top of the (centred)
/// populated branch in the FtileIfDown layout. PlantUML centres the branch in
/// the vertical band, leaving `ARROW_LEN` below the branch (to the merge
/// diamond) and a slightly larger lead above it: the diamond's south label
/// (the populated branch's label) reserves half its text height beneath the
/// hexagon, pushing the branch down by `text_height(11)/2 - 2`.
const IF_DOWN_LEAD: f64 = ARROW_LEN + 4.477539062500001; // 24.4775

/// Tip offset of the empty-corridor's mid-arrow above the corridor midpoint.
/// PlantUML's `Snake.emphasizeDirection(DOWN)` lands the arrowhead tip
/// `2.2388` px below the geometric midpoint of the corridor's vertical run.
const IF_CORRIDOR_ARROW_OFFSET: f64 = 2.238769531250023;

/// Left lead past the diamond's left vertex in the FtileIfDown layout.
const IF_DOWN_LEFT_PAD: f64 = 9.0;
/// Right corridor reservation past the diamond's right vertex.
const IF_DOWN_RIGHT_PAD: f64 = 27.218200000000003;
/// Extra gap stretched onto the middle inter-action arrow of an even-action
/// populated branch in the FtileIfDown layout.
const IF_DOWN_MID_STRETCH: f64 = 15.0;

/// Asymmetric "down" layout for an `if/else` where one branch is empty.
/// The populated branch flows down the centre spine; the empty branch is a
/// thin corridor on the right that exits the diamond's east vertex and rejoins
/// the merge diamond's east vertex.
fn emit_if_down(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    then_label: Option<&str>,
    else_label: Option<&str>,
    plan: &IfDownPlan,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);

    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // PlantUML's FtileIfDown.drawU emits the populated branch FIRST, then the
    // condition diamond, then the merge diamond. The branch's internal
    // connectors land in the connectors buffer before the if-frame connectors.
    let branch_top = diamond_bottom + IF_DOWN_LEAD;
    // When the populated branch has an even number of flow nodes, PlantUML's
    // vertical centring stretches the inter-action gap straddling the branch's
    // midpoint by 15 px (the gap before the (N/2)-th flow node).
    let flow_count = plan.populated.iter().filter(|n| node_is_flow(n)).count();
    let mid_stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
        Some((flow_count / 2, IF_DOWN_MID_STRETCH))
    } else {
        None
    };
    let branch_bottom = emit_sequence_ex(svg, plan.populated, cx, branch_top, mid_stretch, None);

    // Condition hexagon.
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // Diamond labels then condition text, matching PlantUML's draw order:
    // polygon, SOUTH label (the populated branch's label), condition text,
    // EAST label (the empty branch's label).
    let (south_label, east_label) = if plan.then_populated {
        (then_label, else_label)
    } else {
        (else_label, then_label)
    };
    if let Some(label) = south_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            SMALL_FONT,
            lw,
            cx + 4.0,
            diamond_bottom + pm::ascent(SMALL_FONT),
            label,
            false,
        );
    }
    // Condition text (centred under cx).
    let cond_text_color = svg.palette.text_color.clone();
    let text_y = y + DIAMOND_HALF + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        &cond_text_color,
        "sans-serif",
        SMALL_FONT,
        cond_text_w,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
    );
    if let Some(label) = east_label {
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

    // Merge diamond ARROW_LEN below the branch.
    let merge_top = branch_bottom + ARROW_LEN;
    let merge_cy = merge_top + DIAMOND_HALF;
    svg.polygon_shape(
        &diamond_fill,
        &[
            (cx, merge_top),
            (cx + DIAMOND_HALF, merge_cy),
            (cx, merge_top + DIAMOND_HALF * 2.0),
            (cx - DIAMOND_HALF, merge_cy),
        ],
        &diamond_stroke,
        &diamond_stroke_width,
    );

    // Diamond → populated branch (down arrow on the spine).
    svg.down_arrow(cx, diamond_bottom, branch_top, &arrow_color);

    // Empty corridor on the right: exit east vertex, run down, rejoin merge
    // east vertex with a left arrow. A mid-corridor down arrow marks flow.
    let corridor_x = diamond_right + DIAMOND_HALF;
    let merge_right = cx + DIAMOND_HALF;
    // The corridor is a single PlantUML snake: exit-horizontal, then the
    // emphasised mid down-arrow, then the vertical run, the merge-horizontal,
    // and finally the terminal left-arrow into the merge diamond. The arrow
    // polygons interleave with the line segments in that order.
    // Exit horizontal: diamond east → corridor.
    svg.connector_line(
        &arrow_color,
        diamond_right,
        corridor_x,
        diamond_cy,
        diamond_cy,
        false,
    );
    // Mid-corridor down arrowhead (emphasizeDirection on the vertical run).
    let arrow_tip = (diamond_cy + merge_cy) / 2.0 + IF_CORRIDOR_ARROW_OFFSET;
    svg.polygon_connector(
        &arrow_color,
        &[
            (corridor_x - 4.0, arrow_tip - 10.0),
            (corridor_x, arrow_tip),
            (corridor_x + 4.0, arrow_tip - 10.0),
            (corridor_x, arrow_tip - 6.0),
        ],
        &arrow_color,
        "1",
    );
    // Corridor vertical: down to merge cy.
    svg.connector_line(
        &arrow_color,
        corridor_x,
        corridor_x,
        diamond_cy,
        merge_cy,
        false,
    );
    // Corridor → merge east vertex (left arrow).
    svg.connector_line(
        &arrow_color,
        corridor_x,
        merge_right,
        merge_cy,
        merge_cy,
        false,
    );
    svg.left_arrow(merge_right, merge_cy, &arrow_color);

    // Branch → merge (down arrow on the spine).
    svg.down_arrow(cx, branch_bottom, merge_top, &arrow_color);

    merge_top + DIAMOND_HALF * 2.0
}

enum SwitchConn {
    Outer,
    Inner,
    Center,
}

fn emit_switch(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    cases: &[SwitchCase],
) -> f64 {
    if cases.is_empty() {
        return y;
    }
    let n = cases.len();

    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    // Faithful FtileSwitchWithDiamonds layout. The passed-in cx is the spine,
    // which aligns to the condition/merge diamond (NOT the block centre); the
    // block extends asymmetrically around it per the BIG/SMALL diamond model.
    let layout = switch_x_layout(cases, condition);
    let block_left = cx - layout.diamond_dx;
    let centers: Vec<f64> = layout.centers.iter().map(|c| block_left + c).collect();
    let diamond_cx = cx;

    // Switch condition diamond (inner edge clamped; text measured).
    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = diamond_cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = diamond_cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;
    let pts = vec![
        (diamond_cx - cond_inner_w / 2.0, y),
        (diamond_cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (diamond_cx + cond_inner_w / 2.0, diamond_bottom),
        (diamond_cx - cond_inner_w / 2.0, diamond_bottom),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);
    let cond_text_color = svg.palette.text_color.clone();
    let cond_text_y = diamond_cy + pm::text_height(SMALL_FONT) / 2.0 - pm::descent(SMALL_FONT);
    svg.text_element(
        &cond_text_color,
        "sans-serif",
        SMALL_FONT,
        cond_text_w,
        diamond_cx - cond_text_w / 2.0,
        cond_text_y,
        condition,
        false,
    );

    let cases_top = diamond_bottom + switch_below_diamond(cases, layout.big_diamond);

    // Case bodies (shapes + internal connectors) in source order.
    let mut bottoms = Vec::with_capacity(n);
    for (i, case) in cases.iter().enumerate() {
        bottoms.push(emit_sequence(svg, &case.body, centers[i], cases_top));
    }
    let max_bottom = bottoms.iter().cloned().fold(0.0f64, f64::max);

    let has_center = !n.is_multiple_of(2);
    let merge_gap = if has_center {
        ARROW_LEN
    } else {
        ARROW_LEN / 2.0
    };
    let merge_top = max_bottom + merge_gap;
    let merge_cy = merge_top + DIAMOND_HALF;
    let merge_bottom = merge_top + DIAMOND_HALF * 2.0;

    let center_idx = if has_center { Some(n / 2) } else { None };
    let classify = |i: usize| -> SwitchConn {
        if Some(i) == center_idx {
            SwitchConn::Center
        } else if i == 0 || i == n - 1 {
            SwitchConn::Outer
        } else {
            SwitchConn::Inner
        }
    };

    // Connector emission order: first, last, then inner indices left-to-right.
    let mut order: Vec<usize> = Vec::with_capacity(n);
    order.push(0);
    if n > 1 {
        order.push(n - 1);
    }
    for i in 1..n.saturating_sub(1) {
        order.push(i);
    }

    // The outer-branch label baseline sits slightly lower in BIG_DIAMOND mode
    // (the wider diamond shifts the label box down by ~1.7 px).
    let outer_label_dy = if layout.big_diamond {
        21.5
    } else {
        SWITCH_LABEL_OUTER_DY
    };

    // ── Top connections (diamond → cases). ──
    for &i in &order {
        let bcx = centers[i];
        match classify(i) {
            SwitchConn::Outer => {
                let vertex_x = if bcx <= diamond_cx {
                    diamond_left
                } else {
                    diamond_right
                };
                svg.connector_line(&arrow_color, vertex_x, bcx, diamond_cy, diamond_cy, false);
                svg.connector_line(&arrow_color, bcx, bcx, diamond_cy, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                switch_case_label(svg, &cases[i].label, bcx, cases_top - outer_label_dy);
            }
            SwitchConn::Inner => {
                svg.connector_line(&arrow_color, bcx, bcx, diamond_cy, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                switch_case_label(svg, &cases[i].label, bcx, cases_top - SWITCH_LABEL_INNER_DY);
            }
            SwitchConn::Center => {
                // Drop from the diamond bottom at diamond_cx to the split, jog
                // horizontally to the branch centre (zero-length when aligned,
                // in which case PlantUML omits the horizontal), then down.
                let split = cases_top - SWITCH_CENTER_TOP_SPLIT;
                svg.connector_line(
                    &arrow_color,
                    diamond_cx,
                    diamond_cx,
                    diamond_bottom,
                    split,
                    false,
                );
                if bcx != diamond_cx {
                    svg.connector_line(&arrow_color, diamond_cx, bcx, split, split, false);
                }
                svg.connector_line(&arrow_color, bcx, bcx, split, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                switch_case_label(
                    svg,
                    &cases[i].label,
                    bcx,
                    cases_top - SWITCH_LABEL_CENTER_DY,
                );
            }
        }
    }

    // ── Bottom connections (cases → merge). ──
    for &i in &order {
        let bcx = centers[i];
        let bottom = bottoms[i];
        match classify(i) {
            SwitchConn::Outer => {
                let vertex_x = if bcx <= diamond_cx {
                    diamond_cx - DIAMOND_HALF
                } else {
                    diamond_cx + DIAMOND_HALF
                };
                svg.connector_line(&arrow_color, bcx, bcx, bottom, merge_cy, false);
                svg.connector_line(&arrow_color, bcx, vertex_x, merge_cy, merge_cy, false);
                if bcx <= diamond_cx {
                    svg.right_arrow(vertex_x, merge_cy, &arrow_color);
                } else {
                    svg.left_arrow(vertex_x, merge_cy, &arrow_color);
                }
            }
            SwitchConn::Inner => {
                svg.connector_line(&arrow_color, bcx, bcx, bottom, merge_cy, false);
                switch_down_head(svg, &arrow_color, bcx, merge_cy);
            }
            SwitchConn::Center => {
                // Rise from the branch bottom to the split, jog horizontally to
                // diamond_cx (omitted when aligned), then up into the merge top.
                let split = merge_top - SWITCH_CENTER_BOT_SPLIT;
                svg.connector_line(&arrow_color, bcx, bcx, bottom, split, false);
                if bcx != diamond_cx {
                    svg.connector_line(&arrow_color, bcx, diamond_cx, split, split, false);
                }
                svg.connector_line(
                    &arrow_color,
                    diamond_cx,
                    diamond_cx,
                    split,
                    merge_top,
                    false,
                );
                switch_down_head(svg, &arrow_color, diamond_cx, merge_top);
            }
        }
    }

    // Merge diamond (small rhombus; 7-point closed form per PlantUML).
    svg.polygon_shape(
        &diamond_fill,
        &[
            (diamond_cx, merge_top),
            (diamond_cx, merge_top),
            (diamond_cx + DIAMOND_HALF, merge_cy),
            (diamond_cx, merge_bottom),
            (diamond_cx, merge_bottom),
            (diamond_cx - DIAMOND_HALF, merge_cy),
        ],
        &diamond_stroke,
        &diamond_stroke_width,
    );

    merge_bottom
}

/// Down-pointing arrowhead with tip at (cx, tip_y) into the connectors buffer.
fn switch_down_head(svg: &mut SvgEmitter, color: &str, cx: f64, tip_y: f64) {
    svg.polygon_connector(
        color,
        &[
            (cx - 4.0, tip_y - 10.0),
            (cx, tip_y),
            (cx + 4.0, tip_y - 10.0),
            (cx, tip_y - 6.0),
        ],
        color,
        "1",
    );
}

/// Switch case label text, left-anchored at the branch centre (connectors).
fn switch_case_label(svg: &mut SvgEmitter, label: &str, x: f64, baseline_y: f64) {
    if label.is_empty() {
        return;
    }
    let lw = text_render::measure(label, SMALL_FONT, false);
    svg.connector_text(
        TEXT_COLOR,
        "sans-serif",
        SMALL_FONT,
        lw,
        x,
        baseline_y,
        label,
    );
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
    let bar_w = FORK_INNER_PAD * 2.0 + total_branch_w + inter_gaps * FORK_BRANCH_GAP + even_extra;
    // No empirical floor: PlantUML's fork bar spans exactly the leftmost
    // branch box's left edge minus 12 px to the rightmost box's right edge
    // plus 12 px, i.e. 24 px outer pad + summed branch widths + inter-branch
    // gaps (10 px each, +18 px in the middle gap for even branch counts).
    // Clamping to a minimum width shifts every branch off PlantUML's spine.

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

#[allow(clippy::too_many_arguments)]
fn emit_while(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    is_label: &Option<String>,
    end_label: &Option<String>,
    body: &[LayoutNode],
    special_out: Option<&LayoutNode>,
) -> f64 {
    // PlantUML's FtileWhile layout (reverse-engineered):
    //   - condition hexagon at the top
    //   - "is (yes)" label sits just below the diamond, on the body-path
    //   - inbound arrow from diamond bottom down to body top — long enough
    //     (~37 px) to accommodate the "yes" label
    //   - body (sequence inside the loop)
    //   - junction 12 px below body where the loop-back arm attaches
    //   - loop-back arm: junction → right (loop_x) → up to diamond_cy →
    //     left into diamond's right vertex (the "back to condition" path)
    //   - exit arm: from diamond's left vertex left to exit_x, then down to
    //     the next sibling's y (for byte-exact match this would route around
    //     the full body — we approximate by stopping at junction_y here)
    //   - "endwhile (no)" label sits just outside the diamond's left vertex
    //
    // Geometry constants are derived empirically from goldens. Byte-exact
    // match across all cases (with/without specialOut, with/without
    // compression) requires more work — see project notes.
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();
    let text_color = svg.palette.text_color.clone();

    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;
    let diamond_left_vertex_x = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right_vertex_x = cx + cond_inner_w / 2.0 + DIAMOND_HALF;

    // Inbound arrow length from diamond bottom to body top. The "is (yes)"
    // label sits below the diamond; the inbound arrow must accommodate it
    // plus the gap to body_top. Base reservation: text_height(11) + 2*halfHex
    // ≈ 36.95. When the body is non-empty AND there's no `endwhile (label)`,
    // PlantUML's slot compression removes ~4.82 of slack between diamond and
    // body, giving 32.1348 empirically. (Empty body doesn't compress because
    // it bridges directly to the junction, leaving no compressible slack.)
    // The "is (yes)" label below the diamond reserves text_height(11) plus
    // two hexagon half-sizes of vertical lead before the body top. There is
    // no slot compression here: PlantUML's FtileWhile reserves the full
    // 4*halfHex + label height regardless of whether `endwhile` carries a
    // trailing label (faithful port of calculateDimensionFtile).
    let body_top_offset = if is_label.is_some() {
        pm::text_height(SMALL_FONT) + 2.0 * DIAMOND_HALF
    } else {
        ARROW_LEN
    };
    let body_top = diamond_bottom + body_top_offset;

    // Body below diamond — emit it first (PlantUML emits body shapes before
    // diamond shapes in document order).
    let body_bottom = emit_sequence(svg, body, cx, body_top);

    // Junction y: 12 px below the body for empty bodies, 10 px for
    // non-empty bodies. PlantUML's UEmpty(5, halfHex=12) placeholder is
    // compressed by 2 when adjacent to body content (slot finder removes
    // the slack); for empty bodies there's no adjacent content so the
    // gap stays at 12. The arrowhead midpoint below still uses the
    // un-compressed value (body_bottom + 12) — PlantUML draws the
    // arrowhead at the midpoint of the segment BEFORE compression
    // transforms the line endpoints.
    let junction_y = if body.is_empty() {
        body_bottom + DIAMOND_HALF
    } else {
        body_bottom + 10.0
    };

    // Loop-back arm x position: 12 past whichever is wider, the diamond or
    // the body's right extent.
    let (body_left_ext, body_right_ext) = sequence_extents(body);
    let body_right_x = cx + body_right_ext;
    // A deprecated body pulls the left corridor 2 px tighter (see
    // while_body_left); the special-terminator placement uses the adjusted
    // extent so the terminator lands at the same absolute x as a
    // non-deprecated body. The body box itself stays centred on the spine.
    let body_left_x = cx - while_body_left(body, body_left_ext);
    let loop_x = diamond_right_vertex_x.max(body_right_x) + DIAMOND_HALF;

    // Exit arm geometry. Two modes:
    //
    // (a) WITH special_out (Stop/End/Detach/Kill after endwhile): exit goes
    //     LEFT from diamond_left_vertex to the special's cx column, then
    //     DOWN to the special's top. The special is drawn INSIDE the
    //     while's frame. exit_x = special's cx in absolute coordinates,
    //     positioned per PlantUML's translateForSpecial formula.
    //
    // (b) WITHOUT special_out (regular flow continuation): exit goes LEFT
    //     to halfHex past the widest content, then DOWN to junction_y.
    //     The exit_x is just past the body's left extent (or the diamond's
    //     left vertex, whichever is further out).
    let geo_left_x = diamond_left_vertex_x.min(body_left_x);
    let (exit_x, exit_bottom_y) = if let Some(special) = special_out {
        let special_w = node_width(special);
        // translateForSpecial.x in FtileWhile-local =
        //   min(xWhile - halfHex, xDiamond) - xDeltaBecauseSpecial
        // where xWhile = body_left in FtileWhile-local, xDiamond =
        // diamond_left in FtileWhile-local. Translating to absolute:
        //   min(body_left_x - halfHex, diamond_left_vertex_x) - special_w
        // The special's cx is at translateForSpecial.x + special_w/2.
        let special_left_abs = (body_left_x - DIAMOND_HALF).min(diamond_left_vertex_x) - special_w;
        let special_cx = special_left_abs + special_w / 2.0;
        // translateForSpecial.y in FtileWhile-local =
        //   max(3*half, 4*halfHex) where half = diamond hexagon's
        //   (outY - inY)/2 = 12. So translateForSpecial.y = max(36, 48) = 48.
        // Absolute: special_top = y + (48 - DIAMOND_HALF*2) below diamond.
        // y is the diamond's top. Diamond extends 24 below y. So
        // special_top_abs = y + 48 = diamond_top + 4*halfHex.
        let special_top = y + 4.0 * DIAMOND_HALF;
        (special_cx, special_top)
    } else {
        let exit_x = geo_left_x - DIAMOND_HALF;
        (exit_x, junction_y)
    };

    // Diamond polygon (after body shapes are in `shapes`).
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right_vertex_x, diamond_cy),
        (cx + cond_inner_w / 2.0, diamond_bottom),
        (cx - cond_inner_w / 2.0, diamond_bottom),
        (diamond_left_vertex_x, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // "is (yes)" label below diamond on the body-down path. PlantUML emits
    // this BEFORE the inside-diamond condition text.
    if let Some(label) = is_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
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

    // Condition text inside diamond.
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

    // "endwhile (no)" label just outside diamond's left vertex, with its
    // baseline at diamond_cy - descent(11).
    if let Some(label) = end_label {
        let lw = text_render::measure(label, SMALL_FONT, false);
        svg.text_element(
            &text_color,
            "sans-serif",
            SMALL_FONT,
            lw,
            diamond_left_vertex_x - lw,
            diamond_cy - pm::descent(SMALL_FONT),
            label,
            false,
        );
    }

    // Connector emission order (matches PlantUML's snake walk):
    //   1. body cx vertical: diamond_bottom → body_top (with inbound arrowhead)
    //   2. body cx vertical: body_bottom → junction_y (no arrowhead)
    //   3. junction horizontal: cx → loop_x
    //   4. loop arm UP arrowhead at midpoint
    //   5. loop arm vertical: loop_x from diamond_cy → junction_y
    //   6. loop arm horizontal: diamond_cy from loop_x → diamond_right_vertex
    //   7. loop arm LEFT arrowhead at diamond_right_vertex
    //   8. exit arm horizontal: diamond_cy from diamond_left_vertex → exit_x
    //   9. exit arm vertical: exit_x from diamond_cy → junction_y
    //  10. exit arm DOWN arrowhead at the end (or at midpoint for long arms)

    // 1. Inbound arrow from diamond bottom to body top.
    svg.down_arrow(cx, diamond_bottom, body_top, &arrow_color);

    // 2. Body bottom → junction (only if body has content; for empty body
    // the inbound arrow already reaches the junction-equivalent point).
    if !body.is_empty() {
        svg.line_styled(&arrow_color, "1", cx, cx, body_bottom, junction_y, false);
    }

    // 3. Horizontal at junction from body cx out to loop_x.
    svg.line_styled(&arrow_color, "1", cx, loop_x, junction_y, junction_y, false);

    // 4. UP arrowhead at midpoint of the loop arm's vertical run.
    // PlantUML draws this at the midpoint of (diamond_cy, body_bottom +
    // halfHex), adjusted by the same compression that shifts body_top up.
    let mid_y = (diamond_cy + body_bottom + DIAMOND_HALF) / 2.0;
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

    // 5. Loop arm vertical at loop_x.
    svg.line_styled(
        &arrow_color,
        "1",
        loop_x,
        loop_x,
        diamond_cy,
        junction_y,
        false,
    );

    // 6. Loop arm horizontal at diamond_cy: loop_x → diamond_right_vertex.
    svg.line_styled(
        &arrow_color,
        "1",
        loop_x,
        diamond_right_vertex_x,
        diamond_cy,
        diamond_cy,
        false,
    );

    // 7. LEFT arrowhead at diamond_right_vertex.
    svg.polygon_connector(
        &arrow_color,
        &[
            (diamond_right_vertex_x + 10.0, diamond_cy - 4.0),
            (diamond_right_vertex_x, diamond_cy),
            (diamond_right_vertex_x + 10.0, diamond_cy + 4.0),
            (diamond_right_vertex_x + 6.0, diamond_cy),
        ],
        &arrow_color,
        "1",
    );

    // 8. Exit arm horizontal at diamond_cy: diamond_left_vertex → exit_x.
    svg.line_styled(
        &arrow_color,
        "1",
        diamond_left_vertex_x,
        exit_x,
        diamond_cy,
        diamond_cy,
        false,
    );

    // 9. Exit arm vertical at exit_x — single line from diamond_cy down
    // to the final exit y. For specialOut, that's exit_bottom_y (= special
    // top). For no-specialOut, that's wrap_y (= exit_bottom_y + halfHex),
    // and we emit a wrap-back horizontal to cx afterward.
    let wrap_y = if special_out.is_some() {
        exit_bottom_y
    } else {
        exit_bottom_y + DIAMOND_HALF
    };
    svg.line_styled(&arrow_color, "1", exit_x, exit_x, diamond_cy, wrap_y, false);

    // 10. DOWN arrowhead. When special_out is present, the arrowhead lands
    // AT the terminator's top (ConnectionOutSpecial uses endDecoration);
    // otherwise the arrowhead is at the midpoint of the long exit arm
    // (ConnectionOut with emphasizeDirection).
    let arrow_y = if special_out.is_some() {
        wrap_y
    } else {
        (diamond_cy + wrap_y) / 2.0
    };
    svg.polygon_connector(
        &arrow_color,
        &[
            (exit_x - 4.0, arrow_y - 10.0),
            (exit_x, arrow_y),
            (exit_x + 4.0, arrow_y - 10.0),
            (exit_x, arrow_y - 6.0),
        ],
        &arrow_color,
        "1",
    );

    // 11. Either emit the special_out terminator INSIDE the while's frame
    // (ConnectionOutSpecial) or emit the wrap-back horizontal from exit_x
    // back to cx (ConnectionOut's snake2).
    if let Some(special) = special_out {
        emit_node(svg, special, exit_x, wrap_y)
    } else {
        svg.line_styled(&arrow_color, "1", exit_x, cx, wrap_y, wrap_y, false);
        wrap_y
    }
}

fn emit_repeat(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    body: &[LayoutNode],
    condition: &str,
    is_label: &Option<String>,
    backward: Option<&str>,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();
    let text_color = svg.palette.text_color.clone();

    // PlantUML emits repeat in this order: body shapes → top entry diamond
    // → condition diamond → labels → connectors. Compute positions
    // up-front so we can defer the diamond emits until after the body.
    let top_diamond_size = DIAMOND_HALF;
    let top_bottom = y + top_diamond_size * 2.0;
    let body_y = top_bottom + ARROW_LEN;

    // Body first — its rects/texts land in `shapes` before either diamond.
    let body_bottom = emit_sequence(svg, body, cx, body_y);
    // A `backward :label;` box sits on the return arm at the body's vertical
    // band; FtileRepeat distributes its `8*halfHex` slack so the condition
    // diamond drops an extra halfHex (10 px) below the body to clear the
    // return path's arrowhead into the box bottom.
    let cond_y = if backward.is_some() {
        body_bottom + ARROW_LEN + 10.0
    } else {
        body_bottom + ARROW_LEN
    };

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
        &diamond_stroke_width,
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
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

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
    let arm_x = diamond_right.max(body_right) + 12.0;
    let top_cy = y + top_diamond_size;

    if let Some(label) = backward {
        // `backward :label;` draws an action box on the return arm. The loop
        // arm runs up the right side through the centre of this box, which sits
        // at the same vertical band as the body. Mirrors FtileRepeat's
        // ConnectionBackBackward1/2 routing: cond diamond → up into box bottom,
        // box → up out of box top → across to the entry diamond.
        //
        // The box's left edge clears the widest of the body's right edge and
        // the condition diamond's east vertex plus its `is (...)` label, then
        // adds a 10 px gap (PlantUML's return-arm reservation).
        let is_label_w = is_label
            .as_ref()
            .map(|l| text_render::measure(l, SMALL_FONT, false))
            .unwrap_or(0.0);
        let bw = repeat_backward_box_w(label);
        let box_left = body_right.max(diamond_right + is_label_w) + 10.0;
        let box_cx = box_left + bw / 2.0;
        let box_top = body_y;
        let box_bottom = body_y + action_height(label);

        // Box shape first — PlantUML emits the backward tile's shapes before
        // the loop-back connectors in document order.
        let backward_node = LayoutNode::Action {
            text: label.to_string(),
            text_width: text_render::measure(label, FONT_SIZE, false),
        };
        emit_node(svg, &backward_node, box_cx, box_top);

        // Horizontal from cond diamond's east vertex out to the arm column.
        svg.line_styled(
            &arrow_color,
            "1",
            diamond_right,
            box_cx,
            cond_diamond_cy,
            cond_diamond_cy,
            false,
        );
        // Up from the cond-diamond line into the box's bottom. PlantUML emits
        // the line first, then the arrowhead polygon (tip at the box bottom).
        svg.line_styled(
            &arrow_color,
            "1",
            box_cx,
            box_cx,
            box_bottom,
            cond_diamond_cy,
            false,
        );
        svg.polygon_connector(
            &arrow_color,
            &[
                (box_cx - 4.0, box_bottom + 10.0),
                (box_cx, box_bottom),
                (box_cx + 4.0, box_bottom + 10.0),
                (box_cx, box_bottom + 6.0),
            ],
            &arrow_color,
            "1",
        );
        // Up from the box's top to the entry-diamond row.
        svg.line_styled(&arrow_color, "1", box_cx, box_cx, top_cy, box_top, false);
        // Across to the entry diamond's east vertex (arrowhead left into it).
        svg.line_styled(
            &arrow_color,
            "1",
            box_cx,
            cx + top_diamond_size,
            top_cy,
            top_cy,
            false,
        );
        svg.left_arrow(cx + top_diamond_size, top_cy, &arrow_color);
    } else {
        let loop_x = arm_x;
        svg.line_styled(
            &arrow_color,
            "1",
            diamond_right,
            loop_x,
            cond_diamond_cy,
            cond_diamond_cy,
            false,
        );
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
    }

    // Body → condition diamond connector (after loop-back path).
    svg.down_arrow(cx, body_bottom, cond_y, &arrow_color);

    cond_y + DIAMOND_HALF * 2.0
}

/// Emit a swimlanes block. Lanes are arranged left-to-right with vertical
/// dividers between them; each lane's content flows in its own column with
/// cross-lane arrows joining steps that change lane.
///
/// Layout (reverse-engineered from goldens):
///   - Header band at the top: text_height(18) ≈ 21.2 px tall, spans all
///     lanes. Title text per lane sits centered on its lane's content cx.
///   - Vertical dividers (stroke-width 1.5) at each lane boundary, from
///     header top to last_content_y.
///   - Per-lane content centered on lane.cx with 6 left + 4 right padding.
///   - Cross-lane arrow: source_cx vertical down 5 → horizontal at +5 →
///     target_cx vertical down (15 more) with arrowhead.
fn emit_swimlanes(svg: &mut SvgEmitter, cx: f64, y: f64, lanes: &[Lane]) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let text_color = svg.palette.text_color.clone();
    let divider_color = "#000000";

    // PlantUML uses a slightly shifted header_top for swimlanes (y=17.2969
    // ≈ MARGIN_LEAD + 1.30) and a +4/+9 asymmetric extra padding around
    // the lanes (see node_extents below). We compute lane_left from the
    // passed cx, which has been positioned to make lane_lefts[0] = 20.
    //
    // The header band sits 1.2969 px below the content origin handed in by
    // render() (= MARGIN_LEAD = 16 in the common case → 17.2969). When the
    // diagram carries deprecation banners, render() raises that origin by
    // one baseline pitch per banner, so the swimlane chrome floats below the
    // banner band (e.g. y=38.9375 for one banner) instead of being pinned to
    // the hardcoded 17.2969.
    let header_text_h = pm::text_height(LANE_TITLE_FONT);
    let header_top = y + 1.2969;
    let header_bottom = header_top + header_text_h;
    let body_top = header_bottom + 15.0;

    let lane_widths: Vec<f64> = lanes.iter().map(lane_width).collect();
    let total_w: f64 = lane_widths.iter().sum();
    let mut lane_left = cx - total_w / 2.0;
    let mut lane_lefts: Vec<f64> = Vec::with_capacity(lanes.len());
    let mut lane_cxs: Vec<f64> = Vec::with_capacity(lanes.len());
    for (lane, &lw) in lanes.iter().zip(lane_widths.iter()) {
        lane_lefts.push(lane_left);
        lane_cxs.push(lane_content_cx(lane, lane_left));
        lane_left += lw;
    }
    let right_edge = lane_left;

    // Empty header rect spanning all lanes (fill=none, stroke=none).
    // PlantUML's header_rect.width = sum_lane_widths + 1.8477 (empirical
    // constant — purpose unknown, but consistent across all golden cases).
    write!(
        svg.shapes,
        r#"<rect fill="none" height="{}" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        f(header_text_h),
        f(total_w + 1.8476),
        f(lane_lefts[0]),
        f(header_top),
    )
    .unwrap();

    // Pre-compute the final last_y so dividers (emitted interleaved with
    // lane bodies) can use the full vertical extent. Round to 4 decimals
    // (HALF_UP) before adding to body_top — this matches PlantUML's
    // intermediate precision and avoids accumulating IEEE 754 sub-ULPs
    // that would push final_last_y across rounding boundaries.
    let mut body_h_total = 0.0_f64;
    for (i, lane) in lanes.iter().enumerate() {
        let mut h = sequence_height(&lane.body);
        if matches!(lane.body.first(), Some(LayoutNode::Start)) {
            h -= 9.0; // Start contributes only START_R inside swimlane.
        }
        body_h_total += h;
        if i + 1 < lanes.len() {
            body_h_total += ARROW_LEN;
        }
    }
    body_h_total = (body_h_total * 10000.0 + 0.5).floor() / 10000.0;
    let final_last_y = body_top + body_h_total;

    // Emit lane bodies, interleaving the LEFT divider of each lane after
    // its body shapes land. Cross-lane arrow CONNECTORS are deferred
    // (collected per-lane and emitted after all bodies) so they appear at
    // the end of the connectors buffer, matching PlantUML's golden order
    // (per-lane internal arrows first, then all cross-lane arrows).
    let mut last_y = body_top;
    let mut prev_lane_idx: Option<usize> = None;
    // (prev_cx, prev_last_y, target_cx, target_y) for each cross-lane.
    let mut deferred_cross_lanes: Vec<(f64, f64, f64, f64)> = Vec::new();
    for (lane_idx, lane) in lanes.iter().enumerate() {
        let lane_cx = lane_cxs[lane_idx];
        let lane_y = if let Some(prev) = prev_lane_idx {
            let prev_cx = lane_cxs[prev];
            // Round the cross-lane drop target to 4 decimals (HALF_UP) before
            // feeding it into the next lane's body layout. PlantUML rounds
            // tile coordinates at each boundary, so without this the
            // accumulated float carries sub-ULP excess that pushes downstream
            // y-values one ULP above the golden at the 4th decimal.
            let target_y = ((last_y + ARROW_LEN) * 10000.0 + 0.5).floor() / 10000.0;
            deferred_cross_lanes.push((prev_cx, last_y, lane_cx, target_y));
            target_y
        } else {
            body_top
        };
        last_y = emit_sequence(svg, &lane.body, lane_cx, lane_y);

        // Emit this lane's LEFT divider with the FULL final_last_y so it
        // spans the entire diagram height (not just up to the current
        // lane's bottom). Skip on the last lane — its left divider + the
        // final right divider are emitted together after the loop.
        if lane_idx + 1 < lanes.len() {
            write!(
                svg.shapes,
                r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                divider_color,
                f(lane_lefts[lane_idx]),
                f(lane_lefts[lane_idx]),
                f(header_top),
                f(final_last_y),
            )
            .unwrap();
        }
        prev_lane_idx = Some(lane_idx);
    }

    // Now flush the deferred cross-lane arrows to the connectors buffer.
    // These appear AFTER all per-lane internal arrows in the SVG, matching
    // PlantUML's emission order.
    for &(prev_cx, prev_y, lane_cx, target_y) in &deferred_cross_lanes {
        let cross_y = prev_y + 5.0;
        svg.line_styled(&arrow_color, "1", prev_cx, prev_cx, prev_y, cross_y, false);
        svg.line_styled(&arrow_color, "1", prev_cx, lane_cx, cross_y, cross_y, false);
        svg.line_styled(
            &arrow_color,
            "1",
            lane_cx,
            lane_cx,
            cross_y,
            target_y,
            false,
        );
        svg.polygon_connector(
            &arrow_color,
            &[
                (lane_cx - 4.0, target_y - 10.0),
                (lane_cx, target_y),
                (lane_cx + 4.0, target_y - 10.0),
                (lane_cx, target_y - 6.0),
            ],
            &arrow_color,
            "1",
        );
    }

    // After the last lane: emit its LEFT divider, then the final RIGHT
    // divider. Both use final_last_y.
    if let Some(last_idx) = lanes.len().checked_sub(1) {
        write!(
            svg.shapes,
            r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            divider_color,
            f(lane_lefts[last_idx]),
            f(lane_lefts[last_idx]),
            f(header_top),
            f(final_last_y),
        )
        .unwrap();
        write!(
            svg.shapes,
            r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            divider_color,
            f(right_edge),
            f(right_edge),
            f(header_top),
            f(final_last_y),
        )
        .unwrap();
    }

    // Lane titles emitted LAST (after all connectors). Titles are centred
    // on the LANE'S GEOMETRIC MID (lane_left + lane_w/2), NOT the content
    // cx — these differ when content is anchored off-centre in its lane
    // (which it is, since content sits at lane_left + 6 + content_left).
    let title_baseline = header_top + pm::ascent(LANE_TITLE_FONT);
    let _ = lane_cxs; // content cx is used for arrows, not titles
    for (i, lane) in lanes.iter().enumerate() {
        if lane.name.is_empty() {
            continue;
        }
        let lane_mid = lane_lefts[i] + lane_widths[i] / 2.0;
        let tw = text_render::measure(&lane.name, LANE_TITLE_FONT, false);
        std::mem::swap(&mut svg.shapes, &mut svg.connectors);
        svg.text_element(
            &text_color,
            "sans-serif",
            LANE_TITLE_FONT,
            tw,
            lane_mid - tw / 2.0,
            title_baseline,
            &lane.name,
            false,
        );
        std::mem::swap(&mut svg.shapes, &mut svg.connectors);
    }

    last_y
}

/// Render an activity diagram with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, the renderer replays
/// the body verbatim inside the PlantUML envelope. Otherwise it falls back
/// to the geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &ActivityDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "ACTIVITY");
    }
    render(diagram, theme)
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
    // A leading `floating note` grows the start node's tile to the note's
    // height (it is centred on the note). When the note is taller than the
    // start ellipse, the spine is pushed down by `note_h - 2*START_R`. The
    // start node already contributes its own `2*START_R`-equivalent tile to
    // sequence_height, so we add only the surplus here.
    let lead_note = leading_floating_note(&tree, diagram.meta.source.as_deref());
    let lead_note_h = lead_note.as_ref().and_then(|(text, _, _)| {
        let h = note_box_height(text);
        (h > 2.0 * START_R).then_some(h)
    });
    let content_h = sequence_height(&tree) + lead_note_h.map_or(0.0, |h| h - 2.0 * START_R);

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
    // A top-level swimlanes block carries its own internal top structure
    // (header band at MARGIN_LEAD + 1.2969 = 17.2969). The generic
    // deprecation gap formula (13 + warn_band_h + 17) is calibrated for the
    // flat-flow layout whose natural top is MARGIN_LEAD; applying it to a
    // swimlane double-counts. Instead, each deprecation banner pushes the
    // swimlane down by exactly one baseline pitch (warn_h_each + 5 =
    // 21.6406): golden header_top = 17.2969 + num_warnings * 21.6406 (e.g.
    // 38.9375 for one warning), so the content origin handed to
    // emit_swimlanes is MARGIN_LEAD + num_warnings * 21.6406 (the +1.2969 is
    // re-added inside emit_swimlanes).
    let is_top_swimlanes = matches!(tree.first(), Some(LayoutNode::Swimlanes { .. }));
    let start_y = if is_top_swimlanes {
        margin_top + num_warnings * (warn_h_each + 5.0)
    } else if has_deprecated {
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

    // A leading floating note is drawn first (before the start ellipse) so
    // its paths/text precede the spine in document order, matching PlantUML.
    if let (Some((text, position, color)), Some(_)) = (&lead_note, lead_note_h) {
        emit_leading_floating_note(&mut svg, text, position, color.as_deref(), cx);
    }

    // Emit all nodes.
    emit_sequence_ex(&mut svg, &tree, cx, start_y, None, lead_note_h);

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
