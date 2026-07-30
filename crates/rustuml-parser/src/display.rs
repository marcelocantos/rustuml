// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Shared PlantUML `Display` source handling.

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
    fn unmatched_raw_block_keeps_the_remaining_source_together() {
        assert_eq!(
            split_escaped_newlines("before\\n[[https://example.com/a\\nb"),
            ["before", "[[https://example.com/a\\nb"]
        );
    }
}
