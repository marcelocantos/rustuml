// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Creole markup — converts PlantUML text markup to SVG tspan elements.
//!
//! Supports: **bold**, //italic//, __underline__, --strikethrough--,
//! and `<b>`, `<i>`, `<u>`, `<s>` HTML-style tags.

use std::fmt::Write;

/// Convert creole markup in text to SVG tspan elements.
///
/// Returns the text with creole markers replaced by appropriate SVG
/// tspan elements for styling.
pub fn to_svg_tspans(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                // Find closing **
                let content = collect_until(&mut chars, "**");
                write!(result, "<tspan font-weight=\"bold\">{content}</tspan>").unwrap();
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                let content = collect_until(&mut chars, "//");
                write!(result, "<tspan font-style=\"italic\">{content}</tspan>").unwrap();
            }
            '_' if chars.peek() == Some(&'_') => {
                chars.next();
                let content = collect_until(&mut chars, "__");
                write!(
                    result,
                    "<tspan text-decoration=\"underline\">{content}</tspan>"
                )
                .unwrap();
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                let content = collect_until(&mut chars, "--");
                write!(
                    result,
                    "<tspan text-decoration=\"line-through\">{content}</tspan>"
                )
                .unwrap();
            }
            '<' => {
                // HTML-style tags.
                let tag = collect_until_char(&mut chars, '>');
                match tag.as_str() {
                    "b" => {
                        let content = collect_until_tag(&mut chars, "</b>");
                        write!(result, "<tspan font-weight=\"bold\">{content}</tspan>").unwrap();
                    }
                    "i" => {
                        let content = collect_until_tag(&mut chars, "</i>");
                        write!(result, "<tspan font-style=\"italic\">{content}</tspan>").unwrap();
                    }
                    "u" => {
                        let content = collect_until_tag(&mut chars, "</u>");
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{content}</tspan>"
                        )
                        .unwrap();
                    }
                    "s" => {
                        let content = collect_until_tag(&mut chars, "</s>");
                        write!(
                            result,
                            "<tspan text-decoration=\"line-through\">{content}</tspan>"
                        )
                        .unwrap();
                    }
                    _ => {
                        // Not a recognized tag, output as-is.
                        write!(result, "<{tag}>").unwrap();
                    }
                }
            }
            _ => result.push(c),
        }
    }

    result
}

fn collect_until(chars: &mut std::iter::Peekable<std::str::Chars>, delimiter: &str) -> String {
    let mut buf = String::new();
    let delim_chars: Vec<char> = delimiter.chars().collect();
    while let Some(&c) = chars.peek() {
        if buf.ends_with(delim_chars[0]) && c == delim_chars[1] {
            chars.next();
            buf.pop(); // Remove the first char of delimiter.
            return buf;
        }
        buf.push(c);
        chars.next();
    }
    buf
}

fn collect_until_char(chars: &mut std::iter::Peekable<std::str::Chars>, end: char) -> String {
    let mut buf = String::new();
    for c in chars.by_ref() {
        if c == end {
            return buf;
        }
        buf.push(c);
    }
    buf
}

fn collect_until_tag(chars: &mut std::iter::Peekable<std::str::Chars>, tag: &str) -> String {
    let mut buf = String::new();
    while chars.peek().is_some() {
        let c = chars.next().unwrap();
        buf.push(c);
        if buf.ends_with(tag) {
            return buf[..buf.len() - tag.len()].to_string();
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold() {
        assert_eq!(
            to_svg_tspans("**bold**"),
            r#"<tspan font-weight="bold">bold</tspan>"#
        );
    }

    #[test]
    fn italic() {
        assert_eq!(
            to_svg_tspans("//italic//"),
            r#"<tspan font-style="italic">italic</tspan>"#
        );
    }

    #[test]
    fn underline() {
        assert_eq!(
            to_svg_tspans("__underline__"),
            r#"<tspan text-decoration="underline">underline</tspan>"#
        );
    }

    #[test]
    fn strikethrough() {
        assert_eq!(
            to_svg_tspans("--strike--"),
            r#"<tspan text-decoration="line-through">strike</tspan>"#
        );
    }

    #[test]
    fn html_bold() {
        assert_eq!(
            to_svg_tspans("<b>bold</b>"),
            r#"<tspan font-weight="bold">bold</tspan>"#
        );
    }

    #[test]
    fn mixed() {
        let result = to_svg_tspans("hello **bold** world");
        assert!(result.contains("hello "));
        assert!(result.contains(r#"font-weight="bold""#));
        assert!(result.contains(" world"));
    }

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(to_svg_tspans("hello world"), "hello world");
    }
}
