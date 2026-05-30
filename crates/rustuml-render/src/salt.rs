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

use rustuml_parser::diagram::salt::{BlockKind, SaltBlock, SaltDiagram, SaltWidget, SeparatorKind};

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

// ElementDroplist: the arrow box is a fixed 12px wide region on the right.
const DROP_BOX: f64 = 12.0;

// ElementPyramidScrolled: scrollbar thickness (v1) and arrow-track inset (v2).
const SCROLL_BAR: f64 = 15.0; // v1
const SCROLL_INSET: f64 = 12.0; // v2
const SCROLL_GAP: f64 = 4.0; // gap between content and scrollbar
const SCROLL_PAD: f64 = 30.0; // delta added to preferred dim per scrolled axis

/// Which scrollbars a `{S` family block draws.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScrollStrategy {
    Both,
    VerticalOnly,
    HorizontalOnly,
}

impl ScrollStrategy {
    fn from_kind(kind: BlockKind) -> Option<ScrollStrategy> {
        match kind {
            BlockKind::Scroll => Some(ScrollStrategy::Both),
            BlockKind::ScrollInput => Some(ScrollStrategy::VerticalOnly),
            BlockKind::ScrollHorizontal => Some(ScrollStrategy::HorizontalOnly),
            _ => None,
        }
    }

    fn has_vertical(self) -> bool {
        matches!(self, ScrollStrategy::Both | ScrollStrategy::VerticalOnly)
    }

    fn has_horizontal(self) -> bool {
        matches!(self, ScrollStrategy::Both | ScrollStrategy::HorizontalOnly)
    }

    /// Extra `(dx, dy)` added to the base pyramid's preferred dimension.
    fn delta(self) -> (f64, f64) {
        match self {
            ScrollStrategy::Both => (SCROLL_PAD, SCROLL_PAD),
            ScrollStrategy::VerticalOnly => (SCROLL_PAD, 0.0),
            ScrollStrategy::HorizontalOnly => (0.0, SCROLL_PAD),
        }
    }
}

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
    kind: BlockKind,
    n_rows: usize,
    n_cols: usize,
    /// Title for a `{^Title` group box (DRAW_OUTSIDE_WITH_TITLE), plus its
    /// rendered height. `None` for all other blocks.
    title: Option<String>,
    title_height: f64,
    /// Scrollbar decoration for `{S`/`{SI`/`{S-` blocks.
    scroll: Option<ScrollStrategy>,
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

        // A `{^Title` group box reserves half the title height above the
        // grid (rowsStart[i] starts at titleHeight/2), and row-0 cells gain a
        // further titleHeight/2 so the title sits inside the top border.
        let title = block.title.clone();
        let title_height = if title.is_some() {
            pm::text_height(FONT_SIZE)
        } else {
            0.0
        };
        let half_title = title_height / 2.0;

        // Column widths (LeftFirst): ensure col span >= dim.width + 2.
        let mut cols_start = vec![0f64; n_cols + 1];
        for cell in &cells {
            let (w, _) = widget_dim(cell.widget);
            ensure_span(&mut cols_start, cell.col, cell.col + 1, w + CELL_PAD);
        }

        // Row heights (TopFirst): ensure row span >= dim.height + supY + 2,
        // where supY = titleHeight/2 for row-0 cells (matching ElementPyramid).
        let mut rows_start = vec![half_title; n_rows + 1];
        for cell in &cells {
            let (_, h) = widget_dim(cell.widget);
            let sup_y = if cell.row == 0 { half_title } else { 0.0 };
            ensure_span(
                &mut rows_start,
                cell.row,
                cell.row + 1,
                h + sup_y + CELL_PAD,
            );
        }

        Grid {
            cells,
            cols_start,
            rows_start,
            kind: block.kind,
            n_rows,
            n_cols,
            title,
            title_height,
            scroll: ScrollStrategy::from_kind(block.kind),
        }
    }

    /// Base pyramid width (excluding any scrollbar allocation).
    fn base_width(&self) -> f64 {
        *self.cols_start.last().unwrap_or(&0.0)
    }

    /// Base pyramid height (excluding any scrollbar allocation).
    fn base_height(&self) -> f64 {
        *self.rows_start.last().unwrap_or(&0.0) + self.title_height
    }

    fn width(&self) -> f64 {
        let (dx, _) = self.scroll.map_or((0.0, 0.0), ScrollStrategy::delta);
        self.base_width() + dx
    }

    fn height(&self) -> f64 {
        let (_, dy) = self.scroll.map_or((0.0, 0.0), ScrollStrategy::delta);
        self.base_height() + dy
    }

    fn draw(&self, ox: f64, oy: f64, buf: &mut String) {
        let half_title = self.title_height / 2.0;
        for cell in &self.cells {
            // Row-0 cells are pushed down by titleHeight/2 (supY).
            let sup_y = if cell.row == 0 { half_title } else { 0.0 };
            let cx = ox + self.cols_start[cell.col] + DRAW_OFFSET;
            let cy = oy + self.rows_start[cell.row] + sup_y + DRAW_OFFSET;
            // dimToUse for the cell (span minus 1, per ElementPyramid).
            let cell_w = self.cols_start[cell.col + 1] - self.cols_start[cell.col] - 1.0;
            let cell_h = self.rows_start[cell.row + 1] - self.rows_start[cell.row] - 1.0;
            draw_widget(cell.widget, cx, cy, cell_w, cell_h, buf);
        }
        if self.kind == BlockKind::Table {
            self.draw_grid_lines(ox, oy, buf);
        }
        if self.kind == BlockKind::Frame || self.scroll.is_some() {
            // A scrolled pyramid is a DRAW_OUTSIDE block (outer border only).
            self.draw_outside_border(ox, oy, buf);
        }
        if let Some(strategy) = self.scroll {
            // The scrollbar decoration is emitted once per z-index pass (z=0
            // and z=1); ElementText only draws on z=0, so the border and cell
            // text appear once but the scrollbars appear twice.
            self.draw_scrollbars(ox, oy, strategy, buf);
            self.draw_scrollbars(ox, oy, strategy, buf);
        }
        if let Some(title) = &self.title {
            self.draw_group_box(ox, oy, title, buf);
        }
    }

    /// Draw the vertical/horizontal scrollbars for a `{S` family block,
    /// mirroring `ElementPyramidScrolled.drawU`. `dim` is the base pyramid
    /// size; bars are offset by `SCROLL_GAP` beyond the content edge.
    fn draw_scrollbars(&self, ox: f64, oy: f64, strategy: ScrollStrategy, buf: &mut String) {
        let dim_w = self.base_width();
        let dim_h = self.base_height();
        if strategy.has_vertical() {
            // drawV at translate dx(dim.width + 4): rect (v1 × dim.height).
            let vx = ox + dim_w + SCROLL_GAP;
            self.draw_scroll_v(vx, oy, dim_h, buf);
        }
        if strategy.has_horizontal() {
            // drawH at translate dy(dim.height + 4): rect (dim.width × v1).
            let hy = oy + dim_h + SCROLL_GAP;
            self.draw_scroll_h(ox, hy, dim_w, buf);
        }
    }

    /// Vertical scrollbar: a `SCROLL_BAR`-wide rectangle of `height`, with two
    /// arrow-track separators and up/down triangles (`getTr0`/`getTr180`).
    fn draw_scroll_v(&self, x: f64, y: f64, height: f64, buf: &mut String) {
        emit_rect_1(buf, x, y, SCROLL_BAR, height);
        emit_hline_black(buf, x, y + SCROLL_INSET, SCROLL_BAR);
        emit_hline_black(buf, x, y + height - SCROLL_INSET, SCROLL_BAR);
        // getTr0 at translate(4,4): (3,0)(6,5)(0,5).
        emit_filled_poly(
            buf,
            x + SCROLL_GAP,
            y + SCROLL_GAP,
            &[(3.0, 0.0), (6.0, 5.0), (0.0, 5.0)],
        );
        // getTr180 at translate(4, height - v2 + 4): (3,5)(6,0)(0,0).
        emit_filled_poly(
            buf,
            x + SCROLL_GAP,
            y + height - SCROLL_INSET + SCROLL_GAP,
            &[(3.0, 5.0), (6.0, 0.0), (0.0, 0.0)],
        );
    }

    /// Horizontal scrollbar: a `width`-wide rectangle of height `SCROLL_BAR`,
    /// with two arrow-track separators and left/right triangles
    /// (`getTr90`/`getTr270`).
    fn draw_scroll_h(&self, x: f64, y: f64, width: f64, buf: &mut String) {
        emit_rect_1(buf, x, y, width, SCROLL_BAR);
        emit_vline_black(buf, x + SCROLL_INSET, y, SCROLL_BAR);
        emit_vline_black(buf, x + width - SCROLL_INSET, y, SCROLL_BAR);
        // getTr90 at translate(4,4): (0,3)(5,6)(5,0).
        emit_filled_poly(
            buf,
            x + SCROLL_GAP,
            y + SCROLL_GAP,
            &[(0.0, 3.0), (5.0, 6.0), (5.0, 0.0)],
        );
        // getTr270 at translate(width - v2 + 4, 4): (5,3)(0,6)(0,0).
        emit_filled_poly(
            buf,
            x + width - SCROLL_INSET + SCROLL_GAP,
            y + SCROLL_GAP,
            &[(5.0, 3.0), (0.0, 6.0), (0.0, 0.0)],
        );
    }

    /// Draw the table's outer perimeter (DRAW_OUTSIDE) using the Java `Grid`
    /// segment-set ordering: top and bottom horizontals per column, then left
    /// and right verticals per row.
    fn draw_outside_border(&self, ox: f64, oy: f64, buf: &mut String) {
        let mut horizontals = JavaSegmentSet::new();
        let mut verticals = JavaSegmentSet::new();
        for c in 0..self.n_cols {
            horizontals.insert((0, c));
            horizontals.insert((self.n_rows, c));
        }
        for r in 0..self.n_rows {
            verticals.insert((r, 0));
            verticals.insert((r, self.n_cols));
        }
        for &(row, col) in &horizontals.iter_order() {
            let x1 = ox + self.cols_start[col];
            let x2 = ox + self.cols_start[col + 1];
            let y = oy + self.rows_start[row];
            buf.push_str(&format!(
                r##"<line style="stroke:#000000;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
                x1 = pm::fmt_coord(x1),
                x2 = pm::fmt_coord(x2),
                y = pm::fmt_coord(y),
            ));
        }
        for &(row, col) in &verticals.iter_order() {
            let x = ox + self.cols_start[col];
            let y1 = oy + self.rows_start[row];
            let y2 = oy + self.rows_start[row + 1];
            buf.push_str(&format!(
                r##"<line style="stroke:#000000;stroke-width:1;" x1="{x}" x2="{x}" y1="{y1}" y2="{y2}"/>"##,
                x = pm::fmt_coord(x),
                y1 = pm::fmt_coord(y1),
                y2 = pm::fmt_coord(y2),
            ));
        }
    }

    /// Draw the outer border and title for a `{^Title` group box
    /// (DRAW_OUTSIDE_WITH_TITLE): the border traces the grid perimeter, and
    /// the title text sits on the top edge over a white backing rectangle.
    fn draw_group_box(&self, ox: f64, oy: f64, title: &str, buf: &mut String) {
        self.draw_outside_border(ox, oy, buf);

        // Title: a white-backed text at (x + 6, y), on the top border.
        let tw = pm::text_width(title, FONT_SIZE, false);
        let th = self.title_height;
        let tx = ox + 6.0;
        let ty = oy;
        buf.push_str(&format!(
            r##"<rect fill="#FFFFFF" height="{h}" style="stroke:#FFFFFF;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"##,
            h = pm::fmt_coord(th),
            w = pm::fmt_coord(tw),
            x = pm::fmt_coord(tx),
            y = pm::fmt_coord(ty),
        ));
        emit_text(buf, tx, ty + pm::ascent(FONT_SIZE), title);
    }

    /// Emit the table grid lines for a `{#` block, reproducing the Java
    /// `Grid` segment-set algorithm (DRAW_ALL strategy).
    ///
    /// Segments are keyed `(row, col)` and deduplicated.  A horizontal
    /// segment draws a line at `y = rowsStart[row]` spanning one column;
    /// a vertical segment draws at `x = colsStart[col]` spanning one row.
    fn draw_grid_lines(&self, ox: f64, oy: f64, buf: &mut String) {
        let mut horizontals = JavaSegmentSet::new();
        let mut verticals = JavaSegmentSet::new();

        // addOutside(): outer border of the whole table.
        for c in 0..self.n_cols {
            horizontals.insert((0, c));
            horizontals.insert((self.n_rows, c));
        }
        for r in 0..self.n_rows {
            verticals.insert((r, 0));
            verticals.insert((r, self.n_cols));
        }

        // addCell() for every cell, in row-major (insertion) order, matching
        // the Java `positions1.entrySet()` iteration over the LinkedHashMap.
        for cell in &self.cells {
            let (r, c) = (cell.row, cell.col);
            horizontals.insert((r, c));
            horizontals.insert((r + 1, c));
            verticals.insert((r, c));
            verticals.insert((r, c + 1));
        }

        for &(row, col) in &horizontals.iter_order() {
            let x1 = ox + self.cols_start[col];
            let x2 = ox + self.cols_start[col + 1];
            let y = oy + self.rows_start[row];
            buf.push_str(&format!(
                r##"<line style="stroke:#000000;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
                x1 = pm::fmt_coord(x1),
                x2 = pm::fmt_coord(x2),
                y = pm::fmt_coord(y),
            ));
        }
        for &(row, col) in &verticals.iter_order() {
            let x = ox + self.cols_start[col];
            let y1 = oy + self.rows_start[row];
            let y2 = oy + self.rows_start[row + 1];
            buf.push_str(&format!(
                r##"<line style="stroke:#000000;stroke-width:1;" x1="{x}" x2="{x}" y1="{y1}" y2="{y2}"/>"##,
                x = pm::fmt_coord(x),
                y1 = pm::fmt_coord(y1),
                y2 = pm::fmt_coord(y2),
            ));
        }
    }
}

/// A faithful emulation of `java.util.HashSet<Segment>` iteration order.
///
/// PlantUML's `Grid` collects grid-line segments in `HashSet`s and emits them
/// in the set's iteration order. Because the strict comparator matches SVG
/// elements positionally, we must reproduce that order exactly. The order is
/// governed by `Segment.hashCode() == row*47 + col`, the HashMap spread
/// function `h ^ (h >>> 16)`, an initial capacity of 16 with load factor 0.75,
/// and doubling resizes that preserve within-bucket insertion order.
struct JavaSegmentSet {
    /// Insertion-ordered unique keys.
    keys: Vec<(usize, usize)>,
}

const JAVA_INITIAL_CAPACITY: usize = 16;
const JAVA_LOAD_FACTOR_NUM: usize = 3; // 0.75 = 3/4
const JAVA_LOAD_FACTOR_DEN: usize = 4;

impl JavaSegmentSet {
    fn new() -> JavaSegmentSet {
        JavaSegmentSet { keys: Vec::new() }
    }

    fn insert(&mut self, key: (usize, usize)) {
        if !self.keys.contains(&key) {
            self.keys.push(key);
        }
    }

    /// `Segment.hashCode()` spread through HashMap's hash function.
    fn spread(key: (usize, usize)) -> u32 {
        let h = (key.0 as u32).wrapping_mul(47).wrapping_add(key.1 as u32);
        h ^ (h >> 16)
    }

    /// Capacity grows to the smallest power of two whose load threshold
    /// (capacity * 0.75) is not exceeded by the element count, matching the
    /// resize sequence of `java.util.HashMap`.
    fn capacity_for(n: usize) -> usize {
        let mut cap = JAVA_INITIAL_CAPACITY;
        while n > cap * JAVA_LOAD_FACTOR_NUM / JAVA_LOAD_FACTOR_DEN {
            cap <<= 1;
        }
        cap
    }

    /// Iterate keys in Java HashSet order: ascending bucket index, and within
    /// each bucket in insertion order (HashMap appends to the bucket chain,
    /// and resize splitting preserves relative order).
    fn iter_order(&self) -> Vec<(usize, usize)> {
        let cap = Self::capacity_for(self.keys.len());
        let mask = (cap - 1) as u32;
        let mut buckets: Vec<Vec<(usize, usize)>> = vec![Vec::new(); cap];
        for &k in &self.keys {
            let b = (Self::spread(k) & mask) as usize;
            buckets[b].push(k);
        }
        buckets.into_iter().flatten().collect()
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
        // ElementDroplist: getTextDimensionAt then delta(4 + box, 4),
        // where box = 12 and getTextDimensionAt = max(textWidth, charLen*8).
        SaltWidget::Dropdown(t) => {
            let display = strip_creole(t);
            let tw = pm::text_width(display.trim(), FONT_SIZE, false);
            let char_len = display.trim().chars().count() as f64;
            let managed = tw.max(char_len * CHAR_SPACE);
            (managed + 4.0 + DROP_BOX, th + 4.0)
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
            let display = strip_creole(label);
            let display = display.trim();
            let pure_w = pm::text_width(display, FONT_SIZE, false);
            let tx = x + (pw - pure_w) / 2.0;
            let ty = y + BTN_STROKE + BTN_MARGIN + ascent;
            emit_text(buf, tx, ty, display);
        }
        SaltWidget::TextField(t) => {
            // drawText at (3, 0); text is trimmed for display.
            let display = strip_creole(t);
            emit_text(buf, x + 3.0, y + ascent, display.trim());
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
        SaltWidget::Dropdown(label) => {
            // ElementDroplist: EE-filled rect, text at (2,2), a vertical
            // divider `box` px from the right edge, and a down-triangle.
            let (pw, ph) = widget_dim(widget);
            emit_droplist_rect(buf, x, y, pw - 1.0, ph - 1.0);
            let display = strip_creole(label);
            emit_text(buf, x + 2.0, y + 2.0 + ascent, display.trim());
            let xline = x + pw - DROP_BOX;
            emit_vline_black(buf, xline, y, ph - 1.0);
            // Down-triangle: points (0,0),(box-6,0),((box-6)/2, textH-8),
            // translated by (xline + 3, 6).
            let tx = xline + 3.0;
            let ty = y + 6.0;
            let tip_y = pref_h - 8.0;
            emit_droplist_arrow(buf, tx, ty, DROP_BOX - 6.0, tip_y);
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
    // PlantUML renders an empty string as a single non-breaking space whose
    // textLength is the width of one space.
    let tl = if style.display.is_empty() {
        pm::text_width(" ", FONT_SIZE, false)
    } else {
        pm::text_width(&style.display, FONT_SIZE, style.bold)
    };
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

/// A `fill:none; stroke-width:1` rectangle (scrollbar track).
fn emit_rect_1(buf: &mut String, x: f64, y: f64, w: f64, h: f64) {
    buf.push_str(&format!(
        r##"<rect fill="none" height="{h}" style="stroke:#000000;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"##,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

/// A filled `<path>` (the scrollbar arrows are `UPath`s, not `<polygon>`s):
/// `moveTo(p0) lineTo(p1..) lineTo(p0)`, no stroke. Points are translated by
/// `(ox, oy)`.
fn emit_filled_poly(buf: &mut String, ox: f64, oy: f64, pts: &[(f64, f64)]) {
    let mut d = String::new();
    for (i, (dx, dy)) in pts.iter().enumerate() {
        let cmd = if i == 0 { 'M' } else { 'L' };
        if i > 0 {
            d.push(' ');
        }
        d.push_str(&format!(
            "{cmd}{},{}",
            pm::fmt_coord(ox + dx),
            pm::fmt_coord(oy + dy)
        ));
    }
    // Close back to the first point with an explicit lineTo (matches UPath).
    let (fx, fy) = pts[0];
    d.push_str(&format!(
        " L{},{}",
        pm::fmt_coord(ox + fx),
        pm::fmt_coord(oy + fy)
    ));
    buf.push_str(&format!(r##"<path d="{d}" fill="#000000"/>"##));
}

fn emit_droplist_rect(buf: &mut String, x: f64, y: f64, w: f64, h: f64) {
    buf.push_str(&format!(
        r##"<rect fill="#EEEEEE" height="{h}" style="stroke:#000000;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"##,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

/// Down-pointing triangle for a droplist: points `(0,0)`, `(base,0)`,
/// `(base/2, tip_y)`, translated to `(ox, oy)`.
fn emit_droplist_arrow(buf: &mut String, ox: f64, oy: f64, base: f64, tip_y: f64) {
    let pts = [(0.0, 0.0), (base, 0.0), (base / 2.0, tip_y)];
    let s: Vec<String> = pts
        .iter()
        .map(|(dx, dy)| format!("{},{}", pm::fmt_coord(ox + dx), pm::fmt_coord(oy + dy)))
        .collect();
    buf.push_str(&format!(
        r##"<polygon fill="#000000" points="{}" style="stroke:#000000;stroke-width:1;"/>"##,
        s.join(",")
    ));
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
///
/// PlantUML trims the text for display (so `textWidth` uses the trimmed
/// string) but counts the raw character length (including trailing spaces)
/// for the `charLen * 8` term.
fn managed_text_width(t: &str) -> f64 {
    let stripped = strip_creole(t);
    let tw = pm::text_width(stripped.trim(), FONT_SIZE, false);
    let char_len = stripped.chars().count() as f64;
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
