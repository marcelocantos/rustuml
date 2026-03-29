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
    pub notes: Vec<ComponentNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub label: String,
    /// All stereotypes (e.g. `["facade", "service"]`).
    pub stereotypes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub from_mult: Option<String>,
    pub to_mult: Option<String>,
    pub dashed: bool,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPackage {
    pub name: String,
    pub label: String,
    pub stereotype: Option<String>,
    pub components: Vec<String>,
    pub packages: Vec<ComponentPackage>,
}

/// A note attached to a component or floating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentNote {
    /// Text content (may be multi-line with `\n`).
    pub text: String,
    /// The id of the element this note is attached to, if any.
    pub target: Option<String>,
}
