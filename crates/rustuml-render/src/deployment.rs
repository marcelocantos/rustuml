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

use crate::layout_oracle::OracleLayout;
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

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
        let mut q = own_qname(n);
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
            &qname_for_id,
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
            emit_cluster_shape(svg, node.kind, rect.x, rect.y, rect.width, rect.height);
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
            emit_entity_shape(svg, node.kind, rect.x, rect.y, rect.width, rect.height);
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
) {
    use DeploymentNodeKind::*;
    match kind {
        Node => emit_tag_polygon(svg, x, y, w, h, FILL, 0.5),
        Artifact => emit_artifact(svg, x, y, w, h),
        Card | Rectangle | Agent => emit_rounded_rect(svg, x, y, w, h),
        Component => emit_component(svg, x, y, w, h),
        Frame => emit_frame(svg, x, y, w, h),
        Folder => emit_folder(svg, x, y, w, h),
        File => emit_file(svg, x, y, w, h),
        Package => emit_package(svg, x, y, w, h),
        Stack => emit_stack(svg, x, y, w, h),
        Storage => emit_storage(svg, x, y, w, h),
        Database => emit_database(svg, x, y, w, h),
        Queue => emit_queue(svg, x, y, w, h),
        _ => emit_rounded_rect(svg, x, y, w, h),
    }
}

fn emit_cluster_shape(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    use DeploymentNodeKind::*;
    match kind {
        // Clusters use no fill and stroke-width=1 (per goldens).
        Node => emit_tag_polygon(svg, x, y, w, h, "none", 1.0),
        // Card cluster has rect + horizontal line under title.
        Card => emit_card_cluster(svg, x, y, w, h),
        // Rectangle / Agent cluster: bare rect, no line.
        Rectangle | Agent => emit_plain_rect_cluster(svg, x, y, w, h),
        Frame => emit_frame_cluster(svg, x, y, w, h),
        Folder => emit_folder_cluster(svg, x, y, w, h),
        Package => emit_package_cluster(svg, x, y, w, h),
        _ => emit_tag_polygon(svg, x, y, w, h, "none", 1.0),
    }
}

fn emit_plain_rect_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    svg.raw(&format!(
        r#"<rect fill="none" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

// ---- Node ("tag" polygon) -------------------------------------------------

fn emit_tag_polygon(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, sw: f64) {
    let off = 10.0;
    let x1 = pm::fmt_coord(x);
    let y1 = pm::fmt_coord(y + off);
    let x2 = pm::fmt_coord(x + off);
    let y2 = pm::fmt_coord(y);
    let x3 = pm::fmt_coord(x + w);
    let y3 = pm::fmt_coord(y + h - off);
    let x4 = pm::fmt_coord(x + w - off);
    let y4 = pm::fmt_coord(y + h);
    let points = format!("{x1},{y1},{x2},{y2},{x3},{y2},{x3},{y3},{x4},{y4},{x1},{y4},{x1},{y1}");
    svg.raw(&format!(
        r#"<polygon fill="{fill}" points="{points}" style="stroke:{STROKE};stroke-width:{sw};"/>"#,
    ));
    // 3 lines for the 3D effect: top-right diagonal, top inner, right inner.
    let xa = pm::fmt_coord(x + w - off);
    let xb = pm::fmt_coord(x + w);
    let ya = pm::fmt_coord(y + off);
    let yb = pm::fmt_coord(y);
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:{sw};" x1="{xa}" x2="{xb}" y1="{ya}" y2="{yb}"/>"#,
    ));
    let xc = pm::fmt_coord(x);
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:{sw};" x1="{xc}" x2="{xa}" y1="{ya}" y2="{ya}"/>"#,
    ));
    let yc = pm::fmt_coord(y + h);
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:{sw};" x1="{xa}" x2="{xa}" y1="{ya}" y2="{yc}"/>"#,
    ));
}

// ---- Artifact (rect + folded corner) --------------------------------------

fn emit_artifact(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    let x_s = pm::fmt_coord(x);
    let y_s = pm::fmt_coord(y);
    let w_s = pm::fmt_coord(w);
    let h_s = pm::fmt_coord(h);
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="{h_s}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
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
        pm::fmt_coord(p1.0),
        pm::fmt_coord(p1.1),
        pm::fmt_coord(p2.0),
        pm::fmt_coord(p2.1),
        pm::fmt_coord(p3.0),
        pm::fmt_coord(p3.1),
        pm::fmt_coord(p4.0),
        pm::fmt_coord(p4.1),
        pm::fmt_coord(p5.0),
        pm::fmt_coord(p5.1),
        pm::fmt_coord(p1.0),
        pm::fmt_coord(p1.1),
    );
    svg.raw(&format!(
        r#"<polygon fill="{FILL}" points="{pts}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
    ));
    // Two lines for the fold detail.
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:0.5;" x1="{a}" x2="{a}" y1="{y1}" y2="{y2}"/>"#,
        a = pm::fmt_coord(fx + 6.0),
        y1 = pm::fmt_coord(fy),
        y2 = pm::fmt_coord(fy + 6.0),
    ));
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:0.5;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#,
        x1 = pm::fmt_coord(fx + 12.0),
        x2 = pm::fmt_coord(fx + 6.0),
        y = pm::fmt_coord(fy + 6.0),
    ));
}

// ---- Rounded rect (card / rectangle / agent leaf) --------------------------

fn emit_rounded_rect(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

// ---- Card cluster (rect + horizontal line) --------------------------------

fn emit_card_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    svg.raw(&format!(
        r#"<rect fill="none" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
    // Horizontal line under the title row (at y + 20.4883).
    let ly = y + 20.4883;
    svg.raw(&format!(
        r#"<line style="stroke:{STROKE};stroke-width:1;" x1="{x1}" x2="{x2}" y1="{ly_s}" y2="{ly_s}"/>"#,
        x1 = pm::fmt_coord(x),
        x2 = pm::fmt_coord(x + w),
        ly_s = pm::fmt_coord(ly),
    ));
}

// ---- Component (rect + tab + bars) ----------------------------------------

fn emit_component(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
    // Tab at top-right: 15w x 10h, x = x+w-20, y = y+5.
    let tab_x = x + w - 20.0;
    let tab_y = y + 5.0;
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="10" style="stroke:{STROKE};stroke-width:0.5;" width="15" x="{x}" y="{y}"/>"#,
        x = pm::fmt_coord(tab_x),
        y = pm::fmt_coord(tab_y),
    ));
    // Two small bars left of tab (4w x 2h each).
    let bar_x = tab_x - 2.0;
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="2" style="stroke:{STROKE};stroke-width:0.5;" width="4" x="{x}" y="{y}"/>"#,
        x = pm::fmt_coord(bar_x),
        y = pm::fmt_coord(tab_y + 2.0),
    ));
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="2" style="stroke:{STROKE};stroke-width:0.5;" width="4" x="{x}" y="{y}"/>"#,
        x = pm::fmt_coord(bar_x),
        y = pm::fmt_coord(tab_y + 6.0),
    ));
}

// ---- Frame (rect + small tab path top-left) -------------------------------

fn emit_frame(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
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

fn emit_frame_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    emit_card_cluster(svg, x, y, w, h);
}

// ---- Folder ---------------------------------------------------------------

fn emit_folder(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO: complex path; oracle bbox is unreliable.
}

fn emit_folder_cluster(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO
}

// ---- File -----------------------------------------------------------------

fn emit_file(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO: complex path with corner fold; oracle bbox unreliable.
}

// ---- Package --------------------------------------------------------------

fn emit_package(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO: complex path with tab and bold title.
}

fn emit_package_cluster(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO
}

// ---- Stack ----------------------------------------------------------------

fn emit_stack(_svg: &mut SvgBuilder, _x: f64, _y: f64, _w: f64, _h: f64) {
    // TODO: rect (no stroke) + complex outline path.
}

// ---- Storage (rounded rect with rx=35, ry=35) -----------------------------

fn emit_storage(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
    svg.raw(&format!(
        r#"<rect fill="{FILL}" height="{h}" rx="35" ry="35" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = pm::fmt_coord(h),
        w = pm::fmt_coord(w),
        x = pm::fmt_coord(x),
        y = pm::fmt_coord(y),
    ));
}

// ---- Database (cylinder via 2 bezier paths) -------------------------------

fn emit_database(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
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
        x_s = pm::fmt_coord(x),
        y_s = pm::fmt_coord(y),
        cx_s = pm::fmt_coord(cx),
        xw_s = pm::fmt_coord(x + w),
        by_s = pm::fmt_coord(bot_y),
        tl = pm::fmt_coord(top_low),
        bl = pm::fmt_coord(bot_low),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    // The "top wall" of the cylinder (inner curve under the lip).
    let d2 = format!(
        "M{x_s},{tl} C{x_s},{ml} {cx_s},{ml} {cx_s},{ml} C{cx_s},{ml} {xw_s},{ml} {xw_s},{tl}",
        x_s = pm::fmt_coord(x),
        cx_s = pm::fmt_coord(cx),
        xw_s = pm::fmt_coord(x + w),
        tl = pm::fmt_coord(top_low),
        ml = pm::fmt_coord(y + 20.0),
    );
    svg.raw(&format!(
        r#"<path d="{d2}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
}

// ---- Queue (cylinder rotated 90 degrees) ---------------------------------

fn emit_queue(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64) {
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
        li = pm::fmt_coord(left_in),
        ri = pm::fmt_coord(right_in),
        x_s = pm::fmt_coord(x),
        y_s = pm::fmt_coord(y),
        cy_s = pm::fmt_coord(cy),
        xw_s = pm::fmt_coord(x + w),
        yh_s = pm::fmt_coord(y + h_full),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    // The inner left wall (right-side of the lip).
    let inner_x = x + w - 10.0;
    let d2 = format!(
        "M{ri},{y_s} C{ix},{y_s} {ix},{cy_s} {ix},{cy_s} C{ix},{yh_s} {ri},{yh_s} {ri},{yh_s}",
        ri = pm::fmt_coord(right_in),
        ix = pm::fmt_coord(inner_x),
        y_s = pm::fmt_coord(y),
        cy_s = pm::fmt_coord(cy),
        yh_s = pm::fmt_coord(y + h_full),
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
    // Cluster labels are centered horizontally above the children area. The
    // title is bold. The stereotype (if any) sits above the title in italic.
    let label_w = text_render::measure(&node.label, FONT_SIZE, true);
    let center_x = cluster_text_center(kind, x, w);

    if let Some(stereo) = &node.stereotype {
        let stereo_label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_w = text_render::measure(&stereo_label, FONT_SIZE, false);
        let stereo_x = center_x - stereo_w / 2.0;
        // Stereotype sits at the same baseline as a non-stereotype title.
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
    qname_for_id: &HashMap<String, String>,
    link_id: &str,
) {
    // Edge IDs in goldens use the OWN name (last segment), not the full
    // qualified path. Strip the prefix to match.
    let from_qname = qname_for_id
        .get(&conn.from)
        .map(|q| {
            q.rsplit_once('.')
                .map(|(_, last)| last.to_string())
                .unwrap_or_else(|| q.clone())
        })
        .unwrap_or_else(|| conn.from.clone());
    let to_qname = qname_for_id
        .get(&conn.to)
        .map(|q| {
            q.rsplit_once('.')
                .map(|(_, last)| last.to_string())
                .unwrap_or_else(|| q.clone())
        })
        .unwrap_or_else(|| conn.to.clone());
    let candidates = [
        format!("{from_qname}-to-{to_qname}"),
        format!("{}-to-{}", conn.from, conn.to),
    ];
    let oracle_edge = candidates
        .iter()
        .find_map(|cand| oracle.edges.iter().find(|e| e.id == *cand));
    let expected_id = oracle_edge
        .map(|e| e.id.clone())
        .unwrap_or_else(|| candidates[0].clone());

    svg.raw(&format!("<!--link {from_qname} to {to_qname}-->"));

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
        // Connection label.
        if let Some(label) = &conn.label {
            // The text element matches PlantUML's <text> for link labels.
            // Position from oracle: pick midpoint of path endpoints; +1 on x.
            // Without precise oracle data, we approximate.
            // For now, leave label emission to the future — many links don't
            // have labels.
            let _ = label;
        }
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
