// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram models — the typed ASTs produced by parsing.
//!
//! All types derive `Serialize`/`Deserialize` so diagrams can be
//! specified in YAML or JSON as an alternative to PlantUML text syntax.

pub mod activity;
pub mod class;
pub mod component;
pub mod sequence;
pub mod state;
pub mod usecase;

use serde::{Deserialize, Serialize};

/// A parsed diagram, ready for layout and rendering.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "diagram")]
pub enum Diagram {
    Sequence(sequence::SequenceDiagram),
    Class(class::ClassDiagram),
    State(state::StateDiagram),
    Activity(activity::ActivityDiagram),
    Component(component::ComponentDiagram),
    UseCase(usecase::UseCaseDiagram),
}

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

/// Common metadata that any diagram can carry.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DiagramMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legend: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skinparams: Vec<SkinParam>,
}

/// A skinparam key-value pair (e.g., `skinparam backgroundColor #FFFFFF`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinParam {
    pub key: String,
    pub value: String,
}
