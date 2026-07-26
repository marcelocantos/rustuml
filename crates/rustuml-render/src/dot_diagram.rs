// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Graphviz DOT diagram SVG renderer.

use rustuml_parser::diagram::dot::DotDiagram;

use crate::style::Theme;

/// Render PlantUML's suppression notice for a DOT diagram.
///
/// PlantUML deliberately disabled SVG `@startdot` rendering in
/// `PSystemDot.exportDiagramNow`; every DOT body produces this fixed link to
/// the tracking issue. Keep the parsed model so other formats can eventually
/// follow Java's Graphviz path without making SVG depend on an external tool.
pub fn render(_diagram: &DotDiagram, _theme: &Theme) -> String {
    concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<svg xmlns=\"http://www.w3.org/2000/svg\" ",
        "xmlns:xlink=\"http://www.w3.org/1999/xlink\">\n",
        "<a xlink:href=\"https://github.com/plantuml/plantuml/issues/2495\">\n",
        "<text x=\"10\" y=\"30\" font-family=\"sans-serif\" font-size=\"14\" ",
        "fill=\"blue\" text-decoration=\"underline\">",
        "This feature has been suppressed</text>\n",
        "</a>\n",
        "</svg>"
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rustuml_parser::diagram::{DiagramMeta, dot::DotEdge};

    use super::*;

    fn diagram(edges: &[(&str, &str)]) -> DotDiagram {
        DotDiagram {
            meta: DiagramMeta::default(),
            directed: true,
            name: "Perturbed".to_string(),
            attrs: HashMap::new(),
            node_defaults: HashMap::new(),
            edge_defaults: HashMap::new(),
            nodes: Vec::new(),
            edges: edges
                .iter()
                .map(|(from, to)| DotEdge {
                    from: (*from).to_string(),
                    to: (*to).to_string(),
                    attrs: HashMap::new(),
                })
                .collect(),
            clusters: Vec::new(),
        }
    }

    #[test]
    fn svg_suppression_is_independent_of_dot_body() {
        let short = render(&diagram(&[("renamed_a", "renamed_b")]), &Theme::default());
        let deeper = render(
            &diagram(&[
                ("different_root", "middle"),
                ("middle", "leaf"),
                ("different_root", "leaf"),
            ]),
            &Theme::default(),
        );

        assert_eq!(short, deeper);
        assert!(short.contains("plantuml/issues/2495"));
        assert!(short.contains("This feature has been suppressed"));
        assert!(!short.contains("renamed_a"));
    }
}
