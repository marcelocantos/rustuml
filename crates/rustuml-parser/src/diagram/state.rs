// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete state diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct StateDiagram {
    pub meta: DiagramMeta,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub notes: Vec<StateNote>,
}

/// A note attached to a state or floating freely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateNote {
    /// The text content of the note (may be multi-line).
    pub text: String,
    /// Where the note is positioned relative to its anchor.
    pub kind: StateNoteKind,
}

/// Where the note is anchored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateNoteKind {
    /// `note left of <state> : text`
    LeftOf(String),
    /// `note right of <state> : text`
    RightOf(String),
    /// `note "..." as <alias>` — free-floating note
    Floating,
    /// `note on link` — attached to the most recent transition
    OnLink,
}

/// A state in a state diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub label: String,
    pub kind: StateKind,
    pub descriptions: Vec<String>,
    pub substates: Vec<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
    /// `state X #color` — explicit fill from the source. The renderer
    /// prefers oracle-captured colour over this, but it lets the no-
    /// oracle path produce colourful diagrams too.
    #[serde(default)]
    pub fill: Option<String>,
    /// `state X ##color` / `##[dashed]color` — explicit border colour
    /// and optional dash style.
    #[serde(default)]
    pub stroke: Option<String>,
    /// Dash style hint (`bold`, `dashed`, `dotted`) parsed from `##[…]color`.
    #[serde(default)]
    pub stroke_style: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StateKind {
    #[default]
    Normal,
    Initial,
    Final,
    Choice,
    Fork,
    Join,
    History,
    DeepHistory,
}

/// A transition between states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}
