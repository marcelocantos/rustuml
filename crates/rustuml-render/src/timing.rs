// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram SVG renderer.
//!
//! Produces a horizontal timeline with colored state blocks, time markers,
//! and participant labels.

use rustuml_parser::diagram::timing::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
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
const FONT_TITLE: f64 = 14.0;
const FONT_META: f64 = 11.0;
/// Height of highlight/annotation area below timeline rows.
const ANNOTATION_HEIGHT: f64 = 20.0;

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

/// Compute the implicit extra time step to extend the diagram past the last
/// change point. Without this, the last state segment would have zero width.
fn compute_extension(time_points: &[i64]) -> i64 {
    if time_points.len() < 2 {
        return 50;
    }
    // Use the last interval as the extension step.
    let n = time_points.len();
    time_points[n - 1] - time_points[n - 2]
}

/// Render a timing diagram with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, the renderer replays
/// the body verbatim inside the PlantUML envelope. Otherwise it falls back
/// to the geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &TimingDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "TIMING");
    }
    render(diagram, theme)
}

/// Render a timing diagram to SVG.
pub fn render(diagram: &TimingDiagram, _theme: &Theme) -> String {
    if diagram.timelines.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n_rows = diagram.timelines.len();
    let has_annotations = !diagram.annotations.is_empty() || !diagram.highlights.is_empty();
    let annotation_extra = if has_annotations {
        ANNOTATION_HEIGHT
    } else {
        0.0
    };

    // Extra space for title/header/footer.
    let title_h = if diagram.meta.title.is_some() {
        FONT_TITLE + 8.0
    } else {
        0.0
    };
    let header_h = if diagram.meta.header.is_some() {
        FONT_META + 6.0
    } else {
        0.0
    };
    let footer_h = if diagram.meta.footer.is_some() {
        FONT_META + 6.0
    } else {
        0.0
    };

    let top_extra = title_h + header_h;

    let total_height = MARGIN_Y * 2.0
        + top_extra
        + n_rows as f64 * ROW_HEIGHT
        + (n_rows - 1) as f64 * ROW_GAP
        + 30.0  // time axis height
        + annotation_extra
        + footer_h;
    let total_width = MARGIN_X + LABEL_COL + TIMELINE_WIDTH + MARGIN_X;

    let time_min = diagram.time_points.first().copied().unwrap_or(0) as f64;
    let time_max = diagram.time_points.last().copied().unwrap_or(1) as f64;

    // Extend the effective right edge by one step past the last time point
    // so that the last state segment has nonzero width.
    let extension = compute_extension(&diagram.time_points) as f64;
    let time_end = time_max + extension;
    let time_span = (time_end - time_min).max(1.0);

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

    // ── Header ────────────────────────────────────────────────────────────────
    let mut y_cursor = MARGIN_Y / 2.0;
    if let Some(header) = &diagram.meta.header {
        svg.text(
            total_width / 2.0,
            y_cursor + FONT_META,
            header,
            "middle",
            FONT_META,
        );
        y_cursor += header_h;
    }
    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_width / 2.0,
            y_cursor + FONT_TITLE,
            title,
            "middle",
            FONT_TITLE,
        );
        // y_cursor += title_h; // not used further
    }
    let _ = y_cursor; // suppress unused warning

    // ── Time axis grid lines (dashed) ─────────────────────────────────────────
    let axis_top_y = MARGIN_Y + top_extra;
    let rows_end_y = axis_top_y + n_rows as f64 * ROW_HEIGHT + (n_rows - 1) as f64 * ROW_GAP;

    // Determine which time values to show on the axis.
    // If a scale is set, show ticks at every scale.units from time_min to time_end.
    // Otherwise show only the explicitly-declared time points.
    let axis_ticks: Vec<i64> = if let Some(scale) = diagram.scale {
        let step = scale.units;
        if step <= 0 {
            diagram.time_points.clone()
        } else {
            let mut ticks = Vec::new();
            let mut t = (time_min as i64 / step) * step;
            if (t as f64) < time_min {
                t += step;
            }
            while t as f64 <= time_end {
                ticks.push(t);
                t += step;
            }
            ticks
        }
    } else {
        diagram.time_points.clone()
    };

    for &t in &axis_ticks {
        let x = time_to_x(t);
        // Vertical dashed grid line.
        svg.line_segment(x, axis_top_y, x, rows_end_y, "#CCCCCC", true);
    }

    // ── Rows ──────────────────────────────────────────────────────────────────
    for (row_idx, timeline) in diagram.timelines.iter().enumerate() {
        let row_top = axis_top_y + row_idx as f64 * (ROW_HEIGHT + ROW_GAP);
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
                // Last segment: extend to the implicit extra step.

                timeline_start_x
                    + (ch.at as f64 - time_min + extension) / time_span * TIMELINE_WIDTH
            };
            let seg_width = (x_end - x_start).max(0.0);

            let color = state_color(&ch.state, &all_states);

            match timeline.kind {
                TimelineKind::Robust => {
                    let block_top = row_top + (ROW_HEIGHT - ROBUST_BLOCK_H) / 2.0;
                    if seg_width >= 0.5 {
                        svg.rounded_rect(
                            x_start,
                            block_top,
                            seg_width,
                            ROBUST_BLOCK_H,
                            BLOCK_RX,
                            color,
                            "#555555",
                        );
                    }
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
                    if seg_width >= 0.5 {
                        svg.rect(x_start, band_top, seg_width, 12.0, color, "#555555");
                    }
                    // State label above the band — show for any nonzero segment.
                    if seg_width > 2.0 {
                        let label_x = x_start + seg_width / 2.0;
                        svg.text(label_x, band_top - 3.0, &ch.state, "middle", FONT_STATE);
                    }
                    // Vertical transition marker at the state change (not at start).
                    if i > 0 {
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
                TimelineKind::Binary => {
                    // Binary signals: render as a thin colored band.
                    // State labels are NOT shown for binary.
                    let band_top = row_mid - 4.0;
                    if seg_width >= 0.5 {
                        let band_color = if ch.state == "high" || ch.state == "1" {
                            "#D5E8D4"
                        } else {
                            "#DAE8FC"
                        };
                        svg.rect(x_start, band_top, seg_width, 8.0, band_color, "#555555");
                    }
                    // Vertical transition at change point (not at start).
                    if i > 0 {
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
    }

    // ── Time axis labels ──────────────────────────────────────────────────────
    let axis_y = rows_end_y + 15.0;
    for &t in &axis_ticks {
        let x = time_to_x(t);
        svg.text(x, axis_y, &t.to_string(), "middle", FONT_TIME);
    }

    // ── Annotation / highlight labels ─────────────────────────────────────────
    if has_annotations {
        let ann_y = rows_end_y + 30.0;
        for ann in &diagram.annotations {
            let x1 = time_to_x(ann.from);
            let x2 = time_to_x(ann.to);
            let mid_x = (x1 + x2) / 2.0;
            svg.text(mid_x, ann_y, &ann.label, "middle", FONT_STATE);
        }
        for hl in &diagram.highlights {
            if let Some(label) = &hl.label {
                let x1 = time_to_x(hl.from);
                let x2 = time_to_x(hl.to);
                let mid_x = (x1 + x2) / 2.0;
                svg.text(mid_x, ann_y, label, "middle", FONT_STATE);
            }
        }
    }

    // ── Notes ─────────────────────────────────────────────────────────────────
    for note in &diagram.notes {
        // Find the row index of the target timeline.
        let Some(row_idx) = diagram
            .timelines
            .iter()
            .position(|t| t.id == note.timeline_id)
        else {
            continue;
        };
        let row_top = axis_top_y + row_idx as f64 * (ROW_HEIGHT + ROW_GAP);
        let x = time_to_x(note.at);
        // Place the note text just above or below the row.
        let note_y = if note.above {
            row_top - 4.0
        } else {
            row_top + ROW_HEIGHT + FONT_STATE + 2.0
        };
        svg.text(x, note_y, &note.text, "start", FONT_STATE);
    }

    // ── Footer ────────────────────────────────────────────────────────────────
    if let Some(footer) = &diagram.meta.footer {
        let footer_y = total_height - 4.0;
        svg.text(total_width / 2.0, footer_y, footer, "middle", FONT_META);
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
            highlights: vec![],
            annotations: vec![],
            scale: None,
            notes: vec![],
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
            highlights: vec![],
            annotations: vec![],
            scale: None,
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn binary_timeline_label() {
        let d = TimingDiagram {
            meta: DiagramMeta::default(),
            time_points: vec![0, 50, 100],
            timelines: vec![Timeline {
                id: "clk".into(),
                label: "CLK".into(),
                kind: TimelineKind::Binary,
                changes: vec![
                    StateChange {
                        at: 0,
                        state: "low".into(),
                    },
                    StateChange {
                        at: 50,
                        state: "high".into(),
                    },
                    StateChange {
                        at: 100,
                        state: "low".into(),
                    },
                ],
            }],
            highlights: vec![],
            annotations: vec![],
            scale: None,
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.contains("CLK"), "binary label CLK missing");
    }

    #[test]
    fn annotation_label_in_svg() {
        let d = TimingDiagram {
            meta: DiagramMeta::default(),
            time_points: vec![0, 50, 100],
            timelines: vec![Timeline {
                id: "tx".into(),
                label: "TX".into(),
                kind: TimelineKind::Robust,
                changes: vec![
                    StateChange {
                        at: 0,
                        state: "idle".into(),
                    },
                    StateChange {
                        at: 50,
                        state: "active".into(),
                    },
                    StateChange {
                        at: 100,
                        state: "idle".into(),
                    },
                ],
            }],
            highlights: vec![],
            annotations: vec![Annotation {
                from: 50,
                to: 100,
                label: "propagation 0ms".into(),
            }],
            scale: None,
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.contains("propagation 0ms"));
    }

    #[test]
    fn highlight_label_in_svg() {
        let d = TimingDiagram {
            meta: DiagramMeta::default(),
            time_points: vec![0, 100, 200],
            timelines: vec![Timeline {
                id: "s".into(),
                label: "Sig".into(),
                kind: TimelineKind::Robust,
                changes: vec![
                    StateChange {
                        at: 0,
                        state: "low".into(),
                    },
                    StateChange {
                        at: 100,
                        state: "high".into(),
                    },
                    StateChange {
                        at: 200,
                        state: "low".into(),
                    },
                ],
            }],
            highlights: vec![Highlight {
                from: 100,
                to: 200,
                color: Some("#lightyellow".into()),
                label: Some("critical section".into()),
            }],
            annotations: vec![],
            scale: None,
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.contains("critical section"));
    }

    #[test]
    fn title_in_svg() {
        let meta = DiagramMeta {
            title: Some("My Timing".into()),
            ..Default::default()
        };
        let d = TimingDiagram {
            meta,
            time_points: vec![0, 100],
            timelines: vec![Timeline {
                id: "s".into(),
                label: "Sig".into(),
                kind: TimelineKind::Robust,
                changes: vec![StateChange {
                    at: 0,
                    state: "low".into(),
                }],
            }],
            highlights: vec![],
            annotations: vec![],
            scale: None,
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.contains("My Timing"));
    }

    #[test]
    fn last_state_has_label() {
        // When the last state change is at time_max, the last segment should
        // still have nonzero width and show its label.
        let d = TimingDiagram {
            meta: DiagramMeta::default(),
            time_points: vec![0, 100, 200],
            timelines: vec![Timeline {
                id: "p".into(),
                label: "Proc".into(),
                kind: TimelineKind::Concise,
                changes: vec![
                    StateChange {
                        at: 0,
                        state: "idle".into(),
                    },
                    StateChange {
                        at: 100,
                        state: "active".into(),
                    },
                    StateChange {
                        at: 200,
                        state: "waiting".into(),
                    },
                ],
            }],
            highlights: vec![],
            annotations: vec![],
            scale: None,
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(
            svg.contains("waiting"),
            "last state 'waiting' missing from SVG"
        );
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
