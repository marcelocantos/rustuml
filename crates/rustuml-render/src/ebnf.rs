// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! EBNF railroad diagram renderer.
//!
//! This is a faithful semantic port of PlantUML's `net.sourceforge.plantuml.ebnf`
//! ETile model. Each production rule renders as a bold title above a railroad
//! "tile" built from the grammar structure: boxes for terminals / nonterminals,
//! concatenation laid out left-to-right, alternation stacked vertically with
//! curved fork/join connectors, optionals as a bypass arc, and repetitions as a
//! loop-back. All geometry is computed from the parsed grammar plus Java AWT
//! font metrics — nothing is echoed from the golden output.

use std::fmt::Write as _;

use rustuml_parser::diagram::ebnf::{EbnfDiagram, EbnfExpr};

use crate::layout_oracle::OracleLayout;
use crate::plantuml_metrics as pm;
use crate::style::Theme;

// ── Constants (mirroring the Java tile classes) ────────────────────────────────

const FONT_SIZE: f64 = 14.0;
const LINE_COLOR: &str = "#181818";
/// Outer image margin applied by PlantUML's ImageBuilder.
const IMAGE_MARGIN: f64 = 10.0;
/// Top margin between the title and the main tile (`withMargin(main, 0,0,10,15)`).
const MAIN_MARGIN_TOP: f64 = 10.0;
const MAIN_MARGIN_BOTTOM: f64 = 15.0;
/// Minimum gap that triggers a directed-line arrowhead.
const MIN_ARROW: f64 = 25.0;

fn n(v: f64) -> String {
    pm::fmt_coord(v)
}

// ── Tile model ─────────────────────────────────────────────────────────────────

/// A laid-out railroad tile. Mirrors the Java `ETile` hierarchy. Heights are
/// expressed as `h1` (above the entry/exit rail) and `h2` (below it); `width`
/// is the full horizontal extent.
enum Tile {
    /// `ETileBox`: terminal (`is_terminal = true`, plain rect, stroke 0.5) or
    /// nonterminal/special (rounded rect, stroke 1.5).
    Box { text: String, terminal: bool },
    /// `ETileConcatenation`: a horizontal run of tiles joined by directed rails.
    Concat(Vec<Tile>),
    /// `ETileAlternation`: vertically stacked branches with curved fork/join.
    Alt(Vec<Tile>),
    /// `ETileOptional2`: a bypass rail above, the inner tile reachable below.
    Optional(Box<Tile>),
    /// `ETileOneOrMore`: the inner tile with a loop-back rail beneath.
    OneOrMore(Box<Tile>),
    /// `ETileZeroOrMore` == `ETileOptional2(ETileOneOrMore(inner))`.
    ZeroOrMore(Box<Tile>),
}

// ── Building tiles from the parsed grammar ──────────────────────────────────────

fn build(expr: &EbnfExpr) -> Tile {
    match expr {
        EbnfExpr::Terminal(s) => Tile::Box {
            text: s.clone(),
            terminal: true,
        },
        EbnfExpr::Nonterminal(s) => Tile::Box {
            text: s.clone(),
            terminal: false,
        },
        EbnfExpr::Group(inner) => build(inner),
        EbnfExpr::Sequence(items) => {
            if items.len() == 1 {
                build(&items[0])
            } else {
                Tile::Concat(items.iter().map(build).collect())
            }
        }
        EbnfExpr::Alternation(branches) => Tile::Alt(branches.iter().map(build).collect()),
        EbnfExpr::Optional(inner) => Tile::Optional(Box::new(build(inner))),
        EbnfExpr::Repetition(inner) => Tile::ZeroOrMore(Box::new(build(inner))),
    }
}

// ── Box metrics (ETileBox) ──────────────────────────────────────────────────────

/// Text dimensions for a box value, matching `stringBounder.calculateDimension`.
fn text_dim(text: &str) -> (f64, f64) {
    (
        pm::text_width(text, FONT_SIZE, false),
        pm::text_height(FONT_SIZE),
    )
}

/// `ETileBox.getPureH1`: (textHeight + 10) / 2.
fn box_pure_h1(text: &str) -> f64 {
    (text_dim(text).1 + 10.0) / 2.0
}

// ── Tile measurement ────────────────────────────────────────────────────────────

impl Tile {
    fn width(&self) -> f64 {
        match self {
            Tile::Box { text, .. } => text_dim(text).0 + 10.0,
            Tile::Concat(tiles) => {
                let mut w = 0.0;
                for (i, t) in tiles.iter().enumerate() {
                    w += t.width();
                    if i != tiles.len() - 1 {
                        w += 20.0; // marginx
                    }
                }
                w
            }
            Tile::Alt(tiles) => {
                let max = tiles.iter().map(|t| t.width()).fold(0.0_f64, f64::max);
                max + 2.0 * 2.0 * 12.0 // 2*2*marginx
            }
            Tile::Optional(inner) => inner.width() + 2.0 * 24.0, // 2*deltax
            Tile::OneOrMore(inner) => inner.width() + 2.0 * 15.0, // 2*deltax
            Tile::ZeroOrMore(inner) => {
                // Optional2 over OneOrMore(inner)
                (inner.width() + 2.0 * 15.0) + 2.0 * 24.0
            }
        }
    }

    fn h1(&self) -> f64 {
        match self {
            Tile::Box { text, .. } => box_pure_h1(text),
            Tile::Concat(tiles) => tiles.iter().map(|t| t.h1()).fold(0.0_f64, f64::max),
            Tile::Alt(tiles) => tiles[0].h1(),
            // `[ x ]` builds an ETileOptional2 directly; `getH1` is a flat 10.
            Tile::Optional(_) => 10.0,
            Tile::OneOrMore(inner) => 12.0 + inner.h1(), // deltay (no brace label)
            Tile::ZeroOrMore(_) => 10.0,                 // Optional2.getH1
        }
    }

    fn h2(&self) -> f64 {
        match self {
            Tile::Box { text, .. } => box_pure_h1(text),
            Tile::Concat(tiles) => tiles.iter().map(|t| t.h2()).fold(0.0_f64, f64::max),
            Tile::Alt(tiles) => {
                let mut h = tiles[0].h2();
                for t in &tiles[1..] {
                    h += t.h1() + t.h2() + 10.0;
                }
                h
            }
            // ETileOptional2.getH2 = 10 + inner.h1 + inner.h2.
            Tile::Optional(inner) => 10.0 + inner.h1() + inner.h2(),
            Tile::OneOrMore(inner) => inner.h2(),
            Tile::ZeroOrMore(inner) => {
                // Optional2.getH2 over OneOrMore(inner)
                let oom = Tile::OneOrMore(Box::new(clone_tile(inner)));
                10.0 + oom.h1() + oom.h2()
            }
        }
    }

    fn height(&self) -> f64 {
        self.h1() + self.h2()
    }
}

/// Tiles are tree-shaped and small; cloning to re-measure a wrapped child is
/// cheaper and clearer than threading a borrow through the recursion.
fn clone_tile(t: &Tile) -> Tile {
    match t {
        Tile::Box { text, terminal } => Tile::Box {
            text: text.clone(),
            terminal: *terminal,
        },
        Tile::Concat(v) => Tile::Concat(v.iter().map(clone_tile).collect()),
        Tile::Alt(v) => Tile::Alt(v.iter().map(clone_tile).collect()),
        Tile::Optional(i) => Tile::Optional(Box::new(clone_tile(i))),
        Tile::OneOrMore(i) => Tile::OneOrMore(Box::new(clone_tile(i))),
        Tile::ZeroOrMore(i) => Tile::ZeroOrMore(Box::new(clone_tile(i))),
    }
}

// ── SVG primitive emission (PlantUML format) ────────────────────────────────────

struct Canvas {
    buf: String,
    /// Current cumulative translate (UGraphic-style).
    ox: f64,
    oy: f64,
}

impl Canvas {
    fn at(&self, x: f64, y: f64) -> (f64, f64) {
        (self.ox + x, self.oy + y)
    }

    fn hline(&mut self, y: f64, x1: f64, x2: f64) {
        let (ax1, ay) = self.at(x1, y);
        let (ax2, _) = self.at(x2, y);
        let _ = write!(
            self.buf,
            r#"<line style="stroke:{LINE_COLOR};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            n(ax1),
            n(ax2),
            n(ay),
            n(ay)
        );
    }

    fn vline(&mut self, x: f64, y1: f64, y2: f64) {
        let (ax, ay1) = self.at(x, y1);
        let (_, ay2) = self.at(x, y2);
        let _ = write!(
            self.buf,
            r#"<line style="stroke:{LINE_COLOR};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            n(ax),
            n(ax),
            n(ay1),
            n(ay2)
        );
    }

    /// `drawHlineDirected`: a horizontal line plus a rightward arrowhead at
    /// `x1*(1-coef)+x2*coef` when the span exceeds `min_arrow`.
    fn hline_directed(&mut self, y: f64, x1: f64, x2: f64, coef: f64, min_arrow: f64) {
        self.hline(y, x1, x2);
        if x2 > x1 + min_arrow {
            let ax = x1 * (1.0 - coef) + x2 * coef - 2.0;
            self.arrow_right(ax, y);
        }
    }

    /// `drawHlineAntiDirected`: line plus a leftward arrowhead (coef fixed point).
    fn hline_anti_directed(&mut self, y: f64, x1: f64, x2: f64, coef: f64) {
        self.hline(y, x1, x2);
        let ax = x1 * (1.0 - coef) + x2 * coef - 2.0;
        self.arrow_left(ax, y);
    }

    fn arrow_right(&mut self, x: f64, y: f64) {
        // moveTo(0,0) lineTo(0,-3) lineTo(6,0) lineTo(0,3) lineTo(0,0)
        let (x0, y0) = self.at(x, y);
        let _ = write!(
            self.buf,
            r##"<path d="M{},{} L{},{} L{},{} L{},{} L{},{}" fill="{LINE_COLOR}"/>"##,
            n(x0),
            n(y0),
            n(x0),
            n(y0 - 3.0),
            n(x0 + 6.0),
            n(y0),
            n(x0),
            n(y0 + 3.0),
            n(x0),
            n(y0)
        );
    }

    fn arrow_left(&mut self, x: f64, y: f64) {
        // moveTo(0,0) lineTo(0,-3) lineTo(-6,0) lineTo(0,3) lineTo(0,0)
        let (x0, y0) = self.at(x, y);
        let _ = write!(
            self.buf,
            r##"<path d="M{},{} L{},{} L{},{} L{},{} L{},{}" fill="{LINE_COLOR}"/>"##,
            n(x0),
            n(y0),
            n(x0),
            n(y0 - 3.0),
            n(x0 - 6.0),
            n(y0),
            n(x0),
            n(y0 + 3.0),
            n(x0),
            n(y0)
        );
    }

    /// `CornerCurved` cubic at translate (tx, ty). `a = delta/4`.
    fn corner(&mut self, kind: Corner, delta: f64, tx: f64, ty: f64) {
        let a = delta / 4.0;
        let (sx, sy, c1x, c1y, c2x, c2y, ex, ey) = match kind {
            Corner::Sw => (0.0, -delta, 0.0, -a, a, 0.0, delta, 0.0),
            Corner::Se => (0.0, -delta, 0.0, -a, -a, 0.0, -delta, 0.0),
            Corner::Ne => (-delta, 0.0, -a, 0.0, 0.0, a, 0.0, delta),
            Corner::Nw => (0.0, delta, 0.0, a, a, 0.0, delta, 0.0),
        };
        let (asx, asy) = self.at(tx + sx, ty + sy);
        let (ac1x, ac1y) = self.at(tx + c1x, ty + c1y);
        let (ac2x, ac2y) = self.at(tx + c2x, ty + c2y);
        let (aex, aey) = self.at(tx + ex, ty + ey);
        let _ = write!(
            self.buf,
            r#"<path d="M{},{} C{},{} {},{} {},{}" fill="none" style="stroke:{LINE_COLOR};stroke-width:1.5;"/>"#,
            n(asx),
            n(asy),
            n(ac1x),
            n(ac1y),
            n(ac2x),
            n(ac2y),
            n(aex),
            n(aey)
        );
    }

    fn ellipse(&mut self, cx: f64, cy: f64, size: f64, filled: bool, stroke_w: f64) {
        let (acx, acy) = self.at(cx, cy);
        let r = size / 2.0;
        let fill = if filled { LINE_COLOR } else { "none" };
        let _ = write!(
            self.buf,
            r#"<ellipse cx="{}" cy="{}" fill="{fill}" rx="{}" ry="{}" style="stroke:{LINE_COLOR};stroke-width:{};"/>"#,
            n(acx),
            n(acy),
            n(r),
            n(r),
            n(stroke_w)
        );
    }

    fn box_rect(&mut self, x: f64, y: f64, w: f64, h: f64, terminal: bool) {
        let (ax, ay) = self.at(x, y);
        if terminal {
            let _ = write!(
                self.buf,
                r#"<rect fill="none" height="{}" style="stroke:{LINE_COLOR};stroke-width:0.5;" width="{}" x="{}" y="{}"/>"#,
                n(h),
                n(w),
                n(ax),
                n(ay)
            );
        } else {
            let _ = write!(
                self.buf,
                r##"<rect fill="#F1F1F1" height="{}" rx="5" ry="5" style="stroke:{LINE_COLOR};stroke-width:1.5;" width="{}" x="{}" y="{}"/>"##,
                n(h),
                n(w),
                n(ax),
                n(ay)
            );
        }
    }

    fn text(&mut self, x: f64, y: f64, text: &str, bold: bool) {
        let (ax, ay) = self.at(x, y);
        let tl = if bold {
            pm::text_width(text, FONT_SIZE, true)
        } else {
            pm::text_width(text, FONT_SIZE, false)
        };
        let weight = if bold { r#" font-weight="700""# } else { "" };
        let _ = write!(
            self.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="14"{weight} lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            n(tl),
            n(ax),
            n(ay),
            escape_xml(text)
        );
    }
}

#[derive(Clone, Copy)]
enum Corner {
    Nw,
    Ne,
    Se,
    Sw,
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ── Tile drawing (ports each ETile.drawU) ───────────────────────────────────────

fn draw(tile: &Tile, c: &mut Canvas) {
    match tile {
        Tile::Box { text, terminal } => {
            draw_box(text, *terminal, tile.width(), tile.h1() + tile.h2(), c)
        }
        Tile::Concat(tiles) => draw_concat(tiles, c),
        Tile::Alt(tiles) => draw_alt(tiles, c),
        Tile::Optional(inner) => draw_optional(inner, c),
        Tile::OneOrMore(inner) => draw_one_or_more(inner, c, false),
        Tile::ZeroOrMore(inner) => draw_zero_or_more(inner, c),
    }
}

fn draw_box(text: &str, terminal: bool, full_w: f64, full_h: f64, c: &mut Canvas) {
    let (tw, th) = text_dim(text);
    let box_w = tw + 10.0;
    let box_h = th + 10.0;
    let posx_box = (full_w - box_w) / 2.0;
    let h1 = (th + 10.0) / 2.0;

    c.box_rect(posx_box, 0.0, box_w, box_h, terminal);
    // utext baseline: posy + 5 + textHeight - descent ; posy = 0.
    let baseline = 5.0 + th - pm::descent(FONT_SIZE);
    c.text(5.0 + posx_box, baseline, text, false);

    let _ = full_h;
    if posx_box > 0.0 {
        c.hline_directed(h1, 0.0, posx_box, 0.5, MIN_ARROW);
        c.hline_directed(h1, posx_box + box_w, full_w, 0.5, MIN_ARROW);
    }
}

fn draw_concat(tiles: &[Tile], c: &mut Canvas) {
    let full_line = Tile::Concat(tiles.iter().map(clone_tile).collect()).h1();
    let mut x = 0.0;
    // ETileConcatenation emits a (zero-length) rail segment at the origin.
    c.hline(full_line, 0.0, x);
    for (i, tile) in tiles.iter().enumerate() {
        let line_pos = tile.h1();
        let (sox, soy) = (c.ox, c.oy);
        c.ox = sox + x;
        c.oy = soy + (full_line - line_pos);
        draw(tile, c);
        c.ox = sox;
        c.oy = soy;
        x += tile.width();
        if i != tiles.len() - 1 {
            c.hline_directed(full_line, x, x + 20.0, 0.5, MIN_ARROW);
            x += 20.0;
        }
    }
}

fn draw_alt(tiles: &[Tile], c: &mut Canvas) {
    let marginx = 12.0;
    let a = 0.0;
    let b = a + marginx;
    let cc = b + marginx;
    let full_w = Tile::Alt(tiles.iter().map(clone_tile).collect()).width();
    let r = full_w;
    let q = r - marginx;
    let p = q - marginx;
    let line_pos = tiles[0].h1();

    let mut y = 0.0;
    let mut last_line_pos = 0.0;
    for (i, tile) in tiles.iter().enumerate() {
        let dim_w = tile.width();
        let dim_h = tile.height();
        last_line_pos = y + tile.h1();
        let (sox, soy) = (c.ox, c.oy);
        c.ox = sox + cc;
        c.oy = soy + y;
        draw(tile, c);
        c.ox = sox;
        c.oy = soy;

        if i == 0 {
            c.hline(last_line_pos, a, cc);
            c.hline(last_line_pos, cc + dim_w, r);
        } else if i < tiles.len() - 1 {
            c.corner(Corner::Sw, marginx, b, last_line_pos);
            c.hline_directed(last_line_pos, cc + dim_w, p, 0.5, MIN_ARROW);
            c.corner(Corner::Se, marginx, q, last_line_pos);
        } else {
            c.hline_directed(last_line_pos, cc + dim_w, p, 0.5, MIN_ARROW);
        }
        y += dim_h + 10.0;
    }

    let height42 = last_line_pos - line_pos;
    // ug_b at (b, line_pos), ug_q at (q, line_pos)
    c.corner(Corner::Sw, marginx, b, line_pos + height42);
    {
        let (sox, soy) = (c.ox, c.oy);
        c.ox = sox + b;
        c.oy = soy + line_pos;
        c.vline(0.0, marginx, height42 - marginx);
        c.ox = sox;
        c.oy = soy;
    }
    c.corner(Corner::Ne, marginx, b, line_pos);

    c.corner(Corner::Se, marginx, q, line_pos + height42);
    {
        let (sox, soy) = (c.ox, c.oy);
        c.ox = sox + q;
        c.oy = soy + line_pos;
        c.vline(0.0, marginx, height42 - marginx);
        c.ox = sox;
        c.oy = soy;
    }
    c.corner(Corner::Nw, marginx, q, line_pos);
}

fn draw_optional(inner: &Tile, c: &mut Canvas) {
    // ETileOptional2
    let deltax = 24.0;
    let deltay = 20.0;
    let full_w = inner.width() + 2.0 * deltax;
    let line_pos = 10.0; // getH1 (no note)
    let delta_y = deltay; // getDeltaY (no note)

    c.hline_directed(line_pos, 0.0, full_w, 0.4, MIN_ARROW);
    let corner = 12.0;
    let height = delta_y + inner.h1() - line_pos;
    draw_zigzag_down(c, 0.0, line_pos, 2.0 * corner, height);
    draw_zigzag_up(c, full_w - 2.0 * corner, line_pos, 2.0 * corner, height);

    let (sox, soy) = (c.ox, c.oy);
    c.ox = sox + deltax;
    c.oy = soy + delta_y;
    draw(inner, c);
    c.ox = sox;
    c.oy = soy;
}

fn draw_zero_or_more(inner: &Tile, c: &mut Canvas) {
    // ETileZeroOrMore = ETileOptional2(ETileOneOrMore(inner))
    let deltax = 24.0;
    let deltay = 20.0;
    let oom = Tile::OneOrMore(Box::new(clone_tile(inner)));
    let full_w = oom.width() + 2.0 * deltax;
    let line_pos = 10.0;
    let delta_y = deltay;

    c.hline_directed(line_pos, 0.0, full_w, 0.4, MIN_ARROW);
    let corner = 12.0;
    let height = delta_y + oom.h1() - line_pos;
    draw_zigzag_down(c, 0.0, line_pos, 2.0 * corner, height);
    draw_zigzag_up(c, full_w - 2.0 * corner, line_pos, 2.0 * corner, height);

    let (sox, soy) = (c.ox, c.oy);
    c.ox = sox + deltax;
    c.oy = soy + delta_y;
    draw_one_or_more(inner, c, true);
    c.ox = sox;
    c.oy = soy;
}

fn draw_one_or_more(inner: &Tile, c: &mut Canvas, _in_zom: bool) {
    let deltax = 15.0;
    let deltay = 12.0;
    let full_w = inner.width() + 2.0 * deltax;
    let h1 = deltay + inner.h1(); // no brace label
    let brace = 0.0;

    c.corner(Corner::Sw, 8.0, 8.0, h1);
    {
        let (sox, soy) = (c.ox, c.oy);
        c.ox = sox + 8.0;
        c.oy = soy;
        c.vline(0.0, 8.0 + 5.0 + brace, h1 - 8.0);
        c.ox = sox;
        c.oy = soy;
    }
    c.corner(Corner::Nw, 8.0, 8.0, 5.0 + brace);

    c.hline_anti_directed(5.0 + brace, deltax, full_w - deltax, 0.6);

    c.corner(Corner::Se, 8.0, full_w - 8.0, h1);
    {
        let (sox, soy) = (c.ox, c.oy);
        c.ox = sox + (full_w - 8.0);
        c.oy = soy;
        c.vline(0.0, 8.0 + 5.0 + brace, h1 - 8.0);
        c.ox = sox;
        c.oy = soy;
    }
    c.corner(Corner::Ne, 8.0, full_w - 8.0, 5.0 + brace);

    c.hline(h1, 0.0, deltax);
    c.hline(h1, full_w - deltax, full_w);

    let (sox, soy) = (c.ox, c.oy);
    c.ox = sox + deltax;
    c.oy = soy + deltay + brace;
    draw(inner, c);
    c.ox = sox;
    c.oy = soy;
}

/// `Zigzag.pathDown` at translate (tx, ty).
fn draw_zigzag_down(c: &mut Canvas, tx: f64, ty: f64, width: f64, height: f64) {
    let ctrl = 9.0;
    let xm = width / 2.0;
    let ym = height / 2.0;
    let pts = [
        (0.0, 0.0),
        (ctrl, 0.0),
        (xm, ym - ctrl),
        (xm, ym),
        (xm, ym + ctrl),
        (width - ctrl, height),
        (width, height),
    ];
    emit_two_cubics(c, tx, ty, &pts);
}

/// `Zigzag.pathUp` at translate (tx, ty).
fn draw_zigzag_up(c: &mut Canvas, tx: f64, ty: f64, width: f64, height: f64) {
    let ctrl = 9.0;
    let xm = width / 2.0;
    let ym = height / 2.0;
    let pts = [
        (0.0, height),
        (ctrl, height),
        (xm, ym + ctrl),
        (xm, ym),
        (xm, ym - ctrl),
        (width - ctrl, 0.0),
        (width, 0.0),
    ];
    emit_two_cubics(c, tx, ty, &pts);
}

/// Emit a path `M p0 C p1 p2 p3 C p4 p5 p6`, translated by (tx, ty).
fn emit_two_cubics(c: &mut Canvas, tx: f64, ty: f64, pts: &[(f64, f64); 7]) {
    let p: Vec<(f64, f64)> = pts.iter().map(|&(x, y)| c.at(tx + x, ty + y)).collect();
    let _ = write!(
        c.buf,
        r#"<path d="M{},{} C{},{} {},{} {},{} C{},{} {},{} {},{}" fill="none" style="stroke:{LINE_COLOR};stroke-width:1.5;"/>"#,
        n(p[0].0),
        n(p[0].1),
        n(p[1].0),
        n(p[1].1),
        n(p[2].0),
        n(p[2].1),
        n(p[3].0),
        n(p[3].1),
        n(p[4].0),
        n(p[4].1),
        n(p[5].0),
        n(p[5].1),
        n(p[6].0),
        n(p[6].1)
    );
}

// ── ETileWithCircles ────────────────────────────────────────────────────────────

fn with_circles_dim(inner: &Tile) -> (f64, f64, f64) {
    let deltax = 30.0;
    (inner.width() + 2.0 * deltax, inner.h1(), inner.h2())
}

fn draw_with_circles(inner: &Tile, c: &mut Canvas) {
    const SIZE: f64 = 8.0;
    let deltax = 30.0;
    let line_pos = inner.h1();
    let (full_w, _, _) = with_circles_dim(inner);

    // inner at dx = deltax
    let (sox, soy) = (c.ox, c.oy);
    c.ox = sox + deltax;
    c.oy = soy;
    draw(inner, c);
    c.ox = sox;
    c.oy = soy;

    // Start circle (open, stroke 2) at (0, line_pos - SIZE/2); end circle filled.
    c.ellipse(SIZE / 2.0, line_pos, SIZE, false, 2.0);
    c.ellipse(full_w - SIZE / 2.0 + SIZE / 2.0, line_pos, SIZE, true, 1.0);

    c.hline_directed(line_pos, SIZE, deltax, 0.5, MIN_ARROW);
    c.hline_directed(
        line_pos,
        full_w - deltax,
        full_w - SIZE / 2.0,
        0.5,
        MIN_ARROW,
    );
}

// ── Public entry points ─────────────────────────────────────────────────────────

/// Render an EBNF diagram. The `_oracle` parameter is accepted for API
/// symmetry with the other renderers but is intentionally unused: the
/// geometry is computed from the parsed grammar plus font metrics, never
/// echoed from the golden output.
pub fn render_with_oracle(
    diagram: &EbnfDiagram,
    theme: &Theme,
    _oracle: Option<&OracleLayout>,
) -> String {
    render(diagram, theme)
}

pub fn render(diagram: &EbnfDiagram, _theme: &Theme) -> String {
    if diagram.rules.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Measure each rule into a (title, tile) pair and compute the stacked layout.
    struct Laid {
        name: String,
        tile: Tile,
        title_w: f64,
        title_h: f64,
        block_w: f64,
        block_h: f64,
    }

    let title_h = pm::text_height(FONT_SIZE);
    let mut blocks: Vec<Laid> = Vec::new();
    for rule in &diagram.rules {
        let inner = build(&rule.body);
        let title_w = pm::text_width(&rule.name, FONT_SIZE, true);
        let (wc_w, _, _) = with_circles_dim(&inner);
        let block_w = title_w.max(wc_w);
        let main_h = inner.height();
        let block_h = title_h + MAIN_MARGIN_TOP + main_h + MAIN_MARGIN_BOTTOM;
        blocks.push(Laid {
            name: rule.name.clone(),
            tile: inner,
            title_w,
            title_h,
            block_w,
            block_h,
        });
    }

    let content_w = blocks.iter().map(|b| b.block_w).fold(0.0_f64, f64::max);
    let content_h: f64 = blocks.iter().map(|b| b.block_h).sum();
    let canvas_w = (content_w + 2.0 * IMAGE_MARGIN).ceil();
    let canvas_h = (content_h + 2.0 * IMAGE_MARGIN).ceil();

    let mut c = Canvas {
        buf: String::new(),
        ox: IMAGE_MARGIN,
        oy: IMAGE_MARGIN,
    };

    let mut y = 0.0;
    for b in &blocks {
        // Title baseline = y + titleHeight - descent.
        let baseline = y + b.title_h - pm::descent(FONT_SIZE);
        let saved = (c.ox, c.oy);
        c.text(0.0, baseline, &b.name, true);
        let _ = b.title_w;

        // Main tile (ETileWithCircles) placed below the title + top margin.
        c.ox = saved.0;
        c.oy = saved.1 + y + b.title_h + MAIN_MARGIN_TOP;
        draw_with_circles(&b.tile, &mut c);
        c.ox = saved.0;
        c.oy = saved.1;

        y += b.block_h;
    }

    let w = canvas_w as i64;
    let h = canvas_h as i64;
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="EBNF" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#
    );
    out.push_str("<?plantuml ?><defs/><g>");
    out.push_str(&c.buf);
    out.push_str("</g></svg>");
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_simple_ebnf() {
        let input = "@startebnf\nop = \"+\" | \"-\";\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"), "should produce SVG");
        assert!(svg.contains('+'), "svg should contain terminal '+'");
        assert!(svg.contains("op"), "svg should contain rule name 'op'");
    }

    #[test]
    fn renders_repetition() {
        let input = "@startebnf\nlist = { item };\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("item"));
        assert!(svg.contains("list"));
    }

    #[test]
    fn empty_ebnf_does_not_panic() {
        let input = "@startebnf\n@endebnf";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"));
    }
}
