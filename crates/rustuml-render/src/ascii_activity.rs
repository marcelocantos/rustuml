// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! ASCII art renderer for activity diagrams.

use rustuml_parser::diagram::activity::*;

// Box-drawing characters.
const H: char = '─';
const V: char = '│';
const TL: char = '┌';
const TR: char = '┐';
const BL: char = '└';
const BR: char = '┘';

/// Padding inside action boxes.
const PAD: usize = 1;
/// Initial/start symbol.
const START: char = '●';
/// Final/stop/end symbol.
const STOP: char = '◉';
/// Diamond for decisions.
const DIAMOND: char = '◇';
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

/// Render an activity diagram as ASCII art.
///
/// Produces a simple top-to-bottom vertical flow:
/// - `start` renders as `●`
/// - Actions render as boxed text
/// - `if` renders as `◇ condition?` with branch labels
/// - `stop`/`end` renders as `◉`
pub fn render_ascii(diagram: &ActivityDiagram) -> String {
    if diagram.steps.is_empty() {
        return String::new();
    }

    // Compute max action width for uniform box sizing.
    let max_action_width = diagram
        .steps
        .iter()
        .filter_map(|s| match s {
            ActivityStep::Action(text) => Some(text.len()),
            _ => None,
        })
        .max()
        .unwrap_or(6)
        .max(6);

    let box_inner = max_action_width;
    let box_w = box_inner + PAD * 2 + 2;
    let grid_width = (box_w + 10).max(40);
    let centre = grid_width / 2;
    let mut grid = Grid::new(grid_width);
    let mut row = 0usize;
    let mut need_connector = false;

    for step in &diagram.steps {
        match step {
            ActivityStep::Start => {
                grid.set(centre, row, START);
                row += 1;
                need_connector = true;
            }

            ActivityStep::Stop | ActivityStep::End => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                    grid.set(centre, row, ARROW_DOWN);
                    row += 1;
                }
                grid.set(centre, row, STOP);
                row += 1;
                need_connector = false;
            }

            ActivityStep::Action(text) => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                    grid.set(centre, row, ARROW_DOWN);
                    row += 1;
                }

                let inner = text.len().max(4);
                let total_w = inner + PAD * 2;
                let bw = total_w + 2;
                let left = centre.saturating_sub(bw / 2);

                // Top border.
                grid.set(left, row, TL);
                grid.hline(left + 1, row, total_w, H);
                grid.set(left + bw - 1, row, TR);
                row += 1;

                // Content row.
                grid.set(left, row, V);
                let pad_left = PAD + (inner.saturating_sub(text.len())) / 2;
                grid.write_str(left + 1 + pad_left, row, text);
                grid.set(left + bw - 1, row, V);
                row += 1;

                // Bottom border.
                grid.set(left, row, BL);
                grid.hline(left + 1, row, total_w, H);
                grid.set(left + bw - 1, row, BR);
                row += 1;

                need_connector = true;
            }

            ActivityStep::If(block) => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }

                let text = format!("{} {}?", DIAMOND, block.condition);
                let start_col = centre.saturating_sub(text.len() / 2);
                grid.write_str(start_col, row, &text);
                row += 1;

                if let Some(ref label) = block.then_label {
                    let label_col = centre + 2;
                    grid.write_str(label_col, row, label);
                    row += 1;
                }

                need_connector = true;
            }

            ActivityStep::Else(label) => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                let text = if let Some(l) = label {
                    format!("[else: {l}]")
                } else {
                    "[else]".to_string()
                };
                let start_col = centre.saturating_sub(text.len() / 2);
                grid.write_str(start_col, row, &text);
                row += 1;
                need_connector = true;
            }

            ActivityStep::ElseIf(branch) => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                let text = format!("{} {}?", DIAMOND, branch.condition);
                let start_col = centre.saturating_sub(text.len() / 2);
                grid.write_str(start_col, row, &text);
                row += 1;
                need_connector = true;
            }

            ActivityStep::EndIf => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                need_connector = true;
            }

            ActivityStep::While(block) => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                let text = format!("{} {}?", DIAMOND, block.condition);
                let start_col = centre.saturating_sub(text.len() / 2);
                grid.write_str(start_col, row, &text);
                row += 1;
                need_connector = true;
            }

            ActivityStep::EndWhile(label) => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                if let Some(l) = label {
                    let start_col = centre + 2;
                    grid.write_str(start_col, row.saturating_sub(1), l);
                }
                need_connector = true;
            }

            ActivityStep::Fork | ActivityStep::Split => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                let bar_half = 5;
                let left = centre.saturating_sub(bar_half);
                grid.hline(left, row, bar_half * 2 + 1, H);
                row += 1;
                need_connector = true;
            }

            ActivityStep::ForkAgain | ActivityStep::SplitAgain => {
                grid.set(centre, row, V);
                row += 1;
                need_connector = true;
            }

            ActivityStep::EndFork | ActivityStep::EndSplit => {
                if need_connector {
                    grid.set(centre, row, V);
                    row += 1;
                }
                let bar_half = 5;
                let left = centre.saturating_sub(bar_half);
                grid.hline(left, row, bar_half * 2 + 1, H);
                row += 1;
                need_connector = true;
            }

            ActivityStep::Note(note) => {
                let note_col = centre + 4;
                let pos_str = match note.position {
                    NotePosition::Left => "left",
                    NotePosition::Right => "right",
                };
                let text = format!("[{pos_str}: {}]", note.text);
                grid.write_str(note_col, row.saturating_sub(1).max(0), &text);
            }

            ActivityStep::Swimlane(name) => {
                let text = format!("|{name}|");
                grid.write_str(0, row, &text);
                row += 1;
            }

            // Steps that don't produce visible output in simple ASCII mode.
            ActivityStep::Partition(_)
            | ActivityStep::EndPartition
            | ActivityStep::Arrow(_)
            | ActivityStep::DeprecatedColorAction(_)
            | ActivityStep::Backward(_)
            | ActivityStep::Break
            | ActivityStep::Detach
            | ActivityStep::Kill
            | ActivityStep::Repeat
            | ActivityStep::RepeatWhile(_)
            | ActivityStep::Switch(_)
            | ActivityStep::Case(_)
            | ActivityStep::EndSwitch => {}
        }
    }

    grid.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn make_diagram(steps: Vec<ActivityStep>) -> ActivityDiagram {
        ActivityDiagram {
            meta: DiagramMeta::default(),
            steps,
        }
    }

    #[test]
    fn simple_linear_flow() {
        let diagram = make_diagram(vec![
            ActivityStep::Start,
            ActivityStep::Action("Step 1".to_string()),
            ActivityStep::Action("Step 2".to_string()),
            ActivityStep::Stop,
        ]);
        let out = render_ascii(&diagram);
        assert!(out.contains('●'), "missing start symbol\n{out}");
        assert!(out.contains("Step 1"), "missing Step 1\n{out}");
        assert!(out.contains("Step 2"), "missing Step 2\n{out}");
        assert!(out.contains('◉'), "missing stop symbol\n{out}");
        assert!(out.contains('▼'), "missing arrow\n{out}");
    }

    #[test]
    fn if_else_flow() {
        let diagram = make_diagram(vec![
            ActivityStep::Start,
            ActivityStep::If(IfBlock {
                condition: "x > 0".to_string(),
                then_label: Some("yes".to_string()),
                source_line: 0,
            }),
            ActivityStep::Action("Positive".to_string()),
            ActivityStep::Else(Some("no".to_string())),
            ActivityStep::Action("Non-positive".to_string()),
            ActivityStep::EndIf,
            ActivityStep::Stop,
        ]);
        let out = render_ascii(&diagram);
        assert!(out.contains('◇'), "missing decision diamond\n{out}");
        assert!(out.contains("x > 0"), "missing condition\n{out}");
        assert!(out.contains("Positive"), "missing action\n{out}");
        assert!(out.contains("Non-positive"), "missing else action\n{out}");
    }

    #[test]
    fn empty_diagram() {
        let diagram = make_diagram(vec![]);
        assert_eq!(render_ascii(&diagram), "");
    }

    #[test]
    fn start_and_stop_only() {
        let diagram = make_diagram(vec![ActivityStep::Start, ActivityStep::Stop]);
        let out = render_ascii(&diagram);
        assert!(out.contains('●'), "missing start\n{out}");
        assert!(out.contains('◉'), "missing stop\n{out}");
    }

    #[test]
    fn action_boxes_have_borders() {
        let diagram = make_diagram(vec![
            ActivityStep::Start,
            ActivityStep::Action("Do something".to_string()),
            ActivityStep::Stop,
        ]);
        let out = render_ascii(&diagram);
        assert!(out.contains('┌'), "missing top-left corner\n{out}");
        assert!(out.contains('┘'), "missing bottom-right corner\n{out}");
        assert!(out.contains("Do something"), "missing action text\n{out}");
    }

    #[test]
    fn fork_produces_bars() {
        let diagram = make_diagram(vec![
            ActivityStep::Start,
            ActivityStep::Fork,
            ActivityStep::Action("Branch 1".to_string()),
            ActivityStep::ForkAgain,
            ActivityStep::Action("Branch 2".to_string()),
            ActivityStep::EndFork,
            ActivityStep::Stop,
        ]);
        let out = render_ascii(&diagram);
        assert!(out.contains('─'), "missing horizontal bar\n{out}");
        assert!(out.contains("Branch 1"), "missing branch 1\n{out}");
        assert!(out.contains("Branch 2"), "missing branch 2\n{out}");
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nstart\n:Do work;\nstop\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let out = crate::render_ascii(&diagram);
        assert!(out.contains('●'), "missing start\n{out}");
        assert!(out.contains("Do work"), "missing action\n{out}");
        assert!(out.contains('◉'), "missing stop\n{out}");
    }
}
