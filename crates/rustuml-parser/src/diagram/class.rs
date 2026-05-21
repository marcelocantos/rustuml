// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// Position of a note relative to its target entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotePosition {
    Top,
    Bottom,
    Left,
    Right,
}

/// A note attached to an entity or floating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Note content lines (may contain Creole/HTML markup).
    pub lines: Vec<String>,
    /// Entity this note is attached to, if any.
    pub target: Option<String>,
    /// Position relative to target entity.
    pub position: Option<NotePosition>,
    /// Named note alias (for `note "..." as N`).
    pub alias: Option<String>,
}

/// A complete class diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClassDiagram {
    pub meta: DiagramMeta,
    pub entities: Vec<ClassEntity>,
    pub relationships: Vec<Relationship>,
    pub packages: Vec<Package>,
    pub notes: Vec<Note>,
    /// Visibility-control directives accumulated from `hide ...` / `show ...`
    /// statements. Stored as the raw argument list after the keyword so the
    /// renderer can interpret per-entity selectors as well as global ones.
    #[serde(default)]
    pub hide_show: Vec<HideShow>,
    /// 1-based source line of the `header`/`footer`/`title`/`caption`/`legend`
    /// declaration, when present. Used to populate `data-source-line` on the
    /// `<g class="header">`-style decoration wrappers PlantUML emits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_line: Option<usize>,
}

/// One `hide`/`show` directive (verbatim arguments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HideShow {
    pub show: bool,
    /// Lower-cased space-collapsed argument text (e.g. `"circle"`, `"empty members"`,
    /// `"<<myStereo>> circle"`, `"myClass attributes"`).
    pub arg: String,
}

/// A class, interface, enum, or other entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEntity {
    pub id: String,
    pub label: String,
    pub kind: EntityKind,
    pub members: Vec<Member>,
    pub stereotypes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional background color (e.g., "#lightblue", "#FF0000").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Optional text colour from `text:colour` shorthand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// The kind of entity in a class diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EntityKind {
    #[default]
    Class,
    AbstractClass,
    Interface,
    Enum,
    Annotation,
    Entity,
}

/// A field or method in a class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub name: String,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_abstract: bool,
    pub kind: MemberKind,
    /// Verbatim display text (after stripping visibility prefix and modifiers).
    /// Preserves original colon spacing, e.g. "field: String" or "field : String".
    pub display_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Visibility {
    #[default]
    Default,
    Public,
    Private,
    Protected,
    Package,
    /// IE (entity-relationship) mandatory column, denoted by the `*` prefix
    /// in PlantUML's entity-syntax diagrams.
    IeMandatory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Field,
    Method,
    /// A labeled separator line within a class body (e.g. `-- Section --`, `== Title ==`).
    Separator,
}

/// A relationship (association, inheritance, etc.) between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub kind: RelationshipKind,
    pub label: Option<String>,
    pub from_multiplicity: Option<String>,
    pub to_multiplicity: Option<String>,
    /// Whether the line is dashed (e.g. `..>` vs `-->`).
    #[serde(default)]
    pub dashed: bool,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    Inheritance,
    Implementation,
    Composition,
    Aggregation,
    Association,
    Dependency,
}

/// Container type for grouping entities in a class diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PackageKind {
    #[default]
    Package,
    Namespace,
    Cloud,
    Database,
    Folder,
    Frame,
    Rectangle,
    Node,
}

/// A package/namespace/container grouping entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub kind: PackageKind,
    /// Optional background color (CSS name or hex without leading `#`).
    pub color: Option<String>,
    pub entities: Vec<String>,
    /// Stereotypes applied to this package (e.g. `<<Application>>`).
    #[serde(default)]
    pub stereotypes: Vec<String>,
    /// Display label override (used for auto-created namespace packages where `name`
    /// is the full qualified path but we only want to show the short last segment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}
