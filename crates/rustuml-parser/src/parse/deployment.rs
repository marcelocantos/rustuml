// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram parser.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::deployment::*;
use crate::diagram::{DiagramMeta, LegendHorizontalAlignment, LegendVerticalAlignment};

/// All keywords that introduce a deployment diagram element.
pub const DEPLOYMENT_KEYWORDS: &[&str] = &[
    "node",
    "artifact",
    "cloud",
    "database",
    "storage",
    "frame",
    "folder",
    "actor",
    "queue",
    "component",
    "rectangle",
    "agent",
    "boundary",
    "card",
    "collections",
    "control",
    "entity",
    "file",
    "package",
    "stack",
];

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

fn kind_from_keyword(keyword: &str) -> DeploymentNodeKind {
    match keyword {
        "artifact" => DeploymentNodeKind::Artifact,
        "cloud" => DeploymentNodeKind::Cloud,
        "database" => DeploymentNodeKind::Database,
        "storage" => DeploymentNodeKind::Storage,
        "frame" => DeploymentNodeKind::Frame,
        "folder" => DeploymentNodeKind::Folder,
        "actor" => DeploymentNodeKind::Actor,
        "queue" => DeploymentNodeKind::Queue,
        "component" => DeploymentNodeKind::Component,
        "rectangle" => DeploymentNodeKind::Rectangle,
        "agent" => DeploymentNodeKind::Agent,
        "boundary" => DeploymentNodeKind::Boundary,
        "card" => DeploymentNodeKind::Card,
        "collections" => DeploymentNodeKind::Collections,
        "control" => DeploymentNodeKind::Control,
        "entity" => DeploymentNodeKind::Entity,
        "file" => DeploymentNodeKind::File,
        "package" => DeploymentNodeKind::Package,
        "stack" => DeploymentNodeKind::Stack,
        _ => DeploymentNodeKind::Node,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_node(
    nodes: &mut Vec<DeploymentNode>,
    id: String,
    label: String,
    kind: DeploymentNodeKind,
    stereotype: Option<String>,
    color: Option<String>,
    declared_container: bool,
    source_line: usize,
) -> bool {
    if !nodes.iter().any(|n| n.id == id) {
        nodes.push(DeploymentNode {
            id,
            label,
            kind,
            stereotype,
            color,
            declared_container,
            children: Vec::new(),
            source_line,
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
    length: usize,
}

/// Try to parse a connection from a trimmed line.
/// Handles:
///   - `from_id --> to_id : label`
///   - `"From Label" --> "To Label" : label`
///   - `keyword "From Label" --> to_id : label`  (uses keyword as FROM id)
///
/// The arrow is any combination of `-`, `.`, `~`, `=`, `<`, `>`, `|`
/// characters (2+ chars).
fn try_parse_connection(
    trimmed: &str,
    keyword_set: &HashSet<&str>,
) -> Option<ParsedDeploymentConnection> {
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
            if keyword_set.contains(kw.to_ascii_lowercase().as_str()) {
                let after_kw = rest[kw_end..].trim_start();
                if let Some(after_open_quote) = after_kw.strip_prefix('"') {
                    // `CommandLinkElement` parses the quoted text between the
                    // first endpoint and arrow as the tail quantifier.
                    if let Some(close_quote) = after_open_quote.find('"') {
                        let tail_label = process_label(&after_open_quote[..close_quote]);
                        let after_label = after_open_quote[close_quote + 1..].trim_start();
                        // Check if what follows is an arrow.
                        let arrow_end = after_label
                            .find(|c: char| !matches!(c, '-' | '.' | '~' | '=' | '<' | '>' | '|'))
                            .unwrap_or(after_label.len());
                        if arrow_end >= 2 {
                            let arrow = &after_label[..arrow_end];
                            // A valid arrow must have a shaft character.
                            // Pure `<<` is a stereotype opener, not an arrow.
                            if arrow.chars().any(|c| matches!(c, '-' | '.' | '~' | '=')) {
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
                                    arrow_at_start: arrow.starts_with('<'),
                                    arrow_at_end: arrow.ends_with('>'),
                                    direction: deployment_link_direction(arrow),
                                    style: deployment_link_style(arrow),
                                    length: deployment_link_length(arrow),
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
    let arrow_end = {
        let bytes = after_from.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if matches!(c, '-' | '.' | '~' | '=' | '<' | '>' | '|') {
                i += 1;
            } else if matches!(c, 'd' | 'u' | 'l' | 'r')
                && i > 0
                && matches!(bytes[i - 1] as char, '-' | '.' | '~' | '=')
            {
                // Look for `down`, `up`, `left`, `right` followed by another
                // shaft character.
                let rest = &after_from[i..];
                let kw_len = ["down", "up", "left", "right"]
                    .iter()
                    .find(|kw| rest.starts_with(*kw))
                    .map(|kw| kw.len())
                    .unwrap_or(0);
                if kw_len > 0
                    && i + kw_len < bytes.len()
                    && matches!(bytes[i + kw_len] as char, '-' | '.' | '~' | '=')
                {
                    i += kw_len;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        i
    };
    if arrow_end < 2 {
        // Need at least 2 arrow characters (e.g., `--`, `->`, `..`).
        return None;
    }
    let arrow = &after_from[..arrow_end];
    // Must contain at least one shaft character.
    // Pure `<<...>>` is a stereotype, not an arrow.
    if !arrow.chars().any(|c| matches!(c, '-' | '.' | '~' | '=')) {
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
        arrow_at_start: arrow.starts_with('<'),
        arrow_at_end: arrow.ends_with('>'),
        direction: deployment_link_direction(arrow),
        style: deployment_link_style(arrow),
        length: deployment_link_length(arrow),
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

/// Accumulator for multiline note bodies.
struct NoteAccum {
    target: Option<String>,
    id: Option<String>,
    position: DeploymentNotePosition,
    source_line: usize,
    lines: Vec<String>,
}

pub fn parse_deployment(lines: &[String]) -> Result<DeploymentDiagram, ParseError> {
    let mut nodes: Vec<DeploymentNode> = Vec::new();
    let mut connections = Vec::new();
    let mut notes = Vec::new();
    let mut meta = DiagramMeta::default();
    let mut direction = DeploymentLayoutDirection::TopToBottom;

    // Stack of node IDs for tracking nesting depth.
    let mut stack: Vec<String> = Vec::new();

    // Multiline note accumulator.
    let mut note_accum: Option<NoteAccum> = None;

    // Legend accumulator.
    let mut in_legend = false;
    let mut legend_lines: Vec<String> = Vec::new();

    // Skinparam block accumulator. When inside `skinparam node { ... }`,
    // nested `Key Value` lines are flattened to `nodeKey` = `Value`.
    let mut skinparam_block_prefix: Option<String> = None;

    let keyword_set: HashSet<&str> = DEPLOYMENT_KEYWORDS.iter().copied().collect();

    // keyword id [as "label"] [<<stereo>>] [#color] [{]
    static RE_NODE_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(\w+)\s+(\w[\w.]*)(?:\s+(?i:as)\s+"([^"]+)")?(?:\s+<<([^>]+)>>)?(?:\s+#(\w+))?(?:\s*\{)?"#,
        )
        .unwrap()
    });

    // keyword "label" [as id] [<<stereo>>] [#color] [{]
    static RE_NODE_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(\w+)\s+"([^"]+)"(?:\s+(?i:as)\s+(\w+))?(?:\s+<<([^>]+)>>)?(?:\s+#(\w+))?(?:\s*\{)?"#,
        )
        .unwrap()
    });

    // [Label] [as id] [<<stereo>>] [#color]  — bracket component notation
    static RE_NODE_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\[([^\]]+)\](?:\s+(?i:as)\s+(\w+))?(?:\s+<<([^>]+)>>)?(?:\s+#(\w+))?\s*$"#)
            .unwrap()
    });

    // note "text" as ID  — floating note
    static RE_NOTE_FLOATING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)^note\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap());

    // note as ID [#color] — multiline floating note
    static RE_NOTE_FLOATING_MULTI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^note\s+as\s+([\w.]+)\s*(?:#\S+)?\s*$").unwrap());

    // note direction of target : text  (inline attached note)
    static RE_NOTE_ATTACHED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)^note\s+(top|bottom|left|right)\s+of\s+("?[^":]+?"?)\s*:\s*(.+)$"#)
            .unwrap()
    });

    // note direction of target  (multiline attached note — no colon)
    static RE_NOTE_ATTACHED_MULTI: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)^note\s+(top|bottom|left|right)\s+of\s+("?[^"]+?"?)\s*$"#).unwrap()
    });

    // N1 .. N2  — note link (N1 is the note ID, N2 is the target)
    static RE_NOTE_LINK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\w+)\s+\.\.\s+(\w+)\s*$").unwrap());

    for (line_idx, line) in lines.iter().enumerate() {
        let (current_line, trimmed) = super::source_line_and_trimmed(line_idx + 1, line);
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
                id.clone(),
                label,
                DeploymentNodeKind::Component,
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

        // Floating note: note "text" as ID
        if let Some(caps) = RE_NOTE_FLOATING.captures(trimmed) {
            let text = caps[1].replace("\\n", "\n");
            let id = caps[2].to_string();
            notes.push(DeploymentNote {
                id: Some(id),
                target: None,
                text,
                position: DeploymentNotePosition::Right,
                source_line: current_line,
            });
            continue;
        }

        if let Some(caps) = RE_NOTE_FLOATING_MULTI.captures(trimmed) {
            note_accum = Some(NoteAccum {
                id: Some(caps[1].to_string()),
                target: None,
                position: DeploymentNotePosition::Right,
                // Preprocessing strips the `@startuml` line before the first
                // content record; the multiline command's Java location is
                // the adjacent original source line.
                source_line: current_line.max(1) + 1,
                lines: Vec::new(),
            });
            continue;
        }

        // Attached note: note direction of target : text  (inline)
        if let Some(caps) = RE_NOTE_ATTACHED.captures(trimmed) {
            let position = deployment_note_position(&caps[1].to_ascii_lowercase());
            let target_raw = caps[2].trim().trim_matches('"').to_string();
            let target = resolve_id(&nodes, &target_raw);
            let text = caps[3].trim().to_string();
            notes.push(DeploymentNote {
                id: None,
                target: Some(target),
                text,
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
            note_accum = Some(NoteAccum {
                id: None,
                target: Some(target),
                position,
                source_line: current_line + 1,
                lines: Vec::new(),
            });
            continue;
        }

        // Note link: N1 .. N2 — only when one endpoint is a known floating
        // note id. Otherwise `X .. Y` is an ordinary (dotted) association
        // between two nodes and must fall through to connection parsing.
        if let Some(caps) = RE_NOTE_LINK.captures(trimmed) {
            let lhs = caps[1].to_string();
            let rhs = caps[2].to_string();
            let lhs_note = notes.iter().any(|n| n.id.as_deref() == Some(lhs.as_str()));
            let rhs_note = notes.iter().any(|n| n.id.as_deref() == Some(rhs.as_str()));
            if lhs_note || rhs_note {
                // Attach the note to the non-note endpoint.
                let (note_id, target_id, position) = if lhs_note {
                    (lhs, rhs, DeploymentNotePosition::Top)
                } else {
                    (rhs, lhs, DeploymentNotePosition::Bottom)
                };
                if let Some(note) = notes.iter_mut().find(|n| n.id.as_deref() == Some(&note_id)) {
                    note.target = Some(target_id);
                    note.position = position;
                }
                continue;
            }
            // Not a note link — fall through to connection handling below.
        }

        // Check if the first word is a deployment keyword.
        let first_word = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        if keyword_set.contains(first_word.as_str()) {
            // Check if this is a connection line (keyword "label" --> ...)
            // before treating it as a pure node declaration.
            if let Some(parsed) = try_parse_connection(trimmed, &keyword_set) {
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
                    length,
                } = parsed;
                let from = resolve_id(&nodes, &raw_from);
                let to = resolve_id(&nodes, &raw_to);

                for (id, lbl) in [(&from, &raw_from), (&to, &raw_to)] {
                    if !nodes.iter().any(|n| n.id == *id) {
                        nodes.push(DeploymentNode {
                            id: id.clone(),
                            label: lbl.clone(),
                            kind: DeploymentNodeKind::Default,
                            stereotype: None,
                            color: None,
                            declared_container: false,
                            children: Vec::new(),
                            source_line: current_line,
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
                    length,
                    source_line: current_line,
                });
                continue;
            }

            // Try bare form first: keyword id [as "label"]
            if let Some(caps) = RE_NODE_BARE.captures(trimmed) {
                // Pattern2.compileInternal applies CASE_INSENSITIVE to
                // CommandCreateElementFull and CommandPackageWithUSymbol.
                let keyword = caps[1].to_ascii_lowercase();
                if keyword_set.contains(keyword.as_str()) {
                    let raw_id = caps[2].to_string();
                    let label = caps
                        .get(3)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| raw_id.clone());
                    let id = raw_id;
                    let stereotype = caps.get(4).map(|m| m.as_str().trim().to_string());
                    let color = caps.get(5).map(|m| m.as_str().to_string());
                    let kind = kind_from_keyword(&keyword);
                    let declared_container = trimmed.contains('{');

                    let created = push_node(
                        &mut nodes,
                        id.clone(),
                        label,
                        kind,
                        stereotype,
                        color,
                        declared_container,
                        current_line,
                    );
                    if created && let Some(parent_id) = stack.last().cloned() {
                        add_child(&mut nodes, &parent_id, &id);
                    }
                    if trimmed.contains('{') {
                        stack.push(id);
                    }
                    continue;
                }
            }

            // Try quoted form: keyword "label" [as id]
            if let Some(caps) = RE_NODE_QUOTED.captures(trimmed) {
                let keyword = caps[1].to_ascii_lowercase();
                if keyword_set.contains(keyword.as_str()) {
                    // Process `\n` escape sequences in quoted labels.
                    let label = process_label(&caps[2]);
                    let id = caps
                        .get(3)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| label_to_id(&label));
                    let stereotype = caps.get(4).map(|m| m.as_str().trim().to_string());
                    let color = caps.get(5).map(|m| m.as_str().to_string());
                    let kind = kind_from_keyword(&keyword);
                    let declared_container = trimmed.contains('{');

                    let created = push_node(
                        &mut nodes,
                        id.clone(),
                        label,
                        kind,
                        stereotype,
                        color,
                        declared_container,
                        current_line,
                    );
                    if created && let Some(parent_id) = stack.last().cloned() {
                        add_child(&mut nodes, &parent_id, &id);
                    }
                    if trimmed.contains('{') {
                        stack.push(id);
                    }
                    continue;
                }
            }
        }

        // Connection line (bare identifiers or quoted labels).
        if let Some(parsed) = try_parse_connection(trimmed, &keyword_set) {
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
                length,
            } = parsed;
            let from = resolve_id(&nodes, &raw_from);
            let to = resolve_id(&nodes, &raw_to);

            // Auto-create nodes for any unknown IDs in connections.
            for (id, lbl) in [(&from, &raw_from), (&to, &raw_to)] {
                if !nodes.iter().any(|n| n.id == *id) {
                    nodes.push(DeploymentNode {
                        id: id.clone(),
                        label: lbl.clone(),
                        kind: DeploymentNodeKind::Node,
                        stereotype: None,
                        color: None,
                        declared_container: false,
                        children: Vec::new(),
                        source_line: current_line,
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
                length,
                source_line: current_line,
            });
        }
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
        assert_eq!(n.id, "application_deb");
    }

    #[test]
    fn quoted_connection() {
        let d = parse("artifact \"app.deb\"\nnode Server\n\"app.deb\" --> Server");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].from, "app_deb");
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
    fn note_attached() {
        let d = parse("node AppServer\nnote right of AppServer : 16 cores");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "16 cores");
        assert_eq!(d.notes[0].target.as_deref(), Some("AppServer"));
        assert_eq!(d.notes[0].position, DeploymentNotePosition::Right);
        assert_eq!(d.notes[0].source_line, 2);
    }

    #[test]
    fn note_floating() {
        let d = parse("node Server\nnote \"Primary server\" as N1\nN1 .. Server");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Primary server");
        assert_eq!(d.notes[0].target.as_deref(), Some("Server"));
        assert_eq!(d.notes[0].position, DeploymentNotePosition::Top);
        assert_eq!(d.notes[0].source_line, 2);
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
            "NoTe as DispatchPayload\n\
             control body remains note text\n\
             EnD NoTe\n\
             node RuntimeNode",
        );
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].id.as_deref(), Some("DispatchPayload"));
        assert_eq!(d.notes[0].text, "control body remains note text");
        assert_eq!(d.notes[0].source_line, 2);
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].id, "RuntimeNode");
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
            vec!["Consensus_Engine".to_string(), "Ledger".to_string()]
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
