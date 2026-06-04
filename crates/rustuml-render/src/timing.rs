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
use crate::plantuml_metrics::{ascent, descent, fmt_coord, text_height, text_width};
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
const FONT_DECORATION: f64 = 10.0;
const LINE_COLOR: &str = "#333333";
const STATE_LINE_COLOR: &str = "#006400";
const CONCISE_FILL: &str = "#E2E2F0";
const DECORATION_COLOR: &str = "#888888";
const TIMING_FOOTER_BOTTOM_GAP: f64 = 13.1348;

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
    /// Vertical space reserved above the band for time-constraint arrows
    /// (`PanelsRobust.getHeightForConstraints`). Defaults to the base 10 and
    /// grows for the timeline that owns the constraint annotations.
    constraints_h: f64,
}

/// Absolute y of a robust state's horizontal line, for a player whose band
/// begins at `band_top`.
fn robust_state_line_y(p: &PlayerLayout, band_top: f64, state: &str) -> f64 {
    band_top + y_of_state(&p.all_states, state)
}

/// `PanelsRobust.getHeightForConstraints`: the vertical space a robust band must
/// reserve above its baseline for the time-constraint arrows that target it.
///
/// Mirrors PlantUML: `max(10, max over constraints of
/// (getConstraintHeight - getConstraintDeltaY))`, where
/// `getConstraintHeight = text_height(state font) + topMargin(5)` and
/// `getConstraintDeltaY` is the lowest `yOfState` the constraint spans.
fn robust_constraints_height(p: &PlayerLayout, annotations: &[Annotation]) -> f64 {
    // getConstraintHeight = dimText.getHeight() + getTopMargin()  (topMargin = 5).
    let constraint_h = text_height(FONT_STATE) + 5.0;
    let mut result = 0.0_f64;
    for ann in annotations {
        // getConstraintDeltaY: yOfState at tick1, lowered by any change strictly
        // inside the span (yOfState grows downward, so `min` picks the topmost).
        let mut delta_y = state_at(p, ann.from)
            .map(|s| y_of_state(&p.all_states, s))
            .unwrap_or(0.0);
        for ch in &p.timeline.changes {
            if ann.from < ch.at && ch.at < ann.to {
                delta_y = delta_y.min(y_of_state(&p.all_states, &ch.state));
            }
        }
        result = result.max(constraint_h - delta_y);
    }
    result.max(10.0)
}

/// `PanelsState.getHeightForConstraints`: concise timelines reserve one
/// constraint label height plus the 5px top margin, with a 5px minimum.
fn state_constraints_height(annotations: &[Annotation]) -> f64 {
    if annotations.is_empty() {
        5.0
    } else {
        (text_height(FONT_STATE) + 5.0).max(5.0)
    }
}

/// The state active at time `t` on a robust timeline (the last change at or
/// before `t`, falling back to the first change).
fn state_at<'a>(p: &'a PlayerLayout, t: i64) -> Option<&'a str> {
    let changes = &p.timeline.changes;
    let mut found: Option<&str> = changes.first().map(|c| c.state.as_str());
    for ch in changes {
        if ch.at <= t {
            found = Some(&ch.state);
        }
    }
    found
}

/// Map a time value to its pixel offset from the first tick.
fn pos_in_pixel(t: i64, time_min: i64, tick_unit: i64, tick_px: f64) -> f64 {
    (t - time_min) as f64 / tick_unit as f64 * tick_px
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
    // Pixels per tick interval: `scale N as M pixels` overrides the default 50.
    let tick_px = match diagram.scale {
        Some(scale) if scale.pixels > 0 => scale.pixels as f64,
        _ => TICK_INTERVAL_PX,
    };
    let delta = time_max - time_min;
    let ruler_width = (delta as f64 / tick_unit as f64 + 1.0) * tick_px;
    let nb_tick = (1 + delta / tick_unit).min(1000) as usize;

    // ── Constraint owner ──────────────────────────────────────────────────────
    // Without an explicit player prefix, PlantUML attaches time constraints to
    // the last declared timeline.
    let anno_owner = if diagram.annotations.is_empty() {
        None
    } else {
        diagram.timelines.len().checked_sub(1)
    };

    // ── Player layout (vertical) ──────────────────────────────────────────────
    let mut players: Vec<PlayerLayout> = Vec::new();
    let mut part1_max_width: f64 = 0.0;
    let mut y = 0.0_f64;
    for (idx, tl) in diagram.timelines.iter().enumerate() {
        let frame_height = text_height(FONT_TITLE) + 1.0;

        let mut all_states: Vec<String> = Vec::new();
        for ch in &tl.changes {
            if !all_states.contains(&ch.state) {
                all_states.push(ch.state.clone());
            }
        }

        let left_panel_width = match tl.kind {
            TimelineKind::Robust => robust_states_width(&all_states),
            TimelineKind::Concise => 0.0,
            // Both binary and clock use `PanelsNoLeft` → `LEFT_PANEL_MIN_WIDTH`.
            TimelineKind::Binary | TimelineKind::Clock => LEFT_PANEL_MIN_WIDTH,
        };
        part1_max_width = part1_max_width.max(left_panel_width);

        let mut player = PlayerLayout {
            timeline: tl,
            frame_top: y,
            body_top: y + frame_height,
            all_states,
            constraints_h: match tl.kind {
                TimelineKind::Robust => 10.0,
                TimelineKind::Concise => 5.0,
                TimelineKind::Binary | TimelineKind::Clock => 0.0,
            },
        };
        if Some(idx) == anno_owner {
            player.constraints_h = match tl.kind {
                TimelineKind::Robust => robust_constraints_height(&player, &diagram.annotations),
                TimelineKind::Concise | TimelineKind::Binary | TimelineKind::Clock => {
                    state_constraints_height(&diagram.annotations)
                }
            };
        }
        let full_height = full_height_of(tl, player.constraints_h);
        y += frame_height + full_height;
        players.push(player);
    }
    let inner_height = y;

    // ── Page decorations (push content down / reserve footer band) ─────────────
    let header_band_h = if diagram.meta.header.is_some() {
        text_height(FONT_DECORATION) + 1.0
    } else {
        0.0
    };
    let footer_band_h = if diagram.meta.footer.is_some() {
        text_height(FONT_DECORATION)
    } else {
        0.0
    };

    // ── Title block (pushes all content down) ──────────────────────────────────
    // The `title` directive renders a centred bold band at the top; everything
    // below shifts down by its height.
    let title_offset = if diagram.meta.title.is_some() {
        ORIGIN + ascent(FONT_TITLE) + descent(FONT_TITLE) + 1.0
    } else {
        0.0
    };
    let vtop = ORIGIN + header_band_h + title_offset;

    // ── Coordinate helpers ────────────────────────────────────────────────────
    let first_tick_x = ORIGIN + part1_max_width + MARGIN_X1;
    let frame_left = ORIGIN;
    let frame_right = first_tick_x + ruler_width + MARGIN_X2;
    let axis_top = vtop;
    let axis_bottom = vtop + inner_height;
    let tx = |t: i64| first_tick_x + pos_in_pixel(t, time_min, tick_unit, tick_px);

    // ── Dimensions ──────────────────────────────────────────────────────────────
    let total_width = round_half_up(frame_right + WIDTH_PAD);
    let max_y = axis_bottom + 5.0 + 1.0 + ascent(FONT_TIME);
    let total_height = round_half_up(max_y + HEIGHT_PAD + footer_band_h);

    let mut svg = SvgBuilder::new_plantuml(total_width, total_height, "TIMING");

    // ── Header ─────────────────────────────────────────────────────────────────
    if let Some(header) = &diagram.meta.header {
        let tw = text_width(header, FONT_DECORATION, false);
        let x = frame_right + 9.0 - tw;
        let y = 10.0 + ascent(FONT_DECORATION);
        let sl = diagram.header_line.unwrap_or(1);
        svg.raw(&format!(r#"<g class="header" data-source-line="{sl}">"#));
        emit_text_colored(
            &mut svg,
            x,
            y,
            header,
            FONT_DECORATION,
            false,
            DECORATION_COLOR,
        );
        svg.raw("</g>");
    }

    // ── Title ───────────────────────────────────────────────────────────────────
    if let Some(title) = &diagram.meta.title {
        let tw = text_width(title, FONT_TITLE, true);
        let tx_title = (frame_right + 19.0 - tw) / 2.0;
        let baseline = ORIGIN + header_band_h + ascent(FONT_TITLE);
        let sl = diagram.title_line.unwrap_or(1);
        svg.raw(&format!(r#"<g class="title" data-source-line="{sl}">"#));
        emit_text_colored(
            &mut svg, tx_title, baseline, title, FONT_TITLE, true, "#000000",
        );
        svg.raw("</g>");
    }

    // ── Frame border (two vertical lines) ──────────────────────────────────────
    emit_vline(&mut svg, frame_left, axis_top, axis_bottom, 0.5, false);
    emit_vline(&mut svg, frame_right, axis_top, axis_bottom, 0.5, false);

    // ── Highlight backgrounds (drawn behind the grid) ──────────────────────────
    for hl in &diagram.highlights {
        let x1 = tx(hl.from);
        let x2 = tx(hl.to);
        let fill = highlight_fill(hl.color.as_deref());
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
            h = fmt_coord(axis_bottom - axis_top),
            w = fmt_coord(x2 - x1),
            x = fmt_coord(x1),
            y = fmt_coord(axis_top),
        ));
    }

    // ── Ruler vertical grid lines ───────────────────────────────────────────────
    for i in 0..=nb_tick {
        let x = first_tick_x + tick_px * i as f64;
        emit_vline(&mut svg, x, axis_top, axis_bottom, 0.5, true);
    }

    // ── Players ─────────────────────────────────────────────────────────────────
    for p in &players {
        // Horizontal separator at the top of each player frame.
        emit_hline(&mut svg, frame_left, frame_right, vtop + p.frame_top, 0.5);
        draw_frame_title(&mut svg, p, vtop);
        match p.timeline.kind {
            TimelineKind::Robust => draw_robust(
                &mut svg,
                p,
                part1_max_width,
                &tx,
                ruler_width,
                first_tick_x,
                vtop,
            ),
            TimelineKind::Concise => {
                draw_concise(&mut svg, p, &tx, ruler_width, first_tick_x, vtop)
            }
            TimelineKind::Binary => draw_binary(&mut svg, p, &tx, ruler_width, first_tick_x, vtop),
            TimelineKind::Clock => draw_clock(
                &mut svg,
                p,
                time_min,
                tick_unit,
                tick_px,
                ruler_width,
                first_tick_x,
                vtop,
            ),
        }
    }

    // ── Time-range annotations (`@T1 <-> @T2 : label`) ──────────────────────────
    // These attach to the last declared timeline. Robust constraints sit 5px
    // above the active state line; concise/binary/clock constraints sit halfway
    // through the reserved constraint band.
    if !diagram.annotations.is_empty()
        && let Some(owner_idx) = anno_owner
        && let Some(p) = players.get(owner_idx)
    {
        for ann in &diagram.annotations {
            let (y, margin_x) = match p.timeline.kind {
                TimelineKind::Robust => {
                    let band_top = vtop + p.body_top + p.constraints_h;
                    let state = state_at(p, ann.from);
                    let line_y = state
                        .map(|s| robust_state_line_y(p, band_top, s))
                        .unwrap_or(band_top);
                    (line_y - 5.0, 2.5)
                }
                TimelineKind::Concise => (vtop + p.body_top + p.constraints_h / 2.0, 1.0),
                TimelineKind::Binary | TimelineKind::Clock => {
                    (vtop + p.body_top + p.constraints_h / 2.0, 2.5)
                }
            };
            draw_annotation(&mut svg, tx(ann.from), tx(ann.to), y, &ann.label, margin_x);
        }
    }

    // ── Time axis ─────────────────────────────────────────────────────────────────
    draw_time_axis(
        &mut svg,
        &times,
        diagram,
        time_min,
        tick_unit,
        tick_px,
        nb_tick,
        first_tick_x,
        axis_bottom,
        ruler_width,
    );

    // ── Highlight boundary lines + labels (drawn on top) ───────────────────────
    for hl in &diagram.highlights {
        let x1 = tx(hl.from);
        let x2 = tx(hl.to);
        emit_dashed_bound(&mut svg, x1, axis_top, axis_bottom);
        emit_dashed_bound(&mut svg, x2, axis_top, axis_bottom);
        if let Some(label) = &hl.label {
            let baseline = vtop + ascent(FONT_STATE) + 2.0;
            emit_text_colored(
                &mut svg,
                x1 + 3.0,
                baseline,
                label,
                FONT_STATE,
                false,
                "#000000",
            );
        }
    }

    // ── Footer ─────────────────────────────────────────────────────────────────
    if let Some(footer) = &diagram.meta.footer {
        let tw = text_width(footer, FONT_DECORATION, false);
        let x = (frame_right + 19.0 - tw) / 2.0;
        let y = total_height - TIMING_FOOTER_BOTTOM_GAP;
        let sl = diagram.footer_line.unwrap_or(1);
        svg.raw(&format!(r#"<g class="footer" data-source-line="{sl}">"#));
        emit_text_colored(
            &mut svg,
            x,
            y,
            footer,
            FONT_DECORATION,
            false,
            DECORATION_COLOR,
        );
        svg.raw("</g>");
    }

    svg.finalize_plantuml()
}

/// Resolve a highlight fill colour. `None` → PlantUML's default `#EEEEEE`.
/// A `#name` token is a named colour; `#RRGGBB` is literal.
fn highlight_fill(color: Option<&str>) -> String {
    match color {
        None => "#EEEEEE".to_string(),
        Some(c) => {
            let stripped = c.strip_prefix('#').unwrap_or(c);
            if stripped.len() == 6 && stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
                format!("#{}", stripped.to_ascii_uppercase())
            } else {
                crate::text_render::normalize_color(stripped)
            }
        }
    }
}

/// Colour PlantUML uses for time-constraint annotations.
const ANNO_COLOR: &str = "#8B0000";

/// Draw a `@T1 <-> @T2 : label` time-range annotation: a double-headed arrow
/// between the two tick x-positions, with the label centred above.
fn draw_annotation(
    svg: &mut SvgBuilder,
    x_from: f64,
    x_to: f64,
    y: f64,
    label: &str,
    margin_x: f64,
) {
    emit_line(
        svg,
        x_from + margin_x + 3.0,
        y,
        x_to - margin_x - 3.0,
        y,
        ANNO_COLOR,
        1.5,
    );
    emit_arrowhead(svg, x_from + margin_x + 8.0, x_from + margin_x, y);
    emit_arrowhead(svg, x_to - margin_x - 8.0, x_to - margin_x, y);
    // Label centred on the span, sitting above the arrow.
    let tw = text_width(label, FONT_STATE, false);
    let cx = (x_from + x_to) / 2.0 - tw / 2.0;
    let baseline = y - 5.0 - descent(FONT_STATE);
    emit_text_colored(svg, cx, baseline, label, FONT_STATE, false, ANNO_COLOR);
}

/// A triangular arrowhead pointing from `base_x` toward `tip_x` at height `y`.
fn emit_arrowhead(svg: &mut SvgBuilder, base_x: f64, tip_x: f64, y: f64) {
    svg.raw(&format!(
        r#"<polygon fill="{ANNO_COLOR}" points="{bx},{yb},{bx},{yt},{tx},{y}" style="stroke:{ANNO_COLOR};stroke-width:1;"/>"#,
        bx = fmt_coord(base_x),
        yb = fmt_coord(y + 4.0),
        yt = fmt_coord(y - 4.0),
        tx = fmt_coord(tip_x),
        y = fmt_coord(y),
    ));
}

/// Dashed highlight boundary line (`stroke-width:2;stroke-dasharray:4,4`).
fn emit_dashed_bound(svg: &mut SvgBuilder, x: f64, y1: f64, y2: f64) {
    svg.raw(&format!(
        r#"<line style="stroke:{LINE_COLOR};stroke-width:2;stroke-dasharray:4,4;" x1="{x}" x2="{x}" y1="{y1}" y2="{y2}"/>"#,
        x = fmt_coord(x),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    ));
}

/// Text with an explicit fill colour.
fn emit_text_colored(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    content: &str,
    size: f64,
    bold: bool,
    fill: &str,
) {
    let tl = fmt_coord(text_width(content, size, bold));
    let weight = if bold { r#" font-weight="700""# } else { "" };
    let escaped = escape(content);
    svg.raw(&format!(
        r#"<text fill="{fill}" font-family="sans-serif" font-size="{sz}"{weight} lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{escaped}</text>"#,
        sz = fmt_coord(size),
        x = fmt_coord(x),
        y = fmt_coord(y),
    ));
}

/// Full panel height for a player (excludes the frame title). `constraints_h` is
/// `PanelsRobust.getHeightForConstraints` for this player (base 10, larger when
/// it owns constraint annotations).
fn full_height_of(tl: &Timeline, constraints_h: f64) -> f64 {
    match tl.kind {
        TimelineKind::Robust => {
            let mut all = Vec::new();
            for ch in &tl.changes {
                if !all.contains(&ch.state) {
                    all.push(ch.state.clone());
                }
            }
            let mut h = constraints_h; // getHeightForConstraints
            if !all.is_empty() {
                h += ROBUST_STEP_HEIGHT * (all.len() as f64 - 1.0);
            }
            h + HISTOGRAM_BOTTOM_MARGIN + 6.0
        }
        TimelineKind::Concise => constraints_h + CONCISE_RIBBON_HEIGHT + BOTTOM_MARGIN,
        // Clock shares the binary `suggestedHeight` (PlayerClock(..., 30)).
        TimelineKind::Binary | TimelineKind::Clock => constraints_h + BINARY_HEIGHT,
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

fn draw_frame_title(svg: &mut SvgBuilder, p: &PlayerLayout, vtop: f64) {
    let label = &p.timeline.label;
    if label.is_empty() {
        return;
    }
    let title_w = text_width(label, FONT_TITLE, true);
    let title_h = text_height(FONT_TITLE);
    let baseline = vtop + p.frame_top + ascent(FONT_TITLE);
    emit_text(svg, ORIGIN + MARGIN_X1, baseline, label, FONT_TITLE, true);

    // L-underline: horizontal then diagonal, matching PlayerFrame.drawLine.
    let h = title_h + 1.0;
    let width_tmp = title_w + 1.0;
    let y_line = vtop + p.frame_top + h;
    let y_top = vtop + p.frame_top;
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

#[allow(clippy::too_many_arguments)]
fn draw_robust(
    svg: &mut SvgBuilder,
    p: &PlayerLayout,
    part1_max_width: f64,
    tx: &dyn Fn(i64) -> f64,
    ruler_width: f64,
    first_tick_x: f64,
    vtop: f64,
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
    let band_top = vtop + p.body_top + p.constraints_h;

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
    vtop: f64,
) {
    let changes = &p.timeline.changes;
    if changes.is_empty() {
        return;
    }
    let ribbon_top = vtop + p.body_top + p.constraints_h;
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
    vtop: f64,
) {
    let changes = &p.timeline.changes;
    if changes.is_empty() {
        return;
    }
    let body = vtop + p.body_top + p.constraints_h;
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

// ── Clock ────────────────────────────────────────────────────────────────

/// Stroke width PlantUML uses for clock waveforms (`PanelsClock`); thinner than
/// the binary trace (2.0).
const CLOCK_LINE_WIDTH: f64 = 1.5;

/// Port of `PanelsClock.drawRightPanel`: an auto-generated square wave with a
/// rising edge every `pulse` units and a falling edge `period - pulse` later,
/// starting at the panel's left edge (after an optional initial `offset`),
/// repeating until the next transition would fall past the ruler width.
#[allow(clippy::too_many_arguments)]
fn draw_clock(
    svg: &mut SvgBuilder,
    p: &PlayerLayout,
    time_min: i64,
    tick_unit: i64,
    tick_px: f64,
    ruler_width: f64,
    first_tick_x: f64,
    vtop: f64,
) {
    let Some(spec) = p.timeline.clock else {
        return;
    };
    let period = spec.period as f64;
    if period <= 0.0 {
        return;
    }
    let body = vtop + p.body_top + p.constraints_h;
    let y_high = body + PANEL_MARGIN_Y;
    let line_height = BINARY_HEIGHT - 2.0 * PANEL_MARGIN_Y; // PanelsClock getLineHeight
    let y_low = y_high + line_height;

    // `xOfTime` = TimingRuler.getPosInPixel: a local pixel offset from the first
    // tick (the ruler min is subtracted, then scaled by tick unit / interval).
    let x_of_time = |t: f64| (t - time_min as f64) / tick_unit as f64 * tick_px;
    // Absolute x for a clock-local time, clamped to the ruler width on the
    // right (`drawHorizontalBetweenTimes` uses `min(ruler.getWidth(), …)`).
    let abs_x = |t: f64| first_tick_x + x_of_time(t);
    let h_line = |svg: &mut SvgBuilder, y: f64, start: f64, end: f64| {
        let x1 = x_of_time(start);
        let x2 = ruler_width.min(x_of_time(end));
        emit_line(
            svg,
            first_tick_x + x1,
            y,
            first_tick_x + x2,
            y,
            STATE_LINE_COLOR,
            CLOCK_LINE_WIDTH,
        );
    };
    let v_line = |svg: &mut SvgBuilder, t: f64| {
        let x = abs_x(t);
        emit_line(svg, x, y_high, x, y_low, STATE_LINE_COLOR, CLOCK_LINE_WIDTH);
    };

    let mut value = 0.0_f64;
    if spec.offset != 0 {
        let off = spec.offset as f64;
        h_line(svg, y_low, value, off);
        value += off;
    }
    if x_of_time(value) > ruler_width {
        return;
    }
    v_line(svg, value);

    let vpulse = if spec.pulse == 0 {
        period / 2.0
    } else {
        spec.pulse as f64
    };
    let remain = period - vpulse;
    for _ in 0..1000 {
        h_line(svg, y_high, value, value + vpulse);
        value += vpulse;
        if x_of_time(value) > ruler_width {
            return;
        }
        v_line(svg, value);
        h_line(svg, y_low, value, value + remain);
        value += remain;
        if x_of_time(value) > ruler_width {
            return;
        }
        v_line(svg, value);
    }
}

// ── Time axis ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_time_axis(
    svg: &mut SvgBuilder,
    times: &[i64],
    diagram: &TimingDiagram,
    time_min: i64,
    tick_unit: i64,
    tick_px: f64,
    nb_tick: usize,
    first_tick_x: f64,
    axis_bottom: f64,
    ruler_width: f64,
) {
    let tick_height = 5.0;
    let x_end = first_tick_x + ruler_width;

    let mut i = 0usize;
    loop {
        let x = first_tick_x + tick_px * i as f64;
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
        let cx = first_tick_x + pos_in_pixel(v, time_min, tick_unit, tick_px);
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
                clock: None,
            }],
            highlights: vec![],
            annotations: vec![],
            scale: None,
            notes: vec![],
            title_line: None,
            header_line: None,
            footer_line: None,
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
            title_line: None,
            header_line: None,
            footer_line: None,
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
    }
}
