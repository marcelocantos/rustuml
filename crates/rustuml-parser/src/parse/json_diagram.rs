// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! JSON/YAML visualization diagram parser.
//!
//! Handles `@startjson`/`@endjson` and `@startyaml`/`@endyaml` blocks.
//! Supports `#highlight "key"` and `#highlight "key1" / "key2"` directives
//! for JSON diagrams.

use crate::diagram::{
    DiagramMeta,
    json_diagram::{DataFormat, JsonDiagram, JsonNode, JsonNodeValue},
};

use super::ParseError;

/// Parse a `@startjson` / `@endjson` block.
///
/// PlantUML's JSON parser is lenient — it accepts JSON without commas
/// between object fields or array elements.  We first try strict parsing
/// and, on failure, fall back to a lenient pass that inserts missing commas.
pub fn parse_json_diagram(lines: &[String]) -> Result<JsonDiagram, ParseError> {
    let (highlights, content) = extract_body(lines);
    // Carried text: decoded escapes are escaped where serde decodes them.
    let value: serde_json::Value = crate::java_utf16::decode_carried(
        &content,
        |text| {
            serde_json::from_str(text)
                .or_else(|_| serde_json::from_str(&insert_missing_commas(text)))
        },
        crate::java_utf16::escape_json,
    )
    .map_err(|e| ParseError {
        line: e.line(),
        message: format!("JSON parse error: {e}"),
    })?;
    let root = json_value_to_node(None, &value, &highlights, &[]);
    Ok(JsonDiagram {
        meta: DiagramMeta::default(),
        format: DataFormat::Json,
        root,
    })
}

/// Parse a `@startyaml` / `@endyaml` block.
pub fn parse_yaml_diagram(lines: &[String]) -> Result<JsonDiagram, ParseError> {
    let (_highlights, content) = extract_body(lines);
    let content = java_yaml_lines(&content);
    let value: serde_yml::Value = crate::java_utf16::decode_carried(
        &content,
        |text| serde_yml::from_str(text),
        crate::java_utf16::escape_yaml,
    )
    .map_err(|e| ParseError {
        line: e.location().map_or(0, |l| l.line()),
        message: format!("YAML parse error: {e}"),
    })?;
    let root = yaml_value_to_node(None, &value);
    Ok(JsonDiagram {
        meta: DiagramMeta::default(),
        format: DataFormat::Yaml,
        root,
    })
}

/// Java's `String.trim()`: characters up to U+0020 at both ends.
fn java_trim(text: &str) -> &str {
    text.trim_matches(|c: char| c <= ' ')
}

/// The `YamlLineType`s this port distinguishes.
#[derive(Debug, PartialEq, Eq)]
enum YamlLineKind {
    /// `EMPTY_LINE`: nothing but a comment or whitespace.
    Empty,
    /// `NO_KEY_ONLY_TEXT`: no key, not a list item.
    OnlyText,
    /// `KEY_AND_BLOCK_STYLE`: the trimmed value is `|`.
    BlockStyle,
    /// `KEY_AND_FLOW_SEQUENCE`: the value is `[...]`, read by `toList`.
    FlowSequence(Vec<String>),
    /// Every other type (keys, values, list items, a plain dash).
    Other,
}

/// What `YamlLine.build` (52-97) reads from one line whose TABs are already
/// four spaces (54).
struct YamlLine {
    kind: YamlLineKind,
    /// `getIndent()`: leading spaces, plus 2 for a `- ` list item, plus 1
    /// for a plain dash.
    indent: usize,
    /// Leading spaces of the line.
    spaces: usize,
    list_item: bool,
    /// `unquote(rawKey)` for the keyed kinds.
    key: String,
}

impl YamlLine {
    fn build(line: &str) -> Self {
        let spaces = line.len() - line.trim_start_matches(' ').len();
        let mut trimmed = remove_yaml_comment(java_trim(&line[spaces..]));
        let mut line_info = YamlLine {
            kind: YamlLineKind::Other,
            indent: spaces,
            spaces,
            list_item: false,
            key: String::new(),
        };
        if trimmed.is_empty() {
            line_info.kind = YamlLineKind::Empty;
            line_info.indent = 0;
            return line_info;
        }
        if trimmed == "-" {
            line_info.indent = spaces + 1;
            return line_info;
        }
        if let Some(rest) = trimmed.strip_prefix("- ") {
            line_info.list_item = true;
            line_info.indent = spaces + 2;
            trimmed = rest;
        }
        let Some(colon) = find_colon_separator(trimmed) else {
            if !line_info.list_item {
                line_info.kind = YamlLineKind::OnlyText;
            }
            return line_info;
        };
        line_info.key = unquote(java_trim(&trimmed[..colon])).to_string();
        let raw_value = java_trim(&trimmed[colon + 1..]);
        if raw_value == "|" {
            line_info.kind = YamlLineKind::BlockStyle;
        } else if raw_value.starts_with('[') && raw_value.ends_with(']') && raw_value.len() >= 2 {
            line_info.kind = YamlLineKind::FlowSequence(flow_sequence_values(
                &raw_value[1..raw_value.len() - 1],
            ));
        }
        line_info
    }

    /// The line's indentation and list prefix, rebuilt for serde.
    fn prefix(&self) -> String {
        format!(
            "{}{}",
            " ".repeat(self.spaces),
            if self.list_item { "- " } else { "" }
        )
    }
}

/// `YamlLine.removeYamlComment` (199-221): a leading `#`, or ` #` outside
/// quotes, starts a comment.
fn remove_yaml_comment(text: &str) -> &str {
    if text.starts_with('#') {
        return "";
    }
    let bytes = text.as_bytes();
    let mut quote = None;
    for (at, ch) in text.char_indices() {
        if ch == '\'' || ch == '"' {
            match quote {
                None => quote = Some(ch),
                Some(open) if open == ch => quote = None,
                Some(_) => {}
            }
        }
        if quote.is_none() && ch == ' ' && bytes.get(at + 1) == Some(&b'#') {
            return &text[..at];
        }
    }
    text
}

/// `YamlLine.findColonSeparator` (101-115): the first `:` outside quotes.
fn find_colon_separator(text: &str) -> Option<usize> {
    let mut quote = None;
    for (at, ch) in text.char_indices() {
        match quote {
            Some(open) if ch == open => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == ':' => return Some(at),
            None => {}
        }
    }
    None
}

/// `YamlLine.unquote` (180-191): one pair of matching quotes.
fn unquote(text: &str) -> &str {
    let bytes = text.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// `YamlLine.toList` (117-170): the fields of a flow sequence's inside.
/// Commas outside quotes separate fields; a field that starts (after
/// spaces) with a quote keeps its quoted text, other fields are trimmed; a
/// backslash appends the next character literally, inside or outside
/// quotes; the last field is always added.
fn flow_sequence_values(inner: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut quoted_field = false;
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if let Some(open) = quote {
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else if ch == open {
                quote = None;
            } else {
                current.push(ch);
            }
        } else if !quoted_field && java_trim(&current).is_empty() && (ch == '\'' || ch == '"') {
            quote = Some(ch);
            quoted_field = true;
            current.clear();
        } else if ch == ',' {
            result.push(field_text(&current, quoted_field));
            current.clear();
            quoted_field = false;
        } else if ch == '\\' {
            if let Some(next) = chars.next() {
                current.push(next);
            }
        } else {
            current.push(ch);
        }
    }
    result.push(field_text(&current, quoted_field));
    result
}

fn field_text(field: &str, quoted: bool) -> String {
    if quoted { field } else { java_trim(field) }.to_string()
}

/// A YAML single-quoted scalar holding `text` literally.
fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// A YAML double-quoted scalar holding `text` literally (a block value may
/// hold line breaks, TABs and backslashes).
fn double_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || (0x7F..=0x9F).contains(&(c as u32)) => {
                out.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// PlantUML reads YAML with its own parser (`yaml/parser/YamlParser`,
/// `YamlLine`); serde_yml reads what this pass hands it. The lines Java
/// reads differently from YAML are rewritten so that serde builds Java's
/// value:
///
/// * every line's TABs are four spaces (`YamlLine.build` 54) before it is
///   read, except the raw body lines of a block;
/// * a `key: |` line and its body become `key: "<value>"`, the value being
///   `YamlParser.getBlockStyleString` (132-149): the body runs until a line
///   that is neither text without a key nor empty and is indented at most
///   as the key; each body line is `cleanBlockStyle`'s trim (151-154) of the
///   raw line followed by a line break;
/// * a `key: [...]` line takes `toList`'s fields as strings (all YAML
///   values are strings in Java, `MonomorphToJson`);
/// * a backslash in a quoted key or value is literal (`unquote` strips one
///   pair of quotes; see [`keep_quoted_backslashes_in_line`]).
///
/// Folded `key: >` blocks are left to serde (Java throws `wip4` there).
fn java_yaml_lines(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut folded_indent: Option<usize> = None;
    let mut index = 0;
    while index < lines.len() {
        let raw = lines[index];
        index += 1;
        if let Some(block) = folded_indent {
            let indent = raw.len() - raw.trim_start().len();
            if raw.trim().is_empty() || indent > block {
                out.push(raw.to_string());
                continue;
            }
            folded_indent = None;
        }
        let line = raw.replace('\t', "    ");
        let yaml = YamlLine::build(&line);
        match yaml.kind {
            YamlLineKind::BlockStyle => {
                let mut value = String::new();
                while index < lines.len() {
                    let body = YamlLine::build(&lines[index].replace('\t', "    "));
                    if !matches!(body.kind, YamlLineKind::OnlyText | YamlLineKind::Empty)
                        && body.indent <= yaml.indent
                    {
                        break;
                    }
                    value.push_str(java_trim(lines[index]));
                    value.push('\n');
                    index += 1;
                }
                out.push(format!(
                    "{}{}: {}",
                    yaml.prefix(),
                    single_quoted(&yaml.key),
                    double_quoted(&value)
                ));
            }
            YamlLineKind::FlowSequence(ref values) => {
                let items: Vec<String> = values.iter().map(|v| single_quoted(v)).collect();
                out.push(format!(
                    "{}{}: [{}]",
                    yaml.prefix(),
                    single_quoted(&yaml.key),
                    items.join(", ")
                ));
            }
            _ => {
                if line.trim().ends_with(": >") {
                    folded_indent = Some(line.len() - line.trim_start().len());
                }
                if line.trim_start().starts_with('#') {
                    out.push(line);
                } else {
                    out.push(keep_quoted_backslashes_in_line(&line));
                }
            }
        }
    }
    out.join("\n")
}

/// PlantUML's YAML parser (`yaml/parser/YamlLine.build`) splits a line's key
/// and value at the first colon outside quotes and only strips one pair of
/// surrounding quotes (`unquote`): a backslash in a quoted key or value is
/// literal. serde_yml would decode it in a double-quoted scalar, so such a
/// scalar is handed to serde single-quoted with the same characters.
fn keep_quoted_backslashes_in_line(line: &str) -> String {
    let mut start = line.len() - line.trim_start().len();
    if line[start..].starts_with("- ") {
        start += 2;
    }
    // YamlLine.removeYamlComment: ' #' outside quotes starts a comment.
    let mut end = line.len();
    let mut quote = None;
    let bytes = line.as_bytes();
    for (offset, ch) in line[start..].char_indices() {
        let at = start + offset;
        if ch == '\'' || ch == '"' {
            match quote {
                None => quote = Some(ch),
                Some(open) if open == ch => quote = None,
                Some(_) => {}
            }
        }
        if quote.is_none() && ch == ' ' && bytes.get(at + 1) == Some(&b'#') {
            end = at;
            break;
        }
    }
    let body = &line[start..end];
    // YamlLine.findColonSeparator: the first ':' outside quotes.
    let mut quote = None;
    let mut colon = None;
    for (offset, ch) in body.char_indices() {
        match quote {
            Some(open) if ch == open => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == ':' => {
                colon = Some(offset);
                break;
            }
            None => {}
        }
    }
    let spans: Vec<std::ops::Range<usize>> = match colon {
        Some(colon) => vec![0..colon, colon + 1..body.len()],
        None => std::iter::once(0..body.len()).collect(),
    };
    let mut result = line.to_string();
    for span in spans.into_iter().rev() {
        let part = &body[span.clone()];
        let trimmed = part.trim();
        if trimmed.len() >= 2
            && trimmed.starts_with('"')
            && trimmed.ends_with('"')
            && trimmed.contains('\\')
        {
            let inner = &trimmed[1..trimmed.len() - 1];
            let lead = part.len() - part.trim_start().len();
            let at = start + span.start + lead;
            result.replace_range(
                at..at + trimmed.len(),
                &format!("'{}'", inner.replace('\'', "''")),
            );
        }
    }
    result
}

/// Insert commas where PlantUML's lenient parser allows them to be omitted.
///
/// Handles the common case: adjacent object fields or array elements on
/// separate lines without trailing commas.
fn insert_missing_commas(content: &str) -> String {
    let mut result = String::with_capacity(content.len() + 64);
    let mut prev_needs_comma = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // A new value/key starting means we may need a comma after the
        // previous line.
        let starts_value = trimmed.starts_with('"')
            || trimmed.starts_with('{')
            || trimmed.starts_with('[')
            || trimmed.starts_with("true")
            || trimmed.starts_with("false")
            || trimmed.starts_with("null")
            || trimmed
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_digit() || b == b'-');

        if prev_needs_comma && starts_value {
            result.push(',');
        }
        result.push_str(line);
        result.push('\n');

        // After this line, should the next line get a comma?
        // Yes if this line ends with a value (not with { [ , or :).
        let ends = trimmed.trim_end_matches(',');
        prev_needs_comma = ends.ends_with('"')
            || ends.ends_with('}')
            || ends.ends_with(']')
            || ends.ends_with("true")
            || ends.ends_with("false")
            || ends.ends_with("null")
            || ends.bytes().last().is_some_and(|b| b.is_ascii_digit());
        // Don't insert comma after lines that already have one.
        if trimmed.ends_with(',') || trimmed.ends_with(':') {
            prev_needs_comma = false;
        }
    }

    result
}

/// Extract `#highlight` directives and the body content from preprocessed lines.
///
/// The caller passes lines that have already been through the TIM preprocessor,
/// which means `@start*` / `@end*` markers have already been stripped.
/// This function separates `#highlight` directives from the data content and
/// applies `StyleExtractor` (59-98): lines Java's `trim()` empties are
/// skipped, and before the first data line a `skinparam` line (with its
/// `{` ... `}` block), `!assume`, `!pragma` and `hide` lines are consumed.
/// Neither diagram factory applies such a skinparam (only `handwritten` is
/// read, and not ported here).
fn extract_body(lines: &[String]) -> (Vec<Vec<String>>, String) {
    let mut highlights: Vec<Vec<String>> = Vec::new();
    let mut body: Vec<&str> = Vec::new();
    // `list.size() <= 1`: only the @start line is in StyleExtractor's list.
    let mut data_seen = false;

    let mut iter = lines.iter();
    while let Some(line) = iter.next() {
        let trimmed = line.trim();
        let java = java_trim(line);
        if java.is_empty() {
            continue;
        }
        if !data_seen {
            if java.starts_with("skinparam ") {
                if java.contains('{') {
                    let mut current = line;
                    while java_trim(current) != "}" {
                        match iter.next() {
                            Some(next) => current = next,
                            None => break,
                        }
                    }
                }
                continue;
            }
            if ["!assume ", "!pragma ", "hide "]
                .iter()
                .any(|prefix| java.starts_with(prefix))
            {
                continue;
            }
        }
        data_seen = true;
        if let Some(rest) = trimmed.strip_prefix("#highlight") {
            let parts: Vec<String> = rest
                .split('/')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                highlights.push(parts);
            }
        } else {
            body.push(line.as_str());
        }
    }

    (highlights, body.join("\n"))
}

fn is_highlighted(path: &[String], highlights: &[Vec<String>]) -> bool {
    highlights.iter().any(|h| h.as_slice() == path)
}

fn json_value_to_node(
    key: Option<String>,
    value: &serde_json::Value,
    highlights: &[Vec<String>],
    parent_path: &[String],
) -> JsonNode {
    let node_path: Vec<String> = match &key {
        Some(k) => {
            let mut p = parent_path.to_vec();
            p.push(k.clone());
            p
        }
        None => parent_path.to_vec(),
    };

    let highlighted = is_highlighted(&node_path, highlights);

    let node_value = match value {
        serde_json::Value::Null => JsonNodeValue::Null,
        serde_json::Value::Bool(b) => JsonNodeValue::Bool { val: *b },
        serde_json::Value::Number(n) => JsonNodeValue::Number { val: n.to_string() },
        serde_json::Value::String(s) => JsonNodeValue::Str { val: s.clone() },
        serde_json::Value::Array(arr) => {
            let items = arr
                .iter()
                .map(|v| json_value_to_node(None, v, highlights, &node_path))
                .collect();
            JsonNodeValue::Array { items }
        }
        serde_json::Value::Object(obj) => {
            let fields = obj
                .iter()
                .map(|(k, v)| json_value_to_node(Some(k.clone()), v, highlights, &node_path))
                .collect();
            JsonNodeValue::Object { fields }
        }
    };

    JsonNode {
        key,
        value: node_value,
        highlighted,
    }
}

fn yaml_value_to_node(key: Option<String>, value: &serde_yml::Value) -> JsonNode {
    let node_value = match value {
        serde_yml::Value::Null => JsonNodeValue::Null,
        serde_yml::Value::Bool(b) => JsonNodeValue::Bool { val: *b },
        serde_yml::Value::Number(n) => JsonNodeValue::Number { val: n.to_string() },
        serde_yml::Value::String(s) => JsonNodeValue::Str { val: s.clone() },
        serde_yml::Value::Sequence(arr) => {
            let items = arr.iter().map(|v| yaml_value_to_node(None, v)).collect();
            JsonNodeValue::Array { items }
        }
        serde_yml::Value::Mapping(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        serde_yml::Value::String(s) => s.clone(),
                        serde_yml::Value::Number(n) => n.to_string(),
                        serde_yml::Value::Bool(b) => b.to_string(),
                        _ => format!("{k:?}"),
                    };
                    yaml_value_to_node(Some(key_str), v)
                })
                .collect();
            JsonNodeValue::Object { fields }
        }
        serde_yml::Value::Tagged(tagged) => {
            return yaml_value_to_node(key, &tagged.value);
        }
    };

    JsonNode {
        key,
        value: node_value,
        highlighted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_extractor_drops_blank_lines_before_the_yaml_parser() {
        let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
        let diagram =
            parse_yaml_diagram(&lines("tide: |\n  Kelp one\n\n  Kelp two\nebb: y")).unwrap();
        let shown = format!("{:?}", diagram.root);
        assert!(shown.contains(r#"Kelp one\nKelp two\n""#), "{shown}");
    }

    #[test]
    fn yaml_block_values_keep_every_line_break_as_get_block_style_string() {
        let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
        let diagram =
            parse_yaml_diagram(&lines("note: |\n  Moss\n  Fern\nafter: x\nlast: |\n  Reed"))
                .unwrap();
        let shown = format!("{:?}", diagram.root);
        assert!(shown.contains(r#"Moss\nFern\n""#), "{shown}");
        assert!(shown.contains(r#"Reed\n""#), "{shown}");
    }

    #[test]
    fn yaml_block_body_is_get_block_style_string_with_trimmed_lines() {
        let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
        // A body line ending ': |' is body text (getBlockStyleString), and
        // every body line is trimmed (cleanBlockStyle).
        let diagram = parse_yaml_diagram(&lines(
            "tide: |\n  Kelp: |\n    Moss\n  Reed\nwrack:\n  deep: |\n      a: |\n      b\n  ebb: q",
        ))
        .unwrap();
        let shown = format!("{:?}", diagram.root);
        assert!(shown.contains(r#"Kelp: |\nMoss\nReed\n""#), "{shown}");
        assert!(shown.contains(r#"a: |\nb\n""#), "{shown}");
        assert!(!shown.contains("|+"), "{shown}");
        assert!(shown.contains(r#"key: Some("ebb")"#), "{shown}");
        // Text lines at any indent stay in the block; a key line at the
        // block's indent ends it. Raw TABs inside a body line are kept.
        let diagram =
            parse_yaml_diagram(&lines("note: |\n  one\tx\nloose text\nafter: x")).unwrap();
        let shown = format!("{:?}", diagram.root);
        assert!(shown.contains(r#"one\tx\nloose text\n""#), "{shown}");
    }

    #[test]
    fn yaml_line_tabs_are_four_spaces_and_flow_sequences_follow_to_list() {
        let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
        let diagram = parse_yaml_diagram(&lines(
            "k: a\tb\nm: \tlead\nf: [a\\nb, \"c\\\\d\", e\\tf, 1.50, 'x, y']",
        ))
        .unwrap();
        let shown = format!("{:?}", diagram.root);
        assert!(shown.contains(r#"val: "a    b""#), "{shown}");
        assert!(shown.contains(r#"val: "lead""#), "{shown}");
        for item in ["anb", r"c\\d", "etf", "1.50", "x, y"] {
            assert!(
                shown.contains(&format!("val: \"{item}\"")),
                "{item}: {shown}"
            );
        }
    }

    #[test]
    fn style_extractor_consumes_preamble_skinparams() {
        let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
        let yaml = parse_yaml_diagram(&lines("skinparam tabSize 4\nskinparam x {\n  y z\n}\nk: v"));
        assert!(format!("{:?}", yaml.unwrap().root).contains(r#"val: "v""#));
        let json = parse_json_diagram(&lines("hide empty\nskinparam tabSize 4\n{\"k\": 1}"));
        assert!(json.is_ok(), "{json:?}");
        // After the first data line, a skinparam line is data (Java: a
        // syntax error).
        assert!(parse_yaml_diagram(&lines("k: v\nskinparam tabSize 4")).is_err());
    }

    #[test]
    fn yaml_quoted_backslashes_stay_literal_as_in_yaml_line_unquote() {
        let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
        let diagram = parse_yaml_diagram(&lines(
            "\"Key \\x\": \"Amber \\U0010FFFD it's\" # note\nplain: \"no escape\"\nlist:\n  - \"a\\nb\"\ntext: |\n  \"kept \\t\"\n",
        ))
        .unwrap();
        let shown = format!("{:?}", diagram.root);
        // Debug shows each literal backslash doubled.
        assert!(shown.contains(r"Amber \\U0010FFFD it's"), "{shown}");
        assert!(shown.contains(r"Key \\x"), "{shown}");
        assert!(shown.contains(r"a\\nb"), "{shown}");
        assert!(shown.contains("no escape"), "{shown}");
        assert!(shown.contains(r#"\"kept \\t\""#), "{shown}");
    }
    use crate::diagram::json_diagram::JsonNodeValue;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    // Tests pass preprocessed lines (no @start*/@end* markers, as the TIM
    // preprocessor strips those before calling the diagram-specific parsers).

    fn find_field<'a>(fields: &'a [JsonNode], key: &str) -> &'a JsonNode {
        fields
            .iter()
            .find(|f| f.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("field '{key}' not found"))
    }

    #[test]
    fn parses_simple_json_object() {
        let input = lines("{\"name\": \"Alice\", \"age\": 30}");
        let diagram = parse_json_diagram(&input).unwrap();
        assert_eq!(diagram.format, DataFormat::Json);
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                assert_eq!(fields.len(), 2);
                assert!(fields.iter().any(|f| f.key.as_deref() == Some("name")));
                assert!(fields.iter().any(|f| f.key.as_deref() == Some("age")));
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_highlight_directive() {
        let input = lines("#highlight \"name\"\n{\"name\": \"Alice\", \"age\": 30}");
        let diagram = parse_json_diagram(&input).unwrap();
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                assert!(
                    find_field(fields, "name").highlighted,
                    "name should be highlighted"
                );
                assert!(
                    !find_field(fields, "age").highlighted,
                    "age should not be highlighted"
                );
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_nested_highlight() {
        let input = lines(
            "#highlight \"address\" / \"city\"\n{\"name\": \"Alice\", \"address\": {\"city\": \"Paris\"}}",
        );
        let diagram = parse_json_diagram(&input).unwrap();
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                let addr = find_field(fields, "address");
                match &addr.value {
                    JsonNodeValue::Object {
                        fields: addr_fields,
                    } => {
                        assert!(
                            find_field(addr_fields, "city").highlighted,
                            "city should be highlighted"
                        );
                    }
                    _ => panic!("expected nested Object"),
                }
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_simple_yaml() {
        let input = lines("name: Alice\nage: 30");
        let diagram = parse_yaml_diagram(&input).unwrap();
        assert_eq!(diagram.format, DataFormat::Yaml);
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].key.as_deref(), Some("name"));
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_yaml_list() {
        let input = lines("- apple\n- banana");
        let diagram = parse_yaml_diagram(&input).unwrap();
        match &diagram.root.value {
            JsonNodeValue::Array { items } => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected Array"),
        }
    }
}
