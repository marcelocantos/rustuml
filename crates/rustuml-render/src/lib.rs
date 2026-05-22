// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! SVG rendering for parsed PlantUML diagrams.

pub mod activity;
pub mod archimate;
pub mod ascii;
pub mod ascii_activity;
pub mod ascii_class;
pub mod ascii_state;
pub mod board;
pub mod class;
pub mod component;
pub mod creole;
pub mod deployment;
pub mod ditaa;
pub mod dot_diagram;
pub mod ebnf;
pub mod eps;
pub mod filter_registry;
pub mod font_metrics;
pub mod gantt;
pub mod git_diagram;
pub mod json_diagram;
pub mod layout_oracle;
pub mod math;
pub mod metrics;
pub mod mindmap;
pub mod nwdiag;
pub mod object;
pub mod openiconic;
pub mod pdf;
pub mod plantuml_metrics;
pub mod png;
pub mod regex_diagram;
pub mod salt;
pub mod sequence;
pub mod skinparam;
pub mod sprite;
pub mod state;
pub mod style;
pub mod svg;
pub mod text_render;
pub mod timing;
pub mod usecase;
pub mod wbs;

use layout_oracle::OracleLayout;
use rustuml_parser::diagram::Diagram;
use style::Theme;

/// Render a parsed diagram to ASCII art text.
///
/// Only sequence diagrams are fully supported; other diagram types return an
/// explanatory placeholder string.
pub fn render_ascii(diagram: &Diagram) -> String {
    match diagram {
        Diagram::Sequence(seq) => ascii::render_ascii(seq),
        Diagram::Class(cls) => ascii_class::render_ascii(cls),
        Diagram::State(st) => ascii_state::render_ascii(st),
        Diagram::Activity(act) => ascii_activity::render_ascii(act),
        _ => String::from("# ASCII rendering is not yet supported for this diagram type.\n"),
    }
}

/// Render a parsed diagram to SVG using the default theme.
pub fn render_svg(diagram: &Diagram) -> String {
    render_svg_with_theme(diagram, &Theme::default())
}

/// Render a parsed diagram to SVG with a specific theme.
/// Skinparams from the diagram's metadata override the theme.
pub fn render_svg_with_theme(diagram: &Diagram, theme: &Theme) -> String {
    // Apply inline skinparam overrides from any diagram type.
    let meta_params = &diagram.meta().skinparams;
    let effective_theme = if meta_params.is_empty() {
        theme.clone()
    } else {
        skinparam::apply_skinparams(theme, meta_params)
    };
    render_under_filter_registry(diagram, |d| render_with_theme(d, &effective_theme))
}

/// Render a parsed diagram to SVG with optional oracle layout data.
///
/// When `oracle` is `Some`, layout coordinates are taken from the oracle
/// instead of running the Graphviz layout engine. Currently supported for
/// class diagrams; other types ignore the oracle.
pub fn render_svg_with_oracle(diagram: &Diagram, oracle: Option<&OracleLayout>) -> String {
    let meta_params = &diagram.meta().skinparams;
    let theme = if meta_params.is_empty() {
        Theme::default()
    } else {
        skinparam::apply_skinparams(&Theme::default(), meta_params)
    };
    render_under_filter_registry(diagram, |d| render_with_theme_and_oracle(d, &theme, oracle))
}

/// Install a fresh background-filter registry for the duration of one
/// render call, then splice the collected `<filter>` elements into the
/// final SVG's `<defs>` block. PlantUML-shaped SVGs ship a `<defs/>` self-
/// closing tag when there is nothing to put in there; if any segment
/// requested a filter, we rewrite that occurrence into a `<defs>...</defs>`
/// block holding the filters in insertion order.
fn render_under_filter_registry(
    diagram: &Diagram,
    render: impl FnOnce(&Diagram) -> String,
) -> String {
    let source = diagram.meta().source.as_deref().unwrap_or("");
    let (svg, registry) = filter_registry::with_registry(source, || render(diagram));
    if registry.is_empty() {
        return svg;
    }
    let defs_content = registry.render_defs_content();
    let replacement = format!("<defs>{defs_content}</defs>");
    // PlantUML-shape SVGs emit `<defs/>`; substitute. Other shapes may not
    // contain `<defs/>` at all (no entry point allocated any background),
    // in which case nothing to substitute.
    svg.replacen("<defs/>", &replacement, 1)
}

fn render_with_theme_and_oracle(
    diagram: &Diagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    match diagram {
        Diagram::Class(cls) => class::render_with_oracle(cls, theme, oracle),
        Diagram::State(st) => state::render_with_oracle(st, theme, oracle),
        Diagram::Component(comp) => component::render_with_oracle(comp, theme, oracle),
        Diagram::Deployment(dep) => deployment::render_with_oracle(dep, theme, oracle),
        Diagram::UseCase(uc) => usecase::render_with_oracle(uc, theme, oracle),
        Diagram::Object(obj) => object::render_with_oracle(obj, theme, oracle),
        Diagram::Json(jd) => json_diagram::render_with_oracle(jd, theme, oracle),
        Diagram::Timing(td) => timing::render_with_oracle(td, theme, oracle),
        Diagram::Gantt(g) => gantt::render_with_oracle(g, theme, oracle),
        Diagram::Salt(s) => salt::render_with_oracle(s, theme, oracle),
        Diagram::Nwdiag(nw) => nwdiag::render_with_oracle(nw, theme, oracle),
        Diagram::Archimate(a) => archimate::render_with_oracle(a, theme, oracle),
        Diagram::Regex(r) => regex_diagram::render_with_oracle(r, theme, oracle),
        Diagram::Ebnf(e) => ebnf::render_with_oracle(e, theme, oracle),
        Diagram::Board(b) => board::render_with_oracle(b, theme, oracle),
        Diagram::MindMap(mm) => mindmap::render_with_oracle(mm, theme, oracle),
        Diagram::Wbs(w) => wbs::render_with_oracle(w, theme, oracle),
        Diagram::Activity(act) => activity::render_with_oracle(act, theme, oracle),
        _ => render_with_theme(diagram, theme),
    }
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
        Diagram::Json(jd) => json_diagram::render(jd, theme),
        Diagram::MindMap(mm) => mindmap::render(mm, theme),
        Diagram::Gantt(g) => gantt::render(g, theme),
        Diagram::Git(g) => git_diagram::render(g, theme),
        Diagram::Timing(td) => timing::render(td, theme),
        Diagram::Object(obj) => object::render(obj, theme),
        Diagram::Wbs(w) => wbs::render(w, theme),
        Diagram::Math(m) => math::render(m, theme),
        Diagram::Salt(s) => salt::render(s, theme),
        Diagram::Nwdiag(nw) => nwdiag::render(nw, theme),
        Diagram::Regex(r) => regex_diagram::render(r, theme),
        Diagram::Ditaa(d) => ditaa::render(d, theme),
        Diagram::Dot(d) => dot_diagram::render(d, theme),
        Diagram::Board(b) => board::render(b, theme),
        Diagram::Ebnf(e) => ebnf::render(e, theme),
        Diagram::Archimate(a) => archimate::render(a, theme),
    }
}
