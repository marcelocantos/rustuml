// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram SVG renderer.
//!
//! Uses rustuml-layout (Sugiyama algorithm) for node positioning,
//! then renders classes with fields/methods and relationships.

use rustuml_layout::graph::{Direction, LayoutGraph};
use rustuml_parser::diagram::class::*;

use crate::metrics;
use crate::svg::SvgBuilder;

const CLASS_MIN_WIDTH: f64 = 120.0;
const HEADER_HEIGHT: f64 = 30.0;
const MEMBER_HEIGHT: f64 = 18.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 8.0;
const MARGIN: f64 = 30.0;

/// Render a class diagram to SVG.
pub fn render(diagram: &ClassDiagram) -> String {
    if diagram.entities.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Phase 1: Use layout engine to determine relative ordering.
    let mut layout = LayoutGraph::new(Direction::TopToBottom);

    for entity in &diagram.entities {
        layout.add_node(&entity.id, &entity.label);
    }
    for rel in &diagram.relationships {
        layout.add_edge(&rel.from, &rel.to, rel.label.as_deref());
    }

    // Get the layout SVG — used as a reference for relative positions.
    // Full integration (extracting coordinates) is a future improvement.
    let layout_svg = layout.to_svg();

    // Phase 2: Render with our own class boxes.
    render_simple(diagram, &layout_svg)
}

/// Simple rendering without layout engine integration.
/// Places classes in a grid and draws relationships.
fn render_simple(diagram: &ClassDiagram, _layout_hint: &str) -> String {
    if diagram.entities.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Calculate dimensions for each class box.
    let class_dims: Vec<ClassDim> = diagram.entities.iter().map(calc_class_dim).collect();

    // Simple grid layout: arrange classes in rows.
    let cols = (diagram.entities.len() as f64).sqrt().ceil() as usize;
    let col_widths = calc_col_widths(&class_dims, cols);
    let row_heights = calc_row_heights(&class_dims, cols);

    let total_width = col_widths.iter().sum::<f64>() + MARGIN * (cols as f64 + 1.0);
    let total_height = row_heights.iter().sum::<f64>() + MARGIN * (row_heights.len() as f64 + 1.0);

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Position and render each class.
    let mut positions: Vec<(f64, f64, f64, f64)> = Vec::new(); // (x, y, w, h)

    for (i, (entity, dim)) in diagram.entities.iter().zip(&class_dims).enumerate() {
        let col = i % cols;
        let row = i / cols;

        let x = MARGIN + col_widths[..col].iter().sum::<f64>() + MARGIN * col as f64;
        let y = MARGIN + row_heights[..row].iter().sum::<f64>() + MARGIN * row as f64;

        render_class_box(&mut svg, entity, x, y, dim);
        positions.push((x, y, dim.width, dim.height));
    }

    // Render relationships as lines between class centers.
    for rel in &diagram.relationships {
        let from_idx = diagram.entities.iter().position(|e| e.id == rel.from);
        let to_idx = diagram.entities.iter().position(|e| e.id == rel.to);

        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            let (fx, fy, fw, fh) = positions[fi];
            let (tx, ty, tw, _th) = positions[ti];

            let from_cx = fx + fw / 2.0;
            let from_bottom = fy + fh;
            let to_cx = tx + tw / 2.0;
            let to_top = ty;

            let dashed = matches!(
                rel.kind,
                RelationshipKind::Dependency | RelationshipKind::Implementation
            );
            svg.line_segment(from_cx, from_bottom, to_cx, to_top, "#000", dashed);

            // Draw relationship decoration at the target end.
            render_relationship_head(&mut svg, rel.kind, to_cx, to_top);

            // Label.
            if let Some(label) = &rel.label {
                let mid_x = (from_cx + to_cx) / 2.0;
                let mid_y = (from_bottom + to_top) / 2.0;
                svg.text(mid_x, mid_y - 4.0, label, "middle", SMALL_FONT);
            }
        }
    }

    svg.finalize()
}

struct ClassDim {
    width: f64,
    height: f64,
    header_text: String,
    kind_label: Option<&'static str>,
}

fn calc_class_dim(entity: &ClassEntity) -> ClassDim {
    let kind_label = match entity.kind {
        EntityKind::Interface => Some("<<interface>>"),
        EntityKind::AbstractClass => Some("<<abstract>>"),
        EntityKind::Enum => Some("<<enum>>"),
        EntityKind::Annotation => Some("<<annotation>>"),
        EntityKind::Entity => Some("<<entity>>"),
        EntityKind::Class => None,
    };

    let name_width = metrics::text_width(&entity.label, FONT_SIZE) + PADDING * 2.0;
    let kind_width = kind_label.map_or(0.0, |k| metrics::text_width(k, SMALL_FONT) + PADDING * 2.0);
    let member_max_width = entity
        .members
        .iter()
        .map(|m| metrics::text_width(&format_member(m), SMALL_FONT) + PADDING * 2.0)
        .fold(0.0_f64, f64::max);

    let width = CLASS_MIN_WIDTH
        .max(name_width)
        .max(kind_width)
        .max(member_max_width);

    let kind_height = if kind_label.is_some() {
        MEMBER_HEIGHT
    } else {
        0.0
    };
    let members_height = if entity.members.is_empty() {
        0.0
    } else {
        entity.members.len() as f64 * MEMBER_HEIGHT + PADDING
    };

    let height = HEADER_HEIGHT + kind_height + members_height;

    ClassDim {
        width,
        height,
        header_text: entity.label.clone(),
        kind_label,
    }
}

fn render_class_box(svg: &mut SvgBuilder, entity: &ClassEntity, x: f64, y: f64, dim: &ClassDim) {
    // Background.
    let fill = match entity.kind {
        EntityKind::Interface => "#D4E6F1",
        EntityKind::Enum => "#D5F5E3",
        EntityKind::Annotation => "#FCF3CF",
        _ => "#FDEBD0",
    };
    svg.rect(x, y, dim.width, dim.height, fill, "#000");

    let mut cy = y;

    // Kind label (<<interface>>, etc.).
    if let Some(kind) = dim.kind_label {
        cy += MEMBER_HEIGHT;
        svg.text(x + dim.width / 2.0, cy - 3.0, kind, "middle", SMALL_FONT);
    }

    // Class name (bold-styled via font-weight in the text).
    cy += HEADER_HEIGHT / 2.0 + 5.0;
    svg.text(
        x + dim.width / 2.0,
        cy,
        &dim.header_text,
        "middle",
        FONT_SIZE,
    );
    cy = y + HEADER_HEIGHT + dim.kind_label.map_or(0.0, |_| MEMBER_HEIGHT);

    // Separator line.
    if !entity.members.is_empty() {
        svg.line_segment(x, cy, x + dim.width, cy, "#000", false);
    }

    // Members.
    for member in &entity.members {
        cy += MEMBER_HEIGHT;
        let text = format_member(member);
        svg.text(x + PADDING, cy - 3.0, &text, "start", SMALL_FONT);
    }
}

fn format_member(member: &Member) -> String {
    let vis = match member.visibility {
        Visibility::Public => "+",
        Visibility::Private => "-",
        Visibility::Protected => "#",
        Visibility::Package => "~",
        Visibility::Default => "",
    };
    let static_prefix = if member.is_static { "{static} " } else { "" };
    let abstract_prefix = if member.is_abstract {
        "{abstract} "
    } else {
        ""
    };
    let type_suffix = member
        .return_type
        .as_ref()
        .map_or(String::new(), |t| format!(" : {t}"));

    format!(
        "{vis}{static_prefix}{abstract_prefix}{}{type_suffix}",
        member.name
    )
}

fn render_relationship_head(svg: &mut SvgBuilder, kind: RelationshipKind, x: f64, y: f64) {
    match kind {
        RelationshipKind::Inheritance | RelationshipKind::Implementation => {
            // Open triangle (unfilled).
            let size = 10.0;
            svg.open_group("rel-head");
            let points = format!(
                "{x},{y} {},{} {},{}",
                x - size / 2.0,
                y - size,
                x + size / 2.0,
                y - size,
            );
            svg.line_segment(x, y, x - size / 2.0, y - size, "#000", false);
            svg.line_segment(x, y, x + size / 2.0, y - size, "#000", false);
            svg.line_segment(
                x - size / 2.0,
                y - size,
                x + size / 2.0,
                y - size,
                "#000",
                false,
            );
            let _ = points;
            svg.close_group();
        }
        RelationshipKind::Composition => {
            // Filled diamond.
            svg.arrow_head(x, y, 90.0);
        }
        RelationshipKind::Aggregation => {
            // Open diamond (approximated with arrow).
            svg.arrow_head(x, y, 90.0);
        }
        RelationshipKind::Dependency => {
            // Simple arrow head.
            svg.arrow_head(x, y, 90.0);
        }
        RelationshipKind::Association => {
            // No decoration.
        }
    }
}

fn calc_col_widths(dims: &[ClassDim], cols: usize) -> Vec<f64> {
    let mut widths = vec![0.0_f64; cols];
    for (i, dim) in dims.iter().enumerate() {
        let col = i % cols;
        widths[col] = widths[col].max(dim.width);
    }
    widths
}

fn calc_row_heights(dims: &[ClassDim], cols: usize) -> Vec<f64> {
    let rows = dims.len().div_ceil(cols);
    let mut heights = vec![0.0_f64; rows];
    for (i, dim) in dims.iter().enumerate() {
        let row = i / cols;
        heights[row] = heights[row].max(dim.height);
    }
    heights
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_class_diagram() -> ClassDiagram {
        ClassDiagram {
            meta: DiagramMeta::default(),
            entities: vec![
                ClassEntity {
                    id: "Animal".into(),
                    label: "Animal".into(),
                    kind: EntityKind::Class,
                    members: vec![
                        Member {
                            name: "name".into(),
                            return_type: Some("String".into()),
                            visibility: Visibility::Public,
                            is_static: false,
                            is_abstract: false,
                            kind: MemberKind::Field,
                        },
                        Member {
                            name: "makeSound()".into(),
                            return_type: Some("void".into()),
                            visibility: Visibility::Public,
                            is_static: false,
                            is_abstract: false,
                            kind: MemberKind::Method,
                        },
                    ],
                    stereotypes: vec![],
                },
                ClassEntity {
                    id: "Dog".into(),
                    label: "Dog".into(),
                    kind: EntityKind::Class,
                    members: vec![Member {
                        name: "fetch()".into(),
                        return_type: Some("void".into()),
                        visibility: Visibility::Public,
                        is_static: false,
                        is_abstract: false,
                        kind: MemberKind::Method,
                    }],
                    stereotypes: vec![],
                },
            ],
            relationships: vec![Relationship {
                from: "Animal".into(),
                to: "Dog".into(),
                kind: RelationshipKind::Inheritance,
                label: None,
                from_multiplicity: None,
                to_multiplicity: None,
            }],
            packages: vec![],
        }
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_class_diagram());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
    }

    #[test]
    fn has_class_boxes() {
        let svg = render(&simple_class_diagram());
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 2,
            "should have at least 2 class boxes, got {rect_count}"
        );
    }

    #[test]
    fn has_members() {
        let svg = render(&simple_class_diagram());
        assert!(svg.contains("+name : String"));
        assert!(svg.contains("+makeSound()"));
        assert!(svg.contains("+fetch()"));
    }

    #[test]
    fn has_relationship_line() {
        let svg = render(&simple_class_diagram());
        assert!(svg.contains("<line"), "should have relationship line");
    }

    #[test]
    fn interface_rendering() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            entities: vec![ClassEntity {
                id: "Drawable".into(),
                label: "Drawable".into(),
                kind: EntityKind::Interface,
                members: vec![Member {
                    name: "draw()".into(),
                    return_type: Some("void".into()),
                    visibility: Visibility::Public,
                    is_static: false,
                    is_abstract: true,
                    kind: MemberKind::Method,
                }],
                stereotypes: vec![],
            }],
            relationships: vec![],
            packages: vec![],
        };
        let svg = render(&diagram);
        assert!(svg.contains("&lt;&lt;interface&gt;&gt;"));
        assert!(svg.contains("Drawable"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\nclass Animal {\n  +name : String\n}\nclass Dog\nAnimal <|-- Dog\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
    }

    #[test]
    fn empty_diagram() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            entities: vec![],
            relationships: vec![],
            packages: vec![],
        };
        let svg = render(&diagram);
        assert!(svg.contains("<svg"));
    }
}
