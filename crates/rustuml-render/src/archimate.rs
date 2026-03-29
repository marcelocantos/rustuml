// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Archimate diagram SVG renderer.

use rustuml_parser::diagram::archimate::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const ELEM_MIN_W: f64 = 120.0;
const ELEM_H: f64 = 50.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 10.0;
const PADDING: f64 = 12.0;
const CORNER_R: f64 = 8.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
const GROUP_PAD: f64 = 15.0;
const GROUP_HEADER: f64 = 20.0;

pub fn render(diagram: &ArchimateDiagram, theme: &Theme) -> String {
    if diagram.elements.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n = diagram.elements.len();
    let cols = (n as f64).sqrt().ceil() as usize;

    let widths: Vec<f64> = diagram
        .elements
        .iter()
        .map(|e| {
            let label_w = metrics::text_width(&e.label, FONT_SIZE) + PADDING * 2.0;
            let kind_w = metrics::text_width(&format!("\u{00ab}{}\u{00bb}", e.kind), SMALL_FONT)
                + PADDING * 2.0;
            label_w.max(kind_w).max(ELEM_MIN_W)
        })
        .collect();

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, w) in widths.iter().enumerate() {
            cw[i % cols] = cw[i % cols].max(*w);
        }
        cw
    };
    let rows = n.div_ceil(cols);

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let total_w =
        (MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64).max(200.0);
    let total_h = (MARGIN * 2.0 + rows as f64 * (ELEM_H + GAP) + title_h).max(80.0);

    let mut svg = SvgBuilder::new(total_w, total_h);

    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_w / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }

    let cs = &theme.class;

    // Compute element positions first (for group bounding boxes).
    let y_start = title_h + MARGIN;
    let mut positions = Vec::new();
    for (i, _elem) in diagram.elements.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (ELEM_H + GAP);
        let w = col_w[col];
        positions.push((x, y, w));
    }

    // Render groups (labelled rectangles) behind elements.
    for group in &diagram.groups {
        if group.element_ids.is_empty() {
            continue;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for eid in &group.element_ids {
            if let Some(idx) = diagram.elements.iter().position(|e| e.id == *eid) {
                let (ex, ey, ew) = positions[idx];
                min_x = min_x.min(ex);
                min_y = min_y.min(ey);
                max_x = max_x.max(ex + ew);
                max_y = max_y.max(ey + ELEM_H);
            }
        }
        if min_x < f64::MAX {
            let gx = min_x - GROUP_PAD;
            let gy = min_y - GROUP_PAD - GROUP_HEADER;
            let gw = max_x - min_x + GROUP_PAD * 2.0;
            let gh = max_y - min_y + GROUP_PAD * 2.0 + GROUP_HEADER;
            svg.rect(gx, gy, gw, gh, "#EEEEEE", "#888888");
            svg.text(
                gx + 6.0,
                gy + GROUP_HEADER - 4.0,
                &group.label,
                "start",
                FONT_SIZE,
            );
        }
    }

    // Render elements.
    for (i, elem) in diagram.elements.iter().enumerate() {
        let (x, y, w) = positions[i];
        let fill = elem.layer.default_color();

        svg.rounded_rect(x, y, w, ELEM_H, CORNER_R, fill, &cs.border_color);

        svg.text_colored(
            x + w / 2.0,
            y + 16.0,
            &format!("\u{00ab}{}\u{00bb}", elem.kind),
            "middle",
            SMALL_FONT,
            "#666666",
        );

        svg.text(
            x + w / 2.0,
            y + ELEM_H / 2.0 + 10.0,
            &elem.label,
            "middle",
            FONT_SIZE,
        );
    }

    for rel in &diagram.relations {
        let fi = diagram.elements.iter().position(|e| e.id == rel.from);
        let ti = diagram.elements.iter().position(|e| e.id == rel.to);

        let (fi, ti) = match (fi, ti) {
            (Some(f), Some(t)) => (f, t),
            _ => continue,
        };

        let (fx, fy, fw) = positions[fi];
        let (tx, ty, tw) = positions[ti];

        let from_cx = fx + fw / 2.0;
        let from_cy = fy + ELEM_H;
        let to_cx = tx + tw / 2.0;
        let to_cy = ty;

        let dashed = matches!(
            rel.kind,
            ArchimateRelationKind::Realization | ArchimateRelationKind::Influence
        );

        svg.line_segment(from_cx, from_cy, to_cx, to_cy, &cs.border_color, dashed);
        svg.arrow_head(to_cx, to_cy, 90.0);

        if let Some(label) = &rel.label {
            let mx = (from_cx + to_cx) / 2.0;
            let my = (from_cy + to_cy) / 2.0;
            svg.text(mx + 6.0, my - 4.0, label, "start", SMALL_FONT);
        }
    }

    svg.finalize()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\n!include <archimate/Archimate>\nBusiness_Actor(cust, \"Customer\")\nApplication_Component(app, \"App\")\nRel_Serving(app, cust, \"serves\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Customer"), "Customer missing: {svg}");
        assert!(svg.contains("App"), "App missing: {svg}");
        assert!(svg.contains("serves"), "serves missing: {svg}");
    }

    #[test]
    fn layer_colors_differ() {
        let input = "@startuml\n!include <archimate/Archimate>\nBusiness_Actor(a, \"Biz\")\nTechnology_Node(b, \"Tech\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("#FFFFB5"), "Business color missing: {svg}");
        assert!(svg.contains("#C9E7B7"), "Technology color missing: {svg}");
    }

    #[test]
    fn stereotype_labels_rendered() {
        let input = "@startuml\n!include <archimate/Archimate>\nMotivation_Goal(g, \"Reduce Costs\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Goal"), "Kind label missing: {svg}");
        assert!(svg.contains("Reduce Costs"), "Element label missing: {svg}");
    }
}
