// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! JSON/YAML visualization diagram parser.
//!
//! Handles `@startjson`/`@endjson` and `@startyaml`/`@endyaml` blocks.
//! Supports `#highlight "key"` and `#highlight "key1" / "key2"` directives
//! for JSON diagrams.

use crate::diagram::{
    json_diagram::{DataFormat, JsonDiagram, JsonNode, JsonNodeValue},
    DiagramMeta,
};

use super::ParseError;

/// Parse a `@startjson` / `@endjson` block.
pub fn parse_json_diagram(lines: &[String]) -> Result<JsonDiagram, ParseError> {
    let (highlights, content) = extract_body(lines);
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| ParseError {
        line: e.line(),
        message: format!("JSON parse error: {e}"),
    })?;
    let root = json_value_to_node(None, &value, &highlights, &[]);
    Ok(JsonDiagram {
        meta: DiagramMeta::default(),
        format: DataFormat::Json,
        root,
    })
}

/// Parse a `@startyaml` / `@endyaml` block.
pub fn parse_yaml_diagram(lines: &[String]) -> Result<JsonDiagram, ParseError> {
    let (_highlights, content) = extract_body(lines);
    let value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| ParseError {
        line: e.location().map_or(0, |l| l.line()),
        message: format!("YAML parse error: {e}"),
    })?;
    let root = yaml_value_to_node(None, &value);
    Ok(JsonDiagram {
        meta: DiagramMeta::default(),
        format: DataFormat::Yaml,
        root,
    })
}

/// Extract `#highlight` directives and the body content from preprocessed lines.
///
/// The caller passes lines that have already been through the TIM preprocessor,
/// which means `@start*` / `@end*` markers have already been stripped.
/// This function separates `#highlight` directives from the data content.
fn extract_body(lines: &[String]) -> (Vec<Vec<String>>, String) {
    let mut highlights: Vec<Vec<String>> = Vec::new();
    let mut body: Vec<&str> = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#highlight") {
            let parts: Vec<String> = rest
                .split('/')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                highlights.push(parts);
            }
        } else {
            body.push(line.as_str());
        }
    }

    (highlights, body.join("\n"))
}

fn is_highlighted(path: &[String], highlights: &[Vec<String>]) -> bool {
    highlights.iter().any(|h| h.as_slice() == path)
}

fn json_value_to_node(
    key: Option<String>,
    value: &serde_json::Value,
    highlights: &[Vec<String>],
    parent_path: &[String],
) -> JsonNode {
    let node_path: Vec<String> = match &key {
        Some(k) => {
            let mut p = parent_path.to_vec();
            p.push(k.clone());
            p
        }
        None => parent_path.to_vec(),
    };

    let highlighted = is_highlighted(&node_path, highlights);

    let node_value = match value {
        serde_json::Value::Null => JsonNodeValue::Null,
        serde_json::Value::Bool(b) => JsonNodeValue::Bool { val: *b },
        serde_json::Value::Number(n) => JsonNodeValue::Number { val: n.to_string() },
        serde_json::Value::String(s) => JsonNodeValue::Str { val: s.clone() },
        serde_json::Value::Array(arr) => {
            let items = arr
                .iter()
                .map(|v| json_value_to_node(None, v, highlights, &node_path))
                .collect();
            JsonNodeValue::Array { items }
        }
        serde_json::Value::Object(obj) => {
            let fields = obj
                .iter()
                .map(|(k, v)| json_value_to_node(Some(k.clone()), v, highlights, &node_path))
                .collect();
            JsonNodeValue::Object { fields }
        }
    };

    JsonNode {
        key,
        value: node_value,
        highlighted,
    }
}

fn yaml_value_to_node(key: Option<String>, value: &serde_yaml::Value) -> JsonNode {
    let node_value = match value {
        serde_yaml::Value::Null => JsonNodeValue::Null,
        serde_yaml::Value::Bool(b) => JsonNodeValue::Bool { val: *b },
        serde_yaml::Value::Number(n) => JsonNodeValue::Number { val: n.to_string() },
        serde_yaml::Value::String(s) => JsonNodeValue::Str { val: s.clone() },
        serde_yaml::Value::Sequence(arr) => {
            let items = arr.iter().map(|v| yaml_value_to_node(None, v)).collect();
            JsonNodeValue::Array { items }
        }
        serde_yaml::Value::Mapping(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        serde_yaml::Value::String(s) => s.clone(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => format!("{k:?}"),
                    };
                    yaml_value_to_node(Some(key_str), v)
                })
                .collect();
            JsonNodeValue::Object { fields }
        }
        serde_yaml::Value::Tagged(tagged) => {
            return yaml_value_to_node(key, &tagged.value);
        }
    };

    JsonNode {
        key,
        value: node_value,
        highlighted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::json_diagram::JsonNodeValue;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    // Tests pass preprocessed lines (no @start*/@end* markers, as the TIM
    // preprocessor strips those before calling the diagram-specific parsers).

    fn find_field<'a>(fields: &'a [JsonNode], key: &str) -> &'a JsonNode {
        fields
            .iter()
            .find(|f| f.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("field '{key}' not found"))
    }

    #[test]
    fn parses_simple_json_object() {
        let input = lines("{\"name\": \"Alice\", \"age\": 30}");
        let diagram = parse_json_diagram(&input).unwrap();
        assert_eq!(diagram.format, DataFormat::Json);
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                assert_eq!(fields.len(), 2);
                assert!(fields.iter().any(|f| f.key.as_deref() == Some("name")));
                assert!(fields.iter().any(|f| f.key.as_deref() == Some("age")));
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_highlight_directive() {
        let input = lines("#highlight \"name\"\n{\"name\": \"Alice\", \"age\": 30}");
        let diagram = parse_json_diagram(&input).unwrap();
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                assert!(find_field(fields, "name").highlighted, "name should be highlighted");
                assert!(!find_field(fields, "age").highlighted, "age should not be highlighted");
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_nested_highlight() {
        let input = lines(
            "#highlight \"address\" / \"city\"\n{\"name\": \"Alice\", \"address\": {\"city\": \"Paris\"}}",
        );
        let diagram = parse_json_diagram(&input).unwrap();
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                let addr = find_field(fields, "address");
                match &addr.value {
                    JsonNodeValue::Object { fields: addr_fields } => {
                        assert!(
                            find_field(addr_fields, "city").highlighted,
                            "city should be highlighted"
                        );
                    }
                    _ => panic!("expected nested Object"),
                }
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_simple_yaml() {
        let input = lines("name: Alice\nage: 30");
        let diagram = parse_yaml_diagram(&input).unwrap();
        assert_eq!(diagram.format, DataFormat::Yaml);
        match &diagram.root.value {
            JsonNodeValue::Object { fields } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].key.as_deref(), Some("name"));
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn parses_yaml_list() {
        let input = lines("- apple\n- banana");
        let diagram = parse_yaml_diagram(&input).unwrap();
        match &diagram.root.value {
            JsonNodeValue::Array { items } => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected Array"),
        }
    }
}
