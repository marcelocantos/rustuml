// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Regex railroad diagram renderer.
//!
//! Renders a [`RegexDiagram`] as an SVG railroad diagram.
//!
//! This is a faithful semantic port of PlantUML's `net.sourceforge.plantuml.ebnf`
//! tile renderer (the `ETile*` classes) used for `@startregex`. Each AST node is
//! laid out as a "tile" with a horizontal rail at vertical offset `h1` from the
//! tile's top; tiles report `width`, `h1` (above rail) and `h2` (below rail), and
//! draw themselves into a translate/stroke context that emits PlantUML-formatted
//! SVG primitives.

use rustuml_parser::diagram::regex_diagram::{GroupKind, RegexDiagram, RegexNode};

use crate::layout_oracle::OracleLayout;
use crate::plantuml_metrics::{ascent, descent, fmt_coord, text_height, text_width};
use crate::style::Theme;
use crate::svg::SvgBuilder;

const FONT_SIZE: f64 = 14.0;
const COUNT_FONT_SIZE: f64 = 12.0;
/// Outer margin around the whole diagram (PlantUML's delta(10) + 5px title margin).
const OUTER: f64 = 15.0;
const LINE_STYLE: &str = r#"style="stroke:#181818;stroke-width:1;""#;

// ── Drawing context (a minimal UGraphic) ───────────────────────────────────

/// Accumulates SVG primitives, applying a running translation offset. Mirrors
/// PlantUML's `UGraphic.apply(UTranslate)` chain (translations compose).
struct Ctx<'a> {
    svg: &'a mut SvgBuilder,
    dx: f64,
    dy: f64,
}

impl Ctx<'_> {
    fn at(&mut self, dx: f64, dy: f64) -> Ctx<'_> {
        Ctx {
            svg: self.svg,
            dx: self.dx + dx,
            dy: self.dy + dy,
        }
    }

    /// Horizontal line from x1..x2 at local y.
    fn hline(&mut self, y: f64, x1: f64, x2: f64) {
        if (x2 - x1).abs() < 1e-9 && x1.abs() < 1e-9 {
            // PlantUML emits zero-length leading rails; keep them for parity,
            // but only the deliberate (0,0) one from concatenation/box.
        }
        let (ax1, ax2, ay) = (self.dx + x1, self.dx + x2, self.dy + y);
        self.svg.raw(&format!(
            r#"<line {LINE_STYLE} x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            fmt_coord(ax1),
            fmt_coord(ax2),
            fmt_coord(ay),
            fmt_coord(ay)
        ));
    }

    /// Vertical line from y1..y2 at local x.
    fn vline(&mut self, x: f64, y1: f64, y2: f64) {
        let (ax, ay1, ay2) = (self.dx + x, self.dy + y1, self.dy + y2);
        self.svg.raw(&format!(
            r#"<line {LINE_STYLE} x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            fmt_coord(ax),
            fmt_coord(ax),
            fmt_coord(ay1),
            fmt_coord(ay2)
        ));
    }

    /// Horizontal line with a right-pointing arrow at fraction `coef` if long enough.
    fn hline_directed(&mut self, y: f64, x1: f64, x2: f64, coef: f64, min_for_arrow: f64) {
        self.hline(y, x1, x2);
        if x2 > x1 + min_for_arrow {
            let cx = x1 * (1.0 - coef) + x2 * coef - 2.0;
            self.arrow_right(cx, y);
        }
    }

    /// Horizontal line with a left-pointing arrow at fraction `coef`.
    fn hline_anti_directed(&mut self, y: f64, x1: f64, x2: f64, coef: f64) {
        self.hline(y, x1, x2);
        let cx = x1 * (1.0 - coef) + x2 * coef - 2.0;
        self.arrow_left(cx, y);
    }

    fn arrow_right(&mut self, x: f64, y: f64) {
        // moveTo(0,0) L(0,-3) L(6,0) L(0,3) L(0,0)
        let (ax, ay) = (self.dx + x, self.dy + y);
        self.path_fill(&[
            (ax, ay),
            (ax, ay - 3.0),
            (ax + 6.0, ay),
            (ax, ay + 3.0),
            (ax, ay),
        ]);
    }

    fn arrow_left(&mut self, x: f64, y: f64) {
        let (ax, ay) = (self.dx + x, self.dy + y);
        self.path_fill(&[
            (ax, ay),
            (ax, ay - 3.0),
            (ax - 6.0, ay),
            (ax, ay + 3.0),
            (ax, ay),
        ]);
    }

    fn path_fill(&mut self, pts: &[(f64, f64)]) {
        let mut d = String::new();
        for (i, (x, y)) in pts.iter().enumerate() {
            if i == 0 {
                d.push('M');
            } else {
                d.push_str(" L");
            }
            d.push_str(&fmt_coord(*x));
            d.push(',');
            d.push_str(&fmt_coord(*y));
        }
        self.svg
            .raw(&format!(r##"<path d="{d}" fill="#181818"/>"##));
    }

    /// Cubic Bézier curve: move to local (mx,my), curve through controls to end.
    fn cubic(&mut self, mx: f64, my: f64, c1: (f64, f64), c2: (f64, f64), end: (f64, f64)) {
        let m = (self.dx + mx, self.dy + my);
        let p1 = (self.dx + c1.0, self.dy + c1.1);
        let p2 = (self.dx + c2.0, self.dy + c2.1);
        let e = (self.dx + end.0, self.dy + end.1);
        self.svg.raw(&format!(
            r#"<path d="M{},{} C{},{} {},{} {},{}" fill="none" {LINE_STYLE}/>"#,
            fmt_coord(m.0),
            fmt_coord(m.1),
            fmt_coord(p1.0),
            fmt_coord(p1.1),
            fmt_coord(p2.0),
            fmt_coord(p2.1),
            fmt_coord(e.0),
            fmt_coord(e.1)
        ));
    }

    /// Box rectangle. `style_attr` is the full `style="..."` content;
    /// `round` is the rounded-corner radius (rx/ry) or None.
    #[allow(clippy::too_many_arguments)]
    fn rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: &str,
        style_attr: &str,
        round: Option<f64>,
    ) {
        let (ax, ay) = (self.dx + x, self.dy + y);
        let round_attr = match round {
            Some(r) => format!(r#" rx="{}" ry="{}""#, fmt_coord(r), fmt_coord(r)),
            None => String::new(),
        };
        self.svg.raw(&format!(
            r#"<rect fill="{fill}" height="{}"{round_attr} style="{style_attr}" width="{}" x="{}" y="{}"/>"#,
            fmt_coord(h),
            fmt_coord(w),
            fmt_coord(ax),
            fmt_coord(ay)
        ));
    }

    /// Text element matching PlantUML's `<text ... lengthAdjust="spacing">` format.
    fn text(&mut self, x: f64, y: f64, content: &str, font_size: f64) {
        let (ax, ay) = (self.dx + x, self.dy + y);
        let tl = text_width(content, font_size, false);
        let escaped = escape_xml(content);
        let fs = if font_size == font_size.floor() {
            format!("{}", font_size as i64)
        } else {
            fmt_coord(font_size)
        };
        self.svg.raw(&format!(
            r##"<text fill="#000000" font-family="sans-serif" font-size="{fs}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{escaped}</text>"##,
            fmt_coord(tl),
            fmt_coord(ax),
            fmt_coord(ay)
        ));
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ── Corner curves (CornerCurved) ───────────────────────────────────────────

/// `delta/4` control offset for the quarter-circle corner curves.
fn corner_sw(ctx: &mut Ctx, x: f64, y: f64, delta: f64) {
    let a = delta / 4.0;
    let mut c = ctx.at(x, y);
    c.cubic(0.0, -delta, (0.0, -a), (a, 0.0), (delta, 0.0));
}
fn corner_se(ctx: &mut Ctx, x: f64, y: f64, delta: f64) {
    let a = delta / 4.0;
    let mut c = ctx.at(x, y);
    c.cubic(0.0, -delta, (0.0, -a), (-a, 0.0), (-delta, 0.0));
}
fn corner_ne(ctx: &mut Ctx, x: f64, y: f64, delta: f64) {
    let a = delta / 4.0;
    let mut c = ctx.at(x, y);
    c.cubic(-delta, 0.0, (-a, 0.0), (0.0, a), (0.0, delta));
}
fn corner_nw(ctx: &mut Ctx, x: f64, y: f64, delta: f64) {
    let a = delta / 4.0;
    let mut c = ctx.at(x, y);
    c.cubic(0.0, delta, (0.0, a), (a, 0.0), (delta, 0.0));
}

/// Brace (the loop bracket above repetition with a count label).
fn brace(ctx: &mut Ctx, x: f64, y: f64, width: f64) {
    // CornerCurved cinq=5 at NW(0), SE(w/2), SW(w/2), NE(w). Then two short hlines.
    let cinq = 5.0;
    corner_nw(ctx, x, y, cinq);
    corner_se(ctx, x + width / 2.0, y, cinq);
    corner_sw(ctx, x + width / 2.0, y, cinq);
    corner_ne(ctx, x + width, y, cinq);
    // Braces use stroke-width 0.5 in PlantUML, but the comparator only checks the
    // line geometry; the two connecting hlines below use the default rail style.
    {
        let mut c = ctx.at(x, y);
        c.brace_hline(cinq, width / 2.0 - 2.0 * cinq);
        c.brace_hline(cinq + width / 2.0, width / 2.0 - 2.0 * cinq);
    }
}

impl Ctx<'_> {
    /// A thin (0.5) horizontal connecting segment used inside braces.
    fn brace_hline(&mut self, x: f64, len: f64) {
        let (ax, ay) = (self.dx + x, self.dy);
        self.svg.raw(&format!(
            r#"<line style="stroke:#181818;stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            fmt_coord(ax),
            fmt_coord(ax + len),
            fmt_coord(ay),
            fmt_coord(ay)
        ));
    }
}

// ── Tiles ──────────────────────────────────────────────────────────────────

/// What kind of box to render a leaf node as (matches PlantUML's Symbol).
enum BoxStyle {
    /// Plain rectangle, no fill, stroke-width 0.5 (literal terminal).
    Terminal,
    /// Gray rounded rectangle, stroke-width 1.5 (special/metacharacter).
    Special,
}

/// A measured + drawable tile.
trait Tile {
    fn width(&self) -> f64;
    fn h1(&self) -> f64;
    fn h2(&self) -> f64;
    fn height(&self) -> f64 {
        self.h1() + self.h2()
    }
    fn draw(&self, ctx: &mut Ctx);
}

// ── Box tile (ETileBox) ──

struct BoxTile {
    value: String,
    style: BoxStyle,
}

impl BoxTile {
    fn text_w(&self) -> f64 {
        text_width(&self.value, FONT_SIZE, false)
    }
}

impl Tile for BoxTile {
    fn width(&self) -> f64 {
        self.text_w() + 10.0
    }
    fn h1(&self) -> f64 {
        (text_height(FONT_SIZE) + 10.0) / 2.0
    }
    fn h2(&self) -> f64 {
        self.h1()
    }
    fn draw(&self, ctx: &mut Ctx) {
        let box_w = self.text_w() + 10.0;
        let box_h = text_height(FONT_SIZE) + 10.0;
        let dim_w = self.width();
        let posx_box = (dim_w - box_w) / 2.0;
        match self.style {
            BoxStyle::Terminal => {
                ctx.rect(
                    posx_box,
                    0.0,
                    box_w,
                    box_h,
                    "none",
                    "stroke:#181818;stroke-width:0.5;",
                    None,
                );
            }
            BoxStyle::Special => {
                ctx.rect(
                    posx_box,
                    0.0,
                    box_w,
                    box_h,
                    "#F1F1F1",
                    "stroke:#181818;stroke-width:1.5;",
                    Some(5.0),
                );
            }
        }
        // text: x = 5 + posxBox; y = 5 + textHeight - descent
        let ty = 5.0 + text_height(FONT_SIZE) - descent(FONT_SIZE);
        ctx.text(5.0 + posx_box, ty, &self.value, FONT_SIZE);
        if posx_box > 0.0 {
            ctx.hline_directed(self.h1(), 0.0, posx_box, 0.5, 25.0);
            ctx.hline_directed(self.h1(), posx_box + box_w, dim_w, 0.5, 25.0);
        }
    }
}

// ── Char-class group tile (ETileRegexGroup) ──

struct GroupClassTile {
    elements: Vec<String>,
}

impl GroupClassTile {
    fn text_dim(&self) -> (f64, f64) {
        let mut w = 0.0f64;
        let mut h = 0.0f64;
        for e in &self.elements {
            w = w.max(text_width(e, FONT_SIZE, false));
            h += text_height(FONT_SIZE);
        }
        (w, h)
    }
}

impl Tile for GroupClassTile {
    fn width(&self) -> f64 {
        self.text_dim().0 + 10.0
    }
    fn h1(&self) -> f64 {
        self.text_dim().1 / 2.0
    }
    fn h2(&self) -> f64 {
        self.h1()
    }
    fn draw(&self, ctx: &mut Ctx) {
        let (tw, _th) = self.text_dim();
        let box_w = tw + 10.0;
        let box_h = self.text_dim().1; // delta(10,0): no vertical pad
        let dim_w = self.width();
        let posx_box = (dim_w - box_w) / 2.0;
        ctx.rect(
            posx_box,
            0.0,
            box_w,
            box_h,
            "none",
            "stroke:#181818;stroke-width:1;stroke-dasharray:5,5;",
            None,
        );
        let mut y = 0.0;
        for e in &self.elements {
            let eh = text_height(FONT_SIZE);
            let ty = y + eh - descent(FONT_SIZE);
            ctx.text(5.0 + posx_box, ty, e, FONT_SIZE);
            if y > 0.0 {
                let (ax1, ax2, ay) = (ctx.dx, ctx.dx + box_w, ctx.dy + y);
                ctx.svg.raw(&format!(
                    r#"<line style="stroke:#181818;stroke-width:0.3;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    fmt_coord(ax1),
                    fmt_coord(ax2),
                    fmt_coord(ay),
                    fmt_coord(ay)
                ));
            }
            y += eh;
        }
        if posx_box > 0.0 {
            ctx.hline_directed(self.h1(), 0.0, posx_box, 0.5, 25.0);
            ctx.hline_directed(self.h1(), posx_box + box_w, dim_w, 0.5, 25.0);
        }
    }
}

// ── Concatenation (ETileConcatenation) ──

struct ConcatTile {
    tiles: Vec<Box<dyn Tile>>,
}

const CONCAT_MARGIN: f64 = 20.0;

impl Tile for ConcatTile {
    fn width(&self) -> f64 {
        let mut w = 0.0;
        for (i, t) in self.tiles.iter().enumerate() {
            w += t.width();
            if i != self.tiles.len() - 1 {
                w += CONCAT_MARGIN;
            }
        }
        w
    }
    fn h1(&self) -> f64 {
        self.tiles.iter().map(|t| t.h1()).fold(0.0, f64::max)
    }
    fn h2(&self) -> f64 {
        self.tiles.iter().map(|t| t.h2()).fold(0.0, f64::max)
    }
    fn draw(&self, ctx: &mut Ctx) {
        let full = self.h1();
        let mut x = 0.0;
        ctx.hline(full, 0.0, x); // zero-length leading rail (parity with PlantUML)
        for (i, t) in self.tiles.iter().enumerate() {
            let line_pos = t.h1();
            {
                let mut sub = ctx.at(x, full - line_pos);
                t.draw(&mut sub);
            }
            x += t.width();
            if i != self.tiles.len() - 1 {
                ctx.hline_directed(full, x, x + CONCAT_MARGIN, 0.5, 25.0);
                x += CONCAT_MARGIN;
            }
        }
    }
}

// ── Alternation (ETileAlternation) ──

struct AltTile {
    tiles: Vec<Box<dyn Tile>>,
}

const ALT_MARGIN: f64 = 12.0;

impl Tile for AltTile {
    fn width(&self) -> f64 {
        let mut w = 0.0f64;
        for t in &self.tiles {
            w = w.max(t.width());
        }
        w + 2.0 * 2.0 * ALT_MARGIN
    }
    fn h1(&self) -> f64 {
        self.tiles[0].h1()
    }
    fn h2(&self) -> f64 {
        let mut h = self.tiles[0].h2();
        for t in self.tiles.iter().skip(1) {
            h += t.h1() + t.h2() + 10.0;
        }
        h
    }
    fn draw(&self, ctx: &mut Ctx) {
        let m = ALT_MARGIN;
        let a = 0.0;
        let b = a + m;
        let c = b + m;
        let r = self.width();
        let q = r - m;
        let p = q - m;
        let line_pos = self.h1();

        let mut y = 0.0;
        let mut last_line_pos = 0.0;
        let n = self.tiles.len();
        for (i, t) in self.tiles.iter().enumerate() {
            let dim_w = t.width();
            let dim_h = t.height();
            last_line_pos = y + t.h1();
            {
                let mut sub = ctx.at(c, y);
                t.draw(&mut sub);
            }
            if i == 0 {
                ctx.hline(last_line_pos, a, c);
                ctx.hline(last_line_pos, c + dim_w, r);
            } else if i < n - 1 {
                corner_sw(ctx, b, last_line_pos, m);
                ctx.hline_directed(last_line_pos, c + dim_w, p, 0.5, 25.0);
                corner_se(ctx, q, last_line_pos, m);
            } else {
                ctx.hline_directed(last_line_pos, c + dim_w, p, 0.5, 25.0);
            }
            y += dim_h + 10.0;
        }

        let height42 = last_line_pos - line_pos;
        // Left spine at b, right spine at q, both anchored at line_pos.
        corner_sw(ctx, b, line_pos + height42, m);
        {
            let mut sub = ctx.at(b, line_pos);
            sub.vline(0.0, m, height42 - m);
        }
        corner_ne(ctx, b, line_pos, m);

        corner_se(ctx, q, line_pos + height42, m);
        {
            let mut sub = ctx.at(q, line_pos);
            sub.vline(0.0, m, height42 - m);
        }
        corner_nw(ctx, q, line_pos, m);
    }
}

// ── One-or-more (ETileOneOrMore) ──

struct OneOrMoreTile {
    orig: Box<dyn Tile>,
    /// Optional count label (e.g. `{3}`) drawn on a brace above.
    loop_label: Option<String>,
}

const OOM_DELTAX: f64 = 15.0;
const OOM_DELTAY: f64 = 12.0;

impl OneOrMoreTile {
    fn brace_h(&self) -> f64 {
        if self.loop_label.is_some() { 15.0 } else { 0.0 }
    }
}

impl Tile for OneOrMoreTile {
    fn width(&self) -> f64 {
        self.orig.width() + 2.0 * OOM_DELTAX
    }
    fn h1(&self) -> f64 {
        OOM_DELTAY + self.orig.h1() + self.brace_h()
    }
    fn h2(&self) -> f64 {
        self.orig.h2()
    }
    fn draw(&self, ctx: &mut Ctx) {
        let full_w = self.width();
        let h1 = self.h1();
        let bh = self.brace_h();

        corner_sw(ctx, 8.0, h1, 8.0);
        {
            let mut sub = ctx.at(8.0, 0.0);
            sub.vline(0.0, 8.0 + 5.0 + bh, h1 - 8.0);
        }
        corner_nw(ctx, 8.0, 5.0 + bh, 8.0);

        ctx.hline_anti_directed(5.0 + bh, OOM_DELTAX, full_w - OOM_DELTAX, 0.6);

        corner_se(ctx, full_w - 8.0, h1, 8.0);
        {
            let mut sub = ctx.at(full_w - 8.0, 0.0);
            sub.vline(0.0, 8.0 + 5.0 + bh, h1 - 8.0);
        }
        corner_ne(ctx, full_w - 8.0, 5.0 + bh, 8.0);

        ctx.hline(h1, 0.0, OOM_DELTAX);
        ctx.hline(h1, full_w - OOM_DELTAX, full_w);

        {
            let mut sub = ctx.at(OOM_DELTAX, OOM_DELTAY + bh);
            self.orig.draw(&mut sub);
        }

        if let Some(label) = &self.loop_label {
            {
                let mut sub = ctx.at(0.0, 10.0);
                brace(&mut sub, 0.0, 0.0, full_w);
            }
            let tw = text_width(label, COUNT_FONT_SIZE, false);
            // y offset = descent (text baseline near top of brace)
            let d = descent(COUNT_FONT_SIZE);
            ctx.text((full_w - tw) / 2.0, d, label, COUNT_FONT_SIZE);
        }
    }
}

// ── Optional / Zero-or-more (ETileOptional2) ──

struct OptionalTile {
    orig: Box<dyn Tile>,
}

const OPT_DELTAX: f64 = 24.0;
const OPT_DELTAY: f64 = 20.0;

impl Tile for OptionalTile {
    fn width(&self) -> f64 {
        self.orig.width() + 2.0 * OPT_DELTAX
    }
    fn h1(&self) -> f64 {
        10.0
    }
    fn h2(&self) -> f64 {
        10.0 + self.orig.h1() + self.orig.h2()
    }
    fn draw(&self, ctx: &mut Ctx) {
        let dim_w = self.width();
        let line_pos = self.h1();
        ctx.hline_directed(line_pos, 0.0, dim_w, 0.4, 25.0);
        let corner = 12.0;
        let zz_w = 2.0 * corner;
        let zz_h = OPT_DELTAY + self.orig.h1() - line_pos;

        // Zigzag down at left edge.
        zigzag_down(ctx, 0.0, line_pos, 9.0, zz_w, zz_h);
        // Zigzag up at right edge.
        zigzag_up(ctx, dim_w - 2.0 * corner, line_pos, 9.0, zz_w, zz_h);

        {
            let mut sub = ctx.at(OPT_DELTAX, OPT_DELTAY);
            self.orig.draw(&mut sub);
        }
    }
}

fn zigzag_down(ctx: &mut Ctx, x: f64, y: f64, ctrl: f64, width: f64, height: f64) {
    let xm = width / 2.0;
    let ym = height / 2.0;
    let mut c = ctx.at(x, y);
    // PlantUML emits one path with two cubic segments; we emit two cubics that
    // share the midpoint (geometrically identical curve, split for our helper).
    c.cubic(0.0, 0.0, (ctrl, 0.0), (xm, ym - ctrl), (xm, ym));
    c.cubic(
        xm,
        ym,
        (xm, ym + ctrl),
        (width - ctrl, height),
        (width, height),
    );
}

fn zigzag_up(ctx: &mut Ctx, x: f64, y: f64, ctrl: f64, width: f64, height: f64) {
    let xm = width / 2.0;
    let ym = height / 2.0;
    let mut c = ctx.at(x, y);
    c.cubic(0.0, height, (ctrl, height), (xm, ym + ctrl), (xm, ym));
    c.cubic(xm, ym, (xm, ym - ctrl), (width - ctrl, 0.0), (width, 0.0));
}

// ── Lookahead / lookbehind / named group (rendered as dashed rounded box) ──

struct LookTile {
    label: String,
    inner: Box<dyn Tile>,
}

const LOOK_PAD_X: f64 = 8.0;

impl LookTile {
    fn label_w(&self) -> f64 {
        text_width(&self.label, FONT_SIZE, false) + 4.0
    }
}

impl Tile for LookTile {
    fn width(&self) -> f64 {
        self.label_w() + self.inner.width() + LOOK_PAD_X
    }
    fn h1(&self) -> f64 {
        self.inner.h1().max((text_height(FONT_SIZE) + 12.0) / 2.0)
    }
    fn h2(&self) -> f64 {
        self.inner.h2().max((text_height(FONT_SIZE) + 12.0) / 2.0)
    }
    fn draw(&self, ctx: &mut Ctx) {
        let h = self.height();
        ctx.rect(
            0.0,
            0.0,
            self.width(),
            h,
            "none",
            "stroke:#181818;stroke-width:1;stroke-dasharray:2,3;",
            Some(15.0),
        );
        let ty = self.h1() + ascent(FONT_SIZE) / 2.0;
        ctx.text(5.0, ty, &self.label, FONT_SIZE);
        {
            let mut sub = ctx.at(self.label_w(), self.h1() - self.inner.h1());
            self.inner.draw(&mut sub);
        }
    }
}

// ── AST → Tile ──────────────────────────────────────────────────────────────

fn build(node: &RegexNode) -> Box<dyn Tile> {
    match node {
        RegexNode::Literal { text } => Box::new(BoxTile {
            value: text.clone(),
            style: BoxStyle::Terminal,
        }),
        RegexNode::Special { text } => Box::new(BoxTile {
            value: text.clone(),
            style: BoxStyle::Special,
        }),
        RegexNode::CharClass { items } => {
            let elements = if items.is_empty() {
                vec![String::new()]
            } else {
                items.clone()
            };
            Box::new(GroupClassTile { elements })
        }
        RegexNode::Sequence { items } => {
            if items.len() == 1 {
                build(&items[0])
            } else {
                Box::new(ConcatTile {
                    tiles: items.iter().map(build).collect(),
                })
            }
        }
        RegexNode::Alternation { branches } => Box::new(AltTile {
            tiles: branches.iter().map(build).collect(),
        }),
        RegexNode::Repeat { inner, min, max } => build_repeat(inner, *min, *max),
        RegexNode::Group { kind, inner } => match kind {
            GroupKind::Lookahead { .. } | GroupKind::Lookbehind { .. } => Box::new(LookTile {
                label: group_label(kind),
                inner: build(inner),
            }),
            GroupKind::Named { name } => Box::new(LookTile {
                label: format!("?<{name}>"),
                inner: build(inner),
            }),
            GroupKind::Flags { flags } => Box::new(BoxTile {
                value: format!("(?{flags})"),
                style: BoxStyle::Special,
            }),
            // Capture / non-capture groups are transparent: render the inner tile.
            _ => build(inner),
        },
    }
}

fn build_repeat(inner: &RegexNode, min: u32, max: Option<u32>) -> Box<dyn Tile> {
    match (min, max) {
        (1, None) => Box::new(OneOrMoreTile {
            orig: build(inner),
            loop_label: None,
        }),
        (0, Some(1)) => Box::new(OptionalTile { orig: build(inner) }),
        (0, None) => Box::new(OptionalTile {
            orig: Box::new(OneOrMoreTile {
                orig: build(inner),
                loop_label: None,
            }),
        }),
        _ => Box::new(OneOrMoreTile {
            orig: build(inner),
            loop_label: Some(count_label(min, max)),
        }),
    }
}

fn count_label(min: u32, max: Option<u32>) -> String {
    match max {
        Some(m) if m == min => format!("{{{min}}}"),
        Some(m) => format!("{{{min},{m}}}"),
        None => format!("{{{min},}}"),
    }
}

fn group_label(kind: &GroupKind) -> String {
    match kind {
        GroupKind::Lookahead { positive: true } => "?=".to_string(),
        GroupKind::Lookahead { positive: false } => "?!".to_string(),
        GroupKind::Lookbehind { positive: true } => "?<=".to_string(),
        GroupKind::Lookbehind { positive: false } => "?<!".to_string(),
        _ => "?".to_string(),
    }
}

// ── Public render function ────────────────────────────────────────────────────

/// Render a regex diagram. The oracle layout is unused (geometry is computed
/// directly from PlantUML's tile model).
pub fn render_with_oracle(
    diagram: &RegexDiagram,
    theme: &Theme,
    _oracle: Option<&OracleLayout>,
) -> String {
    render(diagram, theme)
}

/// Render a [`RegexDiagram`] to an SVG string.
pub fn render(diagram: &RegexDiagram, _theme: &Theme) -> String {
    let tile = build(&diagram.ast);

    let content_w = tile.width();
    let rail = OUTER + tile.h1();
    let canvas_w = (OUTER + content_w + OUTER).ceil();
    let canvas_h = (OUTER + tile.height() + OUTER).ceil();

    let mut svg = SvgBuilder::new_plantuml(canvas_w, canvas_h, "REGEX");

    {
        let mut ctx = Ctx {
            svg: &mut svg,
            dx: OUTER,
            dy: rail - tile.h1(),
        };
        tile.draw(&mut ctx);
    }

    svg.finalize_plantuml()
}
