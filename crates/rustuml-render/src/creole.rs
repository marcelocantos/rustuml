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
            '~' if chars.peek() == Some(&'~') => {
                // `~~wavy underline~~` — two tildes as delimiter.
                chars.next(); // consume second ~
                let (content, found) = collect_until(&mut chars, "~~");
                if found {
                    let inner = to_svg_tspans(&content);
                    write!(
                        result,
                        "<tspan text-decoration=\"wavy underline\">{inner}</tspan>"
                    )
                    .unwrap();
                    last_char = content.chars().last();
                } else {
                    result.push('~');
                    result.push('~');
                    result.push_str(&content);
                    last_char = content.chars().last().or(Some('~'));
                }
            }
            '~' => {
                // Tilde escape — if the next chars form a recognized multi-character
                // delimiter, escape the whole delimiter so it is emitted literally.
                let mut peeked = chars.clone().take(2);
                let c1 = peeked.next();
                let c2 = peeked.next();
                let is_two_char_delim = matches!(
                    (c1, c2),
                    (Some('/'), Some('/'))
                        | (Some('*'), Some('*'))
                        | (Some('-'), Some('-'))
                        | (Some('_'), Some('_'))
                );
                if is_two_char_delim {
                    let ch1 = chars.next().unwrap();
                    let ch2 = chars.next().unwrap();
                    result.push(ch1);
                    result.push(ch2);
                    last_char = Some(ch2);
                } else if let Some(next) = chars.next() {
                    last_char = Some(next);
                    result.push(next);
                }
            }
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                // ""monospace"" — collect until closing ""
                let (content, _found) = collect_until(&mut chars, "\"\"");
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
                let (content, found) = collect_until(&mut chars, "**");
                if found {
                    let inner = to_svg_tspans(&content);
                    write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                    last_char = content.chars().last();
                } else {
                    // No closing ** — emit opener literally and the collected text.
                    result.push('*');
                    result.push('*');
                    result.push_str(&content);
                    last_char = content.chars().last().or(Some('*'));
                }
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
                    let (content, found) = collect_until(&mut chars, "//");
                    if found && !content.is_empty() {
                        let inner = to_svg_tspans(&content);
                        write!(result, "<tspan font-style=\"italic\">{inner}</tspan>").unwrap();
                        last_char = content.chars().last();
                    } else if found && content.is_empty() {
                        // Empty italic `////` — treat as literal `////`.
                        result.push('/');
                        result.push('/');
                        result.push('/');
                        result.push('/');
                        last_char = Some('/');
                    } else {
                        result.push('/');
                        result.push('/');
                        result.push_str(&content);
                        last_char = content.chars().last().or(Some('/'));
                    }
                }
            }
            '_' if chars.peek() == Some(&'_') => {
                chars.next(); // consume second underscore
                let (content, found) = collect_until(&mut chars, "__");
                if found {
                    let inner = to_svg_tspans(&content);
                    write!(result, "<tspan text-decoration=\"underline\">{inner}</tspan>").unwrap();
                    last_char = content.chars().last();
                } else {
                    result.push('_');
                    result.push('_');
                    result.push_str(&content);
                    last_char = content.chars().last().or(Some('_'));
                }
            }
            '_' => {
                result.push('_');
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                let (content, found) = collect_until(&mut chars, "--");
                if found {
                    let inner = to_svg_tspans(&content);
                    write!(
                        result,
                        "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                    )
                    .unwrap();
                } else {
                    result.push('-');
                    result.push('-');
                    result.push_str(&content);
                    last_char = content.chars().last().or(Some('-'));
                }
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
                    "mono" | "code" => {
                        let close = if tag == "code" { "</code>" } else { "</mono>" };
                        let content = collect_until_tag(&mut chars, close);
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
                    _ if tag.starts_with("img:") => {
                        // <img:url> or <img:file> — render fallback text since we can't
                        // embed images in SVG tspan content.
                        let src = &tag["img:".len()..];
                        if src.starts_with("https://") || src.starts_with("http://") {
                            let escaped = escape_creole_text(src);
                            write!(result, "(Cannot decode: {escaped})").unwrap();
                        } else {
                            result.push_str("(Cannot decode)");
                        }
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
            '[' if chars.peek() == Some(&'[') => {
                // [[URL]] or [[URL label]] or [[URL|label]] — hyperlink.
                chars.next(); // consume second '['
                // Collect until closing ']]'.
                let mut inner = String::new();
                loop {
                    match chars.next() {
                        Some(']') if chars.peek() == Some(&']') => {
                            chars.next(); // consume second ']'
                            break;
                        }
                        Some(ch) => inner.push(ch),
                        None => break,
                    }
                }
                // Strip tooltip `{...}` if present (e.g. `URL{tooltip} label`).
                let inner = if let Some(brace) = inner.find('{') {
                    if let Some(end) = inner.find('}') {
                        format!("{}{}", &inner[..brace], &inner[end + 1..])
                    } else {
                        inner
                    }
                } else {
                    inner
                };
                // Parse: "URL label" or "URL|label" or "URL|label|tooltip"
                let (url, label) = if let Some(pos) = inner.find('|') {
                    (inner[..pos].trim().to_string(), inner[pos + 1..].split('|').next().unwrap_or("").trim().to_string())
                } else if let Some(pos) = inner.find(' ') {
                    (inner[..pos].to_string(), inner[pos + 1..].trim().to_string())
                } else {
                    (inner.clone(), inner.clone())
                };
                let display = if label.is_empty() { &url } else { &label };
                let escaped_url = escape_creole_text(&url);
                let escaped_label = escape_creole_text(display);
                write!(result, "<tspan fill=\"#0000FF\" text-decoration=\"underline\">{escaped_label}</tspan>").unwrap();
                let _ = escaped_url; // URL is not embedded in tspan SVG but recorded for accessibility
                last_char = display.chars().last();
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

/// Collect characters until a two-character delimiter is found.
///
/// Returns `(content, found)` where `found` is true iff the closing delimiter
/// was located.  If `found` is false, the caller should treat the opener as
/// literal text (otherwise unmatched markup would silently swallow content).
fn collect_until(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    delimiter: &str,
) -> (String, bool) {
    let mut buf = String::new();
    let delim_chars: Vec<char> = delimiter.chars().collect();
    while let Some(&c) = chars.peek() {
        if buf.ends_with(delim_chars[0]) && c == delim_chars[1] {
            chars.next();
            buf.pop(); // Remove the first char of delimiter.
            return (buf, true);
        }
        buf.push(c);
        chars.next();
    }
    (buf, false)
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

/// Per-level counters for numbered lists.  Reset when a non-list line is encountered.
#[derive(Default)]
pub struct ListCounters {
    counters: Vec<usize>,
}

impl ListCounters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render a single line, handling `#` (numbered) and `*` (bullet) list prefixes.
    ///
    /// The counter state is updated in place so successive `#` lines on the same
    /// nesting level produce sequential numbers.  A non-list line resets all counters.
    pub fn render_line(&mut self, line: &str) -> String {
        // Count leading '#' for numbered lists.
        let hash_level = line.chars().take_while(|&c| c == '#').count();
        if hash_level > 0 {
            let content = line[hash_level..].trim_start();
            // Grow counter vec to accommodate this level; deeper levels reset.
            if self.counters.len() > hash_level {
                self.counters.truncate(hash_level);
            }
            while self.counters.len() < hash_level {
                self.counters.push(0);
            }
            self.counters[hash_level - 1] += 1;
            let n = self.counters[hash_level - 1];
            let indent = "  ".repeat(hash_level - 1);
            let inner = to_svg_tspans(content);
            return format!("{indent}{n}. {inner}");
        }

        // Count leading '*' for bullet lists (but not '**' which is bold markup).
        let star_level = line
            .chars()
            .take_while(|&c| c == '*')
            .count();
        let next_after_stars = line[star_level..].chars().next();
        if star_level > 0 && next_after_stars != Some('*') {
            let content = line[star_level..].trim_start();
            let indent = "  ".repeat(star_level - 1);
            let inner = to_svg_tspans(content);
            self.counters.clear();
            return format!("{indent}\u{2022} {inner}");
        }

        // Not a list line — reset counters and process as inline markup.
        self.counters.clear();
        to_svg_tspans(line)
    }
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

    #[test]
    fn tilde_escapes_double_slash() {
        // ~/not italic/ — the `~` should escape `//` as a whole so italic is not triggered.
        assert_eq!(to_svg_tspans("~/not italic/"), "/not italic/");
    }

    #[test]
    fn tilde_escapes_double_star() {
        assert_eq!(to_svg_tspans("~**not bold**"), "**not bold**");
    }

    #[test]
    fn tilde_escapes_double_dash() {
        assert_eq!(to_svg_tspans("~--not strike--"), "--not strike--");
    }

    #[test]
    fn tilde_single_char() {
        // ~ before a single non-delimiter char just escapes that char.
        assert_eq!(to_svg_tspans("~x"), "x");
    }

    #[test]
    fn img_url_fallback() {
        assert_eq!(
            to_svg_tspans("<img:https://example.com/img.png>"),
            "(Cannot decode: https://example.com/img.png)"
        );
    }

    #[test]
    fn img_local_fallback() {
        assert_eq!(to_svg_tspans("<img:localfile.png>"), "(Cannot decode)");
    }

    #[test]
    fn monospace_double_quote() {
        assert_eq!(
            to_svg_tspans("\"\"mono legend text\"\""),
            "<tspan font-family=\"monospace\">mono\u{00a0}legend\u{00a0}text</tspan>"
        );
    }

    #[test]
    fn numbered_list_basic() {
        let mut lc = ListCounters::new();
        assert_eq!(lc.render_line("# First"), "1. First");
        assert_eq!(lc.render_line("# Second"), "2. Second");
        assert_eq!(lc.render_line("# Third"), "3. Third");
    }

    #[test]
    fn numbered_list_nested() {
        let mut lc = ListCounters::new();
        assert_eq!(lc.render_line("# Item"), "1. Item");
        assert_eq!(lc.render_line("## Sub"), "  1. Sub");
        assert_eq!(lc.render_line("## Sub2"), "  2. Sub2");
        assert_eq!(lc.render_line("# Item2"), "2. Item2");
    }

    #[test]
    fn numbered_list_resets_on_plain() {
        let mut lc = ListCounters::new();
        assert_eq!(lc.render_line("# First"), "1. First");
        assert_eq!(lc.render_line("plain"), "plain");
        assert_eq!(lc.render_line("# Restart"), "1. Restart");
    }

    #[test]
    fn bullet_list() {
        let mut lc = ListCounters::new();
        assert_eq!(lc.render_line("* item"), "\u{2022} item");
        assert_eq!(lc.render_line("** nested"), "  \u{2022} nested");
    }
}
