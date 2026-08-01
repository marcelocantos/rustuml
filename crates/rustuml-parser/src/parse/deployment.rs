// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::deployment::*;
use crate::diagram::style::{PlantUmlColorType, PlantUmlColors};
use crate::diagram::{DiagramMeta, LegendHorizontalAlignment, LegendVerticalAlignment};

/// Leaf-only symbols accepted by Java description commands. Braced container
/// symbols come from `DeploymentContainerSymbol::COMMAND_SYMBOLS`.
const DEPLOYMENT_LEAF_ONLY_KEYWORDS: &[&str] = &[
    "actor",
    "agent",
    "boundary",
    "collections",
    "control",
    "entity",
];

pub(super) fn is_deployment_keyword(keyword: &str) -> bool {
    DeploymentContainerSymbol::from_command_keyword(keyword).is_some()
        || DEPLOYMENT_LEAF_ONLY_KEYWORDS
            .iter()
            .any(|candidate| keyword.eq_ignore_ascii_case(candidate))
}

/// Convert a quoted label like "Application Server" to a stable ID:
/// replace whitespace/dots/hyphens with underscores, strip remaining
/// non-alphanumeric-underscore characters.
fn label_to_id(label: &str) -> String {
    let mut id = String::new();
    for ch in label.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            id.push(ch);
        } else if ch == ' ' || ch == '-' || ch == '.' {
            id.push('_');
        }
    }
    if id.is_empty() {
        label.replace(|c: char| !c.is_alphanumeric(), "_")
    } else {
        id
    }
}

/// Resolve a raw string (possibly a quoted label) to a node ID.
/// If an existing node has that ID, return it. If it matches an existing
/// label, return that node's ID. Otherwise derive an ID via label_to_id.
fn resolve_id(nodes: &[DeploymentNode], raw: &str) -> String {
    if nodes.iter().any(|n| n.id == raw) {
        return raw.to_string();
    }
    if let Some(node) = nodes.iter().find(|n| n.label == raw) {
        return node.id.clone();
    }
    label_to_id(raw)
}

fn resolve_connection_endpoint(raw: &str) -> String {
    // Java `CommandLinkElement.getDummy` calls `DescriptionDiagram.cleanId`
    // and then asks the quark registry about that exact code. `parse_endpoint`
    // has already removed ordinary quotes, so neither display-label lookup nor
    // declaration-oriented `label_to_id` normalization belongs on this path.
    raw.to_string()
}

fn deployment_identity_exists(
    nodes: &[DeploymentNode],
    notes: &[DeploymentNote],
    id: &str,
) -> bool {
    nodes.iter().any(|node| node.id == id)
        || notes.iter().any(|note| note.id.as_deref() == Some(id))
}

fn kind_from_keyword(keyword: &str) -> DeploymentNodeKind {
    if let Some(symbol) = DeploymentContainerSymbol::from_command_keyword(keyword) {
        return symbol.renderer_kind();
    }
    match keyword {
        "actor" => DeploymentNodeKind::Actor,
        "agent" => DeploymentNodeKind::Agent,
        "boundary" => DeploymentNodeKind::Boundary,
        "collections" => DeploymentNodeKind::Collections,
        "control" => DeploymentNodeKind::Control,
        "entity" => DeploymentNodeKind::Entity,
        _ => DeploymentNodeKind::Node,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_node(
    nodes: &mut Vec<DeploymentNode>,
    next_quark_order: &mut usize,
    id: String,
    label: String,
    kind: DeploymentNodeKind,
    container_symbol: Option<DeploymentContainerSymbol>,
    stereotype: Option<String>,
    color: Option<String>,
    declared_container: bool,
    source_line: usize,
) -> bool {
    if !nodes.iter().any(|n| n.id == id) {
        let quark_order = *next_quark_order;
        *next_quark_order += 1;
        nodes.push(DeploymentNode {
            id,
            label,
            kind,
            container_symbol,
            stereotype,
            color,
            declared_container,
            children: Vec::new(),
            source_line,
            quark_order,
        });
        return true;
    }
    false
}

fn add_child(nodes: &mut [DeploymentNode], parent_id: &str, child_id: &str) {
    if let Some(parent) = nodes.iter_mut().find(|n| n.id == parent_id) {
        let child_str = child_id.to_string();
        if !parent.children.contains(&child_str) {
            parent.children.push(child_str);
        }
    }
}

/// Extract a quoted or bare identifier and the remaining tail from a string.
/// Returns `(raw_label, rest)` where `raw_label` is the unquoted text.
fn parse_endpoint(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if let Some(inner) = s.strip_prefix('"') {
        // Quoted: find closing quote.
        let close = inner.find('"')?;
        let label = &inner[..close];
        let rest = &inner[close + 1..];
        Some((label, rest))
    } else {
        // Bare: take up to whitespace or end.
        let end = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
        if end == 0 {
            return None;
        }
        Some((&s[..end], &s[end..]))
    }
}

/// Process a raw label (from a quoted string) and convert `\n` escape sequences
/// to actual newlines, as PlantUML does for quoted element labels.
fn process_label(raw: &str) -> String {
    raw.replace("\\n", "\n")
}

struct ParsedDeploymentConnection {
    raw_from: String,
    raw_to: String,
    label: Option<String>,
    tail_label: Option<String>,
    head_label: Option<String>,
    arrow_at_start: bool,
    arrow_at_end: bool,
    direction: Option<DeploymentLinkDirection>,
    style: DeploymentLinkStyle,
    hidden: bool,
    length: usize,
}

fn deployment_arrow_style_is_valid(style: &str, allow_multiple: bool) -> bool {
    let mut groups = style.split(';');
    let Some(first) = groups.next() else {
        return false;
    };
    let mut valid_group = |group: &str| {
        !group.is_empty()
            && group.split(',').all(|token| {
                let token = token.trim();
                matches!(
                    token.to_ascii_lowercase().as_str(),
                    "dotted"
                        | "dashed"
                        | "plain"
                        | "bold"
                        | "hidden"
                        | "norank"
                        | "single"
                        | "node"
                ) || token.strip_prefix("thickness=").is_some_and(|value| {
                    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
                }) || token.strip_prefix('#').is_some_and(|value| {
                    !value.is_empty() && value.chars().all(|c| c.is_alphanumeric() || c == '_')
                })
            })
    };
    if !valid_group(first) {
        return false;
    }
    if !allow_multiple {
        return groups.next().is_none();
    }
    groups.all(&mut valid_group)
}

fn deployment_arrow_end(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut index = 0;
    let mut saw_shaft = false;
    let mut saw_direction = false;
    let mut style_count = 0;
    while index < bytes.len() {
        let character = bytes[index] as char;
        if matches!(character, '-' | '.' | '~' | '=' | '<' | '>' | '|') {
            saw_shaft |= matches!(character, '-' | '.' | '~' | '=');
            index += 1;
            continue;
        }
        if character == '[' {
            if !saw_shaft || style_count == 2 {
                break;
            }
            let relative_end = input[index + 1..].find(']')?;
            let style_end = index + relative_end + 1;
            let allow_multiple = style_count == 0 && !saw_direction;
            if !deployment_arrow_style_is_valid(&input[index + 1..style_end], allow_multiple) {
                let suffix = &input[style_end + 1..];
                let continues_arrow = suffix.starts_with(['-', '.', '~', '=', '<', '>', '|', '['])
                    || ["down", "up", "left", "right"]
                        .iter()
                        .any(|direction| suffix.starts_with(direction));
                if continues_arrow {
                    return None;
                }
                break;
            }
            index += relative_end + 2;
            style_count += 1;
            continue;
        }
        if saw_shaft && !saw_direction && matches!(character, 'd' | 'u' | 'l' | 'r') {
            let tail = &input[index..];
            let direction_length = ["down", "up", "left", "right"]
                .iter()
                .find(|direction| tail.starts_with(**direction))
                .map_or(0, |direction| direction.len());
            if direction_length > 0
                && index + direction_length < bytes.len()
                && matches!(
                    bytes[index + direction_length] as char,
                    '-' | '.' | '~' | '=' | '['
                )
            {
                index += direction_length;
                saw_direction = true;
                continue;
            }
        }
        break;
    }
    Some(index)
}

fn deployment_arrow_shaft(arrow: &str) -> String {
    let mut shaft = String::with_capacity(arrow.len());
    let mut in_style = false;
    for character in arrow.chars() {
        match character {
            '[' => in_style = true,
            ']' => in_style = false,
            _ if !in_style => shaft.push(character),
            _ => {}
        }
    }
    shaft
}

fn deployment_arrow_has_style(arrow: &str, expected: &str) -> bool {
    arrow
        .split('[')
        .skip(1)
        .filter_map(|suffix| suffix.split_once(']').map(|(style, _)| style))
        .flat_map(|style| style.split([',', ';']))
        .any(|style| style.trim().eq_ignore_ascii_case(expected))
}

/// Try to parse a connection from a trimmed line.
/// Handles:
///   - `from_id --> to_id : label`
///   - `"From Label" --> "To Label" : label`
///   - `keyword "From Label" --> to_id : label`  (uses keyword as FROM id)
///
/// The arrow is any combination of `-`, `.`, `~`, `=`, `<`, `>`, `|`
/// characters (2+ chars).
fn try_parse_connection(trimmed: &str) -> Option<ParsedDeploymentConnection> {
    let rest = trimmed;

    // Check if the line starts with a deployment keyword followed by a quoted label
    // and then an arrow. In that case, use the keyword itself as the FROM identifier
    // (matching PlantUML's behaviour where `artifact "label" --> X` creates a FROM
    // endpoint named "artifact", not "label").
    {
        let kw_end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if kw_end < rest.len() {
            let kw = &rest[..kw_end];
            if is_deployment_keyword(kw) {
                let after_kw = rest[kw_end..].trim_start();
                if let Some(after_open_quote) = after_kw.strip_prefix('"') {
                    // `CommandLinkElement` parses the quoted text between the
                    // first endpoint and arrow as the tail quantifier.
                    if let Some(close_quote) = after_open_quote.find('"') {
                        let tail_label = process_label(&after_open_quote[..close_quote]);
                        let after_label = after_open_quote[close_quote + 1..].trim_start();
                        // Check if what follows is an arrow.
                        let arrow_end = deployment_arrow_end(after_label)?;
                        if arrow_end >= 2 {
                            let arrow = &after_label[..arrow_end];
                            let shaft = deployment_arrow_shaft(arrow);
                            // A valid arrow must have a shaft character.
                            // Pure `<<` is a stereotype opener, not an arrow.
                            if shaft.chars().any(|c| matches!(c, '-' | '.' | '~' | '=')) {
                                // keyword "label" ARROW target — use keyword as FROM.
                                let after_arrow = after_label[arrow_end..].trim_start();
                                let (raw_to, after_to) = parse_endpoint(after_arrow)?;
                                let after_to = after_to.trim_start();
                                let label = if let Some(rest_label) = after_to.strip_prefix(':') {
                                    let lbl = rest_label.trim().to_string();
                                    if lbl.is_empty() { None } else { Some(lbl) }
                                } else if after_to.is_empty() {
                                    None
                                } else {
                                    return None;
                                };
                                return Some(ParsedDeploymentConnection {
                                    raw_from: kw.to_string(),
                                    raw_to: raw_to.to_string(),
                                    label,
                                    tail_label: Some(tail_label),
                                    head_label: None,
                                    arrow_at_start: shaft.starts_with('<'),
                                    arrow_at_end: shaft.ends_with('>'),
                                    direction: deployment_link_direction(&shaft),
                                    style: deployment_link_style(&shaft),
                                    hidden: deployment_arrow_has_style(arrow, "hidden"),
                                    length: deployment_link_length(&shaft),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse FROM endpoint.
    let (raw_from, after_from) = parse_endpoint(rest)?;
    let mut after_from = after_from.trim_start();
    let tail_label = if let Some(inner) = after_from.strip_prefix('"') {
        let close = inner.find('"')?;
        let label = process_label(&inner[..close]);
        after_from = inner[close + 1..].trim_start();
        Some(label)
    } else {
        None
    };

    // Parse arrow: one or more shaft/decor characters, optionally with
    // an embedded direction keyword `-down-`, `-up-`, `-left-`, `-right-`
    // (PlantUML uses these to hint layout direction).
    let arrow_end = deployment_arrow_end(after_from)?;
    if arrow_end < 2 {
        // Need at least 2 arrow characters (e.g., `--`, `->`, `..`).
        return None;
    }
    let arrow = &after_from[..arrow_end];
    let shaft = deployment_arrow_shaft(arrow);
    // Must contain at least one shaft character.
    // Pure `<<...>>` is a stereotype, not an arrow.
    if !shaft.chars().any(|c| matches!(c, '-' | '.' | '~' | '=')) {
        return None;
    }
    let after_arrow = after_from[arrow_end..].trim_start();

    // A quoted head quantifier is distinguishable from a quoted endpoint when
    // another endpoint follows it.
    let (first_to, first_rest) = parse_endpoint(after_arrow)?;
    let first_rest = first_rest.trim_start();
    let (head_label, raw_to, after_to) = if !first_rest.is_empty()
        && !first_rest.starts_with(':')
        && let Some((raw_to, after_to)) = parse_endpoint(first_rest)
    {
        (Some(process_label(first_to)), raw_to, after_to.trim_start())
    } else {
        (None, first_to, first_rest)
    };
    let after_to = after_to.trim_start();

    // Optional label after colon.
    let label = if let Some(rest_label) = after_to.strip_prefix(':') {
        let lbl = rest_label.trim().to_string();
        if lbl.is_empty() { None } else { Some(lbl) }
    } else if after_to.is_empty() {
        None
    } else {
        // Unexpected content — not a valid connection line.
        return None;
    };

    Some(ParsedDeploymentConnection {
        raw_from: raw_from.to_string(),
        raw_to: raw_to.to_string(),
        label,
        tail_label,
        head_label,
        arrow_at_start: shaft.starts_with('<'),
        arrow_at_end: shaft.ends_with('>'),
        direction: deployment_link_direction(&shaft),
        style: deployment_link_style(&shaft),
        hidden: deployment_arrow_has_style(arrow, "hidden"),
        length: deployment_link_length(&shaft),
    })
}

fn deployment_link_direction(arrow: &str) -> Option<DeploymentLinkDirection> {
    [
        ("down", DeploymentLinkDirection::Down),
        ("up", DeploymentLinkDirection::Up),
        ("left", DeploymentLinkDirection::Left),
        ("right", DeploymentLinkDirection::Right),
    ]
    .into_iter()
    .find_map(|(keyword, direction)| arrow.contains(keyword).then_some(direction))
}

fn deployment_link_style(arrow: &str) -> DeploymentLinkStyle {
    // Java `CommandLinkElement.getLinkType` inspects the combined shaft.
    if arrow.contains('.') {
        DeploymentLinkStyle::Dashed
    } else if arrow.contains('~') {
        DeploymentLinkStyle::Dotted
    } else if arrow.contains('=') {
        DeploymentLinkStyle::Bold
    } else {
        DeploymentLinkStyle::Solid
    }
}

fn deployment_link_length(arrow: &str) -> usize {
    arrow
        .chars()
        .filter(|character| matches!(character, '-' | '.' | '~' | '='))
        .count()
}

fn deployment_note_position(value: &str) -> DeploymentNotePosition {
    match value {
        "top" => DeploymentNotePosition::Top,
        "bottom" => DeploymentNotePosition::Bottom,
        "left" => DeploymentNotePosition::Left,
        _ => DeploymentNotePosition::Right,
    }
}

enum DeploymentLinkNoteCommand {
    Inline(DeploymentLinkNote),
    Multiline {
        colors: PlantUmlColors,
        position: DeploymentNotePosition,
    },
}

fn is_java_pattern_space(character: char) -> bool {
    matches!(
        character,
        ' ' | '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | '\u{00A0}'
    )
}

fn trim_java_pattern_space_start(value: &str) -> &str {
    value.trim_start_matches(is_java_pattern_space)
}

fn single_line_link_note_suffix(suffix: &str) -> Option<(Option<&str>, &str)> {
    if let Some(body) = suffix.strip_prefix(':') {
        return Some((None, trim_java_pattern_space_start(body)));
    }
    if !suffix.starts_with('#') {
        return None;
    }

    // Java registers the single-line command first. Within its optional color
    // leaf PART2 is attempted before COLOR_REGEXP, and each alternative may
    // backtrack until the rest of the complete command regex can consume the
    // body colon.
    for part2 in [true, false] {
        for end in suffix
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(suffix.len()))
            .filter(|end| *end > 1)
            .rev()
        {
            let color = &suffix[..end];
            let matches = if part2 {
                super::plantuml_color_matches_part2_syntax(color)
            } else {
                super::plantuml_color_matches_ordinary_syntax(color)
            };
            if !matches {
                continue;
            }
            let remainder = &suffix[end..];
            let after_space = trim_java_pattern_space_start(remainder);
            let Some(body) = after_space.strip_prefix(':') else {
                continue;
            };
            // PART2's final negative lookahead excludes an immediate colon.
            // A consumed semicolon or intervening Pattern2 space makes the
            // boundary explicit; otherwise only COLOR_REGEXP can own it.
            if part2 && remainder.starts_with(':') && !color.ends_with(';') {
                continue;
            }
            return Some((Some(color), trim_java_pattern_space_start(body)));
        }
    }
    None
}

/// Parse the command owned by Java `CommandFactoryNoteOnLink`.
///
/// The outer `Option` distinguishes this factory's prefix from unrelated note
/// commands. Once the prefix matches, malformed suffixes stay terminal through
/// the inner `Result`, matching `PSystemCommandFactory` command ownership.
fn parse_deployment_link_note_command(
    line: &str,
) -> Option<Result<DeploymentLinkNoteCommand, &'static str>> {
    static PREFIX: LazyLock<Regex> = LazyLock::new(|| {
        // Java provenance: `CommandFactoryNoteOnLink` lines 75-103 at
        // 71806a23780b04a5ccde2f8ceb5121edad5eb711. The optional position is
        // followed by zero-or-more spaces, so Java also accepts `lefton`.
        Regex::new(r"(?i)^note\s+(?:(right|left|top|bottom)\s*)?(?:on|of)\s+link\b(.*)$").unwrap()
    });

    let captures = PREFIX.captures(line)?;
    let position = captures
        .get(1)
        .map(|value| deployment_note_position(&value.as_str().to_ascii_lowercase()))
        // `CommandFactoryNoteOnLink.executeInternal` defaults to BOTTOM.
        .unwrap_or(DeploymentNotePosition::Bottom);
    let suffix = trim_java_pattern_space_start(captures.get(2)?.as_str());
    if let Some((color, text)) = single_line_link_note_suffix(suffix) {
        let colors = if let Some(color) = color {
            let Some(colors) = super::parse_plantuml_colors(color, PlantUmlColorType::Back) else {
                return Some(Err("invalid note-on-link color"));
            };
            colors
        } else {
            PlantUmlColors::default()
        };
        return Some(Ok(DeploymentLinkNoteCommand::Inline(DeploymentLinkNote {
            text: text.to_string(),
            colors,
            position,
        })));
    }

    if suffix.is_empty() {
        return Some(Ok(DeploymentLinkNoteCommand::Multiline {
            colors: PlantUmlColors::default(),
            position,
        }));
    }
    if suffix.starts_with('#') {
        if !super::plantuml_color_matches_part2_syntax(suffix)
            && !super::plantuml_color_matches_ordinary_syntax(suffix)
        {
            return Some(Err("invalid note-on-link color"));
        }
        let Some(colors) = super::parse_plantuml_colors(suffix, PlantUmlColorType::Back) else {
            return Some(Err("invalid note-on-link color"));
        };
        return Some(Ok(DeploymentLinkNoteCommand::Multiline {
            colors,
            position,
        }));
    }
    Some(Err("invalid note-on-link command"))
}

fn last_note_eligible_connection_index(
    connections: &[DeploymentConnection],
    notes: &[DeploymentNote],
) -> Option<usize> {
    let is_note_id = |id: &str| {
        notes
            .iter()
            .any(|note| note.id.as_deref().is_some_and(|note_id| note_id == id))
    };
    connections
        .iter()
        .rposition(|connection| !is_note_id(&connection.from) && !is_note_id(&connection.to))
}

/// Accumulator for multiline note bodies.
struct NoteAccum {
    target: Option<String>,
    id: Option<String>,
    color: Option<String>,
    tags: Vec<String>,
    stereotype: Option<String>,
    owner: Option<String>,
    quark_order: usize,
    position: DeploymentNotePosition,
    source_line: usize,
    lines: Vec<String>,
}

struct LinkNoteAccum {
    connection_index: usize,
    colors: PlantUmlColors,
    position: DeploymentNotePosition,
    command_line: usize,
    lines: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct DeploymentNodeCommand {
    id: String,
    label: String,
    kind: DeploymentNodeKind,
    container_symbol: Option<DeploymentContainerSymbol>,
    stereotype: Option<String>,
    color: Option<String>,
    declared_container: bool,
}

fn node_command_from_captures(
    captures: regex::Captures<'_>,
    container_symbol: Option<DeploymentContainerSymbol>,
) -> Option<DeploymentNodeCommand> {
    let keyword = captures.name("keyword")?.as_str().to_ascii_lowercase();
    if container_symbol.is_none() && !is_deployment_keyword(&keyword) {
        return None;
    }

    let raw_code = captures.name("code")?.as_str();
    let id = raw_code.trim_matches('"').to_string();
    let label = captures
        .name("display")
        .map(|value| process_label(value.as_str()))
        .unwrap_or_else(|| process_label(&id));
    let stereotype = captures
        .name("pre_stereotype")
        .or_else(|| captures.name("stereotype"))
        .map(|value| value.as_str().trim().to_string());
    let color = captures
        .name("color")
        .map(|value| value.as_str().to_string());

    Some(DeploymentNodeCommand {
        id,
        label,
        kind: container_symbol.map_or_else(|| kind_from_keyword(&keyword), |s| s.renderer_kind()),
        container_symbol,
        stereotype,
        color,
        declared_container: container_symbol.is_some(),
    })
}

fn parse_deployment_node_command(line: &str) -> Option<DeploymentNodeCommand> {
    // Java `CommandPackageWithUSymbol#getRegexConcat` accepts broad quark
    // codes for braced containers and anchors the complete command. The leaf
    // command has a narrower identifier grammar and is independently anchored.
    static CONTAINER_DISPLAY_CODE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+\"(?P<display>[^\"]+)\"(?:\s+<<(?P<pre_stereotype>[^>]+)>>)?\s+(?i:as)\s+(?P<code>[^#\s{}\"]+)(?:\s+\$[^\s{}]+)*(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+\$[^\s{}]+)*(?:\s+\[\[[^\r\n]*\]\])?(?:\s+#(?P<color>[^\s{}]+))?\s*\{\s*$"#,
        )
        .unwrap()
    });
    static CONTAINER_CODE_DISPLAY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<code>[^#\s{}\"]+)(?:\s+<<(?P<pre_stereotype>[^>]+)>>)?\s+(?i:as)\s+\"(?P<display>[^\"]+)\"(?:\s+\$[^\s{}]+)*(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+\$[^\s{}]+)*(?:\s+\[\[[^\r\n]*\]\])?(?:\s+#(?P<color>[^\s{}]+))?\s*\{\s*$"#,
        )
        .unwrap()
    });
    static CONTAINER_DISPLAY_CODE_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<display>[^#\s{}\"]+)(?:\s+<<(?P<pre_stereotype>[^>]+)>>)?\s+(?i:as)\s+(?P<code>[^#\s{}\"]+)(?:\s+\$[^\s{}]+)*(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+\$[^\s{}]+)*(?:\s+\[\[[^\r\n]*\]\])?(?:\s+#(?P<color>[^\s{}]+))?\s*\{\s*$"#,
        )
        .unwrap()
    });
    static CONTAINER_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<code>\"[^\"]+\")(?:\s+\$[^\s{}]+)*(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+\$[^\s{}]+)*(?:\s+\[\[[^\r\n]*\]\])?(?:\s+#(?P<color>[^\s{}]+))?\s*\{\s*$"#,
        )
        .unwrap()
    });
    static CONTAINER_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<code>[^#\s{}\"]+)(?:\s+\$[^\s{}]+)*(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+\$[^\s{}]+)*(?:\s+\[\[[^\r\n]*\]\])?(?:\s+#(?P<color>[^\s{}]+))?\s*\{\s*$"#,
        )
        .unwrap()
    });
    static LEAF_CODE_DISPLAY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<code>[\p{L}\p{N}_.]+)\s+(?i:as)\s+\"(?P<display>[^\"]+)\"(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+#(?P<color>\w+))?\s*$"#,
        )
        .unwrap()
    });
    static LEAF_DISPLAY_CODE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+\"(?P<display>[^\"]+)\"\s+(?i:as)\s+(?P<code>[\p{L}\p{N}_.]+)(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+#(?P<color>\w+))?\s*$"#,
        )
        .unwrap()
    });
    static LEAF_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<code>\"[^\"]+\")(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+#(?P<color>\w+))?\s*$"#,
        )
        .unwrap()
    });
    static LEAF_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?P<keyword>\w+)\s+(?P<code>[\p{L}\p{N}_][\p{L}\p{N}_.]*)(?:\s+<<(?P<stereotype>[^>]+)>>)?(?:\s+#(?P<color>\w+))?\s*$"#,
        )
        .unwrap()
    });

    if line.trim_end().ends_with('{') {
        let keyword = line.split_whitespace().next()?;
        let symbol = DeploymentContainerSymbol::from_command_keyword(keyword)?;
        for pattern in [
            &*CONTAINER_DISPLAY_CODE,
            &*CONTAINER_CODE_DISPLAY,
            &*CONTAINER_DISPLAY_CODE_BARE,
            &*CONTAINER_QUOTED,
            &*CONTAINER_BARE,
        ] {
            if let Some(command) = pattern
                .captures(line)
                .and_then(|captures| node_command_from_captures(captures, Some(symbol)))
            {
                return Some(command);
            }
        }
        return None;
    }

    for pattern in [
        &*LEAF_CODE_DISPLAY,
        &*LEAF_DISPLAY_CODE,
        &*LEAF_QUOTED,
        &*LEAF_BARE,
    ] {
        if let Some(command) = pattern
            .captures(line)
            .and_then(|captures| node_command_from_captures(captures, None))
        {
            return Some(command);
        }
    }
    None
}

fn deployment_link_note_text(lines: &[String]) -> String {
    // Java `CommandFactoryNoteOnLink.createMultiLine` calls
    // `BlocLines.removeEmptyColumns`: remove only the leading columns shared
    // by every nonblank body line, preserving relative and trailing space.
    let common_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.bytes()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count()
        })
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| line.get(common_indent..).unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_deployment(lines: &[String]) -> Result<DeploymentDiagram, ParseError> {
    let mut nodes: Vec<DeploymentNode> = Vec::new();
    let mut connections: Vec<DeploymentConnection> = Vec::new();
    let mut notes = Vec::new();
    let mut meta = DiagramMeta::default();
    let mut direction = DeploymentLayoutDirection::TopToBottom;
    let mut next_quark_order = 0usize;

    // Stack of node IDs for tracking nesting depth.
    let mut stack: Vec<String> = Vec::new();

    // Multiline note accumulator.
    let mut note_accum: Option<NoteAccum> = None;
    let mut link_note_accum: Option<LinkNoteAccum> = None;

    // Legend accumulator.
    let mut in_legend = false;
    let mut legend_lines: Vec<String> = Vec::new();

    // Skinparam block accumulator. When inside `skinparam node { ... }`,
    // nested `Key Value` lines are flattened to `nodeKey` = `Value`.
    let mut skinparam_block_prefix: Option<String> = None;

    // [Label] [as id] [<<stereo>>] [#color]  — bracket component notation
    static RE_NODE_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\[([^\]]+)\](?:\s+(?i:as)\s+(\w+))?(?:\s+<<([^>]+)>>)?(?:\s+#(\w+))?\s*$"#)
            .unwrap()
    });

    // note direction of target : text  (inline attached note)
    static RE_NOTE_ATTACHED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?i)^note\s+(top|bottom|left|right)\s+of\s+("?[^":]+?"?)\s*(#\S+)?\s*:\s*(.+)$"#,
        )
        .unwrap()
    });

    // note direction of target  (multiline attached note — no colon)
    static RE_NOTE_ATTACHED_MULTI: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)^note\s+(top|bottom|left|right)\s+of\s+("?[^"]+?"?)\s*(#\S+)?\s*$"#)
            .unwrap()
    });

    for (line_idx, line) in lines.iter().enumerate() {
        let (current_line, trimmed) = super::source_line_and_trimmed(line_idx + 1, line);

        // Multiline link-note body. Java keeps this as one command and assigns
        // the completed value to the existing Link without allocating a quark.
        // Consume it before the empty-line fast path so body whitespace remains
        // available to `BlocLines.removeEmptyColumns` semantics.
        if link_note_accum.is_some() {
            if super::is_ordinary_note_terminator(trimmed) {
                let accum = link_note_accum.take().unwrap();
                if accum.lines.is_empty() {
                    return Err(ParseError {
                        line: accum.command_line,
                        message: "no note defined".to_string(),
                    });
                }
                connections[accum.connection_index].note = Some(DeploymentLinkNote {
                    text: deployment_link_note_text(&accum.lines),
                    colors: accum.colors,
                    position: accum.position,
                });
            } else {
                link_note_accum
                    .as_mut()
                    .unwrap()
                    .lines
                    .push(super::source_text(line).to_string());
            }
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Inside a `skinparam <prefix> { ... }` block: flatten nested
        // `Key Value` entries to `<prefix>Key` until the closing `}`.
        if let Some(prefix) = &skinparam_block_prefix {
            if trimmed == "}" {
                skinparam_block_prefix = None;
            } else {
                let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                if parts.len() == 2 {
                    meta.skinparams.push(crate::diagram::SkinParam {
                        key: format!("{prefix}{}", parts[0]),
                        value: parts[1].trim().to_string(),
                    });
                }
            }
            continue;
        }

        // Multiline note body.
        if note_accum.is_some() {
            if super::is_ordinary_note_terminator(trimmed) {
                let accum = note_accum.take().unwrap();
                let text = accum.lines.join("\n");
                notes.push(DeploymentNote {
                    id: accum.id,
                    target: accum.target,
                    text,
                    color: accum.color,
                    tags: accum.tags,
                    stereotype: accum.stereotype,
                    owner: accum.owner,
                    quark_order: accum.quark_order,
                    position: accum.position,
                    source_line: accum.source_line,
                });
            } else {
                note_accum.as_mut().unwrap().lines.push(trimmed.to_string());
            }
            continue;
        }

        // Legend block.
        if trimmed == "legend" || trimmed.starts_with("legend ") {
            in_legend = true;
            legend_lines.clear();
            meta.legend_line = Some(current_line);
            for token in trimmed.split_whitespace().skip(1) {
                match token.to_ascii_lowercase().as_str() {
                    "left" => meta.legend_horizontal_alignment = LegendHorizontalAlignment::Left,
                    "center" => {
                        meta.legend_horizontal_alignment = LegendHorizontalAlignment::Center;
                    }
                    "right" => meta.legend_horizontal_alignment = LegendHorizontalAlignment::Right,
                    "top" => meta.legend_vertical_alignment = LegendVerticalAlignment::Top,
                    "bottom" => meta.legend_vertical_alignment = LegendVerticalAlignment::Bottom,
                    _ => {}
                }
            }
            continue;
        }
        if trimmed == "endlegend" {
            in_legend = false;
            meta.legend = Some(legend_lines.join("\n"));
            continue;
        }
        if in_legend {
            legend_lines.push(trimmed.to_string());
            continue;
        }

        // Title directive.
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
            continue;
        }
        // Header/footer.
        if let Some(rest) = trimmed.strip_prefix("header ") {
            meta.header = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("footer ") {
            meta.footer = Some(rest.trim().to_string());
            continue;
        }
        // Skinparam.
        if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            let rest = rest.trim();
            // Block form: `skinparam node {` opens a nested block whose
            // `Key Value` entries are flattened to `<prefix>Key`.
            if let Some(prefix) = rest.strip_suffix('{') {
                skinparam_block_prefix = Some(prefix.trim().to_string());
                continue;
            }
            let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
            if parts.len() == 2 {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: parts[0].to_string(),
                    value: parts[1].trim().to_string(),
                });
            }
            continue;
        }
        if trimmed == "left to right direction" {
            direction = DeploymentLayoutDirection::LeftToRight;
            continue;
        }
        if trimmed == "top to bottom direction" {
            direction = DeploymentLayoutDirection::TopToBottom;
            continue;
        }

        // Skip other decoration lines.
        if trimmed.starts_with("hide ") || trimmed.starts_with("show ") {
            continue;
        }

        // Handle closing brace — pop the current container from the stack.
        if trimmed == "}" {
            stack.pop();
            continue;
        }

        // Bracket notation: [Label] [as id] [<<stereo>>]  — component shorthand.
        if trimmed.starts_with('[')
            && let Some(caps) = RE_NODE_BRACKET.captures(trimmed)
        {
            let label = caps[1].trim().to_string();
            let id = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| label_to_id(&label));
            let stereotype = caps.get(3).map(|m| m.as_str().trim().to_string());
            let color = caps.get(4).map(|m| m.as_str().to_string());
            let created = push_node(
                &mut nodes,
                &mut next_quark_order,
                id.clone(),
                label,
                DeploymentNodeKind::Component,
                None,
                stereotype,
                color,
                false,
                current_line,
            );
            if created && let Some(parent_id) = stack.last().cloned() {
                add_child(&mut nodes, &parent_id, &id);
            }
            continue;
        }

        if let Some(command) = parse_deployment_link_note_command(trimmed) {
            let command = command.map_err(|message| ParseError {
                line: current_line,
                message: message.to_string(),
            })?;
            let Some(connection_index) = last_note_eligible_connection_index(&connections, &notes)
            else {
                return Err(ParseError {
                    line: current_line,
                    message: "no link defined for note on link".to_string(),
                });
            };
            match command {
                DeploymentLinkNoteCommand::Inline(note) => {
                    // Java `Link.addNote` is replacement, not accumulation.
                    connections[connection_index].note = Some(note);
                }
                DeploymentLinkNoteCommand::Multiline { colors, position } => {
                    link_note_accum = Some(LinkNoteAccum {
                        connection_index,
                        colors,
                        position,
                        command_line: current_line,
                        lines: Vec::new(),
                    });
                }
            }
            continue;
        }

        // Floating note: note "text" as ID
        if let Some(command) = super::parse_named_note_inline(trimmed) {
            let quark_order = next_quark_order;
            next_quark_order += 1;
            notes.push(DeploymentNote {
                id: Some(command.code),
                target: None,
                text: command.display.expect("inline named note has display text"),
                color: command.color,
                tags: command.tags,
                stereotype: command.stereotype,
                owner: stack.last().cloned(),
                quark_order,
                position: DeploymentNotePosition::Right,
                source_line: current_line,
            });
            continue;
        }
        if super::looks_like_named_note_inline_command(trimmed) {
            return Err(ParseError {
                line: current_line,
                message: "invalid named note command".to_string(),
            });
        }

        if let Some(command) = super::parse_named_note_multiline(trimmed) {
            let quark_order = next_quark_order;
            next_quark_order += 1;
            note_accum = Some(NoteAccum {
                id: Some(command.code),
                target: None,
                color: command.color,
                tags: command.tags,
                stereotype: command.stereotype,
                owner: stack.last().cloned(),
                quark_order,
                position: DeploymentNotePosition::Right,
                // Preprocessing strips the `@startuml` line before the first
                // content record; the multiline command's Java location is
                // the adjacent original source line.
                source_line: current_line.max(1) + 1,
                lines: Vec::new(),
            });
            continue;
        }
        if super::looks_like_named_note_multiline_command(trimmed) {
            return Err(ParseError {
                line: current_line,
                message: "invalid named note command".to_string(),
            });
        }

        // Attached note: note direction of target : text  (inline)
        if let Some(caps) = RE_NOTE_ATTACHED.captures(trimmed) {
            let position = deployment_note_position(&caps[1].to_ascii_lowercase());
            let target_raw = caps[2].trim().trim_matches('"').to_string();
            let target = resolve_id(&nodes, &target_raw);
            let text = caps[4].trim().to_string();
            let quark_order = next_quark_order;
            next_quark_order += 1;
            notes.push(DeploymentNote {
                id: None,
                target: Some(target),
                text,
                color: caps.get(3).map(|value| value.as_str().to_string()),
                tags: Vec::new(),
                stereotype: None,
                owner: stack.last().cloned(),
                quark_order,
                position,
                source_line: current_line,
            });
            continue;
        }

        // Multiline attached note: note direction of target  (no colon)
        if let Some(caps) = RE_NOTE_ATTACHED_MULTI.captures(trimmed) {
            let position = deployment_note_position(&caps[1].to_ascii_lowercase());
            let target_raw = caps[2].trim().trim_matches('"').to_string();
            let target = resolve_id(&nodes, &target_raw);
            let quark_order = next_quark_order;
            next_quark_order += 1;
            note_accum = Some(NoteAccum {
                id: None,
                target: Some(target),
                color: caps.get(3).map(|value| value.as_str().to_string()),
                tags: Vec::new(),
                stereotype: None,
                owner: stack.last().cloned(),
                quark_order,
                position,
                source_line: current_line + 1,
                lines: Vec::new(),
            });
            continue;
        }

        // Check if the first word is a deployment keyword.
        let first_word = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        if is_deployment_keyword(&first_word) {
            // Check if this is a connection line (keyword "label" --> ...)
            // before treating it as a pure node declaration.
            if let Some(parsed) = try_parse_connection(trimmed) {
                let ParsedDeploymentConnection {
                    raw_from,
                    raw_to,
                    label,
                    tail_label,
                    head_label,
                    arrow_at_start,
                    arrow_at_end,
                    direction,
                    style,
                    hidden,
                    length,
                } = parsed;
                let from = resolve_connection_endpoint(&raw_from);
                let to = resolve_connection_endpoint(&raw_to);

                for (id, lbl) in [(&from, &raw_from), (&to, &raw_to)] {
                    if !deployment_identity_exists(&nodes, &notes, id) {
                        let quark_order = next_quark_order;
                        next_quark_order += 1;
                        nodes.push(DeploymentNode {
                            id: id.clone(),
                            label: lbl.clone(),
                            kind: DeploymentNodeKind::Default,
                            container_symbol: None,
                            stereotype: None,
                            color: None,
                            declared_container: false,
                            children: Vec::new(),
                            source_line: current_line,
                            quark_order,
                        });
                    }
                }
                connections.push(DeploymentConnection {
                    from,
                    to,
                    label,
                    tail_label,
                    head_label,
                    arrow_at_start,
                    arrow_at_end,
                    direction,
                    style,
                    note: None,
                    hidden,
                    length,
                    source_line: current_line,
                });
                continue;
            }

            if let Some(mut command) = parse_deployment_node_command(trimmed) {
                if command.id.is_empty() {
                    command.id = format!("##{}", next_quark_order + 1);
                }
                if command.label.is_empty() {
                    command.label = command.id.clone();
                }
                let created = push_node(
                    &mut nodes,
                    &mut next_quark_order,
                    command.id.clone(),
                    command.label,
                    command.kind,
                    command.container_symbol,
                    command.stereotype,
                    command.color,
                    command.declared_container,
                    current_line,
                );
                if created && let Some(parent_id) = stack.last().cloned() {
                    add_child(&mut nodes, &parent_id, &command.id);
                }
                if command.declared_container {
                    stack.push(command.id);
                }
                continue;
            }

            return Err(ParseError {
                line: current_line,
                message: "invalid deployment element declaration".to_string(),
            });
        }

        // Connection line (bare identifiers or quoted labels).
        if let Some(parsed) = try_parse_connection(trimmed) {
            let ParsedDeploymentConnection {
                raw_from,
                raw_to,
                label,
                tail_label,
                head_label,
                arrow_at_start,
                arrow_at_end,
                direction,
                style,
                hidden,
                length,
            } = parsed;
            let from = resolve_connection_endpoint(&raw_from);
            let to = resolve_connection_endpoint(&raw_to);

            // Auto-create nodes for any unknown IDs in connections.
            for (id, lbl) in [(&from, &raw_from), (&to, &raw_to)] {
                if !deployment_identity_exists(&nodes, &notes, id) {
                    let quark_order = next_quark_order;
                    next_quark_order += 1;
                    nodes.push(DeploymentNode {
                        id: id.clone(),
                        label: lbl.clone(),
                        // `CommandLinkElement.getDummy` creates an unknown
                        // plain endpoint as `LeafType.STILL_UNKNOWN`, not as
                        // an explicitly declared `node` symbol.
                        kind: DeploymentNodeKind::Default,
                        container_symbol: None,
                        stereotype: None,
                        color: None,
                        declared_container: false,
                        children: Vec::new(),
                        source_line: current_line,
                        quark_order,
                    });
                }
            }

            connections.push(DeploymentConnection {
                from,
                to,
                label,
                tail_label,
                head_label,
                arrow_at_start,
                arrow_at_end,
                direction,
                style,
                note: None,
                hidden,
                length,
                source_line: current_line,
            });
        }
    }

    if let Some(accum) = link_note_accum {
        return Err(ParseError {
            line: accum.command_line,
            message: "unterminated note on link".to_string(),
        });
    }

    Ok(DeploymentDiagram {
        meta,
        direction,
        nodes,
        connections,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> DeploymentDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_deployment(&lines).unwrap()
    }

    #[test]
    fn preserves_global_layout_direction_and_allows_reset() {
        let left_to_right = parse(
            "left to right direction\nnode RenamedOne\nnode RenamedTwo\nRenamedOne --> RenamedTwo",
        );
        assert_eq!(
            left_to_right.direction,
            DeploymentLayoutDirection::LeftToRight
        );

        let reset = parse(
            "left to right direction\ntop to bottom direction\nnode RenamedOne\nnode RenamedTwo",
        );
        assert_eq!(reset.direction, DeploymentLayoutDirection::TopToBottom);
    }

    #[test]
    fn basic_deployment() {
        let d = parse("node WebServer {\n  artifact app\n}\ndatabase DB\nWebServer --> DB");
        assert!(d.nodes.iter().any(|n| n.id == "WebServer"));
        assert!(d.nodes.iter().any(|n| n.id == "DB"));
        assert_eq!(d.connections.len(), 1);
    }

    #[test]
    fn every_java_usymbol_container_enters_and_owns_its_children() {
        let mut source = String::new();
        for (index, (keyword, _)) in DeploymentContainerSymbol::COMMAND_SYMBOLS
            .iter()
            .enumerate()
        {
            source.push_str(&format!(
                "{keyword} Container{index} {{\nartifact Child{index}\n}}\n"
            ));
        }

        let diagram = parse(&source);
        for (index, (_, expected_symbol)) in DeploymentContainerSymbol::COMMAND_SYMBOLS
            .iter()
            .enumerate()
        {
            let container = diagram
                .nodes
                .iter()
                .find(|node| node.id == format!("Container{index}"))
                .unwrap_or_else(|| panic!("missing container {index}"));
            assert_eq!(container.container_symbol, Some(*expected_symbol));
            assert_eq!(container.children, [format!("Child{index}")]);
        }
    }

    #[test]
    fn action_process_and_hexagon_preserve_symbol_identity_and_nested_ownership() {
        let diagram = parse(
            "PrOcEsS \"Telemetry Mesh\" as proc_v7 {\n\
               HEXAGON zone-east as \"Zone East\" {\n\
                 action \"Retry Stage\" as retry_11 {\n\
                   artifact Worker_17\n\
                 }\n\
               }\n\
             }",
        );

        let process = diagram
            .nodes
            .iter()
            .find(|node| node.id == "proc_v7")
            .unwrap();
        assert_eq!(process.label, "Telemetry Mesh");
        assert_eq!(
            process.container_symbol,
            Some(DeploymentContainerSymbol::Process)
        );
        assert_eq!(process.children, ["zone-east"]);

        let hexagon = diagram
            .nodes
            .iter()
            .find(|node| node.id == "zone-east")
            .unwrap();
        assert_eq!(hexagon.label, "Zone East");
        assert_eq!(
            hexagon.container_symbol,
            Some(DeploymentContainerSymbol::Hexagon)
        );
        assert_eq!(hexagon.children, ["retry_11"]);

        let action = diagram
            .nodes
            .iter()
            .find(|node| node.id == "retry_11")
            .unwrap();
        assert_eq!(action.label, "Retry Stage");
        assert_eq!(
            action.container_symbol,
            Some(DeploymentContainerSymbol::Action)
        );
        assert_eq!(action.children, ["Worker_17"]);
    }

    #[test]
    fn malformed_usymbol_container_commands_fail_closed_before_later_lines() {
        for (invalid, expected_line) in [
            ("process \"Broken\" as {", 1),
            ("node ValidRoot {\nhexagon region extra unexpected {", 2),
            ("action retry_19 { trailing\nartifact Later_23", 1),
            ("cloud region/new unexpected {\ndatabase Later_29", 1),
        ] {
            let lines = invalid.lines().map(str::to_string).collect::<Vec<_>>();
            let error = parse_deployment(&lines).unwrap_err();
            assert_eq!(error.line, expected_line, "{invalid}");
            assert_eq!(error.message, "invalid deployment element declaration");
        }
    }

    #[test]
    fn usymbol_keywords_at_relationship_start_remain_endpoints() {
        let diagram = parse(
            "node Target_31\n\
             process --> Target_31 : schedules\n\
             action \"many_37\" --> Target_31 : retries\n\
             hexagon ..> Target_31",
        );

        assert_eq!(diagram.connections.len(), 3);
        assert_eq!(diagram.connections[0].from, "process");
        assert_eq!(diagram.connections[1].from, "action");
        assert_eq!(
            diagram.connections[1].tail_label.as_deref(),
            Some("many_37")
        );
        assert_eq!(diagram.connections[2].from, "hexagon");
    }

    #[test]
    fn element_commands_are_case_insensitive_but_preserve_identity() {
        let d = parse(
            "NoDe \"Runtime\" AS Runtime {\n\
               DATABASE AuditStore\n\
               QuEuE \"Retry Work\" As RetryQueue\n\
               BoUnDaRy AccessEdge\n\
             }",
        );
        let expected = [
            ("Runtime", "Runtime", DeploymentNodeKind::Node),
            ("AuditStore", "AuditStore", DeploymentNodeKind::Database),
            ("RetryQueue", "Retry Work", DeploymentNodeKind::Queue),
            ("AccessEdge", "AccessEdge", DeploymentNodeKind::Boundary),
        ];
        for (id, label, kind) in expected {
            let node = d
                .nodes
                .iter()
                .find(|node| node.id == id)
                .unwrap_or_else(|| panic!("missing {id}; parsed nodes: {:?}", d.nodes));
            assert_eq!(node.label, label);
            assert_eq!(node.kind, kind);
        }
    }

    #[test]
    fn cloud_and_storage() {
        let d = parse("cloud Internet\nstorage S3\nInternet --> S3");
        assert_eq!(
            d.nodes.iter().find(|n| n.id == "Internet").unwrap().kind,
            DeploymentNodeKind::Cloud
        );
        assert_eq!(
            d.nodes.iter().find(|n| n.id == "S3").unwrap().kind,
            DeploymentNodeKind::Storage
        );
    }

    #[test]
    fn bare_with_label() {
        let d = parse(r#"node n1 as "Primary Web Server""#);
        let n = d.nodes.iter().find(|n| n.id == "n1").unwrap();
        assert_eq!(n.label, "Primary Web Server");
    }

    #[test]
    fn quoted_label_no_id() {
        let d = parse(r#"artifact "application.deb""#);
        let n = &d.nodes[0];
        assert_eq!(n.label, "application.deb");
        assert_eq!(n.id, "application.deb");
    }

    #[test]
    fn quoted_connection() {
        let d = parse("artifact \"app.deb\"\nnode Server\n\"app.deb\" --> Server");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.connections[0].from, "app.deb");
        assert_eq!(d.connections[0].to, "Server");
    }

    #[test]
    fn quoted_connection_with_hyphens() {
        let d = parse(
            "node \"server-01.example.com\"\nnode \"db_primary\"\n\"server-01.example.com\" --> \"db_primary\" : port 5432",
        );
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("port 5432"));
    }

    #[test]
    fn keyword_prefixed_connection() {
        let d =
            parse("artifact \"app.war\"\nnode Server\nartifact \"app.war\" --> Server : deploy");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("deploy"));
        assert_eq!(d.connections[0].tail_label.as_deref(), Some("app.war"));
    }

    #[test]
    fn preserves_endpoint_quantifiers() {
        let d = parse("node A\nnode B\nA \"one\" --> \"many\" B : owns");
        let connection = &d.connections[0];
        assert_eq!(connection.from, "A");
        assert_eq!(connection.to, "B");
        assert_eq!(connection.tail_label.as_deref(), Some("one"));
        assert_eq!(connection.head_label.as_deref(), Some("many"));
        assert_eq!(connection.label.as_deref(), Some("owns"));
    }

    #[test]
    fn preserves_explicit_link_directions() {
        let diagram = parse("node A\nnode B\nA -left-> B\nA -right-> B\nA -up-> B\nA -down-> B");

        assert_eq!(
            diagram
                .connections
                .iter()
                .map(|connection| connection.direction)
                .collect::<Vec<_>>(),
            vec![
                Some(DeploymentLinkDirection::Left),
                Some(DeploymentLinkDirection::Right),
                Some(DeploymentLinkDirection::Up),
                Some(DeploymentLinkDirection::Down),
            ]
        );
    }

    #[test]
    fn preserves_link_shaft_styles() {
        let diagram = parse("node A\nnode B\nA --> B\nA -right.> B\nA ~~> B\nA ==> B\nA ....> B");

        assert_eq!(
            diagram
                .connections
                .iter()
                .map(|connection| connection.style)
                .collect::<Vec<_>>(),
            vec![
                DeploymentLinkStyle::Solid,
                DeploymentLinkStyle::Dashed,
                DeploymentLinkStyle::Dotted,
                DeploymentLinkStyle::Bold,
                DeploymentLinkStyle::Dashed,
            ]
        );
        assert_eq!(diagram.connections.last().unwrap().length, 4);
    }

    #[test]
    fn preserves_link_decorations_at_both_ends() {
        let diagram = parse("node A\nnode B\nA -- B\nA --> B\nA <-- B\nA <-> B");
        let ends: Vec<_> = diagram
            .connections
            .iter()
            .map(|connection| (connection.arrow_at_start, connection.arrow_at_end))
            .collect();

        assert_eq!(
            ends,
            vec![(false, false), (false, true), (true, false), (true, true)]
        );
    }

    #[test]
    fn hidden_link_is_retained_as_a_paint_hidden_source_order_event() {
        let d = parse(
            "note \"Visible plus hidden\" as HiddenControlMemo\n\
             node VisiblePeer\n\
             node HiddenPeer\n\
             HiddenControlMemo --> VisiblePeer\n\
             HiddenControlMemo -[hidden]- HiddenPeer\n\
             artifact AfterHidden",
        );

        assert_eq!(d.connections.len(), 2);
        assert!(!d.connections[0].hidden);
        assert!(d.connections[1].hidden);
        assert_eq!(d.connections[1].from, "HiddenControlMemo");
        assert_eq!(d.connections[1].to, "HiddenPeer");
        assert_eq!(d.connections[1].source_line, 5);
        assert_eq!(d.connections[1].length, 2);
        assert_eq!(d.nodes.last().unwrap().id, "AfterHidden");
    }

    #[test]
    fn link_style_slots_follow_command_link_element_grammar() {
        let d = parse(
            "node A\n\
             node B\n\
             A -[hidden,dashed;#red,bold]down[#blue]-> B",
        );
        assert_eq!(d.connections.len(), 1);
        assert!(d.connections[0].hidden);
        assert_eq!(
            d.connections[0].direction,
            Some(DeploymentLinkDirection::Down)
        );

        let unknown = parse("node A\nnode B\nA -[invented]- B");
        assert!(unknown.connections.is_empty());

        let second_slot_multiple = parse("node A\nnode B\nA -down[hidden;bold]-> B");
        assert!(second_slot_multiple.connections.is_empty());
    }

    #[test]
    fn note_attached() {
        let d = parse("node AppServer\nnote right of AppServer : 16 cores");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "16 cores");
        assert_eq!(d.notes[0].target.as_deref(), Some("AppServer"));
        assert_eq!(d.notes[0].position, DeploymentNotePosition::Right);
        assert_eq!(d.notes[0].source_line, 2);
    }

    #[test]
    fn named_note_relation_reuses_the_note_without_turning_it_into_an_attachment() {
        let d = parse("node Server\nnote \"Primary server\" as N1\nN1 .. Server");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Primary server");
        assert_eq!(d.notes[0].target, None);
        assert_eq!(d.notes[0].position, DeploymentNotePosition::Right);
        assert_eq!(d.notes[0].source_line, 2);
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].id, "Server");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].from, "N1");
        assert_eq!(d.connections[0].to, "Server");
        assert_eq!(d.connections[0].style, DeploymentLinkStyle::Dashed);
        assert_eq!(d.connections[0].source_line, 3);
    }

    #[test]
    fn quoted_relation_code_does_not_alias_an_existing_display_label() {
        let d = parse(
            "node \"Service Label\" as ServiceAlias\n\
             note \"Alias probe\" as AliasMemo\n\
             AliasMemo -right-> \"Service Label\" : quoted target",
        );

        assert_eq!(d.nodes.len(), 2);
        assert!(d.nodes.iter().any(|node| node.id == "ServiceAlias"));
        let dummy = d
            .nodes
            .iter()
            .find(|node| node.id == "Service Label")
            .unwrap();
        assert_eq!(dummy.label, "Service Label");
        assert_eq!(dummy.kind, DeploymentNodeKind::Default);
        assert_eq!(d.connections[0].from, "AliasMemo");
        assert_eq!(d.connections[0].to, "Service Label");
    }

    #[test]
    fn relation_code_reuses_alias_but_not_single_token_display_text() {
        let d = parse(
            "node \"DisplayToken\" as DeclaredAlias\n\
             note \"memo\" as Memo\n\
             Memo --> DeclaredAlias\n\
             Memo --> DisplayToken",
        );

        assert_eq!(d.nodes.len(), 2);
        assert!(d.nodes.iter().any(|node| node.id == "DeclaredAlias"));
        assert!(d.nodes.iter().any(|node| node.id == "DisplayToken"));
        assert_eq!(d.connections[0].to, "DeclaredAlias");
        assert_eq!(d.connections[1].to, "DisplayToken");
    }

    #[test]
    fn repeated_quoted_relation_code_reuses_one_exact_dummy_identity() {
        let d = parse(
            "note \"memo\" as Memo\n\
             node FirstPeer\n\
             node SecondPeer\n\
             Memo --> \"Métrique.v2 Label\"\n\
             SecondPeer --> \"Métrique.v2 Label\"",
        );

        assert_eq!(
            d.nodes
                .iter()
                .filter(|node| node.id == "Métrique.v2 Label")
                .count(),
            1
        );
        assert_eq!(d.connections[0].to, "Métrique.v2 Label");
        assert_eq!(d.connections[1].to, "Métrique.v2 Label");
    }

    #[test]
    fn multiline_note_preserves_side_and_first_body_line() {
        let d = parse("node Server\nnote left of Server\n  first\n  second\nend note");
        assert_eq!(d.notes[0].text, "first\nsecond");
        assert_eq!(d.notes[0].position, DeploymentNotePosition::Left);
        assert_eq!(d.notes[0].source_line, 3);
    }

    #[test]
    fn floating_multiline_note_owns_payload_before_later_nodes() {
        let d = parse(
            "NoTe as DispatchPayload #MistyRose\n\
             control body remains note text\n\
             EnD NoTe\n\
             node RuntimeNode",
        );
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].id.as_deref(), Some("DispatchPayload"));
        assert_eq!(d.notes[0].text, "control body remains note text");
        assert_eq!(d.notes[0].color.as_deref(), Some("#MistyRose"));
        assert_eq!(d.notes[0].source_line, 2);
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].id, "RuntimeNode");
    }

    #[test]
    fn explicit_note_colors_survive_every_deployment_command_form() {
        let d = parse(
            "node Server\n\
             note \"inline floating\" as Inline.Float $audit <<InlineLedger>> #AliceBlue\n\
             note as Multi.Float $retained <<MetricLedger>> #MistyRose\n\
             multiline floating\n\
             endnote\n\
             note right of Server #LightGreen : inline attached\n\
             note left of Server #Wheat\n\
             multiline attached\n\
             end note",
        );

        assert_eq!(d.notes.len(), 4);
        assert_eq!(d.notes[0].color.as_deref(), Some("#AliceBlue"));
        assert_eq!(d.notes[1].color.as_deref(), Some("#MistyRose"));
        assert_eq!(d.notes[2].color.as_deref(), Some("#LightGreen"));
        assert_eq!(d.notes[3].color.as_deref(), Some("#Wheat"));
        assert_eq!(d.notes[0].id.as_deref(), Some("Inline.Float"));
        assert_eq!(d.notes[1].id.as_deref(), Some("Multi.Float"));
        assert_eq!(d.notes[0].tags, ["audit"]);
        assert_eq!(d.notes[1].tags, ["retained"]);
        assert_eq!(d.notes[0].stereotype.as_deref(), Some("InlineLedger"));
        assert_eq!(d.notes[1].stereotype.as_deref(), Some("MetricLedger"));
    }

    #[test]
    fn invalid_named_note_commands_are_terminal() {
        for source in [
            "node Anchor\nnote \"bad code\" as Bad-Deployment $audit\nBad-Deployment --> Anchor",
            "node Anchor\nnote \"bad order\" as Bad.Deployment #MistyRose <<WrongOrder>>\nBad.Deployment --> Anchor",
            "node Anchor\nnote \"bad color\" as BadColor #R\nBadColor --> Anchor",
            "node Anchor\nnote \"left\u{e121}right\" as Embedded.Quote\nEmbedded.Quote --> Anchor",
            "node Anchor\nnote as BadCompositeValue #back:LightBlue;line.dashed:NoSuchColor\npayload\nendnote\nBadCompositeValue --> Anchor",
        ] {
            let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
            let Err(error) = parse_deployment(&lines) else {
                panic!("invalid named note parsed: {source}");
            };
            assert_eq!(error.line, 2, "{source}");
            assert_eq!(error.message, "invalid named note command");
        }
    }

    #[test]
    fn pattern2_quoted_named_notes_keep_nested_identity_for_later_relations() {
        for (opening, closing) in [
            ('\u{201c}', '\u{201d}'),
            ('\u{201d}', '\u{201c}'),
            ('"', '\u{201d}'),
            ('\u{e121}', '\u{e121}'),
        ] {
            let source = format!(
                "node Outer {{\nnode Anchor\nnote {opening}quoted payload{closing} as Ledger.Note $audit <<Trace>> #MistyRose\nLedger.Note --> Anchor\n}}"
            );
            let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
            let diagram =
                parse_deployment(&lines).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
            assert_eq!(diagram.notes.len(), 1, "{source:?}");
            assert_eq!(diagram.notes[0].id.as_deref(), Some("Ledger.Note"));
            assert_eq!(diagram.notes[0].owner.as_deref(), Some("Outer"));
            assert_eq!(diagram.connections.len(), 1);
            assert_eq!(diagram.connections[0].from, "Ledger.Note");
            assert!(!diagram.nodes.iter().any(|item| item.id == "Ledger.Note"));
        }
    }

    #[test]
    fn dotted_inline_named_note_is_reused_by_a_relation() {
        let d = parse(
            "note \"Dotted code note\" as memo.v1\n\
             node \"Service A\" as ServiceA\n\
             memo.v1 ..> ServiceA : audit route",
        );

        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].id.as_deref(), Some("memo.v1"));
        assert!(!d.nodes.iter().any(|node| node.id == "memo_v1"));
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].from, "memo.v1");
        assert_eq!(d.connections[0].to, "ServiceA");
    }

    #[test]
    fn description_leaves_retain_cross_kind_quark_order_and_owner() {
        let d = parse(
            "node Outer {\n\
               note as FirstLedger\n\
                 payload\n\
               endnote\n\
               artifact LaterRuntime\n\
             }\n\
             file RootLeaf",
        );

        let outer = d.nodes.iter().find(|node| node.id == "Outer").unwrap();
        let nested = d
            .nodes
            .iter()
            .find(|node| node.id == "LaterRuntime")
            .unwrap();
        let root = d.nodes.iter().find(|node| node.id == "RootLeaf").unwrap();
        assert!(outer.quark_order < d.notes[0].quark_order);
        assert!(d.notes[0].quark_order < nested.quark_order);
        assert!(nested.quark_order < root.quark_order);
        assert_eq!(d.notes[0].owner.as_deref(), Some("Outer"));
    }

    #[test]
    fn link_note_command_matrix_is_owned_by_the_connection() {
        for (command, expected_position, expected_color, expected_text) in [
            (
                "NoTe On LiNk : default renamed memo",
                DeploymentNotePosition::Bottom,
                None,
                "default renamed memo",
            ),
            (
                "note left of link #LightBlue : left renamed memo",
                DeploymentNotePosition::Left,
                Some("#LightBlue"),
                "left renamed memo",
            ),
            (
                "note right on link : right renamed memo",
                DeploymentNotePosition::Right,
                None,
                "right renamed memo",
            ),
            (
                "note top of link : top renamed memo",
                DeploymentNotePosition::Top,
                None,
                "top renamed memo",
            ),
            (
                "note bottom on link : bottom renamed memo",
                DeploymentNotePosition::Bottom,
                None,
                "bottom renamed memo",
            ),
        ] {
            let diagram = parse(&format!(
                "node RenamedIngress71\nnode RenamedArchive73\nRenamedIngress71 --> RenamedArchive73 : renamed route\n{command}"
            ));
            assert!(diagram.notes.is_empty(), "{command}");
            assert_eq!(diagram.connections.len(), 1, "{command}");
            assert_eq!(diagram.connections[0].source_line, 3, "{command}");
            let note = diagram.connections[0]
                .note
                .as_ref()
                .unwrap_or_else(|| panic!("missing link note for {command}"));
            assert_eq!(note.position, expected_position, "{command}");
            assert_eq!(note.colors.back.as_deref(), expected_color, "{command}");
            assert_eq!(note.text, expected_text, "{command}");
        }
    }

    #[test]
    fn link_note_part2_colors_retain_independent_style_channels() {
        let diagram = parse(
            "node RenamedSourceColor\n\
             database RenamedTargetColor\n\
             RenamedSourceColor --> RenamedTargetColor\n\
             note left on link #back:FF0000;line.dashed:00FF00;text:0000FF;header:Gold;shadowing:false : renamed chromatic payload",
        );
        let note = diagram.connections[0].note.as_ref().unwrap();
        assert_eq!(note.colors.back.as_deref(), Some("FF0000"));
        assert_eq!(note.colors.line.as_deref(), Some("00FF00"));
        assert_eq!(note.colors.text.as_deref(), Some("0000FF"));
        assert_eq!(note.colors.header.as_deref(), Some("Gold"));
        assert_eq!(
            note.colors.line_style,
            Some(crate::diagram::style::PlantUmlLineStyle::Dashed)
        );
        assert_eq!(note.colors.shadowing, Some(false));

        let multiline = parse(
            "node RenamedSourceMulti\n\
             node RenamedTargetMulti\n\
             RenamedSourceMulti --> RenamedTargetMulti\n\
             note on link #Wheat;line.bold:Navy;text:Red\n\
               retained multiline body\n\
             endnote",
        );
        let note = multiline.connections[0].note.as_ref().unwrap();
        assert_eq!(note.colors.back.as_deref(), Some("#Wheat"));
        assert_eq!(note.colors.line.as_deref(), Some("Navy"));
        assert_eq!(note.colors.text.as_deref(), Some("Red"));
        assert_eq!(
            note.colors.line_style,
            Some(crate::diagram::style::PlantUmlLineStyle::Bold)
        );
    }

    #[test]
    fn link_note_color_boundary_follows_single_then_multiline_factory_order() {
        for command in [
            "note on link #back:Red : spaced body",
            "note on link #back:Red;: semicolon body",
        ] {
            let diagram = parse(&format!(
                "node BoundaryA\nnode BoundaryB\nBoundaryA --> BoundaryB\n{command}"
            ));
            let note = diagram.connections[0].note.as_ref().unwrap();
            assert_eq!(note.colors.back.as_deref(), Some("Red"), "{command}");
        }

        for command in [
            "note on link #back:Red: compact body",
            "note on link #header:Gold\nmultiline body\nendnote",
            "note on link #back:Wheat;line.dotted\nmultiline body\nendnote",
        ] {
            let lines =
                format!("node BoundaryA\nnode BoundaryB\nBoundaryA --> BoundaryB\n{command}")
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
            let error = parse_deployment(&lines).unwrap_err();
            assert_eq!(error.line, 4, "{command}");
            assert_eq!(error.message, "invalid note-on-link color", "{command}");
        }

        for command in [
            "note on link #line.dotted\nmultiline body\nendnote",
            "note on link #line.bold:Navy\nmultiline body\nendnote",
            "note on link #Wheat;line.dotted\nmultiline body\nendnote",
        ] {
            let diagram = parse(&format!(
                "node BoundaryA\nnode BoundaryB\nBoundaryA --> BoundaryB\n{command}"
            ));
            assert_eq!(
                diagram.connections[0].note.as_ref().unwrap().text,
                "multiline body",
                "{command}"
            );
        }
    }

    #[test]
    fn multiline_link_note_preserves_nested_topology_without_a_quark() {
        let diagram = parse(
            "node RenamedOuter101 {\n\
               node RenamedIngress103\n\
               node RenamedArchive107\n\
               RenamedIngress103 -right-> RenamedArchive107 : nested route\n\
               note top of link #FFCCAA\n\
                 first renamed line\n\
                 second renamed line\n\
               EnDnOtE\n\
               artifact RenamedLater109\n\
             }",
        );

        let connection = &diagram.connections[0];
        let note = connection.note.as_ref().unwrap();
        assert_eq!(note.text, "first renamed line\nsecond renamed line");
        assert_eq!(note.colors.back.as_deref(), Some("#FFCCAA"));
        assert_eq!(note.position, DeploymentNotePosition::Top);
        assert_eq!(connection.direction, Some(DeploymentLinkDirection::Right));
        assert_eq!(connection.source_line, 4);
        assert!(diagram.notes.is_empty());
        let later = diagram
            .nodes
            .iter()
            .find(|node| node.id == "RenamedLater109")
            .unwrap();
        assert_eq!(later.quark_order, 3);
        assert!(diagram.nodes[0].children.contains(&later.id));
    }

    #[test]
    fn multiline_link_note_dedents_only_the_common_body_columns() {
        let lines = [
            "node RenamedA113",
            "node RenamedB127",
            "RenamedA113 --> RenamedB127",
            "note on link",
            "  alpha  ",
            "    beta",
            "",
            "  gamma",
            "end note",
        ]
        .map(str::to_string);
        let diagram = parse_deployment(&lines).unwrap();

        assert_eq!(
            diagram.connections[0].note.as_ref().unwrap().text,
            "alpha  \n  beta\n\ngamma"
        );
    }

    #[test]
    fn repeated_link_note_replaces_and_skips_note_endpoint_links() {
        let diagram = parse(
            "node RenamedOrigin131\n\
             node RenamedTarget137\n\
             RenamedOrigin131 --> RenamedTarget137 : durable route\n\
             note on link : superseded memo\n\
             note \"floating control\" as Floating139\n\
             Floating139 --> RenamedTarget137\n\
             note left on link #PaleGreen : replacement memo",
        );

        assert_eq!(diagram.connections.len(), 2);
        let note = diagram.connections[0].note.as_ref().unwrap();
        assert_eq!(note.text, "replacement memo");
        assert_eq!(note.colors.back.as_deref(), Some("#PaleGreen"));
        assert_eq!(note.position, DeploymentNotePosition::Left);
        assert!(diagram.connections[1].note.is_none());
        assert_eq!(diagram.notes.len(), 1);
    }

    #[test]
    fn link_note_without_an_eligible_prior_link_is_terminal() {
        for (source, expected_line) in [
            ("node RenamedOnly151\nnote on link : rejected memo", 2),
            (
                "note \"floating\" as Floating157\nnode RenamedPeer163\nFloating157 --> RenamedPeer163\nnote of link : rejected note edge",
                4,
            ),
        ] {
            let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
            let error = parse_deployment(&lines).unwrap_err();
            assert_eq!(error.line, expected_line, "{source}");
            assert_eq!(error.message, "no link defined for note on link");
        }
    }

    #[test]
    fn invalid_or_unterminated_link_note_commands_are_terminal() {
        for (source, expected_message) in [
            (
                "node RenamedA167\nnode RenamedB173\nRenamedA167 --> RenamedB173\nnote on link #NotAPlantUmlColor : rejected",
                "invalid note-on-link color",
            ),
            (
                "node RenamedA167\nnode RenamedB173\nRenamedA167 --> RenamedB173\nnote on link\nunterminated body",
                "unterminated note on link",
            ),
            (
                "node RenamedA167\nnode RenamedB173\nRenamedA167 --> RenamedB173\nnote on link\nend note",
                "no note defined",
            ),
        ] {
            let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
            let error = parse_deployment(&lines).unwrap_err();
            assert_eq!(error.line, 4, "{source}");
            assert_eq!(error.message, expected_message, "{source}");
        }
    }

    #[test]
    fn header_footer() {
        let d = parse("header My Header\nfooter My Footer\nnode Server");
        assert_eq!(d.meta.header.as_deref(), Some("My Header"));
        assert_eq!(d.meta.footer.as_deref(), Some("My Footer"));
    }

    #[test]
    fn legend_preserves_alignment_and_source_line() {
        let d = parse("node Server\nlegend top right\n| Key | Value |\nendlegend");

        assert_eq!(d.meta.legend.as_deref(), Some("| Key | Value |"));
        assert_eq!(d.meta.legend_line, Some(2));
        assert_eq!(
            d.meta.legend_horizontal_alignment,
            LegendHorizontalAlignment::Right
        );
        assert_eq!(
            d.meta.legend_vertical_alignment,
            LegendVerticalAlignment::Top
        );
    }

    #[test]
    fn bracket_component_with_alias() {
        let d = parse("node Container {\n  [Frontend Module] as fe\n  [API Module] as api\n}");
        let fe = d.nodes.iter().find(|n| n.id == "fe").unwrap();
        assert_eq!(fe.label, "Frontend Module");
        assert_eq!(fe.kind, DeploymentNodeKind::Component);
        let container = d.nodes.iter().find(|n| n.id == "Container").unwrap();
        assert!(container.children.contains(&"fe".to_string()));
        assert!(container.children.contains(&"api".to_string()));
    }

    #[test]
    fn bracket_component_alias_is_case_insensitive_and_keeps_nested_owner() {
        let d = parse(
            "NoDe RenamedHost9751 {\n\
               [Worker Port 9767] AS Worker9767 <<edge_9769>> #LightBlue\n\
             }",
        );

        assert_eq!(d.nodes[0].id, "RenamedHost9751");
        assert_eq!(d.nodes[0].children, ["Worker9767"]);
        let worker = d
            .nodes
            .iter()
            .find(|node| node.id == "Worker9767")
            .expect("mixed-case bracket alias");
        assert_eq!(worker.label, "Worker Port 9767");
        assert_eq!(worker.kind, DeploymentNodeKind::Component);
        assert_eq!(worker.stereotype.as_deref(), Some("edge_9769"));
        assert_eq!(worker.color.as_deref(), Some("LightBlue"));
        assert_eq!(worker.source_line, 2);
    }

    #[test]
    fn bracket_component_no_alias() {
        let d = parse("[MyComponent]");
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].label, "MyComponent");
        assert_eq!(d.nodes[0].id, "MyComponent");
        assert_eq!(d.nodes[0].kind, DeploymentNodeKind::Component);
    }

    #[test]
    fn duplicate_nested_declarations_keep_first_parent() {
        let d = parse(
            r#"node "Validator Node 1" {
  component "Consensus Engine"
  database "Ledger"
}
node "Validator Node 2" {
  component "Consensus Engine"
  database "Ledger"
}"#,
        );
        let first = d
            .nodes
            .iter()
            .find(|n| n.label == "Validator Node 1")
            .unwrap();
        assert!(first.declared_container);
        assert_eq!(
            first.children,
            vec!["Consensus Engine".to_string(), "Ledger".to_string()]
        );
        let second = d
            .nodes
            .iter()
            .find(|n| n.label == "Validator Node 2")
            .unwrap();
        assert!(second.children.is_empty());
        assert!(second.declared_container);
    }

    #[test]
    fn element_color_captured() {
        let d =
            parse("node AppNode #Pink\nartifact a #FF8888\ncloud C #LightBlue {\n  node Inner\n}");
        assert_eq!(
            d.nodes
                .iter()
                .find(|n| n.id == "AppNode")
                .unwrap()
                .color
                .as_deref(),
            Some("Pink")
        );
        assert_eq!(
            d.nodes
                .iter()
                .find(|n| n.id == "a")
                .unwrap()
                .color
                .as_deref(),
            Some("FF8888")
        );
        // Colour on a container is captured too.
        assert_eq!(
            d.nodes
                .iter()
                .find(|n| n.id == "C")
                .unwrap()
                .color
                .as_deref(),
            Some("LightBlue")
        );
        // Plain element has no colour.
        assert_eq!(
            d.nodes.iter().find(|n| n.id == "Inner").unwrap().color,
            None
        );
    }
}
