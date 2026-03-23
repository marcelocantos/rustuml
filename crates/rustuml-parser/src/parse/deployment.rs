// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::deployment::*;

pub fn parse_deployment(lines: &[String]) -> Result<DeploymentDiagram, ParseError> {
    let mut nodes: Vec<DeploymentNode> = Vec::new();
    let mut connections = Vec::new();
    let meta = DiagramMeta::default();

    // Stack of node IDs for tracking nesting depth.
    let mut stack: Vec<String> = Vec::new();

    static RE_NODE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(node|artifact|cloud|database|storage|frame|folder|actor|queue|component|rectangle|agent|boundary|card|collections|control|entity|file|package|stack)\s+(?:"([^"]+)"\s+as\s+)?(\w+)(?:\s+<<([^>]+)>>)?(?:\s+#\w+)?(?:\s*\{)?"#,
        )
        .unwrap()
    });
    static RE_CONN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\w+)\s*([-.\|>]+)\s*(\w+)(?:\s*:\s*(.+))?$").unwrap());

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle closing brace — pop the current container from the stack.
        if trimmed == "}" {
            stack.pop();
            continue;
        }

        if let Some(caps) = RE_NODE.captures(trimmed) {
            let kind = match &caps[1] {
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
            };
            let label = caps
                .get(2)
                .map_or_else(|| caps[3].to_string(), |m| m.as_str().to_string());
            let id = caps[3].to_string();
            let stereotype = caps.get(4).map(|m| m.as_str().trim().to_string());

            // Push node to the flat list if not already present.
            if !nodes.iter().any(|n: &DeploymentNode| n.id == id) {
                nodes.push(DeploymentNode {
                    id: id.clone(),
                    label,
                    kind,
                    stereotype,
                    children: Vec::new(),
                });
            }

            // Add as child of current container (if inside one).
            if let Some(parent_id) = stack.last().cloned() {
                if let Some(parent) = nodes.iter_mut().find(|n| n.id == parent_id) {
                    if !parent.children.contains(&id) {
                        parent.children.push(id.clone());
                    }
                }
            }

            // If this line opens a block, push onto the stack.
            if trimmed.contains('{') {
                stack.push(id);
            }
        } else if let Some(caps) = RE_CONN.captures(trimmed) {
            let from = caps[1].to_string();
            let to = caps[3].to_string();
            let label = caps.get(4).map(|m| m.as_str().trim().to_string());

            // Auto-create nodes for any unknown IDs in connections.
            for id in [&from, &to] {
                if !nodes.iter().any(|n| n.id == *id) {
                    nodes.push(DeploymentNode {
                        id: id.clone(),
                        label: id.clone(),
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
}
