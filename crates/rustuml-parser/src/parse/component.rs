// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::component::*;

pub fn parse_component(lines: &[String]) -> Result<ComponentDiagram, ParseError> {
    let mut components = Vec::new();
    let mut interfaces = Vec::new();
    let mut connections = Vec::new();
    let mut packages = Vec::new();
    let meta = DiagramMeta::default();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Component declaration: component "Label" as ID  or  [Label]
        static RE_COMP: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^component\s+"([^"]+)"\s+as\s+(\w+)"#).unwrap());
        static RE_BRACKET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[(\w+)\]$").unwrap());
        static RE_IFACE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^interface\s+"([^"]+)"\s+as\s+(\w+)"#).unwrap());
        static RE_CONN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(?:\[(\w+)\]|(\w+))\s*([-.\|>]+)\s*(?:\[(\w+)\]|(\w+))(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });
        static RE_PKG: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^package\s+"([^"]+)"\s*\{"#).unwrap());

        if let Some(caps) = RE_COMP.captures(trimmed) {
            components.push(Component {
                id: caps[2].to_string(),
                label: caps[1].to_string(),
            });
        } else if let Some(caps) = RE_BRACKET.captures(trimmed) {
            let name = caps[1].to_string();
            if !components.iter().any(|c: &Component| c.id == name) {
                components.push(Component {
                    id: name.clone(),
                    label: name,
                });
            }
        } else if let Some(caps) = RE_IFACE.captures(trimmed) {
            interfaces.push(Interface {
                id: caps[2].to_string(),
                label: caps[1].to_string(),
            });
        } else if let Some(caps) = RE_CONN.captures(trimmed) {
            let from = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let arrow = &caps[3];
            let to = caps
                .get(4)
                .or(caps.get(5))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let label = caps.get(6).map(|m| m.as_str().trim().to_string());
            let dashed = arrow.contains("..");

            // Auto-create components.
            for id in [&from, &to] {
                if !components.iter().any(|c| c.id == *id)
                    && !interfaces.iter().any(|i| i.id == *id)
                {
                    components.push(Component {
                        id: id.clone(),
                        label: id.clone(),
                    });
                }
            }

            connections.push(Connection {
                from,
                to,
                label,
                dashed,
            });
        } else if let Some(caps) = RE_PKG.captures(trimmed) {
            packages.push(ComponentPackage {
                name: caps[1].to_string(),
                components: Vec::new(),
            });
        }
    }

    Ok(ComponentDiagram {
        meta,
        components,
        interfaces,
        connections,
        packages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ComponentDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_component(&lines).unwrap()
    }

    #[test]
    fn basic_components() {
        let d = parse("component \"Web\" as WS\ncomponent \"DB\" as DB\nWS --> DB : query");
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("query"));
    }

    #[test]
    fn bracket_syntax() {
        let d = parse("[UI]\n[API]\n[UI] --> [API]");
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.connections.len(), 1);
    }
}
