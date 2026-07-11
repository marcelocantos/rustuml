// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) parser — turns preprocessed @startboard lines into a
//! `BoardDiagram`.
//!
//! Syntax:
//! ```text
//! @startboard
//! Board Title
//! +Backlog+
//! * Task 1
//! * Task 2
//! +Done+
//! * Task 3
//! @endboard
//! ```
//!
//! The first non-empty, non-marker line is the *root* box. Lines starting
//! with `+` are column headers; lines starting with `*` are cards. PlantUML
//! strips only the *leading* marker from each label (so `+Backlog+` renders
//! as `Backlog+`), then trims surrounding whitespace.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::board::{BoardDiagram, BoardItem, BoardItemKind};

/// Parse preprocessed lines from a `@startboard` block.
pub fn parse_board(lines: &[String]) -> Result<BoardDiagram, ParseError> {
    let mut meta = DiagramMeta::default();
    let mut items: Vec<BoardItem> = Vec::new();
    let mut saw_root = false;

    for (line_no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('+') {
            let label = rest.trim().to_string();
            if label.is_empty() {
                return Err(ParseError {
                    line: line_no + 1,
                    message: "board column has no label".to_string(),
                });
            }
            items.push(BoardItem {
                kind: BoardItemKind::Column,
                label,
            });
        } else if let Some(rest) = trimmed.strip_prefix('*') {
            let label = rest.trim().to_string();
            if label.is_empty() {
                return Err(ParseError {
                    line: line_no + 1,
                    message: "board card has no text".to_string(),
                });
            }
            items.push(BoardItem {
                kind: BoardItemKind::Card,
                label,
            });
        } else if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            if let Some((key, value)) = rest.split_once(' ') {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
        } else if !saw_root {
            // First non-empty, non-marker line is the root box.
            saw_root = true;
            items.push(BoardItem {
                kind: BoardItemKind::Root,
                label: trimmed.to_string(),
            });
        }
        // Other lines are ignored (comments, etc.)
    }

    Ok(BoardDiagram { meta, items })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    fn parse(s: &str) -> BoardDiagram {
        parse_board(&lines(s)).expect("parse failed")
    }

    #[test]
    fn simple_board() {
        let d = parse("Simple Kanban\n+Backlog+\n* Task 1\n* Task 2\n+Done+\n* Task 3");
        assert_eq!(d.items.len(), 6);
        assert_eq!(d.items[0].kind, BoardItemKind::Root);
        assert_eq!(d.items[0].label, "Simple Kanban");
        assert_eq!(d.items[1].kind, BoardItemKind::Column);
        assert_eq!(d.items[1].label, "Backlog+");
        assert_eq!(d.items[2].kind, BoardItemKind::Card);
        assert_eq!(d.items[2].label, "Task 1");
    }

    #[test]
    fn keeps_trailing_marker_in_label() {
        let d = parse("Board\n+Want to Read+\n* Item");
        assert_eq!(d.items[1].label, "Want to Read+");
    }

    #[test]
    fn empty_columns() {
        let d = parse("Board\n+Empty+\n+Has Card+\n* Card 1");
        assert_eq!(d.items.len(), 4);
        assert_eq!(d.items[1].kind, BoardItemKind::Column);
        assert_eq!(d.items[2].kind, BoardItemKind::Column);
        assert_eq!(d.items[3].kind, BoardItemKind::Card);
    }

    #[test]
    fn empty_column_label_error() {
        let err = parse_board(&lines("Title\n+")).unwrap_err();
        assert!(err.message.contains("no label"), "{}", err.message);
    }

    #[test]
    fn skips_empty_lines() {
        let d = parse("Title\n\n+Col+\n\n* Card");
        assert_eq!(d.items.len(), 3);
    }
}
