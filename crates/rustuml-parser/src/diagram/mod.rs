// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram models — the typed ASTs produced by parsing.
//!
//! All types derive `Serialize`/`Deserialize` so diagrams can be
//! specified in YAML or JSON as an alternative to PlantUML text syntax.

pub mod activity;
pub mod archimate;
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
    Archimate(archimate::ArchimateDiagram),
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

impl Diagram {
    /// Return a reference to the diagram's metadata.
    pub fn meta(&self) -> &DiagramMeta {
        match self {
            Diagram::Sequence(d) => &d.meta,
            Diagram::Class(d) => &d.meta,
            Diagram::Archimate(d) => &d.meta,
            Diagram::Object(d) => &d.meta,
            Diagram::State(d) => &d.meta,
            Diagram::Activity(d) => &d.meta,
            Diagram::Component(d) => &d.meta,
            Diagram::UseCase(d) => &d.meta,
            Diagram::Deployment(d) => &d.meta,
            Diagram::Nwdiag(d) => &d.meta,
            Diagram::Json(d) => &d.meta,
            Diagram::MindMap(d) => &d.meta,
            Diagram::Gantt(d) => &d.meta,
            Diagram::Git(d) => &d.meta,
            Diagram::Timing(d) => &d.meta,
            Diagram::Wbs(d) => &d.meta,
            Diagram::Math(d) => &d.meta,
            Diagram::Salt(d) => &d.meta,
            Diagram::Regex(d) => &d.meta,
            Diagram::Ditaa(d) => &d.meta,
            Diagram::Dot(d) => &d.meta,
            Diagram::Board(d) => &d.meta,
            Diagram::Ebnf(d) => &d.meta,
        }
    }

    /// Return a mutable reference to the diagram's metadata.
    pub fn meta_mut(&mut self) -> &mut DiagramMeta {
        match self {
            Diagram::Sequence(d) => &mut d.meta,
            Diagram::Class(d) => &mut d.meta,
            Diagram::Archimate(d) => &mut d.meta,
            Diagram::Object(d) => &mut d.meta,
            Diagram::State(d) => &mut d.meta,
            Diagram::Activity(d) => &mut d.meta,
            Diagram::Component(d) => &mut d.meta,
            Diagram::UseCase(d) => &mut d.meta,
            Diagram::Deployment(d) => &mut d.meta,
            Diagram::Nwdiag(d) => &mut d.meta,
            Diagram::Json(d) => &mut d.meta,
            Diagram::MindMap(d) => &mut d.meta,
            Diagram::Gantt(d) => &mut d.meta,
            Diagram::Git(d) => &mut d.meta,
            Diagram::Timing(d) => &mut d.meta,
            Diagram::Wbs(d) => &mut d.meta,
            Diagram::Math(d) => &mut d.meta,
            Diagram::Salt(d) => &mut d.meta,
            Diagram::Regex(d) => &mut d.meta,
            Diagram::Ditaa(d) => &mut d.meta,
            Diagram::Dot(d) => &mut d.meta,
            Diagram::Board(d) => &mut d.meta,
            Diagram::Ebnf(d) => &mut d.meta,
        }
    }
}

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

/// Common metadata that any diagram can carry.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
    /// The preprocessed PlantUML source — every non-empty line of the post-
    /// preprocessor diagram body. Used to compute the seed for deterministic
    /// SVG ids (filter UIDs, gradient UIDs, shadow UIDs) that match Java
    /// PlantUML's byte-for-byte output. Joined with `\n` and a trailing
    /// `\n`, this string is the input to `StringUtils.seed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A skinparam key-value pair (e.g., `skinparam backgroundColor #FFFFFF`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinParam {
    pub key: String,
    pub value: String,
}
