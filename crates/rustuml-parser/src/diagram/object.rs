// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Object diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete object diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectDiagram {
    pub meta: DiagramMeta,
    pub objects: Vec<ObjectInstance>,
    pub links: Vec<ObjectLink>,
    pub notes: Vec<ObjectNote>,
    pub packages: Vec<ObjectPackage>,
}

/// The kind of object node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ObjectKind {
    #[default]
    Object,
    Map,
}

/// An object or map instance in an object diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInstance {
    pub id: String,
    pub label: String,
    pub kind: ObjectKind,
    pub fields: Vec<ObjectField>,
    /// Optional stereotype (e.g., "entity", "boundary").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
    /// Optional color override (e.g., "#blue", "#FF0000").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// A note in an object diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectNote {
    /// The note's own identifier (for floating notes declared with `as ID`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The object this note is attached to, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Note text content (may contain `\n` for multi-line).
    pub text: String,
}

/// A single field/entry within an object or map instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectField {
    pub name: String,
    /// The field's value, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// A package or namespace that groups object instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectPackage {
    pub id: String,
    pub label: String,
    pub object_ids: Vec<String>,
}

/// A directed link between object instances (or object fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectLink {
    /// Source: either `"obj_id"` or `"obj_id::field"`.
    pub from: String,
    /// Target: either `"obj_id"` or `"obj_id::field"`.
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Multiplicity label on the source end (e.g. `"1"`, `"0..*"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_multiplicity: Option<String>,
    /// Multiplicity label on the target end (e.g. `"1"`, `"0..*"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_multiplicity: Option<String>,
}
