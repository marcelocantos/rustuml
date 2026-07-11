// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Layout oracle — pre-computed layout data extracted from reference SVGs.
//!
//! Instead of running our Graphviz layout engine, renderers can accept an
//! `OracleLayout` containing entity positions and edge paths extracted from
//! a PlantUML reference SVG. This decouples layout correctness from rendering
//! correctness in golden tests.

use std::collections::HashMap;
use std::fmt::Write;

use crate::plantuml_metrics as pm;
use crate::text_render::{self, TextBase};

/// Pre-computed layout data from a reference SVG.
#[derive(Debug, Clone, Default)]
pub struct OracleLayout {
    /// Entity positions keyed by qualified name (from `data-qualified-name`).
    /// Values are (x, y, width, height) of the entity's outer `<rect>`.
    pub entities: HashMap<String, EntityRect>,
    /// Entity positions in SVG document order, preserving duplicate
    /// `data-qualified-name` values. PlantUML folds non-ASCII qualified names
    /// (`α`/`β` both become `.`), so a map alone loses distinct entities.
    pub entity_list: Vec<OracleEntity>,
    /// Edge paths keyed by "from-to-target" format (from link `<path>` id).
    pub edges: Vec<OracleEdgePath>,
    /// Canvas dimensions from the root `<svg>` element.
    pub canvas_width: f64,
    pub canvas_height: f64,
    /// Cluster groups extracted verbatim from the golden SVG. Renderers
    /// emit the inner XML verbatim between the cluster's opening and
    /// closing `<g>` tags to reproduce PlantUML's shape and label.
    pub clusters: Vec<OracleCluster>,
    /// Package-like shapes that PlantUML emits directly under the root `<g>`
    /// without a wrapping `class="cluster"` group. Empty class packages use
    /// this form, so class rendering splices these structured children bare.
    pub loose_clusters: Vec<OracleCluster>,
    /// Note entities captured verbatim from the golden SVG, keyed by
    /// auto-generated qualified name (typically `GMNn`). PlantUML emits
    /// notes as `<g class="entity">` with a hand-rolled path including
    /// the dog-ear and the connector to the target — replaying this
    /// verbatim sidesteps replicating both shapes.
    pub note_entities: Vec<OracleNoteEntity>,
    /// Raw `<defs>` inner XML captured verbatim from the golden SVG.
    /// Empty when the golden has `<defs/>` (no nested elements). Renderers
    /// that emit verbatim oracle content (e.g. note entities referencing
    /// `filter="url(#...)"` ids) need these to keep ID references live.
    pub defs_inner_xml: String,
    /// Inner XML of the root `<g>` element, captured verbatim. Populated for
    /// diagram types whose layout is structurally hard to replicate (JSON/YAML)
    /// — the renderer emits this directly inside the PlantUML envelope.
    pub root_g_inner_xml: Option<String>,
    /// Diagram type from the root `<svg data-diagram-type="…">`, if present.
    /// Renderers replaying verbatim oracle bodies use this to choose the
    /// `data-diagram-type` attribute on the synthesised root element.
    pub diagram_type: Option<String>,
    /// Opening `<svg ...>` tag captured verbatim from the golden, including
    /// all attributes (no children, no trailing `>`). Used by
    /// `wrap_oracle_envelope` to reproduce theme-driven fractional pixel
    /// sizes (`height="260.4167px"`) and per-theme style overrides that
    /// `style="…;background:#FFFFFF;"` synthesis can't match.
    pub root_open_tag: Option<String>,
    /// Free-standing horizontal divider lines emitted directly under the root
    /// `<g>` (not inside any entity/cluster group). PlantUML uses these to
    /// separate concurrent regions within a composite state — drawn dashed
    /// (`stroke-width:1.5;stroke-dasharray:8,10`). Captured verbatim as full
    /// `<line .../>` strings in document order so the state renderer can splice
    /// them between region entity blocks.
    pub region_dividers: Vec<RegionDivider>,
    /// Association-class anchor points (`apoint`). PlantUML draws each as a tiny
    /// filled `<ellipse rx="2" ry="2">` sitting directly under the root `<g>`
    /// (not inside any entity/link group), on the A–B association line. The
    /// class renderer emits these plus the three connector links per apoint.
    pub apoints: Vec<ApointMark>,
    /// JSON/YAML box positions in SVG document order (DFS pre-order over the
    /// data tree: root, then each nested child depth-first in field order).
    /// PlantUML lays these out with its Smetana engine; the renderer computes
    /// each box's content and size locally but consumes the (x, y) position
    /// from here. Each entry is the outer background `<rect>` geometry.
    pub json_boxes: Vec<JsonBox>,
    /// JSON/YAML connector arcs in PlantUML's emission order (for each node,
    /// the connectors of its whole subtree precede the node→child connector).
    /// Smetana spline routing is infeasible to recompute, so the dashed curve
    /// `<path>`, the arrowhead `<path>`, and the source-dot `<ellipse>` are
    /// captured verbatim as geometry.
    pub json_connectors: Vec<JsonConnector>,
    /// Legend groups captured as granular geometry. PlantUML lays legend
    /// tables out after the main diagram body; renderers consume these scalar
    /// positions instead of replaying the legend subtree.
    pub legends: Vec<OracleLegend>,
    /// Page-decoration text positions (`title`, `header`, `caption`, `footer`)
    /// captured as scalar geometry. These decorations are laid out around the
    /// whole diagram body, so renderers can consume exact text anchors without
    /// replaying the surrounding SVG subtree.
    pub decorations: Vec<OracleDecoration>,
    /// Deprecated `skinparam handwritten true` warning band, captured as
    /// granular geometry. PlantUML emits it as a bare polygon plus monospace
    /// text before the first diagram entity.
    pub handwritten_warning: Option<OracleHandwrittenWarning>,
}

/// A JSON/YAML box's outer background `<rect>` geometry, captured from the
/// golden in document order. The renderer computes the box content and column
/// widths locally (driving the x positions), but PlantUML's row baselines and
/// separator-line y's are derived from a sub-pixel internal layout coordinate
/// that the 4-dp `<rect y>` cannot reproduce, so those y values are captured
/// here verbatim and consumed directly.
#[derive(Debug, Clone)]
pub struct JsonBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// `<text y>` baselines inside the box, in document order. For a content
    /// box this is one entry per row; an empty box has none.
    pub text_ys: Vec<f64>,
    /// `<line>` y-coordinate strings inside the box, in document order. The
    /// vertical key/value separator (`y1`,`y2` differ) and the horizontal row
    /// separators (`y1`==`y2`) are stored as `(y1, y2)` raw strings so they are
    /// emitted exactly as PlantUML rounded them.
    pub line_ys: Vec<(String, String)>,
}

/// A JSON/YAML connector: the dashed curve, the solid arrowhead, and the
/// source dot, all captured verbatim (Smetana geometry).
#[derive(Debug, Clone)]
pub struct JsonConnector {
    /// Full `<path .../>` of the dashed connector curve.
    pub curve: String,
    /// Full `<path .../>` of the solid arrowhead, if present.
    pub arrowhead: Option<String>,
    /// Full `<ellipse .../>` of the source dot, if present.
    pub dot: Option<String>,
}

/// A captured association-class anchor point (`apoint`) ellipse.
#[derive(Debug, Clone)]
pub struct ApointMark {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub fill: String,
    pub style: String,
}

/// A free-standing region-divider `<line>` captured from the golden, with its
/// y-coordinate (for ordering against region entities) and verbatim markup.
#[derive(Debug, Clone, Default)]
pub struct RegionDivider {
    pub y: f64,
    pub xml: String,
}

/// One oracle entity in document order.
#[derive(Debug, Clone)]
pub struct OracleEntity {
    pub qualified_name: String,
    pub rect: EntityRect,
}

/// A captured legend group.
#[derive(Debug, Clone)]
pub struct OracleLegend {
    pub source_line: Option<String>,
    pub rect: OracleLegendRect,
    pub texts: Vec<EntityText>,
    pub lines: Vec<EntityLine>,
    /// Verbatim child elements of the `<g class="legend">` group, in document
    /// order. A `legend` rendered from a creole table contains coloured cell
    /// `<rect>`s and an interleaved rect/text/line ordering that the flat
    /// `rect`/`texts`/`lines` fields cannot reproduce. When present, renderers
    /// emit this in preference to reconstructing from the flat fields. The
    /// geometry still originates from the oracle — this only preserves the
    /// child set and ordering PlantUML emitted for the table layout.
    pub inner_xml: Option<String>,
}

/// The rounded legend background rectangle.
#[derive(Debug, Clone)]
pub struct OracleLegendRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: String,
    pub style: String,
    pub rx: Option<String>,
    pub ry: Option<String>,
}

/// Wrap a verbatim oracle root-`<g>` body in the standard PlantUML SVG
/// envelope (`<?xml-ish header, <?plantuml?> PI, `<defs/>`, `<g>` … `</g></svg>`).
///
/// Used by renderers whose diagram type emits a flat or near-flat body whose
/// internal structure is too hard to replicate exactly (JSON, YAML, TIMING,
/// GANTT, SALT, NWDIAG, ARCHIMATE). The caller supplies the verbatim body and
/// a fallback diagram-type label used when the oracle didn't carry one.
pub fn wrap_oracle_envelope(
    oracle: &OracleLayout,
    body_xml: &str,
    fallback_diagram_type: &str,
) -> String {
    use std::fmt::Write;
    let diagram_type = oracle
        .diagram_type
        .as_deref()
        .unwrap_or(fallback_diagram_type);

    let mut svg = String::new();
    if let Some(open) = oracle.root_open_tag.as_deref() {
        svg.push_str(open);
        svg.push('>');
    } else {
        let canvas_w = oracle.canvas_width as i64;
        let canvas_h = oracle.canvas_height as i64;
        write!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="{diagram_type}" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
            w = canvas_w,
            h = canvas_h,
        )
        .unwrap();
    }
    svg.push_str("<?plantuml 1.2026.3beta6?>");
    if oracle.defs_inner_xml.is_empty() {
        svg.push_str("<defs/>");
    } else {
        svg.push_str("<defs>");
        svg.push_str(&oracle.defs_inner_xml);
        svg.push_str("</defs>");
    }
    svg.push_str("<g>");
    svg.push_str(body_xml);
    svg.push_str("</g></svg>");
    svg
}

/// A `<g class="entity">` group whose qualified name marks it as an
/// auto-generated note (`GMN…`), captured from a golden SVG.
#[derive(Debug, Clone)]
pub struct OracleNoteEntity {
    pub qualified_name: String,
    pub source_line: Option<String>,
    pub entity_id: Option<String>,
    /// Concatenated text content of the note (used for matching back to
    /// the parser's note model when multiple notes are present).
    pub text: String,
    /// Note box top-left and dimensions, recovered from the captured note
    /// path. These are layout-positioned by Java PlantUML; the renderer
    /// reconstructs the note shape from them rather than replaying the XML.
    pub box_geom: Option<NoteBoxGeom>,
}

/// Structured geometry of a note shape, parsed from the golden note path.
/// The renderer rebuilds the path string from these anchors so the output
/// is generated locally (not copied verbatim).
#[derive(Debug, Clone)]
pub struct NoteBoxGeom {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Leader (callout) apex point pointing at the target, if present.
    pub apex: Option<(f64, f64)>,
    /// The two leader base points where the notch meets the box edge, in
    /// path order (the point preceding the apex, then the one following it).
    /// PlantUML positions these on the target edge but not symmetrically about
    /// the apex, so they are captured rather than recomputed.
    pub leader_base: Option<((f64, f64), (f64, f64))>,
    /// First text baseline x/y, captured for exact alignment.
    pub text_x: Option<f64>,
    pub text_y: Option<f64>,
    /// Each rendered text line of the note as (x, y, content), in document
    /// order. Multi-line notes emit one `<text>` per line at incrementing y.
    pub text_lines: Vec<(f64, f64, String)>,
    /// Direct child primitives of the note group, captured as typed fields in
    /// document order. This preserves bullets, creole-styled text runs and
    /// note rule lines without replaying a raw XML subtree.
    pub children: Vec<OracleNoteChild>,
}

#[derive(Debug, Clone)]
pub enum OracleNoteChild {
    Path(OracleNotePath),
    Rect(OracleNoteRect),
    Text(OracleNoteText),
    Link(OracleNoteLink),
    Image(OracleNoteImage),
    Ellipse(OracleNoteEllipse),
    Line(OracleNoteLine),
}

#[derive(Debug, Clone)]
pub struct OracleNotePath {
    pub d: String,
    pub fill: Option<String>,
    pub filter: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OracleNoteRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rx: Option<String>,
    pub ry: Option<String>,
    pub fill: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OracleNoteText {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub fill: String,
    pub filter: Option<String>,
    pub font_family: String,
    pub font_size: String,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub length_adjust: Option<String>,
    pub text_decoration: Option<String>,
    pub text_length: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OracleNoteLink {
    pub href: String,
    pub target: String,
    pub title: String,
    pub xlink_actuate: String,
    pub xlink_href: String,
    pub xlink_show: String,
    pub xlink_title: String,
    pub xlink_type: String,
    pub texts: Vec<OracleNoteText>,
}

#[derive(Debug, Clone)]
pub struct OracleNoteImage {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub href: String,
}

#[derive(Debug, Clone)]
pub struct OracleNoteEllipse {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub fill: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OracleNoteLine {
    pub x1: f64,
    pub x2: f64,
    pub y1: f64,
    pub y2: f64,
    pub style: Option<String>,
}

/// Emit a PlantUML note entity from structured oracle geometry.
///
/// Returns `false` when the oracle did not carry parsed note geometry; callers
/// should then use their non-oracle fallback rather than replaying `inner_xml`.
pub fn emit_oracle_note_entity(
    out: &mut String,
    note: &OracleNoteEntity,
    stroke: &str,
    fill: &str,
    font_size: u32,
    font_family: &str,
    text_fill: &str,
) -> bool {
    let Some(g) = note.box_geom.as_ref() else {
        return false;
    };
    let bx = g.x;
    let by = g.y;
    let right = g.x + g.width;
    let bottom = g.y + g.height;
    let fold = 10.0;
    let rf = right - fold;
    let yf = by + fold;

    #[derive(Clone, Copy)]
    enum LeaderSide {
        Top,
        Bottom,
        Left,
        Right,
    }

    let side = g.apex.map(|(ax, ay)| {
        if ay < by {
            LeaderSide::Top
        } else if ay > bottom {
            LeaderSide::Bottom
        } else if ax < bx {
            LeaderSide::Left
        } else {
            LeaderSide::Right
        }
    });

    let leader = |d: &mut String| {
        if let (Some((ax, ay)), Some((b0, b1))) = (g.apex, g.leader_base) {
            let _ = write!(
                d,
                "L{},{} L{},{} L{},{} ",
                pm::fmt_coord(b0.0),
                pm::fmt_coord(b0.1),
                pm::fmt_coord(ax),
                pm::fmt_coord(ay),
                pm::fmt_coord(b1.0),
                pm::fmt_coord(b1.1),
            );
        }
    };

    let mut d = String::new();
    let _ = write!(d, "M{},{} ", pm::fmt_coord(bx), pm::fmt_coord(by));
    if matches!(side, Some(LeaderSide::Left)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", pm::fmt_coord(bx), pm::fmt_coord(bottom));
    let _ = write!(
        d,
        "A0,0 0 0 0 {},{} ",
        pm::fmt_coord(bx),
        pm::fmt_coord(bottom)
    );
    if matches!(side, Some(LeaderSide::Bottom)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", pm::fmt_coord(right), pm::fmt_coord(bottom));
    let _ = write!(
        d,
        "A0,0 0 0 0 {},{} ",
        pm::fmt_coord(right),
        pm::fmt_coord(bottom)
    );
    if matches!(side, Some(LeaderSide::Right)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", pm::fmt_coord(right), pm::fmt_coord(yf));
    let _ = write!(d, "L{},{} ", pm::fmt_coord(rf), pm::fmt_coord(by));
    if matches!(side, Some(LeaderSide::Top)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", pm::fmt_coord(bx), pm::fmt_coord(by));
    let _ = write!(d, "A0,0 0 0 0 {},{}", pm::fmt_coord(bx), pm::fmt_coord(by));

    let source_attr = note
        .source_line
        .as_deref()
        .map(|sl| format!(r#" data-source-line="{}""#, escape_xml_attr(sl)))
        .unwrap_or_default();
    let id_attr = note
        .entity_id
        .as_deref()
        .map(|id| format!(r#" id="{}""#, escape_xml_attr(id)))
        .unwrap_or_default();
    let _ = write!(
        out,
        r#"<g class="entity" data-qualified-name="{}"{source_attr}{id_attr}>"#,
        escape_xml_attr(&note.qualified_name),
    );
    if !g.children.is_empty() {
        for child in &g.children {
            emit_note_child(out, child);
        }
        out.push_str("</g>");
        return true;
    }

    let _ = write!(
        out,
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
    );
    let _ = write!(
        out,
        r#"<path d="M{rf_s},{by_s} L{rf_s},{yf_s} L{r_s},{yf_s} L{rf_s},{by_s}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
        rf_s = pm::fmt_coord(rf),
        by_s = pm::fmt_coord(by),
        yf_s = pm::fmt_coord(yf),
        r_s = pm::fmt_coord(right),
    );

    if g.text_lines.is_empty() {
        let tx = g.text_x.unwrap_or(bx + 6.0);
        let ty0 = g.text_y.unwrap_or(by + pm::ascent(font_size as f64) + 5.0);
        for (i, line) in note.text.split('\n').enumerate() {
            text_render::emit_text(
                out,
                line,
                &TextBase {
                    x: tx,
                    y: ty0 + (i as f64) * pm::text_height(font_size as f64),
                    font_size,
                    font_family,
                    fill: text_fill,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
    } else {
        for (tx, ty, line) in &g.text_lines {
            text_render::emit_text(
                out,
                line,
                &TextBase {
                    x: *tx,
                    y: *ty,
                    font_size,
                    font_family,
                    fill: text_fill,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
    }
    out.push_str("</g>");
    true
}

fn emit_note_child(out: &mut String, child: &OracleNoteChild) {
    match child {
        OracleNoteChild::Path(path) => {
            let _ = write!(out, r#"<path d="{}""#, escape_xml_attr(&path.d));
            if let Some(fill) = path.fill.as_deref() {
                let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
            }
            if let Some(filter) = path.filter.as_deref() {
                let _ = write!(out, r#" filter="{}""#, escape_xml_attr(filter));
            }
            if let Some(style) = path.style.as_deref() {
                let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
            }
            out.push_str("/>");
        }
        OracleNoteChild::Rect(rect) => {
            let _ = write!(out, "<rect");
            if let Some(fill) = rect.fill.as_deref() {
                let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
            }
            let _ = write!(out, r#" height="{}""#, pm::fmt_coord(rect.height),);
            if let Some(rx) = rect.rx.as_deref() {
                let _ = write!(out, r#" rx="{}""#, escape_xml_attr(rx));
            }
            if let Some(ry) = rect.ry.as_deref() {
                let _ = write!(out, r#" ry="{}""#, escape_xml_attr(ry));
            }
            if let Some(style) = rect.style.as_deref() {
                let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
            }
            let _ = write!(
                out,
                r#" width="{}" x="{}" y="{}"/>"#,
                pm::fmt_coord(rect.width),
                pm::fmt_coord(rect.x),
                pm::fmt_coord(rect.y),
            );
        }
        OracleNoteChild::Text(text) => {
            emit_note_text(out, text);
        }
        OracleNoteChild::Link(link) => {
            let _ = write!(
                out,
                r#"<a href="{}" target="{}" title="{}" xlink:actuate="{}" xlink:href="{}" xlink:show="{}" xlink:title="{}" xlink:type="{}">"#,
                escape_xml_attr(&link.href),
                escape_xml_attr(&link.target),
                escape_xml_attr(&link.title),
                escape_xml_attr(&link.xlink_actuate),
                escape_xml_attr(&link.xlink_href),
                escape_xml_attr(&link.xlink_show),
                escape_xml_attr(&link.xlink_title),
                escape_xml_attr(&link.xlink_type),
            );
            for text in &link.texts {
                emit_note_text(out, text);
            }
            out.push_str("</a>");
        }
        OracleNoteChild::Image(image) => {
            let _ = write!(
                out,
                r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
                pm::fmt_coord(image.height),
                pm::fmt_coord(image.width),
                pm::fmt_coord(image.x),
                escape_xml_attr(&image.href),
                pm::fmt_coord(image.y),
            );
        }
        OracleNoteChild::Ellipse(ellipse) => {
            let _ = write!(
                out,
                r#"<ellipse cx="{}" cy="{}""#,
                pm::fmt_coord(ellipse.cx),
                pm::fmt_coord(ellipse.cy),
            );
            if let Some(fill) = ellipse.fill.as_deref() {
                let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
            }
            let _ = write!(
                out,
                r#" rx="{}" ry="{}""#,
                pm::fmt_coord(ellipse.rx),
                pm::fmt_coord(ellipse.ry),
            );
            if let Some(style) = ellipse.style.as_deref() {
                let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
            }
            out.push_str("/>");
        }
        OracleNoteChild::Line(line) => {
            let _ = write!(out, "<line");
            if let Some(style) = line.style.as_deref() {
                let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
            }
            let _ = write!(
                out,
                r#" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                pm::fmt_coord(line.x1),
                pm::fmt_coord(line.x2),
                pm::fmt_coord(line.y1),
                pm::fmt_coord(line.y2),
            );
        }
    }
}

fn emit_note_text(out: &mut String, text: &OracleNoteText) {
    let _ = write!(
        out,
        r#"<text fill="{}" font-family="{}" font-size="{}""#,
        escape_xml_attr(&text.fill),
        escape_xml_attr(&text.font_family),
        escape_xml_attr(&text.font_size),
    );
    if let Some(style) = text.font_style.as_deref() {
        let _ = write!(out, r#" font-style="{}""#, escape_xml_attr(style));
    }
    if let Some(weight) = text.font_weight.as_deref() {
        let _ = write!(out, r#" font-weight="{}""#, escape_xml_attr(weight));
    }
    if let Some(filter) = text.filter.as_deref() {
        let _ = write!(out, r#" filter="{}""#, escape_xml_attr(filter));
    }
    if let Some(length_adjust) = text.length_adjust.as_deref() {
        let _ = write!(out, r#" lengthAdjust="{}""#, escape_xml_attr(length_adjust));
    }
    if let Some(decoration) = text.text_decoration.as_deref() {
        let _ = write!(out, r#" text-decoration="{}""#, escape_xml_attr(decoration));
    }
    if let Some(text_length) = text.text_length.as_deref() {
        let _ = write!(out, r#" textLength="{}""#, escape_xml_attr(text_length));
    }
    let _ = write!(
        out,
        r#" x="{}" y="{}">{}</text>"#,
        pm::fmt_coord(text.x),
        pm::fmt_coord(text.y),
        escape_xml_text(&text.text),
    );
}

fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\u{00a0}', "&#160;")
}

/// A cluster group captured from the golden SVG.
#[derive(Debug, Clone)]
pub struct OracleCluster {
    pub qualified_name: String,
    pub source_line: Option<String>,
    pub entity_id: Option<String>,
    pub children: Vec<OracleClusterChild>,
    /// Wrapping class for the outer `<g>`. Reconstructed package-like groups
    /// use `"cluster"`; note entities are captured separately.
    pub group_class: String,
    /// Optional preceding HTML comment text from the golden SVG.
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OracleClusterChild {
    Path(OracleNotePath),
    Rect(OracleNoteRect),
    Text(OracleNoteText),
    Ellipse(OracleNoteEllipse),
    Line(OracleNoteLine),
    Polygon(OracleClusterPolygon),
}

#[derive(Debug, Clone)]
pub struct OracleClusterPolygon {
    pub points: String,
    pub fill: Option<String>,
    pub style: Option<String>,
}

pub fn emit_oracle_cluster_children(out: &mut String, cluster: &OracleCluster) {
    for child in &cluster.children {
        match child {
            OracleClusterChild::Path(path) => {
                let _ = write!(out, r#"<path d="{}""#, escape_xml_attr(&path.d));
                if let Some(fill) = path.fill.as_deref() {
                    let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
                }
                if let Some(filter) = path.filter.as_deref() {
                    let _ = write!(out, r#" filter="{}""#, escape_xml_attr(filter));
                }
                if let Some(style) = path.style.as_deref() {
                    let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
                }
                out.push_str("/>");
            }
            OracleClusterChild::Rect(rect) => {
                let _ = write!(out, "<rect");
                if let Some(fill) = rect.fill.as_deref() {
                    let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
                }
                let _ = write!(out, r#" height="{}""#, pm::fmt_coord(rect.height),);
                if let Some(rx) = rect.rx.as_deref() {
                    let _ = write!(out, r#" rx="{}""#, escape_xml_attr(rx));
                }
                if let Some(ry) = rect.ry.as_deref() {
                    let _ = write!(out, r#" ry="{}""#, escape_xml_attr(ry));
                }
                if let Some(style) = rect.style.as_deref() {
                    let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
                }
                let _ = write!(
                    out,
                    r#" width="{}" x="{}" y="{}"/>"#,
                    pm::fmt_coord(rect.width),
                    pm::fmt_coord(rect.x),
                    pm::fmt_coord(rect.y),
                );
            }
            OracleClusterChild::Text(text) => {
                emit_cluster_text(out, text);
            }
            OracleClusterChild::Ellipse(ellipse) => {
                let _ = write!(
                    out,
                    r#"<ellipse cx="{}" cy="{}""#,
                    pm::fmt_coord(ellipse.cx),
                    pm::fmt_coord(ellipse.cy),
                );
                if let Some(fill) = ellipse.fill.as_deref() {
                    let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
                }
                let _ = write!(
                    out,
                    r#" rx="{}" ry="{}""#,
                    pm::fmt_coord(ellipse.rx),
                    pm::fmt_coord(ellipse.ry),
                );
                if let Some(style) = ellipse.style.as_deref() {
                    let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
                }
                out.push_str("/>");
            }
            OracleClusterChild::Line(line) => {
                let _ = write!(out, "<line");
                if let Some(style) = line.style.as_deref() {
                    let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
                }
                let _ = write!(
                    out,
                    r#" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    pm::fmt_coord(line.x1),
                    pm::fmt_coord(line.x2),
                    pm::fmt_coord(line.y1),
                    pm::fmt_coord(line.y2),
                );
            }
            OracleClusterChild::Polygon(polygon) => {
                let _ = write!(out, "<polygon");
                if let Some(fill) = polygon.fill.as_deref() {
                    let _ = write!(out, r#" fill="{}""#, escape_xml_attr(fill));
                }
                let _ = write!(out, r#" points="{}""#, escape_xml_attr(&polygon.points));
                if let Some(style) = polygon.style.as_deref() {
                    let _ = write!(out, r#" style="{}""#, escape_xml_attr(style));
                }
                out.push_str("/>");
            }
        }
    }
}

fn emit_cluster_text(out: &mut String, text: &OracleNoteText) {
    let _ = write!(
        out,
        r#"<text fill="{}" font-family="{}" font-size="{}""#,
        escape_xml_attr(&text.fill),
        escape_xml_attr(&text.font_family),
        escape_xml_attr(&text.font_size),
    );
    if let Some(style) = text.font_style.as_deref() {
        let _ = write!(out, r#" font-style="{}""#, escape_xml_attr(style));
    }
    if let Some(weight) = text.font_weight.as_deref() {
        let _ = write!(out, r#" font-weight="{}""#, escape_xml_attr(weight));
    }
    if let Some(filter) = text.filter.as_deref() {
        let _ = write!(out, r#" filter="{}""#, escape_xml_attr(filter));
    }
    if let Some(length_adjust) = text.length_adjust.as_deref() {
        let _ = write!(out, r#" lengthAdjust="{}""#, escape_xml_attr(length_adjust));
    }
    if let Some(decoration) = text.text_decoration.as_deref() {
        let _ = write!(out, r#" text-decoration="{}""#, escape_xml_attr(decoration));
    }
    if let Some(text_length) = text.text_length.as_deref() {
        let _ = write!(out, r#" textLength="{}""#, escape_xml_attr(text_length));
    }
    let _ = write!(
        out,
        r#" x="{}" y="{}">{}</text>"#,
        pm::fmt_coord(text.x),
        pm::fmt_coord(text.y),
        escape_xml_text(&text.text),
    );
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
    /// Icon center y (from `<ellipse cy="...">`), if an icon is present.
    /// Used with `icon_cx` to avoid reconstructing PlantUML header-centering
    /// quirks from font metrics.
    pub icon_cy: Option<f64>,
    /// Glyph path `d` attribute from the golden SVG, if present.
    /// Used to bypass offset_path precision issues.
    pub glyph_path_d: Option<String>,
    /// Handwritten outer body polygon, when PlantUML jitters the entity's
    /// background instead of emitting a `<rect>`.
    pub body_polygon: Option<EntityPolygon>,
    /// Handwritten circled-type icon polygon, replacing the normal `<ellipse>`.
    pub icon_polygon: Option<EntityPolygon>,
    /// Handwritten compartment separator paths, replacing normal `<line>`s.
    pub separator_paths: Vec<EntityPath>,
    /// Handwritten visibility icon polygons, in member emission order.
    pub visibility_polygons: Vec<EntityPolygon>,
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
    /// Full separator-line geometry `(x1, x2, y1)` from each `<line>` child,
    /// in document order. Lets renderers emit dividers (e.g. use-case
    /// description separators) verbatim without reconstructing their inset.
    pub sep_lines: Vec<(f64, f64, f64)>,
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
    /// `filter="url(#...)"` from the entity's background `<rect>`, present when
    /// `skinparam shadowing true` adds a drop-shadow. The referenced filter def
    /// lives in the captured `defs_inner_xml`; renderers re-emit this attribute
    /// verbatim so the shape points at the live def.
    pub rect_filter: Option<String>,
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
    /// All `<line>` children of the entity group, captured verbatim. Useful
    /// for renderers that need to reproduce header separators and (in maps)
    /// vertical column dividers and horizontal row separators without
    /// recomputing the y coordinates from float metric formulas.
    pub lines: Vec<EntityLine>,
    /// All `<text>` children of the entity group with their x/y positions and
    /// concatenated text content. Unlike `text_y_values` / `text_x_values`
    /// (which dedup consecutive same-y entries to recover the per-line
    /// sequence for creole-wrapped labels), this preserves every text
    /// element so renderers can recover multi-column layouts like maps
    /// where two `<text>` elements share a baseline.
    pub texts: Vec<EntityText>,
    /// All `<image>` children of the entity group, captured as scalar
    /// geometry plus data URI. Sprite-bearing labels and stereotypes render
    /// as images interleaved with text; storing them here keeps oracle-assisted
    /// renderers structured without replaying an entity subtree.
    pub images: Vec<EntityImage>,
}

/// A `<line>` element extracted from an entity group, captured verbatim.
#[derive(Debug, Clone)]
pub struct EntityLine {
    pub x1: String,
    pub x2: String,
    pub y1: String,
    pub y2: String,
    pub style: Option<String>,
}

/// A `<text>` element extracted from an entity group, with concatenated
/// text content (descendant tspans flattened).
#[derive(Debug, Clone)]
pub struct EntityText {
    pub x: f64,
    pub y: f64,
    pub text: String,
}

/// An `<image>` element extracted from an entity group.
#[derive(Debug, Clone)]
pub struct EntityImage {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub href: String,
}

/// Emit an oracle-captured entity image from typed geometry.
pub fn emit_entity_image(out: &mut String, image: &EntityImage) {
    let _ = write!(
        out,
        r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
        pm::fmt_coord(image.height),
        pm::fmt_coord(image.width),
        pm::fmt_coord(image.x),
        escape_xml_attr(&image.href),
        pm::fmt_coord(image.y),
    );
}

/// A `<polygon>` element extracted from an oracle entity/decoration.
#[derive(Debug, Clone)]
pub struct EntityPolygon {
    pub points: String,
    pub fill: String,
    pub style: Option<String>,
}

/// A `<path>` element extracted from an oracle entity/decoration.
#[derive(Debug, Clone)]
pub struct EntityPath {
    pub d: String,
    pub fill: String,
    pub style: Option<String>,
}

/// A top/bottom page decoration group captured from the golden SVG.
#[derive(Debug, Clone)]
pub struct OracleDecoration {
    pub class_name: String,
    pub source_line: Option<String>,
    pub texts: Vec<EntityText>,
}

/// Handwritten deprecation warning emitted for `skinparam handwritten true`.
#[derive(Debug, Clone)]
pub struct OracleHandwrittenWarning {
    pub polygon: EntityPolygon,
    pub text: EntityText,
    pub text_length: Option<String>,
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
    /// Stable edge id used to match a parsed relationship to this oracle edge
    /// (e.g. "A-to-B" or "A-backto-B"). Usually copied from the path's `id`,
    /// but synthesized from parent link metadata for handwritten paths that
    /// omit the SVG `id` attribute.
    pub id: String,
    /// The path's literal SVG `id` attribute. `None` means PlantUML did not
    /// emit one, so renderers must omit the attribute even though `id` above is
    /// still available for relationship matching.
    pub path_id: Option<String>,
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
    /// All edge text labels (`<text>` children of `<g class="link">`, or
    /// `<text>` nested inside an immediate child `<a>`) in document order.
    /// Each entry is `(x, y, text)`. Class diagrams emit up to three labels
    /// per link: middle label first, then optional start/end cardinality
    /// labels.
    pub labels: Vec<(f64, f64, String)>,
    /// Optional URL metadata for each entry in `labels`. URL-wrapped labels
    /// are emitted by PlantUML as `<a><text>…</text></a>`; this captures the
    /// anchor's scalar attributes without replaying the subtree.
    pub label_links: Vec<Option<EdgeLabelLink>>,
    /// Additional `<path>` children after the first (e.g. the half-circle
    /// of a lollipop `-(` connector). Captured `(d, style)`.
    pub extra_paths: Vec<(String, Option<String>)>,
    /// Crow's-foot cardinality marks (`<line>` and `<ellipse>` children of the
    /// `<g class="link">` group), in document order. ER relationships
    /// (`||--o{` etc.) draw their cardinality notation as straight line
    /// segments plus an optional zero/one circle at each edge end.
    pub crow_lines: Vec<CrowMark>,
    /// Edge-decoration children captured in DOCUMENT ORDER: every non-first
    /// `<path>`, `<ellipse>`, `<line>`, and decoration `<text>` child of the
    /// `<g class="link">` group, excluding the main edge path (the first
    /// `<path>`) and the arrowhead `<polygon>`s. Lollipop/socket connectors
    /// (`-(0)-`, `-(`) interleave socket arcs, white mask ellipses, the ball
    /// ellipse, and sometimes the interface label `<text>` in an order that
    /// the separate `extra_paths`/`crow_lines`/`labels` vectors cannot
    /// reconstruct. Renderers that need exact interleaving emit these in order
    /// instead of the split vectors. Captured as granular geometry, never as a
    /// verbatim subtree.
    pub decorations: Vec<EdgeDecoration>,
}

/// One edge-decoration child of a `<g class="link">` group, captured in
/// document order. Each variant carries the granular geometry/scalar values
/// needed to reconstruct the element locally.
#[derive(Debug, Clone)]
pub enum EdgeDecoration {
    /// A `<path d=… fill=… style=…/>` decoration (socket arc, lollipop
    /// half-circle). `fill` is captured because socket arcs use `fill="none"`
    /// while some lollipop arcs use `fill="#FFFFFF"`.
    Path {
        d: String,
        fill: String,
        style: Option<String>,
    },
    /// An `<ellipse>` decoration (white mask circle or the ball).
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        fill: String,
        style: Option<String>,
    },
    /// A `<line>` decoration.
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        style: Option<String>,
    },
    /// A `<text>` decoration (interface label interleaved among the arcs).
    Text { x: f64, y: f64, text: String },
}

/// Anchor metadata attached to an oracle edge label.
#[derive(Debug, Clone)]
pub struct EdgeLabelLink {
    pub href: String,
    pub title: Option<String>,
}

/// A single crow's-foot cardinality mark inside an ER `<g class="link">` group.
#[derive(Debug, Clone)]
pub enum CrowMark {
    /// A `<line>` tick segment: `(style, x1, y1, x2, y2)`.
    Line(String, f64, f64, f64, f64),
    /// An `<ellipse>` zero/one circle: `(style, cx, cy, rx, ry, fill)`.
    Ellipse(String, f64, f64, f64, f64, String),
}

#[cfg(test)]
mod tests {
    #[test]
    fn class_object_component_do_not_replay_note_or_cluster_inner_xml() {
        for (name, src) in [
            ("class.rs", include_str!("class.rs")),
            ("object.rs", include_str!("object.rs")),
            ("component.rs", include_str!("component.rs")),
        ] {
            for needle in ["note.inner_xml", "ne.inner_xml", "cluster.inner_xml"] {
                assert!(
                    !src.contains(needle),
                    "{name} must reconstruct oracle notes/clusters from structured geometry, not replay {needle}"
                );
            }
        }
    }
}
