// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.

use rustuml_parser::diagram::state::*;

use crate::style::Theme;
use crate::svg::SvgBuilder;

const STATE_WIDTH: f64 = 100.0;
const STATE_HEIGHT: f64 = 40.0;
const STATE_RX: f64 = 15.0;
const INITIAL_R: f64 = 10.0;
const MARGIN: f64 = 30.0;
const V_GAP: f64 = 60.0;
const H_GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;

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

    // Simple vertical layout.
    let n = state_ids.len();
    let total_height = MARGIN * 2.0 + n as f64 * (STATE_HEIGHT + V_GAP);
    let total_width = MARGIN * 2.0 + STATE_WIDTH + H_GAP * 2.0;

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Position map: state_id -> (center_x, center_y)
    let cx = total_width / 2.0;
    let state_positions: Vec<(String, f64, f64)> = state_ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let cy = MARGIN + STATE_HEIGHT / 2.0 + i as f64 * (STATE_HEIGHT + V_GAP);
            (id.clone(), cx, cy)
        })
        .collect();

    let pos_of = |id: &str| -> (f64, f64) {
        state_positions
            .iter()
            .find(|(sid, _, _)| sid == id)
            .map(|(_, x, y)| (*x, *y))
            .unwrap_or((cx, MARGIN))
    };

    // Draw states.
    for (id, x, y) in &state_positions {
        if id == "[*]" {
            // Initial/final state — filled circle.
            svg.circle(*x, *y, INITIAL_R, &sts.initial_color, &sts.initial_color);
        } else {
            // Find the state definition.
            let state_def = diagram.states.iter().find(|s| s.id == *id);
            let label = state_def.map_or(id.as_str(), |s| s.label.as_str());

            let fill = match state_def.map(|s| s.kind) {
                Some(StateKind::Choice) => "#FFFACD",
                Some(StateKind::Fork | StateKind::Join) => "#333",
                _ => &sts.state_background,
            };

            svg.rounded_rect(
                *x - STATE_WIDTH / 2.0,
                *y - STATE_HEIGHT / 2.0,
                STATE_WIDTH,
                STATE_HEIGHT,
                STATE_RX,
                fill,
                "#000",
            );
            svg.text(*x, *y + 5.0, label, "middle", FONT_SIZE);

            // Draw descriptions below the state.
            if let Some(s) = state_def {
                for (j, desc) in s.descriptions.iter().enumerate() {
                    svg.text(
                        *x,
                        *y + STATE_HEIGHT / 2.0 + 15.0 + j as f64 * 14.0,
                        desc,
                        "middle",
                        SMALL_FONT,
                    );
                }
            }
        }
    }

    // Draw transitions.
    for t in &diagram.transitions {
        let (fx, fy) = pos_of(&t.from);
        let (tx, ty) = pos_of(&t.to);

        let from_bottom = fy + STATE_HEIGHT / 2.0;
        let to_top = ty - STATE_HEIGHT / 2.0;

        svg.line_segment(fx, from_bottom, tx, to_top, "#000", false);
        svg.arrow_head(tx, to_top, 90.0);

        if let Some(label) = &t.label {
            let mid_x = (fx + tx) / 2.0 + 10.0;
            let mid_y = (from_bottom + to_top) / 2.0;
            svg.text(mid_x, mid_y, label, "start", SMALL_FONT);
        }
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
}
