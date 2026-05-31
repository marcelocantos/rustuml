// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Gantt chart SVG renderer.
//!
//! Produces a horizontal bar chart matching PlantUML's daily-timescale Gantt
//! layout: a time axis (plain day numbers, or a calendar with
//! month/day-of-week/day rows), one row per task, milestone diamonds,
//! dependency arrows, completion overlays, and a repeated bottom axis.

use rustuml_parser::diagram::gantt::{GanttDiagram, GanttRow, GanttTask, TaskStart};

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics::{ascent, descent, fmt_coord, serif_text_width, text_width};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants (matched to PlantUML's daily Gantt geometry) ───────────────

const DAY_WIDTH: f64 = 16.0;
const ROW_STRIDE: f64 = 16.955078125;
/// A separator row is one task-row taller than a task row (it carries a
/// centred label plus a horizontal rule).
const SEP_STRIDE: f64 = ROW_STRIDE + 16.0;
const SEP_RULE_OFF: f64 = 14.477565;
const SEP_TEXT_OFF: f64 = 18.634765;
const SEP_TEXT_X: f64 = 10.0;
const BAR_H: f64 = 12.955078125;
const MILESTONE_LABEL_MIN_W: f64 = 23.0;
const TITLE_FONT: f64 = 14.0;
const TITLE_TOP_PAD: f64 = 10.0;
const TITLE_LINE_H: f64 = 16.48828125;
const TITLE_BOTTOM_PAD: f64 = 11.0;
const BAR_TOP_PLAIN: f64 = 18.0;
const GRID_TOP_PLAIN: f64 = 6.0;
const GRID_BOTTOM_PAD_PLAIN: f64 = 6.0;

const TASK_FONT: f64 = 11.0;
const AXIS_FONT: f64 = 10.0;
const MONTH_FONT: f64 = 12.0;

const DEFAULT_BAR_COLOR: &str = "#E2E2F0";
const DEFAULT_BAR_STROKE: &str = "#181818";
const GRID_COLOR: &str = "#C0C0C0";
const ARROW_COLOR: &str = "#181818";
const TEXT_COLOR: &str = "#000000";
const CLOSED_TEXT_COLOR: &str = "#989898";
const WEEKEND_FILL: &str = "#F1E5E5";
const COMPLETION_FILL: &str = "#FFFFFF";

const CAL_MONTH_Y: f64 = 11.6015625;
const CAL_DOW_Y: f64 = 23.668;
const CAL_DAYNUM_Y: f64 = 35.668;
const CAL_GRID_TOP: f64 = 39.0;
const CAL_BAR_TOP: f64 = 41.0;

const CAL_BOT_DOW_OFF: f64 = 9.66796875;
const CAL_BOT_DAYNUM_OFF: f64 = 23.66796875;
const CAL_BOT_MONTH_OFF: f64 = 38.60156875;

const BOTTOM_DAYNUM_OFF_PLAIN: f64 = 4.66796875;

// ── Resource-load section (below the task rows) ────────────────────────────────
// Each assigned resource gets a row: a Serif name label, a horizontal rule
// beneath it, and one per-day load percentage centred in each occupied day
// column. Rows are a fixed 32px tall. All offsets are measured from the
// section top (the cumulative bottom of the task rows).
const RES_ROW_STRIDE: f64 = 32.0;
const RES_LABEL_OFF: f64 = 10.13671875;
const RES_RULE_OFF: f64 = 12.94873046875;
const RES_LOAD_OFF: f64 = 22.40234375;
/// Pad below the last resource row before the grid bottom (matches the
/// plain-axis bottom pad).
const RES_BOTTOM_PAD: f64 = 6.0;
const RES_LABEL_FONT: f64 = 13.0;
const RES_LOAD_FONT: f64 = 9.0;
const RES_LOAD_X_OFF: f64 = 1.25;

/// Map a CSS color name (as used in PlantUML Gantt) to a hex string.
fn css_color(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "coral" => "#FF7F50".to_string(),
        "lightblue" => "#ADD8E6".to_string(),
        "lightgreen" => "#90EE90".to_string(),
        "gold" => "#FFD700".to_string(),
        "lightsalmon" => "#FFA07A".to_string(),
        "plum" => "#DDA0DD".to_string(),
        "white" => "#FFFFFF".to_string(),
        "black" => "#000000".to_string(),
        "red" => "#FF0000".to_string(),
        "green" => "#008000".to_string(),
        "blue" => "#0000FF".to_string(),
        "yellow" => "#FFFF00".to_string(),
        "orange" => "#FFA500".to_string(),
        "purple" => "#800080".to_string(),
        "pink" => "#FFC0CB".to_string(),
        "cyan" => "#00FFFF".to_string(),
        "magenta" => "#FF00FF".to_string(),
        "gray" | "grey" => "#808080".to_string(),
        "lightgray" | "lightgrey" => "#D3D3D3".to_string(),
        "darkgray" | "darkgrey" => "#A9A9A9".to_string(),
        "silver" => "#C0C0C0".to_string(),
        "lime" => "#00FF00".to_string(),
        "maroon" => "#800000".to_string(),
        "navy" => "#000080".to_string(),
        "olive" => "#808000".to_string(),
        "teal" => "#008080".to_string(),
        "aqua" => "#00FFFF".to_string(),
        "fuchsia" => "#FF00FF".to_string(),
        _ => {
            if name.starts_with('#') {
                name.to_string()
            } else {
                DEFAULT_BAR_COLOR.to_string()
            }
        }
    }
}

// ── Low-level emit helpers (PlantUML attribute conventions) ─────────────────────

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn fmt_n(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        fmt_coord(v)
    }
}

fn gantt_text(svg: &mut SvgBuilder, x: f64, y: f64, content: &str, font_size: f64, fill: &str) {
    let tl = text_width(content, font_size, false);
    let escaped = escape_xml(content);
    svg.raw_inline(&format!(
        r#"<text fill="{fill}" font-family="sans-serif" font-size="{fs}" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{escaped}</text>"#,
        fs = fmt_n(font_size),
        tl = fmt_coord(tl),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

/// Emit a `<text>` element in the Serif font (used by the Gantt
/// resource-load section). `textLength` is computed from the Serif metric
/// table.
fn gantt_text_serif(svg: &mut SvgBuilder, x: f64, y: f64, content: &str, font_size: f64) {
    let tl = serif_text_width(content, font_size);
    let escaped = escape_xml(content);
    svg.raw_inline(&format!(
        r#"<text fill="{TEXT_COLOR}" font-family="Serif" font-size="{fs}" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{escaped}</text>"#,
        fs = fmt_n(font_size),
        tl = fmt_coord(tl),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

fn gantt_text_bold(svg: &mut SvgBuilder, x: f64, y: f64, content: &str, font_size: f64) {
    let tl = text_width(content, font_size, true);
    let escaped = escape_xml(content);
    svg.raw_inline(&format!(
        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{fs}" font-weight="700" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{escaped}</text>"#,
        fs = fmt_n(font_size),
        tl = fmt_coord(tl),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

fn gantt_line(svg: &mut SvgBuilder, x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str) {
    svg.raw_inline(&format!(
        r#"<line style="stroke:{stroke};stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"/>"#,
        x1 = fmt_coord(x1),
        x2 = fmt_coord(x2),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    ));
}

fn gantt_rect_fill(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw_inline(&format!(
        r#"<rect fill="{fill}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fmt_coord(h),
        w = fmt_coord(w),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

fn gantt_rect_outline(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, stroke: &str) {
    svg.raw_inline(&format!(
        r#"<rect fill="none" height="{h}" style="stroke:{stroke};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fmt_coord(h),
        w = fmt_coord(w),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

/// Render a Gantt diagram with an optional oracle layout.
pub fn render_with_oracle(
    diagram: &GanttDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "GANTT");
    }
    render(diagram, theme)
}

/// A laid-out row to be drawn.
enum LaidRow<'a> {
    Task(&'a GanttTask, usize),
    Separator(&'a str),
}

/// Render a Gantt diagram to SVG.
pub fn render(diagram: &GanttDiagram, _theme: &Theme) -> String {
    if diagram.tasks.is_empty() {
        let svg = SvgBuilder::new_plantuml(81.0, 82.0, "GANTT");
        return svg.finalize_plantuml();
    }

    let resolved_wd = resolve_starts(&diagram.tasks);
    let has_closed = !diagram.closed_days.is_empty() && diagram.project_start.is_some();
    let (resolved, total_days) = if has_closed {
        let start_dow = diagram
            .project_start
            .as_deref()
            .and_then(parse_start_dow)
            .unwrap_or(0);
        let cal: Vec<(u32, u32)> = resolved_wd
            .iter()
            .map(|&(wd_start, wd_dur)| {
                let cal_start = wd_to_cal(wd_start, start_dow, &diagram.closed_days);
                // The visible end is one column past the task's last open day.
                // Using wd_to_cal(wd_start + wd_dur) would instead land on the
                // first open day *after* any trailing weekend, over-extending
                // the bar (and the chart width) by the skipped closed days.
                let cal_end = if wd_dur == 0 {
                    cal_start
                } else {
                    wd_to_cal(wd_start + wd_dur - 1, start_dow, &diagram.closed_days) + 1
                };
                (cal_start, cal_end - cal_start)
            })
            .collect();
        let total = cal.iter().map(|&(s, d)| s + d).max().unwrap_or(1).max(1);
        (cal, total)
    } else {
        let total = resolved_wd
            .iter()
            .map(|&(s, d)| s + d)
            .max()
            .unwrap_or(1)
            .max(1);
        (resolved_wd, total)
    };

    let cal = diagram
        .project_start
        .as_deref()
        .and_then(|s| CalendarInfo::parse(s, total_days));
    let has_cal = cal.is_some();

    let fallback_rows: Vec<GanttRow>;
    let rows_list: Vec<&GanttRow> = if !diagram.rows.is_empty() {
        diagram.rows.iter().collect()
    } else {
        fallback_rows = diagram
            .tasks
            .iter()
            .map(|t| GanttRow::Task(t.name.clone()))
            .collect();
        fallback_rows.iter().collect()
    };

    let laid: Vec<LaidRow> = rows_list
        .iter()
        .filter_map(|row| match row {
            GanttRow::Separator(label) => Some(LaidRow::Separator(label.as_str())),
            GanttRow::Task(name) => diagram
                .tasks
                .iter()
                .position(|t| &t.name == name)
                .map(|idx| LaidRow::Task(&diagram.tasks[idx], idx)),
        })
        .collect();
    let n_rows = laid.len();

    let chart_width = total_days as f64 * DAY_WIDTH;
    // A task label that spills past the right edge of its bar widens the
    // canvas to contain it.
    let mut label_right = 0.0_f64;
    for row in &laid {
        if let LaidRow::Task(task, idx) = row {
            let (start_day, dur) = resolved[*idx];
            let label = task_label(task);
            let label_w = text_width(&label, TASK_FONT, false);
            let (lx, reserved_w) = if dur == 0 {
                // A milestone label reserves at least a fixed minimum cell
                // width to the right of its diamond, so a short label still
                // pads the canvas.
                (
                    (start_day as f64 * DAY_WIDTH - 8.0).max(8.0) + 8.0,
                    label_w.max(MILESTONE_LABEL_MIN_W),
                )
            } else {
                let bar_x = start_day as f64 * DAY_WIDTH + 2.0;
                let bar_w = (dur as f64 * DAY_WIDTH - 4.0).max(1.0);
                // PlantUML draws the label inside only when the bar's 6px
                // interior inset (pos1 = start+6, pos2 = end-6, span bar_w-8)
                // strictly exceeds the label width; otherwise the label is
                // pushed past the right edge.
                let lx = if bar_w - 8.0 > label_w {
                    bar_x + 4.0
                } else {
                    bar_x + bar_w + 4.0
                };
                (lx, label_w)
            };
            label_right = label_right.max(lx + reserved_w);
        }
    }
    let total_width = (chart_width + 1.0).max(label_right + 1.0);

    // A `title` pushes the whole chart down by a fixed band: 10px top pad,
    // one title line, then an 11px bottom gap before the calendar/grid.
    let title_h = if diagram.meta.title.is_some() {
        TITLE_TOP_PAD + TITLE_LINE_H + TITLE_BOTTOM_PAD
    } else {
        0.0
    };

    let grid_top = title_h
        + if has_cal {
            CAL_GRID_TOP
        } else {
            GRID_TOP_PLAIN
        };
    let bar_top0 = title_h + if has_cal { CAL_BAR_TOP } else { BAR_TOP_PLAIN };

    // Cumulative top offset of each row. Task rows advance by ROW_STRIDE;
    // separator rows are taller (they carry a label and a rule line).
    let mut row_tops: Vec<f64> = Vec::with_capacity(n_rows + 1);
    let mut acc = bar_top0;
    for row in &laid {
        row_tops.push(acc);
        acc += match row {
            LaidRow::Separator(_) => SEP_STRIDE,
            LaidRow::Task(..) => ROW_STRIDE,
        };
    }
    row_tops.push(acc);
    let row_bar_top = |vi: usize| row_tops[vi];
    let rows_extent = acc - ROW_STRIDE + BAR_H; // bottom of last row's bar

    // Resource-load section: one row per assigned resource, listed in
    // first-appearance order. For each resource, accumulate the load
    // percentage on every calendar day spanned by a task it is assigned to.
    let mut res_names: Vec<&str> = Vec::new();
    let mut res_loads: Vec<Vec<u32>> = Vec::new(); // index by res, then by day
    for (idx, task) in diagram.tasks.iter().enumerate() {
        let (start_day, dur) = resolved[idx];
        for assignment in &task.resources {
            let ri = match res_names.iter().position(|&n| n == assignment.name) {
                Some(ri) => ri,
                None => {
                    res_names.push(assignment.name.as_str());
                    res_loads.push(vec![0u32; total_days as usize]);
                    res_names.len() - 1
                }
            };
            for d in start_day..(start_day + dur) {
                if (d as usize) < res_loads[ri].len() {
                    res_loads[ri][d as usize] += assignment.percent;
                }
            }
        }
    }
    let n_res = res_names.len();
    let res_section_top = acc;

    let grid_bottom = if has_cal {
        rows_extent + 2.0
    } else if n_res > 0 {
        res_section_top + n_res as f64 * RES_ROW_STRIDE + RES_BOTTOM_PAD
    } else {
        acc + GRID_BOTTOM_PAD_PLAIN
    };

    let total_height = if has_cal {
        // Bottom-most drawn element is the month-label text; the SVG box is
        // its baseline plus the font descent, rounded up to a whole pixel.
        (grid_bottom + CAL_BOT_MONTH_OFF + descent(MONTH_FONT)).ceil()
    } else {
        grid_bottom + BOTTOM_DAYNUM_OFF_PLAIN + 2.4669
    };

    let mut svg = SvgBuilder::new_plantuml(total_width, total_height, "GANTT");

    // 0. Title band (centred bold text wrapped in <g class="title">).
    if let Some(title) = &diagram.meta.title {
        let tw = text_width(title, TITLE_FONT, true);
        // PlantUML centres the title over the content extent (the SVG box
        // less its trailing 2px), not the integer canvas width.
        let tx = (total_width - 2.0 - tw) / 2.0;
        let ty = TITLE_TOP_PAD + ascent(TITLE_FONT);
        svg.raw_inline(&format!(
            r#"<g class="title" data-source-line="1"><text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="{tw}" x="{tx}" y="{ty}">{}</text></g>"#,
            escape_xml(title),
            tw = fmt_n(tw),
            tx = fmt_n(tx),
            ty = fmt_n(ty),
        ));
    }

    // 1. Weekend shading (calendar only).
    if let Some(ref c) = cal {
        let mut day_idx = 0usize;
        while day_idx < c.day_of_week.len() {
            let dow = c.day_of_week[day_idx];
            if diagram.closed_days.contains(&dow) {
                // Coalesce consecutive closed days into one rect.
                let start = day_idx;
                while day_idx < c.day_of_week.len()
                    && diagram.closed_days.contains(&c.day_of_week[day_idx])
                {
                    day_idx += 1;
                }
                let gx = start as f64 * DAY_WIDTH;
                let w = (day_idx - start) as f64 * DAY_WIDTH;
                gantt_rect_fill(
                    &mut svg,
                    gx,
                    grid_top,
                    w,
                    grid_bottom - grid_top,
                    WEEKEND_FILL,
                );
            } else {
                day_idx += 1;
            }
        }
    }

    // 2/3. Top axis + grid lines. PlantUML emits the calendar header (text)
    // before the grid lines, but the plain day-number axis after them.
    if let Some(ref c) = cal {
        render_calendar_axis(
            &mut svg,
            c,
            diagram,
            title_h + CAL_DOW_Y,
            title_h + CAL_DAYNUM_Y,
            title_h + CAL_MONTH_Y,
        );
        for day in 0..=total_days {
            let gx = day as f64 * DAY_WIDTH;
            gantt_line(&mut svg, gx, grid_top, gx, grid_bottom, GRID_COLOR);
        }
        let x_end = chart_width - 0.0002;
        gantt_line(&mut svg, 0.0, grid_top, x_end, grid_top, GRID_COLOR);
        gantt_line(&mut svg, 0.0, grid_bottom, x_end, grid_bottom, GRID_COLOR);
    } else {
        for day in 0..=total_days {
            let gx = day as f64 * DAY_WIDTH;
            gantt_line(&mut svg, gx, grid_top, gx, grid_bottom, GRID_COLOR);
        }
        render_day_numbers(&mut svg, total_days, title_h + ascent(AXIS_FONT));
    }

    // 4. Dependency arrows.
    let task_row_index: std::collections::HashMap<&str, usize> = laid
        .iter()
        .enumerate()
        .filter_map(|(vi, r)| match r {
            LaidRow::Task(t, _) => Some((t.name.as_str(), vi)),
            _ => None,
        })
        .collect();

    for (vi, row) in laid.iter().enumerate() {
        if let LaidRow::Task(task, idx) = row
            && let TaskStart::AfterTask(dep) = &task.start
            && let Some(dep_idx) = diagram.tasks.iter().position(|t| &t.name == dep)
            && let Some(&dep_vi) = task_row_index.get(dep.as_str())
        {
            // A milestone successor ("happens at X's end") draws no dependency
            // arrow into the diamond; only real "starts at" tasks do.
            if resolved[*idx].1 == 0 {
                continue;
            }
            let (dep_start, dep_dur) = resolved[dep_idx];
            let pred_end_x = (dep_start + dep_dur) as f64 * DAY_WIDTH;
            // From a milestone predecessor (zero-duration diamond) the arrow
            // departs the diamond centre rather than the bottom of a bar.
            let pred_exit_y = if dep_dur == 0 {
                row_bar_top(dep_vi) + 5.0
            } else {
                row_bar_top(dep_vi) + BAR_H
            };
            let succ_start_x = resolved[*idx].0 as f64 * DAY_WIDTH;
            let succ_center = row_bar_top(vi) + BAR_H / 2.0;
            draw_dependency_arrow(&mut svg, pred_end_x, pred_exit_y, succ_start_x, succ_center);
        } else if let LaidRow::Task(task, idx) = row
            && let TaskStart::WithTask(dep) = &task.start
            && let Some(&dep_vi) = task_row_index.get(dep.as_str())
            && resolved[*idx].1 != 0
        {
            // A parallel start ("starts at X's start") routes the arrow out
            // the predecessor's left edge, down the left margin, and into the
            // successor's left edge.
            let pred_left_x = resolved[*idx].0 as f64 * DAY_WIDTH + 2.0;
            let pred_center = row_bar_top(dep_vi) + BAR_H / 2.0;
            let succ_start_x = resolved[*idx].0 as f64 * DAY_WIDTH;
            let succ_center = row_bar_top(vi) + BAR_H / 2.0;
            draw_parallel_arrow(
                &mut svg,
                pred_left_x,
                pred_center,
                succ_start_x,
                succ_center,
            );
        }
    }

    // 5. Bars / milestones / separator rules.
    for (vi, row) in laid.iter().enumerate() {
        if let LaidRow::Separator(label) = row {
            // Horizontal rule running the full chart width, broken around the
            // (left-aligned) label.
            let line_y = row_bar_top(vi) + SEP_RULE_OFF;
            let tl = text_width(label, TASK_FONT, false);
            gantt_line(
                &mut svg,
                0.0,
                line_y,
                SEP_TEXT_X - 5.0,
                line_y,
                DEFAULT_BAR_STROKE,
            );
            gantt_line(
                &mut svg,
                SEP_TEXT_X + tl + 5.0,
                line_y,
                chart_width - 1.0,
                line_y,
                DEFAULT_BAR_STROKE,
            );
        }
        if let LaidRow::Task(task, idx) = row {
            let (start_day, dur) = resolved[*idx];
            let bar_top = row_bar_top(vi);
            if dur == 0 {
                // A day-0 milestone would push the diamond off the left edge;
                // PlantUML clamps the centre so the diamond stays on-canvas.
                let cx = (start_day as f64 * DAY_WIDTH - 8.0).max(8.0);
                let cy = bar_top + 5.0;
                let fill = task
                    .color
                    .as_deref()
                    .map(css_color)
                    .unwrap_or_else(|| TEXT_COLOR.to_string());
                svg.raw_inline(&format!(
                    r#"<polygon fill="{fill}" points="{cx},{t},{r},{cy},{cx},{b},{l},{cy}" style="stroke:{fill};stroke-width:1;"/>"#,
                    cx = fmt_coord(cx),
                    t = fmt_coord(cy - 5.0),
                    r = fmt_coord(cx + 5.0),
                    cy = fmt_coord(cy),
                    b = fmt_coord(cy + 5.0),
                    l = fmt_coord(cx - 5.0),
                ));
            } else {
                let colored = task.color.is_some();
                let fill = task
                    .color
                    .as_deref()
                    .map(css_color)
                    .unwrap_or_else(|| DEFAULT_BAR_COLOR.to_string());
                let stroke = if colored {
                    fill.clone()
                } else {
                    DEFAULT_BAR_STROKE.to_string()
                };

                // Split the bar's calendar span into runs of consecutive open
                // (non-closed) day columns. With no closed days this yields a
                // single run spanning the whole bar.
                let is_closed = |col: u32| -> bool {
                    cal.as_ref().is_some_and(|c| {
                        (col as usize) < c.day_of_week.len()
                            && diagram.closed_days.contains(&c.day_of_week[col as usize])
                    })
                };
                let mut runs: Vec<(u32, u32)> = Vec::new();
                let mut col = start_day;
                while col < start_day + dur {
                    if is_closed(col) {
                        col += 1;
                        continue;
                    }
                    let run_start = col;
                    while col < start_day + dur && !is_closed(col) {
                        col += 1;
                    }
                    runs.push((run_start, col));
                }
                if runs.is_empty() {
                    runs.push((start_day, start_day + dur));
                }

                let n_runs = runs.len();
                for (ri, &(cs, ce)) in runs.iter().enumerate() {
                    let first = ri == 0;
                    let last = ri == n_runs - 1;
                    // Fill rect: the first run is inset 2px on its left, the
                    // last run is inset 2px on its right; open-ended sides
                    // overshoot the column boundary by 1px.
                    let left = if first {
                        cs as f64 * DAY_WIDTH + 2.0
                    } else {
                        cs as f64 * DAY_WIDTH
                    };
                    let fill_right = if last {
                        ce as f64 * DAY_WIDTH - 2.0
                    } else {
                        ce as f64 * DAY_WIDTH + 1.0
                    };
                    let bar_w = (fill_right - left).max(1.0);

                    if n_runs == 1 {
                        // Single run: classic full rect + outline, with the
                        // optional completion overlay.
                        if let Some(pct) = task.completed {
                            let done_w = (bar_w * pct as f64 / 100.0).max(0.0);
                            if done_w > 0.0 {
                                gantt_rect_fill(&mut svg, left, bar_top, done_w, BAR_H, &fill);
                            }
                            let rem_w = bar_w - done_w;
                            if rem_w > 0.0 {
                                gantt_rect_fill(
                                    &mut svg,
                                    left + done_w,
                                    bar_top,
                                    rem_w,
                                    BAR_H,
                                    COMPLETION_FILL,
                                );
                            }
                        } else {
                            gantt_rect_fill(&mut svg, left, bar_top, bar_w, BAR_H, &fill);
                        }
                        gantt_rect_outline(&mut svg, left, bar_top, bar_w, BAR_H, &stroke);
                        continue;
                    }

                    gantt_rect_fill(&mut svg, left, bar_top, bar_w, BAR_H, &fill);

                    // Outline: end caps only. The first run is open on the
                    // right, the last run open on the left; middle runs get no
                    // outline at all (the abutting fills cover their edges).
                    let top = bar_top;
                    let bot = bar_top + BAR_H;
                    if first {
                        let r = ce as f64 * DAY_WIDTH;
                        svg.raw_inline(&format!(
                            r#"<path d="M{r},{bot} L{l},{bot} L{l},{top} L{r},{top}" fill="none" style="stroke:{stroke};stroke-width:1;"/>"#,
                            r = fmt_coord(r),
                            bot = fmt_coord(bot),
                            l = fmt_coord(left),
                            top = fmt_coord(top),
                        ));
                    } else if last {
                        let r = ce as f64 * DAY_WIDTH - 2.0;
                        svg.raw_inline(&format!(
                            r#"<path d="M{l},{top} L{r},{top} L{r},{bot} L{l},{bot}" fill="none" style="stroke:{stroke};stroke-width:1;"/>"#,
                            l = fmt_coord(left),
                            top = fmt_coord(top),
                            r = fmt_coord(r),
                            bot = fmt_coord(bot),
                        ));
                    } else {
                        // Middle run: plain top and bottom border lines
                        // spanning the fill width (left flush, right at the
                        // column boundary).
                        let r = ce as f64 * DAY_WIDTH;
                        gantt_line(&mut svg, left, top, r, top, &stroke);
                        gantt_line(&mut svg, left, bot, r, bot, &stroke);
                    }
                }

                // Dashed connectors bridging each closed-day gap between
                // consecutive runs, drawn at the bar's top and bottom edges.
                if n_runs > 1 {
                    let top = bar_top;
                    let bot = bar_top + BAR_H;
                    for w in runs.windows(2) {
                        let gap_left = w[0].1 as f64 * DAY_WIDTH + 3.0;
                        let gap_right = w[1].0 as f64 * DAY_WIDTH - 3.0;
                        svg.raw_inline(&format!(
                            r#"<line style="stroke:{stroke};stroke-width:1;stroke-dasharray:2,3;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#,
                            x1 = fmt_coord(gap_left),
                            x2 = fmt_coord(gap_right),
                            y = fmt_coord(top),
                        ));
                        svg.raw_inline(&format!(
                            r#"<line style="stroke:{stroke};stroke-width:1;stroke-dasharray:2,3;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#,
                            x1 = fmt_coord(gap_left),
                            x2 = fmt_coord(gap_right),
                            y = fmt_coord(bot),
                        ));
                    }
                }
            }
        }
    }

    // 6. Task / milestone / separator labels.
    for (vi, row) in laid.iter().enumerate() {
        match row {
            LaidRow::Separator(label) => {
                if !label.is_empty() {
                    let text_y = row_bar_top(vi) + SEP_TEXT_OFF;
                    gantt_text(&mut svg, SEP_TEXT_X, text_y, label, TASK_FONT, TEXT_COLOR);
                }
            }
            LaidRow::Task(task, idx) => {
                let (start_day, dur) = resolved[*idx];
                let ly = row_bar_top(vi) + ascent(TASK_FONT);
                let label = task_label(task);
                let lx = if dur == 0 {
                    // Label sits 8px right of the (clamped) diamond centre.
                    (start_day as f64 * DAY_WIDTH - 8.0).max(8.0) + 8.0
                } else {
                    // Labels normally sit inside the bar (4px from its left
                    // edge). The label fits inside only when the bar's 6px
                    // interior inset (span bar_w-8) strictly exceeds the label
                    // width; otherwise PlantUML places it just past the bar's
                    // right edge instead.
                    let bar_x = start_day as f64 * DAY_WIDTH + 2.0;
                    let bar_w = (dur as f64 * DAY_WIDTH - 4.0).max(1.0);
                    let label_w = text_width(&label, TASK_FONT, false);
                    if bar_w - 8.0 > label_w {
                        bar_x + 4.0
                    } else {
                        bar_x + bar_w + 4.0
                    }
                };
                gantt_text(&mut svg, lx, ly, &label, TASK_FONT, TEXT_COLOR);
            }
        }
    }

    // 6b. Resource-load section. For each resource: a Serif name label, a
    // horizontal rule beneath it spanning the chart, and one load
    // percentage centred in each occupied day column.
    if !has_cal && n_res > 0 {
        let rule_x2 = chart_width - 0.0002;
        for (ri, name) in res_names.iter().enumerate() {
            let row_top = res_section_top + ri as f64 * RES_ROW_STRIDE;
            gantt_text_serif(&mut svg, 0.0, row_top + RES_LABEL_OFF, name, RES_LABEL_FONT);
            gantt_line(
                &mut svg,
                0.0,
                row_top + RES_RULE_OFF,
                rule_x2,
                row_top + RES_RULE_OFF,
                TEXT_COLOR,
            );
            let load_y = row_top + RES_LOAD_OFF;
            for (day, &load) in res_loads[ri].iter().enumerate() {
                if load == 0 {
                    continue;
                }
                gantt_text_serif(
                    &mut svg,
                    day as f64 * DAY_WIDTH + RES_LOAD_X_OFF,
                    load_y,
                    &load.to_string(),
                    RES_LOAD_FONT,
                );
            }
        }
    }

    // 7. Bottom axis.
    if let Some(ref c) = cal {
        render_calendar_axis(
            &mut svg,
            c,
            diagram,
            grid_bottom + CAL_BOT_DOW_OFF,
            grid_bottom + CAL_BOT_DAYNUM_OFF,
            grid_bottom + CAL_BOT_MONTH_OFF,
        );
    } else {
        render_day_numbers(&mut svg, total_days, grid_bottom + BOTTOM_DAYNUM_OFF_PLAIN);
    }

    svg.finalize_plantuml()
}

fn task_label(task: &GanttTask) -> String {
    if task.resources.is_empty() {
        task.name.clone()
    } else {
        let res_part: String = task
            .resources
            .iter()
            .map(|r| {
                if r.percent == 100 {
                    format!(" {{{}}}", r.name)
                } else {
                    format!(" {{{}:{}%}}", r.name, r.percent)
                }
            })
            .collect();
        format!("{}{}", task.name, res_part)
    }
}

fn render_day_numbers(svg: &mut SvgBuilder, total_days: u32, baseline: f64) {
    for day in 0..total_days {
        let label = (day + 1).to_string();
        let tl = text_width(&label, AXIS_FONT, false);
        let cell_x = day as f64 * DAY_WIDTH;
        let tx = cell_x + (DAY_WIDTH - tl) / 2.0;
        gantt_text(svg, tx, baseline, &label, AXIS_FONT, TEXT_COLOR);
    }
}

fn draw_dependency_arrow(
    svg: &mut SvgBuilder,
    pred_end_x: f64,
    pred_bottom: f64,
    succ_start_x: f64,
    succ_center: f64,
) {
    let x1 = pred_end_x - 8.0;
    let x_path_end = succ_start_x - 3.0;
    svg.raw_inline(&format!(
        r#"<path d="M{x1},{y1} L{x1},{ymid} L{xend},{ymid}" fill="none" style="stroke:{ARROW_COLOR};stroke-width:1.5;"/>"#,
        x1 = fmt_coord(x1),
        y1 = fmt_coord(pred_bottom),
        ymid = fmt_coord(succ_center),
        xend = fmt_coord(x_path_end),
    ));
    let tip = succ_start_x;
    svg.raw_inline(&format!(
        r#"<polygon fill="{ARROW_COLOR}" points="{a},{ta},{tip},{cy},{a},{tb},{a},{ta}" style="stroke:{ARROW_COLOR};stroke-width:1;"/>"#,
        a = fmt_coord(tip - 4.0),
        ta = fmt_coord(succ_center - 4.0),
        tip = fmt_coord(tip),
        cy = fmt_coord(succ_center),
        tb = fmt_coord(succ_center + 4.0),
    ));
}

fn draw_parallel_arrow(
    svg: &mut SvgBuilder,
    pred_left_x: f64,
    pred_center: f64,
    succ_start_x: f64,
    succ_center: f64,
) {
    let x_left = pred_left_x - 10.0;
    let x_path_end = succ_start_x - 3.0;
    svg.raw_inline(&format!(
        r#"<path d="M{x1},{y1} L{xl},{y1} L{xl},{ymid} L{xend},{ymid}" fill="none" style="stroke:{ARROW_COLOR};stroke-width:1.5;"/>"#,
        x1 = fmt_coord(pred_left_x),
        y1 = fmt_coord(pred_center),
        xl = fmt_coord(x_left),
        ymid = fmt_coord(succ_center),
        xend = fmt_coord(x_path_end),
    ));
    let tip = succ_start_x;
    svg.raw_inline(&format!(
        r#"<polygon fill="{ARROW_COLOR}" points="{a},{ta},{tip},{cy},{a},{tb},{a},{ta}" style="stroke:{ARROW_COLOR};stroke-width:1;"/>"#,
        a = fmt_coord(tip - 4.0),
        ta = fmt_coord(succ_center - 4.0),
        tip = fmt_coord(tip),
        cy = fmt_coord(succ_center),
        tb = fmt_coord(succ_center + 4.0),
    ));
}

struct CalendarInfo {
    day_of_week: Vec<u8>,
    day_of_month: Vec<u8>,
    month_spans: Vec<(usize, usize, String)>,
}

impl CalendarInfo {
    fn parse(date_str: &str, total_days: u32) -> Option<Self> {
        let parts: Vec<u32> = date_str.split('-').filter_map(|s| s.parse().ok()).collect();
        if parts.len() != 3 {
            return None;
        }
        let (year, month, day) = (parts[0] as i32, parts[1], parts[2]);
        let start_dow = zeller_dow(year, month, day);

        let mut day_of_week = Vec::with_capacity(total_days as usize);
        let mut day_of_month = Vec::with_capacity(total_days as usize);
        let mut month_spans: Vec<(usize, usize, String)> = Vec::new();

        let mut cur_year = year;
        let mut cur_month = month;
        let mut cur_day = day;
        let mut cur_dow = start_dow;

        for i in 0..total_days as usize {
            if i == 0 || cur_day == 1 {
                let label = format!("{} {}", month_name(cur_month), cur_year);
                month_spans.push((i, 0, label));
            }
            day_of_week.push(cur_dow);
            day_of_month.push(cur_day as u8);

            let days_in_mon = days_in_month(cur_year, cur_month);
            if cur_day < days_in_mon {
                cur_day += 1;
            } else {
                cur_day = 1;
                if cur_month == 12 {
                    cur_month = 1;
                    cur_year += 1;
                } else {
                    cur_month += 1;
                }
            }
            cur_dow = (cur_dow + 1) % 7;
        }

        let total = total_days as usize;
        for j in 0..month_spans.len() {
            let end = if j + 1 < month_spans.len() {
                month_spans[j + 1].0
            } else {
                total
            };
            month_spans[j].1 = end;
        }

        Some(CalendarInfo {
            day_of_week,
            day_of_month,
            month_spans,
        })
    }
}

const DOW_ABBR: &[&str] = &["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

fn render_calendar_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    diagram: &GanttDiagram,
    dow_y: f64,
    daynum_y: f64,
    month_y: f64,
) {
    let abbreviated = diagram.printscale.as_deref() == Some("weekly");

    for (day_idx, &dow) in cal.day_of_week.iter().enumerate() {
        let abbr = DOW_ABBR[dow as usize];
        let closed = diagram.closed_days.contains(&dow);
        let fill = if closed {
            CLOSED_TEXT_COLOR
        } else {
            TEXT_COLOR
        };
        let tl = text_width(abbr, AXIS_FONT, false);
        let tx = day_idx as f64 * DAY_WIDTH + (DAY_WIDTH - tl) / 2.0;
        gantt_text(svg, tx, dow_y, abbr, AXIS_FONT, fill);
    }

    for (day_idx, &dom) in cal.day_of_month.iter().enumerate() {
        let label = dom.to_string();
        let dow = cal.day_of_week[day_idx];
        let closed = diagram.closed_days.contains(&dow);
        let fill = if closed {
            CLOSED_TEXT_COLOR
        } else {
            TEXT_COLOR
        };
        let tl = text_width(&label, AXIS_FONT, false);
        let tx = day_idx as f64 * DAY_WIDTH + (DAY_WIDTH - tl) / 2.0;
        gantt_text(svg, tx, daynum_y, &label, AXIS_FONT, fill);
    }

    for &(start_idx, end_idx, ref label) in &cal.month_spans {
        let span_days = (end_idx - start_idx) as f64;
        let span_w = span_days * DAY_WIDTH;
        let mut display_label = if abbreviated {
            abbreviate_month_label(label)
        } else {
            label.clone()
        };
        // When "Month Year" is wider than its column span, PlantUML drops
        // the year and shows just the month name (e.g. a sliver of March at
        // the right edge becomes "March", not "March 2024").
        if text_width(&display_label, MONTH_FONT, true) > span_w
            && let Some((month_only, _)) = display_label.rsplit_once(' ')
        {
            display_label = month_only.to_string();
        }
        // A bare month name that still overflows its sliver is abbreviated to
        // its three-letter form (e.g. "March" -> "Mar").
        if text_width(&display_label, MONTH_FONT, true) > span_w {
            display_label = abbreviate_month_name(&display_label).to_string();
        }
        let tl = text_width(&display_label, MONTH_FONT, true);
        // A label that still overflows its (narrow) span is left-aligned to
        // the span start rather than centred, so it grows rightwards.
        let mx = if tl > span_w {
            start_idx as f64 * DAY_WIDTH
        } else {
            start_idx as f64 * DAY_WIDTH + span_w / 2.0 - tl / 2.0
        };
        gantt_text_bold(svg, mx, month_y, &display_label, MONTH_FONT);
    }
}

fn zeller_dow(year: i32, month: u32, day: u32) -> u8 {
    static T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let d = day as i32;
    let dow = (y + y / 4 - y / 100 + y / 400 + T[(month as usize) - 1] + d) % 7;
    ((dow + 6) % 7) as u8
}

fn parse_start_dow(s: &str) -> Option<u8> {
    let parts: Vec<u32> = s.split('-').filter_map(|p| p.parse().ok()).collect();
    if parts.len() == 3 {
        Some(zeller_dow(parts[0] as i32, parts[1], parts[2]))
    } else {
        None
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn abbreviate_month_name(month: &str) -> &str {
    match month {
        "January" => "Jan",
        "February" => "Feb",
        "March" => "Mar",
        "April" => "Apr",
        "May" => "May",
        "June" => "Jun",
        "July" => "Jul",
        "August" => "Aug",
        "September" => "Sep",
        "October" => "Oct",
        "November" => "Nov",
        "December" => "Dec",
        other => other,
    }
}

fn abbreviate_month_label(label: &str) -> String {
    if let Some(pos) = label.find(' ') {
        let month_part = &label[..pos];
        let year_part = &label[pos..];
        format!("{}{}", abbreviate_month_name(month_part), year_part)
    } else {
        abbreviate_month_name(label).to_string()
    }
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "?",
    }
}

fn wd_to_cal(working_day: u32, start_dow: u8, closed_days: &[u8]) -> u32 {
    if closed_days.is_empty() {
        return working_day;
    }
    let mut open_count = 0u32;
    let mut cal = 0u32;
    loop {
        let dow = ((start_dow as u32 + cal) % 7) as u8;
        if !closed_days.contains(&dow) {
            if open_count == working_day {
                return cal;
            }
            open_count += 1;
        }
        cal += 1;
        if cal > working_day * 7 + 14 {
            return cal;
        }
    }
}

fn resolve_starts(tasks: &[GanttTask]) -> Vec<(u32, u32)> {
    let n = tasks.len();
    let mut resolved: Vec<Option<u32>> = vec![None; n];

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
                        Some(0)
                    }
                }
                TaskStart::WithTask(dep) => {
                    if let Some(dep_idx) = tasks.iter().position(|t| &t.name == dep) {
                        resolved[dep_idx]
                    } else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn task(name: &str, duration: u32, start: TaskStart) -> GanttTask {
        GanttTask {
            name: name.into(),
            duration,
            start,
            color: None,
            completed: None,
            resources: Vec::new(),
        }
    }

    fn simple_diagram() -> GanttDiagram {
        GanttDiagram {
            meta: DiagramMeta::default(),
            project_start: None,
            closed_days: Vec::new(),
            printscale: None,
            resources: Vec::new(),
            notes: Vec::new(),
            rows: vec![
                GanttRow::Task("Task 1".into()),
                GanttRow::Task("Task 2".into()),
                GanttRow::Task("Task 3".into()),
            ],
            tasks: vec![
                task("Task 1", 5, TaskStart::Day(0)),
                task("Task 2", 3, TaskStart::AfterTask("Task 1".into())),
                task("Task 3", 2, TaskStart::AfterTask("Task 2".into())),
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
            project_start: None,
            closed_days: Vec::new(),
            printscale: None,
            resources: Vec::new(),
            notes: Vec::new(),
            rows: vec![],
            tasks: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn resolve_chain() {
        let tasks = vec![
            task("A", 3, TaskStart::Day(0)),
            task("B", 2, TaskStart::AfterTask("A".into())),
            task("C", 4, TaskStart::AfterTask("B".into())),
        ];
        let r = resolve_starts(&tasks);
        assert_eq!(r[0], (0, 3));
        assert_eq!(r[1], (3, 2));
        assert_eq!(r[2], (5, 4));
    }

    #[test]
    fn dow_calculation() {
        assert_eq!(zeller_dow(2024, 1, 1), 0);
        assert_eq!(zeller_dow(2024, 1, 6), 5);
        assert_eq!(zeller_dow(2024, 1, 7), 6);
    }
}
