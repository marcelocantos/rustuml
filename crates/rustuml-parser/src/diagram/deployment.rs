// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

pub const DEFAULT_DEPLOYMENT_LINK_LENGTH: usize = 2;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentDiagram {
    pub meta: DiagramMeta,
    #[serde(default)]
    pub direction: DeploymentLayoutDirection,
    pub nodes: Vec<DeploymentNode>,
    pub connections: Vec<DeploymentConnection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<DeploymentNote>,
}

/// Dot/SVEK rank direction selected by the diagram direction command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeploymentLayoutDirection {
    #[default]
    TopToBottom,
    LeftToRight,
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
    /// Requested side of an attached note.
    #[serde(default)]
    pub position: DeploymentNotePosition,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DeploymentNotePosition {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentNode {
    pub id: String,
    pub label: String,
    pub kind: DeploymentNodeKind,
    pub stereotype: Option<String>,
    /// Explicit background colour from a trailing `#color` token (raw, e.g.
    /// `Pink`, `LightBlue`, or `#FF8888`). The renderer resolves named
    /// colours to hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// The declaration opened a `{ ... }` container, even if every child was
    /// deduplicated against an earlier entity and `children` is empty.
    #[serde(default)]
    pub declared_container: bool,
    pub children: Vec<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
    /// An entity referenced only via a connection (never declared with an
    /// explicit keyword). PlantUML renders these as a bare default circle.
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_label: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub arrow_at_start: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub arrow_at_end: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<DeploymentLinkDirection>,
    #[serde(default, skip_serializing_if = "DeploymentLinkStyle::is_solid")]
    pub style: DeploymentLinkStyle,
    #[serde(
        default = "default_deployment_link_length",
        skip_serializing_if = "deployment_link_length_is_default"
    )]
    pub length: usize,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

fn default_deployment_link_length() -> usize {
    DEFAULT_DEPLOYMENT_LINK_LENGTH
}

fn deployment_link_length_is_default(length: &usize) -> bool {
    *length == DEFAULT_DEPLOYMENT_LINK_LENGTH
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DeploymentLinkStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    Bold,
}

impl DeploymentLinkStyle {
    fn is_solid(&self) -> bool {
        *self == Self::Solid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentLinkDirection {
    Down,
    Up,
    Left,
    Right,
}
