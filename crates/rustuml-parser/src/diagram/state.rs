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
    /// `note "..." as <alias>` — free-floating note. Carries the explicit
    /// alias (`as FN1`) when present so the renderer can pair it with the
    /// oracle entity of the same qualified name.
    Floating(Option<String>),
    /// `note on link` — attached to the most recent transition
    OnLink,
}

/// A state in a state diagram.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// True when this state opened a composite block (`state X { … }`),
    /// i.e. it contains nested states. Rendered as a cluster.
    #[serde(default)]
    pub composite: bool,
    /// Qualified id of the immediately-enclosing composite state, if this
    /// state is nested inside one. `None` for top-level states.
    #[serde(default)]
    pub parent: Option<String>,
    /// `state X [[url]]` — hyperlink target. Rendered as an `<a>` wrapper
    /// around the entity body.
    #[serde(default)]
    pub url: Option<String>,
    /// `state X [[url{tooltip}]]` — optional tooltip text for the hyperlink.
    #[serde(default)]
    pub tooltip: Option<String>,
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
    /// `<<entryPoint>>` — a small ellipse drawn on the composite boundary
    /// marking an entry connection point into the composite state.
    EntryPoint,
    /// `<<exitPoint>>` — like an entry point but marked with an X cross.
    ExitPoint,
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
