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
}

/// A class, interface, enum, or other entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEntity {
    pub id: String,
    pub label: String,
    pub kind: EntityKind,
    pub members: Vec<Member>,
    pub stereotypes: Vec<String>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Field,
    Method,
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
}
