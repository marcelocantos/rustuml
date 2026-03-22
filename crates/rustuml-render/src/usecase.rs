// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram SVG renderer.

use rustuml_parser::diagram::usecase::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const UC_RX: f64 = 60.0;
const UC_RY: f64 = 25.0;
const ACTOR_H: f64 = 50.0;
const MARGIN: f64 = 40.0;
const GAP: f64 = 60.0;
const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 10.0;

pub fn render(diagram: &UseCaseDiagram, theme: &Theme) -> String {
    let total_actors = diagram.actors.len();
    let total_uc = diagram.use_cases.len();
    if total_actors == 0 && total_uc == 0 {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let actor_col_w = 80.0;
    let uc_col_w = UC_RX * 2.0 + 40.0;
    let total_w = MARGIN * 2.0 + actor_col_w + GAP + uc_col_w;
    let max_items = total_actors.max(total_uc).max(1);
    let total_h = MARGIN * 2.0 + max_items as f64 * (ACTOR_H + GAP);

    let mut svg = SvgBuilder::new(total_w, total_h);
    let gs = &theme.global;

    // Position actors on the left.
    let actor_x = MARGIN + actor_col_w / 2.0;
    let mut actor_positions = Vec::new();
    for (i, actor) in diagram.actors.iter().enumerate() {
        let y = MARGIN + i as f64 * (ACTOR_H + GAP) + ACTOR_H / 2.0;
        // Stick figure: head circle + body line + arms + legs.
        svg.circle(actor_x, y - 15.0, 8.0, "none", &gs.border_color);
        svg.line_segment(actor_x, y - 7.0, actor_x, y + 10.0, &gs.border_color, false);
        svg.line_segment(
            actor_x - 12.0,
            y,
            actor_x + 12.0,
            y,
            &gs.border_color,
            false,
        );
        svg.line_segment(
            actor_x,
            y + 10.0,
            actor_x - 10.0,
            y + 22.0,
            &gs.border_color,
            false,
        );
        svg.line_segment(
            actor_x,
            y + 10.0,
            actor_x + 10.0,
            y + 22.0,
            &gs.border_color,
            false,
        );
        svg.text(actor_x, y + 35.0, &actor.label, "middle", FONT_SIZE);
        actor_positions.push((actor.id.clone(), actor_x, y));
    }

    // Position use cases on the right.
    let uc_x = MARGIN + actor_col_w + GAP + uc_col_w / 2.0;
    let mut uc_positions = Vec::new();
    for (i, uc) in diagram.use_cases.iter().enumerate() {
        let y = MARGIN + i as f64 * (UC_RY * 2.0 + GAP) + UC_RY;
        // Ellipse for use case.
        let text_w = metrics::text_width(&uc.label, FONT_SIZE);
        let rx = (text_w / 2.0 + 20.0).max(UC_RX);
        svg.open_group("usecase");
        // Approximate ellipse with a rounded rect.
        svg.rounded_rect(
            uc_x - rx,
            y - UC_RY,
            rx * 2.0,
            UC_RY * 2.0,
            UC_RY,
            "#F8F9FA",
            &gs.border_color,
        );
        svg.text(uc_x, y + 4.0, &uc.label, "middle", FONT_SIZE);
        svg.close_group();
        uc_positions.push((uc.id.clone(), uc_x, y));
    }

    // Draw connections.
    for conn in &diagram.connections {
        let from = actor_positions
            .iter()
            .chain(uc_positions.iter())
            .find(|(id, _, _)| *id == conn.from);
        let to = actor_positions
            .iter()
            .chain(uc_positions.iter())
            .find(|(id, _, _)| *id == conn.to);

        if let (Some((_, fx, fy)), Some((_, tx, ty))) = (from, to) {
            svg.line_segment(*fx, *fy, *tx, *ty, &gs.border_color, false);
            if let Some(label) = &conn.label {
                let mx = (fx + tx) / 2.0;
                let my = (fy + ty) / 2.0;
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
        let input = "@startuml\nactor User\nusecase \"Login\" as UC1\nUser --> UC1\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("User"));
        assert!(svg.contains("Login"));
    }
}
