// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Stable diagnostic signatures for locating parity divergence.

use std::collections::HashMap;

use serde::Serialize;

use crate::compare::{CompareResult, Difference};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DotFingerprint {
    pub rankdir: Option<String>,
    pub nodesep: Option<String>,
    pub ranksep: Option<String>,
    pub splines: Option<String>,
    pub cluster_count: usize,
    pub nodes: Vec<DotNodeFingerprint>,
    pub edges: Vec<DotEdgeFingerprint>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct DotNodeFingerprint {
    pub shape: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub fixed_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DotEdgeFingerprint {
    pub tail_node: usize,
    pub head_node: usize,
    pub minlen: Option<String>,
    pub constraint: Option<String>,
    pub style: Option<String>,
    pub center_label_size: Option<(String, String)>,
    pub tail_label_size: Option<(String, String)>,
    pub head_label_size: Option<(String, String)>,
}

pub fn dot_fingerprint(dot: &str) -> DotFingerprint {
    let mut fingerprint = DotFingerprint {
        rankdir: None,
        nodesep: None,
        ranksep: None,
        splines: None,
        cluster_count: 0,
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let mut node_indices = HashMap::<String, usize>::new();

    for statement in dot_statements(dot) {
        let statement = statement.trim();
        if statement.is_empty() || statement.starts_with("digraph ") {
            continue;
        }
        if statement.starts_with("subgraph ") {
            fingerprint.cluster_count += 1;
            continue;
        }
        for (name, slot) in [
            ("rankdir", &mut fingerprint.rankdir),
            ("nodesep", &mut fingerprint.nodesep),
            ("ranksep", &mut fingerprint.ranksep),
            ("splines", &mut fingerprint.splines),
        ] {
            if let Some(value) = attribute_value(statement, name) {
                let value = normalize_scalar(&value);
                *slot = if name == "rankdir" && value == "tb" {
                    None
                } else {
                    Some(value)
                };
            }
        }

        if let Some(arrow) = find_outside(statement, "->") {
            let tail = normalize_id(&statement[..arrow]);
            let remainder = statement[arrow + 2..].trim_start();
            let head_end = remainder
                .find(['[', ' ', '\t', '\n'])
                .unwrap_or(remainder.len());
            let head = normalize_id(&remainder[..head_end]);
            if tail.is_empty() || head.is_empty() {
                continue;
            }
            let tail_node = node_index(&tail, &mut node_indices, &mut fingerprint.nodes);
            let head_node = node_index(&head, &mut node_indices, &mut fingerprint.nodes);
            fingerprint.edges.push(DotEdgeFingerprint {
                tail_node,
                head_node,
                minlen: attribute_value(statement, "minlen").map(|value| normalize_scalar(&value)),
                constraint: attribute_value(statement, "constraint")
                    .map(|value| normalize_scalar(&value)),
                style: attribute_value(statement, "style").map(|value| normalize_scalar(&value)),
                center_label_size: attribute_value(statement, "label")
                    .and_then(|value| html_label_size(&value)),
                tail_label_size: attribute_value(statement, "taillabel")
                    .and_then(|value| html_label_size(&value)),
                head_label_size: attribute_value(statement, "headlabel")
                    .and_then(|value| html_label_size(&value)),
            });
            continue;
        }

        let Some(bracket) = find_outside(statement, "[") else {
            continue;
        };
        let id = normalize_id(&statement[..bracket]);
        if id.is_empty() || matches!(id.as_str(), "graph" | "node" | "edge") {
            continue;
        }
        let index = node_index(&id, &mut node_indices, &mut fingerprint.nodes);
        let label_is_empty =
            attribute_value(statement, "label").is_some_and(|value| value.is_empty());
        fingerprint.nodes[index] = DotNodeFingerprint {
            shape: attribute_value(statement, "shape").map(|value| normalize_shape(&value)),
            width: attribute_value(statement, "width").map(|value| normalize_scalar(&value)),
            height: attribute_value(statement, "height").map(|value| normalize_scalar(&value)),
            fixed_size: attribute_value(statement, "fixedsize")
                .map(|value| normalize_scalar(&value))
                .filter(|value| !(label_is_empty && value == "true")),
        };
    }

    fingerprint
}

pub fn first_dot_fingerprint_difference(
    expected: &DotFingerprint,
    actual: &DotFingerprint,
) -> Option<String> {
    dot_fingerprint_differences(expected, actual)
        .into_iter()
        .next()
}

pub fn dot_fingerprint_differences(
    expected: &DotFingerprint,
    actual: &DotFingerprint,
) -> Vec<String> {
    let mut differences = Vec::new();
    for (path, left, right) in [
        ("graph.rankdir", &expected.rankdir, &actual.rankdir),
        ("graph.nodesep", &expected.nodesep, &actual.nodesep),
        ("graph.ranksep", &expected.ranksep, &actual.ranksep),
        ("graph.splines", &expected.splines, &actual.splines),
    ] {
        if left != right {
            differences.push(format!("{path}: {left:?} != {right:?}"));
        }
    }
    if expected.cluster_count != actual.cluster_count {
        differences.push(format!(
            "clusters.count: {} != {}",
            expected.cluster_count, actual.cluster_count
        ));
    }
    if expected.nodes.len() != actual.nodes.len() {
        differences.push(format!(
            "nodes.count: {} != {}",
            expected.nodes.len(),
            actual.nodes.len()
        ));
    }
    for (index, (left, right)) in expected.nodes.iter().zip(&actual.nodes).enumerate() {
        if left != right {
            differences.push(format!("nodes[{index}]: {left:?} != {right:?}"));
        }
    }
    if expected.edges.len() != actual.edges.len() {
        differences.push(format!(
            "edges.count: {} != {}",
            expected.edges.len(),
            actual.edges.len()
        ));
    }
    for (index, (left, right)) in expected.edges.iter().zip(&actual.edges).enumerate() {
        if left != right {
            differences.push(format!("edges[{index}]: {left:?} != {right:?}"));
        }
    }
    differences
}

pub fn first_svg_difference_shape(comparison: &CompareResult) -> Option<String> {
    comparison.differences.first().map(difference_shape)
}

pub fn difference_shape(difference: &Difference) -> String {
    match difference {
        Difference::ElementCount { expected, actual } => {
            format!("element-count:{expected}:{actual}")
        }
        Difference::TagMismatch {
            expected, actual, ..
        } => format!("tag:{expected}:{actual}"),
        Difference::AttrMismatch {
            tag,
            expected_attrs,
            actual_attrs,
            ..
        } => {
            let expected = expected_attrs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect::<HashMap<_, _>>();
            let actual = actual_attrs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect::<HashMap<_, _>>();
            let mut keys = expected
                .keys()
                .filter(|key| expected.get(**key) != actual.get(**key))
                .copied()
                .collect::<Vec<_>>();
            keys.extend(
                actual
                    .keys()
                    .filter(|key| !expected.contains_key(**key))
                    .copied(),
            );
            keys.sort_unstable();
            keys.dedup();
            format!("attrs:{tag}:{}", keys.join(","))
        }
        Difference::TextMismatch { tag, .. } => format!("text:{tag}"),
        Difference::DepthMismatch { tag, .. } => format!("depth:{tag}"),
    }
}

fn node_index(
    id: &str,
    indices: &mut HashMap<String, usize>,
    nodes: &mut Vec<DotNodeFingerprint>,
) -> usize {
    if let Some(index) = indices.get(id) {
        return *index;
    }
    let index = nodes.len();
    nodes.push(DotNodeFingerprint::default());
    indices.insert(id.to_string(), index);
    index
}

fn normalize_id(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn normalize_shape(value: &str) -> String {
    match value.trim_matches('"').to_ascii_lowercase().as_str() {
        "rect" | "rectangle" | "box" => "box".to_string(),
        other => other.to_string(),
    }
}

fn normalize_scalar(value: &str) -> String {
    let value = value.trim().trim_matches('"');
    value
        .parse::<f64>()
        .map(|number| format!("{number:.6}"))
        .unwrap_or_else(|_| value.to_ascii_lowercase())
}

fn html_label_size(value: &str) -> Option<(String, String)> {
    let width = html_numeric_attribute(value, "WIDTH")?;
    let height = html_numeric_attribute(value, "HEIGHT")?;
    Some((normalize_scalar(&width), normalize_scalar(&height)))
}

fn html_numeric_attribute(value: &str, name: &str) -> Option<String> {
    let uppercase = value.to_ascii_uppercase();
    let marker = format!("{name}=\"");
    let start = uppercase.find(&marker)? + marker.len();
    let end = uppercase[start..].find('"')? + start;
    Some(value[start..end].to_string())
}

fn attribute_value(statement: &str, name: &str) -> Option<String> {
    let bytes = statement.as_bytes();
    let name_bytes = name.as_bytes();
    let mut cursor = 0;
    while cursor + name_bytes.len() <= bytes.len() {
        let offset = statement[cursor..].find(name)?;
        let start = cursor + offset;
        let before_is_name = start > 0 && is_name_byte(bytes[start - 1]);
        let after = start + name_bytes.len();
        let after_is_name = after < bytes.len() && is_name_byte(bytes[after]);
        if before_is_name || after_is_name {
            cursor = after;
            continue;
        }
        let mut value_start = after;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        if bytes.get(value_start) != Some(&b'=') {
            cursor = after;
            continue;
        }
        value_start += 1;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        if bytes.get(value_start) == Some(&b'"') {
            let mut end = value_start + 1;
            while end < bytes.len() {
                if bytes[end] == b'"' && bytes.get(end.wrapping_sub(1)) != Some(&b'\\') {
                    return Some(statement[value_start + 1..end].to_string());
                }
                end += 1;
            }
            return None;
        }
        if bytes.get(value_start) == Some(&b'<') {
            let mut depth = 0_i32;
            for (offset, byte) in bytes[value_start..].iter().enumerate() {
                if *byte == b'<' {
                    depth += 1;
                } else if *byte == b'>' {
                    depth -= 1;
                    if depth == 0 {
                        return Some(statement[value_start..=value_start + offset].to_string());
                    }
                }
            }
            return None;
        }
        let end = bytes[value_start..]
            .iter()
            .position(|byte| byte.is_ascii_whitespace() || matches!(*byte, b',' | b']'))
            .map_or(bytes.len(), |offset| value_start + offset);
        return Some(statement[value_start..end].to_string());
    }
    None
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn find_outside(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let needle = needle.as_bytes();
    let mut quote = false;
    let mut escaped = false;
    let mut angle_depth = 0_i32;
    let mut cursor = 0;
    while cursor + needle.len() <= bytes.len() {
        let byte = bytes[cursor];
        if quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quote = false;
            }
        } else if byte == b'"' {
            quote = true;
        } else if byte == b'<' {
            angle_depth += 1;
        } else if byte == b'>' && angle_depth > 0 {
            angle_depth -= 1;
        } else if angle_depth == 0 && bytes[cursor..].starts_with(needle) {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn dot_statements(dot: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut quote = false;
    let mut escaped = false;
    let mut angle_depth = 0_i32;
    for character in dot.chars() {
        if quote {
            current.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quote = false;
            }
            continue;
        }
        match character {
            '"' => {
                quote = true;
                current.push(character);
            }
            '<' => {
                angle_depth += 1;
                current.push(character);
            }
            '>' if angle_depth > 0 => {
                angle_depth -= 1;
                current.push(character);
            }
            ';' | '{' | '}' if angle_depth == 0 => {
                let statement = current.split_whitespace().collect::<Vec<_>>().join(" ");
                if !statement.is_empty() {
                    statements.push(statement);
                }
                current.clear();
            }
            _ => current.push(character),
        }
    }
    let statement = current.split_whitespace().collect::<Vec<_>>().join(" ");
    if !statement.is_empty() {
        statements.push(statement);
    }
    statements
}

#[cfg(test)]
mod tests {
    use super::{dot_fingerprint, dot_fingerprint_differences, first_dot_fingerprint_difference};

    #[test]
    fn dot_fingerprint_ignores_generated_ids_and_box_spelling() {
        let java = r##"digraph unix {
            nodesep=0.486111;
            ranksep=0.833333;
            sh0006 [shape=rect,label="",width=0.578600,height=0.666667,color="#000006"];
            sh0007 [shape=rect,label="",width=0.556288,height=0.666667,color="#000007"];
            sh0006->sh0007[arrowtail=none,arrowhead=none,minlen=1,label=<<TABLE FIXEDSIZE="TRUE" WIDTH="32" HEIGHT="17"><TR><TD></TD></TR></TABLE>>];
        }"##;
        let rust = r#"digraph G {
            rankdir=TB;
            nodesep="0.486111";
            ranksep="0.833333";
            First [fixedsize=true, height=0.666667, label="", shape=box, width=0.578600];
            Second [fixedsize=true, height=0.666667, label="", shape=box, width=0.556288];
            First -> Second [label=<<TABLE FIXEDSIZE="TRUE" WIDTH="32" HEIGHT="17"><TR><TD></TD></TR></TABLE>>, minlen=1];
        }"#;
        let java = dot_fingerprint(java);
        let rust = dot_fingerprint(rust);
        assert_eq!(first_dot_fingerprint_difference(&java, &rust), None);
    }

    #[test]
    fn dot_fingerprint_names_the_first_model_difference() {
        let left = dot_fingerprint("digraph G { ranksep=0.8; A [width=1,height=2]; }");
        let right = dot_fingerprint("digraph G { ranksep=0.9; Z [width=1,height=2]; }");
        assert_eq!(
            first_dot_fingerprint_difference(&left, &right).as_deref(),
            Some("graph.ranksep: Some(\"0.800000\") != Some(\"0.900000\")")
        );
    }

    #[test]
    fn dot_fingerprint_reports_every_projected_difference() {
        let left =
            dot_fingerprint("digraph G { ranksep=0.8; splines=ortho; A [width=1,height=2]; }");
        let right =
            dot_fingerprint("digraph G { ranksep=0.9; splines=polyline; Z [width=3,height=2]; }");
        assert_eq!(
            dot_fingerprint_differences(&left, &right),
            vec![
                "graph.ranksep: Some(\"0.800000\") != Some(\"0.900000\")",
                "graph.splines: Some(\"ortho\") != Some(\"polyline\")",
                "nodes[0]: DotNodeFingerprint { shape: None, width: Some(\"1.000000\"), height: Some(\"2.000000\"), fixed_size: None } != DotNodeFingerprint { shape: None, width: Some(\"3.000000\"), height: Some(\"2.000000\"), fixed_size: None }",
            ]
        );
    }
}
