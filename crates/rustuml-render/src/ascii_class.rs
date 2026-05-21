// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! ASCII art renderer for class diagrams.

use rustuml_parser::diagram::class::*;

// Box-drawing characters.
const H: char = '─';
const V: char = '│';
const TL: char = '┌';
const TR: char = '┐';
const BL: char = '└';
const BR: char = '┘';
const T_LEFT: char = '┤';
const T_RIGHT: char = '├';

/// Padding inside class boxes on each side.
const PAD: usize = 1;

/// Vertical gap between stacked class boxes.
const VGAP: usize = 1;

/// Render a visibility prefix character.
fn visibility_char(v: Visibility) -> char {
    match v {
        Visibility::Public => '+',
        Visibility::Private => '-',
        Visibility::Protected => '#',
        Visibility::Package => '~',
        Visibility::Default => ' ',
        Visibility::IeMandatory => '*',
    }
}

/// A mutable character grid for assembling the diagram.
struct Grid {
    cells: Vec<Vec<char>>,
    width: usize,
}

impl Grid {
    fn new(width: usize) -> Self {
        Self {
            cells: Vec::new(),
            width,
        }
    }

    fn ensure_rows(&mut self, row: usize) {
        while self.cells.len() <= row {
            self.cells.push(vec![' '; self.width]);
        }
    }

    fn set(&mut self, col: usize, row: usize, ch: char) {
        if col >= self.width {
            return;
        }
        self.ensure_rows(row);
        self.cells[row][col] = ch;
    }

    fn write_str(&mut self, col: usize, row: usize, s: &str) {
        self.ensure_rows(row);
        for (i, ch) in s.chars().enumerate() {
            let c = col + i;
            if c < self.width {
                self.cells[row][c] = ch;
            }
        }
    }

    fn hline(&mut self, col: usize, row: usize, len: usize, ch: char) {
        for i in 0..len {
            self.set(col + i, row, ch);
        }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for row in &self.cells {
            let trimmed: String = row.iter().collect::<String>().trim_end().to_string();
            out.push_str(&trimmed);
            out.push('\n');
        }
        out
    }
}

/// Compute the required inner width for a class entity box.
fn entity_inner_width(entity: &ClassEntity) -> usize {
    let header_len = entity.label.len();
    let member_widths = entity.members.iter().map(|m| {
        // visibility_char + space + display_text
        2 + m.display_text.len()
    });
    let max_member = member_widths.max().unwrap_or(0);
    header_len.max(max_member).max(4)
}

/// Draw a single class entity box. Returns (box_width, box_height).
fn draw_entity(grid: &mut Grid, col: usize, row: usize, entity: &ClassEntity) -> (usize, usize) {
    let inner = entity_inner_width(entity);
    let total_w = inner + PAD * 2;
    let box_w = total_w + 2; // +2 for left/right borders

    // Top border: ┌───┐
    grid.set(col, row, TL);
    grid.hline(col + 1, row, total_w, H);
    grid.set(col + box_w - 1, row, TR);

    // Header row: │  Label  │
    let r = row + 1;
    grid.set(col, r, V);
    let pad_left = PAD + (inner.saturating_sub(entity.label.len())) / 2;
    grid.write_str(col + 1 + pad_left, r, &entity.label);
    grid.set(col + box_w - 1, r, V);

    let mut cur_row = row + 2;

    // Separator and members.
    if !entity.members.is_empty() {
        grid.set(col, cur_row, T_RIGHT);
        grid.hline(col + 1, cur_row, total_w, H);
        grid.set(col + box_w - 1, cur_row, T_LEFT);
        cur_row += 1;

        for m in &entity.members {
            grid.set(col, cur_row, V);
            let vis = visibility_char(m.visibility);
            let text = format!("{vis} {}", m.display_text);
            grid.write_str(col + 1 + PAD, cur_row, &text);
            grid.set(col + box_w - 1, cur_row, V);
            cur_row += 1;
        }
    }

    // Bottom border: └───┘
    grid.set(col, cur_row, BL);
    grid.hline(col + 1, cur_row, total_w, H);
    grid.set(col + box_w - 1, cur_row, BR);

    let box_h = cur_row - row + 1;
    (box_w, box_h)
}

/// Arrow character for a relationship kind.
fn relationship_arrow(kind: RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Inheritance | RelationshipKind::Implementation => "▲",
        RelationshipKind::Composition => "◆",
        RelationshipKind::Aggregation => "◇",
        RelationshipKind::Association | RelationshipKind::Dependency => "▼",
    }
}

/// Relationship kind to a display string for labels.
fn relationship_label(kind: RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Inheritance => "extends",
        RelationshipKind::Implementation => "implements",
        RelationshipKind::Composition => "*──",
        RelationshipKind::Aggregation => "o──",
        RelationshipKind::Association => "",
        RelationshipKind::Dependency => "uses",
    }
}

/// Compute the height of an entity box.
fn entity_height(entity: &ClassEntity) -> usize {
    if entity.members.is_empty() {
        3 // top + header + bottom
    } else {
        3 + entity.members.len() // top + header + separator + members + bottom
    }
}

/// Render a class diagram as ASCII art.
pub fn render_ascii(diagram: &ClassDiagram) -> String {
    if diagram.entities.is_empty() {
        return String::new();
    }

    // Compute column width needed for the widest entity.
    let max_inner = diagram
        .entities
        .iter()
        .map(entity_inner_width)
        .max()
        .unwrap_or(10);
    let box_w = max_inner + PAD * 2 + 2;

    let grid_width = (box_w + 4).max(60);
    let mut grid = Grid::new(grid_width);
    let left_margin = 1;

    let entity_index =
        |id: &str| -> Option<usize> { diagram.entities.iter().position(|e| e.id == id) };

    // Draw entities stacked vertically.
    let mut entity_centres: Vec<(usize, usize)> = Vec::new(); // (centre_col, bottom_row)
    let mut row = 0usize;

    for entity in &diagram.entities {
        let (bw, bh) = draw_entity(&mut grid, left_margin, row, entity);
        let centre_col = left_margin + bw / 2;
        entity_centres.push((centre_col, row + bh));
        row += bh + VGAP;
    }

    // Draw relationships as vertical connectors between adjacent boxes.
    for rel in &diagram.relationships {
        let from_idx = entity_index(&rel.from);
        let to_idx = entity_index(&rel.to);

        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            if ti <= fi {
                continue; // Skip upward arrows in this simple layout.
            }

            let (centre, from_bottom) = entity_centres[fi];
            let to_top = entity_centres[ti].1 - entity_height(&diagram.entities[ti]);

            let connector_start = from_bottom;
            let to_box_top = to_top;

            if to_box_top > connector_start {
                for r in connector_start..to_box_top {
                    grid.set(centre, r, V);
                }

                // Arrow at the bottom of the connector.
                let arrow = relationship_arrow(rel.kind);
                grid.write_str(
                    centre.saturating_sub(arrow.chars().count() / 2),
                    to_box_top.saturating_sub(1),
                    arrow,
                );

                // Label.
                if let Some(ref label) = rel.label {
                    let label_row = connector_start + (to_box_top - connector_start) / 2;
                    grid.write_str(centre + 2, label_row, label);
                } else {
                    let kind_label = relationship_label(rel.kind);
                    if !kind_label.is_empty() {
                        let label_row = connector_start + (to_box_top - connector_start) / 2;
                        grid.write_str(centre + 2, label_row, kind_label);
                    }
                }
            }
        }
    }

    grid.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn make_entity(name: &str, members: Vec<Member>) -> ClassEntity {
        ClassEntity {
            id: name.to_string(),
            label: name.to_string(),
            kind: EntityKind::Class,
            members,
            stereotypes: vec![],
            url: None,
            color: None,
            text_color: None,
            source_line: 0,
        }
    }

    fn make_field(name: &str, vis: Visibility) -> Member {
        Member {
            name: name.to_string(),
            return_type: None,
            visibility: vis,
            is_static: false,
            is_abstract: false,
            kind: MemberKind::Field,
            display_text: name.to_string(),
        }
    }

    fn make_diagram(entities: Vec<ClassEntity>, relationships: Vec<Relationship>) -> ClassDiagram {
        ClassDiagram {
            meta: DiagramMeta::default(),
            entities,
            relationships,
            packages: vec![],
            notes: vec![],
            hide_show: vec![],
            header_line: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        }
    }

    #[test]
    fn single_class_box() {
        let diagram = make_diagram(
            vec![make_entity(
                "Animal",
                vec![
                    make_field("name: String", Visibility::Public),
                    make_field("sound(): void", Visibility::Public),
                ],
            )],
            vec![],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("Animal"), "missing class name\n{out}");
        assert!(out.contains("name: String"), "missing field\n{out}");
        assert!(out.contains("sound(): void"), "missing method\n{out}");
        assert!(out.contains('┌'), "missing top-left corner\n{out}");
        assert!(out.contains('┘'), "missing bottom-right corner\n{out}");
        assert!(out.contains('├'), "missing separator left\n{out}");
    }

    #[test]
    fn two_classes_with_inheritance() {
        let diagram = make_diagram(
            vec![
                make_entity("Animal", vec![make_field("name", Visibility::Public)]),
                make_entity("Dog", vec![make_field("fetch()", Visibility::Public)]),
            ],
            vec![Relationship {
                from: "Animal".to_string(),
                to: "Dog".to_string(),
                kind: RelationshipKind::Inheritance,
                label: None,
                from_multiplicity: None,
                to_multiplicity: None,
                dashed: false,
                source_line: 0,
            }],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("Animal"), "missing Animal\n{out}");
        assert!(out.contains("Dog"), "missing Dog\n{out}");
        assert!(out.contains('│'), "missing vertical connector\n{out}");
    }

    #[test]
    fn empty_class_no_members() {
        let diagram = make_diagram(vec![make_entity("Empty", vec![])], vec![]);
        let out = render_ascii(&diagram);
        assert!(out.contains("Empty"), "missing class name\n{out}");
        assert!(!out.contains('├'), "unexpected separator\n{out}");
    }

    #[test]
    fn visibility_prefixes() {
        let diagram = make_diagram(
            vec![make_entity(
                "Foo",
                vec![
                    make_field("pub_field", Visibility::Public),
                    make_field("priv_field", Visibility::Private),
                    make_field("prot_field", Visibility::Protected),
                ],
            )],
            vec![],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("+ pub_field"), "missing public prefix\n{out}");
        assert!(
            out.contains("- priv_field"),
            "missing private prefix\n{out}"
        );
        assert!(
            out.contains("# prot_field"),
            "missing protected prefix\n{out}"
        );
    }

    #[test]
    fn empty_diagram_returns_empty() {
        let diagram = make_diagram(vec![], vec![]);
        assert_eq!(render_ascii(&diagram), "");
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nclass Animal {\n  +name: String\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let out = crate::render_ascii(&diagram);
        assert!(out.contains("Animal"), "missing Animal\n{out}");
        assert!(out.contains("name: String"), "missing field\n{out}");
    }
}
