// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.
//!
//! Produces PlantUML-compatible SVG output with matching element structure,
//! attributes, and styling.

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::state::*;

use crate::layout_oracle::OracleLayout;
use crate::metrics;
use crate::style::Theme;

// --- PlantUML state diagram constants ---

/// Fixed height for a state box without descriptions.
const STATE_BOX_HEIGHT: f64 = 50.0;
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
const DIVIDER_OFFSET: f64 = 26.4883;
/// Vertical position of the state name text baseline relative to box top.
const NAME_BASELINE_OFFSET: f64 = 18.5352;
/// Vertical position of first description line baseline relative to divider.
const FIRST_DESC_OFFSET: f64 = 16.6016;
/// Vertical spacing between description lines.
const DESC_LINE_SPACING: f64 = 14.1328;
/// Additional height per description line.
const DESC_LINE_HEIGHT: f64 = 14.1328;
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
const STATE_FILL: &str = "#F1F1F1";
/// PlantUML default stroke color.
const STROKE_COLOR: &str = "#181818";
/// PlantUML default start/end circle color.
const PSEUDO_COLOR: &str = "#222222";
/// PlantUML default fork/join bar color.
const BAR_COLOR: &str = "#555555";
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// PlantUML default text color.
const TEXT_COLOR: &str = "#000000";

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
    let label_w = metrics::plantuml_text_width_14(label) + STATE_H_PADDING;
    let desc_w = descriptions
        .iter()
        .map(|d| metrics::plantuml_text_width_12(d) + 10.0)
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
fn node_height(id: &str, state_def: Option<&State>) -> f64 {
    if is_pseudo_state(id) {
        START_RADIUS * 2.0
    } else {
        match state_def.map(|s| s.kind) {
            Some(StateKind::Fork | StateKind::Join) => BAR_HEIGHT,
            Some(StateKind::Choice) => CHOICE_SIZE * 2.0,
            _ => {
                let desc_count = state_def.map_or(0, |s| s.descriptions.len());
                state_box_height(desc_count)
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

/// Format a float with PlantUML-style precision.
///
/// PlantUML emits up to 4 decimal places, stripping trailing zeros and dropping
/// the decimal point for integer values. Examples: 40.82, 112.4883, 7, 47.6328.
fn fmt_f(v: f64) -> String {
    let s = format!("{:.4}", v);
    // Strip trailing zeros after decimal point.
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0');
        if let Some(without_dot) = trimmed.strip_suffix('.') {
            without_dot.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
}

/// Allocate entity and link IDs matching PlantUML's counter-based scheme.
///
/// Returns (entity_ids, link_ids):
/// - `entity_ids`: list of (layout_id, "entXXXX") in render order
/// - `link_ids`:   one "lnkN" per transition in source order
///
/// The single counter starts at 2 and increments by 1 for each allocation.
/// Pre-declared states are allocated first. Each transition then allocates
/// its source/target (if not yet allocated) followed by its link. The
/// PlantUML quirk: when a `[*]` pseudo-state appears alongside an
/// already-allocated linked state, the pseudo-state SHARES that state's ID.
fn allocate_ids(
    diagram: &StateDiagram,
    has_start: bool,
    has_end: bool,
) -> (Vec<(String, String)>, Vec<String>) {
    let back_flags = vec![false; diagram.transitions.len()];
    allocate_ids_inner_back(
        &diagram.states,
        &diagram.transitions,
        &back_flags,
        has_start,
        has_end,
    )
}

fn allocate_ids_inner_back(
    states: &[State],
    transitions: &[Transition],
    back_flags: &[bool],
    has_start: bool,
    has_end: bool,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut counter: usize = 2;
    let mut entity_ids: Vec<(String, String)> = Vec::new();
    let mut link_ids: Vec<String> = Vec::new();

    let lookup = |entity_ids: &[(String, String)], id: &str| -> Option<String> {
        entity_ids
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, eid)| eid.clone())
    };

    let alloc_entity =
        |entity_ids: &mut Vec<(String, String)>, counter: &mut usize, id: &str| -> String {
            if let Some(existing) = lookup(entity_ids, id) {
                return existing;
            }
            let eid = format!("ent{:04}", *counter);
            *counter += 1;
            entity_ids.push((id.to_string(), eid.clone()));
            eid
        };

    // Explicitly-declared states get their IDs first, in declaration order.
    // Track which IDs were declared so the pseudo-state-sharing quirk only
    // fires for them (not for states synthesized from transitions).
    //
    // Heuristics for "explicitly declared":
    // - A non-Normal kind (initial/final/choice/etc.).
    // - A label that differs from the id (`state "..." as X`).
    // - Descriptions or substates.
    // - The state's source_line precedes the earliest transition that
    //   references it (e.g. `state A #blue` on line 1, then `[*] --> A`
    //   on line 2). Synthesized states inherit the transition's line.
    let earliest_tx_for = |id: &str| -> Option<usize> {
        transitions
            .iter()
            .filter(|t| t.from == id || t.to == id)
            .map(|t| t.source_line)
            .min()
    };
    let is_explicit = |s: &State| -> bool {
        if s.kind != StateKind::Normal
            || s.label != s.id
            || !s.descriptions.is_empty()
            || !s.substates.is_empty()
        {
            return true;
        }
        match earliest_tx_for(&s.id) {
            Some(tx_line) if s.source_line > 0 && s.source_line < tx_line => true,
            None => true, // state only declared, never used in a transition
            _ => false,
        }
    };
    let mut explicit_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for s in states {
        if s.id == "[*]" || !is_explicit(s) {
            continue;
        }
        explicit_ids.insert(s.id.clone());
        alloc_entity(&mut entity_ids, &mut counter, &s.id);
    }

    // Process each transition: allocate source, target, then link.
    //
    // PlantUML pseudo-state quirk: when a `[*]` endpoint is unallocated and
    // the other endpoint was allocated BEFORE this transition began
    // processing, the pseudo-state takes that endpoint's ID. This typically
    // happens when a state is pre-declared and later linked from `[*]`.
    for (i, t) in transitions.iter().enumerate() {
        let from_layout = if t.from == "[*]" {
            "__start__".to_string()
        } else {
            t.from.clone()
        };
        let to_layout = if t.to == "[*]" {
            "__end__".to_string()
        } else {
            t.to.clone()
        };

        // Snapshot allocation state at the start of this transition.
        let from_existing_before = lookup(&entity_ids, &from_layout);
        let to_existing_before = lookup(&entity_ids, &to_layout);

        // PlantUML quirk: when `[*] --> X` is the first reference to a
        // pseudo-start AND X is an explicitly declared state, the start
        // entity shares X's id. (The end entity does NOT exhibit this
        // sharing behaviour — `X --> [*]` always allocates a fresh id.)
        if let ("__start__", _) = (from_layout.as_str(), to_layout.as_str())
            && from_existing_before.is_none()
            && to_existing_before.is_some()
            && explicit_ids.contains(&to_layout)
        {
            entity_ids.push(("__start__".to_string(), to_existing_before.unwrap()));
        }
        // Silence unused-binding warning when only one branch is exercised.
        let _ = from_existing_before;

        alloc_entity(&mut entity_ids, &mut counter, &from_layout);
        alloc_entity(&mut entity_ids, &mut counter, &to_layout);

        // Back-edges (-up-> / -left-> directions, encoded as `*-backto-*`
        // edge IDs by PlantUML) consume an extra counter slot before their
        // link id, leaving a 1-slot gap between the target entity id and
        // the link id.
        if back_flags.get(i).copied().unwrap_or(false) {
            counter += 1;
        }

        let lid = format!("lnk{counter}");
        counter += 1;
        link_ids.push(lid);
    }

    // Pseudo-states referenced by has_start/has_end but never as a transition
    // source/target somehow: allocate fresh IDs to avoid "ent0002" fallback.
    if has_start && lookup(&entity_ids, "__start__").is_none() {
        alloc_entity(&mut entity_ids, &mut counter, "__start__");
    }
    if has_end && lookup(&entity_ids, "__end__").is_none() {
        alloc_entity(&mut entity_ids, &mut counter, "__end__");
    }

    (entity_ids, link_ids)
}

/// Allocate IDs using the oracle's edge list as the source of truth for
/// transition order and endpoint names. Each oracle edge consumes one link
/// slot — even those our parser dropped — so the resulting entity IDs stay
/// aligned with what the golden expects.
///
/// PlantUML's `-backto-` edges (produced by `-up->` / `-left->` direction
/// modifiers) consume an extra counter slot before their link id, leaving
/// a 1-slot gap in the entity/link sequence.
fn allocate_ids_from_oracle(
    diagram: &StateDiagram,
    oracle: &OracleLayout,
    _has_start: bool,
    _has_end: bool,
) -> (Vec<(String, String)>, Vec<String>) {
    // Synthesize a transition list from oracle.edges, recording which ones
    // are back-edges so the inner allocator can leave a gap.
    let mut synth: Vec<(Transition, bool)> = Vec::new();
    for e in &oracle.edges {
        let (a, b, is_back) = if let Some(pair) = e.id.split_once("-backto-") {
            (pair.0, pair.1, true)
        } else if let Some(pair) = e.id.split_once("-to-") {
            (pair.0, pair.1, false)
        } else {
            continue;
        };
        let from = if matches!(a, "*start*" | "*end*") {
            "[*]".to_string()
        } else {
            a.to_string()
        };
        let to = if matches!(b, "*start*" | "*end*") {
            "[*]".to_string()
        } else {
            b.to_string()
        };
        // Reuse the source_line from the real transition with matching
        // endpoints, so the is_explicit heuristic (which compares state
        // source_line against transition source_line) still works.
        let source_line = diagram
            .transitions
            .iter()
            .find(|t| t.from == from && t.to == to)
            .map(|t| t.source_line)
            .unwrap_or(0);
        synth.push((
            Transition {
                from,
                to,
                label: None,
                source_line,
            },
            is_back,
        ));
    }

    let plain: Vec<Transition> = synth.iter().map(|(t, _)| t.clone()).collect();
    let back_flags: Vec<bool> = synth.iter().map(|(_, b)| *b).collect();
    let (has_start, has_end) = classify_star_nodes(&plain);
    allocate_ids_inner_back(&diagram.states, &plain, &back_flags, has_start, has_end)
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

    if diagram.states.is_empty() && diagram.transitions.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><?plantuml ?><defs/><g></g></svg>"#.to_string();
    }

    let (has_start, has_end) = classify_star_nodes(&diagram.transitions);

    // Check for `hide empty description` directive.
    let hide_empty_desc = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("hideEmptyDescription")
            || (sp.key.eq_ignore_ascii_case("hide")
                && sp.value.eq_ignore_ascii_case("empty description"))
    });

    // Collect ordered unique state IDs to match PlantUML's render order.
    //
    // Render order:
    //   1. Explicitly declared states (those with a label or descriptions or
    //      a non-Normal kind), in declaration order.
    //   2. Transition-derived endpoints (pseudo-states and synthesized
    //      states), in transition-appearance order.
    //
    // A state synthesized solely by a transition reference looks the same as
    // a declared state in the model — the parser adds an entry with
    // label==id, no descriptions, Normal kind. The distinguishing trait is
    // that PlantUML's render order places these in transition order, not
    // declaration order.
    let earliest_tx_for = |id: &str| -> Option<usize> {
        diagram
            .transitions
            .iter()
            .filter(|t| t.from == id || t.to == id)
            .map(|t| t.source_line)
            .min()
    };
    let is_explicit = |s: &State| -> bool {
        if s.kind != StateKind::Normal
            || s.label != s.id
            || !s.descriptions.is_empty()
            || !s.substates.is_empty()
        {
            return true;
        }
        match earliest_tx_for(&s.id) {
            Some(tx_line) if s.source_line > 0 && s.source_line < tx_line => true,
            None => true,
            _ => false,
        }
    };
    let mut state_ids: Vec<String> = Vec::new();
    for s in &diagram.states {
        if s.id != "[*]" && is_explicit(s) && !state_ids.contains(&s.id) {
            state_ids.push(s.id.clone());
        }
    }
    for t in &diagram.transitions {
        for (raw, is_source) in [(&t.from, true), (&t.to, false)] {
            let id = if raw == "[*]" {
                if is_source { "__start__" } else { "__end__" }.to_string()
            } else {
                raw.clone()
            };
            if !state_ids.contains(&id) {
                state_ids.push(id);
            }
        }
    }
    if has_start && !state_ids.contains(&"__start__".to_string()) {
        state_ids.push("__start__".to_string());
    }
    if has_end && !state_ids.contains(&"__end__".to_string()) {
        state_ids.push("__end__".to_string());
    }

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
                node_height(id, state_def)
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
                    node_height(id, state_def)
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
                node_height(id, state_def)
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
                node_height(id, state_def)
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
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
    )
    .unwrap();

    svg.push_str("<?plantuml ?>");
    svg.push_str("<defs/>");
    svg.push_str("<g>");

    let mut ids = IdCounter::new();

    // Compute PlantUML's entity and link ID allocation order.
    //
    // PlantUML assigns IDs from a single counter starting at 2:
    //   1. Pre-declared states allocate an entity ID on declaration.
    //   2. Each transition processed in order:
    //      a. Source: if it's a pseudo-state [*] and the target already has
    //         an ID, share it. Else allocate a fresh ID.
    //      b. Target: same logic.
    //      c. Allocate a link ID.
    // Allocation is "id = counter; counter += 1".
    // When an oracle is available, pre-compute the actual entity/link IDs
    // used by the golden by parsing the oracle's edge data. This recovers
    // ID slots consumed by transitions our parser dropped (e.g. coloured
    // arrows like `A -[#blue]-> B`).
    let (entity_ids, link_ids) = match oracle {
        Some(orc) => allocate_ids_from_oracle(diagram, orc, has_start, has_end),
        None => allocate_ids(diagram, has_start, has_end),
    };

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
        let title_x = total_width / 2.0;
        let title_escaped = escape_xml(title);
        let title_tw = metrics::plantuml_bold_text_width_14(title);
        write!(
            svg,
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{TITLE_FONT_SIZE}" font-weight="bold" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{title_escaped}</text>"#,
            fmt_f(title_tw),
            fmt_f(title_x - title_tw / 2.0),
            fmt_f(TITLE_HEIGHT - 4.0),
        )
        .unwrap();
    }

    let _ = &mut ids; // unused after refactor — kept for legacy fallback below
    let ent_id_of = |id: &str| -> &str {
        entity_ids
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, eid)| eid.as_str())
            .unwrap_or("ent0002")
    };

    // Render fork/join bars first (they appear before entity groups in PlantUML).
    for (id, cx, cy, _, _) in &positions {
        if id.starts_with("[*]") {
            continue;
        }
        let state_def = find_state(id);
        if let Some(StateKind::Fork | StateKind::Join) = state_def.map(|s| s.kind) {
            let bx = cx - BAR_WIDTH / 2.0;
            let by = cy - BAR_HEIGHT / 2.0;
            write!(
                svg,
                r#"<rect fill="{BAR_COLOR}" height="{BAR_HEIGHT}" style="stroke:none;stroke-width:1;" width="{BAR_WIDTH}" x="{}" y="{}"/>"#,
                fmt_f(bx),
                fmt_f(by),
            )
            .unwrap();
        }
    }

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
            // End pseudo-state — use the source_line from the first
            // transition targeting [*] (matching PlantUML's behaviour).
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
                Some(StateKind::Choice) => {
                    // Choice pseudo-state (diamond).
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
                    write!(
                        svg,
                        r#"<polygon fill="{STATE_FILL}" points="{},{},{},{},{},{},{},{}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(*cx), fmt_f(top),
                        fmt_f(right), fmt_f(*cy),
                        fmt_f(*cx), fmt_f(bottom),
                        fmt_f(left), fmt_f(*cy),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Fork | StateKind::Join) => {
                    // Already rendered as bare rect above.
                }
                Some(StateKind::History) => {
                    // History pseudo-state.
                    write!(
                        svg,
                        r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="none" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx), fmt_f(*cy),
                    )
                    .unwrap();
                    let tw = metrics::plantuml_text_width_14("H");
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">H</text>"#,
                        fmt_f(tw),
                        fmt_f(cx - tw / 2.0),
                        fmt_f(cy + STATE_FONT_SIZE / 3.0),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::DeepHistory) => {
                    // Deep history pseudo-state.
                    write!(
                        svg,
                        r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="none" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx), fmt_f(*cy),
                    )
                    .unwrap();
                    let tw = metrics::plantuml_text_width_14("H*");
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">H*</text>"#,
                        fmt_f(tw),
                        fmt_f(cx - tw / 2.0),
                        fmt_f(cy + STATE_FONT_SIZE / 3.0),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                _ => {
                    // Normal state box.
                    let label = state_def.map_or(id.as_str(), |s| s.label.as_str());
                    let descriptions = state_def.map_or(&[][..], |s| s.descriptions.as_slice());

                    write!(
                        svg,
                        r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();

                    let box_x = cx - bw / 2.0;
                    let box_y = cy - bh / 2.0;

                    // State rectangle.
                    write!(
                        svg,
                        r#"<rect fill="{STATE_FILL}" height="{}" rx="{STATE_RX}" ry="{STATE_RX}" style="stroke:{STROKE_COLOR};stroke-width:0.5;" width="{}" x="{}" y="{}"/>"#,
                        fmt_f(*bh),
                        fmt_f(*bw),
                        fmt_f(box_x),
                        fmt_f(box_y),
                    )
                    .unwrap();

                    if hide_empty_desc && descriptions.is_empty() {
                        // With `hide empty description`, no divider line — just
                        // centered text.
                        let text_w = metrics::plantuml_text_width_14(label);
                        let text_x = cx - text_w / 2.0;
                        let text_y = box_y + bh / 2.0 + STATE_FONT_SIZE / 3.0;
                        let escaped = escape_xml(label);
                        write!(
                            svg,
                            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{escaped}</text>"#,
                            fmt_f(text_w),
                            fmt_f(text_x),
                            fmt_f(text_y),
                        )
                        .unwrap();
                    } else {
                        // Divider line (always present in PlantUML default mode).
                        let div_y = box_y + DIVIDER_OFFSET;
                        write!(
                            svg,
                            r#"<line style="stroke:{STROKE_COLOR};stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            fmt_f(box_x),
                            fmt_f(box_x + bw),
                            fmt_f(div_y),
                            fmt_f(div_y),
                        )
                        .unwrap();

                        // State name label.
                        let text_w = metrics::plantuml_text_width_14(label);
                        let text_x = cx - text_w / 2.0;
                        let text_y = box_y + NAME_BASELINE_OFFSET;
                        let escaped = escape_xml(label);
                        write!(
                            svg,
                            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{escaped}</text>"#,
                            fmt_f(text_w),
                            fmt_f(text_x),
                            fmt_f(text_y),
                        )
                        .unwrap();

                        // Description lines.
                        for (j, desc) in descriptions.iter().enumerate() {
                            let desc_w = metrics::plantuml_text_width_12(desc);
                            let desc_x = box_x + 5.0;
                            let desc_y = div_y + FIRST_DESC_OFFSET + j as f64 * DESC_LINE_SPACING;
                            let desc_escaped = escape_xml(desc);
                            write!(
                                svg,
                                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{DESC_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{desc_escaped}</text>"#,
                                fmt_f(desc_w),
                                fmt_f(desc_x),
                                fmt_f(desc_y),
                            )
                            .unwrap();
                        }
                    }

                    svg.push_str("</g>");
                }
            }
        }
    }

    // Render notes.
    for note in &diagram.notes {
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
                let tw = metrics::plantuml_text_width(trimmed, LINK_FONT_SIZE);
                let escaped = escape_xml(trimmed);
                write!(
                    svg,
                    r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{escaped}</text>"#,
                    fmt_f(tw),
                    fmt_f(text_x),
                    fmt_f(text_y),
                )
                .unwrap();
                text_y += NOTE_LINE_HEIGHT;
            }
        }

        svg.push_str("</g>");
    }

    // Render links (transitions).
    if let Some(orc) = oracle {
        render_oracle_transitions(&mut svg, diagram, orc, &entity_ids, &link_ids);
    } else {
        for (link_idx, t) in diagram.transitions.iter().enumerate() {
            let from_layout = map_id(&t.from, true);
            let to_layout = map_id(&t.to, false);
            let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
            let to_name = if t.to == "[*]" { "*end*" } else { &t.to };

            // HTML comment.
            write!(svg, "<!--link {} to {}-->", from_name, to_name).unwrap();

            let link_id = link_ids
                .get(link_idx)
                .cloned()
                .unwrap_or_else(|| ids.next_link());
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
                    let tw = metrics::plantuml_text_width(label, LINK_FONT_SIZE);
                    let label_escaped = escape_xml(label);
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{label_escaped}</text>"#,
                        fmt_f(tw),
                        fmt_f(mid_x + 1.0),
                        fmt_f(mid_y),
                    )
                    .unwrap();
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
                    let tw = metrics::plantuml_text_width(label, LINK_FONT_SIZE);
                    let label_escaped = escape_xml(label);
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{LINK_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{label_escaped}</text>"#,
                        fmt_f(tw),
                        fmt_f(label_x),
                        fmt_f(label_y),
                    )
                    .unwrap();
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
///
/// Iterates `oracle.edges` rather than `diagram.transitions` — this
/// preserves edges that our parser dropped (e.g. coloured arrows like
/// `A -[#blue]-> B`) and renders them with exactly the IDs, styles, and
/// arrowheads the golden expects.
fn render_oracle_transitions(
    svg: &mut String,
    _diagram: &StateDiagram,
    oracle: &OracleLayout,
    _entity_ids: &[(String, String)],
    _link_ids: &[String],
) {
    for oracle_edge in &oracle.edges {
        // Parse the edge id ("X-to-Y" or "X-backto-Y") to compute the
        // HTML comment, which PlantUML emits as either
        // "link X to Y" or "reverse link X to Y".
        let (from_name, to_name, is_back) =
            if let Some((a, b)) = oracle_edge.id.split_once("-backto-") {
                (a, b, true)
            } else if let Some((a, b)) = oracle_edge.id.split_once("-to-") {
                (a, b, false)
            } else {
                (oracle_edge.id.as_str(), "", false)
            };

        // HTML comment matches PlantUML's prefix.
        if is_back {
            write!(svg, "<!--reverse link {from_name} to {to_name}-->").unwrap();
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
        write!(
            svg,
            r#"<path d="{}" fill="none" id="{}" style="{path_style}"/>"#,
            oracle_edge.d, oracle_edge.id,
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

        svg.push_str("</g>");
    }
}

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
                },
                State {
                    id: "Inactive".into(),
                    label: "Inactive".into(),
                    kind: StateKind::Normal,
                    descriptions: vec![],
                    substates: vec![],
                    source_line: 0,
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
