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
    notes: Vec<Note>,
    /// Entity currently being parsed (inside { ... } block).
    current_entity: Option<String>,
    /// Stack of active package indices (innermost last), supporting nested packages.
    package_stack: Vec<usize>,
    /// Note currently being accumulated (multi-line `note ... end note`).
    current_note: Option<Note>,
    /// ID of the last declared entity (for shorthand `note right : text`).
    last_entity_id: Option<String>,
}

impl ClassParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            entities: Vec::new(),
            relationships: Vec::new(),
            packages: Vec::new(),
            notes: Vec::new(),
            current_entity: None,
            package_stack: Vec::new(),
            current_note: None,
            last_entity_id: None,
        }
    }

    fn finish(self) -> ClassDiagram {
        ClassDiagram {
            meta: self.meta,
            entities: self.entities,
            relationships: self.relationships,
            packages: self.packages,
            notes: self.notes,
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

    /// The index of the innermost active package scope (top of the stack).
    fn current_package(&self) -> Option<usize> {
        self.package_stack.last().copied()
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
        // Inside a multi-line note?
        if self.current_note.is_some() {
            if line == "end note" {
                let note = self.current_note.take().unwrap();
                self.notes.push(note);
            } else {
                if let Some(note) = self.current_note.as_mut() {
                    note.lines.push(line.to_string());
                }
            }
            return Ok(());
        }

        // Inside a class body?
        if self.current_entity.is_some() {
            if line == "}" || line == "}}" {
                self.current_entity = None;
                return Ok(());
            }
            self.parse_member_line(line);
            return Ok(());
        }

        // Closing brace: pop the innermost package scope.
        if line == "}" {
            self.package_stack.pop();
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
            // Matches either:
            //   keyword "label" as id  — groups 2 (label) and 3 (id)
            //   keyword id             — group 3 (id only)
            //   keyword "label"        — group 4 (quoted-only, no `as`)
            Regex::new(
                r#"^(class|abstract\s+class|interface|enum|annotation|entity)\s+(?:(?:"([^"]+)"\s+as\s+)?(\w+(?:<[^>]+>)?)|"([^"]+)")"#,
            )
            .unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)>>").unwrap());

        if let Some(caps) = RE.captures(line) {
            let kind = parse_entity_kind(caps[1].trim());
            // Group 4: quoted-only form — class "**Name**" with no `as` keyword.
            let (label, id) = if let Some(m) = caps.get(4) {
                let label_raw = m.as_str().to_string();
                let id = strip_creole_for_id(&label_raw);
                (label_raw, id)
            } else {
                let label = caps
                    .get(2)
                    .map_or_else(|| caps[3].to_string(), |m| m.as_str().to_string());
                let id = caps[3].to_string();
                (label, id)
            };

            let stereotypes: Vec<String> = STEREOTYPE_RE
                .captures_iter(line)
                .map(|c| c[1].trim().to_string())
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

            // Register entity in ALL active packages (innermost to outermost),
            // so that outer container bounding boxes include entities from inner
            // nested packages.
            for &pkg_idx in &self.package_stack {
                let pkg = &mut self.packages[pkg_idx];
                if !pkg.entities.contains(&id) {
                    pkg.entities.push(id.clone());
                }
            }

            if line.ends_with('{') || line.ends_with("{{") {
                self.current_entity = Some(id.clone());
            }
            self.last_entity_id = Some(id);
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
                self.current_entity = Some(id.clone());
            }
            self.last_entity_id = Some(id);
            true
        } else {
            false
        }
    }

    fn try_relationship(&mut self, line: &str) -> bool {
        // Relationship format: EntityA ["mult"] ARROW ["mult"] EntityB [: label]
        // Supported arrows: <|--, --|>, ..|>, <|.., *--, --*, o--, --o,
        //                   <-->, <..>, --, -->, <--, <-, .., ..>, <..
        // Multiple dashes (e.g. ---- or ------) are treated as plain association.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(\w+)\s*(?:"([^"]+)")?\s*((?:<\|--|--\|>|\.\.\|>|<\|\.\.|<\.\.>|<\.\.|\*--|--\*|o--|--o|<-->|<--|-->|<-|-{2,}|\.\.|\.\.>))\s*(?:"([^"]+)")?\s*(\w+)(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });
        // ER crow's foot notation: entity1 CROW--CROW entity2 : "label"
        static ER_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(\w+)\s+([|o}][|{o]--[|o][|{])\s+(\w+)(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from_raw = &caps[1];
            let from_mult = caps.get(2).map(|m| m.as_str().to_string());
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
            return true;
        }

        if let Some(caps) = ER_RE.captures(line) {
            let from_raw = &caps[1];
            let to_raw = &caps[3];
            let label = caps
                .get(4)
                .map(|m| m.as_str().trim().trim_matches('"').to_string());

            let from = self.ensure_entity(from_raw);
            let to = self.ensure_entity(to_raw);

            self.relationships.push(Relationship {
                from,
                to,
                kind: RelationshipKind::Association,
                label,
                from_multiplicity: None,
                to_multiplicity: None,
            });
            return true;
        }

        false
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
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(package|namespace|cloud|database|folder|frame|rectangle|node)\s+(?:"([^"]+)"|([^#\s{]+))\s*(?:#([^\s{]+))?\s*\{?"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let kind_str = &caps[1];
            let name = caps
                .get(2)
                .or(caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let color = caps.get(4).map(|m| {
                let s = m.as_str();
                // Keep hex colors as-is (only hex digits → prepend nothing).
                // Named colors: strip no prefix (already without #).
                s.to_string()
            });
            let kind = match kind_str {
                "namespace" => PackageKind::Namespace,
                "cloud" => PackageKind::Cloud,
                "database" => PackageKind::Database,
                "folder" => PackageKind::Folder,
                "frame" => PackageKind::Frame,
                "rectangle" => PackageKind::Rectangle,
                "node" => PackageKind::Node,
                _ => PackageKind::Package,
            };
            let pkg_idx = self.packages.len();
            self.package_stack.push(pkg_idx);
            self.packages.push(Package {
                name,
                kind,
                color,
                entities: Vec::new(),
            });
            true
        } else {
            false
        }
    }


    fn try_note(&mut self, line: &str) -> bool {
        // Single-line attached note: `note <pos> of <entity> : <text>`
        static ATTACHED_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s+of\s+(\w+)\s*:\s*(.+)$").unwrap()
        });
        // Multi-line attached note start: `note <pos> of <entity>`
        static ATTACHED_ML_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s+of\s+(\w+)\s*$").unwrap()
        });
        // Shorthand single-line note attached to last entity: `note <pos> : <text>`
        static SHORT_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s*:\s*(.+)$").unwrap()
        });
        // Shorthand multi-line note attached to last entity: `note <pos>`
        static SHORT_ML_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s*$").unwrap()
        });
        // Floating named note: `note "text" as Name`
        static FLOATING_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap()
        });
        // Multi-line floating note: `note as Name`
        static FLOATING_ML_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+as\s+(\w+)\s*$").unwrap()
        });

        if let Some(caps) = ATTACHED_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let target = caps[2].to_string();
            let text = caps[3].trim().to_string();
            // Expand `\n` escape sequences into actual newlines.
            let lines = text
                .split("\\n")
                .map(|s| s.trim().to_string())
                .collect();
            self.notes.push(Note {
                lines,
                target: Some(target),
                position: Some(position),
                alias: None,
            });
            return true;
        }

        if let Some(caps) = ATTACHED_ML_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let target = caps[2].to_string();
            self.current_note = Some(Note {
                lines: Vec::new(),
                target: Some(target),
                position: Some(position),
                alias: None,
            });
            return true;
        }

        // Shorthand: `note right : text` — attaches to the last declared entity.
        if let Some(caps) = SHORT_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let text = caps[2].trim().to_string();
            let lines = text
                .split("\\n")
                .map(|s| s.trim().to_string())
                .collect();
            let target = self.last_entity_id.clone();
            self.notes.push(Note {
                lines,
                target,
                position: Some(position),
                alias: None,
            });
            return true;
        }

        // Shorthand multi-line: `note right` — attaches to the last declared entity.
        if let Some(caps) = SHORT_ML_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let target = self.last_entity_id.clone();
            self.current_note = Some(Note {
                lines: Vec::new(),
                target,
                position: Some(position),
                alias: None,
            });
            return true;
        }

        if let Some(caps) = FLOATING_RE.captures(line) {
            let text = caps[1].trim().to_string();
            let alias = caps[2].to_string();
            let lines = text
                .split("\\n")
                .map(|s| s.trim().to_string())
                .collect();
            self.notes.push(Note {
                lines,
                target: None,
                position: None,
                alias: Some(alias),
            });
            return true;
        }

        if let Some(caps) = FLOATING_ML_RE.captures(line) {
            let alias = caps[1].to_string();
            self.current_note = Some(Note {
                lines: Vec::new(),
                target: None,
                position: None,
                alias: Some(alias),
            });
            return true;
        }

        // `note on link` — multi-line note attached to the last relationship as a floating note.
        if line == "note on link" || line.starts_with("note on link") {
            self.current_note = Some(Note {
                lines: Vec::new(),
                target: None,
                position: None,
                alias: None,
            });
            return true;
        }

        // `end note` is handled in parse_line; skip it here if encountered standalone.
        if line == "end note" {
            return true;
        }

        // Bare `note` prefix — consume to avoid falling through to unknown.
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
        static SEPARATOR_RE: LazyLock<Regex> = LazyLock::new(|| {
            // Matches labeled separators: -- label --, == label ==, __ label __, .. label ..
            // Bare separators (-- == __ ..) are handled by the equality checks below.
            Regex::new(r"^(--|==|__|\.\.)\s+.+\s+(--|==|__|\.\.)\s*$").unwrap()
        });

        let trimmed = line.trim();
        // Separator lines — bare or labeled.
        if trimmed == "--" || trimmed == ".." || trimmed == "==" || trimmed == "__"
            || SEPARATOR_RE.is_match(trimmed)
        {
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

fn parse_note_position(s: &str) -> NotePosition {
    match s {
        "top" => NotePosition::Top,
        "bottom" => NotePosition::Bottom,
        "left" => NotePosition::Left,
        "right" => NotePosition::Right,
        _ => NotePosition::Right,
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

    // Convert <<stereotype>> notation to «stereotype» guillemets.
    while let Some(start) = text.find("<<") {
        if let Some(end) = text[start..].find(">>") {
            let inner = text[start + 2..start + end].to_string();
            text = format!("{}«{}»{}", &text[..start], inner, &text[start + end + 2..]);
        } else {
            break;
        }
    }

    // Parse visibility prefix.
    let (visibility, rest) = match text.chars().next() {
        Some('+') => (Visibility::Public, &text[1..]),
        Some('-') => (Visibility::Private, &text[1..]),
        Some('#') => (Visibility::Protected, &text[1..]),
        Some('~') => (Visibility::Package, &text[1..]),
        // ER diagrams use '*' to mark required/primary-key fields.
        Some('*') => (Visibility::Default, &text[1..]),
        _ => (Visibility::Default, text.as_str()),
    };

    let rest = rest.trim();

    // Determine if method (contains parens) or field.
    let is_method = rest.contains('(');

    // `rest` is the text after stripping the visibility prefix. It is used
    // verbatim as the display text (preserves original colon spacing).
    let display_text = rest.to_string();

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
            display_text,
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
            display_text,
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
            display_text,
        }
    }
}

/// Strip Creole/HTML markup from a display name to produce a plain identifier.
/// Used when a class is declared with a quoted markup name but no `as` alias.
fn strip_creole_for_id(s: &str) -> String {
    let mut out = s.to_string();
    for marker in &["**", "//", "__", "--"] {
        out = out.replace(marker, "");
    }
    let mut result = String::new();
    let mut in_tag = false;
    for ch in out.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join("_")
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
        assert_eq!(d.entities[0].id, "Container<T>");
        assert_eq!(d.entities[1].id, "Map<K, V>");
    }

    #[test]
    fn separators() {
        let d = parse("class Foo {\n  +field1\n  --\n  +method1()\n  ==\n  -internal\n}");
        // Separators are ignored, members are parsed.
        assert_eq!(d.entities[0].members.len(), 3);
    }

    #[test]
    fn directed_association() {
        let d = parse("A --> B : uses");
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].kind, RelationshipKind::Association);
        assert_eq!(d.relationships[0].label.as_deref(), Some("uses"));
    }

    #[test]
    fn relationship_multiplicity() {
        let d = parse(r#"Company "1" o-- "1..*" Department"#);
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from_multiplicity.as_deref(), Some("1"));
        assert_eq!(d.relationships[0].to_multiplicity.as_deref(), Some("1..*"));
    }

    #[test]
    fn nested_package() {
        let d = parse(
            "cloud Outer {\n  cloud Inner {\n    class MyClass {\n      +void method()\n    }\n  }\n}",
        );
        assert_eq!(d.packages.len(), 2);
        assert_eq!(d.packages[0].name, "Outer");
        assert_eq!(d.packages[1].name, "Inner");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].id, "MyClass");
    }
}
