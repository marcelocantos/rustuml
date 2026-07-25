// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UseCaseDiagram {
    pub meta: DiagramMeta,
    #[serde(default)]
    pub direction: UseCaseLayoutDirection,
    pub actors: Vec<Actor>,
    pub use_cases: Vec<UseCase>,
    pub connections: Vec<UseCaseConnection>,
    pub packages: Vec<UseCasePackage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<UseCaseNote>,
}

/// Dot/SVEK rank direction selected by `left to right direction` or
/// `top to bottom direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UseCaseLayoutDirection {
    #[default]
    TopToBottom,
    LeftToRight,
}

/// A note position as declared in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseCaseNotePosition {
    Right,
    Left,
    Top,
    Bottom,
}

/// The SVEK owner of a note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseCaseNoteKind {
    /// An auto-named `GMN*` entity joined to a diagram entity by an opale link.
    Attached { target: String },
    /// A normal note entity declared with an explicit id.
    Floating { id: String },
    /// A note component owned by an existing connection.
    OnLink { connection: usize },
}

/// A note attached to an entity/link or declared as a floating entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCaseNote {
    pub text: String,
    pub kind: UseCaseNoteKind,
    pub position: UseCaseNotePosition,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub label: String,
    /// Optional UML stereotype text, e.g. `<<system>>` → `"system"`.
    pub stereotype: Option<String>,
    /// Optional inline background colour token, e.g. `#Pink`, `#AAFFAA`.
    /// Stored without the leading `#`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    pub id: String,
    pub label: String,
    /// True when the source supplied an explicit id/alias (`usecase ID as
    /// "Label"` or `usecase "Label" as ID`). PlantUML uses the explicit id as
    /// `data-qualified-name`; implicit quoted labels keep the visible label.
    #[serde(default, skip_serializing_if = "is_false")]
    pub explicit_id: bool,
    /// Optional UML stereotype text, e.g. `<<automated>>` → `"automated"`.
    pub stereotype: Option<String>,
    /// Optional additional description lines (from multiline `as "Title\n--\n..."` syntax).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    /// Optional inline background colour token, e.g. `#Cyan`, `#AAFFAA`.
    /// Stored without the leading `#`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCaseConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub stereotype: Option<String>,
    /// True for dotted relations such as `..>` / `.>`; PlantUML renders these
    /// with a DESCRIPTION dependency dash pattern.
    #[serde(default, skip_serializing_if = "is_false")]
    pub dashed: bool,
    /// True when the relation token carries an arrow head (`>`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub arrow: bool,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// The grouping keyword used to open a use-case container, which selects its
/// drawn shape: `package` renders a folder-tab outline, `rectangle` a plain
/// rounded rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PackageKind {
    #[default]
    Package,
    Rectangle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCasePackage {
    pub name: String,
    pub elements: Vec<String>,
    /// Optional inline background colour token, e.g. `#Yellow`, `#AAFFAA`.
    /// Stored without the leading `#`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Which grouping keyword opened the container (`package` vs `rectangle`).
    #[serde(default)]
    pub kind: PackageKind,
    /// 1-based line number of the package/rectangle opening within the block.
    #[serde(default)]
    pub source_line: usize,
}
