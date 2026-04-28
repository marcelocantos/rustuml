// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! ASCII art renderer for state diagrams.

use rustuml_parser::diagram::state::*;

/// Box-drawing vertical line.
const V: char = '│';
/// Initial state symbol.
const INITIAL: char = '●';
/// Final state symbol.
const FINAL: char = '◉';
/// Down arrow.
const ARROW_DOWN: char = '▼';

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

/// Render a state diagram as ASCII art.
///
/// Produces a simple top-to-bottom vertical layout:
/// - Initial states render as `●`
/// - Normal states render as `[StateName]`
/// - Final states render as `◉`
/// - Transitions render as vertical arrows with optional labels.
pub fn render_ascii(diagram: &StateDiagram) -> String {
    if diagram.states.is_empty() && diagram.transitions.is_empty() {
        return String::new();
    }

    // Build an ordered list of state IDs from transitions (preserving first-seen order).
    let mut ordered_ids: Vec<String> = Vec::new();
    for t in &diagram.transitions {
        if !ordered_ids.contains(&t.from) {
            ordered_ids.push(t.from.clone());
        }
        if !ordered_ids.contains(&t.to) {
            ordered_ids.push(t.to.clone());
        }
    }
    // Add any states not referenced by transitions.
    for s in &diagram.states {
        if !ordered_ids.contains(&s.id) {
            ordered_ids.push(s.id.clone());
        }
    }

    let state_index = |id: &str| -> Option<&State> { diagram.states.iter().find(|s| s.id == id) };

    // Compute max label width for sizing.
    let max_label_width = ordered_ids
        .iter()
        .filter_map(|id| state_index(id))
        .map(|s| s.label.len())
        .max()
        .unwrap_or(6)
        .max(6);

    let grid_width = (max_label_width + 10).max(40);
    let centre = grid_width / 2;
    let mut grid = Grid::new(grid_width);
    let mut row = 0usize;

    for (i, id) in ordered_ids.iter().enumerate() {
        let st = state_index(id);
        let kind = st.map(|s| s.kind).unwrap_or(StateKind::Normal);

        match kind {
            StateKind::Initial => {
                grid.set(centre, row, INITIAL);
                row += 1;
            }
            StateKind::Final => {
                if i > 0 {
                    grid.set(centre, row, V);
                    row += 1;
                    grid.set(centre, row, ARROW_DOWN);
                    row += 1;
                }
                grid.set(centre, row, FINAL);
                row += 1;
            }
            _ => {
                if i > 0 {
                    // Find the transition from the previous state to this one for label.
                    let prev_id = &ordered_ids[i - 1];
                    let transition_label = diagram
                        .transitions
                        .iter()
                        .find(|t| &t.from == prev_id && &t.to == id)
                        .and_then(|t| t.label.as_deref());

                    grid.set(centre, row, V);
                    row += 1;

                    if let Some(label) = transition_label {
                        let label_col = centre + 2;
                        grid.write_str(label_col, row.saturating_sub(1), label);
                    }

                    grid.set(centre, row, ARROW_DOWN);
                    row += 1;
                }

                let label = st.map(|s| s.label.as_str()).unwrap_or(id.as_str());
                let text = format!("[{label}]");
                let start_col = centre.saturating_sub(text.len() / 2);
                grid.write_str(start_col, row, &text);
                row += 1;
            }
        }
    }

    grid.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn make_state(id: &str, kind: StateKind) -> State {
        State {
            id: id.to_string(),
            label: id.to_string(),
            kind,
            descriptions: vec![],
            substates: vec![],
            source_line: 0,
        }
    }

    fn make_transition(from: &str, to: &str, label: Option<&str>) -> Transition {
        Transition {
            from: from.to_string(),
            to: to.to_string(),
            label: label.map(|s| s.to_string()),
            source_line: 0,
        }
    }

    fn make_diagram(states: Vec<State>, transitions: Vec<Transition>) -> StateDiagram {
        StateDiagram {
            meta: DiagramMeta::default(),
            states,
            transitions,
            notes: vec![],
        }
    }

    #[test]
    fn simple_state_machine() {
        let diagram = make_diagram(
            vec![
                make_state("[*]", StateKind::Initial),
                make_state("Active", StateKind::Normal),
                make_state("Inactive", StateKind::Normal),
            ],
            vec![
                make_transition("[*]", "Active", None),
                make_transition("Active", "Inactive", Some("disable")),
            ],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains('●'), "missing initial state\n{out}");
        assert!(out.contains("[Active]"), "missing Active state\n{out}");
        assert!(out.contains("[Inactive]"), "missing Inactive state\n{out}");
        assert!(out.contains("disable"), "missing transition label\n{out}");
    }

    #[test]
    fn initial_to_final() {
        let diagram = make_diagram(
            vec![
                make_state("[*]", StateKind::Initial),
                make_state("Done", StateKind::Normal),
                make_state("[*]end", StateKind::Final),
            ],
            vec![
                make_transition("[*]", "Done", None),
                make_transition("Done", "[*]end", None),
            ],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains('●'), "missing initial\n{out}");
        assert!(out.contains('◉'), "missing final\n{out}");
        assert!(out.contains("[Done]"), "missing Done\n{out}");
    }

    #[test]
    fn empty_diagram() {
        let diagram = make_diagram(vec![], vec![]);
        assert_eq!(render_ascii(&diagram), "");
    }

    #[test]
    fn arrows_present() {
        let diagram = make_diagram(
            vec![
                make_state("[*]", StateKind::Initial),
                make_state("A", StateKind::Normal),
            ],
            vec![make_transition("[*]", "A", None)],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains('▼'), "missing down arrow\n{out}");
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\n[*] --> Active\nActive --> Inactive\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let out = crate::render_ascii(&diagram);
        assert!(out.contains("[Active]"), "missing Active\n{out}");
        assert!(out.contains("[Inactive]"), "missing Inactive\n{out}");
    }
}
