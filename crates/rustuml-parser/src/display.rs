// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Shared PlantUML `Display` source handling.

/// Natural alignment set by `\r` or `\l` in `Display.getWithNewlines`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayAlign {
    Left,
    Center,
    Right,
}

/// One PlantUML `Display`: rows after newline controls, plus an optional
/// natural alignment that overrides the style alignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayText {
    pub lines: Vec<String>,
    pub align: Option<DisplayAlign>,
}

/// `Display.getWithNewlines` (`Display.java:258-341`).
///
/// `Pragma.legacyReplaceBackslashNByNewline` is true. Outside `<math>`,
/// `<latex>`, and `[[...]]`, `\n`, `\r`, and `\l` end a row. `\r` selects
/// right alignment and `\l` selects left. `\\` becomes one backslash and
/// `\t` becomes a tab. A source with no backslash is returned unchanged.
pub fn display_with_newlines(source: &str) -> DisplayText {
    if !source.contains('\\') {
        return DisplayText {
            lines: vec![source.to_string()],
            align: None,
        };
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut align = None;
    let mut raw_mode = false;
    let mut chars = source.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        let rest = &source[offset..];
        if rest.starts_with("<math>") || rest.starts_with("<latex>") || rest.starts_with("[[") {
            raw_mode = true;
        } else if rest.starts_with("</math>")
            || rest.starts_with("</latex>")
            || rest.starts_with("]]")
        {
            raw_mode = false;
        }

        if !raw_mode && ch == '\\' {
            let Some((_, next)) = chars.next() else {
                current.push(ch);
                break;
            };
            match next {
                'n' | 'r' | 'l' => {
                    if next == 'r' {
                        align = Some(DisplayAlign::Right);
                    } else if next == 'l' {
                        align = Some(DisplayAlign::Left);
                    }
                    lines.push(std::mem::take(&mut current));
                }
                't' => current.push('\t'),
                '\\' => current.push('\\'),
                other => {
                    current.push('\\');
                    current.push(other);
                }
            }
            continue;
        }
        current.push(ch);
    }
    lines.push(current);
    DisplayText { lines, align }
}

/// Split literal `\n` row controls while preserving raw Creole blocks.
///
/// Java `Display.getWithNewlines` disables newline interpretation inside URL,
/// math, and LaTeX blocks so their domain parsers receive the complete token.
/// The returned rows borrow the original source and retain every byte other
/// than the outside-raw `\n` delimiters.
pub fn split_escaped_newlines(source: &str) -> Vec<&str> {
    let mut rows = Vec::new();
    let mut row_start = 0;
    let mut offset = 0;
    let mut raw_mode = false;

    while offset < source.len() {
        let rest = &source[offset..];
        if rest.starts_with("<math>") || rest.starts_with("<latex>") || rest.starts_with("[[") {
            raw_mode = true;
        } else if rest.starts_with("</math>")
            || rest.starts_with("</latex>")
            || rest.starts_with("]]")
        {
            raw_mode = false;
        }

        if !raw_mode && rest.starts_with("\\n") {
            rows.push(&source[row_start..offset]);
            offset += 2;
            row_start = offset;
            continue;
        }

        let ch = rest.chars().next().expect("offset is before source end");
        offset += ch.len_utf8();
    }

    rows.push(&source[row_start..]);
    rows
}

#[cfg(test)]
mod tests {
    use super::split_escaped_newlines;

    #[test]
    fn splits_controls_outside_raw_blocks() {
        assert_eq!(
            split_escaped_newlines("alpha\\nbeta\\ngamma"),
            ["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn preserves_url_math_and_latex_tokens() {
        let source = concat!(
            "pre [[https://example.com/a\\nb linked]] mid\\n",
            "<math>x\\ny</math> after\\n",
            "<latex>p\\nq</latex>"
        );
        assert_eq!(
            split_escaped_newlines(source),
            [
                "pre [[https://example.com/a\\nb linked]] mid",
                "<math>x\\ny</math> after",
                "<latex>p\\nq</latex>"
            ]
        );
    }

    #[test]
    fn newlines_split_outside_raw_blocks_and_keep_escapes() {
        use super::{DisplayAlign, display_with_newlines};

        let split = display_with_newlines("g5ckMidden\\nfold");
        assert_eq!(split.lines, ["g5ckMidden", "fold"]);
        assert_eq!(split.align, None);

        let aligned = display_with_newlines("left\\rright");
        assert_eq!(aligned.lines, ["left", "right"]);
        assert_eq!(aligned.align, Some(DisplayAlign::Right));

        let raw = display_with_newlines("pre [[https://example.com/a\\nb]]\\nafter");
        assert_eq!(raw.lines, ["pre [[https://example.com/a\\nb]]", "after"]);
        assert_eq!(display_with_newlines("a\\\\b").lines, ["a\\b"]);
        assert_eq!(display_with_newlines("plain").lines, ["plain"]);
    }

    #[test]
    fn unmatched_raw_block_keeps_the_remaining_source_together() {
        assert_eq!(
            split_escaped_newlines("before\\n[[https://example.com/a\\nb"),
            ["before", "[[https://example.com/a\\nb"]
        );
    }
}
