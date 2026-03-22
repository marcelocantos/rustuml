// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.

use rustuml_parser::diagram::component::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const COMPONENT_MIN_W: f64 = 100.0;
const COMPONENT_H: f64 = 40.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 12.0;

pub fn render(diagram: &ComponentDiagram, theme: &Theme) -> String {
    if diagram.components.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n = diagram.components.len();
    let cols = (n as f64).sqrt().ceil() as usize;

    let widths: Vec<f64> = diagram
        .components
        .iter()
        .map(|c| (metrics::text_width(&c.label, FONT_SIZE) + PADDING * 2.0).max(COMPONENT_MIN_W))
        .collect();

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, w) in widths.iter().enumerate() {
            cw[i % cols] = cw[i % cols].max(*w);
        }
        cw
    };
    let rows = n.div_ceil(cols);
    let total_w = MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64;
    let total_h = MARGIN * 2.0 + rows as f64 * (COMPONENT_H + GAP);

    let mut svg = SvgBuilder::new(total_w, total_h);
    let cs = &theme.class;

    let mut positions = Vec::new();
    for (i, comp) in diagram.components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = MARGIN + row as f64 * (COMPONENT_H + GAP);
        let w = col_w[col];

        svg.rect(x, y, w, COMPONENT_H, &cs.class_background, &cs.border_color);
        svg.rect(
            x - 5.0,
            y + 8.0,
            10.0,
            6.0,
            &cs.class_background,
            &cs.border_color,
        );
        svg.rect(
            x - 5.0,
            y + 20.0,
            10.0,
            6.0,
            &cs.class_background,
            &cs.border_color,
        );
        svg.text(
            x + w / 2.0,
            y + COMPONENT_H / 2.0 + 5.0,
            &comp.label,
            "middle",
            FONT_SIZE,
        );
        positions.push((x, y, w));
    }

    for conn in &diagram.connections {
        let fi = diagram.components.iter().position(|c| c.id == conn.from);
        let ti = diagram.components.iter().position(|c| c.id == conn.to);
        if let (Some(fi), Some(ti)) = (fi, ti) {
            let (fx, fy, fw) = positions[fi];
            let (tx, ty, tw) = positions[ti];
            svg.line_segment(
                fx + fw / 2.0,
                fy + COMPONENT_H,
                tx + tw / 2.0,
                ty,
                &cs.border_color,
                conn.dashed,
            );
            svg.arrow_head(tx + tw / 2.0, ty, 90.0);
            if let Some(label) = &conn.label {
                let mx = (fx + fw / 2.0 + tx + tw / 2.0) / 2.0;
                let my = (fy + COMPONENT_H + ty) / 2.0;
                svg.text(mx, my - 4.0, label, "middle", SMALL_FONT);
            }
        }
    }

    svg.finalize()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\ncomponent \"Web\" as WS\ncomponent \"DB\" as DB\nWS --> DB : query\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Web"));
        assert!(svg.contains("DB"));
        assert!(svg.contains("query"));
    }
}
