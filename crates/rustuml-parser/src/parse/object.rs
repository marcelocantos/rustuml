// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Object diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::object::*;

/// Parse preprocessed lines into an object diagram.
pub fn parse_object(lines: &[String]) -> Result<ObjectDiagram, ParseError> {
    let mut parser = ObjectParser::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }

    Ok(parser.finish())
}

struct ObjectParser {
    meta: DiagramMeta,
    objects: Vec<ObjectInstance>,
    links: Vec<ObjectLink>,
    /// Object currently being parsed (inside { ... } block).
    current_object: Option<String>,
}

impl ObjectParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            objects: Vec::new(),
            links: Vec::new(),
            current_object: None,
        }
    }

    fn finish(self) -> ObjectDiagram {
        ObjectDiagram {
            meta: self.meta,
            objects: self.objects,
            links: self.links,
        }
    }

    fn ensure_object(&mut self, id: &str) -> String {
        let id = id.trim().to_string();
        if !self.objects.iter().any(|o| o.id == id) {
            self.objects.push(ObjectInstance {
                id: id.clone(),
                label: id.clone(),
                kind: ObjectKind::Object,
                fields: Vec::new(),
                color: None,
            });
        }
        id
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
        // Inside an object/map body?
        if self.current_object.is_some() {
            if line == "}" {
                self.current_object = None;
                return Ok(());
            }
            self.parse_field_line(line);
            return Ok(());
        }

        if self.try_object_decl(line) {
            return Ok(());
        }
        if self.try_map_decl(line) {
            return Ok(());
        }
        if self.try_link(line) {
            return Ok(());
        }
        if self.try_meta(line) {
            return Ok(());
        }

        // Silently ignore unknown lines.
        Ok(())
    }

    fn try_object_decl(&mut self, line: &str) -> bool {
        // object "Label" as id #color { ... } or object id #color {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^object\s+(?:"([^"]+)"\s+as\s+(\w+)|(\w+))(?:\s+#(\S+))?\s*(\{.*)?$"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let (label, id) = if let (Some(l), Some(i)) = (caps.get(1), caps.get(2)) {
                (l.as_str().to_string(), i.as_str().to_string())
            } else {
                let id = caps[3].to_string();
                (id.clone(), id)
            };
            let color = caps.get(4).map(|m| m.as_str().to_string());
            let body_part = caps.get(5).map(|m| m.as_str());

            if let Some(obj) = self.objects.iter_mut().find(|o| o.id == id) {
                obj.label = label;
                obj.kind = ObjectKind::Object;
                if color.is_some() {
                    obj.color = color;
                }
            } else {
                self.objects.push(ObjectInstance {
                    id: id.clone(),
                    label,
                    kind: ObjectKind::Object,
                    fields: Vec::new(),
                    color,
                });
            }

            if let Some(body) = body_part {
                let inner = body.trim_start_matches('{').trim();
                if let Some(close) = inner.find('}') {
                    // Inline body: parse fields between { and }.
                    let content = inner[..close].trim();
                    self.current_object = Some(id.clone());
                    for part in content.split(';') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            self.parse_field_line(trimmed);
                        }
                    }
                    self.current_object = None;
                } else {
                    // Opening brace with no closing — multi-line body.
                    self.current_object = Some(id);
                }
            }
            true
        } else {
            false
        }
    }

    fn try_map_decl(&mut self, line: &str) -> bool {
        // map "Label" as id #color { ... } or map "Label" as id {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^map\s+"([^"]+)"\s+as\s+(\w+)(?:\s+#(\S+))?\s*(\{.*)?$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            let color = caps.get(3).map(|m| m.as_str().to_string());
            let body_part = caps.get(4).map(|m| m.as_str());

            if let Some(obj) = self.objects.iter_mut().find(|o| o.id == id) {
                obj.label = label;
                obj.kind = ObjectKind::Map;
                if color.is_some() {
                    obj.color = color;
                }
            } else {
                self.objects.push(ObjectInstance {
                    id: id.clone(),
                    label,
                    kind: ObjectKind::Map,
                    fields: Vec::new(),
                    color,
                });
            }

            if let Some(body) = body_part {
                let inner = body.trim_start_matches('{').trim();
                if let Some(close) = inner.find('}') {
                    let content = inner[..close].trim();
                    self.current_object = Some(id.clone());
                    for part in content.split(';') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            self.parse_field_line(trimmed);
                        }
                    }
                    self.current_object = None;
                } else {
                    self.current_object = Some(id);
                }
            }
            true
        } else {
            false
        }
    }

    fn try_link(&mut self, line: &str) -> bool {
        // A --> B : label  or  A::field --> B
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(\w+(?:::\w+)?)\s*--[>|]\s*(\w+(?:::\w+)?)(?:\s*:\s*(.+))?$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from_raw = caps[1].to_string();
            let to_raw = caps[2].to_string();
            let label = caps.get(3).map(|m| m.as_str().trim().to_string());

            // Ensure the base objects exist.
            let from_base = from_raw.split("::").next().unwrap_or(&from_raw).to_string();
            let to_base = to_raw.split("::").next().unwrap_or(&to_raw).to_string();
            self.ensure_object(&from_base);
            self.ensure_object(&to_base);

            self.links.push(ObjectLink {
                from: from_raw,
                to: to_raw,
                label,
            });
            true
        } else {
            false
        }
    }

    fn try_meta(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(super::strip_title_quotes(rest).to_string());
            return true;
        }
        line.starts_with("skinparam ")
            || line.starts_with("hide ")
            || line.starts_with("show ")
    }

    fn parse_field_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
            return;
        }

        // key => value  (map style) or  key = value  (object style) or bare key.
        let field = if let Some(pos) = trimmed.find("=>") {
            let name = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 2..].trim().to_string();
            ObjectField {
                name,
                value: if value.is_empty() { None } else { Some(value) },
            }
        } else if let Some(pos) = trimmed.find('=') {
            let name = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            ObjectField {
                name,
                value: if value.is_empty() { None } else { Some(value) },
            }
        } else {
            ObjectField {
                name: trimmed.to_string(),
                value: None,
            }
        };

        if let Some(obj_id) = &self.current_object {
            let obj_id = obj_id.clone();
            if let Some(obj) = self.objects.iter_mut().find(|o| o.id == obj_id) {
                obj.fields.push(field);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ObjectDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_object(&lines).unwrap()
    }

    #[test]
    fn bare_objects() {
        let d = parse("object Car\nobject Bike");
        assert_eq!(d.objects.len(), 2);
        assert_eq!(d.objects[0].id, "Car");
        assert_eq!(d.objects[1].id, "Bike");
    }

    #[test]
    fn object_with_fields() {
        let d = parse("object Car {\n  make = \"Toyota\"\n  year = 2023\n}");
        assert_eq!(d.objects[0].fields.len(), 2);
        assert_eq!(d.objects[0].fields[0].name, "make");
        assert_eq!(d.objects[0].fields[0].value.as_deref(), Some("\"Toyota\""));
        assert_eq!(d.objects[0].fields[1].name, "year");
    }

    #[test]
    fn map_with_entries() {
        let d = parse("map \"Config\" as cfg {\n  host => localhost\n  port => 8080\n}");
        assert_eq!(d.objects.len(), 1);
        assert_eq!(d.objects[0].kind, ObjectKind::Map);
        assert_eq!(d.objects[0].id, "cfg");
        assert_eq!(d.objects[0].label, "Config");
        assert_eq!(d.objects[0].fields.len(), 2);
        assert_eq!(d.objects[0].fields[0].name, "host");
        assert_eq!(d.objects[0].fields[0].value.as_deref(), Some("localhost"));
    }

    #[test]
    fn link_between_objects() {
        let d = parse("object A\nobject B\nA --> B");
        assert_eq!(d.links.len(), 1);
        assert_eq!(d.links[0].from, "A");
        assert_eq!(d.links[0].to, "B");
        assert!(d.links[0].label.is_none());
    }

    #[test]
    fn link_with_label() {
        let d = parse("object A\nobject B\nA --> B : owns");
        assert_eq!(d.links[0].label.as_deref(), Some("owns"));
    }

    #[test]
    fn field_level_link() {
        let d = parse("map \"U\" as u {\n  role => admin\n}\nmap \"P\" as p {}\nu::role --> p");
        assert_eq!(d.links[0].from, "u::role");
        assert_eq!(d.links[0].to, "p");
    }

    #[test]
    fn object_with_color() {
        let d = parse("object MyObject #blue {\n  field = \"value\"\n}");
        assert_eq!(d.objects[0].color.as_deref(), Some("blue"));
    }

    #[test]
    fn inline_body() {
        let d = parse("object Node0 { v = 0 }");
        assert_eq!(d.objects.len(), 1);
        assert_eq!(d.objects[0].id, "Node0");
        assert_eq!(d.objects[0].fields.len(), 1);
        assert_eq!(d.objects[0].fields[0].name, "v");
        assert_eq!(d.objects[0].fields[0].value.as_deref(), Some("0"));
    }
}
