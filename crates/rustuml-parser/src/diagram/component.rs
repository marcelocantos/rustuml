// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentDiagram {
    pub meta: DiagramMeta,
    pub components: Vec<Component>,
    pub interfaces: Vec<Interface>,
    pub connections: Vec<Connection>,
    pub packages: Vec<ComponentPackage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub together: Vec<ComponentTogether>,
    pub notes: Vec<ComponentNote>,
}

/// An invisible `together { ... }` layout subgraph.
///
/// PlantUML attaches the current `Together` object to each entity created
/// directly inside the block. A group opened directly inside another together
/// block retains that parent relation; entering a visible package suspends it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTogether {
    /// Index of the directly enclosing together block, when nested.
    pub parent: Option<usize>,
    /// Qualified package that owns this subgraph, or `None` for the graph root.
    pub package: Option<String>,
    /// Leaf entity ids created directly inside this block.
    pub nodes: Vec<String>,
    /// Qualified child-package ids created directly inside this block.
    pub packages: Vec<String>,
}

/// The shape an element renders as. A plain `component` draws the UML tab
/// icon; other leaf declarations draw their PlantUML DESCRIPTION shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComponentElementKind {
    #[default]
    Component,
    Actor,
    Artifact,
    Collections,
    Database,
    Node,
    Queue,
    /// A leaf `cloud "X" as Y` element — drawn as a bumpy cloud outline rather
    /// than the rounded component body. (Container clouds, `cloud X { … }`,
    /// become packages/clusters and never reach this enum.)
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub label: String,
    /// All stereotypes (e.g. `["facade", "service"]`).
    pub stereotypes: Vec<String>,
    /// Optional element-specific fill color from declarations such as
    /// `component Gateway #LightBlue`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
    /// Element shape (component tab vs database cylinder vs queue).
    #[serde(default)]
    pub kind: ComponentElementKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub id: String,
    pub label: String,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub from_mult: Option<String>,
    pub to_mult: Option<String>,
    pub dashed: bool,
    #[serde(default)]
    pub has_arrow: bool,
    #[serde(default)]
    pub arrow_at_start: bool,
    #[serde(default)]
    pub arrow_at_end: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<ConnectionDirection>,
    /// PlantUML `Link.getLength()`, derived from the arrow-body character
    /// count after horizontal arrows are normalized to one.
    #[serde(default = "default_connection_length")]
    pub length: usize,
    #[serde(default)]
    pub shape: LinkShape,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

const fn default_connection_length() -> usize {
    2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionDirection {
    Down,
    Up,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LinkShape {
    #[default]
    Plain,
    TargetSocket,
    TargetBallSocket,
    MiddleBallSocket,
    MiddleFullSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPackage {
    pub name: String,
    pub label: String,
    pub stereotype: Option<String>,
    #[serde(default)]
    pub kind: ComponentPackageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub source_line: usize,
    pub components: Vec<String>,
    pub packages: Vec<ComponentPackage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComponentPackageKind {
    Cloud,
    Component,
    Database,
    Folder,
    Frame,
    Node,
    Package,
    Queue,
    #[default]
    Rectangle,
}

/// A note attached to a component or floating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentNote {
    /// Text content (may be multi-line with `\n`).
    pub text: String,
    /// The id of the element this note is attached to, if any.
    pub target: Option<String>,
    /// Requested side of an attached note.
    #[serde(default)]
    pub position: ComponentNotePosition,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComponentNotePosition {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}
