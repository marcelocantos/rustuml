// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Creole markup — converts PlantUML text markup to SVG tspan elements.
//!
//! Supports inline markup (**bold**, //italic//, __underline__, --strikethrough--,
//! `<b>`, `<i>`, `<u>`, `<s>`, `<w>` HTML-style tags) and line-level constructs:
//! tables (`|= Header | data |`), tree structures (`|_ node`),
//! horizontal rules (`----`, `====`, `....`), and nested lists (`*`, `**`, `#`).

use std::fmt::Write;

/// Convert creole markup in text to SVG tspan elements (standard mode).
///
/// Returns the text with creole markers replaced by appropriate SVG
/// tspan elements for styling.
pub fn to_svg_tspans(text: &str) -> String {
    to_svg_tspans_inner(text, false)
}

/// Convert creole markup in text to SVG tspan elements, treating `__` markers
/// as literal text rather than underline markup.
///
/// Use this for class-diagram entity labels where Java PlantUML does not
/// process `__` as underline.
pub fn to_svg_tspans_no_underline(text: &str) -> String {
    to_svg_tspans_inner(text, true)
}

/// Internal implementation shared by `to_svg_tspans` and `to_svg_tspans_no_underline`.
///
/// When `skip_underline` is true, `__...__` markers are emitted literally
/// instead of being converted to underline tspan elements.  This matches
/// Java PlantUML behaviour for class-diagram entity labels, where `__` is
/// not treated as underline markup.
fn to_svg_tspans_inner(text: &str, skip_underline: bool) -> String {
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
                    // Use escape_creole_text rather than recursive processing so
                    // that a `~` inside wavy-underline content (e.g. `~~~not strike~~~`)
                    // is not consumed by the tilde-escape handler and dropped.
                    let inner = escape_creole_text(&content);
                    write!(
                        result,
                        "<tspan text-decoration=\"wavy underline\">{inner}</tspan>"
                    )
                    .unwrap();
                    last_char = content.chars().last();
                } else {
                    result.push('~');
                    result.push('~');
                    result.push_str(&escape_creole_text(&content));
                    last_char = content.chars().last().or(Some('~'));
                }
            }
            '~' => {
                // Tilde escape — if the next chars form a recognized multi-character
                // delimiter, escape the whole delimiter so it is emitted literally.
                // If the next char is a single markup-start char (e.g. `/`, `*`,
                // `-`, `_`, `~`, `"`, `<`, `` ` ``), consume the `~` and emit the char.
                // Otherwise emit `~` literally and leave the next char for the main loop.
                let mut peeked = chars.clone().take(2);
                let c1 = peeked.next();
                let c2 = peeked.next();
                let is_two_char_delim = matches!(
                    (c1, c2),
                    (Some('/'), Some('/'))
                        | (Some('*'), Some('*'))
                        | (Some('-'), Some('-'))
                        | (Some('_'), Some('_'))
                        | (Some('"'), Some('"'))
                );
                if is_two_char_delim {
                    let ch1 = chars.next().unwrap();
                    let ch2 = chars.next().unwrap();
                    result.push(ch1);
                    result.push(ch2);
                    last_char = Some(ch2);
                } else if let Some(next_ch) = c1 {
                    // Single markup-start chars: consume `~` and emit the char.
                    // For non-markup chars: emit `~` literally, leave the char for the loop.
                    let is_markup_char =
                        matches!(next_ch, '/' | '*' | '-' | '_' | '~' | '"' | '<' | '`');
                    if is_markup_char {
                        chars.next(); // consume the markup char
                        if next_ch == '<' {
                            result.push_str("&lt;");
                        } else {
                            result.push(next_ch);
                        }
                        last_char = Some(next_ch);
                    } else {
                        // Non-markup char: tilde is literal.
                        result.push('~');
                        last_char = Some('~');
                        // Leave next_ch for the main loop to process.
                    }
                } else {
                    // Lone trailing tilde is literal.
                    result.push('~');
                    last_char = Some('~');
                }
            }
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                // ""monospace"" — collect until closing ""
                let (content, found) = collect_until(&mut chars, "\"\"");
                if found {
                    let content = monospace_spaces(&content);
                    write!(result, "<tspan font-family=\"monospace\">{content}</tspan>").unwrap();
                    last_char = content.chars().last();
                } else {
                    // Unterminated `""` (e.g. `default ""`): literal quotes.
                    result.push_str("\"\"");
                    result.push_str(&escape_creole_text(&content));
                    last_char = content.chars().last().or(Some('"'));
                }
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
                    let inner = to_svg_tspans_inner(&content, skip_underline);
                    write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                    last_char = content.chars().last();
                } else {
                    // No closing ** — emit opener literally and the collected text
                    // XML-escaped so raw `<tag>` content cannot break the SVG.
                    result.push('*');
                    result.push('*');
                    result.push_str(&escape_creole_text(&content));
                    last_char = content.chars().last().or(Some('*'));
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                // `//italic//` opens an italic run. PlantUML only suppresses
                // this when the `//` directly follows a URL scheme separator
                // (`http://`, `ftp://`, …) — i.e. the preceding char is `:`.
                // Otherwise `//` is italic markup even when glued to
                // punctuation (e.g. `process(//italicText//)`). The closing
                // `//` requirement (`found`) already keeps single-slash URLs
                // literal, so only the scheme prefix needs guarding.
                let preceded_by_scheme = last_char == Some(':');
                if preceded_by_scheme {
                    // Treat as two literal `/` characters.
                    result.push('/');
                    result.push('/');
                    chars.next(); // consume the second `/`
                    last_char = Some('/');
                } else {
                    chars.next();
                    let (content, found) = collect_until(&mut chars, "//");
                    if found && !content.is_empty() {
                        let inner = to_svg_tspans_inner(&content, skip_underline);
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
                        result.push_str(&escape_creole_text(&content));
                        last_char = content.chars().last().or(Some('/'));
                    }
                }
            }
            '_' if chars.peek() == Some(&'_') => {
                chars.next(); // consume second underscore
                if skip_underline {
                    // In class-diagram entity labels Java PlantUML treats `__` as
                    // literal characters, not underline markup.  Emit them as-is.
                    result.push('_');
                    result.push('_');
                    last_char = Some('_');
                } else {
                    let (content, found) = collect_until(&mut chars, "__");
                    if found {
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{inner}</tspan>"
                        )
                        .unwrap();
                        last_char = content.chars().last();
                    } else {
                        result.push('_');
                        result.push('_');
                        result.push_str(&escape_creole_text(&content));
                        last_char = content.chars().last().or(Some('_'));
                    }
                }
            }
            '_' => {
                result.push('_');
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                let (content, found) = collect_until(&mut chars, "--");
                if found {
                    let inner = to_svg_tspans_inner(&content, skip_underline);
                    write!(
                        result,
                        "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                    )
                    .unwrap();
                } else {
                    result.push('-');
                    result.push('-');
                    result.push_str(&escape_creole_text(&content));
                    last_char = content.chars().last().or(Some('-'));
                }
            }
            '<' => {
                // HTML-style tags.
                let tag = collect_until_char(&mut chars, '>');
                match tag.as_str() {
                    "b" => {
                        let content = collect_until_tag(&mut chars, "</b>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                    }
                    "i" => {
                        let content = collect_until_tag(&mut chars, "</i>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(result, "<tspan font-style=\"italic\">{inner}</tspan>").unwrap();
                    }
                    "u" => {
                        let content = collect_until_tag(&mut chars, "</u>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "s" => {
                        let content = collect_until_tag(&mut chars, "</s>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "mono" | "code" => {
                        let close = if tag == "code" { "</code>" } else { "</mono>" };
                        let content = collect_until_tag(&mut chars, close);
                        let inner =
                            monospace_spaces(&to_svg_tspans_inner(&content, skip_underline));
                        write!(result, "<tspan font-family=\"monospace\">{inner}</tspan>").unwrap();
                    }
                    "del" | "strike" => {
                        let close = if tag == "del" { "</del>" } else { "</strike>" };
                        let content = collect_until_tag(&mut chars, close);
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan text-decoration=\"line-through\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    // `<strong>`, `<em>`, `<ins>` are not part of PlantUML's
                    // HTML subset; they fall through to the unknown-tag arm
                    // and render literally.
                    "sub" => {
                        let content = collect_until_tag(&mut chars, "</sub>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan baseline-shift=\"sub\" font-size=\"0.7em\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    "sup" => {
                        let content = collect_until_tag(&mut chars, "</sup>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan baseline-shift=\"super\" font-size=\"0.7em\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
                    _ if tag.starts_with("color:") || tag.starts_with("COLOR:") => {
                        // <color:blue>text</color> — apply fill color via tspan.
                        let color = if let Some(s) = tag.strip_prefix("color:") {
                            s
                        } else {
                            tag.strip_prefix("COLOR:").unwrap_or("")
                        };
                        let content = collect_until_tag(&mut chars, "</color>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        let escaped_color = crate::svg::escape_attribute(color);
                        write!(result, "<tspan fill=\"{escaped_color}\">{inner}</tspan>").unwrap();
                    }
                    _ if tag.starts_with("size:") => {
                        // <size:N>text</size> — apply font-size.
                        let size_str = &tag["size:".len()..];
                        let content = collect_until_tag(&mut chars, "</size>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        if let Ok(size) = size_str.parse::<u32>() {
                            write!(result, "<tspan font-size=\"{size}\">{inner}</tspan>").unwrap();
                        } else {
                            result.push_str(&inner);
                        }
                    }
                    _ if tag.starts_with("font:") || tag.starts_with("FONT:") => {
                        // <font:Name>text</font> — apply font-family as tspan, NBSP for spaces.
                        let font_name = if let Some(s) = tag.strip_prefix("font:") {
                            s
                        } else {
                            tag.strip_prefix("FONT:").unwrap_or("")
                        };
                        let content = collect_until_tag(&mut chars, "</font>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        let inner = inner.replace(' ', "\u{00a0}");
                        write!(
                            result,
                            "<tspan font-family=\"{font_name}\">{inner}</tspan>",
                            font_name = crate::svg::font_family_attribute(font_name),
                        )
                        .unwrap();
                    }
                    _ if tag.starts_with("font") => {
                        // <font ...>text</font> — strip tag, keep inner markup processed.
                        let content = collect_until_tag(&mut chars, "</font>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        result.push_str(&inner);
                    }
                    _ if tag.starts_with("back:") || tag.starts_with("BACK:") => {
                        // <back:color>text</back> — strip tag, keep inner markup processed.
                        let content = collect_until_tag(&mut chars, "</back>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        result.push_str(&inner);
                    }
                    _ if tag.starts_with("img:") => {
                        // <img:url> or <img:file> — render fallback text in monospace,
                        // matching Java PlantUML's SVG output. Non-breaking spaces
                        // (U+00A0) are used throughout, consistent with monospace text
                        // handling elsewhere.
                        let raw_src = &tag["img:".len()..];
                        let src = if let Some(brace) = raw_src.find('{') {
                            &raw_src[..brace]
                        } else {
                            raw_src
                        };
                        let fallback = if src.starts_with("https://") || src.starts_with("http://")
                        {
                            let escaped = escape_creole_text(src);
                            format!("(Cannot\u{00a0}decode:\u{00a0}{escaped})")
                        } else {
                            "(Cannot\u{00a0}decode)".to_string()
                        };
                        write!(
                            result,
                            "<tspan font-family=\"monospace\">{fallback}</tspan>"
                        )
                        .unwrap();
                    }
                    _ if tag.starts_with("&amp;") || tag.starts_with('&') => {
                        // OpenIconic icon reference `<&name>` — strip silently.
                        // Actual rendering is handled at the SVG builder level via
                        // segment-based text rendering (text_with_icons).
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
                    (
                        inner[..pos].trim().to_string(),
                        inner[pos + 1..]
                            .split('|')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                    )
                } else if let Some(pos) = inner.find(' ') {
                    (
                        inner[..pos].to_string(),
                        inner[pos + 1..].trim().to_string(),
                    )
                } else {
                    (inner.clone(), inner.clone())
                };
                let display = if label.is_empty() { &url } else { &label };
                let escaped_url = escape_creole_text(&url);
                let escaped_label = escape_creole_text(display);
                write!(
                    result,
                    "<tspan fill=\"#0000FF\" text-decoration=\"underline\">{escaped_label}</tspan>"
                )
                .unwrap();
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
pub fn escape_creole_text(s: &str) -> String {
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

/// Like [`collect_until_char`] but reports whether the terminator was
/// actually reached. Returns `(consumed, found_end)` — callers that need to
/// fall back to literal text when the input had no matching `>` should use
/// this variant. The `consumed` portion does NOT include the terminator.
fn collect_until_char_checked(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    end: char,
) -> (String, bool) {
    let mut buf = String::new();
    for c in chars.by_ref() {
        if c == end {
            return (buf, true);
        }
        buf.push(c);
    }
    (buf, false)
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

// ---------------------------------------------------------------------------
// Line-level constructs: lists, tables, trees, horizontal rules
// ---------------------------------------------------------------------------

/// A single cell in a creole table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCell {
    /// The text content of the cell (before inline markup processing).
    pub text: String,
    /// Whether this cell is a header cell (`|=`).
    pub is_header: bool,
    /// Optional background colour from `<#color>` prefix.
    pub bg_color: Option<String>,
}

/// A parsed creole table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// The style of a horizontal rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalRuleStyle {
    /// `----` (single line)
    Single,
    /// `====` (double line)
    Double,
    /// `....` (dotted line)
    Dotted,
}

/// A parsed tree node from `|_` syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// Nesting depth (1 for `|_`, 2 for `|__`, etc.).
    pub depth: usize,
    /// The text content of the node.
    pub text: String,
}

/// Classification of a creole line after parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreoleLine {
    /// A plain text line (may contain inline markup).
    Text(String),
    /// A bullet list item with nesting level (1 = `*`, 2 = `**`, etc.).
    Bullet { level: usize, content: String },
    /// A numbered list item with nesting level (1 = `#`, 2 = `##`, etc.).
    Numbered { level: usize, content: String },
    /// A table row.
    Table(TableRow),
    /// A tree node.
    Tree(TreeNode),
    /// A horizontal rule.
    HorizontalRule(HorizontalRuleStyle),
}

/// Parse a single line into its creole line-level construct.
///
/// This does NOT process inline markup — call `to_svg_tspans()` on the text
/// content of the returned variant for inline processing.
pub fn parse_line(line: &str) -> CreoleLine {
    let trimmed = line.trim();

    // Horizontal rules: 4+ repeated chars of the same kind.
    if trimmed.len() >= 4 {
        if trimmed.chars().all(|c| c == '-') {
            return CreoleLine::HorizontalRule(HorizontalRuleStyle::Single);
        }
        if trimmed.chars().all(|c| c == '=') {
            return CreoleLine::HorizontalRule(HorizontalRuleStyle::Double);
        }
        if trimmed.chars().all(|c| c == '.') {
            return CreoleLine::HorizontalRule(HorizontalRuleStyle::Dotted);
        }
    }

    // Tree syntax: `|_`, `|__`, `|___`, etc.
    if let Some(rest) = trimmed.strip_prefix('|') {
        let underscore_count = rest.chars().take_while(|&c| c == '_').count();
        if underscore_count > 0 {
            let text = rest[underscore_count..].trim().to_string();
            return CreoleLine::Tree(TreeNode {
                depth: underscore_count,
                text,
            });
        }
        // Could be a table row — check below.
    }

    // Table row: starts with `|` and has at least one more `|`.
    if trimmed.starts_with('|')
        && trimmed.len() > 1
        && let Some(row) = parse_table_row(trimmed)
    {
        return CreoleLine::Table(row);
    }

    // Numbered list: `#`, `##`, etc.
    let hash_level = trimmed.chars().take_while(|&c| c == '#').count();
    if hash_level > 0 {
        let content = trimmed[hash_level..].trim_start().to_string();
        return CreoleLine::Numbered {
            level: hash_level,
            content,
        };
    }

    // Bullet list: `*`, `**`, `***`, etc. followed by whitespace or end of line.
    // Distinguish from bold markup: `**text**` is bold, `** text` is a level-2 bullet.
    let star_level = trimmed.chars().take_while(|&c| c == '*').count();
    if star_level > 0 {
        let after_stars = &trimmed[star_level..];
        let next_ch = after_stars.chars().next();
        let is_bullet = matches!(next_ch, None | Some(' ') | Some('\t'));
        if is_bullet {
            let content = after_stars.trim_start().to_string();
            return CreoleLine::Bullet {
                level: star_level,
                content,
            };
        }
    }

    CreoleLine::Text(trimmed.to_string())
}

/// Parse a table row string like `|= Header1 |= Header2 |` or `| cell1 | cell2 |`.
///
/// Returns `None` if the line doesn't look like a valid table row.
pub fn parse_table_row(line: &str) -> Option<TableRow> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }

    // Split on `|` and process each segment.
    let segments: Vec<&str> = trimmed.split('|').collect();
    // First segment is always empty (before first `|`).
    if segments.len() < 3 {
        return None;
    }

    let mut cells = Vec::new();
    // Skip first empty segment; the last segment may be empty (trailing `|`).
    for &seg in &segments[1..] {
        let seg = seg.trim();
        if seg.is_empty() {
            // Trailing `|` produces empty last segment — skip it.
            continue;
        }

        let is_header = seg.starts_with('=');
        let text_part = if is_header { seg[1..].trim() } else { seg };

        // Check for background colour: `<#color>` prefix.
        let (bg_color, text) = if text_part.starts_with("<#") {
            if let Some(end) = text_part.find('>') {
                let color = text_part[2..end].to_string();
                let rest = text_part[end + 1..].trim().to_string();
                (Some(color), rest)
            } else {
                (None, text_part.to_string())
            }
        } else {
            (None, text_part.to_string())
        };

        cells.push(TableCell {
            text,
            is_header,
            bg_color,
        });
    }

    if cells.is_empty() {
        return None;
    }

    Some(TableRow { cells })
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

    /// Render a single line, handling `#` (numbered), `*`/`**`/`***` (bullet) list
    /// prefixes, tree nodes (`|_`), horizontal rules, and table rows.
    ///
    /// The counter state is updated in place so successive `#` lines on the same
    /// nesting level produce sequential numbers.  A non-list line resets all counters.
    pub fn render_line(&mut self, line: &str) -> String {
        match parse_line(line) {
            CreoleLine::Numbered { level, content } => {
                // Grow counter vec to accommodate this level; deeper levels reset.
                if self.counters.len() > level {
                    self.counters.truncate(level);
                }
                while self.counters.len() < level {
                    self.counters.push(0);
                }
                self.counters[level - 1] += 1;
                let n = self.counters[level - 1];
                let indent = "  ".repeat(level - 1);
                let inner = to_svg_tspans(&content);
                format!("{indent}{n}. {inner}")
            }
            CreoleLine::Bullet { level, content } => {
                self.counters.clear();
                let indent = "  ".repeat(level - 1);
                let inner = to_svg_tspans(&content);
                format!("{indent}\u{2022} {inner}")
            }
            CreoleLine::Tree(node) => {
                self.counters.clear();
                let indent = "  ".repeat(node.depth.saturating_sub(1));
                let inner = to_svg_tspans(&node.text);
                if indent.is_empty() {
                    inner
                } else {
                    format!("{indent}{inner}")
                }
            }
            CreoleLine::HorizontalRule(style) => {
                self.counters.clear();
                match style {
                    HorizontalRuleStyle::Single => "\u{2500}".repeat(20),
                    HorizontalRuleStyle::Double => "\u{2550}".repeat(20),
                    HorizontalRuleStyle::Dotted => "\u{2508}".repeat(20),
                }
            }
            CreoleLine::Table(row) => {
                self.counters.clear();
                // Render table row as pipe-delimited text with inline markup processed.
                let mut parts = Vec::new();
                for cell in &row.cells {
                    let inner = to_svg_tspans(&cell.text);
                    if cell.is_header {
                        parts.push(format!("<tspan font-weight=\"bold\">{inner}</tspan>"));
                    } else {
                        parts.push(inner);
                    }
                }
                parts.join(" | ")
            }
            CreoleLine::Text(text) => {
                self.counters.clear();
                to_svg_tspans(&text)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Segment-based parsing
//
// PlantUML emits styled text as separate `<text>` elements rather than nested
// `<tspan>` runs — each contiguous run of uniform style becomes its own text
// element, with the style lifted to attributes on the element itself. The
// renderers consume `parse_segments` to drive that output shape.
// ---------------------------------------------------------------------------

/// Resolved style for a contiguous run of text. `None` fields inherit from
/// the enclosing `<text>` element.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub wavy_underline: bool,
    pub line_through: bool,
    pub monospace: bool,
    /// SVG `fill` attribute when overridden by `<color:...>`.
    pub fill: Option<String>,
    /// Non-default alpha byte from HColorSet RRGGBBAA; absent means opaque.
    /// This renderer-only model is not serialized in diagram YAML/JSON.
    pub fill_opacity: Option<u8>,
    /// Absolute font-size override from `<size:N>` (pixels).
    pub size: Option<u32>,
    /// Custom font-family from `<font:Name>` (only used when not monospace).
    pub font_family: Option<String>,
    /// `"sub"` or `"super"` from `<sub>` / `<sup>`.
    pub baseline_shift: Option<&'static str>,
    /// Hyperlink target from `[[url label]]`; carries blue underline styling.
    pub link_url: Option<String>,
    /// Optional hyperlink tooltip from `[[url{tooltip} label]]`; becomes the
    /// `<a>` element's `title` / `xlink:title`. Falls back to the URL when
    /// absent.
    pub link_title: Option<String>,
    /// Background colour from `<back:color>...</back>`, normalised to upper-
    /// case `#RRGGBB`. The renderer emits an SVG `<filter>` per unique value
    /// and references it via `filter="url(#...)"` on the matching `<text>`.
    pub background: Option<String>,
}

/// A contiguous run of uniformly-styled text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// XML-escaped text content. Spaces are already converted to NBSP for
    /// monospace runs.
    pub text: String,
    pub style: Style,
}

/// Parse creole markup into a sequence of uniformly-styled segments.
///
/// The renderer chooses how to emit them: a single segment may be lifted to
/// `<text>` attributes; multiple segments split into separate `<text>`
/// elements at calculated x offsets.
pub fn parse_segments(text: &str) -> Vec<Segment> {
    parse_segments_inner(text, false)
}

/// Variant that treats `__` markers as literal text rather than underline
/// markup. Use in class-diagram entity labels where Java PlantUML does not
/// process `__` as underline.
pub fn parse_segments_no_underline(text: &str) -> Vec<Segment> {
    parse_segments_inner(text, true)
}

/// Concatenate the unstyled text of all segments — the string PlantUML uses
/// for textLength calculation when measuring a creole-marked label.
pub fn stripped_text(text: &str) -> String {
    parse_segments(text)
        .into_iter()
        .map(|s| unescape_for_metrics(&s.text))
        .collect()
}

/// Like [`stripped_text`] but treats `__` as literal underscores (matching
/// contexts — class entity/link labels — where `__` is not underline markup).
pub fn stripped_text_no_underline(text: &str) -> String {
    parse_segments_no_underline(text)
        .into_iter()
        .map(|s| unescape_for_metrics(&s.text))
        .collect()
}

/// Reverse the XML escaping applied during segment building so the result
/// matches the source string a font-metric calculation expects.
fn unescape_for_metrics(s: &str) -> String {
    crate::text_render::single_pass_unescape(s)
}

fn parse_segments_inner(text: &str, skip_underline: bool) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let style = Style::default();
    walk_segments(text, &style, skip_underline, &mut out);
    // StripeSimple.modifyStripe flushes pending text before every command,
    // even when addSprite finds no sprite. Empty segments are private barriers
    // against push_segment's same-style coalescing, not drawable atoms.
    let had_barriers = out.iter().any(|segment| segment.text.is_empty());
    out.retain(|segment| !segment.text.is_empty());
    if had_barriers && out.is_empty() {
        // StripeSimple.getAtoms supplies one space for an otherwise empty
        // stripe. Size/color commands restore the surrounding configuration
        // after recursively analyzing their content (including EOL forms).
        push_literal(&mut out, " ", &style);
    }
    out
}

fn is_sprite_tag(tag: &str) -> bool {
    // Java Splitter.spritePattern + scaleOrColor, SpriteUtils.SPRITE_NAME,
    // and Pattern2.compileInternal (ASCII case-insensitive keywords/\w).
    // Match the complete tag, not merely a <$ prefix: malformed commands and
    // ordinary angle-bracket text remain literal. No sprite lookup lives here;
    // sprite-aware consumers resolve their images before shared text parsing.
    static SPRITE_TAG: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"\A(?:#[A-Za-z0-9_]+)?\$[-\p{L}0-9_/]+(?:[\{,]?(?:(?:(?i-u:scale=)|\*)[0-9.]+)?(?:,?(?i-u:color)[= :](?:#[0-9a-fA-F]{1,8}|[A-Za-z0-9_]+))?\}?)?\z",
        )
        .expect("Java sprite command grammar")
    });
    SPRITE_TAG.is_match(tag)
}

fn push_segment(out: &mut Vec<Segment>, text: &str, style: &Style) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut()
        && last.style == *style
    {
        last.text.push_str(text);
        return;
    }
    out.push(Segment {
        text: text.to_string(),
        style: style.clone(),
    });
}

/// Emit a literal-character run, applying XML escaping and (for monospace)
/// space-to-NBSP conversion.
fn push_literal(out: &mut Vec<Segment>, text: &str, style: &Style) {
    if text.is_empty() {
        return;
    }
    let escaped = escape_creole_text(&restore_hidden_characters(text));
    let final_text = if style.monospace {
        escaped.replace(' ', "\u{00a0}")
    } else {
        escaped
    };
    push_segment(out, &final_text, style);
}

// CharHidder.hideChar/unhideChar reserve E000..E0FF for hidden syntax.
// Keep that source-defined representation through matching and recursion;
// AtomText restores it only when constructing a literal text atom.
const HIDDEN_CHARACTER_BASE: u32 = 0xE000;

fn hide_characters(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'~') {
            chars.next();
            result.push(char::from_u32(HIDDEN_CHARACTER_BASE + '~' as u32).unwrap());
        } else if c == '~'
            && let Some(next) = chars.next()
        {
            // CharHidder.isToBeHidden: consume exactly one following character,
            // even when it is the first half of an overlapping delimiter.
            if matches!(
                next,
                '_' | '-' | '"' | '#' | ']' | '[' | '*' | '.' | '/' | '<'
            ) {
                result.push(char::from_u32(HIDDEN_CHARACTER_BASE + next as u32).unwrap());
            } else {
                result.push(c);
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn restore_hidden_characters(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (HIDDEN_CHARACTER_BASE..=HIDDEN_CHARACTER_BASE + u8::MAX as u32)
                .contains(&(c as u32))
            {
                char::from_u32(c as u32 - HIDDEN_CHARACTER_BASE).unwrap()
            } else {
                c
            }
        })
        .collect()
}

/// CommandCreoleStyle.createCreole requires at least one captured character
/// before a closing delimiter. A failed candidate leaves input unconsumed so
/// StripeSimple's literal-character fallback can reach subsequent commands.
fn take_style_delimiter(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    delimiter: &str,
) -> Option<String> {
    // The first opener character was read by walk_segments; the second is
    // still pending. Delimiters are existing two-character ASCII commands.
    let remaining: String = chars.clone().skip(1).collect();
    for (offset, _) in remaining.char_indices() {
        if offset > 0 && remaining[offset..].starts_with(delimiter) {
            chars.next();
            for _ in remaining[..offset + delimiter.len()].chars() {
                chars.next();
            }
            return Some(remaining[..offset].to_string());
        }
    }
    None
}

fn walk_segments(text: &str, style: &Style, skip_underline: bool, out: &mut Vec<Segment>) {
    // StripeSimple.analyzeAndAdd applies the same hide operation on every
    // recursive entry. Hidden candidates remain inert until literal emission.
    let hidden = hide_characters(text);
    let mut chars = hidden.chars().peekable();
    let mut buf = String::new();
    let mut last_char: Option<char> = None;

    macro_rules! flush_buf {
        () => {
            if !buf.is_empty() {
                push_literal(out, &buf, style);
                buf.clear();
            }
        };
    }

    while let Some(c) = chars.next() {
        match c {
            '~' if chars.peek() == Some(&'~') => {
                if let Some(content) = take_style_delimiter(&mut chars, "~~") {
                    flush_buf!();
                    let mut nested = style.clone();
                    nested.wavy_underline = true;
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    walk_segments(&content, &nested, skip_underline, out);
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    last_char = content.chars().last();
                } else {
                    buf.push('~');
                    last_char = Some('~');
                }
            }
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                let (content, found) = collect_until(&mut chars, "\"\"");
                flush_buf!();
                if found {
                    let mut nested = style.clone();
                    nested.monospace = true;
                    walk_segments(&content, &nested, skip_underline, out);
                    last_char = content.chars().last();
                } else {
                    // Unterminated `""` (e.g. `default ""`): literal quotes,
                    // not an open monospace run.
                    push_literal(out, "\"\"", style);
                    push_literal(out, &content, style);
                    last_char = content.chars().last().or(Some('"'));
                }
            }
            '`' => {
                let content = collect_until_char(&mut chars, '`');
                flush_buf!();
                let mut nested = style.clone();
                nested.monospace = true;
                walk_segments(&content, &nested, skip_underline, out);
                last_char = content.chars().last();
            }
            '*' if chars.peek() == Some(&'*') => {
                if let Some(content) = take_style_delimiter(&mut chars, "**") {
                    flush_buf!();
                    let mut nested = style.clone();
                    nested.bold = true;
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    walk_segments(&content, &nested, skip_underline, out);
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    last_char = content.chars().last();
                } else {
                    buf.push('*');
                    last_char = Some('*');
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                // Preserve the existing caller's URL-scheme exclusion.
                if last_char == Some(':') {
                    buf.push_str("//");
                    chars.next();
                    last_char = Some('/');
                } else if let Some(content) = take_style_delimiter(&mut chars, "//") {
                    flush_buf!();
                    let mut nested = style.clone();
                    nested.italic = true;
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    walk_segments(&content, &nested, skip_underline, out);
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    last_char = content.chars().last();
                } else {
                    buf.push('/');
                    last_char = Some('/');
                }
            }
            '_' if chars.peek() == Some(&'_') => {
                if skip_underline {
                    chars.next();
                    buf.push_str("__");
                    last_char = Some('_');
                } else if let Some(content) = take_style_delimiter(&mut chars, "__") {
                    flush_buf!();
                    let mut nested = style.clone();
                    nested.underline = true;
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    walk_segments(&content, &nested, skip_underline, out);
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    last_char = content.chars().last();
                } else {
                    buf.push('_');
                    last_char = Some('_');
                }
            }
            '_' => {
                buf.push('_');
                last_char = Some('_');
            }
            '-' if chars.peek() == Some(&'-') => {
                if let Some(content) = take_style_delimiter(&mut chars, "--") {
                    flush_buf!();
                    let mut nested = style.clone();
                    nested.line_through = true;
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    walk_segments(&content, &nested, skip_underline, out);
                    out.push(Segment {
                        text: String::new(),
                        style: style.clone(),
                    });
                    last_char = content.chars().last();
                } else {
                    buf.push('-');
                    last_char = Some('-');
                }
            }
            '<' => {
                // StripeSimple.searchCommand/modifyStripe: an unrecognized
                // candidate consumes only one literal character. In particular,
                // an outer unknown `<` must not hide an inner sprite command.
                let mut candidate = chars.clone();
                let (tag, found_close) = collect_until_char_checked(&mut candidate, '>');
                if found_close {
                    flush_buf!();
                }
                if found_close && handle_tag(&tag, &mut candidate, style, skip_underline, out) {
                    chars = candidate;
                    last_char = None;
                } else {
                    buf.push('<');
                    last_char = Some('<');
                }
            }
            '[' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut inner = String::new();
                loop {
                    match chars.next() {
                        Some(']') if chars.peek() == Some(&']') => {
                            chars.next();
                            break;
                        }
                        Some(ch) => inner.push(ch),
                        None => break,
                    }
                }
                // A `{tooltip}` segment becomes the link's title attribute and
                // is stripped before URL/label parsing.
                let mut tooltip: Option<String> = None;
                let inner = if let Some(brace) = inner.find('{') {
                    if let Some(end) = inner.find('}') {
                        tooltip = Some(inner[brace + 1..end].to_string());
                        format!("{}{}", &inner[..brace], &inner[end + 1..])
                    } else {
                        inner
                    }
                } else {
                    inner
                };
                let (url, label) = if let Some(pos) = inner.find('|') {
                    (
                        inner[..pos].trim().to_string(),
                        inner[pos + 1..]
                            .split('|')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                    )
                } else if let Some(pos) = inner.find(' ') {
                    (
                        inner[..pos].to_string(),
                        inner[pos + 1..].trim().to_string(),
                    )
                } else {
                    (inner.clone(), inner.clone())
                };
                let display = if label.is_empty() { &url } else { &label };
                flush_buf!();
                let mut nested = style.clone();
                nested.link_url = Some(url.clone());
                nested.link_title = tooltip;
                nested.fill = Some("#0000FF".to_string());
                nested.fill_opacity = None;
                nested.underline = true;
                push_literal(out, display, &nested);
                last_char = display.chars().last();
            }
            _ => {
                buf.push(c);
                last_char = Some(c);
            }
        }
    }
    flush_buf!();
}

/// Candidate recognition runs on CharHidder's stream, before consuming a body.
/// Splitter supplies the size/color/family/image/icon grammars; the combined
/// font command and BACKCOLOR use their own patterns (not Splitter.fontPattern).
enum ParameterCommand<'a> {
    Size(&'a str),
    Color(&'a str),
    Font {
        size: Option<&'a str>,
        color: Option<&'a str>,
    },
    Family(&'a str),
    Background(&'a str),
    Image(&'a str),
    Icon,
}

fn parameter_command(tag: &str) -> Option<ParameterCommand<'_>> {
    static PATTERNS: std::sync::LazyLock<[regex::Regex; 8]> = std::sync::LazyLock::new(|| {
        // Pattern2 uses ASCII CASE_INSENSITIVE, \d and \w. Its [%s] also
        // includes NBSP; [%g] includes curly quotes and Jaws' invisible quote.
        let space = r"[\t\n\x0B\x0C\r \u{00A0}]";
        let separator = r"[\t\n\x0B\x0C\r :]";
        let quote = r#"["“”\u{E121}]"#;
        let attribute = format!(
            r"{space}+(?:(?i-u:size){space}*={space}*{quote}?([0-9]+){quote}?|(?i-u:color){space}*={space}*{quote}?(#[0-9a-fA-F]{{6}}|[A-Za-z0-9_]+){quote}?)"
        );
        [
            format!(r"\A(?i-u:size){separator}+([0-9]+){space}*\z"),
            format!(r"\A(?i-u:color){separator}+(#[0-9a-fA-F]{{1,6}}|#?[A-Za-z0-9_]+){space}*\z"),
            format!(r"\A(?i-u:font)(?:{attribute})+{space}*\z"),
            attribute,
            format!(r"\A(?i-u:font){separator}+([^>]+)/?\z"),
            // FontStyle.getUbrexActivationPattern + CharClassRaw.WORD.
            r"\A(?i-u:back):((?:#[0-9a-fA-F]{6}|[A-Za-z0-9_]+)(?:[-\\|/](?:[0-9a-fA-F]{6}|[A-Za-z0-9_]+))?)\z".into(),
            format!(r"\A(?i-u:img){separator}+([^>{{}}]+)(?:\{{(?i-u:scale)=[0-9.]+\}})?\z"),
            r"\A(?:#[A-Za-z0-9_]+)?&[-A-Za-z0-9_]+(?:[\{,]?(?:(?:(?i-u:scale=)|\*)[0-9.]+)?(?:,?(?i-u:color)[= :](?:#[0-9a-fA-F]{1,8}|[A-Za-z0-9_]+))?\}?)?\z".into(),
        ].map(|pattern| regex::Regex::new(&pattern).expect("Java parameter candidate grammar"))
    });
    let [
        size,
        color,
        font,
        attribute,
        family,
        background,
        image,
        icon,
    ] = &*PATTERNS;
    if let Some(capture) = size.captures(tag) {
        return Some(ParameterCommand::Size(capture.get(1).unwrap().as_str()));
    }
    if let Some(capture) = color.captures(tag) {
        return Some(ParameterCommand::Color(capture.get(1).unwrap().as_str()));
    }
    // CommandCreoleBuilder registers combined attributes before the broader
    // font-family candidate. Java retains the last capture for each attribute.
    if font.is_match(tag) {
        let (mut size, mut color) = (None, None);
        for capture in attribute.captures_iter(tag) {
            if let Some(value) = capture.get(1) {
                size = Some(value.as_str());
            }
            if let Some(value) = capture.get(2) {
                color = Some(value.as_str());
            }
        }
        return Some(ParameterCommand::Font { size, color });
    }
    if let Some(capture) = family.captures(tag) {
        return Some(ParameterCommand::Family(capture.get(1).unwrap().as_str()));
    }
    if let Some(capture) = background.captures(tag) {
        return Some(ParameterCommand::Background(
            capture.get(1).unwrap().as_str(),
        ));
    }
    if let Some(capture) = image.captures(tag) {
        return Some(ParameterCommand::Image(capture.get(1).unwrap().as_str()));
    }
    icon.is_match(tag).then_some(ParameterCommand::Icon)
}

// HColorSet.parseColor/parseSimpleColor separate resolution from command syntax.
// Valid values retain their existing emission path; this predicate only decides
// which command-specific failure behavior applies.
pub(crate) fn is_resolvable_color(color: &str) -> bool {
    fn simple(color: &str) -> bool {
        let color = color.strip_prefix('#').unwrap_or(color);
        if matches!(color.len(), 1 | 3 | 6 | 8)
            && color.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return true;
        }
        let name = color.to_ascii_lowercase();
        // ColorTrieNode's registry shares the existing CSS lookup except these
        // seven Archimate names and CSS4 RebeccaPurple (absent in pinned Java).
        match name.as_str() {
            "business" | "application" | "motivation" | "strategy" | "technology" | "physical"
            | "implementation" => true,
            "rebeccapurple" => false,
            _ => crate::text_render::normalize_color(&name) != name,
        }
    }

    let color = color.strip_prefix('#').unwrap_or(color);
    if matches!(
        color.to_ascii_lowercase().as_str(),
        "transparent" | "background" | "automatic"
    ) || simple(color)
    {
        return true;
    }
    // HColorSet resolves both gradient operands as simple colors. This does
    // not implement gradient painting; an unresolved operand makes it invalid.
    color
        .split_once(['-', '\\', '|', '/'])
        .is_some_and(|(first, second)| simple(first) && simple(second))
}

fn walk_parameter_body(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    close: &str,
    style: &Style,
    nested: &Style,
    skip_underline: bool,
    out: &mut Vec<Segment>,
) {
    // Parameter commands select the first case-insensitive bounded capture,
    // otherwise their EOL alternative. Hidden closing candidates stay inert.
    let remaining: String = chars.clone().collect();
    let end = remaining.to_ascii_lowercase().find(close);
    let content_end = end.unwrap_or(remaining.len());
    let consumed = end.map_or(remaining.len(), |offset| offset + close.len());
    for _ in remaining[..consumed].chars() {
        chars.next();
    }
    out.push(Segment {
        text: String::new(),
        style: style.clone(),
    });
    walk_segments(&remaining[..content_end], nested, skip_underline, out);
    out.push(Segment {
        text: String::new(),
        style: style.clone(),
    });
}

fn handle_tag(
    tag: &str,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    style: &Style,
    skip_underline: bool,
    out: &mut Vec<Segment>,
) -> bool {
    if let Some(command) = parameter_command(tag) {
        let mut nested = style.clone();
        let close = match command {
            ParameterCommand::Size(size) => {
                // Lexical eligibility is distinct from Java's parseInt failure
                // for oversized values. Keep the existing nonpanicking contract.
                if let Ok(size) = size.parse::<u32>() {
                    nested.size = Some(size);
                }
                Some("</size>")
            }
            ParameterCommand::Color(color) => {
                // CommandCreoleColorChange catches NoSuchColorException and
                // keeps fc1 while still consuming/analyzing the command body.
                if is_resolvable_color(color) {
                    let (_, opacity) = crate::text_render::color_with_opacity(color);
                    nested.fill = Some(color.to_string());
                    nested.fill_opacity = opacity;
                }
                Some("</color>")
            }
            ParameterCommand::Font { size, color } => {
                if let Some(size) = size.and_then(|value| value.parse::<u32>().ok()) {
                    nested.size = Some(size);
                }
                if let Some(color) = color {
                    let (_, opacity) = crate::text_render::color_with_opacity(color);
                    nested.fill = Some(color.to_string());
                    nested.fill_opacity = opacity;
                }
                Some("</font>")
            }
            ParameterCommand::Family(family) => {
                // CommandCreoleFontFamilyChange keeps the name. Widths are
                // Font.decode of that name. The flag is only the SvgGraphics
                // NBSP rule for courier, monospace, and monospaced.
                nested.font_family = Some(family.to_string());
                nested.monospace = matches!(
                    family
                        .trim_matches(|c| c == '\"' || c == '\'')
                        .to_ascii_lowercase()
                        .as_str(),
                    "courier" | "monospace" | "monospaced"
                );
                Some("</font>")
            }
            ParameterCommand::Background(color) => {
                return walk_style_tag(
                    chars,
                    &["back"],
                    style,
                    // CommandCreoleStyle.getExtendedColor uses getColorOrWhite,
                    // unlike ColorChange's inherited-color failure behavior.
                    |s| {
                        s.background = Some(
                            if is_resolvable_color(color) {
                                color
                            } else {
                                "#FFFFFF"
                            }
                            .to_string(),
                        );
                    },
                    skip_underline,
                    out,
                );
            }
            ParameterCommand::Image(src) => {
                // CommandCreoleImg removes src= and paired double quotes only
                // after the complete image candidate grammar has matched.
                let src = if src.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("src=")) {
                    &src[4..]
                } else {
                    src
                };
                let src = src
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(src);
                let fallback = if src.starts_with("https://") || src.starts_with("http://") {
                    format!("(Cannot\u{00a0}decode:\u{00a0}{})", escape_creole_text(src))
                } else {
                    "(Cannot\u{00a0}decode)".to_string()
                };
                // AtomImg.create constructs its own blackBlueTrue(monospaced(14))
                // configuration. Reset nested markup, including face, color,
                // decoration, family and position; consumer TextBase inheritance
                // is outside this atom's lexical configuration.
                nested = Style {
                    monospace: true,
                    size: Some(14),
                    fill: Some("#000000".to_string()),
                    ..Style::default()
                };
                out.push(Segment {
                    text: String::new(),
                    style: style.clone(),
                });
                push_segment(out, &fallback, &nested);
                None
            }
            ParameterCommand::Icon => {
                // Keep the existing lexical handoff to text_with_icons. This
                // parser validates the candidate; the consumer resolves its atom.
                return true;
            }
        };
        if let Some(close) = close {
            walk_parameter_body(chars, close, style, &nested, skip_underline, out);
        } else {
            out.push(Segment {
                text: String::new(),
                style: style.clone(),
            });
        }
        return true;
    }
    match tag {
        _ if is_sprite_tag(tag) => {
            // CommandCreoleSprite.executeAndGetRemaining consumes the entire
            // token; StripeSimple.addSprite adds nothing when lookup fails.
            // Keep the command's flush boundary until final segment projection.
            out.push(Segment {
                text: String::new(),
                style: style.clone(),
            });
        }
        // PlantUML's HTML subset supports only the short tags (`<b>`, `<i>`,
        // `<u>`, `<s>`, `<w>`, `<del>`, `<strike>`, `<code>`, `<sub>`, `<sup>`,
        // `<color:>`, `<size:>`, `<font:>`, `<back:>`, `<img:>`). The HTML5
        // synonyms `<strong>`, `<em>`, `<ins>` are NOT recognized and render
        // literally, so they deliberately fall through to the unknown-tag arm.
        "b" | "B" => {
            return walk_style_tag(
                chars,
                &["b", "B"],
                style,
                |s| s.bold = true,
                skip_underline,
                out,
            );
        }
        "i" | "I" => {
            return walk_style_tag(
                chars,
                &["i", "I"],
                style,
                |s| s.italic = true,
                skip_underline,
                out,
            );
        }
        "u" | "U" => {
            return walk_style_tag(
                chars,
                &["u", "U"],
                style,
                |s| s.underline = true,
                skip_underline,
                out,
            );
        }
        "w" | "W" => {
            return walk_style_tag(
                chars,
                &["w", "W"],
                style,
                |s| s.wavy_underline = true,
                skip_underline,
                out,
            );
        }
        "s" | "S" | "del" | "DEL" | "strike" | "STRIKE" => {
            return walk_style_tag(
                chars,
                &["s", "S", "del", "DEL", "strike", "STRIKE"],
                style,
                |s| s.line_through = true,
                skip_underline,
                out,
            );
        }
        "mono" => walk_with(
            chars,
            "</mono>".into(),
            tag,
            "",
            style,
            |s| s.monospace = true,
            skip_underline,
            out,
        ),
        "code" => walk_with(
            chars,
            "</code>".into(),
            tag,
            "",
            style,
            |s| s.monospace = true,
            skip_underline,
            out,
        ),
        "sub" => walk_with(
            chars,
            "</sub>".into(),
            tag,
            "",
            style,
            |s| s.baseline_shift = Some("sub"),
            skip_underline,
            out,
        ),
        "sup" => walk_with(
            chars,
            "</sup>".into(),
            tag,
            "",
            style,
            |s| s.baseline_shift = Some("super"),
            skip_underline,
            out,
        ),
        _ => return false,
    }
    true
}

/// CommandCreoleBuilder orders the bounded style candidate before its EOL
/// alternative. CommandCreoleStyle.matchingSize returns captured body length,
/// unlike size/color commands, whose parameter groups determine eligibility.
fn walk_style_tag(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    closing_names: &[&str],
    style: &Style,
    apply: impl FnOnce(&mut Style),
    skip_underline: bool,
    out: &mut Vec<Segment>,
) -> bool {
    let remaining: String = chars.clone().collect();
    let mut closed = None;
    for (offset, c) in remaining.char_indices() {
        if c == '<'
            && let Some(rest) = remaining[offset..].strip_prefix("</")
            && let Some(end) = rest.find('>')
        {
            let name = &rest[..end];
            let matches = closing_names.contains(&name)
                || (closing_names == ["back"] && name.eq_ignore_ascii_case("back"));
            if matches {
                closed = Some((offset, offset + end + "</>".len()));
                break;
            }
        }
    }
    let (content_end, consumed) = match closed {
        Some((body, consumed)) if body > 0 => (body, consumed),
        _ if !remaining.is_empty() => (remaining.len(), remaining.len()),
        _ => return false,
    };
    for _ in remaining[..consumed].chars() {
        chars.next();
    }
    let mut nested = style.clone();
    apply(&mut nested);
    // StripeSimple flushes before a selected command and at the end of its
    // recursive body, even when the surrounding and nested styles coincide.
    out.push(Segment {
        text: String::new(),
        style: style.clone(),
    });
    walk_segments(&remaining[..content_end], &nested, skip_underline, out);
    out.push(Segment {
        text: String::new(),
        style: style.clone(),
    });
    true
}

/// Helper that pushes a style modification, parses inner content until the
/// closing tag, and emits resulting segments. The two name args are unused
/// placeholders kept for call-site symmetry with the dispatch table above —
/// they let the dispatch match arms read uniformly.
#[allow(clippy::too_many_arguments)]
fn walk_with(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    close: String,
    _open_name: &str,
    _alias: &str,
    style: &Style,
    apply: impl FnOnce(&mut Style),
    skip_underline: bool,
    out: &mut Vec<Segment>,
) {
    let content = collect_until_tag(chars, &close);
    let mut nested = style.clone();
    apply(&mut nested);
    walk_segments(&content, &nested, skip_underline, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_follows_fill_changes_and_restores_after_recursion() {
        let segments = parse_segments("<color:#8044aa77>a<font color=red>b</font>c</color>d");
        let runs: Vec<_> = segments
            .iter()
            .filter(|s| !s.text.is_empty())
            .map(|s| (s.text.as_str(), s.style.fill_opacity))
            .collect();
        assert_eq!(
            runs,
            [("a", Some(119)), ("b", None), ("c", Some(119)), ("d", None)]
        );
    }

    // --- Segment-based parsing tests ---

    fn seg(text: &str, style: Style) -> Segment {
        Segment {
            text: text.to_string(),
            style,
        }
    }

    fn bold_style() -> Style {
        Style {
            bold: true,
            ..Default::default()
        }
    }

    fn italic_style() -> Style {
        Style {
            italic: true,
            ..Default::default()
        }
    }

    fn mono_style() -> Style {
        Style {
            monospace: true,
            ..Default::default()
        }
    }

    fn wavy_underline_style() -> Style {
        Style {
            wavy_underline: true,
            ..Default::default()
        }
    }

    #[test]
    fn hidden_characters_share_candidate_and_recursive_text_semantics() {
        assert_eq!(
            parse_segments(r"q<b>a\~</b>r"),
            vec![
                seg("q", Style::default()),
                seg("a~", bold_style()),
                seg("r", Style::default()),
            ]
        );
        assert_eq!(
            parse_segments("q~***x**r"),
            vec![
                seg("q*", Style::default()),
                seg("x", bold_style()),
                seg("r", Style::default()),
            ]
        );
        assert_eq!(parse_segments(r"q<b><i>a\~</i></b>r")[1].text, "a~");
        assert_eq!(stripped_text(r"~#~.~[~]~<~a~Ω\~"), "#.[]<~a~Ω~");
        assert_eq!(stripped_text(r"q**a\~\~b**r"), "qa~~br");
    }

    #[test]
    fn parameter_commands_flush_empty_and_same_style_bodies() {
        for markup in [
            "q<size:13></size>r",
            "q<color:black></color>r",
            "q<size:13><$twig></size>r",
            "q<color:black><$twig></color>r",
        ] {
            assert_eq!(
                parse_segments(markup),
                vec![seg("q", Style::default()), seg("r", Style::default())],
                "{markup}"
            );
        }
        let segments = parse_segments("<size:13>q<size:13>x");
        assert_eq!(
            segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            ["q", "x"]
        );
        assert!(segments.iter().all(|s| s.style.size == Some(13)));
        assert_eq!(
            parse_segments("<size:25></size><color:red></color>"),
            vec![seg(" ", Style::default())]
        );
        assert!(parse_segments("").is_empty());
    }

    #[test]
    fn color_resolution_is_separate_from_candidate_eligibility() {
        let segments = parse_segments(
            "<font color=navy>a<color:#GGHHII>b</color>c<color:#12></color>d</font>e",
        );
        assert_eq!(
            segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c", "d", "e"]
        );
        assert!(
            segments[..4]
                .iter()
                .all(|s| s.style.fill.as_deref() == Some("navy"))
        );
        assert_eq!(segments[4].style.fill, None);
        assert_eq!(
            parse_segments("<color:UnknownTone>x</color>")[0].style.fill,
            None
        );
        // A hidden marker still fails grammar and leaves literal text, rather
        // than selecting the command and applying its resolution fallback.
        assert_eq!(
            stripped_text("<color:~#123456>x</color>"),
            "<color:#123456>x</color>"
        );
        for color in [
            "red",
            "#RED",
            "F",
            "123",
            "#112233",
            "#8044aa77",
            "BUSINESS",
            "transparent",
            "red-blue",
        ] {
            assert!(is_resolvable_color(color), "{color}");
        }
        for color in [
            "#GGGGGG",
            "#12",
            "RebeccaPurple",
            "UnknownTone",
            "red-UnknownTone",
        ] {
            assert!(!is_resolvable_color(color), "{color}");
        }
    }

    #[test]
    fn unresolved_background_uses_white_then_restores_outer_style() {
        let segments = parse_segments("<back:blue>a<back:UnknownTone>b</back>c</back>d");
        assert_eq!(
            segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c&lt;/back&gt;d"]
        );
        // Java's first closing capture ends the outer command; this is not a
        // balanced-tag parser. The remaining closing spelling is literal.
        assert_eq!(
            segments
                .iter()
                .map(|s| s.style.background.as_deref())
                .collect::<Vec<_>>(),
            [Some("blue"), Some("#FFFFFF"), None]
        );
        assert_eq!(
            parse_segments("<back:red-UnknownTone>x</back>")[0]
                .style
                .background
                .as_deref(),
            Some("#FFFFFF")
        );
        assert_eq!(
            parse_segments("<back:red-blue>x</back>")[0]
                .style
                .background
                .as_deref(),
            Some("red-blue")
        );
        assert_eq!(
            parse_segments("<back:#GGGGGG>x</back>")[0].style.background,
            None
        );
    }

    #[test]
    fn missing_image_resets_nested_markup_and_restores_its_neighbors() {
        let segments = parse_segments(
            "<b><i><u><s><color:red><back:yellow><font:Serif><size:21><sub>a<img:absent.png>b</sub></size></font></back></color></s></u></i></b>",
        );
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].style, segments[2].style);
        assert!(segments[0].style.bold && segments[0].style.italic);
        assert_eq!(segments[1].text, "(Cannot\u{00a0}decode)");
        assert_eq!(
            segments[1].style,
            Style {
                monospace: true,
                size: Some(14),
                fill: Some("#000000".to_string()),
                ..Style::default()
            }
        );
    }

    #[test]
    fn parameter_font_aliases_preserve_shared_consumer_flags() {
        // Font-family grammar is broad; only the existing shared resolver's
        // named alias set activates its monospace spacing/metrics path.
        for family in [
            "Courier",
            "cOuRiEr",
            "monospace",
            "Monospaced",
            "\"Courier\"",
            "'MONOSPACE'",
        ] {
            let segments = parse_segments(&format!("<font:{family}>code here</font>"));
            assert!(segments[0].style.monospace, "{family}");
            assert_eq!(segments[0].style.font_family.as_deref(), Some(family));
        }
        for family in [
            "SansSerif",
            "Dialog",
            "Arial",
            "color=blue",
            "Courier New",
            "Courier ",
        ] {
            let segments = parse_segments(&format!("<font:{family}>code here</font>"));
            assert!(!segments[0].style.monospace, "{family}");
        }
    }

    #[test]
    fn parameter_candidates_reject_hidden_and_malformed_values() {
        for (tag, close) in [
            ("size:~13", "size"),
            (r"size:1\~3", "size"),
            ("size:13~", "size"),
            ("color:~#123456", "color"),
            (r"color:bl\~ue", "color"),
            ("back:~#123456", "back"),
            ("back:#12", "back"),
            ("font:", "font"),
            ("fontish:red", "font"),
        ] {
            let source = format!("X<{tag}>fir</{close}>Y");
            let segments = parse_segments(&source);
            assert_eq!(segments.len(), 1, "{source}");
            assert_eq!(segments[0].style, Style::default(), "{source}");
            assert_eq!(
                stripped_text(&source),
                restore_hidden_characters(&hide_characters(&source))
            );
        }
        for tag in [
            "img:",
            "img:absent{scale=bad}",
            "&",
            "&home{scale=bad}",
            "&home{color=\u{E023}123456}",
        ] {
            assert!(parameter_command(tag).is_none(), "{tag}");
        }
        assert!(matches!(
            parameter_command("&home"),
            Some(ParameterCommand::Icon)
        ));
        assert!(matches!(
            parameter_command("img:absent{scale=2}"),
            Some(ParameterCommand::Image("absent"))
        ));
        assert!(matches!(
            parameter_command(r"back:red\blue"),
            Some(ParameterCommand::Background(_))
        ));
    }

    #[test]
    fn parameter_grammar_preserves_priority_case_and_capture_boundaries() {
        let segments = parse_segments("X<SiZe :13 >fir</SIZE>Y");
        assert_eq!(
            segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            ["X", "fir", "Y"]
        );
        assert_eq!(segments[1].style.size, Some(13));
        assert!(parameter_command("size\u{a0}13").is_none());
        assert!(matches!(
            parameter_command("size:13\u{a0}"),
            Some(ParameterCommand::Size("13"))
        ));
        assert!(matches!(
            parameter_command("font size=13 color=red size=12 color=blue"),
            Some(ParameterCommand::Font {
                size: Some("12"),
                color: Some("blue")
            })
        ));
        assert!(matches!(
            parameter_command("font color='red'"),
            Some(ParameterCommand::Family("color='red'"))
        ));
        // Unlike color values, Java's broad family grammar permits hidden chars.
        assert!(matches!(
            parameter_command("font color=\u{E023}123456"),
            Some(ParameterCommand::Family(_))
        ));
        let segments = parse_segments("X<font size=13 color=blue></FONT>Y");
        assert_eq!(
            segments,
            [seg("X", Style::default()), seg("Y", Style::default())]
        );
    }

    #[test]
    fn failed_parameter_candidate_leaves_inner_commands_reachable() {
        let segments = parse_segments("X<size:~13<color:red>fir</color>Y");
        assert_eq!(
            segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            ["X&lt;size:~13", "fir", "Y"]
        );
        assert_eq!(segments[1].style.fill.as_deref(), Some("red"));
        assert_eq!(stripped_text("X<color:~#<$twig>>firY"), "X<color:#>firY");
    }

    #[test]
    fn segments_plain_text() {
        assert_eq!(
            parse_segments("hello world"),
            vec![seg("hello world", Style::default())]
        );
    }

    #[test]
    fn delimited_style_candidates_preserve_empty_and_unclosed_input() {
        for delimiter in ["**", "//", "__", "--", "~~"] {
            let empty = delimiter.repeat(2);
            assert_eq!(stripped_text(&empty), empty);
            let one = format!(
                "{delimiter}{}{delimiter}",
                delimiter.chars().next().unwrap()
            );
            let segments = parse_segments(&one);
            assert_eq!(segments.len(), 1, "{one}");
            assert_ne!(segments[0].style, Style::default(), "{one}");
            let unclosed = format!("{delimiter}A<$twig>B");
            assert_eq!(stripped_text(&unclosed), format!("{delimiter}AB"));
        }
    }

    #[test]
    fn style_candidates_require_nonempty_captures() {
        for (open, close) in [
            ("b", "b"),
            ("i", "I"),
            ("u", "u"),
            ("s", "del"),
            ("w", "W"),
            ("back:red", "back"),
        ] {
            let markup = format!("<{open}></{close}>");
            let segments = parse_segments(&markup);
            assert_eq!(segments.len(), 1, "{markup}");
            assert_eq!(
                unescape_for_metrics(&segments[0].text),
                format!("</{close}>")
            );
            assert_ne!(segments[0].style, Style::default(), "{markup}");
        }
        assert_eq!(stripped_text("<b>"), "<b>");
        assert_eq!(
            parse_segments("<size:25></size>"),
            vec![seg(" ", Style::default())]
        );
        assert_eq!(
            parse_segments("<color:red></color>"),
            vec![seg(" ", Style::default())]
        );
        assert_eq!(
            parse_segments("<b><$bud></b>"),
            vec![seg(" ", Style::default())]
        );
    }

    #[test]
    fn style_candidate_order_preserves_closing_literals_and_escapes() {
        assert_eq!(
            parse_segments("<b><b></b></b>"),
            vec![
                seg("&lt;b&gt;", bold_style()),
                seg("&lt;/b&gt;", Style::default()),
            ]
        );
        assert_eq!(
            parse_segments("<b>A~</b>B</b>C"),
            vec![
                seg("A&lt;/b&gt;B", bold_style()),
                seg("C", Style::default()),
            ]
        );
        assert_eq!(
            parse_segments("A<outer<b></b>>B<$bud>C"),
            vec![
                seg("A&lt;outer", Style::default()),
                seg("&lt;/b&gt;&gt;B", bold_style()),
                seg("C", bold_style()),
            ]
        );
        assert_eq!(stripped_text("~<b></b>"), "<b></b>");
    }

    #[test]
    fn missing_sprites_consume_commands_but_keep_atom_boundaries() {
        assert_eq!(
            parse_segments("Oak<$unavailable>Grove<$other> end"),
            vec![
                seg("Oak", Style::default()),
                seg("Grove", Style::default()),
                seg(" end", Style::default()),
            ]
        );
        assert_eq!(
            parse_segments("<size:20><$unavailable></size>"),
            vec![seg(" ", Style::default())]
        );
        assert_eq!(
            parse_segments("<$one><$two>"),
            vec![seg(" ", Style::default())]
        );
        assert_eq!(
            parse_segments("**Oak<$missing>Grove**"),
            vec![seg("Oak", bold_style()), seg("Grove", bold_style())]
        );
    }

    #[test]
    fn unknown_angle_candidates_leave_inner_commands_visible() {
        assert_eq!(
            parse_segments("Ω<outer<$枝>>🙂"),
            vec![
                seg("Ω&lt;outer", Style::default()),
                seg("&gt;🙂", Style::default())
            ]
        );
        assert_eq!(stripped_text("A<<$leaf>>B"), "A<>B");
        assert_eq!(stripped_text("Ω<outer~<$枝>>🙂"), "Ω<outer<$枝>>🙂");
        assert_eq!(
            stripped_text("Ω<ordinary>🙂<unfinished"),
            "Ω<ordinary>🙂<unfinished"
        );
        assert_eq!(
            parse_segments("<b>A<outer<$leaf>>B</b>"),
            vec![seg("A&lt;outer", bold_style()), seg("&gt;B", bold_style())]
        );
        assert!(parse_segments("").is_empty());
    }

    #[test]
    fn sprite_command_grammar_preserves_neighboring_literal_syntax() {
        for tag in [
            "$oak/leaf-2",
            "$枝",
            "#red$oak{scale=2,color=blue}",
            "$oak*1.5",
            "$oak,color:red",
            "$oak{SCALE=2,COLOR=#ff0000}",
        ] {
            assert!(is_sprite_tag(tag), "{tag}");
        }
        for tag in [
            "$",
            "$bad.name",
            "$bad name",
            "$oak*bad",
            "$oak{scale=-2}",
            "space key",
        ] {
            assert!(!is_sprite_tag(tag), "{tag}");
            let text = format!("a <{tag}> b");
            assert_eq!(stripped_text(&text), text);
        }
        for text in [
            "~<$oak>",
            "<$oak",
            "https://example.test/a",
            "[[https://example.test/a <$oak>]]",
        ] {
            let segments = parse_segments(text);
            assert!(!segments.is_empty(), "{text}");
        }
        assert_eq!(stripped_text("~<$oak>"), "<$oak>");
        assert_eq!(stripped_text("<$oak"), "<$oak");
        assert_eq!(
            stripped_text("https://example.test/a"),
            "https://example.test/a"
        );
    }

    #[test]
    fn segments_xml_escaping() {
        // Unknown tags emit literally (matching existing to_svg_tspans behaviour).
        assert_eq!(
            parse_segments("<unknown>"),
            vec![seg("&lt;unknown&gt;", Style::default())]
        );
        // `<&name>` is an OpenIconic icon — silently dropped from segments
        // (handled at the SVG-builder level by text_with_icons).
        assert_eq!(parse_segments("<&heart>"), vec![]);
        // Real escape characters in text are escaped.
        assert_eq!(
            parse_segments("a & b"),
            vec![seg("a &amp; b", Style::default())]
        );
    }

    #[test]
    fn segments_bold_only() {
        // PlantUML's uniform-style case: one segment, bold lifted to text attrs.
        assert_eq!(parse_segments("**bold**"), vec![seg("bold", bold_style())]);
    }

    #[test]
    fn segments_nested_bold_italic() {
        // **//bold italic//** → one segment with both flags.
        let s = Style {
            bold: true,
            italic: true,
            ..Style::default()
        };
        assert_eq!(
            parse_segments("**//bold italic//**"),
            vec![seg("bold italic", s)]
        );
    }

    #[test]
    fn segments_mono_with_nbsp() {
        // Spaces in monospace runs become NBSP — matches PlantUML's
        // Date.toString()-style mono output.
        assert_eq!(
            parse_segments(r#"""mono activity"""#),
            vec![seg("mono\u{00a0}activity", mono_style())]
        );
    }

    #[test]
    fn segments_img_fallback_is_monospace_size_14() {
        let mut style = mono_style();
        style.size = Some(14);
        // AtomImg.create uses blackBlueTrue, not inherited text color.
        style.fill = Some("#000000".to_string());
        assert_eq!(
            parse_segments("<img:sprite.png>"),
            vec![seg("(Cannot\u{00a0}decode)", style)]
        );
    }

    #[test]
    fn segments_quoted_font_attrs_become_family() {
        let style = Style {
            font_family: Some("color='red'".to_string()),
            ..Style::default()
        };
        assert_eq!(
            parse_segments("<font color='red'>font color</font>"),
            vec![seg("font color", style)]
        );
    }

    #[test]
    fn segments_unquoted_font_color_becomes_fill() {
        let style = Style {
            fill: Some("red".to_string()),
            ..Style::default()
        };
        assert_eq!(
            parse_segments("<font color=red>Red Title</font>"),
            vec![seg("Red Title", style)]
        );
    }

    #[test]
    fn segments_split_on_style_change() {
        // <color:blue>**field**</color>: String → two segments:
        //   "field" with fill=blue + bold
        //   ": String" plain
        let segs = parse_segments("<color:blue>**field**</color>: String");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "field");
        assert_eq!(segs[0].style.fill.as_deref(), Some("blue"));
        assert!(segs[0].style.bold);
        assert_eq!(segs[1].text, ": String");
        assert_eq!(segs[1].style, Style::default());
    }

    #[test]
    fn segments_text_around_italic() {
        let segs = parse_segments("before //middle// after");
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].text, "before ");
        assert_eq!(segs[0].style, Style::default());
        assert_eq!(segs[1].text, "middle");
        assert_eq!(segs[1].style, italic_style());
        assert_eq!(segs[2].text, " after");
        assert_eq!(segs[2].style, Style::default());
    }

    #[test]
    fn segments_strike_html_tag() {
        let segs = parse_segments("<s>gone</s>");
        let s = Style {
            line_through: true,
            ..Style::default()
        };
        assert_eq!(segs, vec![seg("gone", s)]);
    }

    #[test]
    fn segments_hyperlink() {
        let segs = parse_segments("[[https://example.com label]]");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "label");
        assert_eq!(
            segs[0].style.link_url.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(segs[0].style.fill.as_deref(), Some("#0000FF"));
        assert!(segs[0].style.underline);
    }

    #[test]
    fn segments_skip_underline_for_class_labels() {
        // In class entity labels, `__` is literal, not underline.
        let segs = parse_segments_no_underline("__not__");
        assert_eq!(segs, vec![seg("__not__", Style::default())]);
    }

    #[test]
    fn uppercase_underline_tag_consumes_rest() {
        let underlined = Style {
            underline: true,
            ..Style::default()
        };
        assert_eq!(
            parse_segments_no_underline("Future<U> thenApply(Function<T,U> f)"),
            vec![
                seg("Future", Style::default()),
                seg(" thenApply(Function&lt;T,U&gt; f)", underlined),
            ]
        );
    }

    #[test]
    fn segments_unmatched_bold_is_literal() {
        let segs = parse_segments("**no close");
        assert_eq!(segs, vec![seg("**no close", Style::default())]);
    }

    #[test]
    fn segments_lone_trailing_tilde_is_literal() {
        assert_eq!(
            parse_segments("done~"),
            vec![seg("done~", Style::default())]
        );
    }

    #[test]
    fn segments_tilde_escapes_less_than() {
        assert_eq!(
            parse_segments("~<code>"),
            vec![seg("&lt;code&gt;", Style::default())]
        );
    }

    #[test]
    fn segments_wavy_underline_preserves_trailing_tilde() {
        assert_eq!(
            parse_segments("~~~not strike~~~"),
            vec![
                seg("~not strike", wavy_underline_style()),
                seg("~", Style::default()),
            ]
        );
    }

    #[test]
    fn segments_html_wavy_underline_tag() {
        assert_eq!(
            parse_segments("<w>wave</w> <W>upper</W>"),
            vec![
                seg("wave", wavy_underline_style()),
                seg(" ", Style::default()),
                seg("upper", wavy_underline_style()),
            ]
        );
    }

    #[test]
    fn stripped_text_drops_markers() {
        assert_eq!(stripped_text("**hello** //world//"), "hello world");
        assert_eq!(stripped_text(r#"""mono"""#), "mono");
        assert_eq!(stripped_text("<color:red>x</color>"), "x");
    }

    // --- Inline markup tests ---

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
    fn underline_skipped_in_class_label() {
        // to_svg_tspans_no_underline keeps `__` as literal characters.
        assert_eq!(to_svg_tspans_no_underline("__under__"), "__under__");
        // Bold and italic are still processed.
        assert_eq!(
            to_svg_tspans_no_underline("**bold** __under__"),
            r#"<tspan font-weight="bold">bold</tspan> __under__"#
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
        // ~ before a non-markup char is emitted literally; the next char is also emitted.
        assert_eq!(to_svg_tspans("~x"), "~x");
    }

    #[test]
    fn tilde_trailing_char() {
        assert_eq!(to_svg_tspans("done~"), "done~");
    }

    #[test]
    fn tilde_escapes_less_than() {
        assert_eq!(to_svg_tspans("~<"), "&lt;");
    }

    #[test]
    fn tilde_single_markup_char() {
        // ~ before a single markup char (not a two-char delim) escapes that char.
        assert_eq!(to_svg_tspans("~/not italic/"), "/not italic/");
    }

    #[test]
    fn img_url_fallback() {
        // Monospace tspan with non-breaking spaces matches Java PlantUML's SVG output.
        assert_eq!(
            to_svg_tspans("<img:https://example.com/img.png>"),
            "<tspan font-family=\"monospace\">(Cannot\u{00a0}decode:\u{00a0}https://example.com/img.png)</tspan>"
        );
    }

    #[test]
    fn img_local_fallback() {
        // Local file: monospace "(Cannot decode)" with non-breaking space.
        assert_eq!(
            to_svg_tspans("<img:localfile.png>"),
            "<tspan font-family=\"monospace\">(Cannot\u{00a0}decode)</tspan>"
        );
    }

    #[test]
    fn img_url_scaled_fallback() {
        // {scale=...} suffix stripped from URL in fallback message.
        assert_eq!(
            to_svg_tspans("<img:https://example.com/img.png{scale=0.5}>"),
            "<tspan font-family=\"monospace\">(Cannot\u{00a0}decode:\u{00a0}https://example.com/img.png)</tspan>"
        );
    }

    #[test]
    fn img_url_width_fallback() {
        // {width=...} suffix stripped from URL in fallback message.
        assert_eq!(
            to_svg_tspans("<img:https://example.com/img.png{width=100}>"),
            "<tspan font-family=\"monospace\">(Cannot\u{00a0}decode:\u{00a0}https://example.com/img.png)</tspan>"
        );
    }

    #[test]
    fn img_inline_with_text() {
        // Image tag embedded in surrounding text.
        assert_eq!(
            to_svg_tspans("before <img:https://example.com/x.png> after"),
            "before <tspan font-family=\"monospace\">(Cannot\u{00a0}decode:\u{00a0}https://example.com/x.png)</tspan> after"
        );
    }

    #[test]
    fn monospace_double_quote() {
        assert_eq!(
            to_svg_tspans("\"\"mono legend text\"\""),
            "<tspan font-family=\"monospace\">mono\u{00a0}legend\u{00a0}text</tspan>"
        );
    }

    // --- Nested inline markup tests ---

    #[test]
    fn nested_bold_in_italic() {
        assert_eq!(
            to_svg_tspans("**//bold italic//**"),
            "<tspan font-weight=\"bold\"><tspan font-style=\"italic\">bold italic</tspan></tspan>"
        );
    }

    #[test]
    fn nested_italic_in_bold() {
        // **//text//** — bold wrapping italic.
        let result = to_svg_tspans("**//nested//**");
        assert!(result.contains("font-weight=\"bold\""));
        assert!(result.contains("font-style=\"italic\""));
        assert!(result.contains("nested"));
    }

    #[test]
    fn nested_underline_bold() {
        assert_eq!(
            to_svg_tspans("__**underline bold**__"),
            "<tspan text-decoration=\"underline\"><tspan font-weight=\"bold\">underline bold</tspan></tspan>"
        );
    }

    #[test]
    fn nested_three_levels() {
        // Bold wrapping italic wrapping underline.
        let result = to_svg_tspans("**//__ deep __//**");
        assert!(result.contains("font-weight=\"bold\""));
        assert!(result.contains("font-style=\"italic\""));
        assert!(result.contains("text-decoration=\"underline\""));
        assert!(result.contains(" deep "));
    }

    // --- List tests ---

    #[test]
    fn bold_not_mistaken_for_bullet() {
        // `**bold**` must not be treated as a level-2 bullet list item.
        let mut lc = ListCounters::new();
        let result = lc.render_line("**bold**");
        assert!(result.contains("bold"), "Expected bold in: {result}");
        assert!(
            !result.contains('\u{2022}'),
            "Unexpected bullet in: {result}"
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
    }

    #[test]
    fn nested_bullet_list() {
        // `** text` (space after stars) is a nested bullet, not bold markup.
        let mut lc = ListCounters::new();
        assert_eq!(lc.render_line("* level 1"), "\u{2022} level 1");
        assert_eq!(lc.render_line("** level 2"), "  \u{2022} level 2");
        assert_eq!(lc.render_line("*** level 3"), "    \u{2022} level 3");
        assert_eq!(lc.render_line("** back to 2"), "  \u{2022} back to 2");
        assert_eq!(lc.render_line("* back to 1"), "\u{2022} back to 1");
    }

    #[test]
    fn bold_vs_nested_bullet() {
        // `**bold**` (no space, closing **) = bold markup.
        // `** text` (space after **) = level-2 bullet.
        let mut lc = ListCounters::new();
        let bold_result = lc.render_line("**bold**");
        assert!(bold_result.contains("font-weight=\"bold\""));
        assert!(!bold_result.contains('\u{2022}'));

        let bullet_result = lc.render_line("** nested item");
        assert!(bullet_result.contains('\u{2022}'));
        assert!(bullet_result.contains("nested item"));
    }

    // --- Table tests ---

    #[test]
    fn parse_simple_table() {
        let row = parse_table_row("|= Header1 |= Header2 |").unwrap();
        assert_eq!(row.cells.len(), 2);
        assert!(row.cells[0].is_header);
        assert_eq!(row.cells[0].text, "Header1");
        assert!(row.cells[1].is_header);
        assert_eq!(row.cells[1].text, "Header2");
    }

    #[test]
    fn parse_data_table() {
        let row = parse_table_row("| cell1 | cell2 |").unwrap();
        assert_eq!(row.cells.len(), 2);
        assert!(!row.cells[0].is_header);
        assert_eq!(row.cells[0].text, "cell1");
        assert!(!row.cells[1].is_header);
        assert_eq!(row.cells[1].text, "cell2");
    }

    #[test]
    fn parse_table_with_bg_color() {
        let row = parse_table_row("|<#red> error | 42 |").unwrap();
        assert_eq!(row.cells.len(), 2);
        assert_eq!(row.cells[0].bg_color.as_deref(), Some("red"));
        assert_eq!(row.cells[0].text, "error");
        assert_eq!(row.cells[1].text, "42");
    }

    #[test]
    fn parse_three_column_table() {
        let row = parse_table_row("|= Col1 |= Col2 |= Col3 |").unwrap();
        assert_eq!(row.cells.len(), 3);
        for cell in &row.cells {
            assert!(cell.is_header);
        }
    }

    #[test]
    fn table_render_header_bold() {
        let mut lc = ListCounters::new();
        let result = lc.render_line("|= Name |= Type |");
        assert!(result.contains("font-weight=\"bold\""));
        assert!(result.contains("Name"));
        assert!(result.contains("Type"));
    }

    #[test]
    fn table_render_data_cells() {
        let mut lc = ListCounters::new();
        let result = lc.render_line("| Alice | User |");
        assert!(result.contains("Alice"));
        assert!(result.contains("User"));
        assert!(!result.contains("font-weight=\"bold\""));
    }

    #[test]
    fn table_render_markup_in_cells() {
        let mut lc = ListCounters::new();
        let result = lc.render_line("|= **Bold** |= //Italic// |");
        assert!(result.contains("font-weight=\"bold\""));
        assert!(result.contains("font-style=\"italic\""));
    }

    // --- Tree tests ---

    #[test]
    fn parse_tree_simple() {
        match parse_line("|_ root") {
            CreoleLine::Tree(node) => {
                assert_eq!(node.depth, 1);
                assert_eq!(node.text, "root");
            }
            other => panic!("Expected Tree, got {other:?}"),
        }
    }

    #[test]
    fn parse_tree_nested() {
        match parse_line("|__ child") {
            CreoleLine::Tree(node) => {
                assert_eq!(node.depth, 2);
                assert_eq!(node.text, "child");
            }
            other => panic!("Expected Tree, got {other:?}"),
        }
        match parse_line("|___ grandchild") {
            CreoleLine::Tree(node) => {
                assert_eq!(node.depth, 3);
                assert_eq!(node.text, "grandchild");
            }
            other => panic!("Expected Tree, got {other:?}"),
        }
    }

    #[test]
    fn tree_render() {
        let mut lc = ListCounters::new();
        assert_eq!(lc.render_line("|_ root"), "root");
        assert_eq!(lc.render_line("|__ child"), "  child");
        assert_eq!(lc.render_line("|___ grandchild"), "    grandchild");
    }

    // --- Horizontal rule tests ---

    #[test]
    fn parse_hline_single() {
        assert_eq!(
            parse_line("----"),
            CreoleLine::HorizontalRule(HorizontalRuleStyle::Single)
        );
        // Longer runs also match.
        assert_eq!(
            parse_line("--------"),
            CreoleLine::HorizontalRule(HorizontalRuleStyle::Single)
        );
    }

    #[test]
    fn parse_hline_double() {
        assert_eq!(
            parse_line("===="),
            CreoleLine::HorizontalRule(HorizontalRuleStyle::Double)
        );
    }

    #[test]
    fn parse_hline_dotted() {
        assert_eq!(
            parse_line("...."),
            CreoleLine::HorizontalRule(HorizontalRuleStyle::Dotted)
        );
    }

    #[test]
    fn hline_render() {
        let mut lc = ListCounters::new();
        let single = lc.render_line("----");
        assert_eq!(single.chars().count(), 20);
        assert!(single.contains('\u{2500}'));

        let double = lc.render_line("====");
        assert!(double.contains('\u{2550}'));

        let dotted = lc.render_line("....");
        assert!(dotted.contains('\u{2508}'));
    }

    #[test]
    fn short_dashes_not_hline() {
        // `---` (only 3 dashes) should not be a horizontal rule.
        assert!(matches!(parse_line("---"), CreoleLine::Text(_)));
    }

    // --- parse_line classification tests ---

    #[test]
    fn parse_line_plain_text() {
        assert_eq!(
            parse_line("hello world"),
            CreoleLine::Text("hello world".to_string())
        );
    }

    #[test]
    fn parse_line_numbered() {
        assert_eq!(
            parse_line("# item"),
            CreoleLine::Numbered {
                level: 1,
                content: "item".to_string()
            }
        );
        assert_eq!(
            parse_line("## sub"),
            CreoleLine::Numbered {
                level: 2,
                content: "sub".to_string()
            }
        );
    }

    #[test]
    fn parse_line_bullet() {
        assert_eq!(
            parse_line("* item"),
            CreoleLine::Bullet {
                level: 1,
                content: "item".to_string()
            }
        );
        assert_eq!(
            parse_line("** nested"),
            CreoleLine::Bullet {
                level: 2,
                content: "nested".to_string()
            }
        );
    }

    #[test]
    fn parse_line_bold_not_bullet() {
        // `**bold**` has no space after stars — treated as text (bold markup).
        assert_eq!(
            parse_line("**bold**"),
            CreoleLine::Text("**bold**".to_string())
        );
    }

    #[test]
    fn mixed_list_resets_counters() {
        let mut lc = ListCounters::new();
        assert_eq!(
            lc.render_line("* unordered item"),
            "\u{2022} unordered item"
        );
        assert_eq!(lc.render_line("# ordered item"), "1. ordered item");
        assert_eq!(
            lc.render_line("* another unordered"),
            "\u{2022} another unordered"
        );
    }

    #[test]
    fn list_with_markup() {
        let mut lc = ListCounters::new();
        let result = lc.render_line("* **bold item**");
        assert!(result.contains('\u{2022}'));
        assert!(result.contains("font-weight=\"bold\""));
    }

    #[test]
    fn color_tag_emits_fill() {
        let result = to_svg_tspans("<color:red>red text</color>");
        assert_eq!(result, "<tspan fill=\"red\">red text</tspan>");
    }

    #[test]
    fn color_tag_hex() {
        let result = to_svg_tspans("<color:#0000FF>blue text</color>");
        assert_eq!(result, "<tspan fill=\"#0000FF\">blue text</tspan>");
    }

    #[test]
    fn color_tag_with_nested_bold() {
        let result = to_svg_tspans("<color:red>**Bold red**</color>");
        assert_eq!(
            result,
            "<tspan fill=\"red\"><tspan font-weight=\"bold\">Bold red</tspan></tspan>"
        );
    }

    #[test]
    fn color_tag_uppercase() {
        let result = to_svg_tspans("<COLOR:blue>text</color>");
        assert_eq!(result, "<tspan fill=\"blue\">text</tspan>");
    }

    #[test]
    fn openiconic_stripped_in_creole() {
        // OpenIconic references are silently stripped in creole processing.
        // Actual rendering happens at the SVG builder level.
        let result = to_svg_tspans("before <&heart> after");
        assert_eq!(result, "before  after");
    }

    #[test]
    fn openiconic_with_markup() {
        let result = to_svg_tspans("**bold** <&check>");
        assert!(result.contains("font-weight=\"bold\""));
        // Icon reference should be stripped, not rendered as escaped text.
        assert!(!result.contains("&amp;check"));
    }
}
