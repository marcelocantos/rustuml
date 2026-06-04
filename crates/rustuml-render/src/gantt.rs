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
use crate::plantuml_metrics::{
    ascent, descent, fmt_coord, serif_text_width, text_height, text_width,
};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants (matched to PlantUML's daily Gantt geometry) ───────────────

const DAY_WIDTH: f64 = 16.0;
const WEEKLY_DAY_WIDTH: f64 = 4.0;
const MONTHLY_DAY_WIDTH: f64 = 32.0 / 30.0;
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

// ── Note (Opale) geometry, matching net.sourceforge.plantuml.svek.image.Opale ──
// A note attached to a task is drawn as an "opale" box: a rectangle with a
// folded top-right corner. Its width is the widest text line plus left/right
// margins; its height is the stacked text plus top/bottom margins.
const NOTE_FONT: f64 = 9.0;
/// Opale.marginX1 (left text inset).
const NOTE_MARGIN_X1: f64 = 6.0;
/// Opale.marginX2 (right text inset).
const NOTE_MARGIN_X2: f64 = 15.0;
/// Opale.marginY (top and bottom text inset).
const NOTE_MARGIN_Y: f64 = 5.0;
/// Opale.cornersize (the folded corner is a 10×10 triangle).
const NOTE_CORNER: f64 = 10.0;
/// Gantt task style margin (top == bottom). The note sits this far below the
/// task bar's bottom edge, and the next overlapping task sits this far below
/// the note's bottom edge.
const TASK_MARGIN: f64 = 2.0;
const NOTE_FILL: &str = "#FEFFDD";
const NOTE_STROKE: &str = "#181818";

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
const WEEKLY_GRID_TOP: f64 = 16.0;
const WEEKLY_BAR_TOP: f64 = 29.0;
const WEEKLY_WEEK_LABEL_Y: f64 = 25.668;
const WEEKLY_WEEK_RULE_Y: f64 = 27.0;
const SCALED_MONTH_ROW_H: f64 = 14.0;
const SCALED_YEAR_ROW_H: f64 = 16.0;
const MONTHLY_GRID_TOP: f64 = SCALED_YEAR_ROW_H + SCALED_MONTH_ROW_H;
const MONTHLY_BAR_TOP: f64 = MONTHLY_GRID_TOP + 2.0;

const CAL_BOT_DOW_OFF: f64 = 9.66796875;
const CAL_BOT_DAYNUM_OFF: f64 = 23.66796875;
const CAL_BOT_MONTH_OFF: f64 = 38.60156875;
const SCALED_AXIS_MONTH_BASELINE: f64 = 9.66796875;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrintScale {
    Daily,
    Weekly,
    Monthly,
}

impl PrintScale {
    fn from_str(scale: Option<&str>) -> Self {
        match scale {
            Some("weekly") => Self::Weekly,
            Some("monthly") => Self::Monthly,
            _ => Self::Daily,
        }
    }

    fn day_width(self) -> f64 {
        match self {
            Self::Daily => DAY_WIDTH,
            Self::Weekly => WEEKLY_DAY_WIDTH,
            Self::Monthly => MONTHLY_DAY_WIDTH,
        }
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

/// A note (Opale box) attached to a task row, with its computed dimensions.
struct RowNote {
    /// Left edge (the attached task's start position).
    x: f64,
    /// Box width: widest line + left/right margins.
    width: f64,
    /// Box height: stacked text + top/bottom margins.
    height: f64,
    /// Text lines (escaped at draw time).
    lines: Vec<String>,
    /// Per-line text length (SVG `textLength`), parallel to `lines`.
    line_widths: Vec<f64>,
}

impl RowNote {
    fn new(lines: &[String], x: f64) -> Self {
        // AWT SansSerif advances are perfectly linear in point size, so the
        // size-9 width is the size-10 metric scaled by 0.9 (no size-9 table
        // exists, and the global metric fallback would use size 12).
        let line_widths: Vec<f64> = lines
            .iter()
            .map(|l| text_width(l, 10.0, false) * (NOTE_FONT / 10.0))
            .collect();
        let text_w = line_widths.iter().cloned().fold(0.0_f64, f64::max);
        let text_h = lines.len() as f64 * text_height(NOTE_FONT);
        RowNote {
            x,
            width: text_w + NOTE_MARGIN_X1 + NOTE_MARGIN_X2,
            height: text_h + 2.0 * NOTE_MARGIN_Y,
            lines: lines.to_vec(),
            line_widths,
        }
    }
}

/// Render a Gantt diagram to SVG.
pub fn render(diagram: &GanttDiagram, _theme: &Theme) -> String {
    if diagram.tasks.is_empty() {
        let svg = SvgBuilder::new_plantuml(81.0, 82.0, "GANTT");
        return svg.finalize_plantuml();
    }

    let print_scale = PrintScale::from_str(diagram.printscale.as_deref());
    let day_width = print_scale.day_width();
    let day_x = |day: u32| day as f64 * day_width;

    let resolved_wd = resolve_starts(&diagram.tasks);
    // Closures comprise repeating weekday closures plus specific holiday dates.
    // Both require a project start to position them on the calendar.
    let closures = diagram
        .project_start
        .as_deref()
        .map(|ps| Closures::new(ps, &diagram.closed_days, &diagram.closed_dates));
    let has_closed = closures.as_ref().is_some_and(Closures::any);
    let (resolved, total_days) = if has_closed {
        let closures = closures.as_ref().unwrap();
        let cal: Vec<(u32, u32)> = resolved_wd
            .iter()
            .map(|&(wd_start, wd_dur)| {
                let cal_start = closures.wd_to_cal(wd_start);
                // The visible end is one column past the task's last open day.
                // Using wd_to_cal(wd_start + wd_dur) would instead land on the
                // first open day *after* any trailing weekend, over-extending
                // the bar (and the chart width) by the skipped closed days.
                let cal_end = if wd_dur == 0 {
                    cal_start
                } else {
                    closures.wd_to_cal(wd_start + wd_dur - 1) + 1
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

    // Per-column closed flag for the calendar range (weekday + holiday dates).
    let closed_cols: Vec<bool> = match (&closures, has_closed) {
        (Some(c), true) => (0..total_days).map(|col| c.is_col_closed(col)).collect(),
        _ => vec![false; total_days as usize],
    };
    let is_closed_col =
        |col: u32| -> bool { closed_cols.get(col as usize).copied().unwrap_or(false) };

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

    let chart_width = day_x(total_days);
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
                    (day_x(start_day) - 8.0).max(8.0) + 8.0,
                    label_w.max(MILESTONE_LABEL_MIN_W),
                )
            } else {
                let bar_x = day_x(start_day) + 2.0;
                let bar_w = (dur as f64 * day_width - 4.0).max(1.0);
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

    // A parallel-start arrow ("starts at X's start") routes out to the left of
    // the bar and back in. Its arrowhead polygon pushes the drawn content's
    // left edge negative: PlantUML's LimitFinder pads polygons by 10px on each
    // side (HACK_X_FOR_POLYGON), and the arrowhead's leftmost point sits 4px
    // left of the successor's start. The image width is (maxX - minX) + 1, so a
    // negative minX widens the canvas.
    let mut content_min_x = 0.0_f64;
    for row in &laid {
        if let LaidRow::Task(task, idx) = row
            && let TaskStart::WithTask(dep) = &task.start
            && resolved[*idx].1 != 0
            && diagram.tasks.iter().any(|t| &t.name == dep)
        {
            let succ_start_x = day_x(resolved[*idx].0);
            content_min_x = content_min_x.min(succ_start_x - 4.0 - 10.0);
        }
    }
    let total_width = (chart_width.max(label_right) - content_min_x) + 1.0;

    // A `title` pushes the whole chart down by a fixed band: 10px top pad,
    // one title line, then an 11px bottom gap before the calendar/grid.
    let title_h = if diagram.meta.title.is_some() {
        TITLE_TOP_PAD + TITLE_LINE_H + TITLE_BOTTOM_PAD
    } else {
        0.0
    };

    let grid_top = title_h
        + if has_cal {
            match print_scale {
                PrintScale::Daily => CAL_GRID_TOP,
                PrintScale::Weekly => WEEKLY_GRID_TOP,
                PrintScale::Monthly => MONTHLY_GRID_TOP,
            }
        } else {
            GRID_TOP_PLAIN
        };
    let bar_top0 = title_h
        + if has_cal {
            match print_scale {
                PrintScale::Daily => CAL_BAR_TOP,
                PrintScale::Weekly => WEEKLY_BAR_TOP,
                PrintScale::Monthly => MONTHLY_BAR_TOP,
            }
        } else {
            BAR_TOP_PLAIN
        };

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

    // A note attached to a task is laid out below that task's bar. Compute
    // each note's box (lines + dimensions) and the row it belongs to. The
    // box top sits `BAR_H + TASK_MARGIN` below the row's bar top (PlantUML's
    // getYNotePosition), spanning from the task's left edge.
    let row_notes: Vec<Option<RowNote>> = laid
        .iter()
        .map(|row| match row {
            LaidRow::Task(task, idx) => diagram
                .notes
                .iter()
                .find(|n| n.task == task.name && !n.lines.is_empty())
                .map(|n| RowNote::new(&n.lines, day_x(resolved[*idx].0))),
            LaidRow::Separator(_) => None,
        })
        .collect();

    // Resolve note/task overlaps (PlantUML's TaskDrawRegistryData.resolveNoteOverlaps).
    // A task whose bar overlaps an earlier note's box — both horizontally and
    // vertically — is pushed down so it clears the note. Because PlantUML's
    // layout chains task Y constraints, pushing one row shifts every later row
    // (and its attached note) by the same delta; earlier notes stay put.
    if row_notes.iter().any(|n| n.is_some()) {
        for vi in 0..n_rows {
            // Bar fingerprint of this task row.
            let (bar_x0, bar_x1, bar_y0, bar_y1) = match &laid[vi] {
                LaidRow::Task(_, idx) => {
                    let (sd, dur) = resolved[*idx];
                    let x0 = day_x(sd);
                    let x1 = day_x(sd + dur.max(1));
                    let y0 = row_tops[vi];
                    (x0, x1, y0, y0 + ROW_STRIDE)
                }
                LaidRow::Separator(_) => continue,
            };
            let mut required_top = row_tops[vi];
            for nvi in 0..vi {
                if let Some(note) = &row_notes[nvi] {
                    let n_top = row_tops[nvi] + BAR_H + TASK_MARGIN;
                    let n_bot = n_top + note.height;
                    let n_x0 = note.x;
                    let n_x1 = note.x + note.width;
                    // 2D fingerprint overlap (FingerPrint.overlap).
                    let x_overlap = bar_x0 < n_x1 && n_x0 < bar_x1;
                    let y_overlap = bar_y0 < n_bot && n_top < bar_y1;
                    if x_overlap && y_overlap {
                        required_top = required_top.max(n_bot + TASK_MARGIN);
                    }
                }
            }
            let delta = required_top - row_tops[vi];
            if delta > 0.0 {
                for t in row_tops.iter_mut().skip(vi) {
                    *t += delta;
                }
            }
        }
    }
    acc = row_tops[n_rows];

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

    // A note whose box hangs below the last row extends the content bottom
    // (PlantUML's computeBottomY maxes task.Y + heightMax over all tasks; a
    // noted task contributes its note's bottom edge). The +TASK_MARGIN keeps
    // the same offset the row accumulator carries past the last bar.
    let note_bottom_extent = laid
        .iter()
        .enumerate()
        .filter_map(|(vi, _)| {
            row_notes[vi]
                .as_ref()
                .map(|note| row_tops[vi] + BAR_H + TASK_MARGIN + note.height + TASK_MARGIN)
        })
        .fold(0.0_f64, f64::max);

    let grid_bottom = if has_cal {
        rows_extent + 2.0
    } else if n_res > 0 {
        res_section_top + n_res as f64 * RES_ROW_STRIDE + RES_BOTTOM_PAD
    } else {
        acc.max(note_bottom_extent) + GRID_BOTTOM_PAD_PLAIN
    };

    let total_height = if has_cal {
        match print_scale {
            PrintScale::Daily => {
                // Bottom-most drawn element is the month-label text; the SVG box is
                // its baseline plus the font descent, rounded up to a whole pixel.
                (grid_bottom + CAL_BOT_MONTH_OFF + descent(MONTH_FONT)).ceil()
            }
            PrintScale::Weekly => (grid_bottom + SCALED_YEAR_ROW_H + 1.0).ceil(),
            PrintScale::Monthly => {
                (grid_bottom + SCALED_MONTH_ROW_H + SCALED_YEAR_ROW_H + 1.0).ceil()
            }
        }
    } else {
        // PlantUML composes the title as a fixed band of height `title_h`
        // stacked above the chart body (ImageBuilder), so the image height is
        // ceil(title_h + body_bound). The body bound is the LimitFinder min/max
        // over the body's drawn content: the top day-number axis to the bottom
        // day-number axis, each text run bounded from baseline - textHeight +
        // 1.5 to baseline + 1.5. (grid_bottom and the top-axis baseline both
        // carry the +title_h offset, so their difference is title-independent;
        // the title band is then added back explicitly.)
        let bottom_baseline = grid_bottom + BOTTOM_DAYNUM_OFF_PLAIN;
        let max_y = bottom_baseline + 1.5;
        let top_day_baseline = title_h + ascent(AXIS_FONT);
        let top_day_top = top_day_baseline - text_height(AXIS_FONT) + 1.5;
        let body_bound = max_y - top_day_top;
        (title_h + body_bound).ceil()
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

    // 1. Weekend / holiday shading (calendar only).
    if cal.is_some() {
        let mut day_idx = 0usize;
        while (day_idx as u32) < total_days {
            if is_closed_col(day_idx as u32) {
                // Coalesce consecutive closed days into one rect.
                let start = day_idx;
                while (day_idx as u32) < total_days && is_closed_col(day_idx as u32) {
                    day_idx += 1;
                }
                let gx = day_x(start as u32);
                let w = (day_idx - start) as f64 * day_width;
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
        match print_scale {
            PrintScale::Daily => {
                render_calendar_axis(
                    &mut svg,
                    c,
                    diagram,
                    &closed_cols,
                    CalendarAxisGeom {
                        day_width,
                        dow_y: title_h + CAL_DOW_Y,
                        daynum_y: title_h + CAL_DAYNUM_Y,
                        month_y: title_h + CAL_MONTH_Y,
                    },
                );
                for day in 0..=total_days {
                    let gx = day_x(day);
                    gantt_line(&mut svg, gx, grid_top, gx, grid_bottom, GRID_COLOR);
                }
                let x_end = chart_width - 0.0002;
                gantt_line(&mut svg, 0.0, grid_top, x_end, grid_top, GRID_COLOR);
                gantt_line(&mut svg, 0.0, grid_bottom, x_end, grid_bottom, GRID_COLOR);
            }
            PrintScale::Weekly => {
                render_weekly_top_axis(&mut svg, c, total_days, chart_width, grid_bottom, title_h);
            }
            PrintScale::Monthly => {
                render_monthly_top_axis(&mut svg, c, chart_width, title_h);
            }
        }
    } else {
        for day in 0..=total_days {
            let gx = day_x(day);
            gantt_line(&mut svg, gx, grid_top, gx, grid_bottom, GRID_COLOR);
        }
        render_day_numbers(&mut svg, total_days, day_width, title_h + ascent(AXIS_FONT));
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
            let pred_end_x = day_x(dep_start + dep_dur);
            // From a milestone predecessor (zero-duration diamond) the arrow
            // departs the diamond centre rather than the bottom of a bar.
            let pred_exit_y = if dep_dur == 0 {
                row_bar_top(dep_vi) + 5.0
            } else {
                row_bar_top(dep_vi) + BAR_H
            };
            let succ_start_x = day_x(resolved[*idx].0);
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
            let pred_left_x = day_x(resolved[*idx].0) + 2.0;
            let pred_center = row_bar_top(dep_vi) + BAR_H / 2.0;
            let succ_start_x = day_x(resolved[*idx].0);
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

    // 4b. Notes (Opale boxes). Drawn after dependency arrows and before the
    // bars, matching PlantUML's document order (drawNote runs first in
    // TaskDrawRegular.drawU). The box top sits BAR_H + TASK_MARGIN below the
    // task's bar top.
    for (vi, _) in laid.iter().enumerate() {
        if let Some(note) = &row_notes[vi] {
            let note_top = row_bar_top(vi) + BAR_H + TASK_MARGIN;
            draw_note(&mut svg, note, note_top);
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
                let cx = (day_x(start_day) - 8.0).max(8.0);
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
                let is_closed = |col: u32| -> bool { has_cal && is_closed_col(col) };
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
                    let left = if first { day_x(cs) + 2.0 } else { day_x(cs) };
                    let fill_right = if last {
                        day_x(ce) - 2.0
                    } else {
                        day_x(ce) + 1.0
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
                        let r = day_x(ce);
                        svg.raw_inline(&format!(
                            r#"<path d="M{r},{bot} L{l},{bot} L{l},{top} L{r},{top}" fill="none" style="stroke:{stroke};stroke-width:1;"/>"#,
                            r = fmt_coord(r),
                            bot = fmt_coord(bot),
                            l = fmt_coord(left),
                            top = fmt_coord(top),
                        ));
                    } else if last {
                        let r = day_x(ce) - 2.0;
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
                        let r = day_x(ce);
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
                        let gap_left = day_x(w[0].1) + 3.0;
                        let gap_right = day_x(w[1].0) - 3.0;
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
                    (day_x(start_day) - 8.0).max(8.0) + 8.0
                } else {
                    // Labels normally sit inside the bar (4px from its left
                    // edge). The label fits inside only when the bar's 6px
                    // interior inset (span bar_w-8) strictly exceeds the label
                    // width; otherwise PlantUML places it just past the bar's
                    // right edge instead.
                    let bar_x = day_x(start_day) + 2.0;
                    let bar_w = (dur as f64 * day_width - 4.0).max(1.0);
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
                    day as f64 * day_width + RES_LOAD_X_OFF,
                    load_y,
                    &load.to_string(),
                    RES_LOAD_FONT,
                );
            }
        }
    }

    // 7. Bottom axis.
    if let Some(ref c) = cal {
        match print_scale {
            PrintScale::Daily => render_calendar_axis(
                &mut svg,
                c,
                diagram,
                &closed_cols,
                CalendarAxisGeom {
                    day_width,
                    dow_y: grid_bottom + CAL_BOT_DOW_OFF,
                    daynum_y: grid_bottom + CAL_BOT_DAYNUM_OFF,
                    month_y: grid_bottom + CAL_BOT_MONTH_OFF,
                },
            ),
            PrintScale::Weekly => render_weekly_bottom_axis(&mut svg, c, chart_width, grid_bottom),
            PrintScale::Monthly => {
                render_monthly_bottom_axis(&mut svg, c, chart_width, grid_bottom)
            }
        }
    } else {
        render_day_numbers(
            &mut svg,
            total_days,
            day_width,
            grid_bottom + BOTTOM_DAYNUM_OFF_PLAIN,
        );
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

fn render_day_numbers(svg: &mut SvgBuilder, total_days: u32, day_width: f64, baseline: f64) {
    for day in 0..total_days {
        let label = (day + 1).to_string();
        let tl = text_width(&label, AXIS_FONT, false);
        let cell_x = day as f64 * day_width;
        let tx = cell_x + (day_width - tl) / 2.0;
        gantt_text(svg, tx, baseline, &label, AXIS_FONT, TEXT_COLOR);
    }
}

/// Draw an Opale note box (rounded-corner-fold rectangle) plus its text
/// lines, with the box top-left at `(note.x, top)`. Geometry mirrors
/// net.sourceforge.plantuml.svek.image.Opale (roundCorner == 0).
fn draw_note(svg: &mut SvgBuilder, note: &RowNote, top: f64) {
    let x = note.x;
    let w = note.width;
    let h = note.height;
    let r = x + w; // right edge
    let b = top + h; // bottom edge
    let fold_x = r - NOTE_CORNER; // x of the diagonal fold start
    let fold_y = top + NOTE_CORNER; // y where the right edge meets the fold

    // Main body outline: top-left → bottom-left → bottom-right → up to the
    // fold → diagonal to the folded top → back to the start.
    svg.raw_inline(&format!(
        r#"<path d="M{x},{top} L{x},{b} L{r},{b} L{r},{fy} L{fx},{top} L{x},{top}" fill="{NOTE_FILL}" style="stroke:{NOTE_STROKE};stroke-width:0.5;"/>"#,
        x = fmt_coord(x),
        top = fmt_coord(top),
        b = fmt_coord(b),
        r = fmt_coord(r),
        fy = fmt_coord(fold_y),
        fx = fmt_coord(fold_x),
    ));
    // Folded corner triangle.
    svg.raw_inline(&format!(
        r#"<path d="M{fx},{top} L{fx},{fy} L{r},{fy} L{fx},{top}" fill="{NOTE_FILL}" style="stroke:{NOTE_STROKE};stroke-width:0.5;"/>"#,
        fx = fmt_coord(fold_x),
        top = fmt_coord(top),
        fy = fmt_coord(fold_y),
        r = fmt_coord(r),
    ));
    // Text lines, left-aligned at marginX1, stacked from marginY down.
    let text_x = x + NOTE_MARGIN_X1;
    let mut baseline = top + NOTE_MARGIN_Y + ascent(NOTE_FONT);
    for (line, &tl) in note.lines.iter().zip(&note.line_widths) {
        svg.raw_inline(&format!(
            r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="9" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{}</text>"#,
            escape_xml(line),
            tl = fmt_coord(tl),
            x = fmt_coord(text_x),
            y = fmt_coord(baseline),
        ));
        baseline += text_height(NOTE_FONT);
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

struct CalendarAxisGeom {
    day_width: f64,
    dow_y: f64,
    daynum_y: f64,
    month_y: f64,
}

fn render_calendar_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    diagram: &GanttDiagram,
    closed_cols: &[bool],
    geom: CalendarAxisGeom,
) {
    let abbreviated = diagram.printscale.as_deref() == Some("weekly");
    let is_closed = |idx: usize| closed_cols.get(idx).copied().unwrap_or(false);

    for (day_idx, &dow) in cal.day_of_week.iter().enumerate() {
        let abbr = DOW_ABBR[dow as usize];
        let fill = if is_closed(day_idx) {
            CLOSED_TEXT_COLOR
        } else {
            TEXT_COLOR
        };
        let tl = text_width(abbr, AXIS_FONT, false);
        let tx = day_idx as f64 * geom.day_width + (geom.day_width - tl) / 2.0;
        gantt_text(svg, tx, geom.dow_y, abbr, AXIS_FONT, fill);
    }

    for (day_idx, &dom) in cal.day_of_month.iter().enumerate() {
        let label = dom.to_string();
        let fill = if is_closed(day_idx) {
            CLOSED_TEXT_COLOR
        } else {
            TEXT_COLOR
        };
        let tl = text_width(&label, AXIS_FONT, false);
        let tx = day_idx as f64 * geom.day_width + (geom.day_width - tl) / 2.0;
        gantt_text(svg, tx, geom.daynum_y, &label, AXIS_FONT, fill);
    }

    for &(start_idx, end_idx, ref label) in &cal.month_spans {
        let span_days = (end_idx - start_idx) as f64;
        let span_w = span_days * geom.day_width;
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
            start_idx as f64 * geom.day_width
        } else {
            start_idx as f64 * geom.day_width + span_w / 2.0 - tl / 2.0
        };
        gantt_text_bold(svg, mx, geom.month_y, &display_label, MONTH_FONT);
    }
}

fn render_weekly_top_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    total_days: u32,
    chart_width: f64,
    grid_bottom: f64,
    title_h: f64,
) {
    render_week_numbers(svg, total_days, title_h + WEEKLY_WEEK_LABEL_Y);
    for day in weekly_boundaries(total_days) {
        let x = day as f64 * WEEKLY_DAY_WIDTH;
        gantt_line(
            svg,
            x,
            title_h + WEEKLY_GRID_TOP,
            x,
            grid_bottom,
            GRID_COLOR,
        );
    }
    render_weekly_month_axis(svg, cal, title_h, title_h + SCALED_YEAR_ROW_H);
    gantt_line(svg, 0.0, title_h, chart_width, title_h, GRID_COLOR);
    gantt_line(
        svg,
        0.0,
        title_h + SCALED_YEAR_ROW_H,
        chart_width,
        title_h + SCALED_YEAR_ROW_H,
        GRID_COLOR,
    );
    gantt_line(
        svg,
        0.0,
        title_h + WEEKLY_WEEK_RULE_Y,
        chart_width,
        title_h + WEEKLY_WEEK_RULE_Y,
        GRID_COLOR,
    );
}

fn render_weekly_bottom_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    chart_width: f64,
    grid_bottom: f64,
) {
    gantt_line(svg, 0.0, grid_bottom, chart_width, grid_bottom, GRID_COLOR);
    render_weekly_month_axis(svg, cal, grid_bottom, grid_bottom + SCALED_YEAR_ROW_H);
    gantt_line(
        svg,
        0.0,
        grid_bottom + SCALED_YEAR_ROW_H,
        chart_width,
        grid_bottom + SCALED_YEAR_ROW_H,
        GRID_COLOR,
    );
}

fn render_monthly_top_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    chart_width: f64,
    title_h: f64,
) {
    render_year_axis(svg, cal, title_h, title_h + SCALED_YEAR_ROW_H, chart_width);
    render_month_name_axis(
        svg,
        cal,
        title_h + SCALED_YEAR_ROW_H,
        title_h + MONTHLY_GRID_TOP,
        title_h + SCALED_YEAR_ROW_H + SCALED_AXIS_MONTH_BASELINE,
    );
    gantt_line(svg, 0.0, title_h, chart_width, title_h, GRID_COLOR);
    gantt_line(
        svg,
        0.0,
        title_h + SCALED_YEAR_ROW_H,
        chart_width,
        title_h + SCALED_YEAR_ROW_H,
        GRID_COLOR,
    );
    gantt_line(
        svg,
        0.0,
        title_h + MONTHLY_GRID_TOP,
        chart_width,
        title_h + MONTHLY_GRID_TOP,
        GRID_COLOR,
    );
}

fn render_monthly_bottom_axis(
    svg: &mut SvgBuilder,
    cal: &CalendarInfo,
    chart_width: f64,
    grid_bottom: f64,
) {
    render_month_name_axis(
        svg,
        cal,
        grid_bottom,
        grid_bottom + SCALED_MONTH_ROW_H,
        grid_bottom + SCALED_AXIS_MONTH_BASELINE,
    );
    render_year_axis(
        svg,
        cal,
        grid_bottom + SCALED_MONTH_ROW_H,
        grid_bottom + SCALED_MONTH_ROW_H + SCALED_YEAR_ROW_H,
        chart_width,
    );
    gantt_line(svg, 0.0, grid_bottom, chart_width, grid_bottom, GRID_COLOR);
    gantt_line(
        svg,
        0.0,
        grid_bottom + SCALED_MONTH_ROW_H,
        chart_width,
        grid_bottom + SCALED_MONTH_ROW_H,
        GRID_COLOR,
    );
    gantt_line(
        svg,
        0.0,
        grid_bottom + SCALED_MONTH_ROW_H + SCALED_YEAR_ROW_H,
        chart_width,
        grid_bottom + SCALED_MONTH_ROW_H + SCALED_YEAR_ROW_H,
        GRID_COLOR,
    );
}

fn render_week_numbers(svg: &mut SvgBuilder, total_days: u32, baseline: f64) {
    for week in 0..total_days.div_ceil(7) {
        let label = (week + 1).to_string();
        let tx = week as f64 * 7.0 * WEEKLY_DAY_WIDTH + 5.0;
        gantt_text(svg, tx, baseline, &label, AXIS_FONT, TEXT_COLOR);
    }
}

fn weekly_boundaries(total_days: u32) -> Vec<u32> {
    let mut out: Vec<u32> = (0..=total_days).step_by(7).collect();
    if out.last().copied() != Some(total_days) {
        out.push(total_days);
    }
    out
}

fn render_weekly_month_axis(svg: &mut SvgBuilder, cal: &CalendarInfo, y1: f64, y2: f64) {
    if let Some((start_idx, _, _)) = cal.month_spans.first() {
        let x = *start_idx as f64 * WEEKLY_DAY_WIDTH;
        gantt_line(svg, x, y1, x, y2, GRID_COLOR);
    }
    for &(start_idx, end_idx, ref label) in &cal.month_spans {
        let end_x = end_idx as f64 * WEEKLY_DAY_WIDTH;
        gantt_line(svg, end_x, y1, end_x, y2, GRID_COLOR);
        let span_w = (end_idx - start_idx) as f64 * WEEKLY_DAY_WIDTH;
        let mut display_label = abbreviate_month_label(label);
        if text_width(&display_label, MONTH_FONT, true) > span_w
            && let Some((month_only, _)) = display_label.rsplit_once(' ')
        {
            display_label = month_only.to_string();
        }
        if text_width(&display_label, MONTH_FONT, true) > span_w {
            display_label = abbreviate_month_name(&display_label).to_string();
        }
        let tl = text_width(&display_label, MONTH_FONT, true);
        let x = start_idx as f64 * WEEKLY_DAY_WIDTH + span_w / 2.0 - tl / 2.0;
        gantt_text_bold(svg, x, y1 + CAL_MONTH_Y, &display_label, MONTH_FONT);
    }
}

fn render_month_name_axis(svg: &mut SvgBuilder, cal: &CalendarInfo, y1: f64, y2: f64, text_y: f64) {
    if let Some((start_idx, _, _)) = cal.month_spans.first() {
        let x = *start_idx as f64 * MONTHLY_DAY_WIDTH;
        gantt_line(svg, x, y1, x, y2, GRID_COLOR);
    }
    for (idx, &(start_idx, end_idx, ref label)) in cal.month_spans.iter().enumerate() {
        let span_w = (end_idx - start_idx) as f64 * MONTHLY_DAY_WIDTH;
        let mut display_label = label
            .rsplit_once(' ')
            .map(|(month, _)| month.to_string())
            .unwrap_or_else(|| label.clone());
        if text_width(&display_label, AXIS_FONT, false) > span_w {
            display_label = abbreviate_month_name(&display_label).to_string();
        }
        let tl = text_width(&display_label, AXIS_FONT, false);
        let x = start_idx as f64 * MONTHLY_DAY_WIDTH + span_w / 2.0 - tl / 2.0;
        let end_x = end_idx as f64 * MONTHLY_DAY_WIDTH;
        if idx + 1 < cal.month_spans.len() {
            gantt_line(svg, end_x, y1, end_x, y2, GRID_COLOR);
        }
        gantt_text(svg, x, text_y, &display_label, AXIS_FONT, TEXT_COLOR);
        if idx + 1 == cal.month_spans.len() {
            gantt_line(svg, end_x, y1, end_x, y2, GRID_COLOR);
        }
    }
}

fn render_year_axis(svg: &mut SvgBuilder, cal: &CalendarInfo, y1: f64, y2: f64, chart_width: f64) {
    for (start_idx, end_idx, year) in year_spans(cal) {
        let x = start_idx as f64 * MONTHLY_DAY_WIDTH;
        gantt_line(svg, x, y1, x, y2, GRID_COLOR);
        let span_w = (end_idx - start_idx) as f64 * MONTHLY_DAY_WIDTH;
        let tl = text_width(&year, MONTH_FONT, true);
        let tx = x + span_w / 2.0 - tl / 2.0;
        gantt_text_bold(svg, tx, y1 + CAL_MONTH_Y, &year, MONTH_FONT);
    }
    gantt_line(svg, chart_width, y1, chart_width, y2, GRID_COLOR);
}

fn year_spans(cal: &CalendarInfo) -> Vec<(usize, usize, String)> {
    let mut out: Vec<(usize, usize, String)> = Vec::new();
    for &(start_idx, end_idx, ref label) in &cal.month_spans {
        let Some((_, year)) = label.rsplit_once(' ') else {
            continue;
        };
        if let Some((_, prev_end, prev_year)) = out.last_mut()
            && prev_year == year
        {
            *prev_end = end_idx;
            continue;
        }
        out.push((start_idx, end_idx, year.to_string()));
    }
    out
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

/// Calendar closures: repeating weekday closures plus specific holiday dates,
/// positioned relative to the project start date.
struct Closures {
    start_dow: u8,
    closed_days: Vec<u8>,
    /// Column offsets (days from project start) that are specific holidays.
    closed_date_cols: Vec<u32>,
}

impl Closures {
    fn new(project_start: &str, closed_days: &[u8], closed_dates: &[String]) -> Self {
        let start_dow = parse_start_dow(project_start).unwrap_or(0);
        let closed_date_cols = closed_dates
            .iter()
            .filter_map(|d| date_diff_days(project_start, d))
            .collect();
        Closures {
            start_dow,
            closed_days: closed_days.to_vec(),
            closed_date_cols,
        }
    }

    /// Whether any closures are defined at all.
    fn any(&self) -> bool {
        !self.closed_days.is_empty() || !self.closed_date_cols.is_empty()
    }

    /// Whether calendar column `col` (days from project start) is closed.
    fn is_col_closed(&self, col: u32) -> bool {
        let dow = ((self.start_dow as u32 + col) % 7) as u8;
        self.closed_days.contains(&dow) || self.closed_date_cols.contains(&col)
    }

    /// Map a working-day index (counting only open days) to its calendar
    /// column, skipping closed columns.
    fn wd_to_cal(&self, working_day: u32) -> u32 {
        if !self.any() {
            return working_day;
        }
        let mut open_count = 0u32;
        let mut cal = 0u32;
        loop {
            if !self.is_col_closed(cal) {
                if open_count == working_day {
                    return cal;
                }
                open_count += 1;
            }
            cal += 1;
            if cal > working_day * 7 + 366 {
                return cal;
            }
        }
    }
}

/// Number of calendar days between two YYYY-MM-DD dates, or `None` on failure.
fn date_diff_days(from: &str, to: &str) -> Option<u32> {
    fn to_jdn(y: i32, m: u32, d: u32) -> i64 {
        let a = (14 - m as i32) / 12;
        let yr = y + 4800 - a;
        let mo = m as i32 + 12 * a - 3;
        d as i64 + (153 * mo + 2) as i64 / 5 + 365 * yr as i64 + yr as i64 / 4 - yr as i64 / 100
            + yr as i64 / 400
            - 32045
    }
    fn parse(s: &str) -> Option<(i32, u32, u32)> {
        let p: Vec<&str> = s.split('-').collect();
        if p.len() != 3 {
            return None;
        }
        Some((p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?))
    }
    let (fy, fm, fd) = parse(from)?;
    let (ty, tm, td) = parse(to)?;
    let diff = to_jdn(ty, tm, td) - to_jdn(fy, fm, fd);
    if diff < 0 { None } else { Some(diff as u32) }
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
            closed_dates: Vec::new(),
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
            closed_dates: Vec::new(),
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
