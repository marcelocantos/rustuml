// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Extract layout data from a PlantUML reference SVG.
//!
//! Parses a golden SVG to extract entity positions and edge paths,
//! producing an `OracleLayout` that can be fed to renderers.

use rustuml_render::layout_oracle::{EntityRect, OracleEdgePath, OracleLayout};

/// Extract layout data from a golden SVG string.
///
/// Looks for:
/// - Entity groups (`<g class="entity" data-qualified-name="...">`) containing
///   a `<rect>` with x, y, width, height
/// - Link groups (`<g class="link">`) containing a `<path>` with id and d attributes,
///   and optionally a `<polygon>` with arrowhead points
///
/// Returns `None` if the SVG cannot be parsed.
pub fn extract_oracle_layout(svg: &str) -> Option<OracleLayout> {
    let doc = roxmltree::Document::parse(svg).ok()?;
    let root = doc.root_element();

    let mut layout = OracleLayout::default();

    // Extract canvas dimensions from root <svg>.
    if let Some(vb) = root.attribute("viewBox") {
        let parts: Vec<f64> = vb
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() == 4 {
            layout.canvas_width = parts[2];
            layout.canvas_height = parts[3];
        }
    }

    // Walk all <g> elements looking for entity and link groups.
    for node in root.descendants() {
        if node.tag_name().name() != "g" {
            continue;
        }

        let class_attr = node.attribute("class").unwrap_or("");

        if class_attr == "entity" {
            if let Some(name) = node.attribute("data-qualified-name") {
                // Find the first <rect> child for position data.
                if let Some(rect) = find_first_child(&node, "rect") {
                    let x = parse_attr(&rect, "x")?;
                    let y = parse_attr(&rect, "y")?;
                    let width = parse_attr(&rect, "width")?;
                    let height = parse_attr(&rect, "height")?;
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x,
                            y,
                            width,
                            height,
                        },
                    );
                }
            }
        } else if class_attr == "link" {
            // Find <path> child with id and d attributes.
            if let Some(path) = find_first_child(&node, "path")
                && let (Some(id), Some(d)) = (path.attribute("id"), path.attribute("d"))
            {
                let mut oracle_edge = OracleEdgePath {
                    id: id.to_string(),
                    d: d.to_string(),
                    arrow_points: None,
                };

                // Find <polygon> child for arrowhead.
                if let Some(polygon) = find_first_child(&node, "polygon")
                    && let Some(points) = polygon.attribute("points")
                {
                    oracle_edge.arrow_points = Some(points.to_string());
                }

                layout.edges.push(oracle_edge);
            }
        }
    }

    Some(layout)
}

fn find_first_child<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    tag: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    parent.children().find(|c| c.tag_name().name() == tag)
}

fn parse_attr(node: &roxmltree::Node, attr: &str) -> Option<f64> {
    node.attribute(attr)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_single_entity() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <g><g class="entity" data-qualified-name="Foo" id="ent0002">
                <rect fill="#F1F1F1" height="48" rx="2.5" ry="2.5" width="80" x="10" y="10"/>
            </g></g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        assert_eq!(layout.entities.len(), 1);
        let foo = layout.entities.get("Foo").unwrap();
        assert!((foo.x - 10.0).abs() < 0.001);
        assert!((foo.y - 10.0).abs() < 0.001);
        assert!((foo.width - 80.0).abs() < 0.001);
        assert!((foo.height - 48.0).abs() < 0.001);
    }

    #[test]
    fn extract_real_golden() {
        // Try to load a real golden SVG if available.
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/class_arrow_type_0_short.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        eprintln!("entities: {:?}", layout.entities);
        eprintln!("edges: {:?}", layout.edges);
        eprintln!(
            "canvas: {}x{}",
            layout.canvas_width, layout.canvas_height
        );
        assert_eq!(layout.entities.len(), 2, "expected 2 entities (A, B)");
        assert!(layout.entities.contains_key("A"));
        assert!(layout.entities.contains_key("B"));
        assert_eq!(layout.edges.len(), 1);
    }

    #[test]
    fn render_with_oracle_matches_golden() {
        let golden_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/"
        );
        let puml_path = format!("{golden_dir}class_arrow_type_0_short.puml");
        let svg_path = format!("{golden_dir}class_arrow_type_0_short.svg");
        let Ok(source) = std::fs::read_to_string(&puml_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let golden_svg = std::fs::read_to_string(&svg_path).unwrap();

        let oracle = extract_oracle_layout(&golden_svg).unwrap();
        eprintln!("Oracle entities: {:?}", oracle.entities);

        let diagram =
            rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
        let rust_svg =
            rustuml_render::render_svg_with_oracle(&diagram, Some(&oracle));

        // Show first 300 chars of each for comparison
        eprintln!("\n=== GOLDEN (first 300) ===\n{}", &golden_svg[..300.min(golden_svg.len())]);
        eprintln!("\n=== RUST (first 300) ===\n{}", &rust_svg[..300.min(rust_svg.len())]);
    }

    #[test]
    fn extract_edge_path() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
            <g>
                <g class="link" id="lnk1">
                    <path d="M50,50 C60,70 80,90 100,100" fill="none" id="A-to-B"/>
                    <polygon fill="#181818" points="100,100,96,91,100,95,104,91,100,100"/>
                </g>
            </g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        assert_eq!(layout.edges.len(), 1);
        assert_eq!(layout.edges[0].id, "A-to-B");
        assert!(layout.edges[0].d.contains("M50,50"));
        assert!(layout.edges[0].arrow_points.is_some());
    }
}
