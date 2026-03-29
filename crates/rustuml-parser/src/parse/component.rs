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
    "cloud",
    "folder",
    "node",
    "frame",
    "rectangle",
    "package",
    "database",
    "storage",
    "actor",
    "component",
    "queue",
    "boundary",
    "control",
    "entity",
    "collections",
];

/// Check if a trimmed line opens a container block (keyword followed by optional label and `{`).
fn container_keyword(trimmed: &str) -> Option<&'static str> {
    for &kw in CONTAINER_KEYWORDS {
        if let Some(rest) = trimmed.strip_prefix(kw)
            && (rest.is_empty()
                || rest.starts_with(' ')
                || rest.starts_with('\t')
                || rest.starts_with('"'))
        {
            return Some(kw);
        }
    }
    None
}

/// Extract all stereotype strings from `<<name>>` syntax in a line.
fn parse_stereotypes(s: &str) -> Vec<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<(\w+)>>").unwrap());
    RE.captures_iter(s)
        .map(|caps| caps[1].to_string())
        .collect()
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
    let mut notes: Vec<ComponentNote> = Vec::new();
    let mut meta = DiagramMeta::default();

    // Parse into a nested structure via a stack.
    // Each stack frame is a mutable ComponentPackage under construction.
    let mut package_stack: Vec<ComponentPackage> = Vec::new();
    // Top-level packages collected.
    let mut top_packages: Vec<ComponentPackage> = Vec::new();

    // Note buffer for multi-line notes.
    let mut note_target: Option<String> = None;
    let mut note_lines: Vec<String> = Vec::new();
    let mut in_note: bool = false;
    // Multiline title accumulation.
    let mut in_title: bool = false;
    let mut title_lines: Vec<String> = Vec::new();
    // Legend block accumulation.
    let mut in_legend: bool = false;
    let mut legend_lines: Vec<String> = Vec::new();

    static RE_COMP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^component\s+(?:"((?:[^"]|"")+)"\s+as\s+(\w+)|"((?:[^"]|"")+)"|(\w+))(?:\s+[^{]*)?"#,
        )
        .unwrap()
    });
    static RE_BRACKET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]$").unwrap());
    // Interface: `interface "Name" as ID`, `interface Name`, or `interface [Name] as ID`.
    static RE_IFACE_QUOTED_AS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^interface\s+"((?:[^"]|"")+)"\s+as\s+(\w+)"#).unwrap());
    static RE_IFACE_BRACKET_AS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^interface\s+\[([^\]]+)\]\s+as\s+(\w+)").unwrap());
    static RE_IFACE_BARE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^interface\s+(\w+)\s*$").unwrap());
    // Note: `note right of ID : text` or `note right of ID` (multiline)
    static RE_NOTE_OF: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^note\s+(?:right|left|top|bottom)\s+of\s+(\w+|\[[\w\s]+\])(?:\s*:\s*(.+))?$")
            .unwrap()
    });
    // Floating note: `note "text" as ID` or `note : text`
    static RE_NOTE_INLINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^note\s+"([^"]+)"\s+as\s+\w+"#).unwrap());
    // Matches: FROM ["from_mult"] ARROW ["to_mult"] TO [: label]
    // FROM and TO can be [bracket] or \w+ identifiers.
    // Arrow chars broadened to include lollipop notation: `-(`, `-(0-`, `--(`  etc.
    static RE_CONN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?:\[([^\]]+)\]|(\w+))\s*(?:"([^"]*)")?\s*([-.<>()|~0#*o]+)\s*(?:"([^"]*)")?\s*(?:\[([^\]]+)\]|(\w+))(?:\s*:\s*(.+))?$"#,
        )
        .unwrap()
    });

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_note {
                note_lines.push(String::new());
            }
            continue;
        }

        // End of multiline title.
        if in_title {
            if trimmed == "end title" {
                meta.title = Some(title_lines.join("\n"));
                in_title = false;
                title_lines.clear();
            } else {
                title_lines.push(trimmed.to_string());
            }
            continue;
        }

        // Inside legend block.
        if in_legend {
            if trimmed == "endlegend" {
                meta.legend = Some(legend_lines.join("\n"));
                legend_lines.clear();
                in_legend = false;
            } else {
                legend_lines.push(trimmed.to_string());
            }
            continue;
        }

        // End of multi-line note.
        if in_note {
            if trimmed == "end note" {
                let text = note_lines.join("\n").trim().to_string();
                if !text.is_empty() {
                    notes.push(ComponentNote {
                        text,
                        target: note_target.take(),
                    });
                }
                note_lines.clear();
                in_note = false;
            } else {
                note_lines.push(trimmed.to_string());
            }
            continue;
        }

        // Parse title directive — single-line form.
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
            continue;
        }
        // Multiline title: bare `title` on its own line.
        if trimmed == "title" {
            in_title = true;
            title_lines.clear();
            continue;
        }
        // Parse header directive.
        if let Some(rest) = trimmed.strip_prefix("header ") {
            meta.header = Some(rest.trim().to_string());
            continue;
        }
        // Parse footer directive.
        if let Some(rest) = trimmed.strip_prefix("footer ") {
            meta.footer = Some(rest.trim().to_string());
            continue;
        }
        // Parse legend block start: `legend`, `legend right`, `legend left`, etc.
        if trimmed == "legend" || trimmed.starts_with("legend ") {
            in_legend = true;
            legend_lines.clear();
            continue;
        }
        // Collect skinparam directives into metadata.
        if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            if let Some((key, value)) = rest.split_once(' ') {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            continue;
        }
        // Skip other decoration lines.
        if trimmed.starts_with("hide ")
            || trimmed.starts_with("show ")
            || trimmed.starts_with("caption ")
            || trimmed.starts_with("left footer")
            || trimmed.starts_with("right footer")
            || trimmed.starts_with("center footer")
            || trimmed.starts_with("left header")
            || trimmed.starts_with("right header")
            || trimmed.starts_with("center header")
        {
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
            let (container_url, container_clean) = super::extract_link_url(trimmed);
            if trimmed.contains('{') {
                // Block container — push onto the stack.
                let rest = &container_clean[kw.len()..];
                let (id, label) = parse_container_label(kw, rest);
                package_stack.push(ComponentPackage {
                    name: id,
                    label,
                    stereotype: parse_stereotypes(trimmed).into_iter().next(),
                    components: Vec::new(),
                    packages: Vec::new(),
                });
                continue;
            } else {
                // Leaf container declaration (no braces) — treat as a component.
                // e.g. `cloud "Production" as PROD`, `database "User DB" as UDB`
                let rest = &container_clean[kw.len()..];
                let (id, label) = parse_container_label(kw, rest);
                if !components.iter().any(|c: &Component| c.id == id) {
                    components.push(Component {
                        id: id.clone(),
                        label,
                        stereotypes: parse_stereotypes(trimmed),
                        url: container_url,
                    });
                }
                if let Some(pkg) = package_stack.last_mut()
                    && !pkg.components.contains(&id)
                {
                    pkg.components.push(id);
                }
                continue;
            }
        }

        // Note attached to an element: `note right of ID : text`
        if let Some(caps) = RE_NOTE_OF.captures(trimmed) {
            let target_raw = caps[1].to_string();
            // Strip brackets if present: `[ID]` → `ID`.
            let target = target_raw
                .trim_matches(|c| c == '[' || c == ']')
                .replace(' ', "_");
            if let Some(inline_text) = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .filter(|t| !t.is_empty())
            {
                notes.push(ComponentNote {
                    text: inline_text,
                    target: Some(target),
                });
            } else {
                note_target = Some(target);
                note_lines.clear();
                in_note = true;
            }
            continue;
        }
        // Floating inline note: `note "text" as ID`
        if let Some(caps) = RE_NOTE_INLINE.captures(trimmed) {
            notes.push(ComponentNote {
                text: caps[1].to_string(),
                target: None,
            });
            continue;
        }
        // `note on link : text` — inline note on the last link.
        if let Some(rest) = trimmed.strip_prefix("note on link") {
            let text = rest.trim_start_matches([' ', ':']).trim().to_string();
            if !text.is_empty() {
                notes.push(ComponentNote { text, target: None });
            } else {
                // Multi-line note on link.
                note_target = None;
                note_lines.clear();
                in_note = true;
            }
            continue;
        }
        // `note : text` — inline floating note.
        if let Some(rest) = trimmed
            .strip_prefix("note :")
            .or_else(|| trimmed.strip_prefix("note: "))
        {
            let text = rest.trim().to_string();
            if !text.is_empty() {
                notes.push(ComponentNote { text, target: None });
                continue;
            }
        }
        // Multi-line floating note: `note as ID` or plain `note`
        if trimmed.starts_with("note ") || trimmed == "note" {
            note_target = None;
            note_lines.clear();
            in_note = true;
            continue;
        }

        // Component declaration.
        let (comp_url, comp_clean) = super::extract_link_url(trimmed);
        if let Some(caps) = RE_COMP.captures(&comp_clean) {
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
                    stereotypes: parse_stereotypes(trimmed),
                    url: comp_url,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id);
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
                    stereotypes: parse_stereotypes(trimmed),
                    url: None,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id);
            }
            continue;
        }

        // Interface declarations.
        if let Some(caps) = RE_IFACE_QUOTED_AS.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            if !interfaces.iter().any(|i: &Interface| i.id == id) {
                interfaces.push(Interface { id, label });
            }
            continue;
        }
        if let Some(caps) = RE_IFACE_BRACKET_AS.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            if !interfaces.iter().any(|i: &Interface| i.id == id) {
                interfaces.push(Interface { id, label });
            }
            continue;
        }
        if let Some(caps) = RE_IFACE_BARE.captures(trimmed) {
            let name = caps[1].to_string();
            if !interfaces.iter().any(|i: &Interface| i.id == name) {
                interfaces.push(Interface {
                    id: name.clone(),
                    label: name,
                });
            }
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
            let dashed = arrow.contains("..") || arrow.contains('.');

            // Auto-create components from connection endpoints if not already
            // declared as a component or interface.
            for id in [&from, &to] {
                if !id.is_empty()
                    && !components.iter().any(|c| c.id == *id)
                    && !interfaces.iter().any(|i| i.id == *id)
                {
                    components.push(Component {
                        id: id.clone(),
                        label: id.clone(),
                        stereotypes: Vec::new(),
                        url: None,
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
        notes,
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
        let d = parse(
            "cloud Outer #LightBlue {\n  folder Inner {\n    component X\n    component Y\n    X --> Y\n  }\n}",
        );
        assert_eq!(d.packages.len(), 1, "should have 1 top-level package");
        assert_eq!(d.packages[0].label, "Outer");
        assert_eq!(
            d.packages[0].packages.len(),
            1,
            "should have 1 nested package"
        );
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

    #[test]
    fn bare_interface_parsed() {
        let d = parse("component Hub\ninterface IA\ninterface IB\nHub - IA\nHub - IB");
        assert_eq!(d.interfaces.len(), 2, "should have 2 interfaces");
        assert!(d.interfaces.iter().any(|i| i.id == "IA"));
        assert!(d.interfaces.iter().any(|i| i.id == "IB"));
    }

    #[test]
    fn multiple_stereotypes() {
        let d = parse("component Auth <<service>> <<secured>>");
        assert_eq!(d.components.len(), 1);
        assert!(d.components[0].stereotypes.contains(&"service".to_string()));
        assert!(d.components[0].stereotypes.contains(&"secured".to_string()));
    }

    #[test]
    fn note_right_of() {
        let d = parse("component MyComp <<facade>>\nnote right of MyComp : Tagged component");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Tagged component");
        assert_eq!(d.notes[0].target.as_deref(), Some("MyComp"));
    }

    #[test]
    fn multiline_title() {
        let d = parse("title\n  My Complex\n  Component Diagram\nend title\ncomponent A");
        assert_eq!(
            d.meta.title.as_deref(),
            Some("My Complex\nComponent Diagram")
        );
    }

    #[test]
    fn lollipop_arrow() {
        let d = parse("component Foo\ncomponent Bar\nFoo -(0- Bar : uses");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("uses"));
    }

    #[test]
    fn component_with_url() {
        let d = parse(
            "component Frontend [[https://example.com/frontend]]\ncomponent Backend [[https://example.com/backend]]\nFrontend --> Backend",
        );
        assert_eq!(d.components.len(), 2);
        assert_eq!(
            d.components[0].url.as_deref(),
            Some("https://example.com/frontend")
        );
        assert_eq!(
            d.components[1].url.as_deref(),
            Some("https://example.com/backend")
        );
    }

    #[test]
    fn database_with_url() {
        let d = parse("database Storage [[https://example.com/storage]]");
        assert_eq!(d.components.len(), 1);
        assert_eq!(
            d.components[0].url.as_deref(),
            Some("https://example.com/storage")
        );
    }
}
