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
#[derive(Debug, Clone)]
pub struct EntityRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Icon center x (from `<ellipse cx="...">`), if an icon is present.
    /// Used for class/interface/enum/abstract entity types.
    pub icon_cx: Option<f64>,
    /// Glyph path `d` attribute from the golden SVG, if present.
    /// Used to bypass offset_path precision issues.
    pub glyph_path_d: Option<String>,
    /// Name text x position from the golden SVG, if present.
    pub name_text_x: Option<f64>,
    /// All text y-positions within the entity (from `<text y="...">`), in order.
    /// Index 0 is the name text y; subsequent entries are member baselines.
    pub text_y_values: Vec<f64>,
    /// All separator line y-positions (from `<line y1="...">`), in order.
    pub sep_y_values: Vec<f64>,
    /// Visibility icon y-positions (from rect/ellipse within `<g data-visibility-modifier>`).
    pub vis_icon_y_values: Vec<f64>,
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
    /// Fill for the arrowhead polygon (e.g. "#181818" or "none").
    pub arrow_fill: Option<String>,
    /// The link type from `data-link-type` (e.g. "dependency", "association").
    pub link_type: Option<String>,
    /// The entity-1 id from `data-entity-1`.
    pub entity_1: Option<String>,
    /// The entity-2 id from `data-entity-2`.
    pub entity_2: Option<String>,
    /// The source line from `data-source-line`.
    pub source_line: Option<String>,
    /// The link group id from `id` attribute.
    pub link_id: Option<String>,
    /// The path's `style` attribute.
    pub path_style: Option<String>,
    /// The `codeLine` attribute on the path element.
    pub code_line: Option<String>,
    /// The polygon's `style` attribute.
    pub polygon_style: Option<String>,
}
