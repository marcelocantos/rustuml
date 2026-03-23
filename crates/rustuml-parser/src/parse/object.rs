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
    notes: Vec<ObjectNote>,
    packages: Vec<ObjectPackage>,
    /// Object currently being parsed (inside { ... } block).
    current_object: Option<String>,
    /// Index into `packages` of the package block currently being parsed.
    current_package: Option<usize>,
}

impl ObjectParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            objects: Vec::new(),
            links: Vec::new(),
            notes: Vec::new(),
            packages: Vec::new(),
            current_object: None,
            current_package: None,
        }
    }

    fn finish(self) -> ObjectDiagram {
        ObjectDiagram {
            meta: self.meta,
            objects: self.objects,
            links: self.links,
            notes: self.notes,
            packages: self.packages,
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
                stereotype: None,
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

        // Closing a package/namespace block?
        if line == "}" {
            if self.current_package.is_some() {
                self.current_package = None;
            }
            return Ok(());
        }

        if self.try_package(line) {
            return Ok(());
        }
        let before = self.objects.len();
        if self.try_object_decl(line) {
            self.register_package_objects(before);
            return Ok(());
        }
        let before = self.objects.len();
        if self.try_map_decl(line) {
            self.register_package_objects(before);
            return Ok(());
        }
        if self.try_link(line) {
            return Ok(());
        }
        if self.try_inline_field(line) {
            return Ok(());
        }
        if self.try_note(line) {
            return Ok(());
        }
        if self.try_meta(line) {
            return Ok(());
        }

        // Silently ignore unknown lines.
        Ok(())
    }

    /// Record any newly added objects as members of the current package (if any).
    fn register_package_objects(&mut self, before: usize) {
        if let Some(pkg_idx) = self.current_package {
            for i in before..self.objects.len() {
                let id = self.objects[i].id.clone();
                self.packages[pkg_idx].object_ids.push(id);
            }
        }
    }

    fn try_object_decl(&mut self, line: &str) -> bool {
        // object "Label" as id <<stereotype>> #color { ... } or object id <<stereotype>> #color {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            // Note: color capture uses [^\s{;]+ to avoid greedily consuming the
            // opening brace when writing e.g. `object Foo #yellow {`.
            Regex::new(
                r#"^object\s+(?:"([^"]+)"\s+as\s+(\w+)|(\w+))(?:\s+<<([^>]+)>>)?(?:\s+#([^\s{;]+))?\s*(\{.*)?$"#,
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
            let stereotype = caps.get(4).map(|m| m.as_str().trim().to_string());
            let color = caps.get(5).map(|m| m.as_str().to_string());
            let body_part = caps.get(6).map(|m| m.as_str());

            if let Some(obj) = self.objects.iter_mut().find(|o| o.id == id) {
                obj.label = label;
                obj.kind = ObjectKind::Object;
                if stereotype.is_some() {
                    obj.stereotype = stereotype;
                }
                if color.is_some() {
                    obj.color = color;
                }
            } else {
                self.objects.push(ObjectInstance {
                    id: id.clone(),
                    label,
                    kind: ObjectKind::Object,
                    fields: Vec::new(),
                    stereotype,
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
            // Note: color capture uses [^\s{;]+ to avoid greedily consuming the
            // opening brace when writing e.g. `map "Foo" as foo #yellow {`.
            Regex::new(r#"^map\s+"([^"]+)"\s+as\s+(\w+)(?:\s+#([^\s{;]+))?\s*(\{.*)?$"#).unwrap()
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
                    stereotype: None,
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
        // Handles all PlantUML object link styles:
        //   A --> B : label          basic directed
        //   A -- B                   undirected
        //   A o--> B                 aggregation
        //   A *--> B                 composition
        //   A ..> B                  dotted
        //   A --|> B                 inheritance
        //   A "1" --> "0..*" B : has with cardinality
        // Groups: (from)(from-card?)(connector)(to-card?)(to)(label?)
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(\w+(?:::\w+)?)\s*(?:"([^"]*)"\s*)?([o*<>][-.o*<>|]*|[-.][-.o*<>|]*)\s*(?:"([^"]*)"\s*)?(\w+(?:::\w+)?)(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from_raw = caps[1].to_string();
            let from_multiplicity = caps.get(2).map(|m| m.as_str().to_string());
            let to_multiplicity = caps.get(4).map(|m| m.as_str().to_string());
            let to_raw = caps[5].to_string();
            let raw_label = caps.get(6).map(|m| m.as_str().trim().to_string());
            // Strip surrounding quotes from label if present.
            let label = raw_label.map(|s| {
                if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1].to_string()
                } else {
                    s
                }
            });

            let from_base = from_raw.split("::").next().unwrap_or(&from_raw).to_string();
            let to_base = to_raw.split("::").next().unwrap_or(&to_raw).to_string();
            self.ensure_object(&from_base);
            self.ensure_object(&to_base);

            self.links.push(ObjectLink { from: from_raw, to: to_raw, label, from_multiplicity, to_multiplicity });
            true
        } else {
            false
        }
    }

    fn try_package(&mut self, line: &str) -> bool {
        // package "Label" {  or  namespace com.example {  or  package Name
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^(?:package|namespace)\s+(?:"([^"]+)"|([^\s{]+))\s*\{?\s*$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let label = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            // Build a safe identifier (replace non-alphanumeric with '_').
            let id: String = label
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                .collect();
            self.packages.push(ObjectPackage { id, label, object_ids: Vec::new() });
            self.current_package = Some(self.packages.len() - 1);
            true
        } else {
            false
        }
    }

    /// Handle `ObjectId : field = value` or `ObjectId : field` inline field syntax.
    fn try_inline_field(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let obj_id = caps[1].to_string();
            let field_text = caps[2].trim().to_string();
            self.ensure_object(&obj_id);
            // Temporarily set current_object so parse_field_line stores the field.
            let saved = self.current_object.take();
            self.current_object = Some(obj_id);
            self.parse_field_line(&field_text);
            self.current_object = saved;
            true
        } else {
            false
        }
    }

    fn try_note(&mut self, line: &str) -> bool {
        // note "text" as ID  — floating note
        static RE_FLOATING: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap()
        });
        // note right/left/top/bottom of X : text
        static RE_ATTACHED: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(?:right|left|top|bottom)\s+of\s+(\w+)\s*:\s*(.+)$").unwrap()
        });
        // note right : text  (shorthand without "of X", attaches to last object)
        static RE_SHORTHAND: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(?:right|left|top|bottom)\s*:\s*(.+)$").unwrap()
        });

        if let Some(caps) = RE_FLOATING.captures(line) {
            let text = caps[1].replace("\\n", "\n");
            let id = caps[2].to_string();
            self.notes.push(ObjectNote { id: Some(id), target: None, text });
            return true;
        }
        if let Some(caps) = RE_ATTACHED.captures(line) {
            let target = caps[1].to_string();
            let text = caps[2].trim().to_string();
            self.notes.push(ObjectNote { id: None, target: Some(target), text });
            return true;
        }
        if let Some(caps) = RE_SHORTHAND.captures(line) {
            let text = caps[1].trim().to_string();
            let target = self.objects.last().map(|o| o.id.clone());
            self.notes.push(ObjectNote { id: None, target, text });
            return true;
        }
        // Silently swallow other note forms we don't fully handle yet.
        if line.starts_with("note ") {
            return true;
        }
        false
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
            ObjectField { name: trimmed.to_string(), value: None }
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
    fn link_with_quoted_label() {
        let d = parse("object A\nobject B\nA --> B : \"owns\"");
        assert_eq!(d.links[0].label.as_deref(), Some("owns"));
    }

    #[test]
    fn link_various_arrow_types() {
        let d = parse(
            "object A\nobject B\nobject C\nobject D\nobject E\nobject F\nobject G\nA o--> B\nA *--> C\nA ..> D\nA --|> E\nA -- F\nA --* G",
        );
        assert_eq!(d.links.len(), 6);
        assert_eq!(d.links[0].from, "A");
        assert_eq!(d.links[0].to, "B");
    }

    #[test]
    fn link_with_cardinality() {
        let d = parse("object Parent\nobject Child\nParent \"1\" --> \"0..*\" Child : has");
        assert_eq!(d.links.len(), 1);
        assert_eq!(d.links[0].from, "Parent");
        assert_eq!(d.links[0].to, "Child");
        assert_eq!(d.links[0].label.as_deref(), Some("has"));
        assert_eq!(d.links[0].from_multiplicity.as_deref(), Some("1"));
        assert_eq!(d.links[0].to_multiplicity.as_deref(), Some("0..*"));
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

    #[test]
    fn package_groups_objects() {
        let d = parse(
            "package \"Domain\" {\n  object User\n  object Role\n}\nobject Other",
        );
        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].label, "Domain");
        assert_eq!(d.packages[0].object_ids, vec!["User", "Role"]);
        assert_eq!(d.objects.len(), 3);
    }

    #[test]
    fn namespace_groups_objects() {
        let d = parse("namespace com.example {\n  object Entity\n}");
        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].label, "com.example");
        assert_eq!(d.packages[0].object_ids, vec!["Entity"]);
    }
}
