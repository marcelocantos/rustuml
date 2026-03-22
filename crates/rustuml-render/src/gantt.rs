// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Gantt chart SVG renderer.
//!
//! Produces a horizontal bar chart with:
//! - A time axis at the top (day numbers).
//! - One row per task, each row showing a filled bar.
//! - Dependency arrows from the end of a predecessor to the start of its
//!   successor.
//! - Task labels to the left of the chart area.

use rustuml_parser::diagram::gantt::{GanttDiagram, GanttTask, TaskStart};

use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ──────────────────────────────────────────────────────────

const LABEL_WIDTH: f64 = 120.0;
const DAY_WIDTH: f64 = 22.0;
const ROW_HEIGHT: f64 = 28.0;
const ROW_GAP: f64 = 8.0;
const AXIS_HEIGHT: f64 = 30.0;
const MARGIN: f64 = 16.0;
const BAR_H: f64 = 18.0;
const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 10.0;

// Bar colours — cycle through these for visual distinction.
const BAR_COLORS: &[&str] = &[
    "#5B9BD5", "#ED7D31", "#A9D18E", "#FFC000", "#9B59B6", "#1ABC9C", "#E74C3C",
];

/// Render a Gantt diagram to SVG.
pub fn render(diagram: &GanttDiagram, _theme: &Theme) -> String {
    if diagram.tasks.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"60\"></svg>\n"
            .to_string();
    }

    // ── Resolve absolute start days ───────────────────────────────────────────

    let resolved = resolve_starts(&diagram.tasks);

    // Total project duration.
    let total_days = resolved
        .iter()
        .map(|&(start, dur)| start + dur)
        .max()
        .unwrap_or(1);

    // ── Geometry ──────────────────────────────────────────────────────────────

    let chart_width = total_days as f64 * DAY_WIDTH;
    let n = diagram.tasks.len();
    let total_width = MARGIN + LABEL_WIDTH + chart_width + MARGIN;
    let total_height = MARGIN + AXIS_HEIGHT + n as f64 * (ROW_HEIGHT + ROW_GAP) + MARGIN;

    let mut svg = SvgBuilder::new(total_width, total_height);

    // ── Axis ──────────────────────────────────────────────────────────────────

    let axis_y = MARGIN + AXIS_HEIGHT;
    let chart_x = MARGIN + LABEL_WIDTH;

    // Axis baseline.
    svg.line_segment(
        chart_x,
        axis_y,
        chart_x + chart_width,
        axis_y,
        "#888",
        false,
    );

    // Day tick marks and labels.
    for day in 0..=total_days {
        let tx = chart_x + day as f64 * DAY_WIDTH;
        svg.line_segment(tx, axis_y, tx, axis_y - 5.0, "#888", false);
        if day < total_days {
            svg.text(
                tx + DAY_WIDTH / 2.0,
                axis_y - 8.0,
                &(day + 1).to_string(),
                "middle",
                SMALL_FONT,
            );
        }
    }

    // Vertical grid lines (subtle).
    for day in 0..=total_days {
        let gx = chart_x + day as f64 * DAY_WIDTH;
        let grid_bottom = axis_y + n as f64 * (ROW_HEIGHT + ROW_GAP);
        svg.line_segment(gx, axis_y, gx, grid_bottom, "#E0E0E0", false);
    }

    // ── Rows ──────────────────────────────────────────────────────────────────

    // Build a name -> row_index map for dependency arrows.
    let row_y: Vec<f64> = (0..n)
        .map(|i| axis_y + ROW_GAP / 2.0 + i as f64 * (ROW_HEIGHT + ROW_GAP))
        .collect();

    for (i, task) in diagram.tasks.iter().enumerate() {
        let (start_day, dur) = resolved[i];
        let bar_x = chart_x + start_day as f64 * DAY_WIDTH;
        let bar_w = dur as f64 * DAY_WIDTH;
        let bar_y = row_y[i] + (ROW_HEIGHT - BAR_H) / 2.0;

        let color = BAR_COLORS[i % BAR_COLORS.len()];

        // Task label (left panel).
        svg.text(
            MARGIN + LABEL_WIDTH - 6.0,
            row_y[i] + ROW_HEIGHT / 2.0 + 4.0,
            &task.name,
            "end",
            FONT_SIZE,
        );

        // Bar.
        svg.rounded_rect(bar_x, bar_y, bar_w.max(2.0), BAR_H, 3.0, color, "#333");

        // Duration label inside the bar (if it fits).
        let label = format!("{} d", dur);
        let label_x = bar_x + bar_w / 2.0;
        let label_y = bar_y + BAR_H / 2.0 + 4.0;
        if bar_w >= 24.0 {
            svg.text(label_x, label_y, &label, "middle", SMALL_FONT);
        }
    }

    // ── Dependency arrows ─────────────────────────────────────────────────────

    for (i, task) in diagram.tasks.iter().enumerate() {
        if let TaskStart::AfterTask(dep_name) = &task.start
            && let Some(dep_idx) = diagram.tasks.iter().position(|t| &t.name == dep_name)
        {
            let (dep_start, dep_dur) = resolved[dep_idx];
            // Arrow from end of predecessor bar to start of this bar.
            let ax1 = chart_x + (dep_start + dep_dur) as f64 * DAY_WIDTH;
            let ay1 = row_y[dep_idx] + ROW_HEIGHT / 2.0;
            let ax2 = chart_x + resolved[i].0 as f64 * DAY_WIDTH;
            let ay2 = row_y[i] + ROW_HEIGHT / 2.0;

            draw_elbow_arrow(&mut svg, ax1, ay1, ax2, ay2);
        }
    }

    // ── Title ─────────────────────────────────────────────────────────────────

    if let Some(title) = &diagram.meta.title {
        // Prepend title above the diagram (shift everything down is complex;
        // instead we place it in the top margin area).
        svg.text(
            total_width / 2.0,
            MARGIN - 2.0,
            title,
            "middle",
            FONT_SIZE + 2.0,
        );
    }

    svg.finalize()
}

/// Compute absolute start days for all tasks by resolving dependency chains.
///
/// Returns a `Vec<(start_day, duration)>` parallel to `tasks`.
/// Circular dependencies are broken by treating them as day 0.
fn resolve_starts(tasks: &[GanttTask]) -> Vec<(u32, u32)> {
    let n = tasks.len();
    let mut resolved: Vec<Option<u32>> = vec![None; n];

    // Up to n passes to handle chains of any length.
    for _ in 0..n {
        for i in 0..n {
            if resolved[i].is_some() {
                continue;
            }
            let start = match &tasks[i].start {
                TaskStart::Day(d) => Some(*d),
                TaskStart::AfterTask(dep) => {
                    if let Some(dep_idx) = tasks.iter().position(|t| &t.name == dep) {
                        resolved[dep_idx].map(|ds| ds + tasks[dep_idx].duration)
                    } else {
                        // Unknown dependency — start at 0.
                        Some(0)
                    }
                }
            };
            resolved[i] = start;
        }
    }

    resolved
        .into_iter()
        .zip(tasks.iter())
        .map(|(s, t)| (s.unwrap_or(0), t.duration))
        .collect()
}

/// Draw an L-shaped arrow from (x1,y1) to (x2,y2).
fn draw_elbow_arrow(svg: &mut SvgBuilder, x1: f64, y1: f64, x2: f64, y2: f64) {
    let color = "#555";
    // Horizontal segment to x2, then vertical to y2.
    svg.line_segment(x1, y1, x2, y1, color, true);
    svg.line_segment(x2, y1, x2, y2, color, true);
    // Arrow head pointing right into the bar start.
    svg.arrow_head(x2, y2, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_diagram() -> GanttDiagram {
        GanttDiagram {
            meta: DiagramMeta::default(),
            tasks: vec![
                GanttTask {
                    name: "Task 1".into(),
                    duration: 5,
                    start: TaskStart::Day(0),
                },
                GanttTask {
                    name: "Task 2".into(),
                    duration: 3,
                    start: TaskStart::AfterTask("Task 1".into()),
                },
                GanttTask {
                    name: "Task 3".into(),
                    duration: 2,
                    start: TaskStart::AfterTask("Task 2".into()),
                },
            ],
        }
    }

    #[test]
    fn renders_to_svg() {
        let d = simple_diagram();
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Task 1"));
        assert!(svg.contains("Task 2"));
        assert!(svg.contains("Task 3"));
    }

    #[test]
    fn empty_diagram() {
        let d = GanttDiagram {
            meta: DiagramMeta::default(),
            tasks: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn resolve_chain() {
        let tasks = vec![
            GanttTask {
                name: "A".into(),
                duration: 3,
                start: TaskStart::Day(0),
            },
            GanttTask {
                name: "B".into(),
                duration: 2,
                start: TaskStart::AfterTask("A".into()),
            },
            GanttTask {
                name: "C".into(),
                duration: 4,
                start: TaskStart::AfterTask("B".into()),
            },
        ];
        let r = resolve_starts(&tasks);
        assert_eq!(r[0], (0, 3));
        assert_eq!(r[1], (3, 2));
        assert_eq!(r[2], (5, 4));
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startgantt\n\
                     [Task 1] lasts 5 days\n\
                     [Task 2] lasts 3 days\n\
                     [Task 2] starts at [Task 1]'s end\n\
                     [Task 3] lasts 2 days\n\
                     [Task 3] starts at [Task 2]'s end\n\
                     @endgantt";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Task 1"));
        assert!(svg.contains("Task 2"));
        assert!(svg.contains("Task 3"));
    }
}
