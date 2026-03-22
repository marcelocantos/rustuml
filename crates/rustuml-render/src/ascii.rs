// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! ASCII art text renderer for sequence diagrams.
//!
//! Produces Unicode box-drawing output suitable for embedding in README files,
//! terminal output, and plain-text documents.

use rustuml_parser::diagram::sequence::*;

// Box-drawing characters.
const H: char = '─';
const V: char = '│';
const TL: char = '┌';
const TR: char = '┐';
const BL: char = '└';
const BR: char = '┘';
const T_DOWN: char = '┬';
const T_UP: char = '┴';
const ARROW_R: char = '>';
const ARROW_L: char = '<';

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

    /// Ensure grid has at least `row + 1` rows, filling with spaces.
    fn ensure_rows(&mut self, row: usize) {
        while self.cells.len() <= row {
            self.cells.push(vec![' '; self.width]);
        }
    }

    /// Set a character at (col, row), extending as needed.
    fn set(&mut self, col: usize, row: usize, ch: char) {
        if col >= self.width {
            return;
        }
        self.ensure_rows(row);
        self.cells[row][col] = ch;
    }

    /// Write a string at (col, row), clipping at grid width.
    fn write_str(&mut self, col: usize, row: usize, s: &str) {
        self.ensure_rows(row);
        for (i, ch) in s.chars().enumerate() {
            let c = col + i;
            if c < self.width {
                self.cells[row][c] = ch;
            }
        }
    }

    /// Draw a horizontal run of `ch` from col to col+len (exclusive).
    fn hline(&mut self, col: usize, row: usize, len: usize, ch: char) {
        for i in 0..len {
            self.set(col + i, row, ch);
        }
    }

    /// Draw a vertical run of `ch` from row to row+len (exclusive).
    fn vline(&mut self, col: usize, row: usize, len: usize, ch: char) {
        for i in 0..len {
            self.set(col, row + i, ch);
        }
    }

    /// Return the grid as a string with newlines.
    fn render(&self) -> String {
        let mut out = String::new();
        for row in &self.cells {
            // Trim trailing spaces.
            let trimmed: String = row.iter().collect::<String>().trim_end().to_string();
            out.push_str(&trimmed);
            out.push('\n');
        }
        out
    }
}

/// Layout constants (in character cells).
const PARTICIPANT_PADDING: usize = 1; // spaces inside box on each side
const LANE_GAP: usize = 10; // minimum gap between participant centres
const BOX_ROWS: usize = 3; // ┌───┐ / │...│ / └───┘ = 3 rows

/// Draw a participant box (3 rows tall) at (col, row) with given inner_width.
/// Returns the column of the box centre (the lifeline column).
fn draw_box(grid: &mut Grid, col: usize, row: usize, label: &str, inner_width: usize) {
    // Top: ┌───┐  or  ┌─┬─┐ style
    grid.set(col, row, TL);
    grid.hline(col + 1, row, inner_width + PARTICIPANT_PADDING * 2, H);
    grid.set(col + inner_width + PARTICIPANT_PADDING * 2 + 1, row, TR);

    // Middle: │ label │
    grid.set(col, row + 1, V);
    // Centre the label within the inner area.
    let total_inner = inner_width + PARTICIPANT_PADDING * 2;
    let pad_left = (total_inner.saturating_sub(label.len())) / 2;
    grid.write_str(col + 1 + pad_left, row + 1, label);
    grid.set(col + total_inner + 1, row + 1, V);

    // Bottom: └───┘
    grid.set(col, row + 2, BL);
    grid.hline(col + 1, row + 2, total_inner, H);
    grid.set(col + total_inner + 1, row + 2, BR);
}

/// Replace the centre character of a box edge with the T-junction for a lifeline.
/// For the top edge we use ┬, for the bottom edge ┴.
fn patch_lifeline_top(grid: &mut Grid, centre_col: usize, row: usize) {
    grid.set(centre_col, row, T_DOWN);
}

fn patch_lifeline_bottom(grid: &mut Grid, centre_col: usize, row: usize) {
    grid.set(centre_col, row, T_UP);
}

/// Compute a label padded with spaces to `width` chars, centred.
fn centre_label(label: &str, width: usize) -> String {
    if label.len() >= width {
        return label[..width].to_string();
    }
    let pad_left = (width - label.len()) / 2;
    let pad_right = width - label.len() - pad_left;
    format!("{}{}{}", " ".repeat(pad_left), label, " ".repeat(pad_right))
}

/// Compute the inner width (label area) for a participant box.
fn box_inner_width(label: &str) -> usize {
    label.len().max(3) // at least 3 chars wide
}

/// Compute box total width including borders.
fn box_total_width(label: &str) -> usize {
    box_inner_width(label) + PARTICIPANT_PADDING * 2 + 2 // 2 for left/right │
}

/// Render a sequence diagram as ASCII art.
pub fn render_ascii(diagram: &SequenceDiagram) -> String {
    if diagram.participants.is_empty() {
        return String::new();
    }

    // --- Phase 1: compute participant column positions ---
    // Each participant has a "centre column" (the lifeline column).
    // Boxes are placed so that the centre is at the middle of the label area.

    let n = diagram.participants.len();
    let inner_widths: Vec<usize> = diagram
        .participants
        .iter()
        .map(|p| box_inner_width(&p.label))
        .collect();

    // centre_col[i] = column of the lifeline (│) for participant i.
    let mut centre_cols: Vec<usize> = Vec::with_capacity(n);
    {
        let mut x: usize = 1; // leave 1 char margin on the left
        for i in 0..n {
            // The centre of the box is at x + 1 (border) + pad + inner/2
            let half_box = 1 + PARTICIPANT_PADDING + inner_widths[i] / 2;
            let centre = x + half_box;
            centre_cols.push(centre);
            x += box_total_width(&diagram.participants[i].label) + LANE_GAP;
        }
    }

    // Total grid width: rightmost box right edge + margin.
    let last_idx = n - 1;
    let last_box_right =
        centre_cols[last_idx] + inner_widths[last_idx] / 2 + PARTICIPANT_PADDING + 1 + 2; // +2 for right border + 1 margin
    let grid_width = last_box_right.max(80);

    // --- Phase 2: compute total row count ---
    // We'll lay out events first to know how many rows we need, then draw.

    // Rows: top boxes (BOX_ROWS) + events + bottom boxes (BOX_ROWS)
    // We do a two-pass layout: first count rows, then draw.

    // Count event rows (each message/divider/note/delay = 2 rows: label + arrow).
    let event_rows: Vec<usize> = count_event_rows(diagram);
    let total_event_rows: usize = event_rows.iter().sum();

    // Row layout:
    //   0..BOX_ROWS          — top participant boxes
    //   BOX_ROWS..BOX_ROWS+total_event_rows — events
    //   BOX_ROWS+total_event_rows..+BOX_ROWS — bottom participant boxes
    let bottom_box_start = BOX_ROWS + total_event_rows;
    let total_rows = bottom_box_start + BOX_ROWS;

    let mut grid = Grid::new(grid_width);
    grid.ensure_rows(total_rows);

    // --- Phase 3: draw top and bottom participant boxes ---
    for i in 0..n {
        let label = &diagram.participants[i].label;
        let inner_w = inner_widths[i];
        let half_inner = inner_w / 2;
        let box_left = centre_cols[i].saturating_sub(1 + PARTICIPANT_PADDING + half_inner);

        draw_box(&mut grid, box_left, 0, label, inner_w);
        patch_lifeline_top(&mut grid, centre_cols[i], 2); // row 2 = bottom of top box

        draw_box(&mut grid, box_left, bottom_box_start, label, inner_w);
        patch_lifeline_bottom(&mut grid, centre_cols[i], bottom_box_start); // top of bottom box
    }

    // --- Phase 4: draw lifelines ---
    for i in 0..n {
        let cx = centre_cols[i];
        // Lifeline spans from row BOX_ROWS to bottom_box_start (exclusive).
        grid.vline(cx, BOX_ROWS, total_event_rows, V);
    }

    // --- Phase 5: draw events ---
    let mut row = BOX_ROWS;
    let participant_index =
        |id: &str| -> Option<usize> { diagram.participants.iter().position(|p| p.id == id) };

    let mut autonumber: Option<u32> = diagram.autonumber.as_ref().map(|a| a.start);

    for (ei, event) in diagram.events.iter().enumerate() {
        let h = event_rows[ei];
        if h == 0 {
            continue;
        }
        match event {
            Event::Message(msg) => {
                let from_idx = participant_index(&msg.from);
                let to_idx = participant_index(&msg.to);

                let from_cx = from_idx.map(|i| centre_cols[i]).unwrap_or(0);
                let to_cx = to_idx.map(|i| centre_cols[i]).unwrap_or(grid_width - 1);

                // Label row: centre label above arrow.
                let label_row = row;
                let arrow_row = row + 1;

                // Build the label string (with optional autonumber prefix).
                let label = if let Some(ref mut num) = autonumber {
                    let s = format!("{} {}", num, msg.label);
                    let step = diagram.autonumber.as_ref().map(|a| a.step).unwrap_or(1);
                    *num += step;
                    s
                } else {
                    msg.label.clone()
                };

                // Determine the arrow char and direction.
                let dashed = msg.arrow.line == LineStyle::Dotted;
                let arrow_fill = if dashed { '-' } else { H };

                // Self-message: loops back.
                if msg.arrow.direction == ArrowDirection::Self_
                    || (from_idx == to_idx && from_idx.is_some())
                {
                    // Draw self-arrow as a small loop to the right.
                    let cx = from_cx;
                    let loop_right = cx + 4;
                    if !label.trim().is_empty() {
                        grid.write_str(cx + 2, label_row, &label);
                    }
                    // Arrow rows: down-right-up-left
                    grid.hline(cx + 1, label_row, 4, arrow_fill);
                    grid.set(loop_right, label_row, TR);
                    grid.set(loop_right, arrow_row, BR);
                    grid.hline(cx + 1, arrow_row, 3, arrow_fill);
                    grid.set(cx + 1, arrow_row, ARROW_L);
                    // Restore lifeline over the arrow char.
                    grid.set(cx, label_row, V);
                    grid.set(cx, arrow_row, V);
                } else {
                    // Visual direction is based on actual x positions, not the
                    // ArrowDirection enum (which reflects the literal arrow token).
                    let going_right = to_cx >= from_cx;
                    let (left_cx, right_cx) = if going_right {
                        (from_cx, to_cx)
                    } else {
                        (to_cx, from_cx)
                    };

                    // Place label centred between the two lifelines.
                    if !label.trim().is_empty() {
                        let span = right_cx.saturating_sub(left_cx);
                        let label_start = left_cx + 1 + span.saturating_sub(label.len()) / 2;
                        grid.write_str(label_start, label_row, &label);
                    }

                    // Draw arrow line: fill between the two lifeline columns,
                    // then place the arrowhead at the destination column.
                    if from_cx == to_cx {
                        // Same participant but not a Self_ arrow — treat as self.
                        // (Shouldn't normally happen, but guard against it.)
                    } else if going_right {
                        // from_cx < to_cx: line from from_cx+1 to to_cx-1, '>' at to_cx.
                        let len = to_cx - from_cx - 1;
                        grid.hline(from_cx + 1, arrow_row, len, arrow_fill);
                        grid.set(to_cx, arrow_row, ARROW_R);
                        // Restore source lifeline (it was never overwritten — left side of line).
                    } else {
                        // to_cx < from_cx: line from to_cx+1 to from_cx-1, '<' at to_cx.
                        let len = from_cx - to_cx - 1;
                        grid.hline(to_cx + 1, arrow_row, len, arrow_fill);
                        grid.set(to_cx, arrow_row, ARROW_L);
                        // Restore source lifeline at from_cx (right side).
                        grid.set(from_cx, arrow_row, V);
                    }
                }

                row += h;
            }

            Event::Divider(text) => {
                // Draw a horizontal rule spanning all participants with the text centred.
                let left = centre_cols[0];
                let right = centre_cols[n - 1];
                let span = right - left;
                let label_row = row;

                // Fill the full span with dashes, then place text.
                grid.hline(left, label_row, span + 1, '=');
                if !text.is_empty() {
                    let text_padded = format!(" {text} ");
                    let label_start = left + span.saturating_sub(text_padded.len()) / 2;
                    grid.write_str(label_start, label_row, &text_padded);
                }

                // Restore lifelines at their positions on the divider row.
                for i in 0..n {
                    grid.set(centre_cols[i], label_row, V);
                }

                row += h;
            }

            Event::Delay(text) => {
                if let Some(t) = text {
                    let left = centre_cols[0];
                    let right = centre_cols[n - 1];
                    let span = right - left;
                    let text_padded = format!("...{t}...");
                    let label_start = left + span.saturating_sub(text_padded.len()) / 2;
                    grid.write_str(label_start, row, &text_padded);
                }
                row += h;
            }

            Event::Note(note) => {
                // Draw a note box to the right/left/over a participant.
                let anchor_idx = note
                    .participants
                    .first()
                    .and_then(|id| participant_index(id))
                    .unwrap_or(0);
                let anchor_cx = centre_cols[anchor_idx];

                let text = &note.text;
                let note_inner = text.len().max(4);
                let note_width = note_inner + 4; // "| " + text + " |"

                let note_col = match note.position {
                    NotePosition::Right => anchor_cx + 2,
                    NotePosition::Left => anchor_cx.saturating_sub(note_width + 2),
                    NotePosition::Over => anchor_cx.saturating_sub(note_width / 2),
                };

                // Top border.
                grid.set(note_col, row, TL);
                grid.hline(note_col + 1, row, note_inner + 2, H);
                grid.set(note_col + note_inner + 3, row, TR);

                // Text row.
                grid.set(note_col, row + 1, V);
                grid.write_str(note_col + 2, row + 1, text);
                grid.set(note_col + note_inner + 3, row + 1, V);

                // Bottom border.
                grid.set(note_col, row + 2, BL);
                grid.hline(note_col + 1, row + 2, note_inner + 2, H);
                grid.set(note_col + note_inner + 3, row + 2, BR);

                // Restore lifelines on all three rows.
                for i in 0..n {
                    let cx = centre_cols[i];
                    for dr in 0..3usize {
                        let ch = grid.cells[row + dr][cx];
                        if ch == ' ' || ch == H {
                            grid.set(cx, row + dr, V);
                        }
                    }
                }

                row += h;
            }

            Event::GroupStart(g) => {
                // Draw a labelled bracket across the top.
                let left = centre_cols[0].saturating_sub(2);
                let right = centre_cols[n - 1] + 2;
                let kind_str = format!("{:?}", g.kind).to_lowercase();
                let label_text = match &g.label {
                    Some(l) => format!("[{kind_str}: {l}]"),
                    None => format!("[{kind_str}]"),
                };
                // Place label at left, draw bracket line.
                grid.hline(left, row, right - left + 1, H);
                grid.write_str(left + 1, row, &label_text);
                // Restore lifelines.
                for i in 0..n {
                    grid.set(centre_cols[i], row, V);
                }
                row += h;
            }

            Event::GroupElse(g) => {
                let left = centre_cols[0].saturating_sub(2);
                let right = centre_cols[n - 1] + 2;
                let label = g.label.as_deref().unwrap_or("else");
                grid.hline(left, row, right - left + 1, H);
                grid.write_str(left + 1, row, &format!("[{label}]"));
                for i in 0..n {
                    grid.set(centre_cols[i], row, V);
                }
                row += h;
            }

            Event::GroupEnd => {
                // Draw a closing bracket line.
                let left = centre_cols[0].saturating_sub(2);
                let right = centre_cols[n - 1] + 2;
                grid.hline(left, row, right - left + 1, H);
                for i in 0..n {
                    grid.set(centre_cols[i], row, V);
                }
                row += h;
            }

            Event::Ref(r) => {
                // Reference box spanning all participants.
                let left = centre_cols[0].saturating_sub(2);
                let text = format!("ref: {}", r.text);
                let width = text.len() + 4;
                grid.set(left, row, TL);
                grid.hline(left + 1, row, width - 2, H);
                grid.set(left + width - 1, row, TR);
                grid.set(left, row + 1, V);
                grid.write_str(left + 2, row + 1, &text);
                grid.set(left + width - 1, row + 1, V);
                grid.set(left, row + 2, BL);
                grid.hline(left + 1, row + 2, width - 2, H);
                grid.set(left + width - 1, row + 2, BR);
                // Restore lifelines.
                for i in 0..n {
                    let cx = centre_cols[i];
                    for dr in 0..3usize {
                        let ch = grid.cells[row + dr][cx];
                        if ch == ' ' || ch == H {
                            grid.set(cx, row + dr, V);
                        }
                    }
                }
                row += h;
            }

            Event::Return(ret) => {
                // Treat as a dotted message from the rightmost to leftmost.
                if n >= 2 {
                    let from_cx = centre_cols[n - 1];
                    let to_cx = centre_cols[0];
                    let label = &ret.label;

                    if !label.trim().is_empty() {
                        let span = from_cx.saturating_sub(to_cx);
                        let label_start = to_cx + span.saturating_sub(label.len()) / 2;
                        grid.write_str(label_start, row, label);
                    }
                    grid.set(to_cx, row + 1, ARROW_L);
                    grid.hline(to_cx + 1, row + 1, from_cx - to_cx - 1, '-');
                    grid.set(to_cx, row + 1, V);
                }
                row += h;
            }

            Event::NoteOnLink(text) => {
                let left = centre_cols[0];
                let right = centre_cols[n - 1];
                let span = right.saturating_sub(left);
                let padded = format!("[{text}]");
                let label_start = left + span.saturating_sub(padded.len()) / 2;
                grid.write_str(label_start, row, &padded);
                // Restore lifelines.
                for i in 0..n {
                    grid.set(centre_cols[i], row, V);
                }
                row += h;
            }

            Event::Space(px_opt) => {
                let extra = px_opt.map(|p| (p as usize / 8).max(1)).unwrap_or(1);
                // Lifelines are already drawn; just advance row.
                row += extra;
            }

            // Events that don't consume vertical space in ASCII mode.
            Event::Activate(_)
            | Event::Deactivate(_)
            | Event::Destroy(_)
            | Event::Create(_)
            | Event::NewPage(_) => {}
        }
    }

    // Trim the grid to the actual content (remove trailing empty rows).
    grid.render()
}

/// Return the number of rows each event consumes. Zero means "no space".
fn count_event_rows(diagram: &SequenceDiagram) -> Vec<usize> {
    diagram
        .events
        .iter()
        .map(|e| match e {
            Event::Message(msg) => {
                if msg.arrow.direction == ArrowDirection::Self_
                    || diagram.participants.iter().position(|p| p.id == msg.from)
                        == diagram.participants.iter().position(|p| p.id == msg.to)
                {
                    2 // label row + arrow row (loop stays within 2 rows)
                } else {
                    2
                }
            }
            Event::Divider(_) => 1,
            Event::Delay(_) => 1,
            Event::Note(_) => 3,
            Event::GroupStart(_) | Event::GroupElse(_) | Event::GroupEnd => 1,
            Event::Ref(_) => 3,
            Event::Return(_) => 2,
            Event::NoteOnLink(_) => 1,
            Event::Space(px_opt) => px_opt.map(|p| (p as usize / 8).max(1)).unwrap_or(1),
            Event::Activate(_)
            | Event::Deactivate(_)
            | Event::Destroy(_)
            | Event::Create(_)
            | Event::NewPage(_) => 0,
        })
        .collect()
}

/// Convenience: render with just the label, used for testing.
#[allow(dead_code)]
fn label_width(label: &str) -> usize {
    label.len()
}

/// Produces a centred label for display in fixed-width contexts.
#[allow(dead_code)]
pub fn centre_str(label: &str, width: usize) -> String {
    centre_label(label, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn make_diagram(participants: &[&str], events: Vec<Event>) -> SequenceDiagram {
        SequenceDiagram {
            meta: DiagramMeta::default(),
            participants: participants
                .iter()
                .enumerate()
                .map(|(i, &name)| Participant {
                    id: name.to_string(),
                    label: name.to_string(),
                    kind: ParticipantKind::Participant,
                    order: Some(i),
                })
                .collect(),
            events,
            autonumber: None,
        }
    }

    fn simple_message(from: &str, to: &str, label: &str) -> Event {
        Event::Message(Message {
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
            arrow: Arrow {
                line: LineStyle::Solid,
                head: ArrowHead::Filled,
                direction: ArrowDirection::LeftToRight,
            },
            activation: None,
        })
    }

    fn reply_message(from: &str, to: &str, label: &str) -> Event {
        Event::Message(Message {
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
            arrow: Arrow {
                line: LineStyle::Dotted,
                head: ArrowHead::Open,
                direction: ArrowDirection::RightToLeft,
            },
            activation: None,
        })
    }

    #[test]
    fn basic_hello_contains_participants() {
        let diagram = make_diagram(
            &["Alice", "Bob"],
            vec![simple_message("Alice", "Bob", "hello")],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("Alice"), "missing Alice\n{out}");
        assert!(out.contains("Bob"), "missing Bob\n{out}");
        assert!(out.contains("hello"), "missing label\n{out}");
    }

    #[test]
    fn basic_hello_has_arrow() {
        let diagram = make_diagram(
            &["Alice", "Bob"],
            vec![simple_message("Alice", "Bob", "hello")],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains('>'), "missing right arrow\n{out}");
    }

    #[test]
    fn boxes_have_corners() {
        let diagram = make_diagram(
            &["Alice", "Bob"],
            vec![simple_message("Alice", "Bob", "hello")],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains('┌'), "missing TL corner\n{out}");
        assert!(out.contains('┐'), "missing TR corner\n{out}");
        assert!(out.contains('└'), "missing BL corner\n{out}");
        assert!(out.contains('┘'), "missing BR corner\n{out}");
    }

    #[test]
    fn lifelines_present() {
        let diagram = make_diagram(
            &["Alice", "Bob"],
            vec![simple_message("Alice", "Bob", "hello")],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains('│'), "missing vertical lifeline\n{out}");
    }

    #[test]
    fn reply_message_has_arrow() {
        let diagram = make_diagram(&["Alice", "Bob"], vec![reply_message("Bob", "Alice", "hi")]);
        let out = render_ascii(&diagram);
        assert!(out.contains('<'), "missing left arrow for reply\n{out}");
        assert!(out.contains("hi"), "missing reply label\n{out}");
    }

    #[test]
    fn divider_rendered() {
        let diagram = make_diagram(
            &["Alice", "Bob"],
            vec![
                simple_message("Alice", "Bob", "hello"),
                Event::Divider("phase 2".to_string()),
                simple_message("Bob", "Alice", "bye"),
            ],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("phase 2"), "missing divider text\n{out}");
        assert!(out.contains('='), "missing divider line\n{out}");
    }

    #[test]
    fn note_rendered() {
        let diagram = make_diagram(
            &["Alice", "Bob"],
            vec![
                simple_message("Alice", "Bob", "hello"),
                Event::Note(Note {
                    position: NotePosition::Right,
                    participants: vec!["Bob".to_string()],
                    text: "a note".to_string(),
                }),
            ],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("a note"), "missing note text\n{out}");
    }

    #[test]
    fn three_participants() {
        let diagram = make_diagram(
            &["A", "B", "C"],
            vec![
                simple_message("A", "B", "step1"),
                simple_message("B", "C", "step2"),
            ],
        );
        let out = render_ascii(&diagram);
        assert!(out.contains("step1"), "missing step1\n{out}");
        assert!(out.contains("step2"), "missing step2\n{out}");
    }

    #[test]
    fn parsed_then_rendered_ascii() {
        let input = "@startuml\nAlice -> Bob : hello\nBob --> Alice : hi\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let out = crate::render_ascii(&diagram);
        assert!(out.contains("Alice"), "missing Alice\n{out}");
        assert!(out.contains("hello"), "missing hello\n{out}");
        assert!(out.contains("hi"), "missing hi\n{out}");
    }

    #[test]
    fn empty_diagram_returns_empty() {
        let diagram = SequenceDiagram {
            meta: DiagramMeta::default(),
            participants: vec![],
            events: vec![],
            autonumber: None,
        };
        assert_eq!(render_ascii(&diagram), "");
    }

    #[test]
    fn autonumber_prefix() {
        let mut diagram = make_diagram(
            &["Alice", "Bob"],
            vec![
                simple_message("Alice", "Bob", "first"),
                simple_message("Bob", "Alice", "second"),
            ],
        );
        diagram.autonumber = Some(AutoNumber {
            start: 1,
            step: 1,
            format: None,
        });
        let out = render_ascii(&diagram);
        assert!(out.contains("1 first"), "missing autonumbered label\n{out}");
        assert!(out.contains("2 second"), "missing second autonumber\n{out}");
    }
}
