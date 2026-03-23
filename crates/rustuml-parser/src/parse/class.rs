// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::class::*;

/// Which multi-line meta block we are currently accumulating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetaBlock {
    Header,
    Footer,
    Legend,
    Caption,
    Title,
}

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
    /// Whether `set namespaceSeparator none` was seen (dots allowed in class names).
    namespace_sep_none: bool,
    /// Namespace separator string (default "."; `None` when `set namespaceSeparator none`).
    namespace_sep: Option<String>,
    /// Whether we are inside a multi-line header/footer/legend block.
    meta_block: Option<MetaBlock>,
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
            namespace_sep_none: false,
            namespace_sep: Some(".".to_string()),
            meta_block: None,
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

    /// Given a fully-qualified class name like `com.example.MyClass`, ensure
    /// intermediate namespace packages exist and register the entity in them.
    /// Returns `(entity_id, entity_label)` where `entity_id` is the full qualified
    /// name and `entity_label` is just the last segment (short name).
    ///
    /// If no separator is active or the name contains no separator, returns
    /// `(qualified, qualified)` unchanged.
    fn ensure_namespace_packages(&mut self, qualified: &str) -> (String, String) {
        let sep = match self.namespace_sep.clone() {
            Some(s) if !s.is_empty() => s,
            _ => return (qualified.to_string(), qualified.to_string()),
        };
        let parts: Vec<&str> = qualified.split(sep.as_str()).collect();
        if parts.len() < 2 {
            return (qualified.to_string(), qualified.to_string());
        }

        // Build the namespace package hierarchy for all segments except the last.
        let mut prefix = String::new();
        let mut parent_pkg_idx: Option<usize> = self.package_stack.last().copied();
        for part in &parts[..parts.len() - 1] {
            if !prefix.is_empty() {
                prefix.push_str(&sep);
            }
            prefix.push_str(part);
            let pkg_id = prefix.clone();
            let pkg_label = part.to_string();

            let pkg_idx = if let Some(idx) = self.packages.iter().position(|p| p.name == pkg_id) {
                idx
            } else {
                let new_idx = self.packages.len();
                self.packages.push(Package {
                    name: pkg_id.clone(),
                    kind: PackageKind::Package,
                    color: None,
                    entities: Vec::new(),
                    stereotypes: Vec::new(),
                    display_name: Some(pkg_label),
                });
                // Register this pkg in its parent package's entity list.
                if let Some(p_idx) = parent_pkg_idx {
                    let parent = &mut self.packages[p_idx];
                    if !parent.entities.contains(&pkg_id) {
                        parent.entities.push(pkg_id.clone());
                    }
                }
                new_idx
            };
            parent_pkg_idx = Some(pkg_idx);
        }

        // Register the entity in all namespace packages (innermost = last namespace segment).
        let entity_id = qualified.to_string();
        if let Some(innermost_idx) = parent_pkg_idx {
            // Register entity in innermost namespace package.
            let pkg = &mut self.packages[innermost_idx];
            if !pkg.entities.contains(&entity_id) {
                pkg.entities.push(entity_id.clone());
            }
            // Also register in outer namespace packages and any active user packages.
            // Build list of all ancestor namespace pkg indices.
            let mut prefix2 = String::new();
            let mut ancestor_indices = Vec::new();
            for part in &parts[..parts.len() - 1] {
                if !prefix2.is_empty() {
                    prefix2.push_str(&sep);
                }
                prefix2.push_str(part);
                if let Some(idx) = self.packages.iter().position(|p| p.name == prefix2) {
                    ancestor_indices.push(idx);
                }
            }
            // Register in all ancestors (except innermost already done).
            for &idx in ancestor_indices.iter().rev().skip(1) {
                let pkg = &mut self.packages[idx];
                if !pkg.entities.contains(&entity_id) {
                    pkg.entities.push(entity_id.clone());
                }
            }
        }
        // Also register in any active user-defined package scopes.
        for &pkg_idx in &self.package_stack {
            let pkg = &mut self.packages[pkg_idx];
            if !pkg.entities.contains(&entity_id) {
                pkg.entities.push(entity_id.clone());
            }
        }

        let entity_label = parts[parts.len() - 1].to_string();
        (entity_id, entity_label)
    }

    /// The index of the innermost active package scope (top of the stack).
    fn current_package(&self) -> Option<usize> {
        self.package_stack.last().copied()
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
        // Inside a multi-line meta block (header/footer/legend/caption/title)?
        if let Some(block) = self.meta_block {
            let end1 = match block {
                MetaBlock::Header => "endheader",
                MetaBlock::Footer => "endfooter",
                MetaBlock::Legend => "endlegend",
                MetaBlock::Caption => "endcaption",
                MetaBlock::Title => "end title",
            };
            let end2 = match block {
                MetaBlock::Header => "end header",
                MetaBlock::Footer => "end footer",
                MetaBlock::Legend => "end legend",
                MetaBlock::Caption => "end caption",
                MetaBlock::Title => "end title",
            };
            if line == end1 || line == end2 {
                self.meta_block = None;
            } else {
                match block {
                    MetaBlock::Header => {
                        let h = self.meta.header.get_or_insert_with(String::new);
                        if !h.is_empty() { h.push(' '); }
                        h.push_str(line);
                    }
                    MetaBlock::Footer => {
                        let f = self.meta.footer.get_or_insert_with(String::new);
                        if !f.is_empty() { f.push(' '); }
                        f.push_str(line);
                    }
                    MetaBlock::Legend => {
                        let l = self.meta.legend.get_or_insert_with(String::new);
                        if !l.is_empty() { l.push(' '); }
                        l.push_str(line);
                    }
                    MetaBlock::Caption => {
                        let c = self.meta.caption.get_or_insert_with(String::new);
                        if !c.is_empty() { c.push(' '); }
                        c.push_str(line);
                    }
                    MetaBlock::Title => {
                        let t = self.meta.title.get_or_insert_with(String::new);
                        if !t.is_empty() { t.push(' '); }
                        t.push_str(line);
                    }
                }
            }
            return Ok(());
        }

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
            //   keyword id             — group 3 (id only, word chars only)
            //   keyword "label"        — group 4 (quoted-only, no `as`)
            // `abstract` without `class` is also a valid class keyword.
            Regex::new(
                r#"^(class|abstract\s+class|abstract|interface|enum|annotation|entity)\s+(?:(?:"([^"]+)"\s+as\s+)?(\w+(?:<[^>]+>)?)|"([^"]+)")"#,
            )
            .unwrap()
        });
        // Same as RE but allows dots in the identifier (for `set namespaceSeparator none`).
        static RE_DOTTED: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(class|abstract\s+class|abstract|interface|enum|annotation|entity)\s+(?:(?:"([^"]+)"\s+as\s+)?(\w[\w.]*(?:<[^>]+>)?)|"([^"]+)")"#,
            )
            .unwrap()
        });
        // Permissive regex: accepts any non-whitespace name (for custom namespace separators).
        static RE_PERMISSIVE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(class|abstract\s+class|abstract|interface|enum|annotation|entity)\s+(?:(?:"([^"]+)"\s+as\s+)?([^\s{<>]+(?:<[^>]+>)?)|"([^"]+)")"#,
            )
            .unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)>>").unwrap());

        let re = if self.namespace_sep_none {
            // `namespaceSeparator none` — dots are part of the name, no splitting.
            &*RE_DOTTED
        } else if self.namespace_sep.as_deref() == Some(".") {
            // Default "." separator: use dotted regex so `com.example.MyClass` is captured fully.
            &*RE_DOTTED
        } else {
            // Custom separator (e.g. "::" or "/") or no separator: use permissive regex.
            &*RE_PERMISSIVE
        };
        if let Some(caps) = re.captures(line) {
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
                .map(|c| process_spot_stereotype(c[1].trim()))
                .filter(|s| !s.is_empty())
                .collect();

            // Handle namespace separation: split `com.example.MyClass` or `com::example::MyClass`
            // into package hierarchy + short entity name.  For default separator ".", we must
            // use RE_DOTTED (which already matches dotted names) — but for the default RE above
            // (`\w+`), dots aren't matched so id won't contain them.  In the permissive case
            // (custom separator), `id` may contain the separator.
            //
            // Additionally, even when using the default "." separator but the user declared
            // `class com.example.MyClass`, the permissive re wasn't used — we use a second
            // pass to split names that contain the default separator even when using RE.
            let (final_id, final_label) = if caps.get(2).is_none() {
                // Only apply namespace splitting to unquoted, non-aliased names.
                self.ensure_namespace_packages(&id)
            } else {
                (id.clone(), label.clone())
            };

            // Use the namespace-derived label if available (not aliased).
            let display_label = if caps.get(2).is_none() && final_label != final_id {
                final_label.clone()
            } else {
                label
            };

            if let Some(entity) = self.find_entity_mut(&final_id) {
                entity.kind = kind;
                entity.label = display_label;
                entity.stereotypes.extend(stereotypes);
            } else {
                self.entities.push(ClassEntity {
                    id: final_id.clone(),
                    label: display_label,
                    kind,
                    members: Vec::new(),
                    stereotypes,
                });
            }

            // Register entity in ALL active packages (innermost to outermost),
            // so that outer container bounding boxes include entities from inner
            // nested packages.
            // (Note: namespace package registration was already done in ensure_namespace_packages)
            for &pkg_idx in &self.package_stack {
                let pkg = &mut self.packages[pkg_idx];
                if !pkg.entities.contains(&final_id) {
                    pkg.entities.push(final_id.clone());
                }
            }

            if line.ends_with('{') || line.ends_with("{{") {
                self.current_entity = Some(final_id.clone());
            }
            self.last_entity_id = Some(final_id);
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
        //                   <-->, <..>, --, -->, <--, <-, ->, .., ..>, <..
        //                   <|--|> (bidirectional inheritance), <..|.> etc.
        // Multiple dashes (e.g. ---- or ------) are treated as plain association.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^([\w.]+)\s*(?:"([^"]+)")?\s*((?:<\|--\|>|<\.\.>|<\|--|--\|>|\.\.\|>|<\|\.\.|<\.\.|\*--|--\*|o--|--o|<-->|<--|-->|->|<-|-{2,}|\.\.|\.\.>))\s*(?:"([^"]+)")?\s*([\w.]+)(?:\s*:\s*(.+))?$"#,
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

        // Lollipop notation: `Foo -() Interface` or `Foo --() Interface` (provided interface)
        // or `Foo ()- Interface` or `Foo ()-- Interface` (required interface).
        static LOLLIPOP_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(\w+)\s*-{1,2}\(\)\s*(\w+)$|^(\w+)\s*\(\)-{1,2}\s*(\w+)$").unwrap()
        });
        if let Some(caps) = LOLLIPOP_RE.captures(line) {
            let (from_raw, to_raw) = if caps.get(1).is_some() {
                (caps[1].to_string(), caps[2].to_string())
            } else {
                (caps[3].to_string(), caps[4].to_string())
            };
            // Ensure the interface entity exists.
            if !self.entities.iter().any(|e| e.id == to_raw) {
                self.entities.push(ClassEntity {
                    id: to_raw.clone(),
                    label: to_raw.clone(),
                    kind: EntityKind::Interface,
                    members: Vec::new(),
                    stereotypes: Vec::new(),
                });
            }
            let from = self.ensure_entity(&from_raw);
            self.relationships.push(Relationship {
                from,
                to: to_raw,
                kind: RelationshipKind::Association,
                label: None,
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
                r#"^(package|namespace|cloud|database|folder|frame|rectangle|node)\s+(?:"([^"]+)"|([^#\s{<]+))\s*(?:#([^\s{<]+))?\s*(?:<<\s*([^>]+?)\s*>>)?\s*\{?"#,
            )
            .unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)>>").unwrap());

        if let Some(caps) = RE.captures(line) {
            let kind_str = &caps[1];
            let name = caps
                .get(2)
                .or(caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let color = caps.get(4).map(|m| m.as_str().to_string());
            let stereotypes: Vec<String> = STEREOTYPE_RE
                .captures_iter(line)
                .map(|c| c[1].trim().to_string())
                .collect();
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
                stereotypes,
                display_name: None,
            });
            true
        } else {
            false
        }
    }


    fn try_note(&mut self, line: &str) -> bool {
        // Single-line attached note: `note <pos> of <entity> : <text>`
        // Entity may be a dotted name (e.g. `domain.User` in namespace diagrams).
        static ATTACHED_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s+of\s+([\w.]+)\s*:\s*(.+)$").unwrap()
        });
        // Multi-line attached note start: `note <pos> of <entity>` (optional color: `#color`)
        static ATTACHED_ML_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s+of\s+([\w.]+)\s*(?:#\S+)?\s*$").unwrap()
        });
        // Shorthand single-line note attached to last entity: `note <pos> : <text>`
        static SHORT_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s*:\s*(.+)$").unwrap()
        });
        // Shorthand multi-line note attached to last entity: `note <pos>` (optional color)
        static SHORT_ML_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(top|bottom|left|right)\s*(?:#\S+)?\s*$").unwrap()
        });
        // Floating named note: `note "text" as Name`
        static FLOATING_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap()
        });
        // Multi-line floating note: `note as Name` (optional color suffix like `#yellow`).
        static FLOATING_ML_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+as\s+(\w+)\s*(?:#\S+)?\s*$").unwrap()
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

        // `note on link : text` or `note on link: text` — inline single-line note on the last relationship.
        let note_on_link_text = line
            .strip_prefix("note on link :")
            .or_else(|| line.strip_prefix("note on link:"));
        if let Some(text) = note_on_link_text {
            let text = text.trim().to_string();
            let lines = if text.is_empty() {
                Vec::new()
            } else {
                text.split("\\n").map(|s| s.trim().to_string()).collect()
            };
            self.notes.push(Note {
                lines,
                target: None,
                position: None,
                alias: None,
            });
            return true;
        }
        // `note on link` — multi-line note attached to the last relationship.
        if line == "note on link" {
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
        if line == "title" {
            self.meta_block = Some(MetaBlock::Title);
            return true;
        }
        if let Some(rest) = line.strip_prefix("header ") {
            self.meta.header = Some(rest.trim().to_string());
            return true;
        }
        if line == "header"
            || line.starts_with("left header")
            || line.starts_with("right header")
            || line.starts_with("center header")
        {
            self.meta_block = Some(MetaBlock::Header);
            return true;
        }
        if let Some(rest) = line.strip_prefix("footer ") {
            self.meta.footer = Some(rest.trim().to_string());
            return true;
        }
        if line == "footer"
            || line.starts_with("left footer")
            || line.starts_with("right footer")
            || line.starts_with("center footer")
        {
            self.meta_block = Some(MetaBlock::Footer);
            return true;
        }
        if let Some(rest) = line.strip_prefix("caption ") {
            self.meta.caption = Some(rest.trim().to_string());
            return true;
        }
        if line == "caption" {
            self.meta_block = Some(MetaBlock::Caption);
            return true;
        }
        if line == "legend"
            || line == "legend left"
            || line == "legend right"
            || line == "legend center"
            || line == "legend top"
            || line == "legend bottom"
        {
            self.meta_block = Some(MetaBlock::Legend);
            return true;
        }
        if line == "set namespaceSeparator none" {
            self.namespace_sep_none = true;
            self.namespace_sep = None;
            return true;
        }
        if let Some(sep) = line.strip_prefix("set namespaceSeparator ") {
            let sep = sep.trim();
            if sep.is_empty() || sep == "." {
                self.namespace_sep = Some(".".to_string());
            } else {
                self.namespace_sep = Some(sep.to_string());
            }
            return true;
        }
        // Parse skinparam key value (store for renderer use).
        if let Some(rest) = line.strip_prefix("skinparam ") {
            if let Some((key, value)) = rest.split_once(' ') {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            return true;
        }
        // Skip hide, show, together, etc.
        line.starts_with("hide ")
            || line.starts_with("show ")
            || line.starts_with("together")
            || line.starts_with("allowmixing")
            || line.starts_with("map ")
            || line.starts_with("object ")
            || line.starts_with("set ")
    }

    fn parse_member_line(&mut self, line: &str) {
        static SEPARATOR_LABELED_RE: LazyLock<Regex> = LazyLock::new(|| {
            // Matches labeled separators: -- label --, == label ==, __ label __, .. label ..
            // Captures the label text (trimmed) in group 2.
            Regex::new(r"^(--|==|__|\.\.)\s+(.+?)\s+(--|==|__|\.\.)\s*$").unwrap()
        });

        let trimmed = line.trim();
        // Empty or brace-only.
        if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
            return;
        }

        // Labeled separator — store as Separator member so the label can be rendered.
        if let Some(caps) = SEPARATOR_LABELED_RE.captures(trimmed) {
            let label = caps[2].to_string();
            let member = Member {
                name: label.clone(),
                return_type: None,
                visibility: Visibility::Default,
                is_static: false,
                is_abstract: false,
                kind: MemberKind::Separator,
                display_text: label,
            };
            if let Some(entity_id) = &self.current_entity
                && let Some(entity) = self.entities.iter_mut().find(|e| e.id == *entity_id)
            {
                entity.members.push(member);
            }
            return;
        }

        // Bare separator lines — render as unlabeled separator.
        if trimmed == "--" || trimmed == ".." || trimmed == "==" || trimmed == "__" {
            let member = Member {
                name: String::new(),
                return_type: None,
                visibility: Visibility::Default,
                is_static: false,
                is_abstract: false,
                kind: MemberKind::Separator,
                display_text: String::new(),
            };
            if let Some(entity_id) = &self.current_entity
                && let Some(entity) = self.entities.iter_mut().find(|e| e.id == *entity_id)
            {
                entity.members.push(member);
            }
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
        "abstract class" | "abstract" => EntityKind::AbstractClass,
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
    } else if s.contains("..>") || s == "->" || s == "<-" {
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
    // PlantUML treats `\\` as an escaped backslash (renders as single `\`).
    let display_text = rest.replace("\\\\", "\\");

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
/// Process a stereotype string that may contain spot notation `(S,#color) Name`.
///
/// PlantUML's behavior:
/// - If the color is a named color (e.g. `#red`, `#blue`), keep the full `(S,#color) Name` prefix.
/// - If the color is a hex code (e.g. `#FF7700`, `#00AAFF`), strip the `(S,#color)` prefix
///   and return just the name.
fn process_spot_stereotype(s: &str) -> String {
    let s = s.trim();
    // Look for spot notation: `(X,#color) Name`
    if let Some(rest) = s.strip_prefix('(') {
        if let Some(close) = rest.find(')') {
            let spot_inner = &rest[..close];
            let after = rest[close + 1..].trim();
            // spot_inner should be like `A,#red` or `F,#FF7700`
            if let Some(comma) = spot_inner.find(',') {
                let color_part = spot_inner[comma + 1..].trim();
                if color_part.starts_with('#') {
                    let color_hex = &color_part[1..]; // strip leading #
                    let is_hex = !color_hex.is_empty()
                        && color_hex.chars().all(|c| c.is_ascii_hexdigit());
                    if is_hex {
                        // Hex color: strip spot prefix, return just the name.
                        return after.to_string();
                    }
                }
            }
        }
    }
    // Named color or no spot notation: return as-is.
    s.to_string()
}

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
