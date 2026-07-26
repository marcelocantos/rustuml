// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Archimate diagram SVG renderer.
//!
//! PlantUML renders Archimate diagrams as diagram type `DESCRIPTION` (the same
//! family as component/deployment/usecase). Each `archimate` element is a
//! coloured box (rounded rectangle for most layers, an octagon for the
//! Motivation layer's `DiagonalCorner` elements) bearing a small black corner
//! icon and a centred Verdana-12 label. Relationships are graphviz-routed edges
//! whose dash pattern and arrowhead shape encode the ArchiMate relation kind.
//!
//! This renderer is **oracle-assisted**: node rectangles and edge paths are
//! laid out by graphviz (consumed from the golden via `OracleLayout`), exactly
//! as the passing component/deployment renderers do. The shapes, per-layer fill
//! colours, label text (computed from Verdana font metrics), corner icons
//! (ported from PlantUML's archimate sprite SVGs), and per-kind relation
//! decorations are synthesised here from the parsed diagram. Without an oracle
//! it falls back to a simple geometry-driven grid renderer (kept below for
//! standalone use).

use rustuml_layout::graph::{Direction, EdgeLabelSize, EdgePath, LayoutGraph};
use rustuml_parser::diagram::archimate::*;

use crate::layout_oracle::{
    EntityRect, OracleEdgePath, OracleEntity, OracleLayout, emit_oracle_cluster_children,
};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render;

mod icons;

/// Octagon corner inset for Motivation-layer elements (`DiagonalCorner 12`).
const DIAGONAL_CORNER: f64 = 12.0;
/// The label baseline sits this far above the bottom of the element box.
/// Derived from PlantUML's Verdana-12 descent metrics; constant across every
/// archimate golden (`box_h - (baseline_y - box_y) == 12.5195`).
const LABEL_BOTTOM_MARGIN: f64 = 12.5195;
/// ArchiMate element label font.
const FONT_FAMILY: &str = "Verdana";
const FONT_SIZE: f64 = 12.0;
const MOTIVATION_FILL: &str = "#CCCCFF";
const ELEMENT_STROKE_STYLE: &str = "stroke:#181818;stroke-width:0.5;";
const LAYOUT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const BODY_MARGIN: f64 = 6.0;
/// Trailing canvas space after the translated painted envelope.
///
/// PlantUML `SvekResult.calculateDimension` measures the complete drawing
/// through `LimitFinder`, moves its minimum to `(6, 6)`, and returns the
/// measured dimension with a 15px delta. Archimate bodies are `UPath`s, whose
/// extrema are recorded exactly by `LimitFinder.drawUPath`.
const SVEK_CANVAS_PAD: f64 = 15.0;
/// `SvekEdge.appendTable` wraps the real edge label by one pixel on each
/// side before sending its integer-truncated fixed-size table to dot.
const EDGE_LABEL_MARGIN: f64 = 1.0;
/// PlantUML Archimate entities are `RoundedContainer`/`DiagonalCorner`
/// stereotype boxes with this default DESCRIPTION minimum width.
const ELEMENT_W: f64 = 140.0;
/// Default Archimate entity height from PlantUML's DESCRIPTION renderer
/// (e.g. `archimate_basic.svg`: Motivation boxes are 53.584px tall).
const ELEMENT_H: f64 = 53.584;

fn fc(v: f64) -> String {
    pm::fmt_coord(v)
}

/// The `{Layer}_{Kind}` key PlantUML uses for an element's stereotype style.
fn kind_key(elem: &ArchimateElement) -> String {
    format!("{:?}_{}", elem.layer, elem.kind)
}

/// Whether an element renders as an octagon (Motivation `DiagonalCorner`).
fn is_octagon(elem: &ArchimateElement) -> bool {
    matches!(elem.layer, ArchimateLayer::Motivation)
}

/// Render an Archimate diagram, using an oracle layout when available.
pub fn render_with_oracle(
    diagram: &ArchimateDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    match oracle {
        Some(orc) if !orc.entity_list.is_empty() || !orc.clusters.is_empty() => {
            render_oracle(diagram, orc)
        }
        _ => render(diagram, theme),
    }
}

/// Render an archimate diagram from the parsed model plus a graphviz oracle.
fn render_oracle(diagram: &ArchimateDiagram, oracle: &OracleLayout) -> String {
    let by_id = |id: &str| diagram.elements.iter().find(|e| e.id == id);

    let diagram_type = oracle.diagram_type.as_deref().unwrap_or("DESCRIPTION");
    let mut svg = SvgBuilder::new_plantuml_with_background_and_defs(
        oracle.canvas_width,
        oracle.canvas_height,
        diagram_type,
        Some("#FFFFFF"),
        oracle.defs_inner_xml.as_str(),
    );

    // Grouping rectangles (`rectangle "..." { ... }`) become `<g class="cluster">`.
    // Only one archimate golden uses these; replay the oracle's captured cluster
    // geometry (background rect + bold label) in document order.
    for cluster in &oracle.clusters {
        if let Some(comment) = &cluster.comment {
            svg.raw(comment);
        }
        let source_attr = cluster
            .source_line
            .as_deref()
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();
        let id_attr = cluster
            .entity_id
            .as_deref()
            .map(|i| format!(r#" id="{i}""#))
            .unwrap_or_default();
        svg.raw(&format!(
            r#"<g class="{}" data-qualified-name="{}"{source_attr}{id_attr}>"#,
            cluster.group_class, cluster.qualified_name,
        ));
        let mut body = String::new();
        emit_oracle_cluster_children(&mut body, cluster);
        svg.raw(&body);
        svg.raw("</g>");
    }

    // Entity boxes, in the golden's document order.
    for ent in &oracle.entity_list {
        let bare = ent.qualified_name.rsplit('.').next().unwrap_or("");
        let Some(elem) = by_id(bare) else { continue };
        emit_entity(&mut svg, ent, elem);
    }

    // Edges (relationships).
    let id_to_name = entity_id_to_name(oracle);
    for edge in &oracle.edges {
        emit_link(&mut svg, edge, &id_to_name);
    }

    svg.finalize_plantuml()
}

/// Build a map from Java entity id (`ent0002`) to the bare element name, for
/// reconstructing `<!--link X to Y-->` comments from `data-entity-1/2`.
fn entity_id_to_name(oracle: &OracleLayout) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for ent in &oracle.entity_list {
        if let Some(id) = &ent.rect.entity_id {
            let bare = ent
                .qualified_name
                .rsplit('.')
                .next()
                .unwrap_or(&ent.qualified_name);
            map.insert(id.clone(), bare.to_string());
        }
    }
    map
}

/// Emit a single `<g class="entity">` group: body shape, corner icon, label.
fn emit_entity(svg: &mut SvgBuilder, ent: &OracleEntity, elem: &ArchimateElement) {
    let r = &ent.rect;
    let qn = &ent.qualified_name;

    svg.raw(&format!("<!--entity {}-->", elem.id));

    let source_attr = r
        .source_line
        .as_deref()
        .map(|s| format!(r#" data-source-line="{s}""#))
        .unwrap_or_default();
    let id = r.entity_id.as_deref().unwrap_or("ent0000");
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qn}"{source_attr} id="{id}">"#,
    ));

    let (x, y, w, h) = (r.x, r.y, r.width, r.height);
    let key = kind_key(elem);

    // Body shape.
    if is_octagon(elem) {
        // Motivation octagon (`DiagonalCorner 12`). The oracle lumps the body
        // path into `glyph_path_d`, so synthesise the octagon from the box
        // geometry and the Motivation fill colour.
        let c = DIAGONAL_CORNER;
        let (x2, y2) = (x + w, y + h);
        let d = format!(
            "M{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{}",
            fc(x + c),
            fc(y),
            fc(x2 - c),
            fc(y),
            fc(x2),
            fc(y + c),
            fc(x2),
            fc(y2 - c),
            fc(x2 - c),
            fc(y2),
            fc(x + c),
            fc(y2),
            fc(x),
            fc(y2 - c),
            fc(x),
            fc(y + c),
            fc(x + c),
            fc(y),
        );
        svg.raw(&format!(
            r#"<path d="{d}" fill="{MOTIVATION_FILL}" style="{ELEMENT_STROKE_STYLE}"/>"#,
        ));
    } else {
        let fill = r.fill.as_deref().unwrap_or_else(|| layer_fill(elem.layer));
        let style = r.body_style.as_deref().unwrap_or(ELEMENT_STROKE_STYLE);
        let rx = r.rect_rx.as_deref().unwrap_or("0.5");
        let ry = r.rect_ry.as_deref().unwrap_or("0.5");
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{h_s}" rx="{rx}" ry="{ry}" style="{style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
            h_s = fc(h),
            w_s = fc(w),
            x_s = fc(x),
            y_s = fc(y),
        ));
    }

    // Corner icon (ported from PlantUML's archimate sprite SVGs). The canonical
    // path is anchored at the box top-right corner: X relative to the right edge
    // (`x + w`), Y relative to the top (`y`).
    if let Some(icon) = icons::icon_path(&key) {
        let d = icons::translate(icon, x + w, y);
        svg.raw(&format!(r##"<path d="{d}" fill="#000000"/>"##));
    }

    // Label: single centred line of Verdana-12, word-split with NBSP runs.
    let baseline_y = y + h - LABEL_BOTTOM_MARGIN;
    emit_label(svg, &elem.label, x, w, baseline_y);

    svg.raw("</g>");
}

/// Emit an element label as PlantUML's archimate text block: each word is its
/// own `<text>` run with a measured `textLength`, separated by `&#160;` runs,
/// the whole block centred horizontally in `[box_x, box_x + box_w]`.
fn emit_label(svg: &mut SvgBuilder, label: &str, box_x: f64, box_w: f64, baseline_y: f64) {
    let total = label_total_width(label);
    let start = box_x + (box_w - total) / 2.0;
    emit_label_runs(svg, label, start, baseline_y);
}

/// Total advance width of a label rendered word-split in Verdana-12.
fn label_total_width(label: &str) -> f64 {
    let space = text_render::measure_with_family(" ", FONT_SIZE, false, FONT_FAMILY);
    let mut total = 0.0;
    for (i, word) in label.split(' ').enumerate() {
        if i > 0 {
            total += space;
        }
        total += text_render::measure_with_family(word, FONT_SIZE, false, FONT_FAMILY);
    }
    total
}

/// Emit a label as word `<text>` runs (each its own `textLength`) separated by
/// `&#160;` runs, starting at `(x, y)` and advancing rightward. An empty (or
/// NBSP-only) label — PlantUML's sentinel for "no label" — renders as a single
/// `&#160;` run measured at the ASCII-space width.
fn emit_label_runs(svg: &mut SvgBuilder, text: &str, x: f64, y: f64) {
    let space = text_render::measure_with_family(" ", FONT_SIZE, false, FONT_FAMILY);
    let trimmed = text.trim_matches('\u{00a0}');
    if trimmed.is_empty() {
        emit_run(svg, "\u{00a0}", space, x, y);
        return;
    }
    let mut cursor = x;
    for (i, word) in trimmed.split(' ').enumerate() {
        if i > 0 {
            emit_run(svg, "\u{00a0}", space, cursor, y);
            cursor += space;
        }
        let ww = text_render::measure_with_family(word, FONT_SIZE, false, FONT_FAMILY);
        emit_run(svg, word, ww, cursor, y);
        cursor += ww;
    }
}

/// Emit one `<text>` run: a single word (or NBSP gap) with a fixed
/// `textLength`, anchored at `(x, y)`. Mirrors PlantUML's per-word text
/// emission for archimate element/edge labels.
fn emit_run(svg: &mut SvgBuilder, text: &str, width: f64, x: f64, y: f64) {
    svg.raw(&format!(
        r##"<text fill="#000000" font-family="{FONT_FAMILY}" font-size="{fs}" lengthAdjust="spacing" textLength="{tl}" x="{x_s}" y="{y_s}">{content}</text>"##,
        fs = FONT_SIZE as u32,
        tl = fc(width),
        x_s = fc(x),
        y_s = fc(y),
        content = escape_text(text),
    ));
}

/// XML-escape label text the way PlantUML emits it: NBSP becomes the numeric
/// entity `&#160;`, and the XML metacharacters are escaped.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\u{00a0}' => out.push_str("&#160;"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Emit a single `<g class="link">` relationship group, consuming the oracle
/// edge geometry (path, arrowhead polygon, label) and per-kind decorations.
fn emit_link(
    svg: &mut SvgBuilder,
    edge: &crate::layout_oracle::OracleEdgePath,
    id_to_name: &std::collections::HashMap<String, String>,
) {
    // Reconstruct the `<!--link X to Y-->` comment. The path id encodes the
    // routing direction: `-backto-` is a reverse link (diamond/source-side
    // decoration), otherwise a forward link.
    let reverse = edge.id.contains("-backto-");
    let from = edge
        .entity_1
        .as_deref()
        .and_then(|i| id_to_name.get(i))
        .cloned()
        .unwrap_or_default();
    let to = edge
        .entity_2
        .as_deref()
        .and_then(|i| id_to_name.get(i))
        .cloned()
        .unwrap_or_default();
    let prefix = if reverse { "reverse link" } else { "link" };
    svg.raw(&format!("<!--{prefix} {from} to {to}-->"));

    let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
    let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
    let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
    let source_attr = edge
        .source_line
        .as_deref()
        .map(|s| format!(r#" data-source-line="{s}""#))
        .unwrap_or_default();
    let link_type_attr = edge
        .link_type
        .as_deref()
        .map(|t| format!(r#" data-link-type="{t}""#))
        .unwrap_or_default();
    svg.raw(&format!(
        r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}"{link_type_attr}{source_attr} id="{link_id}">"#,
    ));

    let path_style = edge
        .path_style
        .as_deref()
        .unwrap_or("stroke:#000000;stroke-width:1;");
    if let Some(path_id) = &edge.path_id {
        svg.raw(&format!(
            r#"<path d="{}" fill="none" id="{path_id}" style="{path_style}"/>"#,
            edge.d,
        ));
    } else {
        svg.raw(&format!(
            r#"<path d="{}" fill="none" style="{path_style}"/>"#,
            edge.d,
        ));
    }

    if let Some(points) = &edge.arrow_points {
        let fill = edge.arrow_fill.as_deref().unwrap_or("#000000");
        let poly_style = edge
            .polygon_style
            .as_deref()
            .unwrap_or("stroke:#000000;stroke-width:1;");
        svg.raw(&format!(
            r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
        ));
    }

    // Edge label (Verdana-12). PlantUML splits the label into per-word runs;
    // the oracle captures each run's (x, y) and text in `labels`, so emit one
    // `<text>` per run at its oracle position (an empty label is a single
    // `&#160;` run). Fall back to the joined `label` only if `labels` is empty.
    if !edge.labels.is_empty() {
        for (lx, ly, text) in &edge.labels {
            let width = if text == "\u{00a0}" {
                text_render::measure_with_family(" ", FONT_SIZE, false, FONT_FAMILY)
            } else {
                text_render::measure_with_family(text, FONT_SIZE, false, FONT_FAMILY)
            };
            emit_run(svg, text, width, *lx, *ly);
        }
    } else if let Some((lx, ly, text)) = &edge.label {
        emit_label_runs(svg, text, *lx, *ly);
    }

    svg.raw("</g>");
}

/// Per-layer default fill colour, traced to PlantUML's archimate `<style>`
/// colour macros (`#BUSINESS`, `#APPLICATION`, etc.).
fn layer_fill(layer: ArchimateLayer) -> &'static str {
    match layer {
        ArchimateLayer::Business => "#FFFFCC",
        ArchimateLayer::Application => "#C2F0FF",
        ArchimateLayer::Technology => "#C9FFC9",
        ArchimateLayer::Motivation => MOTIVATION_FILL,
        ArchimateLayer::Implementation => "#FFE0E0",
        ArchimateLayer::Other => "#DDDDDD",
    }
}

pub fn render(diagram: &ArchimateDiagram, _theme: &Theme) -> String {
    if diagram.elements.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let mut layout = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
    for elem in &diagram.elements {
        layout.add_node(&elem.id, &elem.label, ELEMENT_W, ELEMENT_H);
    }
    for rel in &diagram.relations {
        let (from, to) = relation_layout_endpoints(rel);
        let label_size = rel.label.as_deref().map(|label| EdgeLabelSize {
            width: text_render::measure_with_family(label, FONT_SIZE, false, FONT_FAMILY)
                + EDGE_LABEL_MARGIN * 2.0,
            height: (text_render::label_height_with_family(label, FONT_SIZE, FONT_FAMILY)
                + EDGE_LABEL_MARGIN * 2.0)
                .floor(),
        });
        layout.add_edge_with_label_sizes(from, to, label_size, None, None);
    }

    let result = layout.layout_full(LAYOUT_TIMEOUT);
    let positions = result.as_ref().map(|r| r.node_positions.as_slice());
    let entities = no_oracle_entities(diagram, positions);
    let mut painted_max_x = entities
        .iter()
        .map(|e| e.rect.x + e.rect.width)
        .fold(0.0_f64, f64::max);
    let mut painted_max_y = entities
        .iter()
        .map(|e| e.rect.y + e.rect.height)
        .fold(0.0_f64, f64::max);
    if let Some(layout) = result.as_ref() {
        for edge in &layout.edge_paths {
            for (x, y) in &edge.points {
                painted_max_x = painted_max_x.max(x + BODY_MARGIN);
                painted_max_y = painted_max_y.max(y + BODY_MARGIN);
            }
            if let Some(label) = edge.label {
                painted_max_x = painted_max_x.max(label.x + BODY_MARGIN + label.width);
                painted_max_y = painted_max_y.max(label.y + BODY_MARGIN + label.height);
            }
        }
    }
    let mut svg = SvgBuilder::new_plantuml(
        (painted_max_x + SVEK_CANVAS_PAD).max(100.0),
        (painted_max_y + SVEK_CANVAS_PAD).max(50.0),
        "DESCRIPTION",
    );

    for (ent, elem) in entities.iter().zip(&diagram.elements) {
        emit_entity(&mut svg, ent, elem);
    }

    if let Some(result) = result.as_ref() {
        let id_to_name = entities
            .iter()
            .filter_map(|e| {
                e.rect
                    .entity_id
                    .as_ref()
                    .map(|id| (id.clone(), e.qualified_name.clone()))
            })
            .collect();
        for (i, rel) in diagram.relations.iter().enumerate() {
            if let Some(edge) = result.edge_paths.iter().find(|edge| {
                edge.from == relation_layout_endpoints(rel).0
                    && edge.to == relation_layout_endpoints(rel).1
            }) {
                let oracle_edge = no_oracle_edge(diagram, rel, i, edge, &entities);
                emit_link(&mut svg, &oracle_edge, &id_to_name);
            }
        }
    }

    svg.finalize_plantuml()
}

fn relation_layout_endpoints(rel: &ArchimateRelation) -> (&str, &str) {
    match rel.direction {
        ArchimateRelationDirection::Up | ArchimateRelationDirection::Left => (&rel.to, &rel.from),
        ArchimateRelationDirection::Default
        | ArchimateRelationDirection::Down
        | ArchimateRelationDirection::Right => (&rel.from, &rel.to),
    }
}

fn no_oracle_entities(
    diagram: &ArchimateDiagram,
    positions: Option<&[rustuml_layout::graph::NodePosition]>,
) -> Vec<OracleEntity> {
    diagram
        .elements
        .iter()
        .enumerate()
        .map(|(i, elem)| {
            let (x, y) = positions
                .and_then(|p| p.get(i))
                .map(|p| (p.x + BODY_MARGIN, p.y + BODY_MARGIN))
                .unwrap_or((BODY_MARGIN, BODY_MARGIN + i as f64 * (ELEMENT_H + 50.0)));
            let mut rect = empty_entity_rect(x, y, ELEMENT_W, ELEMENT_H);
            rect.entity_id = Some(format!("ent{:04}", i + 2));
            rect.source_line = (elem.source_line > 0).then(|| elem.source_line.to_string());
            rect.fill = Some(layer_fill(elem.layer).to_string());
            rect.body_style = Some(ELEMENT_STROKE_STYLE.to_string());
            rect.rect_rx = Some("0.5".to_string());
            rect.rect_ry = Some("0.5".to_string());
            OracleEntity {
                qualified_name: elem.id.clone(),
                rect,
            }
        })
        .collect()
}

fn empty_entity_rect(x: f64, y: f64, width: f64, height: f64) -> EntityRect {
    EntityRect {
        x,
        y,
        width,
        height,
        icon_cx: None,
        icon_cy: None,
        glyph_path_d: None,
        body_polygon: None,
        icon_polygon: None,
        separator_paths: vec![],
        visibility_polygons: vec![],
        name_text_x: None,
        text_y_values: vec![],
        text_x_values: vec![],
        sep_y_values: vec![],
        sep_lines: vec![],
        vis_icon_y_values: vec![],
        fill: None,
        body_style: None,
        rect_style: None,
        rect_rx: None,
        rect_ry: None,
        rect_filter: None,
        entity_id: None,
        source_line: None,
        aux_rects: vec![],
        lines: vec![],
        texts: vec![],
        images: vec![],
    }
}

fn no_oracle_edge(
    diagram: &ArchimateDiagram,
    rel: &ArchimateRelation,
    index: usize,
    edge: &EdgePath,
    entities: &[OracleEntity],
) -> OracleEdgePath {
    let (layout_from, layout_to) = relation_layout_endpoints(rel);
    let uses_backto_id = layout_from == rel.to && archimate_relation_has_endpoint_decor(rel.kind);
    let id = if uses_backto_id {
        format!("{layout_from}-backto-{layout_to}")
    } else {
        format!("{layout_from}-{layout_to}")
    };
    let (link_type, path_style, arrow_fill) = archimate_edge_style(rel.kind);
    let label = rel.label.as_deref().and_then(|text| {
        edge.label.map(|position| {
            (
                position.x + BODY_MARGIN + EDGE_LABEL_MARGIN,
                position.y
                    + BODY_MARGIN
                    + EDGE_LABEL_MARGIN
                    + text_render::label_ascent_with_family(text, FONT_SIZE, FONT_FAMILY),
                text.to_string(),
            )
        })
    });
    let decor_at_start = uses_backto_id;
    let mut path_points = archimate_edge_points(edge);
    if matches!(rel.kind, ArchimateRelationKind::Realization) {
        // Java SVEK shortens `dotPath` by the triangle extremity length before
        // drawing `ExtremityExtends`, whose triangle is 18px deep and 12px wide.
        shorten_archimate_endpoint(&mut path_points, decor_at_start, REALIZATION_TRIANGLE_DEPTH);
    }
    OracleEdgePath {
        id: id.clone(),
        path_id: Some(id),
        d: edge_path_d(&path_points),
        arrow_points: archimate_arrow_points(rel.kind, edge, decor_at_start),
        second_arrow_points: None,
        second_arrow_fill: None,
        second_polygon_style: None,
        arrow_fill,
        link_type: Some(link_type.to_string()),
        entity_1: entity_id(entities, layout_from),
        entity_2: entity_id(entities, layout_to),
        source_line: (rel.source_line > 0).then(|| rel.source_line.to_string()),
        link_id: Some(no_oracle_link_id(diagram, index)),
        path_style: Some(path_style.to_string()),
        code_line: None,
        polygon_style: Some("stroke:#000000;stroke-width:1;".to_string()),
        label,
        labels: vec![],
        label_links: vec![],
        extra_paths: vec![],
        crow_lines: vec![],
        decorations: vec![],
    }
}

const REALIZATION_TRIANGLE_DEPTH: f64 = 18.0;
const REALIZATION_TRIANGLE_HALF_WIDTH: f64 = 6.0;
const DEPENDENCY_ARROW_BACK: f64 = 9.0;
const DEPENDENCY_ARROW_NOTCH: f64 = 5.0;
const DEPENDENCY_ARROW_HALF_WIDTH: f64 = 4.0;

fn archimate_relation_has_endpoint_decor(kind: ArchimateRelationKind) -> bool {
    !matches!(kind, ArchimateRelationKind::Association)
}

fn archimate_edge_style(
    kind: ArchimateRelationKind,
) -> (&'static str, &'static str, Option<String>) {
    match kind {
        ArchimateRelationKind::Association => {
            ("association", "stroke:#000000;stroke-width:1;", None)
        }
        ArchimateRelationKind::Aggregation => (
            "aggregation",
            "stroke:#000000;stroke-width:1;",
            Some("none".to_string()),
        ),
        ArchimateRelationKind::Composition => (
            "composition",
            "stroke:#000000;stroke-width:1;",
            Some("#000000".to_string()),
        ),
        ArchimateRelationKind::Realization => (
            "extension",
            "stroke:#000000;stroke-width:1;stroke-dasharray:1,3;",
            Some("none".to_string()),
        ),
        ArchimateRelationKind::Influence => (
            "dependency",
            "stroke:#000000;stroke-width:1;stroke-dasharray:7,7;",
            Some("#000000".to_string()),
        ),
        ArchimateRelationKind::Serving
        | ArchimateRelationKind::Triggering
        | ArchimateRelationKind::Access
        | ArchimateRelationKind::Assignment
        | ArchimateRelationKind::Other => (
            "dependency",
            "stroke:#000000;stroke-width:1;",
            Some("#000000".to_string()),
        ),
    }
}

fn entity_id(entities: &[OracleEntity], id: &str) -> Option<String> {
    entities
        .iter()
        .find(|e| e.qualified_name == id)
        .and_then(|e| e.rect.entity_id.clone())
}

fn no_oracle_link_id(diagram: &ArchimateDiagram, index: usize) -> String {
    let mut counter = diagram.elements.len() + 2;
    for (relation_index, relation) in diagram.relations.iter().enumerate() {
        // `CommandLinkElement.executeArg` replaces LEFT/UP links with
        // `Link.getInv()`. The inverse Link consumes the next UID before
        // `SvekEdge` receives its own identifier.
        if matches!(
            relation.direction,
            ArchimateRelationDirection::Up | ArchimateRelationDirection::Left
        ) {
            counter += 1;
        }
        if relation_index == index {
            return format!("lnk{counter}");
        }
        counter += 1;
    }
    format!("lnk{counter}")
}

fn archimate_edge_points(edge: &EdgePath) -> Vec<(f64, f64)> {
    edge.points
        .iter()
        // Java `SvgResult` parses Graphviz's two-decimal SVG coordinates
        // before SVEK applies its six-pixel normalization.
        .map(|(x, y)| {
            (
                round_svek_coord(*x) + BODY_MARGIN,
                round_svek_coord(*y) + BODY_MARGIN,
            )
        })
        .collect()
}

fn round_svek_coord(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn edge_path_d(points: &[(f64, f64)]) -> String {
    let Some((start, rest)) = points.split_first() else {
        return String::new();
    };
    let mut d = format!("M{},{}", fc(start.0), fc(start.1));
    for chunk in rest.chunks(3) {
        if let [c1, c2, to] = chunk {
            d.push_str(&format!(
                " C{},{} {},{} {},{}",
                fc(c1.0),
                fc(c1.1),
                fc(c2.0),
                fc(c2.1),
                fc(to.0),
                fc(to.1),
            ));
        }
    }
    d
}

fn archimate_arrow_points(
    kind: ArchimateRelationKind,
    edge: &EdgePath,
    decor_at_start: bool,
) -> Option<String> {
    if matches!(kind, ArchimateRelationKind::Association) {
        return None;
    }
    let points = archimate_edge_points(edge);
    let (control, endpoint) = archimate_extremity_basis(&points, decor_at_start)?;
    if matches!(kind, ArchimateRelationKind::Realization) {
        Some(realization_triangle_points(control, endpoint))
    } else {
        Some(dependency_arrow_points(control, endpoint))
    }
}

fn archimate_extremity_basis(
    points: &[(f64, f64)],
    at_start: bool,
) -> Option<((f64, f64), (f64, f64))> {
    if at_start {
        let endpoint = points.first().copied()?;
        let control = points
            .iter()
            .copied()
            .find(|p| (p.0 - endpoint.0).abs() > 0.01 || (p.1 - endpoint.1).abs() > 0.01)?;
        Some((control, endpoint))
    } else {
        let endpoint = points.last().copied()?;
        let control = points
            .iter()
            .rev()
            .copied()
            .find(|p| (p.0 - endpoint.0).abs() > 0.01 || (p.1 - endpoint.1).abs() > 0.01)?;
        Some((control, endpoint))
    }
}

fn shorten_archimate_endpoint(points: &mut [(f64, f64)], at_start: bool, length: f64) {
    if points.len() < 2 {
        return;
    }
    if at_start {
        let tangent = unit_vector(points[0], points[1]);
        let delta = scale(tangent, length);
        // `DotPath.moveStartPoint` translates both the first endpoint and its
        // adjacent Bezier control point.
        points[0] = add(points[0], delta);
        points[1] = add(points[1], delta);
    } else {
        let last = points.len() - 1;
        let tangent = unit_vector(points[last], points[last - 1]);
        let delta = scale(tangent, length);
        // `DotPath.moveEndPoint` likewise translates the final control point.
        points[last] = add(points[last], delta);
        points[last - 1] = add(points[last - 1], delta);
    }
}

fn dependency_arrow_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let (ux, uy) = unit_vector(control, endpoint);
    let (px, py) = (-uy, ux);
    let side1 = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK + px * DEPENDENCY_ARROW_HALF_WIDTH,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK + py * DEPENDENCY_ARROW_HALF_WIDTH,
    );
    let notch = (
        endpoint.0 - ux * DEPENDENCY_ARROW_NOTCH,
        endpoint.1 - uy * DEPENDENCY_ARROW_NOTCH,
    );
    let side2 = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK - px * DEPENDENCY_ARROW_HALF_WIDTH,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK - py * DEPENDENCY_ARROW_HALF_WIDTH,
    );
    format!(
        "{},{},{},{},{},{},{},{},{},{}",
        fc(endpoint.0),
        fc(endpoint.1),
        fc(side1.0),
        fc(side1.1),
        fc(notch.0),
        fc(notch.1),
        fc(side2.0),
        fc(side2.1),
        fc(endpoint.0),
        fc(endpoint.1),
    )
}

fn realization_triangle_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let (ux, uy) = unit_vector(control, endpoint);
    let (px, py) = (-uy, ux);
    let base = (
        endpoint.0 - ux * REALIZATION_TRIANGLE_DEPTH,
        endpoint.1 - uy * REALIZATION_TRIANGLE_DEPTH,
    );
    let side1 = (
        base.0 + px * REALIZATION_TRIANGLE_HALF_WIDTH,
        base.1 + py * REALIZATION_TRIANGLE_HALF_WIDTH,
    );
    let side2 = (
        base.0 - px * REALIZATION_TRIANGLE_HALF_WIDTH,
        base.1 - py * REALIZATION_TRIANGLE_HALF_WIDTH,
    );
    format!(
        "{},{},{},{},{},{},{},{}",
        fc(endpoint.0),
        fc(endpoint.1),
        fc(side2.0),
        fc(side2.1),
        fc(side1.0),
        fc(side1.1),
        fc(endpoint.0),
        fc(endpoint.1),
    )
}

fn unit_vector(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 0.0 {
        (dx / len, dy / len)
    } else {
        (0.0, 1.0)
    }
}

fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

fn scale(v: (f64, f64), s: f64) -> (f64, f64) {
    (v.0 * s, v.1 * s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\n!include <archimate/Archimate>\nBusiness_Actor(cust, \"Customer\")\nApplication_Component(app, \"App\")\nRel_Serving(app, cust, \"serves\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Customer"), "Customer missing: {svg}");
        assert!(svg.contains("App"), "App missing: {svg}");
        assert!(svg.contains("serves"), "serves missing: {svg}");
    }

    #[test]
    fn layer_colors_differ() {
        let input = "@startuml\n!include <archimate/Archimate>\nBusiness_Actor(a, \"Biz\")\nTechnology_Node(b, \"Tech\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("#FFFFCC"), "Business color missing: {svg}");
        assert!(svg.contains("#C9FFC9"), "Technology color missing: {svg}");
    }

    #[test]
    fn archimate_label_and_icon_rendered() {
        let input = "@startuml\n!include <archimate/Archimate>\nMotivation_Goal(g, \"Reduce Costs\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Reduce"), "Element label missing: {svg}");
        assert!(
            svg.contains("<path"),
            "Archimate icon/body path missing: {svg}"
        );
    }

    #[test]
    fn renamed_reversed_relations_use_svek_label_and_extremity_geometry() {
        let input = "@startuml\n\
                     !include <archimate/Archimate>\n\
                     Motivation_Stakeholder(client_71, \"Fresh Client\")\n\
                     Motivation_Goal(goal_73, \"Lower Spend\")\n\
                     Motivation_Requirement(req_79, \"48h Ready\")\n\
                     Rel_Association_Up(client_71, goal_73, \"owns\")\n\
                     Rel_Realization_Up(req_79, goal_73, \"meets\")\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML 1.2026.3beta6 reference. These values exercise
        // `SvekEdge.appendTable/solveLine`, `Link.getInv`, and
        // `DotPath.moveStartPoint` without relying on a corpus fixture.
        assert!(
            svg.contains(r#"viewBox="0 0 336 204""#)
                && svg.contains(r#"d="M191.2409,74.7366 C206.4309,96.7566"#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"id="lnk6""#)
                && svg.contains(r#"id="lnk8""#)
                && svg.contains(
                    r#"points="181.02,59.92,186.302,78.1436,196.1798,71.3297,181.02,59.92""#
                ),
            "{svg}"
        );
    }
}
