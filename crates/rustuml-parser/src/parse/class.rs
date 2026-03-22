// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::class::*;

/// Parse preprocessed lines into a class diagram.
pub fn parse_class(lines: &[String]) -> Result<ClassDiagram, ParseError> {
    let mut parser = ClassParser::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }

    Ok(parser.finish())
}

struct ClassParser {
    meta: DiagramMeta,
    entities: Vec<ClassEntity>,
    relationships: Vec<Relationship>,
    packages: Vec<Package>,
    /// Entity currently being parsed (inside { ... } block).
    current_entity: Option<String>,
    /// Current package scope.
    current_package: Option<String>,
}

impl ClassParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            entities: Vec::new(),
            relationships: Vec::new(),
            packages: Vec::new(),
            current_entity: None,
            current_package: None,
        }
    }

    fn finish(self) -> ClassDiagram {
        ClassDiagram {
            meta: self.meta,
            entities: self.entities,
            relationships: self.relationships,
            packages: self.packages,
        }
    }

    fn ensure_entity(&mut self, id: &str) -> String {
        let id = id.trim().to_string();
        if !self.entities.iter().any(|e| e.id == id) {
            self.entities.push(ClassEntity {
                id: id.clone(),
                label: id.clone(),
                kind: EntityKind::Class,
                members: Vec::new(),
                stereotypes: Vec::new(),
            });
        }
        id
    }

    fn find_entity_mut(&mut self, id: &str) -> Option<&mut ClassEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
        // Inside a class body?
        if self.current_entity.is_some() {
            if line == "}" || line == "}}" {
                self.current_entity = None;
                return Ok(());
            }
            self.parse_member_line(line);
            return Ok(());
        }

        if self.try_entity_decl(line) {
            return Ok(());
        }
        if self.try_relationship(line) {
            return Ok(());
        }
        if self.try_inline_member(line) {
            return Ok(());
        }
        if self.try_package(line) {
            return Ok(());
        }
        if self.try_package_end(line) {
            return Ok(());
        }
        if self.try_enum_decl(line) {
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

    fn try_entity_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(class|abstract\s+class|interface|enum|annotation|entity)\s+(?:"([^"]+)"\s+as\s+)?(\w+)"#,
            )
            .unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<(\w+)>>").unwrap());

        if let Some(caps) = RE.captures(line) {
            let kind = parse_entity_kind(caps[1].trim());
            let label = caps
                .get(2)
                .map_or_else(|| caps[3].to_string(), |m| m.as_str().to_string());
            let id = caps[3].to_string();

            let stereotypes: Vec<String> = STEREOTYPE_RE
                .captures_iter(line)
                .map(|c| c[1].to_string())
                .collect();

            if let Some(entity) = self.find_entity_mut(&id) {
                entity.kind = kind;
                entity.label = label;
                entity.stereotypes.extend(stereotypes);
            } else {
                self.entities.push(ClassEntity {
                    id: id.clone(),
                    label,
                    kind,
                    members: Vec::new(),
                    stereotypes,
                });
            }

            if line.ends_with('{') || line.ends_with("{{") {
                self.current_entity = Some(id);
            }
            true
        } else {
            false
        }
    }

    fn try_enum_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^enum\s+(\w+)\s*\{?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = caps[1].to_string();
            if !self.entities.iter().any(|e| e.id == id) {
                self.entities.push(ClassEntity {
                    id: id.clone(),
                    label: id.clone(),
                    kind: EntityKind::Enum,
                    members: Vec::new(),
                    stereotypes: Vec::new(),
                });
            }
            if line.ends_with('{') {
                self.current_entity = Some(id);
            }
            true
        } else {
            false
        }
    }

    fn try_relationship(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(?:"([^"]+)"\s+)?(\w+)\s+((?:<\|--|\.\.\|>|\*--|o--|--|\.\.>|<\.\.))\s+(?:"([^"]+)"\s+)?(\w+)(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from_mult = caps.get(1).map(|m| m.as_str().to_string());
            let from_raw = &caps[2];
            let rel_str = &caps[3];
            let to_mult = caps.get(4).map(|m| m.as_str().to_string());
            let to_raw = &caps[5];
            let label = caps.get(6).map(|m| m.as_str().trim().to_string());

            let kind = parse_relationship_kind(rel_str);
            let from = self.ensure_entity(from_raw);
            let to = self.ensure_entity(to_raw);

            self.relationships.push(Relationship {
                from,
                to,
                kind,
                label,
                from_multiplicity: from_mult,
                to_multiplicity: to_mult,
            });
            true
        } else {
            false
        }
    }

    fn try_inline_member(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let entity_id = caps[1].to_string();
            let member_text = caps[2].trim();

            self.ensure_entity(&entity_id);
            let member = parse_member(member_text);

            if let Some(entity) = self.find_entity_mut(&entity_id) {
                entity.members.push(member);
            }
            true
        } else {
            false
        }
    }

    fn try_package(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^package\s+(?:"([^"]+)"|(\S+))\s*\{?"#).unwrap());

        if let Some(caps) = RE.captures(line) {
            let name = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            self.current_package = Some(name.clone());
            self.packages.push(Package {
                name,
                entities: Vec::new(),
            });
            true
        } else {
            false
        }
    }

    fn try_package_end(&mut self, line: &str) -> bool {
        if line == "}" && self.current_package.is_some() {
            self.current_package = None;
            true
        } else {
            false
        }
    }

    fn try_note(&mut self, line: &str) -> bool {
        line.starts_with("note ") || line == "end note"
    }

    fn try_meta(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(rest.trim().to_string());
            return true;
        }
        // Skip skinparam, hide, show, together, etc.
        line.starts_with("skinparam ")
            || line.starts_with("hide ")
            || line.starts_with("show ")
            || line.starts_with("together")
            || line.starts_with("allowmixing")
            || line.starts_with("note on link")
            || line.starts_with("map ")
            || line.starts_with("object ")
    }

    fn parse_member_line(&mut self, line: &str) {
        let trimmed = line.trim();
        // Separator lines.
        if trimmed == "--" || trimmed == ".." || trimmed == "==" {
            return;
        }
        // Empty or brace-only.
        if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
            return;
        }

        let member = parse_member(trimmed);
        if let Some(entity_id) = &self.current_entity
            && let Some(entity) = self.entities.iter_mut().find(|e| e.id == *entity_id)
        {
            entity.members.push(member);
        }
    }
}

fn parse_entity_kind(s: &str) -> EntityKind {
    match s {
        "abstract class" => EntityKind::AbstractClass,
        "interface" => EntityKind::Interface,
        "enum" => EntityKind::Enum,
        "annotation" => EntityKind::Annotation,
        "entity" => EntityKind::Entity,
        _ => EntityKind::Class,
    }
}

fn parse_relationship_kind(s: &str) -> RelationshipKind {
    if s.contains("<|--") {
        RelationshipKind::Inheritance
    } else if s.contains("..|>") || s.contains("<..") {
        RelationshipKind::Implementation
    } else if s.contains("*--") {
        RelationshipKind::Composition
    } else if s.contains("o--") {
        RelationshipKind::Aggregation
    } else if s.contains("..>") {
        RelationshipKind::Dependency
    } else {
        RelationshipKind::Association
    }
}

fn parse_member(s: &str) -> Member {
    let mut text = s.to_string();
    let mut is_static = false;
    let mut is_abstract = false;

    // Check for {static} and {abstract} modifiers.
    if text.contains("{static}") {
        is_static = true;
        text = text.replace("{static}", "").trim().to_string();
    }
    if text.contains("{abstract}") {
        is_abstract = true;
        text = text.replace("{abstract}", "").trim().to_string();
    }

    // Parse visibility prefix.
    let (visibility, rest) = match text.chars().next() {
        Some('+') => (Visibility::Public, &text[1..]),
        Some('-') => (Visibility::Private, &text[1..]),
        Some('#') => (Visibility::Protected, &text[1..]),
        Some('~') => (Visibility::Package, &text[1..]),
        _ => (Visibility::Default, text.as_str()),
    };

    let rest = rest.trim();

    // Determine if method (contains parens) or field.
    let is_method = rest.contains('(');

    if is_method {
        let (name, return_type) = if let Some(colon_pos) = rest.rfind(':') {
            let before = rest[..colon_pos].trim();
            let after = rest[colon_pos + 1..].trim();
            (before.to_string(), Some(after.to_string()))
        } else {
            (rest.to_string(), None)
        };
        Member {
            name,
            return_type,
            visibility,
            is_static,
            is_abstract,
            kind: MemberKind::Method,
        }
    } else if let Some(colon_pos) = rest.find(':') {
        let name = rest[..colon_pos].trim().to_string();
        let typ = rest[colon_pos + 1..].trim().to_string();
        Member {
            name,
            return_type: Some(typ),
            visibility,
            is_static,
            is_abstract,
            kind: MemberKind::Field,
        }
    } else {
        // Bare name (e.g., enum value).
        Member {
            name: rest.to_string(),
            return_type: None,
            visibility,
            is_static,
            is_abstract,
            kind: MemberKind::Field,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ClassDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_class(&lines).unwrap()
    }

    #[test]
    fn simple_class() {
        let d = parse("class Animal {\n  +name : String\n  +makeSound() : void\n}");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].id, "Animal");
        assert_eq!(d.entities[0].members.len(), 2);
        assert_eq!(d.entities[0].members[0].visibility, Visibility::Public);
        assert_eq!(d.entities[0].members[0].name, "name");
        assert_eq!(
            d.entities[0].members[0].return_type.as_deref(),
            Some("String")
        );
        assert_eq!(d.entities[0].members[1].kind, MemberKind::Method);
    }

    #[test]
    fn entity_types() {
        let d = parse("class A\nabstract class B\ninterface C\nenum D\nannotation E\nentity F");
        assert_eq!(d.entities[0].kind, EntityKind::Class);
        assert_eq!(d.entities[1].kind, EntityKind::AbstractClass);
        assert_eq!(d.entities[2].kind, EntityKind::Interface);
        assert_eq!(d.entities[3].kind, EntityKind::Enum);
        assert_eq!(d.entities[4].kind, EntityKind::Annotation);
        assert_eq!(d.entities[5].kind, EntityKind::Entity);
    }

    #[test]
    fn visibility() {
        let d = parse("class Foo {\n  +pub\n  -priv\n  #prot\n  ~pkg\n}");
        assert_eq!(d.entities[0].members[0].visibility, Visibility::Public);
        assert_eq!(d.entities[0].members[1].visibility, Visibility::Private);
        assert_eq!(d.entities[0].members[2].visibility, Visibility::Protected);
        assert_eq!(d.entities[0].members[3].visibility, Visibility::Package);
    }

    #[test]
    fn static_abstract() {
        let d = parse("class Foo {\n  {static} counter : int\n  {abstract} process()\n}");
        assert!(d.entities[0].members[0].is_static);
        assert!(d.entities[0].members[1].is_abstract);
    }

    #[test]
    fn inheritance() {
        let d = parse("A <|-- B");
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].kind, RelationshipKind::Inheritance);
        assert_eq!(d.relationships[0].from, "A");
        assert_eq!(d.relationships[0].to, "B");
    }

    #[test]
    fn all_relationships() {
        let d = parse("A <|-- B\nC ..|> D\nE *-- F\nG o-- H\nI -- J\nK ..> L");
        assert_eq!(d.relationships.len(), 6);
        assert_eq!(d.relationships[0].kind, RelationshipKind::Inheritance);
        assert_eq!(d.relationships[1].kind, RelationshipKind::Implementation);
        assert_eq!(d.relationships[2].kind, RelationshipKind::Composition);
        assert_eq!(d.relationships[3].kind, RelationshipKind::Aggregation);
        assert_eq!(d.relationships[4].kind, RelationshipKind::Association);
        assert_eq!(d.relationships[5].kind, RelationshipKind::Dependency);
    }

    #[test]
    fn relationship_label() {
        let d = parse("Parent -- Child : has");
        assert_eq!(d.relationships[0].label.as_deref(), Some("has"));
    }

    #[test]
    fn package() {
        let d = parse("package com.example {\n  class Foo\n  class Bar\n}");
        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].name, "com.example");
        assert_eq!(d.entities.len(), 2);
    }

    #[test]
    fn stereotype() {
        let d = parse("class Foo <<singleton>>");
        assert_eq!(d.entities[0].stereotypes, vec!["singleton"]);
    }

    #[test]
    fn enum_values() {
        let d = parse("enum Color {\n  RED\n  GREEN\n  BLUE\n}");
        assert_eq!(d.entities[0].kind, EntityKind::Enum);
        assert_eq!(d.entities[0].members.len(), 3);
        assert_eq!(d.entities[0].members[0].name, "RED");
    }

    #[test]
    fn inline_member() {
        let d = parse("class User\nUser : +name : String\nUser : +login()");
        assert_eq!(d.entities[0].members.len(), 2);
    }

    #[test]
    fn generics() {
        let d = parse("class Container<T>\nclass Map<K, V>");
        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.entities[0].id, "Container");
        assert_eq!(d.entities[1].id, "Map");
    }

    #[test]
    fn separators() {
        let d = parse("class Foo {\n  +field1\n  --\n  +method1()\n  ==\n  -internal\n}");
        // Separators are ignored, members are parsed.
        assert_eq!(d.entities[0].members.len(), 3);
    }
}
