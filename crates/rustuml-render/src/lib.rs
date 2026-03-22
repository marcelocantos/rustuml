// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! SVG rendering for parsed PlantUML diagrams.

pub mod sequence;
pub mod svg;

use rustuml_parser::diagram::Diagram;

/// Render a parsed diagram to SVG.
pub fn render_svg(diagram: &Diagram) -> String {
    match diagram {
        Diagram::Sequence(seq) => sequence::render(seq),
        _ => format!(
            "<!-- unsupported diagram type: {:?} -->",
            std::mem::discriminant(diagram)
        ),
    }
}
