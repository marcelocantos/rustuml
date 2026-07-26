// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! JSON/YAML visualization SVG renderer.
//!
//! PlantUML emits JSON/YAML diagrams as `data-diagram-type="JSON"` (or
//! `"YAML"`) SVGs. A flat object/array is drawn as a single rounded box: a
//! `#F1F1F1` fill rect, then per row a bold key `<text>`, a plain value
//! `<text>`, a vertical `<line>` separating the key and value columns, and a
//! horizontal `<line>` separating consecutive rows. A `fill="none"` border
//! rect closes the box.
//!
//! Nested objects/arrays are laid out by PlantUML via its Smetana (graphviz)
//! engine as detached boxes joined by dashed bezier connectors. The no-oracle
//! path builds that record-node graph locally; the strict path can still
//! consume captured box positions and connector splines (see `render_nested`).

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph, RecordLayoutMetrics};
use rustuml_parser::diagram::json_diagram::{DataFormat, JsonDiagram, JsonNode, JsonNodeValue};

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics::{ascent, fmt_coord, text_height, text_width};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── PlantUML JSON layout constants ───────────────────────────────────────────

const FONT_SIZE: f64 = 14.0;
/// Outer margin from the SVG edge to the box.
const MARGIN: f64 = 10.0;
/// Horizontal padding either side of text within a column (5px each side).
const CELL_PAD: f64 = 5.0;
/// Extra vertical space added to the text height to form a row.
const ROW_EXTRA: f64 = 4.0;
/// `SmetanaForJson.createNode` removes Graphviz's four-point record margin
/// from both sides before encoding each text span in its `_dim_` label.
const RECORD_SPAN_HORIZONTAL_MARGIN: f64 = 8.0;
/// Baseline offset of text below the row top (above the ascent).
const TEXT_TOP_PAD: f64 = 2.0;
/// Corner radius of the rounded box.
const RX: f64 = 5.0;

const FILL: &str = "#F1F1F1";
const BORDER: &str = "#000000";

// ── Public entry points ───────────────────────────────────────────────────────

/// Render a JSON/YAML diagram to SVG (no oracle).
pub fn render(diagram: &JsonDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a JSON/YAML diagram. For a flat single box the renderer computes all
/// geometry from PlantUML-compatible font metrics. For nested structures it
/// uses oracle geometry when supplied and otherwise builds PlantUML's Smetana
/// record-node model locally.
pub fn render_with_oracle(
    diagram: &JsonDiagram,
    _theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    let diagram_type = match diagram.format {
        DataFormat::Json => "JSON",
        DataFormat::Yaml => "YAML",
    };

    // A flat single box is only possible when every value is a scalar
    // (a nested object/array spawns a detached box via the Smetana layout).
    if let Some(rows) = flat_rows(&diagram.root, diagram.format) {
        return render_single_box(&rows, diagram_type);
    }

    // Nested: place locally-computed boxes at oracle positions, then emit the
    // captured connector geometry.
    if let Some(oracle) = oracle
        && !oracle.json_boxes.is_empty()
        && let Some(svg) = render_nested(diagram, oracle, diagram_type)
    {
        return svg;
    }

    if let Some(svg) = render_nested_no_oracle(diagram, diagram_type) {
        return svg;
    }

    render_fallback(diagram, diagram_type)
}

// ── Nested (multi-box) rendering ──────────────────────────────────────────────

/// A box to draw: either a content box (rows) or an empty placeholder box.
struct BoxSpec {
    rows: Vec<FlatRow>,
}

struct LayoutBoxSpec {
    id: String,
    rows: Vec<FlatRow>,
    is_array: bool,
    parent: Option<String>,
    parent_port: Option<usize>,
}

fn postorder_edges(specs: &[LayoutBoxSpec]) -> Vec<usize> {
    fn visit(parent: &str, specs: &[LayoutBoxSpec], out: &mut Vec<usize>) {
        for (index, spec) in specs.iter().enumerate() {
            if spec.parent.as_deref() == Some(parent) {
                visit(&spec.id, specs, out);
                out.push(index);
            }
        }
    }

    let mut result = Vec::new();
    if let Some(root) = specs.first() {
        visit(&root.id, specs, &mut result);
    }
    result
}

/// Walk the data tree in PlantUML's box-emission order (DFS pre-order: a node,
/// then each of its nested children depth-first in field/item order) collecting
/// one `BoxSpec` per box. The traversal order matches PlantUML's `<rect>`
/// document order, verified against `json_array_of_objects.svg` and
/// `json_api_response.svg`.
fn collect_boxes(node: &JsonNode, format: DataFormat, out: &mut Vec<BoxSpec>) {
    // Build this node's own box rows (nested children show as the nbsp×3
    // placeholder; scalars show their display string).
    let children: Vec<&JsonNode> = match &node.value {
        JsonNodeValue::Object { fields } => fields.iter().collect(),
        JsonNodeValue::Array { items } => items.iter().collect(),
        _ => Vec::new(),
    };
    let is_object = matches!(node.value, JsonNodeValue::Object { .. });
    let mut rows = Vec::with_capacity(children.len());
    for child in &children {
        let value = scalar_display(&child.value, format).unwrap_or_else(nested_placeholder);
        rows.push(FlatRow {
            key: if is_object {
                child.key.clone().unwrap_or_default()
            } else {
                String::new()
            },
            value,
            highlighted: child.highlighted,
        });
    }
    out.push(BoxSpec { rows });

    // Recurse into nested children depth-first, in order.
    for child in children {
        if matches!(
            child.value,
            JsonNodeValue::Object { .. } | JsonNodeValue::Array { .. }
        ) {
            collect_boxes(child, format, out);
        }
    }
}

/// PlantUML renders any nested value (object or array, empty or not) as three
/// non-breaking spaces in the parent row.
fn nested_placeholder() -> String {
    "\u{00a0}\u{00a0}\u{00a0}".to_string()
}

fn render_nested(
    diagram: &JsonDiagram,
    oracle: &OracleLayout,
    diagram_type: &str,
) -> Option<String> {
    let mut specs = Vec::new();
    collect_boxes(&diagram.root, diagram.format, &mut specs);

    // Each spec must line up with a captured box position (document order).
    if specs.len() != oracle.json_boxes.len() {
        return None;
    }

    let mut body = String::new();
    for (spec, geom) in specs.iter().zip(oracle.json_boxes.iter()) {
        body.push_str(&render_box_at(&spec.rows, geom));
    }
    for conn in &oracle.json_connectors {
        body.push_str(&conn.curve);
        if let Some(a) = &conn.arrowhead {
            body.push_str(a);
        }
        if let Some(d) = &conn.dot {
            body.push_str(d);
        }
    }

    Some(wrap_oracle_envelope(oracle, &body, diagram_type))
}

fn render_nested_no_oracle(diagram: &JsonDiagram, diagram_type: &str) -> Option<String> {
    let mut specs = Vec::new();
    collect_layout_boxes(&diagram.root, diagram.format, None, None, &mut specs);
    if specs.len() <= 1 {
        return None;
    }

    // PlantUML routes nested JSON/YAML boxes through its Smetana/Graphviz path:
    // measured boxes become graph nodes and parent-child placeholders become
    // dashed connector edges. This mirrors that graph-construction step with
    // the vendored Graphviz wrapper instead of replaying oracle connector SVG.
    let metrics: Vec<BoxLayoutMetrics> = specs
        .iter()
        .map(|spec| box_layout_metrics(&spec.rows))
        .collect();
    // PlantUML `SmetanaForJson.initGraph` submits the graph without the SVEK
    // spacing overrides. Its `createNode` uses record nodes and `createEdge`
    // binds each child to the parent row's P{index} port.
    let mut graph = LayoutGraph::new(Direction::TopToBottom);
    for (spec, metrics) in specs.iter().zip(&metrics) {
        let ports = (0..spec.rows.len())
            .map(|index| format!("P{index}"))
            .collect::<Vec<_>>();
        // `SmetanaForJson.createNode` serializes the measured dimensions
        // without rounding and deliberately swaps width and height before
        // layout; `getPosition` and `JsonCurve` swap the solved axes back.
        if metrics.row_heights.is_empty() {
            if spec.is_array {
                graph.add_record_node(&spec.id, metrics.height, metrics.graph_width, &ports);
            } else {
                graph.add_keyed_record_node(&spec.id, metrics.height, metrics.graph_width, &ports);
            }
        } else {
            let record_metrics = RecordLayoutMetrics {
                row_heights: metrics.row_heights.clone(),
                key_width: (!spec.is_array)
                    .then_some(metrics.key_width - RECORD_SPAN_HORIZONTAL_MARGIN),
                value_width: metrics.value_width - RECORD_SPAN_HORIZONTAL_MARGIN,
            };
            graph.add_dimensional_record_node(
                &spec.id,
                metrics.height,
                metrics.graph_width,
                &ports,
                record_metrics,
            );
        }
    }
    for index in postorder_edges(&specs) {
        let spec = &specs[index];
        if let (Some(parent), Some(parent_port)) = (&spec.parent, spec.parent_port) {
            let port = format!("P{parent_port}");
            graph.add_edge_with_ports(parent, &spec.id, None, Some(&port), None);
        }
    }
    // `LayoutGraph` serializes Graphviz FFI internally. A per-call timeout
    // starts before that lock is acquired, so a parallel golden census can
    // time out valid JSON trees merely while they wait in the queue.
    let mut layout = graph.layout_full_no_timeout();
    for position in &mut layout.node_positions {
        // Java `SmetanaForJson.getPosition` swaps the solved Graphviz axes
        // without quantizing the point-valued coordinates.
        (position.x, position.y) = (position.y, position.x);
        (position.width, position.height) = (position.height, position.width);
    }
    for edge in &mut layout.edge_paths {
        for point in &mut edge.points {
            *point = (point.1, point.0);
        }
        if let Some(point) = &mut edge.start_point {
            *point = (point.1, point.0);
        }
        if let Some(point) = &mut edge.end_point {
            *point = (point.1, point.0);
        }
    }
    let edge_order = postorder_edges(&specs);
    layout.edge_paths.sort_by_key(|edge| {
        edge_order
            .iter()
            .position(|index| {
                let target = &specs[*index];
                target.id == edge.to && target.parent.as_deref() == Some(edge.from.as_str())
            })
            .unwrap_or(usize::MAX)
    });

    let mut body = String::new();
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for (spec, pos) in specs.iter().zip(&layout.node_positions) {
        let x = pos.x + MARGIN;
        let y = pos.y + MARGIN;
        let (width, height) = box_dimensions(&spec.rows);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
        body.push_str(&render_box_rows_at(&spec.rows, x, y));
    }
    for edge in &layout.edge_paths {
        let rendered = render_nested_connector(edge, MARGIN, MARGIN);
        if let Some((x, y)) = edge.end_point {
            max_x = max_x.max(x + MARGIN);
            max_y = max_y.max(y + MARGIN);
        }
        body.push_str(&rendered);
    }

    let mut svg = SvgBuilder::new_plantuml(
        max_x.ceil() + MARGIN + 1.0,
        max_y.ceil() + MARGIN + 1.0,
        diagram_type,
    );
    svg.raw_inline(&body);
    Some(svg.finalize_plantuml())
}

fn collect_layout_boxes(
    node: &JsonNode,
    format: DataFormat,
    parent: Option<String>,
    parent_port: Option<usize>,
    out: &mut Vec<LayoutBoxSpec>,
) {
    let id = format!("json{}", out.len());
    let children: Vec<&JsonNode> = match &node.value {
        JsonNodeValue::Object { fields } => fields.iter().collect(),
        JsonNodeValue::Array { items } => items.iter().collect(),
        _ => Vec::new(),
    };
    let is_object = matches!(node.value, JsonNodeValue::Object { .. });
    let mut rows = Vec::with_capacity(children.len());
    for child in &children {
        let value = scalar_display(&child.value, format).unwrap_or_else(nested_placeholder);
        rows.push(FlatRow {
            key: if is_object {
                child.key.clone().unwrap_or_default()
            } else {
                String::new()
            },
            value,
            highlighted: child.highlighted,
        });
    }
    out.push(LayoutBoxSpec {
        id: id.clone(),
        rows,
        is_array: matches!(node.value, JsonNodeValue::Array { .. }),
        parent,
        parent_port,
    });

    for (index, child) in children.into_iter().enumerate() {
        if matches!(
            child.value,
            JsonNodeValue::Object { .. } | JsonNodeValue::Array { .. }
        ) {
            collect_layout_boxes(child, format, Some(id.clone()), Some(index), out);
        }
    }
}

/// Render one box at the oracle position, reproducing PlantUML's row layout.
/// Column widths and text content are computed locally; the box position, the
/// per-row text baselines and the separator-line y's are consumed from the
/// oracle (PlantUML derives them from a sub-pixel internal coordinate the
/// 4-dp `<rect y>` can't reproduce). An empty object/array (no rows) is the
/// 30×15 placeholder box.
fn render_box_at(rows: &[FlatRow], geom: &crate::layout_oracle::JsonBox) -> String {
    let box_x = geom.x;
    let box_y = geom.y;
    let mut out = String::new();

    if rows.is_empty() {
        // Empty object/array: a 30×15 rounded box (fill rect + border rect).
        out.push_str(&rounded_rect(box_x, box_y, 30.0, 15.0, FILL, FILL, 1.5));
        out.push_str(&rounded_rect(box_x, box_y, 30.0, 15.0, "none", BORDER, 1.5));
        return out;
    }

    let has_keys = rows.iter().any(|r| !r.key.is_empty());

    let key_text_w = rows
        .iter()
        .map(|r| text_width(&r.key, FONT_SIZE, true))
        .fold(0.0_f64, f64::max);
    let val_text_w = rows
        .iter()
        .map(|r| text_width(&r.value, FONT_SIZE, false))
        .fold(0.0_f64, f64::max);

    let key_col_w = if has_keys {
        key_text_w + 2.0 * CELL_PAD
    } else {
        0.0
    };
    let val_col_w = val_text_w + 2.0 * CELL_PAD;
    let box_w = key_col_w + val_col_w;

    let row_h = text_height(FONT_SIZE) + ROW_EXTRA;
    let box_h = row_h * rows.len() as f64;

    let box_right = box_x + box_w;
    let val_col_x = box_x + key_col_w;

    // Consume the oracle's exact y coordinates. Fall back to computed values
    // when (defensively) the captured counts don't line up.
    let baseline_of = |i: usize, row_top: f64| {
        geom.text_ys
            .get(i)
            .copied()
            .unwrap_or(row_top + TEXT_TOP_PAD + ascent(FONT_SIZE))
    };
    let mut line_iter = geom.line_ys.iter();

    out.push_str(&rounded_rect(box_x, box_y, box_w, box_h, FILL, FILL, 1.5));

    let highlight_rect = |y: f64| highlight_box(box_x, y, box_w, row_h);
    if rows[0].highlighted {
        out.push_str(&highlight_rect(box_y));
    }

    let mut row_top = box_y;
    for (i, row) in rows.iter().enumerate() {
        let baseline = baseline_of(i, row_top);
        let row_bottom = row_top + row_h;

        if has_keys && !row.key.is_empty() {
            out.push_str(&key_text(
                box_x + CELL_PAD,
                baseline,
                &row.key,
                text_width(&row.key, FONT_SIZE, true),
            ));
        }

        out.push_str(&value_text(
            val_col_x + CELL_PAD,
            baseline,
            &row.value,
            text_width(&row.value, FONT_SIZE, false),
        ));

        // Vertical key/value separator (per row). Emit using the oracle's y's
        // when available so rounded box-relative coordinates match exactly.
        if has_keys {
            match line_iter.next() {
                Some((y1, y2)) => out.push_str(&line_raw_y(val_col_x, val_col_x, y1, y2)),
                None => out.push_str(&line(val_col_x, row_top, val_col_x, row_bottom)),
            }
        }

        // Horizontal separator below this row (except after the last).
        if i + 1 < rows.len() {
            if rows[i + 1].highlighted {
                out.push_str(&highlight_rect(row_bottom));
            }
            match line_iter.next() {
                Some((y1, y2)) => out.push_str(&line_raw_y(box_x, box_right, y1, y2)),
                None => out.push_str(&line(box_x, row_bottom, box_right, row_bottom)),
            }
        }

        row_top = row_bottom;
    }

    out.push_str(&rounded_rect(
        box_x, box_y, box_w, box_h, "none", BORDER, 1.5,
    ));
    out
}

fn box_dimensions(rows: &[FlatRow]) -> (f64, f64) {
    let metrics = box_layout_metrics(rows);
    (metrics.width, metrics.height)
}

struct BoxLayoutMetrics {
    width: f64,
    height: f64,
    graph_width: f64,
    key_width: f64,
    value_width: f64,
    row_heights: Vec<f64>,
}

fn box_layout_metrics(rows: &[FlatRow]) -> BoxLayoutMetrics {
    if rows.is_empty() {
        return BoxLayoutMetrics {
            width: 30.0,
            height: 15.0,
            // Java `TextBlockJson.calculateDimensionSlow` reports zero width
            // for an empty collection. `drawU` alone expands the painted box
            // to `MIN_WIDTH`, after `SmetanaForJson.createNode` has consumed
            // the zero-width layout dimension.
            graph_width: 0.0,
            key_width: 0.0,
            value_width: 30.0,
            row_heights: Vec::new(),
        };
    }
    let has_keys = rows.iter().any(|r| !r.key.is_empty());
    let key_text_w = rows
        .iter()
        .map(|r| text_width(&r.key, FONT_SIZE, true))
        .fold(0.0_f64, f64::max);
    let val_text_w = rows
        .iter()
        .map(|r| text_width(&r.value, FONT_SIZE, false))
        .fold(0.0_f64, f64::max);
    let key_col_w = if has_keys {
        key_text_w + 2.0 * CELL_PAD
    } else {
        0.0
    };
    let val_col_w = val_text_w + 2.0 * CELL_PAD;
    let row_h = text_height(FONT_SIZE) + ROW_EXTRA;
    let row_heights = vec![row_h; rows.len()];
    BoxLayoutMetrics {
        width: key_col_w + val_col_w,
        height: row_heights.iter().sum(),
        graph_width: key_col_w + val_col_w,
        key_width: key_col_w,
        value_width: val_col_w,
        row_heights,
    }
}

fn render_box_rows_at(rows: &[FlatRow], box_x: f64, box_y: f64) -> String {
    if rows.is_empty() {
        let mut out = String::new();
        out.push_str(&rounded_rect(box_x, box_y, 30.0, 15.0, FILL, FILL, 1.5));
        out.push_str(&rounded_rect(box_x, box_y, 30.0, 15.0, "none", BORDER, 1.5));
        return out;
    }

    let has_keys = rows.iter().any(|r| !r.key.is_empty());
    let (box_w, box_h) = box_dimensions(rows);
    let key_text_w = rows
        .iter()
        .map(|r| text_width(&r.key, FONT_SIZE, true))
        .fold(0.0_f64, f64::max);
    let key_col_w = if has_keys {
        key_text_w + 2.0 * CELL_PAD
    } else {
        0.0
    };
    let val_col_x = box_x + key_col_w;
    let row_h = text_height(FONT_SIZE) + ROW_EXTRA;
    let box_right = box_x + box_w;
    let mut out = String::new();

    out.push_str(&rounded_rect(box_x, box_y, box_w, box_h, FILL, FILL, 1.5));
    let highlight_rect = |y: f64| highlight_box(box_x, y, box_w, row_h);
    if rows[0].highlighted {
        out.push_str(&highlight_rect(box_y));
    }
    let mut row_top = box_y;
    for (i, row) in rows.iter().enumerate() {
        let baseline = row_top + TEXT_TOP_PAD + ascent(FONT_SIZE);
        let row_bottom = row_top + row_h;
        if has_keys && !row.key.is_empty() {
            out.push_str(&key_text(
                box_x + CELL_PAD,
                baseline,
                &row.key,
                text_width(&row.key, FONT_SIZE, true),
            ));
        }
        out.push_str(&value_text(
            val_col_x + CELL_PAD,
            baseline,
            &row.value,
            text_width(&row.value, FONT_SIZE, false),
        ));
        if has_keys {
            out.push_str(&line(val_col_x, row_top, val_col_x, row_bottom));
        }
        if i + 1 < rows.len() {
            if rows[i + 1].highlighted {
                out.push_str(&highlight_rect(row_bottom));
            }
            out.push_str(&line(box_x, row_bottom, box_right, row_bottom));
        }
        row_top = row_bottom;
    }
    out.push_str(&rounded_rect(
        box_x, box_y, box_w, box_h, "none", BORDER, 1.5,
    ));
    out
}

fn render_nested_connector(edge: &EdgePath, dx: f64, dy: f64) -> String {
    // `SmetanaForJson.createEdge` asks dot for a normal .75-size arrow.
    // `JsonCurve` then starts 13px behind the first spline point, draws a
    // straight lead-in, the clipped cubic, the arrow, and finally a 3px spot.
    const LEAD_IN: f64 = 13.0;
    let mut out = String::new();
    if edge.points.len() >= 4 {
        // Java `SmetanaForJson.createEdge` delegates the control polygon to
        // Graphviz and `JsonCurve.drawCurve` consumes it verbatim. Preserve
        // the vendored router's points here; rebuilding a direct cubic from
        // visible box bounds loses Graphviz's sub-point record-port geometry.
        let mut points = edge.points.clone();
        let last = points.len() - 1;
        let arrow_tip = points[last];

        // Dot clips its last cubic at the arrow base. Reproduce that operation
        // with the same de Casteljau subdivision as Smetana.
        let segment_start = last - 3;
        let clipped = clip_cubic_for_smetana_arrow([
            points[segment_start],
            points[segment_start + 1],
            points[segment_start + 2],
            points[segment_start + 3],
        ]);
        points[segment_start..=last].copy_from_slice(&clipped);

        let lead = extend_back(points[0], points[1], LEAD_IN);
        let mut d = format!("M{},{}", fmt_coord(lead.0 + dx), fmt_coord(lead.1 + dy));
        write!(
            d,
            " L{},{}",
            fmt_coord(points[0].0 + dx),
            fmt_coord(points[0].1 + dy)
        )
        .unwrap();
        let mut i = 1;
        while i + 2 < points.len() {
            write!(
                d,
                " C{},{} {},{} {},{}",
                fmt_coord(points[i].0 + dx),
                fmt_coord(points[i].1 + dy),
                fmt_coord(points[i + 1].0 + dx),
                fmt_coord(points[i + 1].1 + dy),
                fmt_coord(points[i + 2].0 + dx),
                fmt_coord(points[i + 2].1 + dy),
            )
            .unwrap();
            i += 3;
        }
        out.push_str(&format!(
            r#"<path d="{d}" fill="none" style="stroke:#000000;stroke-width:1;stroke-dasharray:3,3;"/>"#
        ));
        out.push_str(&json_arrow_path(
            (points[last].0 + dx, points[last].1 + dy),
            (arrow_tip.0 + dx, arrow_tip.1 + dy),
        ));
        out.push_str(&format!(
            r##"<ellipse cx="{}" cy="{}" fill="#000000" rx="3" ry="3" style="stroke:#000000;stroke-width:1;"/>"##,
            fmt_coord(lead.0 + dx),
            fmt_coord(lead.1 + dy),
        ));
    }
    out
}

fn unit_vector(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = dx.hypot(dy);
    if length == 0.0 {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}

fn extend_back(center: (f64, f64), direction: (f64, f64), length: f64) -> (f64, f64) {
    let unit = unit_vector(direction, center);
    (center.0 + unit.0 * length, center.1 + unit.1 * length)
}

fn cubic_point(points: [(f64, f64); 4], t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (
        points
            .iter()
            .zip(weights)
            .map(|(point, weight)| point.0 * weight)
            .sum(),
        points
            .iter()
            .zip(weights)
            .map(|(point, weight)| point.1 * weight)
            .sum(),
    )
}

fn clip_cubic_for_smetana_arrow(points: [(f64, f64); 4]) -> [(f64, f64); 4] {
    // `SmetanaForJson.createEdge` selects a normal arrow at .75 scale.
    // `arrows__c.arrow_length` therefore clips at 10 * .75 points, while
    // `splines__c.bezier_clip` keeps the last outside subdivision and stops
    // once consecutive probe coordinates differ by at most .5 points.
    const ARROW_LENGTH: f64 = 10.0 * 0.75;
    const PROBE_TOLERANCE: f64 = 0.5;
    const MAX_CLIP_STEPS: usize = 64;

    let end = points[3];
    let mut low = 0.0;
    let mut high = 1.0;
    let mut previous = end;
    let mut latest = points;
    let mut best = None;
    for _ in 0..MAX_CLIP_STEPS {
        let middle = (low + high) / 2.0;
        let point = cubic_point(points, middle);
        latest = split_cubic_left(points, middle);
        if (end.0 - point.0).hypot(end.1 - point.1) <= ARROW_LENGTH {
            high = middle;
        } else {
            low = middle;
            best = Some(latest);
        }
        if (previous.0 - point.0).abs() <= PROBE_TOLERANCE
            && (previous.1 - point.1).abs() <= PROBE_TOLERANCE
        {
            break;
        }
        previous = point;
    }
    best.unwrap_or(latest)
}

fn split_cubic_left(points: [(f64, f64); 4], t: f64) -> [(f64, f64); 4] {
    let lerp = |a: (f64, f64), b: (f64, f64)| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
    let p01 = lerp(points[0], points[1]);
    let p12 = lerp(points[1], points[2]);
    let p23 = lerp(points[2], points[3]);
    let p012 = lerp(p01, p12);
    let p123 = lerp(p12, p23);
    [points[0], p01, p012, lerp(p012, p123)]
}

fn json_arrow_path(base: (f64, f64), tip: (f64, f64)) -> String {
    // Java `jsondiagram.Arrow.drawArrow`.
    let dx = tip.0 - base.0;
    let dy = tip.1 - base.1;
    let distance = dx.hypot(dy);
    let along = unit_vector(base, tip);
    let perpendicular = (-along.1, along.0);
    let factor = distance * 0.4;
    let notch = (
        base.0 + along.0 * distance * 0.3,
        base.1 + along.1 * distance * 0.3,
    );
    let lower = (
        base.0 + perpendicular.0 * factor,
        base.1 + perpendicular.1 * factor,
    );
    let upper = (
        base.0 - perpendicular.0 * factor,
        base.1 - perpendicular.1 * factor,
    );
    let d = format!(
        "M{},{} L{},{} L{},{} L{},{} L{},{}",
        fmt_coord(lower.0),
        fmt_coord(lower.1),
        fmt_coord(notch.0),
        fmt_coord(notch.1),
        fmt_coord(upper.0),
        fmt_coord(upper.1),
        fmt_coord(tip.0),
        fmt_coord(tip.1),
        fmt_coord(lower.0),
        fmt_coord(lower.1),
    );
    format!(r##"<path d="{d}" fill="#000000"/>"##)
}

// ── Single-box (flat) rendering ───────────────────────────────────────────────

struct FlatRow {
    /// Key text (empty for array items).
    key: String,
    /// Display text for the value (already in PlantUML display form).
    value: String,
    highlighted: bool,
}

/// If `node` is an object or array whose every value is a scalar, return its
/// rows. Otherwise `None` (nested children require the Smetana layout).
fn flat_rows(node: &JsonNode, format: DataFormat) -> Option<Vec<FlatRow>> {
    match &node.value {
        JsonNodeValue::Object { fields } if fields.is_empty() => Some(vec![FlatRow {
            key: String::new(),
            value: "\u{00a0}".to_string(),
            highlighted: node.highlighted,
        }]),
        JsonNodeValue::Object { fields } if !fields.is_empty() => {
            let mut rows = Vec::with_capacity(fields.len());
            for f in fields {
                let value = scalar_display(&f.value, format)?;
                rows.push(FlatRow {
                    key: f.key.clone().unwrap_or_default(),
                    value,
                    highlighted: f.highlighted,
                });
            }
            Some(rows)
        }
        JsonNodeValue::Array { items } if items.is_empty() => Some(vec![FlatRow {
            key: String::new(),
            value: "\u{00a0}".to_string(),
            highlighted: node.highlighted,
        }]),
        JsonNodeValue::Array { items } if !items.is_empty() => {
            let mut rows = Vec::with_capacity(items.len());
            for item in items {
                let value = scalar_display(&item.value, format)?;
                rows.push(FlatRow {
                    key: String::new(),
                    value,
                    highlighted: item.highlighted,
                });
            }
            Some(rows)
        }
        _ => None,
    }
}

/// PlantUML display string for a scalar value, or `None` for nested
/// objects/arrays (including empty ones, which PlantUML draws as detached
/// boxes connected by a dashed link).
fn scalar_display(v: &JsonNodeValue, format: DataFormat) -> Option<String> {
    match v {
        // YAML renders bool/null as bare text; JSON uses checkbox/null glyphs.
        JsonNodeValue::Null => Some(match format {
            DataFormat::Json => "\u{2400}".to_string(),
            DataFormat::Yaml => "null".to_string(),
        }),
        JsonNodeValue::Bool { val } => Some(match format {
            DataFormat::Json => {
                if *val {
                    "\u{2611} true".to_string()
                } else {
                    "\u{2610} false".to_string()
                }
            }
            DataFormat::Yaml => if *val { "true" } else { "false" }.to_string(),
        }),
        JsonNodeValue::Number { val } => Some(val.clone()),
        JsonNodeValue::Str { val } => {
            if val.is_empty() {
                // PlantUML renders an empty string as a single non-breaking space.
                Some("\u{00a0}".to_string())
            } else {
                Some(val.clone())
            }
        }
        // Nested (including empty) structures are not flat.
        JsonNodeValue::Array { .. } | JsonNodeValue::Object { .. } => None,
    }
}

fn render_single_box(rows: &[FlatRow], diagram_type: &str) -> String {
    let has_keys = rows.iter().any(|r| !r.key.is_empty());

    // Column widths: 5px padding either side of the widest text in each column.
    let key_text_w = rows
        .iter()
        .map(|r| text_width(&r.key, FONT_SIZE, true))
        .fold(0.0_f64, f64::max);
    let val_text_w = rows
        .iter()
        .map(|r| text_width(&r.value, FONT_SIZE, false))
        .fold(0.0_f64, f64::max);

    let key_col_w = if has_keys {
        key_text_w + 2.0 * CELL_PAD
    } else {
        0.0
    };
    let val_col_w = val_text_w + 2.0 * CELL_PAD;
    let box_w = key_col_w + val_col_w;

    let row_h = text_height(FONT_SIZE) + ROW_EXTRA;
    let box_h = row_h * rows.len() as f64;

    let box_x = MARGIN;
    let box_y = MARGIN;
    let box_right = box_x + box_w;
    let val_col_x = box_x + key_col_w;

    // Canvas dimensions: box plus the outer margin, rounded up and padded by
    // one pixel to match PlantUML's reported integer px in the root <svg>.
    let canvas_w = (box_right + MARGIN).ceil() + 1.0;
    let canvas_h = (box_y + box_h + MARGIN).ceil() + 1.0;

    let mut svg = SvgBuilder::new_plantuml(canvas_w, canvas_h, diagram_type);

    // Background fill rect (stroke matches fill so only the fill shows).
    svg.raw_inline(&rounded_rect(box_x, box_y, box_w, box_h, FILL, FILL, 1.5));

    // The highlight rect for a row is emitted just before that row's top edge:
    // immediately after the fill for the first row, otherwise just before the
    // horizontal separator above the row.
    let highlight_rect = |y: f64| highlight_box(box_x, y, box_w, row_h);
    if rows[0].highlighted {
        svg.raw_inline(&highlight_rect(box_y));
    }

    let mut row_top = box_y;
    for (i, row) in rows.iter().enumerate() {
        let baseline = row_top + TEXT_TOP_PAD + ascent(FONT_SIZE);
        let row_bottom = row_top + row_h;

        if has_keys && !row.key.is_empty() {
            svg.raw_inline(&key_text(
                box_x + CELL_PAD,
                baseline,
                &row.key,
                text_width(&row.key, FONT_SIZE, true),
            ));
        }

        svg.raw_inline(&value_text(
            val_col_x + CELL_PAD,
            baseline,
            &row.value,
            text_width(&row.value, FONT_SIZE, false),
        ));

        // Vertical separator at the key/value column boundary (per row).
        if has_keys {
            svg.raw_inline(&line(val_col_x, row_top, val_col_x, row_bottom));
        }

        // Horizontal separator below this row, except after the last. The next
        // row's highlight rect (if any) is drawn immediately before it.
        if i + 1 < rows.len() {
            if rows[i + 1].highlighted {
                svg.raw_inline(&highlight_rect(row_bottom));
            }
            svg.raw_inline(&line(box_x, row_bottom, box_right, row_bottom));
        }

        row_top = row_bottom;
    }

    // Border rect.
    svg.raw_inline(&rounded_rect(
        box_x, box_y, box_w, box_h, "none", BORDER, 1.5,
    ));

    svg.finalize_plantuml()
}

// ── Element emitters (PlantUML attribute order) ───────────────────────────────

/// Highlight rect drawn behind a `#highlight`-ed row: inset 1.5px from the box
/// sides, `rx`/`ry` 2, fill and stroke both `#CCFF02`.
fn highlight_box(box_x: f64, row_top: f64, box_w: f64, row_h: f64) -> String {
    format!(
        r##"<rect fill="#CCFF02" height="{h}" rx="2" ry="2" style="stroke:#CCFF02;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"##,
        h = fmt_coord(row_h),
        w = fmt_coord(box_w - 2.0),
        x = fmt_coord(box_x + 1.5),
        y = fmt_coord(row_top),
    )
}

fn rounded_rect(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: f64) -> String {
    format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX}" ry="{RX}" style="stroke:{stroke};stroke-width:{sw};" width="{w}" x="{x}" y="{y}"/>"#,
        h = fmt_coord(h),
        w = fmt_coord(w),
        x = fmt_coord(x),
        y = fmt_coord(y),
    )
}

fn key_text(x: f64, y: f64, content: &str, text_len: f64) -> String {
    format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{c}</text>"##,
        tl = fmt_coord(text_len),
        x = fmt_coord(x),
        y = fmt_coord(y),
        c = escape_text(content),
    )
}

fn value_text(x: f64, y: f64, content: &str, text_len: f64) -> String {
    format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="14" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{c}</text>"##,
        tl = fmt_coord(text_len),
        x = fmt_coord(x),
        y = fmt_coord(y),
        c = escape_text(content),
    )
}

/// Like `line`, but with the y endpoints supplied verbatim (the oracle's
/// sub-pixel-rounded strings) while the x endpoints are formatted locally.
fn line_raw_y(x1: f64, x2: f64, y1: &str, y2: &str) -> String {
    format!(
        r#"<line style="stroke:#000000;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"/>"#,
        x1 = fmt_coord(x1),
        x2 = fmt_coord(x2),
    )
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    format!(
        r#"<line style="stroke:#000000;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"/>"#,
        x1 = fmt_coord(x1),
        x2 = fmt_coord(x2),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    )
}

/// Escape text for SVG, matching PlantUML's numeric-entity encoding for
/// non-breaking spaces and the special JSON glyphs.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\u{00a0}' => out.push_str("&#160;"),
            '\u{2400}' => out.push_str("&#9216;"),
            '\u{2610}' => out.push_str("&#9744;"),
            '\u{2611}' => out.push_str("&#9745;"),
            _ => out.push(c),
        }
    }
    out
}

// ── Fallback rendering (nested structures, errors) ────────────────────────────

/// Best-effort render for diagrams the single-box path can't handle (nested
/// objects/arrays). Emits a PlantUML envelope so output is well-formed. These
/// cases require the Smetana layout for exact parity and are expected to remain
/// failing until that is implemented.
fn render_fallback(diagram: &JsonDiagram, diagram_type: &str) -> String {
    let rows: Vec<FlatRow> = match &diagram.root.value {
        JsonNodeValue::Object { fields } => fields
            .iter()
            .map(|f| FlatRow {
                key: f.key.clone().unwrap_or_default(),
                value: scalar_display(&f.value, diagram.format)
                    .unwrap_or_else(|| fallback_value(&f.value)),
                highlighted: f.highlighted,
            })
            .collect(),
        JsonNodeValue::Array { items } => items
            .iter()
            .map(|item| FlatRow {
                key: String::new(),
                value: scalar_display(&item.value, diagram.format)
                    .unwrap_or_else(|| fallback_value(&item.value)),
                highlighted: item.highlighted,
            })
            .collect(),
        _ => vec![FlatRow {
            key: String::new(),
            value: scalar_display(&diagram.root.value, diagram.format).unwrap_or_default(),
            highlighted: diagram.root.highlighted,
        }],
    };

    if rows.is_empty() {
        let svg = SvgBuilder::new_plantuml(20.0, 20.0, diagram_type);
        return svg.finalize_plantuml();
    }

    render_single_box(&rows, diagram_type)
}

/// Placeholder text for a nested value in the fallback path.
fn fallback_value(v: &JsonNodeValue) -> String {
    match v {
        JsonNodeValue::Object { fields } if fields.is_empty() => "\u{00a0}\u{00a0}\u{00a0}".into(),
        JsonNodeValue::Array { items } if items.is_empty() => "\u{00a0}\u{00a0}\u{00a0}".into(),
        JsonNodeValue::Object { .. } => "{...}".into(),
        JsonNodeValue::Array { .. } => "[...]".into(),
        _ => String::new(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rustuml_parser::diagram::Diagram;

    fn render_input(input: &str) -> String {
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        crate::render_svg(&diagram)
    }

    #[test]
    fn renders_json_object() {
        let svg = render_input("@startjson\n{\"name\": \"Alice\", \"age\": 30}\n@endjson");
        assert!(svg.contains("name"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("age"));
        assert!(svg.contains("30"));
    }

    #[test]
    fn emits_plantuml_envelope() {
        let svg = render_input("@startjson\n{\"name\": \"Alice\"}\n@endjson");
        assert!(svg.contains(r#"data-diagram-type="JSON""#));
        assert!(svg.contains("<defs/>"));
        assert!(svg.contains("</g></svg>"));
        assert!(svg.contains("font-weight=\"700\""));
        assert!(svg.contains("lengthAdjust=\"spacing\""));
        assert!(svg.contains("textLength="));
    }

    #[test]
    fn yaml_uses_yaml_diagram_type() {
        let svg = render_input("@startyaml\nname: Alice\nage: 30\n@endyaml");
        assert!(svg.contains(r#"data-diagram-type="YAML""#));
        assert!(svg.contains("Alice"));
    }

    #[test]
    fn renders_yaml_list() {
        let svg = render_input("@startyaml\n- apple\n- banana\n- cherry\n@endyaml");
        assert!(svg.contains("apple"));
        assert!(svg.contains("banana"));
        assert!(svg.contains("cherry"));
    }

    #[test]
    fn renders_bool_glyphs() {
        let svg = render_input("@startjson\n{\"a\": true, \"b\": false}\n@endjson");
        assert!(svg.contains("&#9745; true"));
        assert!(svg.contains("&#9744; false"));
    }

    #[test]
    fn renders_null_glyph() {
        let svg = render_input("@startjson\n{\"a\": null}\n@endjson");
        assert!(svg.contains("&#9216;"));
    }

    #[test]
    fn empty_string_renders_nbsp() {
        let svg = render_input("@startjson\n{\"a\": \"\"}\n@endjson");
        assert!(svg.contains("&#160;"));
    }

    #[test]
    fn empty_root_object_and_array_render_nbsp_box() {
        for source in ["@startjson\n{}\n@endjson", "@startjson\n[]\n@endjson"] {
            let svg = render_input(source);
            assert!(svg.contains(r#"width="36px""#), "{svg}");
            assert!(svg.contains(r#"height="42px""#), "{svg}");
            assert!(svg.contains(r#"textLength="4.4297""#), "{svg}");
            assert!(svg.contains("&#160;"), "{svg}");
        }
    }

    #[test]
    fn renamed_branching_json_uses_record_ports_and_postorder_edges() {
        let source = r#"@startjson
{
  "quasar_ledger": {
    "checkpoint": {"epoch": 17, "sealed": true},
    "owner": "delta"
  },
  "batches": [
    {"token": "r9", "ready": false},
    {"token": "s4", "ready": true}
  ]
}
@endjson"#;
        let diagram = rustuml_parser::parse::parse(source).unwrap();
        let Diagram::Json(diagram) = diagram else {
            panic!("expected JSON diagram");
        };
        let mut specs = Vec::new();
        super::collect_layout_boxes(&diagram.root, diagram.format, None, None, &mut specs);

        assert_eq!(specs.len(), 6);
        assert_eq!(super::postorder_edges(&specs), vec![2, 1, 4, 5, 3]);

        let svg = crate::render_svg(&Diagram::Json(diagram));
        assert_eq!(svg.matches("stroke-dasharray:3,3").count(), 5);
        assert_eq!(svg.matches("<ellipse").count(), 5);
        assert_eq!(svg.matches(r##"fill="#000000"/>"##).count(), 5);
        assert!(!svg.contains("{...}"));
        assert!(svg.contains("quasar_ledger"));
        assert!(svg.contains("checkpoint"));
        assert!(svg.contains("token"));
    }

    #[test]
    fn renamed_nested_yaml_layout_is_deterministic() {
        let source = r#"@startyaml
observatory:
  instruments:
    - name: heliograph
      online: true
    - name: spectrometer
      online: false
  region: south
revision: 23
@endyaml"#;
        let first = render_input(source);
        let second = render_input(source);

        assert_eq!(first, second);
        assert_eq!(first.matches("stroke-dasharray:3,3").count(), 4);
        assert_eq!(first.matches("<ellipse").count(), 4);
        assert!(first.contains("heliograph"));
        assert!(first.contains("spectrometer"));
        assert!(!first.contains("[...]"));
    }

    #[test]
    fn renamed_keyed_map_routes_four_branches_with_deeper_mixed_nesting() {
        let source = r#"@startjson
{
  "aurora_relay": {"reading": 11},
  "borealis_relay": {"reading": 23},
  "comet_relay": {"inner_beacon": {"reading": 37}},
  "drift_relay": [
    {"reading": 41},
    {"reading": 53}
  ]
}
@endjson"#;
        let first = render_input(source);
        let second = render_input(source);

        assert_eq!(first, second);
        assert_eq!(first.matches("stroke-dasharray:3,3").count(), 7);
        assert_eq!(first.matches("<ellipse").count(), 7);
        assert!(first.contains("aurora_relay"));
        assert!(first.contains("inner_beacon"));
        assert!(!first.contains("{...}"));
    }

    #[test]
    fn dimensional_records_handle_renamed_deeper_changed_row_counts() {
        let source = r#"@startjson
{
  "alpha_scalar": 1,
  "renamed_branch": {
    "north": 2,
    "east": 3,
    "deeper_branch": {
      "violet": 5,
      "indigo": 8,
      "ultraviolet": 13
    },
    "west": 21
  },
  "gamma_scalar": 34,
  "delta_scalar": 55,
  "epsilon_scalar": 89
}
@endjson"#;
        let diagram = rustuml_parser::parse::parse(source).unwrap();
        let Diagram::Json(diagram) = diagram else {
            panic!("expected JSON diagram");
        };
        let mut specs = Vec::new();
        super::collect_layout_boxes(&diagram.root, diagram.format, None, None, &mut specs);
        let row_counts = specs
            .iter()
            .map(|spec| super::box_layout_metrics(&spec.rows).row_heights.len())
            .collect::<Vec<_>>();
        assert_eq!(row_counts, vec![5, 4, 3]);

        let first = crate::render_svg(&Diagram::Json(diagram));
        let second = render_input(source);
        assert_eq!(first, second);
        assert_eq!(first.matches("stroke-dasharray:3,3").count(), 2);
        assert!(first.contains("renamed_branch"));
        assert!(first.contains("ultraviolet"));
    }

    #[test]
    fn dimensional_array_fanout_keeps_every_renamed_connector() {
        let source = r#"@startjson
{
  "parts": [
    {"stock_id": "string", "amount": "integer"}
  ],
  "delivery": {"location": "string", "carrier": "string"},
  "invoice": {"channel": "string"},
  "revision": 7
}
@endjson"#;
        let first = render_input(source);
        let second = render_input(source);

        assert_eq!(first, second);
        assert_eq!(first.matches("stroke-dasharray:3,3").count(), 4);
        assert_eq!(first.matches("<ellipse").count(), 4);
        assert!(first.contains("parts"));
        assert!(first.contains("stock_id"));
        assert!(first.contains("revision"));
    }

    #[test]
    fn renamed_changed_array_rows_choose_legacy_horizontal_record_side() {
        let source = r#"@startjson
{
  "renamed_pair": [2, 3],
  "renamed_quartet": [5, 7, 11, 13],
  "renamed_sextet": [17, 19, 23, 29, 31, 37]
}
@endjson"#;
        let svg = render_input(source);
        let style = svg.find("stroke-dasharray:3,3").unwrap();
        let path_start = svg[..style].rfind("<path d=\"").unwrap() + "<path d=\"".len();
        let path = &svg[path_start..path_start + svg[path_start..].find('"').unwrap()];
        let (move_to, remainder) = path.strip_prefix('M').unwrap().split_once(" L").unwrap();
        let line_to = remainder.split_once(" C").unwrap().0;
        let parse_point = |point: &str| {
            let (x, y) = point.split_once(',').unwrap();
            (x.parse::<f64>().unwrap(), y.parse::<f64>().unwrap())
        };
        let lead = parse_point(move_to);
        let boundary = parse_point(line_to);

        assert_eq!(svg.matches("stroke-dasharray:3,3").count(), 3);
        assert!((lead.1 - boundary.1).abs() < 0.001);
        assert!((lead.0 - boundary.0).abs() > 1.0);
        assert!(svg.contains("renamed_sextet"));
    }

    #[test]
    fn renamed_rectangular_matrix_routes_changed_row_and_column_counts() {
        let source = r#"@startjson
{
  "renamed_matrix": [
    [2, 3, 5, 7],
    [11, 13, 17, 19],
    [23, 29, 31, 37]
  ],
  "matrix_rows": 3
}
@endjson"#;
        let first = render_input(source);
        let second = render_input(source);

        assert_eq!(first, second);
        assert_eq!(first.matches("stroke-dasharray:3,3").count(), 4);
        assert_eq!(first.matches("<ellipse").count(), 4);
        assert!(first.contains("renamed_matrix"));
        assert!(first.contains("matrix_rows"));
    }

    #[test]
    fn renamed_deeper_records_keep_every_boundary_connector() {
        let source = r#"@startjson
{
  "renamed_root": {
    "layer_two": {
      "renamed_grid": [
        [41, 43, 47],
        [53, 59, 61]
      ],
      "layer_value": 67
    },
    "renamed_sibling": {"leaf_value": 71}
  },
  "revision": 73
}
@endjson"#;
        let first = render_input(source);
        let second = render_input(source);

        assert_eq!(first, second);
        assert_eq!(first.matches("stroke-dasharray:3,3").count(), 6);
        assert_eq!(first.matches("<ellipse").count(), 6);
        assert!(first.contains("renamed_root"));
        assert!(first.contains("layer_two"));
        assert!(first.contains("renamed_grid"));
    }

    #[test]
    fn empty_collection_uses_pre_paint_width_for_smetana() {
        let metrics = super::box_layout_metrics(&[]);
        assert_eq!(metrics.width, 30.0);
        assert_eq!(metrics.height, 15.0);
        assert_eq!(metrics.graph_width, 0.0);
    }

    #[test]
    fn parses_diagram_kind() {
        let d = rustuml_parser::parse::parse("@startjson\n{\"a\":1}\n@endjson").unwrap();
        assert!(matches!(d, Diagram::Json(_)));
    }
}
