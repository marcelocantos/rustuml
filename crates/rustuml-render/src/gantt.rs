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
use crate::plantuml_metrics::{ascent, fmt_coord, text_width};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants (matched to PlantUML's daily Gantt geometry) ───────────────

const DAY_WIDTH: f64 = 16.0;
const ROW_STRIDE: f64 = 16.955078125;
const BAR_H: f64 = 12.955078125;
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

const CAL_BOT_DOW_OFF: f64 = 9.668;
const CAL_BOT_DAYNUM_OFF: f64 = 23.668;
const CAL_BOT_MONTH_OFF: f64 = 38.6016;

const BOTTOM_DAYNUM_OFF_PLAIN: f64 = 4.668;

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
                let cal_end = wd_to_cal(wd_start + wd_dur, start_dow, &diagram.closed_days);
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
    let total_width = chart_width + 1.0;

    let grid_top = if has_cal {
        CAL_GRID_TOP
    } else {
        GRID_TOP_PLAIN
    };
    let bar_top0 = if has_cal { CAL_BAR_TOP } else { BAR_TOP_PLAIN };
    let last_bar_bottom = bar_top0 + (n_rows.saturating_sub(1)) as f64 * ROW_STRIDE + BAR_H;
    let grid_bottom = if has_cal {
        last_bar_bottom + 2.0
    } else {
        bar_top0 + n_rows as f64 * ROW_STRIDE + GRID_BOTTOM_PAD_PLAIN
    };

    let total_height = if has_cal {
        grid_bottom + CAL_BOT_MONTH_OFF + 6.0
    } else {
        grid_bottom + BOTTOM_DAYNUM_OFF_PLAIN + 2.4669
    };

    let row_bar_top = |vi: usize| bar_top0 + vi as f64 * ROW_STRIDE;

    let mut svg = SvgBuilder::new_plantuml(total_width, total_height, "GANTT");

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
        render_calendar_axis(&mut svg, c, diagram, CAL_DOW_Y, CAL_DAYNUM_Y, CAL_MONTH_Y);
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
        render_day_numbers(&mut svg, total_days, ascent(AXIS_FONT));
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
            let (dep_start, dep_dur) = resolved[dep_idx];
            let pred_end_x = (dep_start + dep_dur) as f64 * DAY_WIDTH;
            let pred_bottom = row_bar_top(dep_vi) + BAR_H;
            let succ_start_x = resolved[*idx].0 as f64 * DAY_WIDTH;
            let succ_center = row_bar_top(vi) + BAR_H / 2.0;
            draw_dependency_arrow(&mut svg, pred_end_x, pred_bottom, succ_start_x, succ_center);
        }
    }

    // 5. Bars / milestones.
    for (vi, row) in laid.iter().enumerate() {
        if let LaidRow::Task(task, idx) = row {
            let (start_day, dur) = resolved[*idx];
            let bar_top = row_bar_top(vi);
            if dur == 0 {
                let cx = start_day as f64 * DAY_WIDTH - 8.0;
                let cy = bar_top + 5.0;
                let fill = task
                    .color
                    .as_deref()
                    .map(css_color)
                    .unwrap_or_else(|| TEXT_COLOR.to_string());
                svg.raw_inline(&format!(
                    r#"<polygon fill="{fill}" points="{cx},{t} {r},{cy} {cx},{b} {l},{cy}" style="stroke:{fill};stroke-width:1;"/>"#,
                    cx = fmt_coord(cx),
                    t = fmt_coord(cy - 5.0),
                    r = fmt_coord(cx + 5.0),
                    cy = fmt_coord(cy),
                    b = fmt_coord(cy + 5.0),
                    l = fmt_coord(cx - 5.0),
                ));
            } else {
                let bar_x = start_day as f64 * DAY_WIDTH + 2.0;
                let bar_w = (dur as f64 * DAY_WIDTH - 4.0).max(1.0);
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

                if let Some(pct) = task.completed {
                    let done_w = (bar_w * pct as f64 / 100.0).max(0.0);
                    if done_w > 0.0 {
                        gantt_rect_fill(&mut svg, bar_x, bar_top, done_w, BAR_H, &fill);
                    }
                    let rem_w = bar_w - done_w;
                    if rem_w > 0.0 {
                        gantt_rect_fill(
                            &mut svg,
                            bar_x + done_w,
                            bar_top,
                            rem_w,
                            BAR_H,
                            COMPLETION_FILL,
                        );
                    }
                    gantt_rect_outline(&mut svg, bar_x, bar_top, bar_w, BAR_H, &stroke);
                } else {
                    gantt_rect_fill(&mut svg, bar_x, bar_top, bar_w, BAR_H, &fill);
                    gantt_rect_outline(&mut svg, bar_x, bar_top, bar_w, BAR_H, &stroke);
                }
            }
        }
    }

    // 6. Task / milestone / separator labels.
    for (vi, row) in laid.iter().enumerate() {
        match row {
            LaidRow::Separator(label) => {
                if !label.is_empty() {
                    let ly = row_bar_top(vi) + ascent(TASK_FONT);
                    gantt_text(
                        &mut svg,
                        chart_width / 2.0,
                        ly,
                        label,
                        TASK_FONT,
                        TEXT_COLOR,
                    );
                }
            }
            LaidRow::Task(task, idx) => {
                let (start_day, dur) = resolved[*idx];
                let ly = row_bar_top(vi) + ascent(TASK_FONT);
                let lx = if dur == 0 {
                    start_day as f64 * DAY_WIDTH
                } else {
                    start_day as f64 * DAY_WIDTH + 6.0
                };
                let label = task_label(task);
                gantt_text(&mut svg, lx, ly, &label, TASK_FONT, TEXT_COLOR);
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
        let display_label = if abbreviated {
            abbreviate_month_label(label)
        } else {
            label.clone()
        };
        let tl = text_width(&display_label, MONTH_FONT, true);
        let span_days = (end_idx - start_idx) as f64;
        let center = start_idx as f64 * DAY_WIDTH + span_days * DAY_WIDTH / 2.0;
        let mx = center - tl / 2.0;
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

fn abbreviate_month_label(label: &str) -> String {
    if let Some(pos) = label.find(' ') {
        let month_part = &label[..pos];
        let year_part = &label[pos..];
        let abbr = match month_part {
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
        };
        format!("{}{}", abbr, year_part)
    } else {
        label.to_string()
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
