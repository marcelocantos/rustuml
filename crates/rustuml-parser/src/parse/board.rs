// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) parser — turns preprocessed @startboard lines into a
//! `BoardDiagram`.
//!
//! Syntax:
//! ```text
//! @startboard
//! Simple Kanban
//! + Backlog
//! * Task 1
//! * Task 2
//! + In Progress
//! * Task 3
//! + Done
//! * Task 4
//! @endboard
//! ```
//!
//! The first non-empty line is the title.  Lines starting with `+` define
//! columns; lines starting with `*` define cards in the current column.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::board::{BoardColumn, BoardDiagram};

/// Parse preprocessed lines from a `@startboard` block.
pub fn parse_board(lines: &[String]) -> Result<BoardDiagram, ParseError> {
    let mut meta = DiagramMeta::default();
    let mut title: Option<String> = None;
    let mut columns: Vec<BoardColumn> = Vec::new();

    for (line_no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('+') {
            // Column definition.  Syntax: `+Column Name+`.
            // Java PlantUML keeps the trailing `+` in the rendered label.
            let label = rest.trim().to_string();
            if label.is_empty() {
                return Err(ParseError {
                    line: line_no + 1,
                    message: "board column has no label".to_string(),
                });
            }
            columns.push(BoardColumn {
                label,
                cards: Vec::new(),
            });
        } else if let Some(rest) = trimmed.strip_prefix('*') {
            // Card definition.
            let card = rest.trim().to_string();
            if card.is_empty() {
                return Err(ParseError {
                    line: line_no + 1,
                    message: "board card has no text".to_string(),
                });
            }
            if let Some(col) = columns.last_mut() {
                col.cards.push(card);
            } else {
                return Err(ParseError {
                    line: line_no + 1,
                    message: "card without a preceding column".to_string(),
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            if let Some((key, value)) = rest.split_once(' ') {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
        } else if title.is_none() {
            // First non-empty, non-marker line is the title.
            title = Some(trimmed.to_string());
        }
        // Other lines are ignored (comments, etc.)
    }

    let title = title.unwrap_or_default();

    Ok(BoardDiagram {
        meta,
        title,
        columns,
    })
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
        let d = parse("Simple Kanban\n+ Backlog\n* Task 1\n* Task 2\n+ Done\n* Task 3");
        assert_eq!(d.title, "Simple Kanban");
        assert_eq!(d.columns.len(), 2);
        assert_eq!(d.columns[0].label, "Backlog");
        assert_eq!(d.columns[0].cards, vec!["Task 1", "Task 2"]);
        assert_eq!(d.columns[1].label, "Done");
        assert_eq!(d.columns[1].cards, vec!["Task 3"]);
    }

    #[test]
    fn empty_column() {
        let d = parse("Board\n+ Empty\n+ Has Card\n* Card 1");
        assert_eq!(d.columns.len(), 2);
        assert!(d.columns[0].cards.is_empty());
        assert_eq!(d.columns[1].cards.len(), 1);
    }

    #[test]
    fn card_without_column_error() {
        let err = parse_board(&lines("Title\n* Orphan")).unwrap_err();
        assert!(
            err.message.contains("without a preceding column"),
            "{}",
            err.message
        );
    }

    #[test]
    fn empty_column_label_error() {
        let err = parse_board(&lines("Title\n+")).unwrap_err();
        assert!(err.message.contains("no label"), "{}", err.message);
    }

    #[test]
    fn skips_empty_lines() {
        let d = parse("Title\n\n+ Col\n\n* Card");
        assert_eq!(d.columns.len(), 1);
        assert_eq!(d.columns[0].cards.len(), 1);
    }
}
