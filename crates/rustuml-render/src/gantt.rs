// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Gantt chart SVG renderer.
//!
//! Produces a horizontal bar chart with:
//! - A time axis at the top (day numbers, and optional day-of-week/month).
//! - One row per task, each row showing a filled bar.
//! - Dependency arrows from the end of a predecessor to the start of its
//!   successor.
//! - Task labels inside the bars (left-aligned).
//! - An identical time axis repeated at the bottom.

use rustuml_parser::diagram::gantt::{GanttDiagram, GanttNote, GanttRow, GanttTask, TaskStart};

use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ──────────────────────────────────────────────────────────

const DAY_WIDTH: f64 = 22.0;
const ROW_HEIGHT: f64 = 28.0;
const ROW_GAP: f64 = 8.0;
const MARGIN: f64 = 16.0;
const BAR_H: f64 = 18.0;
const FONT_SIZE: f64 = 11.0;
const SMALL_FONT: f64 = 10.0;

// Default bar colour when no task colour is specified.
const DEFAULT_BAR_COLOR: &str = "#E2E2F0";
const DEFAULT_BAR_STROKE: &str = "#181818";

// Row height including gap.
const ROW_STRIDE: f64 = ROW_HEIGHT + ROW_GAP;

// Height of each axis area (top or bottom).
fn axis_height(has_calendar: bool) -> f64 {
    if has_calendar {
        // month row + dow row + daynum row, each ~12px + gaps
        39.0
    } else {
        // just day numbers
        16.0
    }
}

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

/// Render a Gantt diagram to SVG.
pub fn render(diagram: &GanttDiagram, _theme: &Theme) -> String {
    if diagram.tasks.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"60\"></svg>\n"
            .to_string();
    }

    // ── Resolve absolute start days ───────────────────────────────────────────

    // Resolved positions are in working-day units.
    let resolved_wd = resolve_starts(&diagram.tasks);

    // If we have a project start date and closed days, convert working days to
    // calendar days.
    let (resolved, total_days) =
        if !diagram.closed_days.is_empty() && diagram.project_start.is_some() {
            let start_dow = diagram
                .project_start
                .as_deref()
                .and_then(|s| {
                    let parts: Vec<u32> = s.split('-').filter_map(|p| p.parse().ok()).collect();
                    if parts.len() == 3 {
                        Some(zeller_dow(parts[0] as i32, parts[1], parts[2]))
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let cal_resolved: Vec<(u32, u32)> = resolved_wd
                .iter()
                .map(|&(wd_start, wd_dur)| {
                    let cal_start = wd_to_cal(wd_start, start_dow, &diagram.closed_days);
                    // Duration: count cal days needed to cover wd_dur working days.
                    let cal_end = wd_to_cal(wd_start + wd_dur, start_dow, &diagram.closed_days);
                    (cal_start, cal_end - cal_start)
                })
                .collect();
            let has_milestone = diagram.tasks.iter().any(|t| t.duration == 0);
            let total = cal_resolved.iter().map(|&(s, d)| s + d).max().unwrap_or(1);
            let total = if has_milestone { total + 1 } else { total };
            (cal_resolved, total)
        } else {
            let has_milestone = diagram.tasks.iter().any(|t| t.duration == 0);
            let total = resolved_wd.iter().map(|&(s, d)| s + d).max().unwrap_or(1);
            let total = if has_milestone { total + 1 } else { total };
            (resolved_wd, total)
        };

    // ── Calendar info ─────────────────────────────────────────────────────────

    let cal = diagram
        .project_start
        .as_deref()
        .and_then(|s| CalendarInfo::parse(s, total_days));

    let has_cal = cal.is_some();
    let ah = axis_height(has_cal);

    // ── Geometry ──────────────────────────────────────────────────────────────

    // Number of visual rows: use `rows` if populated, else fall back to tasks.
    let n_rows = if !diagram.rows.is_empty() {
        diagram.rows.len()
    } else {
        diagram.tasks.len()
    };
    // Resource utilization rows below main chart.
    let n_resources = diagram.resources.len();
    let resource_row_h = 32.0_f64; // height per resource block
    let resources_h = if n_resources > 0 {
        n_resources as f64 * resource_row_h
    } else {
        0.0
    };
    // Notes below resource rows.
    let note_line_h = 14.0_f64;
    let notes_h: f64 = diagram
        .notes
        .iter()
        .map(|n| n.lines.len() as f64 * note_line_h + 8.0)
        .sum();

    let chart_width = total_days as f64 * DAY_WIDTH;
    let total_width = MARGIN + chart_width + MARGIN;
    let total_height =
        MARGIN + ah + n_rows as f64 * ROW_STRIDE + ah + resources_h + notes_h + MARGIN;

    let chart_x = MARGIN;
    let chart_top = MARGIN + ah;
    let chart_bottom = chart_top + n_rows as f64 * ROW_STRIDE;

    let mut svg = SvgBuilder::new(total_width, total_height);

    // ── Weekend shading ───────────────────────────────────────────────────────

    if let Some(ref c) = cal {
        for (day_idx, &dow) in c.day_of_week.iter().enumerate() {
            if diagram.closed_days.contains(&dow) {
                let gx = chart_x + day_idx as f64 * DAY_WIDTH;
                svg.rect(
                    gx,
                    chart_top,
                    DAY_WIDTH,
                    chart_bottom - chart_top,
                    "#F1E5E5",
                    "none",
                );
            }
        }
    }

    // ── Top axis labels ───────────────────────────────────────────────────────

    let abbreviated = diagram.printscale.as_deref() == Some("weekly");
    if let Some(ref c) = cal {
        render_calendar_axis(&mut svg, c, chart_x, MARGIN, true, total_days, abbreviated);
    } else {
        render_day_numbers(&mut svg, total_days, chart_x, MARGIN + ah - 4.0);
    }

    // ── Vertical grid lines ───────────────────────────────────────────────────

    for day in 0..=total_days {
        let gx = chart_x + day as f64 * DAY_WIDTH;
        svg.line_segment(gx, chart_top, gx, chart_bottom, "#C0C0C0", false);
    }

    // Horizontal borders.
    svg.line_segment(
        chart_x,
        chart_top,
        chart_x + chart_width,
        chart_top,
        "#C0C0C0",
        false,
    );
    svg.line_segment(
        chart_x,
        chart_bottom,
        chart_x + chart_width,
        chart_bottom,
        "#C0C0C0",
        false,
    );

    // ── Task rows ─────────────────────────────────────────────────────────────

    // Build row_y indexed by visual row position.
    let row_y: Vec<f64> = (0..n_rows)
        .map(|i| chart_top + ROW_GAP / 2.0 + i as f64 * ROW_STRIDE)
        .collect();

    // Build owned fallback rows from tasks when `rows` is empty.
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

    // Name → visual row index map.
    let task_visual_row: std::collections::HashMap<&str, usize> = rows_list
        .iter()
        .enumerate()
        .filter_map(|(vi, row)| {
            if let GanttRow::Task(name) = row {
                Some((name.as_str(), vi))
            } else {
                None
            }
        })
        .collect();

    // Render each row.
    for (vi, row) in rows_list.iter().enumerate() {
        let ry = row_y[vi];
        match row {
            GanttRow::Separator(label) => {
                // Draw a thin horizontal line with the label centered.
                let line_y = ry + ROW_HEIGHT / 2.0;
                svg.line_segment(
                    chart_x,
                    line_y,
                    chart_x + chart_width,
                    line_y,
                    "#808080",
                    false,
                );
                if !label.is_empty() {
                    svg.text(
                        chart_x + chart_width / 2.0,
                        line_y - 2.0,
                        label,
                        "middle",
                        FONT_SIZE,
                    );
                }
            }
            GanttRow::Task(name) => {
                if let Some(task) = diagram.tasks.iter().find(|t| &t.name == name) {
                    let task_idx = diagram.tasks.iter().position(|t| &t.name == name).unwrap();
                    let (start_day, dur) = resolved[task_idx];
                    let bar_x = chart_x + start_day as f64 * DAY_WIDTH;
                    let bar_w = dur as f64 * DAY_WIDTH;
                    let bar_y = ry + (ROW_HEIGHT - BAR_H) / 2.0;

                    let fill = task
                        .color
                        .as_deref()
                        .map(css_color)
                        .unwrap_or_else(|| DEFAULT_BAR_COLOR.to_string());
                    let stroke = task
                        .color
                        .as_deref()
                        .map(|_| DEFAULT_BAR_STROKE.to_string())
                        .unwrap_or_else(|| DEFAULT_BAR_STROKE.to_string());

                    if dur == 0 {
                        // Milestone: diamond at the start position.
                        let mx = bar_x;
                        let my = ry + ROW_HEIGHT / 2.0;
                        let s = 6.0_f64;
                        svg.raw(&format!(
                            r#"  <polygon points="{},{} {},{} {},{} {},{}" fill="{}" stroke="{}"/>"#,
                            mx, my - s,
                            mx + s, my,
                            mx, my + s,
                            mx - s, my,
                            fill, stroke
                        ));
                    } else {
                        // Fill rect (no stroke).
                        svg.rect(bar_x, bar_y, bar_w.max(2.0), BAR_H, &fill, "none");
                        // Outline rect (no fill).
                        svg.rect(bar_x, bar_y, bar_w.max(2.0), BAR_H, "none", &stroke);
                    }

                    // Task label: include resource(s) in label if present.
                    let label = if task.resources.is_empty() {
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
                    };
                    let label_x = bar_x + 4.0;
                    let label_y = bar_y + BAR_H / 2.0 + 4.0;
                    svg.text(label_x, label_y, &label, "start", FONT_SIZE);
                }
            }
        }
    }

    // ── Dependency arrows ─────────────────────────────────────────────────────

    for (task_idx, task) in diagram.tasks.iter().enumerate() {
        if let TaskStart::AfterTask(dep_name) = &task.start
            && let Some(dep_task_idx) = diagram.tasks.iter().position(|t| &t.name == dep_name)
            && let Some(&vi) = task_visual_row.get(task.name.as_str())
            && let Some(&dep_vi) = task_visual_row.get(dep_name.as_str())
        {
            let (dep_start, dep_dur) = resolved[dep_task_idx];
            let ax1 = chart_x + (dep_start + dep_dur) as f64 * DAY_WIDTH;
            let ay1 = row_y[dep_vi] + ROW_HEIGHT / 2.0;
            let ax2 = chart_x + resolved[task_idx].0 as f64 * DAY_WIDTH;
            let ay2 = row_y[vi] + ROW_HEIGHT / 2.0;
            draw_elbow_arrow(&mut svg, ax1, ay1, ax2, ay2);
        }
    }

    // ── Bottom axis labels ────────────────────────────────────────────────────

    if let Some(ref c) = cal {
        render_calendar_axis(
            &mut svg,
            c,
            chart_x,
            chart_bottom,
            false,
            total_days,
            abbreviated,
        );
    } else {
        render_day_numbers(&mut svg, total_days, chart_x, chart_bottom + 14.0);
    }

    // ── Resource utilization rows ─────────────────────────────────────────────

    if !diagram.resources.is_empty() {
        let mut resource_y = chart_bottom + ah + MARGIN;
        for res in &diagram.resources {
            // Resource name label.
            svg.raw(&format!(
                r#"  <text x="{x}" y="{y}" text-anchor="start" font-family="Serif" font-size="13">{name}</text>"#,
                x = chart_x,
                y = resource_y + 13.0,
                name = res.name,
            ));
            // Compute utilization per calendar day (sum of percentages from all tasks).
            let util_y = resource_y + 20.0;
            let mut day_utilization: std::collections::HashMap<u32, u32> =
                std::collections::HashMap::new();
            for (task_idx, task) in diagram.tasks.iter().enumerate() {
                // Find this resource's contribution to this task.
                let percent: u32 = task
                    .resources
                    .iter()
                    .filter(|tr| tr.name == res.name)
                    .map(|tr| tr.percent)
                    .sum();
                if percent > 0 {
                    let (s, d) = resolved[task_idx];
                    for day in s..s + d {
                        *day_utilization.entry(day).or_insert(0) += percent;
                    }
                }
            }
            // Render utilization labels.
            let mut days_sorted: Vec<u32> = day_utilization.keys().cloned().collect();
            days_sorted.sort();
            for day in days_sorted {
                let util = day_utilization[&day];
                let tx = chart_x + day as f64 * DAY_WIDTH + DAY_WIDTH / 2.0;
                svg.text(tx, util_y, &util.to_string(), "middle", 9.0);
            }
            resource_y += resource_row_h;
        }
    }

    // ── Notes ─────────────────────────────────────────────────────────────────

    render_notes(
        &mut svg,
        &diagram.notes,
        chart_x,
        chart_bottom + ah + resources_h + MARGIN,
    );

    // ── Title ─────────────────────────────────────────────────────────────────

    if let Some(title) = &diagram.meta.title {
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

/// Render note blocks below the chart.
fn render_notes(svg: &mut SvgBuilder, notes: &[GanttNote], chart_x: f64, start_y: f64) {
    let mut y = start_y;
    for note in notes {
        for line in &note.lines {
            y += 14.0;
            svg.text(chart_x, y, line, "start", FONT_SIZE);
        }
        y += 8.0;
    }
}

/// Render a row of day numbers (1..=total_days) at the given y.
fn render_day_numbers(svg: &mut SvgBuilder, total_days: u32, chart_x: f64, y: f64) {
    for day in 0..total_days {
        let tx = chart_x + day as f64 * DAY_WIDTH + DAY_WIDTH / 2.0;
        svg.text(tx, y, &(day + 1).to_string(), "middle", SMALL_FONT);
    }
}

/// Calendar information for a day-based timeline.
struct CalendarInfo {
    /// Day-of-week for each calendar day (0=Monday..6=Sunday).
    day_of_week: Vec<u8>,
    /// Calendar day-of-month for each day index.
    day_of_month: Vec<u8>,
    /// Month spans: (start_day_index, end_day_index_exclusive, "Month Year").
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
            // Record month boundary.
            if i == 0 || cur_day == 1 {
                let label = format!("{} {}", month_name(cur_month), cur_year);
                month_spans.push((i, 0, label));
            }

            day_of_week.push(cur_dow);
            day_of_month.push(cur_day as u8);

            // Advance one calendar day.
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

        // Fill in end indices.
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

/// Day-of-week abbreviations (0=Monday..6=Sunday).
const DOW_ABBR: &[&str] = &["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Render top or bottom calendar axis (month names, day-of-week, day numbers).
/// `area_top` is the y coordinate where this axis area starts.
/// `abbreviated` controls whether month names are abbreviated (e.g. "Jan 2024").
fn render_calendar_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    chart_x: f64,
    area_top: f64,
    _is_top: bool,
    _total_days: u32,
    abbreviated: bool,
) {
    let month_y = area_top + 12.0;
    let dow_y = area_top + 24.0;
    let daynum_y = area_top + 36.0;

    // Month/year labels (bold, centred over their span).
    // When abbreviated, reformat "January 2024" → "Jan 2024".
    for &(start_idx, end_idx, ref label) in &cal.month_spans {
        let display_label = if abbreviated {
            abbreviate_month_label(label)
        } else {
            label.clone()
        };
        let span_days = (end_idx - start_idx) as f64;
        let mx = chart_x + start_idx as f64 * DAY_WIDTH + span_days * DAY_WIDTH / 2.0;
        // Emit bold text via raw SVG.
        svg.raw(&format!(
            r#"  <text x="{mx}" y="{month_y}" text-anchor="middle" font-family="sans-serif" font-size="12" font-weight="700">{display_label}</text>"#
        ));
    }

    // Day-of-week abbreviations.
    for (day_idx, &dow) in cal.day_of_week.iter().enumerate() {
        let tx = chart_x + day_idx as f64 * DAY_WIDTH + DAY_WIDTH / 2.0;
        svg.text(tx, dow_y, DOW_ABBR[dow as usize], "middle", SMALL_FONT);
    }

    // Day-of-month numbers.
    for (day_idx, &dom) in cal.day_of_month.iter().enumerate() {
        let tx = chart_x + day_idx as f64 * DAY_WIDTH + DAY_WIDTH / 2.0;
        svg.text(tx, daynum_y, &dom.to_string(), "middle", SMALL_FONT);
    }
}

/// Compute day-of-week (0=Monday..6=Sunday) for a given date using
/// Tomohiko Sakamoto's algorithm.
fn zeller_dow(year: i32, month: u32, day: u32) -> u8 {
    static T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let m = month as i32;
    let d = day as i32;
    let _ = m;
    let dow = (y + y / 4 - y / 100 + y / 400 + T[(month as usize) - 1] + d) % 7;
    // Sakamoto returns 0=Sunday; shift to 0=Monday.
    ((dow + 6) % 7) as u8
}

/// Return the number of days in a given month/year.
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

/// Convert "January 2024" → "Jan 2024".
fn abbreviate_month_label(label: &str) -> String {
    // label format is "{MonthName} {Year}"
    if let Some(pos) = label.find(' ') {
        let month_part = &label[..pos];
        let year_part = &label[pos..];
        // Map full name to abbreviated.
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

/// English month names.
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

/// Convert a working-day offset to a calendar-day offset.
///
/// `working_day` is the 0-based working-day index (0 = project start).
/// `start_dow` is the day-of-week of project start (0=Monday..6=Sunday).
/// `closed_days` lists which days of the week are non-working (0=Mon..6=Sun).
///
/// Returns the calendar-day index for the start of the given working day.
fn wd_to_cal(working_day: u32, start_dow: u8, closed_days: &[u8]) -> u32 {
    if closed_days.is_empty() {
        return working_day;
    }
    // Count up calendar days until we've accumulated `working_day` open days.
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
        // Safety: prevent infinite loop if all days are closed.
        if cal > working_day * 7 + 14 {
            return cal;
        }
    }
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

/// Draw an L-shaped elbow arrow from (x1,y1) to (x2,y2).
fn draw_elbow_arrow(svg: &mut SvgBuilder, x1: f64, y1: f64, x2: f64, y2: f64) {
    let color = "#181818";
    svg.line_segment(x1, y1, x2, y1, color, false);
    svg.line_segment(x2, y1, x2, y2, color, false);
    svg.arrow_head(x2, y2, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_diagram() -> GanttDiagram {
        use rustuml_parser::diagram::gantt::GanttRow;
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
                GanttTask {
                    name: "Task 1".into(),
                    duration: 5,
                    start: TaskStart::Day(0),
                    color: None,
                    resources: Vec::new(),
                },
                GanttTask {
                    name: "Task 2".into(),
                    duration: 3,
                    start: TaskStart::AfterTask("Task 1".into()),
                    color: None,
                    resources: Vec::new(),
                },
                GanttTask {
                    name: "Task 3".into(),
                    duration: 2,
                    start: TaskStart::AfterTask("Task 2".into()),
                    color: None,
                    resources: Vec::new(),
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
            GanttTask {
                name: "A".into(),
                duration: 3,
                start: TaskStart::Day(0),
                color: None,
                resources: Vec::new(),
            },
            GanttTask {
                name: "B".into(),
                duration: 2,
                start: TaskStart::AfterTask("A".into()),
                color: None,
                resources: Vec::new(),
            },
            GanttTask {
                name: "C".into(),
                duration: 4,
                start: TaskStart::AfterTask("B".into()),
                color: None,
                resources: Vec::new(),
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

    #[test]
    fn calendar_header_rendered() {
        let input = "@startgantt\n\
                     Project starts 2024-01-01\n\
                     saturday are closed\n\
                     sunday are closed\n\
                     [Auth] lasts 6 days\n\
                     @endgantt";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Mo"), "should contain day-of-week Mo");
        assert!(svg.contains("January"), "should contain month name");
    }

    #[test]
    fn dow_calculation() {
        // 2024-01-01 is a Monday → dow 0.
        assert_eq!(zeller_dow(2024, 1, 1), 0);
        // 2024-01-06 is a Saturday → dow 5.
        assert_eq!(zeller_dow(2024, 1, 6), 5);
        // 2024-01-07 is a Sunday → dow 6.
        assert_eq!(zeller_dow(2024, 1, 7), 6);
    }

    #[test]
    fn day_numbers_repeated_top_and_bottom() {
        let input = "@startgantt\n\
                     [T1] lasts 5 days\n\
                     @endgantt";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        // Day number "5" should appear twice (top and bottom axis).
        assert_eq!(svg.matches(">5<").count(), 2, "day 5 should appear twice");
    }
}
