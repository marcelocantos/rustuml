// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Archimate diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::archimate::*;

pub fn parse_archimate(lines: &[String]) -> Result<ArchimateDiagram, ParseError> {
    let mut elements = Vec::new();
    let mut relations = Vec::new();
    let mut groups: Vec<ArchimateGroup> = Vec::new();
    let mut meta = DiagramMeta::default();
    let mut group_stack: Vec<(String, usize, Vec<String>)> = Vec::new();
    // Java `SkinLoader` concatenates nested block names with leaf keys.
    let mut skinparam_context: Option<Vec<String>> = None;

    static RE_ELEM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^archimate_element\s+(\w+)\s+(\w+)\s+(\w+)\s+"([^"]*)""#).unwrap()
    });
    static RE_REL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^archimate_rel\s+(\S+)\s+(\w+)\s+(\w+)\s+"([^"]*)""#).unwrap()
    });
    static RE_GROUP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^rectangle\s+"([^"]+)"\s*\{"#).unwrap());
    static RE_ARROW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(\w+)\s+(-+>|\.+>|--+>|\.\.+>)\s+(\w+)(?:\s*:\s*(.+))?$"#).unwrap()
    });

    for (i, line) in lines.iter().enumerate() {
        let (source_line, trimmed) = super::source_line_and_trimmed(i + 1, line);
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
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
        if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            if let Some(group) = rest.strip_suffix('{').map(str::trim) {
                skinparam_context = Some(vec![group.to_string()]);
            } else if let Some((key, value)) = rest.split_once(' ') {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            continue;
        }
        if trimmed.starts_with("hide ") || trimmed.starts_with("show ") {
            continue;
        }
        if trimmed == "}" {
            if let Some((label, source_line, element_ids)) = group_stack.pop() {
                groups.push(ArchimateGroup {
                    label,
                    source_line,
                    element_ids,
                });
            }
            continue;
        }
        if let Some(caps) = RE_GROUP.captures(trimmed) {
            group_stack.push((caps[1].to_string(), source_line, Vec::new()));
            continue;
        }
        if let Some(caps) = RE_ELEM.captures(trimmed) {
            let layer: ArchimateLayer = caps[1].parse().unwrap_or(ArchimateLayer::Other);
            let kind = caps[2].to_string();
            let id = caps[3].to_string();
            let label = caps[4].to_string();
            if !elements.iter().any(|e: &ArchimateElement| e.id == id) {
                elements.push(ArchimateElement {
                    id: id.clone(),
                    label,
                    source_line,
                    layer,
                    kind,
                });
            }
            if let Some((_, _, ids)) = group_stack.last_mut() {
                ids.push(id);
            }
            continue;
        }
        if let Some(caps) = RE_REL.captures(trimmed) {
            let (rel_kind, direction) = ArchimateRelationKind::parse_name(&caps[1]);
            let from = caps[2].to_string();
            let to = caps[3].to_string();
            let label_str = caps[4].trim().to_string();
            let label = if label_str.is_empty() {
                None
            } else {
                Some(label_str)
            };
            relations.push(ArchimateRelation {
                from,
                to,
                label,
                source_line,
                kind: rel_kind,
                direction,
            });
            continue;
        }
        if let Some(caps) = RE_ARROW.captures(trimmed) {
            let from = caps[1].to_string();
            let to = caps[3].to_string();
            let label = caps.get(4).map(|m| m.as_str().trim().to_string());
            relations.push(ArchimateRelation {
                from,
                to,
                label,
                source_line,
                kind: ArchimateRelationKind::Association,
                direction: ArchimateRelationDirection::Default,
            });
            continue;
        }
    }

    while let Some((label, source_line, element_ids)) = group_stack.pop() {
        groups.push(ArchimateGroup {
            label,
            source_line,
            element_ids,
        });
    }

    Ok(ArchimateDiagram {
        meta,
        elements,
        relations,
        groups,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ArchimateDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_archimate(&lines).unwrap()
    }

    #[test]
    fn basic_elements() {
        let d = parse(
            "archimate_element Business Actor cust \"Customer\"\narchimate_element Technology Node srv \"Server\"\n",
        );
        assert_eq!(d.elements.len(), 2);
        assert_eq!(d.elements[0].id, "cust");
        assert_eq!(d.elements[0].label, "Customer");
        assert!(matches!(d.elements[0].layer, ArchimateLayer::Business));
        assert_eq!(d.elements[0].kind, "Actor");
        assert_eq!(d.elements[0].source_line, 1);
        assert_eq!(d.elements[1].id, "srv");
        assert!(matches!(d.elements[1].layer, ArchimateLayer::Technology));
    }

    #[test]
    fn basic_relations() {
        let d = parse(
            "archimate_element Business Actor a \"A\"\narchimate_element Business Actor b \"B\"\narchimate_rel Association a b \"uses\"\n",
        );
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.relations[0].from, "a");
        assert_eq!(d.relations[0].to, "b");
        assert_eq!(d.relations[0].label.as_deref(), Some("uses"));
        assert_eq!(d.relations[0].source_line, 3);
    }

    #[test]
    fn empty_label_becomes_none() {
        let d = parse(
            "archimate_element Business Actor a \"A\"\narchimate_element Business Actor b \"B\"\narchimate_rel Triggering a b \"\"\n",
        );
        assert!(d.relations[0].label.is_none());
    }

    #[test]
    fn grouping_rectangles() {
        let d = parse(
            "skinparam rectangle {\nBackgroundColor #E6F4EA\n}\nrectangle \"Business Layer\" {\narchimate_element Business Actor a \"A\"\narchimate_element Business Process b \"B\"\n}\n",
        );
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].label, "Business Layer");
        assert_eq!(d.groups[0].source_line, 4);
        assert_eq!(d.groups[0].element_ids, vec!["a", "b"]);
        assert_eq!(d.meta.skinparams[0].key, "rectangleBackgroundColor");
        assert_eq!(d.meta.skinparams[0].value, "#E6F4EA");
    }

    #[test]
    fn relation_kinds() {
        let d = parse(
            "archimate_element Business Actor a \"A\"\narchimate_element Business Actor b \"B\"\narchimate_rel Composition a b \"\"\narchimate_rel Serving a b \"\"\narchimate_rel Access_ReadWrite a b \"\"\narchimate_rel Realization_Up a b \"\"\n",
        );
        assert_eq!(d.relations.len(), 4);
        assert!(matches!(
            d.relations[0].kind,
            ArchimateRelationKind::Composition
        ));
        assert!(matches!(
            d.relations[1].kind,
            ArchimateRelationKind::Serving
        ));
        assert!(matches!(d.relations[2].kind, ArchimateRelationKind::Access));
        assert!(matches!(
            d.relations[3].kind,
            ArchimateRelationKind::Realization
        ));
        assert_eq!(d.relations[3].direction, ArchimateRelationDirection::Up);
    }
}
