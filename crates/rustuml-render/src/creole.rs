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
    // Track the last non-markup character emitted so we can decide whether
    // `//` starts italic (only when preceded by whitespace or start of string).
    let mut last_char: Option<char> = None;

    while let Some(c) = chars.next() {
        match c {
            '~' => {
                // Tilde escape — next character is literal, not markup.
                if let Some(next) = chars.next() {
                    last_char = Some(next);
                    result.push(next);
                }
            }
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                // ""monospace"" — collect until closing ""
                let content = collect_until(&mut chars, "\"\"");
                let content = monospace_spaces(&content);
                write!(result, "<tspan font-family=\"monospace\">{content}</tspan>").unwrap();
                last_char = content.chars().last();
            }
            '`' => {
                // `code` backtick monospace
                let content = collect_until_char(&mut chars, '`');
                let content = monospace_spaces(&content);
                write!(result, "<tspan font-family=\"monospace\">{content}</tspan>").unwrap();
                last_char = content.chars().last();
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                // Find closing **; recursively process nested markup.
                let content = collect_until(&mut chars, "**");
                let inner = to_svg_tspans(&content);
                write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                last_char = content.chars().last();
            }
            '/' if chars.peek() == Some(&'/') => {
                // `//italic//` — but only start italic if preceded by whitespace
                // or at the start of the string. This prevents `http://url`
                // from being treated as italic markup.
                let preceded_by_word = last_char.map(|c| !c.is_whitespace()).unwrap_or(false);
                if preceded_by_word {
                    // Treat as two literal `/` characters.
                    result.push('/');
                    result.push('/');
                    chars.next(); // consume the second `/`
                    last_char = Some('/');
                } else {
                    chars.next();
                    let content = collect_until(&mut chars, "//");
                    let inner = to_svg_tspans(&content);
                    write!(result, "<tspan font-style=\"italic\">{inner}</tspan>").unwrap();
                    last_char = content.chars().last();
                }
            }
            '_' => {
                // Underscores are left as-is; PlantUML renders __text__ as literal
                // text in most contexts (class members, notes, etc.).
                result.push('_');
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                let content = collect_until(&mut chars, "--");
                let inner = to_svg_tspans(&content);
                write!(
                    result,
                    "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                )
                .unwrap();
            }
            '<' => {
                // HTML-style tags.
                let tag = collect_until_char(&mut chars, '>');
                match tag.as_str() {
                    "b" => {
                        let content = collect_until_tag(&mut chars, "</b>");
                        let inner = to_svg_tspans(&content);
                        write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                    }
                    "i" => {
                        let content = collect_until_tag(&mut chars, "</i>");
                        let inner = to_svg_tspans(&content);
                        write!(result, "<tspan font-style=\"italic\">{inner}</tspan>").unwrap();
                    }
                    "u" => {
                        let content = collect_until_tag(&mut chars, "</u>");
                        let inner = to_svg_tspans(&content);
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "s" => {
                        let content = collect_until_tag(&mut chars, "</s>");
                        let inner = to_svg_tspans(&content);
                        write!(
                            result,
                            "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "mono" => {
                        let content = collect_until_tag(&mut chars, "</mono>");
                        let inner = monospace_spaces(&to_svg_tspans(&content));
                        write!(result, "<tspan font-family=\"monospace\">{inner}</tspan>").unwrap();
                    }
                    "del" | "strike" => {
                        let close = if tag == "del" { "</del>" } else { "</strike>" };
                        let content = collect_until_tag(&mut chars, close);
                        let inner = to_svg_tspans(&content);
                        write!(
                            result,
                            "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "strong" => {
                        let content = collect_until_tag(&mut chars, "</strong>");
                        let inner = to_svg_tspans(&content);
                        write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                    }
                    "em" => {
                        let content = collect_until_tag(&mut chars, "</em>");
                        let inner = to_svg_tspans(&content);
                        write!(result, "<tspan font-style=\"italic\">{inner}</tspan>").unwrap();
                    }
                    "ins" => {
                        let content = collect_until_tag(&mut chars, "</ins>");
                        let inner = to_svg_tspans(&content);
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "sub" => {
                        let content = collect_until_tag(&mut chars, "</sub>");
                        let inner = to_svg_tspans(&content);
                        write!(
                            result,
                            "<tspan baseline-shift=\"sub\" font-size=\"0.7em\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "sup" => {
                        let content = collect_until_tag(&mut chars, "</sup>");
                        let inner = to_svg_tspans(&content);
                        write!(
                            result,
                            "<tspan baseline-shift=\"super\" font-size=\"0.7em\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    _ if tag.starts_with("color:") || tag.starts_with("COLOR:") => {
                        // <color:blue>text</color> — strip tag, keep inner markup processed.
                        let content = collect_until_tag(&mut chars, "</color>");
                        let inner = to_svg_tspans(&content);
                        result.push_str(&inner);
                    }
                    _ if tag.starts_with("size:") => {
                        // <size:N>text</size> — apply font-size.
                        let size_str = &tag["size:".len()..];
                        let content = collect_until_tag(&mut chars, "</size>");
                        let inner = to_svg_tspans(&content);
                        if let Ok(size) = size_str.parse::<u32>() {
                            write!(
                                result,
                                "<tspan font-size=\"{size}\">{inner}</tspan>"
                            )
                            .unwrap();
                        } else {
                            result.push_str(&inner);
                        }
                    }
                    _ if tag.starts_with("font:") || tag.starts_with("FONT:") => {
                        // <font:Name>text</font> — apply font-family as tspan, NBSP for spaces.
                        let font_name = if tag.starts_with("font:") {
                            &tag["font:".len()..]
                        } else {
                            &tag["FONT:".len()..]
                        };
                        let content = collect_until_tag(&mut chars, "</font>");
                        let inner = to_svg_tspans(&content);
                        let inner = inner.replace(' ', "\u{00a0}");
                        write!(
                            result,
                            "<tspan font-family=\"{font_name}\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    _ if tag.starts_with("font") => {
                        // <font ...>text</font> — strip tag, keep inner markup processed.
                        let content = collect_until_tag(&mut chars, "</font>");
                        let inner = to_svg_tspans(&content);
                        result.push_str(&inner);
                    }
                    _ if tag.starts_with("back:") || tag.starts_with("BACK:") => {
                        // <back:color>text</back> — strip tag, keep inner markup processed.
                        let content = collect_until_tag(&mut chars, "</back>");
                        let inner = to_svg_tspans(&content);
                        result.push_str(&inner);
                    }
                    _ if tag.starts_with('/') => {
                        // Unknown closing tag — emit escaped so text isn't silently dropped.
                        let escaped_tag = escape_creole_text(&tag);
                        write!(result, "&lt;{escaped_tag}&gt;").unwrap();
                    }
                    _ => {
                        // Unknown opening tag — escape and output as literal text.
                        let escaped_tag = escape_creole_text(&tag);
                        write!(result, "&lt;{escaped_tag}&gt;").unwrap();
                    }
                }
            }
            _ => {
                last_char = Some(c);
                result.push(c);
            }
        }
    }

    result
}

/// Convert ASCII spaces to non-breaking spaces (U+00A0) in monospace content,
/// matching PlantUML's SVG output for monospace text.
fn monospace_spaces(s: &str) -> String {
    s.replace(' ', "\u{00a0}")
}

/// Escape special XML characters in a string for safe embedding in SVG text content.
fn escape_creole_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
        // Underscores are left as literal text; PlantUML renders __text__ literally
        // in class members, notes, and other contexts.
        assert_eq!(to_svg_tspans("__underline__"), "__underline__");
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
