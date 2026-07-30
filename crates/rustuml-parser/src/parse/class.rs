// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram parser.

use std::collections::HashMap;
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

#[derive(Clone, Copy)]
enum QuarkLookup {
    CurrentContext,
    ReuseUnique,
}

/// Parse preprocessed lines into a class diagram.
pub fn parse_class(lines: &[String]) -> Result<ClassDiagram, ParseError> {
    let mut parser = ClassParser::new();

    for (i, line) in lines.iter().enumerate() {
        let (source_line, trimmed) = super::source_line_and_trimmed(i + 1, line);
        if parser.current_note.is_some() {
            // Java `CommandFactoryNote.createMultiLine` delegates to
            // `BlocLines.removeEmptyColumns`, which removes only common
            // leading columns. Terminal spaces remain in the Display and
            // therefore contribute to note layout.
            parser.parse_line(source_line, super::source_text(line))?;
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        parser.parse_line(source_line, trimmed)?;
    }

    Ok(parser.finish())
}

struct ClassParser {
    meta: DiagramMeta,
    direction: ClassLayoutDirection,
    entities: Vec<ClassEntity>,
    relationships: Vec<Relationship>,
    association_classes: Vec<crate::diagram::class::AssociationClass>,
    uid_events: Vec<ClassUidEvent>,
    together: Vec<crate::diagram::class::TogetherGroup>,
    packages: Vec<Package>,
    notes: Vec<Note>,
    /// Entity currently being parsed (inside { ... } block).
    current_entity: Option<String>,
    /// The next non-closing line supplies the location used by Java's
    /// multiline class command for the entity and its declaration links.
    current_entity_needs_body_location: bool,
    /// Stack of active package indices (innermost last), supporting nested packages.
    package_stack: Vec<usize>,
    scope_stack: Vec<ClassScope>,
    /// Canonical quark path for each package, parallel to `packages`.
    package_paths: Vec<Vec<String>>,
    /// Package lookup by canonical quark path.
    package_by_path: HashMap<Vec<String>, usize>,
    /// Entity lookup by canonical quark path.
    entity_by_path: HashMap<Vec<String>, usize>,
    /// Named note lookup in the same canonical quark namespace.
    note_by_path: HashMap<Vec<String>, usize>,
    /// Canonical quark paths in the order PlantUML's Plasma tree creates them.
    quark_creation_order: Vec<Vec<String>>,
    /// Note currently being accumulated (multi-line `note ... end note`).
    current_note: Option<Note>,
    /// ID of the last declared entity (for shorthand `note right : text`).
    last_entity_id: Option<String>,
    /// Namespace separator string (default "."; `None` when `set namespaceSeparator none`).
    namespace_sep: Option<String>,
    /// Whether we are inside a multi-line header/footer/legend block.
    meta_block: Option<MetaBlock>,
    /// Prefix of an open `skinparam X { ... }` block.
    current_skinparam_prefix: Option<String>,
    /// Current 1-based source line number (set before each parse_line call).
    current_line: usize,
    /// Accumulated `hide` / `show` directives, in source order.
    hide_show: Vec<crate::diagram::class::HideShow>,
    header_line: Option<usize>,
    footer_line: Option<usize>,
    title_line: Option<usize>,
    caption_line: Option<usize>,
    legend_line: Option<usize>,
}

#[derive(Clone, Copy)]
enum ClassScope {
    Package,
    Together(usize),
}

impl ClassParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            direction: ClassLayoutDirection::TopToBottom,
            entities: Vec::new(),
            relationships: Vec::new(),
            association_classes: Vec::new(),
            uid_events: Vec::new(),
            together: Vec::new(),
            packages: Vec::new(),
            notes: Vec::new(),
            current_entity: None,
            current_entity_needs_body_location: false,
            package_stack: Vec::new(),
            scope_stack: Vec::new(),
            package_paths: Vec::new(),
            package_by_path: HashMap::new(),
            entity_by_path: HashMap::new(),
            note_by_path: HashMap::new(),
            quark_creation_order: Vec::new(),
            current_note: None,
            last_entity_id: None,
            namespace_sep: Some(".".to_string()),
            meta_block: None,
            current_skinparam_prefix: None,
            current_line: 0,
            hide_show: Vec::new(),
            header_line: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        }
    }

    fn finish(self) -> ClassDiagram {
        ClassDiagram {
            meta: self.meta,
            direction: self.direction,
            uid_events: self.uid_events,
            entities: self.entities,
            relationships: self.relationships,
            association_classes: self.association_classes,
            together: self.together,
            packages: self.packages,
            notes: self.notes,
            hide_show: self.hide_show,
            header_line: self.header_line,
            footer_line: self.footer_line,
            title_line: self.title_line,
            caption_line: self.caption_line,
            legend_line: self.legend_line,
        }
    }

    fn push_relationship(&mut self, relationship: Relationship) {
        self.uid_events
            .push(ClassUidEvent::Relationship(self.relationships.len()));
        self.relationships.push(relationship);
    }

    fn take_relationship(&mut self, index: usize) -> Relationship {
        let relationship = self.relationships.remove(index);
        for event in &mut self.uid_events {
            match event {
                ClassUidEvent::Relationship(event_index) if *event_index == index => {
                    *event = ClassUidEvent::DiscardedRelationship {
                        inverted: relationship.style.inverted,
                    };
                }
                ClassUidEvent::Relationship(event_index) if *event_index > index => {
                    *event_index -= 1;
                }
                _ => {}
            }
        }
        relationship
    }

    fn push_note(&mut self, mut note: Note) {
        if let Some(alias) = note.alias.as_deref() {
            // `CommandFactoryNote` uses `quarkInContext(false, alias)`: the
            // note is real data on a current-package quark, not a side table
            // entry repaired after relationship parsing.
            let path = self.resolve_quark_path(alias, QuarkLookup::CurrentContext);
            let id = self.path_id(&path);
            self.register_quark_path(&path);
            self.note_by_path.insert(path, self.notes.len());
            note.id = Some(id);
        }
        self.uid_events.push(ClassUidEvent::Note {
            index: self.notes.len(),
            owner_package: self
                .package_stack
                .last()
                .map(|&index| self.packages[index].name.clone()),
        });
        self.notes.push(note);
    }

    fn current_group_path(&self) -> &[String] {
        self.package_stack
            .last()
            .map(|&idx| self.package_paths[idx].as_slice())
            .unwrap_or(&[])
    }

    fn identity_separator(&self) -> &str {
        self.namespace_sep.as_deref().unwrap_or(".")
    }

    fn path_id(&self, path: &[String]) -> String {
        path.join(self.identity_separator())
    }

    fn split_identity(&self, raw: &str) -> (bool, Vec<String>) {
        let raw = raw.trim();
        let Some(separator) = self.namespace_sep.as_deref() else {
            return (false, vec![raw.to_string()]);
        };
        let rooted = raw.starts_with(separator);
        let raw = if rooted {
            raw.trim_start_matches(separator)
        } else {
            raw
        };
        (
            rooted,
            raw.split(separator)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect(),
        )
    }

    fn unique_quark_path_named(&self, name: &str) -> Option<Vec<String>> {
        let mut matches = self
            .quark_creation_order
            .iter()
            .filter(|path| path.last().is_some_and(|part| part == name));
        let first = matches.next()?.clone();
        matches.next().is_none().then_some(first)
    }

    fn first_quark_path_named(&self, name: &str) -> Option<Vec<String>> {
        self.quark_creation_order
            .iter()
            .find(|path| path.last().is_some_and(|part| part == name))
            .cloned()
    }

    fn register_quark_path(&mut self, path: &[String]) {
        for depth in 1..=path.len() {
            let prefix = &path[..depth];
            if !self
                .quark_creation_order
                .iter()
                .any(|existing| existing == prefix)
            {
                self.quark_creation_order.push(prefix.to_vec());
            }
        }
    }

    /// Mirrors `CucaDiagram#quarkInContextSafe`: qualified names resolve from
    /// the root when their first group exists, otherwise from the current
    /// group; a reusable unqualified name binds to its sole existing quark.
    fn resolve_quark_path(&self, raw: &str, lookup: QuarkLookup) -> Vec<String> {
        let raw = raw.trim();
        if self.namespace_sep.is_none() {
            // `CucaDiagram#quarkInContextSafe` bypasses both lookup modes
            // here and delegates directly to creation-ordered
            // `Plasma#firstWithName`.
            if let Some(path) = self.first_quark_path_named(raw) {
                return path;
            }
            let mut path = self.current_group_path().to_vec();
            path.push(raw.to_string());
            return path;
        }

        let (rooted, parts) = self.split_identity(raw);
        if rooted || self.entity_by_path.contains_key(&parts) {
            return parts;
        }
        if parts.len() == 1 {
            if matches!(lookup, QuarkLookup::ReuseUnique)
                && let Some(path) = self.unique_quark_path_named(&parts[0])
            {
                return path;
            }
            let mut path = self.current_group_path().to_vec();
            path.extend(parts);
            return path;
        }
        if self.package_by_path.contains_key(&vec![parts[0].clone()]) {
            return parts;
        }
        let mut path = self.current_group_path().to_vec();
        path.extend(parts);
        path
    }

    fn resolve_group_path(&self, raw: &str) -> Vec<String> {
        let raw = raw.trim();
        if self.namespace_sep.is_none() {
            // Package commands call the same `CucaDiagram#quarkInContextSafe`
            // path as entity commands. With a null separator Java delegates
            // to creation-ordered `Plasma#firstWithName` across the shared
            // entity/package quark namespace.
            if let Some(path) = self.first_quark_path_named(raw) {
                return path;
            }
            let mut path = self.current_group_path().to_vec();
            path.push(raw.to_string());
            return path;
        }

        let (rooted, parts) = self.split_identity(raw);
        if rooted {
            return parts;
        }
        if parts.len() > 1 && self.package_by_path.contains_key(&vec![parts[0].clone()]) {
            return parts;
        }
        let mut path = self.current_group_path().to_vec();
        path.extend(parts);
        path
    }

    fn register_entity_path(&mut self, path: &[String], entity_id: &str) {
        for (package_idx, package_path) in self.package_paths.iter().enumerate() {
            if package_path.len() < path.len() && path.starts_with(package_path) {
                let package = &mut self.packages[package_idx];
                if !package.entities.iter().any(|id| id == entity_id) {
                    package.entities.push(entity_id.to_string());
                }
            }
        }
    }

    fn current_together(&self) -> Option<usize> {
        match self.scope_stack.last() {
            Some(ClassScope::Together(group_idx)) => Some(*group_idx),
            Some(ClassScope::Package) | None => None,
        }
    }

    fn enroll_new_entity_in_current_together(&mut self, entity_id: &str) {
        let Some(group_idx) = self.current_together() else {
            return;
        };
        self.together[group_idx]
            .entities
            .push(entity_id.to_string());
    }

    /// `CucaDiagram#eventuallyBuildPhantomGroups` scans every registered
    /// data-less quark after a class-like leaf constructor. Quarks with
    /// children become package entities in global Plasma creation order.
    fn materialize_phantom_packages(&mut self) {
        let paths = self.quark_creation_order.clone();
        for path in paths {
            let has_children = self
                .quark_creation_order
                .iter()
                .any(|candidate| candidate.len() > path.len() && candidate.starts_with(&path));
            if !has_children || self.entity_by_path.contains_key(&path) {
                continue;
            }
            if let Some(&idx) = self.package_by_path.get(&path) {
                if self.packages[idx].phantom && self.packages[idx].source_line == 0 {
                    self.packages[idx].source_line = self.current_line;
                    self.uid_events
                        .push(ClassUidEvent::Package(self.packages[idx].name.clone()));
                }
                continue;
            }
            let parent = path
                .split_last()
                .and_then(|(_, parent)| self.package_by_path.get(parent).copied());
            let idx = self.packages.len();
            let name = self.path_id(&path);
            self.uid_events.push(ClassUidEvent::Package(name.clone()));
            self.packages.push(Package {
                name,
                kind: PackageKind::Package,
                color: None,
                url: None,
                url_tooltip: None,
                entities: Vec::new(),
                parent,
                source_line: self.current_line,
                stereotypes: Vec::new(),
                symbol_from_keyword: false,
                display_name: path.last().cloned(),
                phantom: true,
            });
            self.package_paths.push(path.clone());
            self.package_by_path.insert(path.clone(), idx);
        }
    }

    fn create_entity_at_path(
        &mut self,
        path: Vec<String>,
        label: String,
        kind: EntityKind,
        explicit_alias: bool,
    ) -> String {
        let id = self.path_id(&path);
        self.register_quark_path(&path);
        // `reallyCreateLeaf` consumes the entity UID before
        // `eventuallyBuildPhantomGroups` performs its global quark sweep.
        self.uid_events.push(ClassUidEvent::Entity(id.clone()));
        let idx = self.entities.len();
        self.entities.push(ClassEntity {
            id: id.clone(),
            label,
            explicit_alias,
            kind,
            members: Vec::new(),
            stereotypes: Vec::new(),
            generic: None,
            spot_color: None,
            spot_character: None,
            url: None,
            url_tooltip: None,
            color: None,
            text_color: None,
            line_color: None,
            line_style: None,
            source_line: self.current_line,
        });
        self.entity_by_path.insert(path.clone(), idx);
        self.materialize_phantom_packages();
        self.register_entity_path(&path, &id);
        self.enroll_new_entity_in_current_together(&id);
        id
    }

    fn ensure_entity(&mut self, raw: &str) -> String {
        let path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique);
        if let Some(&idx) = self.entity_by_path.get(&path) {
            return self.entities[idx].id.clone();
        }
        let label = path.last().cloned().unwrap_or_default();
        self.create_entity_at_path(path, label, EntityKind::Class, false)
    }

    fn ensure_entity_kind(&mut self, raw: &str, kind: EntityKind) -> String {
        let path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique);
        if let Some(&idx) = self.entity_by_path.get(&path) {
            return self.entities[idx].id.clone();
        }
        let label = path.last().cloned().unwrap_or_default();
        self.create_entity_at_path(path, label, kind, false)
    }

    fn relationship_endpoint_path(&self, raw: &str, quoted: bool) -> Vec<String> {
        // `CommandLinkClass#executeArg` calls
        // `CucaDiagram#quarkInContextSafe(true, endpoint)`. An existing group
        // quark is retained and may later be muted to `EMPTY_PACKAGE`; only a
        // genuinely missing quark is created as `LeafType.CLASS`.
        if quoted {
            let exact_path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique);
            if self.entity_by_path.contains_key(&exact_path)
                || self.package_by_path.contains_key(&exact_path)
            {
                return exact_path;
            }
            return self.resolve_quark_path(&strip_creole_for_id(raw), QuarkLookup::ReuseUnique);
        }
        self.resolve_quark_path(raw, QuarkLookup::ReuseUnique)
    }

    fn materialize_relationship_endpoint(&mut self, path: Vec<String>) -> String {
        if let Some(&idx) = self.entity_by_path.get(&path) {
            return self.entities[idx].id.clone();
        }
        if let Some(&idx) = self.note_by_path.get(&path) {
            return self.notes[idx]
                .id
                .clone()
                .unwrap_or_else(|| self.path_id(&path));
        }
        if let Some(&idx) = self.package_by_path.get(&path) {
            return self.packages[idx].name.clone();
        }
        let label = path.last().cloned().unwrap_or_default();
        self.create_entity_at_path(path, label, EntityKind::Class, false)
    }

    fn resolve_note_target(&self, raw: &str) -> String {
        let path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique);
        if let Some(&idx) = self.entity_by_path.get(&path) {
            return self.entities[idx].id.clone();
        }
        if let Some(&idx) = self.package_by_path.get(&path) {
            return self.packages[idx].name.clone();
        }
        raw.to_string()
    }

    fn find_entity_mut(&mut self, id: &str) -> Option<&mut ClassEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    fn materialize_active_phantom_packages(&mut self) {
        for package_idx in self.package_stack.clone() {
            let mut newly_materialized = Vec::new();
            let mut parent = self.packages[package_idx].parent;
            while let Some(parent_idx) = parent {
                let package = &mut self.packages[parent_idx];
                if package.phantom && package.source_line == 0 {
                    package.source_line = self.current_line;
                    newly_materialized.push(parent_idx);
                }
                parent = package.parent;
            }
            for parent_idx in newly_materialized.into_iter().rev() {
                self.uid_events.push(ClassUidEvent::Package(
                    self.packages[parent_idx].name.clone(),
                ));
            }
        }
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        self.current_line = line_num;
        // Inside a grouped skinparam block?
        if let Some(prefix) = self.current_skinparam_prefix.clone() {
            if line == "}" {
                self.current_skinparam_prefix = None;
            } else if let Some((key, value)) = line.split_once(char::is_whitespace) {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() && !value.is_empty() {
                    self.meta.skinparams.push(crate::diagram::SkinParam {
                        key: format!("{prefix}{key}"),
                        value: value.to_string(),
                    });
                }
            }
            return Ok(());
        }

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
                        if !h.is_empty() {
                            h.push('\n');
                        }
                        h.push_str(line);
                    }
                    MetaBlock::Footer => {
                        let f = self.meta.footer.get_or_insert_with(String::new);
                        if !f.is_empty() {
                            f.push('\n');
                        }
                        f.push_str(line);
                    }
                    MetaBlock::Legend => {
                        let l = self.meta.legend.get_or_insert_with(String::new);
                        if !l.is_empty() {
                            l.push('\n');
                        }
                        l.push_str(line);
                    }
                    MetaBlock::Caption => {
                        let c = self.meta.caption.get_or_insert_with(String::new);
                        if !c.is_empty() {
                            c.push('\n');
                        }
                        c.push_str(line);
                    }
                    MetaBlock::Title => {
                        let t = self.meta.title.get_or_insert_with(String::new);
                        if !t.is_empty() {
                            t.push('\n');
                        }
                        t.push_str(line);
                    }
                }
            }
            return Ok(());
        }

        // Inside a multi-line note?
        if self.current_note.is_some() {
            if line.trim() == "end note" {
                let mut note = self.current_note.take().unwrap();
                if note.lines.is_empty() {
                    // A body line that is itself an inline diagram directive
                    // is consumed by preprocessing. Java still attributes the
                    // empty note entity to that body location, immediately
                    // after the declaration.
                    note.source_line = note.source_line.saturating_add(1);
                }
                dedent_note_lines(&mut note.lines);
                self.push_note(note);
            } else if let Some(note) = self.current_note.as_mut() {
                if note.lines.is_empty() {
                    // `CommandFactoryNote` attributes a multiline note entity
                    // to its first body line. An inline nested `@startuml ...
                    // @enduml` can retain the declaration's preprocessor
                    // marker; advance to the adjacent body line in that one
                    // ambiguous case.
                    note.source_line = if self.current_line == note.source_line {
                        note.source_line + 1
                    } else {
                        self.current_line
                    };
                }
                note.lines.push(line.to_string());
            }
            return Ok(());
        }

        // Inside a class body?
        if self.current_entity.is_some() {
            if line == "}" || line == "}}" {
                self.current_entity = None;
                self.current_entity_needs_body_location = false;
                return Ok(());
            }
            if self.current_entity_needs_body_location {
                self.attribute_multiline_declaration_to_first_body_line();
                self.current_entity_needs_body_location = false;
            }
            self.parse_member_line(line);
            return Ok(());
        }

        // Closing brace: leave the exact structural scope that opened it.
        if line == "}" {
            match self.scope_stack.pop() {
                Some(ClassScope::Package) => {
                    self.package_stack.pop();
                }
                Some(ClassScope::Together(_)) => {}
                None => {}
            }
            return Ok(());
        }

        // Java `CommandRankDir.executeArg` stores the command in
        // `SkinParam.rankdir`; `DotStringFactory` emits `rankdir=LR` for SVEK.
        if line.eq_ignore_ascii_case("left to right direction") {
            self.direction = ClassLayoutDirection::LeftToRight;
            return Ok(());
        }
        if line.eq_ignore_ascii_case("top to bottom direction") {
            self.direction = ClassLayoutDirection::TopToBottom;
            return Ok(());
        }

        if self.try_together(line) {
            return Ok(());
        }
        if self.try_entity_decl(line) {
            return Ok(());
        }
        if self.try_association_class(line) {
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
        if looks_like_package_opener(line) {
            return Err(ParseError {
                line: self.current_line,
                message: "invalid package or symbol-container declaration".to_string(),
            });
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
        let (url, clean_line) = super::extract_link_url(line);
        let url_tooltip = super::extract_link_tooltip(line);
        let line = clean_line.as_str();
        // Allows dots in the identifier (for `set namespaceSeparator none`).
        static RE_DOTTED: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"(?i)^(class|abstract\s+class|abstract|interface|enum|annotation|entity|object|state|circle|diamond|actor|usecase|component|database|queue|boundary|node|rectangle)\s+(?:(?:"([^"]+)"\s+as\s+)?(\w[\w.]*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?)|"([^"]+)")"#,
            )
            .unwrap()
        });
        // Permissive regex: accepts any non-whitespace name (for custom namespace separators).
        static RE_PERMISSIVE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"(?i)^(class|abstract\s+class|abstract|interface|enum|annotation|entity|object|state|circle|diamond|actor|usecase|component|database|queue|boundary|node|rectangle)\s+(?:(?:"([^"]+)"\s+as\s+)?([^\s{<>]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?)|"([^"]+)")"#,
            )
            .unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)>>").unwrap());

        let re = if matches!(self.namespace_sep.as_deref(), None | Some(".")) {
            // With Java's null separator, dots are literal name characters.
            // The default dot separator uses the same lexical grammar and
            // splits the captured identity later.
            &*RE_DOTTED
        } else {
            // Custom separators (e.g. "::" or "/") use the permissive grammar.
            &*RE_PERMISSIVE
        };
        if let Some(caps) = re.captures(line) {
            // Pattern2.compileInternal makes PlantUML command regexes
            // case-insensitive; normalize the captured command for semantic
            // matching while preserving labels and identifiers verbatim.
            let declaration_kind = caps[1].trim().to_ascii_lowercase();
            if line.trim_end().ends_with('{')
                && PackageKind::from_command_symbol(&declaration_kind).is_some()
            {
                return false;
            }
            let kind = parse_entity_kind(&declaration_kind);
            // Group 4: quoted-only form — class "**Name**" with no `as` keyword.
            let (mut label, mut id) = if let Some(m) = caps.get(4) {
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
            label = normalize_inline_stereotypes(&label);

            // A trailing `<...>` is a generic type parameter, not part of the
            // entity id/label/qualified-name. Split it off the id; mirror the
            // split onto the label only when the label was derived from the id
            // (no explicit alias / quoted name).
            let label_was_id = caps.get(2).is_none() && caps.get(4).is_none();
            let generic = split_generic(&mut id);
            if label_was_id {
                split_generic(&mut label);
            }

            let stereotype_source = text_outside_double_quotes(line);
            let mut spot_color: Option<String> = None;
            let mut spot_character: Option<char> = None;
            let stereotypes: Vec<String> = STEREOTYPE_RE
                .captures_iter(&stereotype_source)
                .map(|c| {
                    let (text, character, color) = process_spot_stereotype(c[1].trim());
                    if spot_color.is_none() {
                        spot_color = color;
                        spot_character = character;
                    }
                    text
                })
                .filter(|s| !s.is_empty())
                .collect();

            let entity_colors = parse_entity_colors(line);

            let explicit_alias = caps.get(2).is_some();
            // The ordinary class commands use
            // `CucaDiagram#quarkInContextSafe(false, idShort)`, while
            // `CommandCreateElementFull2` uses the unique-reuse form.
            let lookup = match declaration_kind.as_str() {
                "class" | "abstract class" | "abstract" | "interface" | "enum" | "annotation"
                | "entity" => QuarkLookup::CurrentContext,
                _ => QuarkLookup::ReuseUnique,
            };
            let ordinary_class_command = matches!(
                declaration_kind.as_str(),
                "class"
                    | "abstract class"
                    | "abstract"
                    | "interface"
                    | "enum"
                    | "annotation"
                    | "entity"
            );
            let quoted_identity_has_separator =
                caps.get(4).is_some() && self.split_identity(&id).1.len() > 1;
            let entity_path = self.resolve_quark_path(&id, lookup);
            // CODE4 is a quoted identity, not an explicit DISPLAY. Java leaves
            // DISPLAY absent and falls back to the short Quark name, so a
            // separator-bearing `"fresh.domain.Record"` displays `Record`.
            // A one-segment quoted identity still has the whole quoted text as
            // its leaf and therefore preserves spaces and Creole presentation.
            let display_label = if explicit_alias
                || (caps.get(4).is_some() && !quoted_identity_has_separator)
            {
                label
            } else {
                entity_path.last().cloned().unwrap_or(label)
            };
            let final_id = self.path_id(&entity_path);
            let (entity_idx, entity_was_created) =
                if let Some(&idx) = self.entity_by_path.get(&entity_path) {
                    (idx, false)
                } else {
                    self.create_entity_at_path(
                        entity_path.clone(),
                        display_label.clone(),
                        kind,
                        explicit_alias,
                    );
                    (self.entity_by_path[&entity_path], true)
                };

            {
                let entity = &mut self.entities[entity_idx];
                // `CommandCreateClass` preserves the display owned by the
                // first materialization. `CommandCreateElementFull2` is the
                // separate mixed-description command that calls setDisplay
                // even when it reuses an existing quark.
                if entity_was_created || !ordinary_class_command {
                    entity.label = display_label;
                }
                if explicit_alias {
                    entity.explicit_alias = true;
                }
                if !stereotypes.is_empty() {
                    entity.stereotypes = stereotypes;
                }
                if spot_color.is_some() {
                    entity.spot_color = spot_color.clone();
                    entity.spot_character = spot_character;
                }
                if url.is_some() {
                    entity.url = url.clone();
                    entity.url_tooltip = url_tooltip.clone();
                }
                if entity_colors.back.is_some() {
                    entity.color = entity_colors.back.clone();
                }
                if entity_colors.text.is_some() {
                    entity.text_color = entity_colors.text.clone();
                }
                if entity_colors.line.is_some() {
                    entity.line_color = entity_colors.line.clone();
                }
                if entity_colors.line_style.is_some() {
                    entity.line_style = entity_colors.line_style;
                }
                if generic.is_some() {
                    entity.generic = generic.clone();
                }
            }

            self.register_entity_path(&entity_path, &final_id);
            self.materialize_active_phantom_packages();

            // `CommandCreateClassMultilines.manageExtends` constructs each
            // declaration relationship as parent -> child with
            // `LinkType(NONE, EXTENDS)`. Preserve that SVEK orientation:
            // reversing an otherwise equivalent child -> parent edge changes
            // Graphviz ranks, endpoint UIDs, and the SVG path identity.
            self.parse_supertypes(line, &final_id);

            if line.ends_with('{') || line.ends_with("{{") {
                self.current_entity = Some(final_id.clone());
                self.current_entity_needs_body_location = true;
            }
            self.last_entity_id = Some(final_id);
            true
        } else {
            false
        }
    }

    /// Parse `extends`/`implements` clauses on a class-declaration line into
    /// Inheritance/Implementation relationships from the declared entity to
    /// each named supertype. Generic (`<...>`) and stereotype (`<<...>>`) spans
    /// are stripped first so a bounded type param like `class Foo<T extends X>`
    /// is not mistaken for an inheritance clause.
    fn parse_supertypes(&mut self, line: &str, child_id: &str) {
        let body = strip_empty_inline_body(line);
        // Drop angle-bracket spans (generics + `<<stereotype>>`).
        let mut scan = String::new();
        let mut depth: u32 = 0;
        for ch in body.chars() {
            match ch {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                _ if depth == 0 => scan.push(ch),
                _ => {}
            }
        }
        let child_is_interface = self
            .entities
            .iter()
            .find(|entity| entity.id == child_id)
            .is_some_and(|entity| entity.kind == EntityKind::Interface);
        // `CommandCreateClass` calls `manageExtends` for EXTENDS and then
        // IMPLEMENTS. Missing implements targets, and missing parents of an
        // interface, are materialized as interfaces.
        let mut supers: Vec<(String, RelationshipKind)> = Vec::new();
        let mut current: Option<RelationshipKind> = None;
        for tok in scan.split_whitespace() {
            match tok {
                "extends" => current = Some(RelationshipKind::Inheritance),
                "implements" => current = Some(RelationshipKind::Implementation),
                _ => {
                    if let Some(kind) = current {
                        for name in tok.split(',') {
                            let name = name.trim();
                            if !name.is_empty() {
                                supers.push((name.to_string(), kind));
                            }
                        }
                    }
                }
            }
        }
        for (name, kind) in supers {
            let parent_kind = if kind == RelationshipKind::Implementation || child_is_interface {
                EntityKind::Interface
            } else {
                EntityKind::Class
            };
            let parent = self.ensure_entity_kind(&name, parent_kind);
            let dashed = parent_kind == EntityKind::Interface && !child_is_interface;
            self.push_relationship(Relationship {
                from: parent,
                to: child_id.to_string(),
                kind,
                label: None,
                label_arrow: LinkArrow::None,
                from_multiplicity: None,
                to_multiplicity: None,
                from_decor: None,
                to_decor: None,
                decorated_end: RelationshipEnd::From,
                dashed,
                length: 2,
                style: RelationshipStyle {
                    declaration: true,
                    ..RelationshipStyle::default()
                },
                source_line: self.current_line,
            });
        }
    }

    fn attribute_multiline_declaration_to_first_body_line(&mut self) {
        let Some(entity_id) = self.current_entity.as_deref() else {
            return;
        };
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        else {
            return;
        };
        let declaration_line = entity.source_line;

        for relationship in &mut self.relationships {
            if relationship.style.declaration
                && relationship.to == entity_id
                && relationship.source_line == declaration_line
            {
                relationship.source_line = self.current_line;
            }
        }
    }

    fn try_enum_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^enum\s+(\w+)\s*\{?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = self.ensure_entity_kind(&caps[1], EntityKind::Enum);
            if line.ends_with('{') {
                self.current_entity = Some(id.clone());
                self.current_entity_needs_body_location = true;
            }
            self.last_entity_id = Some(id);
            true
        } else {
            false
        }
    }

    /// Association class: `(A, B) .. C` or `(A, B) -- C`. PlantUML draws a tiny
    /// anchor (`apoint`) on the A–B line and a dashed (`..`) / solid (`--`)
    /// connector to the association class `C`. Endpoints may be quoted.
    fn try_association_class(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^\(\s*(?:"([^"]+)"|([\w.]+))\s*,\s*(?:"([^"]+)"|([\w.]+))\s*\)\s*(\.\.|-{2,})\s*(?:"([^"]+)"|([\w.]+))\s*$"#,
            )
            .unwrap()
        });
        let Some(caps) = RE.captures(line) else {
            return false;
        };
        let pick = |q: usize, b: usize| -> String {
            if let Some(m) = caps.get(q) {
                strip_creole_for_id(m.as_str())
            } else {
                caps.get(b).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        };
        let a_raw = pick(1, 2);
        let b_raw = pick(3, 4);
        let connector = &caps[5];
        let c_raw = pick(6, 7);
        let dashed = connector.starts_with('.');

        let a = self.ensure_entity(&a_raw);
        let b = self.ensure_entity(&b_raw);
        let c = self.ensure_entity(&c_raw);
        // Java `foundLink` searches live links from newest to oldest and
        // `Association.createNew` removes the selected base before inserting
        // its three split links.
        let replaced_relationship = self
            .relationships
            .iter()
            .rposition(|relationship| {
                (relationship.from == a && relationship.to == b)
                    || (relationship.from == b && relationship.to == a)
            })
            .map(|index| self.take_relationship(index));

        self.uid_events
            .push(ClassUidEvent::Association(self.association_classes.len()));
        self.association_classes
            .push(crate::diagram::class::AssociationClass {
                a,
                b,
                c,
                replaced_relationship,
                dashed,
                source_line: self.current_line,
            });
        true
    }

    fn try_relationship(&mut self, line: &str) -> bool {
        // Relationship format: EntityA ["mult"] ARROW ["mult"] EntityB [: label]
        // Supported arrows: <|--, --|>, ..|>, <|.., *--, --*, o--, --o,
        //                   <-->, <..>, <->, --, -->, -->>, <--, <-, ->, ->>,
        //                   .., ..>, ...>, ..>>, <..
        //                   <|--|> (bidirectional inheritance), <..|.> etc.
        // Multiple dashes (e.g. ---- or ------) are treated as plain association.
        //
        // Colour / direction / bold / thickness modifiers attach to the arrow.
        // Preserve visual modifiers while stripping them from the arrow shape
        // used for relationship-kind detection.
        let (stripped_line, mut style, direction) = strip_arrow_modifiers(line);
        let line = stripped_line.as_str();
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            // Endpoint may be a bare identifier or a quoted name (`"any text"`)
            // so labels with whitespace or punctuation work. The bare form
            // accepts `/` and `:` (in addition to word chars and `.`) so a
            // custom-namespace-separator endpoint (`com::service::UserService`,
            // `com/example/Foo`) is captured whole — `[\w.]+` alone would stop
            // at the first `:`/`/`, truncating the name and synthesising a
            // phantom `com` entity. The label suffix still uses `\s*:\s*` (a
            // separator colon flanked by optional space), which a bare endpoint
            // (no spaces) never matches, so `A::B --> C::D : label` still splits.
            Regex::new(
                r#"^(?:"([^"]+)"|([\w./:]+))\s*(?:"([^"]+)")?\s*((?:<\|--\|>|<\.\.>|<\|--|--\|>|\.\.\|>|<\|\.\.|<\.\.|<-->>|<-->|<->|\*--|--\*|o--|--o|-->>|<-{2,}|-{2,}>|<--|-->|->>|->|<-|\)-{1,}\(|\)-{1,}|-{1,}\(|-{1,}|\.{2,}>>|\.{2,}>|\.\.))\s*(?:"([^"]+)")?\s*(?:"([^"]+)"|([\w./:]+))(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });
        // ER crow's foot notation: entity1 CROW--CROW entity2 : "label"
        static ER_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^(\w+)\s+([|o}][|{o])--([|o][|{])\s+(\w+)(?:\s*:\s*(.+))?$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from_quoted = caps.get(1).map(|m| m.as_str());
            let from_raw = from_quoted.unwrap_or_else(|| caps.get(2).unwrap().as_str());
            let from_mult = caps.get(3).map(|m| m.as_str().to_string());
            let rel_str = &caps[4];
            let to_mult = caps.get(5).map(|m| m.as_str().to_string());
            let to_quoted = caps.get(6).map(|m| m.as_str());
            let to_raw = to_quoted.unwrap_or_else(|| caps.get(7).unwrap().as_str());
            let (label, label_arrow) = parse_label_arrow(caps.get(8).map(|m| m.as_str()));
            let label = label.map(|label| normalize_link_label_guillemets(&label));

            // Java `CommandLinkClass` parses `ARROW_HEAD1` and `ARROW_HEAD2`
            // independently through `LinkDecor`; `)` and `(` are the two
            // endpoint spellings of `LinkDecor.PARENTHESIS`.
            let mut from_decor = rel_str
                .starts_with(')')
                .then_some(EndpointDecor::Parenthesis);
            let mut to_decor = rel_str.ends_with('(').then_some(EndpointDecor::Parenthesis);
            let rel_body = rel_str.trim_start_matches(')').trim_end_matches('(');
            let (kind, dashed, mut decorated_end) = parse_relationship_kind(rel_body);
            let length = if direction.is_some_and(QueueDirection::is_horizontal) {
                1
            } else {
                relationship_length(rel_str)
            };
            // Java's `CommandLinkClass` selects both endpoint quarks before
            // either missing leaf constructor runs. The first constructor's
            // phantom-group sweep can therefore materialize ancestors of the
            // second endpoint before that endpoint consumes its own UID.
            let from_path = self.relationship_endpoint_path(from_raw, from_quoted.is_some());
            self.register_quark_path(&from_path);
            let to_path = self.relationship_endpoint_path(to_raw, to_quoted.is_some());
            self.register_quark_path(&to_path);
            let mut from = self.materialize_relationship_endpoint(from_path);
            let mut to = self.materialize_relationship_endpoint(to_path);
            let mut from_mult = from_mult;
            let mut to_mult = to_mult;
            if direction.is_some_and(QueueDirection::inverts_link) {
                // Java `CommandLinkClass.executeArg` constructs the source
                // link first, then replaces it with `Link.getInv()` for left
                // and up. `Link.getInv` swaps entities, quantifiers, roles and
                // ports while `LinkType.getInversed` swaps its decorations.
                std::mem::swap(&mut from, &mut to);
                std::mem::swap(&mut from_mult, &mut to_mult);
                std::mem::swap(&mut from_decor, &mut to_decor);
                decorated_end = invert_relationship_end(decorated_end);
                style.inverted = true;
            }

            self.push_relationship(Relationship {
                from,
                to,
                kind,
                label,
                label_arrow,
                from_multiplicity: from_mult,
                to_multiplicity: to_mult,
                from_decor,
                to_decor,
                decorated_end,
                dashed,
                length,
                style,
                source_line: self.current_line,
            });
            return true;
        }

        if let Some(caps) = ER_RE.captures(line) {
            let from_raw = &caps[1];
            let from_decor = parse_endpoint_decor(&caps[2]);
            let to_decor = parse_endpoint_decor(&caps[3]);
            let to_raw = &caps[4];
            let label = caps
                .get(5)
                .map(|m| m.as_str().trim().trim_matches('"').to_string());

            let from_path = self.relationship_endpoint_path(from_raw, false);
            self.register_quark_path(&from_path);
            let to_path = self.relationship_endpoint_path(to_raw, false);
            self.register_quark_path(&to_path);
            let from = self.materialize_relationship_endpoint(from_path);
            let to = self.materialize_relationship_endpoint(to_path);

            self.push_relationship(Relationship {
                from,
                to,
                kind: RelationshipKind::Association,
                label,
                label_arrow: LinkArrow::None,
                from_multiplicity: None,
                to_multiplicity: None,
                from_decor,
                to_decor,
                decorated_end: RelationshipEnd::None,
                dashed: false,
                length: 2,
                style: RelationshipStyle::default(),
                source_line: self.current_line,
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
            let to = self.ensure_entity_kind(&to_raw, EntityKind::Interface);
            let from = self.ensure_entity(&from_raw);
            self.push_relationship(Relationship {
                from,
                to,
                kind: RelationshipKind::Association,
                label: None,
                label_arrow: LinkArrow::None,
                from_multiplicity: None,
                to_multiplicity: None,
                from_decor: None,
                to_decor: None,
                decorated_end: RelationshipEnd::None,
                dashed: false,
                length: 2,
                style: RelationshipStyle::default(),
                source_line: self.current_line,
            });
            return true;
        }

        false
    }

    fn try_inline_member(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let entity_id = self.ensure_entity(&caps[1]);
            let member_text = caps[2].trim();

            let member = parse_member(member_text);

            if let Some(entity) = self.find_entity_mut(&entity_id) {
                entity.members.push(member);
            }
            true
        } else {
            false
        }
    }

    fn try_together(&mut self, line: &str) -> bool {
        let compact = line.split_whitespace().collect::<String>();
        if !compact.eq_ignore_ascii_case("together{") {
            return false;
        }
        let idx = self.together.len();
        self.together.push(crate::diagram::class::TogetherGroup {
            parent: self.current_together(),
            owner_package: self
                .package_stack
                .last()
                .map(|&package_idx| self.packages[package_idx].name.clone()),
            entities: Vec::new(),
            packages: Vec::new(),
        });
        self.scope_stack.push(ClassScope::Together(idx));
        true
    }

    fn try_package(&mut self, line: &str) -> bool {
        static ORDINARY_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(?i:package)\s+(?:"([^"]+)"|([^\s#{}"]+))(?:\s+(?i:as)\s+([[:alnum:]_.]+))?(.*?)\{\s*$"#,
            )
            .unwrap()
        });
        static ORDINARY_MODIFIERS_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^\s*(?:\$[^\s{}"<>$]+(?:\s+\$[^\s{}"<>$]+)*)?\s*(<<.*>>)?\s*(?:\$[^\s{}"<>$]+(?:\s+\$[^\s{}"<>$]+)*)?\s*(\[\[.*?\]\])?\s*(?:#([^\s{]+))?\s*$"#,
            )
            .unwrap()
        });
        static SYMBOL_RE: LazyLock<Regex> = LazyLock::new(|| {
            let keywords = PackageKind::COMMAND_SYMBOLS
                .iter()
                .map(|(keyword, _)| *keyword)
                .collect::<Vec<_>>()
                .join("|");
            Regex::new(&format!(
                r#"^((?i:{keywords}))\s+(?:"([^"]+)"(?:\s+((?:<<\s*[^<>]+?\s*>>\s*)+))?\s+(?i:as)\s+([^\s#{{}}"]+)|([^\s#{{}}"]+)(?:\s+((?:<<\s*[^<>]+?\s*>>\s*)+))?\s+(?i:as)\s+"([^"]+)"|([^\s#{{}}"]+)(?:\s+((?:<<\s*[^<>]+?\s*>>\s*)+))?\s+(?i:as)\s+([^\s#{{}}"]+)|"([^"]+)"|([^\s#{{}}"]+))(.*?)\{{\s*$"#
            ))
            .unwrap()
        });
        static NAMESPACE_RE: LazyLock<Regex> = LazyLock::new(|| {
            // Java registers namespace as a separate command. Keep its
            // identity separate even though Rust shares the modifier parser.
            Regex::new(
                r#"^((?i:namespace))\s+(?:"([^"]+)"(?:\s+((?:<<\s*[^<>]+?\s*>>\s*)+))?\s+(?i:as)\s+([^\s#{}"]+)|([^\s#{}"]+)(?:\s+((?:<<\s*[^<>]+?\s*>>\s*)+))?\s+(?i:as)\s+"([^"]+)"|([^\s#{}"]+)(?:\s+((?:<<\s*[^<>]+?\s*>>\s*)+))?\s+(?i:as)\s+([^\s#{}"]+)|"([^"]+)"|([^\s#{}"]+))(.*?)\{\s*$"#,
            )
            .unwrap()
        });
        static SYMBOL_MODIFIERS_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^\s*(?:\$[^\s{}"<>$]+(?:\s+\$[^\s{}"<>$]+)*)?\s*((?:<<\s*[^<>]+?\s*>>\s*)+)?(?:\$[^\s{}"<>$]+(?:\s+\$[^\s{}"<>$]+)*)?\s*(\[\[.*?\]\])?\s*(?:#([^\s{]+))?\s*$"#,
            )
            .unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<<\s*([^<>]+?)\s*>>").unwrap());

        let topurl = self
            .meta
            .skinparams
            .iter()
            .rev()
            .find(|param| param.key.eq_ignore_ascii_case("topurl"))
            .map(|param| param.value.as_str());

        let ordinary = ORDINARY_RE.captures(line).and_then(|caps| {
            let modifiers = ORDINARY_MODIFIERS_RE.captures(caps.get(4)?.as_str())?;
            let display = caps.get(1).or(caps.get(2))?.as_str().to_string();
            let alias = caps.get(3).map(|value| value.as_str().to_string());
            let code = alias.clone().unwrap_or_else(|| display.clone());
            let (url, url_tooltip) = if let Some(link) = modifiers.get(2) {
                let (href, tooltip) = parse_package_link(link.as_str(), topurl)?;
                (Some(href), tooltip)
            } else {
                (None, None)
            };
            let color = modifiers.get(3).map(|m| m.as_str().to_string());
            let stereotypes = modifiers
                .get(1)
                .into_iter()
                .flat_map(|value| STEREOTYPE_RE.captures_iter(value.as_str()))
                .map(|capture| capture[1].trim().to_string())
                .collect::<Vec<_>>();
            Some((
                PackageKind::Package,
                display,
                code,
                alias.is_some(),
                stereotypes,
                url,
                url_tooltip,
                color,
                false,
            ))
        });

        let parsed = ordinary.or_else(|| {
            let (caps, kind, symbol_from_keyword) = if let Some(caps) = SYMBOL_RE.captures(line) {
                let kind = PackageKind::from_command_symbol(&caps[1])?;
                (caps, kind, true)
            } else {
                (NAMESPACE_RE.captures(line)?, PackageKind::Namespace, false)
            };
            let modifiers = SYMBOL_MODIFIERS_RE.captures(caps.get(13)?.as_str())?;
            let (display, code, explicit_alias, identity_stereotype) =
                if let (Some(display), Some(code)) = (caps.get(2), caps.get(4)) {
                    (
                        display.as_str().to_string(),
                        code.as_str().to_string(),
                        true,
                        caps.get(3),
                    )
                } else if let (Some(code), Some(display)) = (caps.get(5), caps.get(7)) {
                    (
                        display.as_str().to_string(),
                        code.as_str().to_string(),
                        true,
                        caps.get(6),
                    )
                } else if let (Some(display), Some(code)) = (caps.get(8), caps.get(10)) {
                    (
                        display.as_str().to_string(),
                        code.as_str().to_string(),
                        true,
                        caps.get(9),
                    )
                } else {
                    let code = caps
                        .get(11)
                        .or(caps.get(12))
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    (code.clone(), code, false, None)
                };
            let (url, url_tooltip) = if let Some(link) = modifiers.get(2) {
                let (href, tooltip) = parse_package_link(link.as_str(), topurl)?;
                (Some(href), tooltip)
            } else {
                (None, None)
            };
            let color = modifiers.get(3).map(|m| m.as_str().to_string());
            // `CommandPackageWithUSymbol#executeArg` asks RegexResult for one
            // lazy STEREOTYPE capture. A populated trailing capture owns the
            // metadata; the identity-slot capture is only the fallback.
            let stereotype_source = modifiers.get(1).or(identity_stereotype);
            let stereotypes = stereotype_source
                .into_iter()
                .flat_map(|value| STEREOTYPE_RE.captures_iter(value.as_str()))
                .map(|capture| capture[1].trim().to_string())
                .collect::<Vec<_>>();
            Some((
                kind,
                display,
                code,
                explicit_alias,
                stereotypes,
                url,
                url_tooltip,
                color,
                symbol_from_keyword,
            ))
        });

        if let Some((
            kind,
            display,
            code,
            explicit_alias,
            stereotypes,
            url,
            url_tooltip,
            color,
            symbol_from_keyword,
        )) = parsed
        {
            // `CommandPackage#getRegexConcat` consumes the complete line.
            // `CommandPackage#executeArg` sends `AS` to `quarkInContext` and
            // keeps `NAME` solely as display; `CommandNamespace#executeArg`
            // follows the same quark-backed group discipline.
            let path = self.resolve_group_path(&code);
            if path.is_empty() {
                return false;
            }
            let requested_display_name = (explicit_alias || path.len() > 1).then(|| {
                if explicit_alias {
                    display.clone()
                } else {
                    path.last().cloned().unwrap_or_default()
                }
            });

            let final_depth = path.len();
            let mut parent = None;
            let mut final_was_created = false;
            for depth in 1..=final_depth {
                let prefix = path[..depth].to_vec();
                let is_final = depth == final_depth;
                let idx = if let Some(&idx) = self.package_by_path.get(&prefix) {
                    if is_final {
                        let package = &mut self.packages[idx];
                        package.kind = kind;
                        package.color = color.clone();
                        if url.is_some() {
                            package.url = url.clone();
                            package.url_tooltip = url_tooltip.clone();
                        }
                        // CommandPackage.executeArg and
                        // CommandPackageWithUSymbol.executeArg call
                        // Entity.setStereotype only when this declaration
                        // supplied one.
                        if !stereotypes.is_empty() {
                            package.stereotypes = stereotypes.clone();
                        }
                        package.symbol_from_keyword = symbol_from_keyword;
                        if package.source_line == 0 {
                            package.source_line = self.current_line;
                        }
                        package.phantom = false;
                    }
                    idx
                } else {
                    if is_final {
                        final_was_created = true;
                    }
                    let idx = self.packages.len();
                    let name = self.path_id(&prefix);
                    if is_final {
                        self.uid_events.push(ClassUidEvent::Package(name.clone()));
                    }
                    let display_name = if is_final {
                        requested_display_name.clone()
                    } else {
                        prefix.last().cloned()
                    };
                    self.packages.push(Package {
                        name,
                        kind: if is_final { kind } else { PackageKind::Package },
                        color: if is_final { color.clone() } else { None },
                        url: if is_final { url.clone() } else { None },
                        url_tooltip: if is_final { url_tooltip.clone() } else { None },
                        entities: Vec::new(),
                        parent,
                        // `CucaDiagram#eventuallyBuildPhantomGroups` gives
                        // intermediate quarks their first leaf's location.
                        source_line: if is_final { self.current_line } else { 0 },
                        stereotypes: if is_final {
                            stereotypes.clone()
                        } else {
                            Vec::new()
                        },
                        symbol_from_keyword: is_final && symbol_from_keyword,
                        display_name,
                        phantom: !is_final,
                    });
                    self.package_paths.push(prefix.clone());
                    self.package_by_path.insert(prefix.clone(), idx);
                    self.register_quark_path(&prefix);
                    idx
                };
                parent = Some(idx);
            }
            let package_idx = parent.unwrap();
            if final_was_created && let Some(group_idx) = self.current_together() {
                let package_name = self.packages[package_idx].name.clone();
                self.together[group_idx].packages.push(package_name);
            }
            self.package_stack.push(package_idx);
            self.scope_stack.push(ClassScope::Package);
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
            Regex::new(r"^note\s+(top|bottom|left|right)\s+of\s+([\w.]+)\s*(#\S+)?\s*$").unwrap()
        });
        // Shorthand single-line note attached to last entity: `note <pos> : <text>`
        static SHORT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^note\s+(top|bottom|left|right)\s*:\s*(.+)$").unwrap());
        // Shorthand multi-line note attached to last entity: `note <pos>` (optional color)
        static SHORT_ML_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^note\s+(top|bottom|left|right)\s*(#\S+)?\s*$").unwrap());
        // Floating named note: `note "text" as Name`
        static FLOATING_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap());
        // Multi-line floating note: `note as Name` (optional color suffix like `#yellow`).
        static FLOATING_ML_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^note\s+as\s+(\w+)\s*(#\S+)?\s*$").unwrap());

        if let Some(caps) = ATTACHED_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let target = self.resolve_note_target(&caps[2]);
            let text = caps[3].trim().to_string();
            // Expand `\n` escape sequences into actual newlines.  Preserve
            // leading whitespace on each segment so that indented `* items`
            // (e.g. `  * point 2`) are distinguishable from top-level bullet
            // items (`* point 1`) in the renderer.  Only strip trailing
            // whitespace from each segment.
            let lines = crate::display::split_escaped_newlines(&text)
                .into_iter()
                .map(|s| s.trim_end().to_string())
                .collect();
            self.push_note(Note {
                lines,
                id: None,
                target: Some(target),
                position: Some(position),
                alias: None,
                color: None,
                source_line: self.current_line,
            });
            return true;
        }

        if let Some(caps) = ATTACHED_ML_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let target = self.resolve_note_target(&caps[2]);
            self.current_note = Some(Note {
                lines: Vec::new(),
                id: None,
                target: Some(target),
                position: Some(position),
                alias: None,
                color: caps.get(3).map(|m| m.as_str().to_string()),
                source_line: self.current_line,
            });
            return true;
        }

        // Shorthand: `note right : text` — attaches to the last declared entity.
        if let Some(caps) = SHORT_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let text = caps[2].trim().to_string();
            let lines = crate::display::split_escaped_newlines(&text)
                .into_iter()
                .map(|s| s.trim_end().to_string())
                .collect();
            let target = self.last_entity_id.clone();
            self.push_note(Note {
                lines,
                id: None,
                target,
                position: Some(position),
                alias: None,
                color: None,
                source_line: self.current_line,
            });
            return true;
        }

        // Shorthand multi-line: `note right` — attaches to the last declared entity.
        if let Some(caps) = SHORT_ML_RE.captures(line) {
            let position = parse_note_position(&caps[1]);
            let target = self.last_entity_id.clone();
            self.current_note = Some(Note {
                lines: Vec::new(),
                id: None,
                target,
                position: Some(position),
                alias: None,
                color: caps.get(2).map(|m| m.as_str().to_string()),
                source_line: self.current_line,
            });
            return true;
        }

        if let Some(caps) = FLOATING_RE.captures(line) {
            let text = caps[1].trim().to_string();
            let alias = caps[2].to_string();
            let lines = crate::display::split_escaped_newlines(&text)
                .into_iter()
                .map(|s| s.trim_end().to_string())
                .collect();
            self.push_note(Note {
                lines,
                id: None,
                target: None,
                position: None,
                alias: Some(alias),
                color: None,
                source_line: self.current_line,
            });
            return true;
        }

        if let Some(caps) = FLOATING_ML_RE.captures(line) {
            let alias = caps[1].to_string();
            self.current_note = Some(Note {
                lines: Vec::new(),
                id: None,
                target: None,
                position: None,
                alias: Some(alias),
                color: caps.get(2).map(|m| m.as_str().to_string()),
                source_line: self.current_line,
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
                crate::display::split_escaped_newlines(&text)
                    .into_iter()
                    .map(|s| s.trim_end().to_string())
                    .collect()
            };
            self.push_note(Note {
                lines,
                id: None,
                target: None,
                position: None,
                alias: None,
                color: None,
                source_line: self.current_line,
            });
            return true;
        }
        // `note on link` — multi-line note attached to the last relationship.
        if line == "note on link" {
            self.current_note = Some(Note {
                lines: Vec::new(),
                id: None,
                target: None,
                position: None,
                alias: None,
                color: None,
                source_line: self.current_line,
            });
            return true;
        }

        // `end note` is handled in parse_line; skip it here if encountered standalone.
        if line == "end note" {
            return true;
        }

        // `note : text` — Java PlantUML treats bare `note` as an entity named
        // "note" with an inline member `: text`.  Create or update that entity.
        if let Some(text) = line.strip_prefix("note :").map(|s| s.trim()) {
            if !text.is_empty() {
                let member = Member {
                    name: text.to_string(),
                    return_type: None,
                    visibility: Visibility::Default,
                    is_static: false,
                    is_abstract: false,
                    kind: MemberKind::Field,
                    display_text: text.to_string(),
                };
                let id = self.ensure_entity("note");
                if let Some(ent) = self.find_entity_mut(&id) {
                    ent.members.push(member);
                }
            }
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
            self.title_line.get_or_insert(self.current_line);
            return true;
        }
        if line == "title" {
            self.meta_block = Some(MetaBlock::Title);
            self.title_line.get_or_insert(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("header ") {
            self.meta.header = Some(rest.trim().to_string());
            self.header_line.get_or_insert(self.current_line);
            return true;
        }
        if line == "header"
            || line.starts_with("left header")
            || line.starts_with("right header")
            || line.starts_with("center header")
        {
            self.meta_block = Some(MetaBlock::Header);
            self.header_line.get_or_insert(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("footer ") {
            self.meta.footer = Some(rest.trim().to_string());
            self.footer_line.get_or_insert(self.current_line);
            return true;
        }
        if line == "footer"
            || line.starts_with("left footer")
            || line.starts_with("right footer")
            || line.starts_with("center footer")
        {
            self.meta_block = Some(MetaBlock::Footer);
            self.footer_line.get_or_insert(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("caption ") {
            self.meta.caption = Some(rest.trim().to_string());
            self.caption_line.get_or_insert(self.current_line);
            return true;
        }
        if line == "caption" {
            self.meta_block = Some(MetaBlock::Caption);
            self.caption_line.get_or_insert(self.current_line);
            return true;
        }
        if line == "legend"
            || line.starts_with("legend left")
            || line.starts_with("legend right")
            || line.starts_with("legend center")
            || line.starts_with("legend top")
            || line.starts_with("legend bottom")
        {
            self.meta_block = Some(MetaBlock::Legend);
            self.legend_line.get_or_insert(self.current_line);
            return true;
        }
        static NAMESPACE_SEPARATOR_RE: LazyLock<Regex> = LazyLock::new(|| {
            // Java `CommandNamespaceSeparator#getRegexConcat` accepts both
            // command spellings; its command regex is case-insensitive.
            Regex::new(r"(?i)^set\s+(?:separator|namespaceseparator)\s+(\S+)\s*$").unwrap()
        });
        if let Some(caps) = NAMESPACE_SEPARATOR_RE.captures(line) {
            let sep = &caps[1];
            // `CommandNamespaceSeparator#executeArg` replaces the diagram's
            // sole separator state and compares `none` case-insensitively.
            self.namespace_sep = if sep.eq_ignore_ascii_case("none") {
                None
            } else {
                Some(sep.to_string())
            };
            return true;
        }
        // Parse skinparam key value (store for renderer use).
        if let Some(rest) = line.strip_prefix("skinparam ") {
            let rest = rest.trim();
            if let Some(prefix) = rest.strip_suffix('{') {
                let prefix = prefix.trim();
                if !prefix.is_empty() {
                    self.current_skinparam_prefix = Some(prefix.to_string());
                }
                return true;
            }
            if let Some((key, value)) = rest.split_once(' ') {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            return true;
        }
        // Capture hide/show directives so the renderer can suppress
        // circles, members, attributes, etc.
        if let Some(rest) = line.strip_prefix("hide ") {
            self.hide_show.push(crate::diagram::class::HideShow {
                show: false,
                remove: false,
                arg: rest.split_whitespace().collect::<Vec<_>>().join(" "),
            });
            return true;
        }
        if let Some(rest) = line.strip_prefix("show ") {
            self.hide_show.push(crate::diagram::class::HideShow {
                show: true,
                remove: false,
                arg: rest.split_whitespace().collect::<Vec<_>>().join(" "),
            });
            return true;
        }
        // `remove X` drops the named entity (or `<<stereotype>>`-matched
        // entities) from the diagram entirely, along with their links.
        if let Some(rest) = line.strip_prefix("remove ") {
            self.hide_show.push(crate::diagram::class::HideShow {
                show: false,
                remove: true,
                arg: rest.split_whitespace().collect::<Vec<_>>().join(" "),
            });
            return true;
        }
        if super::is_allow_mixing_command(line) {
            return true;
        }
        // Skip other layout/format directives.
        line.starts_with("map ") || line.starts_with("set ")
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
        // `return_type` is repurposed to hold the separator symbol (one of
        // `--`, `..`, `==`, `__`) so the renderer can pick the right stroke
        // style.
        if let Some(caps) = SEPARATOR_LABELED_RE.captures(trimmed) {
            let symbol = caps[1].to_string();
            let label = caps[2].to_string();
            let member = Member {
                name: label.clone(),
                return_type: Some(symbol),
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

        // Bare separator lines — render as unlabeled separator. `return_type`
        // carries the symbol so the renderer can dispatch on line style.
        if trimmed == "--" || trimmed == ".." || trimmed == "==" || trimmed == "__" {
            let member = Member {
                name: String::new(),
                return_type: Some(trimmed.to_string()),
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

        let mut member = parse_member(trimmed);
        if let Some(entity_id) = &self.current_entity
            && let Some(entity) = self.entities.iter_mut().find(|e| e.id == *entity_id)
        {
            if entity.kind == EntityKind::Enum
                && member.kind == MemberKind::Method
                && member.visibility == Visibility::Default
                && !entity
                    .members
                    .iter()
                    .any(|m| m.kind == MemberKind::Separator)
            {
                member.kind = MemberKind::Field;
            }
            entity.members.push(member);
        }
    }
}

fn dedent_note_lines(lines: &mut [String]) {
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.bytes()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count()
        })
        .min()
        .unwrap_or(0);
    for line in lines {
        if line.trim().is_empty() {
            line.clear();
        } else {
            line.drain(..indent);
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

/// If `name` ends with a balanced `<...>` generic suffix, strip it in place
/// and return the inner text (e.g. `Foo<T>` → name becomes `Foo`, returns
/// `Some("T")`). Handles nested angle brackets (`Wrapper<Container<T>>`).
/// Returns `None` when there is no trailing generic.
fn split_generic(name: &mut String) -> Option<String> {
    let trimmed = name.trim_end();
    if !trimmed.ends_with('>') {
        return None;
    }
    // Walk back from the end matching nested angle brackets.
    let bytes = trimmed.as_bytes();
    let mut depth = 0i32;
    let mut open_idx = None;
    for (i, &b) in bytes.iter().enumerate().rev() {
        match b {
            b'>' => depth += 1,
            b'<' => {
                depth -= 1;
                if depth == 0 {
                    open_idx = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open_idx?;
    // The base name must be non-empty (avoid stripping a leading `<...>`).
    if open == 0 {
        return None;
    }
    let inner = trimmed[open + 1..trimmed.len() - 1].trim().to_string();
    let base = trimmed[..open].trim_end().to_string();
    *name = base;
    Some(inner)
}

fn strip_empty_inline_body(line: &str) -> &str {
    let trimmed = line.trim_end();
    if let Some(before_close) = trimmed.strip_suffix('}') {
        let before_close = before_close.trim_end();
        if let Some(before_open) = before_close.strip_suffix('{') {
            return before_open.trim_end();
        }
        return trimmed;
    }
    trimmed.trim_end_matches('{').trim_end()
}

fn parse_entity_kind(s: &str) -> EntityKind {
    match s {
        "abstract class" | "abstract" => EntityKind::AbstractClass,
        "interface" => EntityKind::Interface,
        "enum" => EntityKind::Enum,
        "annotation" => EntityKind::Annotation,
        "entity" => EntityKind::Entity,
        "object" => EntityKind::Object,
        "state" => EntityKind::State,
        "circle" => EntityKind::Circle,
        "diamond" => EntityKind::Diamond,
        "actor" => EntityKind::Actor,
        "usecase" => EntityKind::UseCase,
        "component" => EntityKind::Component,
        "database" => EntityKind::Database,
        "queue" => EntityKind::Queue,
        "boundary" => EntityKind::Boundary,
        "node" => EntityKind::Node,
        "rectangle" => EntityKind::Rectangle,
        _ => EntityKind::Class,
    }
}

/// Strip arrow modifiers from a relationship line so the shape regex can
/// match cleanly. PlantUML allows colour, thickness, direction and style
/// adornments inside square brackets on the arrow body
/// (e.g. `A -[#blue]- B`, `A -[#red,dashed]-> B`), plus bare direction
/// words between dashes (`A -down-> B`, `A .left.> B`).
///
/// The replacement collapses bracketed modifiers to nothing and bare
/// direction keywords to the empty string so the resulting arrow shape
/// (`--`, `-->`, `..`, etc.) survives untouched.
#[derive(Clone, Copy)]
enum QueueDirection {
    Left,
    Right,
    Up,
    Down,
}

impl QueueDirection {
    fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    fn inverts_link(self) -> bool {
        matches!(self, Self::Left | Self::Up)
    }
}

fn strip_arrow_modifiers(line: &str) -> (String, RelationshipStyle, Option<QueueDirection>) {
    static BRACKETED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[^\]]*\]").unwrap());
    // Direction keywords appearing between dash/dot runs on the arrow body.
    static DIRECTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"([-.])(left|right|up|down|le?|ri?|up?|do?)([-.])").unwrap());
    // Bracketed `[...]` arrow modifiers must be stripped, but brackets inside a
    // double-quoted endpoint name (e.g. `"Class[WithBrackets]"`) are part of
    // the name. Strip only outside quoted spans by masking quoted regions.
    let mut style = RelationshipStyle::default();
    let mut strip_segment = |segment: &str| {
        BRACKETED
            .replace_all(segment, |caps: &regex::Captures<'_>| {
                apply_relationship_style(&caps[0][1..caps[0].len() - 1], &mut style);
                ""
            })
            .into_owned()
    };
    let s: String = if line.contains('"') {
        let mut out = String::with_capacity(line.len());
        let mut rest = line;
        loop {
            match rest.find('"') {
                None => {
                    out.push_str(&strip_segment(rest));
                    break;
                }
                Some(open) => {
                    out.push_str(&strip_segment(&rest[..open]));
                    let after = &rest[open + 1..];
                    match after.find('"') {
                        None => {
                            // Unterminated quote: keep the remainder verbatim.
                            out.push_str(&rest[open..]);
                            break;
                        }
                        Some(close) => {
                            out.push_str(&rest[open..open + 1 + close + 1]);
                            rest = &after[close + 1..];
                        }
                    }
                }
            }
        }
        out
    } else {
        strip_segment(line)
    };
    let direction = DIRECTION.captures_iter(&s).find_map(|caps| {
        let direction = caps.get(2)?.as_str().as_bytes()[0].to_ascii_lowercase();
        match direction {
            b'l' => Some(QueueDirection::Left),
            b'r' => Some(QueueDirection::Right),
            b'u' => Some(QueueDirection::Up),
            b'd' => Some(QueueDirection::Down),
            _ => None,
        }
    });
    let stripped = DIRECTION.replace_all(&s, "$1$3").into_owned();
    (stripped, style, direction)
}

fn invert_relationship_end(end: RelationshipEnd) -> RelationshipEnd {
    match end {
        RelationshipEnd::From => RelationshipEnd::To,
        RelationshipEnd::To => RelationshipEnd::From,
        RelationshipEnd::None | RelationshipEnd::Both => end,
    }
}

/// Port of PlantUML `WithLinkType.applyOneStyle`. `CommandLinkClass` accepts
/// these comma-separated tokens in either arrow-style bracket and applies
/// them to one generative link model before layout and rendering.
fn apply_relationship_style(raw: &str, style: &mut RelationshipStyle) {
    for token in raw
        .split([',', ';'])
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let lower = token.to_ascii_lowercase();
        match lower.as_str() {
            "dashed" => style.line_style = Some(EntityLineStyle::Dashed),
            "dotted" => style.line_style = Some(EntityLineStyle::Dotted),
            "bold" => style.line_style = Some(EntityLineStyle::Bold),
            "hidden" => style.hidden = true,
            _ => {
                if let Some(value) = lower.strip_prefix("thickness=") {
                    if let Ok(thickness) = value.parse() {
                        style.thickness = Some(thickness);
                    }
                } else if token.starts_with('#') {
                    style.color = Some(token.to_string());
                }
            }
        }
    }
}

/// Port of PlantUML `CommandLinkClass.getQueueLength`: endpoint decoration
/// characters do not affect rank length; only the arrow's line run does.
fn relationship_length(arrow: &str) -> usize {
    arrow
        .chars()
        .filter(|ch| matches!(ch, '-' | '.' | '='))
        .count()
        .max(1)
}

/// Parse the relationship kind, whether the line style is dashed, and which
/// endpoint carries PlantUML's built-in decoration.
fn parse_relationship_kind(s: &str) -> (RelationshipKind, bool, RelationshipEnd) {
    if s.contains("<|--") || s.contains("--|>") || s.contains("<|--|>") {
        let decorated_end = if s.contains("<|--|>") {
            RelationshipEnd::Both
        } else if s.contains("<|--") {
            RelationshipEnd::From
        } else {
            RelationshipEnd::To
        };
        (RelationshipKind::Inheritance, false, decorated_end)
    } else if s.contains("..|>") || s.contains("<|..") {
        let decorated_end = if s.contains("<|..") {
            RelationshipEnd::From
        } else {
            RelationshipEnd::To
        };
        (RelationshipKind::Implementation, true, decorated_end)
    } else if s.contains("*--") || s.contains("--*") {
        let decorated_end = if s.contains("*--") {
            RelationshipEnd::From
        } else {
            RelationshipEnd::To
        };
        (RelationshipKind::Composition, false, decorated_end)
    } else if s.contains("o--") || s.contains("--o") {
        let decorated_end = if s.contains("o--") {
            RelationshipEnd::From
        } else {
            RelationshipEnd::To
        };
        (RelationshipKind::Aggregation, false, decorated_end)
    } else if s.contains("..>") || s.contains("<..") {
        // Dashed dependency (..>)
        // PlantUML `CommandLinkClass.getLinkType` looks up `ARROW_HEAD1` and
        // `ARROW_HEAD2` independently through `LinkDecor`, so `<..>` carries
        // an `ARROW` decoration at both endpoints.
        let decorated_end = if s.contains("<..") && s.contains("..>") {
            RelationshipEnd::Both
        } else if s.contains("<..") {
            RelationshipEnd::From
        } else {
            RelationshipEnd::To
        };
        (RelationshipKind::Dependency, true, decorated_end)
    } else if s.contains("-->")
        || s.contains("<--")
        || s.contains("->>")
        || s == "->"
        || s == "<-"
        || s == "<-->"
        || s == "<->"
    {
        // Solid dependency (-->)
        let decorated_end = if s == "<-->" || s == "<->" {
            RelationshipEnd::Both
        } else if s.contains("<--") || s == "<-" {
            RelationshipEnd::From
        } else {
            RelationshipEnd::To
        };
        (RelationshipKind::Dependency, false, decorated_end)
    } else if s.contains("..") {
        // Dashed association (..)
        (RelationshipKind::Association, true, RelationshipEnd::None)
    } else {
        // Plain association (-- or ---- etc.)
        (RelationshipKind::Association, false, RelationshipEnd::None)
    }
}

/// PlantUML `StringWithArrow` treats a lone, leading, or trailing `<`/`>` in
/// a single-line relationship label as a directional guide arrow and removes
/// it from the text passed to SVEK.
fn parse_label_arrow(label: Option<&str>) -> (Option<String>, LinkArrow) {
    let Some(label) = label else {
        return (None, LinkArrow::None);
    };
    let label = label.trim();
    let label = label
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(label);

    if label.contains("\\n") || label.contains('\n') {
        return (Some(label.to_string()), LinkArrow::None);
    }

    let (text, arrow) = if label == "<" {
        (None, LinkArrow::Backward)
    } else if label == ">" {
        (None, LinkArrow::Direct)
    } else if let Some(text) = label.strip_prefix("< ") {
        (Some(text.trim().to_string()), LinkArrow::Backward)
    } else if let Some(text) = label.strip_prefix("> ") {
        (Some(text.trim().to_string()), LinkArrow::Direct)
    } else if let Some(text) = label.strip_suffix(" >") {
        (Some(text.trim().to_string()), LinkArrow::Direct)
    } else if let Some(text) = label.strip_suffix(" <") {
        (Some(text.trim().to_string()), LinkArrow::Backward)
    } else {
        (Some(label.to_string()), LinkArrow::None)
    };
    (text.filter(|text| !text.is_empty()), arrow)
}

/// Port of `LinkArg.build` -> `Display.manageGuillemet` ->
/// `Guillemet.GUILLEMET.manageGuillemet`. Class link labels replace each
/// comparator-delimited stereotype group before both layout and rendering.
fn normalize_link_label_guillemets(label: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<<\s?((?:<&\w+>|[^<>])+?)\s?>>").expect("valid guillemet regex")
    });
    RE.replace_all(label, "«$1»").into_owned()
}

fn parse_endpoint_decor(s: &str) -> Option<EndpointDecor> {
    match s {
        "}" | "{" => Some(EndpointDecor::CrowFoot),
        "}o" | "o{" => Some(EndpointDecor::CircleCrowFoot),
        "|o" | "o|" => Some(EndpointDecor::CircleLine),
        "||" => Some(EndpointDecor::DoubleLine),
        "}|" | "|{" => Some(EndpointDecor::LineCrowFoot),
        _ => None,
    }
}

#[derive(Default)]
struct ParsedEntityColors {
    back: Option<String>,
    line: Option<String>,
    text: Option<String>,
    line_style: Option<EntityLineStyle>,
}

/// Port of PlantUML `Colors(String, HColorSet, ColorType)`: split the entity
/// color suffix on semicolons, route named channels independently, and retain
/// the specific line stroke. The terminating-character check excludes custom
/// spot colors inside `<< (X,#RRGGBB) Name >>`.
fn parse_entity_colors(line: &str) -> ParsedEntityColors {
    // `CommandCreateClass.getRegexConcat` parses this suffix separately from
    // `ColorParser.simpleColor(BACK)`, then adds it as `ColorType.LINE`.
    static LEGACY_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"##(?:\[(dotted|dashed|bold)\])?([A-Za-z0-9_-]+)?(?:\s|\{|$)").unwrap()
    });
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"#([A-Za-z0-9.:;|/\\#-]+)(?:\s|\{|$)").unwrap());
    let legacy_line = LEGACY_LINE_RE.captures(line);
    let color_scope = legacy_line
        .as_ref()
        .and_then(|caps| caps.get(0))
        .map_or(line, |suffix| &line[..suffix.start()]);
    let raw = RE
        .captures_iter(color_scope)
        .last()
        .map(|caps| caps[1].replace('#', ""));

    let lower = raw.as_deref().unwrap_or_default().to_ascii_lowercase();
    let mut colors = ParsedEntityColors {
        line: legacy_line
            .as_ref()
            .and_then(|caps| caps.get(2))
            .map(|color| color.as_str().to_string()),
        line_style: legacy_line
            .as_ref()
            .and_then(|caps| caps.get(1))
            .and_then(|style| match style.as_str() {
                "dotted" => Some(EntityLineStyle::Dotted),
                "dashed" => Some(EntityLineStyle::Dashed),
                "bold" => Some(EntityLineStyle::Bold),
                _ => None,
            }),
        ..ParsedEntityColors::default()
    };
    colors.line_style = if lower.contains("line.dashed") {
        Some(EntityLineStyle::Dashed)
    } else if lower.contains("line.dotted") {
        Some(EntityLineStyle::Dotted)
    } else if lower.contains("line.bold") {
        Some(EntityLineStyle::Bold)
    } else {
        colors.line_style
    };
    for token in raw
        .as_deref()
        .unwrap_or_default()
        .split(';')
        .filter(|token| !token.is_empty())
    {
        let Some((name, value)) = token.split_once(':') else {
            if !token.contains('.') {
                colors.back = Some(format!("#{token}"));
            }
            continue;
        };
        match name
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "back" => colors.back = Some(value.to_string()),
            "line" => colors.line = Some(value.to_string()),
            "text" => colors.text = Some(value.to_string()),
            _ => {}
        }
    }
    colors
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

    text = normalize_inline_stereotypes(&text);

    // Parse visibility prefix. Double-character creole markers (`**`, `--`,
    // `~~`, `__`) take precedence over visibility prefixes that share the
    // same leading character — leave the markup intact for the renderer.
    let (visibility, rest) = match text.chars().next() {
        Some('+') => (Visibility::Public, &text[1..]),
        Some('-') if !text.starts_with("--") => (Visibility::Private, &text[1..]),
        Some('#') => (Visibility::Protected, &text[1..]),
        Some('~') if !text.starts_with("~~") => (Visibility::Package, &text[1..]),
        // ER diagrams use '*' to mark required/primary-key fields.
        Some('*') if !text.starts_with("**") => (Visibility::IeMandatory, &text[1..]),
        _ => (Visibility::Default, text.as_str()),
    };

    let rest = rest.trim();

    // Determine if method (contains parens) or field. A `:` BEFORE any `(`
    // signals the line is a typed field (e.g. ER-table `name : VARCHAR(20)`):
    // the parens belong to the type, not a method signature.
    let is_method = match (rest.find('('), rest.find(':')) {
        (Some(paren), Some(colon)) => paren < colon,
        (Some(_), None) => true,
        _ => false,
    };

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

/// Process a stereotype string that may contain spot notation `(S,#color) Name`.
///
/// Returns the stereotype display text, character, and hex spot color (with
/// leading `#`) when the spot uses valid spot syntax. PlantUML's behavior:
/// - Named color (e.g. `#red`, `#blue`): keep the full `(S,#color) Name` prefix
///   in the text and return no spot color (named colors don't fill the circle).
/// - Hex code (e.g. `#FF7700`, `#00AAFF`): strip the `(S,#color)` prefix,
///   returning just the name plus the hex color for the circle fill.
fn process_spot_stereotype(s: &str) -> (String, Option<char>, Option<String>) {
    let s = s.trim();
    // Look for spot notation: `(X,#color) Name`
    if let Some(rest) = s.strip_prefix('(')
        && let Some(close) = rest.find(')')
    {
        let spot_inner = &rest[..close];
        let after = rest[close + 1..].trim();
        // spot_inner should be like `A,#red` or `F,#FF7700`
        if let Some(comma) = spot_inner.find(',') {
            let character = spot_inner[..comma].trim();
            let color_part = spot_inner[comma + 1..].trim();
            if let Some(color_hex) = color_part.strip_prefix('#') {
                // strip leading #
                let is_hex =
                    !color_hex.is_empty() && color_hex.chars().all(|c| c.is_ascii_hexdigit());
                let valid_character = character.len() == 1
                    && character
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_');
                if is_hex && color_hex.len() == 6 && valid_character {
                    // Hex color: strip spot prefix, return just the name and
                    // capture the hex color for the circle fill.
                    return (
                        after.to_string(),
                        character.chars().next(),
                        Some(format!("#{color_hex}")),
                    );
                }
            }
        }
    }
    // Named color or no spot notation: return as-is.
    (s.to_string(), None, None)
}

fn normalize_inline_stereotypes(s: &str) -> String {
    let mut text = s.to_string();
    while let Some(start) = text.find("<<") {
        if let Some(end) = text[start..].find(">>") {
            let inner = text[start + 2..start + end].to_string();
            text = format!("{}«{}»{}", &text[..start], inner, &text[start + end + 2..]);
        } else {
            break;
        }
    }
    text
}

/// Parse the package command's URL slot with `UrlBuilder`'s STRICT grammar.
///
/// Java provenance: `UrlBuilder#getUrl` accepts quoted and unquoted hrefs,
/// optional brace tooltips, and optional labels. `UrlBuilder#withTopUrl`
/// prefixes every non-http(s)/file href with the active `topurl`.
fn parse_package_link(raw: &str, topurl: Option<&str>) -> Option<(String, Option<String>)> {
    let inner = raw.strip_prefix("[[")?.strip_suffix("]]")?.trim();
    if inner.is_empty() || inner.contains(['[', ']']) {
        return None;
    }

    let (href, mut rest, has_href) = if let Some(quoted) = inner.strip_prefix('"') {
        let quote_end = quoted.find('"')?;
        if quote_end == 0 {
            return None;
        }
        (
            quoted[..quote_end].to_string(),
            &quoted[quote_end + 1..],
            true,
        )
    } else if inner.starts_with('{') {
        (String::new(), inner, false)
    } else {
        let href_end = inner
            .find(|ch: char| ch.is_whitespace() || ch == '{')
            .unwrap_or(inner.len());
        if href_end == 0 {
            return None;
        }
        let (href, remaining) = super::extract_link_url(raw);
        if !remaining.is_empty() {
            return None;
        }
        (href?, &inner[href_end..], true)
    };

    let leading_separator = rest.chars().next().is_some_and(char::is_whitespace);
    rest = rest.trim_start();
    let tooltip = if let Some(after_open) = rest.strip_prefix('{') {
        let close = after_open.find('}')?;
        let value = &after_open[..close];
        if value.contains('{') {
            return None;
        }
        rest = &after_open[close + 1..];
        if value.is_empty() {
            Some(String::new())
        } else {
            super::extract_link_tooltip(raw)
        }
    } else {
        None
    };

    let had_label_separator = if tooltip.is_some() && !has_href {
        true
    } else if tooltip.is_some() {
        rest.chars().next().is_some_and(char::is_whitespace)
    } else {
        leading_separator
    };
    rest = rest.trim_start();
    if !rest.is_empty() && (!had_label_separator || rest.contains(['{', '}', '[', ']'])) {
        return None;
    }

    let href = if has_href
        && !href.starts_with("http:")
        && !href.starts_with("https:")
        && !href.starts_with("file:")
        && let Some(topurl) = topurl
    {
        format!("{topurl}{href}")
    } else {
        href
    };
    Some((href, tooltip))
}

fn looks_like_package_opener(line: &str) -> bool {
    if !line.trim_end().ends_with('{') {
        return false;
    }
    let keyword = line.split_whitespace().next().unwrap_or_default();
    keyword.eq_ignore_ascii_case("namespace") || PackageKind::from_command_symbol(keyword).is_some()
}

fn text_outside_double_quotes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_quote = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            out.push(' ');
        } else if in_quote {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
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
    fn direction_commands_update_class_layout_direction() {
        let left_to_right = parse("left to right direction\nclass FreshA\nclass FreshB");
        assert_eq!(left_to_right.direction, ClassLayoutDirection::LeftToRight);

        let reset = parse(
            "left to right direction\n\
             top to bottom direction\n\
             class FreshA\n\
             class FreshB",
        );
        assert_eq!(reset.direction, ClassLayoutDirection::TopToBottom);
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
    fn multiline_title_preserves_lines() {
        let d = parse("title\n  My Class Diagram\n  Version 1.0\nend title\nclass A");
        assert_eq!(
            d.meta.title.as_deref(),
            Some("My Class Diagram\nVersion 1.0")
        );
    }

    #[test]
    fn entity_types() {
        let d = parse(
            "class A\nabstract class B\ninterface C\nenum D\nannotation E\nentity F\nstate G",
        );
        assert_eq!(d.entities[0].kind, EntityKind::Class);
        assert_eq!(d.entities[1].kind, EntityKind::AbstractClass);
        assert_eq!(d.entities[2].kind, EntityKind::Interface);
        assert_eq!(d.entities[3].kind, EntityKind::Enum);
        assert_eq!(d.entities[4].kind, EntityKind::Annotation);
        assert_eq!(d.entities[5].kind, EntityKind::Entity);
        assert_eq!(d.entities[6].kind, EntityKind::State);
    }

    #[test]
    fn symbol_entity_declarations() {
        let d = parse("class Foo\ncircle Bar\ndiamond Baz");
        assert_eq!(d.entities[1].id, "Bar");
        assert_eq!(d.entities[1].kind, EntityKind::Circle);
        assert_eq!(d.entities[1].source_line, 2);
        assert_eq!(d.entities[2].id, "Baz");
        assert_eq!(d.entities[2].kind, EntityKind::Diamond);
        assert_eq!(d.entities[2].source_line, 3);
    }

    #[test]
    fn allowmixing_description_leaves_keep_declaration_kind_and_location() {
        let d = parse(
            "allowmixing\n\
             actor \"Release Operator\" as Operator\n\
             usecase \"Approve Rollout\" as Approval\n\
             component PolicyEngine\n\
             database MetricsStore\n\
             queue RetryQueue\n\
             BoUnDaRy AccessEdge\n\
             node EdgeHost\n\
             rectangle TrustBoundary\n\
             Operator --> Approval\n\
             Approval --> PolicyEngine",
        );
        let expected = [
            ("Operator", EntityKind::Actor, 2),
            ("Approval", EntityKind::UseCase, 3),
            ("PolicyEngine", EntityKind::Component, 4),
            ("MetricsStore", EntityKind::Database, 5),
            ("RetryQueue", EntityKind::Queue, 6),
            ("AccessEdge", EntityKind::Boundary, 7),
            ("EdgeHost", EntityKind::Node, 8),
            ("TrustBoundary", EntityKind::Rectangle, 9),
        ];
        for (id, kind, source_line) in expected {
            let entity = d.entities.iter().find(|entity| entity.id == id).unwrap();
            assert_eq!(entity.kind, kind);
            assert_eq!(entity.source_line, source_line);
        }
    }

    #[test]
    fn separator_none_reuses_the_first_created_duplicate_quark() {
        let d = parse(
            "package SepNoneAlpha {\n\
               class EchoToken\n\
             }\n\
             package SepNoneBeta {\n\
               class EchoToken\n\
             }\n\
             set namespaceSeparator none\n\
             class \"Root Echo Declaration\" as EchoToken\n\
             EchoToken --> SepNoneSink",
        );

        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            [
                "SepNoneAlpha.EchoToken",
                "SepNoneBeta.EchoToken",
                "SepNoneSink"
            ]
        );
        assert_eq!(d.entities[0].label, "EchoToken");
        assert_eq!(d.relationships[0].from, "SepNoneAlpha.EchoToken");
    }

    #[test]
    fn namespace_separator_none_is_replaced_by_later_custom_separators() {
        let d = parse(
            "set separator NONE\n\
             class Literal.Dot\n\
             SET namespaceSeparator ::\n\
             class ColonRealm::InnerRealm::TransitionLeaf\n\
             set namespaceseparator nOnE\n\
             class \"Renamed Literal\" as Literal.Dot\n\
             set namespaceSeparator /\n\
             class SlashRealm/InnerRealm/FinalLeaf",
        );

        assert_eq!(
            d.packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>(),
            [
                "ColonRealm",
                "ColonRealm::InnerRealm",
                "SlashRealm",
                "SlashRealm/InnerRealm",
            ]
        );
        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            [
                "Literal.Dot",
                "ColonRealm::InnerRealm::TransitionLeaf",
                "SlashRealm/InnerRealm/FinalLeaf",
            ]
        );
        assert_eq!(d.entities[0].label, "Literal.Dot");
        assert_eq!(d.entities[1].source_line, 4);
        assert_eq!(d.entities[2].source_line, 8);
    }

    #[test]
    fn separator_none_package_lookup_uses_shared_quark_creation_order() {
        let first_then_second = parse(
            "package FirstRealm {\n\
               package SharedGate {\n\
               }\n\
             }\n\
             package SecondRealm {\n\
               package SharedGate {\n\
               }\n\
             }\n\
             set separator none\n\
             package \"Reopened First Gate\" as SharedGate {\n\
               class NestedLeaf\n\
             }",
        );

        let reopened = first_then_second
            .packages
            .iter()
            .find(|package| package.name == "FirstRealm.SharedGate")
            .unwrap();
        assert_eq!(reopened.display_name.as_deref(), Some("SharedGate"));
        assert_eq!(
            first_then_second
                .entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["FirstRealm.SharedGate.NestedLeaf"]
        );

        let second_then_first = parse(
            "package SecondRealm {\n\
               package SharedGate {\n\
               }\n\
             }\n\
             package FirstRealm {\n\
               package SharedGate {\n\
               }\n\
             }\n\
             set separator NONE\n\
             package \"Reopened Second Gate\" as SharedGate {\n\
               class NestedLeaf\n\
             }",
        );
        let reopened = second_then_first
            .packages
            .iter()
            .find(|package| package.name == "SecondRealm.SharedGate")
            .unwrap();
        assert_eq!(reopened.display_name.as_deref(), Some("SharedGate"));
        assert_eq!(
            second_then_first
                .entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["SecondRealm.SharedGate.NestedLeaf"]
        );
    }

    #[test]
    fn separator_none_package_lookup_can_reuse_an_entity_quark() {
        let d = parse(
            "class SharedIdentity\n\
             set namespaceseparator NoNe\n\
             package \"Shared Package\" as SharedIdentity {\n\
               class PackageChild\n\
             }",
        );

        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].name, "SharedIdentity");
        assert_eq!(
            d.packages[0].display_name.as_deref(),
            Some("Shared Package")
        );
        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["SharedIdentity", "SharedIdentity.PackageChild"]
        );
    }

    #[test]
    fn mixed_remote_reuse_leaves_the_current_group_without_owned_entities() {
        let d = parse(
            "allowmixing\n\
             package BridgeClasses {\n\
               class TransitBridge\n\
             }\n\
             package BridgeActors {\n\
               actor \"Transit Bridge Actor\" as TransitBridge\n\
             }\n\
             TransitBridge --> BridgeTerminal",
        );

        assert_eq!(d.entities[0].id, "BridgeClasses.TransitBridge");
        assert_eq!(d.entities[0].label, "Transit Bridge Actor");
        assert_eq!(d.packages[0].entities, ["BridgeClasses.TransitBridge"]);
        assert!(d.packages[1].entities.is_empty());
    }

    #[test]
    fn later_alias_refines_implicit_endpoint_identity_without_moving_ownership() {
        let d = parse(
            "allowmixing\n\
             class DispatchAnchor\n\
             SlateOperator --> SlateApproval\n\
             SlateApproval --> SlatePolicy\n\
             SlatePolicy --> SlateRetry\n\
             actor \"Slate Operator\" as SlateOperator\n\
             usecase \"Approve Slate\" as SlateApproval\n\
             component \"Slate Policy\" as SlatePolicy\n\
             queue \"Slate Retry\" as SlateRetry",
        );

        for (id, label, source_line) in [
            ("SlateOperator", "Slate Operator", 3),
            ("SlateApproval", "Approve Slate", 3),
            ("SlatePolicy", "Slate Policy", 4),
            ("SlateRetry", "Slate Retry", 5),
        ] {
            let entity = d.entities.iter().find(|entity| entity.id == id).unwrap();
            assert_eq!(entity.label, label);
            assert!(entity.explicit_alias);
            assert_eq!(entity.source_line, source_line);
            assert_eq!(entity.kind, EntityKind::Class);
        }
    }

    #[test]
    fn allow_mixing_command_consumption_uses_the_shared_complete_grammar() {
        let mut parser = ClassParser::new();
        assert!(parser.try_meta("ALLOW_MIXING"));
        assert!(parser.try_meta("allowmixing"));
        assert!(!parser.try_meta("allow_mixing trailing"));
        assert!(!parser.try_meta("allowmixing trailing"));
    }

    #[test]
    fn braced_description_keywords_remain_containers() {
        let d = parse(
            "allowmixing\n\
             database DataZone {\n\
               class Record\n\
             }\n\
             node Runtime {\n\
               class Worker\n\
             }\n\
             rectangle Boundary {\n\
               class Gateway\n\
             }",
        );
        assert_eq!(d.packages.len(), 3);
        assert_eq!(
            d.packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>(),
            ["DataZone", "Runtime", "Boundary"]
        );
        assert!(d.entities.iter().all(|entity| !matches!(
            entity.kind,
            EntityKind::Database | EntityKind::Node | EntityKind::Rectangle
        )));
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
    fn relationship_arrow_styles_form_one_link_model() {
        let d = parse(
            "SignalEmitter701 -[#2E8B57,dotted]-> AuditSink709\n\
             AuditSink709 -[thickness=3]-> ColdStore719\n\
             SignalEmitter701 -[hidden]-> ColdStore719",
        );
        assert_eq!(d.relationships.len(), 3);
        assert!(
            d.relationships
                .iter()
                .all(|relationship| relationship.length == 2)
        );
        assert_eq!(
            d.relationships[0].style,
            RelationshipStyle {
                color: Some("#2E8B57".into()),
                line_style: Some(EntityLineStyle::Dotted),
                thickness: None,
                hidden: false,
                inverted: false,
                declaration: false,
            }
        );
        assert_eq!(d.relationships[1].style.thickness, Some(3));
        assert!(d.relationships[2].style.hidden);
    }

    #[test]
    fn relationship_label() {
        let d = parse("Parent -- Child : has");
        assert_eq!(d.relationships[0].label.as_deref(), Some("has"));
    }

    #[test]
    fn qualified_relationship_endpoint_resolves_package_member() {
        let d = parse(
            "package service {\n  class UserService\n}\npackage model {\n  class User\n}\nservice.UserService ..> model.User",
        );

        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from, "service.UserService");
        assert_eq!(d.relationships[0].to, "model.User");
    }

    #[test]
    fn empty_package_relationship_endpoint_reuses_group_identity() {
        let d = parse(
            "package \"Empty Parcel\" as EmptyParcel {\n\
             }\n\
             class Receiver\n\
             EmptyParcel --> Receiver",
        );

        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].id, "Receiver");
        assert_eq!(d.relationships[0].from, "EmptyParcel");
        assert_eq!(
            d.uid_events
                .iter()
                .filter(|event| matches!(event, ClassUidEvent::Entity(_)))
                .count(),
            1
        );
    }

    #[test]
    fn quoted_relationship_prefers_exact_existing_package_identity() {
        let d = parse(
            "package \"Quoted Parcel\" {\n\
             }\n\
             class Receiver\n\
             \"Quoted Parcel\" --> Receiver",
        );

        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.packages[0].name, "Quoted Parcel");
        assert_eq!(d.relationships[0].from, "Quoted Parcel");
    }

    #[test]
    fn quoted_relationship_falls_back_to_normalized_class_identity() {
        let d = parse(
            "class \"Fish & Chips\"\n\
             class Receiver\n\
             \"Fish & Chips\" --> Receiver",
        );

        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.relationships[0].from, d.entities[0].id);
    }

    #[test]
    fn package() {
        let d = parse("package com.example {\n  class Foo\n  class Bar\n}");
        assert_eq!(d.packages.len(), 2);
        assert_eq!(d.packages[0].name, "com");
        assert_eq!(d.packages[1].name, "com.example");
        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.entities[0].id, "com.example.Foo");
        assert_eq!(d.entities[1].id, "com.example.Bar");
    }

    #[test]
    fn renamed_together_scopes_preserve_direct_entities_packages_and_children() {
        let d = parse(
            "together {\n\
               class FreshAlpha3527\n\
               class FreshBeta3529\n\
               together {\n\
                 class FreshNested3533\n\
               }\n\
               package FreshBundle3539 {\n\
                 class FreshPackaged3541\n\
               }\n\
             }\n\
             class FreshOutside3547",
        );

        assert_eq!(d.together.len(), 2);
        assert_eq!(d.together[0].parent, None);
        assert_eq!(d.together[0].entities, ["FreshAlpha3527", "FreshBeta3529"]);
        assert_eq!(d.together[0].packages, ["FreshBundle3539"]);
        assert_eq!(d.together[1].parent, Some(0));
        assert_eq!(d.together[1].entities, ["FreshNested3533"]);
    }

    #[test]
    fn preprocessed_renamed_together_siblings_do_not_become_nested() {
        let source = "@startuml\n\
            together {\n\
              class FreshFirst3557\n\
              class FreshSecond3559\n\
            }\n\
            together {\n\
              class FreshThird3563\n\
              class FreshFourth3571\n\
            }\n\
            @enduml";
        let crate::diagram::Diagram::Class(d) = crate::parse::parse(source).unwrap() else {
            panic!("expected class diagram");
        };

        assert_eq!(d.together.len(), 2);
        assert_eq!(d.together[0].parent, None);
        assert_eq!(d.together[1].parent, None);
    }

    #[test]
    fn together_membership_belongs_to_first_materialization() {
        let d = parse(
            "together {\n\
               FreshImplicit3613 -- FreshImplicit3617\n\
               class FreshImplicit3613\n\
             }\n\
             together {\n\
               class FreshSibling3623\n\
               class FreshImplicit3613\n\
             }",
        );

        assert_eq!(
            d.together[0].entities,
            ["FreshImplicit3613", "FreshImplicit3617"]
        );
        assert_eq!(d.together[1].entities, ["FreshSibling3623"]);
    }

    #[test]
    fn package_top_breaks_outer_together_parentage() {
        let d = parse(
            "together {\n\
               package FreshOuterPackage3631 {\n\
                 together {\n\
                   class FreshPackaged3637\n\
                   class FreshPackaged3643\n\
                 }\n\
               }\n\
             }",
        );

        assert_eq!(d.together.len(), 2);
        assert_eq!(d.together[0].packages, ["FreshOuterPackage3631"]);
        assert_eq!(d.together[1].parent, None);
        assert_eq!(
            d.together[1].owner_package.as_deref(),
            Some("FreshOuterPackage3631")
        );
        assert_eq!(
            d.together[1].entities,
            [
                "FreshOuterPackage3631.FreshPackaged3637",
                "FreshOuterPackage3631.FreshPackaged3643"
            ]
        );
    }

    #[test]
    fn explicit_dotted_namespace_materializes_parent_quarks_on_first_leaf() {
        let d = parse(
            "namespace telemetry.pipeline.archive {\n\
             skinparam classBorderColor DarkGreen\n\
             class FreshPacket1301\n\
             class FreshLedger1303\n\
             }",
        );

        assert_eq!(d.packages.len(), 3);
        assert_eq!(d.packages[0].name, "telemetry");
        assert_eq!(d.packages[0].source_line, 3);
        assert!(d.packages[0].phantom);
        assert_eq!(d.packages[1].name, "telemetry.pipeline");
        assert_eq!(d.packages[1].source_line, 3);
        assert!(d.packages[1].phantom);
        assert_eq!(d.packages[2].name, "telemetry.pipeline.archive");
        assert_eq!(d.packages[2].source_line, 1);
        assert!(!d.packages[2].phantom);
        assert_eq!(d.packages[2].parent, Some(1));
        assert_eq!(
            d.packages[2].entities,
            [
                "telemetry.pipeline.archive.FreshPacket1301",
                "telemetry.pipeline.archive.FreshLedger1303"
            ]
        );
    }

    #[test]
    fn relationship_preselects_both_quarks_before_phantom_constructors() {
        let d = parse(
            "Client --> Outer.Ui.Facade.Controller\n\
             frame Outer.Ui.Facade as \"UI Facade\" {\n\
               class Controller\n\
             }\n\
             class Client",
        );
        let events = d
            .uid_events
            .iter()
            .map(|event| match event {
                ClassUidEvent::Package(name) => format!("package:{name}"),
                ClassUidEvent::Entity(id) => format!("entity:{id}"),
                ClassUidEvent::Relationship(index) => format!("relationship:{index}"),
                ClassUidEvent::Note { .. }
                | ClassUidEvent::DiscardedRelationship { .. }
                | ClassUidEvent::Association(_) => "other".to_string(),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            events,
            [
                "entity:Client",
                "package:Outer",
                "package:Outer.Ui",
                "package:Outer.Ui.Facade",
                "entity:Outer.Ui.Facade.Controller",
                "relationship:0",
            ]
        );
        assert_eq!(d.packages[2].display_name.as_deref(), Some("Facade"));
    }

    #[test]
    fn preselected_custom_separator_paths_share_one_phantom_sweep() {
        let d = parse(
            "set namespaceSeparator ::\n\
             Left::Branch::Leaf --> Right::Wing::Target",
        );
        let events = d
            .uid_events
            .iter()
            .filter_map(|event| match event {
                ClassUidEvent::Package(name) => Some(format!("package:{name}")),
                ClassUidEvent::Entity(id) => Some(format!("entity:{id}")),
                ClassUidEvent::Relationship(index) => Some(format!("relationship:{index}")),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            events,
            [
                "entity:Left::Branch::Leaf",
                "package:Left",
                "package:Left::Branch",
                "package:Right",
                "package:Right::Wing",
                "entity:Right::Wing::Target",
                "relationship:0",
            ]
        );
    }

    #[test]
    fn package_alias_separates_display_from_canonical_identity() {
        let d = parse(
            "package \"Service Display\" as ServiceCode {\n\
               class Gateway\n\
             }\n\
             ServiceCode.Gateway --> Audit",
        );

        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].name, "ServiceCode");
        assert_eq!(
            d.packages[0].display_name.as_deref(),
            Some("Service Display")
        );
        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["ServiceCode.Gateway", "Audit"]
        );
        assert_eq!(d.relationships[0].from, "ServiceCode.Gateway");
    }

    #[test]
    fn symbol_container_alias_separates_display_from_canonical_identity() {
        let d = parse(
            "node \"Compute Display\" as ComputeCode {\n\
               class Gateway\n\
             }\n\
             frame \"Outer Frame\" as OuterCode {\n\
               cloud \"Cloud Display\" as CloudCode {\n\
                 class Worker\n\
               }\n\
             }\n\
             ComputeCode.Gateway --> OuterCode.CloudCode.Worker",
        );

        assert_eq!(
            d.packages
                .iter()
                .map(|package| (
                    package.name.as_str(),
                    package.display_name.as_deref(),
                    package.kind
                ))
                .collect::<Vec<_>>(),
            [
                ("ComputeCode", Some("Compute Display"), PackageKind::Node),
                ("OuterCode", Some("Outer Frame"), PackageKind::Frame),
                (
                    "OuterCode.CloudCode",
                    Some("Cloud Display"),
                    PackageKind::Cloud
                )
            ]
        );
        assert_eq!(d.relationships[0].from, "ComputeCode.Gateway");
        assert_eq!(d.relationships[0].to, "OuterCode.CloudCode.Worker");
    }

    #[test]
    fn symbol_container_reverse_alias_uses_code_as_identity() {
        let d = parse(
            "folder ServiceCode as \"Service Display\" {\n\
               class Gateway\n\
             }\n\
             rectangle OuterCode as \"Outer Display\" {\n\
               class Audit\n\
             }\n\
             ServiceCode.Gateway --> OuterCode.Audit",
        );

        assert_eq!(
            d.packages
                .iter()
                .map(|package| (
                    package.name.as_str(),
                    package.display_name.as_deref(),
                    package.kind
                ))
                .collect::<Vec<_>>(),
            [
                ("ServiceCode", Some("Service Display"), PackageKind::Folder),
                ("OuterCode", Some("Outer Display"), PackageKind::Rectangle)
            ]
        );
        assert_eq!(d.relationships[0].from, "ServiceCode.Gateway");
        assert_eq!(d.relationships[0].to, "OuterCode.Audit");
    }

    #[test]
    fn package_modifiers_follow_java_command_order() {
        let d = parse(
            "package Service $before <<Application>> $after \
             [[https://example.com{Service docs}]] #LightBlue {\n\
               class Gateway\n\
             }\n\
             cloud CloudCode <<Platform>> #aliceblue {\n\
               class Worker\n\
             }",
        );

        assert_eq!(d.packages[0].name, "Service");
        assert_eq!(d.packages[0].stereotypes, ["Application"]);
        assert_eq!(d.packages[0].color.as_deref(), Some("LightBlue"));
        assert_eq!(d.packages[1].name, "CloudCode");
        assert_eq!(d.packages[1].stereotypes, ["Platform"]);
        assert_eq!(d.packages[1].color.as_deref(), Some("aliceblue"));
    }

    #[test]
    fn package_stereotype_sequences_preserve_group_ownership() {
        let d = parse(
            "package \"API\" as FallbackNode <<ObservabilityPortal>> <<AuditTrail>> {\n\
               class RootChild\n\
             }\n\
             database Envelope {\n\
               frame \"FR\" <<ApprovalWindow>> <<Beta>> as ExplicitFrame {\n\
                 class NestedChild\n\
               }\n\
             }",
        );

        let fallback = d
            .packages
            .iter()
            .find(|package| package.name == "FallbackNode")
            .unwrap();
        assert_eq!(fallback.stereotypes, ["ObservabilityPortal", "AuditTrail"]);
        assert!(
            d.entities
                .iter()
                .any(|entity| entity.id == "FallbackNode.RootChild")
        );

        let envelope_idx = d
            .packages
            .iter()
            .position(|package| package.name == "Envelope")
            .unwrap();
        let frame = d
            .packages
            .iter()
            .find(|package| package.name == "Envelope.ExplicitFrame")
            .unwrap();
        assert_eq!(frame.parent, Some(envelope_idx));
        assert_eq!(frame.stereotypes, ["ApprovalWindow", "Beta"]);
        assert!(
            d.entities
                .iter()
                .any(|entity| entity.id == "Envelope.ExplicitFrame.NestedChild")
        );
    }

    #[test]
    fn package_command_precedence_preserves_java_identity_and_stereotype_slots() {
        let d = parse(
            "package Host {\n\
               package \"Split Fallback\" <<PreOne>> as SplitFallback $core \
                 <<PostTwo>> <<PostThree>> #MistyRose {\n\
                 class SplitLeaf\n\
               }\n\
             }\n\
             cloud \"Split Explicit\" <<North>> <<South>> as SplitExplicit \
               $ops <<East>> #Thistle {\n\
               class CloudLeaf\n\
             }",
        );

        let fallback = d
            .packages
            .iter()
            .find(|package| package.name == "Host.Split Fallback")
            .unwrap();
        assert_eq!(fallback.stereotypes, ["PreOne", "PostTwo", "PostThree"]);
        assert!(
            d.entities
                .iter()
                .any(|entity| entity.id == "Host.Split Fallback.SplitLeaf")
        );

        let explicit = d
            .packages
            .iter()
            .find(|package| package.name == "SplitExplicit")
            .unwrap();
        assert_eq!(explicit.stereotypes, ["East"]);
        assert!(
            d.entities
                .iter()
                .any(|entity| entity.id == "SplitExplicit.CloudLeaf")
        );
    }

    #[test]
    fn every_package_usymbol_command_owns_its_nested_leaf() {
        for (keyword, expected_kind) in PackageKind::COMMAND_SYMBOLS {
            let code = format!("{keyword}Code");
            let declaration = if keyword == "package" {
                format!("{keyword} \"{keyword} Display\" as {code} <<Trailing>> {{")
            } else {
                format!("{keyword} \"{keyword} Display\" <<Identity>> as {code} <<Trailing>> {{")
            };
            let source = format!(
                "package Host {{\n\
                   {declaration}\n\
                     class Child\n\
                   }}\n\
                 }}"
            );
            let diagram = parse(&source);
            let qualified = format!("Host.{code}");
            let package = diagram
                .packages
                .iter()
                .find(|package| package.name == qualified)
                .unwrap_or_else(|| panic!("{keyword} did not create {qualified}"));
            let expected_display = format!("{keyword} Display");

            assert_eq!(package.kind, expected_kind, "{keyword}");
            assert_eq!(
                package.symbol_from_keyword,
                keyword != "package",
                "{keyword}"
            );
            assert_eq!(
                package.display_name.as_deref(),
                Some(expected_display.as_str()),
                "{keyword}"
            );
            assert_eq!(package.stereotypes, ["Trailing"], "{keyword}");
            assert!(
                diagram
                    .entities
                    .iter()
                    .any(|entity| entity.id == format!("{qualified}.Child")),
                "{keyword}"
            );
        }
    }

    #[test]
    fn malformed_package_usymbol_openers_all_fail_closed() {
        for (keyword, _) in PackageKind::COMMAND_SYMBOLS {
            let lines = [
                format!("{keyword} \"Broken\" <<Open> {{"),
                "class EscapedLeaf".to_string(),
                "}".to_string(),
            ];
            let error = parse_class(&lines).unwrap_err();
            assert_eq!(error.line, 1, "{keyword}");
            assert_eq!(
                error.message, "invalid package or symbol-container declaration",
                "{keyword}"
            );
        }
    }

    #[test]
    fn malformed_package_opener_fails_closed() {
        let lines = [
            "package \"Broken\" <<PreOne>> as Broken $ops <<PostTwo> #Pink {",
            "class BrokenLeaf",
            "}",
        ]
        .map(str::to_string);

        let error = parse_class(&lines).unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(
            error.message,
            "invalid package or symbol-container declaration"
        );
    }

    #[test]
    fn package_urls_retain_structured_metadata_on_creation_and_promotion() {
        let d = parse(
            "skinparam topurl https://docs.example/\n\
             class Promoted.Live\n\
             package Promoted $before <<Application>> $after \
             [[guide{Guide & docs} ignored-label]] #LightBlue {\n\
             }\n\
             package \"Absolute Display\" as Absolute \
             [[https://example.com/api optional-label]] {\n\
             }\n\
             node Systems.Runtime as \"Runtime Node\" $ops <<Execution>> $live \
             [[https://example.com/runtime]] #AliceBlue {\n\
             }\n\
             package Promoted {\n\
             }",
        );

        let promoted = d
            .packages
            .iter()
            .find(|package| package.name == "Promoted")
            .unwrap();
        assert_eq!(promoted.url.as_deref(), Some("https://docs.example/guide"));
        assert_eq!(promoted.url_tooltip.as_deref(), Some("Guide & docs"));
        assert_eq!(promoted.stereotypes, ["Application"]);
        assert!(!promoted.phantom);

        let absolute = d
            .packages
            .iter()
            .find(|package| package.name == "Absolute")
            .unwrap();
        assert_eq!(absolute.url.as_deref(), Some("https://example.com/api"));
        assert_eq!(absolute.url_tooltip, None);
        assert_eq!(absolute.display_name.as_deref(), Some("Absolute Display"));

        let runtime = d
            .packages
            .iter()
            .find(|package| package.name == "Systems.Runtime")
            .unwrap();
        assert_eq!(runtime.kind, PackageKind::Node);
        assert_eq!(runtime.display_name.as_deref(), Some("Runtime Node"));
        assert_eq!(runtime.stereotypes, ["Execution"]);
        assert_eq!(runtime.url.as_deref(), Some("https://example.com/runtime"));
        assert_eq!(runtime.color.as_deref(), Some("AliceBlue"));
    }

    #[test]
    fn package_link_parser_matches_strict_urlbuilder_forms() {
        assert_eq!(
            parse_package_link(
                r#"[[ "guide path" {Quoted docs} label ]]"#,
                Some("https://docs.example/")
            ),
            Some((
                "https://docs.example/guide path".to_string(),
                Some("Quoted docs".to_string())
            ))
        );
        assert_eq!(
            parse_package_link("[[{Tooltip only}label]]", Some("https://docs.example/")),
            Some(("".to_string(), Some("Tooltip only".to_string())))
        );
        assert_eq!(
            parse_package_link("[[file:guide.txt]]", Some("https://docs.example/")),
            Some(("file:guide.txt".to_string(), None))
        );
        assert_eq!(
            parse_package_link("[[https://example.com{broken]]", None),
            None
        );
    }

    #[test]
    fn alias_local_stereotype_precedes_as_clause() {
        let d = parse(
            "node \"Service Display\" <<Platform>> as ServiceCode #wheat {\n\
               class Gateway\n\
             }",
        );

        assert_eq!(d.packages[0].name, "ServiceCode");
        assert_eq!(
            d.packages[0].display_name.as_deref(),
            Some("Service Display")
        );
        assert_eq!(d.packages[0].stereotypes, ["Platform"]);
        assert_eq!(d.packages[0].color.as_deref(), Some("wheat"));
    }

    #[test]
    fn reopened_package_preserves_its_first_display() {
        let d = parse(
            "package \"Original Display\" as ServiceCode {\n\
               class First\n\
             }\n\
             together {\n\
               package ServiceCode {\n\
                 class Second\n\
               }\n\
             }",
        );

        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].name, "ServiceCode");
        assert_eq!(
            d.packages[0].display_name.as_deref(),
            Some("Original Display")
        );
        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["ServiceCode.First", "ServiceCode.Second"]
        );
    }

    #[test]
    fn nested_qualified_endpoint_reuses_the_existing_leaf() {
        let d = parse(
            "package Outer {\n\
               package \"Inner Display\" as Inner {\n\
                 class Gateway\n\
               }\n\
             }\n\
             Outer.Inner.Gateway --> Audit",
        );

        assert_eq!(
            d.packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>(),
            ["Outer", "Outer.Inner"]
        );
        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["Outer.Inner.Gateway", "Audit"]
        );
        assert_eq!(d.relationships[0].from, "Outer.Inner.Gateway");
    }

    #[test]
    fn custom_separator_resolves_nested_paths_without_duplication() {
        let d = parse(
            "set namespaceSeparator ::\n\
             namespace Platform::Ingress {\n\
               class Gateway\n\
             }\n\
             Platform::Ingress::Gateway --> Audit",
        );

        assert_eq!(
            d.entities
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<Vec<_>>(),
            ["Platform::Ingress::Gateway", "Audit"]
        );
        assert_eq!(d.relationships[0].from, "Platform::Ingress::Gateway");
    }

    #[test]
    fn unique_unqualified_endpoint_reuses_a_nested_leaf() {
        let d = parse(
            "package Services {\n\
               class Gateway\n\
             }\n\
             Gateway --> Audit",
        );

        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.relationships[0].from, "Services.Gateway");
    }

    #[test]
    fn stereotype() {
        let d = parse("class Foo <<singleton>>");
        assert_eq!(d.entities[0].stereotypes, vec!["singleton"]);
    }

    #[test]
    fn hex_spot_stereotype_preserves_arbitrary_character() {
        let d = parse("class Renamed << (G,#12ABEF) NewKind >>");
        let entity = &d.entities[0];
        assert_eq!(entity.stereotypes, vec!["NewKind"]);
        assert_eq!(entity.spot_character, Some('G'));
        assert_eq!(entity.spot_color.as_deref(), Some("#12ABEF"));
    }

    #[test]
    fn invalid_spot_syntax_remains_stereotype_text() {
        for source in [
            "class Named << (G,#red) NewKind >>",
            "class Punctuated << (!,#12ABEF) NewKind >>",
        ] {
            let entity = &parse(source).entities[0];
            assert!(entity.stereotypes[0].starts_with('('));
            assert_eq!(entity.spot_character, None);
            assert_eq!(entity.spot_color, None);
        }
    }

    #[test]
    fn entity_color_channels_and_stroke_are_independent() {
        let entity =
            &parse("class Renamed #back:azure;line:#12ABEF;line.dashed;text:navy").entities[0];
        assert_eq!(entity.color.as_deref(), Some("azure"));
        assert_eq!(entity.line_color.as_deref(), Some("12ABEF"));
        assert_eq!(entity.text_color.as_deref(), Some("navy"));
        assert_eq!(entity.line_style, Some(EntityLineStyle::Dashed));
    }

    #[test]
    fn legacy_double_hash_routes_border_separately_from_background() {
        let entity = &parse("class Renamed #azure ##[dashed]12ABEF").entities[0];
        assert_eq!(entity.color.as_deref(), Some("#azure"));
        assert_eq!(entity.line_color.as_deref(), Some("12ABEF"));
        assert_eq!(entity.line_style, Some(EntityLineStyle::Dashed));
    }

    #[test]
    fn quoted_alias_label_keeps_inline_stereotype_text() {
        let d = parse("class \"**BoundaryClass** <<boundary>>\" as C");
        let e = &d.entities[0];
        assert_eq!(e.id, "C");
        assert_eq!(e.label, "**BoundaryClass** «boundary»");
        assert!(e.explicit_alias);
        assert!(e.stereotypes.is_empty());
    }

    #[test]
    fn quoted_alias_stereotype_after_alias_is_entity_stereotype() {
        let d = parse("class \"Service\" as C <<service>>");
        let e = &d.entities[0];
        assert_eq!(e.id, "C");
        assert_eq!(e.label, "Service");
        assert!(e.explicit_alias);
        assert_eq!(e.stereotypes, vec!["service"]);
    }

    #[test]
    fn redeclared_entity_replaces_explicit_stereotypes() {
        let d = parse("class Foo <<service>>\nclass Foo <<controller>>");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].stereotypes, vec!["controller"]);
    }

    #[test]
    fn skinparam_stereotype_block_flattens() {
        let d = parse("skinparam class<<service>> {\n  FontStyle bold\n}\nclass User <<service>>");
        assert!(
            d.meta
                .skinparams
                .iter()
                .any(|sp| { sp.key == "class<<service>>FontStyle" && sp.value == "bold" })
        );
    }

    #[test]
    fn entity_gradient_color_is_captured_as_one_token() {
        let d = parse("class Foo #red|blue {\n  field: String\n}");
        assert_eq!(d.entities[0].color.as_deref(), Some("#red|blue"));
    }

    #[test]
    fn object_declaration_keeps_object_kind_in_class_diagram() {
        let d = parse("class Person\nobject alice\nPerson <|.. alice");
        let object = d.entities.iter().find(|e| e.id == "alice").unwrap();
        assert_eq!(object.kind, EntityKind::Object);
    }

    #[test]
    fn multiline_decoration_blocks_preserve_line_breaks() {
        let d = parse(
            "header\n  Company Name\n  Page %page%\nendheader\nfooter\n  Generated by RustUML\n  Date: %date%\nendfooter\nlegend right\n  First\n  Second\nendlegend\nclass Foo",
        );

        assert_eq!(d.meta.header.as_deref(), Some("Company Name\nPage %page%"));
        assert_eq!(
            d.meta.footer.as_deref(),
            Some("Generated by RustUML\nDate: %date%")
        );
        assert_eq!(d.meta.legend.as_deref(), Some("First\nSecond"));
    }

    #[test]
    fn enum_values() {
        let d = parse("enum Color {\n  RED\n  GREEN\n  BLUE\n}");
        assert_eq!(d.entities[0].kind, EntityKind::Enum);
        assert_eq!(d.entities[0].members.len(), 3);
        assert_eq!(d.entities[0].members[0].name, "RED");
    }

    #[test]
    fn enum_constructor_constants_before_separator_are_fields() {
        let d = parse(
            "enum Planet {\n  MERCURY (3.303e+23, 2.4397e6)\n  VENUS (4.869e+24, 6.0518e6)\n  --\n  +surfaceGravity(): double\n}",
        );
        let e = &d.entities[0];
        assert_eq!(e.kind, EntityKind::Enum);
        assert_eq!(e.members[0].kind, MemberKind::Field);
        assert_eq!(e.members[1].kind, MemberKind::Field);
        assert_eq!(e.members[2].kind, MemberKind::Separator);
        assert_eq!(e.members[3].kind, MemberKind::Method);
    }

    #[test]
    fn inline_member() {
        let d = parse("class User\nUser : +name : String\nUser : +login()");
        assert_eq!(d.entities[0].members.len(), 2);
    }

    #[test]
    fn generics() {
        // The trailing `<...>` is a generic type parameter, captured separately
        // and stripped from the entity id/label (it must not pollute the
        // qualified name, which is keyed on the bare name for edge lookup).
        let d = parse("class Container<T>\nclass Map<K, V>");
        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.entities[0].id, "Container");
        assert_eq!(d.entities[0].label, "Container");
        assert_eq!(d.entities[0].generic.as_deref(), Some("T"));
        assert_eq!(d.entities[1].id, "Map");
        assert_eq!(d.entities[1].generic.as_deref(), Some("K, V"));
    }

    #[test]
    fn nested_generics() {
        let d = parse("class Foo<T extends Comparable<T>>");
        assert_eq!(d.entities[0].id, "Foo");
        assert_eq!(
            d.entities[0].generic.as_deref(),
            Some("T extends Comparable<T>")
        );
    }

    #[test]
    fn inline_empty_body_after_extends_is_not_a_supertype() {
        let d = parse("class Animal\nclass Dog extends Animal {}\nclass Cat extends Animal { }");
        assert_eq!(d.entities.len(), 3);
        assert!(d.entities.iter().any(|e| e.id == "Animal"));
        assert!(d.entities.iter().any(|e| e.id == "Dog"));
        assert!(d.entities.iter().any(|e| e.id == "Cat"));
        assert!(!d.entities.iter().any(|e| e.id == "{}"));
        assert!(!d.entities.iter().any(|e| e.id == "{"));
        assert_eq!(d.relationships.len(), 2);
        assert!(d.relationships.iter().all(|rel| {
            rel.from == "Animal"
                && matches!(rel.to.as_str(), "Dog" | "Cat")
                && rel.kind == RelationshipKind::Inheritance
                && rel.decorated_end == RelationshipEnd::From
        }));
    }

    #[test]
    fn declaration_supertypes_follow_java_parent_to_child_svek_orientation() {
        let d = parse(
            "interface FreshReadable4603\n\
             interface FreshWritable4621\n\
             class FreshRecord4637 extends FreshBase4651 implements FreshReadable4603, FreshWritable4621 {\n\
               +renamedValue: String\n\
             }",
        );

        assert_eq!(d.relationships.len(), 3);
        assert_eq!(
            d.entities
                .iter()
                .find(|entity| entity.id == "FreshRecord4637")
                .unwrap()
                .source_line,
            3
        );
        assert!(
            d.relationships
                .iter()
                .all(|rel| { rel.style.declaration && rel.source_line == 4 })
        );
        assert_eq!(
            d.relationships
                .iter()
                .map(|rel| (
                    rel.from.as_str(),
                    rel.to.as_str(),
                    rel.kind,
                    rel.decorated_end
                ))
                .collect::<Vec<_>>(),
            [
                (
                    "FreshBase4651",
                    "FreshRecord4637",
                    RelationshipKind::Inheritance,
                    RelationshipEnd::From
                ),
                (
                    "FreshReadable4603",
                    "FreshRecord4637",
                    RelationshipKind::Implementation,
                    RelationshipEnd::From
                ),
                (
                    "FreshWritable4621",
                    "FreshRecord4637",
                    RelationshipKind::Implementation,
                    RelationshipEnd::From
                ),
            ]
        );
    }

    #[test]
    fn missing_declaration_supertypes_use_java_materialization_kinds() {
        let d = parse(
            "class Child extends Base implements Port\n\
             interface DerivedPort extends ParentPort\n\
             class Existing\n\
             class Other implements Existing",
        );

        let kind = |id: &str| {
            d.entities
                .iter()
                .find(|entity| entity.id == id)
                .map(|entity| entity.kind)
                .unwrap()
        };
        assert_eq!(kind("Base"), EntityKind::Class);
        assert_eq!(kind("Port"), EntityKind::Interface);
        assert_eq!(kind("ParentPort"), EntityKind::Interface);
        assert_eq!(kind("Existing"), EntityKind::Class);
        assert!(
            d.relationships
                .iter()
                .find(|rel| rel.from == "Port")
                .unwrap()
                .dashed
        );
        assert!(
            !d.relationships
                .iter()
                .find(|rel| rel.from == "ParentPort")
                .unwrap()
                .dashed
        );
    }

    #[test]
    fn declaration_supertypes_record_interleaved_cuca_uid_events() {
        let d = parse("class Child extends Base implements Port");

        assert!(matches!(
            d.uid_events.as_slice(),
            [
                ClassUidEvent::Entity(child),
                ClassUidEvent::Entity(base),
                ClassUidEvent::Relationship(0),
                ClassUidEvent::Entity(port),
                ClassUidEvent::Relationship(1)
            ] if child == "Child" && base == "Base" && port == "Port"
        ));
    }

    #[test]
    fn separators() {
        let d = parse("class Foo {\n  +field1\n  --\n  +method1()\n  ==\n  -internal\n}");
        // 3 real members + 2 separators (-- and ==) stored as MemberKind::Separator.
        assert_eq!(d.entities[0].members.len(), 5);
    }

    #[test]
    fn directed_association() {
        let d = parse("A --> B : uses");
        assert_eq!(d.relationships.len(), 1);
        // PlantUML treats --> as a dependency (solid line with arrowhead).
        assert_eq!(d.relationships[0].kind, RelationshipKind::Dependency);
        assert!(!d.relationships[0].dashed);
        assert_eq!(d.relationships[0].label.as_deref(), Some("uses"));
    }

    #[test]
    fn relationship_labels_normalize_java_guillemet_groups() {
        let d = parse(
            "A --> B : prefix << first >> and <<second>> suffix\n\
             B -left-> C : < << renamed >>",
        );
        assert_eq!(
            d.relationships[0].label.as_deref(),
            Some("prefix «first» and «second» suffix")
        );
        assert_eq!(d.relationships[1].label.as_deref(), Some("«renamed»"));
        assert_eq!(d.relationships[1].label_arrow, LinkArrow::Backward);
    }

    #[test]
    fn relationship_labels_preserve_malformed_guillemet_groups() {
        let d = parse(
            "A --> B : <<>>\n\
             B --> C : <<outer <inner> tail>>\n\
             C --> D : unmatched <<label",
        );
        assert_eq!(d.relationships[0].label.as_deref(), Some("<<>>"));
        assert_eq!(
            d.relationships[1].label.as_deref(),
            Some("<<outer <inner> tail>>")
        );
        assert_eq!(
            d.relationships[2].label.as_deref(),
            Some("unmatched <<label")
        );
    }

    #[test]
    fn long_directional_arrows_are_dependencies() {
        let d = parse("A ---> B\nA ----> C");
        assert_eq!(d.relationships.len(), 2);
        assert_eq!(d.relationships[0].kind, RelationshipKind::Dependency);
        assert_eq!(d.relationships[1].kind, RelationshipKind::Dependency);
        assert!(!d.relationships[0].dashed);
        assert!(!d.relationships[1].dashed);
    }

    #[test]
    fn command_link_directions_invert_left_and_up_with_endpoint_metadata() {
        let d = parse(
            "class FreshCompassOrigin3251\n\
             class FreshCompassWest3253\n\
             class FreshCompassNorth3257\n\
             class FreshCompassEast3259\n\
             class FreshCompassSouth3271\n\
             FreshCompassOrigin3251 \"origin-west\" -left-> \"west-origin\" FreshCompassWest3253\n\
             FreshCompassOrigin3251 -u-> FreshCompassNorth3257\n\
             FreshCompassOrigin3251 -right-> FreshCompassEast3259\n\
             FreshCompassOrigin3251 -d-> FreshCompassSouth3271",
        );

        assert_eq!(d.relationships.len(), 4);
        let left = &d.relationships[0];
        assert_eq!(left.from, "FreshCompassWest3253");
        assert_eq!(left.to, "FreshCompassOrigin3251");
        assert_eq!(left.from_multiplicity.as_deref(), Some("west-origin"));
        assert_eq!(left.to_multiplicity.as_deref(), Some("origin-west"));
        assert_eq!(left.decorated_end, RelationshipEnd::From);
        assert_eq!(left.length, 1);
        assert!(left.style.inverted);

        let up = &d.relationships[1];
        assert_eq!(up.from, "FreshCompassNorth3257");
        assert_eq!(up.to, "FreshCompassOrigin3251");
        assert_eq!(up.decorated_end, RelationshipEnd::From);
        assert_eq!(up.length, 2);
        assert!(up.style.inverted);

        let right = &d.relationships[2];
        assert_eq!(right.from, "FreshCompassOrigin3251");
        assert_eq!(right.to, "FreshCompassEast3259");
        assert_eq!(right.decorated_end, RelationshipEnd::To);
        assert_eq!(right.length, 1);
        assert!(!right.style.inverted);

        let down = &d.relationships[3];
        assert_eq!(down.from, "FreshCompassOrigin3251");
        assert_eq!(down.to, "FreshCompassSouth3271");
        assert_eq!(down.decorated_end, RelationshipEnd::To);
        assert_eq!(down.length, 2);
        assert!(!down.style.inverted);
    }

    #[test]
    fn dotted_thick_dependency() {
        let d = parse("A ..>> B : dotted thick");
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].kind, RelationshipKind::Dependency);
        assert!(d.relationships[0].dashed);
        assert_eq!(d.relationships[0].label.as_deref(), Some("dotted thick"));
    }

    #[test]
    fn class_dependency_arrow_variants() {
        let d = parse("A <-> B\nA ->> B\nA -->> B\nA <-->> B\nA ...> B");
        assert_eq!(d.relationships.len(), 5);
        assert!(
            d.relationships
                .iter()
                .all(|rel| rel.kind == RelationshipKind::Dependency)
        );
        assert!(!d.relationships[0].dashed);
        assert!(!d.relationships[1].dashed);
        assert!(!d.relationships[2].dashed);
        assert!(!d.relationships[3].dashed);
        assert!(d.relationships[4].dashed);
    }

    #[test]
    fn bidirectional_dashed_dependency_decorates_both_renamed_endpoints() {
        let d = parse(
            "@startuml\nclass RenamedSource\nclass RenamedTarget\nRenamedSource <..> RenamedTarget\n@enduml",
        );
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].kind, RelationshipKind::Dependency);
        assert!(d.relationships[0].dashed);
        assert_eq!(d.relationships[0].decorated_end, RelationshipEnd::Both);
    }

    #[test]
    fn relationship_multiplicity() {
        let d = parse(r#"Company "1" o-- "1..*" Department"#);
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from_multiplicity.as_deref(), Some("1"));
        assert_eq!(d.relationships[0].to_multiplicity.as_deref(), Some("1..*"));
    }

    #[test]
    fn relationship_label_arrows_are_separate_from_label_text() {
        let d = parse(
            "A -- B : < renamed backward\n\
             B -- C : renamed direct >\n\
             C -- D : >\n\
             D -- E : \"< quoted label\"",
        );
        assert_eq!(d.relationships.len(), 4);
        assert_eq!(
            d.relationships[0].label.as_deref(),
            Some("renamed backward")
        );
        assert_eq!(d.relationships[0].label_arrow, LinkArrow::Backward);
        assert_eq!(d.relationships[1].label.as_deref(), Some("renamed direct"));
        assert_eq!(d.relationships[1].label_arrow, LinkArrow::Direct);
        assert_eq!(d.relationships[2].label, None);
        assert_eq!(d.relationships[2].label_arrow, LinkArrow::Direct);
        assert_eq!(d.relationships[3].label.as_deref(), Some("quoted label"));
        assert_eq!(d.relationships[3].label_arrow, LinkArrow::Backward);
    }

    #[test]
    fn er_crowfoot_endpoint_decorations_are_preserved() {
        let d = parse(
            "AlphaZero ||--o| BetaZero : maybe\n\
             GammaMany }|--|{ DeltaMany\n\
             EchoOptional }o--|| FoxtrotOne",
        );
        assert_eq!(d.relationships.len(), 3);
        assert_eq!(
            d.relationships[0].from_decor,
            Some(EndpointDecor::DoubleLine)
        );
        assert_eq!(d.relationships[0].to_decor, Some(EndpointDecor::CircleLine));
        assert_eq!(
            d.relationships[1].from_decor,
            Some(EndpointDecor::LineCrowFoot)
        );
        assert_eq!(
            d.relationships[1].to_decor,
            Some(EndpointDecor::LineCrowFoot)
        );
        assert_eq!(
            d.relationships[2].from_decor,
            Some(EndpointDecor::CircleCrowFoot)
        );
        assert_eq!(d.relationships[2].to_decor, Some(EndpointDecor::DoubleLine));
    }

    #[test]
    fn parenthesis_endpoint_decorations_use_ordinary_relationships() {
        let d = parse(
            "package Outer {\n\
               interface ITarget\n\
             }\n\
             Outer -( ITarget\n\
             \"source node\" )-- Outer.ITarget\n\
             Alpha )-( Omega",
        );
        assert_eq!(d.relationships.len(), 3);
        assert_eq!(d.relationships[0].from, "Outer");
        assert_eq!(d.relationships[0].to, "Outer.ITarget");
        assert_eq!(
            d.relationships[0].to_decor,
            Some(EndpointDecor::Parenthesis)
        );
        assert_eq!(
            d.relationships[1].from_decor,
            Some(EndpointDecor::Parenthesis)
        );
        assert_eq!(
            d.relationships[2].from_decor,
            Some(EndpointDecor::Parenthesis)
        );
        assert_eq!(
            d.relationships[2].to_decor,
            Some(EndpointDecor::Parenthesis)
        );
    }

    #[test]
    fn nested_package() {
        let d = parse(
            "cloud Outer {\n  cloud Inner {\n    class MyClass {\n      +void method()\n    }\n  }\n}",
        );
        assert_eq!(d.packages.len(), 2);
        assert_eq!(d.packages[0].name, "Outer");
        assert_eq!(d.packages[1].name, "Outer.Inner");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].id, "Outer.Inner.MyClass");
    }

    #[test]
    fn class_with_url() {
        let d = parse("class MyClass [[https://example.com]] {\n  + method()\n}");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].url.as_deref(), Some("https://example.com"));
        assert_eq!(d.entities[0].id, "MyClass");
    }

    #[test]
    fn class_url_with_tooltip() {
        let d = parse("class Svc [[https://docs.example.com{API Documentation}]]");
        assert_eq!(
            d.entities[0].url.as_deref(),
            Some("https://docs.example.com")
        );
    }

    #[test]
    fn class_no_url() {
        let d = parse("class Plain");
        assert_eq!(d.entities[0].url, None);
    }

    #[test]
    fn multiline_note_tracks_first_content_line_and_color() {
        let d = parse(
            "class Renamed\n\
             note left of Renamed #aliceblue\n\
             First content line\n\
             Second content line\n\
             end note",
        );

        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].source_line, 3);
        assert_eq!(d.notes[0].color.as_deref(), Some("#aliceblue"));
        assert_eq!(
            d.notes[0].lines,
            ["First content line", "Second content line"]
        );
    }

    #[test]
    fn nested_attached_note_keeps_resolved_target_and_constructor_owner() {
        let d = parse(
            "package FreshScope4121 {\n\
               class FreshNested4127\n\
               note left of FreshNested4127 : scoped note\n\
             }",
        );

        assert_eq!(
            d.notes[0].target.as_deref(),
            Some("FreshScope4121.FreshNested4127")
        );
        assert!(d.uid_events.iter().any(|event| matches!(
            event,
            ClassUidEvent::Note {
                index: 0,
                owner_package: Some(owner),
            } if owner == "FreshScope4121"
        )));
    }

    #[test]
    fn multiline_note_dedents_common_margin_but_preserves_code_structure() {
        let d = parse(
            "note as FreshCode759\n  <code>\n  fn audit() {\n      record();\n\n  }\n  </code>\nend note",
        );

        assert_eq!(
            d.notes[0].lines,
            [
                "<code>",
                "fn audit() {",
                "    record();",
                "",
                "}",
                "</code>"
            ]
        );
    }

    #[test]
    fn multiline_note_preserves_terminal_spaces_after_common_dedent() {
        let d = parse("note as FreshWhitespaceLedger4171\n  renamed audit value:   \nend note");

        assert_eq!(d.notes[0].lines, ["renamed audit value:   "]);
    }

    #[test]
    fn named_note_relationships_survive_without_phantom_classes() {
        let d = parse(
            "class FreshArchive4211\n\
             note \"renamed archive memo\" as FreshMemo4217\n\
             note \"renamed peer memo\" as FreshPeer4219\n\
             FreshArchive4211 .. FreshMemo4217\n\
             FreshMemo4217 .. FreshPeer4219",
        );

        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].id, "FreshArchive4211");
        assert_eq!(d.relationships.len(), 2);
        assert_eq!(d.relationships[0].from, "FreshArchive4211");
        assert_eq!(d.relationships[0].to, "FreshMemo4217");
        assert_eq!(d.relationships[1].from, "FreshMemo4217");
        assert_eq!(d.relationships[1].to, "FreshPeer4219");
    }

    #[test]
    fn package_named_note_relationship_reuses_the_note_quark() {
        let d = parse(
            "class FreshRoot5411\n\
             package FreshArchive5413 {\n\
               class FreshLeaf5417\n\
             }\n\
             package FreshArchive5413 {\n\
               note \"renamed package memo\" as FreshMemo5419\n\
               FreshMemo5419 .. FreshArchive5413.FreshLeaf5417\n\
             }",
        );

        assert_eq!(d.entities.len(), 2);
        assert_eq!(
            d.notes[0].id.as_deref(),
            Some("FreshArchive5413.FreshMemo5419")
        );
        assert_eq!(d.relationships[0].from, "FreshArchive5413.FreshMemo5419");
        assert_eq!(d.relationships[0].to, "FreshArchive5413.FreshLeaf5417");
        assert!(d.uid_events.iter().all(|event| !matches!(
            event,
            ClassUidEvent::Entity(id) if id == "FreshArchive5413.FreshMemo5419"
        )));
    }

    #[test]
    fn duplicate_local_note_aliases_resolve_in_their_current_packages() {
        let d = parse(
            "package FreshWest5511 {\n\
               class FreshWestLeaf5513\n\
               note \"west memo\" as FreshMemo5517\n\
               FreshMemo5517 .. FreshWestLeaf5513\n\
             }\n\
             package FreshEast5521 {\n\
               class FreshEastLeaf5523\n\
               note \"east memo\" as FreshMemo5517\n\
               FreshMemo5517 .. FreshEastLeaf5523\n\
             }",
        );

        assert_eq!(
            d.notes
                .iter()
                .filter_map(|note| note.id.as_deref())
                .collect::<Vec<_>>(),
            ["FreshWest5511.FreshMemo5517", "FreshEast5521.FreshMemo5517"]
        );
        assert_eq!(d.relationships[0].from, "FreshWest5511.FreshMemo5517");
        assert_eq!(d.relationships[1].from, "FreshEast5521.FreshMemo5517");
        assert_eq!(d.entities.len(), 2);
    }

    #[test]
    fn association_replaces_latest_base_but_preserves_its_uid_event() {
        let d = parse(
            "class Existing\n\
             MissingA -- Existing : retained label\n\
             (MissingA, Existing) .. MissingAssoc\n\
             MissingB --> Existing",
        );

        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from, "MissingB");
        assert_eq!(d.association_classes.len(), 1);
        let base = d.association_classes[0]
            .replaced_relationship
            .as_ref()
            .unwrap();
        assert_eq!(base.from, "MissingA");
        assert_eq!(base.to, "Existing");
        assert_eq!(base.label.as_deref(), Some("retained label"));
        assert!(d.uid_events.iter().any(|event| matches!(
            event,
            ClassUidEvent::DiscardedRelationship { inverted: false }
        )));
        assert!(
            d.uid_events
                .iter()
                .any(|event| matches!(event, ClassUidEvent::Association(0)))
        );
    }
}
