// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Creole markup — converts PlantUML text markup to SVG tspan elements.
//!
//! Supports inline markup (**bold**, //italic//, __underline__, --strikethrough--,
//! `<b>`, `<i>`, `<u>`, `<s>` HTML-style tags) and line-level constructs:
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
                // `-`, `_`, `~`, `"`, `` ` ``), consume the `~` and emit the char.
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
                    let is_markup_char = matches!(next_ch, '/' | '*' | '-' | '_' | '~' | '"' | '`');
                    if is_markup_char {
                        chars.next(); // consume the markup char
                        result.push(next_ch);
                        last_char = Some(next_ch);
                    } else {
                        // Non-markup char: tilde is literal.
                        result.push('~');
                        last_char = Some('~');
                        // Leave next_ch for the main loop to process.
                    }
                }
                // If c1 is None (~ at end of string), nothing to emit.
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
                    "strong" => {
                        let content = collect_until_tag(&mut chars, "</strong>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(result, "<tspan font-weight=\"bold\">{inner}</tspan>").unwrap();
                    }
                    "em" => {
                        let content = collect_until_tag(&mut chars, "</em>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(result, "<tspan font-style=\"italic\">{inner}</tspan>").unwrap();
                    }
                    "ins" => {
                        let content = collect_until_tag(&mut chars, "</ins>");
                        let inner = to_svg_tspans_inner(&content, skip_underline);
                        write!(
                            result,
                            "<tspan text-decoration=\"underline\">{inner}</tspan>"
                        )
                        .unwrap();
                    }
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
                        let escaped_color = escape_creole_text(color);
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
                        write!(result, "<tspan font-family=\"{font_name}\">{inner}</tspan>")
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
    /// Absolute font-size override from `<size:N>` (pixels).
    pub size: Option<u32>,
    /// Custom font-family from `<font:Name>` (only used when not monospace).
    pub font_family: Option<String>,
    /// `"sub"` or `"super"` from `<sub>` / `<sup>`.
    pub baseline_shift: Option<&'static str>,
    /// Hyperlink target from `[[url label]]`; carries blue underline styling.
    pub link_url: Option<String>,
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

/// Reverse the XML escaping applied during segment building so the result
/// matches the source string a font-metric calculation expects.
fn unescape_for_metrics(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn parse_segments_inner(text: &str, skip_underline: bool) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let style = Style::default();
    walk_segments(text, &style, skip_underline, &mut out);
    out
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
    let escaped = escape_creole_text(text);
    let final_text = if style.monospace {
        escaped.replace(' ', "\u{00a0}")
    } else {
        escaped
    };
    push_segment(out, &final_text, style);
}

fn walk_segments(text: &str, style: &Style, skip_underline: bool, out: &mut Vec<Segment>) {
    let mut chars = text.chars().peekable();
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
                chars.next();
                let (content, found) = collect_until(&mut chars, "~~");
                flush_buf!();
                if found {
                    let mut nested = style.clone();
                    nested.wavy_underline = true;
                    push_literal(out, &content, &nested);
                    last_char = content.chars().last();
                } else {
                    push_literal(out, "~~", style);
                    push_literal(out, &content, style);
                    last_char = content.chars().last().or(Some('~'));
                }
            }
            '~' => {
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
                    buf.push(ch1);
                    buf.push(ch2);
                    last_char = Some(ch2);
                } else if let Some(next_ch) = c1 {
                    let is_markup_char = matches!(next_ch, '/' | '*' | '-' | '_' | '~' | '"' | '`');
                    if is_markup_char {
                        chars.next();
                        buf.push(next_ch);
                        last_char = Some(next_ch);
                    } else {
                        buf.push('~');
                        last_char = Some('~');
                    }
                }
            }
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                let (content, _found) = collect_until(&mut chars, "\"\"");
                flush_buf!();
                let mut nested = style.clone();
                nested.monospace = true;
                walk_segments(&content, &nested, skip_underline, out);
                last_char = content.chars().last();
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
                chars.next();
                let (content, found) = collect_until(&mut chars, "**");
                flush_buf!();
                if found {
                    let mut nested = style.clone();
                    nested.bold = true;
                    walk_segments(&content, &nested, skip_underline, out);
                    last_char = content.chars().last();
                } else {
                    push_literal(out, "**", style);
                    push_literal(out, &content, style);
                    last_char = content.chars().last().or(Some('*'));
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                let preceded_by_word = last_char.map(|c| !c.is_whitespace()).unwrap_or(false);
                if preceded_by_word {
                    buf.push('/');
                    buf.push('/');
                    chars.next();
                    last_char = Some('/');
                } else {
                    chars.next();
                    let (content, found) = collect_until(&mut chars, "//");
                    flush_buf!();
                    if found && !content.is_empty() {
                        let mut nested = style.clone();
                        nested.italic = true;
                        walk_segments(&content, &nested, skip_underline, out);
                        last_char = content.chars().last();
                    } else if found {
                        push_literal(out, "////", style);
                        last_char = Some('/');
                    } else {
                        push_literal(out, "//", style);
                        push_literal(out, &content, style);
                        last_char = content.chars().last().or(Some('/'));
                    }
                }
            }
            '_' if chars.peek() == Some(&'_') => {
                chars.next();
                if skip_underline {
                    buf.push('_');
                    buf.push('_');
                    last_char = Some('_');
                } else {
                    let (content, found) = collect_until(&mut chars, "__");
                    flush_buf!();
                    if found {
                        let mut nested = style.clone();
                        nested.underline = true;
                        walk_segments(&content, &nested, skip_underline, out);
                        last_char = content.chars().last();
                    } else {
                        push_literal(out, "__", style);
                        push_literal(out, &content, style);
                        last_char = content.chars().last().or(Some('_'));
                    }
                }
            }
            '_' => {
                buf.push('_');
                last_char = Some('_');
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                let (content, found) = collect_until(&mut chars, "--");
                flush_buf!();
                if found {
                    let mut nested = style.clone();
                    nested.line_through = true;
                    walk_segments(&content, &nested, skip_underline, out);
                    last_char = content.chars().last();
                } else {
                    push_literal(out, "--", style);
                    push_literal(out, &content, style);
                    last_char = content.chars().last().or(Some('-'));
                }
            }
            '<' => {
                let tag = collect_until_char(&mut chars, '>');
                flush_buf!();
                handle_tag(&tag, &mut chars, style, skip_underline, out);
                last_char = None; // tag content already emitted
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
                let inner = if let Some(brace) = inner.find('{') {
                    if let Some(end) = inner.find('}') {
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
                nested.fill = Some("#0000FF".to_string());
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

fn handle_tag(
    tag: &str,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    style: &Style,
    skip_underline: bool,
    out: &mut Vec<Segment>,
) {
    match tag {
        "b" | "strong" => walk_with(
            chars,
            "</b>".replace("b", tag),
            tag,
            "strong",
            style,
            |s| s.bold = true,
            skip_underline,
            out,
        ),
        "i" | "em" => walk_with(
            chars,
            format!("</{tag}>"),
            tag,
            "em",
            style,
            |s| s.italic = true,
            skip_underline,
            out,
        ),
        "u" => walk_with(
            chars,
            "</u>".into(),
            tag,
            "",
            style,
            |s| s.underline = true,
            skip_underline,
            out,
        ),
        "ins" => walk_with(
            chars,
            "</ins>".into(),
            tag,
            "",
            style,
            |s| s.underline = true,
            skip_underline,
            out,
        ),
        "s" => walk_with(
            chars,
            "</s>".into(),
            tag,
            "",
            style,
            |s| s.line_through = true,
            skip_underline,
            out,
        ),
        "del" => walk_with(
            chars,
            "</del>".into(),
            tag,
            "",
            style,
            |s| s.line_through = true,
            skip_underline,
            out,
        ),
        "strike" => walk_with(
            chars,
            "</strike>".into(),
            tag,
            "",
            style,
            |s| s.line_through = true,
            skip_underline,
            out,
        ),
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
        _ if tag.starts_with("color:") || tag.starts_with("COLOR:") => {
            let color = tag
                .split_once(':')
                .map(|(_, c)| c.to_string())
                .unwrap_or_default();
            let content = collect_until_tag(chars, "</color>");
            let mut nested = style.clone();
            nested.fill = Some(color);
            walk_segments(&content, &nested, skip_underline, out);
        }
        _ if tag.starts_with("size:") => {
            let size = tag["size:".len()..].parse::<u32>().ok();
            let content = collect_until_tag(chars, "</size>");
            let mut nested = style.clone();
            if let Some(s) = size {
                nested.size = Some(s);
            }
            walk_segments(&content, &nested, skip_underline, out);
        }
        _ if tag.starts_with("font:") || tag.starts_with("FONT:") => {
            let font_name = tag
                .split_once(':')
                .map(|(_, n)| n.to_string())
                .unwrap_or_default();
            let content = collect_until_tag(chars, "</font>");
            let mut nested = style.clone();
            nested.font_family = Some(font_name);
            // <font:Name> behaves like monospace for spacing per existing impl.
            nested.monospace = true;
            walk_segments(&content, &nested, skip_underline, out);
        }
        _ if tag.starts_with("font") => {
            let content = collect_until_tag(chars, "</font>");
            walk_segments(&content, style, skip_underline, out);
        }
        _ if tag.starts_with("back:") || tag.starts_with("BACK:") => {
            let content = collect_until_tag(chars, "</back>");
            walk_segments(&content, style, skip_underline, out);
        }
        _ if tag.starts_with("img:") => {
            let raw_src = &tag["img:".len()..];
            let src = raw_src.find('{').map(|i| &raw_src[..i]).unwrap_or(raw_src);
            let fallback = if src.starts_with("https://") || src.starts_with("http://") {
                format!("(Cannot\u{00a0}decode:\u{00a0}{})", escape_creole_text(src))
            } else {
                "(Cannot\u{00a0}decode)".to_string()
            };
            let mut nested = style.clone();
            nested.monospace = true;
            push_segment(out, &fallback, &nested);
        }
        _ if tag.starts_with("&amp;") || tag.starts_with('&') => {
            // OpenIconic icon — handled at the SVG builder level via text_with_icons.
        }
        _ => {
            // Unknown tag — emit literally.
            push_literal(out, &format!("<{tag}>"), style);
        }
    }
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

    #[test]
    fn segments_plain_text() {
        assert_eq!(
            parse_segments("hello world"),
            vec![seg("hello world", Style::default())]
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
        let mut s = Style::default();
        s.bold = true;
        s.italic = true;
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
        let mut s = Style::default();
        s.line_through = true;
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
    fn segments_unmatched_bold_is_literal() {
        let segs = parse_segments("**no close");
        assert_eq!(segs, vec![seg("**no close", Style::default())]);
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
