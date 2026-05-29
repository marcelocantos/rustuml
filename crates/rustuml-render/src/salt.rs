// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Salt (UI wireframe) diagram renderer.
//!
//! This is a geometry-faithful port of PlantUML's salt layout engine
//! (`net.sourceforge.plantuml.salt`).  Widgets are laid out on a grid
//! (`ElementPyramid`): every cell contributes `dim.width + 2` to its
//! column span and `dim.height + 2` to its row span.  Each element is
//! drawn at `(colsStart[col] + 1, rowsStart[row] + 1)`, and the whole
//! drawing is offset by a 5px margin on every side.
//!
//! Font metrics come from [`crate::plantuml_metrics`], which reproduce
//! Java AWT's `SansSerif` advances exactly, so `textLength` values match
//! the golden SVGs.

use rustuml_parser::diagram::salt::{SaltBlock, SaltDiagram, SaltWidget, SeparatorKind};

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;

// ── Metrics constants (from the Java element classes) ────────────────────────

const FONT_SIZE: f64 = 12.0;
const MARGIN: f64 = 5.0; // PSystemSalt default margin (all sides)
const CELL_PAD: f64 = 2.0; // ElementPyramid: dim + 2 per cell span
const DRAW_OFFSET: f64 = 1.0; // ElementPyramid draws at colsStart+1 / rowsStart+1

// ElementRadioCheckbox
const RC_MARGIN: f64 = 20.0; // text offset
const RC_RECT: f64 = 10.0;
const RC_ELLIPSE2: f64 = 4.0;

// AbstractElementText: getSingleSpace() returns a fixed 8px.
const CHAR_SPACE: f64 = 8.0;

// ElementButton
const BTN_STROKE: f64 = 2.5;
const BTN_MARGIN: f64 = 2.0;

// ── Public entry point ───────────────────────────────────────────────────────

/// Render a Salt diagram with an optional oracle layout.
pub fn render_with_oracle(
    diagram: &SaltDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "SALT");
    }
    render(diagram, theme)
}

/// Render a [`SaltDiagram`] to an SVG string.
pub fn render(diagram: &SaltDiagram, _theme: &Theme) -> String {
    let grid = Grid::layout(&diagram.root);
    let total_w = grid.width() + MARGIN * 2.0;
    let total_h = grid.height() + MARGIN * 2.0;

    // PlantUML rounds the SVG canvas up to whole pixels.
    let w = total_w.ceil() as i64;
    let h = total_h.ceil() as i64;

    let mut body = String::new();
    grid.draw(MARGIN, MARGIN, &mut body);

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="SALT" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify"><?plantuml 1.2026.3beta6?><defs/><g>{body}</g></svg>"#
    )
}

// ── Grid layout ──────────────────────────────────────────────────────────────

/// A laid-out cell: the widget plus its grid position.
struct PlacedCell<'a> {
    widget: &'a SaltWidget,
    row: usize,
    col: usize,
}

struct Grid<'a> {
    cells: Vec<PlacedCell<'a>>,
    cols_start: Vec<f64>,
    rows_start: Vec<f64>,
}

impl<'a> Grid<'a> {
    fn layout(block: &'a SaltBlock) -> Grid<'a> {
        // Assign row/col positions (Positionner2).
        let mut cells: Vec<PlacedCell<'a>> = Vec::new();
        let mut n_cols = 1usize;
        for (r, row) in block.rows.iter().enumerate() {
            for (c, widget) in row.cells.iter().enumerate() {
                cells.push(PlacedCell {
                    widget,
                    row: r,
                    col: c,
                });
                n_cols = n_cols.max(c + 1);
            }
        }
        let n_rows = block.rows.len().max(1);

        // Column widths (LeftFirst): ensure col span >= dim.width + 2.
        let mut cols_start = vec![0f64; n_cols + 1];
        for cell in &cells {
            let (w, _) = widget_dim(cell.widget);
            ensure_span(&mut cols_start, cell.col, cell.col + 1, w + CELL_PAD);
        }

        // Row heights (TopFirst): ensure row span >= dim.height + 2.
        let mut rows_start = vec![0f64; n_rows + 1];
        for cell in &cells {
            let (_, h) = widget_dim(cell.widget);
            ensure_span(&mut rows_start, cell.row, cell.row + 1, h + CELL_PAD);
        }

        Grid {
            cells,
            cols_start,
            rows_start,
        }
    }

    fn width(&self) -> f64 {
        *self.cols_start.last().unwrap_or(&0.0)
    }

    fn height(&self) -> f64 {
        *self.rows_start.last().unwrap_or(&0.0)
    }

    fn draw(&self, ox: f64, oy: f64, buf: &mut String) {
        for cell in &self.cells {
            let cx = ox + self.cols_start[cell.col] + DRAW_OFFSET;
            let cy = oy + self.rows_start[cell.row] + DRAW_OFFSET;
            // dimToUse for the cell (span minus 1, per ElementPyramid).
            let cell_w = self.cols_start[cell.col + 1] - self.cols_start[cell.col] - 1.0;
            let cell_h = self.rows_start[cell.row + 1] - self.rows_start[cell.row] - 1.0;
            draw_widget(cell.widget, cx, cy, cell_w, cell_h, buf);
        }
    }
}

/// Distribute the missing width/height so that `start[last] - start[first] >= size`.
fn ensure_span(start: &mut [f64], first: usize, last: usize, size: f64) {
    let actual = start[last] - start[first];
    let missing = size - actual;
    if missing > 0.0 {
        for s in start.iter_mut().skip(last) {
            *s += missing;
        }
    }
}

// ── Widget measurement ───────────────────────────────────────────────────────

/// Preferred `(width, height)` of a widget, matching the Java
/// `getPreferredDimension`.
fn widget_dim(widget: &SaltWidget) -> (f64, f64) {
    let th = pm::text_height(FONT_SIZE);
    match widget {
        SaltWidget::Label(t) => {
            let s = TextStyle::parse(t);
            (pm::text_width(&s.display, FONT_SIZE, s.bold), th)
        }
        SaltWidget::Checkbox { label, .. } | SaltWidget::Radio { label, .. } => {
            let s = TextStyle::parse(label);
            (
                pm::text_width(&s.display, FONT_SIZE, s.bold) + RC_MARGIN,
                th,
            )
        }
        SaltWidget::Separator(_) => (RC_RECT, 6.0), // ElementLine: (10, 6)
        SaltWidget::Button(t) => {
            // managed length: max(textWidth, charLen * 8), then + 2*marginX + 2*stroke.
            let mw = managed_text_width(t);
            (
                mw + 2.0 * BTN_MARGIN + 2.0 * BTN_STROKE,
                th + 2.0 * BTN_MARGIN + 2.0 * BTN_STROKE,
            )
        }
        SaltWidget::TextField(t) => {
            // managed length, then delta(6, 2).
            let mw = managed_text_width(t);
            (mw + 6.0, th + 2.0)
        }
        // Best-effort for widgets not yet fully reproduced.
        SaltWidget::Dropdown(t) => {
            let tw = pm::text_width(t, FONT_SIZE, false);
            (tw + 20.0, th)
        }
        SaltWidget::TreeNode { depth, label } => (
            (*depth as f64) * 8.0 + pm::text_width(label, FONT_SIZE, false),
            th,
        ),
        SaltWidget::Block(b) => {
            let g = Grid::layout(b);
            (g.width(), g.height())
        }
    }
}

// ── Widget drawing ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_widget(widget: &SaltWidget, x: f64, y: f64, cell_w: f64, cell_h: f64, buf: &mut String) {
    let ascent = pm::ascent(FONT_SIZE);
    let pref_h = pm::text_height(FONT_SIZE);

    match widget {
        SaltWidget::Label(t) => {
            emit_text(buf, x, y + ascent, t);
        }

        SaltWidget::Checkbox { checked, label } => {
            // Text at translate(margin).
            emit_text(buf, x + RC_MARGIN, y + ascent, label);
            // Box at translate(2, (prefH - 10) / 2), stroke 1.5.
            let bx = x + 2.0;
            let by = y + (pref_h - RC_RECT) / 2.0;
            emit_rect_15(buf, bx, by, RC_RECT, RC_RECT, "none");
            if *checked {
                // Polygon at translate(2,...) box origin? No: from element
                // origin translate(3,6): points (0,0)(3,3)(10,-6)(3,1).
                let px = x + 3.0;
                let py = y + 6.0;
                emit_check_poly(buf, px, py);
            }
        }

        SaltWidget::Radio { selected, label } => {
            emit_text(buf, x + RC_MARGIN, y + ascent, label);
            // Ellipse at translate(2, (prefH-10)/2), 10x10 → cx = x+2+5.
            let ecx = x + 2.0 + RC_RECT / 2.0;
            let ecy = y + (pref_h - RC_RECT) / 2.0 + RC_RECT / 2.0;
            emit_ellipse_15(buf, ecx, ecy, RC_RECT / 2.0, "none");
            if *selected {
                // Inner ellipse 4x4 at translate(2 + (10-4)/2, (prefH-4)/2).
                let icx = x + 2.0 + (RC_RECT - RC_ELLIPSE2) / 2.0 + RC_ELLIPSE2 / 2.0;
                let icy = y + (pref_h - RC_ELLIPSE2) / 2.0 + RC_ELLIPSE2 / 2.0;
                emit_ellipse_15(buf, icx, icy, RC_ELLIPSE2 / 2.0, "#000000");
            }
        }

        SaltWidget::Separator(kind) => {
            // ElementLine: line at y = cell_h/2, spanning the cell width.
            let y2 = cell_h / 2.0;
            emit_separator(buf, x, y + y2, cell_w, *kind);
        }

        SaltWidget::Button(label) => {
            // Preferred dimension (uses managed width).
            let (pw, ph) = widget_dim(widget);
            // Rounded rect at translate(stroke, stroke), inset by 2*stroke.
            emit_rounded_rect_stroke(
                buf,
                x + BTN_STROKE,
                y + BTN_STROKE,
                pw - 2.0 * BTN_STROKE,
                ph - 2.0 * BTN_STROKE,
                "#EEEEEE",
                BTN_STROKE,
            );
            // Text centred: drawText at ((pw - pureTextWidth)/2, stroke + marginY).
            let pure_w = pm::text_width(&strip_creole(label), FONT_SIZE, false);
            let tx = x + (pw - pure_w) / 2.0;
            let ty = y + BTN_STROKE + BTN_MARGIN + ascent;
            emit_text(buf, tx, ty, label);
        }
        SaltWidget::TextField(t) => {
            // drawText at (3, 0).
            emit_text(buf, x + 3.0, y + ascent, t);
            let (pw, _) = widget_dim(widget);
            let text_h = pref_h; // getTextDimensionAt height
            let managed_w = managed_text_width(t);
            // hline at translate(1, text_h), width = preferred.width - 3.
            emit_hline_black(buf, x + 1.0, y + text_h, pw - 3.0);
            // Two vertical ticks at translate(1, y3) and (3 + managedW + 1, y3),
            // each 2px tall, where y3 = text_h - 3.
            let y3 = text_h - 3.0;
            emit_vline_black(buf, x + 1.0, y + y3, 2.0);
            emit_vline_black(buf, x + 3.0 + managed_w + 1.0, y + y3, 2.0);
        }
        // ── Best-effort fallbacks (geometry not yet exact) ──
        SaltWidget::Dropdown(label) => {
            emit_text(buf, x, y + ascent, label);
        }
        SaltWidget::TreeNode { depth, label } => {
            let indent = (*depth as f64) * 8.0;
            emit_text(buf, x + indent, y + ascent, label);
        }
        SaltWidget::Block(b) => {
            let g = Grid::layout(b);
            g.draw(x, y, buf);
        }
    }
}

// ── SVG emit helpers (PlantUML-style attributes) ─────────────────────────────

fn emit_text(buf: &mut String, x: f64, y: f64, content: &str) {
    let style = TextStyle::parse(content);
    let tl = pm::text_width(&style.display, FONT_SIZE, style.bold);
    let escaped = escape_text(&style.display);
    // PlantUML emits attributes alphabetically; the comparator sorts them
    // anyway, so we just include the relevant style attributes.
    let weight = if style.bold {
        r#" font-weight="700""#
    } else {
        ""
    };
    let italic = if style.italic {
        r#" font-style="italic""#
    } else {
        ""
    };
    let underline = if style.underline {
        r#" text-decoration="underline""#
    } else {
        ""
    };
    buf.push_str(&format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="12"{italic}{weight} lengthAdjust="spacing"{underline} textLength="{tl}" x="{x}" y="{y}">{escaped}</text>"##,
        tl = pm::fmt_coord(tl),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

/// Text content with simple creole `<b>`/`<i>`/`<u>` styling stripped out.
struct TextStyle {
    display: String,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl TextStyle {
    fn parse(content: &str) -> TextStyle {
        let bold = content.contains("<b>");
        let italic = content.contains("<i>");
        let underline = content.contains("<u>");
        TextStyle {
            display: strip_creole(content),
            bold,
            italic,
            underline,
        }
    }
}

fn emit_rect_15(buf: &mut String, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    buf.push_str(&format!(
        r##"<rect fill="{fill}" height="{h}" style="stroke:#000000;stroke-width:1.5;" width="{w}" x="{x}" y="{y}"/>"##,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

fn emit_rounded_rect_stroke(
    buf: &mut String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke_w: f64,
) {
    buf.push_str(&format!(
        r##"<rect fill="{fill}" height="{h}" rx="5" ry="5" style="stroke:#000000;stroke-width:{sw};" width="{w}" x="{x}" y="{y}"/>"##,
        h = pm::fmt_coord(h),
        sw = pm::fmt_coord(stroke_w),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

fn emit_ellipse_15(buf: &mut String, cx: f64, cy: f64, r: f64, fill: &str) {
    buf.push_str(&format!(
        r##"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{r}" ry="{r}" style="stroke:#000000;stroke-width:1.5;"/>"##,
        cx = pm::fmt_coord(cx),
        cy = pm::fmt_coord(cy),
        r = pm::fmt_coord(r),
    ));
}

fn emit_check_poly(buf: &mut String, px: f64, py: f64) {
    // Polygon points (0,0)(3,3)(10,-6)(3,1) translated by (px,py).
    let pts = [(0.0, 0.0), (3.0, 3.0), (10.0, -6.0), (3.0, 1.0)];
    let s: Vec<String> = pts
        .iter()
        .map(|(dx, dy)| format!("{},{}", pm::fmt_coord(px + dx), pm::fmt_coord(py + dy)))
        .collect();
    buf.push_str(&format!(
        r##"<polygon fill="#000000" points="{}" style="stroke:#000000;stroke-width:1.5;"/>"##,
        s.join(",")
    ));
}

fn emit_separator(buf: &mut String, x: f64, y: f64, width: f64, kind: SeparatorKind) {
    let x2 = x + width;
    match kind {
        SeparatorKind::Dots => {
            buf.push_str(&format!(
                r##"<line style="stroke:#AAAAAA;stroke-width:1;stroke-dasharray:1,2;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
                x1 = pm::fmt_coord(x),
                x2 = pm::fmt_coord(x2),
                y = pm::fmt_coord(y),
            ));
        }
        SeparatorKind::Double => {
            for dy in [-1.0, 1.0] {
                buf.push_str(&format!(
                    r##"<line style="stroke:#AAAAAA;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{ya}" y2="{ya}"/>"##,
                    x1 = pm::fmt_coord(x),
                    x2 = pm::fmt_coord(x2),
                    ya = pm::fmt_coord(y + dy),
                ));
            }
        }
        SeparatorKind::Single => {
            buf.push_str(&format!(
                r##"<line style="stroke:#AAAAAA;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
                x1 = pm::fmt_coord(x),
                x2 = pm::fmt_coord(x2),
                y = pm::fmt_coord(y),
            ));
        }
        SeparatorKind::Solid => {
            buf.push_str(&format!(
                r##"<line style="stroke:#AAAAAA;stroke-width:1.5;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
                x1 = pm::fmt_coord(x),
                x2 = pm::fmt_coord(x2),
                y = pm::fmt_coord(y),
            ));
        }
    }
}

fn emit_hline_black(buf: &mut String, x: f64, y: f64, width: f64) {
    buf.push_str(&format!(
        r##"<line style="stroke:#000000;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
        x1 = pm::fmt_coord(x),
        x2 = pm::fmt_coord(x + width),
        y = pm::fmt_coord(y),
    ));
}

fn emit_vline_black(buf: &mut String, x: f64, y: f64, height: f64) {
    buf.push_str(&format!(
        r##"<line style="stroke:#000000;stroke-width:1;" x1="{x}" x2="{x}" y1="{y1}" y2="{y2}"/>"##,
        x = pm::fmt_coord(x),
        y1 = pm::fmt_coord(y),
        y2 = pm::fmt_coord(y + height),
    ));
}

/// Managed text width for buttons/textfields: `max(textWidth, charLen * 8)`.
fn managed_text_width(t: &str) -> f64 {
    let display = strip_creole(t);
    let tw = pm::text_width(&display, FONT_SIZE, false);
    let char_len = display.chars().count() as f64;
    tw.max(char_len * CHAR_SPACE)
}

/// Strip simple creole markup tags for measurement/display fallback.
fn strip_creole(s: &str) -> String {
    s.replace("<b>", "")
        .replace("</b>", "")
        .replace("<i>", "")
        .replace("</i>", "")
        .replace("<u>", "")
        .replace("</u>", "")
}

fn escape_text(s: &str) -> String {
    if s.is_empty() {
        return "&#160;".to_string();
    }
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
