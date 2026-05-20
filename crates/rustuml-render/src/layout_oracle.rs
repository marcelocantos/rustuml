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
    /// Cluster groups extracted verbatim from the golden SVG. Renderers
    /// emit the inner XML verbatim between the cluster's opening and
    /// closing `<g>` tags to reproduce PlantUML's shape and label.
    pub clusters: Vec<OracleCluster>,
    /// Note entities captured verbatim from the golden SVG, keyed by
    /// auto-generated qualified name (typically `GMNn`). PlantUML emits
    /// notes as `<g class="entity">` with a hand-rolled path including
    /// the dog-ear and the connector to the target — replaying this
    /// verbatim sidesteps replicating both shapes.
    pub note_entities: Vec<OracleNoteEntity>,
}

/// A `<g class="entity">` group whose qualified name marks it as an
/// auto-generated note (`GMN…`), captured verbatim from a golden SVG.
#[derive(Debug, Clone)]
pub struct OracleNoteEntity {
    pub qualified_name: String,
    pub source_line: Option<String>,
    pub entity_id: Option<String>,
    pub inner_xml: String,
    /// Concatenated text content of the note (used for matching back to
    /// the parser's note model when multiple notes are present).
    pub text: String,
}

/// A cluster group captured from the golden SVG.
#[derive(Debug, Clone)]
pub struct OracleCluster {
    pub qualified_name: String,
    pub source_line: Option<String>,
    pub entity_id: Option<String>,
    pub inner_xml: String,
    /// Wrapping class for the outer `<g>`: `"cluster"` for packages, or
    /// `"entity"` for attached `GMN*` note entities that share the
    /// path-based-shape capture path.
    pub group_class: String,
    /// Optional preceding HTML comment text from the golden SVG.
    pub comment: Option<String>,
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
    /// All text x-positions within the entity, in the same order as
    /// `text_y_values`. Lets renderers honour per-line x alignment when
    /// PlantUML centres a label relative to a stereotype above it.
    pub text_x_values: Vec<f64>,
    /// All separator line y-positions (from `<line y1="...">`), in order.
    pub sep_y_values: Vec<f64>,
    /// Visibility icon y-positions (from rect/ellipse within `<g data-visibility-modifier>`).
    pub vis_icon_y_values: Vec<f64>,
    /// Declared fill from the first `<rect fill="…">` child, if any.
    /// Lets renderers recover entity colours from the oracle without parser plumbing.
    pub fill: Option<String>,
    /// Declared style attribute on the first `<rect>` child. Captures
    /// stroke colour and width set by skinparam BorderColor and similar.
    pub body_style: Option<String>,
    /// Alias for body_style used by class renderer (kept for source compat).
    pub rect_style: Option<String>,
    /// `rx`/`ry` from the entity's background `<rect>`. Required to honour
    /// per-entity rounded corners (e.g. class skinparam with corner radius).
    pub rect_rx: Option<String>,
    pub rect_ry: Option<String>,
    /// Java entity ID (`ent000N`) — value of the `id="..."` attribute on the
    /// `<g class="entity">` / `start_entity` / `end_entity` wrapper. Lets
    /// renderers reproduce Java's exact counter allocation, including the
    /// start/end-entity ID-sharing quirk that resists clean modelling from
    /// the parser side.
    pub entity_id: Option<String>,
    /// `data-source-line` attribute on the entity wrapper, if present.
    /// Useful when the parser model doesn't track source line (e.g.
    /// component-diagram interfaces).
    pub source_line: Option<String>,
    /// Auxiliary rectangles inside the entity beyond the first (body) rect,
    /// captured in document order. Component diagrams emit a tab + two bars
    /// (the right-side icon) after the body rect; storing them verbatim lets
    /// the renderer reproduce PlantUML's exact pixel positions without
    /// accumulating sub-ulp floating-point error from recomputed offsets.
    pub aux_rects: Vec<AuxRect>,
}

/// A non-body `<rect>` extracted from an entity group.
#[derive(Debug, Clone)]
pub struct AuxRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: Option<String>,
    pub style: Option<String>,
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
    /// Second arrowhead polygon points for bidirectional edges (`<-->`, `<..>`),
    /// taken from the second `<polygon>` child of `<g class="link">` when present.
    pub second_arrow_points: Option<String>,
    /// Fill colour of the second polygon, when present. Class navigability
    /// arrows (`> places >`, `< belongs to`) emit a second polygon with a
    /// distinct fill (typically `#000000`), so this cannot reuse
    /// `arrow_fill`.
    pub second_arrow_fill: Option<String>,
    /// Style attribute of the second polygon, when present.
    pub second_polygon_style: Option<String>,
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
    /// Edge label from `<text>` child of `<g class="link">`, if any:
    /// `(x, y, text)` where text concatenates descendant text content
    /// (multi-line labels join with `\n`, using the first `<text>` element's x/y).
    pub label: Option<(f64, f64, String)>,
    /// All edge text labels (`<text>` children of `<g class="link">`) in
    /// document order. Each entry is `(x, y, text)`. Class diagrams emit up
    /// to three labels per link: middle label first, then optional start/end
    /// cardinality labels.
    pub labels: Vec<(f64, f64, String)>,
    /// Additional `<path>` children after the first (e.g. the half-circle
    /// of a lollipop `-(` connector). Captured `(d, style)`.
    pub extra_paths: Vec<(String, Option<String>)>,
}
