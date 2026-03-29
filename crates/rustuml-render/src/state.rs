// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.
//!
//! Produces PlantUML-compatible SVG output with matching element structure,
//! attributes, and styling.

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::state::*;

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
    let label_w = metrics::text_width(label, STATE_FONT_SIZE) + STATE_H_PADDING;
    let desc_w = descriptions
        .iter()
        .map(|d| metrics::text_width(d, DESC_FONT_SIZE) + 10.0)
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

/// Format a float with PlantUML-style precision (remove trailing zeros but keep
/// at least one decimal).
fn fmt_f(v: f64) -> String {
    // PlantUML uses 2 decimal places for most coordinates.
    let s = format!("{:.2}", v);
    // Strip trailing zeros after decimal point, but keep at least one digit.
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0');
        if let Some(without_dot) = trimmed.strip_suffix('.') {
            // Integer value — drop the dot entirely for PlantUML compatibility.
            without_dot.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
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

    // Collect ordered unique state IDs.
    // Start [*] first, then declared states, then undeclared transition targets.
    let mut state_ids: Vec<String> = Vec::new();
    if has_start {
        state_ids.push("__start__".to_string());
    }
    for s in &diagram.states {
        if !state_ids.contains(&s.id) && s.id != "[*]" {
            state_ids.push(s.id.clone());
        }
    }
    for t in &diagram.transitions {
        for id in [&t.from, &t.to] {
            if id == "[*]" {
                continue; // Handled separately as start/end.
            }
            if !state_ids.contains(id) {
                state_ids.push(id.clone());
            }
        }
    }
    if has_end {
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

    // Attempt Sugiyama layout.
    let layout_result = {
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
    let edge_paths: &[EdgePath] = layout_result
        .as_ref()
        .map(|r| r.edge_paths.as_slice())
        .unwrap_or(&[]);

    let use_sugiyama = layout_positions.is_some_and(|p| p.len() >= state_ids.len());

    // Compute positions: (id, center_x, center_y, box_width, box_height).
    let (positions, total_width, total_height) = if use_sugiyama {
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
        let title_tw = metrics::text_width(title, TITLE_FONT_SIZE);
        write!(
            svg,
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{TITLE_FONT_SIZE}" font-weight="bold" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{title_escaped}</text>"#,
            fmt_f(title_tw),
            fmt_f(title_x - title_tw / 2.0),
            fmt_f(TITLE_HEIGHT - 4.0),
        )
        .unwrap();
    }

    // Assign entity IDs for all nodes.
    let mut entity_ids: Vec<(String, String)> = Vec::new();
    for (id, _, _, _, _) in &positions {
        let ent_id = ids.next_entity();
        entity_ids.push((id.clone(), ent_id));
    }

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
            // End pseudo-state — use the source_line from the last transition
            // targeting [*].
            let source_line = diagram
                .transitions
                .iter()
                .rev()
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
                    let tw = metrics::text_width("H", STATE_FONT_SIZE);
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
                    let tw = metrics::text_width("H*", STATE_FONT_SIZE);
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
                        let text_w = metrics::text_width(label, STATE_FONT_SIZE);
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
                        let text_w = metrics::text_width(label, STATE_FONT_SIZE);
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
                            let desc_w = metrics::text_width(desc, DESC_FONT_SIZE);
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
                let tw = metrics::text_width(trimmed, LINK_FONT_SIZE);
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
                let tw = metrics::text_width(label, LINK_FONT_SIZE);
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
                let tw = metrics::text_width(label, LINK_FONT_SIZE);
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
