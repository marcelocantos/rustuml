// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentDiagram {
    pub meta: DiagramMeta,
    pub nodes: Vec<DeploymentNode>,
    pub connections: Vec<DeploymentConnection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<DeploymentNote>,
}

/// A note attached to or near a node, or a floating note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentNote {
    /// Optional ID (for floating notes declared with `as ID`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Node this note is attached to (for `note direction of target`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The note text.
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentNode {
    pub id: String,
    pub label: String,
    pub kind: DeploymentNodeKind,
    pub stereotype: Option<String>,
    pub children: Vec<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DeploymentNodeKind {
    #[default]
    Node,
    Artifact,
    Cloud,
    Database,
    Storage,
    Frame,
    Folder,
    Actor,
    Queue,
    Component,
    Rectangle,
    Agent,
    Boundary,
    Card,
    Collections,
    Control,
    Entity,
    File,
    Package,
    Stack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}
