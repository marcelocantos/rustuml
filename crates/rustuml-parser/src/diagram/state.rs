// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram model.

use super::{DiagramMeta, SkinParam};
use serde::{Deserialize, Serialize};

/// A complete state diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct StateDiagram {
    pub meta: DiagramMeta,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub notes: Vec<StateNote>,
}

const TRANSITION_COLOR_KEY: &str = "__stateTransitionColor";
const TRANSITION_LINE_STYLE_KEY: &str = "__stateTransitionLineStyle";
const TRANSITION_THICKNESS_KEY: &str = "__stateTransitionThickness";

impl StateDiagram {
    /// Return the style parsed for a transition at its source-order index.
    pub fn transition_style(&self, index: usize) -> TransitionStyle {
        let value = |prefix: &str| {
            let key = format!("{prefix}{index}");
            self.meta
                .skinparams
                .iter()
                .rev()
                .find(|param| param.key == key)
                .map(|param| param.value.as_str())
        };
        TransitionStyle {
            color: value(TRANSITION_COLOR_KEY).map(str::to_string),
            line_style: value(TRANSITION_LINE_STYLE_KEY).and_then(TransitionLineStyle::parse),
            thickness: value(TRANSITION_THICKNESS_KEY).and_then(|value| value.parse().ok()),
        }
    }
}

/// Style carried by a state transition's bracketed arrow specification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransitionStyle {
    pub color: Option<String>,
    pub line_style: Option<TransitionLineStyle>,
    pub thickness: Option<f64>,
}

impl TransitionStyle {
    /// Store parser-only transition metadata without widening `Transition`,
    /// whose public struct literals are also used by the ASCII renderer.
    pub(crate) fn record_in(&self, meta: &mut DiagramMeta, index: usize) {
        let mut record = |prefix: &str, value: String| {
            meta.skinparams.push(SkinParam {
                key: format!("{prefix}{index}"),
                value,
            });
        };
        if let Some(color) = &self.color {
            record(TRANSITION_COLOR_KEY, color.clone());
        }
        if let Some(line_style) = self.line_style {
            record(TRANSITION_LINE_STYLE_KEY, line_style.as_str().to_string());
        }
        if let Some(thickness) = self.thickness {
            record(TRANSITION_THICKNESS_KEY, thickness.to_string());
        }
    }
}

/// Stroke pattern selected by `dashed`, `dotted`, or `bold` arrow styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionLineStyle {
    Dashed,
    Dotted,
    Bold,
}

impl TransitionLineStyle {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "dashed" => Some(Self::Dashed),
            "dotted" => Some(Self::Dotted),
            "bold" => Some(Self::Bold),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Dashed => "dashed",
            Self::Dotted => "dotted",
            Self::Bold => "bold",
        }
    }
}

/// A note attached to a state or floating freely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateNote {
    /// The text content of the note (may be multi-line).
    pub text: String,
    /// Where the note is positioned relative to its anchor.
    pub kind: StateNoteKind,
    /// 1-based line number within the parsed diagram body.
    #[serde(default)]
    pub source_line: usize,
    /// 1-based line containing the opening note command. Multi-line notes use
    /// the first body line for `source_line`, matching PlantUML's entity
    /// metadata, while UID allocation still follows this command location.
    #[serde(default)]
    pub command_line: usize,
    /// Zero-based order in which note commands created their model entities.
    #[serde(default)]
    pub creation_order: usize,
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
    /// `note [position] on link` — attached to the most recent transition.
    OnLink {
        /// Index of the transition owned by this note.
        transition_index: usize,
        /// Placement relative to the transition's ordinary label.
        position: StateNotePosition,
    },
}

/// Placement of a note composed into a transition label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateNotePosition {
    Left,
    Right,
    Top,
    Bottom,
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
    /// Ordinary UML stereotype (`state A <<important>>`). Pseudo-state
    /// stereotypes such as `<<choice>>` are represented by `kind` instead.
    #[serde(default)]
    pub stereotype: Option<String>,
    /// True when this state opened a composite block (`state X { … }`),
    /// i.e. it contains nested states. Rendered as a cluster.
    #[serde(default)]
    pub composite: bool,
    /// Separator used between this composite's concurrent regions.
    ///
    /// PlantUML records the first character of `--` / `||` on the owning
    /// state group and lets `ConcurrentStates.Separator` choose whether the
    /// independently-laid-out region images stack vertically or horizontally.
    #[serde(default)]
    pub concurrent_separator: Option<char>,
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
    /// 1-based line of the first explicit `state X` declaration or
    /// `X : description` field line, if any. `None` when the state was only
    /// discovered as a transition endpoint. PlantUML registers explicitly
    /// declared/described states in the entity factory before it materialises
    /// the `[*]` start/end pseudo-states (which are created lazily during link
    /// resolution), so this line — when present — orders the state ahead of the
    /// pseudo-states in the rendered emission sequence.
    #[serde(default)]
    pub decl_line: Option<usize>,
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
