// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram SVG renderer.
//!
//! Produces a horizontal timeline with colored state blocks, time markers,
//! and participant labels.

use rustuml_parser::diagram::timing::*;

use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ─────────────────────────────────────────────────────────
/// Width of the participant label column on the left.
const LABEL_COL: f64 = 120.0;
/// Height of each timeline row.
const ROW_HEIGHT: f64 = 40.0;
/// Vertical gap between rows.
const ROW_GAP: f64 = 20.0;
/// Top/bottom margin.
const MARGIN_Y: f64 = 30.0;
/// Left/right margin (outside the label column).
const MARGIN_X: f64 = 20.0;
/// Width of the drawable timeline area per unit of time is computed
/// dynamically from the total time span.
const TIMELINE_WIDTH: f64 = 600.0;
/// Height of a robust state block.
const ROBUST_BLOCK_H: f64 = 30.0;
/// Radius for robust block corners.
const BLOCK_RX: f64 = 4.0;
/// Font sizes.
const FONT_LABEL: f64 = 13.0;
const FONT_STATE: f64 = 11.0;
const FONT_TIME: f64 = 10.0;

/// A palette of colors cycled for successive state names.
const STATE_COLORS: &[&str] = &[
    "#D5E8D4", // soft green
    "#DAE8FC", // soft blue
    "#FFE6CC", // soft orange
    "#F8CECC", // soft red
    "#E1D5E7", // soft purple
    "#FFF2CC", // soft yellow
    "#D0E0E3", // soft teal
    "#FCE4D6", // soft salmon
];

/// Render a timing diagram to SVG.
pub fn render(diagram: &TimingDiagram, _theme: &Theme) -> String {
    if diagram.timelines.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n_rows = diagram.timelines.len();
    let total_height =
        MARGIN_Y * 2.0 + n_rows as f64 * ROW_HEIGHT + (n_rows - 1) as f64 * ROW_GAP + 30.0;
    let total_width = MARGIN_X + LABEL_COL + TIMELINE_WIDTH + MARGIN_X;

    let time_min = diagram.time_points.first().copied().unwrap_or(0) as f64;
    let time_max = diagram.time_points.last().copied().unwrap_or(1) as f64;
    let time_span = (time_max - time_min).max(1.0);

    // Map a time value to an x coordinate on the timeline.
    let timeline_start_x = MARGIN_X + LABEL_COL;
    let time_to_x =
        |t: i64| -> f64 { timeline_start_x + (t as f64 - time_min) / time_span * TIMELINE_WIDTH };

    // Build a deterministic state→color mapping for each timeline.
    let state_color = |state: &str, all_states: &[String]| -> &'static str {
        let idx = all_states.iter().position(|s| s == state).unwrap_or(0);
        STATE_COLORS[idx % STATE_COLORS.len()]
    };

    let mut svg = SvgBuilder::new(total_width, total_height);

    // ── Background ────────────────────────────────────────────────────────────
    svg.rect(0.0, 0.0, total_width, total_height, "#FFFFFF", "none");

    // ── Time axis markers ─────────────────────────────────────────────────────
    let axis_y = MARGIN_Y / 2.0 + 10.0;
    for &t in &diagram.time_points {
        let x = time_to_x(t);
        // Tick line spanning full height.
        svg.line_segment(x, axis_y, x, total_height - MARGIN_Y / 2.0, "#CCCCCC", true);
        // Time label.
        svg.text(x, axis_y, &t.to_string(), "middle", FONT_TIME);
    }

    // ── Rows ──────────────────────────────────────────────────────────────────
    for (row_idx, timeline) in diagram.timelines.iter().enumerate() {
        let row_top = MARGIN_Y + row_idx as f64 * (ROW_HEIGHT + ROW_GAP);
        let row_mid = row_top + ROW_HEIGHT / 2.0;

        // Participant label (right-aligned in the label column).
        svg.text(
            MARGIN_X + LABEL_COL - 8.0,
            row_mid + 5.0,
            &timeline.label,
            "end",
            FONT_LABEL,
        );

        // Horizontal baseline.
        svg.line_segment(
            timeline_start_x,
            row_mid,
            timeline_start_x + TIMELINE_WIDTH,
            row_mid,
            "#999999",
            false,
        );

        // Collect all unique state names for this timeline (in order of first appearance).
        let mut all_states: Vec<String> = Vec::new();
        for ch in &timeline.changes {
            if !all_states.contains(&ch.state) {
                all_states.push(ch.state.clone());
            }
        }

        // Draw state segments between consecutive changes.
        let changes = &timeline.changes;
        for i in 0..changes.len() {
            let ch = &changes[i];
            let x_start = time_to_x(ch.at);
            let x_end = if i + 1 < changes.len() {
                time_to_x(changes[i + 1].at)
            } else {
                timeline_start_x + TIMELINE_WIDTH
            };
            let seg_width = (x_end - x_start).max(0.0);
            if seg_width < 0.5 {
                continue;
            }

            let color = state_color(&ch.state, &all_states);

            match timeline.kind {
                TimelineKind::Robust => {
                    let block_top = row_top + (ROW_HEIGHT - ROBUST_BLOCK_H) / 2.0;
                    svg.rounded_rect(
                        x_start,
                        block_top,
                        seg_width,
                        ROBUST_BLOCK_H,
                        BLOCK_RX,
                        color,
                        "#555555",
                    );
                    // State label centred inside block (clip long names).
                    if seg_width > 20.0 {
                        let label_x = x_start + seg_width / 2.0;
                        let label_y = block_top + ROBUST_BLOCK_H / 2.0 + 4.0;
                        svg.text(label_x, label_y, &ch.state, "middle", FONT_STATE);
                    }
                }
                TimelineKind::Concise => {
                    // For concise, draw a thin colored band and state label above the line.
                    let band_top = row_mid - 6.0;
                    svg.rect(x_start, band_top, seg_width, 12.0, color, "#555555");
                    // State label above the band.
                    if seg_width > 20.0 {
                        let label_x = x_start + seg_width / 2.0;
                        svg.text(label_x, band_top - 3.0, &ch.state, "middle", FONT_STATE);
                    }
                    // Vertical transition marker at the state change.
                    svg.line_segment(
                        x_start,
                        row_top + 5.0,
                        x_start,
                        row_top + ROW_HEIGHT - 5.0,
                        "#555555",
                        false,
                    );
                }
            }
        }
    }

    svg.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn make_diagram() -> TimingDiagram {
        TimingDiagram {
            meta: DiagramMeta::default(),
            time_points: vec![0, 100, 300],
            timelines: vec![
                Timeline {
                    id: "W".into(),
                    label: "Web".into(),
                    kind: TimelineKind::Robust,
                    changes: vec![
                        StateChange {
                            at: 0,
                            state: "Idle".into(),
                        },
                        StateChange {
                            at: 100,
                            state: "Processing".into(),
                        },
                        StateChange {
                            at: 300,
                            state: "Idle".into(),
                        },
                    ],
                },
                Timeline {
                    id: "U".into(),
                    label: "User".into(),
                    kind: TimelineKind::Concise,
                    changes: vec![
                        StateChange {
                            at: 0,
                            state: "Idle".into(),
                        },
                        StateChange {
                            at: 100,
                            state: "Waiting".into(),
                        },
                        StateChange {
                            at: 300,
                            state: "Idle".into(),
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn renders_to_svg() {
        let svg = render(&make_diagram(), &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn contains_participant_labels() {
        let svg = render(&make_diagram(), &Theme::default());
        assert!(svg.contains("Web"));
        assert!(svg.contains("User"));
    }

    #[test]
    fn contains_state_names() {
        let svg = render(&make_diagram(), &Theme::default());
        assert!(svg.contains("Idle"));
        assert!(svg.contains("Processing"));
        assert!(svg.contains("Waiting"));
    }

    #[test]
    fn contains_time_markers() {
        let svg = render(&make_diagram(), &Theme::default());
        assert!(svg.contains(">0<"));
        assert!(svg.contains(">100<"));
        assert!(svg.contains(">300<"));
    }

    #[test]
    fn empty_diagram() {
        let d = TimingDiagram {
            meta: DiagramMeta::default(),
            timelines: vec![],
            time_points: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\n\
            robust \"Web\" as W\n\
            concise \"User\" as U\n\
            @0\n\
            W is Idle\n\
            U is Idle\n\
            @100\n\
            W is Processing\n\
            U is Waiting\n\
            @300\n\
            W is Idle\n\
            U is Idle\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Web"));
        assert!(svg.contains("Idle"));
    }
}
