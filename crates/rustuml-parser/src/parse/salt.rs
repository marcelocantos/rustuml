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
use crate::diagram::salt::{BlockKind, SaltBlock, SaltDiagram, SaltRow, SaltWidget, SeparatorKind};

/// Parse preprocessed lines into a [`SaltDiagram`].
pub fn parse_salt(lines: &[String]) -> Result<SaltDiagram, ParseError> {
    // Salt diagrams still run the common single-line `header`/`footer`/`title`/
    // `caption` commands (PSystemSaltFactory registers them via
    // CommonCommands.addTitleCommands). Any body line — even one inside a
    // `{...}` block — that matches such a command is consumed before the salt
    // sub-parser sees it, so e.g. `Header 0 | Header 1` becomes the diagram
    // header `0 | Header 1` rather than the first table row. Strip these
    // directive lines out and record them in the diagram meta, keeping the
    // 1-indexed body line for the `data-source-line` attribute.
    let mut meta = DiagramMeta::default();
    let mut body: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if let Some(directive) = match_chrome_directive(line) {
            apply_chrome_directive(&mut meta, directive, i + 1);
            continue;
        }
        body.push(line.clone());
    }

    // Find the first non-empty line — must be the opening brace.
    let start = body.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);

    if start >= body.len() {
        return Err(ParseError {
            line: 1,
            message: "empty salt diagram".into(),
        });
    }

    if !body[start].trim().starts_with('{') {
        return Err(ParseError {
            line: start + 1,
            message: format!(
                "expected '{{' to open Salt block, got: {:?}",
                body[start].trim()
            ),
        });
    }

    let (block, _) = parse_block(&body, start)?;

    Ok(SaltDiagram { meta, root: block })
}

/// Which diagram-chrome command a body line matched, plus its parsed argument
/// and (for header/footer) horizontal alignment.
struct ChromeDirective {
    kind: ChromeKind,
    text: String,
    align: ChromeAlign,
}

#[derive(Clone, Copy)]
enum ChromeKind {
    Header,
    Footer,
    Title,
    Caption,
}

#[derive(Clone, Copy)]
enum ChromeAlign {
    Default,
    Left,
    Right,
    Center,
}

/// Recognise a salt body line as one of the common single-line chrome commands
/// (`header`/`footer`/`title`/`caption`), mirroring PlantUML's `CommandHeader`
/// family. The grammar is, case-insensitively:
///
/// ```text
/// ^\s*(left|right|center)?\s*<keyword>(\s*:\s*|\s+)<label>$
/// ```
///
/// where `<label>` is either a `"…"`/`'…'`-quoted string or any text containing
/// at least one letter, digit, `_` or `.` (the `[%pLN_.]` class). The position
/// prefix only applies to `header`/`footer`.
fn match_chrome_directive(line: &str) -> Option<ChromeDirective> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Optional leading alignment keyword (header/footer only — but harmless to
    // accept on title/caption; PlantUML's title/caption have no POSITION group,
    // so a leading `left ` etc. would simply not match. Keep it conservative.)
    let mut rest = trimmed;
    let mut align = ChromeAlign::Default;
    let mut has_position = false;
    for (kw, a) in [
        ("left", ChromeAlign::Left),
        ("right", ChromeAlign::Right),
        ("center", ChromeAlign::Center),
    ] {
        if let Some(after) = strip_keyword_ci(rest, kw) {
            rest = after.trim_start();
            align = a;
            has_position = true;
            break;
        }
    }

    let (kind, allow_position) = if let Some(after) = strip_keyword_ci(rest, "header") {
        rest = after;
        (ChromeKind::Header, true)
    } else if let Some(after) = strip_keyword_ci(rest, "footer") {
        rest = after;
        (ChromeKind::Footer, true)
    } else if let Some(after) = strip_keyword_ci(rest, "title") {
        rest = after;
        (ChromeKind::Title, false)
    } else if let Some(after) = strip_keyword_ci(rest, "caption") {
        rest = after;
        (ChromeKind::Caption, false)
    } else {
        return None;
    };

    // A position prefix is only valid before header/footer.
    if has_position && !allow_position {
        return None;
    }

    // Separator: either `:` (optionally space-padded) or one-or-more spaces.
    // PlantUML requires a separator, so a bare keyword with no argument (e.g.
    // a cell literally named "Title") is NOT a command — but those are handled
    // by the keyword strip below requiring a following separator/argument.
    let label_src = if let Some(after) = rest.strip_prefix(':') {
        after.trim()
    } else if rest.starts_with(char::is_whitespace) {
        rest.trim()
    } else {
        // No valid separator: keyword was glued to other text (e.g. "headerx").
        return None;
    };

    if label_src.is_empty() {
        return None;
    }

    // Label must be a quoted string or contain a letter/digit/`_`/`.`.
    let text = if (label_src.starts_with('"') && label_src.ends_with('"') && label_src.len() >= 2)
        || (label_src.starts_with('\'') && label_src.ends_with('\'') && label_src.len() >= 2)
    {
        label_src[1..label_src.len() - 1].to_string()
    } else if label_src
        .chars()
        .any(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        label_src.to_string()
    } else {
        return None;
    };

    Some(ChromeDirective { kind, text, align })
}

/// If `s` (after trimming leading whitespace) begins with `kw` case-insensitively
/// and the keyword is followed by a non-alphabetic boundary, return the
/// remainder after the keyword. Used to match command keywords without
/// swallowing words that merely start with them (e.g. `header` vs `headers`).
fn strip_keyword_ci<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    let s = s.trim_start();
    if s.len() < kw.len() {
        return None;
    }
    let (head, tail) = s.split_at(kw.len());
    if !head.eq_ignore_ascii_case(kw) {
        return None;
    }
    // The next char must not continue an identifier word.
    if let Some(c) = tail.chars().next()
        && c.is_alphabetic()
    {
        return None;
    }
    Some(tail)
}

fn apply_chrome_directive(meta: &mut DiagramMeta, d: ChromeDirective, line_1indexed: usize) {
    match d.kind {
        ChromeKind::Header => {
            meta.header = Some(d.text);
            meta.header_line = Some(line_1indexed);
            // Alignment is carried separately by the renderer's default (right);
            // a non-default explicit alignment is rare for salt and unused here.
            let _ = d.align;
        }
        ChromeKind::Footer => {
            meta.footer = Some(d.text);
            meta.footer_line = Some(line_1indexed);
        }
        ChromeKind::Title => {
            meta.title = Some(d.text);
            meta.title_line = Some(line_1indexed);
        }
        ChromeKind::Caption => {
            meta.caption = Some(d.text);
            meta.caption_line = Some(line_1indexed);
        }
    }
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
            pos += sub_consumed;

            let mut cells = std::mem::take(&mut current_cells);
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
    if let Some(stripped) = rest.strip_prefix('+') {
        return (BlockKind::Frame, None, stripped.trim_start());
    }
    if let Some(stripped) = rest.strip_prefix('#') {
        return (BlockKind::Table, None, stripped.trim_start());
    }
    if let Some(stripped) = rest.strip_prefix("SI") {
        return (BlockKind::ScrollInput, None, stripped.trim_start());
    }
    if let Some(stripped) = rest.strip_prefix("S-") {
        return (BlockKind::ScrollHorizontal, None, stripped.trim_start());
    }
    // `{S` (scroll, both bars). Only when the next char is not part of an
    // identifier (so `{Something` stays a plain block whose first widget is a
    // label).
    if rest.starts_with('S')
        && rest[1..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric())
    {
        return (BlockKind::Scroll, None, rest[1..].trim_start());
    }
    if rest.starts_with('T')
        && rest[1..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric())
    {
        return (BlockKind::Tree, None, rest[1..].trim_start());
    }
    if let Some(stripped) = rest.strip_prefix('/') {
        return (BlockKind::Tabs, None, stripped.trim_start());
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

    // Checkbox: `[X] label`, `[ ] label`, or `[] label`.
    if trimmed.starts_with("[X]") || trimmed.starts_with("[ ]") || trimmed.starts_with("[]") {
        let checked = trimmed.starts_with("[X]");
        let label_start = if trimmed.starts_with("[]") { 2 } else { 3 };
        let label = trimmed[label_start..].trim().to_string();
        return vec![SaltWidget::Checkbox { checked, label }];
    }

    // Radio: `(X) label`, `( ) label`, or `() label`.
    if trimmed.starts_with("(X)") || trimmed.starts_with("( )") || trimmed.starts_with("()") {
        let selected = trimmed.starts_with("(X)");
        let label_start = if trimmed.starts_with("()") { 2 } else { 3 };
        let label = trimmed[label_start..].trim().to_string();
        return vec![SaltWidget::Radio { selected, label }];
    }

    // Button: `[label]` — must not be checkbox (already handled above).
    // The label keeps its interior spacing: PlantUML counts the raw character
    // length (including padding spaces) when sizing the button, even though
    // the displayed text is trimmed.
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let label = trimmed[1..trimmed.len() - 1].to_string();
        return vec![SaltWidget::Button(label)];
    }

    // Text field: `"text"`.  Preserve the inner text verbatim (including
    // trailing spaces): PlantUML uses the raw character count for the
    // field's managed width, even though it trims the text for display.
    if trimmed.starts_with('"') && trimmed.len() >= 2 && trimmed.ends_with('"') {
        let inner = trimmed[1..trimmed.len() - 1].to_string();
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
  [] Option C
  (X) Choice 1
  ( ) Choice 2
  () Choice 3
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert!(matches!(
            &diag.root.rows[0].cells[0],
            SaltWidget::Checkbox { checked: true, .. }
        ));
        assert!(matches!(
            &diag.root.rows[1].cells[0],
            SaltWidget::Checkbox { checked: false, .. }
        ));
        assert!(matches!(
            &diag.root.rows[2].cells[0],
            SaltWidget::Checkbox { checked: false, .. }
        ));
        assert!(matches!(
            &diag.root.rows[3].cells[0],
            SaltWidget::Radio { selected: true, .. }
        ));
        assert!(matches!(
            &diag.root.rows[4].cells[0],
            SaltWidget::Radio {
                selected: false,
                ..
            }
        ));
        assert!(matches!(
            &diag.root.rows[5].cells[0],
            SaltWidget::Radio {
                selected: false,
                ..
            }
        ));
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
        assert!(matches!(
            &diag.root.rows[0].cells[0],
            SaltWidget::TreeNode { depth: 1, .. }
        ));
        assert!(matches!(
            &diag.root.rows[1].cells[0],
            SaltWidget::TreeNode { depth: 2, .. }
        ));
        assert!(matches!(
            &diag.root.rows[2].cells[0],
            SaltWidget::TreeNode { depth: 3, .. }
        ));
    }

    #[test]
    fn parse_scroll_kinds() {
        assert_eq!(
            parse_salt(&lines("{S\n  Item 1\n}")).unwrap().root.kind,
            BlockKind::Scroll
        );
        assert_eq!(
            parse_salt(&lines("{SI\n  Item 1\n}")).unwrap().root.kind,
            BlockKind::ScrollInput
        );
        assert_eq!(
            parse_salt(&lines("{S-\n  Item 1\n}")).unwrap().root.kind,
            BlockKind::ScrollHorizontal
        );
        // `{Something` must stay a plain block (not misread as scroll).
        let diag = parse_salt(&lines("{Stuff\n}")).unwrap();
        assert_eq!(diag.root.kind, BlockKind::Plain);
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
    fn header_command_consumes_first_row() {
        // A `{#` table whose first row matches the `header` command grammar:
        // the line is pulled out as the diagram header (case-insensitive `Header`),
        // leaving the table with just the body rows.
        let input = lines(
            r#"{#
  Header 0 | Header 1
  R0C0 | R0C1
  R1C0 | R1C1
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert_eq!(diag.root.kind, BlockKind::Table);
        // Only the two R*C* rows remain; the header row was consumed.
        assert_eq!(diag.root.rows.len(), 2);
        assert_eq!(diag.meta.header.as_deref(), Some("0 | Header 1"));
        // 1-indexed body line: {# is line 1, the header directive is line 2.
        assert_eq!(diag.meta.header_line, Some(2));
    }

    #[test]
    fn chrome_directives_extracted() {
        let input = lines(
            r#"{
  title My Title
  caption My Caption
  footer My Footer
  [OK]
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert_eq!(diag.meta.title.as_deref(), Some("My Title"));
        assert_eq!(diag.meta.caption.as_deref(), Some("My Caption"));
        assert_eq!(diag.meta.footer.as_deref(), Some("My Footer"));
        // Only the [OK] button row survives in the block.
        assert_eq!(diag.root.rows.len(), 1);
        assert!(matches!(&diag.root.rows[0].cells[0], SaltWidget::Button(b) if b == "OK"));
    }

    #[test]
    fn header_keyword_not_glued() {
        // A cell whose text merely starts with "header" (no separator) must not
        // be mistaken for the header command.
        let input = lines(
            r#"{
  headerline
}"#,
        );
        let diag = parse_salt(&input).unwrap();
        assert!(diag.meta.header.is_none());
        assert_eq!(diag.root.rows.len(), 1);
        assert!(matches!(&diag.root.rows[0].cells[0], SaltWidget::Label(l) if l == "headerline"));
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
        assert!(matches!(
            &diag.root.rows[0].cells[0],
            SaltWidget::Separator(SeparatorKind::Dots)
        ));
        assert!(matches!(
            &diag.root.rows[1].cells[0],
            SaltWidget::Separator(SeparatorKind::Double)
        ));
        assert!(matches!(
            &diag.root.rows[2].cells[0],
            SaltWidget::Separator(SeparatorKind::Single)
        ));
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
