// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct UseCaseDiagram {
    pub meta: DiagramMeta,
    pub actors: Vec<Actor>,
    pub use_cases: Vec<UseCase>,
    pub connections: Vec<UseCaseConnection>,
    pub packages: Vec<UseCasePackage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<UseCaseNote>,
}

/// An inline note attached to a diagram element or floating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCaseNote {
    pub text: String,
    /// The element id this note is attached to (None for floating notes).
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub label: String,
    /// Optional UML stereotype text, e.g. `<<system>>` → `"system"`.
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    pub id: String,
    pub label: String,
    /// Optional UML stereotype text, e.g. `<<automated>>` → `"automated"`.
    pub stereotype: Option<String>,
    /// Optional additional description lines (from multiline `as "Title\n--\n..."` syntax).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCaseConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCasePackage {
    pub name: String,
    pub elements: Vec<String>,
}
