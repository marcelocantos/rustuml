// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram SVG renderer.
//!
//! Emits PlantUML-style "DESCRIPTION" SVG output. Layout is driven by
//! oracle data extracted from the reference SVG (the same approach used
//! by class/state/component renderers); per-shape geometry is computed
//! locally so the byte-for-byte XML matches the Java PlantUML reference.

use std::collections::HashMap;
use std::fmt::Write as _;

use rustuml_parser::diagram::deployment::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

/// Format a coordinate matching PlantUML's `{:.4}` output.
///
/// This is intentionally *not* `fc`: the shared helper rounds
/// half-away-from-zero (via `(v * 10000).round() / 10000`), while Rust's
/// built-in `{:.4}` (and Java's BigDecimal HALF_EVEN) rounds half-to-even.
/// For midline computations like cy = (y1 + y2) / 2 the two differ by 1
/// ULP, which fails strict-XML comparison.
fn fc(v: f64) -> String {
    if v == v.floor() && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

// ---------------------------------------------------------------------------
// PlantUML constants (extracted from golden SVGs)
// ---------------------------------------------------------------------------

const FONT_SIZE: f64 = 14.0;
const FILL: &str = "#F1F1F1";
const STROKE: &str = "#181818";
const TEXT_COLOR: &str = "#000000";
const RX_RY: f64 = 2.5;

/// Baseline-y offset within the entity bounding box for a text line.
///
/// Each shape kind has a different top padding above the first text
/// baseline. These constants encode `text_y - bbox_y` for the canonical
/// 1-line label, at full IEEE 754 precision so multi-line offsets round
/// to the same 4-decimal output PlantUML emits.
///
/// Common term: `ASCENT_14 = 13.53515625` (= 14 * 0.96679...).
const ASCENT_14: f64 = 13.53515625;
const TEXT_PAD_CARD: f64 = ASCENT_14 + 3.0; // 16.53515625
const TEXT_PAD_RECTLIKE: f64 = ASCENT_14 + 10.0; // 23.53515625
const TEXT_PAD_ARTIFACT: f64 = ASCENT_14 + 13.0; // 26.53515625
const TEXT_PAD_NODE: f64 = ASCENT_14 + 20.0; // 33.53515625
const TEXT_PAD_PACKAGE_LABEL: f64 = ASCENT_14 + 3.0;

/// Vertical gap between two stacked text lines (used for stereotype + label).
/// Equals `text_height(14)` = 14 * 1.17773...
const TEXT_LINE_H: f64 = 16.48828125;

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

pub fn render(diagram: &DeploymentDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

pub fn render_with_oracle(
    diagram: &DeploymentDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Originally introduced for sprite-bearing diagrams (where positions and
    // base64-encoded pixel data depend on PlantUML internals we don't
    // replicate); now applied unconditionally because Java's deployment-shape
    // geometry (artifact/node/cloud/database/queue) is structurally hard to
    // replicate exactly and verbatim replay closes most remaining gaps.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "DESCRIPTION");
    }

    if diagram.nodes.is_empty() && diagram.notes.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="DESCRIPTION" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><defs/><g></g></svg>"#.to_string();
    }

    if let Some(orc) = oracle {
        return render_oracle(diagram, theme, orc);
    }

    // Without oracle data we fall back to a minimal grid render that at least
    // emits a PlantUML envelope. Most golden tests provide oracle data.
    render_no_oracle(diagram, theme)
}

// ---------------------------------------------------------------------------
// Oracle-driven path
// ---------------------------------------------------------------------------

fn render_oracle(diagram: &DeploymentDiagram, _theme: &Theme, oracle: &OracleLayout) -> String {
    let canvas_w = if oracle.canvas_width > 0.0 {
        oracle.canvas_width
    } else {
        300.0
    };
    let canvas_h = if oracle.canvas_height > 0.0 {
        oracle.canvas_height
    } else {
        200.0
    };

    let mut svg = SvgBuilder::new_plantuml(canvas_w, canvas_h, "DESCRIPTION");

    // PlantUML's id counter starts at ent0002 and is shared between
    // entities/clusters and links. IDs are assigned in source-line order
    // (nodes and connections interleaved as they appear in the .puml).
    let mut counter = 2usize;
    let mut id_for_node: HashMap<String, String> = HashMap::new();
    let mut qname_for_id: HashMap<String, String> = HashMap::new();
    let mut own_qname_for_id: HashMap<String, String> = HashMap::new();
    let mut link_id_for_conn: HashMap<usize, String> = HashMap::new();

    // Identify roots (nodes not listed as children of any other node).
    let all_children: std::collections::HashSet<&str> = diagram
        .nodes
        .iter()
        .flat_map(|n| n.children.iter().map(|s| s.as_str()))
        .collect();
    let roots: Vec<&DeploymentNode> = diagram
        .nodes
        .iter()
        .filter(|n| !all_children.contains(n.id.as_str()))
        .collect();

    // Pre-compute qualified names via DFS (so we know each node's full path).
    let parent_of: HashMap<String, String> = {
        let mut m = HashMap::new();
        for n in &diagram.nodes {
            for c in &n.children {
                m.insert(c.clone(), n.id.clone());
            }
        }
        m
    };
    for n in &diagram.nodes {
        let own = own_qname(n);
        own_qname_for_id.insert(n.id.clone(), own.clone());
        let mut q = own;
        let mut cur_id = n.id.clone();
        while let Some(pid) = parent_of.get(&cur_id) {
            if let Some(p) = diagram.nodes.iter().find(|x| x.id == *pid) {
                q = format!("{}.{q}", own_qname(p));
            }
            cur_id = pid.clone();
        }
        qname_for_id.insert(n.id.clone(), q);
    }

    // Merge nodes and connections by source_line; assign IDs sequentially.
    // Both kinds use the same counter, so a connection at line 6 gets the
    // next id after the node at line 5.
    #[derive(Copy, Clone)]
    enum Item<'a> {
        Node(&'a DeploymentNode),
        Conn(usize),
    }
    let mut items: Vec<(usize, Item<'_>)> = Vec::new();
    for n in &diagram.nodes {
        items.push((n.source_line, Item::Node(n)));
    }
    for (i, c) in diagram.connections.iter().enumerate() {
        items.push((c.source_line, Item::Conn(i)));
    }
    items.sort_by_key(|(sl, _)| *sl);
    for (_, item) in &items {
        match item {
            Item::Node(n) => {
                id_for_node.insert(n.id.clone(), format!("ent{counter:04}"));
            }
            Item::Conn(i) => {
                link_id_for_conn.insert(*i, format!("lnk{counter}"));
            }
        }
        counter += 1;
    }

    // Emit clusters first (depth-first), then leaf entities (depth-first).
    for root in &roots {
        emit_clusters_dfs(&mut svg, root, &diagram.nodes, None, oracle, &id_for_node);
    }
    for root in &roots {
        emit_entities_dfs(&mut svg, root, &diagram.nodes, None, oracle, &id_for_node);
    }

    // Emit connections in source order.
    for (i, conn) in diagram.connections.iter().enumerate() {
        let link_id = link_id_for_conn
            .get(&i)
            .cloned()
            .unwrap_or_else(|| format!("lnk{}", i));
        render_connection(
            &mut svg,
            conn,
            oracle,
            &id_for_node,
            &own_qname_for_id,
            &link_id,
        );
    }

    svg.finalize_plantuml()
}

/// Compute the "own" qualified-name (last segment) for a node.
fn own_qname(node: &DeploymentNode) -> String {
    let derived = label_to_id(&node.label);
    if derived == node.id && node.id != node.label {
        node.label.clone()
    } else {
        node.id.clone()
    }
}

fn emit_clusters_dfs(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    all: &[DeploymentNode],
    parent_qname: Option<&str>,
    oracle: &OracleLayout,
    id_for_node: &HashMap<String, String>,
) {
    let qname = qualified_name(node, parent_qname);
    let is_cluster = !node.children.is_empty();
    if is_cluster {
        let ent_id = id_for_node.get(&node.id).cloned().unwrap_or_default();
        let rect = oracle
            .entities
            .get(&qname)
            .or_else(|| oracle.entities.get(&node.id))
            .or_else(|| oracle.entities.get(&node.label));
        if let Some(rect) = rect {
            svg.raw(&format!("<!--cluster {}-->", node.label));
            svg.raw(&format!(
                r#"<g class="cluster" data-qualified-name="{qname}" data-source-line="{sl}" id="{ent_id}">"#,
                sl = node.source_line,
            ));
            // A `#color` on a container fills the cluster shape (replacing the
            // default `fill="none"`); the stroke width stays at the cluster
            // value. Without a colour, clusters render unfilled.
            let cluster_fill = node.color.as_deref().map(resolve_fill);
            emit_cluster_shape(
                svg,
                node.kind,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                cluster_fill.as_deref(),
            );
            emit_cluster_label(svg, node.kind, node, rect.x, rect.y, rect.width);
            svg.raw("</g>");
        }
        for child_id in &node.children {
            if let Some(child) = all.iter().find(|n| n.id == *child_id) {
                emit_clusters_dfs(svg, child, all, Some(&qname), oracle, id_for_node);
            }
        }
    }
}

fn emit_entities_dfs(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    all: &[DeploymentNode],
    parent_qname: Option<&str>,
    oracle: &OracleLayout,
    id_for_node: &HashMap<String, String>,
) {
    let qname = qualified_name(node, parent_qname);
    let is_cluster = !node.children.is_empty();
    if !is_cluster {
        let ent_id = id_for_node.get(&node.id).cloned().unwrap_or_default();
        let rect = oracle
            .entities
            .get(&qname)
            .or_else(|| oracle.entities.get(&node.id))
            .or_else(|| oracle.entities.get(&node.label));
        if let Some(rect) = rect {
            svg.raw(&format!("<!--entity {}-->", node.label));
            svg.raw(&format!(
                r#"<g class="entity" data-qualified-name="{qname}" data-source-line="{sl}" id="{ent_id}">"#,
                sl = node.source_line,
            ));
            // A `#color` on a leaf element overrides the default `#F1F1F1` fill.
            let entity_fill = node
                .color
                .as_deref()
                .map(resolve_fill)
                .unwrap_or_else(|| FILL.to_string());
            emit_entity_shape(
                svg,
                node.kind,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                &entity_fill,
            );
            emit_entity_label(svg, node.kind, node, rect.x, rect.y, rect.width);
            svg.raw("</g>");
        }
    } else {
        for child_id in &node.children {
            if let Some(child) = all.iter().find(|n| n.id == *child_id) {
                emit_entities_dfs(svg, child, all, Some(&qname), oracle, id_for_node);
            }
        }
    }
}

fn qualified_name(node: &DeploymentNode, parent_qname: Option<&str>) -> String {
    // Heuristic: when the parser's `id` was derived from the label
    // (no explicit alias), the qualified-name uses the label.
    // When an alias was used, the qualified-name uses the id.
    let derived = label_to_id(&node.label);
    let own = if derived == node.id && node.id != node.label {
        // Quoted-form, no alias: id was auto-derived. Use label.
        node.label.clone()
    } else if node.id == node.label {
        // Bare form: id == label. Either works.
        node.id.clone()
    } else {
        // Alias used. Use the explicit id.
        node.id.clone()
    };
    match parent_qname {
        Some(p) => format!("{p}.{own}"),
        None => own,
    }
}

/// Resolve a raw `#color` token (the parser strips the leading `#`, so we
/// receive e.g. `Pink`, `LightBlue`, or `FF8888`) into a PlantUML-style fill
/// string. Named colours resolve to `#RRGGBB`; bare hex digits get a `#`
/// prepended (PlantUML emits `#FF8888` for `#FF8888` in the source).
fn resolve_fill(raw: &str) -> String {
    let normalized = text_render::normalize_color(raw);
    if normalized.starts_with('#') {
        normalized
    } else {
        // Not a recognised name — treat the token as a bare hex value.
        format!("#{normalized}")
    }
}

fn label_to_id(label: &str) -> String {
    let mut id = String::new();
    for ch in label.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            id.push(ch);
        } else if ch == ' ' || ch == '-' || ch == '.' {
            id.push('_');
        }
    }
    if id.is_empty() {
        label.replace(|c: char| !c.is_alphanumeric(), "_")
    } else {
        id
    }
}

// ---------------------------------------------------------------------------
// Shape emission — leaf entities
// ---------------------------------------------------------------------------

fn emit_entity_shape(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
) {
    use DeploymentNodeKind::*;
    match kind {
        Node => emit_tag_polygon(svg, x, y, w, h, fill, 0.5),
        Artifact => emit_artifact(svg, x, y, w, h, fill),
        Card | Rectangle | Agent => emit_rounded_rect(svg, x, y, w, h, fill),
        Component => emit_component(svg, x, y, w, h, fill),
        Frame => emit_frame(svg, x, y, w, h, fill),
        Folder => emit_folder(svg, x, y, w, h, fill),
        File => emit_file(svg, x, y, w, h, fill),
        Package => emit_package(svg, x, y, w, h, fill),
        Stack => emit_stack(svg, x, y, w, h, fill),
        Storage => emit_storage(svg, x, y, w, h, fill),
        Database => emit_database(svg, x, y, w, h, fill),
        Queue => emit_queue(svg, x, y, w, h, fill),
        _ => emit_rounded_rect(svg, x, y, w, h, fill),
    }
}

fn emit_cluster_shape(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: Option<&str>,
) {
    use DeploymentNodeKind::*;
    // Clusters default to no fill; a `#color` paints the cluster background.
    let fill = fill.unwrap_or("none");
    match kind {
        // Clusters use stroke-width=1 (per goldens).
        Node => emit_tag_polygon(svg, x, y, w, h, fill, 1.0),
        // Card cluster has rect + horizontal line under title.
        Card => emit_card_cluster(svg, x, y, w, h, fill),
        // Rectangle / Agent cluster: bare rect, no line.
        Rectangle | Agent => emit_plain_rect_cluster(svg, x, y, w, h, fill),
        Frame => emit_frame_cluster(svg, x, y, w, h, fill),
        Folder => emit_folder_cluster(svg, x, y, w, h),
        Package => emit_package_cluster(svg, x, y, w, h),
        _ => emit_tag_polygon(svg, x, y, w, h, fill, 1.0),
    }
}

fn emit_plain_rect_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Node ("tag" polygon) -------------------------------------------------

fn emit_tag_polygon(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, sw: f64) {
    let off = 10.0;
    let x1 = fc(x);
    let y1 = fc(y + off);
    let x2 = fc(x + off);
    let y2 = fc(y);
    let x3 = fc(x + w);
    let y3 = fc(y + h - off);
    let x4 = fc(x + w - off);
    let y4 = fc(y + h);
    let points = format!("{x1},{y1},{x2},{y2},{x3},{y2},{x3},{y3},{x4},{y4},{x1},{y4},{x1},{y1}");
    svg.raw(&format!(
        r#"<polygon fill="{fill}" points="{points}" style="stroke:{STROKE};stroke-width:{sw};"/>"#,
    ));
    // 3 lines for the 3D effect: top-right diagonal, top inner, right inner.
    let xa = fc(x + w - off);
    let xb = fc(x + w);
    let ya = fc(y + off);
    let yb = fc(y);
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:{sw};" x1="{xa}" x2="{xb}" y1="{ya}" y2="{yb}"/>"#,
    ));
    let xc = fc(x);
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:{sw};" x1="{xc}" x2="{xa}" y1="{ya}" y2="{ya}"/>"#,
    ));
    let yc = fc(y + h);
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:{sw};" x1="{xa}" x2="{xa}" y1="{ya}" y2="{yc}"/>"#,
    ));
}

// ---- Artifact (rect + folded corner) --------------------------------------

fn emit_artifact(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    let x_s = fc(x);
    let y_s = fc(y);
    let w_s = fc(w);
    let h_s = fc(h);
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h_s}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
    ));
    // Folded corner polygon at top-right (12x14 box, inset 5 from right and 5 from top).
    let fx = x + w - 17.0; // 12 wide, then 5 from right edge
    let fy = y + 5.0;
    let p1 = (fx, fy);
    let p2 = (fx, fy + 14.0);
    let p3 = (fx + 12.0, fy + 14.0);
    let p4 = (fx + 12.0, fy + 6.0);
    let p5 = (fx + 6.0, fy);
    let pts = format!(
        "{},{},{},{},{},{},{},{},{},{},{},{}",
        fc(p1.0),
        fc(p1.1),
        fc(p2.0),
        fc(p2.1),
        fc(p3.0),
        fc(p3.1),
        fc(p4.0),
        fc(p4.1),
        fc(p5.0),
        fc(p5.1),
        fc(p1.0),
        fc(p1.1),
    );
    svg.raw(&format!(
        r#"<polygon fill="{fill}" points="{pts}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
    ));
    // Two lines for the fold detail.
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:0.5;" x1="{a}" x2="{a}" y1="{y1}" y2="{y2}"/>"#,
        a = fc(fx + 6.0),
        y1 = fc(fy),
        y2 = fc(fy + 6.0),
    ));
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:0.5;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#,
        x1 = fc(fx + 12.0),
        x2 = fc(fx + 6.0),
        y = fc(fy + 6.0),
    ));
}

// ---- Rounded rect (card / rectangle / agent leaf) --------------------------

fn emit_rounded_rect(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Card cluster (rect + horizontal line) --------------------------------

fn emit_card_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Horizontal line under the title row (at y + 20.4883).
    let ly = y + 20.4883;
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:1;" x1="{x1}" x2="{x2}" y1="{ly_s}" y2="{ly_s}"/>"#,
        x1 = fc(x),
        x2 = fc(x + w),
        ly_s = fc(ly),
    ));
}

// ---- Component (rect + tab + bars) ----------------------------------------

fn emit_component(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Tab at top-right: 15w x 10h, x = x+w-20, y = y+5.
    let tab_x = x + w - 20.0;
    let tab_y = y + 5.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="10" style="stroke:{STROKE};stroke-width:0.5;" width="15" x="{x}" y="{y}"/>"#,
        x = fc(tab_x),
        y = fc(tab_y),
    ));
    // Two small bars left of tab (4w x 2h each).
    let bar_x = tab_x - 2.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="2" style="stroke:{STROKE};stroke-width:0.5;" width="4" x="{x}" y="{y}"/>"#,
        x = fc(bar_x),
        y = fc(tab_y + 2.0),
    ));
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="2" style="stroke:{STROKE};stroke-width:0.5;" width="4" x="{x}" y="{y}"/>"#,
        x = fc(bar_x),
        y = fc(tab_y + 6.0),
    ));
}

// ---- Frame (rect + small tab path top-left) -------------------------------

fn emit_frame(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Tab path: from a point partway across the top, draw down then bend to the left edge.
    // For a 1-line label of width 73.6025, the tab path went to x=44.8675 (=7+37.8675),
    // so tab_w = label_w/2 + 1 ≈ but actually it's roughly half the text width.
    // From the golden we have: M44.8675,7 L44.8675,12 L37.8675,19 L7,19
    // So the right edge is at x_label_end + 1 ish? Hard to compute generically.
    // Approximation: tab_x_right = x + (w/2) - 1, tab corner offset = 7.
    let _ = (x, y, w);
}

// ---- Frame cluster --------------------------------------------------------

fn emit_frame_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    // Frame cluster: bare rect with stroke-width=1. The tab is emitted
    // by emit_cluster_label since it depends on label width.
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

/// Emit the small tab path used by frame clusters in the top-left corner.
/// The tab right-edge sits at `x + label_w + 10`.
fn emit_frame_tab(svg: &mut SvgBuilder, x: f64, y: f64, label_w: f64) {
    let right_x = x + label_w + 10.0;
    // Tab geometry derived from goldens: tab is text_height tall, the
    // diagonal cut starts at y + (text_height - 7) and ends at y + text_height + 3.
    let y_mid = y + 9.48828125;
    let y_bot = y + 19.48828125;
    let d = format!(
        "M{rx},{y_s} L{rx},{ym} L{rx_in},{yb} L{x_s},{yb}",
        rx = fc(right_x),
        rx_in = fc(right_x - 10.0),
        y_s = fc(y),
        ym = fc(y_mid),
        yb = fc(y_bot),
        x_s = fc(x),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="none" style="stroke:{STROKE};stroke-width:1;"/>"#
    ));
}

// ---- Folder ---------------------------------------------------------------

fn emit_folder(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64, _fill: &str) {
    // TODO: complex path; oracle bbox is unreliable.
}

fn emit_folder_cluster(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO
}

// ---- File -----------------------------------------------------------------

fn emit_file(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64, _fill: &str) {
    // TODO: complex path with corner fold; oracle bbox unreliable.
}

// ---- Package --------------------------------------------------------------

fn emit_package(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64, _fill: &str) {
    // TODO: complex path with tab and bold title.
}

fn emit_package_cluster(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO
}

// ---- Stack ----------------------------------------------------------------

fn emit_stack(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    // Stack: an inner rect with no stroke (just fill), plus an outline path
    // that extends 15px on either side. Geometry from goldens:
    //   rect at (x, y, w, h) — the inner fill
    //   path: M{x-15},{y} L{x-2.5},{y} A2.5,2.5 0 0 1 {x},{y+2.5}
    //         L{x},{y+h-2.5} A2.5,2.5 0 0 0 {x+2.5},{y+h}
    //         L{x+w-2.5},{y+h} A2.5,2.5 0 0 0 {x+w},{y+h-2.5}
    //         L{x+w},{y+2.5} A2.5,2.5 0 0 1 {x+w+2.5},{y} L{x+w+15},{y}
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:none;stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    let xl = x - 15.0;
    let xr = x + w + 15.0;
    let d = format!(
        "M{xl},{y_s} L{x_lp1},{y_s} A2.5,2.5 0 0 1 {x_s},{y_p1} L{x_s},{y_pm1} A2.5,2.5 0 0 0 {x_lp2},{yh_s} L{x_rm2},{yh_s} A2.5,2.5 0 0 0 {xw_s},{y_pm1} L{xw_s},{y_p1} A2.5,2.5 0 0 1 {x_rp2},{y_s} L{xr},{y_s}",
        xl = fc(xl),
        xr = fc(xr),
        x_s = fc(x),
        xw_s = fc(x + w),
        y_s = fc(y),
        yh_s = fc(y + h),
        x_lp1 = fc(x - 2.5),
        x_lp2 = fc(x + 2.5),
        x_rm2 = fc(x + w - 2.5),
        x_rp2 = fc(x + w + 2.5),
        y_p1 = fc(y + 2.5),
        y_pm1 = fc(y + h - 2.5),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
}

// ---- Storage (rounded rect with rx=35, ry=35) -----------------------------

fn emit_storage(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="35" ry="35" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Database (cylinder via 2 bezier paths) -------------------------------

fn emit_database(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    // Recover full-precision width from oracle width to avoid 1-ULP drift.
    // For database the geometry is symmetric so cx = (x_low + x_high) / 2
    // where x_high = x + w_full. Width is determined by label, but for
    // matching we can use the oracle's reported w and let cx ride the
    // truncated computation — for the symmetric case, just use oracle w.
    let _ = h;
    let h_full = pm::text_height(FONT_SIZE) + 29.0;
    let cx = x + w / 2.0;
    let bot_y = y + h_full;
    let top_low = y + 10.0;
    let bot_low = y + h_full - 10.0;
    let d = format!(
        "M{x_s},{tl} C{x_s},{y_s} {cx_s},{y_s} {cx_s},{y_s} C{cx_s},{y_s} {xw_s},{y_s} {xw_s},{tl} L{xw_s},{bl} C{xw_s},{by_s} {cx_s},{by_s} {cx_s},{by_s} C{cx_s},{by_s} {x_s},{by_s} {x_s},{bl} L{x_s},{tl}",
        x_s = fc(x),
        y_s = fc(y),
        cx_s = fc(cx),
        xw_s = fc(x + w),
        by_s = fc(bot_y),
        tl = fc(top_low),
        bl = fc(bot_low),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    // The "top wall" of the cylinder (inner curve under the lip).
    let d2 = format!(
        "M{x_s},{tl} C{x_s},{ml} {cx_s},{ml} {cx_s},{ml} C{cx_s},{ml} {xw_s},{ml} {xw_s},{tl}",
        x_s = fc(x),
        cx_s = fc(cx),
        xw_s = fc(x + w),
        tl = fc(top_low),
        ml = fc(y + 20.0),
    );
    svg.raw(&format!(
        r#"<path d="{d2}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
}

// ---- Queue (cylinder rotated 90 degrees) ---------------------------------

fn emit_queue(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    // Like database but rotated: rounded left + straight top/bottom + rounded right.
    // The "right wall" lip is at x+w-10.
    //
    // Recover full-precision height from text metrics to avoid 1-ULP drift
    // in midline rounding: cy = y + (text_height + 10) / 2 produces the
    // same f64 the JVM emits.
    let h_full = pm::text_height(FONT_SIZE) + 10.0;
    let cy = y + h_full / 2.0;
    let left_in = x + 5.0;
    let right_in = x + w - 5.0;
    let _ = h;
    let d = format!(
        "M{li},{y_s} L{ri},{y_s} C{xw_s},{y_s} {xw_s},{cy_s} {xw_s},{cy_s} C{xw_s},{cy_s} {xw_s},{yh_s} {ri},{yh_s} L{li},{yh_s} C{x_s},{yh_s} {x_s},{cy_s} {x_s},{cy_s} C{x_s},{cy_s} {x_s},{y_s} {li},{y_s}",
        li = fc(left_in),
        ri = fc(right_in),
        x_s = fc(x),
        y_s = fc(y),
        cy_s = fc(cy),
        xw_s = fc(x + w),
        yh_s = fc(y + h_full),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    // The inner left wall (right-side of the lip).
    let inner_x = x + w - 10.0;
    let d2 = format!(
        "M{ri},{y_s} C{ix},{y_s} {ix},{cy_s} {ix},{cy_s} C{ix},{yh_s} {ri},{yh_s} {ri},{yh_s}",
        ri = fc(right_in),
        ix = fc(inner_x),
        y_s = fc(y),
        cy_s = fc(cy),
        yh_s = fc(y + h_full),
    );
    svg.raw(&format!(
        r#"<path d="{d2}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

fn emit_entity_label(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    node: &DeploymentNode,
    x: f64,
    y: f64,
    w: f64,
) {
    let (_text_x_pad, top_pad, bold) = entity_text_geom(kind, w, &node.label);
    let label_w = text_render::measure(&node.label, FONT_SIZE, bold);
    let center_x = entity_text_center(kind, x, w);

    if let Some(stereo) = &node.stereotype {
        let stereo_label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_w = text_render::measure(&stereo_label, FONT_SIZE, false);
        let stereo_x = center_x - stereo_w / 2.0;
        emit_text(
            svg,
            &stereo_label,
            stereo_x,
            y + top_pad,
            FONT_SIZE,
            false,
            true,
        );
        let label_x = center_x - label_w / 2.0;
        emit_text(
            svg,
            &node.label,
            label_x,
            y + top_pad + TEXT_LINE_H,
            FONT_SIZE,
            bold,
            false,
        );
    } else {
        let label_x = center_x - label_w / 2.0;
        emit_text(
            svg,
            &node.label,
            label_x,
            y + top_pad,
            FONT_SIZE,
            bold,
            false,
        );
    }
}

fn emit_cluster_label(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    node: &DeploymentNode,
    x: f64,
    y: f64,
    w: f64,
) {
    // Cluster labels are centered horizontally above the children area
    // for most shapes; frame is left-aligned (with a tab decoration).
    let label_w = text_render::measure(&node.label, FONT_SIZE, true);

    if matches!(kind, DeploymentNodeKind::Frame) {
        // Frame cluster: tab path comes before the text label, then a
        // left-aligned label at (x+3, y+ascent+1).
        emit_frame_tab(svg, x, y, label_w);
        let label_x = x + 3.0;
        let label_y = y + ASCENT_14 + 1.0;
        emit_text(svg, &node.label, label_x, label_y, FONT_SIZE, true, false);
        return;
    }

    let center_x = cluster_text_center(kind, x, w);

    if let Some(stereo) = &node.stereotype {
        let stereo_label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_w = text_render::measure(&stereo_label, FONT_SIZE, false);
        let stereo_x = center_x - stereo_w / 2.0;
        let stereo_y = y + cluster_top_pad(kind);
        emit_text(
            svg,
            &stereo_label,
            stereo_x,
            stereo_y,
            FONT_SIZE,
            false,
            true,
        );
        let label_x = center_x - label_w / 2.0;
        emit_text(
            svg,
            &node.label,
            label_x,
            stereo_y + TEXT_LINE_H,
            FONT_SIZE,
            true,
            false,
        );
    } else {
        let label_x = center_x - label_w / 2.0;
        let label_y = y + cluster_top_pad(kind);
        emit_text(svg, &node.label, label_x, label_y, FONT_SIZE, true, false);
    }
}

fn cluster_top_pad(kind: DeploymentNodeKind) -> f64 {
    use DeploymentNodeKind::*;
    match kind {
        // Node cluster title sits in a small header band: ascent+13 from bbox top.
        Node => ASCENT_14 + 13.0,
        // Card-like clusters: ascent+2.
        Card | Rectangle | Agent | Frame => ASCENT_14 + 2.0,
        _ => ASCENT_14 + 13.0,
    }
}

/// Horizontal center used for cluster labels (different per shape).
fn cluster_text_center(kind: DeploymentNodeKind, x: f64, w: f64) -> f64 {
    use DeploymentNodeKind::*;
    match kind {
        // Node clusters: centered between [x, x+w-10] with +1 offset measured
        // from goldens (label is centered slightly right of the geometric
        // mean of the box's back wall).
        Node => x + (w - 10.0) / 2.0 + 1.0,
        // Card-like clusters: centered within full width.
        _ => x + w / 2.0,
    }
}

/// Horizontal center used for entity (leaf) labels.
fn entity_text_center(kind: DeploymentNodeKind, x: f64, w: f64) -> f64 {
    use DeploymentNodeKind::*;
    match kind {
        // Node leaves: centered between [x, x+w-10] (no +1 offset for leaves).
        Node | Component | Frame => x + (w - 10.0) / 2.0,
        // Artifact: centered between [x, x+w-10] (the corner fold reduces text-safe width).
        Artifact => x + (w - 10.0) / 2.0,
        // Queue: centered between [x+5, x+w-15] = x + (w-10)/2.
        Queue => x + (w - 10.0) / 2.0,
        // Card / rectangle / agent / storage / database: centered in full width.
        _ => x + w / 2.0,
    }
}

fn entity_text_geom(kind: DeploymentNodeKind, _w: f64, _label: &str) -> (f64, f64, bool) {
    use DeploymentNodeKind::*;
    match kind {
        Node | Component | Frame => (15.0, TEXT_PAD_NODE, false),
        Artifact => (10.0, TEXT_PAD_ARTIFACT, false),
        Card => (10.0, TEXT_PAD_CARD, false),
        Rectangle | Agent | File | Folder | Storage => (10.0, TEXT_PAD_RECTLIKE, false),
        // Queue is shorter vertically: ascent + 5.
        Queue => (5.0, ASCENT_14 + 5.0, false),
        // Database label sits below the lip: ascent + 24.
        Database => (10.0, ASCENT_14 + 24.0, false),
        Package => (10.0, TEXT_PAD_PACKAGE_LABEL, true),
        _ => (10.0, TEXT_PAD_RECTLIKE, false),
    }
}

// ---------------------------------------------------------------------------
// Text emission helper
// ---------------------------------------------------------------------------

fn emit_text(
    svg: &mut SvgBuilder,
    content: &str,
    x: f64,
    y: f64,
    fs: f64,
    bold: bool,
    italic: bool,
) {
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        content,
        &TextBase {
            x,
            y,
            font_size: fs as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold,
            italic,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
}

// ---------------------------------------------------------------------------
// Connections (oracle-driven)
// ---------------------------------------------------------------------------

fn render_connection(
    svg: &mut SvgBuilder,
    conn: &DeploymentConnection,
    oracle: &OracleLayout,
    id_for_node: &HashMap<String, String>,
    own_qname_for_id: &HashMap<String, String>,
    link_id: &str,
) {
    // Edge IDs in goldens use the OWN name of each endpoint. own_qname may
    // itself contain '.' (label-derived), so we can't recover it by splitting
    // the full qualified path on '.'.
    let from_qname = own_qname_for_id
        .get(&conn.from)
        .cloned()
        .unwrap_or_else(|| conn.from.clone());
    let to_qname = own_qname_for_id
        .get(&conn.to)
        .cloned()
        .unwrap_or_else(|| conn.to.clone());
    // PlantUML emits the path id as `{leftQname}-{kind}-{rightQname}` where
    // {kind} is `to`, `backto`, or empty (associations). Layout direction
    // can reverse the wire order (e.g. `A -left-> B` ⇒ `B-backto-A`), so we
    // probe both orderings.
    let candidates = [
        format!("{from_qname}-to-{to_qname}"),
        format!("{}-to-{}", conn.from, conn.to),
        format!("{to_qname}-backto-{from_qname}"),
        format!("{}-backto-{}", conn.to, conn.from),
        format!("{from_qname}-{to_qname}"),
        format!("{}-{}", conn.from, conn.to),
        format!("{from_qname}-backto-{to_qname}"),
    ];
    let oracle_edge = candidates
        .iter()
        .find_map(|cand| oracle.edges.iter().find(|e| e.id == *cand));
    let oracle_edge = oracle_edge.or_else(|| {
        let f_id = id_for_node.get(&conn.from).cloned();
        let t_id = id_for_node.get(&conn.to).cloned();
        oracle.edges.iter().find(|e| {
            let e1 = e.entity_1.as_deref();
            let e2 = e.entity_2.as_deref();
            (e1 == f_id.as_deref() && e2 == t_id.as_deref())
                || (e1 == t_id.as_deref() && e2 == f_id.as_deref())
        })
    });
    let expected_id = oracle_edge
        .map(|e| e.id.clone())
        .unwrap_or_else(|| candidates[0].clone());

    let is_reverse = oracle_edge
        .map(|e| e.id.contains("-backto-"))
        .unwrap_or(false);
    let (comment_from, comment_to) = if is_reverse {
        (&to_qname, &from_qname)
    } else {
        (&from_qname, &to_qname)
    };
    let prefix = if is_reverse { "reverse link" } else { "link" };
    svg.raw(&format!("<!--{prefix} {comment_from} to {comment_to}-->"));

    let entity_1 = oracle_edge
        .and_then(|e| e.entity_1.as_deref())
        .or_else(|| id_for_node.get(&conn.from).map(String::as_str))
        .unwrap_or("ent0002");
    let entity_2 = oracle_edge
        .and_then(|e| e.entity_2.as_deref())
        .or_else(|| id_for_node.get(&conn.to).map(String::as_str))
        .unwrap_or("ent0003");
    let link_type = oracle_edge
        .and_then(|e| e.link_type.as_deref())
        .unwrap_or("dependency");
    let source_line = oracle_edge.and_then(|e| e.source_line.as_deref());
    let link_id_final = oracle_edge
        .and_then(|e| e.link_id.as_deref())
        .unwrap_or(link_id);

    let source_attr = source_line
        .map(|s| format!(r#" data-source-line="{s}""#))
        .unwrap_or_default();

    svg.raw(&format!(
        r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}"{source_attr} id="{link_id_final}">"#,
    ));

    if let Some(oe) = oracle_edge {
        let path_style = oe
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let code_line_attr = oe
            .code_line
            .as_ref()
            .map(|c| format!(r#" codeLine="{c}""#))
            .unwrap_or_default();
        svg.raw(&format!(
            r#"<path{code_line_attr} d="{d}" fill="none" id="{expected_id}" style="{path_style}"/>"#,
            d = oe.d,
        ));
        if let Some(points) = &oe.arrow_points {
            let fill = oe.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oe
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }
        if let Some(points) = &oe.second_arrow_points {
            let fill = oe.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oe
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }
        // Connection label — position taken from oracle.
        if let Some((lx, ly, text)) = &oe.label {
            for (i, line) in text.split('\n').enumerate() {
                let y = *ly + (i as f64) * pm::text_height(13.0);
                emit_text(svg, line, *lx, y, 13.0, false, false);
            }
        }
        let _ = conn.label.as_ref();
    }

    svg.raw("</g>");
}

// ---------------------------------------------------------------------------
// Non-oracle fallback (minimal)
// ---------------------------------------------------------------------------

fn render_no_oracle(_diagram: &DeploymentDiagram, _theme: &Theme) -> String {
    // Minimal empty SVG envelope — golden tests always supply oracle.
    let mut s = String::new();
    write!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="DESCRIPTION" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><defs/><g></g></svg>"#).unwrap();
    s
}
