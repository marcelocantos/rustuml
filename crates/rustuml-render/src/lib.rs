// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! SVG rendering for parsed PlantUML diagrams.

pub mod activity;
pub mod class;
pub mod metrics;
pub mod png;
pub mod sequence;
pub mod state;
pub mod style;
pub mod svg;

use rustuml_parser::diagram::Diagram;
use style::Theme;

/// Render a parsed diagram to SVG using the default theme.
pub fn render_svg(diagram: &Diagram) -> String {
    render_svg_with_theme(diagram, &Theme::default())
}

/// Render a parsed diagram to SVG with a specific theme.
pub fn render_svg_with_theme(diagram: &Diagram, theme: &Theme) -> String {
    match diagram {
        Diagram::Sequence(seq) => sequence::render(seq, theme),
        Diagram::Class(cls) => class::render(cls, theme),
        Diagram::State(st) => state::render(st, theme),
        Diagram::Activity(act) => activity::render(act, theme),
        _ => format!(
            "<!-- rendering not yet implemented for {:?} -->",
            std::mem::discriminant(diagram)
        ),
    }
}
