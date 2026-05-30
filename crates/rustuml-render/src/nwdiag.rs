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

    // ─── Column assignment via tetris packing on the bar's STAGE span ──────
    // A server's vertical bar does not span every network row it touches.
    // Following Java NServer.connectTo / NBar.addStage: the bar starts at the
    // main (first-connected) network's stage, and each *additional* connection
    // to a network at row r extends the bar to that network's `up` stage
    // (= row r-1), never to r itself. The tetris packer burns this stage span,
    // which lets a downstream-only host reuse a column occupied by an
    // upstream-only host on a different network row.
    let mut burned: BTreeSet<(usize, usize)> = BTreeSet::new();
    for s in &mut servers {
        let main = s.rows[0];
        let mut start = main;
        let mut end = main;
        for &r in &s.rows[1..] {
            let up = r.saturating_sub(1);
            start = start.min(up);
            end = end.max(up);
        }
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

            // A vertical connector that passes *through* an intermediate
            // network bar (its x lies strictly within that bar's extent) gets
            // a small rounded jog where it crosses. Mirror Java's `skip` set:
            // the y of every network bar whose [xmin,xmax] contains x_middle.
            let x_middle_abs = xstart + x_link_pos;
            let mut skip: Vec<f64> = Vec::new();
            for k in 0..num_rows {
                let xmin = gx + net_xmin[k];
                let xmax = gx + net_xmax[k];
                if x_middle_abs > xmin && x_middle_abs < xmax {
                    skip.push(gy + row_y[k]);
                }
            }
            skip.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Main connector.
            let cx = xstart + x_link_pos + magic_delta(i);
            vertical_line(&mut s, cx, ynet1 + NETWORK_THIN, ynet1 + alpha, &skip);

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
                vertical_line(&mut s, cx2, ynet1 + y_middle + box_h / 2.0, ynet2, &skip);
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

/// Vertical connector line from `(x, y1)` to `(x, y2)`, jogging around each
/// network bar y in `skip` that the line passes through. Mirrors Java
/// `nwdiag.VerticalLine.drawU`: at a crossed bar at `step`, the line stops at
/// `step-3`, arcs (r=4) to `step+9`, then resumes. Each emitted path segment is
/// a separate `<path>` element, exactly as Java's `ug.draw(path)` calls do.
fn vertical_line(buf: &mut String, x: f64, ya: f64, yb: f64, skip: &[f64]) {
    let y1 = ya.min(yb);
    let y2 = ya.max(yb);

    let emit = |buf: &mut String, d: &str| {
        buf.push_str(&format!(
            r#"<path d="{d}" fill="none" style="stroke:{LINE_COLOR};stroke-width:1;"/>"#,
        ));
    };

    let mut drawn = false;
    // `pending` is true once `seg` holds path commands past its leading moveTo
    // that have not yet been flushed via `emit`.
    let mut pending = false;
    let mut seg = format!("M{},{}", fmt_coord(x), fmt_coord(y1));
    for &step in skip {
        if step < y1 {
            continue;
        }
        drawn = true;
        if step == y2 {
            seg.push_str(&format!(" L{},{}", fmt_coord(x), fmt_coord(y2)));
        } else {
            let stop = y2.min(step - 3.0);
            seg.push_str(&format!(" L{},{}", fmt_coord(x), fmt_coord(stop)));
            pending = true;
            if y2 > step {
                seg.push_str(&format!(
                    " A4,4 0 0 1 {},{}",
                    fmt_coord(x),
                    fmt_coord(step + 9.0)
                ));
                continue;
            }
        }
        emit(buf, &seg);
        pending = false;
        let current = step + 9.0;
        seg = format!("M{},{}", fmt_coord(x), fmt_coord(current));
        if current >= y2 {
            break;
        }
    }
    if !drawn {
        // No bars crossed: a single straight segment.
        seg.push_str(&format!(" L{},{}", fmt_coord(x), fmt_coord(y2)));
        emit(buf, &seg);
    } else if pending {
        // Path ended mid-jog (arc was the last command): finish it down to y2.
        seg.push_str(&format!(" L{},{}", fmt_coord(x), fmt_coord(y2)));
        emit(buf, &seg);
    }
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
