// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Salt (UI wireframe) diagram parser.
//!
//! Grammar overview:
//! ```text
//! salt      = block
//! block     = '{' modifier title? NL row* '}'
//! modifier  = '' | '#' | 'T' | '/' | 'SI' | '^' <title text>
//! row       = cell ('|' cell)* NL
//! cell      = widget | block
//! widget    = button | textfield | checkbox | radio | dropdown
//!           | separator | treenode | label
//! ```
//!
//! The preprocessor already strips `@startsalt`/`@endsalt`, so `lines`
//! contains only body content.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::salt::{
    BlockKind, SaltBlock, SaltDiagram, SaltRow, SaltWidget, SeparatorKind,
};

/// Parse preprocessed lines into a [`SaltDiagram`].
pub fn parse_salt(lines: &[String]) -> Result<SaltDiagram, ParseError> {
    // Find the first non-empty line — must be the opening brace.
    let start = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(0);

    if start >= lines.len() {
        return Err(ParseError {
            line: 1,
            message: "empty salt diagram".into(),
        });
    }

    if !lines[start].trim().starts_with('{') {
        return Err(ParseError {
            line: start + 1,
            message: format!(
                "expected '{{' to open Salt block, got: {:?}",
                lines[start].trim()
            ),
        });
    }

    let (block, _) = parse_block(lines, start)?;

    Ok(SaltDiagram {
        meta: DiagramMeta::default(),
        root: block,
    })
}

/// Parse a block starting at `lines[pos]` (which must begin with `{`).
///
/// Returns `(block, next_pos)` where `next_pos` points to the line after the
/// matching `}`.
fn parse_block(lines: &[String], pos: usize) -> Result<(SaltBlock, usize), ParseError> {
    let header = lines[pos].trim();

    // Parse block kind and optional title from the opening brace line.
    let (kind, title, inline_content) = parse_block_header(header);

    let mut pos = pos + 1;
    let mut rows: Vec<SaltRow> = vec![];
    // Cells being accumulated for the current row (may contain blocks).
    let mut current_cells: Vec<SaltWidget> = vec![];
    let mut row_in_progress = false;

    // If the header line has inline content (rare but valid), process it.
    if !inline_content.is_empty() {
        let cells = parse_row_part(inline_content);
        if !cells.is_empty() {
            rows.push(SaltRow { cells });
        }
    }

    while pos < lines.len() {
        let line = lines[pos].trim();

        if line.is_empty() {
            pos += 1;
            continue;
        }

        // Closing brace — end of this block.
        if line.starts_with('}') {
            // Flush any in-progress row.
            if !current_cells.is_empty() {
                rows.push(SaltRow {
                    cells: std::mem::take(&mut current_cells),
                });
            }
            pos += 1;
            return Ok((SaltBlock { kind, title, rows }, pos));
        }

        // Tree-node line — only meaningful inside `{T` blocks, but we parse
        // them anywhere so the model is consistent.
        if kind == BlockKind::Tree && line.starts_with('+') {
            // Flush any prior row.
            if row_in_progress {
                rows.push(SaltRow {
                    cells: std::mem::take(&mut current_cells),
                });
                row_in_progress = false;
            }
            let depth = line.chars().take_while(|&c| c == '+').count();
            let label = line[depth..].trim().to_string();
            rows.push(SaltRow {
                cells: vec![SaltWidget::TreeNode { depth, label }],
            });
            pos += 1;
            continue;
        }

        // Nested block start.
        if line.starts_with('{') {
            let (sub_block, new_pos) = parse_block(lines, pos)?;
            pos = new_pos;

            current_cells.push(SaltWidget::Block(Box::new(sub_block)));

            // Inspect the close-brace line (lines[new_pos - 1]) to detect `} |`.
            let close_line = lines[new_pos - 1].trim();
            let after_close = close_line.trim_start_matches('}').trim_start();

            if let Some(rest) = after_close.strip_prefix('|') {
                // More cells continue in the same row.
                let rest = rest.trim();
                if !rest.is_empty() {
                    current_cells.extend(parse_row_part(rest));
                }
                row_in_progress = true;
            } else {
                // Row is complete.
                rows.push(SaltRow {
                    cells: std::mem::take(&mut current_cells),
                });
                row_in_progress = false;
            }
            continue;
        }

        // Regular widget line — check for an inline block opener of the form
        // `cell | {BlockType`.  This handles patterns like:
        //   Description: | {SI
        //     content line
        //   }
        // where the block is embedded as a cell value rather than on its own
        // line.
        if let Some((prefix, block_header)) = find_inline_block(line) {
            // Parse cells that appear before the `|` that introduces the block.
            let before_cells: Vec<SaltWidget> = prefix
                .split('|')
                .flat_map(|p| parse_row_part(p.trim()))
                .collect();

            // Build a synthetic slice: `block_header` + the remaining original
            // lines starting at pos + 1, so that parse_block can consume the
            // block body and its closing `}` normally.
            let synthetic: Vec<String> = std::iter::once(block_header.to_string())
                .chain(lines[pos + 1..].iter().cloned())
                .collect();
            let (sub_block, sub_consumed) = parse_block(&synthetic, 0)?;

            // `synthetic[0]` corresponds to `lines[pos]` (same line, just
            // the block-header portion), and `synthetic[k]` corresponds to
            // `lines[pos + k]` for k > 0.  So consuming `sub_consumed` lines
            // from the synthetic slice advances `pos` by `sub_consumed`.
            pos = pos + sub_consumed;

            let mut cells = current_cells.drain(..).collect::<Vec<_>>();
            cells.extend(before_cells);
            cells.push(SaltWidget::Block(Box::new(sub_block)));
            rows.push(SaltRow { cells });
            row_in_progress = false;
            continue;
        }

        // Regular widget line.
        if row_in_progress {
            // A sub-block was the first cell; now regular widgets follow.
            current_cells.extend(parse_row_part(line));
            rows.push(SaltRow {
                cells: std::mem::take(&mut current_cells),
            });
            row_in_progress = false;
        } else {
            let cells = parse_row_line(line);
            if !cells.is_empty() {
                rows.push(SaltRow { cells });
            }
        }
        pos += 1;
    }

    // Ran out of lines without a closing brace — return what we have.
    if !current_cells.is_empty() {
        rows.push(SaltRow {
            cells: std::mem::take(&mut current_cells),
        });
    }

    Ok((SaltBlock { kind, title, rows }, pos))
}

/// Parse the opening brace line into `(kind, title, inline_content)`.
///
/// Examples:
/// - `{`         → (Plain, None, "")
/// - `{#`        → (Table, None, "")
/// - `{T`        → (Tree, None, "")
/// - `{/`        → (Tabs, None, "")
/// - `{SI`       → (ScrollInput, None, "")
/// - `{^My Title`→ (Plain, Some("My Title"), "")
/// - `{^My Title\n  content` — title parsing stops at end of token
fn parse_block_header(line: &str) -> (BlockKind, Option<String>, &str) {
    let rest = line.trim_start_matches('{');

    if let Some(title) = rest.strip_prefix('^') {
        return (BlockKind::Plain, Some(title.trim().to_string()), "");
    }
    if rest.starts_with('#') {
        return (BlockKind::Table, None, rest[1..].trim_start());
    }
    if rest.starts_with("SI") {
        return (BlockKind::ScrollInput, None, rest[2..].trim_start());
    }
    if rest.starts_with('T') && rest[1..].chars().next().map_or(true, |c| !c.is_alphanumeric()) {
        return (BlockKind::Tree, None, rest[1..].trim_start());
    }
    if rest.starts_with('/') {
        return (BlockKind::Tabs, None, rest[1..].trim_start());
    }
    // Plain block — any trailing content after `{` is inline content.
    (BlockKind::Plain, None, rest.trim_start())
}

/// Parse a full row line (possibly containing `|`-separated cells).
fn parse_row_line(line: &str) -> Vec<SaltWidget> {
    // Whole-line separators take precedence (no `|` splitting).
    if let Some(sep) = detect_separator(line) {
        return vec![SaltWidget::Separator(sep)];
    }

    let mut cells = Vec::new();
    for part in line.split('|') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            // An empty cell between `|` delimiters — skip.
            continue;
        }
        cells.extend(parse_row_part(trimmed));
    }
    cells
}

/// Parse a single cell's text content into one or more widgets.
///
/// A cell rarely contains multiple widgets, but a line like
/// `"field" | [OK]` will reach here already split, so each call
/// typically returns a single widget.
fn parse_row_part(part: &str) -> Vec<SaltWidget> {
    let trimmed = part.trim();

    if trimmed.is_empty() {
        return vec![];
    }

    // Whole-part separator.
    if let Some(sep) = detect_separator(trimmed) {
        return vec![SaltWidget::Separator(sep)];
    }

    // Checkbox: `[X] label` or `[ ] label`.
    if trimmed.starts_with("[X]") || trimmed.starts_with("[ ]") {
        let checked = trimmed.starts_with("[X]");
        let label = trimmed[3..].trim().to_string();
        return vec![SaltWidget::Checkbox { checked, label }];
    }

    // Radio: `(X) label` or `( ) label`.
    if trimmed.starts_with("(X)") || trimmed.starts_with("( )") {
        let selected = trimmed.starts_with("(X)");
        let label = trimmed[3..].trim().to_string();
        return vec![SaltWidget::Radio { selected, label }];
    }

    // Button: `[label]` — must not be checkbox (already handled above).
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let label = trimmed[1..trimmed.len() - 1].trim().to_string();
        return vec![SaltWidget::Button(label)];
    }

    // Text field: `"text"`.
    if trimmed.starts_with('"') {
        let inner = trimmed
            .trim_matches('"')
            .trim_end()
            .to_string();
        return vec![SaltWidget::TextField(inner)];
    }

    // Dropdown: `^label^`.
    if trimmed.starts_with('^') && trimmed.ends_with('^') && trimmed.len() > 1 {
        let label = trimmed[1..trimmed.len() - 1].trim().to_string();
        return vec![SaltWidget::Dropdown(label)];
    }

    // Default: plain label.
    vec![SaltWidget::Label(trimmed.to_string())]
}

/// Check whether a trimmed line contains an inline block opener of the form
/// `... | {Modifier`.  If so, returns `(prefix, block_header)` where
/// `prefix` is everything before the `|` that starts the block, and
/// `block_header` is the `{Modifier` token (possibly followed by content).
///
/// Only the *last* `| {` occurrence is considered, so that a row like
/// `A | B | {SI` is handled as `prefix = "A | B"`, `block_header = "{SI"`.
fn find_inline_block(line: &str) -> Option<(&str, &str)> {
    // Find the last '|' after which the trimmed remainder starts with '{'.
    let bytes = line.as_bytes();
    let mut last_pipe = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'|' {
            let after = line[i + 1..].trim_start();
            if after.starts_with('{') {
                last_pipe = Some(i);
            }
        }
    }
    let pipe_pos = last_pipe?;
    let prefix = line[..pipe_pos].trim_end();
    let block_header = line[pipe_pos + 1..].trim_start();
    Some((prefix, block_header))
}

/// Detect if a trimmed line is a separator; return the kind if so.
fn detect_separator(line: &str) -> Option<SeparatorKind> {
    // Must consist entirely of the separator character(s).
    if line == ".." || line.chars().all(|c| c == '.') && line.len() >= 2 {
        return Some(SeparatorKind::Dots);
    }
    if line.chars().all(|c| c == '=') && line.len() >= 2 {
        return Some(SeparatorKind::Double);
    }
    if line.chars().all(|c| c == '-') && line.len() >= 2 {
        return Some(SeparatorKind::Single);
    }
    if line == "_" {
        return Some(SeparatorKind::Solid);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(String::from).collect()
    }

    #[test]
    fn parse_basic_form() {
        let input = lines(
            r#"{
  Name: | "Alice"
  [Submit] | [Cancel]
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert_eq!(diag.root.kind, BlockKind::Plain);
        assert_eq!(diag.root.rows.len(), 2);
        // First row: Label("Name:"), TextField("Alice")
        let row0 = &diag.root.rows[0];
        assert!(matches!(&row0.cells[0], SaltWidget::Label(l) if l == "Name:"));
        assert!(matches!(&row0.cells[1], SaltWidget::TextField(_)));
        // Second row: Button("Submit"), Button("Cancel")
        let row1 = &diag.root.rows[1];
        assert!(matches!(&row1.cells[0], SaltWidget::Button(b) if b == "Submit"));
        assert!(matches!(&row1.cells[1], SaltWidget::Button(b) if b == "Cancel"));
    }

    #[test]
    fn parse_checkbox_radio() {
        let input = lines(
            r#"{
  [X] Option A
  [ ] Option B
  (X) Choice 1
  ( ) Choice 2
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert!(matches!(&diag.root.rows[0].cells[0], SaltWidget::Checkbox { checked: true, .. }));
        assert!(matches!(&diag.root.rows[1].cells[0], SaltWidget::Checkbox { checked: false, .. }));
        assert!(matches!(&diag.root.rows[2].cells[0], SaltWidget::Radio { selected: true, .. }));
        assert!(matches!(&diag.root.rows[3].cells[0], SaltWidget::Radio { selected: false, .. }));
    }

    #[test]
    fn parse_tree() {
        let input = lines(
            r#"{T
  + Files
++ src
+++ main.rs
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert_eq!(diag.root.kind, BlockKind::Tree);
        assert!(matches!(&diag.root.rows[0].cells[0], SaltWidget::TreeNode { depth: 1, .. }));
        assert!(matches!(&diag.root.rows[1].cells[0], SaltWidget::TreeNode { depth: 2, .. }));
        assert!(matches!(&diag.root.rows[2].cells[0], SaltWidget::TreeNode { depth: 3, .. }));
    }

    #[test]
    fn parse_table() {
        let input = lines(
            r#"{#
  Name | Age
  Alice | 30
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert_eq!(diag.root.kind, BlockKind::Table);
        assert_eq!(diag.root.rows.len(), 2);
    }

    #[test]
    fn parse_separator() {
        let input = lines(
            r#"{
  ..
  ===
  ---
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert!(matches!(&diag.root.rows[0].cells[0], SaltWidget::Separator(SeparatorKind::Dots)));
        assert!(matches!(&diag.root.rows[1].cells[0], SaltWidget::Separator(SeparatorKind::Double)));
        assert!(matches!(&diag.root.rows[2].cells[0], SaltWidget::Separator(SeparatorKind::Single)));
    }

    #[test]
    fn parse_group_title() {
        let input = lines(
            r#"{^My Group
  Name: | "Alice"
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert_eq!(diag.root.title.as_deref(), Some("My Group"));
    }

    #[test]
    fn parse_nested_blocks() {
        let input = lines(
            r#"{
  {
    [Button 1]
    [Button 2]
  } |
  {
    "Text field"
    "Another"
  }
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        // Should have one row with two block cells.
        assert_eq!(diag.root.rows.len(), 1);
        assert_eq!(diag.root.rows[0].cells.len(), 2);
        assert!(matches!(&diag.root.rows[0].cells[0], SaltWidget::Block(_)));
        assert!(matches!(&diag.root.rows[0].cells[1], SaltWidget::Block(_)));
    }
}
