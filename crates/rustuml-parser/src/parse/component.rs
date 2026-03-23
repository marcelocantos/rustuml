// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::component::*;

/// Container keywords recognised by the component diagram parser.
const CONTAINER_KEYWORDS: &[&str] = &[
    "cloud", "folder", "node", "frame", "rectangle", "package", "database", "storage", "actor",
    "component",
];

/// Check if a trimmed line opens a container block (keyword followed by optional label and `{`).
fn container_keyword(trimmed: &str) -> Option<&'static str> {
    for &kw in CONTAINER_KEYWORDS {
        if trimmed.starts_with(kw) {
            let rest = &trimmed[kw.len()..];
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') || rest.starts_with('"') {
                return Some(kw);
            }
        }
    }
    None
}

/// Extract a stereotype string from `<<name>>` syntax in a line.
fn parse_stereotype(s: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<(\w+)>>").unwrap());
    RE.captures(s).map(|caps| caps[1].to_string())
}

/// Parse a container label from a line like:
///   `cloud Outer #LightBlue {`
///   `folder Inner {`
///   `package "My Package" {`
///   `node Server`
///
/// Returns `(id, label)`.
fn parse_container_label(kw: &str, rest: &str) -> (String, String) {
    static RE_QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^\s*"([^"]+)"(?:\s+as\s+(\w+))?(?:\s+[^{]*)?\{?"#).unwrap());
    static RE_WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^\s*(\w+)(?:\s+[^{]*)?\{?"#).unwrap());

    if let Some(caps) = RE_QUOTED.captures(rest) {
        let label = caps[1].to_string();
        let id = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| label.clone());
        return (id, label);
    }
    if let Some(caps) = RE_WORD.captures(rest) {
        let name = caps[1].to_string();
        return (name.clone(), name);
    }
    // Fallback: use keyword as both id and label.
    (kw.to_string(), kw.to_string())
}

pub fn parse_component(lines: &[String]) -> Result<ComponentDiagram, ParseError> {
    let mut components = Vec::new();
    let mut interfaces = Vec::new();
    let mut connections = Vec::new();
    let meta = DiagramMeta::default();

    // Parse into a nested structure via a stack.
    // Each stack frame is a mutable ComponentPackage under construction.
    let mut package_stack: Vec<ComponentPackage> = Vec::new();
    // Top-level packages collected.
    let mut top_packages: Vec<ComponentPackage> = Vec::new();

    static RE_COMP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^component\s+(?:"([^"]+)"\s+as\s+(\w+)|"([^"]+)"|(\w+))(?:\s+[^{]*)?"#).unwrap());
    static RE_BRACKET: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]$").unwrap());
    static RE_IFACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^interface\s+"([^"]+)"\s+as\s+(\w+)"#).unwrap());
    // Matches: FROM ["from_mult"] ARROW ["to_mult"] TO [: label]
    // FROM and TO can be [bracket] or \w+ identifiers.
    // Optional quoted multiplicities immediately adjoin the arrow on either side.
    static RE_CONN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?:\[([^\]]+)\]|(\w+))\s*(?:"([^"]*)")?\s*([-.\|<>~]+)\s*(?:"([^"]*)")?\s*(?:\[([^\]]+)\]|(\w+))(?:\s*:\s*(.+))?$"#,
        )
        .unwrap()
    });

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Closing brace — pop the stack.
        if trimmed == "}" {
            if let Some(finished) = package_stack.pop() {
                if let Some(parent) = package_stack.last_mut() {
                    parent.packages.push(finished);
                } else {
                    top_packages.push(finished);
                }
            }
            continue;
        }

        // Opening container block?
        if let Some(kw) = container_keyword(trimmed) {
            // Only open a block if the line contains '{'.
            if trimmed.contains('{') {
                let rest = &trimmed[kw.len()..];
                let (id, label) = parse_container_label(kw, rest);
                package_stack.push(ComponentPackage {
                    name: id,
                    label,
                    stereotype: parse_stereotype(trimmed),
                    components: Vec::new(),
                    packages: Vec::new(),
                });
                continue;
            }
        }

        // Component declaration.
        if let Some(caps) = RE_COMP.captures(trimmed) {
            // Variants:
            //   component "Label" as ID → caps[1]=Label, caps[2]=ID
            //   component "Label"       → caps[3]=Label, id=Label
            //   component ID            → caps[4]=ID
            let (id, label) = if caps.get(1).is_some() {
                (caps[2].to_string(), caps[1].to_string())
            } else if caps.get(3).is_some() {
                let l = caps[3].to_string();
                (l.clone(), l)
            } else {
                let id = caps[4].to_string();
                (id.clone(), id)
            };

            if !components.iter().any(|c: &Component| c.id == id) {
                components.push(Component {
                    id: id.clone(),
                    label,
                    stereotype: parse_stereotype(trimmed),
                });
            }
            if let Some(pkg) = package_stack.last_mut() {
                if !pkg.components.contains(&id) {
                    pkg.components.push(id);
                }
            }
            continue;
        }

        if let Some(caps) = RE_BRACKET.captures(trimmed) {
            let name = caps[1].to_string();
            // Use the bracket label as both id and display label.
            let id = name.replace(' ', "_");
            if !components.iter().any(|c: &Component| c.id == id) {
                components.push(Component {
                    id: id.clone(),
                    label: name,
                    stereotype: parse_stereotype(trimmed),
                });
            }
            if let Some(pkg) = package_stack.last_mut() {
                if !pkg.components.contains(&id) {
                    pkg.components.push(id);
                }
            }
            continue;
        }

        if let Some(caps) = RE_IFACE.captures(trimmed) {
            interfaces.push(Interface {
                id: caps[2].to_string(),
                label: caps[1].to_string(),
            });
            continue;
        }

        if let Some(caps) = RE_CONN.captures(trimmed) {
            let from = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().replace(' ', "_"))
                .unwrap_or_default();
            let from_mult = caps.get(3).map(|m| m.as_str().to_string());
            let arrow = &caps[4];
            let to_mult = caps.get(5).map(|m| m.as_str().to_string());
            let to = caps
                .get(6)
                .or(caps.get(7))
                .map(|m| m.as_str().replace(' ', "_"))
                .unwrap_or_default();
            let label = caps.get(8).map(|m| m.as_str().trim().to_string());
            let dashed = arrow.contains("..");

            // Auto-create components from connection endpoints.
            for id in [&from, &to] {
                if !id.is_empty()
                    && !components.iter().any(|c| c.id == *id)
                    && !interfaces.iter().any(|i| i.id == *id)
                {
                    components.push(Component {
                        id: id.clone(),
                        label: id.clone(),
                        stereotype: None,
                    });
                }
            }

            if !from.is_empty() && !to.is_empty() {
                connections.push(Connection {
                    from,
                    to,
                    label,
                    from_mult,
                    to_mult,
                    dashed,
                });
            }
        }
    }

    // Close any unclosed blocks (defensive).
    while let Some(finished) = package_stack.pop() {
        if let Some(parent) = package_stack.last_mut() {
            parent.packages.push(finished);
        } else {
            top_packages.push(finished);
        }
    }

    Ok(ComponentDiagram {
        meta,
        components,
        interfaces,
        connections,
        packages: top_packages,
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

    #[test]
    fn cloud_container_label() {
        let d = parse("cloud Outer #LightBlue {\n  folder Inner {\n    component X\n    component Y\n    X --> Y\n  }\n}");
        assert_eq!(d.packages.len(), 1, "should have 1 top-level package");
        assert_eq!(d.packages[0].label, "Outer");
        assert_eq!(d.packages[0].packages.len(), 1, "should have 1 nested package");
        assert_eq!(d.packages[0].packages[0].label, "Inner");
        assert!(d.components.iter().any(|c| c.id == "X"));
        assert!(d.components.iter().any(|c| c.id == "Y"));
        assert_eq!(d.connections.len(), 1);
    }

    #[test]
    fn parallel_containers() {
        let d = parse(
            "cloud G1 {\n  component AA\n}\nfolder G2 {\n  component BB\n}\nnode G3 {\n  component CC\n}\nAA --> BB\nBB --> CC",
        );
        assert_eq!(d.packages.len(), 3);
        assert_eq!(d.packages[0].label, "G1");
        assert_eq!(d.packages[1].label, "G2");
        assert_eq!(d.packages[2].label, "G3");
    }
}
