// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! SVG rendering for parsed PlantUML diagrams.

pub mod activity;
pub mod ascii;
pub mod class;
pub mod component;
pub mod creole;
pub mod deployment;
pub mod gantt;
pub mod metrics;
pub mod mindmap;
pub mod pdf;
pub mod png;
pub mod sequence;
pub mod skinparam;
pub mod state;
pub mod style;
pub mod svg;
pub mod timing;
pub mod usecase;
pub mod wbs;

use rustuml_parser::diagram::Diagram;
use style::Theme;

/// Render a parsed diagram to ASCII art text.
///
/// Only sequence diagrams are fully supported; other diagram types return an
/// explanatory placeholder string.
pub fn render_ascii(diagram: &Diagram) -> String {
    match diagram {
        Diagram::Sequence(seq) => ascii::render_ascii(seq),
        _ => String::from("# ASCII rendering is only supported for sequence diagrams.\n"),
    }
}

/// Render a parsed diagram to SVG using the default theme.
pub fn render_svg(diagram: &Diagram) -> String {
    render_svg_with_theme(diagram, &Theme::default())
}

/// Render a parsed diagram to SVG with a specific theme.
/// Skinparams from the diagram's metadata override the theme.
pub fn render_svg_with_theme(diagram: &Diagram, theme: &Theme) -> String {
    // Apply inline skinparam overrides.
    let meta_params = match diagram {
        Diagram::Sequence(s) => &s.meta.skinparams,
        Diagram::Class(c) => &c.meta.skinparams,
        Diagram::State(s) => &s.meta.skinparams,
        Diagram::Activity(a) => &a.meta.skinparams,
        _ => return render_with_theme(diagram, theme),
    };
    let effective_theme = if meta_params.is_empty() {
        theme.clone()
    } else {
        skinparam::apply_skinparams(theme, meta_params)
    };
    render_with_theme(diagram, &effective_theme)
}

fn render_with_theme(diagram: &Diagram, theme: &Theme) -> String {
    match diagram {
        Diagram::Sequence(seq) => sequence::render(seq, theme),
        Diagram::Class(cls) => class::render(cls, theme),
        Diagram::State(st) => state::render(st, theme),
        Diagram::Activity(act) => activity::render(act, theme),
        Diagram::Component(comp) => component::render(comp, theme),
        Diagram::UseCase(uc) => usecase::render(uc, theme),
        Diagram::Deployment(dep) => deployment::render(dep, theme),
        Diagram::MindMap(mm) => mindmap::render(mm, theme),
        Diagram::Gantt(g) => gantt::render(g, theme),
        Diagram::Timing(td) => timing::render(td, theme),
        Diagram::Wbs(w) => wbs::render(w, theme),
    }
}
