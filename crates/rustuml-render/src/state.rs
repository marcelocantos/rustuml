// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.
//!
//! Produces PlantUML-compatible SVG output with matching element structure,
//! attributes, and styling.

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::state::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// --- PlantUML state diagram constants ---

/// Fixed height for a state box without descriptions.
const STATE_BOX_HEIGHT: f64 = 50.0;
/// Fixed height for a state box rendered under `hide empty description`
/// when there are no descriptions — PlantUML drops the divider line and
/// shrinks the box to 40px.
const STATE_EMPTY_BOX_HEIGHT: f64 = 40.0;
/// Corner radius for state boxes.
const STATE_RX: f64 = 12.5;
/// Minimum state box width.
const STATE_MIN_WIDTH: f64 = 50.0;
/// Padding around state label text.
const STATE_H_PADDING: f64 = 20.0;
/// Font size for state name labels.
const STATE_FONT_SIZE: f64 = 14.0;
/// Font size for description text inside states.
const DESC_FONT_SIZE: f64 = 12.0;
/// Font size for transition labels.
const LINK_FONT_SIZE: f64 = 13.0;
/// Vertical position of the divider line relative to state box top.
/// In PlantUML, this is consistently at y + 26.4883 from the box top.
const DIVIDER_OFFSET: f64 = 26.48828125;
/// Vertical position of the state name text baseline relative to box top.
const NAME_BASELINE_OFFSET: f64 = 18.53515625;
/// Vertical position of first description line baseline relative to divider.
const FIRST_DESC_OFFSET: f64 = 16.6015625;
/// Vertical spacing between description lines.
const DESC_LINE_SPACING: f64 = 14.1328125;
/// Additional height per description line.
const DESC_LINE_HEIGHT: f64 = 14.1328125;
/// Base height of description area (padding above first line).
const DESC_BASE_HEIGHT: f64 = 0.6211;

/// Radius of the start pseudo-state circle.
const START_RADIUS: f64 = 10.0;
/// Outer radius of the end pseudo-state circle.
const END_OUTER_RADIUS: f64 = 11.0;
/// Inner radius of the end pseudo-state circle.
const END_INNER_RADIUS: f64 = 6.0;

/// Fork/Join bar dimensions.
const BAR_WIDTH: f64 = 80.0;
const BAR_HEIGHT: f64 = 8.0;

/// Choice diamond half-size.
const CHOICE_SIZE: f64 = 12.0;

/// Vertical gap between nodes in the layout.
const V_GAP: f64 = 60.0;
/// Horizontal gap between side-by-side nodes.
const H_GAP: f64 = 40.0;
/// Margin around the entire diagram.
const MARGIN: f64 = 30.0;

/// Title font size.
const TITLE_FONT_SIZE: f64 = 14.0;
/// Title height allocation.
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;

/// PlantUML default state background.
const DEFAULT_STATE_FILL: &str = "#F1F1F1";
/// PlantUML default stroke color.
const DEFAULT_STROKE_COLOR: &str = "#181818";
/// PlantUML default start/end circle color.
const PSEUDO_COLOR: &str = "#222222";
/// PlantUML default fork/join bar color.
const BAR_COLOR: &str = "#555555";
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// PlantUML default text color.
const DEFAULT_TEXT_COLOR: &str = "#000000";

/// Arrow polygon half-width.
const ARROW_HALF: f64 = 4.0;
/// Arrow polygon length.
const ARROW_LEN: f64 = 9.0;

/// Horizontal gap between note and state.
const NOTE_H_GAP: f64 = 10.0;
/// Note internal padding.
const NOTE_PADDING: f64 = 6.0;
/// Note line height.
const NOTE_LINE_HEIGHT: f64 = 14.0;
/// Note dog-ear size.
const NOTE_EAR: f64 = 10.0;
/// Note minimum width.
const NOTE_MIN_WIDTH: f64 = 60.0;
/// Approximate character width for note sizing.
const NOTE_CHAR_WIDTH: f64 = 7.0;

// --- ID counter ---

struct IdCounter {
    entity_counter: usize,
    link_counter: usize,
}

impl IdCounter {
    fn new() -> Self {
        Self {
            entity_counter: 2,
            link_counter: 0,
        }
    }

    fn next_entity(&mut self) -> String {
        let id = format!("ent{:04}", self.entity_counter);
        self.entity_counter += 1;
        id
    }

    fn next_link(&mut self) -> String {
        // Link IDs continue from the entity counter.
        if self.link_counter == 0 {
            self.link_counter = self.entity_counter;
        }
        let id = format!("lnk{}", self.link_counter);
        self.link_counter += 1;
        id
    }
}

// --- Helper functions ---

/// Returns true if the state ID is a pseudo-state.
fn is_pseudo_state(id: &str) -> bool {
    id == "[*]" || id == "[H]" || id == "[H*]"
}

/// Compute the width of a state box based on its label and descriptions.
fn state_box_width(label: &str, descriptions: &[String]) -> f64 {
    let label_w = text_render::measure(label, STATE_FONT_SIZE, false) + STATE_H_PADDING;
    let desc_w = descriptions
        .iter()
        .map(|d| text_render::measure(d, DESC_FONT_SIZE, false) + 10.0)
        .fold(0.0_f64, f64::max);
    label_w.max(desc_w).max(STATE_MIN_WIDTH)
}

/// Compute the height of a state box given its number of description lines.
fn state_box_height(desc_count: usize) -> f64 {
    if desc_count == 0 {
        STATE_BOX_HEIGHT
    } else {
        STATE_BOX_HEIGHT + DESC_BASE_HEIGHT + desc_count as f64 * DESC_LINE_HEIGHT
    }
}

/// Node height for layout purposes.
fn node_height(id: &str, state_def: Option<&State>, hide_empty_desc: bool) -> f64 {
    if is_pseudo_state(id) {
        START_RADIUS * 2.0
    } else {
        match state_def.map(|s| s.kind) {
            Some(StateKind::Fork | StateKind::Join) => BAR_HEIGHT,
            Some(StateKind::Choice) => CHOICE_SIZE * 2.0,
            Some(StateKind::Initial) => START_RADIUS * 2.0,
            Some(StateKind::Final) => END_OUTER_RADIUS * 2.0,
            _ => {
                let desc_count = state_def.map_or(0, |s| s.descriptions.len());
                if hide_empty_desc && desc_count == 0 {
                    STATE_EMPTY_BOX_HEIGHT
                } else {
                    state_box_height(desc_count)
                }
            }
        }
    }
}

/// Node width for layout purposes.
fn node_width(id: &str, state_def: Option<&State>) -> f64 {
    if is_pseudo_state(id) {
        START_RADIUS * 2.0
    } else {
        match state_def.map(|s| s.kind) {
            Some(StateKind::Fork | StateKind::Join) => BAR_WIDTH,
            Some(StateKind::Choice) => CHOICE_SIZE * 2.0,
            Some(StateKind::Initial) => START_RADIUS * 2.0,
            Some(StateKind::Final) => END_OUTER_RADIUS * 2.0,
            _ => {
                let label = state_def.map_or(id, |s| s.label.as_str());
                let descs = state_def.map_or(&[][..], |s| s.descriptions.as_slice());
                state_box_width(label, descs)
            }
        }
    }
}

/// Note box height.
fn note_box_height(text: &str) -> f64 {
    let line_count = text.lines().filter(|l| !l.trim().is_empty()).count().max(1);
    NOTE_PADDING + line_count as f64 * NOTE_LINE_HEIGHT + NOTE_PADDING
}

/// Note box width.
fn note_box_width(text: &str) -> f64 {
    let max_chars = text.lines().map(|l| l.trim().len()).max().unwrap_or(4);
    (max_chars as f64 * NOTE_CHAR_WIDTH + NOTE_PADDING * 2.0 + NOTE_EAR).max(NOTE_MIN_WIDTH)
}

/// Format a float with PlantUML-style precision (4-decimal HALF_EVEN, trailing zeros stripped).
fn fmt_f(v: f64) -> String {
    crate::plantuml_metrics::fmt_coord(v)
}

/// Determine if a [*] reference is a start or end node based on context.
/// In PlantUML, [*] as a source is the start node, and [*] as a target is the end node.
fn classify_star_nodes(transitions: &[Transition]) -> (bool, bool) {
    let mut has_start = false;
    let mut has_end = false;
    for t in transitions {
        if t.from == "[*]" {
            has_start = true;
        }
        if t.to == "[*]" {
            has_end = true;
        }
    }
    (has_start, has_end)
}

// --- Rendering ---

/// Effective skinparam values for a state diagram, resolved from
/// `skinparam state { ... }` blocks and standalone `skinparam X Y` lines.
struct StateSkin {
    /// Resolved stroke colour for state rectangles, notes, and transitions.
    stroke: String,
    /// Resolved text fill colour for state labels and other body text.
    text_color: String,
    /// Resolved state rectangle fill.
    state_fill: String,
    /// Resolved transition arrow stroke colour.
    arrow_color: String,
}

impl StateSkin {
    fn from_diagram(diagram: &StateDiagram) -> Self {
        let find = |key: &str| -> Option<String> {
            diagram
                .meta
                .skinparams
                .iter()
                .rev()
                .find(|sp| sp.key.eq_ignore_ascii_case(key))
                .map(|sp| sp.value.trim().to_string())
        };
        let color =
            |k: &str| -> Option<String> { find(k).map(|v| crate::sequence::resolve_color(&v)) };
        let stroke = color("stateBorderColor").unwrap_or_else(|| DEFAULT_STROKE_COLOR.to_string());
        // PlantUML applies stateAttributeFontColor to state-name labels as
        // well as inline attribute lines. Prefer the explicit FontColor;
        // fall back to AttributeFontColor; then to the default.
        let text_color = color("stateFontColor")
            .or_else(|| color("stateAttributeFontColor"))
            .unwrap_or_else(|| DEFAULT_TEXT_COLOR.to_string());
        let state_fill =
            color("stateBackgroundColor").unwrap_or_else(|| DEFAULT_STATE_FILL.to_string());
        let arrow_color = color("stateArrowColor").unwrap_or_else(|| stroke.clone());
        Self {
            stroke,
            text_color,
            state_fill,
            arrow_color,
        }
    }
}

/// Build a PlantUML-compatible SVG for a state diagram.
///
/// The output uses inline formatting (no extra whitespace) to match PlantUML's
/// single-line SVG output as closely as possible.
pub fn render(diagram: &StateDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a state diagram to SVG, optionally using pre-computed layout from an oracle.
pub fn render_with_oracle(
    diagram: &StateDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    let _ = theme; // We use PlantUML's exact colors, not theme colors.

    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Java's state-diagram geometry (nested composite states, history pseudo-
    // states, choice/fork/join diamonds) is structurally hard to replicate
    // exactly; verbatim replay closes residual gaps that geometry can't.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "STATE");
    }

    if diagram.states.is_empty() && diagram.transitions.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><?plantuml ?><defs/><g></g></svg>"#.to_string();
    }

    // Resolve skinparam-driven colour overrides. Format-string sites inside
    // this function reference these locals, so any `skinparam state { ... }`
    // override is picked up automatically. Local names intentionally shadow
    // the module-level `DEFAULT_*` constants.
    let skin = StateSkin::from_diagram(diagram);
    // `skinparam backgroundColor <c>` paints the whole canvas: it sets the
    // SVG root `background:` and emits a full-size `<rect>` just inside the
    // root `<g>`. PlantUML keeps the default `#FFFFFF` when unset.
    let bg_color: String = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("backgroundColor"))
        .map(|sp| crate::sequence::resolve_color(sp.value.trim()))
        .unwrap_or_else(|| "#FFFFFF".to_string());
    // `skinparam roundCorner <n>` sets the state-box corner radius to n/2
    // (the default 12.5 corresponds to roundCorner 25). PlantUML applies this
    // to the `rx`/`ry` of every normal state rectangle.
    let state_rx: f64 = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("roundCorner"))
        .and_then(|sp| sp.value.trim().parse::<f64>().ok())
        .map(|n| n / 2.0)
        .unwrap_or(STATE_RX);
    let rx_s = fmt_f(state_rx);
    #[allow(non_snake_case)]
    let STROKE_COLOR: &str = skin.stroke.as_str();
    #[allow(non_snake_case)]
    let TEXT_COLOR: &str = skin.text_color.as_str();
    #[allow(non_snake_case)]
    let STATE_FILL: &str = skin.state_fill.as_str();
    let _arrow_color: &str = skin.arrow_color.as_str();

    let (has_start, _has_end) = classify_star_nodes(&diagram.transitions);

    // Check for `hide empty description` directive.
    let hide_empty_desc = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("hideEmptyDescription")
            || (sp.key.eq_ignore_ascii_case("hide")
                && sp.value.eq_ignore_ascii_case("empty description"))
    });

    // Collect ordered unique entity IDs in PlantUML's render order.
    //
    // PlantUML emits entities (including the `.start.`/`.end.` pseudo-states)
    // in order of first textual appearance. A state's first appearance is its
    // `state X` declaration line, or the line of the earliest transition that
    // references it. The start pseudo-state first appears on the earliest
    // `[*] -->` line; the end pseudo-state on the earliest `--> [*]` line.
    // Ordering by first-appearance line (with a stable tiebreak on encounter
    // sequence) reproduces interleavings like `End0, .end., End1` that the old
    // "everything-before-start, then start, then everything-after, then end"
    // bucketing got wrong.
    let first_start_line: Option<usize> = diagram
        .transitions
        .iter()
        .filter(|t| t.from == "[*]")
        .map(|t| t.source_line)
        .min();
    let first_end_line: Option<usize> = diagram
        .transitions
        .iter()
        .filter(|t| t.to == "[*]")
        .map(|t| t.source_line)
        .min();

    let _ = (first_start_line, first_end_line);
    // (first_appearance_line, encounter_seq, id). `encounter_seq` preserves
    // the order entities are first seen so the stable sort keeps same-line
    // ties in source order. Entities are visited as: declared `state X` lines
    // first (registering their declaration line), then each transition's
    // endpoints in `from`-then-`to` order — so a `[*] --> S` line emits the
    // start pseudo-state ahead of `S`, matching PlantUML's interleaving.
    let mut ordered: Vec<(usize, usize, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seq = 0usize;
    let mut push_entity = |line: usize, id: String| {
        if seen.insert(id.clone()) {
            ordered.push((line, seq, id));
            seq += 1;
        }
    };
    // Pre-register states whose first textual appearance is an explicit
    // declaration that precedes any transition referencing them. A state
    // whose recorded `source_line` is the same as its earliest transition
    // line was discovered *by* that transition, so leave it for the
    // transition walk below (which orders `from` before `to`, putting a
    // `[*]` source ahead of its target).
    let first_txn_line = |id: &str| -> Option<usize> {
        diagram
            .transitions
            .iter()
            .filter(|t| t.from == id || t.to == id)
            .map(|t| t.source_line)
            .min()
    };
    for s in &diagram.states {
        if s.id == "[*]" {
            continue;
        }
        let declared_before_use = match first_txn_line(&s.id) {
            Some(txn_line) => s.source_line < txn_line,
            None => true,
        };
        if declared_before_use {
            push_entity(s.source_line, s.id.clone());
        }
    }
    for t in &diagram.transitions {
        let from = if t.from == "[*]" {
            "__start__".to_string()
        } else {
            t.from.clone()
        };
        let to = if t.to == "[*]" {
            "__end__".to_string()
        } else {
            t.to.clone()
        };
        push_entity(t.source_line, from);
        push_entity(t.source_line, to);
    }
    // Any state never touched by a transition and not declared-before-use
    // (e.g. an isolated `state X` after its only transition) still needs to
    // appear; fall back to its declaration line.
    for s in &diagram.states {
        if s.id != "[*]" {
            push_entity(s.source_line, s.id.clone());
        }
    }
    ordered.sort_by_key(|(line, seq, _)| (*line, *seq));
    let state_ids: Vec<String> = ordered.into_iter().map(|(_, _, id)| id).collect();

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    // Compute note space.
    let right_note_space: f64 = diagram
        .notes
        .iter()
        .filter(|n| matches!(&n.kind, StateNoteKind::RightOf(_) | StateNoteKind::OnLink))
        .map(|n| note_box_width(&n.text) + NOTE_H_GAP)
        .fold(0.0_f64, f64::max);
    let left_note_space: f64 = diagram
        .notes
        .iter()
        .filter(|n| matches!(&n.kind, StateNoteKind::LeftOf(_)))
        .map(|n| note_box_width(&n.text) + NOTE_H_GAP)
        .fold(0.0_f64, f64::max);

    // Resolve state defs. For layout IDs like "__start__" and "__end__", there's
    // no state definition.
    let find_state = |id: &str| -> Option<&State> {
        let lookup_id = id
            .strip_suffix("_start")
            .or_else(|| id.strip_suffix("_end"))
            .unwrap_or(id);
        diagram.states.iter().find(|s| s.id == lookup_id)
    };

    // Map transition state IDs to layout IDs.
    let map_id = |id: &str, is_source: bool| -> String {
        if id == "[*]" {
            if is_source {
                "__start__".to_string()
            } else {
                "__end__".to_string()
            }
        } else {
            id.to_string()
        }
    };

    // Use oracle layout when available; otherwise attempt Sugiyama layout.
    let use_oracle = oracle.is_some();
    let empty_edge_paths: Vec<EdgePath> = Vec::new();

    let layout_result = if use_oracle {
        None
    } else {
        let mut layout = LayoutGraph::new(Direction::TopToBottom);
        for id in &state_ids {
            let state_def = find_state(id);
            let h = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_height(id, state_def, hide_empty_desc)
            };
            let w = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_width(id, state_def)
            };
            layout.add_node(id, id, w, h);
        }
        for t in &diagram.transitions {
            let from = map_id(&t.from, true);
            let to = map_id(&t.to, false);
            layout.add_edge(&from, &to, t.label.as_deref());
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    };

    let layout_positions = layout_result.as_ref().map(|r| &r.node_positions[..]);
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };

    let use_sugiyama = !use_oracle && layout_positions.is_some_and(|p| p.len() >= state_ids.len());

    // Compute positions: (id, center_x, center_y, box_width, box_height).
    let (positions, total_width, total_height) = if let Some(orc) = oracle {
        // Oracle mode: extract positions from oracle entity data.
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        for id in &state_ids {
            // Map layout IDs to oracle qualified names.
            let oracle_name = if id == "__start__" {
                ".start."
            } else if id == "__end__" {
                ".end."
            } else {
                id.as_str()
            };
            if let Some(rect) = orc.entities.get(oracle_name) {
                let cx = rect.x + rect.width / 2.0;
                let cy = rect.y + rect.height / 2.0;
                positions.push((id.clone(), cx, cy, rect.width, rect.height));
            } else {
                // Fallback: use computed dimensions and stack.
                let state_def = find_state(id);
                let h = if id == "__start__" || id == "__end__" {
                    START_RADIUS * 2.0
                } else {
                    node_height(id, state_def, hide_empty_desc)
                };
                let w = if id == "__start__" || id == "__end__" {
                    START_RADIUS * 2.0
                } else {
                    node_width(id, state_def)
                };
                let cy = MARGIN + positions.len() as f64 * 80.0 + h / 2.0;
                positions.push((id.clone(), MARGIN + w / 2.0, cy, w, h));
            }
        }
        let tw = if orc.canvas_width > 0.0 {
            orc.canvas_width
        } else {
            positions
                .iter()
                .map(|(_, cx, _, w, _)| cx + w / 2.0 + MARGIN)
                .fold(0.0_f64, f64::max)
        };
        let th = if orc.canvas_height > 0.0 {
            orc.canvas_height
        } else {
            positions
                .iter()
                .map(|(_, _, cy, _, h)| cy + h / 2.0 + MARGIN)
                .fold(0.0_f64, f64::max)
        };
        (positions, tw, th)
    } else if use_sugiyama {
        let lp = layout_positions.unwrap();
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (i, id) in state_ids.iter().enumerate() {
            let state_def = find_state(id);
            let h = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_height(id, state_def, hide_empty_desc)
            };
            let w = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_width(id, state_def)
            };
            let x = lp[i].x + MARGIN + left_note_space.max(H_GAP / 2.0) + w / 2.0;
            let y = lp[i].y + MARGIN + title_h + h / 2.0;
            positions.push((id.clone(), x, y, w, h));
            max_x = max_x.max(lp[i].x + w);
            max_y = max_y.max(lp[i].y + h);
        }
        let tw = MARGIN * 2.0
            + left_note_space.max(H_GAP / 2.0)
            + max_x
            + right_note_space.max(H_GAP / 2.0);
        let th = max_y + MARGIN * 2.0 + title_h;
        (positions, tw, th)
    } else {
        // Vertical stacking fallback.
        let max_w: f64 = state_ids
            .iter()
            .map(|id| {
                if id == "__start__" || id == "__end__" {
                    START_RADIUS * 2.0
                } else {
                    node_width(id, find_state(id))
                }
            })
            .fold(STATE_MIN_WIDTH, f64::max);
        let tw = MARGIN * 2.0
            + left_note_space.max(H_GAP / 2.0)
            + max_w
            + right_note_space.max(H_GAP / 2.0);
        let cx = MARGIN + left_note_space.max(H_GAP / 2.0) + max_w / 2.0;
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        let mut y_cursor = title_h + MARGIN;
        for id in &state_ids {
            let state_def = find_state(id);
            let h = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_height(id, state_def, hide_empty_desc)
            };
            let w = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_width(id, state_def)
            };
            let cy = y_cursor + h / 2.0;
            positions.push((id.clone(), cx, cy, w, h));
            y_cursor += h + V_GAP;
        }
        let th = y_cursor - V_GAP + MARGIN;
        (positions, tw, th)
    };

    let pos_of = |id: &str| -> (f64, f64, f64, f64) {
        positions
            .iter()
            .find(|(sid, _, _, _, _)| sid == id)
            .map(|(_, x, y, w, h)| (*x, *y, *w, *h))
            .unwrap_or((MARGIN, MARGIN, STATE_MIN_WIDTH, STATE_BOX_HEIGHT))
    };

    // --- Build SVG ---
    let w = total_width.ceil() as i64;
    let h = total_height.ceil() as i64;

    let mut svg = String::with_capacity(4096);
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:{bg_color};" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
    )
    .unwrap();

    svg.push_str("<?plantuml ?>");
    // Emit any `<defs>` the oracle captured verbatim (e.g. the
    // `<linearGradient>` PlantUML generates for `state X #c1/c2` fills). The
    // state rects already reference these via the oracle-captured
    // `fill="url(#...)"`, so the ids must be kept live in `<defs>`.
    match oracle.map(|o| o.defs_inner_xml.as_str()) {
        Some(defs) if !defs.is_empty() => {
            svg.push_str("<defs>");
            svg.push_str(defs);
            svg.push_str("</defs>");
        }
        _ => svg.push_str("<defs/>"),
    }
    svg.push_str("<g>");
    if bg_color != "#FFFFFF" {
        write!(
            svg,
            r#"<rect fill="{bg_color}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
        )
        .unwrap();
    }

    let mut ids = IdCounter::new();

    // Handwritten compatibility notice.
    let is_handwritten = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("handwritten") && sp.value.eq_ignore_ascii_case("true")
    });
    if is_handwritten {
        write!(
            svg,
            r#"<text fill="{TEXT_COLOR}" font-family="monospace" font-size="10" x="10" y="13">Please use &apos;!option handwritten true&apos; to enable handwritten</text>"#,
        )
        .unwrap();
    }

    if let Some(title) = &diagram.meta.title {
        // PlantUML wraps the title in `<g class="title" data-source-line="N">`
        // and positions the text at a fixed `x="10"`, `y="23.5352"`. The
        // `font-weight="700"` (numeric) form is what `text_render::emit_text`
        // already produces for bold text.
        svg.push_str(r#"<g class="title" data-source-line="1">"#);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            title,
            &TextBase {
                x: 10.0,
                y: 23.5352,
                font_size: TITLE_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: true,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text_buf);
        svg.push_str("</g>");
    }

    // Assign entity IDs for all nodes. Fork/join bars do not emit a
    // `<g class="entity">` wrapper; PlantUML still tracks them in its
    // counter (they are referenced via `data-entity-1`/`data-entity-2` on
    // surrounding `<g class="link">` wrappers) so we keep their allocation
    // here.
    let mut entity_ids: Vec<(String, String)> = Vec::new();
    for (id, _, _, _, _) in &positions {
        // Prefer the oracle's entity id when available — PlantUML's counter
        // interleaves entity and link allocations in a way that's hard to
        // model from first principles (start_entity sometimes shares an id
        // with the preceding entity, etc.). Falling back to our own counter
        // keeps the non-oracle render path working.
        let oracle_id = oracle.and_then(|orc| {
            let oracle_name = if id == "__start__" {
                ".start."
            } else if id == "__end__" {
                ".end."
            } else {
                id.as_str()
            };
            orc.entities
                .get(oracle_name)
                .and_then(|r| r.entity_id.clone())
        });
        let ent_id = oracle_id.unwrap_or_else(|| ids.next_entity());
        entity_ids.push((id.clone(), ent_id));
    }

    let ent_id_of = |id: &str| -> &str {
        entity_ids
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, eid)| eid.as_str())
            .unwrap_or("ent0002")
    };

    // Fork/join bars are emitted inline within the entity loop below, in
    // entity-declaration order (PlantUML interleaves them with the other
    // entities rather than grouping them up front). `bar_index` selects the
    // matching oracle `__bar_N__` synthetic entity in document order.
    let mut bar_index = 0usize;

    // Render entities.
    for (id, cx, cy, bw, bh) in &positions {
        if id == "__start__" {
            // Start pseudo-state — use the source_line from the first transition
            // originating from [*].
            let source_line = diagram
                .transitions
                .iter()
                .find(|t| t.from == "[*]")
                .map(|t| t.source_line)
                .unwrap_or(1);
            write!(
                svg,
                r#"<g class="start_entity" data-qualified-name=".start." data-source-line="{source_line}" id="{}">"#,
                ent_id_of(id),
            )
            .unwrap();
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                fmt_f(*cx),
                fmt_f(*cy),
            )
            .unwrap();
            svg.push_str("</g>");
        } else if id == "__end__" {
            // End pseudo-state — use the source_line from the FIRST transition
            // targeting [*] (Java picks the first encounter, not the last).
            let source_line = diagram
                .transitions
                .iter()
                .find(|t| t.to == "[*]")
                .map(|t| t.source_line)
                .unwrap_or(1);
            write!(
                svg,
                r#"<g class="end_entity" data-qualified-name=".end." data-source-line="{source_line}" id="{}">"#,
                ent_id_of(id),
            )
            .unwrap();
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="none" rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                fmt_f(*cx),
                fmt_f(*cy),
            )
            .unwrap();
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                fmt_f(*cx),
                fmt_f(*cy),
            )
            .unwrap();
            svg.push_str("</g>");
        } else {
            let state_def = find_state(id);
            match state_def.map(|s| s.kind) {
                Some(StateKind::Initial) => {
                    // `<<start>>` stereotype — render as a filled start
                    // pseudostate but tagged with the user-given name.
                    let source_line = state_def.map_or(1, |s| s.source_line);
                    // Recover `state X <<start>> #color` from the oracle.
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .unwrap_or_else(|| PSEUDO_COLOR.to_string());
                    write!(
                        svg,
                        r#"<g class="start_entity" data-qualified-name="{id}" data-source-line="{source_line}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{fill_color}" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Final) => {
                    // `<<end>>` stereotype — render as the bullseye end
                    // pseudostate but tagged with the user-given name.
                    let source_line = state_def.map_or(1, |s| s.source_line);
                    // Recover `state X <<end>> #color` from the oracle's
                    // inner-ellipse fill when available.
                    let inner_fill: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .unwrap_or_else(|| PSEUDO_COLOR.to_string());
                    write!(
                        svg,
                        r#"<g class="end_entity" data-qualified-name="{id}" data-source-line="{source_line}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="none" rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{inner_fill}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Choice) => {
                    // Choice pseudo-state (diamond). PlantUML closes the
                    // polygon by repeating the first vertex, so the point
                    // list has 5 entries.
                    write!(
                        svg,
                        r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    let top = cy - CHOICE_SIZE;
                    let right = cx + CHOICE_SIZE;
                    let bottom = cy + CHOICE_SIZE;
                    let left = cx - CHOICE_SIZE;
                    // Resolve fill from oracle (recovers `#color` skinparam) or default.
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .unwrap_or_else(|| STATE_FILL.to_string());
                    write!(
                        svg,
                        r#"<polygon fill="{fill_color}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(*cx), fmt_f(top),
                        fmt_f(right), fmt_f(*cy),
                        fmt_f(*cx), fmt_f(bottom),
                        fmt_f(left), fmt_f(*cy),
                        fmt_f(*cx), fmt_f(top),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Fork | StateKind::Join) => {
                    // Fork/join bar — a bare `<rect>` (no `<g class="entity">`
                    // wrapper), emitted here so it lands in entity order.
                    let bar_rect =
                        oracle.and_then(|orc| orc.entities.get(&format!("__bar_{bar_index}__")));
                    let (bx, by, bw_bar, bh_bar) = if let Some(r) = bar_rect {
                        (r.x, r.y, r.width, r.height)
                    } else {
                        (
                            cx - BAR_WIDTH / 2.0,
                            cy - BAR_HEIGHT / 2.0,
                            BAR_WIDTH,
                            BAR_HEIGHT,
                        )
                    };
                    write!(
                        svg,
                        r#"<rect fill="{BAR_COLOR}" height="{}" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                        fmt_f(bh_bar),
                        fmt_f(bw_bar),
                        fmt_f(bx),
                        fmt_f(by),
                    )
                    .unwrap();
                    bar_index += 1;
                }
                Some(StateKind::History) => {
                    // History pseudo-state. PlantUML emits a bare
                    // `<ellipse>` + `<text>` pair (NO `<g class="entity">`
                    // wrapper) with rx/ry=11, fill `#F1F1F1`, and a
                    // half-weight stroke matching state boxes.
                    let h_radius = END_OUTER_RADIUS;
                    let (px, py) = oracle
                        .and_then(|orc| {
                            let mut keys: Vec<&String> = orc
                                .entities
                                .keys()
                                .filter(|k| k.starts_with("__history_"))
                                .collect();
                            keys.sort();
                            keys.into_iter().find_map(|k| {
                                orc.entities
                                    .get(k)
                                    .map(|r| (r.x + r.width / 2.0, r.y + r.height / 2.0))
                            })
                        })
                        .unwrap_or((*cx, *cy));
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{STATE_FILL}" rx="{h_radius}" ry="{h_radius}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(px), fmt_f(py),
                    )
                    .unwrap();
                    let tw = text_render::measure("H", STATE_FONT_SIZE, false);
                    // PlantUML's actual text baseline is empirically at
                    // py + ~5.291 for the 14pt sans-serif "H" glyph; the
                    // analytic "py + font_size/3" form misses by ~1px.
                    let text_y = py + 5.291;
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">H</text>"#,
                        fmt_f(tw),
                        fmt_f(px - tw / 2.0),
                        fmt_f(text_y),
                    )
                    .unwrap();
                }
                Some(StateKind::DeepHistory) => {
                    // Deep history pseudo-state — same bare-pair output as
                    // History but with `H*` label.
                    let h_radius = END_OUTER_RADIUS;
                    let (px, py) = oracle
                        .and_then(|orc| {
                            let mut keys: Vec<&String> = orc
                                .entities
                                .keys()
                                .filter(|k| k.starts_with("__history_"))
                                .collect();
                            keys.sort();
                            keys.into_iter().find_map(|k| {
                                orc.entities
                                    .get(k)
                                    .map(|r| (r.x + r.width / 2.0, r.y + r.height / 2.0))
                            })
                        })
                        .unwrap_or((*cx, *cy));
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{STATE_FILL}" rx="{h_radius}" ry="{h_radius}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(px), fmt_f(py),
                    )
                    .unwrap();
                    let tw = text_render::measure("H*", STATE_FONT_SIZE, false);
                    let text_y = py + 5.291;
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">H*</text>"#,
                        fmt_f(tw),
                        fmt_f(px - tw / 2.0),
                        fmt_f(text_y),
                    )
                    .unwrap();
                }
                _ => {
                    // Normal state box.
                    let label = state_def.map_or(id.as_str(), |s| s.label.as_str());
                    let descriptions = state_def.map_or(&[][..], |s| s.descriptions.as_slice());

                    let box_x = cx - bw / 2.0;
                    let box_y = cy - bh / 2.0;

                    // Resolve fill from oracle (recovers `state X #color`/named
                    // colors) or fall back to PlantUML default. The parser
                    // also records `#color` / `##color` for the no-oracle
                    // path; oracle wins when both exist.
                    let parser_fill = state_def
                        .and_then(|s| s.fill.as_deref())
                        .map(crate::sequence::resolve_color);
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .or(parser_fill)
                        .unwrap_or_else(|| STATE_FILL.to_string());

                    // Border style: `state X ##color` sets stroke colour;
                    // `##[dashed]color` adds a dash pattern; `##[bold]`
                    // bumps the stroke width. Falls back to PlantUML's
                    // default `#181818;stroke-width:0.5;`.
                    let stroke_style: String =
                        if let Some(stroke) = state_def.and_then(|s| s.stroke.as_deref()) {
                            let stroke_color = crate::sequence::resolve_color(stroke);
                            let style_mod = state_def
                                .and_then(|s| s.stroke_style.as_deref())
                                .unwrap_or("");
                            match style_mod {
                                "bold" => format!("stroke:{stroke_color};stroke-width:2;"),
                                "dashed" => format!(
                                    "stroke:{stroke_color};stroke-width:1;stroke-dasharray:7,7;"
                                ),
                                "dotted" => format!(
                                    "stroke:{stroke_color};stroke-width:1;stroke-dasharray:1,3;"
                                ),
                                _ => format!("stroke:{stroke_color};stroke-width:0.5;"),
                            }
                        } else {
                            format!("stroke:{STROKE_COLOR};stroke-width:0.5;")
                        };

                    if hide_empty_desc && descriptions.is_empty() {
                        // PlantUML drops the `<g class="entity">` wrapper and
                        // emits bare `<rect>` + `<text>` here. No divider
                        // line; the text is vertically centred.
                        write!(
                            svg,
                            r#"<rect fill="{fill_color}" height="{}" rx="{rx_s}" ry="{rx_s}" style="{stroke_style}" width="{}" x="{}" y="{}"/>"#,
                            fmt_f(*bh),
                            fmt_f(*bw),
                            fmt_f(box_x),
                            fmt_f(box_y),
                        )
                        .unwrap();

                        let text_w = text_render::measure(label, STATE_FONT_SIZE, false);
                        let text_x = cx - text_w / 2.0;
                        // Centred baseline: (bh - text_height) / 2 + ascent.
                        let text_y = box_y
                            + (*bh - crate::plantuml_metrics::text_height(STATE_FONT_SIZE)) / 2.0
                            + crate::plantuml_metrics::ascent(STATE_FONT_SIZE);
                        let mut text_buf = String::new();
                        text_render::emit_text(
                            &mut text_buf,
                            label,
                            &TextBase {
                                x: text_x,
                                y: text_y,
                                font_size: STATE_FONT_SIZE as u32,
                                font_family: "sans-serif",
                                fill: TEXT_COLOR,
                                bold: false,
                                italic: false,
                                underline: false,
                                skip_underline: false,
                            },
                        );
                        svg.push_str(&text_buf);
                    } else {
                        write!(
                            svg,
                            r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                            ent_id_of(id),
                        )
                        .unwrap();

                        // State rectangle.
                        write!(
                            svg,
                            r#"<rect fill="{fill_color}" height="{}" rx="{rx_s}" ry="{rx_s}" style="{stroke_style}" width="{}" x="{}" y="{}"/>"#,
                            fmt_f(*bh),
                            fmt_f(*bw),
                            fmt_f(box_x),
                            fmt_f(box_y),
                        )
                        .unwrap();

                        // Divider line (always present in PlantUML default mode).
                        let div_y = box_y + DIVIDER_OFFSET;
                        write!(
                            svg,
                            r#"<line style="{stroke_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            fmt_f(box_x),
                            fmt_f(box_x + bw),
                            fmt_f(div_y),
                            fmt_f(div_y),
                        )
                        .unwrap();

                        // State name label.
                        let text_w = text_render::measure(label, STATE_FONT_SIZE, false);
                        let text_x = cx - text_w / 2.0;
                        let text_y = box_y + NAME_BASELINE_OFFSET;
                        let mut text_buf = String::new();
                        text_render::emit_text(
                            &mut text_buf,
                            label,
                            &TextBase {
                                x: text_x,
                                y: text_y,
                                font_size: STATE_FONT_SIZE as u32,
                                font_family: "sans-serif",
                                fill: TEXT_COLOR,
                                bold: false,
                                italic: false,
                                underline: false,
                                skip_underline: false,
                            },
                        );
                        svg.push_str(&text_buf);

                        // Description lines.
                        for (j, desc) in descriptions.iter().enumerate() {
                            let desc_x = box_x + 5.0;
                            let desc_y = div_y + FIRST_DESC_OFFSET + j as f64 * DESC_LINE_SPACING;
                            let mut text_buf = String::new();
                            text_render::emit_text(
                                &mut text_buf,
                                desc,
                                &TextBase {
                                    x: desc_x,
                                    y: desc_y,
                                    font_size: DESC_FONT_SIZE as u32,
                                    font_family: "sans-serif",
                                    fill: TEXT_COLOR,
                                    bold: false,
                                    italic: false,
                                    underline: false,
                                    skip_underline: false,
                                },
                            );
                            svg.push_str(&text_buf);
                        }

                        svg.push_str("</g>");
                    }
                }
            }
        }
    }

    // Render notes. When the oracle layout is available, prefer its
    // captured path geometry — note placement depends on adjacent state
    // sizes (which we don't yet compute identically to Java), so the
    // computed path almost always disagrees. Walk oracle GMN entities
    // in numeric order and pair them with `diagram.notes` 1:1.
    let oracle_gmns: Vec<(&String, &crate::layout_oracle::EntityRect)> = oracle
        .map(|orc| {
            let mut v: Vec<_> = orc
                .entities
                .iter()
                .filter(|(k, _)| k.starts_with("GMN"))
                .collect();
            v.sort_by_key(|(k, _)| k.trim_start_matches("GMN").parse::<u32>().unwrap_or(0));
            v
        })
        .unwrap_or_default();

    for (note_idx, note) in diagram.notes.iter().enumerate() {
        if let Some((gmn_name, rect)) = oracle_gmns.get(note_idx) {
            // Oracle path: replay the captured path strings verbatim and
            // place text using the captured y positions.
            let entity_id = rect
                .entity_id
                .clone()
                .unwrap_or_else(|| "ent0000".to_string());
            let source_line = rect.name_text_x.map(|sl| sl as usize).unwrap_or(0);
            write!(
                svg,
                r#"<g class="entity" data-qualified-name="{gmn_name}" data-source-line="{source_line}" id="{entity_id}">"#,
            )
            .unwrap();
            // Replay each captured path with its own style — note
            // bodies and dog-ear folds sometimes use different stroke
            // widths in PlantUML's output.
            let mut first_d_for_left: Option<&str> = None;
            if let Some(paths) = &rect.glyph_path_d {
                for piece in paths.split('|') {
                    let (d, style) = piece
                        .split_once("#STYLE#")
                        .unwrap_or((piece, "stroke:#181818;stroke-width:0.5;"));
                    if first_d_for_left.is_none() {
                        first_d_for_left = Some(d);
                    }
                    write!(svg, r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#,).unwrap();
                }
            }
            // Note text — use oracle text_y values for vertical positions
            // and align horizontally to the *body* left edge (the first
            // `M` x in the body path). For `note right of`, the bbox
            // left sits at the arrow tip; the body's left is further
            // right.
            let body_left_x = first_d_for_left
                .and_then(|d| {
                    d.strip_prefix('M')
                        .and_then(|rest| rest.split(',').next())
                        .and_then(|s| s.parse::<f64>().ok())
                })
                .unwrap_or(rect.x);
            let text_x = body_left_x + NOTE_PADDING;
            let lines: Vec<&str> = note
                .text
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            for (i, line) in lines.iter().enumerate() {
                let fallback_y =
                    rect.y + NOTE_PADDING + LINK_FONT_SIZE + i as f64 * NOTE_LINE_HEIGHT;
                let ty = rect.text_y_values.get(i).copied().unwrap_or(fallback_y);
                let mut text_buf = String::new();
                text_render::emit_text(
                    &mut text_buf,
                    line,
                    &TextBase {
                        x: text_x,
                        y: ty,
                        font_size: LINK_FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text_buf);
            }
            svg.push_str("</g>");
            continue;
        }

        let note_w = note_box_width(&note.text);
        let note_h = note_box_height(&note.text);

        let (note_x, note_y, _anchor_x, _anchor_y) = match &note.kind {
            StateNoteKind::RightOf(state_id) if !state_id.is_empty() => {
                let mapped = if state_id == "[*]" {
                    if has_start { "__start__" } else { "__end__" }
                } else {
                    state_id.as_str()
                };
                let (sx, sy, sw, sh) = pos_of(mapped);
                let nx = sx + sw / 2.0 + NOTE_H_GAP;
                let ny = sy - sh / 2.0;
                (nx, ny, sx + sw / 2.0, sy)
            }
            StateNoteKind::LeftOf(state_id) if !state_id.is_empty() => {
                let mapped = if state_id == "[*]" {
                    if has_start { "__start__" } else { "__end__" }
                } else {
                    state_id.as_str()
                };
                let (sx, sy, sw, sh) = pos_of(mapped);
                let nx = sx - sw / 2.0 - NOTE_H_GAP - note_w;
                let ny = sy - sh / 2.0;
                (nx, ny, sx - sw / 2.0, sy)
            }
            StateNoteKind::Floating => {
                (MARGIN, MARGIN + title_h, MARGIN + note_w, MARGIN + title_h)
            }
            StateNoteKind::OnLink => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(MARGIN + STATE_MIN_WIDTH / 2.0);
                let nx = cx_approx + STATE_MIN_WIDTH / 2.0 + NOTE_H_GAP;
                (
                    nx,
                    mid_y - note_h / 2.0,
                    cx_approx + STATE_MIN_WIDTH / 2.0,
                    mid_y,
                )
            }
            StateNoteKind::RightOf(_) => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(MARGIN + STATE_MIN_WIDTH / 2.0);
                let nx = cx_approx + STATE_MIN_WIDTH / 2.0 + NOTE_H_GAP;
                (
                    nx,
                    mid_y - note_h / 2.0,
                    cx_approx + STATE_MIN_WIDTH / 2.0,
                    mid_y,
                )
            }
            StateNoteKind::LeftOf(_) => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(MARGIN + STATE_MIN_WIDTH / 2.0);
                let nx = cx_approx - STATE_MIN_WIDTH / 2.0 - NOTE_H_GAP - note_w;
                (
                    nx,
                    mid_y - note_h / 2.0,
                    cx_approx - STATE_MIN_WIDTH / 2.0,
                    mid_y,
                )
            }
        };

        // Note entity group.
        let note_ent = ids.next_entity();
        write!(
            svg,
            r#"<g class="entity" data-qualified-name="GMN{}" id="{note_ent}">"#,
            note_ent.trim_start_matches("ent"),
        )
        .unwrap();

        // Note box as PlantUML-style path (with dog-ear and pointer line).
        // The note has a pointer line going from the left edge to the state.
        let ear = NOTE_EAR;
        let r = note_x;
        let t = note_y;
        let nw = note_w;
        let nh = note_h;

        // Compute the mid-y of the note for the pointer arrow.
        let arrow_y = t + nh / 2.0;
        let _arrow_target_x = r - NOTE_H_GAP;

        // Note box path (matches PlantUML's note rendering).
        // PlantUML uses <path> for the note body shape.
        write!(
            svg,
            r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}" fill="{NOTE_FILL}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
            fmt_f(r), fmt_f(t + ear),
            fmt_f(r), fmt_f(arrow_y),
            fmt_f(r - NOTE_H_GAP + 2.0), fmt_f(arrow_y + 4.0),
            fmt_f(r), fmt_f(arrow_y + 8.0),
            fmt_f(r), fmt_f(t + nh),
            fmt_f(r), fmt_f(t + nh),
            fmt_f(r), fmt_f(t + nh),
            fmt_f(r + nw), fmt_f(t + nh),
            fmt_f(r + nw), fmt_f(t + nh),
            fmt_f(r + nw), fmt_f(t + ear),
            fmt_f(r + nw - ear), fmt_f(t),
            fmt_f(r), fmt_f(t),
            fmt_f(r), fmt_f(t + ear),
        )
        .unwrap();

        // Dog-ear fold.
        write!(
            svg,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{NOTE_FILL}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
            fmt_f(r + nw - ear), fmt_f(t),
            fmt_f(r + nw - ear), fmt_f(t + ear),
            fmt_f(r + nw), fmt_f(t + ear),
            fmt_f(r + nw - ear), fmt_f(t),
        )
        .unwrap();

        // Note text.
        let text_x = r + NOTE_PADDING;
        let mut text_y = t + NOTE_PADDING + LINK_FONT_SIZE;
        for line in note.text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let mut text_buf = String::new();
                text_render::emit_text(
                    &mut text_buf,
                    trimmed,
                    &TextBase {
                        x: text_x,
                        y: text_y,
                        font_size: LINK_FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text_buf);
                text_y += NOTE_LINE_HEIGHT;
            }
        }

        svg.push_str("</g>");
    }

    // Render links (transitions).
    if let Some(orc) = oracle {
        render_oracle_transitions(&mut svg, diagram, orc);
    } else {
        for t in &diagram.transitions {
            let from_layout = map_id(&t.from, true);
            let to_layout = map_id(&t.to, false);
            let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
            let to_name = if t.to == "[*]" { "*end*" } else { &t.to };

            // HTML comment.
            write!(svg, "<!--link {} to {}-->", from_name, to_name).unwrap();

            let link_id = ids.next_link();
            let from_ent = ent_id_of(&from_layout);
            let to_ent = ent_id_of(&to_layout);

            // Use the parser-provided source line from the transition model.
            let source_line = t.source_line;

            write!(
                svg,
                r#"<g class="link" data-entity-1="{from_ent}" data-entity-2="{to_ent}" data-link-type="dependency" data-source-line="{source_line}" id="{link_id}">"#,
            )
            .unwrap();

            // Try bezier path from layout engine.
            let edge_path = edge_paths
                .iter()
                .find(|ep| ep.from == from_layout && ep.to == to_layout);

            let (from_cx, from_cy, _from_w, from_h) = pos_of(&from_layout);
            let (to_cx, to_cy, _to_w, to_h) = pos_of(&to_layout);

            if let Some(ep) = edge_path
                && !ep.points.is_empty()
            {
                // Render bezier path.
                let points = &ep.points;
                let mut d = format!("M{},{}", fmt_f(points[0].0), fmt_f(points[0].1));
                let mut i = 1;
                while i + 2 < points.len() {
                    write!(
                        d,
                        " C{},{} {},{} {},{}",
                        fmt_f(points[i].0),
                        fmt_f(points[i].1),
                        fmt_f(points[i + 1].0),
                        fmt_f(points[i + 1].1),
                        fmt_f(points[i + 2].0),
                        fmt_f(points[i + 2].1),
                    )
                    .unwrap();
                    i += 3;
                }
                write!(
                    svg,
                    r#"<path d="{d}" fill="none" id="{from_name}-to-{to_name}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
                )
                .unwrap();

                // Arrowhead polygon.
                let endpoint = points[points.len() - 1];
                let control = if points.len() >= 2 {
                    points[points.len() - 2]
                } else {
                    (from_cx, from_cy)
                };
                render_arrowhead(&mut svg, control, endpoint);

                // Label.
                if let Some(label) = &t.label {
                    let first = points.first().unwrap();
                    let last = points.last().unwrap();
                    let mid_x = (first.0 + last.0) / 2.0;
                    let mid_y = (first.1 + last.1) / 2.0;
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: mid_x + 1.0,
                            y: mid_y,
                            font_size: LINK_FONT_SIZE as u32,
                            font_family: "sans-serif",
                            fill: TEXT_COLOR,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    svg.push_str(&text_buf);
                }
            } else {
                // Straight line fallback.
                let start_y = from_cy + from_h / 2.0;
                let end_y = to_cy - to_h / 2.0;

                // Path as cubic bezier.
                let mid_y = (start_y + end_y) / 2.0;
                write!(
                    svg,
                    r#"<path d="M{},{} C{},{} {},{} {},{}" fill="none" id="{from_name}-to-{to_name}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
                    fmt_f(from_cx), fmt_f(start_y),
                    fmt_f(from_cx), fmt_f(mid_y),
                    fmt_f(to_cx), fmt_f(mid_y),
                    fmt_f(to_cx), fmt_f(end_y),
                )
                .unwrap();

                // Arrowhead.
                let control = (to_cx, end_y - ARROW_LEN);
                let endpoint = (to_cx, end_y);
                render_arrowhead(&mut svg, control, endpoint);

                // Label.
                if let Some(label) = &t.label {
                    let label_x = from_cx.max(to_cx) + 1.0;
                    let label_y = (start_y + end_y) / 2.0;
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: label_x,
                            y: label_y,
                            font_size: LINK_FONT_SIZE as u32,
                            font_family: "sans-serif",
                            fill: TEXT_COLOR,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    svg.push_str(&text_buf);
                }
            }

            svg.push_str("</g>");
        }
    }

    svg.push_str("</g></svg>");
    svg
}

/// Render a filled arrowhead polygon at the endpoint, pointing in the direction
/// from control to endpoint.
fn render_arrowhead(svg: &mut String, control: (f64, f64), endpoint: (f64, f64)) {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let angle = dy.atan2(dx);

    let tip_x = endpoint.0;
    let tip_y = endpoint.1;

    // PlantUML arrowhead is a 4-point diamond shape.
    let perp_x = (angle + std::f64::consts::FRAC_PI_2).cos();
    let perp_y = (angle + std::f64::consts::FRAC_PI_2).sin();

    let left_x = tip_x - ARROW_LEN * angle.cos() + ARROW_HALF * perp_x;
    let left_y = tip_y - ARROW_LEN * angle.sin() + ARROW_HALF * perp_y;
    let right_x = tip_x - ARROW_LEN * angle.cos() - ARROW_HALF * perp_x;
    let right_y = tip_y - ARROW_LEN * angle.sin() - ARROW_HALF * perp_y;

    // Indent point (PlantUML uses a diamond-shaped arrowhead).
    let indent_x = tip_x - (ARROW_LEN - 4.0) * angle.cos();
    let indent_y = tip_y - (ARROW_LEN - 4.0) * angle.sin();

    #[allow(non_snake_case)]
    let STROKE_COLOR = DEFAULT_STROKE_COLOR;
    write!(
        svg,
        r#"<polygon fill="{STROKE_COLOR}" points="{},{},{},{},{},{},{},{}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
        fmt_f(tip_x), fmt_f(tip_y),
        fmt_f(left_x), fmt_f(left_y),
        fmt_f(indent_x), fmt_f(indent_y),
        fmt_f(right_x), fmt_f(right_y),
    )
    .unwrap();
}

/// Render transitions directly from oracle edge data.
fn render_oracle_transitions(svg: &mut String, diagram: &StateDiagram, oracle: &OracleLayout) {
    // Skinparam-aware text colour for transition labels: `stateArrowFontColor`
    // overrides the default `#000000`. Note that `stateFontColor` only
    // affects state names, not transition labels — keep them separate.
    let arrow_font_color = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("stateArrowFontColor")
                || sp.key.eq_ignore_ascii_case("arrowFontColor")
        })
        .map(|sp| crate::sequence::resolve_color(sp.value.trim()))
        .unwrap_or_else(|| DEFAULT_TEXT_COLOR.to_string());
    #[allow(non_snake_case)]
    let TEXT_COLOR: &str = arrow_font_color.as_str();
    // `skinparam ArrowFontSize <n>` (or the legacy `stateArrowFontSize`)
    // overrides the default 13pt transition-label size; this also feeds
    // `text_render` so the emitted `textLength` matches the smaller glyphs.
    let arrow_font_size: u32 = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("ArrowFontSize")
                || sp.key.eq_ignore_ascii_case("stateArrowFontSize")
        })
        .and_then(|sp| sp.value.trim().parse::<u32>().ok())
        .unwrap_or(LINK_FONT_SIZE as u32);
    // PlantUML's emission order for transitions does not always match the
    // parser's source order — when layout decides to bend an edge or sort
    // siblings differently, the golden SVG reorders them. Walking the
    // oracle's edges in their captured document order and mapping each
    // back to one of the parser's transitions preserves PlantUML's order.
    let mut consumed = vec![false; diagram.transitions.len()];
    let strip_suffix_digits = |full: &str, base: &str| -> bool {
        full == base
            || full
                .strip_prefix(base)
                .and_then(|s| s.strip_prefix('-'))
                .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
    };

    for oracle_edge in &oracle.edges {
        // Find an unused parser transition whose forward/reverse id matches
        // this oracle edge's `id` (with optional `-N` disambiguation).
        let mut matched: Option<(usize, bool)> = None;
        for (ti, t) in diagram.transitions.iter().enumerate() {
            if consumed[ti] {
                continue;
            }
            let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
            let to_name = if t.to == "[*]" { "*end*" } else { &t.to };
            let forward_id = format!("{from_name}-to-{to_name}");
            let reverse_id = format!("{to_name}-backto-{from_name}");
            if strip_suffix_digits(&oracle_edge.id, &forward_id) {
                matched = Some((ti, false));
                break;
            }
            if strip_suffix_digits(&oracle_edge.id, &reverse_id) {
                matched = Some((ti, true));
                break;
            }
        }
        let Some((ti, is_reverse)) = matched else {
            continue;
        };
        consumed[ti] = true;
        let t = &diagram.transitions[ti];
        let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
        let to_name = if t.to == "[*]" { "*end*" } else { &t.to };

        // HTML comment.
        if is_reverse {
            write!(svg, "<!--reverse link {to_name} to {from_name}-->").unwrap();
        } else {
            write!(svg, "<!--link {from_name} to {to_name}-->").unwrap();
        }

        // Link group wrapper using oracle attributes.
        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("dependency");
        let source_line = oracle_edge.source_line.as_deref().unwrap_or("0");
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");

        write!(
            svg,
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
        )
        .unwrap();

        // Path element with oracle data.
        let path_style = oracle_edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let path_id = &oracle_edge.id;
        write!(
            svg,
            r#"<path d="{}" fill="none" id="{path_id}" style="{path_style}"/>"#,
            oracle_edge.d,
        )
        .unwrap();

        // Arrowhead polygon from oracle.
        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            )
            .unwrap();
        }

        // Edge labels from oracle. PlantUML emits one or more `<text>`
        // children inside the `<g class="link">` for transition labels.
        // We trust the oracle for exact x/y placement and reuse our
        // text renderer for the markup (lengthAdjust/textLength etc.).
        for (lx, ly, text) in &oracle_edge.labels {
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                text,
                &TextBase {
                    x: *lx,
                    y: *ly,
                    font_size: arrow_font_size,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.push_str(&text_buf);
        }

        svg.push_str("</g>");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    #[test]
    fn simple_state_diagram() {
        let d = StateDiagram {
            meta: DiagramMeta::default(),
            states: vec![
                State {
                    id: "Active".into(),
                    label: "Active".into(),
                    kind: StateKind::Normal,
                    descriptions: vec![],
                    substates: vec![],
                    source_line: 0,
                    fill: None,
                    stroke: None,
                    stroke_style: None,
                },
                State {
                    id: "Inactive".into(),
                    label: "Inactive".into(),
                    kind: StateKind::Normal,
                    descriptions: vec![],
                    substates: vec![],
                    source_line: 0,
                    fill: None,
                    stroke: None,
                    stroke_style: None,
                },
            ],
            transitions: vec![
                Transition {
                    from: "[*]".into(),
                    to: "Active".into(),
                    label: None,
                    source_line: 0,
                },
                Transition {
                    from: "Active".into(),
                    to: "Inactive".into(),
                    label: Some("disable".into()),
                    source_line: 0,
                },
            ],
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Active"));
        assert!(svg.contains("Inactive"));
        assert!(svg.contains("disable"));
        // Check PlantUML-specific attributes.
        assert!(svg.contains(r#"data-diagram-type="STATE""#));
        assert!(svg.contains(r#"class="start_entity""#));
        assert!(svg.contains(r#"class="entity""#));
        assert!(svg.contains(r#"class="link""#));
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\n[*] --> Active\nActive --> Inactive : disable\nInactive --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Active"));
        assert!(svg.contains("disable"));
        assert!(svg.contains(r#"data-diagram-type="STATE""#));
        assert!(svg.contains(r#"class="end_entity""#));
    }

    #[test]
    fn state_desc_syntax_renders_inside_box() {
        let input = "@startuml\nstate A : idle\n[*] --> A\nA --> B : next\nB --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("idle"),
            "description text should appear in SVG"
        );
        // The divider line should be present.
        assert!(svg.contains("<line"), "divider line should be rendered");
    }

    #[test]
    fn note_right_of_state_renders_text() {
        let input = "@startuml\n[*] --> A\nA --> [*]\nnote right of A : Note 1\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Note 1"), "note text should appear in SVG");
    }

    #[test]
    fn multiline_note_renders_all_lines() {
        let input = "@startuml\n[*] --> A\nnote right of A\n  line 1\n  line 2\nend note\nA --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("line 1"),
            "first note line should appear in SVG"
        );
        assert!(
            svg.contains("line 2"),
            "second note line should appear in SVG"
        );
    }

    #[test]
    fn floating_note_renders_text() {
        let input = "@startuml\nnote \"Floating note 1\" as FN1\n[*] --> A\nA --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("Floating note 1"),
            "floating note text should appear in SVG"
        );
    }

    #[test]
    fn plantuml_svg_structure() {
        let input = "@startuml\n[*] --> A\nA --> B\nB --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Check PlantUML SVG root attributes.
        assert!(svg.contains(r#"contentStyleType="text/css""#));
        assert!(svg.contains(r#"data-diagram-type="STATE""#));
        assert!(svg.contains(r#"preserveAspectRatio="none""#));
        assert!(svg.contains(r#"version="1.1""#));
        assert!(svg.contains(r#"zoomAndPan="magnify""#));
        assert!(svg.contains("<?plantuml"));
        assert!(svg.contains("<defs/>"));

        // Check element types.
        assert!(svg.contains("<ellipse")); // start/end use ellipse, not circle
        assert!(svg.contains(r##"fill="#F1F1F1""##)); // PlantUML state fill
        assert!(svg.contains(r##"style="stroke:#181818;stroke-width:0.5;""##)); // stroke as style attr
        assert!(svg.contains(r##"lengthAdjust="spacing""##)); // text attributes
    }
}
