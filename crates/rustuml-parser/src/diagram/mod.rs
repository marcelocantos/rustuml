// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram models — the typed ASTs produced by parsing.
//!
//! All types derive `Serialize`/`Deserialize` so diagrams can be
//! specified in YAML or JSON as an alternative to PlantUML text syntax.

pub mod activity;
pub mod board;
pub mod class;
pub mod component;
pub mod deployment;
pub mod ditaa;
pub mod dot;
pub mod ebnf;
pub mod gantt;
pub mod git_diagram;
pub mod json_diagram;
pub mod math;
pub mod mindmap;
pub mod nwdiag;
pub mod object;
pub mod regex_diagram;
pub mod salt;
pub mod sequence;
pub mod state;
pub mod timing;
pub mod usecase;
pub mod wbs;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Pixel data for a single sprite definition.
///
/// Each row is a sequence of hex-digit characters (0–F) where 0 is fully
/// transparent and F is fully opaque white.  The `[WxH/Z]` header is
/// optional in the source; when absent the dimensions are inferred from
/// the pixel rows themselves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteData {
    /// Width in pixels (inferred from the first row if not explicit).
    pub width: u32,
    /// Height in pixels (number of rows).
    pub height: u32,
    /// Raw pixel rows as written in the source (hex digits 0–F).
    pub rows: Vec<String>,
}

/// A parsed diagram, ready for layout and rendering.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "diagram")]
pub enum Diagram {
    Sequence(sequence::SequenceDiagram),
    Class(class::ClassDiagram),
    Object(object::ObjectDiagram),
    State(state::StateDiagram),
    Activity(activity::ActivityDiagram),
    Component(component::ComponentDiagram),
    UseCase(usecase::UseCaseDiagram),
    Deployment(deployment::DeploymentDiagram),
    Nwdiag(nwdiag::NwdiagDiagram),
    Json(json_diagram::JsonDiagram),
    MindMap(mindmap::MindMapDiagram),
    Gantt(gantt::GanttDiagram),
    Git(git_diagram::GitDiagram),
    Timing(timing::TimingDiagram),
    Wbs(wbs::WbsDiagram),
    Math(math::MathDiagram),
    Salt(salt::SaltDiagram),
    Regex(regex_diagram::RegexDiagram),
    Ditaa(ditaa::DitaaDiagram),
    Dot(dot::DotDiagram),
    Board(board::BoardDiagram),
    Ebnf(ebnf::EbnfDiagram),
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
    /// Sprite definitions collected from the source (`sprite $name { ... }`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub sprites: HashMap<String, SpriteData>,
}

/// A skinparam key-value pair (e.g., `skinparam backgroundColor #FFFFFF`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinParam {
    pub key: String,
    pub value: String,
}
