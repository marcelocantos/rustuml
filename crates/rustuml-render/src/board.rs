// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) SVG renderer.
//!
//! PlantUML renders `@startboard` as a two-row grid of identical 150×70
//! shadowed boxes — a degenerate work-breakdown layout, *not* the columns-
//! of-cards model the syntax suggests. The placement rule (derived from the
//! oracle output) is:
//!
//! * the root box occupies column 0, row 0;
//! * a `*` card advances to the next column on row 0;
//! * a `+` column header sits on row 1, reusing the current column unless the
//!   previously placed item was also a `+` (an empty column), in which case
//!   it advances first.
//!
//! Cards carry a small bullet ellipse; the root and column headers do not.
//! PlantUML emits the root and every card box twice (a known quirk), so we
//! reproduce that to keep the element stream identical.

use std::fmt::Write;

use rustuml_parser::diagram::board::{BoardDiagram, BoardItemKind};

use crate::filter_registry;
use crate::layout_oracle::OracleLayout;
use crate::plantuml_metrics::fmt_coord;
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// ── Layout constants (in oracle pixels) ─────────────────────────────────────

const BOX_W: f64 = 150.0;
const BOX_H: f64 = 70.0;
const COL_PITCH: f64 = 170.0; // BOX_W + 20 gap
const ROW_PITCH: f64 = 90.0; // BOX_H + 20 gap
const ORIGIN_X: f64 = 10.0;
const ORIGIN_Y: f64 = 10.0;
const FONT_SIZE: u32 = 14;

// Text/bullet offsets relative to a box's top-left corner.
const TEXT_DX_PLAIN: f64 = 3.0; // root + column headers
const TEXT_DX_CARD: f64 = 15.0; // cards (after the bullet)
const TEXT_DY: f64 = 16.5352;
const BULLET_DX: f64 = 8.5;
const BULLET_DY: f64 = 11.9883;
const BULLET_R: &str = "2.5";

// Canvas sizing.
const CANVAS_H: i64 = 193; // always two rows tall
const SEP_Y: f64 = 90.0; // dashed separator between the rows
const SEP_INSET_RIGHT: f64 = 22.0; // line x2 = width - SEP_INSET_RIGHT

// Colours.
const BOX_FILL: &str = "#C0C0C0";
const BOX_STROKE_STYLE: &str = "stroke:#000000;stroke-width:1;";
const TEXT_FILL: &str = "#000000";
const BULLET_FILL: &str = "#000000";

// ── Public entry points ─────────────────────────────────────────────────────

pub fn render_with_oracle(
    diagram: &BoardDiagram,
    theme: &Theme,
    _oracle: Option<&OracleLayout>,
) -> String {
    render(diagram, theme)
}

pub fn render(diagram: &BoardDiagram, _theme: &Theme) -> String {
    let source = diagram.meta.source.as_deref().unwrap_or("");
    let shadow_id = filter_registry::shadow_id_for(source);

    // Assign a (column, row) grid cell to each item in document order.
    let mut cells: Vec<(usize, usize)> = Vec::with_capacity(diagram.items.len());
    let mut cur_col: usize = 0;
    let mut prev_was_column = false;
    for (idx, item) in diagram.items.iter().enumerate() {
        match item.kind {
            BoardItemKind::Root => {
                cur_col = 0;
                cells.push((0, 0));
                prev_was_column = false;
            }
            BoardItemKind::Card => {
                // The very first item is always treated as the root box, so a
                // card can only appear after at least one placed item.
                if idx != 0 {
                    cur_col += 1;
                }
                cells.push((cur_col, 0));
                prev_was_column = false;
            }
            BoardItemKind::Column => {
                if prev_was_column {
                    cur_col += 1;
                }
                cells.push((cur_col, 1));
                prev_was_column = true;
            }
        }
    }

    let n_cols = cells.iter().map(|(c, _)| c + 1).max().unwrap_or(1);
    // The dashed separator (and the slot grid) end at the last column's slot
    // boundary, regardless of any text overflow.
    let slot_right = ORIGIN_X + n_cols as f64 * COL_PITCH;

    // Emit the box stream first, tracking the rightmost text extent. A long
    // column label or card can spill past its slot; PlantUML widens the canvas
    // (but not the dashed line) to accommodate it.
    let mut body = String::with_capacity(2048);
    let mut max_text_right = 0.0_f64;
    for (item, &(col, row)) in diagram.items.iter().zip(&cells) {
        let bx = ORIGIN_X + col as f64 * COL_PITCH;
        let by = ORIGIN_Y + row as f64 * ROW_PITCH;
        let is_card = item.kind == BoardItemKind::Card;
        // PlantUML emits the root and every card box twice; column headers once.
        let repeats = if matches!(item.kind, BoardItemKind::Column) {
            1
        } else {
            2
        };
        for _ in 0..repeats {
            let text_right = emit_box(&mut body, bx, by, &shadow_id, is_card, &item.label);
            max_text_right = max_text_right.max(text_right);
        }
    }

    // Width spans whichever is wider: the slot grid or the rightmost text,
    // each plus the constant right margin.
    let width = (slot_right + SEP_INSET_RIGHT).max(max_text_right.floor() + SEP_INSET_RIGHT);
    let width_i = width as i64;

    let mut svg = String::with_capacity(body.len() + 1024);
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="BOARD" height="{CANVAS_H}px" preserveAspectRatio="none" style="width:{width_i}px;height:{CANVAS_H}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {width_i} {CANVAS_H}" width="{width_i}px" zoomAndPan="magnify">"#
    )
    .unwrap();
    svg.push_str("<?plantuml ?>");
    write!(
        svg,
        "<defs>{}</defs>",
        filter_registry::shadow_filter_def(&shadow_id)
    )
    .unwrap();
    svg.push_str("<g>");
    svg.push_str(&body);

    // Dashed separator between the two rows.
    write!(
        svg,
        r#"<line style="stroke:#000000;stroke-width:0.5;stroke-dasharray:5,5;" x1="{ORIGIN_X}" x2="{slot_right}" y1="{SEP_Y}" y2="{SEP_Y}"/>"#,
        slot_right = fmt_coord(slot_right),
    )
    .unwrap();

    svg.push_str("</g></svg>");
    svg
}

/// Emit one box (rectangle, optional bullet ellipse for cards, and label) and
/// return the rightmost x reached by its text.
fn emit_box(
    svg: &mut String,
    bx: f64,
    by: f64,
    shadow_id: &str,
    is_card: bool,
    label: &str,
) -> f64 {
    write!(
        svg,
        r#"<rect fill="{BOX_FILL}" filter="url(#{shadow_id})" height="{BOX_H}" style="{BOX_STROKE_STYLE}" width="{BOX_W}" x="{x}" y="{y}"/>"#,
        x = fmt_coord(bx),
        y = fmt_coord(by),
    )
    .unwrap();
    let text_dx = if is_card {
        write!(
            svg,
            r#"<ellipse cx="{cx}" cy="{cy}" fill="{BULLET_FILL}" rx="{BULLET_R}" ry="{BULLET_R}"/>"#,
            cx = fmt_coord(bx + BULLET_DX),
            cy = fmt_coord(by + BULLET_DY),
        )
        .unwrap();
        TEXT_DX_CARD
    } else {
        TEXT_DX_PLAIN
    };
    let text_x = bx + text_dx;
    let text_w = text_render::emit_text(
        svg,
        label,
        &TextBase {
            x: text_x,
            y: by + TEXT_DY,
            font_size: FONT_SIZE,
            font_family: "sans-serif",
            fill: TEXT_FILL,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: true,
        },
    );
    text_x + text_w
}
