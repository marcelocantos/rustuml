// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram SVG renderer.
//!
//! This is a faithful port of PlantUML's `net.sourceforge.plantuml.timingdiagram`
//! rendering pipeline. It reproduces the exact geometry (coordinates, tick
//! intervals, frame titles, state shapes) so that strict XML comparison against
//! the Java reference output matches element-for-element.

use rustuml_parser::diagram::timing::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics::{ascent, fmt_coord, text_height, text_width};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Constants ported from PlantUML ─────────────────────────────────────────
/// Outer content origin (TextBlockUtils.withMargin + image margin bake out to 20).
const ORIGIN: f64 = 20.0;
/// `TimingDiagram.marginX1` / `marginX2`.
const MARGIN_X1: f64 = 5.0;
const MARGIN_X2: f64 = 5.0;
/// `TimingRuler.tickIntervalInPixels`.
const TICK_INTERVAL_PX: f64 = 50.0;
/// `Panels.MARGIN_X` / `MARGIN_Y`.
const PANEL_MARGIN_X: f64 = 12.0;
const PANEL_MARGIN_Y: f64 = 8.0;
const BOTTOM_MARGIN: f64 = 10.0;
const LEFT_PANEL_MIN_WIDTH: f64 = 5.0;
/// `PanelsRobust` constants.
const HISTOGRAM_BOTTOM_MARGIN: f64 = 12.0;
const ROBUST_STEP_HEIGHT: f64 = 20.0;
/// Hexa / Penta horizontal delta.
const SHAPE_DELTA: f64 = 12.0;
/// Binary `suggestedHeight`.
const BINARY_HEIGHT: f64 = 30.0;
/// Concise ribbon default height (`PanelsState.DEFAULT_RIBBON_HEIGHT`).
const CONCISE_RIBBON_HEIGHT: f64 = 24.0;
/// Empirically-derived padding added by the image cropper.
const WIDTH_PAD: f64 = 20.373;
const HEIGHT_PAD: f64 = 16.877;

const FONT_TITLE: f64 = 14.0;
const FONT_STATE: f64 = 12.0;
const FONT_TIME: f64 = 11.0;
const LINE_COLOR: &str = "#333333";
const STATE_LINE_COLOR: &str = "#006400";
const CONCISE_FILL: &str = "#E2E2F0";

/// Render a timing diagram with an optional oracle layout.
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

/// A laid-out player with its computed vertical bands.
struct PlayerLayout<'a> {
    timeline: &'a Timeline,
    /// y of the frame top (relative to ORIGIN).
    frame_top: f64,
    /// y where the player body begins (relative to ORIGIN, after the frame title).
    body_top: f64,
    /// Ordered distinct states (for robust left labels / yOfState).
    all_states: Vec<String>,
}

/// Map a time value to its pixel offset from the first tick.
fn pos_in_pixel(t: i64, time_min: i64, tick_unit: i64) -> f64 {
    (t - time_min) as f64 / tick_unit as f64 * TICK_INTERVAL_PX
}

/// Highest common factor of the absolute non-zero tick values.
fn hcf(values: &[i64]) -> i64 {
    let mut acc: i64 = -1;
    for &v in values {
        let v = v.abs();
        if v == 0 {
            continue;
        }
        if acc == -1 {
            acc = v;
        } else {
            acc = gcd(acc, v);
        }
    }
    if acc <= 0 { 1 } else { acc }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.abs()
}

fn round_half_up(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// Render a timing diagram to SVG.
pub fn render(diagram: &TimingDiagram, _theme: &Theme) -> String {
    if diagram.timelines.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // ── Ruler ───────────────────────────────────────────────────────────────
    let mut times: Vec<i64> = diagram.time_points.clone();
    times.sort_unstable();
    times.dedup();
    if times.is_empty() {
        times.push(0);
    }
    let time_min = *times.first().unwrap();
    let time_max = *times.last().unwrap();

    let tick_unit = match diagram.scale {
        Some(scale) if scale.units > 0 => scale.units,
        _ => hcf(&times),
    };
    let delta = time_max - time_min;
    let ruler_width = (delta as f64 / tick_unit as f64 + 1.0) * TICK_INTERVAL_PX;
    let nb_tick = (1 + delta / tick_unit).min(1000) as usize;

    // ── Player layout (vertical) ──────────────────────────────────────────────
    let mut players: Vec<PlayerLayout> = Vec::new();
    let mut part1_max_width: f64 = 0.0;
    let mut y = 0.0_f64;
    for tl in &diagram.timelines {
        let frame_height = text_height(FONT_TITLE) + 1.0;
        let full_height = full_height_of(tl);

        let mut all_states: Vec<String> = Vec::new();
        for ch in &tl.changes {
            if !all_states.contains(&ch.state) {
                all_states.push(ch.state.clone());
            }
        }

        let left_panel_width = match tl.kind {
            TimelineKind::Robust => robust_states_width(&all_states),
            TimelineKind::Concise => 0.0,
            TimelineKind::Binary => LEFT_PANEL_MIN_WIDTH,
        };
        part1_max_width = part1_max_width.max(left_panel_width);

        players.push(PlayerLayout {
            timeline: tl,
            frame_top: y,
            body_top: y + frame_height,
            all_states,
        });
        y += frame_height + full_height;
    }
    let inner_height = y;

    // ── Coordinate helpers ────────────────────────────────────────────────────
    let first_tick_x = ORIGIN + part1_max_width + MARGIN_X1;
    let frame_left = ORIGIN;
    let frame_right = first_tick_x + ruler_width + MARGIN_X2;
    let axis_top = ORIGIN;
    let axis_bottom = ORIGIN + inner_height;
    let tx = |t: i64| first_tick_x + pos_in_pixel(t, time_min, tick_unit);

    // ── Dimensions ──────────────────────────────────────────────────────────────
    let total_width = round_half_up(frame_right + WIDTH_PAD);
    let max_y = axis_bottom + 5.0 + 1.0 + ascent(FONT_TIME);
    let total_height = round_half_up(max_y + HEIGHT_PAD);

    let mut svg = SvgBuilder::new_plantuml(total_width, total_height, "TIMING");

    // ── Frame border (two vertical lines) ──────────────────────────────────────
    emit_vline(&mut svg, frame_left, axis_top, axis_bottom, 0.5, false);
    emit_vline(&mut svg, frame_right, axis_top, axis_bottom, 0.5, false);

    // ── Ruler vertical grid lines ───────────────────────────────────────────────
    for i in 0..=nb_tick {
        let x = first_tick_x + TICK_INTERVAL_PX * i as f64;
        emit_vline(&mut svg, x, axis_top, axis_bottom, 0.5, true);
    }

    // ── Players ─────────────────────────────────────────────────────────────────
    for p in &players {
        // Horizontal separator at the top of each player frame.
        emit_hline(&mut svg, frame_left, frame_right, ORIGIN + p.frame_top, 0.5);
        draw_frame_title(&mut svg, p);
        match p.timeline.kind {
            TimelineKind::Robust => {
                draw_robust(&mut svg, p, part1_max_width, &tx, ruler_width, first_tick_x)
            }
            TimelineKind::Concise => draw_concise(&mut svg, p, &tx, ruler_width, first_tick_x),
            TimelineKind::Binary => draw_binary(&mut svg, p, &tx, ruler_width, first_tick_x),
        }
    }

    // ── Time axis ─────────────────────────────────────────────────────────────────
    draw_time_axis(
        &mut svg,
        &times,
        diagram,
        time_min,
        tick_unit,
        nb_tick,
        first_tick_x,
        axis_bottom,
        ruler_width,
    );

    svg.finalize_plantuml()
}

/// Full panel height for a player (excludes the frame title).
fn full_height_of(tl: &Timeline) -> f64 {
    match tl.kind {
        TimelineKind::Robust => {
            let mut all = Vec::new();
            for ch in &tl.changes {
                if !all.contains(&ch.state) {
                    all.push(ch.state.clone());
                }
            }
            let mut h = 10.0; // getHeightForConstraints = max(10, 0)
            if !all.is_empty() {
                h += ROBUST_STEP_HEIGHT * (all.len() as f64 - 1.0);
            }
            h + HISTOGRAM_BOTTOM_MARGIN + 6.0
        }
        TimelineKind::Concise => 5.0 + CONCISE_RIBBON_HEIGHT + BOTTOM_MARGIN,
        TimelineKind::Binary => BINARY_HEIGHT,
    }
}

fn robust_states_width(all_states: &[String]) -> f64 {
    let mut w = 0.0_f64;
    for s in all_states {
        w = w.max(text_width(s, FONT_STATE, false));
    }
    w
}

// ── Emission helpers (PlantUML `style="..."` form) ───────────────────────────

fn emit_vline(svg: &mut SvgBuilder, x: f64, y1: f64, y2: f64, width: f64, dashed: bool) {
    let dash = if dashed { "stroke-dasharray:3,5;" } else { "" };
    svg.raw(&format!(
        r#"<line style="stroke:{LINE_COLOR};stroke-width:{w};{dash}" x1="{x}" x2="{x}" y1="{y1}" y2="{y2}"/>"#,
        w = fmt_coord(width),
        x = fmt_coord(x),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    ));
}

fn emit_hline(svg: &mut SvgBuilder, x1: f64, x2: f64, y: f64, width: f64) {
    svg.raw(&format!(
        r#"<line style="stroke:{LINE_COLOR};stroke-width:{w};" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#,
        w = fmt_coord(width),
        x1 = fmt_coord(x1),
        x2 = fmt_coord(x2),
        y = fmt_coord(y),
    ));
}

/// Arbitrary line in a given stroke color.
fn emit_line(svg: &mut SvgBuilder, x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64) {
    svg.raw(&format!(
        r#"<line style="stroke:{color};stroke-width:{w};" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"/>"#,
        w = fmt_coord(width),
        x1 = fmt_coord(x1),
        x2 = fmt_coord(x2),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    ));
}

fn emit_text(svg: &mut SvgBuilder, x: f64, y: f64, content: &str, size: f64, bold: bool) {
    let tl = fmt_coord(text_width(content, size, bold));
    let weight = if bold { r#" font-weight="700""# } else { "" };
    let escaped = escape(content);
    svg.raw(&format!(
        r#"<text fill="{LINE_COLOR}" font-family="sans-serif" font-size="{sz}"{weight} lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{escaped}</text>"#,
        sz = fmt_coord(size),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ── Frame title ───────────────────────────────────────────────────────────────

fn draw_frame_title(svg: &mut SvgBuilder, p: &PlayerLayout) {
    let label = &p.timeline.label;
    if label.is_empty() {
        return;
    }
    let title_w = text_width(label, FONT_TITLE, true);
    let title_h = text_height(FONT_TITLE);
    let baseline = ORIGIN + p.frame_top + ascent(FONT_TITLE);
    emit_text(svg, ORIGIN + MARGIN_X1, baseline, label, FONT_TITLE, true);

    // L-underline: horizontal then diagonal, matching PlayerFrame.drawLine.
    let h = title_h + 1.0;
    let width_tmp = title_w + 1.0;
    let y_line = ORIGIN + p.frame_top + h;
    let y_top = ORIGIN + p.frame_top;
    let x0 = ORIGIN;
    let x1 = ORIGIN + MARGIN_X1 + width_tmp;
    let x2 = ORIGIN + MARGIN_X1 + width_tmp + 10.0;
    emit_line(svg, x0, y_line, x1, y_line, LINE_COLOR, 0.5);
    emit_line(svg, x1, y_line, x2, y_top, LINE_COLOR, 0.5);
}

// ── Robust ──────────────────────────────────────────────────────────────────

fn y_of_state(all_states: &[String], state: &str) -> f64 {
    let idx = all_states.iter().position(|s| s == state).unwrap_or(0);
    let nb = all_states.len().saturating_sub(1).saturating_sub(idx);
    ROBUST_STEP_HEIGHT * nb as f64
}

fn draw_robust(
    svg: &mut SvgBuilder,
    p: &PlayerLayout,
    part1_max_width: f64,
    tx: &dyn Fn(i64) -> f64,
    ruler_width: f64,
    first_tick_x: f64,
) {
    let changes = &p.timeline.changes;
    if changes.is_empty() {
        return;
    }
    let states_width = robust_states_width(&p.all_states);
    let label_x = if part1_max_width > states_width + 5.0 {
        ORIGIN + MARGIN_X1 + (part1_max_width - states_width - 5.0)
    } else {
        ORIGIN + MARGIN_X1 + (part1_max_width - states_width)
    };
    let constraints_h = 10.0;
    let band_top = ORIGIN + p.body_top + constraints_h;

    for state in &p.all_states {
        let yo = y_of_state(&p.all_states, state);
        let label_baseline =
            band_top + yo - text_height(FONT_STATE) / 2.0 + 1.0 + ascent(FONT_STATE);
        emit_text(svg, label_x, label_baseline, state, FONT_STATE, false);
    }

    for (i, ch) in changes.iter().enumerate() {
        let a = tx(ch.at);
        let b = if i + 1 < changes.len() {
            tx(changes[i + 1].at)
        } else {
            first_tick_x + ruler_width
        };
        let yo = band_top + y_of_state(&p.all_states, &ch.state);
        emit_line(svg, a, yo, b, yo, STATE_LINE_COLOR, 2.0);
    }
    for i in 1..changes.len() {
        let x = tx(changes[i].at);
        let y_prev = band_top + y_of_state(&p.all_states, &changes[i - 1].state);
        let y_cur = band_top + y_of_state(&p.all_states, &changes[i].state);
        let (y1, y2) = if y_prev <= y_cur {
            (y_prev, y_cur)
        } else {
            (y_cur, y_prev)
        };
        emit_line(svg, x, y1, x, y2, STATE_LINE_COLOR, 2.0);
    }
}

// ── Concise ────────────────────────────────────────────────────────────────

fn draw_concise(
    svg: &mut SvgBuilder,
    p: &PlayerLayout,
    tx: &dyn Fn(i64) -> f64,
    ruler_width: f64,
    first_tick_x: f64,
) {
    let changes = &p.timeline.changes;
    if changes.is_empty() {
        return;
    }
    let ribbon_top = ORIGIN + p.body_top + 5.0;
    let height = CONCISE_RIBBON_HEIGHT;

    for (i, ch) in changes.iter().enumerate() {
        let a = tx(ch.at);
        if i + 1 < changes.len() {
            let b = tx(changes[i + 1].at);
            draw_hexa(svg, a, ribbon_top, b - a, height);
        } else {
            let len = first_tick_x + ruler_width - a;
            draw_penta_b(svg, a, ribbon_top, len, height);
        }
    }

    let label_baseline =
        ribbon_top + height / 2.0 - text_height(FONT_STATE) / 2.0 + ascent(FONT_STATE);
    for (i, ch) in changes.iter().enumerate() {
        let x = tx(ch.at);
        let dim = text_width(&ch.state, FONT_STATE, true);
        let xtext = if i == changes.len() - 1 {
            x + PANEL_MARGIN_X
        } else {
            let x2 = tx(changes[i + 1].at);
            (x + x2) / 2.0 - dim / 2.0
        };
        emit_text(svg, xtext, label_baseline, &ch.state, FONT_STATE, true);
    }
}

/// Hexagon: points (delta,0)(w-delta,0)(w,h/2)(w-delta,h)(delta,h)(0,h/2).
fn draw_hexa(svg: &mut SvgBuilder, ox: f64, oy: f64, w: f64, h: f64) {
    let pts = [
        (SHAPE_DELTA, 0.0),
        (w - SHAPE_DELTA, 0.0),
        (w, h / 2.0),
        (w - SHAPE_DELTA, h),
        (SHAPE_DELTA, h),
        (0.0, h / 2.0),
    ];
    emit_polygon(svg, ox, oy, &pts, CONCISE_FILL, STATE_LINE_COLOR, 1.5);
}

/// PentaB: filled polygon (delta,0)(w,0)(w,h)(delta,h)(0,h/2) drawn with fill
/// color as stroke, then an open path with the line color.
fn draw_penta_b(svg: &mut SvgBuilder, ox: f64, oy: f64, w: f64, h: f64) {
    let poly = [
        (SHAPE_DELTA, 0.0),
        (w, 0.0),
        (w, h),
        (SHAPE_DELTA, h),
        (0.0, h / 2.0),
    ];
    emit_polygon(svg, ox, oy, &poly, CONCISE_FILL, CONCISE_FILL, 1.5);
    let path = [
        (w, 0.0),
        (SHAPE_DELTA, 0.0),
        (0.0, h / 2.0),
        (SHAPE_DELTA, h),
        (w, h),
    ];
    emit_path(svg, ox, oy, &path, CONCISE_FILL, STATE_LINE_COLOR, 1.5);
}

fn emit_polygon(
    svg: &mut SvgBuilder,
    ox: f64,
    oy: f64,
    pts: &[(f64, f64)],
    fill: &str,
    stroke: &str,
    width: f64,
) {
    let points: Vec<String> = pts
        .iter()
        .map(|(x, y)| format!("{},{}", fmt_coord(ox + x), fmt_coord(oy + y)))
        .collect();
    svg.raw(&format!(
        r#"<polygon fill="{fill}" points="{p}" style="stroke:{stroke};stroke-width:{w};"/>"#,
        p = points.join(","),
        w = fmt_coord(width),
    ));
}

fn emit_path(
    svg: &mut SvgBuilder,
    ox: f64,
    oy: f64,
    pts: &[(f64, f64)],
    fill: &str,
    stroke: &str,
    width: f64,
) {
    let mut d = String::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        let cmd = if i == 0 { "M" } else { " L" };
        d.push_str(&format!("{cmd}{},{}", fmt_coord(ox + x), fmt_coord(oy + y)));
    }
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:{w};"/>"#,
        w = fmt_coord(width),
    ));
}

// ── Binary ────────────────────────────────────────────────────────────────

fn is_high(state: &str) -> bool {
    let s = state.trim().to_ascii_lowercase();
    s == "1" || s == "high"
}

fn draw_binary(
    svg: &mut SvgBuilder,
    p: &PlayerLayout,
    tx: &dyn Fn(i64) -> f64,
    ruler_width: f64,
    first_tick_x: f64,
) {
    let changes = &p.timeline.changes;
    if changes.is_empty() {
        return;
    }
    let body = ORIGIN + p.body_top;
    let y_high = body + PANEL_MARGIN_Y;
    let y_low = body + BINARY_HEIGHT - PANEL_MARGIN_Y;

    // PanelsBinary.drawRightPanel: the trace starts at the LOW level (no initial
    // state in the supported model), at the left edge. For each change: draw a
    // flat segment at the previous level, then a vertical edge if the level
    // changed.
    let mut lastx = first_tick_x;
    let mut last_high = false;
    for ch in changes.iter() {
        let x = tx(ch.at);
        let cur_high = is_high(&ch.state);
        let yl = if last_high { y_high } else { y_low };
        emit_line(svg, lastx, yl, x, yl, STATE_LINE_COLOR, 2.0);
        if cur_high != last_high {
            emit_line(svg, x, y_high, x, y_low, STATE_LINE_COLOR, 2.0);
        }
        lastx = x;
        last_high = cur_high;
    }
    let yl = if last_high { y_high } else { y_low };
    emit_line(
        svg,
        lastx,
        yl,
        first_tick_x + ruler_width,
        yl,
        STATE_LINE_COLOR,
        2.0,
    );
}

// ── Time axis ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_time_axis(
    svg: &mut SvgBuilder,
    times: &[i64],
    diagram: &TimingDiagram,
    time_min: i64,
    tick_unit: i64,
    nb_tick: usize,
    first_tick_x: f64,
    axis_bottom: f64,
    ruler_width: f64,
) {
    let tick_height = 5.0;
    let x_end = first_tick_x + ruler_width;

    let mut i = 0usize;
    loop {
        let x = first_tick_x + TICK_INTERVAL_PX * i as f64;
        if x > x_end + 1e-6 {
            break;
        }
        emit_line(
            svg,
            x,
            axis_bottom,
            x,
            axis_bottom + tick_height,
            LINE_COLOR,
            2.0,
        );
        i += 1;
        if i > nb_tick + 2 {
            break;
        }
    }
    emit_hline_w(svg, first_tick_x, x_end, axis_bottom, 2.0);

    let label_baseline = axis_bottom + tick_height + 1.0 + ascent(FONT_TIME);
    let label_values: Vec<i64> = if diagram.scale.is_some() {
        (0..=nb_tick)
            .map(|k| time_min + tick_unit * k as i64)
            .collect()
    } else {
        times.to_vec()
    };
    for v in label_values {
        let s = v.to_string();
        let w = text_width(&s, FONT_TIME, false);
        let cx = first_tick_x + pos_in_pixel(v, time_min, tick_unit);
        emit_text(svg, cx - w / 2.0, label_baseline, &s, FONT_TIME, false);
    }
}

fn emit_hline_w(svg: &mut SvgBuilder, x1: f64, x2: f64, y: f64, width: f64) {
    emit_hline(svg, x1, x2, y, width);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn make_diagram() -> TimingDiagram {
        TimingDiagram {
            meta: DiagramMeta::default(),
            time_points: vec![0, 100, 300],
            timelines: vec![Timeline {
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
            }],
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
        assert!(svg.contains("data-diagram-type=\"TIMING\""));
    }

    #[test]
    fn contains_labels_and_states() {
        let svg = render(&make_diagram(), &Theme::default());
        assert!(svg.contains("Web"));
        assert!(svg.contains("Idle"));
        assert!(svg.contains("Processing"));
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
}
