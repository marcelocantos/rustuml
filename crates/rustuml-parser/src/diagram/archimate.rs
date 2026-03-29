// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Archimate diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchimateDiagram {
    pub meta: DiagramMeta,
    pub elements: Vec<ArchimateElement>,
    pub relations: Vec<ArchimateRelation>,
    pub groups: Vec<ArchimateGroup>,
}

/// An Archimate element (e.g. `Business_Actor`, `Technology_Node`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchimateElement {
    pub id: String,
    pub label: String,
    /// The Archimate layer (Business, Application, Technology, Motivation, Implementation).
    pub layer: ArchimateLayer,
    /// The element kind within the layer (Actor, Process, Component, Node, etc.).
    pub kind: String,
}

/// Archimate layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchimateLayer {
    Business,
    Application,
    Technology,
    Motivation,
    Implementation,
    /// Fallback for unknown layers.
    Other,
}

impl ArchimateLayer {
    /// Default fill colour for each layer.
    pub fn default_color(&self) -> &'static str {
        match self {
            ArchimateLayer::Business => "#FFFFB5",
            ArchimateLayer::Application => "#B5FFFF",
            ArchimateLayer::Technology => "#C9E7B7",
            ArchimateLayer::Motivation => "#CCCCFF",
            ArchimateLayer::Implementation => "#FFE0C0",
            ArchimateLayer::Other => "#DDDDDD",
        }
    }
}

impl std::str::FromStr for ArchimateLayer {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Business" => Ok(ArchimateLayer::Business),
            "Application" => Ok(ArchimateLayer::Application),
            "Technology" => Ok(ArchimateLayer::Technology),
            "Motivation" => Ok(ArchimateLayer::Motivation),
            "Implementation" => Ok(ArchimateLayer::Implementation),
            _ => Ok(ArchimateLayer::Other),
        }
    }
}

/// A relation between two Archimate elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchimateRelation {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: ArchimateRelationKind,
}

/// Archimate relation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchimateRelationKind {
    Association,
    Composition,
    Aggregation,
    Serving,
    Realization,
    Triggering,
    Access,
    Influence,
    Assignment,
    /// Fallback.
    Other,
}

impl ArchimateRelationKind {
    pub fn from_str_prefix(s: &str) -> Self {
        let base = s
            .strip_suffix("_Up")
            .or_else(|| s.strip_suffix("_Down"))
            .or_else(|| s.strip_suffix("_Left"))
            .or_else(|| s.strip_suffix("_Right"))
            .unwrap_or(s);
        match base {
            "Association" => ArchimateRelationKind::Association,
            "Composition" => ArchimateRelationKind::Composition,
            "Aggregation" => ArchimateRelationKind::Aggregation,
            "Serving" => ArchimateRelationKind::Serving,
            "Realization" => ArchimateRelationKind::Realization,
            "Triggering" => ArchimateRelationKind::Triggering,
            s if s.starts_with("Access") => ArchimateRelationKind::Access,
            "Influence" => ArchimateRelationKind::Influence,
            "Assignment" => ArchimateRelationKind::Assignment,
            _ => ArchimateRelationKind::Other,
        }
    }
}

/// A grouping rectangle (from `rectangle "name" { ... }` blocks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchimateGroup {
    pub label: String,
    pub element_ids: Vec<String>,
}
