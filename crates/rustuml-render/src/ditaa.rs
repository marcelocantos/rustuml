// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Ditaa (ASCII art → diagram) SVG renderer.

use rustuml_parser::diagram::ditaa::*;

use crate::style::Theme;
use crate::svg::SvgBuilder;

/// Pixels per grid character.
const CELL_W: f64 = 9.0;
const CELL_H: f64 = 16.0;
const MARGIN: f64 = 10.0;
const FONT_SIZE: f64 = 12.0;
const BOX_FILL: &str = "#FEFECE";
const BOX_STROKE: &str = "#A0A0A0";
const LINE_STROKE: &str = "#000000";
const ARROW_SIZE: f64 = 6.0;
const CORNER_RADIUS: f64 = 8.0;

pub fn render(diagram: &DitaaDiagram, _theme: &Theme) -> String {
    if diagram.shapes.is_empty() && diagram.connections.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"60\"></svg>\n"
            .to_string();
    }

    let total_w = MARGIN * 2.0 + diagram.grid_width as f64 * CELL_W;
    let total_h = MARGIN * 2.0 + diagram.grid_height as f64 * CELL_H;

    let mut svg = SvgBuilder::new(total_w, total_h);

    // Draw shapes.
    for shape in &diagram.shapes {
        draw_shape(&mut svg, shape);
    }

    // Draw connections.
    for conn in &diagram.connections {
        draw_connection(&mut svg, conn);
    }

    svg.finalize()
}

fn grid_x(col: usize) -> f64 {
    MARGIN + col as f64 * CELL_W
}

fn grid_y(row: usize) -> f64 {
    MARGIN + row as f64 * CELL_H
}

fn draw_shape(svg: &mut SvgBuilder, shape: &DitaaShape) {
    let x = grid_x(shape.col);
    let y = grid_y(shape.row);
    let w = shape.width as f64 * CELL_W;
    let h = shape.height as f64 * CELL_H;

    let fill = shape
        .color
        .as_ref()
        .map(|c| c.fill())
        .unwrap_or(BOX_FILL);

    match shape.kind {
        DitaaShapeKind::Box | DitaaShapeKind::Storage => {
            svg.rect(x, y, w, h, fill, BOX_STROKE);
        }
        DitaaShapeKind::RoundedBox => {
            svg.rounded_rect(x, y, w, h, CORNER_RADIUS, fill, BOX_STROKE);
        }
        DitaaShapeKind::Document => {
            // Draw a box with a wavy bottom edge using a path.
            let x2 = x + w;
            let y2 = y + h;
            let wave_h = 6.0;
            let mid = x + w / 2.0;
            let path = format!(
                r#"<path d="M {x} {y} L {x2} {y} L {x2} {wave_y} Q {mid3} {wave_lo} {mid} {wave_y} Q {mid1} {wave_hi} {x} {wave_y} Z" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
                wave_y = y2 - wave_h,
                wave_lo = y2 + wave_h * 0.5,
                wave_hi = y2 - wave_h * 2.5,
                mid3 = mid + w / 4.0,
                mid1 = mid - w / 4.0,
                stroke = BOX_STROKE,
            );
            svg.raw(&path);
        }
    }

    // Draw text centred in the shape.
    if let Some(text) = &shape.text {
        let lines: Vec<&str> = text.lines().collect();
        let total_text_h = lines.len() as f64 * (FONT_SIZE + 2.0);
        let base_y = y + (h - total_text_h) / 2.0 + FONT_SIZE;

        for (i, line) in lines.iter().enumerate() {
            let ty = base_y + i as f64 * (FONT_SIZE + 2.0);
            svg.text(x + w / 2.0, ty, line, "middle", FONT_SIZE);
        }
    }
}

fn draw_connection(svg: &mut SvgBuilder, conn: &DitaaConnection) {
    for seg in &conn.segments {
        let x1 = grid_x(seg.start_col) + CELL_W / 2.0;
        let y1 = grid_y(seg.start_row) + CELL_H / 2.0;
        let x2 = grid_x(seg.end_col) + CELL_W / 2.0;
        let y2 = grid_y(seg.end_row) + CELL_H / 2.0;

        svg.line_segment(x1, y1, x2, y2, LINE_STROKE, conn.dashed);
    }

    // Draw arrow heads.
    if let Some(last_seg) = conn.segments.last() {
        if conn.end_arrow {
            let x = grid_x(last_seg.end_col) + CELL_W / 2.0;
            let y = grid_y(last_seg.end_row) + CELL_H / 2.0;
            let direction = if last_seg.end_col > last_seg.start_col {
                0.0 // right
            } else if last_seg.end_col < last_seg.start_col {
                180.0
            } else if last_seg.end_row > last_seg.start_row {
                90.0 // down
            } else {
                270.0
            };
            draw_arrowhead(svg, x, y, direction);
        }
    }

    if let Some(first_seg) = conn.segments.first() {
        if conn.start_arrow {
            let x = grid_x(first_seg.start_col) + CELL_W / 2.0;
            let y = grid_y(first_seg.start_row) + CELL_H / 2.0;
            let direction = if first_seg.start_col < first_seg.end_col {
                180.0 // pointing left (away from end)
            } else if first_seg.start_col > first_seg.end_col {
                0.0
            } else if first_seg.start_row < first_seg.end_row {
                270.0 // pointing up
            } else {
                90.0
            };
            draw_arrowhead(svg, x, y, direction);
        }
    }
}

fn draw_arrowhead(svg: &mut SvgBuilder, x: f64, y: f64, direction_deg: f64) {
    let angle = direction_deg.to_radians();
    let p1x = x - ARROW_SIZE * (angle - 0.4).cos();
    let p1y = y - ARROW_SIZE * (angle - 0.4).sin();
    let p2x = x - ARROW_SIZE * (angle + 0.4).cos();
    let p2y = y - ARROW_SIZE * (angle + 0.4).sin();
    svg.polygon(&[(x, y), (p1x, p1y), (p2x, p2y)], LINE_STROKE, LINE_STROKE);
}
