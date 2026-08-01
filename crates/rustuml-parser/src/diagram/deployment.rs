// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;
use super::style::PlantUmlColors;

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
    /// Explicit note background color, retained as a PlantUML color token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Dollar-prefixed tags from the shared named-note command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Chevron stereotype from the shared named-note command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
    /// Immediate containing DESCRIPTION group, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Stable ordinal assigned when the note's quark is first registered.
    #[serde(default)]
    pub quark_order: usize,
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
    /// The exact `CommandPackageWithUSymbol` symbol selected by a braced
    /// container declaration. This remains separate from `kind`, which is the
    /// renderer's currently supported shape projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_symbol: Option<DeploymentContainerSymbol>,
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
    /// Stable ordinal assigned when this identifier's quark is first registered.
    #[serde(default)]
    pub quark_order: usize,
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

/// Complete symbol inventory accepted by Java
/// `CommandPackageWithUSymbol#getRegexConcat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentContainerSymbol {
    Package,
    Rectangle,
    Hexagon,
    Node,
    Artifact,
    Folder,
    File,
    Frame,
    Cloud,
    Action,
    Process,
    Database,
    Storage,
    Component,
    Card,
    Queue,
    Stack,
}

impl DeploymentContainerSymbol {
    pub const COMMAND_SYMBOLS: [(&'static str, Self); 17] = [
        ("package", Self::Package),
        ("rectangle", Self::Rectangle),
        ("hexagon", Self::Hexagon),
        ("node", Self::Node),
        ("artifact", Self::Artifact),
        ("folder", Self::Folder),
        ("file", Self::File),
        ("frame", Self::Frame),
        ("cloud", Self::Cloud),
        ("action", Self::Action),
        ("process", Self::Process),
        ("database", Self::Database),
        ("storage", Self::Storage),
        ("component", Self::Component),
        ("card", Self::Card),
        ("queue", Self::Queue),
        ("stack", Self::Stack),
    ];

    pub fn from_command_keyword(keyword: &str) -> Option<Self> {
        Self::COMMAND_SYMBOLS
            .iter()
            .find_map(|(candidate, symbol)| {
                keyword.eq_ignore_ascii_case(candidate).then_some(*symbol)
            })
    }

    /// Existing renderer projection. Action, process, and hexagon stay
    /// distinguishable in `DeploymentNode::container_symbol`; choosing their
    /// concrete painting belongs to a renderer mechanism.
    pub fn renderer_kind(self) -> DeploymentNodeKind {
        match self {
            Self::Package => DeploymentNodeKind::Package,
            Self::Rectangle => DeploymentNodeKind::Rectangle,
            Self::Hexagon | Self::Action | Self::Process => DeploymentNodeKind::Node,
            Self::Node => DeploymentNodeKind::Node,
            Self::Artifact => DeploymentNodeKind::Artifact,
            Self::Folder => DeploymentNodeKind::Folder,
            Self::File => DeploymentNodeKind::File,
            Self::Frame => DeploymentNodeKind::Frame,
            Self::Cloud => DeploymentNodeKind::Cloud,
            Self::Database => DeploymentNodeKind::Database,
            Self::Storage => DeploymentNodeKind::Storage,
            Self::Component => DeploymentNodeKind::Component,
            Self::Card => DeploymentNodeKind::Card,
            Self::Queue => DeploymentNodeKind::Queue,
            Self::Stack => DeploymentNodeKind::Stack,
        }
    }
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
    /// A `note on link` value owned by this connection. Java stores this on
    /// `Link`, so it has no independent quark, source line, or UID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<DeploymentLinkNote>,
    /// Paint-hidden link style: retained for solving and UID order, not painted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    #[serde(
        default = "default_deployment_link_length",
        skip_serializing_if = "deployment_link_length_is_default"
    )]
    pub length: usize,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentLinkNote {
    pub text: String,
    #[serde(default, skip_serializing_if = "PlantUmlColors::is_empty")]
    pub colors: PlantUmlColors,
    pub position: DeploymentNotePosition,
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
