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
            '~' => {
                // Tilde escape — next character is literal, not markup.
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            }
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                // ""monospace"" — collect until closing ""
                let content = collect_until(&mut chars, "\"\"");
                let content = monospace_spaces(&content);
                write!(result, "<tspan font-family=\"monospace\">{content}</tspan>").unwrap();
            }
            '`' => {
                // `code` backtick monospace
                let content = collect_until_char(&mut chars, '`');
                let content = monospace_spaces(&content);
                write!(result, "<tspan font-family=\"monospace\">{content}</tspan>").unwrap();
            }
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
                    "mono" => {
                        let content = collect_until_tag(&mut chars, "</mono>");
                        let content = monospace_spaces(&content);
                        write!(result, "<tspan font-family=\"monospace\">{content}</tspan>").unwrap();
                    }
                    "del" | "strike" => {
                        let close = if tag == "del" { "</del>" } else { "</strike>" };
                        let content = collect_until_tag(&mut chars, close);
                        write!(
                            result,
                            "<tspan text-decoration=\"line-through\">{content}</tspan>"
                        )
                        .unwrap();
                    }
                    "strong" => {
                        let content = collect_until_tag(&mut chars, "</strong>");
                        write!(result, "<tspan font-weight=\"bold\">{content}</tspan>").unwrap();
                    }
                    "em" => {
                        let content = collect_until_tag(&mut chars, "</em>");
                        write!(result, "<tspan font-style=\"italic\">{content}</tspan>").unwrap();
                    }
                    "ins" => {
                        let content = collect_until_tag(&mut chars, "</ins>");
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{content}</tspan>"
                        )
                        .unwrap();
                    }
                    "sub" => {
                        let content = collect_until_tag(&mut chars, "</sub>");
                        write!(
                            result,
                            "<tspan baseline-shift=\"sub\" font-size=\"0.7em\">{content}</tspan>"
                        )
                        .unwrap();
                    }
                    "sup" => {
                        let content = collect_until_tag(&mut chars, "</sup>");
                        write!(
                            result,
                            "<tspan baseline-shift=\"super\" font-size=\"0.7em\">{content}</tspan>"
                        )
                        .unwrap();
                    }
                    _ if tag.starts_with("color:") || tag.starts_with("COLOR:") => {
                        // <color:blue>text</color> — strip tag, keep content.
                        let content = collect_until_tag(&mut chars, "</color>");
                        result.push_str(&content);
                    }
                    _ if tag.starts_with("size:") => {
                        // <size:N>text</size> — apply font-size.
                        let size_str = &tag["size:".len()..];
                        let content = collect_until_tag(&mut chars, "</size>");
                        if let Ok(size) = size_str.parse::<u32>() {
                            write!(
                                result,
                                "<tspan font-size=\"{size}\">{content}</tspan>"
                            )
                            .unwrap();
                        } else {
                            result.push_str(&content);
                        }
                    }
                    _ if tag.starts_with("font") => {
                        // <font ...>text</font> — strip tag, keep content.
                        let content = collect_until_tag(&mut chars, "</font>");
                        result.push_str(&content);
                    }
                    _ if tag.starts_with("back:") || tag.starts_with("BACK:") => {
                        // <back:color>text</back> — strip tag, keep content.
                        let content = collect_until_tag(&mut chars, "</back>");
                        result.push_str(&content);
                    }
                    _ if tag.starts_with('/') => {
                        // Unknown closing tag — emit escaped so text isn't silently dropped.
                        write!(result, "&lt;{tag}&gt;").unwrap();
                    }
                    _ => {
                        // Unknown opening tag — escape and output as literal text.
                        write!(result, "&lt;{tag}&gt;").unwrap();
                    }
                }
            }
            _ => result.push(c),
        }
    }

    result
}

/// Convert ASCII spaces to non-breaking spaces (U+00A0) in monospace content,
/// matching PlantUML's SVG output for monospace text.
fn monospace_spaces(s: &str) -> String {
    s.replace(' ', "\u{00a0}")
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
