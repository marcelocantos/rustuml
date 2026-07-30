// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::component::*;
use crate::diagram::{DiagramMeta, LegendHorizontalAlignment, LegendVerticalAlignment};

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
    "artifact",
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
        if trimmed
            .get(..kw.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(kw))
            && let Some(rest) = trimmed.get(kw.len()..)
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
    static RE_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\s*"([^"]+)"(?:\s+(?i:as)\s+(\w+))?(?:\s+[^{]*)?\{?"#).unwrap()
    });
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

fn component_package_kind(kw: &str) -> ComponentPackageKind {
    match kw {
        "cloud" => ComponentPackageKind::Cloud,
        "component" => ComponentPackageKind::Component,
        "database" | "storage" => ComponentPackageKind::Database,
        "folder" => ComponentPackageKind::Folder,
        "frame" => ComponentPackageKind::Frame,
        "node" => ComponentPackageKind::Node,
        "package" => ComponentPackageKind::Package,
        "queue" => ComponentPackageKind::Queue,
        "rectangle" | "boundary" | "control" | "entity" | "collections" | "actor" | "artifact" => {
            ComponentPackageKind::Rectangle
        }
        _ => ComponentPackageKind::Rectangle,
    }
}

fn parse_container_color(line: &str) -> Option<String> {
    line.split_whitespace()
        .find(|part| part.starts_with('#') && part.len() > 1)
        .map(|part| part.trim_end_matches('{').to_string())
}

fn parse_link_shape(arrow: &str) -> LinkShape {
    if arrow.contains("(0)-") {
        LinkShape::MiddleFullSocket
    } else if arrow.contains("(0-") {
        LinkShape::MiddleBallSocket
    } else if arrow.contains("(0") {
        LinkShape::TargetBallSocket
    } else if arrow.contains('(') {
        LinkShape::TargetSocket
    } else {
        LinkShape::Plain
    }
}

#[derive(Clone, Copy)]
enum ComponentBlock {
    Package,
    Together(usize),
}

fn active_together(blocks: &[ComponentBlock]) -> Option<usize> {
    match blocks.last() {
        Some(ComponentBlock::Together(index)) => Some(*index),
        Some(ComponentBlock::Package) | None => None,
    }
}

fn package_qualified_name(packages: &[ComponentPackage]) -> Option<String> {
    (!packages.is_empty()).then(|| {
        packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    })
}

fn add_together_node(together: &mut [ComponentTogether], blocks: &[ComponentBlock], node_id: &str) {
    if let Some(group) = active_together(blocks).and_then(|index| together.get_mut(index))
        && !group.nodes.iter().any(|member| member == node_id)
    {
        group.nodes.push(node_id.to_string());
    }
}

pub fn parse_component(lines: &[String]) -> Result<ComponentDiagram, ParseError> {
    let mut components = Vec::new();
    let mut interfaces = Vec::new();
    let mut connections = Vec::new();
    let mut notes: Vec<ComponentNote> = Vec::new();
    let mut together: Vec<ComponentTogether> = Vec::new();
    let mut meta = DiagramMeta::default();
    let mut direction = ComponentLayoutDirection::TopToBottom;

    // Parse into a nested structure via a stack.
    // Each stack frame is a mutable ComponentPackage under construction.
    let mut package_stack: Vec<ComponentPackage> = Vec::new();
    // Every brace-bearing construct gets a typed stack entry so closing a
    // together block cannot accidentally pop a visible package (or vice versa).
    let mut block_stack: Vec<ComponentBlock> = Vec::new();
    // Names of every block container ever opened, so a connection that targets
    // a container by name is not mistaken for an undeclared interface endpoint.
    let mut known_packages: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Ids of floating notes (`note "..." as N1`), so a `N1 .. Foo` link does
    // not auto-create N1 as an interface endpoint.
    let mut known_note_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Top-level packages collected.
    let mut top_packages: Vec<ComponentPackage> = Vec::new();
    // `hide` and `remove` have distinct SVEK lifecycles. Hidden elements still
    // participate in layout, whereas removed elements are excluded after
    // their declarations and links have consumed global UIDs.
    let mut hidden_ids: Vec<String> = Vec::new();
    let mut hidden_stereotypes: Vec<String> = Vec::new();
    let mut removed_ids: Vec<String> = Vec::new();
    let mut removed_stereotypes: Vec<String> = Vec::new();
    // Note buffer for multi-line notes.
    let mut note_target: Option<String> = None;
    let mut note_connection: Option<usize> = None;
    let mut note_position = ComponentNotePosition::Right;
    let mut note_source_line = 0;
    let mut note_lines: Vec<String> = Vec::new();
    let mut in_note: bool = false;
    // Multiline title accumulation.
    let mut in_title: bool = false;
    let mut title_lines: Vec<String> = Vec::new();
    // Legend block accumulation.
    let mut in_legend: bool = false;
    let mut legend_lines: Vec<String> = Vec::new();
    // Java `SkinLoader` concatenates the active group names with each leaf
    // key, so `skinparam component { BorderColor Red }` is stored as
    // `componentBorderColor`. The first entry is also the block sentinel:
    // an empty string represents a root `skinparam { ... }` block.
    let mut skinparam_context: Option<Vec<String>> = None;

    static RE_COMP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?i:component)\s+(?:"((?:[^"]|"")+)"\s+(?i:as)\s+(\w+)|"((?:[^"]|"")+)"|(\w+))(?:\s+[^{]*)?"#,
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
    // Lollipop interface shorthand: `() IFoo`, `() "Label"`, `() "Label" as ID`.
    // Matched as a standalone declaration only (no trailing arrow), so it must
    // be tried before RE_CONN, whose arrow class also contains `(`/`)`.
    static RE_IFACE_PAREN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\(\)\s+(?:"([^"]+)"|(\w+))(?:\s+as\s+(\w+))?\s*$"#).unwrap()
    });
    // Note: `note right of ID : text` or `note right of ID` (multiline)
    static RE_NOTE_OF: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^note\s+(right|left|top|bottom)\s+of\s+(\w+|\[[\w\s]+\])(?:\s*:\s*(.+))?$")
            .unwrap()
    });
    // Floating note: `note "text" as ID` or `note : text`
    static RE_NOTE_INLINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)"#).unwrap());
    // Matches: FROM ["from_mult"] ARROW ["to_mult"] TO [: label]
    // FROM and TO can be [bracket], "quoted label", or \w+ identifiers.
    // Group map: 1=from-bracket 2=from-quoted 3=from-word 4=from-mult
    // 5=arrow 6=to-mult 7=to-bracket 8=to-quoted 9=to-word 10=label.
    // Arrow chars broadened to include lollipop notation: `-(`, `-(0-`, `--(`  etc.
    static RE_CONN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(?:\[([^\]]+)\]|"([^"]+)"|(\w+))\s*(?:"([^"]*)")?\s*([-.<>()|~0#*o]+)\s*(?:"([^"]*)")?\s*(?:\[([^\]]+)\]|"([^"]+)"|(\w+))(?:\s*:\s*(.+))?$"#,
        )
        .unwrap()
    });

    for (line_idx, line) in lines.iter().enumerate() {
        let (current_line, trimmed) = super::source_line_and_trimmed(line_idx + 1, line);
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
                        connection: note_connection.take(),
                        position: note_position,
                        source_line: note_source_line,
                    });
                }
                note_lines.clear();
                in_note = false;
            } else {
                note_lines.push(trimmed.to_string());
            }
            continue;
        }

        if let Some(context) = &mut skinparam_context {
            if trimmed == "}" {
                context.pop();
                if context.is_empty() {
                    skinparam_context = None;
                }
                continue;
            }
            if let Some(group) = trimmed.strip_suffix('{').map(str::trim)
                && !group.is_empty()
            {
                context.push(group.to_string());
                continue;
            }
            if let Some((key, value)) = trimmed.split_once(char::is_whitespace) {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: format!("{}{key}", context.concat()),
                    value: value.trim().to_string(),
                });
            }
            continue;
        }

        // Parse title directive — single-line form.
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
            meta.title_line = Some(current_line);
            continue;
        }
        // Multiline title: bare `title` on its own line.
        if trimmed == "title" {
            meta.title_line = Some(current_line);
            in_title = true;
            title_lines.clear();
            continue;
        }
        // Parse header directive.
        if let Some(rest) = trimmed.strip_prefix("header ") {
            meta.header = Some(rest.trim().to_string());
            meta.header_line = Some(current_line);
            continue;
        }
        // Parse footer directive.
        if let Some(rest) = trimmed.strip_prefix("footer ") {
            meta.footer = Some(rest.trim().to_string());
            meta.footer_line = Some(current_line);
            continue;
        }
        // Parse legend block start: `legend`, `legend right`, `legend left`, etc.
        if trimmed == "legend" || trimmed.starts_with("legend ") {
            let words: Vec<_> = trimmed.split_ascii_whitespace().collect();
            meta.legend_line = Some(current_line);
            meta.legend_horizontal_alignment = if words.contains(&"left") {
                LegendHorizontalAlignment::Left
            } else if words.contains(&"right") {
                LegendHorizontalAlignment::Right
            } else {
                LegendHorizontalAlignment::Center
            };
            meta.legend_vertical_alignment = if words.contains(&"top") {
                LegendVerticalAlignment::Top
            } else {
                LegendVerticalAlignment::Bottom
            };
            in_legend = true;
            legend_lines.clear();
            continue;
        }
        // Collect skinparam directives into metadata.
        if trimmed == "left to right direction" {
            direction = ComponentLayoutDirection::LeftToRight;
            continue;
        }
        if trimmed == "top to bottom direction" {
            direction = ComponentLayoutDirection::TopToBottom;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            if let Some(group) = rest.strip_suffix('{').map(str::trim) {
                skinparam_context = Some(vec![group.to_string()]);
                continue;
            }
            if let Some((key, value)) = rest.split_once(char::is_whitespace) {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            continue;
        }
        // Element and stereotype selectors are resolved after parsing because
        // PlantUML commands can precede their declarations.
        let suppression = trimmed
            .strip_prefix("hide ")
            .map(|arg| (arg, false))
            .or_else(|| trimmed.strip_prefix("remove ").map(|arg| (arg, true)));
        if let Some((arg, is_remove)) = suppression {
            let arg = arg.trim();
            if let Some(stereo) = arg.strip_prefix("<<").and_then(|s| s.strip_suffix(">>")) {
                let stereotypes = if is_remove {
                    &mut removed_stereotypes
                } else {
                    &mut hidden_stereotypes
                };
                stereotypes.push(stereo.trim().to_string());
                continue;
            }
            // A bare identifier (optionally bracketed `[Name]`) names an element.
            let id = arg.trim_matches(|c| c == '[' || c == ']');
            const DISPLAY_HINTS: &[&str] = &[
                "stereotype",
                "stereotypes",
                "empty",
                "members",
                "methods",
                "fields",
                "attributes",
                "circle",
                "footbox",
                "unlinked",
            ];
            let first_word = id.split_whitespace().next().unwrap_or("");
            if !id.is_empty()
                && !id.contains(char::is_whitespace)
                && !DISPLAY_HINTS.contains(&first_word)
            {
                let ids = if is_remove {
                    &mut removed_ids
                } else {
                    &mut hidden_ids
                };
                ids.push(id.replace(' ', "_"));
            }
            continue;
        }

        // Skip other decoration lines.
        if trimmed.starts_with("show ")
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

        // Java `CucaDiagram.gotoTogether` pushes a Together whose parent is the
        // currently active Together. `Cluster.printTogether` later emits these
        // as nested cluster-prefixed dot subgraphs.
        if trimmed == "together {" || trimmed == "together{" {
            let index = together.len();
            together.push(ComponentTogether {
                parent: active_together(&block_stack),
                package: package_qualified_name(&package_stack),
                nodes: Vec::new(),
                packages: Vec::new(),
            });
            block_stack.push(ComponentBlock::Together(index));
            continue;
        }

        // Closing brace — pop the stack.
        if trimmed == "}" {
            if matches!(block_stack.pop(), Some(ComponentBlock::Package))
                && let Some(finished) = package_stack.pop()
            {
                if let Some(parent_package) = package_stack.last_mut() {
                    parent_package.packages.push(finished);
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
                known_packages.insert(id.clone());
                let qname = package_qualified_name(&package_stack)
                    .map(|parent| format!("{parent}.{id}"))
                    .unwrap_or_else(|| id.clone());
                if let Some(group) =
                    active_together(&block_stack).and_then(|index| together.get_mut(index))
                {
                    group.packages.push(qname);
                }
                package_stack.push(ComponentPackage {
                    name: id,
                    label,
                    stereotype: parse_stereotypes(trimmed).into_iter().next(),
                    kind: component_package_kind(kw),
                    color: parse_container_color(trimmed),
                    source_line: current_line,
                    components: Vec::new(),
                    packages: Vec::new(),
                });
                block_stack.push(ComponentBlock::Package);
                continue;
            } else {
                // A leaf `component ...` declaration has richer syntax than other
                // container-shaped elements, including quoted labels that contain
                // doubled quotes for Creole markup. Let the dedicated component
                // declaration parser below own it. Block-form `component Foo { ... }`
                // is still handled above.
                if kw != "component" {
                    // Leaf container declaration (no braces) — treat as a component.
                    // e.g. `cloud "Production" as PROD`, `database "User DB" as UDB`
                    let rest = &container_clean[kw.len()..];
                    let (id, label) = parse_container_label(kw, rest);
                    let kind = match kw {
                        "actor" => ComponentElementKind::Actor,
                        "artifact" => ComponentElementKind::Artifact,
                        "collections" => ComponentElementKind::Collections,
                        "database" => ComponentElementKind::Database,
                        "node" => ComponentElementKind::Node,
                        "queue" => ComponentElementKind::Queue,
                        "storage" => ComponentElementKind::Storage,
                        "cloud" => ComponentElementKind::Cloud,
                        _ => ComponentElementKind::Component,
                    };
                    if !components.iter().any(|c: &Component| c.id == id) {
                        components.push(Component {
                            id: id.clone(),
                            label,
                            stereotypes: parse_stereotypes(trimmed),
                            color: parse_container_color(trimmed),
                            url: container_url,
                            source_line: current_line,
                            kind,
                        });
                    }
                    if let Some(pkg) = package_stack.last_mut()
                        && !pkg.components.contains(&id)
                    {
                        pkg.components.push(id.clone());
                    }
                    add_together_node(&mut together, &block_stack, &id);
                    continue;
                }
            }
        }

        // Note attached to an element: `note right of ID : text`
        if let Some(caps) = RE_NOTE_OF.captures(trimmed) {
            let position = match &caps[1] {
                "top" => ComponentNotePosition::Top,
                "bottom" => ComponentNotePosition::Bottom,
                "left" => ComponentNotePosition::Left,
                _ => ComponentNotePosition::Right,
            };
            let target_raw = caps[2].to_string();
            // Strip brackets if present: `[ID]` → `ID`.
            let target = target_raw
                .trim_matches(|c| c == '[' || c == ']')
                .replace(' ', "_");
            if let Some(inline_text) = caps
                .get(3)
                .map(|m| m.as_str().trim().to_string())
                .filter(|t| !t.is_empty())
            {
                notes.push(ComponentNote {
                    text: inline_text,
                    target: Some(target),
                    connection: None,
                    position,
                    source_line: current_line,
                });
            } else {
                note_target = Some(target);
                note_connection = None;
                note_position = position;
                // `CommandFactoryNoteOnEntity.createMultiLine` creates the
                // entity at the body `BlocLines` location after removing the
                // opening command line.
                note_source_line = current_line + 1;
                note_lines.clear();
                in_note = true;
            }
            continue;
        }
        // Floating inline note: `note "text" as ID`
        if let Some(caps) = RE_NOTE_INLINE.captures(trimmed) {
            known_note_ids.insert(caps[2].to_string());
            notes.push(ComponentNote {
                text: caps[1].to_string(),
                target: None,
                connection: None,
                position: ComponentNotePosition::Right,
                source_line: current_line,
            });
            continue;
        }
        // `note on link : text` — inline note on the last link.
        if let Some(rest) = trimmed.strip_prefix("note on link") {
            let Some(connection) = connections.len().checked_sub(1) else {
                continue;
            };
            let text = rest.trim_start_matches([' ', ':']).trim().to_string();
            if !text.is_empty() {
                notes.push(ComponentNote {
                    text,
                    target: None,
                    connection: Some(connection),
                    position: ComponentNotePosition::Bottom,
                    source_line: current_line,
                });
            } else {
                // Multi-line note on link.
                note_target = None;
                note_connection = Some(connection);
                note_position = ComponentNotePosition::Bottom;
                // `CommandFactoryNoteOnLink.createMultiLine` creates its note
                // component from the body `BlocLines`, after the command.
                note_source_line = current_line + 1;
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
                notes.push(ComponentNote {
                    text,
                    target: None,
                    connection: None,
                    position: ComponentNotePosition::Right,
                    source_line: current_line,
                });
                continue;
            }
        }
        // Multi-line floating note: `note as ID` or plain `note`
        if trimmed.starts_with("note ") || trimmed == "note" {
            note_target = None;
            note_connection = None;
            note_position = ComponentNotePosition::Right;
            note_source_line = current_line;
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
                    color: parse_container_color(trimmed),
                    url: comp_url,
                    source_line: current_line,
                    kind: ComponentElementKind::Component,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id.clone());
            }
            add_together_node(&mut together, &block_stack, &id);
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
                    color: None,
                    url: None,
                    source_line: current_line,
                    kind: ComponentElementKind::Component,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id.clone());
            }
            add_together_node(&mut together, &block_stack, &id);
            continue;
        }

        // Interface declarations.
        if let Some(caps) = RE_IFACE_QUOTED_AS.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            if !interfaces.iter().any(|i: &Interface| i.id == id) {
                interfaces.push(Interface {
                    id: id.clone(),
                    label,
                    source_line: current_line,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id.clone());
            }
            add_together_node(&mut together, &block_stack, &id);
            continue;
        }
        if let Some(caps) = RE_IFACE_BRACKET_AS.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            // PlantUML `CommandCreateElementFull.executeArg` selects
            // `USymbolComponent2` whenever either the code or display starts
            // with `[`. The `interface` keyword does not override that symbol.
            if !components.iter().any(|c: &Component| c.id == id) {
                components.push(Component {
                    id: id.clone(),
                    label,
                    stereotypes: parse_stereotypes(trimmed),
                    color: parse_container_color(trimmed),
                    url: None,
                    source_line: current_line,
                    kind: ComponentElementKind::Component,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id.clone());
            }
            add_together_node(&mut together, &block_stack, &id);
            continue;
        }
        if let Some(caps) = RE_IFACE_BARE.captures(trimmed) {
            let name = caps[1].to_string();
            if !interfaces.iter().any(|i: &Interface| i.id == name) {
                interfaces.push(Interface {
                    id: name.clone(),
                    label: name.clone(),
                    source_line: current_line,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&name)
            {
                pkg.components.push(name.clone());
            }
            add_together_node(&mut together, &block_stack, &name);
            continue;
        }
        // `() IFoo` / `() "Label" as ID` — lollipop interface shorthand.
        if let Some(caps) = RE_IFACE_PAREN.captures(trimmed) {
            let label = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let id = caps
                .get(3)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| label.clone());
            if !interfaces.iter().any(|i: &Interface| i.id == id) {
                interfaces.push(Interface {
                    id: id.clone(),
                    label,
                    source_line: current_line,
                });
            }
            if let Some(pkg) = package_stack.last_mut()
                && !pkg.components.contains(&id)
            {
                pkg.components.push(id.clone());
            }
            add_together_node(&mut together, &block_stack, &id);
            continue;
        }

        // Strip embedded direction tokens (`-down-`, `-up->`, `-up.>`,
        // `<-left-`, …) so the arrow falls within the character class used
        // by RE_CONN. Direction is purely a layout hint in PlantUML; the
        // connection structure is unchanged.
        static RE_DIR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"-(down|up|left|right)([-.>])").unwrap());
        let explicit_direction = RE_DIR.captures(trimmed).and_then(|caps| match &caps[1] {
            "down" => Some(ConnectionDirection::Down),
            "up" => Some(ConnectionDirection::Up),
            "left" => Some(ConnectionDirection::Left),
            "right" => Some(ConnectionDirection::Right),
            _ => None,
        });
        let trimmed_owned = RE_DIR.replace_all(trimmed, "-$2").to_string();
        let trimmed = trimmed_owned.as_str();

        if let Some(caps) = RE_CONN.captures(trimmed) {
            // Group map (see RE_CONN): 1/7 bracketed (`[Name]`), 2/8 quoted
            // (`"Name"`), 3/9 bare word. PlantUML treats a *bare*, undeclared
            // endpoint as an interface (drawn as a circle); bracketed or
            // quoted endpoints are components. Quoted endpoints keep their
            // spaces (they reference a declared `component "Name"`); bare
            // identifiers have spaces normalised to underscores.
            let from_bracketed = caps.get(1).is_some();
            let from_quoted = caps.get(2).is_some();
            let from = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .or_else(|| caps.get(3).map(|m| m.as_str().replace(' ', "_")))
                .unwrap_or_default();
            let from_mult = caps.get(4).map(|m| m.as_str().to_string());
            let arrow = &caps[5];
            let to_mult = caps.get(6).map(|m| m.as_str().to_string());
            let to_bracketed = caps.get(7).is_some();
            let to_quoted = caps.get(8).is_some();
            let to = caps
                .get(7)
                .or(caps.get(8))
                .map(|m| m.as_str().to_string())
                .or_else(|| caps.get(9).map(|m| m.as_str().replace(' ', "_")))
                .unwrap_or_default();
            // PlantUML `descdiagram.command.Labels.init` normalizes a
            // relationship's center label before `StringWithArrow` receives
            // it, removing one matching pair of surrounding double quotes.
            let label = caps.get(10).map(|m| {
                super::strip_title_quotes(m.as_str().trim())
                    .trim()
                    .to_string()
            });
            let dashed = arrow.contains("..") || arrow.contains('.');
            let arrow_at_start = arrow.contains('<');
            let arrow_at_end = arrow.contains('>');
            let extension_at_start = arrow.contains("<|");
            let extension_at_end = arrow.contains("|>");
            let has_arrow = arrow_at_start || arrow_at_end;
            let shape = parse_link_shape(arrow);
            let queue_length = arrow
                .chars()
                .filter(|character| matches!(character, '-' | '.' | '~' | '='))
                .count();
            // PlantUML `StringUtils.getQueueDirection` treats a one-character
            // body as RIGHT and every longer body as DOWN. CommandLinkElement
            // then normalizes explicit horizontal links to length one.
            let direction = explicit_direction.or(Some(if queue_length == 1 {
                ConnectionDirection::Right
            } else {
                ConnectionDirection::Down
            }));
            let length = if matches!(
                direction,
                Some(ConnectionDirection::Left | ConnectionDirection::Right)
            ) {
                1
            } else {
                queue_length.max(2)
            };

            // Auto-create endpoints if not already declared. Bracketed and
            // quoted endpoints become components; bare ones become interfaces.
            for (id, bracketed) in [
                (&from, from_bracketed || from_quoted),
                (&to, to_bracketed || to_quoted),
            ] {
                let mut created = false;
                if !id.is_empty()
                    && !components.iter().any(|c| c.id == *id)
                    && !interfaces.iter().any(|i| i.id == *id)
                    && !known_packages.contains(id)
                    && !known_note_ids.contains(id)
                {
                    if bracketed {
                        components.push(Component {
                            id: id.clone(),
                            label: id.clone(),
                            stereotypes: Vec::new(),
                            color: None,
                            url: None,
                            source_line: current_line,
                            kind: ComponentElementKind::Component,
                        });
                        created = true;
                    } else {
                        interfaces.push(Interface {
                            id: id.clone(),
                            label: id.clone(),
                            source_line: current_line,
                        });
                        created = true;
                    }
                }
                if created {
                    if let Some(pkg) = package_stack.last_mut()
                        && !pkg.components.contains(id)
                    {
                        pkg.components.push(id.clone());
                    }
                    add_together_node(&mut together, &block_stack, id);
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
                    has_arrow,
                    arrow_at_start,
                    arrow_at_end,
                    extension_at_start,
                    extension_at_end,
                    direction,
                    length,
                    shape,
                    source_line: current_line,
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

    let mut hidden: std::collections::HashSet<String> = hidden_ids.into_iter().collect();
    let mut removed: std::collections::HashSet<String> = removed_ids.into_iter().collect();
    for component in &components {
        if component
            .stereotypes
            .iter()
            .any(|stereotype| hidden_stereotypes.iter().any(|hidden| hidden == stereotype))
        {
            hidden.insert(component.id.clone());
        }
        if component.stereotypes.iter().any(|stereotype| {
            removed_stereotypes
                .iter()
                .any(|removed| removed == stereotype)
        }) {
            removed.insert(component.id.clone());
        }
    }

    // `GraphvizImageBuilder.printEntities` and its link loop skip removed
    // objects, but the objects already exist and retain their global UID
    // positions. Preserve those objects separately from the active graph.
    let mut removed_components = Vec::new();
    components.retain(|component| {
        if removed.contains(&component.id) {
            removed_components.push(component.clone());
            false
        } else {
            true
        }
    });
    let hidden_components = components
        .iter()
        .filter(|component| hidden.contains(&component.id))
        .map(|component| component.id.clone())
        .collect();

    let mut removed_connections = Vec::new();
    connections.retain(|connection| {
        if removed.contains(&connection.from) || removed.contains(&connection.to) {
            removed_connections.push(connection.clone());
            false
        } else {
            true
        }
    });
    let mut removed_notes = Vec::new();
    notes.retain(|note| {
        if note
            .target
            .as_ref()
            .is_some_and(|target| removed.contains(target))
        {
            removed_notes.push(note.clone());
            false
        } else {
            true
        }
    });

    if !removed.is_empty() {
        for group in &mut together {
            group.nodes.retain(|id| !removed.contains(id));
        }
        fn prune_pkg(pkg: &mut ComponentPackage, removed: &std::collections::HashSet<String>) {
            pkg.components.retain(|id| !removed.contains(id));
            for child in &mut pkg.packages {
                prune_pkg(child, removed);
            }
        }
        for pkg in &mut top_packages {
            prune_pkg(pkg, &removed);
        }
    }

    Ok(ComponentDiagram {
        meta,
        direction,
        components,
        hidden_components,
        removed_components,
        interfaces,
        connections,
        removed_connections,
        packages: top_packages,
        together,
        notes,
        removed_notes,
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
    fn preserves_global_layout_direction_and_allows_reset() {
        let left_to_right = parse(
            "left to right direction\ncomponent RenamedOne\ncomponent RenamedTwo\nRenamedOne --> RenamedTwo",
        );
        assert_eq!(
            left_to_right.direction,
            ComponentLayoutDirection::LeftToRight
        );

        let reset = parse(
            "left to right direction\ntop to bottom direction\ncomponent RenamedOne\ncomponent RenamedTwo",
        );
        assert_eq!(reset.direction, ComponentLayoutDirection::TopToBottom);
    }

    #[test]
    fn preserves_extension_decorations_at_both_link_ends() {
        let diagram = parse(
            "interface RenamedPort\ncomponent FreshAdapter\nFreshAdapter ..|> RenamedPort\nRenamedPort <|.. FreshAdapter",
        );

        assert!(diagram.connections[0].extension_at_end);
        assert!(!diagram.connections[0].extension_at_start);
        assert!(diagram.connections[1].extension_at_start);
        assert!(!diagram.connections[1].extension_at_end);
    }

    #[test]
    fn basic_components() {
        let d = parse("component \"Web\" as WS\ncomponent \"DB\" as DB\nWS --> DB : query");
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].label.as_deref(), Some("query"));
    }

    #[test]
    fn component_quoted_label_with_creole_monospace() {
        let d = parse(
            "component \"\"\"mono\"\" comp\" as C155\ninterface \"\"\"mono\"\" iface\" as I155\nC155 -- I155",
        );
        assert_eq!(d.components.len(), 1);
        assert_eq!(d.components[0].id, "C155");
        assert_eq!(d.components[0].label, "\"\"mono\"\" comp");
        assert_eq!(d.interfaces.len(), 1);
        assert_eq!(d.interfaces[0].id, "I155");
        assert_eq!(d.interfaces[0].label, "\"\"mono\"\" iface");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].from, "C155");
        assert_eq!(d.connections[0].to, "I155");
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
    fn component_leaf_color() {
        let d = parse("component Provider #LightBlue\ncomponent Consumer #Orange");
        assert_eq!(d.components[0].color.as_deref(), Some("#LightBlue"));
        assert_eq!(d.components[1].color.as_deref(), Some("#Orange"));
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
    fn together_tracks_members_package_owner_and_nested_parent() {
        let d = parse(
            "package Outer {\n  together {\n    component A\n    together {\n      component B\n    }\n  }\n  component C\n}\ntogether {\n  package Child {\n    component D\n  }\n}",
        );

        assert_eq!(d.together.len(), 3);
        assert_eq!(d.together[0].package.as_deref(), Some("Outer"));
        assert_eq!(d.together[0].parent, None);
        assert_eq!(d.together[0].nodes, ["A"]);
        assert_eq!(d.together[1].package.as_deref(), Some("Outer"));
        assert_eq!(d.together[1].parent, Some(0));
        assert_eq!(d.together[1].nodes, ["B"]);
        assert_eq!(d.together[2].package, None);
        assert_eq!(d.together[2].packages, ["Child"]);
        assert_eq!(d.packages[0].components, ["A", "B", "C"]);
        assert_eq!(d.packages[1].components, ["D"]);
    }

    #[test]
    fn bare_interface_parsed() {
        let d = parse("component Hub\ninterface IA\ninterface IB\nHub - IA\nHub - IB");
        assert_eq!(d.interfaces.len(), 2, "should have 2 interfaces");
        assert_eq!(d.interfaces[0].id, "IA");
        assert_eq!(d.interfaces[0].source_line, 2);
        assert_eq!(d.interfaces[1].id, "IB");
        assert_eq!(d.interfaces[1].source_line, 3);
    }

    #[test]
    fn renamed_quoted_link_label_is_normalized_like_java_labels_init() {
        let d = parse(
            "component RenamedIngress9803\n\
             component RenamedArchive9811\n\
             RenamedIngress9803 --> RenamedArchive9811 : \"renamed-link-9817\"",
        );

        assert_eq!(d.connections[0].label.as_deref(), Some("renamed-link-9817"));
    }

    #[test]
    fn renamed_legend_keeps_alignment_and_source_line() {
        let d = parse(
            "component RenamedIngress9803\n\
             legend top right\n\
               | Signal 9817 | Meaning 9829 |\n\
             endlegend",
        );

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
    fn interface_declarations_keep_their_renamed_container_owner() {
        let d = parse(
            "component RenamedShell9701 {\n\
               interface \"Quoted Port 9703\" as Quoted9703\n\
               interface [Bracket Port 9709] as Bracket9709\n\
               interface BarePort9719\n\
               () \"Parenthesized Port 9721\" as Paren9721\n\
             }",
        );

        assert_eq!(d.packages.len(), 1);
        assert_eq!(
            d.packages[0].components,
            ["Quoted9703", "Bracket9709", "BarePort9719", "Paren9721"]
        );
        assert!(
            d.components.iter().any(|component| {
                component.id == "Bracket9709" && component.label == "Bracket Port 9709"
            }),
            "bracketed interface syntax selects PlantUML's component symbol"
        );
        assert!(
            !d.interfaces
                .iter()
                .any(|interface| interface.id == "Bracket9709")
        );
    }

    #[test]
    fn implicit_interface_keeps_connection_creation_line() {
        let d = parse("component Relay71\nRelay71 - AuditPort73");
        assert_eq!(d.interfaces.len(), 1);
        assert_eq!(d.interfaces[0].id, "AuditPort73");
        assert_eq!(d.interfaces[0].source_line, 2);
    }

    #[test]
    fn multiple_stereotypes() {
        let d = parse("component Auth <<service>> <<secured>>");
        assert_eq!(d.components.len(), 1);
        assert!(d.components[0].stereotypes.contains(&"service".to_string()));
        assert!(d.components[0].stereotypes.contains(&"secured".to_string()));
    }

    #[test]
    fn hide_and_remove_preserve_distinct_component_lifecycles() {
        let d = parse(
            "component \"Renamed Hidden Relay 9101\" as Hidden9101 <<retired_9101>>\n\
             component \"Renamed Visible Broker 9103\" as Broker9103\n\
             component \"Renamed Removed Sink 9109\" as Removed9109\n\
             component \"Renamed Visible Archive 9113\" as Archive9113\n\
             Hidden9101 --> Broker9103\n\
             Broker9103 --> Removed9109\n\
             Broker9103 --> Archive9113\n\
             hide <<retired_9101>>\n\
             remove Removed9109",
        );

        assert_eq!(d.hidden_components, ["Hidden9101"]);
        assert_eq!(
            d.components
                .iter()
                .map(|component| component.id.as_str())
                .collect::<Vec<_>>(),
            ["Hidden9101", "Broker9103", "Archive9113"]
        );
        assert_eq!(d.removed_components.len(), 1);
        assert_eq!(d.removed_components[0].id, "Removed9109");
        assert_eq!(d.connections.len(), 2);
        assert_eq!(d.removed_connections.len(), 1);
        assert_eq!(d.removed_connections[0].to, "Removed9109");
    }

    #[test]
    fn grouped_skinparams_follow_java_context_concatenation() {
        let d = parse(
            "skinparam component {\n  BackgroundColor<<relay_71>> PaleGreen\n  BorderStyle dashed\n  arrow {\n    FontColor Navy\n  }\n}\ncomponent Relay71 <<relay_71>>",
        );
        let params: Vec<_> = d
            .meta
            .skinparams
            .iter()
            .map(|param| (param.key.as_str(), param.value.as_str()))
            .collect();

        assert_eq!(
            params,
            [
                ("componentBackgroundColor<<relay_71>>", "PaleGreen"),
                ("componentBorderStyle", "dashed"),
                ("componentarrowFontColor", "Navy"),
            ]
        );
        assert_eq!(d.components[0].id, "Relay71");
    }

    #[test]
    fn note_right_of() {
        let d = parse("component MyComp <<facade>>\nnote right of MyComp : Tagged component");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Tagged component");
        assert_eq!(d.notes[0].target.as_deref(), Some("MyComp"));
        assert_eq!(d.notes[0].position, ComponentNotePosition::Right);
        assert_eq!(d.notes[0].source_line, 2);
    }

    #[test]
    fn attached_note_preserves_side_and_source_for_renamed_target() {
        let d = parse(
            "component \"Renamed Relay 71\" as Relay71\nnote left of Relay71\n  A fresh perturbation\nend note",
        );

        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "A fresh perturbation");
        assert_eq!(d.notes[0].target.as_deref(), Some("Relay71"));
        assert_eq!(d.notes[0].position, ComponentNotePosition::Left);
        assert_eq!(d.notes[0].source_line, 3);
    }

    #[test]
    fn renamed_link_note_preserves_connection_ownership() {
        let d = parse(
            "component \"Ingress Relay 941\" as Ingress941\n\
             component \"Archive Sink 947\" as Archive947\n\
             Ingress941 --> Archive947 : streams batches\n\
             note on link\n\
               Validates renamed stream 953\n\
               before durable handoff 967\n\
             end note",
        );

        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].connection, Some(0));
        assert_eq!(d.notes[0].target, None);
        assert_eq!(d.notes[0].position, ComponentNotePosition::Bottom);
        assert_eq!(d.notes[0].source_line, 5);
        assert_eq!(
            d.notes[0].text,
            "Validates renamed stream 953\nbefore durable handoff 967"
        );
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
        assert!(!d.connections[0].has_arrow);
        assert_eq!(d.connections[0].shape, LinkShape::MiddleBallSocket);
    }

    #[test]
    fn preserves_arrow_decorations_at_both_connection_ends() {
        let d = parse(
            "component RenamedSource\ncomponent RenamedTarget\nRenamedSource <..> RenamedTarget",
        );

        let connection = &d.connections[0];
        assert!(connection.has_arrow);
        assert!(connection.arrow_at_start);
        assert!(connection.arrow_at_end);
        assert!(connection.dashed);
    }

    #[test]
    fn preserves_explicit_connection_layout_directions() {
        let d = parse(
            "component A\ncomponent B\ncomponent C\ncomponent D\nA -up- B\nA -down-> C\nA <-left- D\nA -right- B",
        );

        assert_eq!(d.connections[0].direction, Some(ConnectionDirection::Up));
        assert_eq!(d.connections[1].direction, Some(ConnectionDirection::Down));
        assert_eq!(d.connections[2].direction, Some(ConnectionDirection::Left));
        assert!(d.connections[2].arrow_at_start);
        assert_eq!(d.connections[3].direction, Some(ConnectionDirection::Right));
    }

    #[test]
    fn derives_connection_direction_and_length_from_arrow_body() {
        let d = parse(
            "component A17\ncomponent B23\ncomponent C29\ncomponent D31\nA17 -> B23\nA17 --> C29\nA17 ---> D31\nB23 .-. D31",
        );

        assert_eq!(
            (d.connections[0].direction, d.connections[0].length),
            (Some(ConnectionDirection::Right), 1)
        );
        assert_eq!(
            (d.connections[1].direction, d.connections[1].length),
            (Some(ConnectionDirection::Down), 2)
        );
        assert_eq!(
            (d.connections[2].direction, d.connections[2].length),
            (Some(ConnectionDirection::Down), 3)
        );
        assert_eq!(
            (d.connections[3].direction, d.connections[3].length),
            (Some(ConnectionDirection::Down), 3)
        );
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

    #[test]
    fn artifact_and_node_are_leaf_components() {
        let d = parse("artifact Build\nnode Server\nBuild --> Server");
        assert_eq!(d.interfaces.len(), 0);
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.components[0].kind, ComponentElementKind::Artifact);
        assert_eq!(d.components[1].kind, ComponentElementKind::Node);
    }

    #[test]
    fn queue_and_storage_keep_distinct_leaf_symbols() {
        let d = parse("queue Buffer\nstorage Archive\nBuffer --> Archive");
        assert_eq!(d.interfaces.len(), 0);
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.components[0].kind, ComponentElementKind::Queue);
        assert_eq!(d.components[1].kind, ComponentElementKind::Storage);
    }

    #[test]
    fn shared_description_commands_are_case_insensitive() {
        let d = parse(
            "PaCkAgE Outer {\n\
               ClOuD \"Shared Cloud\" AS Shared {\n\
                 QuEuE \"Delivery Work\" As DeliveryQueue\n\
               }\n\
             }",
        );
        assert_eq!(d.packages[0].name, "Outer");
        assert_eq!(d.packages[0].packages[0].name, "Shared");
        let queue = d
            .components
            .iter()
            .find(|component| component.id == "DeliveryQueue")
            .unwrap();
        assert_eq!(queue.label, "Delivery Work");
        assert_eq!(queue.kind, ComponentElementKind::Queue);
    }

    #[test]
    fn component_leaf_command_and_as_token_are_case_insensitive() {
        let d = parse(
            "CoMpOnEnT \"Telemetry Display\" As Telemetry8151\n\
             COMPONENT Api8161",
        );

        assert_eq!(d.components.len(), 2);
        assert_eq!(d.components[0].id, "Telemetry8151");
        assert_eq!(d.components[0].label, "Telemetry Display");
        assert_eq!(d.components[1].id, "Api8161");
        assert_eq!(d.components[1].label, "Api8161");
    }

    #[test]
    fn actor_and_collections_are_leaf_components() {
        let d = parse("actor User\ncollections Cache\nUser --> Cache");
        assert_eq!(d.interfaces.len(), 0);
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.components[0].kind, ComponentElementKind::Actor);
        assert_eq!(d.components[1].kind, ComponentElementKind::Collections);
    }
}
