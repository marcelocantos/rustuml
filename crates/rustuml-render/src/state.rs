// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.

use rustuml_layout::graph::{Direction, LayoutGraph};
use rustuml_parser::diagram::state::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const STATE_WIDTH: f64 = 100.0;
/// Height of the label header portion of a state box.
const STATE_HEADER_HEIGHT: f64 = 30.0;
/// Height of a no-description state box (same as old STATE_HEIGHT = 40).
const STATE_NO_DESC_HEIGHT: f64 = 40.0;
const DESC_LINE_HEIGHT: f64 = 14.0;
const DESC_PADDING: f64 = 8.0;
const STATE_RX: f64 = 15.0;
const INITIAL_R: f64 = 10.0;
const MARGIN: f64 = 30.0;
const V_GAP: f64 = 60.0;
const H_GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;

/// Horizontal gap between a state box edge and a positioned note.
const NOTE_H_GAP: f64 = 10.0;
/// Padding inside a note box.
const NOTE_PADDING: f64 = 6.0;
/// Height of each line of text in a note.
const NOTE_LINE_HEIGHT: f64 = 14.0;
/// Dog-ear fold size for note boxes.
const NOTE_EAR: f64 = 10.0;
/// Minimum note width.
const NOTE_MIN_WIDTH: f64 = 60.0;
/// Approximate character width for rough note sizing.
const NOTE_CHAR_WIDTH: f64 = 7.0;

/// Compute the height of a note box for a given text (may be multi-line).
fn note_box_height(text: &str) -> f64 {
    let line_count = text.lines().filter(|l| !l.trim().is_empty()).count().max(1);
    NOTE_PADDING + line_count as f64 * NOTE_LINE_HEIGHT + NOTE_PADDING
}

/// Compute an approximate width for a note box based on its text content.
fn note_box_width(text: &str) -> f64 {
    let max_chars = text.lines().map(|l| l.trim().len()).max().unwrap_or(4);
    (max_chars as f64 * NOTE_CHAR_WIDTH + NOTE_PADDING * 2.0 + NOTE_EAR).max(NOTE_MIN_WIDTH)
}

/// Render note text lines inside a note box.
fn render_note_text(svg: &mut SvgBuilder, text: &str, text_x: f64, text_start_y: f64) {
    let mut y = text_start_y;
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            svg.text(text_x, y, trimmed, "start", SMALL_FONT);
            y += NOTE_LINE_HEIGHT;
        }
    }
}

/// Compute the total height of a state box given its description line count.
fn state_box_height(desc_count: usize) -> f64 {
    if desc_count == 0 {
        STATE_NO_DESC_HEIGHT
    } else {
        STATE_HEADER_HEIGHT + DESC_PADDING + desc_count as f64 * DESC_LINE_HEIGHT + DESC_PADDING
    }
}

/// Returns true if the state ID is a pseudo-state (rendered as a circle, not a box).
fn is_pseudo_state(id: &str) -> bool {
    id == "[*]" || id == "[H]" || id == "[H*]"
}

/// Height of any node (pseudo or normal) for layout purposes.
fn node_height(id: &str, state_def: Option<&State>) -> f64 {
    if is_pseudo_state(id) {
        INITIAL_R * 2.0
    } else {
        let desc_count = state_def.map_or(0, |s| s.descriptions.len());
        state_box_height(desc_count)
    }
}

/// Render a state diagram to SVG.
pub fn render(diagram: &StateDiagram, theme: &Theme) -> String {
    let sts = &theme.state;
    if diagram.states.is_empty() && diagram.transitions.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Collect all state IDs including [*].
    let mut state_ids: Vec<String> = vec!["[*]".to_string()];
    for s in &diagram.states {
        if !state_ids.contains(&s.id) {
            state_ids.push(s.id.clone());
        }
    }
    // Also include states that appear only in transitions.
    for t in &diagram.transitions {
        for id in [&t.from, &t.to] {
            if !state_ids.contains(id) {
                state_ids.push(id.clone());
            }
        }
    }

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    // Compute extra horizontal space needed for notes on each side.
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

    // Try Sugiyama layout.
    let layout_positions = {
        let mut layout = LayoutGraph::new(Direction::TopToBottom);
        for id in &state_ids {
            let state_def = diagram.states.iter().find(|s| s.id == *id);
            let h = node_height(id, state_def);
            let w = if is_pseudo_state(id) {
                INITIAL_R * 2.0
            } else {
                let label = state_def.map_or(id.as_str(), |s| s.label.as_str());
                metrics::text_width(label, FONT_SIZE).max(STATE_WIDTH)
            };
            layout.add_node(id, id, w, h);
        }
        for t in &diagram.transitions {
            layout.add_edge(&t.from, &t.to, t.label.as_deref());
        }
        layout.layout_positions(std::time::Duration::from_secs(5))
    };

    let use_sugiyama = layout_positions
        .as_ref()
        .is_some_and(|p| p.len() >= state_ids.len());

    // Compute per-node heights and cumulative vertical positions.
    // Entry: (id, center_x, center_y, box_height)
    let (state_positions, total_width, total_height) = if use_sugiyama {
        let lp = layout_positions.as_ref().unwrap();
        let mut positions: Vec<(String, f64, f64, f64)> = Vec::new();
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (i, id) in state_ids.iter().enumerate() {
            let state_def = diagram.states.iter().find(|s| s.id == *id);
            let h = node_height(id, state_def);
            let w = if is_pseudo_state(id) {
                INITIAL_R * 2.0
            } else {
                let label = state_def.map_or(id.as_str(), |s| s.label.as_str());
                metrics::text_width(label, FONT_SIZE).max(STATE_WIDTH)
            };
            let x = lp[i].x + MARGIN + left_note_space.max(H_GAP) + w / 2.0;
            let y = lp[i].y + MARGIN + title_h + h / 2.0;
            positions.push((id.clone(), x, y, h));
            max_x = max_x.max(lp[i].x + w);
            max_y = max_y.max(lp[i].y + h);
        }
        let tw = MARGIN * 2.0 + left_note_space.max(H_GAP) + max_x + right_note_space.max(H_GAP);
        let th = max_y + MARGIN * 2.0 + title_h;
        (positions, tw, th)
    } else {
        // Vertical stacking fallback.
        let tw =
            MARGIN * 2.0 + left_note_space.max(H_GAP) + STATE_WIDTH + right_note_space.max(H_GAP);
        let cx = MARGIN + left_note_space.max(H_GAP) + STATE_WIDTH / 2.0;
        let mut positions: Vec<(String, f64, f64, f64)> = Vec::new();
        let mut y_cursor = title_h + MARGIN;
        for id in &state_ids {
            let state_def = diagram.states.iter().find(|s| s.id == *id);
            let h = node_height(id, state_def);
            let cy = y_cursor + h / 2.0;
            positions.push((id.clone(), cx, cy, h));
            y_cursor += h + V_GAP;
        }
        let th = y_cursor - V_GAP + MARGIN;
        (positions, tw, th)
    };

    // Compute a representative center x for note placement.
    let cx = state_positions
        .first()
        .map(|(_, x, _, _)| *x)
        .unwrap_or(MARGIN + left_note_space.max(H_GAP) + STATE_WIDTH / 2.0);

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Handwritten compatibility notice.
    let is_handwritten = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("handwritten") && sp.value.eq_ignore_ascii_case("true")
    });
    if is_handwritten {
        svg.monospace_text(
            10.0,
            SMALL_FONT + 2.0,
            "Please use '!option handwritten true' to enable handwritten",
            "start",
            10.0,
        );
    }

    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_width / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }

    let pos_of = |id: &str| -> (f64, f64, f64) {
        state_positions
            .iter()
            .find(|(sid, _, _, _)| sid == id)
            .map(|(_, x, y, h)| (*x, *y, *h))
            .unwrap_or((cx, MARGIN, state_box_height(0)))
    };

    // Draw states.
    for (id, x, y, h) in &state_positions {
        if id == "[*]" {
            // Initial/final state — filled circle.
            svg.circle(*x, *y, INITIAL_R, &sts.initial_color, &sts.initial_color);
        } else if id == "[H]" || id == "[H*]" {
            // Shallow/deep history pseudo-state — open circle with "H" (or "H*") inside.
            let label = if id == "[H*]" { "H*" } else { "H" };
            svg.circle(*x, *y, INITIAL_R, "white", "#000");
            svg.text(*x, *y + SMALL_FONT / 3.0, label, "middle", SMALL_FONT);
        } else {
            // Find the state definition.
            let state_def = diagram.states.iter().find(|s| s.id == *id);
            let label = state_def.map_or(id.as_str(), |s| s.label.as_str());
            let descriptions = state_def.map_or(&[][..], |s| s.descriptions.as_slice());

            let fill = match state_def.map(|s| s.kind) {
                Some(StateKind::Choice) => "#FFFACD",
                Some(StateKind::Fork | StateKind::Join) => "#333",
                _ => &sts.state_background,
            };

            // Draw the state box.
            svg.rounded_rect(
                *x - STATE_WIDTH / 2.0,
                *y - h / 2.0,
                STATE_WIDTH,
                *h,
                STATE_RX,
                fill,
                "#000",
            );

            if descriptions.is_empty() {
                // No descriptions: center the label vertically in the box.
                svg.text(*x, *y + FONT_SIZE / 3.0, label, "middle", FONT_SIZE);
            } else {
                // Header area: label centered in the top STATE_HEADER_HEIGHT portion.
                let box_top = *y - h / 2.0;
                let header_mid_y = box_top + STATE_HEADER_HEIGHT / 2.0 + FONT_SIZE / 3.0;
                svg.text(*x, header_mid_y, label, "middle", FONT_SIZE);

                // Divider line between header and descriptions.
                let divider_y = box_top + STATE_HEADER_HEIGHT;
                svg.line_segment(
                    *x - STATE_WIDTH / 2.0,
                    divider_y,
                    *x + STATE_WIDTH / 2.0,
                    divider_y,
                    "#000",
                    false,
                );

                // Description lines inside the box, left-aligned with a small margin.
                let text_x = *x - STATE_WIDTH / 2.0 + 5.0;
                for (j, desc) in descriptions.iter().enumerate() {
                    let desc_y =
                        divider_y + DESC_PADDING + (j as f64 + 1.0) * DESC_LINE_HEIGHT - 2.0;
                    svg.text(text_x, desc_y, desc, "start", SMALL_FONT);
                }
            }
        }
    }

    // Draw transitions.
    for t in &diagram.transitions {
        let (fx, fy, fh) = pos_of(&t.from);
        let (tx, ty, th) = pos_of(&t.to);

        let from_bottom = fy + fh / 2.0;
        let to_top = ty - th / 2.0;

        svg.line_segment(fx, from_bottom, tx, to_top, "#000", false);
        svg.arrow_head(tx, to_top, 90.0);

        if let Some(label) = &t.label {
            let mid_x = (fx + tx) / 2.0 + 10.0;
            let mid_y = (from_bottom + to_top) / 2.0;
            svg.text(mid_x, mid_y, label, "start", SMALL_FONT);
        }
    }

    // Draw notes.
    let note_fill = "#FEFFDD";
    let note_stroke = "#000";
    for note in &diagram.notes {
        let note_w = note_box_width(&note.text);
        let note_h = note_box_height(&note.text);

        let (note_x, note_y, anchor_x, anchor_y) = match &note.kind {
            StateNoteKind::RightOf(state_id) if !state_id.is_empty() => {
                let (sx, sy, sh) = pos_of(state_id);
                let nx = sx + STATE_WIDTH / 2.0 + NOTE_H_GAP;
                let ny = sy - sh / 2.0;
                // Connection line anchor: left edge of note → right edge of state
                (nx, ny, sx + STATE_WIDTH / 2.0, sy)
            }
            StateNoteKind::LeftOf(state_id) if !state_id.is_empty() => {
                let (sx, sy, sh) = pos_of(state_id);
                let nx = sx - STATE_WIDTH / 2.0 - NOTE_H_GAP - note_w;
                let ny = sy - sh / 2.0;
                // Connection line anchor: right edge of note → left edge of state
                (nx, ny, sx - STATE_WIDTH / 2.0, sy)
            }
            StateNoteKind::Floating => {
                // Place floating notes at the top-left, stacking vertically.
                // We use a simple fixed position for now.
                (MARGIN, MARGIN + title_h, cx, MARGIN + title_h)
            }
            StateNoteKind::OnLink => {
                // Place on the right side at mid-diagram height.
                let mid_y = total_height / 2.0;
                let nx = cx + STATE_WIDTH / 2.0 + NOTE_H_GAP;
                (nx, mid_y - note_h / 2.0, cx + STATE_WIDTH / 2.0, mid_y)
            }
            // Unanchored left/right (no "of <state>")
            StateNoteKind::RightOf(_) => {
                let mid_y = total_height / 2.0;
                let nx = cx + STATE_WIDTH / 2.0 + NOTE_H_GAP;
                (nx, mid_y - note_h / 2.0, cx + STATE_WIDTH / 2.0, mid_y)
            }
            StateNoteKind::LeftOf(_) => {
                let mid_y = total_height / 2.0;
                let nx = cx - STATE_WIDTH / 2.0 - NOTE_H_GAP - note_w;
                (nx, mid_y - note_h / 2.0, cx - STATE_WIDTH / 2.0, mid_y)
            }
        };

        // Connection line from note edge to state (skip for floating/on-link).
        match &note.kind {
            StateNoteKind::RightOf(id) if !id.is_empty() => {
                svg.line_segment(
                    note_x,
                    note_y + note_h / 2.0,
                    anchor_x,
                    anchor_y,
                    "#000",
                    true,
                );
            }
            StateNoteKind::LeftOf(id) if !id.is_empty() => {
                svg.line_segment(
                    note_x + note_w,
                    note_y + note_h / 2.0,
                    anchor_x,
                    anchor_y,
                    "#000",
                    true,
                );
            }
            _ => {}
        }

        svg.note_box(
            note_x,
            note_y,
            note_w,
            note_h,
            NOTE_EAR,
            note_fill,
            note_stroke,
        );
        let text_x = note_x + NOTE_PADDING;
        let text_start_y = note_y + NOTE_PADDING + SMALL_FONT;
        render_note_text(&mut svg, &note.text, text_x, text_start_y);
    }

    svg.finalize()
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
                },
                State {
                    id: "Inactive".into(),
                    label: "Inactive".into(),
                    kind: StateKind::Normal,
                    descriptions: vec![],
                    substates: vec![],
                },
            ],
            transitions: vec![
                Transition {
                    from: "[*]".into(),
                    to: "Active".into(),
                    label: None,
                },
                Transition {
                    from: "Active".into(),
                    to: "Inactive".into(),
                    label: Some("disable".into()),
                },
            ],
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Active"));
        assert!(svg.contains("Inactive"));
        assert!(svg.contains("disable"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\n[*] --> Active\nActive --> Inactive : disable\nInactive --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Active"));
        assert!(svg.contains("disable"));
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
        // The divider line should be present (descriptions render inside box).
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
}
