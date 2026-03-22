// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! SVG rendering for parsed PlantUML diagrams.

pub mod activity;
pub mod class;
pub mod metrics;
pub mod sequence;
pub mod state;
pub mod style;
pub mod svg;

use rustuml_parser::diagram::Diagram;

/// Render a parsed diagram to SVG.
pub fn render_svg(diagram: &Diagram) -> String {
    match diagram {
        Diagram::Sequence(seq) => sequence::render(seq),
        Diagram::Class(cls) => class::render(cls),
        Diagram::State(st) => state::render(st),
        Diagram::Activity(act) => activity::render(act),
        _ => format!(
            "<!-- rendering not yet implemented for {:?} -->",
            std::mem::discriminant(diagram)
        ),
    }
}
