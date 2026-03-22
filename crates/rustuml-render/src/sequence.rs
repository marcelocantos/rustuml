// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram SVG renderer.

use rustuml_parser::diagram::sequence::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const MIN_PARTICIPANT_WIDTH: f64 = 60.0;
const PARTICIPANT_HEIGHT: f64 = 35.0;
const PARTICIPANT_GAP: f64 = 50.0;
const PARTICIPANT_PADDING: f64 = 16.0;
const MESSAGE_HEIGHT: f64 = 40.0;
const TOP_MARGIN: f64 = 20.0;
const LEFT_MARGIN: f64 = 20.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;

/// Render a sequence diagram to SVG.
pub fn render(diagram: &SequenceDiagram, theme: &Theme) -> String {
    let ss = &theme.sequence;
    let n = diagram.participants.len().max(1);

    // Calculate participant widths based on text metrics.
    let participant_widths: Vec<f64> = diagram
        .participants
        .iter()
        .map(|p| {
            let text_w = metrics::text_width(&p.label, FONT_SIZE);
            (text_w + PARTICIPANT_PADDING * 2.0).max(MIN_PARTICIPANT_WIDTH)
        })
        .collect();

    let total_participant_width: f64 = participant_widths.iter().sum();
    let total_width =
        LEFT_MARGIN * 2.0 + total_participant_width + ((n - 1) as f64) * PARTICIPANT_GAP;

    // Count events that consume vertical space.
    let event_count = diagram
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::Message(_)
                    | Event::Divider(_)
                    | Event::Delay(_)
                    | Event::Space(_)
                    | Event::Note(_)
            )
        })
        .count();
    let total_height =
        TOP_MARGIN * 2.0 + PARTICIPANT_HEIGHT * 2.0 + (event_count as f64) * MESSAGE_HEIGHT + 20.0;

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Build participant x-coordinate map (left edge of each box).
    let mut px: Vec<f64> = Vec::with_capacity(n);
    let mut x = LEFT_MARGIN;
    for (i, _) in diagram.participants.iter().enumerate() {
        px.push(x);
        x += participant_widths[i] + PARTICIPANT_GAP;
    }
    if px.is_empty() {
        px.push(LEFT_MARGIN);
    }

    let participant_center = |id: &str| -> f64 {
        diagram
            .participants
            .iter()
            .position(|p| p.id == id)
            .map(|i| px[i] + participant_widths[i] / 2.0)
            .unwrap_or(0.0)
    };

    // Draw participant boxes at top.
    let box_y = TOP_MARGIN;
    for (i, p) in diagram.participants.iter().enumerate() {
        let x = px[i];
        let w = participant_widths[i];
        svg.rounded_rect(
            x,
            box_y,
            w,
            PARTICIPANT_HEIGHT,
            5.0,
            &ss.participant_background,
            &ss.participant_border,
        );
        svg.text(
            x + w / 2.0,
            box_y + PARTICIPANT_HEIGHT / 2.0 + 5.0,
            &p.label,
            "middle",
            FONT_SIZE,
        );
    }

    // Draw lifelines.
    let lifeline_start = box_y + PARTICIPANT_HEIGHT;
    let lifeline_end = total_height - TOP_MARGIN - PARTICIPANT_HEIGHT;
    for (i, _p) in diagram.participants.iter().enumerate() {
        let cx = px[i] + participant_widths[i] / 2.0;
        svg.line_segment(
            cx,
            lifeline_start,
            cx,
            lifeline_end,
            &ss.lifeline_color,
            true,
        );
    }

    // Render events.
    let mut y = lifeline_start + 20.0;

    for event in &diagram.events {
        match event {
            Event::Message(msg) => {
                let from_x = participant_center(&msg.from);
                let to_x = participant_center(&msg.to);

                // Handle external messages.
                let from_x = if msg.from == "[" { 0.0 } else { from_x };
                let to_x = if msg.to == "]" { total_width } else { to_x };

                let dashed = msg.arrow.line == LineStyle::Dotted;
                svg.line_segment(from_x, y, to_x, y, &ss.participant_border, dashed);

                // Arrow head (pointing right or left).
                if to_x > from_x {
                    svg.arrow_head(to_x, y, 0.0);
                } else {
                    svg.arrow_head(to_x, y, 180.0);
                }

                // Label.
                if !msg.label.is_empty() {
                    let mid_x = (from_x + to_x) / 2.0;
                    svg.text(mid_x, y - 5.0, &msg.label, "middle", SMALL_FONT);
                }

                y += MESSAGE_HEIGHT;
            }
            Event::Divider(text) => {
                svg.line_segment(
                    LEFT_MARGIN,
                    y,
                    total_width - LEFT_MARGIN,
                    y,
                    &ss.lifeline_color,
                    true,
                );
                let mid = total_width / 2.0;
                svg.text(mid, y - 3.0, text, "middle", SMALL_FONT);
                y += MESSAGE_HEIGHT;
            }
            Event::Delay(text) => {
                if let Some(t) = text {
                    let mid = total_width / 2.0;
                    svg.text(mid, y + 5.0, t, "middle", SMALL_FONT);
                }
                y += MESSAGE_HEIGHT;
            }
            Event::Space(px_opt) => {
                y += px_opt.map(|p| p as f64).unwrap_or(20.0);
            }
            Event::Note(note) => {
                let anchor_x = if let Some(first) = note.participants.first() {
                    participant_center(first)
                } else {
                    total_width / 2.0
                };

                let note_w = 120.0;
                let note_h = 25.0;
                let note_x = match note.position {
                    NotePosition::Right => anchor_x + 20.0,
                    NotePosition::Left => anchor_x - note_w - 20.0,
                    NotePosition::Over => anchor_x - note_w / 2.0,
                };

                svg.rect(
                    note_x,
                    y - note_h / 2.0,
                    note_w,
                    note_h,
                    &ss.note_background,
                    &ss.participant_border,
                );
                svg.text(
                    note_x + PARTICIPANT_PADDING,
                    y + 4.0,
                    &note.text,
                    "start",
                    SMALL_FONT,
                );
                y += MESSAGE_HEIGHT;
            }
            Event::GroupStart(g) => {
                svg.open_group("group");
                svg.rect(
                    LEFT_MARGIN - 5.0,
                    y - 5.0,
                    total_width - LEFT_MARGIN * 2.0 + 10.0,
                    20.0,
                    "none",
                    &ss.lifeline_color,
                );
                let label_text = match &g.label {
                    Some(l) => format!("{:?} {l}", g.kind),
                    None => format!("{:?}", g.kind),
                };
                svg.text(LEFT_MARGIN, y + 10.0, &label_text, "start", SMALL_FONT);
            }
            Event::GroupElse(g) => {
                let label = g.label.as_deref().unwrap_or("else");
                svg.line_segment(
                    LEFT_MARGIN - 5.0,
                    y,
                    total_width - LEFT_MARGIN + 5.0,
                    y,
                    &ss.lifeline_color,
                    true,
                );
                svg.text(LEFT_MARGIN, y + 12.0, label, "start", SMALL_FONT);
            }
            Event::GroupEnd => {
                svg.close_group();
            }
            Event::Return(ret) => {
                // Simplified: draw as a dotted return to the previous sender.
                if !ret.label.is_empty() {
                    let mid = total_width / 2.0;
                    svg.text(mid, y - 5.0, &ret.label, "middle", SMALL_FONT);
                }
                y += MESSAGE_HEIGHT;
            }
            _ => {}
        }
    }

    // Draw participant boxes at bottom.
    let bottom_y = total_height - TOP_MARGIN - PARTICIPANT_HEIGHT;
    for (i, p) in diagram.participants.iter().enumerate() {
        let x = px[i];
        let w = participant_widths[i];
        svg.rounded_rect(
            x,
            bottom_y,
            w,
            PARTICIPANT_HEIGHT,
            5.0,
            &ss.participant_background,
            &ss.participant_border,
        );
        svg.text(
            x + w / 2.0,
            bottom_y + PARTICIPANT_HEIGHT / 2.0 + 5.0,
            &p.label,
            "middle",
            FONT_SIZE,
        );
    }

    svg.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_diagram() -> SequenceDiagram {
        SequenceDiagram {
            meta: DiagramMeta::default(),
            participants: vec![
                Participant {
                    id: "Alice".into(),
                    label: "Alice".into(),
                    kind: ParticipantKind::Participant,
                    order: Some(0),
                },
                Participant {
                    id: "Bob".into(),
                    label: "Bob".into(),
                    kind: ParticipantKind::Participant,
                    order: Some(1),
                },
            ],
            events: vec![Event::Message(Message {
                from: "Alice".into(),
                to: "Bob".into(),
                label: "hello".into(),
                arrow: Arrow {
                    line: LineStyle::Solid,
                    head: ArrowHead::Filled,
                    direction: ArrowDirection::LeftToRight,
                },
                activation: None,
            })],
            autonumber: None,
        }
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_diagram(), &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("Bob"));
        assert!(svg.contains("hello"));
    }

    #[test]
    fn has_participant_boxes() {
        let svg = render(&simple_diagram(), &Theme::default());
        // Two boxes at top, two at bottom.
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 4,
            "should have at least 4 rects (participant boxes), got {rect_count}"
        );
    }

    #[test]
    fn has_lifelines() {
        let svg = render(&simple_diagram(), &Theme::default());
        // Dashed vertical lines.
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn has_arrow() {
        let svg = render(&simple_diagram(), &Theme::default());
        assert!(svg.contains("<polygon"), "should have arrow head polygon");
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nAlice -> Bob : hello\nBob --> Alice : hi\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Alice"));
        assert!(svg.contains("hello"));
        assert!(svg.contains("hi"));
    }
}
