// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Network diagram (nwdiag) SVG renderer.
//!
//! Semantic port of PlantUML's `net.sourceforge.plantuml.nwdiag` layout
//! engine (the `next` package: `NwDiagram`, `GridTextBlock*`, `NServerDraw`,
//! `NPlayField`/`NTetris`). Geometry is reconstructed from the puml source
//! using PlantUML-compatible font metrics; nothing is copied from the golden.

use std::collections::{BTreeSet, HashMap};

use rustuml_parser::diagram::nwdiag::*;

use crate::layout_oracle::OracleLayout;
use crate::plantuml_metrics::{ascent, fmt_coord, text_height, text_width};
use crate::style::Theme;

// ─── Layout constants (mirrors the Java source) ──────────────────────
const OUTER_MARGIN: f64 = 5.0; // NwDiagram.margin
const NETWORK_THIN: f64 = 5.0; // GridTextBlockDecorated.NETWORK_THIN (bar height)
const MAGIC: f64 = 15.0; // NServerDraw.MAGIC (== default topMargin)
const MARGIN_AD: f64 = 10.0; // NServerDraw.marginAd
const MARGIN_BOX_W: f64 = 15.0; // NServerDraw.marginBoxW()
const MINIMUM_WIDTH: f64 = 70.0; // GridTextBlockSimple.MINIMUM_WIDTH
const LABEL_DELTAX_PAD: f64 = 5.0; // `deltaX += 5` in drawMe
const BOX_TEXT_MARGIN: f64 = 10.0; // RECTANGLE asSmall margin around the label
const FONT_SERVER: f64 = 12.0; // server / network label font size
const FONT_ARROW: f64 = 11.0; // link (address) font size
const NET_FILL_DEFAULT: &str = "#E2E2F0";
const HOST_FILL: &str = "#F1F1F1";
const LINE_COLOR: &str = "#181818";

/// A server (host) resolved across all the networks it connects to.
struct Server {
    label: String,
    name: String,
    /// Row indices this server connects to, ascending.
    rows: Vec<usize>,
    /// Per-row connection address keyed by row index.
    addr: HashMap<usize, String>,
    /// Column assigned by the tetris packer.
    col: usize,
}

impl Server {
    fn main_row(&self) -> usize {
        self.rows[0]
    }
}

pub fn render_with_oracle(
    diagram: &NwdiagDiagram,
    theme: &Theme,
    _oracle: Option<&OracleLayout>,
) -> String {
    render(diagram, theme)
}

pub fn render(diagram: &NwdiagDiagram, _theme: &Theme) -> String {
    if diagram.networks.is_empty() {
        return concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" "#,
            r#"xmlns:xlink="http://www.w3.org/1999/xlink" width="200" height="60" "#,
            "viewBox=\"0 0 200 60\"></svg>\n"
        )
        .to_string();
    }

    let num_rows = diagram.networks.len();

    // ─── Collect servers in first-appearance order (Java's LinkedHashMap) ──
    let mut servers: Vec<Server> = Vec::new();
    for (row, net) in diagram.networks.iter().enumerate() {
        for host in &net.hosts {
            let idx = servers.iter().position(|s| s.name == host.name);
            let s = match idx {
                Some(i) => &mut servers[i],
                None => {
                    servers.push(Server {
                        label: host
                            .description
                            .clone()
                            .unwrap_or_else(|| host.name.clone()),
                        name: host.name.clone(),
                        rows: Vec::new(),
                        addr: HashMap::new(),
                        col: 0,
                    });
                    servers.last_mut().unwrap()
                }
            };
            if !s.rows.contains(&row) {
                s.rows.push(row);
            }
            if let Some(d) = &host.description {
                s.label = d.clone();
            }
            if let Some(a) = &host.address {
                s.addr.insert(row, a.clone());
            }
        }
    }

    // ─── Column assignment via tetris packing on [start_row..end_row] ──────
    let mut burned: BTreeSet<(usize, usize)> = BTreeSet::new();
    for s in &mut servers {
        let start = *s.rows.iter().min().unwrap();
        let end = *s.rows.iter().max().unwrap();
        let mut col = 0usize;
        loop {
            if (start..=end).all(|r| !burned.contains(&(col, r))) {
                for r in start..=end {
                    burned.insert((col, r));
                }
                s.col = col;
                break;
            }
            col += 1;
        }
    }
    let num_cols = servers.iter().map(|s| s.col + 1).max().unwrap_or(0);

    // ─── Box (host) dimensions ─────────────────────────────────────────────
    let box_h = text_height(FONT_SERVER) + 2.0 * BOX_TEXT_MARGIN;
    let box_w = |s: &Server| text_width(&s.label, FONT_SERVER, false) + 2.0 * BOX_TEXT_MARGIN;

    // link1 = address for the server's own (first) network row.
    // link2 = address for the immediately following network row, if any.
    let link1_text = |s: &Server| -> Option<String> {
        s.addr.get(&s.main_row()).filter(|x| !x.is_empty()).cloned()
    };
    let link2_text = |s: &Server| -> Option<String> {
        let mr = s.main_row();
        if mr + 1 >= num_rows {
            return None;
        }
        s.addr.get(&(mr + 1)).filter(|x| !x.is_empty()).cloned()
    };

    let natural_w = |s: &Server| -> f64 {
        let dl1 = link1_text(s).map_or(0.0, |t| text_width(&t, FONT_ARROW, false));
        let dl2 = link2_text(s).map_or(0.0, |t| text_width(&t, FONT_ARROW, false));
        let wb = box_w(s) + 2.0 * MARGIN_BOX_W;
        (dl1 + 2.0 * MARGIN_AD).max(wb).max(dl2 + 2.0 * MARGIN_AD)
    };
    let natural_h = |s: &Server| -> f64 {
        let h1 = if link1_text(s).is_some() {
            text_height(FONT_ARROW)
        } else {
            0.0
        };
        let h2 = if link2_text(s).is_some() {
            text_height(FONT_ARROW)
        } else {
            0.0
        };
        h1 + 2.0 * MARGIN_AD + 2.0 * MAGIC + box_h + h2 + 2.0 * MARGIN_AD
    };

    // ─── Column widths and row heights ─────────────────────────────────────
    let col_width: Vec<f64> = (0..num_cols)
        .map(|j| {
            servers
                .iter()
                .filter(|s| s.col == j)
                .map(natural_w)
                .fold(0.0_f64, f64::max)
        })
        .collect();
    let row_height: Vec<f64> = (0..num_rows)
        .map(|i| {
            servers
                .iter()
                .filter(|s| s.main_row() == i)
                .map(natural_h)
                .fold(50.0_f64, f64::max)
        })
        .collect();

    let mut col_x = vec![0.0_f64; num_cols + 1];
    for j in 0..num_cols {
        col_x[j + 1] = col_x[j] + col_width[j];
    }
    let mut row_y = vec![0.0_f64; num_rows + 1];
    for i in 0..num_rows {
        row_y[i + 1] = row_y[i] + row_height[i];
    }
    let grid_w = col_x[num_cols].max(MINIMUM_WIDTH);
    let grid_h = row_y[num_rows];

    // ─── Network label blocks (right-aligned, name over optional address) ───
    let mut delta_x = 0.0_f64;
    let mut delta_y = 0.0_f64;
    for (i, net) in diagram.networks.iter().enumerate() {
        let mut block_w = text_width(&net.name, FONT_SERVER, false);
        let mut lines = 1.0;
        if let Some(a) = &net.address {
            block_w = block_w.max(text_width(a, FONT_SERVER, false));
            lines = 2.0;
        }
        if i == 0 {
            delta_y = (lines * text_height(FONT_SERVER) - NETWORK_THIN) / 2.0;
        }
        delta_x = delta_x.max(block_w);
    }
    delta_x += LABEL_DELTAX_PAD;

    let gx = OUTER_MARGIN + delta_x;
    let gy = OUTER_MARGIN + delta_y;

    // ─── Network x extents (xmin/xmax) per row ──────────────────────────────
    let mut net_xmin = vec![0.0_f64; num_rows];
    let mut net_xmax = vec![0.0_f64; num_rows];
    for i in 0..num_rows {
        let mut xmin = -1.0_f64;
        let mut xmax = 0.0_f64;
        for j in 0..num_cols {
            let linked = servers.iter().any(|s| s.col == j && s.rows.contains(&i));
            if linked && xmin < 0.0 {
                xmin = col_x[j];
            }
            if linked {
                xmax = col_x[j + 1];
            }
        }
        net_xmin[i] = if xmin < 0.0 { 0.0 } else { xmin };
        net_xmax[i] = xmax;
    }

    // magicDelta: +2 on even stages, -2 on odd stages.
    let magic_delta = |row: usize| if row.is_multiple_of(2) { 2.0 } else { -2.0 };

    let content_w = gx + grid_w + OUTER_MARGIN + 1.0;
    let content_h = gy + grid_h + OUTER_MARGIN + 1.0;
    let canvas_w = content_w.ceil() as i64;
    let canvas_h = content_h.ceil() as i64;

    // ─── Emit SVG ───────────────────────────────────────────────────────────
    let mut s = String::new();
    s.push_str(&format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" "#,
            r#"xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" "#,
            r#"data-diagram-type="NWDIAG" height="{h}px" preserveAspectRatio="none" "#,
            r#"style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" "#,
            r#"viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
        ),
        w = canvas_w,
        h = canvas_h,
    ));
    s.push_str("<?plantuml ?><defs/><g>");

    // 1. Network label blocks.
    for (i, net) in diagram.networks.iter().enumerate() {
        let block_top = OUTER_MARGIN + row_y[i];
        let right = OUTER_MARGIN + (delta_x - LABEL_DELTAX_PAD);
        let name_x = right - text_width(&net.name, FONT_SERVER, false);
        emit_text(
            &mut s,
            name_x,
            block_top + ascent(FONT_SERVER),
            FONT_SERVER,
            &net.name,
        );
        if let Some(addr) = &net.address {
            let ax = right - text_width(addr, FONT_SERVER, false);
            let ay = block_top + text_height(FONT_SERVER) + ascent(FONT_SERVER);
            emit_text(&mut s, ax, ay, FONT_SERVER, addr);
        }
    }

    // 2. Network bars.
    for (i, net) in diagram.networks.iter().enumerate() {
        let bar_x = gx + net_xmin[i];
        let bar_w = (net_xmax[i] - net_xmin[i]).max(MINIMUM_WIDTH);
        let bar_y = gy + row_y[i];
        let fill = net.color.as_deref().unwrap_or(NET_FILL_DEFAULT);
        s.push_str(&format!(
            r#"<rect fill="{fill}" height="5" style="stroke:{LINE_COLOR};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
            w = fmt_coord(bar_w),
            x = fmt_coord(bar_x),
            y = fmt_coord(bar_y),
        ));
    }

    // 3. Links: connectors + address labels (row-major, then column).
    for i in 0..num_rows {
        let line_height = row_height[i];
        for j in 0..num_cols {
            let Some(sv) = servers.iter().find(|s| s.col == j && s.main_row() == i) else {
                continue;
            };
            let cw = col_width[j];
            let x_link_pos = cw / 2.0;
            let ynet1 = gy + row_y[i];
            let xstart = gx + col_x[j];
            let y_middle = line_height / 2.0;
            let alpha = y_middle - box_h / 2.0;
            let pos_link1 = alpha / 2.0;

            // Main connector.
            let cx = xstart + x_link_pos + magic_delta(i);
            connector(&mut s, cx, ynet1 + NETWORK_THIN, ynet1 + alpha);

            if let Some(t) = link1_text(sv) {
                let lx =
                    xstart + x_link_pos + magic_delta(i) - text_width(&t, FONT_ARROW, false) / 2.0;
                emit_addr(&mut s, lx, ynet1 + pos_link1, &t);
            }

            // Connectors to further networks.
            let seven = 9.0;
            let extra: Vec<usize> = sv.rows.iter().copied().filter(|&r| r != i).collect();
            let conns_size = extra.len() + 1;
            let mut x = x_link_pos - (conns_size as f64 - 2.0) * seven / 2.0;
            let mut first = true;
            for &r in &extra {
                let ynet2 = gy + row_y[r];
                let cx2 = xstart + x - magic_delta(r);
                connector(&mut s, cx2, ynet1 + y_middle + box_h / 2.0, ynet2);
                if let Some(t) = sv.addr.get(&r).filter(|x| !x.is_empty()) {
                    let tw = text_width(t, FONT_ARROW, false);
                    let xtext = if first && conns_size > 2 {
                        x - tw / 2.0
                    } else {
                        x
                    };
                    let lx = xstart + xtext - magic_delta(r) - tw / 2.0;
                    emit_addr(&mut s, lx, ynet2 - alpha / 2.0, t);
                }
                x += seven;
                first = false;
            }
        }
    }

    // 4. Host boxes.
    for i in 0..num_rows {
        let line_height = row_height[i];
        for j in 0..num_cols {
            let Some(sv) = servers.iter().find(|s| s.col == j && s.main_row() == i) else {
                continue;
            };
            let cw = col_width[j];
            let bw = box_w(sv);
            let bx = gx + col_x[j] + cw / 2.0 - bw / 2.0;
            let by = gy + row_y[i] + line_height / 2.0 - box_h / 2.0;
            s.push_str(&format!(
                r#"<rect fill="{HOST_FILL}" height="{h}" style="stroke:{LINE_COLOR};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
                h = fmt_coord(box_h),
                w = fmt_coord(bw),
                x = fmt_coord(bx),
                y = fmt_coord(by),
            ));
            let tw = text_width(&sv.label, FONT_SERVER, false);
            let lx = bx + (bw - tw) / 2.0;
            let ly = by + (box_h - text_height(FONT_SERVER)) / 2.0 + ascent(FONT_SERVER);
            emit_text(&mut s, lx, ly, FONT_SERVER, &sv.label);
        }
    }

    s.push_str("</g></svg>");
    s
}

/// Vertical connector line from `(x, y1)` to `(x, y2)`.
fn connector(buf: &mut String, x: f64, y1: f64, y2: f64) {
    buf.push_str(&format!(
        r#"<path d="M{x},{y1} L{x},{y2}" fill="none" style="stroke:{LINE_COLOR};stroke-width:1;"/>"#,
        x = fmt_coord(x),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    ));
}

/// A font-12 label `<text>` with `y` as the top of the line (baseline added).
fn emit_text(buf: &mut String, x: f64, baseline_y: f64, size: f64, content: &str) {
    let tl = text_width(content, size, false);
    buf.push_str(&format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="{sz}" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{c}</text>"##,
        sz = fmt_num(size),
        tl = fmt_coord(tl),
        x = fmt_coord(x),
        y = fmt_coord(baseline_y),
        c = escape(content),
    ));
}

/// A font-11 address label; `center_y` is the vertical centre of the single
/// text line (PlantUML draws link blocks centred), the baseline is derived.
fn emit_addr(buf: &mut String, x: f64, center_y: f64, content: &str) {
    let tl = text_width(content, FONT_ARROW, false);
    let baseline = center_y - text_height(FONT_ARROW) / 2.0 + ascent(FONT_ARROW);
    buf.push_str(&format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="11" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{c}</text>"##,
        tl = fmt_coord(tl),
        x = fmt_coord(x),
        y = fmt_coord(baseline),
        c = escape(content),
    ));
}

fn fmt_num(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        fmt_coord(v)
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
