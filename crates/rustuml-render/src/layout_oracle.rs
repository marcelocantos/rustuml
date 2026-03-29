// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Layout oracle — pre-computed layout data extracted from reference SVGs.
//!
//! Instead of running our Graphviz layout engine, renderers can accept an
//! `OracleLayout` containing entity positions and edge paths extracted from
//! a PlantUML reference SVG. This decouples layout correctness from rendering
//! correctness in golden tests.

use std::collections::HashMap;

/// Pre-computed layout data from a reference SVG.
#[derive(Debug, Clone, Default)]
pub struct OracleLayout {
    /// Entity positions keyed by qualified name (from `data-qualified-name`).
    /// Values are (x, y, width, height) of the entity's outer `<rect>`.
    pub entities: HashMap<String, EntityRect>,
    /// Edge paths keyed by "from-to-target" format (from link `<path>` id).
    pub edges: Vec<OracleEdgePath>,
    /// Canvas dimensions from the root `<svg>` element.
    pub canvas_width: f64,
    pub canvas_height: f64,
}

/// Position and size of an entity extracted from a golden SVG.
#[derive(Debug, Clone, Copy)]
pub struct EntityRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// An edge path extracted from a golden SVG.
#[derive(Debug, Clone)]
pub struct OracleEdgePath {
    /// The path id (e.g. "A-to-B" or "A-backto-B").
    pub id: String,
    /// The SVG path `d` attribute.
    pub d: String,
    /// Arrowhead polygon points (if present).
    pub arrow_points: Option<String>,
}
