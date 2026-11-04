// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram parser.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::class::*;
use crate::diagram::{DiagramMeta, LegendHorizontalAlignment, LegendVerticalAlignment};

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

/// Parse preprocessed lines with the same consuming command engine used for
/// factory viability. Java PSystemCommandFactory executes complete commands;
/// body lines never re-enter the outer command dispatcher.
pub fn parse_class(lines: &[String]) -> Result<ClassDiagram, ParseError> {
    Ok(parse_commands(lines, false)?.finish())
}

pub(super) fn native_container_candidate(lines: &[String]) -> bool {
    parse_commands(lines, false).is_ok_and(|parser| {
        // PSystemBuilder tries Sequence before Class. CommandParticipant owns
        // bare `entity` too, but cannot own class-only declarations/containers.
        parser.has_class_only_declaration
            || (!parser.packages.is_empty()
                && (parser.has_native_class || parser.entities.is_empty()))
    })
}

/// PSystemBuilder tries DescriptionDiagramFactory after class execution fails.
/// A full consuming trial is required; native-only declarations and class-only
/// directives cannot be rescued by a description family score.
pub(super) fn description_candidate(lines: &[String]) -> Option<bool> {
    let parser = parse_commands(lines, true).ok()?;
    if parser.entities.is_empty() {
        return None;
    }
    Some(
        parser.packages.is_empty()
            && parser
                .entities
                .iter()
                .all(|entity| matches!(entity.kind, EntityKind::Actor | EntityKind::UseCase)),
    )
}

fn parse_commands(lines: &[String], description_factory: bool) -> Result<ClassParser, ParseError> {
    let mut parser = ClassParser::new();
    parser.allow_mixing = description_factory;
    parser.description_factory = description_factory;
    for (i, line) in lines.iter().enumerate() {
        let (source_line, trimmed) = super::source_line_and_trimmed(i + 1, line);
        if parser.current_note.is_some() {
            // CommandFactoryNote owns indentation and terminal spaces.
            parser.parse_line(source_line, super::source_text(line))?;
        } else if !trimmed.is_empty() || parser.meta_block.is_some() {
            parser.parse_line(source_line, trimmed)?;
        }
    }
    // Java multiline commands cannot succeed before their closing delimiter.
    if parser.current_entity.is_some()
        || parser.current_note.is_some()
        || parser.meta_block.is_some()
        || parser.current_skinparam_prefix.is_some()
        || !parser.scope_stack.is_empty()
    {
        return Err(ParseError {
            line: parser.current_line,
            message: "unclosed class command".into(),
        });
    }
    Ok(parser)
}

/// NameAndCodeParser.nameAndCodeForClassWithGeneric: quoted display text is
/// opaque; `as` is syntax only between complete tokens. The returned tail is
/// owned by the declaration command, not rescanned as another command.
struct NameAndCode<'a> {
    code: String,
    display: String,
    explicit_alias: bool,
    quoted_only: bool,
    alias_display_quoted: bool,
    tail: &'a str,
}

fn name_token(input: &str) -> Option<(&str, bool, &str)> {
    if let Some(rest) = input.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some((&rest[..end], true, &rest[end + 1..]));
    }
    let end = input
        .find(|c: char| c.is_whitespace() || matches!(c, '{' | '}' | '"' | '<' | '>'))
        .unwrap_or(input.len());
    (end > 0).then_some((&input[..end], false, &input[end..]))
}

fn name_and_code(input: &str) -> Option<NameAndCode<'_>> {
    let (first, quoted, tail) = name_token(input)?;
    let rest = tail.trim_start();
    if let Some((word, after_as)) = rest.split_once(char::is_whitespace)
        && word.eq_ignore_ascii_case("as")
    {
        let (second, second_quoted, tail) = name_token(after_as.trim_start())?;
        // Class name grammar has quoted-display/code and code/quoted-display
        // alternatives. CommandPackageWithUSymbol also permits display/code.
        let (code, display) = if second_quoted {
            (first, second)
        } else {
            (second, first)
        };
        return Some(NameAndCode {
            code: code.into(),
            display: display.into(),
            explicit_alias: true,
            quoted_only: false,
            alias_display_quoted: quoted || second_quoted,
            tail,
        });
    }
    Some(NameAndCode {
        code: first.into(),
        display: first.into(),
        explicit_alias: false,
        quoted_only: quoted,
        alias_display_quoted: false,
        tail,
    })
}

struct ClassParser {
    meta: DiagramMeta,
    allow_mixing: bool,
    description_factory: bool,
    has_native_class: bool,
    has_class_only_declaration: bool,
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
    /// Canonical quark paths in the order PlantUML's Plasma tree creates them.
    quark_creation_order: Vec<Vec<String>>,
    /// Note currently being accumulated (multi-line `note ... end note`).
    current_note: Option<Note>,
    /// CommandFactoryNoteOnEntity selects a brace or end-note terminator.
    note_with_bracket: bool,
    /// ID of the last declared entity (for shorthand `note right : text`).
    last_entity_id: Option<String>,
    /// Namespace separator string (default "."; `None` when `set namespaceSeparator none`).
    namespace_sep: Option<String>,
    /// Whether we are inside a multi-line header/footer/legend block.
    meta_block: Option<MetaBlock>,
    meta_block_lines: usize,
    /// Prefix of an open `skinparam X { ... }` block.
    current_skinparam_prefix: Option<String>,
    /// Current 1-based source line number (set before each parse_line call).
    current_line: usize,
    /// Accumulated `hide` / `show` directives, in source order.
    hide_show: Vec<crate::diagram::class::HideShow>,
    header_line: Option<usize>,
    header_alignment: Option<LegendHorizontalAlignment>,
    footer_alignment: Option<LegendHorizontalAlignment>,
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
            allow_mixing: false,
            description_factory: false,
            has_native_class: false,
            has_class_only_declaration: false,
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
            quark_creation_order: Vec::new(),
            current_note: None,
            note_with_bracket: false,
            last_entity_id: None,
            namespace_sep: Some(".".to_string()),
            meta_block: None,
            meta_block_lines: 0,
            current_skinparam_prefix: None,
            current_line: 0,
            hide_show: Vec::new(),
            header_line: None,
            header_alignment: None,
            footer_alignment: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        }
    }

    fn finish(mut self) -> ClassDiagram {
        // CucaDiagram.getTextBlock materializes any remaining parent quarks
        // with null source location after command execution has finished.
        self.current_line = 0;
        self.materialize_phantom_packages();
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
            header_alignment: self.header_alignment,
            footer_alignment: self.footer_alignment,
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

    fn push_note(&mut self, mut note: Note) -> Result<(), ParseError> {
        // CommandFactoryNote.executeInternal creates a NOTE in its resolved
        // quark parent, which need not be the current declaration scope.
        note.owner_package = if let Some(alias) = &note.alias {
            let path = self.resolve_quark_path(alias, QuarkLookup::CurrentContext)?;
            let id = self.path_id(&path);
            if self.entity_by_path.contains_key(&path)
                || self.package_by_path.contains_key(&path)
                || self
                    .notes
                    .iter()
                    .any(|note| note.alias.as_deref() == Some(&id))
            {
                return Err(ParseError {
                    line: self.current_line,
                    message: format!("note already created: {id}"),
                });
            }
            // reallyCreateLeaf allocates the NOTE before phantom ancestors.
            self.uid_events.push(ClassUidEvent::Note(self.notes.len()));
            self.ensure_phantom_packages(&path, false);
            note.alias = Some(id);
            self.register_quark_path(&path);
            self.package_by_path
                .get(&path[..path.len() - 1])
                .map(|&idx| self.packages[idx].name.clone())
        } else {
            // Attached GMN notes are new quarks in the current group.
            self.package_stack
                .last()
                .map(|&idx| self.packages[idx].name.clone())
        };
        if note.alias.is_none() {
            self.uid_events.push(ClassUidEvent::Note(self.notes.len()));
        }
        self.notes.push(note);
        Ok(())
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

    fn split_identity(&self, raw: &str) -> Result<(bool, Vec<String>), ParseError> {
        let raw = raw.trim();
        let Some(separator) = self.namespace_sep.as_deref() else {
            return Ok((false, vec![raw.to_string()]));
        };
        // CucaDiagram#badName validates before quark allocation: empty
        // components are errors, not aliases for a collapsed path.
        if raw.ends_with(separator) || raw.contains(&separator.repeat(2)) {
            return Err(ParseError {
                line: self.current_line,
                message: format!("bad name since {separator} is a separator: {raw}"),
            });
        }
        let rooted = raw.starts_with(separator);
        let raw = raw.strip_prefix(separator).unwrap_or(raw);
        Ok((rooted, raw.split(separator).map(str::to_string).collect()))
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
        if !self
            .quark_creation_order
            .iter()
            .any(|existing| existing == path)
        {
            self.quark_creation_order.push(path.to_vec());
        }
    }

    /// Mirrors `CucaDiagram#quarkInContextSafe`: qualified names resolve from
    /// the root when their first group exists, otherwise from the current
    /// group; a reusable unqualified name binds to its sole existing quark.
    fn resolve_quark_path(
        &self,
        raw: &str,
        lookup: QuarkLookup,
    ) -> Result<Vec<String>, ParseError> {
        let raw = raw.trim();
        if self.namespace_sep.is_none() {
            // CucaDiagram#quarkInContextSafe delegates both lookup modes to
            // creation-ordered Plasma#firstWithName when the separator is null.
            if let Some(path) = self.first_quark_path_named(raw) {
                return Ok(path);
            }
            let mut path = self.current_group_path().to_vec();
            path.push(raw.to_string());
            return Ok(path);
        }

        let (rooted, parts) = self.split_identity(raw)?;
        if rooted {
            return Ok(parts);
        }
        if parts.len() == 1 {
            // CucaDiagram#quarkInContextSafe excludes the current group from
            // unique reuse. Current-context declarations never short-circuit
            // to an existing root leaf with the same short name.
            if matches!(lookup, QuarkLookup::ReuseUnique)
                && let Some(path) = self.unique_quark_path_named(&parts[0])
                && path != self.current_group_path()
            {
                return Ok(path);
            }
        } else {
            let first = vec![parts[0].clone()];
            if self.quark_creation_order.contains(&first) {
                // The Java root child may exist without data; only existing
                // non-group data rejects a qualified path at this stage.
                let first_id = self.path_id(&first);
                if self.entity_by_path.contains_key(&first)
                    || self
                        .notes
                        .iter()
                        .any(|note| note.alias.as_deref() == Some(&first_id))
                {
                    return Err(ParseError {
                        line: self.current_line,
                        message: format!("not a package: {}", parts[0]),
                    });
                }
                return Ok(parts);
            }
        }
        let mut path = self.current_group_path().to_vec();
        path.extend(parts);
        Ok(path)
    }

    fn resolve_group_path(&self, raw: &str) -> Result<Vec<String>, ParseError> {
        // CommandPackage uses quarkInContext(false), the same checked resolver
        // as native declarations, including null-separator firstWithName.
        self.resolve_quark_path(raw, QuarkLookup::CurrentContext)
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

    /// `CucaDiagram#eventuallyBuildPhantomGroups` materializes every empty
    /// parent quark when a class-like leaf is first created below it.
    fn ensure_phantom_packages(&mut self, entity_path: &[String], materialize: bool) {
        for depth in 1..entity_path.len() {
            let path = entity_path[..depth].to_vec();
            if self.package_by_path.contains_key(&path) {
                continue;
            }
            let parent =
                (depth > 1).then(|| self.package_by_path[&entity_path[..depth - 1].to_vec()]);
            let idx = self.packages.len();
            let name = self.path_id(&path);
            self.packages.push(Package {
                visibility: Visibility::Default,
                name,
                tags: Vec::new(),
                url: None,
                url_tooltip: None,
                kind: PackageKind::Package,
                color: None,
                entities: Vec::new(),
                parent,
                source_line: 0,
                stereotypes: Vec::new(),
                display_name: path.last().cloned(),
                phantom: true,
            });
            self.package_paths.push(path.clone());
            self.package_by_path.insert(path.clone(), idx);
            self.register_quark_path(&path);
        }
        if materialize {
            self.materialize_phantom_packages();
        }
    }

    fn materialize_phantom_packages(&mut self) {
        // reallyCreateLeaf calls this only for LeafType.isLikeClass. It visits
        // all missing ancestor data, including quarks introduced by notes.
        for package in &mut self.packages {
            if package.phantom && package.source_line == 0 {
                self.uid_events
                    .push(ClassUidEvent::Package(package.name.clone()));
                package.source_line = self.current_line;
            }
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
        // `reallyCreateLeaf` consumes the entity UID before
        // `eventuallyBuildPhantomGroups` materializes missing parent quarks.
        self.uid_events.push(ClassUidEvent::Entity(id.clone()));
        // LeafType.LIKE_CLASS excludes OBJECT and DESCRIPTION/NOTE leaves.
        let class_like = matches!(
            kind,
            EntityKind::Annotation
                | EntityKind::AbstractClass
                | EntityKind::Class
                | EntityKind::Interface
                | EntityKind::Enum
                | EntityKind::Entity
                | EntityKind::Protocol
                | EntityKind::Struct
                | EntityKind::Exception
                | EntityKind::Metaclass
                | EntityKind::Stereotype
                | EntityKind::Dataclass
                | EntityKind::Record
        );
        self.ensure_phantom_packages(&path, class_like);
        let idx = self.entities.len();
        self.entities.push(ClassEntity {
            id: id.clone(),
            label,
            explicit_alias,
            kind,
            is_static: false,
            visibility: Visibility::Default,
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
        self.register_quark_path(&path);
        self.register_entity_path(&path, &id);
        self.enroll_new_entity_in_current_together(&id);
        id
    }

    fn ensure_entity(&mut self, raw: &str) -> Result<String, ParseError> {
        let path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique)?;
        if let Some(&idx) = self.entity_by_path.get(&path) {
            return Ok(self.entities[idx].id.clone());
        }
        let label = path.last().cloned().unwrap_or_default();
        Ok(self.create_entity_at_path(path, label, EntityKind::Class, false))
    }

    fn ensure_entity_kind(&mut self, raw: &str, kind: EntityKind) -> Result<String, ParseError> {
        let path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique)?;
        if let Some(&idx) = self.entity_by_path.get(&path) {
            return Ok(self.entities[idx].id.clone());
        }
        let label = path.last().cloned().unwrap_or_default();
        Ok(self.create_entity_at_path(path, label, kind, false))
    }

    /// Port of `CommandLinkLollipop`: the visible interface label belongs to a
    /// synthetic leaf beside the existing class, not to a normal interface
    /// entity. The internal suffix only needs to be unique; the renderer
    /// derives Java's public `<normal>lol<uid>` name from the shared UID stream.
    fn create_lollipop_entity(
        &mut self,
        normal_id: &str,
        label: String,
        kind: EntityKind,
    ) -> Option<String> {
        let mut path = self
            .entity_by_path
            .iter()
            .find_map(|(path, &idx)| (self.entities[idx].id == normal_id).then(|| path.clone()))?;
        let ordinal = self
            .entities
            .iter()
            .filter(|entity| {
                matches!(
                    entity.kind,
                    EntityKind::LollipopFull | EntityKind::LollipopHalf
                )
            })
            .count();
        let leaf = path.last_mut()?;
        leaf.push_str(&format!("__rustuml_lollipop_{ordinal}"));
        Some(self.create_entity_at_path(path, label, kind, false))
    }

    fn resolve_relationship_endpoint(&mut self, raw: &str) -> Result<String, ParseError> {
        // `CommandLinkClass#executeArg` calls
        // `CucaDiagram#quarkInContextSafe(true, endpoint)` and first creates a
        // missing endpoint as `LeafType.CLASS` at the relationship location.
        let path = self.resolve_quark_path(raw, QuarkLookup::ReuseUnique)?;
        if let Some(&index) = self.package_by_path.get(&path) {
            return Ok(self.packages[index].name.clone());
        }
        let id = self.path_id(&path);
        if self
            .notes
            .iter()
            .any(|note| note.alias.as_deref() == Some(&id))
        {
            return Ok(id);
        }
        self.ensure_entity(raw)
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
                MetaBlock::Title => "endtitle",
            };
            let end2 = match block {
                MetaBlock::Header => "end header",
                MetaBlock::Footer => "end footer",
                MetaBlock::Legend => "end legend",
                MetaBlock::Caption => "end caption",
                MetaBlock::Title => "end title",
            };
            if line.eq_ignore_ascii_case(end1) || line.eq_ignore_ascii_case(end2) {
                self.meta_block = None;
            } else {
                match block {
                    MetaBlock::Header => {
                        let h = self.meta.header.get_or_insert_with(String::new);
                        if self.meta_block_lines > 0 {
                            h.push('\n');
                        }
                        h.push_str(line);
                    }
                    MetaBlock::Footer => {
                        let f = self.meta.footer.get_or_insert_with(String::new);
                        if self.meta_block_lines > 0 {
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
            self.meta_block_lines += 1;
            return Ok(());
        }

        // Inside a multi-line note?
        if self.current_note.is_some() {
            if if self.note_with_bracket {
                line.trim() == "}"
            } else {
                line.trim().eq_ignore_ascii_case("end note")
                    || line.trim().eq_ignore_ascii_case("endnote")
            } {
                self.note_with_bracket = false;
                let mut note = self.current_note.take().unwrap();
                if note.lines.is_empty() {
                    // A body line that is itself an inline diagram directive
                    // is consumed by preprocessing. Java still attributes the
                    // empty note entity to that body location, immediately
                    // after the declaration.
                    note.source_line = note.source_line.saturating_add(1);
                }
                dedent_note_lines(&mut note.lines);
                self.push_note(note)?;
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

        // The public preprocessed entrypoint also accepts an outer wrapper.
        if line.starts_with("@startuml") || line == "@enduml" {
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
        // ClassDiagramFactory registers shared containers before mixed leaves.
        if self.try_package(line)? {
            return Ok(());
        }
        if self.try_entity_decl(line)? {
            return Ok(());
        }
        if self.try_association_class(line)? {
            return Ok(());
        }
        if self.try_relationship(line)? {
            return Ok(());
        }
        if self.try_inline_member(line)? {
            return Ok(());
        }
        if self.try_enum_decl(line)? {
            return Ok(());
        }
        if self.try_note(line)? {
            return Ok(());
        }
        if self.try_url(line)? {
            return Ok(());
        }
        if self.try_meta(line) {
            return Ok(());
        }

        // PSystemCommandFactory rejects unrecognized commands. In particular,
        // an unsuccessful trial cannot claim viability by dropping a line.
        Err(ParseError {
            line: line_num,
            message: format!("unrecognized class command: {line}"),
        })
    }

    fn try_entity_decl(&mut self, line: &str) -> Result<bool, ParseError> {
        // CommandCreateClass[Multilines].getRegexConcat and
        // CommandCreateElementFull2: Pattern2 is case-insensitive; whitespace
        // separates keywords, while labels/codes retain their original case.
        static DECL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r"(?i)^([+~#-]?)\s*(?:(mix_)?(class|abstract\s+class|static\s+class|abstract|interface|enum|annotation|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record|object|state|circle|diamond|actor|usecase|component|boundary|database|queue|node|rectangle))\s+"
        ).unwrap()
        });
        static SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r#"(?i)^\s*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>\s*)?(?:(?:<<[^>]+>>|\$[^\s{]+|#[^\s{]+)\s*)*(?:(?:extends|implements)\s+[^{}]+)?\s*(?:\{\s*\}|\{\{?)?\s*$"#
        ).unwrap()
        });
        static STEREOTYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)>>").unwrap());
        let Some(caps) = DECL.captures(line) else {
            return Ok(false);
        };
        let declaration_key = caps[3]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        let declaration_kind = declaration_key.as_str();
        let native_grammar = matches!(
            declaration_kind,
            "class"
                | "abstract class"
                | "static class"
                | "abstract"
                | "interface"
                | "enum"
                | "annotation"
                | "entity"
                | "protocol"
                | "struct"
                | "exception"
                | "metaclass"
                | "stereotype"
                | "dataclass"
                | "record"
                | "circle"
                | "diamond"
        );
        // DescriptionDiagramFactory shares interface/entity/circle spelling,
        // but not native class semantics. CommandCreateElementFull chooses its
        // own leaf type; other native declarations are not description commands.
        let shared_description = matches!(declaration_kind, "interface" | "entity" | "circle");
        if self.description_factory && native_grammar && !shared_description {
            return Ok(false);
        }
        let ordinary_class_command = native_grammar && !self.description_factory;
        let mixed = !ordinary_class_command && declaration_kind != "object";
        let Some(name) = name_and_code(&line[caps.get(0).unwrap().end()..]) else {
            return Ok(false);
        };
        // CommandCreateClass parses URLs/colors/supertypes only after the
        // complete name/code alternative. Brackets, # and keywords inside a
        // quoted display cannot become declaration decorations.
        let (url, clean_tail) = super::extract_link_url(name.tail);
        let url_tooltip = super::extract_link_tooltip(name.tail);
        if !SUFFIX.is_match(&clean_tail) {
            return Ok(false);
        }
        // NameAndCodeParser's aliased native class alternatives require a
        // quoted display. Symbol containers separately permit unquoted display.
        if ordinary_class_command && name.explicit_alias && !name.alias_display_quoted {
            return Ok(false);
        }
        // CommandCreateElementFull2.NORMAL_KEYWORD checks the current state,
        // not whether an allowmixing directive exists later in the source.
        if mixed && caps.get(2).is_none() && !self.allow_mixing {
            return Err(ParseError {
                line: self.current_line,
                message: "Use 'allowmixing' before mixing classes and other UML elements".into(),
            });
        }
        if caps.get(2).is_some() && !mixed {
            return Ok(false);
        }
        if ordinary_class_command {
            self.has_native_class = true;
            // CommandParticipantA/A2/A3/A4 share the entity declaration's
            // name/code, optional stereotype, URL (already extracted), color.
            // Visibility, generics, supertypes and braces are Class commands.
            // Only the parsed suffix participates: quoted display is opaque.
            static PARTICIPANT_SUFFIX: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^\s*(?:<<[^>]+>>\s*)?(?:#[^\s{}]+\s*)?$").unwrap());
            self.has_class_only_declaration |= declaration_kind != "entity"
                || !caps[1].is_empty()
                || !PARTICIPANT_SUFFIX.is_match(&clean_tail);
        }
        let kind = parse_entity_kind(declaration_kind);
        let quoted_only = name.quoted_only;
        let explicit_alias = name.explicit_alias;
        // CucaDiagram.cleanId removes only enclosing quotes, which the
        // NameAndCode recognizer already consumed. Preserve the code itself.
        let mut id = name.code;
        let mut label = name.display;
        {
            label = normalize_inline_stereotypes(&label);

            // A trailing `<...>` is a generic type parameter, not part of the
            // entity id/label/qualified-name. Split it off the id; mirror the
            // split onto the label only when the label was derived from the id
            // (no explicit alias / quoted name).
            let label_was_id = !explicit_alias && !quoted_only;
            // NameAndCodeParser permits generics in quoted display or after code.
            let tail = clean_tail.trim_start();
            let generic = if tail.starts_with('<') && !tail.starts_with("<<") {
                let mut depth = 0;
                let end = tail.char_indices().find_map(|(i, ch)| {
                    match ch {
                        '<' => depth += 1,
                        '>' => depth -= 1,
                        _ => {}
                    }
                    (depth == 0).then_some(i)
                });
                end.map(|end| tail[1..end].trim().to_string())
            } else {
                split_generic(&mut id).or_else(|| split_generic(&mut label))
            };
            if label_was_id {
                split_generic(&mut label);
            }

            let stereotype_source = text_outside_double_quotes(&clean_tail);
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

            let entity_colors = parse_entity_colors(&clean_tail);

            // Native commands use current-context quarks; description commands
            // use CommandCreateElementFull2's unique-reuse lookup.
            let lookup = if ordinary_class_command {
                QuarkLookup::CurrentContext
            } else {
                QuarkLookup::ReuseUnique
            };
            let entity_path = self.resolve_quark_path(&id, lookup)?;
            let display_label = if explicit_alias {
                label
            } else if quoted_only {
                match self.namespace_sep.as_deref() {
                    Some(separator) if label.contains(separator) => {
                        label.rsplit(separator).next().unwrap().to_string()
                    }
                    _ => label,
                }
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
                if ordinary_class_command || entity_was_created {
                    entity.kind = kind;
                }
                entity.is_static = declaration_kind == "static class";
                entity.visibility = match &caps[1] {
                    "+" => Visibility::Public,
                    "-" => Visibility::Private,
                    "#" => Visibility::Protected,
                    "~" => Visibility::Package,
                    _ => Visibility::Default,
                };
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
            self.parse_supertypes(&clean_tail, &final_id)?;

            if line.ends_with('{') || line.ends_with("{{") {
                self.current_entity = Some(final_id.clone());
                self.current_entity_needs_body_location = true;
            }
            self.last_entity_id = Some(final_id);
            Ok(true)
        }
    }

    /// Parse `extends`/`implements` clauses on a class-declaration line into
    /// Inheritance/Implementation relationships from the declared entity to
    /// each named supertype. Generic (`<...>`) and stereotype (`<<...>>`) spans
    /// are stripped first so a bounded type param like `class Foo<T extends X>`
    /// is not mistaken for an inheritance clause.
    fn parse_supertypes(&mut self, line: &str, child_id: &str) -> Result<(), ParseError> {
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
            let parent = self.ensure_entity_kind(&name, parent_kind)?;
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
        Ok(())
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

    fn try_enum_decl(&mut self, line: &str) -> Result<bool, ParseError> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^enum\s+(\w+)\s*\{?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = self.ensure_entity_kind(&caps[1], EntityKind::Enum)?;
            if line.ends_with('{') {
                self.current_entity = Some(id.clone());
                self.current_entity_needs_body_location = true;
            }
            self.last_entity_id = Some(id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Association class: `(A, B) .. C` or `(A, B) -- C`. PlantUML draws a tiny
    /// anchor (`apoint`) on the A–B line and a dashed (`..`) / solid (`--`)
    /// connector to the association class `C`. Endpoints may be quoted.
    fn try_association_class(&mut self, line: &str) -> Result<bool, ParseError> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^\(\s*(?:"([^"]+)"|([\w.]+))\s*,\s*(?:"([^"]+)"|([\w.]+))\s*\)\s*(\.\.|-{2,})\s*(?:"([^"]+)"|([\w.]+))\s*$"#,
            )
            .unwrap()
        });
        let Some(caps) = RE.captures(line) else {
            return Ok(false);
        };
        let pick = |q: usize, b: usize| -> String {
            if let Some(m) = caps.get(q) {
                m.as_str().to_string()
            } else {
                caps.get(b).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        };
        let a_raw = pick(1, 2);
        let b_raw = pick(3, 4);
        let connector = &caps[5];
        let c_raw = pick(6, 7);
        let dashed = connector.starts_with('.');

        let a = self.ensure_entity(&a_raw)?;
        let b = self.ensure_entity(&b_raw)?;
        let c = self.ensure_entity(&c_raw)?;

        self.uid_events
            .push(ClassUidEvent::Association(self.association_classes.len()));
        self.association_classes
            .push(crate::diagram::class::AssociationClass {
                a,
                b,
                c,
                dashed,
                source_line: self.current_line,
            });
        Ok(true)
    }

    fn try_relationship(&mut self, line: &str) -> Result<bool, ParseError> {
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
                r#"^(?:"([^"]+)"|([\w./:]+))\s*(?:"([^"]+)")?\s*((?:\)[-.]+\(?|[-.]+\(|<\|--\|>|<\.\.>|<\|--|--\|>|\.\.\|>|<\|\.\.|<\.\.|<-->>|<-->|<->|\*--|--\*|o--|--o|-->>|<-{2,}|-{2,}>|<--|-->|->>|->|<-|-{1,}|\.{2,}>>|\.{2,}>|\.\.))\s*(?:"([^"]+)")?\s*(?:"([^"]+)"|([\w./:]+))(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });
        // ER crow's foot notation: entity1 CROW--CROW entity2 : "label"
        static ER_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^(\w+)\s+([|o}][|{o])--([|o][|{])\s+(\w+)(?:\s*:\s*(.+))?$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            // Quoted endpoints (groups 1/6) must be normalized the same way the
            // entity declaration normalizes a quoted name (whitespace → `_`,
            // creole markers stripped), so a relationship like
            // `"Fish & Chips" --> "Bread & Butter"` resolves to the existing
            // declared entity instead of creating a duplicate.
            let from_raw = if let Some(m) = caps.get(1) {
                m.as_str().to_string()
            } else {
                caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string()
            };
            let from_mult = caps.get(3).map(|m| m.as_str().to_string());
            let rel_str = &caps[4];
            let to_mult = caps.get(5).map(|m| m.as_str().to_string());
            let to_raw = if let Some(m) = caps.get(6) {
                m.as_str().to_string()
            } else {
                caps.get(7).map(|m| m.as_str()).unwrap_or("").to_string()
            };
            let (label, label_arrow) = parse_label_arrow(caps.get(8).map(|m| m.as_str()));

            let (kind, dashed, mut decorated_end) = parse_relationship_kind(rel_str);
            let mut from_decor = rel_str
                .starts_with(')')
                .then_some(EndpointDecor::Parenthesis);
            let mut to_decor = rel_str.ends_with('(').then_some(EndpointDecor::Parenthesis);
            if rel_str.contains(">>") || rel_str.contains("<<") {
                style.arrowhead = RelationshipArrowhead::Triangle;
            }
            let length = if direction.is_some_and(QueueDirection::is_horizontal) {
                1
            } else {
                relationship_length(rel_str)
            };
            let mut from = self.resolve_relationship_endpoint(&from_raw)?;
            let mut to = self.resolve_relationship_endpoint(&to_raw)?;
            let mut from_mult = from_mult;
            let mut to_mult = to_mult;
            if direction.is_some_and(QueueDirection::inverts_link) {
                // Java `CommandLinkClass.executeArg` constructs the source
                // link first, then replaces it with `Link.getInv()` for left
                // and up. `Link.getInv` swaps entities, quantifiers, roles and
                // ports while `LinkType.getInversed` swaps its decorations.
                std::mem::swap(&mut from, &mut to);
                std::mem::swap(&mut from_mult, &mut to_mult);
                decorated_end = invert_relationship_end(decorated_end);
                std::mem::swap(&mut from_decor, &mut to_decor);
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
            return Ok(true);
        }

        if let Some(caps) = ER_RE.captures(line) {
            let from_raw = &caps[1];
            let from_decor = parse_endpoint_decor(&caps[2]);
            let to_decor = parse_endpoint_decor(&caps[3]);
            let to_raw = &caps[4];
            let label = caps
                .get(5)
                .map(|m| normalize_relationship_label(m.as_str().trim().trim_matches('"')));

            let from = self.ensure_entity(from_raw)?;
            let to = self.ensure_entity(to_raw)?;

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
            return Ok(true);
        }

        // `CommandLinkLollipop` creates a synthetic LOLLIPOP_FULL/HALF leaf
        // beside the already-declared normal entity. Quoted endpoints use the
        // same identifier normalization as ordinary class relationships.
        static ENT_THEN_LOLLIPOP_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(?:"([^"]+)"|([\w./:]+))\s*([-=.]+)(\(\)|\(\(|\)\))\s*(?:"([^"]+)"|([\w./:]+))(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });
        static LOLLIPOP_THEN_ENT_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(?:"([^"]+)"|([\w./:]+))\s*(\(\)|\(\(|\)\))([-=.]+)\s*(?:"([^"]+)"|([\w./:]+))(?:\s*:\s*(.+))?$"#,
            )
            .unwrap()
        });
        let endpoint = |caps: &regex::Captures<'_>, quoted: usize, bare: usize| {
            caps.get(quoted).map_or_else(
                || caps.get(bare).map(|m| m.as_str()).unwrap_or("").to_string(),
                |m| m.as_str().to_string(),
            )
        };
        if let Some(caps) = ENT_THEN_LOLLIPOP_RE.captures(line) {
            let normal_raw = endpoint(&caps, 1, 2);
            let display = endpoint(&caps, 5, 6);
            let kind = if &caps[4] == "()" {
                EntityKind::LollipopFull
            } else {
                EntityKind::LollipopHalf
            };
            let normal = self.ensure_entity(&normal_raw)?;
            let Some(lollipop) = self.create_lollipop_entity(&normal, display, kind) else {
                return Ok(false);
            };
            self.push_relationship(Relationship {
                from: normal,
                to: lollipop,
                kind: RelationshipKind::Association,
                label: caps
                    .get(7)
                    .map(|m| normalize_relationship_label(m.as_str().trim().trim_matches('"'))),
                label_arrow: LinkArrow::None,
                from_multiplicity: None,
                to_multiplicity: None,
                from_decor: None,
                to_decor: None,
                decorated_end: RelationshipEnd::None,
                dashed: false,
                length: caps[3].len(),
                style: RelationshipStyle::default(),
                source_line: self.current_line,
            });
            return Ok(true);
        }
        if let Some(caps) = LOLLIPOP_THEN_ENT_RE.captures(line) {
            let display = endpoint(&caps, 1, 2);
            let normal_raw = endpoint(&caps, 5, 6);
            let kind = if &caps[3] == "()" {
                EntityKind::LollipopFull
            } else {
                EntityKind::LollipopHalf
            };
            let normal = self.ensure_entity(&normal_raw)?;
            let Some(lollipop) = self.create_lollipop_entity(&normal, display, kind) else {
                return Ok(false);
            };
            self.push_relationship(Relationship {
                from: lollipop,
                to: normal,
                kind: RelationshipKind::Association,
                label: caps
                    .get(7)
                    .map(|m| normalize_relationship_label(m.as_str().trim().trim_matches('"'))),
                label_arrow: LinkArrow::None,
                from_multiplicity: None,
                to_multiplicity: None,
                from_decor: None,
                to_decor: None,
                decorated_end: RelationshipEnd::None,
                dashed: false,
                length: caps[4].len(),
                style: RelationshipStyle::default(),
                source_line: self.current_line,
            });
            return Ok(true);
        }

        Ok(false)
    }

    fn try_inline_member(&mut self, line: &str) -> Result<bool, ParseError> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let entity_id = self.ensure_entity(&caps[1])?;
            let member_text = caps[2].trim();

            let member = parse_member(member_text);

            if let Some(entity) = self.find_entity_mut(&entity_id) {
                entity.members.push(member);
            }
            Ok(true)
        } else {
            Ok(false)
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

    fn try_package(&mut self, line: &str) -> Result<bool, ParseError> {
        // ClassDiagramFactory registers CommandPackage before the symbol
        // command. Its one non-greedy stereotype capture backtracks to the
        // final >> when needed; apparent `as` inside that blob is not an alias.
        static ORDINARY: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r#"(?i)^[+~#-]?\s*package\s+("[^"]+"|[^#\s{}]*)(?:\s+as\s+([\p{L}\p{N}_.]+))?\s*(?:\$[^\s<>]+\s*)*(<<.+?>>)?\s*(?:\$[^\s<>]+\s*)*(?:\[\[.*?\]\])?\s*(?:#[^\s{}]+)?\s*\{$"#
        ).unwrap()
        });
        let ordinary = ORDINARY.captures(line);
        // CommandPackage.executeArg stores visibility on the group; the symbol
        // command has no visibility prefix, so only consume it for this grammar.
        let visibility = if ordinary.is_some() {
            match line.as_bytes()[0] {
                b'+' => Visibility::Public,
                b'-' => Visibility::Private,
                b'#' => Visibility::Protected,
                b'~' => Visibility::Package,
                _ => Visibility::Default,
            }
        } else {
            Visibility::Default
        };
        let line = if visibility != Visibility::Default {
            line[1..].trim_start()
        } else {
            line
        };
        static STEREO_LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<(.+?)>>").unwrap());
        // CommandPackageWithUSymbol#getRegexConcat/executeArg distinguish code
        // from display in both alias directions. The quark owns code; the
        // group's Display never participates in identity resolution.
        static PREFIX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)^(package|namespace|cloud|database|folder|frame|rectangle|node|action|process|queue)\s+")
                .unwrap()
        });
        // CommandPackageWithUSymbol: identity-local stereotype before `as`,
        // then TAGS1, STEREOTYPE, TAGS2, URL, color and the opening brace.
        static STEREO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<<(.+?)>>\s*").unwrap());
        static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\$([^\s{}<>]+)\s*").unwrap());
        let Some(prefix) = PREFIX.captures(line) else {
            return Ok(false);
        };
        let input = &line[prefix.get(0).unwrap().end()..];
        let Some((first, quoted, mut rest)) = name_token(input) else {
            return Ok(false);
        };
        rest = rest.trim_start();
        let mut identity_stereotypes = Vec::new();
        while let Some(caps) = STEREO.captures(rest) {
            identity_stereotypes.push(caps[1].trim().to_string());
            rest = &rest[caps.get(0).unwrap().end()..];
        }
        let mut code = first.to_string();
        let mut display = first.to_string();
        let mut explicit_alias = false;
        if let Some((word, tail)) = rest.split_once(char::is_whitespace)
            && word.eq_ignore_ascii_case("as")
        {
            let Some((second, second_quoted, tail)) = name_token(tail.trim_start()) else {
                return Ok(false);
            };
            if second_quoted && !quoted {
                display = second.into();
            } else {
                code = second.into();
            }
            explicit_alias = true;
            rest = tail.trim_start();
        }
        let mut tags = Vec::new();
        while let Some(caps) = TAG.captures(rest) {
            tags.push(caps[1].to_string());
            rest = &rest[caps.get(0).unwrap().end()..];
        }
        let mut trailing_stereotypes = Vec::new();
        while let Some(caps) = STEREO.captures(rest) {
            trailing_stereotypes.push(caps[1].trim().to_string());
            rest = &rest[caps.get(0).unwrap().end()..];
        }
        while let Some(caps) = TAG.captures(rest) {
            tags.push(caps[1].to_string());
            rest = &rest[caps.get(0).unwrap().end()..];
        }
        let (mut url, clean_tail) = super::extract_link_url(rest);
        let url_tooltip = super::extract_link_tooltip(rest);
        // UrlBuilder permits a tooltip-only link with an empty href.
        if url.is_none() && rest.starts_with("[[") {
            url = Some(String::new());
        }
        if let Some(value) = &mut url
            && !value.is_empty()
            && !value.contains(":")
            && let Some(top) = self
                .meta
                .skinparams
                .iter()
                .rev()
                .find(|p| p.key.eq_ignore_ascii_case("topurl"))
        {
            *value = format!("{}{value}", top.value);
        }
        let Some(suffix) = clean_tail.trim().strip_suffix('{') else {
            return Ok(false);
        };
        let suffix = suffix.trim();
        let color = if suffix.is_empty() {
            None
        } else if let Some(value) = suffix.strip_prefix('#') {
            if value.is_empty() || value.chars().any(char::is_whitespace) {
                return Ok(false);
            }
            Some(value.into())
        } else {
            return Ok(false);
        };
        {
            let kind_key = prefix[1].to_ascii_lowercase();
            let kind_str = kind_key.as_str();
            let alias = if let Some(caps) = &ordinary {
                display = caps[1].trim_matches('"').into();
                caps.get(2).map(|m| m.as_str().to_string())
            } else {
                explicit_alias.then_some(code)
            };
            // RegexResult.getLazzy selects the unnumbered STEREOTYPE slot first.
            let stereotypes = if let Some(caps) = &ordinary {
                // StereotypeDecoration.cutLabels extracts chevron spans only.
                caps.get(3)
                    .map(|m| {
                        STEREO_LABEL
                            .captures_iter(m.as_str())
                            .map(|c| c[1].trim().to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            } else if trailing_stereotypes.is_empty() {
                identity_stereotypes
            } else {
                trailing_stereotypes
            };
            let kind = match kind_str {
                "namespace" => PackageKind::Namespace,
                "cloud" => PackageKind::Cloud,
                "database" => PackageKind::Database,
                "folder" => PackageKind::Folder,
                "frame" => PackageKind::Frame,
                "rectangle" => PackageKind::Rectangle,
                "node" => PackageKind::Node,
                "action" => PackageKind::Action,
                "process" => PackageKind::Process,
                "queue" => PackageKind::Queue,
                _ => PackageKind::Package,
            };
            // `CommandPackage#getRegexConcat` consumes the complete line.
            // `CommandPackage#executeArg` sends `AS` to `quarkInContext` and
            // keeps `NAME` solely as display; `CommandNamespace#executeArg`
            // follows the same quark-backed group discipline.
            let code = alias.as_deref().unwrap_or(&display);
            let path = self.resolve_group_path(code)?;
            if path.is_empty() {
                return Ok(false);
            }
            let requested_display_name = (alias.is_some() || path.len() > 1).then(|| {
                if alias.is_some() {
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
                        package.visibility = visibility;
                        package.color = color.clone();
                        if !stereotypes.is_empty() {
                            package.stereotypes = stereotypes.clone();
                        }
                        package.tags.extend(tags.clone());
                        if url.is_some() {
                            package.url = url.clone();
                            package.url_tooltip = url_tooltip.clone();
                        }
                        // gotoGroup creates data only when the quark has none;
                        // existing materialized group data retains its location.
                        if package.source_line == 0 {
                            self.uid_events
                                .push(ClassUidEvent::Package(package.name.clone()));
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
                        visibility: if is_final {
                            visibility
                        } else {
                            Visibility::Default
                        },
                        name,
                        tags: if is_final { tags.clone() } else { Vec::new() },
                        url: if is_final { url.clone() } else { None },
                        url_tooltip: if is_final { url_tooltip.clone() } else { None },
                        kind: if is_final { kind } else { PackageKind::Package },
                        color: if is_final { color.clone() } else { None },
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
            Ok(true)
        }
    }

    fn try_url(&mut self, line: &str) -> Result<bool, ParseError> {
        // classdiagram.command.CommandUrl#getRegexConcat: consume the complete
        // command, then resolve existing quark data without allocating a UID.
        // Pattern2 expands %g to ASCII, curly and Jaws invisible quotes.
        static URL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r#"(?i)^url\s*(?:of|for)?\s+(?P<code>[\p{L}\p{N}_.]+|["“”\u{e121}][^"“”\u{e121}]+["“”\u{e121}])\s+(?:is)?\s*(?P<url>\[\[.+\]\])$"#
        ).unwrap()
        });
        let Some(caps) = URL.captures(line) else {
            return Ok(false);
        };
        // CucaDiagram#cleanId delegates to StringUtils#isDoubleQuote, which
        // removes ASCII/curly quotes but not the internal Jaws %g delimiter.
        // Pattern2 recognition and quark identity cleanup are separate stages.
        let code = caps["code"].trim_matches(['"', '“', '”']);
        // CucaDiagram.quarkInContextSafe(true) first reuses a unique
        // short-name quark, then resolves relative to the current group.
        let path = self.resolve_quark_path(code, QuarkLookup::ReuseUnique)?;
        let id = self.path_id(&path);
        let Some((url, tooltip)) = self.parse_note_or_command_url(&caps["url"]) else {
            return Ok(false);
        };
        let url = Some(url);
        if let Some(&index) = self.entity_by_path.get(&path) {
            self.entities[index].url = url;
            self.entities[index].url_tooltip = tooltip;
        } else if let Some(&index) = self.package_by_path.get(&path) {
            self.packages[index].url = url;
            self.packages[index].url_tooltip = tooltip;
        } else if let Some(note) = self
            .notes
            .iter_mut()
            .find(|note| note.alias.as_deref() == Some(&id))
        {
            note.url = url;
            note.url_tooltip = tooltip;
        } else {
            return Err(ParseError {
                line: self.current_line,
                message: format!("{code} does not exist"),
            });
        }
        Ok(true)
    }

    fn parse_note_or_command_url(&self, text: &str) -> Option<(String, Option<String>)> {
        // UrlBuilder#getRegexp/getUrl tests these strict alternatives in order.
        // Keep this consuming grammar local: older declaration extractors are
        // not a validator for a standalone CommandUrl.
        static BUILDERS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
            [
            r#"^\[\[\s*["“”\u{e121}](?P<link>[^"“”\u{e121}]+)["“”\u{e121}](?:\s*\{(?P<tip>[^{}]*)\})?(?:\s[^\s{}\[\]][^\[\]]*)?\s*\]\]$"#,
            r"^\[\[\s*\{(?P<tip>.*)\}\s*\]\]$",
            r"^\[\[\s*\{(?P<tip>[^{}]*)\}\s*[^\s{}\[\]][^\[\]]*\s*\]\]$",
            r#"^\[\[\s*(?P<link>[^\s"“”\u{e121}{}\[\]]+?)\s*\{(?P<tip>.+)\}\s*\]\]$"#,
            r#"^\[\[\s*(?P<link>[^\s"“”\u{e121}\[\]]+?)(?:\s*\{(?P<tip>[^{}]*)\})?(?:\s[^\s{}\[\]][^\[\]]*)?\s*\]\]$"#,
        ].into_iter().map(|pattern| Regex::new(pattern).unwrap()).collect()
        });
        let value = BUILDERS.iter().find_map(|pattern| pattern.captures(text))?;
        let mut url = value
            .name("link")
            .map_or(String::new(), |m| m.as_str().to_string());
        let tooltip = value.name("tip").map(|m| m.as_str().to_string());
        // UrlBuilder.withTopUrl prefixes everything except these three schemes;
        // tooltip-only alternatives bypass that helper altogether.
        if value.name("link").is_some()
            && !["http:", "https:", "file:"]
                .iter()
                .any(|scheme| url.starts_with(scheme))
            && let Some(top) = self
                .meta
                .skinparams
                .iter()
                .rev()
                .find(|p| p.key.eq_ignore_ascii_case("topurl"))
        {
            url = format!("{}{url}", top.value);
        }
        Some((url, tooltip))
    }

    fn try_note(&mut self, line: &str) -> Result<bool, ParseError> {
        // CommandFactoryNoteOnEntity owns the same ordered modifiers for
        // single-line, end-note and braced bodies. A recognized start consumes
        // its complete body in parse_line, including command-looking text.
        // NameAndCodeParser.codeForClass uses Pattern2 %g for both quote
        // boundaries and bare-token exclusions; cleanId below is a separate step.
        static ATTACHED: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r#"(?i)^note\s+(?P<position>top|bottom|left|right)(?:\s+of\s+(?P<target>["“”\u{e121}][^"“”\u{e121}]+["“”\u{e121}]|[^\s{}"“”\u{e121}<>]+))?\s*(?P<tags1>(?:\$[^\s{}"<>$]+\s*)*)(?:<<(?P<stereo>.+?)>>)?\s*(?P<tags2>(?:\$[^\s{}"<>$]+\s*)*)(?P<color>#[^\s{}]+)?\s*(?P<url>\[\[.*?\]\])?\s*(?:(?P<brace>\{)|:\s*(?P<text>.*))?$"#
        ).unwrap()
        });
        // CommandFactoryNote has a named NOTE code, not a class display name;
        // both its forms allow tags, stereotype and background color.
        static FLOATING: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r#"(?i)^note\s+(?:"(?P<text>[^"]+)"\s+)?as\s+(?P<alias>[\p{L}\p{N}_.]+)\s*(?P<tags1>(?:\$[^\s{}"<>$]+\s*)*)(?:<<(?P<stereo>.+?)>>)?\s*(?P<color>#[^\s{}]+)?\s*$"#
        ).unwrap()
        });
        if let Some(caps) = ATTACHED.captures(line).or_else(|| FLOATING.captures(line)) {
            let attached = caps.name("position").is_some();
            let target = if let Some(target) = caps.name("target") {
                let path = self.resolve_quark_path(
                    target.as_str().trim_matches(['"', '“', '”']),
                    QuarkLookup::ReuseUnique,
                )?;
                let id = self.path_id(&path);
                // Quark<Entity>.getData includes NOTE leaves and groups.
                if !self.entity_by_path.contains_key(&path)
                    && !self.package_by_path.contains_key(&path)
                    && !self
                        .notes
                        .iter()
                        .any(|note| note.alias.as_deref() == Some(&id))
                {
                    return Err(ParseError {
                        line: self.current_line,
                        message: format!("unknown note target: {}", target.as_str()),
                    });
                }
                Some(id)
            } else if attached {
                Some(self.last_entity_id.clone().ok_or_else(|| ParseError {
                    line: self.current_line,
                    message: "nothing to note to".into(),
                })?)
            } else {
                None
            };
            // CommandFactoryNoteOnEntity#createSingleLine passes null to
            // executeInternal; only its multiline command constructs a Url.
            // UrlBuilder#getRegexp is part of both command grammars, even
            // when createSingleLine deliberately passes null to executeInternal.
            let (mut url, mut tooltip) = if let Some(value) = caps.name("url") {
                let Some((url, tooltip)) = self.parse_note_or_command_url(value.as_str()) else {
                    return Ok(false);
                };
                (Some(url), tooltip)
            } else {
                (None, None)
            };
            if attached && caps.name("text").is_some() {
                url = None;
                tooltip = None;
            }
            let mut note = Note {
                target,
                position: caps
                    .name("position")
                    .map(|value| parse_note_position(&value.as_str().to_ascii_lowercase())),
                alias: caps.name("alias").map(|value| value.as_str().to_string()),
                color: caps.name("color").map(|value| value.as_str().to_string()),
                stereotypes: caps
                    .name("stereo")
                    .map(|value| vec![value.as_str().trim().to_string()])
                    .unwrap_or_default(),
                tags: ["tags1", "tags2"]
                    .into_iter()
                    .filter_map(|key| caps.name(key))
                    .flat_map(|value| value.as_str().split_whitespace())
                    .map(|tag| tag.trim_start_matches('$').to_string())
                    .collect(),
                url,
                url_tooltip: tooltip,
                source_line: self.current_line,
                ..Note::default()
            };
            if let Some(text) = caps.name("text") {
                note.lines = text
                    .as_str()
                    .split("\\n")
                    .map(|s| s.trim_end().to_string())
                    .collect();
                self.push_note(note)?;
            } else {
                self.note_with_bracket = caps.name("brace").is_some();
                self.current_note = Some(note);
            }
            return Ok(true);
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
                text.split("\\n")
                    .map(|s| s.trim_end().to_string())
                    .collect()
            };
            self.push_note(Note {
                owner_package: None,
                lines,
                target: None,
                position: None,
                alias: None,
                color: None,
                source_line: self.current_line,
                ..Note::default()
            })?;
            return Ok(true);
        }
        // `note on link` — multi-line note attached to the last relationship.
        if line == "note on link" {
            self.current_note = Some(Note {
                owner_package: None,
                lines: Vec::new(),
                target: None,
                position: None,
                alias: None,
                color: None,
                source_line: self.current_line,
                ..Note::default()
            });
            return Ok(true);
        }

        // `end note` is handled in parse_line; skip it here if encountered standalone.
        if line == "end note" {
            return Ok(true);
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
                let id = self.ensure_entity("note")?;
                if let Some(ent) = self.find_entity_mut(&id) {
                    ent.members.push(member);
                }
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn try_meta(&mut self, line: &str) -> bool {
        // CommonCommands' Pattern2 grammar is case insensitive. Preserve data
        // after the command prefix; only recognized syntax is canonicalized.
        static PREFIX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r"(?i)^(?:(left|right|center)\s+)?(header|footer|title|caption|legend|skinparam|hide|show|remove)(?:\s+|$)"
        ).unwrap()
        });
        let normalized;
        let line = if let Some(caps) = PREFIX.captures(line) {
            let prefix = caps.get(0).unwrap();
            let alignment = caps
                .get(1)
                .map(|m| format!("{} ", m.as_str().to_ascii_lowercase()))
                .unwrap_or_default();
            let tail = &line[prefix.end()..];
            normalized = format!(
                "{alignment}{}{}{}",
                caps[2].to_ascii_lowercase(),
                if tail.is_empty() { "" } else { " " },
                tail
            );
            normalized.as_str()
        } else {
            line
        };

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
        // CommandMultilinesHeader/Footer and their single-line equivalents
        // retain the alignment separately from the opaque text body.
        let (alignment, command) = if let Some(rest) = line.strip_prefix("left ") {
            (Some(LegendHorizontalAlignment::Left), rest)
        } else if let Some(rest) = line.strip_prefix("right ") {
            (Some(LegendHorizontalAlignment::Right), rest)
        } else if let Some(rest) = line.strip_prefix("center ") {
            (Some(LegendHorizontalAlignment::Center), rest)
        } else {
            (None, line)
        };
        for (name, block) in [("header", MetaBlock::Header), ("footer", MetaBlock::Footer)] {
            if command == name || command.starts_with(&format!("{name} ")) {
                let value = command[name.len()..].trim();
                if value.is_empty() {
                    self.meta_block = Some(block);
                    self.meta_block_lines = 0;
                }
                if name == "header" {
                    self.header_alignment = alignment;
                    self.header_line.get_or_insert(self.current_line);
                    if !value.is_empty() {
                        self.meta.header = Some(value.into());
                    }
                } else {
                    self.footer_alignment = alignment;
                    self.footer_line.get_or_insert(self.current_line);
                    if !value.is_empty() {
                        self.meta.footer = Some(value.into());
                    }
                }
                return true;
            }
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
        let mut legend_words = line.split_ascii_whitespace();
        let is_legend = legend_words.next() == Some("legend");
        let legend_tail = legend_words
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        let legend_position = legend_tail.split_whitespace().collect::<Vec<_>>();
        let valid_legend_position = matches!(
            legend_position.as_slice(),
            [] | ["left" | "center" | "right"]
                | ["top" | "bottom"]
                | ["top" | "bottom", "left" | "center" | "right"]
        );
        if is_legend && valid_legend_position {
            for word in legend_position {
                match word {
                    "left" => {
                        self.meta.legend_horizontal_alignment = LegendHorizontalAlignment::Left;
                    }
                    "center" => {
                        self.meta.legend_horizontal_alignment = LegendHorizontalAlignment::Center;
                    }
                    "right" => {
                        self.meta.legend_horizontal_alignment = LegendHorizontalAlignment::Right;
                    }
                    "top" => {
                        self.meta.legend_vertical_alignment = LegendVerticalAlignment::Top;
                    }
                    "bottom" => {
                        self.meta.legend_vertical_alignment = LegendVerticalAlignment::Bottom;
                    }
                    _ => unreachable!("validated legend position"),
                }
            }
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
        if !self.description_factory && super::is_allow_mixing_command(line) {
            self.allow_mixing = true;
            return true;
        }
        // CommonCommands.addCommonScaleCommands: rendering consumes scale from
        // DiagramMeta.source (scale::directive_from_meta). Recognize complete
        // forms here; this command does not create a class entity.
        static SCALE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
            r"(?i)^scale\s+(?:max\s+)?[0-9]+(?:\.[0-9]+)?(?:\s*/\s*[0-9]+(?:\.[0-9]+)?|\s*[*x]\s*[0-9]+(?:\.[0-9]+)?|\s+(?:width|height))?\s*$"
        ).unwrap()
        });
        // CommandAssumeTransparent.executeArg is explicitly a no-op in Java.
        static ASSUME: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^!assume\s+transparent\s+(?:dark|light)$").unwrap());
        SCALE.is_match(line) || ASSUME.is_match(line)
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
        "record" => EntityKind::Record,
        "protocol" => EntityKind::Protocol,
        "struct" => EntityKind::Struct,
        "exception" => EntityKind::Exception,
        "metaclass" => EntityKind::Metaclass,
        "stereotype" => EntityKind::Stereotype,
        "dataclass" => EntityKind::Dataclass,
        "object" => EntityKind::Object,
        "state" => EntityKind::State,
        "circle" => EntityKind::Circle,
        "diamond" => EntityKind::Diamond,
        "actor" => EntityKind::Actor,
        "usecase" => EntityKind::UseCase,
        "component" => EntityKind::Component,
        "boundary" => EntityKind::Boundary,
        "database" => EntityKind::Database,
        "queue" => EntityKind::Queue,
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
    } else if s.contains('.') {
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
        return (Some(normalize_relationship_label(label)), LinkArrow::None);
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
    (
        text.filter(|text| !text.is_empty())
            .map(|text| normalize_relationship_label(&text)),
        arrow,
    )
}

/// Java `LinkArg.build` passes every relationship label through
/// `Display.manageGuillemet`, whose default `Guillemet.GUILLEMET` conversion
/// replaces valid `<<...>>` groups while preserving malformed angle syntax.
fn normalize_relationship_label(label: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<<\s?((?:<&\w+>|[^<>])+?)\s?>>").unwrap());
    RE.replace_all(label, "\u{00ab}$1\u{00bb}").into_owned()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ClassDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_class(&lines).unwrap()
    }

    #[test]
    fn checked_quarks_preserve_root_and_local_data_and_uid_order() {
        let d = parse(
            "class Ledger\npackage Harbor {\nclass Ledger\nurl Ledger [[https://example.org/local]]\n}",
        );
        assert_eq!(
            d.entities.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["Ledger", "Harbor.Ledger"]
        );
        assert_eq!(d.entities[0].url, None);
        assert_eq!(
            d.entities[1].url.as_deref(),
            Some("https://example.org/local")
        );
        assert_eq!(
            format!("{:?}", d.uid_events),
            "[Entity(\"Ledger\"), Package(\"Harbor\"), Entity(\"Harbor.Ledger\")]"
        );
        let d = parse(
            "class Ledger\npackage Harbor {\nnote \"receipt\" as Ledger\nurl Ledger [[https://example.org/note]]\n}",
        );
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].alias.as_deref(), Some("Harbor.Ledger"));
        assert_eq!(d.notes[0].owner_package.as_deref(), Some("Harbor"));
        assert_eq!(d.notes[0].url.as_deref(), Some("https://example.org/note"));
        assert_eq!(d.entities[0].url, None);
        assert_eq!(
            format!("{:?}", d.uid_events),
            "[Entity(\"Ledger\"), Package(\"Harbor\"), Note(0)]"
        );
    }

    #[test]
    fn checked_quarks_reject_bad_separators_and_current_group_reuse() {
        for source in [
            "class Harbor.Dock\nurl Harbor..Dock [[https://example.org/bad]]",
            "class Harbor.Dock\nurl Harbor.Dock. [[https://example.org/bad]]",
            "set namespaceSeparator ::\nclass Harbor::Dock\nurl \"Harbor::::Dock\" [[https://example.org/bad]]",
            "package Harbor {\nclass Dock\nurl Harbor [[https://example.org/bad]]\n}",
            "class Harbor\nclass Harbor.Dock",
        ] {
            let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
            assert!(parse_class(&lines).is_err(), "{source}");
            assert!(
                super::super::parse(&format!("@startuml\n{source}\n@enduml")).is_err(),
                "{source}"
            );
        }
        let d =
            parse("package Harbor {\nclass Harbor\nurl Harbor [[https://example.org/child]]\n}");
        assert_eq!(d.entities[0].id, "Harbor.Harbor");
        assert_eq!(
            d.entities[0].url.as_deref(),
            Some("https://example.org/child")
        );
        assert_eq!(d.packages[0].url, None);
    }

    #[test]
    fn command_url_owns_pattern2_quotes_without_changing_url_content() {
        for (left, right) in [('"', '"'), ('“', '”'), ('”', '“')] {
            let d = parse(&format!(
                "note \"receipt\" as Ledger\nurl {left}Ledger{right} [[{left}doc with spaces{right}{{Hint}}]]"
            ));
            assert_eq!(d.notes[0].url.as_deref(), Some("doc with spaces"));
            assert_eq!(d.notes[0].url_tooltip.as_deref(), Some("Hint"));
            assert_eq!(d.uid_events.len(), 1);
        }
    }

    #[test]
    fn command_url_internal_quote_is_not_an_identity_cleanup_quote() {
        let source =
            "note \"receipt\" as Ledger\nurl \u{e121}Ledger\u{e121} [[https://example.org/]]";
        let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
        assert!(parse_class(&lines).is_err());
    }

    #[test]
    fn attached_note_command_form_owns_url_assignment() {
        for position in ["left", "right", "top", "bottom"] {
            for (body, expected) in [
                (" : receipt", None),
                ("\nreceipt\nend note", Some("https://example.org/note")),
                (" {\nreceipt\n}", Some("https://example.org/note")),
            ] {
                let d = parse(&format!(
                    "class Dock\nnote {position} of Dock [[https://example.org/note{{Hint}}]]{body}"
                ));
                assert_eq!(d.entities.len(), 1);
                assert_eq!(d.notes.len(), 1);
                assert_eq!(d.notes[0].target.as_deref(), Some("Dock"));
                assert_eq!(d.notes[0].url.as_deref(), expected);
                assert_eq!(d.notes[0].url_tooltip.as_deref(), expected.map(|_| "Hint"));
                assert_eq!(d.notes[0].lines, ["receipt"]);
            }
        }
    }

    #[test]
    fn url_command_consumes_existing_named_quarks_without_new_uids() {
        for command in ["url Memo", "url of Memo is", "URL for \"Memo\" is"] {
            let source = format!(
                "package Harbor {{\nclass Anchor\nnote \"chart\" as Memo\n{command} [[https://example.org/map{{Open chart}}]]\n}}"
            );
            let d = parse(&source);
            assert_eq!(d.entities.len(), 1);
            assert_eq!(d.notes.len(), 1);
            assert_eq!(d.notes[0].alias.as_deref(), Some("Harbor.Memo"));
            assert_eq!(d.notes[0].url.as_deref(), Some("https://example.org/map"));
            assert_eq!(d.notes[0].url_tooltip.as_deref(), Some("Open chart"));
            assert_eq!(
                d.uid_events
                    .iter()
                    .filter(|e| matches!(e, ClassUidEvent::Note(_)))
                    .count(),
                1
            );
        }
        let d = parse(
            "note \"chart\" as Harbor.Memo\nurl for \"Harbor.Memo\" [[https://example.org/qualified]]\nclass Bolt",
        );
        assert_eq!(
            d.notes[0].url.as_deref(),
            Some("https://example.org/qualified")
        );
    }

    #[test]
    fn url_command_uses_unique_lookup_and_strict_url_builder_forms() {
        let d = parse(
            "skinparam topurl https://example.org/\npackage Harbor {\nnote \"chart\" as Memo\n}\nurl for Memo is [[\"path with spaces\"{Inspect} label]]",
        );
        assert_eq!(
            d.notes[0].url.as_deref(),
            Some("https://example.org/path with spaces")
        );
        assert_eq!(d.notes[0].url_tooltip.as_deref(), Some("Inspect"));
        let d = parse("note \"chart\" as Memo\nurl Memo [[{Only tooltip}]]");
        assert_eq!(d.notes[0].url.as_deref(), Some(""));
        assert_eq!(d.notes[0].url_tooltip.as_deref(), Some("Only tooltip"));
        let source = "package East {\nnote \"one\" as Memo\n}\npackage West {\nnote \"two\" as Memo\n}\nurl Memo [[https://example.org/]]";
        assert!(parse_class(&source.lines().map(str::to_string).collect::<Vec<_>>()).is_err());
    }

    #[test]
    fn url_command_does_not_create_forward_targets_or_accept_inline_note_urls() {
        for source in [
            "class Bolt\nurl of Missing is [[https://example.org/]]",
            "class Bolt\nurl of Memo is [[https://example.org/]]\nnote \"chart\" as Memo",
            "class Bolt\nnote \"chart\" as Memo [[https://example.org/]]",
            "class Bolt\nnote \"chart\" as Memo\nurl of Memo is [[https://example.org/]] garbage",
        ] {
            assert!(
                parse_class(&source.lines().map(str::to_string).collect::<Vec<_>>()).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn root_entity_retains_earlier_participant_command_ownership() {
        for source in [
            "entity Harbor",
            "entity \"record as [Harbor]\" as H #blue",
            "entity H\nH -> H : echo",
        ] {
            let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
            assert!(!native_container_candidate(&lines));
        }
        for source in [
            "record Harbor",
            "struct Harbor",
            "entity Harbor {\n}",
            "+entity Harbor",
            "package Bay {\nentity Harbor\n}",
        ] {
            let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
            assert!(native_container_candidate(&lines));
            assert_eq!(parse_class(&lines).unwrap().entities.len(), 1);
        }
    }

    #[test]
    fn native_trial_owns_brace_notes_and_group_quarks() {
        for kind in ["record", "struct", "class", "protocol"] {
            let input = format!(
                "{kind} Root\n+package Outer {{\n{kind} Inner\n}}\nnote left of Outer {{\ncomponent must stay text\nend note\n}}\nnote \"qualified\" as Outer.Memo\nOuter.Memo .. Outer.Inner"
            );
            let lines = input.lines().map(str::to_owned).collect::<Vec<_>>();
            assert!(native_container_candidate(&lines));
            let diagram = parse_class(&lines).unwrap();
            assert_eq!(diagram.entities.len(), 2);
            assert_eq!(diagram.notes.len(), 2);
            assert_eq!(diagram.notes[0].target.as_deref(), Some("Outer"));
            assert_eq!(
                diagram.notes[0].lines,
                ["component must stay text", "end note"]
            );
            assert_eq!(diagram.notes[1].owner_package.as_deref(), Some("Outer"));
            assert_eq!(diagram.notes[1].alias.as_deref(), Some("Outer.Memo"));
            assert_eq!(diagram.packages[0].visibility, Visibility::Public);
            assert_eq!(diagram.relationships[0].from, "Outer.Memo");
        }
        for input in [
            "class A\nA .. Memo\nnote \"cannot replace class\" as Memo",
            "class A\nnote as Memo\ntext\n}",
        ] {
            assert!(parse_class(&input.lines().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
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
            "allowmixing\nclass A\nabstract class B\ninterface C\nenum D\nannotation E\nentity F\nstate G",
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
    fn lollipop_commands_create_synthetic_leaves_in_both_orientations() {
        let d = parse(
            "class FreshRegistry731\n\
             class FreshConsumer739\n\
             FreshRegistry731 -() ProvidedPort743\n\
             RequiredPort751 ()-- FreshConsumer739\n\
             FreshRegistry731 -(( HalfPort757",
        );

        assert_eq!(d.entities.len(), 5);
        assert_eq!(d.entities[2].label, "ProvidedPort743");
        assert_eq!(d.entities[2].kind, EntityKind::LollipopFull);
        assert_eq!(d.entities[3].label, "RequiredPort751");
        assert_eq!(d.entities[3].kind, EntityKind::LollipopFull);
        assert_eq!(d.entities[4].label, "HalfPort757");
        assert_eq!(d.entities[4].kind, EntityKind::LollipopHalf);
        assert_eq!(d.relationships[0].from, "FreshRegistry731");
        assert_eq!(d.relationships[0].to, d.entities[2].id);
        assert_eq!(d.relationships[0].length, 1);
        assert_eq!(d.relationships[1].from, d.entities[3].id);
        assert_eq!(d.relationships[1].to, "FreshConsumer739");
        assert_eq!(d.relationships[1].length, 2);
        assert_eq!(d.relationships[2].from, "FreshRegistry731");
        assert_eq!(d.relationships[2].to, d.entities[4].id);
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
            ("EdgeHost", EntityKind::Node, 7),
            ("TrustBoundary", EntityKind::Rectangle, 8),
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
                arrowhead: RelationshipArrowhead::Arrow,
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
    fn renamed_relationship_labels_apply_linkarg_guillemet_normalization() {
        let d = parse(
            "FreshEmitter3101 --> FreshSink3109 : <<streams>> and << audited >>\n\
             FreshOwner3119 ||--o{ FreshRecord3121 : <<owns <&key>>>\n\
             FreshLeft3137 -- FreshRight3163 : <<broken>x>>",
        );

        assert_eq!(
            d.relationships[0].label.as_deref(),
            Some("\u{00ab}streams\u{00bb} and \u{00ab}audited\u{00bb}")
        );
        assert_eq!(
            d.relationships[1].label.as_deref(),
            Some("\u{00ab}owns <&key>\u{00bb}")
        );
        assert_eq!(d.relationships[2].label.as_deref(), Some("<<broken>x>>"));
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
    fn renamed_quoted_qualified_class_keeps_leaf_display_and_full_identity() {
        let d = parse(
            "class \"fresh.domain.My Worker\"\n\
             class FreshSink3503\n\
             \"fresh.domain.My Worker\" --> FreshSink3503",
        );

        assert_eq!(d.packages.len(), 2);
        assert_eq!(d.packages[0].name, "fresh");
        assert_eq!(d.packages[1].name, "fresh.domain");
        assert_eq!(d.entities[0].id, "fresh.domain.My Worker");
        assert_eq!(d.entities[0].label, "My Worker");
        assert_eq!(d.relationships[0].from, "fresh.domain.My Worker");
        assert_eq!(d.relationships[0].to, "FreshSink3503");
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
    fn multiline_legend_preserves_both_position_axes() {
        let top_right =
            parse("legend top right\n  Fresh alignment text\nendlegend\nclass WideRenamedEntity");
        assert_eq!(
            top_right.meta.legend_horizontal_alignment,
            LegendHorizontalAlignment::Right
        );
        assert_eq!(
            top_right.meta.legend_vertical_alignment,
            LegendVerticalAlignment::Top
        );

        let bottom_left = parse("legend bottom left\n  Other text\nendlegend\nclass AnotherEntity");
        assert_eq!(
            bottom_left.meta.legend_horizontal_alignment,
            LegendHorizontalAlignment::Left
        );
        assert_eq!(
            bottom_left.meta.legend_vertical_alignment,
            LegendVerticalAlignment::Bottom
        );
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
        assert_eq!(
            d.relationships[0].style.arrowhead,
            RelationshipArrowhead::Triangle
        );
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
    fn consuming_trial_keeps_display_tokens_and_modifiers_as_entity_data() {
        for command in [
            "class",
            "ClAsS",
            "static\tclass",
            "+class",
            "~ class",
            "record",
        ] {
            let input = format!(
                "@startuml\nNoDe\t\"Orchard District\" {{\n{command} \"East as [Inventory]\" as Store\n}}\n@enduml"
            );
            let crate::diagram::Diagram::Class(d) = super::super::parse(&input).unwrap() else {
                panic!("{input}")
            };
            assert_eq!(d.entities.len(), 1, "{input}");
            let e = &d.entities[0];
            assert_eq!(e.id, "Orchard District.Store");
            assert_eq!(e.label, "East as [Inventory]");
            assert_eq!(e.is_static, command.starts_with("static"));
            if command.starts_with('+') {
                assert_eq!(e.visibility, Visibility::Public);
            }
            if command == "record" {
                assert_eq!(e.kind, EntityKind::Record);
            }
        }
    }

    #[test]
    fn consuming_trial_owns_page_note_and_member_bodies() {
        for (open, close) in [
            ("left header", "endheader"),
            ("RIGHT HEADER", "END HEADER"),
            ("center footer", "endfooter"),
            ("legend right", "endlegend"),
            ("title", "endtitle"),
        ] {
            let input = format!(
                "@startuml\n{open}\ncomponent fictitious\n[display]\n{close}\nnode \"Orchard District\" {{\nclass Actual {{\ncomponent attribute\n[member]\n}}\nNOTE right of Actual\ncomponent prose\nENDNOTE\n}}\n@enduml"
            );
            let crate::diagram::Diagram::Class(d) = super::super::parse(&input).unwrap() else {
                panic!("{input}")
            };
            assert_eq!(d.entities.len(), 1, "{input}");
            assert_eq!(d.entities[0].members.len(), 2);
            assert_eq!(d.notes.len(), 1);
            assert_eq!(
                d.notes[0].target.as_deref(),
                Some("Orchard District.Actual")
            );
            assert_eq!(d.notes[0].lines, ["component prose"]);
        }
    }

    #[test]
    fn ordered_mixing_rejection_survives_the_public_dispatcher() {
        for declaration in ["component Endpoint", "COMPONENT\tEndpoint"] {
            let early = format!(
                "@startuml\nnode \"Orchard District\" {{\nclass Record\nallowmixing\n{declaration}\n}}\n@enduml"
            );
            let late = format!(
                "@startuml\nnode \"Orchard District\" {{\nclass Record\n{declaration}\nallowmixing\n}}\n@enduml"
            );
            let crate::diagram::Diagram::Class(d) = super::super::parse(&early).unwrap() else {
                panic!("{early}")
            };
            assert_eq!(d.entities.len(), 2);
            assert_eq!(d.entities[1].kind, EntityKind::Component);
            assert!(super::super::parse(&late).is_err(), "{late}");
        }
        let d = parse("node Root {\nclass Data\nmix_component \"Inbound Port\" as Port\n}");
        assert_eq!(d.entities[1].id, "Root.Port");
        assert_eq!(d.entities[1].label, "Inbound Port");
        assert_eq!(d.entities[1].kind, EntityKind::Component);
        let lines = vec!["class Known".into(), "invented command".into()];
        assert!(parse_class(&lines).is_err());
    }

    #[test]
    fn symbol_aliases_reuse_canonical_scope_and_notes_follow_identity() {
        for outer in [
            "node \"Display as [text]\" as Root",
            "node Root as \"Display as [text]\"",
        ] {
            let d = parse(&format!(
                "{outer} {{\nfolder \"Storage Area\" as Inner {{\nclass \"Shelf Label\" as Shelf\nnote right of Shelf : attached\n}}\n}}\nnode Root {{\nclass Spare\n}}"
            ));
            assert_eq!(d.packages.len(), 2);
            assert_eq!(d.packages[0].name, "Root");
            assert_eq!(
                d.packages[0].display_name.as_deref(),
                Some("Display as [text]")
            );
            assert_eq!(d.entities[0].id, "Root.Inner.Shelf");
            assert_eq!(d.notes[0].target.as_deref(), Some("Root.Inner.Shelf"));
            assert_eq!(d.entities[1].id, "Root.Spare");
        }
    }
    #[test]
    fn quoted_codes_remain_distinct_from_underscore_names_and_own_decorations() {
        let d = parse(
            "node Site {\nclass \"Air Probe\"\nclass Air_Probe\n\"Air Probe\" --> Air_Probe\nnote right of \"Air Probe\" : attached\nclass \"X extends Ghost [[literal]] #red\" as Quoted\n}",
        );
        assert_eq!(d.entities.len(), 3);
        assert_eq!(d.entities[0].id, "Site.Air Probe");
        assert_eq!(d.entities[1].id, "Site.Air_Probe");
        assert_eq!(d.notes[0].target.as_deref(), Some("Site.Air Probe"));
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from, "Site.Air Probe");
        assert!(d.entities[2].color.is_none());
        assert!(d.entities[2].url.is_none());
    }

    #[test]
    fn restored_command_families_retain_declarations_and_group_endpoints() {
        let d = parse(
            "action \"Audit Gate\" as Gate <<Phase>> #Wheat {\nclass Record\nallowmixing\nBOUNDARY\t\"Intake\" as Intake\n}\nprocess Review {\nclass Verdict\n}\nGate )--( Review",
        );
        assert_eq!(d.packages.len(), 2);
        assert_eq!(d.packages[0].kind, PackageKind::Action);
        assert_eq!(d.packages[1].kind, PackageKind::Process);
        assert_eq!(d.entities.len(), 3);
        assert_eq!(d.entities[1].id, "Gate.Intake");
        assert_eq!(d.entities[1].kind, EntityKind::Boundary);
        assert_eq!(d.relationships[0].from, "Gate");
        assert_eq!(d.relationships[0].to, "Review");
        assert_eq!(
            d.relationships[0].from_decor,
            Some(EndpointDecor::Parenthesis)
        );
        assert_eq!(
            d.relationships[0].to_decor,
            Some(EndpointDecor::Parenthesis)
        );
    }

    #[test]
    fn package_command_precedence_retains_its_own_identity_and_modifiers() {
        let d = parse(
            "package \"Audit Zone\" <<First>> as Alias $blue <<Second>> <<Third>> [[https://example.org/a{Docs}]] #Azure {\nclass Entry\n}\ndatabase Data <<Old>> as \"Storage\" $hot <<Current>> [[https://example.org/db]] {\nclass Row\n}",
        );
        assert_eq!(d.packages[0].name, "Audit Zone");
        assert_eq!(d.packages[0].stereotypes, ["First", "Second", "Third"]);
        assert_eq!(d.entities[0].id, "Audit Zone.Entry");
        assert_eq!(d.packages[0].url_tooltip.as_deref(), Some("Docs"));
        assert_eq!(d.packages[1].name, "Data");
        assert_eq!(d.packages[1].display_name.as_deref(), Some("Storage"));
        assert_eq!(d.packages[1].stereotypes, ["Current"]);
        assert_eq!(d.packages[1].tags, ["hot"]);
    }

    #[test]
    fn chrome_body_and_note_owner_are_independent_of_attachment_target() {
        let d = parse(
            "left header\ncomponent prose\nendheader\nright footer final\nnode Source {\nclass Entry\nnote right of Entry : inside\n}\nnote left of Source.Entry : outside",
        );
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.header_alignment, Some(LegendHorizontalAlignment::Left));
        assert_eq!(d.footer_alignment, Some(LegendHorizontalAlignment::Right));
        assert_eq!(d.meta.header.as_deref(), Some("component prose"));
        assert_eq!(d.notes[0].owner_package.as_deref(), Some("Source"));
        assert_eq!(d.notes[1].owner_package, None);
        assert_eq!(d.notes[0].target, d.notes[1].target);
    }

    #[test]
    fn complete_description_fallback_executes_shared_declarations() {
        let input = "@startuml\ncomponent \"Adapter\" as Port\nINTERFACE\t\"Contract\" as API\nqueue Pending\nPort --> API\n@enduml";
        let crate::diagram::Diagram::Component(d) = super::super::parse(input).unwrap() else {
            panic!("description factory expected")
        };
        assert_eq!(d.components.len(), 2);
        assert_eq!(d.interfaces.len(), 1);
        assert!(d.interfaces.iter().any(|e| e.id == "API"));
        assert!(
            super::super::parse("@startuml\nclass Data\ncomponent Port\nallowmixing\n@enduml")
                .is_err()
        );
    }
    #[test]
    fn named_notes_are_scoped_quarks_and_empty_packages_survive_dispatch() {
        let d = parse("node Yard {\nclass Gate\nnote \"Manual\" as N\nGate .. N\n}");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.notes[0].alias.as_deref(), Some("Yard.N"));
        assert_eq!(d.relationships[0].to, "Yard.N");
        assert_eq!(d.notes[0].owner_package.as_deref(), Some("Yard"));
        let crate::diagram::Diagram::Class(empty) = super::super::parse(
            "@startuml\npackage \"Manual\" as M [[https://example.org/m]] {\n}\n@enduml",
        )
        .unwrap() else {
            panic!("package-only class factory")
        };
        assert!(empty.entities.is_empty());
        assert_eq!(empty.packages[0].name, "M");
        assert_eq!(
            empty.packages[0].url.as_deref(),
            Some("https://example.org/m")
        );
    }

    #[test]
    fn note_modifiers_own_the_body_and_retain_metadata() {
        for (start, end) in [
            (
                "note left of Ledger $audit <<warning>> $export #pink [[https://example.org/ledger{detail}]] {",
                "}",
            ),
            (
                "note bottom of Ledger $audit <<warning>> $export #pink [[https://example.org/ledger{detail}]]",
                "end note",
            ),
        ] {
            let diagram = parse(&format!(
                "record Ledger\n{start}\ncomponent body is opaque\nclass BodyWord\n{end}"
            ));
            assert_eq!(diagram.entities.len(), 1);
            assert_eq!(diagram.entities[0].kind, EntityKind::Record);
            let note = &diagram.notes[0];
            assert_eq!(note.target.as_deref(), Some("Ledger"));
            assert_eq!(note.stereotypes, ["warning"]);
            assert_eq!(note.tags, ["audit", "export"]);
            assert_eq!(note.color.as_deref(), Some("#pink"));
            assert_eq!(note.url.as_deref(), Some("https://example.org/ledger"));
            assert_eq!(note.url_tooltip.as_deref(), Some("detail"));
            assert_eq!(note.lines, ["component body is opaque", "class BodyWord"]);
        }
    }

    #[test]
    fn attached_notes_resolve_scoped_named_note_quarks() {
        for position in ["left", "bottom"] {
            let d = parse(&format!(
                "package Harbor {{\nclass Ledger\nnote \"memo\" as Memo $audit <<warning>> #yellow\nnote {position} of Memo : follow-up\nMemo .. Ledger\n}}"
            ));
            assert_eq!(d.entities.len(), 1);
            assert_eq!(d.notes.len(), 2);
            assert_eq!(d.notes[1].target.as_deref(), Some("Harbor.Memo"));
            assert_eq!(d.notes[0].tags, ["audit"]);
            assert_eq!(d.relationships[0].from, "Harbor.Memo");
        }
    }

    #[test]
    fn note_parent_data_materializes_after_the_triggering_class_leaf() {
        let d = parse("note \"memo\" as Port.Archive\nclass Bolt");
        assert!(matches!(d.uid_events[0], ClassUidEvent::Note(0)));
        assert!(matches!(&d.uid_events[1], ClassUidEvent::Entity(id) if id == "Bolt"));
        assert!(matches!(&d.uid_events[2], ClassUidEvent::Package(id) if id == "Port"));
        assert_eq!(d.packages[0].source_line, 2);
    }

    #[test]
    fn explicit_group_preserves_materialized_location_but_creates_missing_note_parent_data() {
        let existing = parse("class Port.Bolt\npackage Port {\nclass Pin\n}");
        assert_eq!(existing.packages[0].source_line, 1);
        let missing = parse("note \"memo\" as Port.Archive\npackage Port {\nclass Pin\n}");
        assert_eq!(missing.packages[0].source_line, 2);
        assert!(matches!(missing.uid_events[0], ClassUidEvent::Note(0)));
        assert!(matches!(&missing.uid_events[1], ClassUidEvent::Package(id) if id == "Port"));
    }

    #[test]
    fn ribbon_body_counts_leading_and_only_blank_stripes() {
        let d = parse("header\n\nheading\nend header\nright footer\n\n\nend footer\nclass Ledger");
        assert_eq!(d.meta.header.as_deref(), Some("\nheading"));
        assert_eq!(d.meta.footer.as_deref(), Some("\n"));
    }

    #[test]
    fn note_command_trials_reject_unknown_targets_and_trailing_modifier_garbage() {
        for input in [
            "class Ledger\nnote right of Missing : absent",
            "class Ledger\nnote right of Ledger <<hint>> unexpected {\ntext\n}",
        ] {
            let lines = input.lines().map(str::to_string).collect::<Vec<_>>();
            assert!(parse_class(&lines).is_err());
            assert!(!native_container_candidate(&lines));
        }
    }

    #[test]
    fn attached_note_pattern2_target_and_url_share_consuming_grammar() {
        for (left, right) in [('"', '"'), ('“', '”'), ('”', '"')] {
            for (body, has_url) in [
                (" : memo", false),
                ("\nmemo\nend note", true),
                (" {\nmemo\n}", true),
            ] {
                let d = parse(&format!(
                    "skinparam topurl https://example.org/\npackage Bay {{\nclass Dock\nnote right of {left}Dock{right} [[{left}local docs{right}{{read}}]]{body}\n}}"
                ));
                assert_eq!(d.entities.len(), 1);
                assert_eq!(d.notes.len(), 1);
                let note = &d.notes[0];
                assert_eq!(note.target.as_deref(), Some("Bay.Dock"));
                assert_eq!(note.owner_package.as_deref(), Some("Bay"));
                assert_eq!(note.lines, ["memo"]);
                assert_eq!(
                    note.url.as_deref(),
                    has_url.then_some("https://example.org/local docs")
                );
                assert_eq!(note.url_tooltip.as_deref(), has_url.then_some("read"));
                assert!(matches!(d.uid_events.last(), Some(ClassUidEvent::Note(0))));
            }
        }
    }

    #[test]
    fn attached_url_validation_precedes_single_line_null_url_semantics() {
        for command in [
            "note right of Dock [[“”]] : memo",
            "note right of “Dock : memo",
            "note right of \u{e121}Dock\u{e121} : memo",
            "note \"memo\" as Archive [[docs]]",
        ] {
            let input = format!("class Dock\n{command}");
            let lines = input.lines().map(str::to_string).collect::<Vec<_>>();
            assert!(parse_class(&lines).is_err(), "{input}");
        }
    }
}
