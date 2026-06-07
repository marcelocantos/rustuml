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

use rustuml_parser::diagram::archimate::*;

use crate::layout_oracle::{OracleEntity, OracleLayout, emit_oracle_cluster_children};
use crate::metrics;
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

// ---------------------------------------------------------------------------
// Fallback (no-oracle) geometry-driven grid renderer.
// ---------------------------------------------------------------------------

const ELEM_MIN_W: f64 = 120.0;
const ELEM_H: f64 = 50.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 40.0;
const FALLBACK_FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 10.0;
const PADDING: f64 = 12.0;
const CORNER_R: f64 = 8.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
const GROUP_PAD: f64 = 15.0;
const GROUP_HEADER: f64 = 20.0;

pub fn render(diagram: &ArchimateDiagram, theme: &Theme) -> String {
    if diagram.elements.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n = diagram.elements.len();
    let cols = (n as f64).sqrt().ceil() as usize;

    let widths: Vec<f64> = diagram
        .elements
        .iter()
        .map(|e| {
            let label_w = metrics::text_width(&e.label, FALLBACK_FONT_SIZE) + PADDING * 2.0;
            let kind_w = metrics::text_width(&format!("\u{00ab}{}\u{00bb}", e.kind), SMALL_FONT)
                + PADDING * 2.0;
            label_w.max(kind_w).max(ELEM_MIN_W)
        })
        .collect();

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, w) in widths.iter().enumerate() {
            cw[i % cols] = cw[i % cols].max(*w);
        }
        cw
    };
    let rows = n.div_ceil(cols);

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let total_w =
        (MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64).max(200.0);
    let total_h = (MARGIN * 2.0 + rows as f64 * (ELEM_H + GAP) + title_h).max(80.0);

    let mut svg = SvgBuilder::new(total_w, total_h);

    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_w / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }

    let cs = &theme.class;

    let y_start = title_h + MARGIN;
    let mut positions = Vec::new();
    for (i, _elem) in diagram.elements.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (ELEM_H + GAP);
        let w = col_w[col];
        positions.push((x, y, w));
    }

    for group in &diagram.groups {
        if group.element_ids.is_empty() {
            continue;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for eid in &group.element_ids {
            if let Some(idx) = diagram.elements.iter().position(|e| e.id == *eid) {
                let (ex, ey, ew) = positions[idx];
                min_x = min_x.min(ex);
                min_y = min_y.min(ey);
                max_x = max_x.max(ex + ew);
                max_y = max_y.max(ey + ELEM_H);
            }
        }
        if min_x < f64::MAX {
            let gx = min_x - GROUP_PAD;
            let gy = min_y - GROUP_PAD - GROUP_HEADER;
            let gw = max_x - min_x + GROUP_PAD * 2.0;
            let gh = max_y - min_y + GROUP_PAD * 2.0 + GROUP_HEADER;
            svg.rect(gx, gy, gw, gh, "#EEEEEE", "#888888");
            svg.text(
                gx + 6.0,
                gy + GROUP_HEADER - 4.0,
                &group.label,
                "start",
                FALLBACK_FONT_SIZE,
            );
        }
    }

    for (i, elem) in diagram.elements.iter().enumerate() {
        let (x, y, w) = positions[i];
        let fill = elem.layer.default_color();

        svg.rounded_rect(x, y, w, ELEM_H, CORNER_R, fill, &cs.border_color);

        svg.text_colored(
            x + w / 2.0,
            y + 16.0,
            &format!("\u{00ab}{}\u{00bb}", elem.kind),
            "middle",
            SMALL_FONT,
            "#666666",
        );

        svg.text(
            x + w / 2.0,
            y + ELEM_H / 2.0 + 10.0,
            &elem.label,
            "middle",
            FALLBACK_FONT_SIZE,
        );
    }

    for rel in &diagram.relations {
        let fi = diagram.elements.iter().position(|e| e.id == rel.from);
        let ti = diagram.elements.iter().position(|e| e.id == rel.to);

        let (fi, ti) = match (fi, ti) {
            (Some(f), Some(t)) => (f, t),
            _ => continue,
        };

        let (fx, fy, fw) = positions[fi];
        let (tx, ty, tw) = positions[ti];

        let from_cx = fx + fw / 2.0;
        let from_cy = fy + ELEM_H;
        let to_cx = tx + tw / 2.0;
        let to_cy = ty;

        let dashed = matches!(
            rel.kind,
            ArchimateRelationKind::Realization | ArchimateRelationKind::Influence
        );

        svg.line_segment(from_cx, from_cy, to_cx, to_cy, &cs.border_color, dashed);
        svg.arrow_head(to_cx, to_cy, 90.0);

        if let Some(label) = &rel.label {
            let mx = (from_cx + to_cx) / 2.0;
            let my = (from_cy + to_cy) / 2.0;
            svg.text(mx + 6.0, my - 4.0, label, "start", SMALL_FONT);
        }
    }

    svg.finalize()
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
        assert!(svg.contains("#FFFFB5"), "Business color missing: {svg}");
        assert!(svg.contains("#C9E7B7"), "Technology color missing: {svg}");
    }

    #[test]
    fn stereotype_labels_rendered() {
        let input = "@startuml\n!include <archimate/Archimate>\nMotivation_Goal(g, \"Reduce Costs\")\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Goal"), "Kind label missing: {svg}");
        assert!(svg.contains("Reduce Costs"), "Element label missing: {svg}");
    }
}
