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

        if class_attr == "entity" || class_attr == "cluster" {
            if let Some(name) = node.attribute("data-qualified-name") {
                // Find the first <rect> child for position data.
                if let Some(rect) = find_first_child(&node, "rect") {
                    let x = parse_attr(&rect, "x")?;
                    let y = parse_attr(&rect, "y")?;
                    let width = parse_attr(&rect, "width")?;
                    let height = parse_attr(&rect, "height")?;
                    // Look for an icon ellipse to extract icon_cx.
                    let icon_cx =
                        find_first_child(&node, "ellipse").and_then(|e| parse_attr(&e, "cx"));
                    // Extract glyph path d attribute (the <path> with fill="#000000").
                    let glyph_path_d = node
                        .children()
                        .find(|c| {
                            c.tag_name().name() == "path" && c.attribute("fill") == Some("#000000")
                        })
                        .and_then(|p| p.attribute("d").map(String::from));
                    // Extract name text x (first <text> child).
                    let name_text_x = node
                        .children()
                        .find(|c| c.tag_name().name() == "text")
                        .and_then(|t| parse_attr(&t, "x"));
                    // Extract all text y-values and separator line y-values.
                    let text_y_values: Vec<f64> = node
                        .children()
                        .filter(|c| c.tag_name().name() == "text")
                        .filter_map(|t| parse_attr(&t, "y"))
                        .collect();
                    let sep_y_values: Vec<f64> = node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .filter_map(|l| parse_attr(&l, "y1"))
                        .collect();
                    // Extract visibility icon y-positions from
                    // <g data-visibility-modifier><rect y="..."> or <ellipse cy="...">
                    // Extract visibility icon center-y: for rects (y + height/2),
                    // for ellipses (cy directly). Stored as icon_cy for uniformity.
                    let vis_icon_y_values: Vec<f64> = node
                        .children()
                        .filter(|c| {
                            c.tag_name().name() == "g"
                                && c.attribute("data-visibility-modifier").is_some()
                        })
                        .filter_map(|g| {
                            g.children()
                                .find(|c| {
                                    c.tag_name().name() == "rect"
                                        || c.tag_name().name() == "ellipse"
                                        || c.tag_name().name() == "polygon"
                                })
                                .and_then(|el| match el.tag_name().name() {
                                    "rect" => {
                                        let y = parse_attr(&el, "y")?;
                                        let h = parse_attr(&el, "height")?;
                                        Some(y + h / 2.0)
                                    }
                                    "ellipse" => parse_attr(&el, "cy"),
                                    _ => None, // polygon — skip for now
                                })
                        })
                        .collect();
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x,
                            y,
                            width,
                            height,
                            icon_cx,
                            glyph_path_d,
                            name_text_x,
                            text_y_values,
                            sep_y_values,
                            vis_icon_y_values,
                        },
                    );
                } else if let Some(ellipse) = find_first_child(&node, "ellipse") {
                    // Start/end pseudo-states and other circular entities use <ellipse>.
                    let cx = parse_attr(&ellipse, "cx")?;
                    let cy = parse_attr(&ellipse, "cy")?;
                    let rx = parse_attr(&ellipse, "rx")?;
                    let ry = parse_attr(&ellipse, "ry")?;
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x: cx - rx,
                            y: cy - ry,
                            width: rx * 2.0,
                            height: ry * 2.0,
                            icon_cx: None,
                            glyph_path_d: None,
                            name_text_x: None,
                            text_y_values: Vec::new(),
                            sep_y_values: Vec::new(),
                            vis_icon_y_values: Vec::new(),
                        },
                    );
                } else if let Some(polygon) = find_first_child(&node, "polygon") {
                    // Choice pseudo-states use <polygon> (diamond).
                    if let Some(points) = polygon.attribute("points") {
                        let coords: Vec<f64> = points
                            .split(|c: char| c == ',' || c.is_whitespace())
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if coords.len() >= 8 {
                            let xs: Vec<f64> = coords.iter().step_by(2).copied().collect();
                            let ys: Vec<f64> = coords.iter().skip(1).step_by(2).copied().collect();
                            let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
                            let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
                            let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            layout.entities.insert(
                                name.to_string(),
                                EntityRect {
                                    x: min_x,
                                    y: min_y,
                                    width: max_x - min_x,
                                    height: max_y - min_y,
                                    icon_cx: None,
                                    glyph_path_d: None,
                                    name_text_x: None,
                                    text_y_values: Vec::new(),
                                    sep_y_values: Vec::new(),
                                    vis_icon_y_values: Vec::new(),
                                },
                            );
                        }
                    }
                } else if let Some(path) = find_first_child(&node, "path")
                    && let Some(d) = path.attribute("d")
                    && let Some(bbox) = path_bounding_box(d)
                {
                    // Some entities (notes, clouds) use <path>. Extract bounding box.
                    layout.entities.insert(name.to_string(), bbox);
                }
            }
        } else if class_attr == "start_entity" || class_attr == "end_entity" {
            if let Some(name) = node.attribute("data-qualified-name") {
                // Start/end pseudo-states use <ellipse>.
                if let Some(ellipse) = find_first_child(&node, "ellipse") {
                    let cx = parse_attr(&ellipse, "cx")?;
                    let cy = parse_attr(&ellipse, "cy")?;
                    let rx = parse_attr(&ellipse, "rx")?;
                    let ry = parse_attr(&ellipse, "ry")?;
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x: cx - rx,
                            y: cy - ry,
                            width: rx * 2.0,
                            height: ry * 2.0,
                            icon_cx: None,
                            glyph_path_d: None,
                            name_text_x: None,
                            text_y_values: Vec::new(),
                            sep_y_values: Vec::new(),
                            vis_icon_y_values: Vec::new(),
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
                    arrow_fill: None,
                    link_type: node.attribute("data-link-type").map(String::from),
                    entity_1: node.attribute("data-entity-1").map(String::from),
                    entity_2: node.attribute("data-entity-2").map(String::from),
                    source_line: node.attribute("data-source-line").map(String::from),
                    link_id: node.attribute("id").map(String::from),
                    path_style: path.attribute("style").map(String::from),
                    code_line: path.attribute("codeLine").map(String::from),
                    polygon_style: None,
                };

                // Find <polygon> child for arrowhead.
                if let Some(polygon) = find_first_child(&node, "polygon")
                    && let Some(points) = polygon.attribute("points")
                {
                    oracle_edge.arrow_points = Some(points.to_string());
                    oracle_edge.arrow_fill = polygon.attribute("fill").map(String::from);
                    oracle_edge.polygon_style = polygon.attribute("style").map(String::from);
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

/// Extract a rough bounding box from an SVG path `d` attribute.
///
/// Only considers M/L/C coordinates (ignores arcs and relative commands).
/// Returns `None` if no coordinates could be parsed.
fn path_bounding_box(d: &str) -> Option<EntityRect> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    // Simple tokenizer: split on command letters and commas/spaces.
    let nums: Vec<f64> = d
        .replace(|c: char| c.is_ascii_alphabetic(), " ")
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();

    // Take pairs as (x, y).
    for pair in nums.chunks(2) {
        if pair.len() == 2 {
            min_x = min_x.min(pair[0]);
            max_x = max_x.max(pair[0]);
            min_y = min_y.min(pair[1]);
            max_y = max_y.max(pair[1]);
            found = true;
        }
    }

    if found {
        Some(EntityRect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
            icon_cx: None,
            glyph_path_d: None,
            name_text_x: None,
            text_y_values: Vec::new(),
            sep_y_values: Vec::new(),
            vis_icon_y_values: Vec::new(),
        })
    } else {
        None
    }
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
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/class_arrow_type_0_short.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        assert_eq!(layout.entities.len(), 2, "expected 2 entities (A, B)");
        assert!(layout.entities.contains_key("A"));
        assert!(layout.entities.contains_key("B"));
        assert_eq!(layout.edges.len(), 1);
        assert_eq!(layout.edges[0].id, "A-to-B");
        assert!(layout.edges[0].link_type.as_deref() == Some("dependency"));
    }

    /// Verify that oracle-based rendering produces output identical to the golden SVG.
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
        let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
        let rust_svg = rustuml_render::render_svg_with_oracle(&diagram, Some(&oracle));

        let cmp = crate::compare::compare_svg_strict(&golden_svg, &rust_svg).unwrap();
        assert!(
            cmp.is_match(),
            "Oracle rendering should match golden:\n{cmp}"
        );
    }

    #[test]
    fn render_4combo_with_oracle() {
        let golden_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/"
        );
        let test_name = "class_4combo_abstract_class_none_nocolor_private";
        let puml_path = format!("{golden_dir}{test_name}.puml");
        let svg_path = format!("{golden_dir}{test_name}.svg");
        let Ok(source) = std::fs::read_to_string(&puml_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let golden_svg = std::fs::read_to_string(&svg_path).unwrap();

        let oracle = extract_oracle_layout(&golden_svg).unwrap();

        // Verify icon_cx was extracted.
        let combo = oracle.entities.get("Combo").expect("entity Combo");
        eprintln!("Combo: icon_cx={:?}", combo.icon_cx);
        assert!(
            combo.icon_cx.is_some(),
            "icon_cx should be extracted from golden SVG"
        );

        let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
        let rust_svg = rustuml_render::render_svg_with_oracle(&diagram, Some(&oracle));

        let cmp = crate::compare::compare_svg_strict(&golden_svg, &rust_svg).unwrap();
        assert!(
            cmp.is_match(),
            "Oracle rendering should match golden:\n{cmp}"
        );
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

    #[test]
    fn extract_state_golden() {
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/state/state_alias_len_1.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        // Should extract: entity "S", start_entity ".start.", end_entity ".end."
        assert!(
            layout.entities.contains_key("S"),
            "should extract state entity S, got: {:?}",
            layout.entities.keys().collect::<Vec<_>>()
        );
        assert!(
            layout.entities.contains_key(".start."),
            "should extract start entity, got: {:?}",
            layout.entities.keys().collect::<Vec<_>>()
        );
        assert!(
            layout.entities.contains_key(".end."),
            "should extract end entity, got: {:?}",
            layout.entities.keys().collect::<Vec<_>>()
        );
        // Should have 2 edges.
        assert_eq!(layout.edges.len(), 2, "expected 2 transition edges");
        assert!(layout.canvas_width > 0.0);
        assert!(layout.canvas_height > 0.0);
    }

    #[test]
    fn extract_component_golden() {
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/component/comp_3cont_cloud_folder_node.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        // Should have cluster and entity groups.
        assert!(
            !layout.entities.is_empty(),
            "should extract entities from component diagram"
        );
        assert!(layout.canvas_width > 0.0);
    }
}
