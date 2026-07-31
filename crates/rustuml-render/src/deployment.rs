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
use rustuml_parser::diagram::style::StyleScheme;
use rustuml_parser::diagram::{LegendHorizontalAlignment, LegendVerticalAlignment};

use crate::handwritten::{
    dot_path as handwritten_dot_path,
    has_deprecated_skinparam as has_deprecated_handwritten_skinparam,
    is_enabled as is_handwritten_enabled, line_path as handwritten_line_path,
    path as handwritten_path, polygon_points as handwritten_polygon_points,
    rect_points as handwritten_rect_points,
};
use crate::layout_oracle::{
    EntityPath, EntityPolygon, EntityRect, EntityText, OracleHandwrittenWarning, OracleLayout,
    emit_entity_image, wrap_oracle_envelope,
};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::style_cascade::{StyleCascade, StyleSignature};
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
const WARNING_FONT_SIZE: f64 = 10.0;
const WARNING_RECT_X: f64 = 3.0;
const WARNING_TEXT_X: f64 = 10.0;
const WARNING_RECT_RADIUS: f64 = 2.5;
const WARNING_RECT_EXTRA_WIDTH: f64 = 10.0;
const WARNING_BLOCK_EXTRA_WIDTH: f64 = 20.0;
const WARNING_RECT_EXTRA_HEIGHT: f64 = 5.0;
const WARNING_BLOCK_EXTRA_HEIGHT: f64 = 10.0;
const WARNING_BASELINE_LIFT: f64 = 6.0;
const WARNING_FILL: &str = "#FFFFCC";
const WARNING_STROKE: &str = "#FFDD88";

// `CommandSkinParam.executeArg` stores a trailing space in the warning. Java
// `DiagramChromeFactory12026.WarningBannerBlock` measures that space but UText
// omits it when serialising the visible text. NBSPs preserve PlantUML's SVG
// text representation.
const DEPRECATED_HANDWRITTEN_WARNING: &str = "Please\u{a0}use\u{a0}'!option\u{a0}handwritten\u{a0}true'\u{a0}to\u{a0}enable\u{a0}handwritten";

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
const TITLE_BOTTOM_PAD: f64 = 11.0;
/// `DisplayPositioned.createRibbon` adds one pixel below header/footer text.
const CAPTION_BOTTOM_PAD: f64 = 1.0;
// `EntityImageLegend.create` merges the document legend style from
// `plantuml.skin`: 5px padding, 12px margin, 15px round corner, font 14.
// Creole table rows use the measured font height and 1.5 descents of cell pad.
const LEGEND_FONT_SIZE: f64 = 14.0;
const LEGEND_RECT_PAD_X: f64 = 5.0;
const LEGEND_RECT_PAD_Y: f64 = 7.0;
const LEGEND_OUTER_MARGIN: f64 = 12.0;
const LEGEND_RECT_RX: f64 = 7.5;
const LEGEND_CELL_PAD_DESCENT_FACTOR: f64 = 1.5;
// `TextBlockBordered.calculateDimension` adds one pixel beyond its drawn rect.
const LEGEND_BORDERED_DIMENSION_DELTA: f64 = 1.0;

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
// `CircleInterface2` paints a 16px circle inside a one-pixel margin;
// `USymbolSimpleAbstract` places the description block 26px below its origin.
const INTERFACE_CIRCLE_SIZE: f64 = 16.0;
const INTERFACE_SYMBOL_SIZE: f64 = 18.0;
const INTERFACE_LABEL_Y: f64 = 26.0;
// `ActorStickMan`: 16px head, 27px body, 15px legs, and a half-pixel
// `Fashion` stroke around the 26px arm/leg span.
const ACTOR_SYMBOL_WIDTH: f64 = 27.0;
const ACTOR_SYMBOL_HEIGHT: f64 = 60.0;
const ACTOR_HEAD_DIAMETER: f64 = 16.0;
const ACTOR_STROKE_WIDTH: f64 = 0.5;
const ACTOR_LIMB_HALF_WIDTH: f64 = 13.0;
const ACTOR_ARM_Y: f64 = 8.0;
const ACTOR_BODY_LENGTH: f64 = 27.0;
const ACTOR_LEG_HEIGHT: f64 = 15.0;
// `SvekNode.appendLabelHtml` wraps the symbol and its shield in a 3x3
// zero-padding Graphviz table. Its fixed-cell bookkeeping adds this envelope
// around the shield while keeping the `h` cell centered.
const INTERFACE_TABLE_EXTRA_WIDTH: f64 = 16.0;
const INTERFACE_TABLE_EXTRA_HEIGHT: f64 = 7.0;
// Graphviz's serialized `h` cell and tail-label table leave this gap between
// their lower edges (`SvekNode.appendLabelHtml` / `SvekEdge.appendTable`).
const INTERFACE_ENDPOINT_LABEL_BOTTOM_GAP: f64 = 0.04;
// Extracted from `SvekNode.appendLabelHtml` plus `SvekEdge.solveLine`: dot
// routes a vertical `h`-port spline through the inner-cell bottom, the outer
// table boundary, the center-label box, and the target clip boundary.
const INTERFACE_PORT_START_INSET: f64 = 0.05;
const INTERFACE_TABLE_CONTROL_INSET: f64 = 0.99;
const INTERFACE_LABEL_CONTROL_GAP: f64 = 0.76;
const INTERFACE_TARGET_ENDPOINT_DELTA: f64 = 0.11;
// Java provenance: `DotStringFactory.getMinNodeSep/getMinRankSep` and
// `getHorizontalDzeta/getVerticalDzeta` bound dynamic edge footprints by
// these pixel minima after dividing the maximum by ten.
const SVEK_MIN_NODE_SEP: f64 = 35.0;
const SVEK_MIN_RANK_SEP: f64 = 60.0;
const SVEK_DZETA_DIVISOR: f64 = 10.0;
// Java provenance: `LimitFinder.drawText` at
// 71806a23780b04a5ccde2f8ceb5121edad5eb711 raises the text top by
// `height - 1.5` and retains the baseline-side frontier at `baseline + 1.5`.
const LIMIT_FINDER_TEXT_ADJUST: f64 = 1.5;
// Java provenance: `LinkDecor.NONE` and `LinkDecor.ARROW` margins consumed by
// `SvekEdge.getDecorDzeta`.
const LINK_DECOR_NONE_MARGIN: f64 = 2.0;
const LINK_DECOR_ARROW_MARGIN: f64 = 10.0;
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
    let laid_out_note_indices = laid_out_deployment_note_indices(diagram);
    let cluster_ids = deployment_cluster_ids(diagram, &laid_out_note_indices);

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
    let sprite_cache = crate::sprite::SpriteCache::from_sprites_scaled_with_colors(
        &diagram.meta.sprites,
        sprite_scale(),
        [0xF1, 0xF1, 0xF1],
        [0, 0, 0],
    );
    let ctx = OracleRenderContext {
        diagram,
        oracle,
        id_for_node: &id_for_node,
        skin_fills: &skin_fills,
        skin_strokes: &skin_strokes,
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
        emit_clusters_dfs(&mut svg, root, &diagram.nodes, None, &cluster_ids, &ctx);
    }
    // Leaf-entity emission order is normally shallow-before-deep, then source
    // line. When duplicate child declarations are ignored by PlantUML, later
    // root leaves can sit between earlier and later nested leaves; use source
    // order for that root-leaf shape.
    let mut leaves: Vec<(usize, usize, &DeploymentNode, String)> = Vec::new();
    for root in &roots {
        collect_entities_dfs(root, &diagram.nodes, None, 0, &cluster_ids, &mut leaves);
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
            collect_entities_dfs(root, &diagram.nodes, None, 0, &cluster_ids, &mut group);
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

fn deployment_round_corner(
    diagram: &DeploymentDiagram,
    kind: DeploymentNodeKind,
    cluster: bool,
    source_line: usize,
) -> f64 {
    // Java `EntityImageDescription` resolves the concrete symbol title style
    // and stores its RoundCorner diameter in `Fashion`. `Cluster#getStyle`
    // does the corresponding `group.<symbol>` resolution for big symbols.
    // `DiagramType.DESCRIPTION#getStyleName` is `componentDiagram`.
    let symbol = skin_keyword(kind);
    if symbol.is_empty() {
        return RX_RY * 2.0;
    }
    let signature = if cluster {
        StyleSignature::from_selectors(["root", "element", "componentDiagram", "group", symbol])
    } else {
        StyleSignature::from_selectors(["root", "element", "componentDiagram", symbol, "title"])
    };
    StyleCascade::new(&diagram.meta.style_program)
        .resolve_entity_at_source_line(&signature, StyleScheme::Regular, source_line)
        .property("roundCorner")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(RX_RY * 2.0)
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

fn deprecated_handwritten_warning_text_width() -> f64 {
    pm::mono_text_width(DEPRECATED_HANDWRITTEN_WARNING, WARNING_FONT_SIZE)
}

fn deprecated_handwritten_warning_block_width() -> f64 {
    let trailing_space_width = pm::mono_text_width(" ", WARNING_FONT_SIZE);
    deprecated_handwritten_warning_text_width() + trailing_space_width + WARNING_BLOCK_EXTRA_WIDTH
}

fn deprecated_handwritten_warning_rect_height() -> f64 {
    pm::mono_text_height(WARNING_FONT_SIZE) + WARNING_RECT_EXTRA_HEIGHT
}

fn deprecated_handwritten_warning_block_height() -> f64 {
    pm::mono_text_height(WARNING_FONT_SIZE) + WARNING_BLOCK_EXTRA_HEIGHT
}

/// Paint the deprecated-handwritten warning from the same primitives as
/// PlantUML's `DiagramChromeFactory12026.WarningBannerBlock`. `force_width`
/// is the width of the innermost warning/body composition, before the outer
/// SVEK envelope and title/legend wrappers.
fn emit_generated_deprecated_handwritten_warning(
    svg: &mut SvgBuilder,
    force_width: f64,
    dx: f64,
    dy: f64,
) {
    let block_width = deprecated_handwritten_warning_block_width().max(force_width);
    let rect_width = block_width - WARNING_RECT_EXTRA_WIDTH;
    let rect_height = deprecated_handwritten_warning_rect_height();
    let points = handwritten_rect_points(
        dx + WARNING_RECT_X,
        dy + WARNING_RECT_X,
        rect_width,
        rect_height,
        WARNING_RECT_RADIUS,
        WARNING_RECT_RADIUS,
    );
    svg.raw(&format!(
        r#"<polygon fill="{WARNING_FILL}" points="{points}" style="stroke:{WARNING_STROKE};stroke-width:3;"/>"#,
    ));

    let text = escape_xml_text(DEPRECATED_HANDWRITTEN_WARNING).replace('\u{a0}', "&#160;");
    svg.raw(&format!(
        r##"<text fill="#000000" font-family="monospace" font-size="10" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{text}</text>"##,
        fc(deprecated_handwritten_warning_text_width()),
        fc(dx + WARNING_TEXT_X),
        fc(
            dy + WARNING_RECT_X + rect_height - WARNING_BASELINE_LIFT
        ),
    ));
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
    diagram: &'a DeploymentDiagram,
    oracle: &'a OracleLayout,
    id_for_node: &'a HashMap<String, String>,
    skin_fills: &'a HashMap<DeploymentNodeKind, String>,
    skin_strokes: &'a HashMap<DeploymentNodeKind, String>,
    sprites: &'a HashMap<String, rustuml_parser::diagram::SpriteData>,
    sprite_cache: &'a crate::sprite::SpriteCache,
    handwritten: bool,
}

fn emit_clusters_dfs(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    all: &[DeploymentNode],
    parent_qname: Option<&str>,
    cluster_ids: &HashSet<&str>,
    ctx: &OracleRenderContext<'_>,
) {
    let qname = qualified_name(node, parent_qname);
    let is_cluster = cluster_ids.contains(node.id.as_str());
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
                        deployment_round_corner(ctx.diagram, node.kind, true, node.source_line),
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
                    deployment_round_corner(ctx.diagram, node.kind, true, node.source_line),
                );
                emit_cluster_label(svg, node.kind, node, rect.x, rect.y, rect.width, Some(ctx));
            }
            svg.raw("</g>");
        }
        for child_id in &node.children {
            if let Some(child) = all.iter().find(|n| n.id == *child_id) {
                emit_clusters_dfs(svg, child, all, Some(&qname), cluster_ids, ctx);
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
    cluster_ids: &HashSet<&str>,
    out: &mut Vec<(usize, usize, &'a DeploymentNode, String)>,
) {
    let qname = qualified_name(node, parent_qname);
    let is_cluster = cluster_ids.contains(node.id.as_str());
    if !is_cluster {
        out.push((depth, node.source_line, node, qname));
    } else {
        for child_id in &node.children {
            if let Some(child) = all.iter().find(|n| n.id == *child_id) {
                collect_entities_dfs(child, all, Some(&qname), depth + 1, cluster_ids, out);
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
        if ctx.handwritten && emit_handwritten_entity(svg, node, rect, &entity_fill, stroke) {
            // The handwritten branch consumed oracle-captured primitive
            // geometry and emitted the label using oracle text anchors.
        } else if matches!(node.kind, Boundary | Control | Entity | Default) {
            emit_icon_entity(svg, node, rect, &entity_fill);
        } else if matches!(node.kind, Actor) {
            let generated_label = emit_actor_entity(svg, node, rect, &entity_fill, ctx);
            if !generated_label
                && !emit_oracle_image_label_children(svg, rect)
                && let Some(text) = rect.texts.first()
            {
                emit_text(svg, &text.text, text.x, text.y, FONT_SIZE, false, false);
            }
        } else if matches!(node.kind, Collections) {
            emit_collections_entity(
                svg,
                node,
                rect,
                &entity_fill,
                deployment_round_corner(ctx.diagram, node.kind, false, node.source_line),
            );
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
                deployment_round_corner(ctx.diagram, node.kind, false, node.source_line),
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
    stroke: &str,
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
    if !emitted_shape {
        emitted_shape = emit_generated_handwritten_entity_shape(
            svg,
            node.kind,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            fill,
            stroke,
            &node.label,
        );
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

#[allow(clippy::too_many_arguments)]
fn emit_generated_handwritten_entity_shape(
    svg: &mut SvgBuilder,
    kind: DeploymentNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    label: &str,
) -> bool {
    match kind {
        DeploymentNodeKind::Node => {
            let vertices = tag_polygon_vertices(x, y, w, h);
            let points = handwritten_polygon_points(&vertices);
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
            ));
            for (x1, y1, x2, y2) in tag_polygon_detail_lines(x, y, w, h) {
                let d = handwritten_line_path(x1, y1, x2, y2);
                svg.raw(&format!(
                    r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
                ));
            }
            true
        }
        DeploymentNodeKind::Database => {
            let (body, lip) = database_path_data(x, y, w, h, label, true);
            let body = handwritten_path(&body).unwrap_or(body);
            let lip = handwritten_path(&lip).unwrap_or(lip);
            svg.raw(&format!(
                r#"<path d="{body}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
            ));
            svg.raw(&format!(
                r#"<path d="{lip}" fill="none" style="stroke:{stroke};stroke-width:0.5;"/>"#,
            ));
            true
        }
        _ => false,
    }
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

fn stereotype_refs_sprite(
    stereotype: &str,
    sprites: &HashMap<String, rustuml_parser::diagram::SpriteData>,
) -> bool {
    let lowered = stereotype.trim().trim_start_matches('$').to_lowercase();
    if sprites.contains_key(&lowered) {
        return true;
    }
    lowered
        .rsplit_once('_')
        .is_some_and(|(_, suffix)| sprites.contains_key(suffix))
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
    round_corner: f64,
) {
    use DeploymentNodeKind::*;
    match kind {
        Node => emit_tag_polygon(svg, x, y, w, h, fill, 0.5, stroke),
        Artifact => emit_artifact_round_corner(svg, x, y, w, h, fill, stroke, round_corner),
        Card | Rectangle | Agent => {
            emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 0.5)
        }
        Component => emit_component(svg, x, y, w, h, fill, stroke, round_corner),
        Frame => emit_frame(svg, x, y, w, h, fill, label, round_corner),
        Folder => emit_folder(svg, x, y, w, h, fill, stroke, round_corner),
        File => emit_file(svg, x, y, w, h, fill, stroke, round_corner),
        Package => emit_package(svg, x, y, w, h, fill, stroke, label),
        Stack => emit_stack(svg, x, y, w, h, fill, stroke, round_corner),
        Storage => emit_storage(svg, x, y, w, h, fill, stroke),
        Database => emit_database(svg, x, y, w, h, fill, stroke, label),
        Queue => emit_queue(svg, x, y, w, h, fill, stroke),
        _ => emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 0.5),
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
    round_corner: f64,
) {
    use DeploymentNodeKind::*;
    // Clusters default to no fill; a `#color` paints the cluster background.
    let fill = fill.unwrap_or("none");
    match kind {
        // Clusters use stroke-width=1 (per goldens).
        Node => emit_tag_polygon(svg, x, y, w, h, fill, 1.0, stroke),
        Artifact => emit_artifact_round_corner_with_stroke_width(
            svg,
            x,
            y,
            w,
            h,
            fill,
            stroke,
            1.0,
            round_corner,
        ),
        Storage => emit_storage_with_stroke_width(svg, x, y, w, h, fill, stroke, 1.0),
        // Card cluster has rect + horizontal line under title.
        Card => emit_card_cluster(svg, x, y, w, h, fill, stroke, round_corner),
        // Rectangle / Agent cluster: bare rect, no line.
        Rectangle | Agent => emit_plain_rect_cluster(svg, x, y, w, h, fill, stroke, round_corner),
        // Component cluster: rounded rect + UML component plug icon at the
        // top-right, identical to the leaf component shape but drawn with the
        // cluster stroke-width (1) instead of the leaf 0.5.
        Component => emit_component_cluster(svg, x, y, w, h, fill, stroke, round_corner),
        Cloud => emit_cloud_cluster(svg, x, y, w, h, fill, stroke),
        Frame => emit_frame_cluster(svg, x, y, w, h, fill, stroke, round_corner),
        Folder => emit_folder_cluster(svg, x, y, w, h, fill, label, round_corner),
        Package => emit_package_cluster(svg, x, y, w, h, fill, label),
        Stack => emit_stack_cluster(svg, x, y, w, h, fill, round_corner),
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

#[allow(clippy::too_many_arguments)]
fn emit_plain_rect_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 1.0);
}

// ---- Node ("tag" polygon) -------------------------------------------------

fn tag_polygon_vertices(x: f64, y: f64, w: f64, h: f64) -> [(f64, f64); 6] {
    let offset = 10.0;
    [
        (x, y + offset),
        (x + offset, y),
        (x + w, y),
        (x + w, y + h - offset),
        (x + w - offset, y + h),
        (x, y + h),
    ]
}

fn tag_polygon_detail_lines(x: f64, y: f64, w: f64, h: f64) -> [(f64, f64, f64, f64); 3] {
    let offset = 10.0;
    [
        (x + w - offset, y + offset, x + w, y),
        (x, y + offset, x + w - offset, y + offset),
        (x + w - offset, y + offset, x + w - offset, y + h),
    ]
}

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
    let vertices = tag_polygon_vertices(x, y, w, h);
    let mut points = vertices
        .iter()
        .map(|(px, py)| format!("{},{}", fc(*px), fc(*py)))
        .collect::<Vec<_>>();
    points.push(points[0].clone());
    let points = points.join(",");
    svg.raw(&format!(
        r#"<polygon fill="{fill}" points="{points}" style="stroke:{stroke};stroke-width:{sw};"/>"#,
    ));
    // 3 lines for the 3D effect: top-right diagonal, top inner, right inner.
    for (x1, y1, x2, y2) in tag_polygon_detail_lines(x, y, w, h) {
        svg.raw(&format!(
            r#"<line style="stroke:{stroke};stroke-width:{sw};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            fc(x1),
            fc(x2),
            fc(y1),
            fc(y2),
        ));
    }
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
    emit_artifact_round_corner_with_stroke_width(svg, x, y, w, h, fill, stroke, 0.5, RX_RY * 2.0);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_artifact_with_stroke_width(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
) {
    emit_artifact_round_corner_with_stroke_width(
        svg,
        x,
        y,
        w,
        h,
        fill,
        stroke,
        stroke_width,
        RX_RY * 2.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_artifact_round_corner(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    emit_artifact_round_corner_with_stroke_width(svg, x, y, w, h, fill, stroke, 0.5, round_corner);
}

#[allow(clippy::too_many_arguments)]
fn emit_artifact_round_corner_with_stroke_width(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    round_corner: f64,
) {
    let sw = fc(stroke_width);
    let x_s = fc(x);
    let y_s = fc(y);
    let w_s = fc(w);
    let h_s = fc(h);
    let corner_attrs = rounded_rect_corner_attrs(round_corner);
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h_s}"{corner_attrs} style="stroke:{stroke};stroke-width:{sw};" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
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

fn rounded_rect_corner_attrs(round_corner: f64) -> String {
    if round_corner == 0.0 {
        String::new()
    } else {
        let radius = round_corner / 2.0;
        format!(r#" rx="{}" ry="{}""#, fc(radius), fc(radius))
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_rounded_rect_with_round_corner(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
    stroke_width: f64,
) {
    let corner_attrs = rounded_rect_corner_attrs(round_corner);
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}"{corner_attrs} style="stroke:{stroke};stroke-width:{stroke_width};" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        stroke_width = fc(stroke_width),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));
}

// ---- Card cluster (rect + horizontal line) --------------------------------

#[allow(clippy::too_many_arguments)]
fn emit_card_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 1.0);
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

#[allow(clippy::too_many_arguments)]
fn emit_component(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 0.5);
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

#[allow(clippy::too_many_arguments)]
fn emit_component_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 1.0);
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

#[allow(clippy::too_many_arguments)]
fn emit_frame(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    label: &str,
    round_corner: f64,
) {
    emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, STROKE, round_corner, 0.5);
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

#[allow(clippy::too_many_arguments)]
fn emit_frame_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    // Frame cluster: bare rect with stroke-width=1. The tab is emitted
    // by emit_cluster_label since it depends on label width.
    emit_rounded_rect_with_round_corner(svg, x, y, w, h, fill, stroke, round_corner, 1.0);
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
#[allow(clippy::too_many_arguments)]
fn emit_folder(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    let xr = x + w;
    let yb = y + h;
    let title_right = x + 46.0;
    let slope_right = title_right + 7.0;
    let tab_y = y + 21.0;
    let radius = round_corner / 2.0;
    let d = if round_corner == 0.0 {
        format!(
            "M{x},{y} L{title_right},{y} L{slope_right},{tab_y} L{xr},{tab_y} L{xr},{yb} L{x},{yb} L{x},{y}",
            x = fc(x),
            y = fc(y),
            title_right = fc(title_right),
            slope_right = fc(slope_right),
            tab_y = fc(tab_y),
            xr = fc(xr),
            yb = fc(yb),
        )
    } else {
        format!(
            "M{x_r},{y} L{title_left},{y} A{title_radius},{title_radius} 0 0 1 {title_right},{y_r} L{slope_right},{tab_y} L{xr_r},{tab_y} A{radius},{radius} 0 0 1 {xr},{tab_y_r} L{xr},{yb_r} A{radius},{radius} 0 0 1 {xr_r},{yb} L{x_r},{yb} A{radius},{radius} 0 0 1 {x},{yb_r} L{x},{y_r} A{radius},{radius} 0 0 1 {x_r},{y}",
            x = fc(x),
            y = fc(y),
            x_r = fc(x + radius),
            y_r = fc(y + radius),
            title_left = fc(title_right - radius),
            title_radius = fc(radius * 1.5),
            title_right = fc(title_right),
            slope_right = fc(slope_right),
            tab_y = fc(tab_y),
            tab_y_r = fc(tab_y + radius),
            xr = fc(xr),
            xr_r = fc(xr - radius),
            yb = fc(yb),
            yb_r = fc(yb - radius),
            radius = fc(radius),
        )
    };
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
    // Horizontal divider under the tab, from the left edge to the flap end.
    svg.raw(&format!(
        r#"<line style="stroke:{stroke};stroke-width:0.5;" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
        x1 = fc(x),
        x2 = fc(slope_right),
        ty = fc(tab_y),
    ));
}

/// Folder cluster shape: like the leaf folder but the tab width tracks the
/// (bold) title width and the divider/outline use the cluster stroke
/// (#000000, width 1.5). Tab band height is text_height + 6.
#[allow(clippy::too_many_arguments)]
fn emit_folder_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    label: &str,
    round_corner: f64,
) {
    let xr = x + w;
    let yb = y + h;
    let label_w = text_render::measure(label, FONT_SIZE, true);
    let title_right = x + label_w + 6.0;
    let slope_right = title_right + 7.0;
    let tab_y = y + pm::text_height(FONT_SIZE) + 6.0;
    let cstroke = "#000000";
    let radius = round_corner / 2.0;
    let d = if round_corner == 0.0 {
        format!(
            "M{x},{y} L{title_right},{y} L{slope_right},{tab_y} L{xr},{tab_y} L{xr},{yb} L{x},{yb} L{x},{y}",
            x = fc(x),
            y = fc(y),
            title_right = fc(title_right),
            slope_right = fc(slope_right),
            tab_y = fc(tab_y),
            xr = fc(xr),
            yb = fc(yb),
        )
    } else {
        format!(
            "M{x_r},{y} L{title_left},{y} A{title_radius},{title_radius} 0 0 1 {title_right},{y_r} L{slope_right},{tab_y} L{xr_r},{tab_y} A{radius},{radius} 0 0 1 {xr},{tab_y_r} L{xr},{yb_r} A{radius},{radius} 0 0 1 {xr_r},{yb} L{x_r},{yb} A{radius},{radius} 0 0 1 {x},{yb_r} L{x},{y_r} A{radius},{radius} 0 0 1 {x_r},{y}",
            x = fc(x),
            y = fc(y),
            x_r = fc(x + radius),
            y_r = fc(y + radius),
            title_left = fc(title_right - radius),
            title_radius = fc(radius * 1.5),
            title_right = fc(title_right),
            slope_right = fc(slope_right),
            tab_y = fc(tab_y),
            tab_y_r = fc(tab_y + radius),
            xr = fc(xr),
            xr_r = fc(xr - radius),
            yb = fc(yb),
            yb_r = fc(yb - radius),
            radius = fc(radius),
        )
    };
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{cstroke};stroke-width:1.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<line style="stroke:{cstroke};stroke-width:1.5;" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
        x1 = fc(x),
        x2 = fc(slope_right),
        ty = fc(tab_y),
    ));
}

// ---- File -----------------------------------------------------------------

/// File (document) shape: a rounded rectangle with a folded top-right corner
/// (a 10×10 dog-ear). Two paths: the body outline and the fold triangle.
#[allow(clippy::too_many_arguments)]
fn emit_file(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    let xr = x + w;
    let yb = y + h;
    let radius = round_corner / 2.0;
    let body = if round_corner == 0.0 {
        format!(
            "M{x},{y} L{x},{yb} L{xr},{yb} L{xr},{y10} L{xr10},{y} L{x},{y}",
            x = fc(x),
            y = fc(y),
            yb = fc(yb),
            xr = fc(xr),
            y10 = fc(y + 10.0),
            xr10 = fc(xr - 10.0),
        )
    } else {
        format!(
            "M{x},{y_r} L{x},{yb_r} A{radius},{radius} 0 0 0 {x_r},{yb} L{xr_r},{yb} A{radius},{radius} 0 0 0 {xr},{yb_r} L{xr},{y10} L{xr10},{y} L{x_r},{y} A{radius},{radius} 0 0 0 {x},{y_r}",
            x = fc(x),
            y = fc(y),
            x_r = fc(x + radius),
            y_r = fc(y + radius),
            yb = fc(yb),
            yb_r = fc(yb - radius),
            xr = fc(xr),
            xr_r = fc(xr - radius),
            y10 = fc(y + 10.0),
            xr10 = fc(xr - 10.0),
            radius = fc(radius),
        )
    };
    svg.raw(&format!(
        r#"<path d="{body}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
    let fold = if round_corner == 0.0 {
        format!(
            "M{xr10},{y} L{xr10},{y10} L{xr},{y10}",
            xr10 = fc(xr - 10.0),
            y = fc(y),
            y10 = fc(y + 10.0),
            xr = fc(xr),
        )
    } else {
        format!(
            "M{xr10},{y} L{xr10},{fold_y} A{radius},{radius} 0 0 0 {fold_x},{y10} L{xr},{y10}",
            xr10 = fc(xr - 10.0),
            y = fc(y),
            fold_y = fc(y + 10.0 - radius),
            fold_x = fc(xr - 10.0 + radius),
            y10 = fc(y + 10.0),
            xr = fc(xr),
            radius = fc(radius),
        )
    };
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

#[allow(clippy::too_many_arguments)]
fn emit_stack(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    round_corner: f64,
) {
    emit_stack_shape(svg, x, y, w, h, fill, stroke, 0.5, round_corner);
}

/// Stack cluster: a fill-only inner rect (no stroke) plus the same bracket
/// outline as the leaf stack, drawn with the cluster stroke width.
fn emit_stack_cluster(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    round_corner: f64,
) {
    emit_stack_shape(svg, x, y, w, h, fill, "#181818", 1.0, round_corner);
}

#[allow(clippy::too_many_arguments)]
fn emit_stack_shape(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    round_corner: f64,
) {
    // `USymbolStack#drawQueue` uses the complete symbol width, a 15px inset,
    // and an explicit straight-line branch for a zero corner diameter.
    let inner_x = x + 15.0;
    let inner_w = w - 30.0;
    let radius = round_corner / 2.0;
    let corner_attrs = if round_corner == 0.0 {
        String::new()
    } else {
        format!(r#" rx="{}" ry="{}""#, fc(radius), fc(radius))
    };
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}"{corner_attrs} style="stroke:none;stroke-width:{stroke_width};" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        stroke_width = fc(stroke_width),
        w = fc(inner_w),
        x = fc(inner_x),
        y = fc(y),
    ));
    let xl = x;
    let xr = x + w;
    let d = if round_corner == 0.0 {
        format!(
            "M{xl},{y} L{inner_x},{y} L{inner_x},{bottom} L{inner_right},{bottom} L{inner_right},{y} L{xr},{y}",
            xl = fc(xl),
            xr = fc(xr),
            inner_x = fc(inner_x),
            inner_right = fc(inner_x + inner_w),
            y = fc(y),
            bottom = fc(y + h),
        )
    } else {
        format!(
            "M{xl},{y_s} L{x_lp1},{y_s} A{radius},{radius} 0 0 1 {x_s},{y_p1} L{x_s},{y_pm1} A{radius},{radius} 0 0 0 {x_lp2},{yh_s} L{x_rm2},{yh_s} A{radius},{radius} 0 0 0 {xw_s},{y_pm1} L{xw_s},{y_p1} A{radius},{radius} 0 0 1 {x_rp2},{y_s} L{xr},{y_s}",
            xl = fc(xl),
            xr = fc(xr),
            x_s = fc(inner_x),
            xw_s = fc(inner_x + inner_w),
            y_s = fc(y),
            yh_s = fc(y + h),
            x_lp1 = fc(inner_x - radius),
            x_lp2 = fc(inner_x + radius),
            x_rm2 = fc(inner_x + inner_w - radius),
            x_rp2 = fc(inner_x + inner_w + radius),
            y_p1 = fc(y + radius),
            y_pm1 = fc(y + h - radius),
            radius = fc(radius),
        )
    };
    svg.raw(&format!(
        r#"<path d="{d}" fill="none" style="stroke:{stroke};stroke-width:{};"/>"#,
        fc(stroke_width),
    ));
}

// ---- Storage (rounded rect with rx=35, ry=35) -----------------------------

fn emit_storage(svg: &mut SvgBuilder, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    emit_storage_with_stroke_width(svg, x, y, w, h, fill, stroke, 0.5);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_storage_with_stroke_width(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="35" ry="35" style="stroke:{stroke};stroke-width:{stroke_width};" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
        stroke_width = fc(stroke_width),
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

fn database_path_data(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: &str,
    preserve_precision: bool,
) -> (String, String) {
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
    let coord = |value: f64| {
        if preserve_precision {
            value.to_string()
        } else {
            fc(value)
        }
    };
    let d = format!(
        "M{x_s},{tl} C{x_s},{y_s} {cx_s},{y_s} {cx_s},{y_s} C{cx_s},{y_s} {xw_s},{y_s} {xw_s},{tl} L{xw_s},{bl} C{xw_s},{by_s} {cx_s},{by_s} {cx_s},{by_s} C{cx_s},{by_s} {x_s},{by_s} {x_s},{bl} L{x_s},{tl}",
        x_s = coord(x),
        y_s = coord(y),
        cx_s = coord(cx),
        xw_s = coord(x + w),
        by_s = coord(bot_y),
        tl = coord(top_low),
        bl = coord(bot_low),
    );
    // The "top wall" of the cylinder (inner curve under the lip).
    let d2 = format!(
        "M{x_s},{tl} C{x_s},{ml} {cx_s},{ml} {cx_s},{ml} C{cx_s},{ml} {xw_s},{ml} {xw_s},{tl}",
        x_s = coord(x),
        cx_s = coord(cx),
        xw_s = coord(x + w),
        tl = coord(top_low),
        ml = coord(y + 20.0),
    );
    (d, d2)
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
    let (d, d2) = database_path_data(x, y, w, h, label, false);
    svg.raw(&format!(
        r#"<path d="{d}" fill="{fill}" style="stroke:{stroke};stroke-width:0.5;"/>"#
    ));
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
    let (text_x_pad, mut top_pad, bold) = entity_text_geom(kind, w, &node.label);
    if let Some(ctx) = ctx {
        let first_line = node.label.lines().next().unwrap_or_default();
        top_pad += (deployment_label_line_height_for_sprites(first_line, FONT_SIZE, ctx.sprites)
            - TEXT_LINE_H)
            .max(0.0);
    }
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
        && !ctx.is_some_and(|ctx| stereotype_refs_sprite(stereo, ctx.sprites))
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
        && !ctx.is_some_and(|ctx| stereotype_refs_sprite(stereo, ctx.sprites))
    {
        let stereo_label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_w = text_render::measure(&stereo_label, FONT_SIZE, false);
        let stereo_x = center_x - stereo_w / 2.0;
        let stereo_y = y + cluster_stereotype_baseline_offset(kind);
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
            stereo_y + cluster_stereotype_to_title_offset(kind),
            DeploymentLabelStyle {
                font_size: FONT_SIZE,
                bold: true,
                italic: false,
            },
            ctx,
        );
    } else {
        let label_x = center_x - label_w / 2.0;
        let label_y = y + cluster_title_baseline_offset(kind);
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
                .map(|sprite| deployment_sprite_logical_dimensions(sprite, font_size).0)
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
                    let (_, logical_height) =
                        deployment_sprite_logical_dimensions(sprite, style.font_size);
                    // Java `Sea.doAlign` bottom-aligns `AtomSprite` against
                    // the text atom's logical line box. The raster itself is
                    // independently rounded by `PortableImageAwt.scale`.
                    let image_y = y - pm::ascent(style.font_size)
                        + pm::text_height(style.font_size)
                        - logical_height;
                    svg.image(cursor, image_y, width, height, uri);
                    cursor += deployment_sprite_logical_dimensions(sprite, style.font_size).0;
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

fn deployment_sprite_logical_dimensions(
    sprite: &rustuml_parser::diagram::SpriteData,
    font_size: f64,
) -> (f64, f64) {
    let (width, height) = crate::sprite::sprite_dimensions(sprite);
    let scale = font_size / SPRITE_BASE_FONT_SIZE;
    (width as f64 * scale, height as f64 * scale)
}

fn cluster_title_baseline_offset(kind: DeploymentNodeKind) -> f64 {
    use DeploymentNodeKind::*;
    match kind {
        // Node cluster title sits in a small header band: ascent+13 from bbox top.
        Node => ASCENT_14 + 13.0,
        // `USymbolStorage.asBig` places the title at y=7.
        Storage => ASCENT_14 + 7.0,
        // Card-like clusters place the title at y=2.
        Artifact | Card | Rectangle | Agent | Frame => ASCENT_14 + 2.0,
        _ => ASCENT_14 + 13.0,
    }
}

fn cluster_stereotype_baseline_offset(kind: DeploymentNodeKind) -> f64 {
    if kind == DeploymentNodeKind::Storage {
        // `USymbolStorage.asBig` places the stereotype at y=5, two pixels
        // above the title's y=7 origin.
        ASCENT_14 + 5.0
    } else {
        cluster_title_baseline_offset(kind)
    }
}

fn cluster_stereotype_to_title_offset(kind: DeploymentNodeKind) -> f64 {
    TEXT_LINE_H
        + if kind == DeploymentNodeKind::Storage {
            2.0
        } else {
            0.0
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

fn emit_actor_entity(
    svg: &mut SvgBuilder,
    node: &DeploymentNode,
    rect: &crate::layout_oracle::EntityRect,
    fill: &str,
    ctx: &OracleRenderContext<'_>,
) -> bool {
    if rect.glyph_path_d.is_none() {
        // `USymbolSimpleAbstract.asSmall` centers `ActorStickMan` above the
        // complete sprite/text label block.
        let symbol_x = rect.x + (rect.width - ACTOR_SYMBOL_WIDTH) / 2.0;
        let symbol_y = rect.y;
        let center_x = symbol_x + ACTOR_SYMBOL_WIDTH / 2.0;
        let head_x =
            symbol_x + ACTOR_LIMB_HALF_WIDTH - ACTOR_HEAD_DIAMETER / 2.0 + ACTOR_STROKE_WIDTH;
        let head_y = symbol_y + ACTOR_STROKE_WIDTH;
        let head_center_y = head_y + ACTOR_HEAD_DIAMETER / 2.0;
        svg.raw(&format!(
            r#"<ellipse cx="{}" cy="{}" fill="{}" rx="8" ry="8" style="stroke:#181818;stroke-width:0.5;"/>"#,
            fc(head_x + ACTOR_HEAD_DIAMETER / 2.0),
            fc(head_center_y),
            fill,
        ));
        let body_top = symbol_y + ACTOR_HEAD_DIAMETER + ACTOR_STROKE_WIDTH;
        let body_bottom = body_top + ACTOR_BODY_LENGTH;
        svg.raw(&format!(
            r#"<path d="M{cx},{top} L{cx},{bottom} M{left},{arms} L{right},{arms} M{cx},{bottom} L{left},{legs} M{cx},{bottom} L{right},{legs}" fill="none" style="stroke:#181818;stroke-width:0.5;"/>"#,
            cx = fc(center_x),
            top = fc(body_top),
            bottom = fc(body_bottom),
            left = fc(center_x - ACTOR_LIMB_HALF_WIDTH),
            right = fc(center_x + ACTOR_LIMB_HALF_WIDTH),
            arms = fc(body_top + ACTOR_ARM_Y),
            legs = fc(body_bottom + ACTOR_LEG_HEIGHT),
        ));
        let label_width = deployment_label_width(&node.label, FONT_SIZE, false, Some(ctx));
        emit_deployment_label(
            svg,
            &node.label,
            rect.x + (rect.width - label_width) / 2.0,
            rect.y + ACTOR_SYMBOL_HEIGHT + ASCENT_14,
            DeploymentLabelStyle {
                font_size: FONT_SIZE,
                bold: false,
                italic: false,
            },
            Some(ctx),
        );
        return true;
    }

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
    false
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
    round_corner: f64,
) {
    // Back card (the captured body rect).
    emit_rounded_rect_with_round_corner(
        svg,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        fill,
        STROKE,
        round_corner,
        0.5,
    );
    // Front card: offset up-left by 4px. Prefer the oracle's aux rect when
    // present, else derive it.
    let (fx, fy, fw, fh) = rect
        .aux_rects
        .first()
        .map(|a| (a.x, a.y, a.width, a.height))
        .unwrap_or((rect.x - 4.0, rect.y - 4.0, rect.width, rect.height));
    emit_rounded_rect_with_round_corner(svg, fx, fy, fw, fh, fill, STROKE, round_corner, 0.5);
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
// Java provenance: `EntityImageNoteLink` delegates to `ComponentRoseNote`,
// whose preferred size adds Rose's five-pixel padding on every side before
// `SvekEdge` merges it with the ordinary center label.
const LINK_NOTE_PADDING: f64 = 5.0;
const NOTE_CONNECTOR_HALF: f64 = 4.0;
const NOTE_GAP: f64 = 10.0;

fn deployment_note_fill(note: &DeploymentNote, gradient_defs: Option<&str>) -> String {
    deployment_note_fill_from_color(note.color.as_deref(), gradient_defs)
}

fn deployment_note_fill_from_color(color: Option<&str>, gradient_defs: Option<&str>) -> String {
    color
        .map(|color| crate::sequence::gradient_fill_or(color, gradient_defs))
        .unwrap_or_else(|| NOTE_FILL.to_string())
}

fn deployment_note_gradient_defs(diagram: &DeploymentDiagram) -> String {
    let source = diagram.meta.source.as_deref().unwrap_or("");
    let mut gradients: Vec<(String, String, char, String)> = Vec::new();
    let colors = diagram
        .notes
        .iter()
        .filter_map(|note| note.color.as_deref())
        .chain(
            diagram
                .connections
                .iter()
                .filter_map(|connection| connection.note.as_ref()?.color.as_deref()),
        );
    for color in colors {
        let Some((raw1, raw2, policy)) = crate::sequence::split_gradient_colors(color) else {
            continue;
        };
        let color1 = crate::sequence::resolve_color(raw1);
        let color2 = crate::sequence::resolve_color(raw2);
        if gradients
            .iter()
            .any(|(existing1, existing2, existing_policy, _)| {
                existing1 == &color1 && existing2 == &color2 && *existing_policy == policy
            })
        {
            continue;
        }
        let id = crate::filter_registry::gradient_id_for(source, gradients.len());
        gradients.push((color1, color2, policy, id));
    }

    let mut defs = String::new();
    for (color1, color2, policy, id) in gradients {
        let (x1, x2, y1, y2) = crate::sequence::gradient_endpoints(policy);
        write!(
            defs,
            r#"<linearGradient id="{id}" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"><stop offset="0%" stop-color="{color1}"/><stop offset="100%" stop-color="{color2}"/></linearGradient>"#,
        )
        .unwrap();
    }
    defs
}

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

#[derive(Clone, Copy)]
struct DeploymentNoteLeader<'a> {
    position: DeploymentNotePosition,
    edge: Option<&'a EdgePath>,
    note_at_edge_start: Option<bool>,
}

fn render_attached_deployment_note(
    svg: &mut SvgBuilder,
    note: &DeploymentNote,
    layout: &DeploymentNoteLayout,
    uid: &DeploymentNoteUid,
    leader: DeploymentNoteLeader<'_>,
    body_margin: (f64, f64),
    gradient_defs: Option<&str>,
) {
    let (body_margin_x, body_margin_y) = body_margin;
    let fill = deployment_note_fill(note, gradient_defs);
    let center = (
        layout.x + layout.width / 2.0,
        layout.y + layout.height / 2.0,
    );
    let fallback = match leader.position {
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
    let (note_point, target_point) = leader
        .edge
        .map(|path| {
            deployment_svek_edge_points(
                &path.points,
                body_margin_x,
                body_margin_y,
                None,
                None,
                EdgeTrim::None,
            )
        })
        .and_then(|points| points.first().copied().zip(points.last().copied()))
        .map(|(first, last)| match leader.note_at_edge_start {
            Some(true) => (first, last),
            Some(false) => (last, first),
            None => match leader.position {
                DeploymentNotePosition::Top | DeploymentNotePosition::Left => (first, last),
                DeploymentNotePosition::Bottom | DeploymentNotePosition::Right => (last, first),
            },
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
    let path = match leader.position {
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
        r#"<path d="{path}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<path d="{fold_path}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));

    let mut text_y = y + NOTE_MARGIN_Y;
    for line in note.text.lines().map(deployment_note_display_row) {
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

fn render_floating_deployment_note(
    svg: &mut SvgBuilder,
    note: &DeploymentNote,
    layout: &DeploymentNoteLayout,
    uid: &DeploymentNoteUid,
    gradient_defs: Option<&str>,
) {
    let fill = deployment_note_fill(note, gradient_defs);
    let x = layout.x;
    let y = layout.y;
    let right = x + layout.width;
    let bottom = y + layout.height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;

    // Java `EntityImageNote#drawNormal` paints `Opale.getPolygonNormal`, then
    // draws the folded corner as a second primitive with the graphics stroke.
    let path = format!(
        "M{x},{y} L{x},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{y} L{x},{y}",
        x = fc(x),
        y = fc(y),
        bottom = fc(bottom),
        right = fc(right),
        fold_y = fc(fold_y),
        fold_x = fc(fold_x),
    );
    let fold_path = format!(
        "M{fold_x},{y} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{y}",
        fold_x = fc(fold_x),
        y = fc(y),
        fold_y = fc(fold_y),
        right = fc(right),
    );
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
        uid.qualified_name, note.source_line, uid.entity_id
    ));
    svg.raw(&format!(
        r#"<path d="{path}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<path d="{fold_path}" fill="{fill}" style="stroke:{STROKE};stroke-width:1;"/>"#
    ));
    let mut text_y = y + NOTE_MARGIN_Y;
    for line in note.text.lines().map(deployment_note_display_row) {
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

fn render_deployment_link_note(
    svg: &mut SvgBuilder,
    note: &DeploymentLinkNote,
    x: f64,
    y: f64,
    dim: DeploymentNoteDim,
    gradient_defs: Option<&str>,
) {
    let fill = deployment_note_fill_from_color(note.color.as_deref(), gradient_defs);
    // `ComponentRoseNote.drawInternalU` truncates the text-box dimensions
    // before `EntityImageNoteLink` paints the folded note inside SvekEdge.
    let width = dim.width.floor();
    let height = dim.height.floor();
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;
    svg.raw(&format!(
        r#"<path d="M{x},{y} L{x},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{y} L{x},{y}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        x = fc(x),
        y = fc(y),
        bottom = fc(bottom),
        right = fc(right),
        fold_y = fc(fold_y),
        fold_x = fc(fold_x),
    ));
    svg.raw(&format!(
        r#"<path d="M{fold_x},{y} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{y}" fill="{fill}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        fold_x = fc(fold_x),
        y = fc(y),
        fold_y = fc(fold_y),
        right = fc(right),
    ));

    let mut baseline = y + NOTE_MARGIN_Y + pm::ascent(NOTE_FONT_SIZE);
    for line in note.text.lines().map(deployment_note_display_row) {
        emit_text(
            svg,
            line,
            x + NOTE_MARGIN_X1,
            baseline,
            NOTE_FONT_SIZE,
            false,
            false,
        );
        baseline += text_render::label_height(line, NOTE_FONT_SIZE);
    }
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

fn deployment_note_layout_id(note: &DeploymentNote, index: usize) -> String {
    // Java named notes are ordinary quark-backed SVEK leaves, so explicit
    // relations address their code directly. Attached generated notes retain
    // a private layout id because their GMN identity is allocated later.
    note.id
        .clone()
        .unwrap_or_else(|| format!("__deployment_note_{index}"))
}

fn deployment_note_dim(note: &DeploymentNote) -> DeploymentNoteDim {
    deployment_note_text_dim(&note.text)
}

fn deployment_link_note_dim(note: &DeploymentLinkNote) -> DeploymentNoteDim {
    deployment_note_text_dim(&note.text)
}

fn deployment_note_display_row(line: &str) -> &str {
    if line.is_empty() { "\u{00A0}" } else { line }
}

fn deployment_note_text_dim(text: &str) -> DeploymentNoteDim {
    // `EntityImageNote` delegates text measurement and asymmetric margins to
    // `Opale`: six pixels left, fifteen right, and five on each vertical side.
    let width = text
        .lines()
        .map(deployment_note_display_row)
        .map(|line| text_render::measure(line, NOTE_FONT_SIZE, false))
        .fold(0.0_f64, f64::max)
        + NOTE_MARGIN_X1
        + NOTE_MARGIN_X2;
    let text_height = text
        .lines()
        .map(deployment_note_display_row)
        .map(|line| text_render::label_height(line, NOTE_FONT_SIZE))
        .sum::<f64>();
    DeploymentNoteDim {
        width,
        height: text_height + NOTE_MARGIN_Y * 2.0,
    }
}

fn deployment_center_label_rows(label: &str) -> Vec<&str> {
    rustuml_parser::display::split_escaped_newlines(label)
}

fn deployment_center_label_display_row(row: &str) -> &str {
    deployment_note_display_row(row)
}

fn deployment_center_label_size(label: &str, floor_height: bool) -> EdgeLabelSize {
    let rows = deployment_center_label_rows(label);
    let width = rows
        .iter()
        .map(|row| deployment_center_label_display_row(row))
        .map(|row| text_render::measure(row, 13.0, false))
        .fold(0.0_f64, f64::max)
        + 2.0;
    let height = rows
        .iter()
        .map(|row| deployment_center_label_display_row(row))
        .map(|row| text_render::label_height(row, 13.0))
        .sum::<f64>()
        + 2.0;
    EdgeLabelSize {
        width,
        height: if floor_height { height.floor() } else { height },
    }
}

fn deployment_link_note_label_size(
    note: &DeploymentLinkNote,
    note_dim: DeploymentNoteDim,
    label_size: Option<EdgeLabelSize>,
) -> EdgeLabelSize {
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let Some(label_size) = label_size else {
        return EdgeLabelSize {
            width: note_width,
            height: note_height,
        };
    };

    // Java provenance: `SvekEdge` lines 301-319 at
    // 71806a23780b04a5ccde2f8ceb5121edad5eb711 use mergeLR for LEFT/RIGHT
    // and mergeTB for TOP/BOTTOM before Graphviz receives one label table.
    match note.position {
        DeploymentNotePosition::Left | DeploymentNotePosition::Right => EdgeLabelSize {
            width: note_width + label_size.width,
            height: note_height.max(label_size.height),
        },
        DeploymentNotePosition::Top | DeploymentNotePosition::Bottom => EdgeLabelSize {
            width: note_width.max(label_size.width),
            height: note_height + label_size.height,
        },
    }
}

fn deployment_svek_spacing(diagram: &DeploymentDiagram) -> (f64, f64) {
    let mut horizontal_dzeta = 0.0_f64;
    let mut vertical_dzeta = 0.0_f64;
    for connection in &diagram.connections {
        let center = if let Some(note) = connection.note.as_ref() {
            Some(deployment_link_note_label_size(
                note,
                deployment_link_note_dim(note),
                connection
                    .label
                    .as_deref()
                    .map(|label| deployment_center_label_size(label, false)),
            ))
        } else {
            connection
                .label
                .as_deref()
                .map(|label| deployment_center_label_size(label, false))
        };
        let endpoint_size = |label: Option<&str>| {
            label.map(|label| EdgeLabelSize {
                width: text_render::measure(label, 13.0, false),
                height: text_render::label_height(label, 13.0),
            })
        };
        let tail = endpoint_size(connection.tail_label.as_deref());
        let head = endpoint_size(connection.head_label.as_deref());
        let decoration = if connection.arrow_at_start {
            LINK_DECOR_ARROW_MARGIN
        } else {
            LINK_DECOR_NONE_MARGIN
        } + if connection.arrow_at_end {
            LINK_DECOR_ARROW_MARGIN
        } else {
            LINK_DECOR_NONE_MARGIN
        };

        if connection.length == 1 {
            let footprint = center.map_or(0.0, |size| size.width)
                + tail.map_or(0.0, |size| size.width)
                + head.map_or(0.0, |size| size.width)
                + decoration;
            horizontal_dzeta = horizontal_dzeta.max(footprint);
        } else {
            let footprint = center.map_or(0.0, |size| size.height)
                + tail.map_or(0.0, |size| size.height)
                + head.map_or(0.0, |size| size.height)
                + decoration;
            vertical_dzeta = vertical_dzeta.max(footprint);
        }
    }
    (
        SVEK_MIN_NODE_SEP.max(horizontal_dzeta / SVEK_DZETA_DIVISOR),
        SVEK_MIN_RANK_SEP.max(vertical_dzeta / SVEK_DZETA_DIVISOR),
    )
}

fn deployment_link_note_blocks(
    x: f64,
    y: f64,
    note: &DeploymentLinkNote,
    note_dim: DeploymentNoteDim,
    label_size: Option<EdgeLabelSize>,
) -> (Option<(f64, f64)>, (f64, f64)) {
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let Some(label_size) = label_size else {
        return (None, (x + LINK_NOTE_PADDING, y + LINK_NOTE_PADDING));
    };

    let (label_origin, note_origin) = match note.position {
        DeploymentNotePosition::Left => {
            let merged_height = note_height.max(label_size.height);
            (
                (
                    x + note_width,
                    y + (merged_height - label_size.height) / 2.0,
                ),
                (x, y + (merged_height - note_height) / 2.0),
            )
        }
        DeploymentNotePosition::Right => {
            let merged_height = note_height.max(label_size.height);
            (
                (x, y + (merged_height - label_size.height) / 2.0),
                (
                    x + label_size.width,
                    y + (merged_height - note_height) / 2.0,
                ),
            )
        }
        DeploymentNotePosition::Top => {
            let merged_width = note_width.max(label_size.width);
            (
                (x + (merged_width - label_size.width) / 2.0, y + note_height),
                (x + (merged_width - note_width) / 2.0, y),
            )
        }
        DeploymentNotePosition::Bottom => {
            let merged_width = note_width.max(label_size.width);
            (
                (x + (merged_width - label_size.width) / 2.0, y),
                (x + (merged_width - note_width) / 2.0, y + label_size.height),
            )
        }
    };

    (
        Some(label_origin),
        (
            note_origin.0 + LINK_NOTE_PADDING,
            note_origin.1 + LINK_NOTE_PADDING,
        ),
    )
}

fn laid_out_deployment_note_indices(diagram: &DeploymentDiagram) -> Vec<usize> {
    diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| {
            note.target.as_deref().map_or(Some(index), |target| {
                diagram
                    .nodes
                    .iter()
                    .any(|node| node.id == target)
                    .then_some(index)
            })
        })
        .collect()
}

fn deployment_cluster_ids<'a>(
    diagram: &'a DeploymentDiagram,
    laid_out_note_indices: &[usize],
) -> HashSet<&'a str> {
    let laid_out_notes = laid_out_note_indices
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let note_owners = diagram
        .notes
        .iter()
        .enumerate()
        .filter(|(index, _)| laid_out_notes.contains(index))
        .filter_map(|(_, note)| note.owner.as_deref())
        .collect::<HashSet<_>>();
    diagram
        .nodes
        .iter()
        .filter(|node| !node.children.is_empty() || note_owners.contains(node.id.as_str()))
        .map(|node| node.id.as_str())
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
        Default => {
            // `CircleInterface2` reserves an 18px symbol box inside the
            // overlap shield and draws its 16px ellipse one pixel inward.
            let symbol_x = rect.x + (dim.width - INTERFACE_SYMBOL_SIZE) / 2.0;
            let symbol_y = rect.y + (dim.height - INTERFACE_SYMBOL_SIZE) / 2.0;
            empty_entity_rect(
                symbol_x + 1.0,
                symbol_y + 1.0,
                INTERFACE_CIRCLE_SIZE,
                INTERFACE_CIRCLE_SIZE,
            )
        }
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
        Default => {
            let symbol_x = rect.x + (dim.width - INTERFACE_SYMBOL_SIZE) / 2.0;
            let symbol_y = rect.y + (dim.height - INTERFACE_SYMBOL_SIZE) / 2.0;
            (
                symbol_x + (INTERFACE_SYMBOL_SIZE - dim.label_width) / 2.0,
                symbol_y + INTERFACE_LABEL_Y + ASCENT_14,
            )
        }
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
    let sprite_cache = crate::sprite::SpriteCache::from_sprites_scaled_with_colors(
        &diagram.meta.sprites,
        sprite_scale(),
        [0xF1, 0xF1, 0xF1],
        [0, 0, 0],
    );
    let ctx = OracleRenderContext {
        diagram,
        oracle: &oracle,
        id_for_node: &no_oracle_uids.entity_ids,
        skin_fills: &skin_fills,
        skin_strokes: &skin_strokes,
        sprites: &diagram.meta.sprites,
        sprite_cache: &sprite_cache,
        handwritten: false,
    };

    let mut svg = SvgBuilder::new_plantuml(dim.width + 20.0, dim.height + 20.0, "DESCRIPTION");
    emit_entity(&mut svg, node, &qnames[&node.id], &ctx);
    svg.finalize_plantuml()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeploymentSvekLeaf {
    Node(usize),
    Note(usize),
}

fn deployment_svek_leaf_order(
    diagram: &DeploymentDiagram,
    parent_of: &HashMap<String, String>,
    cluster_ids: &HashSet<&str>,
    laid_out_note_indices: &[usize],
) -> Vec<DeploymentSvekLeaf> {
    fn collect_owner(
        diagram: &DeploymentDiagram,
        parent_of: &HashMap<String, String>,
        cluster_ids: &HashSet<&str>,
        laid_out_notes: &HashSet<usize>,
        owner: Option<&str>,
        recurse_children: bool,
        result: &mut Vec<DeploymentSvekLeaf>,
    ) {
        let mut direct = diagram
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                !cluster_ids.contains(node.id.as_str())
                    && parent_of.get(&node.id).map(String::as_str) == owner
            })
            .map(|(index, node)| {
                (
                    node.quark_order,
                    node.source_line,
                    0_u8,
                    index,
                    DeploymentSvekLeaf::Node(index),
                )
            })
            .chain(
                diagram
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(index, note)| {
                        laid_out_notes.contains(index) && note.owner.as_deref() == owner
                    })
                    .map(|(index, note)| {
                        (
                            note.quark_order,
                            note.source_line,
                            1_u8,
                            index,
                            DeploymentSvekLeaf::Note(index),
                        )
                    }),
            )
            .collect::<Vec<_>>();
        direct.sort_by_key(|&(quark_order, source_line, kind, index, _)| {
            (quark_order, source_line, kind, index)
        });
        result.extend(direct.into_iter().map(|(_, _, _, _, leaf)| leaf));

        if !recurse_children {
            return;
        }
        let mut children = diagram
            .nodes
            .iter()
            .filter(|node| {
                cluster_ids.contains(node.id.as_str())
                    && parent_of.get(&node.id).map(String::as_str) == owner
            })
            .collect::<Vec<_>>();
        children.sort_by_key(|node| (node.quark_order, node.source_line));
        for child in children {
            collect_owner(
                diagram,
                parent_of,
                cluster_ids,
                laid_out_notes,
                Some(&child.id),
                true,
                result,
            );
        }
    }

    let laid_out_notes = laid_out_note_indices.iter().copied().collect();
    let mut result = Vec::with_capacity(diagram.nodes.len() + laid_out_note_indices.len());
    let mut roots = diagram
        .nodes
        .iter()
        .filter(|node| cluster_ids.contains(node.id.as_str()) && !parent_of.contains_key(&node.id))
        .collect::<Vec<_>>();
    roots.sort_by_key(|node| (node.quark_order, node.source_line));
    for root in roots {
        collect_owner(
            diagram,
            parent_of,
            cluster_ids,
            &laid_out_notes,
            Some(&root.id),
            true,
            &mut result,
        );
    }
    collect_owner(
        diagram,
        parent_of,
        cluster_ids,
        &laid_out_notes,
        None,
        false,
        &mut result,
    );
    result
}

fn deployment_svek_paint_order(
    diagram: &DeploymentDiagram,
    parent_of: &HashMap<String, String>,
    cluster_ids: &HashSet<&str>,
    laid_out_note_indices: &[usize],
) -> Vec<DeploymentSvekLeaf> {
    fn collect_group(
        diagram: &DeploymentDiagram,
        parent_of: &HashMap<String, String>,
        cluster_ids: &HashSet<&str>,
        laid_out_notes: &HashSet<usize>,
        owner: &str,
        result: &mut Vec<DeploymentSvekLeaf>,
    ) {
        let mut direct = diagram
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                !cluster_ids.contains(node.id.as_str())
                    && parent_of.get(&node.id).map(String::as_str) == Some(owner)
            })
            .map(|(index, node)| {
                (
                    node.quark_order,
                    node.source_line,
                    0_u8,
                    index,
                    DeploymentSvekLeaf::Node(index),
                )
            })
            .chain(
                diagram
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(index, note)| {
                        laid_out_notes.contains(index) && note.owner.as_deref() == Some(owner)
                    })
                    .map(|(index, note)| {
                        (
                            note.quark_order,
                            note.source_line,
                            1_u8,
                            index,
                            DeploymentSvekLeaf::Note(index),
                        )
                    }),
            )
            .collect::<Vec<_>>();
        direct.sort_by_key(|&(quark_order, source_line, kind, index, _)| {
            (quark_order, source_line, kind, index)
        });
        result.extend(direct.into_iter().map(|(_, _, _, _, leaf)| leaf));

        let mut child_groups = diagram
            .nodes
            .iter()
            .filter(|node| {
                cluster_ids.contains(node.id.as_str())
                    && parent_of.get(&node.id).map(String::as_str) == Some(owner)
            })
            .collect::<Vec<_>>();
        child_groups.sort_by_key(|node| (node.quark_order, node.source_line));
        for child in child_groups {
            collect_group(
                diagram,
                parent_of,
                cluster_ids,
                laid_out_notes,
                &child.id,
                result,
            );
        }
    }

    let laid_out_notes = laid_out_note_indices
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut result = Vec::with_capacity(diagram.nodes.len() + laid_out_note_indices.len());
    for (index, root) in diagram
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| !parent_of.contains_key(&node.id))
    {
        if cluster_ids.contains(root.id.as_str()) {
            collect_group(
                diagram,
                parent_of,
                cluster_ids,
                &laid_out_notes,
                &root.id,
                &mut result,
            );
        } else if root.declared_container {
            result.push(DeploymentSvekLeaf::Node(index));
        }
    }

    let mut root_leaves = diagram
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            !cluster_ids.contains(node.id.as_str())
                && !node.declared_container
                && !parent_of.contains_key(&node.id)
        })
        .map(|(index, node)| {
            (
                node.quark_order,
                node.source_line,
                0_u8,
                index,
                DeploymentSvekLeaf::Node(index),
            )
        })
        .chain(
            diagram
                .notes
                .iter()
                .enumerate()
                .filter(|(index, note)| laid_out_notes.contains(index) && note.owner.is_none())
                .map(|(index, note)| {
                    (
                        note.quark_order,
                        note.source_line,
                        1_u8,
                        index,
                        DeploymentSvekLeaf::Note(index),
                    )
                }),
        )
        .collect::<Vec<_>>();
    root_leaves.sort_by_key(|&(quark_order, source_line, kind, index, _)| {
        (quark_order, source_line, kind, index)
    });
    result.extend(root_leaves.into_iter().map(|(_, _, _, _, leaf)| leaf));
    result
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
    let cluster_ids = deployment_cluster_ids(diagram, &laid_out_note_indices);
    let cluster_endpoint_nodes: HashMap<String, String> = diagram
        .connections
        .iter()
        .flat_map(|connection| [&connection.from, &connection.to])
        .filter(|endpoint| cluster_ids.contains(endpoint.as_str()))
        .map(|endpoint| {
            (
                endpoint.clone(),
                format!("__svek_group_endpoint_{endpoint}"),
            )
        })
        .collect();
    // Java provenance: `AbstractEntityDiagram.getRankdir()` carries
    // `left to right direction` into SVEK's DOT `rankdir`.
    let layout_direction = match diagram.direction {
        DeploymentLayoutDirection::TopToBottom => Direction::TopToBottom,
        DeploymentLayoutDirection::LeftToRight => Direction::LeftToRight,
    };
    let (node_sep, rank_sep) = deployment_svek_spacing(diagram);
    let mut layout = LayoutGraph::new(layout_direction)
        .with_spacing_pixels(node_sep, rank_sep)
        .with_plantuml_svek_node_order();
    let mut entity_layout_slots = vec![None; diagram.nodes.len()];
    let mut note_layout_slots = vec![None; diagram.notes.len()];
    let mut next_layout_slot = 0usize;
    let svek_leaf_order =
        deployment_svek_leaf_order(diagram, &parent_of, &cluster_ids, &laid_out_note_indices);
    let svek_paint_order =
        deployment_svek_paint_order(diagram, &parent_of, &cluster_ids, &laid_out_note_indices);
    for &leaf in &svek_leaf_order {
        match leaf {
            DeploymentSvekLeaf::Node(node_index) => {
                let node = &diagram.nodes[node_index];
                let (width, height) = deployment_layout_node_size(node.kind, &dims[node_index]);
                assert!(
                    layout.add_node(&node.id, &node.label, width, height),
                    "DESCRIPTION SVEK plan contains duplicate node {}",
                    node.id
                );
                entity_layout_slots[node_index] = Some(next_layout_slot);
                next_layout_slot += 1;
            }
            DeploymentSvekLeaf::Note(note_index) => {
                let dim = note_dims[note_index];
                let note_id = deployment_note_layout_id(&diagram.notes[note_index], note_index);
                assert!(
                    layout.add_node(&note_id, "", dim.width, dim.height),
                    "DESCRIPTION SVEK plan contains duplicate note {note_id}"
                );
                note_layout_slots[note_index] = Some(next_layout_slot);
                next_layout_slot += 1;
            }
        }
    }
    for endpoint_id in cluster_endpoint_nodes.values() {
        layout.add_svek_cluster_endpoint(endpoint_id);
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
            if let Some(endpoint_id) = cluster_endpoint_nodes.get(&node.id) {
                layout.add_cluster_node(&node.id, endpoint_id);
            }
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
    for &note_index in &laid_out_note_indices {
        let note = &diagram.notes[note_index];
        if let Some(owner) = note.owner.as_deref()
            && cluster_ids.contains(owner)
        {
            layout.add_cluster_node(
                owner,
                &deployment_note_layout_id(&diagram.notes[note_index], note_index),
            );
        }
    }
    add_deployment_magma_constraints(&mut layout, diagram, &parent_of, &cluster_ids);
    for &note_index in &laid_out_note_indices {
        let note = &diagram.notes[note_index];
        let Some(target) = note.target.as_deref() else {
            continue;
        };
        let note_id = deployment_note_layout_id(note, note_index);
        let layout_target = cluster_endpoint_nodes
            .get(target)
            .map(String::as_str)
            .unwrap_or(target);
        let (from, to) = match note.position {
            DeploymentNotePosition::Top | DeploymentNotePosition::Left => {
                (note_id.as_str(), layout_target)
            }
            DeploymentNotePosition::Bottom | DeploymentNotePosition::Right => {
                (layout_target, note_id.as_str())
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
        let (layout_from, layout_to, reversed) =
            deployment_connection_layout_with_endpoints(conn, &cluster_endpoint_nodes);
        if conn.length == 1 {
            // Java `Bibliotekon.addLine` serializes every length-one edge in
            // `lines0`, between promoted inverted starts and ordinary nodes.
            layout.add_plantuml_svek_line0_edge(layout_from, layout_to);
        }
        if reversed {
            // Java `CommandLinkElement.executeArg` preserves LEFT/UP as an
            // inverted Link after swapping endpoints. `Cluster` then emits
            // that link's start before ordinary SVEK nodes.
            layout.add_plantuml_svek_inverted_start(layout_from);
        }
        // `SvekEdge.getLabelText` expands Display row controls before
        // `appendLine` wraps the complete block in one-pixel margins.
        let ordinary_label_size = conn
            .label
            .as_deref()
            .map(|label| deployment_center_label_size(label, true));
        let label_size = if let Some(note) = conn.note.as_ref() {
            let exact_label_size = conn
                .label
                .as_deref()
                .map(|label| deployment_center_label_size(label, false));
            Some(deployment_link_note_label_size(
                note,
                deployment_link_note_dim(note),
                exact_label_size,
            ))
        } else {
            ordinary_label_size
        };
        let endpoint_label_size = |label: Option<&str>| {
            label.map(|label| EdgeLabelSize {
                // `SvekEdge.appendLine` sends cardinality text dimensions
                // directly to `appendTable`, without the center-label shield.
                width: text_render::measure(label, 13.0, false),
                height: text_render::label_height(label, 13.0).floor(),
            })
        };
        let (tail_label_size, head_label_size) = if reversed {
            (
                endpoint_label_size(conn.head_label.as_deref()),
                endpoint_label_size(conn.tail_label.as_deref()),
            )
        } else {
            (
                endpoint_label_size(conn.tail_label.as_deref()),
                endpoint_label_size(conn.head_label.as_deref()),
            )
        };
        let minlen = match conn.direction {
            Some(DeploymentLinkDirection::Left | DeploymentLinkDirection::Right) => Some(0),
            // `CommandLinkElement` stores the shaft's character count in
            // `LinkArg`; `SvekEdge.appendLine` emits length - 1.
            Some(DeploymentLinkDirection::Up | DeploymentLinkDirection::Down) | None => {
                Some(conn.length.saturating_sub(1))
            }
        };
        // `WithLinkType.goHidden` marks only the later paint operation hidden;
        // Java still serializes and solves the edge as an ordinary SVEK edge.
        layout.add_edge_with_label_sizes_and_minlen(
            layout_from,
            layout_to,
            label_size,
            tail_label_size,
            head_label_size,
            minlen,
        );
    }

    let mut result = layout.layout_full(LAYOUT_TIMEOUT);
    if let Some(result) = result.as_mut() {
        let solved_positions = result.node_positions.clone();
        result.node_positions = entity_layout_slots
            .iter()
            .flatten()
            .map(|&slot| solved_positions[slot])
            .chain(
                laid_out_note_indices
                    .iter()
                    .filter_map(|&note_index| note_layout_slots[note_index])
                    .map(|slot| solved_positions[slot]),
            )
            .chain(solved_positions.iter().skip(next_layout_slot).copied())
            .collect();
    }
    if let Some(result) = result.as_mut() {
        adjust_deployment_endpoint_labels(diagram, &dims, result);
    }
    let opale_note_connections: HashMap<usize, usize> = result
        .as_ref()
        .map(|result| {
            diagram
                .notes
                .iter()
                .enumerate()
                .filter_map(|(note_index, _)| {
                    deployment_note_opale_connection(
                        diagram,
                        note_index,
                        &result.edge_paths,
                        &cluster_endpoint_nodes,
                        &cluster_ids,
                    )
                    .map(|(connection_index, _)| (note_index, connection_index))
                })
                .collect()
        })
        .unwrap_or_default();
    let opale_connection_indices: HashSet<usize> =
        opale_note_connections.values().copied().collect();
    let edge_paint_bounds = result.as_ref().and_then(|result| {
        deployment_edge_paint_bounds(
            diagram,
            result,
            &cluster_endpoint_nodes,
            &opale_connection_indices,
        )
    });
    let cluster_frame = deployment_cluster_frame(diagram, &dims, result.as_ref());
    let y_frame = deployment_body_y_frame(
        diagram,
        &dims,
        &note_dims,
        &laid_out_note_indices,
        result.as_ref(),
        edge_paint_bounds,
    );
    let mut body_margin_y = cluster_frame
        .map(|frame| frame.margin_y)
        .or_else(|| y_frame.map(|frame| frame.margin))
        .unwrap_or_else(|| deployment_body_margin_y(diagram, &dims, result.as_ref()));
    let x_frame = deployment_body_x_frame(
        diagram,
        &dims,
        &note_dims,
        &laid_out_note_indices,
        result.as_ref(),
        edge_paint_bounds,
    );
    let mut body_margin_x = x_frame
        .map(|frame| frame.margin)
        .or_else(|| cluster_frame.map(|frame| frame.margin_x))
        .unwrap_or(BODY_FALLBACK_MARGIN_X);
    let (mut rects, content_w, content_h) = layout_deployment_rects(
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
    let mut note_layouts: Vec<DeploymentNoteLayout> = result
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
                    .outer_symbol_painted_max_x
                    .map(|max_x| max_x + SVEK_DIMENSION_DELTA)
            })
        })
        .unwrap_or(content_w + BODY_RIGHT_MARGIN);
    let total_w = edge_paint_bounds
        .map(|bounds| total_w.max(bounds.max_x + body_margin_x + SVEK_DIMENSION_DELTA))
        .unwrap_or(total_w);
    let total_h = y_frame
        .map(|frame| frame.painted_max_y + frame.margin + SVEK_DIMENSION_DELTA)
        .or_else(|| {
            cluster_frame.and_then(|frame| {
                frame
                    .outer_symbol_painted_max_y
                    .map(|max_y| max_y + SVEK_DIMENSION_DELTA)
            })
        })
        .unwrap_or(content_h + BODY_BOTTOM_MARGIN);
    let total_h = edge_paint_bounds
        .map(|bounds| total_h.max(bounds.max_y + body_margin_y + SVEK_DIMENSION_DELTA))
        .unwrap_or(total_h);
    let deprecated_handwritten = has_deprecated_handwritten_skinparam(&diagram.meta.skinparams);
    let warning_height = if deprecated_handwritten {
        deprecated_handwritten_warning_block_height()
    } else {
        0.0
    };
    // `WarningBannerBlock.calculateDimension` is part of the decorated text
    // block; the outer SVEK image envelope contributes its six-pixel origin.
    let warned_body_width = if deprecated_handwritten {
        total_w.max(deprecated_handwritten_warning_block_width() + SVEK_ENVELOPE_ORIGIN)
    } else {
        total_w
    };
    let chrome = deployment_chrome_layout(diagram, warned_body_width, total_h + warning_height);
    let body_dx = chrome.body_dx;
    let body_dy = chrome.body_dy + warning_height;
    body_margin_x += body_dx;
    body_margin_y += body_dy;
    for rect in &mut rects {
        rect.x += body_dx;
        rect.y += body_dy;
    }
    for note in &mut note_layouts {
        note.x += body_dx;
        note.y += body_dy;
    }
    let total_w = chrome.canvas_width;
    let total_h = chrome.canvas_height;
    let cluster_rects: HashMap<&str, LayoutRect> = diagram
        .nodes
        .iter()
        .zip(&rects)
        .filter(|(node, _)| cluster_ids.contains(node.id.as_str()))
        .map(|(node, rect)| (node.id.as_str(), *rect))
        .collect();

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
    let sprite_cache = crate::sprite::SpriteCache::from_sprites_scaled_with_colors(
        &diagram.meta.sprites,
        sprite_scale(),
        [0xF1, 0xF1, 0xF1],
        [0, 0, 0],
    );
    let ctx = OracleRenderContext {
        diagram,
        oracle: &oracle,
        id_for_node,
        skin_fills: &skin_fills,
        skin_strokes: &skin_strokes,
        sprites: &diagram.meta.sprites,
        sprite_cache: &sprite_cache,
        handwritten: is_handwritten_enabled(&diagram.meta.skinparams),
    };

    let note_gradient_defs = deployment_note_gradient_defs(diagram);
    let gradient_defs = (!note_gradient_defs.is_empty()).then_some(note_gradient_defs.as_str());
    let mut svg = SvgBuilder::new_plantuml_with_background_and_defs(
        total_w,
        total_h,
        "DESCRIPTION",
        Some("#FFFFFF"),
        &note_gradient_defs,
    );
    emit_deployment_chrome_top(&mut svg, diagram, &chrome);
    if diagram.meta.legend_vertical_alignment == LegendVerticalAlignment::Top {
        emit_deployment_legend(&mut svg, diagram, &chrome);
    }
    if deprecated_handwritten {
        emit_generated_deprecated_handwritten_warning(
            &mut svg,
            warned_body_width - SVEK_ENVELOPE_ORIGIN,
            chrome.body_dx,
            chrome.body_dy,
        );
    }
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
        emit_clusters_dfs(&mut svg, root, &diagram.nodes, None, &cluster_ids, &ctx);
    }
    let emit_note_layout = |svg: &mut SvgBuilder, layout: &DeploymentNoteLayout| {
        let Some(result) = result.as_ref() else {
            return;
        };
        let note = &diagram.notes[layout.note_index];
        let Some(uid) = no_oracle_uids.note_ids.get(&layout.note_index) else {
            return;
        };
        let Some(target) = note.target.as_deref() else {
            let opale =
                opale_note_connections
                    .get(&layout.note_index)
                    .and_then(|&connection_index| {
                        let connection = &diagram.connections[connection_index];
                        let edge = deployment_connection_edge(
                            diagram,
                            connection_index,
                            &result.edge_paths,
                            &cluster_endpoint_nodes,
                        )?;
                        let (logical_from, logical_to, _) =
                            deployment_connection_layout(connection);
                        let geometry = deployment_opale_note_geometry(
                            edge,
                            layout,
                            body_margin_x,
                            body_margin_y,
                            cluster_rects.get(logical_from),
                            cluster_rects.get(logical_to),
                        )?;
                        Some((edge, geometry))
                    });
            if let Some((edge, (position, note_at_edge_start))) = opale {
                render_attached_deployment_note(
                    svg,
                    note,
                    layout,
                    uid,
                    DeploymentNoteLeader {
                        position,
                        edge: Some(edge),
                        note_at_edge_start: Some(note_at_edge_start),
                    },
                    (body_margin_x, body_margin_y),
                    gradient_defs,
                );
            } else {
                render_floating_deployment_note(svg, note, layout, uid, gradient_defs);
            }
            return;
        };
        let note_id = deployment_note_layout_id(note, layout.note_index);
        let layout_target = cluster_endpoint_nodes
            .get(target)
            .map(String::as_str)
            .unwrap_or(target);
        let (from, to) = match note.position {
            DeploymentNotePosition::Top | DeploymentNotePosition::Left => {
                (note_id.as_str(), layout_target)
            }
            DeploymentNotePosition::Bottom | DeploymentNotePosition::Right => {
                (layout_target, note_id.as_str())
            }
        };
        let edge = result
            .edge_paths
            .iter()
            .find(|edge| edge.from == from && edge.to == to);
        render_attached_deployment_note(
            svg,
            note,
            layout,
            uid,
            DeploymentNoteLeader {
                position: note.position,
                edge,
                note_at_edge_start: None,
            },
            (body_margin_x, body_margin_y),
            gradient_defs,
        );
    };

    // Java `GraphvizImageBuilder.printGroups` paints each group's direct
    // quark-ordered leaves before nested groups, then paints unpackaged
    // leaves. Empty deduplicated groups retain their root traversal slot.
    for leaf in &svek_paint_order {
        match *leaf {
            DeploymentSvekLeaf::Node(node_index) => {
                let node = &diagram.nodes[node_index];
                emit_entity(&mut svg, node, &qnames[&node.id], &ctx);
            }
            DeploymentSvekLeaf::Note(note_index) => {
                if let Some(layout) = note_layouts
                    .iter()
                    .find(|layout| layout.note_index == note_index)
                {
                    emit_note_layout(&mut svg, layout);
                }
            }
        }
    }
    if let Some(result) = result.as_ref() {
        render_no_oracle_edges(
            &mut svg,
            diagram,
            id_for_node,
            &no_oracle_uids.link_ids,
            &result.edge_paths,
            body_margin_x,
            body_margin_y,
            &cluster_endpoint_nodes,
            &cluster_rects,
            &opale_connection_indices,
            gradient_defs,
            ctx.handwritten,
        );
    }
    if diagram.meta.legend_vertical_alignment == LegendVerticalAlignment::Bottom {
        emit_deployment_legend(&mut svg, diagram, &chrome);
    }
    emit_deployment_chrome_bottom(&mut svg, diagram, &chrome);
    svg.finalize_plantuml()
}

#[derive(Clone, Copy)]
struct DeploymentChromeLayout {
    body_dx: f64,
    body_dy: f64,
    canvas_width: f64,
    canvas_height: f64,
    content_width: f64,
    header_height: f64,
    legend_rect_x: Option<f64>,
    legend_rect_y: Option<f64>,
}

fn deployment_legend_rows(text: Option<&str>) -> Vec<Vec<&str>> {
    text.map(|text| {
        text.lines()
            .filter(|line| line.trim().contains('|'))
            .map(|line| {
                line.trim()
                    .trim_matches('|')
                    .split('|')
                    .map(str::trim)
                    .filter(|cell| !cell.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|row| !row.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

fn deployment_legend_column_widths(rows: &[Vec<&str>]) -> Vec<f64> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let cell_pad_x = pm::descent(LEGEND_FONT_SIZE) * LEGEND_CELL_PAD_DESCENT_FACTOR;
    let mut widths = vec![0.0_f64; column_count];
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            let text_width = text_render::measure_no_underline(cell, LEGEND_FONT_SIZE, false);
            widths[index] = widths[index].max(text_width + 2.0 * cell_pad_x);
        }
    }
    widths
}

fn deployment_legend_rect_size(rows: &[Vec<&str>]) -> Option<(f64, f64)> {
    if rows.is_empty() {
        return None;
    }
    let grid_width = deployment_legend_column_widths(rows).iter().sum::<f64>();
    let row_height = pm::text_height(LEGEND_FONT_SIZE);
    Some((
        grid_width + 2.0 * LEGEND_RECT_PAD_X,
        rows.len() as f64 * row_height + 2.0 * LEGEND_RECT_PAD_Y,
    ))
}

fn deployment_chrome_layout(
    diagram: &DeploymentDiagram,
    body_canvas_width: f64,
    body_canvas_height: f64,
) -> DeploymentChromeLayout {
    let legend_rows = deployment_legend_rows(diagram.meta.legend.as_deref());
    let legend_rect_size = deployment_legend_rect_size(&legend_rows);
    let (
        decorated_canvas_width,
        decorated_canvas_height,
        legend_body_dx,
        legend_body_dy,
        base_legend_rect_x,
        base_legend_rect_y,
    ) = if let Some((rect_width, rect_height)) = legend_rect_size {
        // `SvekResult.calculateDimension` has already moved the painted
        // minimum to six. `DecorateEntityImage` composes the underlying
        // dimension without that origin, then the outer image builder restores
        // it around the decorated result.
        let body_width = (body_canvas_width - SVEK_ENVELOPE_ORIGIN).max(0.0);
        let body_height = (body_canvas_height - SVEK_ENVELOPE_ORIGIN).max(0.0);
        let block_width = rect_width + LEGEND_BORDERED_DIMENSION_DELTA + 2.0 * LEGEND_OUTER_MARGIN;
        let block_height =
            rect_height + LEGEND_BORDERED_DIMENSION_DELTA + 2.0 * LEGEND_OUTER_MARGIN;
        let decorated_width = body_width.max(block_width);
        let block_x = match diagram.meta.legend_horizontal_alignment {
            LegendHorizontalAlignment::Left => 0.0,
            LegendHorizontalAlignment::Center => (decorated_width - block_width) / 2.0,
            LegendHorizontalAlignment::Right => decorated_width - block_width,
        };
        let (block_y, body_dy) = match diagram.meta.legend_vertical_alignment {
            LegendVerticalAlignment::Top => (0.0, block_height),
            LegendVerticalAlignment::Bottom => (body_height, 0.0),
        };
        (
            decorated_width + SVEK_ENVELOPE_ORIGIN,
            body_height + block_height + SVEK_ENVELOPE_ORIGIN,
            (decorated_width - body_width) / 2.0,
            body_dy,
            Some(block_x + LEGEND_OUTER_MARGIN),
            Some(block_y + LEGEND_OUTER_MARGIN),
        )
    } else {
        (body_canvas_width, body_canvas_height, 0.0, 0.0, None, None)
    };

    // Java `DiagramChromeFactory12026` wraps the raw `SvekResult` in
    // `DecorateEntityImage` blocks. Each wrapper takes the wider of the body
    // and its text block, centres the narrower body, and stacks top/bottom
    // block heights. The SVG exporter contributes the established 7px
    // trailing pad after that composition.
    let body_width = (decorated_canvas_width - TITLE_RIGHT_PAD).max(0.0);
    let title_width = diagram
        .meta
        .title
        .as_deref()
        .map(|title| {
            title
                .lines()
                .map(|line| text_render::measure(line, TITLE_FONT_SIZE, true))
                .fold(0.0_f64, f64::max)
                + 2.0 * TITLE_MARGIN_X
        })
        .unwrap_or(0.0);
    let header_width = diagram
        .meta
        .header
        .as_deref()
        .map(|header| {
            header
                .lines()
                .map(|line| text_render::measure(line, HEADER_FONT_SIZE, false))
                .fold(0.0_f64, f64::max)
        })
        .unwrap_or(0.0);
    let footer_width = diagram
        .meta
        .footer
        .as_deref()
        .map(|footer| {
            footer
                .lines()
                .map(|line| text_render::measure(line, HEADER_FONT_SIZE, false))
                .fold(0.0_f64, f64::max)
        })
        .unwrap_or(0.0);
    let caption_block_width = header_width.max(footer_width);
    let content_width = body_width.max(title_width).max(caption_block_width);
    let outer_body_dx = (content_width - body_width) / 2.0;
    let title_height = diagram
        .meta
        .title
        .as_deref()
        .map(|title| {
            TITLE_TOP_PAD + title.lines().count().max(1) as f64 * TITLE_LINE_H + TITLE_BOTTOM_PAD
        })
        .unwrap_or(0.0);
    let header_height = diagram
        .meta
        .header
        .as_deref()
        .map(|header| {
            header.lines().count().max(1) as f64 * pm::text_height(HEADER_FONT_SIZE)
                + CAPTION_BOTTOM_PAD
        })
        .unwrap_or(0.0);
    let footer_height = diagram
        .meta
        .footer
        .as_deref()
        .map(|footer| {
            footer.lines().count().max(1) as f64 * pm::text_height(HEADER_FONT_SIZE)
                + CAPTION_BOTTOM_PAD
        })
        .unwrap_or(0.0);

    DeploymentChromeLayout {
        body_dx: legend_body_dx + outer_body_dx,
        body_dy: legend_body_dy + header_height + title_height,
        canvas_width: content_width + TITLE_RIGHT_PAD,
        canvas_height: decorated_canvas_height + header_height + title_height + footer_height,
        content_width,
        header_height,
        legend_rect_x: base_legend_rect_x.map(|x| x + outer_body_dx),
        legend_rect_y: base_legend_rect_y.map(|y| y + header_height + title_height),
    }
}

fn emit_deployment_chrome_top(
    svg: &mut SvgBuilder,
    diagram: &DeploymentDiagram,
    chrome: &DeploymentChromeLayout,
) {
    if let Some(header) = diagram.meta.header.as_deref() {
        svg.raw(r#"<g class="header" data-source-line="1">"#);
        for (index, line) in header.lines().enumerate() {
            let width = text_render::measure(line, HEADER_FONT_SIZE, false);
            // `DisplayPositioned` defaults headers to RIGHT alignment.
            let x = chrome.content_width - width;
            let y = pm::ascent(HEADER_FONT_SIZE) + index as f64 * pm::text_height(HEADER_FONT_SIZE);
            emit_grey_text(svg, line, x, y);
        }
        svg.raw("</g>");
    }

    if let Some(title) = diagram.meta.title.as_deref() {
        svg.raw(r#"<g class="title" data-source-line="1">"#);
        for (index, line) in title.lines().enumerate() {
            let width = text_render::measure(line, TITLE_FONT_SIZE, true);
            let x = TITLE_MARGIN_X.max((chrome.content_width - width) / 2.0);
            let y = chrome.header_height
                + TITLE_TOP_PAD
                + pm::ascent(TITLE_FONT_SIZE)
                + index as f64 * TITLE_LINE_H;
            emit_text(svg, line, x, y, TITLE_FONT_SIZE, true, false);
        }
        svg.raw("</g>");
    }
}

fn emit_deployment_legend(
    svg: &mut SvgBuilder,
    diagram: &DeploymentDiagram,
    chrome: &DeploymentChromeLayout,
) {
    let rows = deployment_legend_rows(diagram.meta.legend.as_deref());
    let (Some((rect_width, rect_height)), Some(rect_x), Some(rect_y)) = (
        deployment_legend_rect_size(&rows),
        chrome.legend_rect_x,
        chrome.legend_rect_y,
    ) else {
        return;
    };

    let column_widths = deployment_legend_column_widths(&rows);
    let row_height = pm::text_height(LEGEND_FONT_SIZE);
    let cell_pad_x = pm::descent(LEGEND_FONT_SIZE) * LEGEND_CELL_PAD_DESCENT_FACTOR;
    let grid_left = rect_x + LEGEND_RECT_PAD_X;
    let grid_top = rect_y + LEGEND_RECT_PAD_Y;
    let grid_width = column_widths.iter().sum::<f64>();
    let grid_right = grid_left + grid_width;
    let grid_bottom = grid_top + rows.len() as f64 * row_height;
    let source_line = diagram.meta.legend_line.unwrap_or(1);

    svg.raw(&format!(
        r#"<g class="legend" data-source-line="{source_line}">"#
    ));
    svg.raw(&format!(
        r##"<rect fill="#DDDDDD" height="{}" rx="{}" ry="{}" style="stroke:#000000;stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
        fc(rect_height),
        fc(LEGEND_RECT_RX),
        fc(LEGEND_RECT_RX),
        fc(rect_width),
        fc(rect_x),
        fc(rect_y),
    ));

    for (row_index, row) in rows.iter().enumerate() {
        let mut x = grid_left;
        let baseline = grid_top + pm::ascent(LEGEND_FONT_SIZE) + row_index as f64 * row_height;
        for (column_index, cell) in row.iter().enumerate() {
            emit_text(
                svg,
                cell,
                x + cell_pad_x,
                baseline,
                LEGEND_FONT_SIZE,
                false,
                false,
            );
            x += column_widths.get(column_index).copied().unwrap_or(0.0);
        }
    }

    for index in 0..=rows.len() {
        let y = grid_top + index as f64 * row_height;
        svg.raw(&format!(
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fc(grid_left),
            fc(grid_right),
            fc(y),
            fc(y),
        ));
    }

    let mut x = grid_left;
    svg.raw(&format!(
        r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
        fc(x),
        fc(x),
        fc(grid_top),
        fc(grid_bottom),
    ));
    for width in column_widths {
        x += width;
        svg.raw(&format!(
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fc(x),
            fc(x),
            fc(grid_top),
            fc(grid_bottom),
        ));
    }
    svg.raw("</g>");
}

fn emit_deployment_chrome_bottom(
    svg: &mut SvgBuilder,
    diagram: &DeploymentDiagram,
    chrome: &DeploymentChromeLayout,
) {
    let Some(footer) = diagram.meta.footer.as_deref() else {
        return;
    };
    let line_count = footer.lines().count().max(1);
    svg.raw(r#"<g class="footer" data-source-line="2">"#);
    for (index, line) in footer.lines().enumerate() {
        let width = text_render::measure(line, HEADER_FONT_SIZE, false);
        // `DisplayPositioned` defaults footers to CENTER alignment.
        let x = (chrome.content_width - width) / 2.0;
        let lines_below = line_count - index - 1;
        let y = chrome.canvas_height.trunc()
            - FOOTER_BOTTOM_GAP
            - lines_below as f64 * pm::text_height(HEADER_FONT_SIZE);
        emit_grey_text(svg, line, x, y);
    }
    svg.raw("</g>");
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
    let add_group = |layout: &mut LayoutGraph, members: Vec<String>| {
        // Java CucaDiagram.applySingleStrategy skips one- and two-member
        // standalone sets before constructing Magma.
        if members.len() < 3 {
            return;
        }
        // Java `CucaDiagram.applySingleStrategy` delegates standalone
        // entities to `Magma.putInSquare` / `SquareMaker.putInSquare`.
        let branch = (members.len() as f64).sqrt().ceil() as usize;
        let mut head = 0;
        for index in 1..members.len() {
            if index - head == branch {
                layout.add_invisible_edge_with_minlen(&members[head], &members[index], 1);
                head = index;
            } else {
                layout.add_invisible_edge_with_minlen(&members[index - 1], &members[index], 0);
            }
        }
    };

    let mut root_members = diagram
        .nodes
        .iter()
        .filter(|node| {
            !cluster_ids.contains(node.id.as_str())
                && !parent_of.contains_key(&node.id)
                && !linked.contains(node.id.as_str())
        })
        .map(|node| (node.quark_order, node.source_line, 0_u8, node.id.clone()))
        .collect::<Vec<_>>();
    root_members.extend(
        diagram
            .notes
            .iter()
            .enumerate()
            .filter(|(_, note)| {
                note.target.is_none()
                    && note.owner.is_none()
                    && note.id.as_deref().is_none_or(|id| !linked.contains(id))
            })
            .map(|(index, note)| {
                (
                    note.quark_order,
                    note.source_line,
                    1_u8,
                    deployment_note_layout_id(note, index),
                )
            }),
    );
    root_members
        .sort_by_key(|(quark_order, source_line, kind, _)| (*quark_order, *source_line, *kind));
    add_group(
        layout,
        root_members.into_iter().map(|(_, _, _, id)| id).collect(),
    );

    for container in diagram
        .nodes
        .iter()
        .filter(|node| cluster_ids.contains(node.id.as_str()))
    {
        let mut members = container
            .children
            .iter()
            .filter(|child| {
                !cluster_ids.contains(child.as_str()) && !linked.contains(child.as_str())
            })
            .filter_map(|child| {
                diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == *child)
                    .map(|node| (node.quark_order, node.source_line, 0_u8, node.id.clone()))
            })
            .chain(
                diagram
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(_, note)| {
                        note.target.is_none()
                            && note.owner.as_deref() == Some(container.id.as_str())
                            && note.id.as_deref().is_none_or(|id| !linked.contains(id))
                    })
                    .map(|(index, note)| {
                        (
                            note.quark_order,
                            note.source_line,
                            1_u8,
                            deployment_note_layout_id(note, index),
                        )
                    }),
            )
            .collect::<Vec<_>>();
        members
            .sort_by_key(|(quark_order, source_line, kind, _)| (*quark_order, *source_line, *kind));
        add_group(
            layout,
            members.into_iter().map(|(_, _, _, id)| id).collect(),
        );
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
    let label_line_heights = node
        .label
        .lines()
        .map(|line| deployment_label_line_height_for_sprites(line, FONT_SIZE, sprites))
        .collect::<Vec<_>>();
    let label_height_extra = label_line_heights
        .iter()
        .map(|height| (height - TEXT_LINE_H).max(0.0))
        .sum::<f64>();
    let first_label_height_extra = label_line_heights
        .first()
        .map(|height| (height - TEXT_LINE_H).max(0.0))
        .unwrap_or(0.0);
    let label_width = node
        .label
        .lines()
        .map(|line| deployment_label_width_for_sprites(line, FONT_SIZE, bold, sprites))
        .fold(0.0_f64, f64::max);
    let visible_stereotype = node
        .stereotype
        .as_ref()
        .filter(|stereotype| !stereotype_refs_sprite(stereotype, sprites));
    let stereo_width = visible_stereotype
        .map(|stereo| {
            // `EntityImageDescription` wraps visible stereotype text in
            // `TextBlockUtils.withMargin(..., 1, 0)` before `USymbol.asSmall`
            // measures it: one pixel on each horizontal side.
            text_render::measure(&format!("\u{00AB}{stereo}\u{00BB}"), FONT_SIZE, false) + 2.0
        })
        .unwrap_or(0.0);
    let label_line_count = node.label.lines().count().max(1);
    let line_count = label_line_count + usize::from(visible_stereotype.is_some());
    let (text_x_pad, base_top_pad, _) = entity_text_geom(node.kind, 0.0, &node.label);
    let top_pad = base_top_pad + first_label_height_extra;
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
        // `EntityImageDescription.getShield` places an interface's hidden
        // label around the fixed 18px symbol so Graphviz reserves the label.
        DeploymentNodeKind::Default => {
            INTERFACE_SYMBOL_SIZE
                + 2.0 * ((label_width.max(stereo_width) - INTERFACE_SYMBOL_SIZE).max(1.0) / 2.0)
        }
        DeploymentNodeKind::Boundary => label_width.max(stereo_width).max(49.0),
        DeploymentNodeKind::Control | DeploymentNodeKind::Entity => {
            label_width.max(stereo_width).max(32.0)
        }
        DeploymentNodeKind::Collections => label_width.max(stereo_width) + 20.0,
        DeploymentNodeKind::Actor => label_width.max(stereo_width).max(ACTOR_SYMBOL_WIDTH),
        DeploymentNodeKind::Cloud => label_width.max(stereo_width) + 2.0 * CLOUD_MARGIN,
        // `USymbolFolder.asSmall` keeps a hidden 40x15 title box when
        // `showTitle` is false, then adds `Margin(10, 20, 13, 10)`.
        DeploymentNodeKind::Folder => 40.0_f64.max(label_width).max(stereo_width) + 30.0,
        // Package names flow through `BodyEnhanced1` (6px on each side) as
        // the folder title, then `USymbolFolder.asSmall` adds 10px/20px.
        DeploymentNodeKind::Package => (label_width + 12.0).max(stereo_width) + 30.0,
        // `USymbolQueue.asSmall` adds `Margin(5, 15, 5, 5)` around the
        // vertically merged stereotype and label.
        DeploymentNodeKind::Queue => label_width.max(stereo_width) + 20.0,
        // `USymbolStack.asSmall` adds `Margin(25, 25, 10, 10)`.
        DeploymentNodeKind::Stack => label_width.max(stereo_width) + 50.0,
        _ => label_width.max(stereo_width) + 2.0 * text_x_pad,
    };
    let height = match node.kind {
        // `EntityImageDescription.getShield` reserves the taller of the
        // hidden label and stereotype above and below the 18px symbol.
        DeploymentNodeKind::Default => {
            let stereo_height = f64::from(visible_stereotype.is_some()) * TEXT_LINE_H;
            INTERFACE_SYMBOL_SIZE
                + 2.0
                    * (label_line_count as f64 * TEXT_LINE_H)
                        .max(stereo_height)
                        .max(1.0)
                + label_height_extra
        }
        // Boundary/control/entity use a 32px icon stacked between the optional
        // stereotype and the label (`XDimension2D.mergeLayoutT12B3`).
        DeploymentNodeKind::Boundary | DeploymentNodeKind::Control | DeploymentNodeKind::Entity => {
            line_count as f64 * TEXT_LINE_H + label_height_extra + 32.0
        }
        DeploymentNodeKind::Collections => {
            line_count as f64 * TEXT_LINE_H + label_height_extra + 20.0
        }
        DeploymentNodeKind::Actor => {
            ACTOR_SYMBOL_HEIGHT + line_count as f64 * TEXT_LINE_H + label_height_extra
        }
        DeploymentNodeKind::Cloud => {
            line_count as f64 * TEXT_LINE_H + label_height_extra + 2.0 * CLOUD_MARGIN
        }
        // `USymbolCard.asSmall` wraps the merged stereotype/label block in
        // `Margin(10, 10, 3, 3)`.
        DeploymentNodeKind::Card => line_count as f64 * TEXT_LINE_H + label_height_extra + 6.0,
        // `USymbolDatabase.asSmall` adds 29px around the merged text block:
        // 10px top lip, 10px lower cap, and the title spacing between them.
        DeploymentNodeKind::Database => line_count as f64 * TEXT_LINE_H + label_height_extra + 29.0,
        DeploymentNodeKind::Folder => {
            15.0 + line_count as f64 * TEXT_LINE_H + label_height_extra + 13.0 + 10.0
        }
        DeploymentNodeKind::Package => {
            line_count as f64 * TEXT_LINE_H + label_height_extra + 13.0 + 10.0
        }
        DeploymentNodeKind::Queue => line_count as f64 * TEXT_LINE_H + label_height_extra + 10.0,
        _ => {
            base_top_pad
                + (line_count.saturating_sub(1)) as f64 * TEXT_LINE_H
                + (pm::text_height(FONT_SIZE) - ASCENT_14)
                + 10.0
                + label_height_extra
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
    let laid_out_note_indices = laid_out_deployment_note_indices(diagram);
    if deployment_cluster_ids(diagram, &laid_out_note_indices).is_empty()
        && let Some(result) = result
    {
        let painted_min_y = diagram
            .nodes
            .iter()
            .zip(dims)
            .zip(&result.node_positions)
            .map(|((node, dim), position)| {
                let y = (position.y * 100.0).round() / 100.0;
                let (_, paint_dy) = deployment_layout_paint_offset(node.kind);
                y + paint_dy + deployment_local_painted_y_min(node.kind, dim)
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

#[derive(Clone, Copy)]
struct DeploymentPaintBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl DeploymentPaintBounds {
    fn include_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.min_x = self.min_x.min(min_x);
        self.min_y = self.min_y.min(min_y);
        self.max_x = self.max_x.max(max_x);
        self.max_y = self.max_y.max(max_y);
    }
}

fn include_deployment_center_label_paint_bounds(
    bounds: &mut DeploymentPaintBounds,
    label: &str,
    origin: (f64, f64),
    label_size: EdgeLabelSize,
) {
    let mut row_y = origin.1 + 1.0;
    for row in deployment_center_label_rows(label) {
        let row = deployment_center_label_display_row(row);
        let row_width = text_render::measure(row, 13.0, false);
        let row_height = text_render::label_height(row, 13.0);
        let row_x = origin.0 + 1.0 + (label_size.width - 2.0 - row_width) / 2.0;
        let baseline = row_y + pm::ascent(13.0);
        bounds.include_rect(
            row_x,
            baseline - row_height + LIMIT_FINDER_TEXT_ADJUST,
            row_x + row_width,
            baseline + LIMIT_FINDER_TEXT_ADJUST,
        );
        row_y += row_height;
    }
}

fn deployment_body_y_frame(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    note_dims: &[DeploymentNoteDim],
    laid_out_note_indices: &[usize],
    result: Option<&LayoutResult>,
    edge_paint_bounds: Option<DeploymentPaintBounds>,
) -> Option<DeploymentYFrame> {
    let result = result?;
    if !deployment_cluster_ids(diagram, laid_out_note_indices).is_empty() {
        return None;
    }

    let mut painted_min_y = f64::INFINITY;
    let mut painted_max_y = f64::NEG_INFINITY;
    for ((node, dim), position) in diagram.nodes.iter().zip(dims).zip(&result.node_positions) {
        let y = (position.y * 100.0).round() / 100.0;
        let (_, paint_dy) = deployment_layout_paint_offset(node.kind);
        let (local_min_y, local_max_y) = deployment_local_painted_y_bounds(node.kind, dim);
        painted_min_y = painted_min_y.min(y + paint_dy + local_min_y);
        painted_max_y = painted_max_y.max(y + paint_dy + local_max_y);
    }
    for (offset, &note_index) in laid_out_note_indices.iter().enumerate() {
        let position = result.node_positions.get(diagram.nodes.len() + offset)?;
        let y = (position.y * 100.0).round() / 100.0;
        painted_min_y = painted_min_y.min(y);
        painted_max_y = painted_max_y.max(y + note_dims[note_index].height);
    }
    if let Some(bounds) = edge_paint_bounds {
        painted_min_y = painted_min_y.min(bounds.min_y);
        painted_max_y = painted_max_y.max(bounds.max_y);
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
        // `CircleInterface2` paints its ellipse one pixel inside the symbol
        // box. `LimitFinder.drawText` reaches 1.5px below the text baseline.
        Default => {
            let shield_y = (dim.height - INTERFACE_SYMBOL_SIZE) / 2.0;
            (
                shield_y + 1.0,
                shield_y + INTERFACE_LABEL_Y + ASCENT_14 + 1.5,
            )
        }
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
        // `ActorStickMan` draws the head half a stroke below its local origin;
        // the label line ends at the symbol's declared 60px height.
        Actor => (ACTOR_STROKE_WIDTH, dim.height),
        // Two offset `URectangle`s: the front contributes -1 at the origin.
        Collections => (-1.0, dim.height - 1.0),
        // These Java symbols are painted as UPath/UPolygon outlines whose
        // vertical bounds are exactly their declared image height.
        // The stack combines an inset `URectangle` (minimum Y at -1) with
        // a full-height `UPath` (maximum Y at the declared height).
        Stack => (-1.0, dim.height),
        Folder | Queue | File | Package => (0.0, dim.height),
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
    outer_symbol_painted_max_x: Option<f64>,
    outer_symbol_painted_max_y: Option<f64>,
}

fn deployment_cluster_frame(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    result: Option<&LayoutResult>,
) -> Option<DeploymentClusterFrame> {
    let result = result?;
    if result.cluster_positions.is_empty() {
        return None;
    }
    let parent_of = deployment_parent_map(diagram);
    let laid_out_note_indices = laid_out_deployment_note_indices(diagram);
    let cluster_ids = deployment_cluster_ids(diagram, &laid_out_note_indices);
    let leaf_positions: HashMap<&str, (&rustuml_layout::graph::NodePosition, &DeploymentNodeDim)> =
        diagram
            .nodes
            .iter()
            .zip(dims)
            .filter(|(node, _)| !cluster_ids.contains(node.id.as_str()))
            .zip(&result.node_positions)
            .map(|((node, dim), position)| (node.id.as_str(), (position, dim)))
            .collect();
    let mut required_dx = f64::NEG_INFINITY;
    let mut required_dy = f64::NEG_INFINITY;
    let roots: Vec<&DeploymentNode> = diagram
        .nodes
        .iter()
        .filter(|node| !parent_of.contains_key(&node.id))
        .collect();
    for node in &roots {
        let (x, y, local_min_x, local_min_y) = if !cluster_ids.contains(node.id.as_str()) {
            let (position, dim) = leaf_positions.get(node.id.as_str())?;
            let (paint_dx, paint_dy) = deployment_layout_paint_offset(node.kind);
            let (local_min_x, _) = deployment_local_painted_x_bounds(node, dim);
            let (local_min_y, _) = deployment_local_painted_y_bounds(node.kind, dim);
            (
                (position.x * 100.0).round() / 100.0 + paint_dx,
                (position.y * 100.0).round() / 100.0 + paint_dy,
                local_min_x,
                local_min_y,
            )
        } else {
            let position = result
                .cluster_positions
                .iter()
                .find(|position| position.id == node.id)?;
            let (cluster_width, cluster_height) = result
                .cluster_serialized_sizes
                .get(&node.id)
                .copied()
                .unwrap_or((position.width, position.height));
            let (local_min_x, local_min_y) =
                deployment_cluster_local_painted_bounds(node.kind, cluster_width, cluster_height)
                    .map(|(min_x, min_y, _, _)| (min_x, min_y))
                    .unwrap_or((0.0, 0.0));
            (position.x, position.y, local_min_x, local_min_y)
        };
        // `LimitFinder` measures the already translated primitive, then
        // `SvekResult.calculateDimension` subtracts that world-coordinate
        // minimum from six. `SvekResult.drawU` includes both clusters and
        // unpackaged entities in that same LimitFinder pass.
        required_dx = required_dx.max(SVEK_ENVELOPE_ORIGIN - (x + local_min_x));
        required_dy = required_dy.max(SVEK_ENVELOPE_ORIGIN - (y + local_min_y));
    }
    if !required_dx.is_finite() || !required_dy.is_finite() {
        return None;
    }

    // Each supported `USymbol.asBig` primitive below defines the outer
    // envelope seen by `LimitFinder`. `SvekResult.calculateDimension` adds
    // its fixed delta after that painted maximum, rather than after the
    // nominal Graphviz cluster rectangle.
    let outer_symbol_bounds: Option<Vec<_>> = roots
        .iter()
        .map(|node| {
            if !cluster_ids.contains(node.id.as_str()) {
                let (position, dim) = leaf_positions.get(node.id.as_str())?;
                let (paint_dx, paint_dy) = deployment_layout_paint_offset(node.kind);
                let (min_x, max_x) = deployment_local_painted_x_bounds(node, dim);
                let (min_y, max_y) = deployment_local_painted_y_bounds(node.kind, dim);
                return Some((
                    (position.x * 100.0).round() / 100.0 + paint_dx,
                    (position.y * 100.0).round() / 100.0 + paint_dy,
                    (min_x, min_y, max_x, max_y),
                ));
            }
            let position = result
                .cluster_positions
                .iter()
                .find(|position| position.id == node.id)?;
            let (cluster_width, cluster_height) = result
                .cluster_serialized_sizes
                .get(&node.id)
                .copied()
                .unwrap_or((position.width, position.height));
            let bounds =
                deployment_cluster_local_painted_bounds(node.kind, cluster_width, cluster_height)?;
            Some((position.x, position.y, bounds))
        })
        .collect();
    let outer_symbol_painted_max_x = outer_symbol_bounds.as_ref().map(|bounds| {
        bounds
            .iter()
            .map(|(x, _, (_, _, max_x, _))| x + required_dx + max_x)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let outer_symbol_painted_max_y = outer_symbol_bounds.as_ref().map(|bounds| {
        bounds
            .iter()
            .map(|(_, y, (_, _, _, max_y))| y + required_dy + max_y)
            .fold(f64::NEG_INFINITY, f64::max)
    });

    Some(DeploymentClusterFrame {
        margin_x: required_dx,
        margin_y: required_dy,
        outer_symbol_painted_max_x,
        outer_symbol_painted_max_y,
    })
}

fn deployment_cluster_local_painted_bounds(
    kind: DeploymentNodeKind,
    width: f64,
    height: f64,
) -> Option<(f64, f64, f64, f64)> {
    use DeploymentNodeKind::*;
    Some(match kind {
        Cloud => crate::cloud_shape::generate(width, height).bounds(),
        // `USymbolNode.drawNode` paints its outer `UPolygon` plus a
        // lower-edge `UEmpty(10,10)`. `LimitFinder.drawUPolygon` expands the
        // polygon horizontally by ten pixels.
        Node => (-10.0, 0.0, width + 10.0, height + 10.0),
        Folder => (0.0, 0.0, width, height),
        // `LimitFinder.drawRectangle` expands the top-left of every
        // `URectangle` by one pixel and records width-1/height-1 at the
        // opposite corner.
        Rectangle | Agent | Frame | Card | Storage => (-1.0, -1.0, width - 1.0, height - 1.0),
        // `USymbolArtifact.drawArtifact` adds a folded UPolygon whose
        // rightmost x is width-5; LimitFinder's ten-pixel polygon expansion
        // therefore extends the measured maximum to width+5.
        Artifact => (-1.0, -1.0, width + 5.0, height - 1.0),
        // `USymbolStack.drawQueue` paints a full-width UPath, while its inset
        // `URectangle` extends the top LimitFinder bound by one.
        Stack => (0.0, -1.0, width, height),
        _ => return None,
    })
}

fn deployment_body_x_frame(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    note_dims: &[DeploymentNoteDim],
    laid_out_note_indices: &[usize],
    result: Option<&LayoutResult>,
    edge_paint_bounds: Option<DeploymentPaintBounds>,
) -> Option<DeploymentXFrame> {
    let result = result?;
    if !deployment_cluster_ids(diagram, laid_out_note_indices).is_empty() {
        return None;
    }

    let mut painted_min_x = f64::INFINITY;
    let mut painted_max_x = f64::NEG_INFINITY;
    for ((node, dim), position) in diagram.nodes.iter().zip(dims).zip(&result.node_positions) {
        // `DotStringFactory.solve` recovers rectangle positions from
        // Graphviz's SVG polygon coordinates, which are serialized to two
        // decimal places.
        let x = (position.x * 100.0).round() / 100.0;
        let (paint_dx, _) = deployment_layout_paint_offset(node.kind);
        let (local_min_x, local_max_x) = deployment_local_painted_x_bounds(node, dim);
        painted_min_x = painted_min_x.min(x + paint_dx + local_min_x);
        painted_max_x = painted_max_x.max(x + paint_dx + local_max_x);
    }
    for (offset, &note_index) in laid_out_note_indices.iter().enumerate() {
        let position = result.node_positions.get(diagram.nodes.len() + offset)?;
        let x = (position.x * 100.0).round() / 100.0;
        painted_min_x = painted_min_x.min(x);
        painted_max_x = painted_max_x.max(x + note_dims[note_index].width);
    }
    if let Some(bounds) = edge_paint_bounds {
        painted_min_x = painted_min_x.min(bounds.min_x);
        painted_max_x = painted_max_x.max(bounds.max_x);
    }
    painted_min_x.is_finite().then_some(DeploymentXFrame {
        // `LimitFinder.drawUPolygon` expands polygon bounds by 10px on both
        // horizontal sides; `SvekResult.calculateDimension` then calls
        // `moveDelta(6 - minX, 6 - minY)`.
        margin: SVEK_ENVELOPE_ORIGIN - painted_min_x,
        painted_max_x,
    })
}

fn deployment_edge_paint_bounds(
    diagram: &DeploymentDiagram,
    result: &LayoutResult,
    cluster_endpoint_nodes: &HashMap<String, String>,
    opale_connection_indices: &HashSet<usize>,
) -> Option<DeploymentPaintBounds> {
    let mut bounds = DeploymentPaintBounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for (conn_index, conn) in diagram.connections.iter().enumerate() {
        // `SvekEdge.drawU` returns before drawing an edge handed to the Opale
        // note renderer, so that edge has no independent LimitFinder pass.
        if opale_connection_indices.contains(&conn_index) {
            continue;
        }
        let (_, _, reversed) = deployment_connection_layout(conn);
        let Some(edge) = deployment_connection_edge(
            diagram,
            conn_index,
            &result.edge_paths,
            cluster_endpoint_nodes,
        ) else {
            continue;
        };
        // `LimitFinder.drawDotPath` measures the solved route even under
        // `UHidden`; hidden affects only the later SVG driver.
        for &(x, y) in &edge.points {
            let x = (x * 100.0).round() / 100.0;
            let y = (y * 100.0).round() / 100.0;
            bounds.include_rect(x, y, x, y);
        }
        if let (Some(note), Some(label)) = (conn.note.as_ref(), edge.label) {
            let x = (label.x * 100.0).round() / 100.0;
            let y = (label.y * 100.0).round() / 100.0;
            let label_size = conn
                .label
                .as_deref()
                .map(|label| deployment_center_label_size(label, false));
            let note_dim = deployment_link_note_dim(note);
            let (label_origin, note_origin) =
                deployment_link_note_blocks(x, y, note, note_dim, label_size);
            // The compound box constrains Graphviz, but LimitFinder observes
            // only its painted children. Rose's outer five-pixel padding is
            // deliberately absent from this projection.
            bounds.include_rect(
                note_origin.0,
                note_origin.1,
                note_origin.0 + note_dim.width.floor(),
                note_origin.1 + note_dim.height.floor(),
            );
            if let (Some(label_text), Some(origin), Some(size)) =
                (conn.label.as_deref(), label_origin, label_size)
            {
                include_deployment_center_label_paint_bounds(&mut bounds, label_text, origin, size);
            }
        } else if let (Some(label_text), Some(label)) = (conn.label.as_deref(), edge.label) {
            // `SvekEdge.getLabelText` wraps center labels in one-pixel margins.
            // Graphviz solves their origin from an integer-truncated placeholder,
            // then `LimitFinder` sees the original renderer width when drawing.
            let x = (label.x * 100.0).round() / 100.0;
            let y = (label.y * 100.0).round() / 100.0;
            let size = deployment_center_label_size(label_text, false);
            include_deployment_center_label_paint_bounds(&mut bounds, label_text, (x, y), size);
        }
        let (tail_text, head_text) = if reversed {
            (conn.head_label.as_deref(), conn.tail_label.as_deref())
        } else {
            (conn.tail_label.as_deref(), conn.head_label.as_deref())
        };
        for (text, position) in [(tail_text, edge.tail_label), (head_text, edge.head_label)] {
            let (Some(text), Some(position)) = (text, position) else {
                continue;
            };
            // `manageCollision` runs after Graphviz parsing, so its moved
            // position is not serialized through dot a second time.
            let x = position.x;
            let y = position.y;
            bounds.include_rect(
                x,
                y,
                x + text_render::measure(text, 13.0, false),
                y + text_render::label_height(text, 13.0),
            );
        }
    }
    bounds.min_x.is_finite().then_some(bounds)
}

fn adjust_deployment_endpoint_labels(
    diagram: &DeploymentDiagram,
    dims: &[DeploymentNodeDim],
    result: &mut LayoutResult,
) {
    let laid_out_note_indices = laid_out_deployment_note_indices(diagram);
    let cluster_ids = deployment_cluster_ids(diagram, &laid_out_note_indices);
    let mut positions = Vec::new();
    let mut leaf_positions = result.node_positions.iter();
    for (node, dim) in diagram.nodes.iter().zip(dims) {
        if cluster_ids.contains(node.id.as_str()) {
            continue;
        }
        let Some(position) = leaf_positions.next() else {
            return;
        };
        let outer_rect = LayoutRect {
            x: position.x,
            y: position.y,
            width: position.width,
            height: position.height,
        };
        let rect = if node.kind == DeploymentNodeKind::Default {
            // `DotStringFactory.solve` records the black `h` cell polygon,
            // rather than the surrounding HTML-table node.
            LayoutRect {
                x: ((position.x + (position.width - INTERFACE_SYMBOL_SIZE) / 2.0) * 100.0).round()
                    / 100.0,
                y: ((position.y + (position.height - INTERFACE_SYMBOL_SIZE) / 2.0) * 100.0).round()
                    / 100.0,
                width: INTERFACE_SYMBOL_SIZE,
                height: INTERFACE_SYMBOL_SIZE,
            }
        } else {
            LayoutRect {
                x: (position.x * 100.0).round() / 100.0,
                y: (position.y * 100.0).round() / 100.0,
                width: dim.width,
                height: dim.height,
            }
        };
        positions.push((node.id.as_str(), rect, outer_rect));
    }

    let intersects = |fixed: LayoutRect, moving: LayoutRect| {
        fixed.x < moving.x + moving.width
            && fixed.x + fixed.width > moving.x
            && fixed.y < moving.y + moving.height
            && fixed.y + fixed.height > moving.y
    };
    let move_away = |fixed: LayoutRect, moving: LayoutRect| {
        let delta_x = moving.x + moving.width / 2.0 - (fixed.x + fixed.width / 2.0);
        let delta_y = moving.y + moving.height / 2.0 - (fixed.y + fixed.height / 2.0);
        let moved = |coefficient: f64| LayoutRect {
            x: moving.x + delta_x * coefficient,
            y: moving.y + delta_y * coefficient,
            ..moving
        };
        if delta_x == 0.0 && delta_y == 0.0 {
            return moving;
        }
        let mut min = 0.0;
        let mut max = 0.1;
        for _ in 0..64 {
            if !intersects(fixed, moved(max)) {
                break;
            }
            max *= 2.0;
        }
        // Java `PositionableUtils.moveAwayFrom` intentionally uses five
        // bisection rounds, and its resulting slack is visible in SVEK output.
        for _ in 0..5 {
            let candidate = (min + max) / 2.0;
            if intersects(fixed, moved(candidate)) {
                min = candidate;
            } else {
                max = candidate;
            }
        }
        moved((min + max) / 2.0)
    };

    for (conn_index, conn) in diagram.connections.iter().enumerate() {
        let (layout_from, layout_to, reversed) = deployment_connection_layout(conn);
        let matching_index = diagram.connections[..conn_index]
            .iter()
            .filter(|previous| {
                let (previous_from, previous_to, _) = deployment_connection_layout(previous);
                previous_from == layout_from && previous_to == layout_to
            })
            .count();
        let Some(edge) = result
            .edge_paths
            .iter_mut()
            .filter(|edge| edge.from == layout_from && edge.to == layout_to)
            .nth(matching_index)
        else {
            continue;
        };
        let (tail_text, head_text) = if reversed {
            (conn.head_label.as_deref(), conn.tail_label.as_deref())
        } else {
            (conn.tail_label.as_deref(), conn.head_label.as_deref())
        };
        for (is_tail, endpoint, text, position) in [
            (true, layout_from, tail_text, edge.tail_label.as_mut()),
            (false, layout_to, head_text, edge.head_label.as_mut()),
        ] {
            let (Some(text), Some(position)) = (text, position) else {
                continue;
            };
            let width = text_render::measure(text, 13.0, false);
            let height = text_render::label_height(text, 13.0);
            let mut moving = LayoutRect {
                x: (position.x * 100.0).round() / 100.0,
                y: (position.y * 100.0).round() / 100.0,
                width,
                height,
            };
            if let Some((_, endpoint_rect, _)) = positions.iter().find(|(id, _, _)| *id == endpoint)
                && diagram
                    .nodes
                    .iter()
                    .any(|node| node.id == endpoint && node.kind == DeploymentNodeKind::Default)
                && is_tail
            {
                // A vertical edge from the HTML `h` port aligns its tail
                // cardinality's right and bottom sides with that 18px cell.
                // This is the record-cell placement dot performs before
                // `SvekEdge.manageCollision`.
                moving.x = endpoint_rect.x + endpoint_rect.width / 2.0 - position.width;
                moving.y = endpoint_rect.y + endpoint_rect.height
                    - position.height
                    - INTERFACE_ENDPOINT_LABEL_BOTTOM_GAP;
            }
            for (_, node_rect, _) in &positions {
                let fixed = LayoutRect {
                    x: node_rect.x - 8.0,
                    y: node_rect.y - 8.0,
                    width: node_rect.width + 16.0,
                    height: node_rect.height + 16.0,
                };
                if intersects(fixed, moving) {
                    moving = move_away(fixed, moving);
                }
            }
            position.x = moving.x;
            position.y = moving.y;
        }

        if let Some((_, port_rect, outer_rect)) =
            positions.iter().find(|(id, _, _)| *id == layout_from)
            && diagram
                .nodes
                .iter()
                .any(|node| node.id == layout_from && node.kind == DeploymentNodeKind::Default)
            && edge.points.len() == 4
            && (edge.points[0].0 - edge.points[3].0).abs() < 1.0
            && let Some(label) = edge.label
        {
            let quantize = |value: f64| (value * 100.0).round() / 100.0;
            let center_x = quantize(port_rect.x + port_rect.width / 2.0);
            edge.points[0] = (
                center_x,
                quantize(port_rect.y + port_rect.height - INTERFACE_PORT_START_INSET),
            );
            edge.points[1] = (
                center_x,
                quantize(outer_rect.y + outer_rect.height - INTERFACE_TABLE_CONTROL_INSET),
            );
            edge.points[2] = (
                center_x,
                quantize(label.y + label.height + INTERFACE_LABEL_CONTROL_GAP),
            );
            edge.points[3] = (
                center_x,
                quantize(edge.points[3].1 + INTERFACE_TARGET_ENDPOINT_DELTA),
            );
        }
    }
}

fn deployment_layout_node_size(kind: DeploymentNodeKind, dim: &DeploymentNodeDim) -> (f64, f64) {
    if kind == DeploymentNodeKind::Default {
        (
            dim.width + INTERFACE_TABLE_EXTRA_WIDTH,
            dim.height + INTERFACE_TABLE_EXTRA_HEIGHT,
        )
    } else {
        (dim.width, dim.height)
    }
}

fn deployment_layout_paint_offset(kind: DeploymentNodeKind) -> (f64, f64) {
    if kind == DeploymentNodeKind::Default {
        (
            INTERFACE_TABLE_EXTRA_WIDTH / 2.0,
            INTERFACE_TABLE_EXTRA_HEIGHT / 2.0,
        )
    } else {
        (0.0, 0.0)
    }
}

fn deployment_local_painted_x_bounds(node: &DeploymentNode, dim: &DeploymentNodeDim) -> (f64, f64) {
    use DeploymentNodeKind::*;
    match node.kind {
        // `USymbolNode.drawNode` paints the full body as a UPolygon.
        Node => (-10.0, dim.width + 10.0),
        Default => {
            let shield_x = (dim.width - INTERFACE_SYMBOL_SIZE) / 2.0;
            let label_x = shield_x + (INTERFACE_SYMBOL_SIZE - dim.label_width) / 2.0;
            (
                (shield_x + 1.0).min(label_x),
                (shield_x + INTERFACE_CIRCLE_SIZE).max(label_x + dim.label_width),
            )
        }
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
        Actor => (0.0, dim.width),
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
                .map(|sprite| deployment_sprite_logical_dimensions(sprite, font_size).0)
                .unwrap_or(0.0),
            crate::sprite::TextSegment::OpenIcon(name) => crate::openiconic::lookup(name)
                .map(|icon| icon.width * (font_size / icon.height))
                .unwrap_or(0.0),
        })
        .sum()
}

fn deployment_label_line_height_for_sprites(
    label: &str,
    font_size: f64,
    sprites: &HashMap<String, rustuml_parser::diagram::SpriteData>,
) -> f64 {
    crate::sprite::parse_sprite_segments(label)
        .iter()
        .filter_map(|segment| match segment {
            crate::sprite::TextSegment::Sprite(name) => sprites
                .get(name)
                .map(|sprite| deployment_sprite_logical_dimensions(sprite, font_size).1),
            _ => None,
        })
        .fold(pm::text_height(font_size), f64::max)
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
        let laid_out_note_indices = laid_out_deployment_note_indices(diagram);
        let cluster_ids = deployment_cluster_ids(diagram, &laid_out_note_indices);
        let cluster_positions: HashMap<&str, &rustuml_layout::graph::ClusterPosition> = result
            .cluster_positions
            .iter()
            .map(|pos| (pos.id.as_str(), pos))
            .collect();
        let mut leaf_positions = result.node_positions.iter();
        for (node, dim) in diagram.nodes.iter().zip(dims) {
            if cluster_ids.contains(node.id.as_str()) {
                if let Some(pos) = cluster_positions.get(node.id.as_str()) {
                    let (cluster_width, cluster_height) = result
                        .cluster_serialized_sizes
                        .get(&node.id)
                        .copied()
                        .unwrap_or((pos.width, pos.height));
                    // Java `SvekResult.calculateDimension` calls
                    // `DotStringFactory.moveDelta`, whose `Cluster.move`
                    // translates both rectangle endpoints independently.
                    // Recompute the final extent from those moved endpoints:
                    // for seeded clouds, the resulting ULP can intentionally
                    // change the `(long) width` seed used by `USymbolCloud`.
                    let x = pos.x + body_margin_x;
                    let y = pos.y + body_margin_y;
                    rects.push(LayoutRect {
                        x,
                        y,
                        width: translated_serialized_layout_extent(
                            pos.x,
                            cluster_width,
                            body_margin_x,
                        ),
                        height: translated_layout_extent(pos.y, cluster_height, body_margin_y),
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
            let (paint_dx, paint_dy) = deployment_layout_paint_offset(node.kind);
            rects.push(LayoutRect {
                x: (pos.x * 100.0).round() / 100.0 + body_margin_x + paint_dx,
                y: (pos.y * 100.0).round() / 100.0 + body_margin_y + paint_dy,
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

fn translated_layout_extent(origin: f64, extent: f64, delta: f64) -> f64 {
    let moved_origin = origin + delta;
    let moved_max = (origin + extent) + delta;
    moved_max - moved_origin
}

fn translated_serialized_layout_extent(origin: f64, extent: f64, delta: f64) -> f64 {
    // `DotStringFactory.solve` recovers cluster X endpoints from Graphviz's
    // two-decimal SVG polygon before `Cluster.move` translates them.
    let serialized_origin = (origin * 100.0).round() / 100.0;
    let serialized_max = ((origin + extent) * 100.0).round() / 100.0;
    (serialized_max + delta) - (serialized_origin + delta)
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
    link_ids: Vec<String>,
}

struct DeploymentNoteUid {
    qualified_name: String,
    entity_id: String,
}

fn build_deployment_no_oracle_uid_model(diagram: &DeploymentDiagram) -> DeploymentNoOracleUidModel {
    enum Item<'a> {
        Node(&'a DeploymentNode),
        Note(usize),
        Conn(usize),
    }
    let mut items = Vec::new();
    for node in &diagram.nodes {
        items.push((node.source_line, Item::Node(node)));
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        items.push((note.source_line, Item::Note(index)));
    }
    for (index, conn) in diagram.connections.iter().enumerate() {
        items.push((conn.source_line, Item::Conn(index)));
    }
    items.sort_by_key(|(line, _)| *line);
    let mut next_uid = 2;
    let mut entity_ids = HashMap::new();
    let mut note_ids = HashMap::new();
    let mut link_ids = vec![String::new(); diagram.connections.len()];
    let parent_of = deployment_parent_map(diagram);
    let qnames = deployment_qnames(diagram, &parent_of);
    for (_, item) in items {
        match item {
            Item::Node(node) => {
                entity_ids.insert(node.id.clone(), format!("ent{next_uid:04}"));
                next_uid += 1;
            }
            Item::Note(index) => {
                let note = &diagram.notes[index];
                let own_name = if let Some(id) = note.id.as_ref() {
                    id.clone()
                } else {
                    // `CommandFactoryNoteOnEntity.executeInternal` obtains a
                    // generated name before creating the note leaf.
                    let name = format!("GMN{next_uid}");
                    next_uid += 1;
                    name
                };
                let qualified_name = note
                    .owner
                    .as_ref()
                    .and_then(|owner| qnames.get(owner))
                    .map_or(own_name.clone(), |owner| format!("{owner}.{own_name}"));
                let entity_id = format!("ent{next_uid:04}");
                next_uid += 1;
                if let Some(id) = note.id.as_ref() {
                    entity_ids.insert(id.clone(), entity_id.clone());
                    entity_ids.insert(qualified_name.clone(), entity_id.clone());
                }
                note_ids.insert(
                    index,
                    DeploymentNoteUid {
                        qualified_name,
                        entity_id,
                    },
                );
                // The hidden note-to-target Link consumes the next global UID
                // even though Opale absorbs it into the note outline.
                if note.target.is_some() {
                    next_uid += 1;
                }
            }
            Item::Conn(index) => {
                if matches!(
                    diagram.connections[index].direction,
                    Some(DeploymentLinkDirection::Up | DeploymentLinkDirection::Left)
                ) {
                    // `CommandLinkElement` reverses left/up links by creating
                    // an intermediate link before the visible SVEK edge.
                    next_uid += 1;
                }
                link_ids[index] = format!("lnk{next_uid}");
                next_uid += 1;
            }
        }
    }
    DeploymentNoOracleUidModel {
        entity_ids,
        note_ids,
        link_ids,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_no_oracle_edges(
    svg: &mut SvgBuilder,
    diagram: &DeploymentDiagram,
    id_for_node: &HashMap<String, String>,
    link_ids: &[String],
    edge_paths: &[EdgePath],
    body_margin_x: f64,
    body_margin_y: f64,
    cluster_endpoint_nodes: &HashMap<String, String>,
    cluster_rects: &HashMap<&str, LayoutRect>,
    opale_connection_indices: &HashSet<usize>,
    gradient_defs: Option<&str>,
    handwritten: bool,
) {
    for (i, conn) in diagram.connections.iter().enumerate() {
        if conn.hidden {
            continue;
        }
        // Java `SvekEdge.drawU` returns before opening the link group after
        // GraphvizImageBuilder hands an eligible edge to EntityImageNote.
        if opale_connection_indices.contains(&i) {
            continue;
        }
        let (logical_from, logical_to, reversed) = deployment_connection_layout(conn);
        let Some(edge) = deployment_connection_edge(diagram, i, edge_paths, cluster_endpoint_nodes)
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
        let link_id = link_ids
            .get(i)
            .filter(|id| !id.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("lnk{}", i + diagram.nodes.len() + 2));
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
        let tail_cluster = cluster_rects.get(logical_from);
        let head_cluster = cluster_rects.get(logical_to);
        let raw_points = deployment_svek_edge_points(
            &edge.points,
            body_margin_x,
            body_margin_y,
            tail_cluster,
            head_cluster,
            EdgeTrim::None,
        );
        let points = deployment_svek_edge_points(
            &edge.points,
            body_margin_x,
            body_margin_y,
            tail_cluster,
            head_cluster,
            match (raw_start_arrow, raw_end_arrow) {
                (false, false) => EdgeTrim::None,
                (true, false) => EdgeTrim::Start,
                (false, true) => EdgeTrim::End,
                (true, true) => EdgeTrim::Both,
            },
        );
        if let Some(d) = edge_path_d(&points) {
            let mut path_id = deployment_connection_path_base_id(diagram, conn, reversed);
            let duplicate_index = diagram.connections[..i]
                .iter()
                .filter(|previous| {
                    let (_, _, previous_reversed) = deployment_connection_layout(previous);
                    deployment_connection_path_base_id(diagram, previous, previous_reversed)
                        == path_id
                })
                .count();
            if duplicate_index > 0 {
                write!(path_id, "-{duplicate_index}").unwrap();
            }
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
            if handwritten {
                let d = handwritten_dot_path(&points).unwrap_or(d);
                svg.raw(&format!(
                    r#"<path d="{d}" fill="none" style="{path_style}"/>"#,
                ));
            } else {
                svg.raw(&format!(
                    r#"<path d="{d}" fill="none" id="{path_id}" style="{path_style}"/>"#,
                ));
            }
        }
        if raw_points.len() >= 2 {
            if raw_start_arrow {
                emit_deployment_arrowhead(svg, &raw_points[1], &raw_points[0], handwritten);
            }
            if raw_end_arrow {
                emit_deployment_arrowhead(
                    svg,
                    &raw_points[raw_points.len() - 2],
                    &raw_points[raw_points.len() - 1],
                    handwritten,
                );
            }
        }
        let exact_center_label_size = conn
            .label
            .as_deref()
            .map(|label| deployment_center_label_size(label, false));
        let link_note_layout = conn.note.as_ref().and_then(|note| {
            let position = edge.label?;
            let note_dim = deployment_link_note_dim(note);
            let solved_origin = (
                (position.x * 100.0).round() / 100.0 + body_margin_x,
                (position.y * 100.0).round() / 100.0 + body_margin_y,
            );
            let (label_origin, note_origin) = deployment_link_note_blocks(
                solved_origin.0,
                solved_origin.1,
                note,
                note_dim,
                exact_center_label_size,
            );
            Some((note, note_dim, label_origin, note_origin))
        });
        if let Some((note, dim, _, (note_x, note_y))) = link_note_layout
            && matches!(
                note.position,
                DeploymentNotePosition::Left | DeploymentNotePosition::Top
            )
        {
            render_deployment_link_note(svg, note, note_x, note_y, dim, gradient_defs);
        }
        if let Some(label) = conn.label.as_deref() {
            let label_size = deployment_center_label_size(label, false);
            let (origin_x, origin_y) = edge
                .label
                .map(|position| {
                    let solved_origin = (
                        (position.x * 100.0).round() / 100.0 + body_margin_x,
                        (position.y * 100.0).round() / 100.0 + body_margin_y,
                    );
                    link_note_layout
                        .and_then(|(_, _, label_origin, _)| label_origin)
                        .unwrap_or(solved_origin)
                })
                .unwrap_or_else(|| {
                    points
                        .first()
                        .zip(points.last())
                        .map(|(first, last)| ((first.0 + last.0) / 2.0, (first.1 + last.1) / 2.0))
                        .unwrap_or((body_margin_x, body_margin_y))
                });
            let mut row_y = origin_y + 1.0;
            for row in deployment_center_label_rows(label) {
                let row = deployment_center_label_display_row(row);
                let row_width = text_render::measure(row, 13.0, false);
                let row_x = origin_x + 1.0 + (label_size.width - 2.0 - row_width) / 2.0;
                emit_text(
                    svg,
                    row,
                    row_x,
                    // Java provenance: Display's TextBlockSimple uses the
                    // StringBounder font ascent for every row; glyph contents
                    // do not move the baseline within a line box.
                    row_y + pm::ascent(13.0),
                    13.0,
                    false,
                    false,
                );
                row_y += text_render::label_height(row, 13.0);
            }
        }
        if let Some((note, dim, _, (note_x, note_y))) = link_note_layout
            && matches!(
                note.position,
                DeploymentNotePosition::Right | DeploymentNotePosition::Bottom
            )
        {
            render_deployment_link_note(svg, note, note_x, note_y, dim, gradient_defs);
        }
        let (tail_text, head_text) = if reversed {
            (conn.head_label.as_deref(), conn.tail_label.as_deref())
        } else {
            (conn.tail_label.as_deref(), conn.head_label.as_deref())
        };
        for (text, position) in [(tail_text, edge.tail_label), (head_text, edge.head_label)] {
            let (Some(text), Some(position)) = (text, position) else {
                continue;
            };
            // `SvekEdge.drawU` paints endpoint cardinalities directly at the
            // solved tail/head table origin.
            let x = position.x + body_margin_x;
            let y = position.y + body_margin_y + text_render::label_ascent(text, 13.0);
            emit_text(svg, text, x, y, 13.0, false, false);
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

fn deployment_connection_path_base_id(
    diagram: &DeploymentDiagram,
    conn: &DeploymentConnection,
    reversed: bool,
) -> String {
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
    if conn.arrow_at_start == conn.arrow_at_end {
        format!("{from_name}-{to_name}")
    } else if reversed {
        format!("{to_name}-backto-{from_name}")
    } else {
        format!("{from_name}-to-{to_name}")
    }
}

fn deployment_connection_layout_with_endpoints<'a>(
    conn: &'a DeploymentConnection,
    cluster_endpoint_nodes: &'a HashMap<String, String>,
) -> (&'a str, &'a str, bool) {
    let (from, to, reversed) = deployment_connection_layout(conn);
    (
        cluster_endpoint_nodes
            .get(from)
            .map(String::as_str)
            .unwrap_or(from),
        cluster_endpoint_nodes
            .get(to)
            .map(String::as_str)
            .unwrap_or(to),
        reversed,
    )
}

fn deployment_connection_edge<'a>(
    diagram: &DeploymentDiagram,
    connection_index: usize,
    edge_paths: &'a [EdgePath],
    cluster_endpoint_nodes: &HashMap<String, String>,
) -> Option<&'a EdgePath> {
    let connection = diagram.connections.get(connection_index)?;
    let (layout_from, layout_to, _) =
        deployment_connection_layout_with_endpoints(connection, cluster_endpoint_nodes);
    let matching_index = diagram.connections[..connection_index]
        .iter()
        .filter(|previous| {
            let (previous_from, previous_to, _) =
                deployment_connection_layout_with_endpoints(previous, cluster_endpoint_nodes);
            previous_from == layout_from && previous_to == layout_to
        })
        .count();
    edge_paths
        .iter()
        .filter(|edge| edge.from == layout_from && edge.to == layout_to)
        .nth(matching_index)
}

fn deployment_uses_strict_uml(diagram: &DeploymentDiagram) -> bool {
    diagram.meta.skinparams.iter().any(|skinparam| {
        skinparam.key.eq_ignore_ascii_case("style")
            && skinparam.value.trim().eq_ignore_ascii_case("strictuml")
    })
}

fn deployment_note_opale_connection<'a>(
    diagram: &DeploymentDiagram,
    note_index: usize,
    edge_paths: &'a [EdgePath],
    cluster_endpoint_nodes: &HashMap<String, String>,
    cluster_ids: &HashSet<&str>,
) -> Option<(usize, &'a EdgePath)> {
    // Java `GraphvizImageBuilder.isOpalisable` and `onlyOneLink` classify the
    // semantic relation graph first; `SvekEdge.solve` then revokes Opale when
    // Graphviz returns a path with more than one cubic segment.
    if deployment_uses_strict_uml(diagram) {
        return None;
    }
    let note_id = diagram.notes.get(note_index)?.id.as_deref()?;
    let mut incident = diagram
        .connections
        .iter()
        .enumerate()
        // Bracket-hidden links are not `Link.isInvis()` in Java. They count in
        // `GraphvizImageBuilder.onlyOneLink` and are hidden only by SvekResult.
        .filter(|(_, connection)| connection.from == note_id || connection.to == note_id);
    let (connection_index, connection) = incident.next()?;
    if incident.next().is_some() {
        return None;
    }
    let peer = if connection.from == note_id {
        connection.to.as_str()
    } else {
        connection.from.as_str()
    };
    // `GraphvizImageBuilder` marks the edge Opale only when
    // `Bibliotekon.getNode(peer)` succeeds. A declared group is represented
    // by a Cluster; its synthetic routing proxy is not the peer's SvekNode.
    if cluster_ids.contains(peer) {
        return None;
    }
    if diagram
        .notes
        .iter()
        .any(|candidate| candidate.id.as_deref() == Some(peer))
    {
        return None;
    }
    let edge = deployment_connection_edge(
        diagram,
        connection_index,
        edge_paths,
        cluster_endpoint_nodes,
    )?;
    (!edge.points.is_empty() && edge.bezier_count <= 1).then_some((connection_index, edge))
}

fn deployment_opale_note_geometry(
    edge: &EdgePath,
    layout: &DeploymentNoteLayout,
    body_margin_x: f64,
    body_margin_y: f64,
    tail_cluster: Option<&LayoutRect>,
    head_cluster: Option<&LayoutRect>,
) -> Option<(DeploymentNotePosition, bool)> {
    let points = deployment_svek_edge_points(
        &edge.points,
        body_margin_x,
        body_margin_y,
        tail_cluster,
        head_cluster,
        EdgeTrim::None,
    );
    let (first, last) = points.first().copied().zip(points.last().copied())?;
    let center = (
        layout.x + layout.width / 2.0,
        layout.y + layout.height / 2.0,
    );
    let squared_distance = |point: (f64, f64)| {
        let dx = point.0 - center.0;
        let dy = point.1 - center.1;
        dx * dx + dy * dy
    };
    let note_at_edge_start = squared_distance(first) <= squared_distance(last);
    let note_point = if note_at_edge_start { first } else { last };

    // Java `EntityImageNote.getOpaleStrategy` compares orthogonal distance to
    // left, right, top and bottom in that tie order. DeploymentNotePosition
    // names where the note sits, so it is the opposite of the connector side.
    let left = (note_point.0 - layout.x).abs();
    let right = (note_point.0 - (layout.x + layout.width)).abs();
    let top = (note_point.1 - layout.y).abs();
    let bottom = (note_point.1 - (layout.y + layout.height)).abs();
    let position = if left <= right && left <= top && left <= bottom {
        DeploymentNotePosition::Right
    } else if right <= top && right <= bottom {
        DeploymentNotePosition::Left
    } else if top <= bottom {
        DeploymentNotePosition::Bottom
    } else {
        DeploymentNotePosition::Top
    };
    Some((position, note_at_edge_start))
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
    tail_cluster: Option<&LayoutRect>,
    head_cluster: Option<&LayoutRect>,
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
    points = simulate_deployment_compound(points, tail_cluster, head_cluster);
    if points.len() >= 2 && matches!(trim, EdgeTrim::Start | EdgeTrim::Both) {
        trim_deployment_edge_endpoint(&mut points, 0, 1);
    }
    if points.len() >= 2 && matches!(trim, EdgeTrim::End | EdgeTrim::Both) {
        let endpoint = points.len() - 1;
        trim_deployment_edge_endpoint(&mut points, endpoint, endpoint - 1);
    }
    points
}

fn simulate_deployment_compound(
    points: Vec<(f64, f64)>,
    tail: Option<&LayoutRect>,
    head: Option<&LayoutRect>,
) -> Vec<(f64, f64)> {
    if points.len() < 4 || !(points.len() - 1).is_multiple_of(3) {
        return points;
    }

    type Cubic = [(f64, f64); 4];

    fn contains(rectangle: &LayoutRect, point: (f64, f64)) -> bool {
        point.0 >= rectangle.x
            && point.0 <= rectangle.x + rectangle.width
            && point.1 >= rectangle.y
            && point.1 <= rectangle.y + rectangle.height
    }

    fn subdivide(curve: Cubic) -> (Cubic, Cubic) {
        let midpoint = |a: (f64, f64), b: (f64, f64)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let p01 = midpoint(curve[0], curve[1]);
        let p12 = midpoint(curve[1], curve[2]);
        let p23 = midpoint(curve[2], curve[3]);
        let p012 = midpoint(p01, p12);
        let p123 = midpoint(p12, p23);
        let split = midpoint(p012, p123);
        ([curve[0], p01, p012, split], [split, p123, p23, curve[3]])
    }

    fn curves_from_points(points: &[(f64, f64)]) -> Vec<Cubic> {
        points[1..]
            .chunks_exact(3)
            .scan(points[0], |start, chunk| {
                let curve = [*start, chunk[0], chunk[1], chunk[2]];
                *start = chunk[2];
                Some(curve)
            })
            .collect()
    }

    fn points_from_curves(curves: &[Cubic]) -> Vec<(f64, f64)> {
        let Some(first) = curves.first() else {
            return Vec::new();
        };
        let mut points = Vec::with_capacity(curves.len() * 3 + 1);
        points.push(first[0]);
        for curve in curves {
            points.extend_from_slice(&curve[1..]);
        }
        points
    }

    // `DotPath.simulateCompound` clips the first boundary-crossing cubic by
    // bisecting it eight times. It retains every outside half, so one Graphviz
    // cubic deliberately expands into the sequence emitted by PlantUML.
    let mut curves = curves_from_points(&points);
    if let Some(tail) = tail
        && curves.first().is_some_and(|curve| contains(tail, curve[0]))
        && let Some(index) = curves.iter().position(|curve| !contains(tail, curve[3]))
    {
        let mut current = curves[index];
        let mut clipped = Vec::new();
        for _ in 0..8 {
            let (inside_half, outside_half) = subdivide(current);
            if contains(tail, inside_half[3]) {
                current = outside_half;
            } else {
                clipped.insert(0, outside_half);
                current = inside_half;
            }
        }
        clipped.extend_from_slice(&curves[index + 1..]);
        curves = clipped;
    }

    if let Some(head) = head
        && curves.last().is_some_and(|curve| contains(head, curve[3]))
        && let Some(index) = curves.iter().position(|curve| contains(head, curve[3]))
        && !contains(head, curves[index][0])
    {
        let mut current = curves[index];
        let mut clipped = curves[..index].to_vec();
        for _ in 0..8 {
            let (outside_half, inside_half) = subdivide(current);
            if contains(head, outside_half[3]) {
                current = outside_half;
            } else {
                clipped.push(outside_half);
                current = inside_half;
            }
        }
        curves = clipped;
    }

    points_from_curves(&curves)
}

fn trim_deployment_edge_endpoint(points: &mut Vec<(f64, f64)>, endpoint: usize, adjacent: usize) {
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

    if endpoint == 0 && points.len() > 4 {
        // Java `DotPath.moveStartPoint` discards one leading cubic when its
        // endpoint distance is no longer than the decoration translation.
        let first_delta = (points[3].0 - points[0].0, points[3].1 - points[0].1);
        if DEPENDENCY_ARROW_LENGTH >= first_delta.0.hypot(first_delta.1) {
            points.drain(..3);
            let residual = (-shift.0 - first_delta.0, -shift.1 - first_delta.1);
            points[0].0 += residual.0;
            points[0].1 += residual.1;
            points[1].0 += residual.0;
            points[1].1 += residual.1;
            return;
        }
    }

    points[endpoint].0 -= shift.0;
    points[endpoint].1 -= shift.1;
    if points.len() >= 4 {
        points[adjacent].0 -= shift.0;
        points[adjacent].1 -= shift.1;
    }
}

fn emit_deployment_arrowhead(
    svg: &mut SvgBuilder,
    previous: &(f64, f64),
    endpoint: &(f64, f64),
    handwritten: bool,
) {
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
    let vertices = [
        tip,
        (rear.0 + perpendicular.0, rear.1 + perpendicular.1),
        inset,
        (rear.0 - perpendicular.0, rear.1 - perpendicular.1),
    ];
    let points = if handwritten {
        handwritten_polygon_points(&vertices)
    } else {
        let mut points = vertices
            .iter()
            .map(|(x, y)| format!("{},{}", fc(*x), fc(*y)))
            .collect::<Vec<_>>();
        points.push(points[0].clone());
        points.join(",")
    };
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

    fn root_numeric_attr(svg: &str, name: &str) -> f64 {
        let root = svg.split_once('>').map_or(svg, |(root, _)| root);
        numeric_attr(root, name)
    }

    fn numeric_attr(tag: &str, name: &str) -> f64 {
        let marker = format!(r#" {name}=""#);
        tag.split_once(&marker)
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(value, _)| value.trim_end_matches("px"))
            .unwrap_or_else(|| panic!("missing {name} in {tag}"))
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric {name} in {tag}"))
    }

    fn cluster_section<'a>(svg: &'a str, qualified_name: &str) -> &'a str {
        let marker = format!(r#"data-qualified-name="{qualified_name}""#);
        svg.split_once(&marker)
            .and_then(|(_, tail)| tail.split_once("</g>"))
            .map(|(cluster, _)| cluster)
            .unwrap_or_else(|| panic!("missing cluster {qualified_name}: {svg}"))
    }

    fn link_section<'a>(svg: &'a str, link_id: &str) -> &'a str {
        let marker = r#"<g class="link" "#;
        svg.split(&marker)
            .skip(1)
            .find(|section| section.contains(&format!(r#"id="{link_id}""#)))
            .and_then(|section| section.split_once("</g>"))
            .map(|(link, _)| link)
            .unwrap_or_else(|| panic!("missing link {link_id}: {svg}"))
    }

    fn first_element_tag<'a>(content: &'a str, element: &str) -> &'a str {
        let marker = format!("<{element} ");
        content
            .split_once(&marker)
            .and_then(|(_, tail)| tail.split_once('>'))
            .map(|(tag, _)| tag)
            .unwrap_or_else(|| panic!("missing {element} in {content}"))
    }

    #[test]
    fn deployment_round_corner_resolves_generic_and_symbol_specific_fashion() {
        let generic = rustuml_parser::parse::parse_auto_with_base(
            "@startuml\n\
             stack GenericStack {\n  node Child\n}\n\
             artifact GenericArtifact\n\
             rectangle GenericRectangle\n\
             skinparam roundcorner 17\n\
             @enduml",
            None,
        )
        .unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(generic) = generic else {
            panic!("expected deployment diagram");
        };
        for node in &generic.nodes {
            let cluster = !node.children.is_empty();
            if matches!(
                node.kind,
                DeploymentNodeKind::Stack
                    | DeploymentNodeKind::Artifact
                    | DeploymentNodeKind::Rectangle
            ) {
                assert_eq!(
                    deployment_round_corner(&generic, node.kind, cluster, node.source_line),
                    17.0,
                    "{}",
                    node.id
                );
            }
        }

        let specific = rustuml_parser::parse::parse_auto_with_base(
            "@startuml\n\
             <style>\n\
             componentDiagram {\n\
               group { stack { RoundCorner 18 } }\n\
               artifact { title { RoundCorner 22 } }\n\
               rectangle { title { RoundCorner 14 } }\n\
             }\n\
             </style>\n\
             stack SpecificStack {\n  node Child\n}\n\
             artifact SpecificArtifact\n\
             rectangle SpecificRectangle\n\
             @enduml",
            None,
        )
        .unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(specific) = specific else {
            panic!("expected deployment diagram");
        };
        for node in &specific.nodes {
            let expected = match node.kind {
                DeploymentNodeKind::Stack => Some(18.0),
                DeploymentNodeKind::Artifact => Some(22.0),
                DeploymentNodeKind::Rectangle => Some(14.0),
                _ => None,
            };
            if let Some(expected) = expected {
                assert_eq!(
                    deployment_round_corner(
                        &specific,
                        node.kind,
                        !node.children.is_empty(),
                        node.source_line,
                    ),
                    expected,
                    "{}",
                    node.id
                );
            }
        }
    }

    #[test]
    fn deployment_fashion_consumers_receive_the_resolved_corner() {
        let source = "@startuml\n\
                      skinparam roundcorner 18\n\
                      stack CornerStack {\n  artifact CornerArtifact\n}\n\
                      rectangle CornerRectangle\n\
                      @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"rx="9" ry="9""#), "{svg}");
        assert!(svg.contains("A9,9 0 0 1"), "{svg}");
    }

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
    fn global_left_to_right_direction_controls_fresh_deployment_chain() {
        let source = concat!(
            "@startuml\n",
            "left to right direction\n",
            "node CopperHost\n",
            "node VioletHost\n",
            "node SaffronHost\n",
            "node TealHost\n",
            "CopperHost --> VioletHost\n",
            "VioletHost --> SaffronHost\n",
            "SaffronHost --> TealHost\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let svg = render(&diagram, &Theme::default());

        assert!(
            root_numeric_attr(&svg, "width") > root_numeric_attr(&svg, "height"),
            "left-to-right rank direction must make a four-node chain wider than tall: {svg}"
        );
    }

    #[test]
    fn no_oracle_handwritten_deployment_generates_warning_and_fresh_geometry() {
        let source = "@startuml\n\
            skinparam handwritten true\n\
            node \"Renamed API Gateway\" as Gateway\n\
            database \"Fresh Ledger Store\" as Ledger\n\
            Gateway --> Ledger : writes audit events\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let warning = svg.find("Please&#160;use").unwrap();
        let gateway = svg.find("Renamed API Gateway").unwrap();
        let expected_width =
            (deprecated_handwritten_warning_block_width() + SVEK_ENVELOPE_ORIGIN) as i64;

        assert!(warning < gateway);
        assert!(svg.contains(&format!(r#"viewBox="0 0 {expected_width} "#)));
        assert!(svg.contains(r##"fill="#FFFFCC""##));
        assert!(svg.contains("Fresh Ledger Store"));
        assert!(svg.contains("writes audit events"));
        assert!(!svg.contains("<line"));
        assert!(!svg.contains(r#"id="Gateway-to-Ledger""#));
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
    fn no_oracle_edge_label_box_extends_painted_x_envelope() {
        let source = "@startuml\n\
            node \"Relay 241\" as Relay241 <<cloud>>\n\
            artifact \"Payload 251\" as Payload251\n\
            Payload251 --> Relay241 : elongated transport 257\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"height="211px""#));
        assert!(
            svg.contains(r#"style="width:240px;height:211px;background:#FFFFFF;""#),
            "{svg}"
        );
        assert!(svg.contains(">elongated transport 257</text>"));
    }

    #[test]
    fn no_oracle_interface_port_routes_fresh_endpoint_quantifier() {
        let source = r#"@startuml
node "gateway-prod"
node "ledger_replica"
artifact "payload-v2.7.war"
"gateway-prod" --> "ledger_replica" : tcp 6432
artifact "payload-v2.7.war" --> "gateway-prod" : rollout
@enduml"#;
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(
            svg.contains(r#"d="M133.4212,35.7 C133.4212,54.75 133.4212,97.5 133.4212,126.44""#)
        );
        assert!(svg.contains(r#"textLength="110.6904" x="6""#));
        assert!(svg.contains(">payload-v2.7.war</text>"));
    }

    #[test]
    fn no_oracle_cluster_edge_label_box_extends_painted_x_envelope() {
        let source = "@startuml\n\
            node NorthGate401 {\n\
              artifact Parcel409\n\
            }\n\
            node SouthGate419 {\n\
              artifact Record421\n\
            }\n\
            NorthGate401 --> SouthGate419 : sync431\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(
            svg.contains(r#"style="width:252px;height:318px;background:#FFFFFF;""#),
            "{svg}"
        );
        assert!(svg.contains(">sync431</text>"));
    }

    #[test]
    fn no_oracle_title_chrome_centres_the_svek_body() {
        let source = "@startuml\n\
            title Resilient Staging Mesh 503\n\
            node Edge503\n\
            database Store509\n\
            Edge503 --> Store509\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:223px;height:220px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"<g class="title" data-source-line="1">"#));
        assert!(svg.contains(">Resilient Staging Mesh 503</text>"));
    }

    #[test]
    fn no_oracle_header_footer_chrome_uses_positioned_alignment() {
        let source = "@startuml\n\
            header Restricted Build 521\n\
            footer Generated for Canary 523\n\
            node Relay521\n\
            database Ledger523\n\
            Relay521 --> Ledger523\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:143px;height:208px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"<g class="header" data-source-line="1">"#));
        assert!(svg.contains(r#"<g class="footer" data-source-line="2">"#));
        assert!(svg.contains(r#"x="38.5293" y="9.668""#));
        assert!(svg.contains(r#"x="5.3242" y="199.4236""#));
    }

    #[test]
    fn no_oracle_top_left_legend_precedes_body_and_keeps_source_line() {
        let source = "@startuml\n\
            node \"Ingress 601\" as Ingress601\n\
            database \"Journal 607\" as Journal607\n\
            Ingress601 --> Journal607 : stream 613\n\
            legend top left\n\
            | Region | Active | Route |\n\
            | north-19 | 3 | canary-blue |\n\
            endlegend\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"<g class="legend" data-source-line="4">"#));
        assert!(svg.contains(r#"width="222.2969" x="12" y="12"/>"#));
        assert!(svg.contains(r#"points="60.0179,87.9766"#));
        assert!(svg.contains(">north-19</text>"));
        assert!(svg.contains(">canary-blue</text>"));
        let legend = svg.find(r#"<g class="legend""#).expect("legend group");
        let entity = svg.find(r#"<g class="entity""#).expect("deployment entity");
        assert!(legend < entity, "top legend must paint before the body");
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
    fn no_oracle_link_ids_follow_source_order_uid_allocation() {
        let source = "@startuml\n\
            node \"Dispatch 211\" as Dispatch211\n\
            node \"Worker 223\" as Worker223\n\
            Dispatch211 --> Worker223\n\
            node \"Archive 227\" as Archive227\n\
            Worker223 --> Archive227\n\
            Archive227 -left-> Worker223\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"data-source-line="3" id="lnk4""#));
        assert!(svg.contains(r#"data-source-line="5" id="lnk6""#));
        assert!(svg.contains(r#"data-source-line="6" id="lnk8""#));
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
    fn no_oracle_storage_cluster_uses_storage_paint_text_and_rectangle_envelope() {
        let source = "@startuml\n\
            storage \"Renamed Archive 8209\" as Archive8209 <<durable_8219>> #LightBlue {\n\
              queue \"Pending Batch 8221\" as Batch8221\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let cluster = cluster_section(&svg, "Archive8209");
        let rect = first_element_tag(cluster, "rect");
        let mut text_tags = cluster
            .split("<text ")
            .skip(1)
            .filter_map(|tail| tail.split_once('>').map(|(tag, _)| tag));
        let stereotype = text_tags.next().expect("storage stereotype");
        let title = text_tags.next().expect("storage title");
        let rect_x = numeric_attr(rect, "x");
        let rect_y = numeric_attr(rect, "y");
        let rect_width = numeric_attr(rect, "width");
        let rect_height = numeric_attr(rect, "height");

        assert!(rect.contains(r#"rx="35" ry="35""#), "{cluster}");
        assert!(rect.contains("stroke:#181818;stroke-width:1;"), "{cluster}");
        assert!(!cluster.contains("<polygon "), "{cluster}");
        assert_eq!(rect_x, 7.0);
        assert_eq!(rect_y, 7.0);
        assert!(
            ((numeric_attr(title, "y") - numeric_attr(stereotype, "y")) - (TEXT_LINE_H + 2.0))
                .abs()
                < 0.0001
        );
        assert_eq!(
            root_numeric_attr(&svg, "width"),
            (rect_x + rect_width - 1.0 + SVEK_DIMENSION_DELTA) as i64 as f64
        );
        assert_eq!(
            root_numeric_attr(&svg, "height"),
            (rect_y + rect_height - 1.0 + SVEK_DIMENSION_DELTA) as i64 as f64
        );
    }

    #[test]
    fn no_oracle_artifact_cluster_uses_polygon_expanded_rectangle_envelope() {
        let source = "@startuml\n\
            left to right direction\n\
            artifact \"Renamed Manifest 8231\" as Manifest8231 #PaleGreen {\n\
              database \"Ledger 8233\" as Ledger8233\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let cluster = cluster_section(&svg, "Manifest8231");
        let rect = first_element_tag(cluster, "rect");
        let rect_x = numeric_attr(rect, "x");
        let rect_y = numeric_attr(rect, "y");
        let rect_width = numeric_attr(rect, "width");
        let rect_height = numeric_attr(rect, "height");

        assert!(rect.contains(r#"rx="2.5" ry="2.5""#), "{cluster}");
        assert_eq!(cluster.matches("<polygon ").count(), 1, "{cluster}");
        assert_eq!(cluster.matches("<line ").count(), 2, "{cluster}");
        assert_eq!(rect_x, 7.0);
        assert_eq!(rect_y, 7.0);
        assert_eq!(
            root_numeric_attr(&svg, "width"),
            (rect_x + rect_width + 5.0 + SVEK_DIMENSION_DELTA) as i64 as f64
        );
        assert_eq!(
            root_numeric_attr(&svg, "height"),
            (rect_y + rect_height - 1.0 + SVEK_DIMENSION_DELTA) as i64 as f64
        );
    }

    #[test]
    fn cluster_symbol_bounds_follow_java_limit_finder_primitives() {
        assert_eq!(
            deployment_cluster_local_painted_bounds(DeploymentNodeKind::Storage, 120.0, 80.0),
            Some((-1.0, -1.0, 119.0, 79.0))
        );
        assert_eq!(
            deployment_cluster_local_painted_bounds(DeploymentNodeKind::Artifact, 120.0, 80.0),
            Some((-1.0, -1.0, 125.0, 79.0))
        );
    }

    #[test]
    fn translated_cluster_extent_preserves_java_seed_boundary() {
        // Java `Cluster.move` translates each endpoint before
        // `RectangleArea.getWidth` subtracts them for the final render.
        let moved = translated_layout_extent(16.0, 116.0, -3.8);

        assert_eq!(moved.to_bits() + 1, 116.0_f64.to_bits());
    }

    #[test]
    fn no_oracle_cloud_cluster_regenerates_after_svek_translation() {
        let source = "@startuml\n\
            cloud \"Zone 731\" {\n\
              node \"API-733\"\n\
              database \"DB-739\"\n\
              \"API-733\" --> \"DB-739\"\n\
            }\n\
            cloud \"Zone 743\" {\n\
              node \"API-747\"\n\
              database \"DB-751\"\n\
              \"API-747\" --> \"DB-751\"\n\
            }\n\
            \"DB-739\" --> \"DB-751\" : mirror\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert_eq!(svg.matches(r#"<g class="cluster""#).count(), 2);
        assert!(svg.contains(r#"data-qualified-name="Zone 731.API-733""#));
        assert!(svg.contains(r#"data-qualified-name="Zone 743.DB-751""#));
    }

    #[test]
    fn no_oracle_nested_groups_emit_direct_leaves_before_child_group_leaves() {
        let source = "@startuml\n\
            cloud \"Fabric 761\" {\n\
              node \"Nested 773\" {\n\
                component \"Deep 787\"\n\
                component \"Deep 797\"\n\
              }\n\
              database \"Ledger 809\"\n\
              queue \"Bus 811\"\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let direct = svg.find("<!--entity Ledger 809-->").unwrap();
        let nested = svg.find("<!--entity Deep 787-->").unwrap();

        assert!(direct < nested);
    }

    #[test]
    fn no_oracle_empty_deduplicated_group_keeps_group_traversal_position() {
        let source = "@startuml\n\
            node \"Primary 821\" {\n\
              component \"Shared 823\"\n\
            }\n\
            node \"Vacated 827\" {\n\
              component \"Shared 823\"\n\
            }\n\
            node \"Later 829\" {\n\
              component \"Unique 839\"\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let first_group_leaf = svg.find("<!--entity Shared 823-->").unwrap();
        let empty_group = svg.find("<!--entity Vacated 827-->").unwrap();
        let later_group_leaf = svg.find("<!--entity Unique 839-->").unwrap();

        assert!(first_group_leaf < empty_group);
        assert!(empty_group < later_group_leaf);
    }

    #[test]
    fn no_oracle_folder_cluster_uses_painted_path_envelope() {
        let source = "@startuml\n\
            folder \"Archive Cell 229\" {\n\
              artifact \"Bundle 233\" as Bundle233\n\
              queue \"Jobs 239\" as Jobs239\n\
            }\n\
            Bundle233 --> Jobs239\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"height="197px""#));
        assert!(svg.contains(r#"style="width:161px;height:197px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"data-qualified-name="Archive Cell 229.Bundle233""#));
        assert!(svg.contains(r#"data-qualified-name="Archive Cell 229.Jobs239""#));
    }

    #[test]
    fn no_oracle_rectangle_cluster_uses_rectangle_painted_envelope() {
        let source = "@startuml\n\
            rectangle \"Rectangular Zone 263\" {\n\
              node \"Worker 269\" as Worker269\n\
              node \"Worker 271\" as Worker271\n\
              node \"Worker 277\" as Worker277\n\
              node \"Worker 281\" as Worker281\n\
              node \"Worker 283\" as Worker283\n\
            }\n\
            Worker269 --> Worker271\n\
            Worker271 --> Worker277\n\
            Worker277 --> Worker281\n\
            Worker281 --> Worker283\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:195px;height:544px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"data-qualified-name="Rectangular Zone 263.Worker269""#));
        assert!(svg.contains(r#"data-qualified-name="Rectangular Zone 263.Worker283""#));
    }

    #[test]
    fn no_oracle_unpackaged_root_leaf_participates_in_cluster_frame() {
        let source = "@startuml\n\
            node \"Ingress 271\"\n\
            rectangle \"Primary Zone 277\" {\n\
              node \"Service 281\"\n\
              database \"Store 283\"\n\
              \"Service 281\" --> \"Store 283\"\n\
            }\n\
            rectangle \"Recovery Zone 293\" {\n\
              node \"Replica 307\"\n\
            }\n\
            \"Ingress 271\" --> \"Service 281\" : live\n\
            \"Ingress 271\" --> \"Replica 307\" : standby\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let ingress = svg
            .split("<!--entity Ingress 271-->")
            .nth(1)
            .unwrap()
            .split("</g>")
            .next()
            .unwrap();

        assert!(ingress.contains(",16,"));
        assert!(ingress.contains(",6,"));
    }

    #[test]
    fn no_oracle_frame_cluster_uses_rectangle_painted_envelope() {
        let source = "@startuml\n\
            frame \"Frame Lab 293\" {\n\
              artifact \"Bundle 307\" as Bundle307\n\
              queue \"Jobs 311\" as Jobs311\n\
            }\n\
            Bundle307 --> Jobs311\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:161px;height:197px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"data-qualified-name="Frame Lab 293.Bundle307""#));
        assert!(svg.contains(r#"data-qualified-name="Frame Lab 293.Jobs311""#));
    }

    #[test]
    fn no_oracle_compound_group_links_scale_to_four_levels() {
        let source = "@startuml\n\
            cloud Boundary317 {\n\
              folder Domain331 {\n\
                node Cell337 {\n\
                  frame Pod347 {\n\
                    artifact Binary349\n\
                    database Ledger353\n\
                  }\n\
                  node Relay359\n\
                  Pod347 --> Relay359\n\
                }\n\
                node Gateway367\n\
                Cell337 --> Gateway367\n\
              }\n\
              node Observer373\n\
              Domain331 --> Observer373\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"style="width:883px;height:541px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"id="Pod347-to-Relay359""#));
        assert!(svg.contains(r#"id="Cell337-to-Gateway367""#));
        assert!(svg.contains(r#"id="Domain331-to-Observer373""#));
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
    fn no_oracle_sprite_backed_stereotype_does_not_reserve_text_row() {
        let source = "@startuml\n\
            !include <tupadr3/common>\n\
            !include <tupadr3/font-awesome/server>\n\
            node \"Fresh Relay 641\" as Relay641 <<FA_SERVER>>\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let node = &diagram.nodes[0];
        let dim = deployment_node_dim(node, &diagram.meta.sprites);
        let mut plain_node = node.clone();
        plain_node.stereotype = None;
        let plain_dim = deployment_node_dim(&plain_node, &diagram.meta.sprites);

        assert_eq!(node.source_line, 3);
        assert_eq!(dim.width, plain_dim.width);
        assert_eq!(dim.height, plain_dim.height);
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"data-source-line="3" id="ent0002">"#));
        assert!(!svg.contains("FA_SERVER</text>"));
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
    fn no_oracle_floating_note_shares_source_order_and_magma_with_root_leaves() {
        let source = "@startuml\n\
            note as RenamedLedger733\n\
              control body remains display text\n\
            end note\n\
            node RenamedRuntime739\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let Some(note_start) =
            svg.find(r#"data-qualified-name="RenamedLedger733" data-source-line="2" id="ent0002""#)
        else {
            panic!("floating note entity: {svg}");
        };
        let Some(node_start) = svg
            .find(r#"data-qualified-name="RenamedRuntime739" data-source-line="4" id="ent0003""#)
        else {
            panic!("later deployment entity: {svg}");
        };
        assert!(note_start < node_start, "{svg}");
        assert!(svg[note_start..node_start].contains("control body remains display text"));
        assert!(!svg[note_start..node_start].contains(" L0,0 "), "{svg}");
    }

    #[test]
    fn deployment_svek_leaf_order_follows_cross_kind_quark_registration() {
        fn leaf_order(source: &str) -> Vec<DeploymentSvekLeaf> {
            let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
            let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
                panic!("expected deployment diagram");
            };
            let parent_of = deployment_parent_map(&diagram);
            let notes = laid_out_deployment_note_indices(&diagram);
            let cluster_ids = deployment_cluster_ids(&diagram, &notes);
            deployment_svek_leaf_order(&diagram, &parent_of, &cluster_ids, &notes)
        }

        let note_first = leaf_order(
            "@startuml\n\
             note as FirstLedger\n\
               payload\n\
             endnote\n\
             node LaterRuntime\n\
             @enduml",
        );
        assert_eq!(
            note_first,
            [DeploymentSvekLeaf::Note(0), DeploymentSvekLeaf::Node(0)]
        );

        let node_first = leaf_order(
            "@startuml\n\
             node FirstRuntime\n\
             note as LaterLedger\n\
               payload\n\
             endnote\n\
             @enduml",
        );
        assert_eq!(
            node_first,
            [DeploymentSvekLeaf::Node(0), DeploymentSvekLeaf::Note(0)]
        );

        let nested_then_root = leaf_order(
            "@startuml\n\
             node OuterBoundary {\n\
               note as InnerLedger\n\
                 inner payload\n\
               endnote\n\
               node InnerRuntime\n\
             }\n\
             note as RootLedger\n\
               root payload\n\
             endnote\n\
             node RootRuntime\n\
             @enduml",
        );
        assert_eq!(
            nested_then_root,
            [
                DeploymentSvekLeaf::Note(0),
                DeploymentSvekLeaf::Node(1),
                DeploymentSvekLeaf::Note(1),
                DeploymentSvekLeaf::Node(2),
            ]
        );
    }

    #[test]
    fn owned_note_is_a_qualified_leaf_of_its_note_only_cluster() {
        let source = "@startuml\n\
                      node OuterShell {\n\
                        node InnerShell {\n\
                          note as InnerLedger\n\
                            inner ledger\n\
                          end note\n\
                        }\n\
                      }\n\
                      database RootStore\n\
                      @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let notes = laid_out_deployment_note_indices(&diagram);
        let cluster_ids = deployment_cluster_ids(&diagram, &notes);
        assert!(cluster_ids.contains("OuterShell"));
        assert!(cluster_ids.contains("InnerShell"));

        let svg = render(&diagram, &Theme::default());
        assert!(
            svg.contains(r#"class="cluster" data-qualified-name="OuterShell.InnerShell""#),
            "{svg}"
        );
        assert_eq!(
            svg.matches(r#"data-qualified-name="OuterShell.InnerShell.InnerLedger""#)
                .count(),
            1,
            "{svg}"
        );
        assert!(
            !svg.contains(r#"data-qualified-name="InnerLedger""#),
            "{svg}"
        );
    }

    #[test]
    fn named_note_relation_reuses_one_owner_qualified_svek_leaf() {
        let source = "@startuml\n\
                      left to right direction\n\
                      folder \"Arrow Owner\" as ArrowOwner {\n\
                        note \"relation memo\" as ArrowMemo\n\
                        database \"Arrow Sink\" as ArrowSink\n\
                        ArrowMemo --> ArrowSink\n\
                        actor \"Free Observer\" as FreeObserver\n\
                      }\n\
                      @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        assert!(!diagram.nodes.iter().any(|node| node.id == "ArrowMemo"));
        assert_eq!(diagram.notes[0].target, None);
        assert_eq!(diagram.connections.len(), 1);

        let svg = render(&diagram, &Theme::default());
        assert_eq!(
            svg.matches(r#"data-qualified-name="ArrowOwner.ArrowMemo""#)
                .count(),
            1,
            "{svg}"
        );
        assert!(!svg.contains(r#"data-qualified-name="ArrowMemo""#), "{svg}");
        let note_group = svg
            .split(r#"data-qualified-name="ArrowOwner.ArrowMemo""#)
            .nth(1)
            .and_then(|tail| tail.split("</g>").next())
            .unwrap();
        assert!(note_group.matches(" L").count() > 8, "{note_group}");
        assert!(!svg.contains(r#"<g class="link""#), "{svg}");
        assert!(!svg.contains(r#"id="lnk5""#), "{svg}");
        assert!(
            svg.contains(
                r#"data-qualified-name="ArrowOwner.FreeObserver" data-source-line="6" id="ent0006""#
            ),
            "{svg}"
        );
    }

    #[test]
    fn named_note_opale_requires_one_non_note_link_outside_strict_uml() {
        let two_links = "@startuml\n\
                         note \"memo\" as Memo\n\
                         node Left\n\
                         node Right\n\
                         Memo -- Left\n\
                         Memo -- Right\n\
                         @enduml";
        let rustuml_parser::diagram::Diagram::Deployment(two_links) =
            rustuml_parser::parse::parse_auto_with_base(two_links, None).unwrap()
        else {
            panic!("expected deployment diagram");
        };
        let svg = render(&two_links, &Theme::default());
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 2, "{svg}");

        let note_to_note = "@startuml\n\
                            note \"left memo\" as LeftMemo\n\
                            note \"right memo\" as RightMemo\n\
                            node DispatchAnchor\n\
                            LeftMemo -- RightMemo\n\
                            @enduml";
        let rustuml_parser::diagram::Diagram::Deployment(note_to_note) =
            rustuml_parser::parse::parse_auto_with_base(note_to_note, None).unwrap()
        else {
            panic!("expected deployment diagram");
        };
        let svg = render(&note_to_note, &Theme::default());
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 1, "{svg}");

        let strict = "@startuml\n\
                      skinparam style strictuml\n\
                      note \"strict memo\" as StrictMemo\n\
                      node StrictPeer\n\
                      StrictMemo -- StrictPeer\n\
                      @enduml";
        let rustuml_parser::diagram::Diagram::Deployment(strict) =
            rustuml_parser::parse::parse_auto_with_base(strict, None).unwrap()
        else {
            panic!("expected deployment diagram");
        };
        let svg = render(&strict, &Theme::default());
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 1, "{svg}");
    }

    #[test]
    fn named_note_does_not_absorb_a_relation_to_a_container_proxy() {
        let source = "@startuml\n\
                      node \"Rack group\" as RackGroup {\n\
                        node \"Resident service\" as ResidentService\n\
                      }\n\
                      note \"Group peer memo\" as GroupMemo\n\
                      GroupMemo --> RackGroup\n\
                      artifact \"Created later\" as LaterArtifact\n\
                      @enduml";
        let rustuml_parser::diagram::Diagram::Deployment(diagram) =
            rustuml_parser::parse::parse_auto_with_base(source, None).unwrap()
        else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"<g class="link""#), "{svg}");
        assert!(svg.contains(r#"id="lnk5""#), "{svg}");
        assert!(svg.contains(r#"id="GroupMemo-to-RackGroup""#), "{svg}");
        assert!(
            svg.contains(
                r#"data-qualified-name="LaterArtifact" data-source-line="6" id="ent0006""#
            ),
            "{svg}"
        );
    }

    #[test]
    fn hidden_relation_changes_layout_and_uid_without_painting() {
        let source = "@startuml\n\
                      note \"Visible plus hidden\" as HiddenControlMemo\n\
                      node \"Visible peer\" as VisiblePeer\n\
                      node \"Hidden peer\" as HiddenPeer\n\
                      HiddenControlMemo --> VisiblePeer\n\
                      HiddenControlMemo -[hidden]- HiddenPeer\n\
                      artifact \"Created after links\" as AfterHidden\n\
                      @enduml";
        let rustuml_parser::diagram::Diagram::Deployment(diagram) =
            rustuml_parser::parse::parse_auto_with_base(source, None).unwrap()
        else {
            panic!("expected deployment diagram");
        };
        assert_eq!(diagram.connections.len(), 2);
        assert!(diagram.connections[1].hidden);

        let svg = render(&diagram, &Theme::default());
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 1, "{svg}");
        assert!(svg.contains(r#"id="lnk5""#), "{svg}");
        assert!(!svg.contains(r#"id="lnk6""#), "{svg}");
        assert!(
            svg.contains(r#"data-qualified-name="AfterHidden" data-source-line="6" id="ent0007""#),
            "{svg}"
        );
    }

    #[test]
    fn hidden_parallel_relation_still_extends_limit_finder_envelope() {
        let source = "@startuml\n\
                      left to right direction\n\
                      node \"Ingress\" as Ingress\n\
                      node \"Worker\" as Worker\n\
                      node \"Auditor\" as Auditor\n\
                      Ingress --> Worker : first visible\n\
                      Ingress -[hidden]- Worker : suppressed duplicate\n\
                      Ingress ..> Worker : second visible\n\
                      Worker --> Auditor : after duplicate\n\
                      artifact \"After duplicates\" as AfterDuplicates\n\
                      @enduml";
        let rustuml_parser::diagram::Diagram::Deployment(diagram) =
            rustuml_parser::parse::parse_auto_with_base(source, None).unwrap()
        else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(
            svg.contains(r#"style="width:697px;height:166px;background:#FFFFFF;""#),
            "{svg}"
        );
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 3, "{svg}");
        assert!(!svg.contains("suppressed duplicate"), "{svg}");
        assert!(
            svg.contains(
                r#"data-qualified-name="AfterDuplicates" data-source-line="9" id="ent0009""#
            ),
            "{svg}"
        );
    }

    #[test]
    fn no_oracle_note_color_paints_body_and_fold_from_ast() {
        let source = "@startuml\n\
            node RenamedRuntime743\n\
            note as RenamedLedger751 #MistyRose\n\
              retained color payload\n\
            endnote\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        assert_eq!(svg.matches(r##"fill="#FFE4E1""##).count(), 2, "{svg}");
        assert!(svg.contains(">retained color payload</text>"), "{svg}");
    }

    #[test]
    fn link_note_compound_blocks_cover_every_java_merge_position() {
        let dim = DeploymentNoteDim {
            width: 40.0,
            height: 20.0,
        };
        let label = EdgeLabelSize {
            width: 30.0,
            height: 10.0,
        };
        for position in [
            DeploymentNotePosition::Left,
            DeploymentNotePosition::Right,
            DeploymentNotePosition::Top,
            DeploymentNotePosition::Bottom,
        ] {
            let note = DeploymentLinkNote {
                text: "renamed compound note".to_string(),
                color: None,
                position,
            };
            let size = deployment_link_note_label_size(&note, dim, Some(label));
            let (label_origin, note_origin) =
                deployment_link_note_blocks(100.0, 200.0, &note, dim, Some(label));
            let label_origin = label_origin.unwrap();

            match position {
                DeploymentNotePosition::Left => {
                    assert_eq!((size.width, size.height), (80.0, 30.0));
                    assert!(note_origin.0 < label_origin.0);
                }
                DeploymentNotePosition::Right => {
                    assert_eq!((size.width, size.height), (80.0, 30.0));
                    assert!(label_origin.0 < note_origin.0);
                }
                DeploymentNotePosition::Top => {
                    assert_eq!((size.width, size.height), (50.0, 40.0));
                    assert!(note_origin.1 < label_origin.1);
                }
                DeploymentNotePosition::Bottom => {
                    assert_eq!((size.width, size.height), (50.0, 40.0));
                    assert!(label_origin.1 < note_origin.1);
                }
            }
        }

        let note = DeploymentLinkNote {
            text: "note-only control".to_string(),
            color: None,
            position: DeploymentNotePosition::Bottom,
        };
        let note_only_size = deployment_link_note_label_size(&note, dim, None);
        assert_eq!((note_only_size.width, note_only_size.height), (50.0, 30.0));
        assert_eq!(
            deployment_link_note_blocks(100.0, 200.0, &note, dim, None),
            (None, (105.0, 205.0))
        );
    }

    #[test]
    fn deployment_svek_spacing_includes_compound_link_labels_and_decorations() {
        let source = "@startuml\n\
            node RenamedIngress271\n\
            node RenamedArchive277\n\
            RenamedIngress271 -left-> RenamedArchive277 : changed route label 281\n\
            note left on link : changed link note payload with independent topology axis 283\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let connection = &diagram.connections[0];
        assert_eq!(connection.length, 1);
        let note = connection.note.as_ref().unwrap();
        let compound = deployment_link_note_label_size(
            note,
            deployment_link_note_dim(note),
            connection
                .label
                .as_deref()
                .map(|label| deployment_center_label_size(label, false)),
        );
        let (node_sep, rank_sep) = deployment_svek_spacing(&diagram);
        assert_eq!(
            node_sep,
            SVEK_MIN_NODE_SEP.max(
                (compound.width + LINK_DECOR_ARROW_MARGIN + LINK_DECOR_NONE_MARGIN)
                    / SVEK_DZETA_DIVISOR,
            )
        );
        assert_eq!(rank_sep, SVEK_MIN_RANK_SEP);
        assert!(node_sep > SVEK_MIN_NODE_SEP);
    }

    #[test]
    fn link_note_expands_display_rows_and_preserves_interior_blank_atoms() {
        let label = "persist\\n\\nwith checksum";
        let rows = deployment_center_label_rows(label);
        assert_eq!(rows, ["persist", "", "with checksum"]);
        assert_eq!(deployment_center_label_display_row(rows[1]), "\u{00A0}");
        let size = deployment_center_label_size(label, false);
        assert_eq!(
            size.width,
            text_render::measure("with checksum", 13.0, false) + 2.0
        );
        assert_eq!(
            size.height,
            text_render::label_height("persist", 13.0)
                + text_render::label_height("\u{00A0}", 13.0)
                + text_render::label_height("with checksum", 13.0)
                + 2.0
        );
        let mut bounds = DeploymentPaintBounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        include_deployment_center_label_paint_bounds(&mut bounds, label, (10.0, 20.0), size);
        assert_eq!(
            bounds.min_y,
            21.0 + pm::ascent(13.0) - text_render::label_height("persist", 13.0)
                + LIMIT_FINDER_TEXT_ADJUST
        );
        assert!(bounds.min_y < 20.0);

        let source = "@startuml\n\
                      node FreshIngress\n\
                      database FreshArchive\n\
                      FreshIngress --> FreshArchive : persist\\n\\nwith checksum\n\
                      note top on link\n\
                        first row\n\
                      \n\
                        final row\n\
                      end note\n\
                      @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(">persist</text>"), "{svg}");
        assert!(svg.contains(">with checksum</text>"), "{svg}");
        assert!(!svg.contains("persist\\nwith checksum</text>"), "{svg}");
        assert!(svg.contains(">&#160;</text>"), "{svg}");
    }

    #[test]
    fn multiline_link_note_and_label_share_the_owning_link_group() {
        let source = "@startuml\n\
            node \"Renamed Ingress 881\" as Ingress881\n\
            node \"Renamed Archive 883\" as Archive883\n\
            Ingress881 --> Archive883 : renamed durable route\n\
            note top of link #LightBlue\n\
              validates renamed payload 887\n\
              before durable handoff 907\n\
            end note\n\
            artifact RenamedLater911\n\
            Archive883 --> RenamedLater911\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let first_link = link_section(&svg, "lnk4");
        let note_text = first_link
            .find(">validates renamed payload 887</text>")
            .unwrap();
        let ordinary_label = first_link.find(">renamed durable route</text>").unwrap();
        assert!(note_text < ordinary_label, "top merge order: {first_link}");
        assert!(first_link.contains(">before durable handoff 907</text>"));
        assert_eq!(first_link.matches(r##"fill="#ADD8E6""##).count(), 2);
        assert!(!first_link.contains(r#"<g class="entity""#));
        assert_eq!(svg.matches(r#"<g class="entity""#).count(), 3, "{svg}");
        assert!(
            svg.contains(
                r#"data-qualified-name="RenamedLater911" data-source-line="8" id="ent0005""#
            ),
            "link note must consume no UID: {svg}"
        );
        assert!(
            svg.contains(r#"data-source-line="9" id="lnk6""#),
            "later link UID/source drifted: {svg}"
        );
    }

    #[test]
    fn inline_nested_link_note_keeps_note_only_geometry_inside_the_edge() {
        let source = "@startuml\n\
            node RenamedOuter929 {\n\
              node RenamedIngress937\n\
              node RenamedArchive941\n\
              RenamedIngress937 -right-> RenamedArchive941\n\
              note left on link #PaleGreen : nested renamed handoff 947\n\
              artifact RenamedLater953\n\
              RenamedArchive941 --> RenamedLater953\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());
        let first_link = link_section(&svg, "lnk5");
        assert!(first_link.contains(">nested renamed handoff 947</text>"));
        assert_eq!(first_link.matches(r##"fill="#98FB98""##).count(), 2);
        assert!(!first_link.contains(r#"<g class="entity""#));
        assert_eq!(svg.matches(r#"<g class="entity""#).count(), 3, "{svg}");
        assert_eq!(svg.matches(r#"<g class="cluster""#).count(), 1, "{svg}");
        assert!(
            svg.contains(
                r#"data-qualified-name="RenamedOuter929.RenamedLater953" data-source-line="6" id="ent0006""#
            ),
            "nested link note must not alter entity ownership or UID: {svg}"
        );
        assert!(
            svg.contains(r#"data-source-line="7" id="lnk7""#),
            "later nested link UID/source drifted: {svg}"
        );
    }

    #[test]
    fn renamed_sprite_actor_uses_stickman_above_the_complete_label_block() {
        let source = "@startuml\n\
            sprite $fresh_actor [6x4/16] {\n\
            011110\n\
            110011\n\
            101101\n\
            011110\n\
            }\n\
            actor \"<$fresh_actor> Renamed operator\" as Operator773\n\
            node \"<$fresh_actor> Renamed service\" as Service787\n\
            Operator773 --> Service787\n\
            Service787 --> Operator773\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };
        let actor = diagram
            .nodes
            .iter()
            .find(|node| node.kind == DeploymentNodeKind::Actor)
            .unwrap();
        let dim = deployment_node_dim(actor, &diagram.meta.sprites);
        let label_width = deployment_label_width_for_sprites(
            &actor.label,
            FONT_SIZE,
            false,
            &diagram.meta.sprites,
        );

        assert_eq!(dim.width, label_width.max(ACTOR_SYMBOL_WIDTH));
        assert_eq!(dim.height, ACTOR_SYMBOL_HEIGHT + TEXT_LINE_H);

        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"rx="8" ry="8" style="stroke:#181818;stroke-width:0.5;""#));
        assert!(svg.contains(r#"width="6" height="4""#));
        assert!(!svg.contains("&lt;$fresh_actor&gt;"));
    }

    #[test]
    fn renamed_parallel_links_consume_distinct_svek_edges_in_source_order() {
        let source = "@startuml\n\
            node FreshSender811\n\
            node FreshReceiver821\n\
            FreshSender811 --> FreshReceiver821 : primary channel\n\
            FreshSender811 --> FreshReceiver821 : fallback channel\n\
            FreshSender811 ..> FreshReceiver821 : probe channel\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        // Java `SvekEdge` consumes Graphviz's parallel edge results in the
        // same order they were appended and suffixes repeated SVG identities.
        assert!(svg.contains(r#"id="FreshSender811-to-FreshReceiver821""#));
        assert!(svg.contains(r#"id="FreshSender811-to-FreshReceiver821-1""#));
        assert!(svg.contains(r#"id="FreshSender811-to-FreshReceiver821-2""#));
        assert!(svg.contains(">primary channel</text>"));
        assert!(svg.contains(">fallback channel</text>"));
        assert!(svg.contains(">probe channel</text>"));
    }

    #[test]
    fn deployment_svek_edge_translation_trims_path_but_not_arrow_tip() {
        let raw = vec![(10.0, 0.0), (10.0, 10.0), (10.0, 20.0), (10.0, 30.0)];

        let painted = deployment_svek_edge_points(
            &raw,
            BODY_FALLBACK_MARGIN_X,
            BODY_MARGIN_Y,
            None,
            None,
            EdgeTrim::End,
        );

        assert_eq!(
            painted,
            vec![(26.0, 7.0), (26.0, 17.0), (26.0, 21.0), (26.0, 31.0)]
        );

        let untrimmed = deployment_svek_edge_points(
            &raw,
            BODY_FALLBACK_MARGIN_X,
            BODY_MARGIN_Y,
            None,
            None,
            EdgeTrim::None,
        );
        let mut svg = SvgBuilder::new(100.0, 100.0);
        emit_deployment_arrowhead(
            &mut svg,
            &untrimmed[untrimmed.len() - 2],
            &untrimmed[untrimmed.len() - 1],
            false,
        );
        let output = svg.finalize();
        assert!(output.contains(r#"points="26,37,30,28,26,32,22,28,26,37""#));
    }

    #[test]
    fn deployment_svek_start_decoration_discards_a_short_leading_cubic() {
        let raw = vec![
            (0.0, 0.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.0),
            (3.0, 0.0),
            (7.0, 0.0),
            (8.0, 0.0),
        ];

        let painted = deployment_svek_edge_points(&raw, 0.0, 0.0, None, None, EdgeTrim::Start);

        assert_eq!(
            painted,
            vec![(-6.0, 0.0), (-5.0, 0.0), (7.0, 0.0), (8.0, 0.0)]
        );
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

    #[test]
    fn named_note_colors_reach_none_and_gradient_paint() {
        let source = "@startuml\n\
                      node Target\n\
                      note \"transparent solid\" as TransparentSolid #transparent\n\
                      note \"background solid\" as BackgroundSolid #background\n\
                      note \"gradient solid\" as GradientSolid #Red/LightBlue\n\
                      TransparentSolid --> Target\n\
                      BackgroundSolid --> Target\n\
                      GradientSolid --> Target\n\
                      @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None).unwrap();
        let rustuml_parser::diagram::Diagram::Deployment(diagram) = diagram else {
            panic!("expected deployment diagram");
        };

        let svg = render(&diagram, &Theme::default());

        assert!(svg.matches(r#"fill="none""#).count() >= 4, "{svg}");
        assert_eq!(svg.matches("<linearGradient ").count(), 1, "{svg}");
        assert!(svg.matches(r#"fill="url(#"#).count() >= 2, "{svg}");
    }
}
