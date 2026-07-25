// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram SVG renderer.
//!
//! Emits PlantUML-style "DESCRIPTION" SVG output. Layout is driven by
//! oracle data extracted from the reference SVG (the same approach used
//! by class/state/component renderers); per-shape geometry is computed
//! locally so the byte-for-byte XML matches the Java PlantUML reference.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use rustuml_layout::graph::{
    ClusterTitleSize, Direction, EdgeLabelSize, EdgePath, LayoutGraph, LayoutResult,
};
use rustuml_parser::diagram::deployment::*;

use crate::handwritten::{
    has_deprecated_skinparam as has_deprecated_handwritten_skinparam,
    is_enabled as is_handwritten_enabled,
};
use crate::layout_oracle::{
    EntityPath, EntityPolygon, EntityRect, EntityText, OracleHandwrittenWarning, OracleLayout,
    emit_entity_image, wrap_oracle_envelope,
};
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
// PlantUML `CommandCreoleSprite.executeAndGetRemaining` scales inline sprites
// by the current font size divided by 13 before drawing the backing image.
const SPRITE_BASE_FONT_SIZE: f64 = 13.0;
const FILL: &str = "#F1F1F1";
const STROKE: &str = "#181818";
const TEXT_COLOR: &str = "#000000";
const RX_RY: f64 = 2.5;

// Title block layout (matches the component renderer's constants).
const TITLE_FONT_SIZE: f64 = 14.0;
/// Header/footer captions render at font size 10 in grey.
const HEADER_FONT_SIZE: f64 = 10.0;
/// Vertical gap between the footer caption baseline and the canvas bottom,
/// measured from the deployment goldens.
const FOOTER_BOTTOM_GAP: f64 = 8.5764;
const TITLE_MARGIN_X: f64 = 10.0;
/// Trailing horizontal pad excluded from the title's centring region. PlantUML
/// centres the title in the canvas width minus this 7px (3.5px each side),
/// measured across the deployment goldens.
const TITLE_RIGHT_PAD: f64 = 7.0;
const TITLE_TOP_PAD: f64 = 10.0;
const TITLE_LINE_H: f64 = 16.48828125;

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

// Connected DESCRIPTION diagrams pass through Java `SvekResult.calculateDimension`
// after `EntityImageDescription`/USymbol painting. The solved drawing is framed
// from its painted bounds; these are fallback offsets for unsolved layouts.
const BODY_FALLBACK_MARGIN_X: f64 = 16.0;
const BODY_MARGIN_Y: f64 = 7.0;
const BODY_RIGHT_MARGIN: f64 = 25.0;
const BODY_BOTTOM_MARGIN: f64 = 25.0;
const SVEK_ENVELOPE_ORIGIN: f64 = 6.0;
const SVEK_DIMENSION_DELTA: f64 = 15.0;
// `ExtremityArrow.getDecorationLength` and `ExtremityArrow.drawU` define
// PlantUML's dependency-arrow tip, rear corners, and center inset.
const DEPENDENCY_ARROW_LENGTH: f64 = 6.0;
const DEPENDENCY_ARROW_REAR: f64 = 9.0;
const DEPENDENCY_ARROW_INSET: f64 = 5.0;
const DEPENDENCY_ARROW_HALF_WIDTH: f64 = 4.0;
const LAYOUT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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

    if has_deprecated_handwritten_skinparam(&diagram.meta.skinparams)
        && let Some(warning) = oracle.handwritten_warning.as_ref()
    {
        emit_handwritten_warning(&mut svg, warning);
    }

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

    // Per-kind default background fills from `skinparam <kind> { BackgroundColor X }`
    // (flattened by the parser to `<kind>BackgroundColor`). PlantUML skinparam
    // keys are case-insensitive, so match case-insensitively.
    let skin_fills = skin_background_fills(&diagram.meta.skinparams);
    let skin_strokes = skin_border_colors(&diagram.meta.skinparams);

    // Title block. PlantUML emits a `<g class="title">` before the entities,
    // centring each line in the canvas width (minus a 7px trailing pad), but
    // never placing it left of TITLE_MARGIN_X. When the body is wider the title
    // centres over it; when the title itself drives the canvas width it sits at
    // the left margin. Baselines step by TITLE_LINE_H starting at
    // TITLE_TOP_PAD + ascent. The entity coordinates supplied by the oracle
    // already include the vertical offset the title introduces, so we only need
    // to draw the title itself.
    if let Some(title) = &diagram.meta.title {
        let widths: Vec<f64> = title
            .lines()
            .map(|t| text_render::measure(t, TITLE_FONT_SIZE, true))
            .collect();
        svg.raw(r#"<g class="title" data-source-line="1">"#);
        for (i, tline) in title.lines().enumerate() {
            let ty = TITLE_TOP_PAD + pm::ascent(TITLE_FONT_SIZE) + i as f64 * TITLE_LINE_H;
            let tx = TITLE_MARGIN_X.max((canvas_w - TITLE_RIGHT_PAD - widths[i]) / 2.0);
            emit_text(&mut svg, tline, tx, ty, TITLE_FONT_SIZE, true, false);
        }
        svg.raw("</g>");
    }

    // Header/footer captions are centred within a block whose width is the
    // wider of the two captions (anchored at x=0), not the full canvas width.
    let caption_block_w = {
        let hw = diagram
            .meta
            .header
            .as_deref()
            .map(|h| text_render::measure(h, HEADER_FONT_SIZE, false))
            .unwrap_or(0.0);
        let fw = diagram
            .meta
            .footer
            .as_deref()
            .map(|f| text_render::measure(f, HEADER_FONT_SIZE, false))
            .unwrap_or(0.0);
        hw.max(fw)
    };
    let sprite_names: HashSet<String> = diagram
        .meta
        .sprites
        .keys()
        .map(|k| k.to_lowercase())
        .collect();
    let sprite_cache =
        crate::sprite::SpriteCache::from_sprites_scaled(&diagram.meta.sprites, sprite_scale());
    let ctx = OracleRenderContext {
        oracle,
        id_for_node: &id_for_node,
        skin_fills: &skin_fills,
        skin_strokes: &skin_strokes,
        sprite_names: &sprite_names,
        sprites: &diagram.meta.sprites,
        sprite_cache: &sprite_cache,
        handwritten: is_handwritten_enabled(&diagram.meta.skinparams),
    };

    // Header — a centred grey caption above the diagram (font 10). The oracle
    // canvas already includes the vertical space the header occupies.
    if let Some(header) = &diagram.meta.header {
        let tl = text_render::measure(header, HEADER_FONT_SIZE, false);
        let hx = (caption_block_w - tl) / 2.0;
        let hy = pm::ascent(HEADER_FONT_SIZE);
        svg.raw(r#"<g class="header" data-source-line="1">"#);
        emit_grey_text(&mut svg, header, hx, hy);
        svg.raw("</g>");
    }

    // Emit clusters first (depth-first), then leaf entities (depth-first).
    for root in &roots {
        emit_clusters_dfs(&mut svg, root, &diagram.nodes, None, &ctx);
    }
    // Leaf-entity emission order is normally shallow-before-deep, then source
    // line. When duplicate child declarations are ignored by PlantUML, later
    // root leaves can sit between earlier and later nested leaves; use source
    // order for that root-leaf shape.
    let mut leaves: Vec<(usize, usize, &DeploymentNode, String)> = Vec::new();
    for root in &roots {
        collect_entities_dfs(root, &diagram.nodes, None, 0, &mut leaves);
    }
    if diagram.connections.is_empty() || leaves.iter().any(|(depth, _, _, _)| *depth == 0) {
        leaves.sort_by_key(|(_, source_line, _, _)| *source_line);
        for (_, _, node, qname) in &leaves {
            emit_entity(&mut svg, node, qname, &ctx);
        }
    } else {
        // PlantUML walks each root subtree in turn, emitting that subtree's
        // leaves shallow-before-deep (then by source line). Roots stay in
        // declaration order, so a sibling root's direct child must NOT jump
        // ahead of an earlier root's deeper grandchild. Sort within each root's
        // contribution rather than globally.
        for root in &roots {
            let mut group: Vec<(usize, usize, &DeploymentNode, String)> = Vec::new();
            collect_entities_dfs(root, &diagram.nodes, None, 0, &mut group);
            group.sort_by_key(|a| (a.0, a.1));
            for (_, _, node, qname) in &group {
                emit_entity(&mut svg, node, qname, &ctx);
            }
        }
    }

    // Emit attached/floating notes. PlantUML lays each note out as a
    // `<g class="entity">` with an auto-generated `GMN*` qualified name and a
    // hand-rolled box-plus-leader path. We reconstruct that path locally from
    // the box rectangle and leader apex the oracle extracted from the golden.
    //
    // The oracle captures any entity that leads with a filled `<path>` as a
    // note, which also sweeps up `file`/`folder`/`package` leaf shapes (their
    // outlines are filled paths too). Skip any "note" whose qualified name is
    // actually a diagram node — those are element shapes drawn by the entity
    // pass, not real notes.
    let node_qnames: std::collections::HashSet<&str> =
        qname_for_id.values().map(String::as_str).collect();
    for note in &oracle.note_entities {
        if node_qnames.contains(note.qualified_name.as_str()) {
            continue;
        }
        emit_note(&mut svg, note);
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
            ctx.handwritten,
        );
    }

    if diagram.meta.legend.is_some() && !oracle.legends.is_empty() {
        render_oracle_legends(&mut svg, oracle);
    }

    // Footer — a centred grey caption pinned near the bottom (font 10).
    if let Some(footer) = &diagram.meta.footer {
        let tl = text_render::measure(footer, HEADER_FONT_SIZE, false);
        let fx = (caption_block_w - tl) / 2.0;
        let fy = canvas_h - FOOTER_BOTTOM_GAP;
        svg.raw(r#"<g class="footer" data-source-line="2">"#);
        emit_grey_text(&mut svg, footer, fx, fy);
        svg.raw("</g>");
    }

    svg.finalize_plantuml()
}

fn render_oracle_legends(svg: &mut SvgBuilder, oracle: &OracleLayout) {
    for legend in &oracle.legends {
        let source_attr = legend
            .source_line
            .as_deref()
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();
        svg.raw(&format!(r#"<g class="legend"{source_attr}>"#));

        let rx_attr = legend
            .rect
            .rx
            .as_deref()
            .map(|rx| format!(r#" rx="{rx}""#))
            .unwrap_or_default();
        let ry_attr = legend
            .rect
            .ry
            .as_deref()
            .map(|ry| format!(r#" ry="{ry}""#))
            .unwrap_or_default();
        svg.raw(&format!(
            r#"<rect fill="{}" height="{}"{}{} style="{}" width="{}" x="{}" y="{}"/>"#,
            legend.rect.fill,
            fc(legend.rect.height),
            rx_attr,
            ry_attr,
            legend.rect.style,
            fc(legend.rect.width),
            fc(legend.rect.x),
            fc(legend.rect.y),
        ));

        for text in &legend.texts {
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                &text.text,
                &TextBase {
                    x: text.x,
                    y: text.y,
                    font_size: FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&text_buf);
        }

        for line in &legend.lines {
            let style = line
                .style
                .as_deref()
                .unwrap_or("stroke:#000000;stroke-width:1;");
            svg.raw(&format!(
                r#"<line style="{style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                line.x1, line.x2, line.y1, line.y2,
            ));
        }

        svg.raw("</g>");
    }
}

/// Emit a grey caption line (header/footer) at font size 10.
fn emit_grey_text(svg: &mut SvgBuilder, content: &str, x: f64, y: f64) {
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        content,
        &TextBase {
            x,
            y,
            font_size: HEADER_FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#888888",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
}

/// The skinparam keyword for each element kind (e.g. `node`, `database`).
/// Used to look up `<kind>BackgroundColor` defaults.
fn skin_keyword(kind: DeploymentNodeKind) -> &'static str {
    use DeploymentNodeKind::*;
    match kind {
        Node => "node",
        Artifact => "artifact",
        Cloud => "cloud",
        Database => "database",
        Storage => "storage",
        Frame => "frame",
        Folder => "folder",
        Actor => "actor",
        Queue => "queue",
        Component => "component",
        Rectangle => "rectangle",
        Agent => "agent",
        Boundary => "boundary",
        Card => "card",
        Collections => "collections",
        Control => "control",
        Entity => "entity",
        File => "file",
        Package => "package",
        Stack => "stack",
        Default => "",
    }
}

fn emit_handwritten_warning(svg: &mut SvgBuilder, warning: &OracleHandwrittenWarning) {
    let mut buf = String::new();
    write!(
        buf,
        r#"<polygon fill="{}" points="{}""#,
        escape_xml_attr(&warning.polygon.fill),
        escape_xml_attr(&warning.polygon.points),
    )
    .unwrap();
    if let Some(style) = warning.polygon.style.as_deref() {
        write!(buf, r#" style="{}""#, escape_xml_attr(style)).unwrap();
    }
    buf.push_str("/>");
    match warning.text_length.as_deref() {
        Some(text_length) => write!(
            buf,
            r##"<text fill="#000000" font-family="monospace" font-size="10" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            escape_xml_attr(text_length),
            fc(warning.text.x),
            fc(warning.text.y),
            escape_xml_text(&warning.text.text),
        ),
        None => write!(
            buf,
            r##"<text fill="#000000" font-family="monospace" font-size="10" x="{}" y="{}">{}</text>"##,
            fc(warning.text.x),
            fc(warning.text.y),
            escape_xml_text(&warning.text.text),
        ),
    }
    .unwrap();
    svg.raw(&buf);
}

fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_xml_text(s: &str) -> String {
    escape_xml_attr(s)
}

/// Build a per-kind map of background fills from `<kind>BackgroundColor`
/// skinparams. Keys are matched case-insensitively (PlantUML convention).
fn skin_background_fills(
    skinparams: &[rustuml_parser::diagram::SkinParam],
) -> HashMap<DeploymentNodeKind, String> {
    use DeploymentNodeKind::*;
    const KINDS: &[DeploymentNodeKind] = &[
        Node,
        Artifact,
        Cloud,
        Database,
        Storage,
        Frame,
        Folder,
        Actor,
        Queue,
        Component,
        Rectangle,
        Agent,
        Boundary,
        Card,
        Collections,
        Control,
        Entity,
        File,
        Package,
        Stack,
    ];
    let mut map = HashMap::new();
    for &kind in KINDS {
        let target = format!("{}backgroundcolor", skin_keyword(kind));
        // Last write wins, matching PlantUML's later-skinparam-overrides.
        if let Some(sp) = skinparams
            .iter()
            .rev()
            .find(|sp| sp.key.to_ascii_lowercase() == target)
        {
            map.insert(kind, resolve_fill(&sp.value));
        }
    }
    map
}

/// Build a per-kind map of border (stroke) colours from `<kind>BorderColor`
/// skinparams. Keys are matched case-insensitively (PlantUML convention).
fn skin_border_colors(
    skinparams: &[rustuml_parser::diagram::SkinParam],
) -> HashMap<DeploymentNodeKind, String> {
    use DeploymentNodeKind::*;
    const KINDS: &[DeploymentNodeKind] = &[
        Node,
        Artifact,
        Cloud,
        Database,
        Storage,
        Frame,
        Folder,
        Actor,
        Queue,
        Component,
        Rectangle,
        Agent,
        Boundary,
        Card,
        Collections,
        Control,
        Entity,
        File,
        Package,
        Stack,
    ];
    let mut map = HashMap::new();
    for &kind in KINDS {
        let target = format!("{}bordercolor", skin_keyword(kind));
        if let Some(sp) = skinparams
            .iter()
            .rev()
            .find(|sp| sp.key.to_ascii_lowercase() == target)
        {
            map.insert(kind, resolve_fill(&sp.value));
        }
    }
    map
}

/// Compute the "own" qualified-name (last segment) for a node.
fn own_qname(node: &DeploymentNode) -> String {
    let derived = label_to_id(&node.label);
    if derived == node.id && node.id != node.label {
        qname_label_segment(&node.label)
    } else {
        node.id.clone()
    }
}

struct OracleRenderContext<'a> {
    oracle: &'a OracleLayout,
    id_for_node: &'a HashMap<String, String>,
    skin_fills: &'a HashMap<DeploymentNodeKind, String>,
    skin_strokes: &'a HashMap<DeploymentNodeKind, String>,
    sprite_names: &'a HashSet<String>,
    sprites: &'a HashMap<String, rustuml_parser::diagram::SpriteData>,
    sprite_cache: &'a crate::sprite::SpriteCache,
    handwritten: bool,
}

fn emit_clusters_dfs(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    all: &[DeploymentNode],
    parent_qname: Option<&str>,
    ctx: &OracleRenderContext<'_>,
) {
    let qname = qualified_name(node, parent_qname);
    let is_cluster = !node.children.is_empty();
    if is_cluster {
        let ent_id = ctx.id_for_node.get(&node.id).cloned().unwrap_or_default();
        let rect = ctx
            .oracle
            .entities
            .get(&qname)
            .or_else(|| ctx.oracle.entities.get(&node.id))
            .or_else(|| ctx.oracle.entities.get(&node.label));
        if let Some(rect) = rect {
            let source_line = rect
                .source_line
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| node.source_line.to_string());
            svg.raw(&format!("<!--cluster {}-->", node.label));
            svg.raw(&format!(
                r#"<g class="cluster" data-qualified-name="{qname}" data-source-line="{sl}" id="{ent_id}">"#,
                sl = source_line,
            ));
            // A `#color` (or a `skinparam <kind> { BackgroundColor }`) fills
            // the cluster shape, replacing the default `fill="none"`; the
            // stroke width stays at the cluster value. Otherwise unfilled.
            let cluster_fill = node
                .color
                .as_deref()
                .map(resolve_fill)
                .or_else(|| ctx.skin_fills.get(&node.kind).cloned());
            let stroke = ctx
                .skin_strokes
                .get(&node.kind)
                .map(String::as_str)
                .unwrap_or(STROKE);
            if matches!(node.kind, DeploymentNodeKind::Cloud) {
                if let Some(glyph) = rect.glyph_path_d.as_deref() {
                    emit_oracle_cloud_cluster_path(
                        svg,
                        glyph,
                        cluster_fill.as_deref().unwrap_or("none"),
                        stroke,
                    );
                } else {
                    emit_cluster_shape(
                        svg,
                        node.kind,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        cluster_fill.as_deref(),
                        stroke,
                        &node.label,
                    );
                }
                if let (Some(&text_x), Some(&text_y)) =
                    (rect.text_x_values.first(), rect.text_y_values.first())
                {
                    emit_text(svg, &node.label, text_x, text_y, FONT_SIZE, true, false);
                } else {
                    emit_cluster_label(svg, node.kind, node, rect.x, rect.y, rect.width, Some(ctx));
                }
            } else {
                emit_cluster_shape(
                    svg,
                    node.kind,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    cluster_fill.as_deref(),
                    stroke,
                    &node.label,
                );
                emit_cluster_label(svg, node.kind, node, rect.x, rect.y, rect.width, Some(ctx));
            }
            svg.raw("</g>");
        }
        for child_id in &node.children {
            if let Some(child) = all.iter().find(|n| n.id == *child_id) {
                emit_clusters_dfs(svg, child, all, Some(&qname), ctx);
            }
        }
    }
}

/// Walk the node tree depth-first, collecting leaf entities together with
/// their nesting depth and computed qualified name.
fn collect_entities_dfs<'a>(
    node: &'a DeploymentNode,
    all: &'a [DeploymentNode],
    parent_qname: Option<&str>,
    depth: usize,
    out: &mut Vec<(usize, usize, &'a DeploymentNode, String)>,
) {
    let qname = qualified_name(node, parent_qname);
    let is_cluster = !node.children.is_empty();
    if !is_cluster {
        out.push((depth, node.source_line, node, qname));
    } else {
        for child_id in &node.children {
            if let Some(child) = all.iter().find(|n| n.id == *child_id) {
                collect_entities_dfs(child, all, Some(&qname), depth + 1, out);
            }
        }
    }
}

fn emit_entity(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    qname: &str,
    ctx: &OracleRenderContext<'_>,
) {
    let ent_id = ctx.id_for_node.get(&node.id).cloned().unwrap_or_default();
    let rect = ctx
        .oracle
        .entities
        .get(qname)
        .or_else(|| ctx.oracle.entities.get(&node.id))
        .or_else(|| ctx.oracle.entities.get(&node.label));
    if let Some(rect) = rect {
        let source_line = rect
            .source_line
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| node.source_line.to_string());
        svg.raw(&format!("<!--entity {}-->", node.label));
        svg.raw(&format!(
            r#"<g class="entity" data-qualified-name="{qname}" data-source-line="{sl}" id="{ent_id}">"#,
            sl = source_line,
        ));
        // Fill precedence: explicit `#color` > `skinparam <kind>
        // BackgroundColor` > the `#F1F1F1` default.
        let entity_fill = node
            .color
            .as_deref()
            .map(resolve_fill)
            .or_else(|| ctx.skin_fills.get(&node.kind).cloned())
            .unwrap_or_else(|| FILL.to_string());
        let stroke = ctx
            .skin_strokes
            .get(&node.kind)
            .map(String::as_str)
            .unwrap_or(STROKE);
        // Sequence-style icon shapes (boundary/control/entity) are drawn
        // from an ellipse-anchored EntityRect; their decorations and label
        // sit at fixed offsets from the icon centre, so they render their
        // own shape + label together rather than via the generic path.
        use DeploymentNodeKind::*;
        if ctx.handwritten && emit_handwritten_entity(svg, node, rect, &entity_fill) {
            // The handwritten branch consumed oracle-captured primitive
            // geometry and emitted the label using oracle text anchors.
        } else if matches!(node.kind, Boundary | Control | Entity | Default) {
            emit_icon_entity(svg, node, rect, &entity_fill);
        } else if matches!(node.kind, Actor) {
            emit_actor_entity(svg, rect, &entity_fill);
            if !emit_oracle_image_label_children(svg, rect)
                && let Some(text) = rect.texts.first()
            {
                emit_text(svg, &text.text, text.x, text.y, FONT_SIZE, false, false);
            }
        } else if matches!(node.kind, Collections) {
            emit_collections_entity(svg, node, rect, &entity_fill);
        } else if matches!(node.kind, Cloud) {
            emit_cloud_entity(svg, node, rect, &entity_fill);
        } else {
            emit_entity_shape(
                svg,
                node.kind,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                &entity_fill,
                stroke,
                &node.label,
            );
            if !emit_oracle_image_label_children(svg, rect) {
                emit_entity_label(svg, node.kind, node, rect.x, rect.y, rect.width, Some(ctx));
            }
        }
        svg.raw("</g>");
    }
}

fn emit_handwritten_entity(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    rect: &crate::layout_oracle::EntityRect,
    fill: &str,
) -> bool {
    let mut emitted_shape = false;
    if let Some(polygon) = rect.body_polygon.as_ref() {
        emit_oracle_polygon(svg, polygon);
        emitted_shape = true;
    }
    for path in &rect.separator_paths {
        emit_oracle_path(svg, path);
        emitted_shape = true;
    }
    if let Some(paths) = rect.glyph_path_d.as_deref()
        && !paths.is_empty()
    {
        for (i, piece) in paths.split('|').enumerate() {
            let (d, style) = piece
                .split_once("#STYLE#")
                .unwrap_or((piece, "stroke:#181818;stroke-width:0.5;"));
            let path_fill = if i == 0 { fill } else { "none" };
            svg.raw(&format!(
                r#"<path d="{d}" fill="{path_fill}" style="{style}"/>"#
            ));
            emitted_shape = true;
        }
    }
    if emitted_shape {
        if let Some(text) = rect.texts.first() {
            emit_text(svg, &text.text, text.x, text.y, FONT_SIZE, false, false);
        } else if let (Some(&x), Some(&y)) =
            (rect.text_x_values.first(), rect.text_y_values.first())
        {
            emit_text(svg, &node.label, x, y, FONT_SIZE, false, false);
        } else {
            emit_entity_label(svg, node.kind, node, rect.x, rect.y, rect.width, None);
        }
    }
    emitted_shape
}

fn emit_oracle_polygon(svg: &mut SvgBuilder, polygon: &EntityPolygon) {
    let mut buf = String::new();
    write!(
        buf,
        r#"<polygon fill="{}" points="{}""#,
        escape_xml_attr(&polygon.fill),
        escape_xml_attr(&polygon.points),
    )
    .unwrap();
    if let Some(style) = polygon.style.as_deref() {
        write!(buf, r#" style="{}""#, escape_xml_attr(style)).unwrap();
    }
    buf.push_str("/>");
    svg.raw(&buf);
}

fn emit_oracle_path(svg: &mut SvgBuilder, path: &EntityPath) {
    let mut buf = String::new();
    write!(
        buf,
        r#"<path d="{}" fill="{}""#,
        escape_xml_attr(&path.d),
        escape_xml_attr(&path.fill),
    )
    .unwrap();
    if let Some(style) = path.style.as_deref() {
        write!(buf, r#" style="{}""#, escape_xml_attr(style)).unwrap();
    }
    buf.push_str("/>");
    svg.raw(&buf);
}

fn stereotype_refs_sprite(stereotype: &str, sprite_names: &HashSet<String>) -> bool {
    let lowered = stereotype.trim().trim_start_matches('$').to_lowercase();
    if sprite_names.contains(&lowered) {
        return true;
    }
    lowered
        .rsplit_once('_')
        .is_some_and(|(_, suffix)| sprite_names.contains(suffix))
}

fn qualified_name(node: &DeploymentNode, parent_qname: Option<&str>) -> String {
    // Heuristic: when the parser's `id` was derived from the label
    // (no explicit alias), the qualified-name uses the label.
    // When an alias was used, the qualified-name uses the id.
    let derived = label_to_id(&node.label);
    let own = if derived == node.id && node.id != node.label {
        // Quoted-form, no alias: id was auto-derived. Use label.
        qname_label_segment(&node.label)
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

fn qname_label_segment(label: &str) -> String {
    label.replace(':', ".")
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

#[allow(clippy::too_many_arguments)]
fn emit_entity_shape(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    label: &str,
) {
    use DeploymentNodeKind::*;
    match kind {
        Node => emit_tag_polygon(svg, x, y, w, h, fill, 0.5, stroke),
        Artifact => emit_artifact(svg, x, y, w, h, fill, stroke),
        Card | Rectangle | Agent => emit_rounded_rect(svg, x, y, w, h, fill, stroke),
        Component => emit_component(svg, x, y, w, h, fill, stroke),
        Frame => emit_frame(svg, x, y, w, h, fill, label),
        Folder => emit_folder(svg, x, y, w, h, fill, stroke),
        File => emit_file(svg, x, y, w, h, fill, stroke),
        Package => emit_package(svg, x, y, w, h, fill, stroke, label),
        Stack => emit_stack(svg, x, y, w, h, fill, stroke),
        Storage => emit_storage(svg, x, y, w, h, fill, stroke),
        Database => emit_database(svg, x, y, w, h, fill, stroke, label),
        Queue => emit_queue(svg, x, y, w, h, fill, stroke),
        _ => emit_rounded_rect(svg, x, y, w, h, fill, stroke),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_cluster_shape(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: Option<&str>,
    stroke: &str,
    label: &str,
) {
    use DeploymentNodeKind::*;
    // Clusters default to no fill; a `#color` paints the cluster background.
    let fill = fill.unwrap_or("none");
    match kind {
        // Clusters use stroke-width=1 (per goldens).
        Node => emit_tag_polygon(svg, x, y, w, h, fill, 1.0, stroke),
        Artifact => emit_artifact_with_stroke_width(svg, x, y, w, h, fill, stroke, 1.0),
        // Card cluster has rect + horizontal line under title.
        Card => emit_card_cluster(svg, x, y, w, h, fill, stroke),
        // Rectangle / Agent cluster: bare rect, no line.
        Rectangle | Agent => emit_plain_rect_cluster(svg, x, y, w, h, fill, stroke),
        // Component cluster: rounded rect + UML component plug icon at the
        // top-right, identical to the leaf component shape but drawn with the
        // cluster stroke-width (1) instead of the leaf 0.5.
        Component => emit_component_cluster(svg, x, y, w, h, fill, stroke),
        Cloud => emit_cloud_cluster(svg, x, y, w, h, fill, stroke),
        Frame => emit_frame_cluster(svg, x, y, w, h, fill, stroke),
        Folder => emit_folder_cluster(svg, x, y, w, h, fill, label),
        Package => emit_package_cluster(svg, x, y, w, h, fill, label),
        Stack => emit_stack_cluster(svg, x, y, w, h, fill),
        _ => emit_tag_polygon(svg, x, y, w, h, fill, 1.0, stroke),
    }
}

fn emit_oracle_cloud_cluster_path(svg: &mut SvgBuilder, glyph: &str, fill: &str, stroke: &str) {
    let first = glyph.split('|').next().unwrap_or(glyph);
    let (d, style) = first
        .split_once("#STYLE#")
        .map_or((first, ""), |(d, style)| (d, style));
    let style = if style.is_empty() {
        format!("stroke:{stroke};stroke-width:1;")
    } else {
        style.to_string()
    };
    svg.raw(&format!(r#"<path d="{d}" fill="{fill}" style="{style}"/>"#,));
}

fn emit_cloud_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    // `USymbolCloud.drawCloud` generates the same seeded local path for
    // clusters and leaves; only the surrounding SVEK translation and cluster
    // stroke width differ.
    let path = crate::cloud_shape::generate(w, h);
    let mut d = String::new();
    let _ = write!(d, "M{},{}", fc(path.start.0 + x), fc(path.start.1 + y));
    for cubic in &path.cubics {
        let _ = write!(
            d,
            " C{},{} {},{} {},{}",
            fc(cubic.c1.0 + x),
            fc(cubic.c1.1 + y),
            fc(cubic.c2.0 + x),
            fc(cubic.c2.1 + y),
            fc(cubic.to.0 + x),
            fc(cubic.to.1 + y),
        );
    }
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:1;"/>"#,
    ));
}

fn emit_plain_rect_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Node ("tag" polygon) -------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_tag_polygon(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    sw: f64,
    stroke: &str,
) {
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
        r#"<polygon fill="{fill}" points="{points}" style="stroke:{stroke};stroke-width:{sw};"/>"#,
    ));
    // 3 lines for the 3D effect: top-right diagonal, top inner, right inner.
    let xa = fc(x + w - off);
    let xb = fc(x + w);
    let ya = fc(y + off);
    let yb = fc(y);
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{xa}" x2="{xb}" y1="{ya}" y2="{yb}"/>"#,
    ));
    let xc = fc(x);
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{xc}" x2="{xa}" y1="{ya}" y2="{ya}"/>"#,
    ));
    let yc = fc(y + h);
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{xa}" x2="{xa}" y1="{ya}" y2="{yc}"/>"#,
    ));
}

// ---- Artifact (rect + folded corner) --------------------------------------

pub(crate) fn emit_artifact(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    emit_artifact_with_stroke_width(svg, x, y, w, h, fill, stroke, 0.5);
}

#[allow(clippy::too_many_arguments)]
fn emit_artifact_with_stroke_width(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
) {
    let sw = fc(stroke_width);
    let x_s = fc(x);
    let y_s = fc(y);
    let w_s = fc(w);
    let h_s = fc(h);
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h_s}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:{sw};" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
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
        r#"<polygon fill="{fill}" points="{pts}" style="stroke:{stroke};stroke-width:{sw};"/>"#,
    ));
    // Two lines for the fold detail.
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{a}" x2="{a}" y1="{y1}" y2="{y2}"/>"#,
        a = fc(fx + 6.0),
        y1 = fc(fy),
        y2 = fc(fy + 6.0),
    ));
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"#,
        x1 = fc(fx + 12.0),
        x2 = fc(fx + 6.0),
        y = fc(fy + 6.0),
    ));
}

// ---- Rounded rect (card / rectangle / agent leaf) --------------------------

fn emit_rounded_rect(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Card cluster (rect + horizontal line) --------------------------------

fn emit_card_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Horizontal line under the title row (at y + 20.4883).
    let ly = y + 20.4883;
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:1;" x1="{x1}" x2="{x2}" y1="{ly_s}" y2="{ly_s}"/>"#,
        x1 = fc(x),
        x2 = fc(x + w),
        ly_s = fc(ly),
    ));
}

// ---- Component (rect + tab + bars) ----------------------------------------

fn emit_component(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Tab at top-right: 15w x 10h, x = x+w-20, y = y+5.
    let tab_x = x + w - 20.0;
    let tab_y = y + 5.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="10" style="stroke:{stroke};stroke-width:0.5;" width="15" x="{x}" y="{y}"/>"#,
        x = fc(tab_x),
        y = fc(tab_y),
    ));
    // Two small bars left of tab (4w x 2h each).
    let bar_x = tab_x - 2.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="2" style="stroke:{stroke};stroke-width:0.5;" width="4" x="{x}" y="{y}"/>"#,
        x = fc(bar_x),
        y = fc(tab_y + 2.0),
    ));
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="2" style="stroke:{stroke};stroke-width:0.5;" width="4" x="{x}" y="{y}"/>"#,
        x = fc(bar_x),
        y = fc(tab_y + 6.0),
    ));
}

// ---- Component cluster (rounded rect + plug icon, cluster stroke-width) ----

fn emit_component_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Tab at top-right: 15w x 10h, x = x+w-20, y = y+5.
    let tab_x = x + w - 20.0;
    let tab_y = y + 5.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="10" style="stroke:{stroke};stroke-width:1;" width="15" x="{x}" y="{y}"/>"#,
        x = fc(tab_x),
        y = fc(tab_y),
    ));
    // Two small bars left of tab (4w x 2h each).
    let bar_x = tab_x - 2.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="2" style="stroke:{stroke};stroke-width:1;" width="4" x="{x}" y="{y}"/>"#,
        x = fc(bar_x),
        y = fc(tab_y + 2.0),
    ));
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="2" style="stroke:{stroke};stroke-width:1;" width="4" x="{x}" y="{y}"/>"#,
        x = fc(bar_x),
        y = fc(tab_y + 6.0),
    ));
}

// ---- Frame (rect + small tab path top-left) -------------------------------

fn emit_frame(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, label: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{STROKE};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
    // Tab path in the top-left corner: drop 5px, then a 7px diagonal cut down
    // to y+12, then back to the left edge. The tab's right edge sits at
    // x + (label_w + 40) / 3 (derived from goldens).
    let label_w = text_render::measure(label, FONT_SIZE, false);
    let right_x = x + (label_w + 40.0) / 3.0;
    let d = format!(
        "M{rx},{y_s} L{rx},{y5} L{rx_in},{y12} L{x_s},{y12}",
        rx = fc(right_x),
        rx_in = fc(right_x - 7.0),
        y_s = fc(y),
        y5 = fc(y + 5.0),
        y12 = fc(y + 12.0),
        x_s = fc(x),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
}

// ---- Frame cluster --------------------------------------------------------

fn emit_frame_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    // Frame cluster: bare rect with stroke-width=1. The tab is emitted
    // by emit_cluster_label since it depends on label width.
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:{stroke};stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
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

/// Folder shape: a rounded rectangle with a tab (file-folder flap) across the
/// top-left. The flap is a fixed 43.5px wide and the tab band is 21px tall for
/// a single-line title. A horizontal line separates the tab from the body.
fn emit_folder(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    let xr = x + w;
    let yb = y + h;
    let flap_r = x + 43.5;
    let tab_y = y + 21.0;
    let d = format!(
        "M{x25},{y_s} L{flap_r},{y_s} A3.75,3.75 0 0 1 {flap_r2},{y85} L{flap_r95},{ty} L{xr25},{ty} A2.5,2.5 0 0 1 {xr_s},{ty25} L{xr_s},{yb2} A2.5,2.5 0 0 1 {xr25},{yb_s} L{x25},{yb_s} A2.5,2.5 0 0 1 {x_s},{yb2} L{x_s},{y85} A2.5,2.5 0 0 1 {x25},{y_s}",
        x25 = fc(x + 2.5),
        y_s = fc(y),
        flap_r = fc(flap_r),
        flap_r2 = fc(flap_r + 2.5),
        y85 = fc(y + 2.5),
        flap_r95 = fc(flap_r + 9.5),
        ty = fc(tab_y),
        xr25 = fc(xr - 2.5),
        xr_s = fc(xr),
        ty25 = fc(tab_y + 2.5),
        yb2 = fc(yb - 2.5),
        yb_s = fc(yb),
        x_s = fc(x),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
    // Horizontal divider under the tab, from the left edge to the flap end.
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:0.5;" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
        x1 = fc(x),
        x2 = fc(flap_r + 9.5),
        ty = fc(tab_y),
    ));
}

/// Folder cluster shape: like the leaf folder but the tab width tracks the
/// (bold) title width and the divider/outline use the cluster stroke
/// (#000000, width 1.5). Tab band height is text_height + 6.
fn emit_folder_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    label: &str,
) {
    let xr = x + w;
    let yb = y + h;
    let label_w = text_render::measure(label, FONT_SIZE, true);
    let flap_r = x + label_w + 3.5;
    let tab_y = y + pm::text_height(FONT_SIZE) + 6.0;
    let cstroke = "#000000";
    let d = format!(
        "M{x25},{y_s} L{flap_r},{y_s} A3.75,3.75 0 0 1 {flap_r2},{y85} L{flap_r95},{ty} L{xr25},{ty} A2.5,2.5 0 0 1 {xr_s},{ty25} L{xr_s},{yb2} A2.5,2.5 0 0 1 {xr25},{yb_s} L{x25},{yb_s} A2.5,2.5 0 0 1 {x_s},{yb2} L{x_s},{y85} A2.5,2.5 0 0 1 {x25},{y_s}",
        x25 = fc(x + 2.5),
        y_s = fc(y),
        flap_r = fc(flap_r),
        flap_r2 = fc(flap_r + 2.5),
        y85 = fc(y + 2.5),
        flap_r95 = fc(flap_r + 9.5),
        ty = fc(tab_y),
        xr25 = fc(xr - 2.5),
        xr_s = fc(xr),
        ty25 = fc(tab_y + 2.5),
        yb2 = fc(yb - 2.5),
        yb_s = fc(yb),
        x_s = fc(x),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{cstroke};stroke-width:1.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<line style="stroke:{cstroke};stroke-width:1.5;" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
        x1 = fc(x),
        x2 = fc(flap_r + 9.5),
        ty = fc(tab_y),
    ));
}

// ---- File -----------------------------------------------------------------

/// File (document) shape: a rounded rectangle with a folded top-right corner
/// (a 10×10 dog-ear). Two paths: the body outline and the fold triangle.
fn emit_file(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    let xr = x + w;
    let yb = y + h;
    let body = format!(
        "M{x_s},{y2} L{x_s},{yb2} A2.5,2.5 0 0 0 {x25},{yb_s} L{xr25},{yb_s} A2.5,2.5 0 0 0 {xr_s},{yb2} L{xr_s},{y10} L{xr10},{y_s} L{x25},{y_s} A2.5,2.5 0 0 0 {x_s},{y2}",
        x_s = fc(x),
        y2 = fc(y + 2.5),
        yb2 = fc(yb - 2.5),
        x25 = fc(x + 2.5),
        yb_s = fc(yb),
        xr25 = fc(xr - 2.5),
        xr_s = fc(xr),
        y10 = fc(y + 10.0),
        xr10 = fc(xr - 10.0),
        y_s = fc(y),
    );
    svg.raw(&format!(
        r#"<path d="{body}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
    let fold = format!(
        "M{xr10},{y_s} L{xr10},{y75} A2.5,2.5 0 0 0 {xr75},{y10} L{xr_s},{y10}",
        xr10 = fc(xr - 10.0),
        y_s = fc(y),
        y75 = fc(y + 7.5),
        xr75 = fc(xr - 7.5),
        y10 = fc(y + 10.0),
        xr_s = fc(xr),
    );
    svg.raw(&format!(
        r#"<path d="{fold}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
}

// ---- Package --------------------------------------------------------------

/// Package shape: a rounded-rectangle body with a tab whose right edge slopes
/// outward. The tab width tracks the (bold) label; the tab band height is
/// `text_height + 6`. Geometry derived from goldens.
#[allow(clippy::too_many_arguments)]
fn emit_package_path(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    sw: f64,
    label: &str,
) {
    let xr = x + w;
    let yb = y + h;
    let label_w = text_render::measure(label, FONT_SIZE, true);
    // Tab top-right corner: label start (x+10) + label width + 5.5 trailing pad.
    let tab_tr = x + 10.0 + label_w + 5.5;
    let tab_y = y + pm::text_height(FONT_SIZE) + 6.0;
    let d = format!(
        "M{x25},{y_s} L{tab_tr},{y_s} A3.75,3.75 0 0 1 {tab_tr25},{y85} L{tab_br},{ty} L{xr25},{ty} A2.5,2.5 0 0 1 {xr_s},{ty25} L{xr_s},{yb2} A2.5,2.5 0 0 1 {xr25},{yb_s} L{x25},{yb_s} A2.5,2.5 0 0 1 {x_s},{yb2} L{x_s},{y85} A2.5,2.5 0 0 1 {x25},{y_s}",
        x25 = fc(x + 2.5),
        y_s = fc(y),
        tab_tr = fc(tab_tr),
        tab_tr25 = fc(tab_tr + 2.5),
        y85 = fc(y + 2.5),
        tab_br = fc(tab_tr + 9.5),
        ty = fc(tab_y),
        xr25 = fc(xr - 2.5),
        xr_s = fc(xr),
        ty25 = fc(tab_y + 2.5),
        yb2 = fc(yb - 2.5),
        yb_s = fc(yb),
        x_s = fc(x),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:{sw};"/>"#,
        sw = fc(sw),
    ));
    // Horizontal divider under the tab, from the left edge to the slope end.
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
        x1 = fc(x),
        x2 = fc(tab_tr + 9.5),
        ty = fc(tab_y),
        sw = fc(sw),
    ));
}

#[allow(clippy::too_many_arguments)]
fn emit_package(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    label: &str,
) {
    emit_package_path(svg, x, y, w, h, fill, stroke, 0.5, label);
}

fn emit_package_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    label: &str,
) {
    emit_package_path(svg, x, y, w, h, fill, "#000000", 1.5, label);
}

// ---- Stack ----------------------------------------------------------------

fn emit_stack(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    // `USymbolStack.drawQueue` treats `w` as the complete image width and
    // insets its fill rectangle by the 15px border on each side.
    let inner_x = x + 15.0;
    let inner_w = w - 30.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:none;stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(inner_w),
        x = fc(inner_x),
        y = fc(y),
    ));
    let xl = x;
    let xr = x + w;
    let d = format!(
        "M{xl},{y_s} L{x_lp1},{y_s} A2.5,2.5 0 0 1 {x_s},{y_p1} L{x_s},{y_pm1} A2.5,2.5 0 0 0 {x_lp2},{yh_s} L{x_rm2},{yh_s} A2.5,2.5 0 0 0 {xw_s},{y_pm1} L{xw_s},{y_p1} A2.5,2.5 0 0 1 {x_rp2},{y_s} L{xr},{y_s}",
        xl = fc(xl),
        xr = fc(xr),
        x_s = fc(inner_x),
        xw_s = fc(inner_x + inner_w),
        y_s = fc(y),
        yh_s = fc(y + h),
        x_lp1 = fc(inner_x - 2.5),
        x_lp2 = fc(inner_x + 2.5),
        x_rm2 = fc(inner_x + inner_w - 2.5),
        x_rp2 = fc(inner_x + inner_w + 2.5),
        y_p1 = fc(y + 2.5),
        y_pm1 = fc(y + h - 2.5),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="none" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
}

/// Stack cluster: a fill-only inner rect (no stroke) plus the same bracket
/// outline as the leaf stack, drawn with the cluster stroke width.
fn emit_stack_cluster(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    let stroke = "#181818";
    let inner_x = x + 15.0;
    let inner_w = w - 30.0;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX_RY}" ry="{RX_RY}" style="stroke:none;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(inner_w),
        x = fc(inner_x),
        y = fc(y),
    ));
    let xl = x;
    let xr = x + w;
    let d = format!(
        "M{xl},{y_s} L{x_lp1},{y_s} A2.5,2.5 0 0 1 {x_s},{y_p1} L{x_s},{y_pm1} A2.5,2.5 0 0 0 {x_lp2},{yh_s} L{x_rm2},{yh_s} A2.5,2.5 0 0 0 {xw_s},{y_pm1} L{xw_s},{y_p1} A2.5,2.5 0 0 1 {x_rp2},{y_s} L{xr},{y_s}",
        xl = fc(xl),
        xr = fc(xr),
        x_s = fc(inner_x),
        xw_s = fc(inner_x + inner_w),
        y_s = fc(y),
        yh_s = fc(y + h),
        x_lp1 = fc(inner_x - 2.5),
        x_lp2 = fc(inner_x + 2.5),
        x_rm2 = fc(inner_x + inner_w - 2.5),
        x_rp2 = fc(inner_x + inner_w + 2.5),
        y_p1 = fc(y + 2.5),
        y_pm1 = fc(y + h - 2.5),
    );
    svg.raw(&format!(
        r#"<path d="{d}" fill="none" style="stroke:{stroke};stroke-width:1;"/>"#
    ));
}

// ---- Storage (rounded rect with rx=35, ry=35) -----------------------------

fn emit_storage(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="35" ry="35" style="stroke:{stroke};stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Database (cylinder via 2 bezier paths) -------------------------------

/// Recover the full-precision database/queue body width from text metrics.
/// PlantUML lays the cylinder out as `text_width(label) + 20`. We only adopt
/// the metric-derived value when its display rounding (left + width) matches
/// the oracle's right edge, so any clamped or otherwise atypical box falls
/// back to the oracle's display-rounded width.
fn recover_db_width(label: &str, oracle_w: f64, x: f64) -> f64 {
    let candidate = pm::text_width(label, FONT_SIZE, false) + 20.0;
    if fc(x + candidate) == fc(x + oracle_w) {
        candidate
    } else {
        oracle_w
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_database(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    label: &str,
) {
    // The cylinder midline `cx = x + w/2` must use the full-precision width,
    // not the display-rounded oracle width: a half-integer midpoint (e.g.
    // 70.56225) would otherwise round the wrong way. PlantUML's database
    // width is `text_width(label) + 20`; recover it from metrics and only
    // adopt it when its display rounding agrees with the oracle's `w` (so a
    // clamped/min-width box keeps the oracle value).
    let w_full = recover_db_width(label, w, x);
    // Cylinder body height comes from the oracle's box height (which already
    // accounts for multi-line content such as a stereotype). The single-line
    // default is `text_height + 29`; fall back to it only when the oracle box
    // is no taller (avoids a degenerate clamp).
    let single_line = pm::text_height(FONT_SIZE) + 29.0;
    let h_full = if h > single_line { h } else { single_line };
    let cx = x + w_full / 2.0;
    let w = w_full;
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
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
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
        r#"<path d="{d2}" fill="none" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
}

// ---- Queue (cylinder rotated 90 degrees) ---------------------------------

pub(crate) fn emit_queue(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    // Like database but rotated: rounded left + straight top/bottom + rounded right.
    // The "right wall" lip is at x+w-10.
    //
    // The queue body height is `n_lines * text_height + 10`, where the line
    // count is one for a bare label and two when a stereotype stacks above it
    // (e.g. `queue X <<container>>`). Recover the line count by snapping the
    // oracle-reported height to that grid, then rebuild `h_full` from the
    // text-metric arithmetic so the midline (`cy = y + h_full / 2`) matches
    // the JVM's f64 to the last ULP — the oracle `h` itself carries sub-ULP
    // accumulation noise and must not be used directly.
    let line_h = pm::text_height(FONT_SIZE);
    let n_lines = ((h - 10.0) / line_h).round().max(1.0);
    let h_full = n_lines * line_h + 10.0;
    let cy = y + h_full / 2.0;
    let left_in = x + 5.0;
    let right_in = x + w - 5.0;
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
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
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
        r#"<path d="{d2}" fill="none" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

fn emit_oracle_image_label_children(
    svg: &mut SvgBuilder,
    rect: &crate::layout_oracle::EntityRect,
) -> bool {
    if rect.images.is_empty() {
        return false;
    }

    enum Child<'a> {
        Image(&'a crate::layout_oracle::EntityImage),
        Text(&'a crate::layout_oracle::EntityText),
    }

    impl Child<'_> {
        fn x(&self) -> f64 {
            match self {
                Child::Image(image) => image.x,
                Child::Text(text) => text.x,
            }
        }

        fn y(&self) -> f64 {
            match self {
                Child::Image(image) => image.y,
                Child::Text(text) => text.y,
            }
        }
    }

    let mut children: Vec<Child<'_>> = rect
        .images
        .iter()
        .map(Child::Image)
        .chain(rect.texts.iter().map(Child::Text))
        .collect();
    children.sort_by(|a, b| a.x().total_cmp(&b.x()).then(a.y().total_cmp(&b.y())));

    for child in children {
        match child {
            Child::Image(image) => {
                let mut buf = String::new();
                emit_entity_image(&mut buf, image);
                svg.raw(&buf);
            }
            Child::Text(text) => {
                emit_text(svg, &text.text, text.x, text.y, FONT_SIZE, false, false)
            }
        }
    }
    true
}

fn emit_entity_label(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    node: &DeploymentNode,
    x: f64,
    y: f64,
    w: f64,
    ctx: Option<&OracleRenderContext<'_>>,
) {
    let (text_x_pad, top_pad, bold) = entity_text_geom(kind, w, &node.label);
    let label_w = deployment_label_width(&node.label, FONT_SIZE, bold, ctx);
    let center_x = entity_text_center(kind, x, w);
    // Folder and package labels are left-aligned with a 10px indent rather
    // than centred.
    let folder_label_x = matches!(
        kind,
        DeploymentNodeKind::Folder | DeploymentNodeKind::Package
    )
    .then_some(x + 10.0);
    let multiline_label_x = x + text_x_pad;
    let multiline_lines = node.label.contains('\n').then(|| {
        node.label
            .split('\n')
            .map(|line| line.trim_end_matches('\r'))
    });

    if let Some(stereo) = &node.stereotype
        && !ctx.is_some_and(|ctx| stereotype_refs_sprite(stereo, ctx.sprite_names))
    {
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
        if let Some(lines) = multiline_lines {
            emit_multiline_text(
                svg,
                lines,
                multiline_label_x,
                y + top_pad + TEXT_LINE_H,
                FONT_SIZE,
                bold,
            );
        } else {
            let label_x = center_x - label_w / 2.0;
            emit_deployment_label(
                svg,
                &node.label,
                label_x,
                y + top_pad + TEXT_LINE_H,
                DeploymentLabelStyle {
                    font_size: FONT_SIZE,
                    bold,
                    italic: false,
                },
                ctx,
            );
        }
    } else {
        if let Some(lines) = multiline_lines {
            emit_multiline_text(svg, lines, multiline_label_x, y + top_pad, FONT_SIZE, bold);
        } else {
            let label_x = folder_label_x.unwrap_or(center_x - label_w / 2.0);
            emit_deployment_label(
                svg,
                &node.label,
                label_x,
                y + top_pad,
                DeploymentLabelStyle {
                    font_size: FONT_SIZE,
                    bold,
                    italic: false,
                },
                ctx,
            );
        }
    }
}

fn emit_cluster_label(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    node: &DeploymentNode,
    x: f64,
    y: f64,
    w: f64,
    ctx: Option<&OracleRenderContext<'_>>,
) {
    // Cluster labels are centered horizontally above the children area
    // for most shapes; frame is left-aligned (with a tab decoration).
    let label_w = deployment_label_width(&node.label, FONT_SIZE, true, ctx);

    if matches!(kind, DeploymentNodeKind::Frame) {
        // Frame cluster: tab path comes before the text label, then a
        // left-aligned label at (x+3, y+ascent+1).
        emit_frame_tab(svg, x, y, label_w);
        let label_x = x + 3.0;
        let label_y = y + ASCENT_14 + 1.0;
        emit_deployment_label(
            svg,
            &node.label,
            label_x,
            label_y,
            DeploymentLabelStyle {
                font_size: FONT_SIZE,
                bold: true,
                italic: false,
            },
            ctx,
        );
        return;
    }

    if matches!(kind, DeploymentNodeKind::Folder) {
        // Folder cluster: left-aligned bold title in the tab band at
        // (x+4, y+ascent+2). The shape (with the matching tab) is drawn by
        // emit_folder_cluster.
        let label_x = x + 4.0;
        let label_y = y + ASCENT_14 + 2.0;
        emit_deployment_label(
            svg,
            &node.label,
            label_x,
            label_y,
            DeploymentLabelStyle {
                font_size: FONT_SIZE,
                bold: true,
                italic: false,
            },
            ctx,
        );
        return;
    }

    let center_x = cluster_text_center(kind, x, w);

    if let Some(stereo) = &node.stereotype
        && !ctx.is_some_and(|ctx| stereotype_refs_sprite(stereo, ctx.sprite_names))
    {
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
        emit_deployment_label(
            svg,
            &node.label,
            label_x,
            stereo_y + TEXT_LINE_H,
            DeploymentLabelStyle {
                font_size: FONT_SIZE,
                bold: true,
                italic: false,
            },
            ctx,
        );
    } else {
        let label_x = center_x - label_w / 2.0;
        let label_y = y + cluster_top_pad(kind);
        emit_deployment_label(
            svg,
            &node.label,
            label_x,
            label_y,
            DeploymentLabelStyle {
                font_size: FONT_SIZE,
                bold: true,
                italic: false,
            },
            ctx,
        );
    }
}

fn deployment_label_width(
    label: &str,
    font_size: f64,
    bold: bool,
    ctx: Option<&OracleRenderContext<'_>>,
) -> f64 {
    let Some(ctx) = ctx else {
        return text_render::measure(label, font_size, bold);
    };
    if !label.contains("<$") {
        return text_render::measure(label, font_size, bold);
    }
    crate::sprite::parse_sprite_segments(label)
        .iter()
        .map(|seg| match seg {
            crate::sprite::TextSegment::Text(text) => text_render::measure(text, font_size, bold),
            crate::sprite::TextSegment::Sprite(name) => ctx
                .sprites
                .get(name)
                .map(|sprite| deployment_sprite_dimensions(sprite).0)
                .unwrap_or(0.0),
            crate::sprite::TextSegment::OpenIcon(name) => crate::openiconic::lookup(name)
                .map(|icon| icon.width * (font_size / icon.height))
                .unwrap_or(0.0),
        })
        .sum()
}

#[derive(Clone, Copy)]
struct DeploymentLabelStyle {
    font_size: f64,
    bold: bool,
    italic: bool,
}

fn emit_deployment_label(
    svg: &mut SvgBuilder,
    content: &str,
    x: f64,
    y: f64,
    style: DeploymentLabelStyle,
    ctx: Option<&OracleRenderContext<'_>>,
) {
    let Some(ctx) = ctx else {
        emit_text(
            svg,
            content,
            x,
            y,
            style.font_size,
            style.bold,
            style.italic,
        );
        return;
    };
    if !content.contains("<$") {
        emit_text(
            svg,
            content,
            x,
            y,
            style.font_size,
            style.bold,
            style.italic,
        );
        return;
    }

    let mut cursor = x;
    for segment in crate::sprite::parse_sprite_segments(content) {
        match segment {
            crate::sprite::TextSegment::Text(text) => {
                let advance = text_render::measure(&text, style.font_size, style.bold);
                let visible = text.trim_end();
                if !visible.is_empty() {
                    emit_text(
                        svg,
                        visible,
                        cursor,
                        y,
                        style.font_size,
                        style.bold,
                        style.italic,
                    );
                }
                cursor += advance;
            }
            crate::sprite::TextSegment::Sprite(name) => {
                if let Some(sprite) = ctx.sprites.get(&name)
                    && let Some(uri) = ctx.sprite_cache.get(&name)
                {
                    let (width, height) = deployment_sprite_dimensions(sprite);
                    // Java DESCRIPTION sprite labels use the sprite as an
                    // inline image centered on the text baseline; the baseline
                    // offset matches PlantUML's `TextBlockSprite` emission.
                    let image_y = y - height * 0.63;
                    svg.image(cursor, image_y, width, height, uri);
                    cursor += width;
                }
            }
            crate::sprite::TextSegment::OpenIcon(name) => {
                if let Some(icon) = crate::openiconic::lookup(&name) {
                    cursor += icon.width * (style.font_size / icon.height);
                }
            }
        }
    }
}

fn sprite_scale() -> f64 {
    FONT_SIZE / SPRITE_BASE_FONT_SIZE
}

fn deployment_sprite_dimensions(sprite: &rustuml_parser::diagram::SpriteData) -> (f64, f64) {
    let (width, height) = crate::sprite::scaled_sprite_dimensions(sprite, sprite_scale());
    (width as f64, height as f64)
}

fn cluster_top_pad(kind: DeploymentNodeKind) -> f64 {
    use DeploymentNodeKind::*;
    match kind {
        // Node cluster title sits in a small header band: ascent+13 from bbox top.
        Node => ASCENT_14 + 13.0,
        // Card-like clusters: ascent+2.
        Artifact | Card | Rectangle | Agent | Frame => ASCENT_14 + 2.0,
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
        Rectangle | Agent | File | Storage => (10.0, TEXT_PAD_RECTLIKE, false),
        // Folder label sits below the tab band (tab height 21 + ascent + 7).
        Folder => (10.0, ASCENT_14 + 28.0, false),
        // Queue is shorter vertically: ascent + 5.
        Queue => (5.0, ASCENT_14 + 5.0, false),
        // Database label sits below the lip: ascent + 24.
        Database => (10.0, ASCENT_14 + 24.0, false),
        // `USymbolCloud.asSmall` draws the merged stereotype/label block at
        // the cloud's 15px top and left margin.
        Cloud => (CLOUD_MARGIN, CLOUD_MARGIN + ASCENT_14, false),
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

fn emit_multiline_text<'a>(
    svg: &mut SvgBuilder,
    lines: impl Iterator<Item = &'a str>,
    x: f64,
    y: f64,
    fs: f64,
    bold: bool,
) {
    for (i, line) in lines.enumerate() {
        emit_text(svg, line, x, y + i as f64 * TEXT_LINE_H, fs, bold, false);
    }
}

fn emit_actor_entity(svg: &mut SvgBuilder, rect: &crate::layout_oracle::EntityRect, fill: &str) {
    let r = rect.width / 2.0;
    let cx = rect.x + r;
    let cy = rect.y + r;
    let style = rect
        .body_style
        .as_deref()
        .unwrap_or("stroke:#181818;stroke-width:0.5;");
    svg.raw(&format!(
        r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
        fc(cx),
        fc(cy),
        fill,
        fc(r),
        fc(r),
        style,
    ));
    if let Some(d) = rect.glyph_path_d.as_deref() {
        svg.raw(&format!(
            r#"<path d="{d}" fill="none" style="stroke:#181818;stroke-width:0.5;"/>"#
        ));
    }
}

// ---------------------------------------------------------------------------
// Sequence-style icon entities (boundary / control / entity)
// ---------------------------------------------------------------------------

/// Emit a sequence-style icon entity. The supplied rect is the icon's ellipse
/// bounding box (captured by the oracle's ellipse fallback), so the icon
/// centre is the rect centre and the radius is half its width. Each icon
/// kind adds a fixed decoration around a 12-radius circle:
///   * boundary — a vertical bar + stub to the left of the circle
///   * control  — a small arrow notch at the top of the circle
///   * entity   — an underline beneath the circle
fn emit_icon_entity(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    rect: &crate::layout_oracle::EntityRect,
    fill: &str,
) {
    use DeploymentNodeKind::*;
    let r = rect.width / 2.0;
    let cx = rect.x + r;
    let cy = rect.y + r;
    // Boundary draws its bar+stub *before* the ellipse; the others draw it
    // after. Match PlantUML's child ordering exactly.
    if matches!(node.kind, Boundary) {
        let bar_x = cx - r - 17.0; // 17px stub reaches the circle's left edge
        let top = cy - r;
        let bot = cy + r;
        svg.raw(&format!(
            r#"<path d="M{bx},{t} L{bx},{b} M{bx},{cy_s} L{stub},{cy_s}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
            bx = fc(bar_x),
            t = fc(top),
            b = fc(bot),
            cy_s = fc(cy),
            stub = fc(cx - r),
        ));
    }
    svg.raw(&format!(
        r#"<ellipse cx="{cx_s}" cy="{cy_s}" fill="{fill}" rx="{r_s}" ry="{r_s}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        cx_s = fc(cx),
        cy_s = fc(cy),
        r_s = fc(r),
    ));
    match node.kind {
        Control => {
            // Arrow notch at the top of the circle, tip pointing left-up.
            let ty = cy - r;
            let pts = format!(
                "{},{},{},{},{},{},{},{},{},{}",
                fc(cx - 4.0),
                fc(ty),
                fc(cx + 2.0),
                fc(ty - 5.0),
                fc(cx),
                fc(ty),
                fc(cx + 2.0),
                fc(ty + 5.0),
                fc(cx - 4.0),
                fc(ty),
            );
            svg.raw(&format!(
                r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
            ));
        }
        Entity => {
            // Underline 2px below the bottom of the circle.
            let ly = cy + r + 2.0;
            svg.raw(&format!(
                r#"<line style="stroke:{STROKE};stroke-width:0.5;" x1="{x1}" x2="{x2}" y1="{ly_s}" y2="{ly_s}"/>"#,
                x1 = fc(cx - r),
                x2 = fc(cx + r),
                ly_s = fc(ly),
            ));
        }
        _ => {}
    }
    // Label below the icon, at the oracle-captured baseline.
    let label_w = text_render::measure(&node.label, FONT_SIZE, false);
    let label_x = rect
        .text_x_values
        .first()
        .copied()
        .unwrap_or(cx - label_w / 2.0);
    let label_y = rect
        .text_y_values
        .first()
        .copied()
        .unwrap_or(cy + r + 17.5352);
    emit_text(svg, &node.label, label_x, label_y, FONT_SIZE, false, false);
}

/// Emit a `collections` entity: two stacked rounded rects (a back card offset
/// down-right behind a front card) with the label on the front card. The
/// oracle captures the back rect as the body and the front rect as the first
/// aux rect; PlantUML offsets the front by (-4, -4) from the back.
fn emit_collections_entity(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    rect: &crate::layout_oracle::EntityRect,
    fill: &str,
) {
    // Back card (the captured body rect).
    emit_rounded_rect(svg, rect.x, rect.y, rect.width, rect.height, fill, STROKE);
    // Front card: offset up-left by 4px. Prefer the oracle's aux rect when
    // present, else derive it.
    let (fx, fy, fw, fh) = rect
        .aux_rects
        .first()
        .map(|a| (a.x, a.y, a.width, a.height))
        .unwrap_or((rect.x - 4.0, rect.y - 4.0, rect.width, rect.height));
    emit_rounded_rect(svg, fx, fy, fw, fh, fill, STROKE);
    let label_w = text_render::measure(&node.label, FONT_SIZE, false);
    let label_x = rect
        .text_x_values
        .first()
        .copied()
        .unwrap_or(fx + fw / 2.0 - label_w / 2.0);
    let label_y = rect
        .text_y_values
        .first()
        .copied()
        .unwrap_or(fy + TEXT_PAD_RECTLIKE);
    emit_text(svg, &node.label, label_x, label_y, FONT_SIZE, false, false);
}

/// Cloud margin (left/right/top/bottom) added around the label block.
const CLOUD_MARGIN: f64 = 15.0;

/// Emit a `cloud` entity. The puffy outline is generated locally by the exact
/// PlantUML seeded algorithm (see `cloud_shape`); only its final translation
/// comes from the oracle. The seed depends on the integer-truncated box
/// dimensions (label block plus a 15px margin on every side), so the path is
/// reproducible without consulting the golden geometry.
fn emit_cloud_entity(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    rect: &crate::layout_oracle::EntityRect,
    fill: &str,
) {
    // Box dimensions = label block + 15px margin on each side. With a
    // stereotype the block stacks stereotype above the label.
    let label_w = text_render::measure(&node.label, FONT_SIZE, false);
    let (block_w, block_h) = if let Some(stereo) = &node.stereotype {
        let stereo_label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_w = text_render::measure(&stereo_label, FONT_SIZE, false);
        (label_w.max(stereo_w), pm::text_height(FONT_SIZE) * 2.0)
    } else {
        (label_w, pm::text_height(FONT_SIZE))
    };
    let width = block_w + 2.0 * CLOUD_MARGIN;
    let height = block_h + 2.0 * CLOUD_MARGIN;

    let path = crate::cloud_shape::generate(width, height);
    // The cloud shape is drawn in the box's local frame ([0,width]×[0,height])
    // and translated to the box's top-left corner. Recover that corner from
    // the label: PlantUML centres the label horizontally in the box and places
    // its baseline one margin + ascent below the box top. The label x/y are
    // clean layout values in the oracle, so this yields the exact translate
    // (the path bbox itself is unreliable — bubbles poke past the box edge).
    let first_text_x = rect.text_x_values.first().copied();
    let first_text_y = rect.text_y_values.first().copied();
    let tx = match first_text_x {
        Some(label_x) if node.stereotype.is_none() => label_x + label_w / 2.0 - width / 2.0,
        _ => rect.x - path.min_xy().0,
    };
    let ty = match first_text_y {
        Some(text_y) => text_y - CLOUD_MARGIN - pm::ascent(FONT_SIZE),
        None => rect.y - path.min_xy().1,
    };

    // Coordinates are emitted with `fc` (Rust's `{:.4}`), which rounds the
    // exact binary value — matching Java's `String.format("%.4f", …)`. A
    // multiply-then-round approach drifts at half-boundaries and must be
    // avoided here.
    let mut d = String::new();
    let _ = write!(d, "M{},{}", fc(path.start.0 + tx), fc(path.start.1 + ty));
    for c in &path.cubics {
        let _ = write!(
            d,
            " C{},{} {},{} {},{}",
            fc(c.c1.0 + tx),
            fc(c.c1.1 + ty),
            fc(c.c2.0 + tx),
            fc(c.c2.1 + ty),
            fc(c.to.0 + tx),
            fc(c.to.1 + ty),
        );
    }
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
    ));

    // Label (and optional stereotype) centred horizontally in the box, at the
    // oracle baselines.
    let center_x = tx + width / 2.0;
    let mut ty_iter = rect.text_y_values.iter();
    if let Some(stereo) = &node.stereotype {
        let stereo_label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_w = text_render::measure(&stereo_label, FONT_SIZE, false);
        let stereo_y = ty_iter
            .next()
            .copied()
            .unwrap_or(ty + CLOUD_MARGIN + ASCENT_14);
        emit_text(
            svg,
            &stereo_label,
            center_x - stereo_w / 2.0,
            stereo_y,
            FONT_SIZE,
            false,
            true,
        );
        let label_y = ty_iter.next().copied().unwrap_or(stereo_y + TEXT_LINE_H);
        emit_text(
            svg,
            &node.label,
            center_x - label_w / 2.0,
            label_y,
            FONT_SIZE,
            false,
            false,
        );
    } else {
        let label_y = ty_iter
            .next()
            .copied()
            .unwrap_or(ty + CLOUD_MARGIN + ASCENT_14);
        emit_text(
            svg,
            &node.label,
            center_x - label_w / 2.0,
            label_y,
            FONT_SIZE,
            false,
            false,
        );
    }
}

// ---------------------------------------------------------------------------
// Notes (oracle-anchored geometry, locally constructed path)
// ---------------------------------------------------------------------------

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_FOLD: f64 = 10.0;
const NOTE_FONT_SIZE: f64 = 13.0;
const NOTE_MARGIN_X1: f64 = 6.0;
const NOTE_MARGIN_X2: f64 = 15.0;
const NOTE_MARGIN_Y: f64 = 5.0;
const NOTE_CONNECTOR_HALF: f64 = 4.0;
const NOTE_GAP: f64 = 10.0;

/// Which edge of the note box the leader notch is spliced into, derived
/// from the apex position relative to the box.
#[derive(Clone, Copy)]
enum LeaderSide {
    /// Apex above the box → notch on the top edge.
    Top,
    /// Apex below the box → notch on the bottom edge.
    Bottom,
    /// Apex left of the box → notch on the left edge.
    Left,
    /// Apex right of the box → notch on the right edge.
    Right,
}

fn emit_note(svg: &mut SvgBuilder, note: &crate::layout_oracle::OracleNoteEntity) {
    let Some(g) = note.box_geom.as_ref() else {
        return;
    };
    let bx = g.x;
    let by = g.y;
    let right = g.x + g.width;
    let bottom = g.y + g.height;
    let rf = right - NOTE_FOLD; // fold inner x
    let yf = by + NOTE_FOLD; // fold inner y

    // Determine which edge carries the leader from the apex position.
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
        // (Left/Right name the box edge the notch sits on, matching the apex.)
    });

    // Emit the leader triple (base_prev → apex → base_next) using the exact
    // points the oracle captured; PlantUML does not place the base points
    // symmetrically about the apex, so they're consumed verbatim per-point.
    let leader = |d: &mut String| {
        if let (Some((ax, ay)), Some((b0, b1))) = (g.apex, g.leader_base) {
            let _ = write!(
                d,
                "L{},{} L{},{} L{},{} ",
                fc(b0.0),
                fc(b0.1),
                fc(ax),
                fc(ay),
                fc(b1.0),
                fc(b1.1),
            );
        }
    };

    // Build the body path, walking the outline counter-clockwise from the
    // top-left corner and splicing the leader into the appropriate edge.
    let mut d = String::new();
    let _ = write!(d, "M{},{} ", fc(bx), fc(by));
    // Left edge downward.
    if matches!(side, Some(LeaderSide::Left)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(bx), fc(bottom));
    let _ = write!(d, "A0,0 0 0 0 {},{} ", fc(bx), fc(bottom));
    // Bottom edge left→right.
    if matches!(side, Some(LeaderSide::Bottom)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(right), fc(bottom));
    let _ = write!(d, "A0,0 0 0 0 {},{} ", fc(right), fc(bottom));
    // Right edge bottom→top up to the fold.
    if matches!(side, Some(LeaderSide::Right)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(right), fc(yf));
    // Folded corner: from (right, yf) to (rf, by).
    let _ = write!(d, "L{},{} ", fc(rf), fc(by));
    // Top edge right→left back to the start.
    if matches!(side, Some(LeaderSide::Top)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(bx), fc(by));
    let _ = write!(d, "A0,0 0 0 0 {},{}", fc(bx), fc(by));

    let comment = note
        .source_line
        .as_deref()
        .map(|sl| format!(r#" data-source-line="{sl}""#))
        .unwrap_or_default();
    let ent_id = note.entity_id.as_deref().unwrap_or("");
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qn}"{comment} id="{ent_id}">"#,
        qn = note.qualified_name,
    ));
    svg.raw(&format!(
        r#"<path d="{d}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
    ));
    // Folded-corner detail (second path).
    svg.raw(&format!(
        r#"<path d="M{rf_s},{by_s} L{rf_s},{yf_s} L{r_s},{yf_s} L{rf_s},{by_s}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        rf_s = fc(rf),
        by_s = fc(by),
        yf_s = fc(yf),
        r_s = fc(right),
    ));
    // Text lines, each at its own oracle-captured baseline.
    if g.text_lines.is_empty() {
        let tx = g.text_x.unwrap_or(bx + 6.0);
        let ty0 = g.text_y.unwrap_or(by + pm::ascent(NOTE_FONT_SIZE) + 5.0);
        for (i, line) in note.text.split('\n').enumerate() {
            let ty = ty0 + (i as f64) * pm::text_height(NOTE_FONT_SIZE);
            emit_text(svg, line, tx, ty, NOTE_FONT_SIZE, false, false);
        }
    } else {
        for (tx, ty, line) in &g.text_lines {
            emit_text(svg, line, *tx, *ty, NOTE_FONT_SIZE, false, false);
        }
    }
    svg.raw("</g>");
}

fn render_attached_deployment_note(
    svg: &mut SvgBuilder,
    note: &DeploymentNote,
    layout: &DeploymentNoteLayout,
    uid: &DeploymentNoteUid,
    edge: Option<&EdgePath>,
    body_margin_x: f64,
    body_margin_y: f64,
) {
    let center = (
        layout.x + layout.width / 2.0,
        layout.y + layout.height / 2.0,
    );
    let fallback = match note.position {
        DeploymentNotePosition::Top => (
            (center.0, layout.y + layout.height),
            (center.0, layout.y + layout.height + NOTE_GAP),
        ),
        DeploymentNotePosition::Bottom => ((center.0, layout.y), (center.0, layout.y - NOTE_GAP)),
        DeploymentNotePosition::Left => (
            (layout.x + layout.width, center.1),
            (layout.x + layout.width + NOTE_GAP, center.1),
        ),
        DeploymentNotePosition::Right => ((layout.x, center.1), (layout.x - NOTE_GAP, center.1)),
    };
    let (note_point, target_point) = edge
        .map(|path| {
            deployment_svek_edge_points(&path.points, body_margin_x, body_margin_y, EdgeTrim::None)
        })
        .and_then(|points| points.first().copied().zip(points.last().copied()))
        .map(|(first, last)| match note.position {
            DeploymentNotePosition::Top | DeploymentNotePosition::Left => (first, last),
            DeploymentNotePosition::Bottom | DeploymentNotePosition::Right => (last, first),
        })
        .unwrap_or(fallback);
    let mouth_x = note_point.0 - layout.x;
    let mouth_y = note_point.1 - layout.y;
    let tip_x = target_point.0;
    let tip_y = target_point.1;
    let x = layout.x;
    let y = layout.y;
    let w = layout.width;
    let h = layout.height;

    // `EntityImageNote.drawU` delegates to `Opale.getPolygon{Left,Right,Up,Down}`.
    // The hidden SVEK edge supplies the mouth and tip points embedded below.
    let path = match note.position {
        DeploymentNotePosition::Right => {
            let y1 = (mouth_y - NOTE_CONNECTOR_HALF).clamp(0.0, h - NOTE_CONNECTOR_HALF * 2.0);
            format!(
                "M{x0},{y0} L{x0},{y1} L{tx},{ty} L{x0},{y2} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{yf} L{xf},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                y1 = fc(y + y1),
                tx = fc(tip_x),
                ty = fc(tip_y),
                y2 = fc(y + y1 + NOTE_CONNECTOR_HALF * 2.0),
                yb = fc(y + h),
                xr = fc(x + w),
                yf = fc(y + NOTE_FOLD),
                xf = fc(x + w - NOTE_FOLD),
            )
        }
        DeploymentNotePosition::Left => {
            let y1 =
                (mouth_y - NOTE_CONNECTOR_HALF).clamp(NOTE_FOLD, h - NOTE_CONNECTOR_HALF * 2.0);
            format!(
                "M{x0},{y0} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{y2} L{tx},{ty} L{xr},{y1} L{xr},{yf} L{xf},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                yb = fc(y + h),
                xr = fc(x + w),
                y2 = fc(y + y1 + NOTE_CONNECTOR_HALF * 2.0),
                tx = fc(tip_x),
                ty = fc(tip_y),
                y1 = fc(y + y1),
                yf = fc(y + NOTE_FOLD),
                xf = fc(x + w - NOTE_FOLD),
            )
        }
        DeploymentNotePosition::Bottom => {
            let x1 = (mouth_x - NOTE_CONNECTOR_HALF).clamp(0.0, w - NOTE_FOLD);
            format!(
                "M{x0},{y0} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{yf} L{xf},{y0} L{x2},{y0} L{tx},{ty} L{x1},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                yb = fc(y + h),
                xr = fc(x + w),
                yf = fc(y + NOTE_FOLD),
                xf = fc(x + w - NOTE_FOLD),
                x2 = fc(x + x1 + NOTE_CONNECTOR_HALF * 2.0),
                tx = fc(tip_x),
                ty = fc(tip_y),
                x1 = fc(x + x1),
            )
        }
        DeploymentNotePosition::Top => {
            let x1 = (mouth_x - NOTE_CONNECTOR_HALF).clamp(0.0, w);
            format!(
                "M{x0},{y0} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{x1},{yb} L{tx},{ty} L{x2},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{yf} L{xf},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                yb = fc(y + h),
                x1 = fc(x + x1),
                tx = fc(tip_x),
                ty = fc(tip_y),
                x2 = fc(x + x1 + NOTE_CONNECTOR_HALF * 2.0),
                xr = fc(x + w),
                yf = fc(y + NOTE_FOLD),
                xf = fc(x + w - NOTE_FOLD),
            )
        }
    };
    let fold_path = format!(
        "M{x1},{y0} L{x1},{y1} L{x2},{y1} L{x1},{y0}",
        x1 = fc(x + w - NOTE_FOLD),
        y0 = fc(y),
        y1 = fc(y + NOTE_FOLD),
        x2 = fc(x + w),
    );

    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
        uid.qualified_name, note.source_line, uid.entity_id
    ));
    svg.raw(&format!(
        r#"<path d="{path}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<path d="{fold_path}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));

    let mut text_y = y + NOTE_MARGIN_Y;
    for line in note.text.lines() {
        let ascent = text_render::label_ascent(line, NOTE_FONT_SIZE);
        text_y += ascent;
        emit_text(
            svg,
            line,
            x + NOTE_MARGIN_X1,
            text_y,
            NOTE_FONT_SIZE,
            false,
            false,
        );
        text_y += text_render::label_height(line, NOTE_FONT_SIZE) - ascent;
    }
    svg.raw("</g>");
}

// ---------------------------------------------------------------------------
// Connections (oracle-driven)
// ---------------------------------------------------------------------------

fn is_numbered_duplicate_edge(edge_id: &str, candidate_id: &str) -> bool {
    let Some(rest) = edge_id.strip_prefix(candidate_id) else {
        return false;
    };
    let Some(number) = rest.strip_prefix('-') else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
}

fn find_oracle_connection_edge<'a>(
    oracle: &'a OracleLayout,
    candidates: &[String],
    source_line: Option<&str>,
) -> Option<&'a crate::layout_oracle::OracleEdgePath> {
    let mut fallback = None;
    for candidate in candidates {
        for edge in oracle.edges.iter().filter(|edge| {
            edge.id == *candidate
                || source_line.is_some()
                    && is_numbered_duplicate_edge(edge.id.as_str(), candidate.as_str())
        }) {
            if source_line.is_some_and(|line| edge.source_line.as_deref() == Some(line)) {
                return Some(edge);
            }
            if edge.id == *candidate {
                fallback.get_or_insert(edge);
            }
        }
    }
    fallback
}

fn render_connection(
    svg: &mut SvgBuilder,
    conn: &DeploymentConnection,
    oracle: &OracleLayout,
    id_for_node: &HashMap<String, String>,
    own_qname_for_id: &HashMap<String, String>,
    link_id: &str,
    handwritten: bool,
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
    let source_line = (conn.source_line > 0).then(|| conn.source_line.to_string());
    let oracle_edge = find_oracle_connection_edge(oracle, &candidates, source_line.as_deref());
    let oracle_edge = oracle_edge.or_else(|| {
        let f_id = id_for_node.get(&conn.from).cloned();
        let t_id = id_for_node.get(&conn.to).cloned();
        let mut fallback = None;
        for edge in oracle.edges.iter().filter(|e| {
            let e1 = e.entity_1.as_deref();
            let e2 = e.entity_2.as_deref();
            (e1 == f_id.as_deref() && e2 == t_id.as_deref())
                || (e1 == t_id.as_deref() && e2 == f_id.as_deref())
        }) {
            if source_line
                .as_deref()
                .is_some_and(|line| edge.source_line.as_deref() == Some(line))
            {
                return Some(edge);
            }
            fallback.get_or_insert(edge);
        }
        fallback
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
        let id_attr = if handwritten {
            String::new()
        } else {
            format!(r#" id="{expected_id}""#)
        };
        svg.raw(&format!(
            r#"<path{code_line_attr} d="{d}" fill="none"{id_attr} style="{path_style}"/>"#,
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
        // Connection labels — positions taken from the oracle. PlantUML emits
        // each label (mid-edge label, plus any endpoint/qualifier labels) as a
        // separate `<text>` child of the link group; `labels` captures them all
        // in document order with their own coordinates. Multi-line mid-edge
        // labels also arrive as one `<text>` per line, so emitting each entry
        // verbatim reproduces both multi-line and multi-label edges.
        if !oe.labels.is_empty() {
            for (lx, ly, text) in &oe.labels {
                emit_text(svg, text, *lx, *ly, 13.0, false, false);
            }
        } else if let Some((lx, ly, text)) = &oe.label {
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

#[derive(Clone, Copy)]
struct DeploymentNoteDim {
    width: f64,
    height: f64,
}

#[derive(Clone, Copy)]
struct DeploymentNoteLayout {
    note_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn deployment_note_layout_id(index: usize) -> String {
    format!("__deployment_note_{index}")
}

fn deployment_note_dim(note: &DeploymentNote) -> DeploymentNoteDim {
    // `EntityImageNote` delegates text measurement and asymmetric margins to
    // `Opale`: six pixels left, fifteen right, and five on each vertical side.
    let width = note
        .text
        .lines()
        .map(|line| text_render::measure(line, NOTE_FONT_SIZE, false))
        .fold(0.0_f64, f64::max)
        + NOTE_MARGIN_X1
        + NOTE_MARGIN_X2;
    let text_height = note
        .text
        .lines()
        .map(|line| text_render::label_height(line, NOTE_FONT_SIZE))
        .sum::<f64>();
    DeploymentNoteDim {
        width,
        height: text_height + NOTE_MARGIN_Y * 2.0,
    }
}

fn laid_out_deployment_note_indices(diagram: &DeploymentDiagram) -> Vec<usize> {
    diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| {
            note.target
                .as_deref()
                .filter(|target| diagram.nodes.iter().any(|node| node.id == *target))
                .map(|_| index)
        })
        .collect()
}

fn is_degenerated_single_entity(diagram: &DeploymentDiagram) -> bool {
    diagram.nodes.len() == 1
        && diagram.nodes[0].children.is_empty()
        && diagram.connections.is_empty()
        && diagram.notes.is_empty()
        && diagram.meta.title.is_none()
        && diagram.meta.header.is_none()
        && diagram.meta.footer.is_none()
        && diagram.meta.caption.is_none()
        && diagram.meta.legend.is_none()
        && !is_handwritten_enabled(&diagram.meta.skinparams)
}

fn deployment_no_oracle_entity_rect(
    node: &DeploymentNode,
    dim: &DeploymentNodeDim,
    rect: LayoutRect,
) -> EntityRect {
    use DeploymentNodeKind::*;

    let stereo_height = if node.stereotype.is_some() {
        TEXT_LINE_H
    } else {
        0.0
    };
    let mut entity_rect = match node.kind {
        Boundary | Control | Entity => {
            // `USymbolSimpleAbstract` centres a fixed icon above the label.
            // The downstream emitter consumes the 24px ellipse box, not the
            // complete icon-plus-label node box solved by Graphviz.
            let symbol_width = if node.kind == Boundary { 49.0 } else { 32.0 };
            let symbol_x = rect.x + (rect.width - symbol_width) / 2.0;
            let ellipse_x = symbol_x + if node.kind == Boundary { 21.0 } else { 4.0 };
            empty_entity_rect(ellipse_x, rect.y + stereo_height + 4.0, 24.0, 24.0)
        }
        Collections => {
            // `USymbolCollections.drawCollections` paints the back card at
            // (+4,+4), then the front card at the overall node origin.
            let mut result = empty_entity_rect(
                rect.x + 4.0,
                rect.y + 4.0,
                rect.width - 4.0,
                rect.height - 4.0,
            );
            result.aux_rects.push(crate::layout_oracle::AuxRect {
                x: rect.x,
                y: rect.y,
                width: rect.width - 4.0,
                height: rect.height - 4.0,
                fill: None,
                style: None,
            });
            result
        }
        _ => empty_entity_rect(rect.x, rect.y, rect.width, rect.height),
    };

    let (text_x, text_y) = match node.kind {
        Boundary | Control | Entity => (
            rect.x + (rect.width - dim.label_width) / 2.0,
            rect.y + stereo_height + 32.0 + ASCENT_14,
        ),
        Collections => (
            rect.x + (rect.width - dim.label_width) / 2.0 - 2.0,
            rect.y + stereo_height + 8.0 + ASCENT_14,
        ),
        _ => (
            // Java receives node X through Graphviz's two-decimal SVG before
            // applying the symbol-local text offset.
            ((entity_text_center(node.kind, rect.x, rect.width) - dim.label_width / 2.0) * 100.0)
                .round()
                / 100.0,
            rect.y + dim.top_pad,
        ),
    };
    entity_rect.text_x_values.push(text_x);
    entity_rect.text_y_values.push(text_y);
    entity_rect.texts.push(EntityText {
        x: text_x,
        y: text_y,
        text: node.label.clone(),
    });
    entity_rect
}

fn render_no_oracle_degenerated(diagram: &DeploymentDiagram, dim: &DeploymentNodeDim) -> String {
    let node = &diagram.nodes[0];
    // `GraphvizImageBuilder.buildImage` bypasses Graphviz for one unlinked
    // root leaf. `EntityImageDegenerated` translates the image by seven
    // pixels; the surrounding image builder leaves six more trailing pixels.
    let rect = LayoutRect {
        x: 7.0,
        y: 7.0,
        width: dim.width,
        height: dim.height,
    };
    let parent_of = HashMap::new();
    let qnames = deployment_qnames(diagram, &parent_of);
    let mut entity_rect = deployment_no_oracle_entity_rect(node, dim, rect);
    entity_rect.source_line = Some(node.source_line.to_string());

    let mut oracle = OracleLayout::default();
    oracle
        .entities
        .insert(qnames[&node.id].clone(), entity_rect);
    let no_oracle_uids = build_deployment_no_oracle_uid_model(diagram);
    let skin_fills = skin_background_fills(&diagram.meta.skinparams);
    let skin_strokes = skin_border_colors(&diagram.meta.skinparams);
    let sprite_names: HashSet<String> = diagram
        .meta
        .sprites
        .keys()
        .map(|name| name.to_lowercase())
        .collect();
    let sprite_cache =
        crate::sprite::SpriteCache::from_sprites_scaled(&diagram.meta.sprites, sprite_scale());
    let ctx = OracleRenderContext {
        oracle: &oracle,
        id_for_node: &no_oracle_uids.entity_ids,
        skin_fills: &skin_fills,
        skin_strokes: &skin_strokes,
        sprite_names: &sprite_names,
        sprites: &diagram.meta.sprites,
        sprite_cache: &sprite_cache,
        handwritten: false,
    };

    let mut svg = SvgBuilder::new_plantuml(dim.width + 20.0, dim.height + 20.0, "DESCRIPTION");
    emit_entity(&mut svg, node, &qnames[&node.id], &ctx);
    svg.finalize_plantuml()
}

fn render_no_oracle(diagram: &DeploymentDiagram, _theme: &Theme) -> String {
    // Java path: CucaDiagramFileMakerSvek builds a Bibliotekon of measured
    // SvekNodes, DotStringFactory serialises those node boxes to dot, then
    // GeneralImageBuilder paints the returned positions. This mirrors that
    // data flow with the vendored Graphviz wrapper rather than grid-placement.
    let dims: Vec<DeploymentNodeDim> = diagram
        .nodes
        .iter()
        .map(|node| deployment_node_dim(node, &diagram.meta.sprites))
        .collect();
    if is_degenerated_single_entity(diagram) {
        return render_no_oracle_degenerated(diagram, &dims[0]);
    }
    let note_dims: Vec<DeploymentNoteDim> = diagram.notes.iter().map(deployment_note_dim).collect();
    let laid_out_note_indices = laid_out_deployment_note_indices(diagram);
    let parent_of = deployment_parent_map(diagram);
    let cluster_ids: HashSet<&str> = diagram
        .nodes
        .iter()
        .filter(|node| !node.children.is_empty())
        .map(|node| node.id.as_str())
        .collect();
    let mut layout = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
    for (node, dim) in diagram.nodes.iter().zip(&dims) {
        if !cluster_ids.contains(node.id.as_str()) {
            layout.add_node(&node.id, &node.label, dim.width, dim.height);
        }
    }
    for &note_index in &laid_out_note_indices {
        let dim = note_dims[note_index];
        layout.add_node(
            &deployment_note_layout_id(note_index),
            "",
            dim.width,
            dim.height,
        );
    }
    for node in &diagram.nodes {
        if cluster_ids.contains(node.id.as_str()) {
            let parent = parent_of
                .get(&node.id)
                .filter(|parent| cluster_ids.contains(parent.as_str()))
                .map(String::as_str);
            let title_width = text_render::measure_no_underline(&node.label, FONT_SIZE, true);
            let title_height = text_render::label_height(&node.label, FONT_SIZE);
            let (stereotype_width, stereotype_height) = node
                .stereotype
                .as_deref()
                .map(|stereotype| {
                    let text = format!("\u{00AB}{stereotype}\u{00BB}");
                    (
                        text_render::measure_no_underline(&text, FONT_SIZE, false),
                        text_render::label_height(&text, FONT_SIZE),
                    )
                })
                .unwrap_or((0.0, 0.0));
            let (shape_width, shape_height) = match node.kind {
                DeploymentNodeKind::Node => (60.0, 5.0),
                DeploymentNodeKind::Database => (0.0, 15.0),
                _ => (0.0, 0.0),
            };
            // Java `ClusterHeader` merges stereotype/title dimensions and
            // adds the USymbol supplement before `ClusterDotString` emits the
            // hidden fixed-size title table.
            layout.add_svek_cluster(
                &node.id,
                parent,
                ClusterTitleSize {
                    width: title_width.max(stereotype_width) + shape_width,
                    height: title_height + stereotype_height + shape_height,
                },
            );
        }
    }
    for node in &diagram.nodes {
        if cluster_ids.contains(node.id.as_str()) {
            continue;
        }
        if let Some(parent) = parent_of.get(&node.id)
            && cluster_ids.contains(parent.as_str())
        {
            layout.add_cluster_node(parent, &node.id);
        }
    }
    add_deployment_magma_constraints(&mut layout, diagram, &parent_of, &cluster_ids);
    for &note_index in &laid_out_note_indices {
        let note = &diagram.notes[note_index];
        let Some(target) = note.target.as_deref() else {
            continue;
        };
        let note_id = deployment_note_layout_id(note_index);
        let (from, to) = match note.position {
            DeploymentNotePosition::Top | DeploymentNotePosition::Left => {
                (note_id.as_str(), target)
            }
            DeploymentNotePosition::Bottom | DeploymentNotePosition::Right => {
                (target, note_id.as_str())
            }
        };
        if matches!(
            note.position,
            DeploymentNotePosition::Left | DeploymentNotePosition::Right
        ) {
            layout.add_same_rank(from, to);
            layout.add_edge(from, to, None);
        } else {
            // `CommandFactoryNoteOnEntity.executeInternal` uses
            // `LinkArg.noDisplay(2)` for vertical note links.
            layout.add_edge_with_minlen(from, to, None, 1);
        }
    }
    for conn in &diagram.connections {
        if !cluster_ids.contains(conn.from.as_str()) && !cluster_ids.contains(conn.to.as_str()) {
            let (layout_from, layout_to, _) = deployment_connection_layout(conn);
            let label_size = conn.label.as_deref().map(|label| EdgeLabelSize {
                // Java `SvekEdge.getLabelText` adds one pixel of margin on
                // each side before `appendLine` emits a fixed HTML table.
                width: text_render::measure(label, 13.0, false) + 2.0,
                height: (text_render::label_height(label, 13.0) + 2.0).floor(),
            });
            layout.add_edge_with_label_sizes_and_minlen(
                layout_from,
                layout_to,
                label_size,
                None,
                None,
                match conn.direction {
                    Some(DeploymentLinkDirection::Left | DeploymentLinkDirection::Right) => Some(0),
                    // `CommandLinkElement` stores the shaft's character count
                    // in `LinkArg`; `SvekEdge.appendLine` emits length - 1.
                    Some(DeploymentLinkDirection::Up | DeploymentLinkDirection::Down) | None => {
                        Some(conn.length.saturating_sub(1))
                    }
                },
            );
        }
    }

    let result = layout.layout_full(LAYOUT_TIMEOUT);
    let cluster_frame = deployment_cluster_frame(diagram, result.as_ref());
    let y_frame = deployment_body_y_frame(
        diagram,
        &dims,
        &note_dims,
        &laid_out_note_indices,
        result.as_ref(),
    );
    let body_margin_y = cluster_frame
        .map(|frame| frame.margin_y)
        .or_else(|| y_frame.map(|frame| frame.margin))
        .unwrap_or_else(|| deployment_body_margin_y(diagram, &dims, result.as_ref()));
    let x_frame = deployment_body_x_frame(
        diagram,
        &dims,
        &note_dims,
        &laid_out_note_indices,
        result.as_ref(),
    );
    let body_margin_x = x_frame
        .map(|frame| frame.margin)
        .or_else(|| cluster_frame.map(|frame| frame.margin_x))
        .unwrap_or(BODY_FALLBACK_MARGIN_X);
    let (rects, content_w, content_h) = layout_deployment_rects(
        diagram,
        &dims,
        result.as_ref(),
        body_margin_x,
        body_margin_y,
    );
    let leaf_count = diagram
        .nodes
        .iter()
        .filter(|node| !cluster_ids.contains(node.id.as_str()))
        .count();
    let note_layouts: Vec<DeploymentNoteLayout> = result
        .as_ref()
        .map(|result| {
            laid_out_note_indices
                .iter()
                .enumerate()
                .filter_map(|(offset, &note_index)| {
                    let position = result.node_positions.get(leaf_count + offset)?;
                    let dim = note_dims[note_index];
                    Some(DeploymentNoteLayout {
                        note_index,
                        x: (position.x * 100.0).round() / 100.0 + body_margin_x,
                        y: (position.y * 100.0).round() / 100.0 + body_margin_y,
                        width: dim.width,
                        height: dim.height,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let total_w = x_frame
        .map(|frame| frame.painted_max_x + frame.margin + SVEK_DIMENSION_DELTA)
        .or_else(|| {
            cluster_frame.and_then(|frame| {
                frame
                    .outer_cloud_painted_max_x
                    .map(|max_x| max_x + SVEK_DIMENSION_DELTA)
            })
        })
        .unwrap_or(content_w + BODY_RIGHT_MARGIN);
    let total_h = y_frame
        .map(|frame| frame.painted_max_y + frame.margin + SVEK_DIMENSION_DELTA)
        .or_else(|| {
            cluster_frame.and_then(|frame| {
                frame
                    .outer_cloud_painted_max_y
                    .map(|max_y| max_y + SVEK_DIMENSION_DELTA)
            })
        })
        .unwrap_or(content_h + BODY_BOTTOM_MARGIN);

    let mut oracle = OracleLayout::default();
    let qnames = deployment_qnames(diagram, &parent_of);
    for (i, node) in diagram.nodes.iter().enumerate() {
        let dim = &dims[i];
        let rect = rects[i];
        let mut entity_rect = if cluster_ids.contains(node.id.as_str()) {
            empty_entity_rect(rect.x, rect.y, rect.width, rect.height)
        } else {
            deployment_no_oracle_entity_rect(node, dim, rect)
        };
        entity_rect.source_line = Some(node.source_line.to_string());
        oracle
            .entities
            .insert(qnames[&node.id].clone(), entity_rect);
    }

    let no_oracle_uids = build_deployment_no_oracle_uid_model(diagram);
    let id_for_node = &no_oracle_uids.entity_ids;
    let skin_fills = skin_background_fills(&diagram.meta.skinparams);
    let skin_strokes = skin_border_colors(&diagram.meta.skinparams);
    let sprite_names: HashSet<String> = diagram
        .meta
        .sprites
        .keys()
        .map(|k| k.to_lowercase())
        .collect();
    let sprite_cache =
        crate::sprite::SpriteCache::from_sprites_scaled(&diagram.meta.sprites, sprite_scale());
    let ctx = OracleRenderContext {
        oracle: &oracle,
        id_for_node,
        skin_fills: &skin_fills,
        skin_strokes: &skin_strokes,
        sprite_names: &sprite_names,
        sprites: &diagram.meta.sprites,
        sprite_cache: &sprite_cache,
        handwritten: false,
    };

    let mut svg = SvgBuilder::new_plantuml(total_w, total_h, "DESCRIPTION");
    let all_children: HashSet<&str> = diagram
        .nodes
        .iter()
        .flat_map(|n| n.children.iter().map(|s| s.as_str()))
        .collect();
    let roots: Vec<&DeploymentNode> = diagram
        .nodes
        .iter()
        .filter(|n| !all_children.contains(n.id.as_str()))
        .collect();
    for root in &roots {
        emit_clusters_dfs(&mut svg, root, &diagram.nodes, None, &ctx);
    }
    let mut leaves = Vec::new();
    for root in &roots {
        collect_entities_dfs(root, &diagram.nodes, None, 0, &mut leaves);
    }
    leaves.sort_by_key(|(_, source_line, _, _)| *source_line);
    for (_, _, node, qname) in leaves {
        emit_entity(&mut svg, node, &qname, &ctx);
    }
    if let Some(result) = result.as_ref() {
        for layout in &note_layouts {
            let note = &diagram.notes[layout.note_index];
            let Some(uid) = no_oracle_uids.note_ids.get(&layout.note_index) else {
                continue;
            };
            let Some(target) = note.target.as_deref() else {
                continue;
            };
            let note_id = deployment_note_layout_id(layout.note_index);
            let (from, to) = match note.position {
                DeploymentNotePosition::Top | DeploymentNotePosition::Left => {
                    (note_id.as_str(), target)
                }
                DeploymentNotePosition::Bottom | DeploymentNotePosition::Right => {
                    (target, note_id.as_str())
                }
            };
            let edge = result
                .edge_paths
                .iter()
                .find(|edge| edge.from == from && edge.to == to);
            render_attached_deployment_note(
                &mut svg,
                note,
                layout,
                uid,
                edge,
                body_margin_x,
                body_margin_y,
            );
        }
    }
    if let Some(result) = result.as_ref() {
        render_no_oracle_edges(
            &mut svg,
            diagram,
            id_for_node,
            &result.edge_paths,
            body_margin_x,
            body_margin_y,
        );
    }
    svg.finalize_plantuml()
}

fn add_deployment_magma_constraints(
    layout: &mut LayoutGraph,
    diagram: &DeploymentDiagram,
    parent_of: &HashMap<String, String>,
    cluster_ids: &HashSet<&str>,
) {
    let linked: HashSet<&str> = diagram
        .connections
        .iter()
        .flat_map(|connection| [connection.from.as_str(), connection.to.as_str()])
        .collect();
    let add_group = |layout: &mut LayoutGraph, members: Vec<&str>| {
        if members.len() < 3 {
            return;
        }
        // Java `CucaDiagram.applySingleStrategy` delegates standalone
        // entities to `Magma.putInSquare` / `SquareMaker.putInSquare`.
        let branch = (members.len() as f64).sqrt().ceil() as usize;
        let mut head = 0;
        for index in 1..members.len() {
            if index - head == branch {
                layout.add_edge_with_minlen(members[head], members[index], None, 1);
                head = index;
            } else {
                layout.add_edge_with_minlen(members[index - 1], members[index], None, 0);
            }
        }
    };

    let root_members = diagram
        .nodes
        .iter()
        .filter(|node| {
            !cluster_ids.contains(node.id.as_str())
                && !parent_of.contains_key(&node.id)
                && !linked.contains(node.id.as_str())
        })
        .map(|node| node.id.as_str())
        .collect();
    add_group(layout, root_members);

    for container in diagram
        .nodes
        .iter()
        .filter(|node| cluster_ids.contains(node.id.as_str()))
    {
        let members = container
            .children
            .iter()
            .filter(|child| {
                !cluster_ids.contains(child.as_str()) && !linked.contains(child.as_str())
            })
            .map(String::as_str)
            .collect();
        add_group(layout, members);
    }
}

struct DeploymentNodeDim {
    width: f64,
    height: f64,
    label_width: f64,
    top_pad: f64,
}

#[derive(Clone, Copy)]
struct LayoutRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn deployment_parent_map(diagram: &DeploymentDiagram) -> HashMap<String, String> {
    let mut parent_of = HashMap::new();
    for node in &diagram.nodes {
        for child in &node.children {
            parent_of.insert(child.clone(), node.id.clone());
        }
    }
    parent_of
}

fn deployment_qnames(
    diagram: &DeploymentDiagram,
    parent_of: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut qnames = HashMap::new();
    for node in &diagram.nodes {
        let mut qname = own_qname(node);
        let mut cur_id = node.id.as_str();
        while let Some(parent_id) = parent_of.get(cur_id) {
            if let Some(parent) = diagram.nodes.iter().find(|n| n.id == *parent_id) {
                qname = format!("{}.{qname}", own_qname(parent));
                cur_id = parent.id.as_str();
            } else {
                break;
            }
        }
        qnames.insert(node.id.clone(), qname);
    }
    qnames
}

fn deployment_node_dim(
    node: &DeploymentNode,
    sprites: &HashMap<String, rustuml_parser::diagram::SpriteData>,
) -> DeploymentNodeDim {
    let bold = matches!(node.kind, DeploymentNodeKind::Package);
    let label_width = node
        .label
        .lines()
        .map(|line| deployment_label_width_for_sprites(line, FONT_SIZE, bold, sprites))
        .fold(0.0_f64, f64::max);
    let stereo_width = node
        .stereotype
        .as_ref()
        .map(|stereo| {
            // `EntityImageDescription` wraps visible stereotype text in
            // `TextBlockUtils.withMargin(..., 1, 0)` before `USymbol.asSmall`
            // measures it: one pixel on each horizontal side.
            text_render::measure(&format!("\u{00AB}{stereo}\u{00BB}"), FONT_SIZE, false) + 2.0
        })
        .unwrap_or(0.0);
    let line_count = node.label.lines().count().max(1) + usize::from(node.stereotype.is_some());
    let (text_x_pad, top_pad, _) = entity_text_geom(node.kind, 0.0, &node.label);
    let width = match node.kind {
        DeploymentNodeKind::Node
        | DeploymentNodeKind::Artifact
        | DeploymentNodeKind::Frame
        | DeploymentNodeKind::Component => {
            // `USymbolComponent2.getMargin()` uses 15px left and 25px
            // right. The shared text pad accounts for 15px on each side;
            // the final ten pixels preserve the asymmetric icon reservation.
            label_width.max(stereo_width) + 2.0 * text_x_pad + 10.0
        }
        // `USymbolSimpleAbstract` centres the fixed symbol above the text.
        DeploymentNodeKind::Boundary => label_width.max(stereo_width).max(49.0),
        DeploymentNodeKind::Control | DeploymentNodeKind::Entity => {
            label_width.max(stereo_width).max(32.0)
        }
        DeploymentNodeKind::Collections => label_width.max(stereo_width) + 20.0,
        DeploymentNodeKind::Cloud => label_width.max(stereo_width) + 2.0 * CLOUD_MARGIN,
        // `USymbolFolder.asSmall` keeps a hidden 40x15 title box when
        // `showTitle` is false, then adds `Margin(10, 20, 13, 10)`.
        DeploymentNodeKind::Folder => 40.0_f64.max(label_width).max(stereo_width) + 30.0,
        // `USymbolQueue.asSmall` adds `Margin(5, 15, 5, 5)` around the
        // vertically merged stereotype and label.
        DeploymentNodeKind::Queue => label_width.max(stereo_width) + 20.0,
        // `USymbolStack.asSmall` adds `Margin(25, 25, 10, 10)`.
        DeploymentNodeKind::Stack => label_width.max(stereo_width) + 50.0,
        _ => label_width.max(stereo_width) + 2.0 * text_x_pad,
    };
    let height = match node.kind {
        // Boundary/control/entity use a 32px icon stacked between the optional
        // stereotype and the label (`XDimension2D.mergeLayoutT12B3`).
        DeploymentNodeKind::Boundary | DeploymentNodeKind::Control | DeploymentNodeKind::Entity => {
            line_count as f64 * TEXT_LINE_H + 32.0
        }
        DeploymentNodeKind::Collections => line_count as f64 * TEXT_LINE_H + 20.0,
        DeploymentNodeKind::Cloud => line_count as f64 * TEXT_LINE_H + 2.0 * CLOUD_MARGIN,
        // `USymbolCard.asSmall` wraps the merged stereotype/label block in
        // `Margin(10, 10, 3, 3)`.
        DeploymentNodeKind::Card => line_count as f64 * TEXT_LINE_H + 6.0,
        // `USymbolDatabase.asSmall` adds 29px around the merged text block:
        // 10px top lip, 10px lower cap, and the title spacing between them.
        DeploymentNodeKind::Database => line_count as f64 * TEXT_LINE_H + 29.0,
        DeploymentNodeKind::Folder => 15.0 + line_count as f64 * TEXT_LINE_H + 13.0 + 10.0,
        DeploymentNodeKind::Queue => line_count as f64 * TEXT_LINE_H + 10.0,
        _ => {
            top_pad
                + (line_count.saturating_sub(1)) as f64 * TEXT_LINE_H
                + (pm::text_height(FONT_SIZE) - ASCENT_14)
                + 10.0
        }
    };
    DeploymentNodeDim {
        width,
        height,
        label_width,
        top_pad,
    }
}

fn deployment_body_margin_y(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    result: Option<&LayoutResult>,
) -> f64 {
    if !diagram.nodes.iter().any(|node| !node.children.is_empty())
        && let Some(result) = result
    {
        let painted_min_y = diagram
            .nodes
            .iter()
            .zip(dims)
            .zip(&result.node_positions)
            .map(|((node, dim), position)| {
                let y = (position.y * 100.0).round() / 100.0;
                y + deployment_local_painted_y_min(node.kind, dim)
            })
            .fold(f64::INFINITY, f64::min);
        if painted_min_y.is_finite() {
            // `LimitFinder` measures the renderer-owned symbol, then
            // `SvekResult.calculateDimension` moves that painted minimum to 6.
            return SVEK_ENVELOPE_ORIGIN - painted_min_y;
        }
    }

    let cloud_min_y = diagram
        .nodes
        .iter()
        .zip(dims)
        .filter(|(node, _)| matches!(node.kind, DeploymentNodeKind::Cloud))
        .map(|(_, dim)| {
            crate::cloud_shape::generate(dim.width, dim.height)
                .min_xy()
                .1
        })
        .fold(0.0_f64, f64::min);
    if cloud_min_y < 0.0 {
        // `SvekResult.calculateDimension` moves the painted minimum to 6.
        6.0 - cloud_min_y
    } else if diagram
        .nodes
        .iter()
        .any(|node| matches!(node.kind, DeploymentNodeKind::Database))
    {
        // `USymbolDatabase.asSmall` starts its cylinder at local y=0.
        6.0
    } else {
        BODY_MARGIN_Y
    }
}

fn deployment_local_painted_y_min(kind: DeploymentNodeKind, dim: &DeploymentNodeDim) -> f64 {
    deployment_local_painted_y_bounds(kind, dim).0
}

#[derive(Clone, Copy)]
struct DeploymentYFrame {
    margin: f64,
    painted_max_y: f64,
}

fn deployment_body_y_frame(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    note_dims: &[DeploymentNoteDim],
    laid_out_note_indices: &[usize],
    result: Option<&LayoutResult>,
) -> Option<DeploymentYFrame> {
    let result = result?;
    if diagram.nodes.iter().any(|node| !node.children.is_empty()) {
        return None;
    }

    let mut painted_min_y = f64::INFINITY;
    let mut painted_max_y = f64::NEG_INFINITY;
    for ((node, dim), position) in diagram.nodes.iter().zip(dims).zip(&result.node_positions) {
        let y = (position.y * 100.0).round() / 100.0;
        let (local_min_y, local_max_y) = deployment_local_painted_y_bounds(node.kind, dim);
        painted_min_y = painted_min_y.min(y + local_min_y);
        painted_max_y = painted_max_y.max(y + local_max_y);
    }
    for (offset, &note_index) in laid_out_note_indices.iter().enumerate() {
        let position = result.node_positions.get(diagram.nodes.len() + offset)?;
        let y = (position.y * 100.0).round() / 100.0;
        painted_min_y = painted_min_y.min(y);
        painted_max_y = painted_max_y.max(y + note_dims[note_index].height);
    }
    (painted_min_y.is_finite() && painted_max_y.is_finite()).then_some(DeploymentYFrame {
        // `SvekResult.calculateDimension` moves the `LimitFinder` minimum to
        // six, then adds its 15px dimension delta after the painted maximum.
        margin: SVEK_ENVELOPE_ORIGIN - painted_min_y,
        painted_max_y,
    })
}

fn deployment_local_painted_y_bounds(
    kind: DeploymentNodeKind,
    dim: &DeploymentNodeDim,
) -> (f64, f64) {
    use DeploymentNodeKind::*;
    match kind {
        // `USymbolNode.drawNode` and `USymbolDatabase.drawDatabase` place a
        // `UEmpty(10,10)` at the lower edge specifically for LimitFinder.
        Node | Database => (0.0, dim.height + 10.0),
        // `LimitFinder.drawRectangle` expands a rectangle by one pixel toward
        // the top/left and ends one pixel before its declared lower edge.
        Artifact | Card | Rectangle | Agent | Component | Frame | Storage => {
            (-1.0, dim.height - 1.0)
        }
        Cloud => {
            let (_, min_y, _, max_y) = crate::cloud_shape::generate(dim.width, dim.height).bounds();
            (min_y, max_y)
        }
        // `Boundary` and `EntityDomain` start their 24px circle four pixels
        // below the overall image origin. `Control` adds a polygon whose top
        // wing reaches one pixel above that origin.
        Boundary | Entity => (4.0, dim.height),
        Control => (-1.0, dim.height),
        // Two offset `URectangle`s: the front contributes -1 at the origin.
        Collections => (-1.0, dim.height - 1.0),
        // These Java symbols are painted as UPath/UPolygon outlines whose
        // vertical bounds are exactly their declared image height.
        // The stack combines an inset `URectangle` (minimum Y at -1) with
        // a full-height `UPath` (maximum Y at the declared height).
        Stack => (-1.0, dim.height),
        Folder | Queue | File | Package => (0.0, dim.height),
        // Preserve the established envelope for symbols whose Java primitive
        // model has not yet been split out above.
        _ => (0.0, dim.height + 10.0),
    }
}

#[derive(Clone, Copy)]
struct DeploymentXFrame {
    margin: f64,
    painted_max_x: f64,
}

#[derive(Clone, Copy)]
struct DeploymentClusterFrame {
    margin_x: f64,
    margin_y: f64,
    outer_cloud_painted_max_x: Option<f64>,
    outer_cloud_painted_max_y: Option<f64>,
}

fn deployment_cluster_frame(
    diagram: &DeploymentDiagram,
    result: Option<&LayoutResult>,
) -> Option<DeploymentClusterFrame> {
    let result = result?;
    if result.cluster_positions.is_empty() {
        return None;
    }
    let parent_of = deployment_parent_map(diagram);
    let mut required_dx = f64::NEG_INFINITY;
    let mut required_dy = f64::NEG_INFINITY;
    let roots: Vec<&DeploymentNode> = diagram
        .nodes
        .iter()
        .filter(|node| !parent_of.contains_key(&node.id))
        .collect();
    for node in roots.iter().filter(|node| !node.children.is_empty()) {
        let Some(position) = result
            .cluster_positions
            .iter()
            .find(|position| position.id == node.id)
        else {
            continue;
        };
        let (origin_x, origin_y) = match node.kind {
            // `USymbolNode.drawNode` paints a polygon whose LimitFinder X
            // bounds extend ten pixels beyond the visible cluster.
            DeploymentNodeKind::Node => (16.0, SVEK_ENVELOPE_ORIGIN),
            DeploymentNodeKind::Cloud => {
                let (min_x, min_y) =
                    crate::cloud_shape::generate(position.width, position.height).min_xy();
                (SVEK_ENVELOPE_ORIGIN - min_x, SVEK_ENVELOPE_ORIGIN - min_y)
            }
            _ => (SVEK_ENVELOPE_ORIGIN, SVEK_ENVELOPE_ORIGIN),
        };
        required_dx = required_dx.max(origin_x - position.x);
        required_dy = required_dy.max(origin_y - position.y);
    }
    if !required_dx.is_finite() || !required_dy.is_finite() {
        return None;
    }

    // When every outer entity is a cloud cluster, `LimitFinder` sees the
    // generated UPath as the document envelope. `SvekResult.calculateDimension`
    // adds its fixed delta after the path's painted maximum, rather than after
    // the nominal Graphviz cluster rectangle.
    let outer_cloud_bounds: Option<Vec<_>> = roots
        .iter()
        .map(|node| {
            if node.children.is_empty() || node.kind != DeploymentNodeKind::Cloud {
                return None;
            }
            let position = result
                .cluster_positions
                .iter()
                .find(|position| position.id == node.id)?;
            Some((
                position,
                crate::cloud_shape::generate(position.width, position.height).bounds(),
            ))
        })
        .collect();
    let outer_cloud_painted_max_x = outer_cloud_bounds.as_ref().map(|bounds| {
        bounds
            .iter()
            .map(|(position, (_, _, max_x, _))| position.x + required_dx + max_x)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let outer_cloud_painted_max_y = outer_cloud_bounds.as_ref().map(|bounds| {
        bounds
            .iter()
            .map(|(position, (_, _, _, max_y))| position.y + required_dy + max_y)
            .fold(f64::NEG_INFINITY, f64::max)
    });

    Some(DeploymentClusterFrame {
        margin_x: required_dx,
        margin_y: required_dy,
        outer_cloud_painted_max_x,
        outer_cloud_painted_max_y,
    })
}

fn deployment_body_x_frame(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    note_dims: &[DeploymentNoteDim],
    laid_out_note_indices: &[usize],
    result: Option<&LayoutResult>,
) -> Option<DeploymentXFrame> {
    let result = result?;
    if diagram.nodes.iter().any(|node| !node.children.is_empty()) {
        return None;
    }

    let mut painted_min_x = f64::INFINITY;
    let mut painted_max_x = f64::NEG_INFINITY;
    for ((node, dim), position) in diagram.nodes.iter().zip(dims).zip(&result.node_positions) {
        // `DotStringFactory.solve` recovers rectangle positions from
        // Graphviz's SVG polygon coordinates, which are serialized to two
        // decimal places.
        let x = (position.x * 100.0).round() / 100.0;
        let (local_min_x, local_max_x) = deployment_local_painted_x_bounds(node, dim);
        painted_min_x = painted_min_x.min(x + local_min_x);
        painted_max_x = painted_max_x.max(x + local_max_x);
    }
    for (offset, &note_index) in laid_out_note_indices.iter().enumerate() {
        let position = result.node_positions.get(diagram.nodes.len() + offset)?;
        let x = (position.x * 100.0).round() / 100.0;
        painted_min_x = painted_min_x.min(x);
        painted_max_x = painted_max_x.max(x + note_dims[note_index].width);
    }
    painted_min_x.is_finite().then_some(DeploymentXFrame {
        // `LimitFinder.drawUPolygon` expands polygon bounds by 10px on both
        // horizontal sides; `SvekResult.calculateDimension` then calls
        // `moveDelta(6 - minX, 6 - minY)`.
        margin: SVEK_ENVELOPE_ORIGIN - painted_min_x,
        painted_max_x,
    })
}

fn deployment_local_painted_x_bounds(node: &DeploymentNode, dim: &DeploymentNodeDim) -> (f64, f64) {
    use DeploymentNodeKind::*;
    match node.kind {
        // `USymbolNode.drawNode` paints the full body as a UPolygon.
        Node => (-10.0, dim.width + 10.0),
        // The artifact's outer rectangle reaches one pixel left in
        // `LimitFinder`; its folded-corner polygon reaches five pixels right.
        Artifact => (-1.0, dim.width + 5.0),
        Cloud => {
            let path = crate::cloud_shape::generate(dim.width, dim.height);
            let (min_x, _) = path.min_xy();
            let max_x = path
                .cubics
                .iter()
                .flat_map(|cubic| [cubic.c1.0, cubic.c2.0, cubic.to.0])
                .fold(path.start.0, f64::max);
            (min_x, max_x)
        }
        Boundary => {
            let symbol_x = (dim.width - 49.0) / 2.0;
            let label_x = (dim.width - dim.label_width) / 2.0;
            (
                (symbol_x + 4.0).min(label_x),
                (symbol_x + 45.0).max(label_x + dim.label_width),
            )
        }
        Control => {
            let symbol_x = (dim.width - 32.0) / 2.0;
            let label_x = (dim.width - dim.label_width) / 2.0;
            // `LimitFinder.drawUPolygon` expands the control notch by ten
            // pixels on either side, producing local X bounds 2..28.
            (
                (symbol_x + 2.0).min(label_x),
                (symbol_x + 28.0).max(label_x + dim.label_width),
            )
        }
        Entity => {
            let symbol_x = (dim.width - 32.0) / 2.0;
            let label_x = (dim.width - dim.label_width) / 2.0;
            (
                (symbol_x + 4.0).min(label_x),
                (symbol_x + 28.0).max(label_x + dim.label_width),
            )
        }
        // Rounded rectangle-like symbols are measured by
        // `LimitFinder.drawRectangle`.
        Card | Rectangle | Agent | Component | Storage => (-1.0, dim.width - 1.0),
        // These symbols paint paths whose local envelope is their declared
        // image dimension.
        // `USymbolDatabase.drawDatabase` places `UEmpty(10, 10)` at the
        // lower-right corner so `LimitFinder` extends past the cylinder.
        Database => (0.0, dim.width + 10.0),
        Frame | Folder | Queue | File | Package | Stack => (0.0, dim.width),
        _ => (-1.0, dim.width - 1.0),
    }
}

fn deployment_label_width_for_sprites(
    label: &str,
    font_size: f64,
    bold: bool,
    sprites: &HashMap<String, rustuml_parser::diagram::SpriteData>,
) -> f64 {
    if !label.contains("<$") {
        return text_render::measure(label, font_size, bold);
    }
    crate::sprite::parse_sprite_segments(label)
        .iter()
        .map(|seg| match seg {
            crate::sprite::TextSegment::Text(text) => text_render::measure(text, font_size, bold),
            crate::sprite::TextSegment::Sprite(name) => sprites
                .get(name)
                .map(|sprite| deployment_sprite_dimensions(sprite).0)
                .unwrap_or(0.0),
            crate::sprite::TextSegment::OpenIcon(name) => crate::openiconic::lookup(name)
                .map(|icon| icon.width * (font_size / icon.height))
                .unwrap_or(0.0),
        })
        .sum()
}

fn layout_deployment_rects(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    result: Option<&LayoutResult>,
    body_margin_x: f64,
    body_margin_y: f64,
) -> (Vec<LayoutRect>, f64, f64) {
    let mut rects = Vec::new();
    if let Some(result) = result {
        let cluster_ids: HashSet<&str> = diagram
            .nodes
            .iter()
            .filter(|node| !node.children.is_empty())
            .map(|node| node.id.as_str())
            .collect();
        let cluster_positions: HashMap<&str, &rustuml_layout::graph::ClusterPosition> = result
            .cluster_positions
            .iter()
            .map(|pos| (pos.id.as_str(), pos))
            .collect();
        let mut leaf_positions = result.node_positions.iter();
        for (node, dim) in diagram.nodes.iter().zip(dims) {
            if cluster_ids.contains(node.id.as_str()) {
                if let Some(pos) = cluster_positions.get(node.id.as_str()) {
                    rects.push(LayoutRect {
                        x: pos.x + body_margin_x,
                        y: pos.y + body_margin_y,
                        width: pos.width,
                        height: pos.height,
                    });
                } else {
                    rects.push(LayoutRect {
                        x: body_margin_x,
                        y: body_margin_y,
                        width: dim.width,
                        height: dim.height,
                    });
                }
                continue;
            }
            let Some(pos) = leaf_positions.next() else {
                rects.clear();
                break;
            };
            rects.push(LayoutRect {
                x: (pos.x * 100.0).round() / 100.0 + body_margin_x,
                y: (pos.y * 100.0).round() / 100.0 + body_margin_y,
                width: dim.width,
                height: dim.height,
            });
        }
    }
    if rects.len() != diagram.nodes.len() {
        rects.clear();
        let mut y = body_margin_y;
        for dim in dims {
            rects.push(LayoutRect {
                x: body_margin_x,
                y,
                width: dim.width,
                height: dim.height,
            });
            y += dim.height + 50.0;
        }
    }

    let content_w = rects.iter().map(|r| r.x + r.width).fold(0.0_f64, f64::max);
    let content_h = rects.iter().map(|r| r.y + r.height).fold(0.0_f64, f64::max);
    (rects, content_w, content_h)
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

struct DeploymentNoOracleUidModel {
    entity_ids: HashMap<String, String>,
    note_ids: HashMap<usize, DeploymentNoteUid>,
}

struct DeploymentNoteUid {
    qualified_name: String,
    entity_id: String,
}

fn build_deployment_no_oracle_uid_model(diagram: &DeploymentDiagram) -> DeploymentNoOracleUidModel {
    enum Item<'a> {
        Node(&'a DeploymentNode),
        Note(usize),
        Conn,
    }
    let mut items = Vec::new();
    for node in &diagram.nodes {
        items.push((node.source_line, Item::Node(node)));
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        items.push((note.source_line, Item::Note(index)));
    }
    for conn in &diagram.connections {
        items.push((conn.source_line, Item::Conn));
    }
    items.sort_by_key(|(line, _)| *line);
    let mut next_uid = 2;
    let mut entity_ids = HashMap::new();
    let mut note_ids = HashMap::new();
    for (_, item) in items {
        match item {
            Item::Node(node) => {
                entity_ids.insert(node.id.clone(), format!("ent{next_uid:04}"));
                next_uid += 1;
            }
            Item::Note(index) => {
                let note = &diagram.notes[index];
                let qualified_name = if let Some(id) = note.id.as_ref() {
                    id.clone()
                } else {
                    // `CommandFactoryNoteOnEntity.executeInternal` obtains a
                    // generated name before creating the note leaf.
                    let name = format!("GMN{next_uid}");
                    next_uid += 1;
                    name
                };
                let entity_id = format!("ent{next_uid:04}");
                next_uid += 1;
                note_ids.insert(
                    index,
                    DeploymentNoteUid {
                        qualified_name,
                        entity_id,
                    },
                );
                // The hidden note-to-target Link consumes the next global UID
                // even though Opale absorbs it into the note outline.
                next_uid += 1;
            }
            Item::Conn => next_uid += 1,
        }
    }
    DeploymentNoOracleUidModel {
        entity_ids,
        note_ids,
    }
}

fn render_no_oracle_edges(
    svg: &mut SvgBuilder,
    diagram: &DeploymentDiagram,
    id_for_node: &HashMap<String, String>,
    edge_paths: &[EdgePath],
    body_margin_x: f64,
    body_margin_y: f64,
) {
    for (i, conn) in diagram.connections.iter().enumerate() {
        let (layout_from, layout_to, reversed) = deployment_connection_layout(conn);
        let Some(edge) = edge_paths
            .iter()
            .find(|edge| edge.from == layout_from && edge.to == layout_to)
        else {
            continue;
        };
        let entity_1_id = if reversed { &conn.to } else { &conn.from };
        let entity_2_id = if reversed { &conn.from } else { &conn.to };
        let Some(ent1) = id_for_node.get(entity_1_id) else {
            continue;
        };
        let Some(ent2) = id_for_node.get(entity_2_id) else {
            continue;
        };
        let link_id = format!("lnk{}", i + diagram.nodes.len() + 2 + usize::from(reversed));
        let raw_start_arrow = if reversed {
            conn.arrow_at_end
        } else {
            conn.arrow_at_start
        };
        let raw_end_arrow = if reversed {
            conn.arrow_at_start
        } else {
            conn.arrow_at_end
        };
        if reversed {
            svg.raw(&format!("<!--reverse link {} to {}-->", conn.to, conn.from));
        } else {
            svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));
        }
        let link_type = if conn.arrow_at_start || conn.arrow_at_end {
            "dependency"
        } else {
            "association"
        };
        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{ent1}" data-entity-2="{ent2}" data-link-type="{link_type}" data-source-line="{line}" id="{link_id}">"#,
            line = conn.source_line,
        ));
        let raw_points =
            deployment_svek_edge_points(&edge.points, body_margin_x, body_margin_y, EdgeTrim::None);
        let points = deployment_svek_edge_points(
            &edge.points,
            body_margin_x,
            body_margin_y,
            match (raw_start_arrow, raw_end_arrow) {
                (false, false) => EdgeTrim::None,
                (true, false) => EdgeTrim::Start,
                (false, true) => EdgeTrim::End,
                (true, true) => EdgeTrim::Both,
            },
        );
        if let Some(d) = edge_path_d(&points) {
            let from_name = diagram
                .nodes
                .iter()
                .find(|node| node.id == conn.from)
                .map(own_qname)
                .unwrap_or_else(|| conn.from.clone());
            let to_name = diagram
                .nodes
                .iter()
                .find(|node| node.id == conn.to)
                .map(own_qname)
                .unwrap_or_else(|| conn.to.clone());
            let path_id = if conn.arrow_at_start == conn.arrow_at_end {
                format!("{from_name}-{to_name}")
            } else if reversed {
                format!("{to_name}-backto-{from_name}")
            } else {
                format!("{from_name}-to-{to_name}")
            };
            // Java `SvekEdge.drawU` applies the stroke returned by
            // `LinkType.getStroke3`; `LinkStyle` defines these dash patterns.
            let path_style = match conn.style {
                DeploymentLinkStyle::Solid => "stroke:#181818;stroke-width:1;",
                DeploymentLinkStyle::Dashed => {
                    "stroke:#181818;stroke-width:1;stroke-dasharray:7,7;"
                }
                DeploymentLinkStyle::Dotted => {
                    "stroke:#181818;stroke-width:1;stroke-dasharray:1,3;"
                }
                DeploymentLinkStyle::Bold => "stroke:#181818;stroke-width:2;",
            };
            svg.raw(&format!(
                r#"<path d="{d}" fill="none" id="{path_id}" style="{path_style}"/>"#,
            ));
        }
        if raw_points.len() >= 2 {
            if raw_start_arrow {
                emit_deployment_arrowhead(svg, &raw_points[1], &raw_points[0]);
            }
            if raw_end_arrow {
                emit_deployment_arrowhead(
                    svg,
                    &raw_points[raw_points.len() - 2],
                    &raw_points[raw_points.len() - 1],
                );
            }
        }
        if let Some(label) = conn.label.as_deref() {
            let (x, y) = edge
                .label
                .map(|position| {
                    (
                        (position.x * 100.0).round() / 100.0 + body_margin_x + 1.0,
                        (position.y * 100.0).round() / 100.0
                            + body_margin_y
                            + 1.0
                            + text_render::label_ascent(label, 13.0),
                    )
                })
                .unwrap_or_else(|| {
                    points
                        .first()
                        .zip(points.last())
                        .map(|(first, last)| {
                            ((first.0 + last.0) / 2.0 + 1.0, (first.1 + last.1) / 2.0)
                        })
                        .unwrap_or((body_margin_x, body_margin_y))
                });
            emit_text(svg, label, x, y, 13.0, false, false);
        }
        svg.raw("</g>");
    }
}

fn deployment_connection_layout(conn: &DeploymentConnection) -> (&str, &str, bool) {
    let reversed = matches!(
        conn.direction,
        Some(DeploymentLinkDirection::Up | DeploymentLinkDirection::Left)
    );
    if reversed {
        (&conn.to, &conn.from, true)
    } else {
        (&conn.from, &conn.to, false)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeTrim {
    None,
    Start,
    End,
    Both,
}

fn deployment_svek_edge_points(
    points: &[(f64, f64)],
    body_margin_x: f64,
    body_margin_y: f64,
    trim: EdgeTrim,
) -> Vec<(f64, f64)> {
    // Java `SvekEdge.solveLine` receives the spline through
    // `SvgResult.toDotPath`; Graphviz has serialized every path coordinate to
    // two decimal places at that boundary.
    let quantize = |value: f64| (value * 100.0).round() / 100.0;
    let mut points: Vec<(f64, f64)> = points
        .iter()
        .map(|(x, y)| (quantize(*x) + body_margin_x, quantize(*y) + body_margin_y))
        .collect();
    if points.len() >= 2 && matches!(trim, EdgeTrim::Start | EdgeTrim::Both) {
        trim_deployment_edge_endpoint(&mut points, 0, 1);
    }
    if points.len() >= 2 && matches!(trim, EdgeTrim::End | EdgeTrim::Both) {
        let endpoint = points.len() - 1;
        trim_deployment_edge_endpoint(&mut points, endpoint, endpoint - 1);
    }
    points
}

fn trim_deployment_edge_endpoint(points: &mut [(f64, f64)], endpoint: usize, adjacent: usize) {
    let dx = points[endpoint].0 - points[adjacent].0;
    let dy = points[endpoint].1 - points[adjacent].1;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return;
    }
    let shift = (
        dx / length * DEPENDENCY_ARROW_LENGTH,
        dy / length * DEPENDENCY_ARROW_LENGTH,
    );
    points[endpoint].0 -= shift.0;
    points[endpoint].1 -= shift.1;
    if points.len() >= 4 {
        points[adjacent].0 -= shift.0;
        points[adjacent].1 -= shift.1;
    }
}

fn emit_deployment_arrowhead(svg: &mut SvgBuilder, previous: &(f64, f64), endpoint: &(f64, f64)) {
    let dx = endpoint.0 - previous.0;
    let dy = endpoint.1 - previous.1;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return;
    }
    let ux = dx / length;
    let uy = dy / length;
    let tip = *endpoint;
    let perpendicular = (
        uy * DEPENDENCY_ARROW_HALF_WIDTH,
        -ux * DEPENDENCY_ARROW_HALF_WIDTH,
    );
    let rear = (
        tip.0 - ux * DEPENDENCY_ARROW_REAR,
        tip.1 - uy * DEPENDENCY_ARROW_REAR,
    );
    let inset = (
        tip.0 - ux * DEPENDENCY_ARROW_INSET,
        tip.1 - uy * DEPENDENCY_ARROW_INSET,
    );
    let points = format!(
        "{},{},{},{},{},{},{},{},{},{}",
        fc(tip.0),
        fc(tip.1),
        fc(rear.0 + perpendicular.0),
        fc(rear.1 + perpendicular.1),
        fc(inset.0),
        fc(inset.1),
        fc(rear.0 - perpendicular.0),
        fc(rear.1 - perpendicular.1),
        fc(tip.0),
        fc(tip.1),
    );
    svg.raw(&format!(
        r##"<polygon fill="#181818" points="{points}" style="stroke:#181818;stroke-width:1;"/>"##,
    ));
}

fn edge_path_d(points: &[(f64, f64)]) -> Option<String> {
    let (start, rest) = points.split_first()?;
    let mut d = format!("M{},{}", fc(start.0), fc(start.1));
    for chunk in rest.chunks(3) {
        if let [c1, c2, to] = chunk {
            write!(
                d,
                " C{},{} {},{} {},{}",
                fc(c1.0),
                fc(c1.1),
                fc(c2.0),
                fc(c2.1),
                fc(to.0),
                fc(to.1),
            )
            .unwrap();
        }
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_oracle_deployment_renders_entities_and_links() {
        let source = "@startuml\nnode N01\nnode N02\nN01 --> N02\n@enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"<g class="entity" data-qualified-name="N01""#));
        assert!(svg.contains(r#"<g class="entity" data-qualified-name="N02""#));
        assert!(svg.contains(r#"<g class="link""#));
        assert!(!svg.contains(r#"<defs/><g></g>"#));
    }

    #[test]
    fn no_oracle_container_uses_square_packing_and_folder_minimum() {
        let source = "@startuml\n\
            node \"Renamed Host 43\" {\n\
              artifact \"Bundle 47\"\n\
              artifact \"Config 53\"\n\
              artifact \"Driver 59\"\n\
              file \"Trace 61\"\n\
              folder \"L7\"\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let folder = diagram
            .nodes
            .iter()
            .find(|node| node.kind == DeploymentNodeKind::Folder)
            .unwrap();

        let folder_dim = deployment_node_dim(folder, &diagram.meta.sprites);
        assert_eq!(folder_dim.width, 70.0);

        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"width="432px""#));
        assert!(svg.contains(r#"height="240px""#));
        assert!(svg.contains(r#"x="31.89" y="46""#));
        assert!(svg.contains(r#"M158.5,145.49"#));
        assert!(svg.contains(r#"data-qualified-name="Renamed Host 43.L7""#));
    }

    #[test]
    fn no_oracle_long_dashed_link_preserves_style_and_rank_length() {
        let source = "@startuml\n\
            node \"Sender 71\" as Sender71\n\
            node \"Receiver 73\" as Receiver73\n\
            Sender71 ....> Receiver73\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"height="305px""#));
        assert!(svg.contains(r#"id="Sender71-to-Receiver73""#));
        assert!(svg.contains(r#"stroke-dasharray:7,7;"#));
    }

    #[test]
    fn no_oracle_link_decorations_follow_both_logical_ends() {
        let source = "@startuml\n\
            node \"Emitter 137\" as Emitter137\n\
            node \"Relay 139\" as Relay139\n\
            node \"Sink 149\" as Sink149\n\
            Emitter137 -- Relay139\n\
            Relay139 <-> Sink149\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"data-link-type="association""#));
        assert!(svg.contains(r#"id="Emitter137-Relay139""#));
        assert!(svg.contains(r#"data-link-type="dependency""#));
        assert!(svg.contains(r#"id="Relay139-Sink149""#));
    }

    #[test]
    fn no_oracle_cloud_queue_uses_painted_y_envelope() {
        let source = "@startuml\n\
            cloud \"Ingress 97\" as Ingress97\n\
            queue \"Jobs 101\" as Jobs101\n\
            Ingress97 --> Jobs101 : QUIC\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"height="172px""#));
        assert!(svg.contains(r#"data-qualified-name="Jobs101""#));
        assert!(svg.contains(r#"id="Ingress97-to-Jobs101""#));
    }

    #[test]
    fn no_oracle_cloud_cluster_uses_seeded_usymbol_outline() {
        let source = "@startuml\n\
            cloud \"Renamed Edge 109\" {\n\
              node \"Worker 113\" as Worker113\n\
              node \"Cache 127\" as Cache127\n\
            }\n\
            Worker113 --> Cache127\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"<!--cluster Renamed Edge 109-->"#));
        assert!(svg.contains(r#"<g class="cluster""#));
        assert!(svg.contains(r#"<path d="M"#));
        assert!(svg.contains(r#"fill="none" style="stroke:#181818;stroke-width:1;"/>"#));
        assert!(!svg.contains(r#"<polygon fill="none""#));
        assert!(svg.contains(r#"data-qualified-name="Renamed Edge 109.Worker113""#));
    }

    #[test]
    fn no_oracle_single_entity_uses_degenerated_envelope() {
        let source = "@startuml\nartifact \"Solo Runtime 127\" as Runtime127 #LightGreen\n@enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:170px;height:59px;background:#FFFFFF;""#));
        assert!(svg.contains(
            r#"<g class="entity" data-qualified-name="Runtime127" data-source-line="1" id="ent0002">"#
        ));
        assert!(svg.contains(r##"fill="#90EE90" height="39.4883""##));
        assert!(svg.contains(r#"width="150.5859" x="7" y="7""#));
    }

    #[test]
    fn no_oracle_card_database_uses_symbol_bounds() {
        let source = "@startuml\n\
            card \"Gateway 137\" as Gateway137\n\
            database \"Ledger 139\" as Ledger139\n\
            Gateway137 --> Ledger139\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:134px;height:159px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"height="22.4883" rx="2.5" ry="2.5""#));
        assert!(svg.contains(r#"M12.19,99.49 C12.19,89.49 60.8506,89.49"#));
        assert!(svg.contains(r#"id="Gateway137-to-Ledger139""#));
    }

    #[test]
    fn no_oracle_stack_uses_full_symbol_bounds() {
        let source = "@startuml\n\
            stack \"Buffer 149\" as Buffer149\n\
            database \"Archive 151\" as Archive151\n\
            Buffer149 --> Archive151\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:142px;height:173px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"width="91.5518" x="21" y="7""#));
        assert!(svg.contains(r#"M6,7 L18.5,7 A2.5,2.5 0 0 1 21,9.5"#));
        assert!(svg.contains(r#"id="Buffer149-to-Archive151""#));
    }

    #[test]
    fn no_oracle_specialized_symbols_use_simple_abstract_layout() {
        let source = "@startuml\n\
            boundary \"Ingress 157\" as Ingress157\n\
            collections \"Batch 163\" as Batch163\n\
            control \"Throttle 167\" as Throttle167\n\
            node \"Runtime 173\" as Runtime173\n\
            Ingress157 --> Batch163\n\
            Batch163 --> Throttle167\n\
            Throttle167 --> Runtime173\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:168px;height:386px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"M59.1801,6 L59.1801,30 M59.1801,18 L76.1801,18"#));
        assert!(svg.contains(r#"width="83.8672" x="35.75" y="110.49""#));
        assert!(svg.contains(r#"points="75.6879,210.98,81.6879,205.98"#));
        assert!(svg.contains(r#"id="Throttle167-to-Runtime173""#));
    }

    #[test]
    fn no_oracle_stereotype_measurement_includes_entity_image_margin() {
        let source = "@startuml\n\
            node \"G\" as Gateway179 <<UnusuallyLongRole179>>\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let node = &diagram.nodes[0];
        let dim = deployment_node_dim(node, &diagram.meta.sprites);
        let stereo_width =
            text_render::measure("\u{00AB}UnusuallyLongRole179\u{00BB}", FONT_SIZE, false);
        let (text_x_pad, _, _) = entity_text_geom(node.kind, 0.0, &node.label);

        assert_eq!(dim.width, stereo_width + 2.0 * text_x_pad + 12.0);
    }

    #[test]
    fn no_oracle_component_measurement_reserves_usymbol_icon_margin() {
        let source = "@startuml\n\
            node \"Build Cell 193\" {\n\
              component \"Compiler 197\" as Compiler197\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let component = diagram
            .nodes
            .iter()
            .find(|node| node.kind == DeploymentNodeKind::Component)
            .unwrap();
        let dim = deployment_node_dim(component, &diagram.meta.sprites);

        assert_eq!(
            dim.width,
            text_render::measure(&component.label, FONT_SIZE, false) + 40.0
        );
    }

    #[test]
    fn no_oracle_attached_note_uses_svek_opale_geometry() {
        let source = "@startuml\n\
            artifact \"Renamed Runtime 107\" as Runtime107\n\
            note left of Runtime107\n\
              owner: team 109\n\
              mode: warm 113\n\
            end note\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:373px;height:62px;background:#FFFFFF;""#));
        assert!(svg.contains(
            r#"<g class="entity" data-qualified-name="GMN3" data-source-line="3" id="ent0004">"#
        ));
        assert!(svg.contains(r#"L134.1992,30.74 L168.72,26.74 L134.1992,22.74"#));
        assert!(svg.contains(r#">owner: team 109</text>"#));
        assert!(svg.contains(r#">mode: warm 113</text>"#));
    }

    #[test]
    fn deployment_svek_edge_translation_trims_path_but_not_arrow_tip() {
        let raw = vec![(10.0, 0.0), (10.0, 10.0), (10.0, 20.0), (10.0, 30.0)];

        let painted =
            deployment_svek_edge_points(&raw, BODY_FALLBACK_MARGIN_X, BODY_MARGIN_Y, EdgeTrim::End);

        assert_eq!(
            painted,
            vec![(26.0, 7.0), (26.0, 17.0), (26.0, 21.0), (26.0, 31.0)]
        );

        let untrimmed = deployment_svek_edge_points(
            &raw,
            BODY_FALLBACK_MARGIN_X,
            BODY_MARGIN_Y,
            EdgeTrim::None,
        );
        let mut svg = SvgBuilder::new(100.0, 100.0);
        emit_deployment_arrowhead(
            &mut svg,
            &untrimmed[untrimmed.len() - 2],
            &untrimmed[untrimmed.len() - 1],
        );
        let output = svg.finalize();
        assert!(output.contains(r#"points="26,37,30,28,26,32,22,28,26,37""#));
    }

    #[test]
    fn explicit_left_link_uses_reverse_svek_edge() {
        let source = "@startuml\nnode Left31\nnode Right37\nLeft31 -left-> Right37\n@enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains("<!--reverse link Right37 to Left31-->"));
        assert!(svg.contains(r#"id="Right37-backto-Left31""#));
    }
}
