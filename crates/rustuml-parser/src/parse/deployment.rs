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

pub fn parse_deployment(lines: &[String]) -> Result<DeploymentDiagram, ParseError> {
    let mut nodes: Vec<DeploymentNode> = Vec::new();
    let mut connections = Vec::new();
    let mut meta = DiagramMeta::default();

    // Stack of node IDs for tracking nesting depth.
    let mut stack: Vec<String> = Vec::new();

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

    // connection: (id|"label") arrow (id|"label") [: label]
    static RE_CONN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^("?[\w. ]+"?)\s*([-.<>|]+)\s*("?[\w. ]+"?)(?:\s*:\s*(.+))?$"#).unwrap()
    });

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Title directive.
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
            continue;
        }
        // Skip skinparam and decoration lines.
        if trimmed.starts_with("skinparam ")
            || trimmed.starts_with("hide ")
            || trimmed.starts_with("show ")
        {
            continue;
        }

        // Handle closing brace — pop the current container from the stack.
        if trimmed == "}" {
            stack.pop();
            continue;
        }

        // Check if the first word is a deployment keyword.
        let first_word: &str = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("");

        if keyword_set.contains(first_word) {
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
                    let label = caps[2].to_string();
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

        // Connection line.
        if let Some(caps) = RE_CONN.captures(trimmed) {
            let raw_from = caps[1].trim_matches('"').to_string();
            let raw_to = caps[3].trim_matches('"').to_string();

            let from = resolve_id(&nodes, &raw_from);
            let to = resolve_id(&nodes, &raw_to);
            let label = caps.get(4).map(|m| m.as_str().trim().to_string());

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
        notes: Vec::new(),
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
}
