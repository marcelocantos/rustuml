// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram parser.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::deployment::*;

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

fn push_node(
    nodes: &mut Vec<DeploymentNode>,
    id: String,
    label: String,
    kind: DeploymentNodeKind,
    stereotype: Option<String>,
) {
    if !nodes.iter().any(|n| n.id == id) {
        nodes.push(DeploymentNode {
            id,
            label,
            kind,
            stereotype,
            children: Vec::new(),
        });
    }
}

fn add_child(nodes: &mut Vec<DeploymentNode>, parent_id: &str, child_id: &str) {
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
    if s.starts_with('"') {
        // Quoted: find closing quote.
        let inner = &s[1..];
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

/// Try to parse a connection from a trimmed line.
/// Handles:
///   - `from_id --> to_id : label`
///   - `"From Label" --> "To Label" : label`
///   - `keyword "From Label" --> to_id : label`  (uses keyword as FROM id)
///
/// The arrow is any combination of `-`, `.`, `<`, `>`, `|` characters (2+ chars).
/// Returns `(raw_from, raw_to, label)` on success.
fn try_parse_connection(
    trimmed: &str,
    keyword_set: &HashSet<&str>,
) -> Option<(String, String, Option<String>)> {
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
            if keyword_set.contains(kw) {
                let after_kw = rest[kw_end..].trim_start();
                if after_kw.starts_with('"') {
                    // keyword "label" ... — skip the quoted label, then look for arrow.
                    if let Some(close_quote) = after_kw[1..].find('"') {
                        let after_label = after_kw[close_quote + 2..].trim_start();
                        // Check if what follows is an arrow.
                        let arrow_end = after_label
                            .find(|c: char| !matches!(c, '-' | '.' | '<' | '>' | '|'))
                            .unwrap_or(after_label.len());
                        if arrow_end >= 2 {
                            let arrow = &after_label[..arrow_end];
                            // A valid arrow must have a shaft character (`-` or `.`).
                            // Pure `<<` is a stereotype opener, not an arrow.
                            if arrow.chars().any(|c| matches!(c, '-' | '.')) {
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
                                return Some((kw.to_string(), raw_to.to_string(), label));
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse FROM endpoint.
    let (raw_from, after_from) = parse_endpoint(rest)?;
    let after_from = after_from.trim_start();

    // Parse arrow: one or more of `-`, `.`, `<`, `>`, `|`.
    let arrow_end = after_from
        .find(|c: char| !matches!(c, '-' | '.' | '<' | '>' | '|'))
        .unwrap_or(after_from.len());
    if arrow_end < 2 {
        // Need at least 2 arrow characters (e.g., `--`, `->`, `..`).
        return None;
    }
    let arrow = &after_from[..arrow_end];
    // Must contain at least one shaft character (`-` or `.`).
    // Pure `<<...>>` is a stereotype, not an arrow.
    if !arrow.chars().any(|c| matches!(c, '-' | '.')) {
        return None;
    }
    let after_arrow = after_from[arrow_end..].trim_start();

    // Parse TO endpoint.
    let (raw_to, after_to) = parse_endpoint(after_arrow)?;
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

    Some((raw_from.to_string(), raw_to.to_string(), label))
}

/// Accumulator for multiline note bodies.
struct NoteAccum {
    target: Option<String>,
    id: Option<String>,
    lines: Vec<String>,
}

pub fn parse_deployment(lines: &[String]) -> Result<DeploymentDiagram, ParseError> {
    let mut nodes: Vec<DeploymentNode> = Vec::new();
    let mut connections = Vec::new();
    let mut notes = Vec::new();
    let mut meta = DiagramMeta::default();

    // Stack of node IDs for tracking nesting depth.
    let mut stack: Vec<String> = Vec::new();

    // Multiline note accumulator.
    let mut note_accum: Option<NoteAccum> = None;

    // Legend accumulator.
    let mut in_legend = false;
    let mut legend_lines: Vec<String> = Vec::new();

    let keyword_set: HashSet<&str> = DEPLOYMENT_KEYWORDS.iter().copied().collect();

    // keyword id [as "label"] [<<stereo>>] [#color] [{]
    static RE_NODE_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(\w+)\s+(\w[\w.]*)(?:\s+as\s+"([^"]+)")?(?:\s+<<([^>]+)>>)?(?:\s+#\w+)?(?:\s*\{)?"#,
        )
        .unwrap()
    });

    // keyword "label" [as id] [<<stereo>>] [#color] [{]
    static RE_NODE_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(\w+)\s+"([^"]+)"(?:\s+as\s+(\w+))?(?:\s+<<([^>]+)>>)?(?:\s+#\w+)?(?:\s*\{)?"#,
        )
        .unwrap()
    });

    // [Label] [as id] [<<stereo>>] [#color]  — bracket component notation
    static RE_NODE_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^\[([^\]]+)\](?:\s+as\s+(\w+))?(?:\s+<<([^>]+)>>)?(?:\s+#\w+)?\s*$"#,
        )
        .unwrap()
    });

    // note "text" as ID  — floating note
    static RE_NOTE_FLOATING: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap()
    });

    // note direction of target : text  (inline attached note)
    static RE_NOTE_ATTACHED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note\s+(?:top|bottom|left|right)\s+of\s+("?[^":]+?"?)\s*:\s*(.+)$"#)
            .unwrap()
    });

    // note direction of target  (multiline attached note — no colon)
    static RE_NOTE_ATTACHED_MULTI: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note\s+(?:top|bottom|left|right)\s+of\s+("?[^"]+?"?)\s*$"#).unwrap()
    });

    // N1 .. N2  — note link (N1 is the note ID, N2 is the target)
    static RE_NOTE_LINK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(\w+)\s+\.\.\s+(\w+)\s*$").unwrap()
    });

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Multiline note body.
        if note_accum.is_some() {
            if trimmed == "end note" || trimmed == "endnote" {
                let accum = note_accum.take().unwrap();
                let text = accum.lines.join("\n");
                notes.push(DeploymentNote {
                    id: accum.id,
                    target: accum.target,
                    text,
                });
            } else {
                note_accum.as_mut().unwrap().lines.push(trimmed.to_string());
            }
            continue;
        }

        // Legend block.
        if trimmed == "legend" || trimmed.starts_with("legend ") {
            in_legend = true;
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
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: parts[0].to_string(),
                    value: parts[1].trim().to_string(),
                });
            }
            continue;
        }
        // Skip other decoration lines.
        if trimmed.starts_with("hide ")
            || trimmed.starts_with("show ")
            || trimmed.starts_with("left to right direction")
            || trimmed.starts_with("top to bottom direction")
        {
            continue;
        }

        // Handle closing brace — pop the current container from the stack.
        if trimmed == "}" {
            stack.pop();
            continue;
        }

        // Bracket notation: [Label] [as id] [<<stereo>>]  — component shorthand.
        if trimmed.starts_with('[') {
            if let Some(caps) = RE_NODE_BRACKET.captures(trimmed) {
                let label = caps[1].trim().to_string();
                let id = caps
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| label_to_id(&label));
                let stereotype = caps.get(3).map(|m| m.as_str().trim().to_string());
                push_node(
                    &mut nodes,
                    id.clone(),
                    label,
                    DeploymentNodeKind::Component,
                    stereotype,
                );
                if let Some(parent_id) = stack.last().cloned() {
                    add_child(&mut nodes, &parent_id, &id);
                }
                continue;
            }
        }

        // Floating note: note "text" as ID
        if let Some(caps) = RE_NOTE_FLOATING.captures(trimmed) {
            let text = caps[1].replace("\\n", "\n");
            let id = caps[2].to_string();
            notes.push(DeploymentNote {
                id: Some(id),
                target: None,
                text,
            });
            continue;
        }

        // Attached note: note direction of target : text  (inline)
        if let Some(caps) = RE_NOTE_ATTACHED.captures(trimmed) {
            let target_raw = caps[1].trim().trim_matches('"').to_string();
            let target = resolve_id(&nodes, &target_raw);
            let text = caps[2].trim().to_string();
            notes.push(DeploymentNote {
                id: None,
                target: Some(target),
                text,
            });
            continue;
        }

        // Multiline attached note: note direction of target  (no colon)
        if let Some(caps) = RE_NOTE_ATTACHED_MULTI.captures(trimmed) {
            let target_raw = caps[1].trim().trim_matches('"').to_string();
            let target = resolve_id(&nodes, &target_raw);
            note_accum = Some(NoteAccum {
                id: None,
                target: Some(target),
                lines: Vec::new(),
            });
            continue;
        }

        // Note link: N1 .. N2
        if let Some(caps) = RE_NOTE_LINK.captures(trimmed) {
            let note_id = caps[1].to_string();
            let target_id = caps[2].to_string();
            // If there's a floating note with this id, set its target.
            if let Some(note) = notes.iter_mut().find(|n| n.id.as_deref() == Some(&note_id)) {
                note.target = Some(target_id);
            }
            continue;
        }

        // Check if the first word is a deployment keyword.
        let first_word: &str = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("");

        if keyword_set.contains(first_word) {
            // Check if this is a connection line (keyword "label" --> ...)
            // before treating it as a pure node declaration.
            if let Some((raw_from, raw_to, label)) =
                try_parse_connection(trimmed, &keyword_set)
            {
                let from = resolve_id(&nodes, &raw_from);
                let to = resolve_id(&nodes, &raw_to);

                for (id, lbl) in [(&from, &raw_from), (&to, &raw_to)] {
                    if !nodes.iter().any(|n| n.id == *id) {
                        nodes.push(DeploymentNode {
                            id: id.clone(),
                            label: lbl.clone(),
                            kind: DeploymentNodeKind::Node,
                            stereotype: None,
                            children: Vec::new(),
                        });
                    }
                }
                connections.push(DeploymentConnection { from, to, label });
                continue;
            }

            // Try bare form first: keyword id [as "label"]
            if let Some(caps) = RE_NODE_BARE.captures(trimmed) {
                let keyword = &caps[1];
                if keyword_set.contains(keyword) {
                    let raw_id = caps[2].to_string();
                    let label = caps
                        .get(3)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| raw_id.clone());
                    let id = raw_id;
                    let stereotype = caps.get(4).map(|m| m.as_str().trim().to_string());
                    let kind = kind_from_keyword(keyword);

                    push_node(&mut nodes, id.clone(), label, kind, stereotype);
                    if let Some(parent_id) = stack.last().cloned() {
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
                let keyword = &caps[1];
                if keyword_set.contains(keyword) {
                    // Process `\n` escape sequences in quoted labels.
                    let label = process_label(&caps[2]);
                    let id = caps
                        .get(3)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| label_to_id(&label));
                    let stereotype = caps.get(4).map(|m| m.as_str().trim().to_string());
                    let kind = kind_from_keyword(keyword);

                    push_node(&mut nodes, id.clone(), label, kind, stereotype);
                    if let Some(parent_id) = stack.last().cloned() {
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
        if let Some((raw_from, raw_to, label)) = try_parse_connection(trimmed, &keyword_set) {
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
                        children: Vec::new(),
                    });
                }
            }

            connections.push(DeploymentConnection { from, to, label });
        }
    }

    Ok(DeploymentDiagram {
        meta,
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
    fn basic_deployment() {
        let d = parse("node WebServer {\n  artifact app\n}\ndatabase DB\nWebServer --> DB");
        assert!(d.nodes.iter().any(|n| n.id == "WebServer"));
        assert!(d.nodes.iter().any(|n| n.id == "DB"));
        assert_eq!(d.connections.len(), 1);
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
        let d = parse("node \"server-01.example.com\"\nnode \"db_primary\"\n\"server-01.example.com\" --> \"db_primary\" : port 5432");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("port 5432"));
    }

    #[test]
    fn keyword_prefixed_connection() {
        let d = parse("artifact \"app.war\"\nnode Server\nartifact \"app.war\" --> Server : deploy");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("deploy"));
    }

    #[test]
    fn note_attached() {
        let d = parse("node AppServer\nnote right of AppServer : 16 cores");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "16 cores");
        assert_eq!(d.notes[0].target.as_deref(), Some("AppServer"));
    }

    #[test]
    fn note_floating() {
        let d = parse("node Server\nnote \"Primary server\" as N1\nN1 .. Server");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Primary server");
        assert_eq!(d.notes[0].target.as_deref(), Some("Server"));
    }

    #[test]
    fn header_footer() {
        let d = parse("header My Header\nfooter My Footer\nnode Server");
        assert_eq!(d.meta.header.as_deref(), Some("My Header"));
        assert_eq!(d.meta.footer.as_deref(), Some("My Footer"));
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
    fn bracket_component_no_alias() {
        let d = parse("[MyComponent]");
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].label, "MyComponent");
        assert_eq!(d.nodes[0].id, "MyComponent");
        assert_eq!(d.nodes[0].kind, DeploymentNodeKind::Component);
    }
}
