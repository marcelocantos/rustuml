// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's use-case diagram rendering.
//! PlantUML emits use-case diagrams as `data-diagram-type="DESCRIPTION"` —
//! the same envelope used by component and deployment diagrams.

use std::collections::HashMap;

use rustuml_parser::diagram::usecase::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

const FONT_SIZE: f64 = 14.0;
const STEREO_FONT: f64 = 14.0;
const STROKE: &str = "#181818";
const ENTITY_FILL: &str = "#F1F1F1";
const TEXT_COLOR: &str = "#000000";

const ACTOR_HEAD_R: f64 = 8.0;
const ACTOR_BODY_LEN: f64 = 27.0;
const ACTOR_ARM_HALF: f64 = 13.0;
const ACTOR_ARM_OFFSET: f64 = 8.0;
const ACTOR_LEG_RUN: f64 = 13.0;
const ACTOR_LEG_DROP: f64 = 15.0;
const ACTOR_LABEL_GAP: f64 = 15.0352;
/// Vertical offset from head centre to stereotype baseline (measured).
const ACTOR_STEREO_OFFSET: f64 = 11.4531;
const LINE_H: f64 = 16.4883;
const UC_TEXT_OFFSET_SINGLE: f64 = 4.7441;

const UC_RX_PAD: f64 = 23.6825;
const UC_RY_PAD: f64 = 23.6825;

const MARGIN: f64 = 7.0;
const GAP: f64 = 40.0;

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_FOLD: f64 = 10.0;
const NOTE_FONT_SIZE: u32 = 13;

/// Which box edge carries the note's leader (callout) notch.
enum LeaderSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// Round a coordinate to PlantUML's 4-decimal format, dropping trailing zeros.
fn fc(v: f64) -> String {
    pm::fmt_coord(v)
}

/// Resolve a raw `#color` token (parser strips the leading `#`, so we receive
/// e.g. `Pink`, `LightBlue`, or `FFC0CB`) into a PlantUML fill string. Named
/// colours resolve to `#RRGGBB`; bare hex digits get a `#` prepended.
fn resolve_fill(raw: &str) -> String {
    let normalized = text_render::normalize_color(raw);
    if normalized.starts_with('#') {
        normalized
    } else {
        format!("#{normalized}")
    }
}

/// Look up a skinparam value case-insensitively (PlantUML convention) and
/// resolve it to a fill string. The parser flattens block skinparams like
/// `skinparam usecase { BackgroundColor X }` to the key `usecaseBackgroundColor`.
fn skin_color(skinparams: &[rustuml_parser::diagram::SkinParam], key: &str) -> Option<String> {
    skinparams
        .iter()
        .find(|p| p.key.eq_ignore_ascii_case(key))
        .map(|p| resolve_fill(p.value.trim_start_matches('#')))
}

/// Per-kind background/border defaults derived from `skinparam` directives.
struct SkinColors {
    actor_fill: Option<String>,
    actor_border: Option<String>,
    uc_fill: Option<String>,
    uc_border: Option<String>,
}

impl SkinColors {
    fn from_meta(skinparams: &[rustuml_parser::diagram::SkinParam]) -> Self {
        SkinColors {
            actor_fill: skin_color(skinparams, "actorBackgroundColor"),
            actor_border: skin_color(skinparams, "actorBorderColor"),
            uc_fill: skin_color(skinparams, "usecaseBackgroundColor"),
            uc_border: skin_color(skinparams, "usecaseBorderColor"),
        }
    }
}

/// Round a coordinate to 4 decimals (HALF_UP), returning the numeric value.
///
/// PlantUML places shapes at 4-decimal-rounded pixel coordinates and then
/// centres text relative to that *rounded* anchor, not the raw f64. Matching
/// this avoids 0.0001 drift in text `x` attributes.
fn round_coord(v: f64) -> f64 {
    let scaled = v * 10000.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        -((-scaled + 0.5).floor())
    };
    rounded / 10000.0
}

pub fn render(diagram: &UseCaseDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

pub fn render_with_oracle(
    diagram: &UseCaseDiagram,
    _theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Use-case diagrams share the DESCRIPTION envelope with component and
    // deployment; the oracle extractor already triggers on DESCRIPTION, so we
    // ride along here.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "DESCRIPTION");
    }

    if diagram.actors.is_empty()
        && diagram.use_cases.is_empty()
        && diagram.packages.is_empty()
        && diagram.notes.is_empty()
        && diagram.meta.title.is_none()
    {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="DESCRIPTION" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><defs/><g></g></svg>"#.to_string();
    }

    let actor_dims: Vec<ActorDim> = diagram.actors.iter().map(actor_dim).collect();
    let uc_dims: Vec<UseCaseDim> = diagram.use_cases.iter().map(use_case_dim).collect();
    let positions = resolve_positions(diagram, &actor_dims, &uc_dims, oracle);
    let id_map = build_entity_id_map(diagram);
    let skin = SkinColors::from_meta(&diagram.meta.skinparams);

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else {
        compute_canvas(&positions, &actor_dims, &uc_dims)
    };

    let mut svg = SvgBuilder::new_plantuml(total_w, total_h, "DESCRIPTION");

    render_header(&mut svg, diagram);
    render_title(&mut svg, diagram, total_w);

    // PlantUML renders each cluster group followed immediately by its member
    // entities (in source-line order), then the top-level (non-member)
    // entities. Track which entity ids belong to a package so we can emit the
    // members under their cluster and skip them in the top-level pass.
    let member_ids: std::collections::HashSet<&str> = diagram
        .packages
        .iter()
        .flat_map(|p| p.elements.iter().map(String::as_str))
        .collect();

    // Helper closures can't borrow svg mutably twice, so emit inline.
    let render_actor_i = |svg: &mut SvgBuilder, i: usize| {
        let (cx, cy) = positions.actors[i];
        render_actor(
            svg,
            &diagram.actors[i],
            &actor_dims[i],
            cx,
            cy,
            oracle,
            &id_map,
            &skin,
        );
    };
    let render_uc_i = |svg: &mut SvgBuilder, i: usize| {
        let (cx, cy) = positions.use_cases[i];
        render_use_case(
            svg,
            &diagram.use_cases[i],
            &uc_dims[i],
            diagram,
            cx,
            cy,
            oracle,
            &id_map,
            &skin,
        );
    };

    // PlantUML emits all cluster groups first (in source-line order), then all
    // member entities (in global source-line order), then the top-level
    // entities. Emit the clusters, then collect and sort the members.
    for pkg in &diagram.packages {
        render_package_group(&mut svg, pkg, oracle, &id_map);
    }
    let mut members: Vec<(usize, bool, usize)> = Vec::new(); // (source_line, is_actor, index)
    for (i, a) in diagram.actors.iter().enumerate() {
        if member_ids.contains(a.id.as_str()) {
            members.push((a.source_line, true, i));
        }
    }
    for (i, u) in diagram.use_cases.iter().enumerate() {
        if member_ids.contains(u.id.as_str()) {
            members.push((u.source_line, false, i));
        }
    }
    members.sort_by_key(|m| m.0);
    for (_, is_actor, i) in members {
        if is_actor {
            render_actor_i(&mut svg, i);
        } else {
            render_uc_i(&mut svg, i);
        }
    }

    // Attached/floating notes. PlantUML lays each note out as a
    // `<g class="entity">` with an auto-generated `GMN*` qualified name and a
    // box-plus-leader path; we reconstruct that path locally from the box
    // rectangle and leader apex captured by the oracle. Notes interleave with
    // the top-level entities in source-line order, and links are emitted last.
    //
    // The oracle also stashes each note's path-based shape in `entities`, so we
    // can't use `entities` keys to tell notes apart — instead skip notes whose
    // qualified name matches a declared diagram node.
    let top_notes: Vec<&crate::layout_oracle::OracleNoteEntity> = if let Some(orc) = oracle {
        let mut node_qnames: std::collections::HashSet<String> = std::collections::HashSet::new();
        for a in &diagram.actors {
            node_qnames.insert(a.id.clone());
            node_qnames.insert(a.label.clone());
        }
        for uc in &diagram.use_cases {
            node_qnames.insert(qualified_name(&uc.id, diagram));
            node_qnames.insert(uc.id.clone());
            node_qnames.insert(uc.label.clone());
        }
        for p in &diagram.packages {
            node_qnames.insert(p.name.clone());
        }
        orc.note_entities
            .iter()
            .filter(|n| !node_qnames.contains(n.qualified_name.as_str()))
            .collect()
    } else {
        Vec::new()
    };

    // Top-level (non-member) entities and notes, interleaved by source line.
    // Kind: 0 = actor, 1 = use case, 2 = note.
    let mut top: Vec<(usize, u8, usize)> = Vec::new();
    for (i, a) in diagram.actors.iter().enumerate() {
        if !member_ids.contains(a.id.as_str()) {
            top.push((a.source_line, 0, i));
        }
    }
    for (i, u) in diagram.use_cases.iter().enumerate() {
        if !member_ids.contains(u.id.as_str()) {
            top.push((u.source_line, 1, i));
        }
    }
    for (i, n) in top_notes.iter().enumerate() {
        let line = n
            .source_line
            .as_deref()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        top.push((line, 2, i));
    }
    // Stable sort by source line; on ties keep declaration order (notes after
    // their target on the same conceptual line never collide in practice).
    top.sort_by_key(|m| m.0);
    for (_, kind, i) in top {
        match kind {
            0 => render_actor_i(&mut svg, i),
            1 => render_uc_i(&mut svg, i),
            _ => emit_note(&mut svg, top_notes[i]),
        }
    }

    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc);
    }

    render_footer(&mut svg, diagram, total_h);

    svg.finalize_plantuml()
}

/// Render a `header` directive as `<g class="header"><text>…</text></g>`.
fn render_header(svg: &mut SvgBuilder, diagram: &UseCaseDiagram) {
    let Some(header) = &diagram.meta.header else {
        return;
    };
    svg.raw(r#"<g class="header" data-source-line="1">"#);
    let tw = text_render::measure(header, 10.0, false);
    let _ = tw;
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        header,
        &TextBase {
            x: 0.0,
            y: 9.668,
            font_size: 10,
            font_family: "sans-serif",
            fill: "#888888",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

/// Render a `footer` directive as `<g class="footer"><text>…</text></g>`.
fn render_footer(svg: &mut SvgBuilder, diagram: &UseCaseDiagram, total_h: f64) {
    let Some(footer) = &diagram.meta.footer else {
        return;
    };
    svg.raw(r#"<g class="footer" data-source-line="1">"#);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        footer,
        &TextBase {
            x: 0.0,
            y: total_h - 9.0241,
            font_size: 10,
            font_family: "sans-serif",
            fill: "#888888",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

/// Render a `title` directive as `<g class="title"><text>…</text></g>`.
fn render_title(svg: &mut SvgBuilder, diagram: &UseCaseDiagram, total_w: f64) {
    let Some(title) = &diagram.meta.title else {
        return;
    };
    svg.raw(r#"<g class="title" data-source-line="1">"#);
    let tw = text_render::measure(title, FONT_SIZE, true);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        title,
        &TextBase {
            // PlantUML centres the title over the content area, which is inset
            // by one MARGIN from the right canvas edge.
            x: (total_w - MARGIN - tw) / 2.0,
            y: 23.5352,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

/// Assign PlantUML-compatible entity IDs by sorting actors, use cases, and
/// packages by `source_line` and numbering sequentially from `ent0002`.
///
/// PlantUML draws entity *and* link uids from a single monotonic counter in
/// source-line order, so a connection declared between two entity declarations
/// consumes a counter slot (it becomes a `lnk` id) and pushes later entities to
/// higher `ent` numbers. We model this by interleaving connections as
/// slot-consuming entries that produce no `ent` mapping.
fn build_entity_id_map(diagram: &UseCaseDiagram) -> HashMap<String, String> {
    struct Entry {
        /// `None` for a connection (consumes a counter slot but emits no `ent` id).
        key: Option<String>,
        line: usize,
    }
    let mut entries: Vec<Entry> = Vec::new();
    for a in &diagram.actors {
        entries.push(Entry {
            key: Some(format!("actor::{}", a.id)),
            line: a.source_line,
        });
    }
    for uc in &diagram.use_cases {
        entries.push(Entry {
            key: Some(format!("uc::{}", uc.id)),
            line: uc.source_line,
        });
    }
    for c in &diagram.connections {
        entries.push(Entry {
            key: None,
            line: c.source_line,
        });
    }
    for p in &diagram.packages {
        let line = p.source_line;
        entries.push(Entry {
            key: Some(format!("pkg::{}", p.name)),
            line,
        });
    }
    entries.sort_by_key(|e| e.line);
    let mut map = HashMap::new();
    for (counter, e) in (2usize..).zip(entries) {
        if let Some(key) = e.key {
            map.insert(key, format!("ent{counter:04}"));
        }
    }
    map
}

struct ActorDim {
    label_w: f64,
    stereo_w: f64,
}

struct UseCaseDim {
    label_w: f64,
    stereo_w: f64,
    line_count: usize,
    rx: f64,
    ry: f64,
}

fn actor_dim(actor: &Actor) -> ActorDim {
    let label_w = text_render::measure(&actor.label, FONT_SIZE, false);
    let stereo_w = actor
        .stereotype
        .as_ref()
        .map(|s| text_render::measure(&format!("\u{00AB}{s}\u{00BB}"), STEREO_FONT, false))
        .unwrap_or(0.0);
    ActorDim { label_w, stereo_w }
}

fn use_case_dim(uc: &UseCase) -> UseCaseDim {
    let label_w = text_render::measure(&uc.label, FONT_SIZE, false);
    let stereo_w = uc
        .stereotype
        .as_ref()
        .map(|s| text_render::measure(&format!("\u{00AB}{s}\u{00BB}"), STEREO_FONT, false))
        .unwrap_or(0.0);
    let desc_max_w = uc
        .description
        .iter()
        .map(|d| text_render::measure(d, FONT_SIZE, false))
        .fold(0.0_f64, f64::max);
    let max_w = label_w.max(stereo_w).max(desc_max_w);
    let line_count = uc.description.len().max(1) + if uc.stereotype.is_some() { 1 } else { 0 };
    let rx = max_w / 2.0 + UC_RX_PAD;
    let ry = (line_count as f64 * LINE_H) / 2.0 + UC_RY_PAD - FONT_SIZE / 2.0;
    UseCaseDim {
        label_w,
        stereo_w,
        line_count,
        rx,
        ry,
    }
}

struct Positions {
    actors: Vec<(f64, f64)>,
    use_cases: Vec<(f64, f64)>,
}

fn resolve_positions(
    diagram: &UseCaseDiagram,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    oracle: Option<&OracleLayout>,
) -> Positions {
    if let Some(orc) = oracle {
        let actors = diagram
            .actors
            .iter()
            .enumerate()
            .map(|(i, a)| {
                lookup_actor_center(orc, a)
                    .unwrap_or_else(|| fallback_actor_center(i, &actor_dims[i]))
            })
            .collect();
        let use_cases = diagram
            .use_cases
            .iter()
            .enumerate()
            .map(|(i, uc)| {
                lookup_use_case_center(orc, uc, diagram)
                    .unwrap_or_else(|| fallback_use_case_center(i, &uc_dims[i]))
            })
            .collect();
        return Positions { actors, use_cases };
    }
    let actors: Vec<(f64, f64)> = actor_dims
        .iter()
        .enumerate()
        .map(|(i, d)| fallback_actor_center(i, d))
        .collect();
    let use_cases: Vec<(f64, f64)> = uc_dims
        .iter()
        .enumerate()
        .map(|(i, d)| fallback_use_case_center(i, d))
        .collect();
    Positions { actors, use_cases }
}

fn lookup_actor_center(oracle: &OracleLayout, actor: &Actor) -> Option<(f64, f64)> {
    let rect = oracle
        .entities
        .get(&actor.id)
        .or_else(|| oracle.entities.get(&actor.label))?;
    Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
}

fn lookup_use_case_center(
    oracle: &OracleLayout,
    uc: &UseCase,
    diagram: &UseCaseDiagram,
) -> Option<(f64, f64)> {
    let qualified = qualified_name(&uc.id, diagram);
    let rect = oracle
        .entities
        .get(&qualified)
        .or_else(|| oracle.entities.get(&uc.id))
        .or_else(|| oracle.entities.get(&uc.label))?;
    Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
}

/// PlantUML strips spaces and punctuation when deriving an entity id from a
/// quoted label (mirrors the parser's `label_to_id`).
fn label_to_id(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// The name PlantUML emits as `data-qualified-name` and keys oracle entities
/// by. For an aliased use case (`usecase "X" as UC1`) this is the alias/id; for
/// a label-declared one (`usecase "Primary Action"`) it is the original label,
/// spaces and all.
fn display_name(uc: &UseCase) -> &str {
    if uc.id == label_to_id(&uc.label) {
        &uc.label
    } else {
        &uc.id
    }
}

fn qualified_name(id: &str, diagram: &UseCaseDiagram) -> String {
    // Resolve the display name for use cases (label-declared ones use their
    // label, not the space-stripped id).
    let display = diagram
        .use_cases
        .iter()
        .find(|u| u.id == id)
        .map(display_name)
        .unwrap_or(id);
    for pkg in &diagram.packages {
        if pkg.elements.iter().any(|e| e == id) {
            return format!("{}.{display}", pkg.name);
        }
    }
    display.to_string()
}

fn fallback_actor_center(i: usize, _dim: &ActorDim) -> (f64, f64) {
    let cx = MARGIN + ACTOR_HEAD_R;
    let cy = MARGIN + ACTOR_HEAD_R + i as f64 * (ACTOR_HEAD_R * 2.0 + ACTOR_BODY_LEN + GAP);
    (cx, cy)
}

fn fallback_use_case_center(i: usize, dim: &UseCaseDim) -> (f64, f64) {
    let cx = MARGIN + 80.0 + dim.rx;
    let cy = MARGIN + dim.ry + i as f64 * (dim.ry * 2.0 + GAP);
    (cx, cy)
}

fn compute_canvas(
    positions: &Positions,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
) -> (f64, f64) {
    let mut max_x: f64 = 100.0;
    let mut max_y: f64 = 50.0;
    for (i, (cx, cy)) in positions.actors.iter().enumerate() {
        let half = actor_dims[i].label_w.max(ACTOR_ARM_HALF * 2.0) / 2.0;
        max_x = max_x.max(cx + half + MARGIN);
        max_y = max_y.max(
            cy + ACTOR_HEAD_R + ACTOR_BODY_LEN + ACTOR_LEG_DROP + ACTOR_LABEL_GAP * 2.0 + MARGIN,
        );
    }
    for (i, (cx, cy)) in positions.use_cases.iter().enumerate() {
        max_x = max_x.max(cx + uc_dims[i].rx + MARGIN);
        max_y = max_y.max(cy + uc_dims[i].ry + MARGIN);
    }
    (max_x, max_y)
}

fn render_package_group(
    svg: &mut SvgBuilder,
    pkg: &UseCasePackage,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
) {
    let Some(orc) = oracle else { return };
    let Some(rect) = orc.entities.get(&pkg.name) else {
        return;
    };
    let ent_id = id_map
        .get(&format!("pkg::{}", pkg.name))
        .cloned()
        .unwrap_or_else(|| "ent0003".to_string());
    let src_attr = source_line_attr(pkg.source_line);
    let fill = pkg
        .color
        .as_deref()
        .map(resolve_fill)
        .unwrap_or_else(|| "none".to_string());
    svg.raw(&format!("<!--cluster {}-->", pkg.name));
    svg.raw(&format!(
        r#"<g class="cluster" data-qualified-name="{}"{src_attr} id="{ent_id}">"#,
        pkg.name
    ));
    let label_w = text_render::measure(&pkg.name, FONT_SIZE, true);
    let (label_x, label_y) = match pkg.kind {
        PackageKind::Rectangle => {
            // Plain rounded rect, centred bold label.
            svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
                h = fc(rect.height),
                w = fc(rect.width),
                x = fc(rect.x),
                y = fc(rect.y),
            ));
            (rect.x + (rect.width - label_w) / 2.0, rect.y + 15.5352)
        }
        PackageKind::Package => {
            // Folder-tab outline: a notched top-left "tab" carrying the label,
            // a diagonal slope down to the body's top edge, then a rounded
            // rectangle body. Reconstructed from the oracle box rect and label
            // width (HALF_UP coords).
            let x = rect.x;
            let y = rect.y;
            let xr = rect.x + rect.width;
            let yb = rect.y + rect.height;
            // Tab top-right corner: label start (x+4) + label width, less 0.5.
            let tab_tr = x + 3.5 + label_w;
            let tab_y = y + pm::text_height(FONT_SIZE) + 6.0;
            let d = format!(
                "M{x25},{y_s} L{tab_tr},{y_s} A3.75,3.75 0 0 1 {tab_tr25},{y25} L{tab_br},{ty} L{xr25},{ty} A2.5,2.5 0 0 1 {xr_s},{ty25} L{xr_s},{yb2} A2.5,2.5 0 0 1 {xr25},{yb_s} L{x25},{yb_s} A2.5,2.5 0 0 1 {x_s},{yb2} L{x_s},{y25} A2.5,2.5 0 0 1 {x25},{y_s}",
                x25 = fc(x + 2.5),
                y_s = fc(y),
                tab_tr = fc(tab_tr),
                tab_tr25 = fc(tab_tr + 2.5),
                y25 = fc(y + 2.5),
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
                r#"<path d="{d}" fill="{fill}" style="stroke:#000000;stroke-width:1.5;"/>"#,
            ));
            // Divider under the tab from the left edge to the slope end.
            svg.raw(&format!(
                r#"<line style="stroke:#000000;stroke-width:1.5;" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
                x1 = fc(x),
                x2 = fc(tab_tr + 9.5),
                ty = fc(tab_y),
            ));
            (x + 4.0, y + 15.5352)
        }
    };
    // Prefer the oracle-captured label baseline (avoids sub-pixel drift from
    // recomputing `rect.y + offset` against the already-rounded oracle rect).
    let label_x = rect.text_x_values.first().copied().unwrap_or(label_x);
    let label_y = rect.text_y_values.first().copied().unwrap_or(label_y);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        &pkg.name,
        &TextBase {
            x: label_x,
            y: label_y,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

#[allow(clippy::too_many_arguments)]
fn render_actor(
    svg: &mut SvgBuilder,
    actor: &Actor,
    dim: &ActorDim,
    cx: f64,
    cy: f64,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
    skin: &SkinColors,
) {
    // Prefer the oracle-captured entity id (PlantUML's real counter allocation,
    // which notes and other synthetic entities perturb), falling back to the
    // source-line-derived map when no oracle is present.
    let oracle_id = oracle.and_then(|orc| {
        orc.entities
            .get(&actor.id)
            .or_else(|| orc.entities.get(&actor.label))
            .and_then(|r| r.entity_id.clone())
    });
    let ent_id = oracle_id
        .or_else(|| id_map.get(&format!("actor::{}", actor.id)).cloned())
        .unwrap_or_else(|| "ent0002".to_string());
    // For a label-declared actor (`actor "External System"`) PlantUML keys the
    // qualified name and comment on the original label (spaces and all); for an
    // aliased/bare actor it uses the id.
    let display = if actor.id == label_to_id(&actor.label) {
        actor.label.as_str()
    } else {
        actor.id.as_str()
    };
    svg.raw(&format!("<!--entity {display}-->"));
    let src_attr = source_line_attr(actor.source_line);
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{display}"{src_attr} id="{ent_id}">"#,
    ));
    // Per-element `#color` overrides skinparam; both override the default.
    let fill = actor
        .color
        .as_deref()
        .map(resolve_fill)
        .or_else(|| skin.actor_fill.clone())
        .unwrap_or_else(|| ENTITY_FILL.to_string());
    let stroke = skin.actor_border.as_deref().unwrap_or(STROKE);
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{ACTOR_HEAD_R}" ry="{ACTOR_HEAD_R}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
        cx = fc(cx),
        cy = fc(cy),
    ));
    let body_top_y = cy + ACTOR_HEAD_R;
    let body_bot_y = body_top_y + ACTOR_BODY_LEN;
    let arm_y = body_top_y + ACTOR_ARM_OFFSET;
    let leg_x_left = cx - ACTOR_LEG_RUN;
    let leg_x_right = cx + ACTOR_LEG_RUN;
    let leg_y = body_bot_y + ACTOR_LEG_DROP;
    let arm_left_x = cx - ACTOR_ARM_HALF;
    let arm_right_x = cx + ACTOR_ARM_HALF;
    svg.raw(&format!(
        r#"<path d="M{cx},{body_top_y} L{cx},{body_bot_y} M{arm_left_x},{arm_y} L{arm_right_x},{arm_y} M{cx},{body_bot_y} L{leg_x_left},{leg_y} M{cx},{body_bot_y} L{leg_x_right},{leg_y}" fill="none" style="stroke:{stroke};stroke-width:0.5;"/>"#,
        cx = fc(cx),
        body_top_y = fc(body_top_y),
        body_bot_y = fc(body_bot_y),
        arm_left_x = fc(arm_left_x),
        arm_right_x = fc(arm_right_x),
        arm_y = fc(arm_y),
        leg_x_left = fc(leg_x_left),
        leg_x_right = fc(leg_x_right),
        leg_y = fc(leg_y),
    ));
    let cx_anchor = round_coord(cx);
    // Prefer PlantUML's captured per-line text x (label first, stereotype
    // second in document order) over reconstructing it from the rounded centre.
    let orc_rect = oracle.and_then(|orc| {
        orc.entities
            .get(&actor.id)
            .or_else(|| orc.entities.get(&actor.label))
    });
    let captured_x = orc_rect.map(|r| r.text_x_values.as_slice()).unwrap_or(&[]);
    let captured_y = orc_rect.map(|r| r.text_y_values.as_slice()).unwrap_or(&[]);
    let label_x = captured_x
        .first()
        .copied()
        .unwrap_or(cx_anchor - dim.label_w / 2.0);
    let label_y = captured_y
        .first()
        .copied()
        .unwrap_or(leg_y + ACTOR_LABEL_GAP);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        &actor.label,
        &TextBase {
            x: label_x,
            y: label_y,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    if let Some(stereo) = &actor.stereotype {
        let stereo_text = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_x = captured_x
            .get(1)
            .copied()
            .unwrap_or(cx_anchor - dim.stereo_w / 2.0);
        let stereo_y = captured_y
            .get(1)
            .copied()
            .unwrap_or(cy - ACTOR_STEREO_OFFSET);
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &stereo_text,
            &TextBase {
                x: stereo_x,
                y: stereo_y,
                font_size: STEREO_FONT as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    }
    svg.raw("</g>");
}

#[allow(clippy::too_many_arguments)]
fn render_use_case(
    svg: &mut SvgBuilder,
    uc: &UseCase,
    dim: &UseCaseDim,
    diagram: &UseCaseDiagram,
    cx: f64,
    cy: f64,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
    skin: &SkinColors,
) {
    let qualified = qualified_name(&uc.id, diagram);
    // Prefer the oracle-captured entity id (PlantUML's real counter allocation),
    // falling back to the source-line-derived map when no oracle is present.
    let oracle_id = oracle.and_then(|orc| {
        orc.entities
            .get(&qualified)
            .or_else(|| orc.entities.get(&uc.id))
            .or_else(|| orc.entities.get(&uc.label))
            .and_then(|r| r.entity_id.clone())
    });
    let ent_id = oracle_id
        .or_else(|| id_map.get(&format!("uc::{}", uc.id)).cloned())
        .unwrap_or_else(|| "ent0003".to_string());
    svg.raw(&format!("<!--entity {}-->", display_name(uc)));
    let src_attr = source_line_attr(uc.source_line);
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qualified}"{src_attr} id="{ent_id}">"#,
    ));
    let orc_rect = oracle.and_then(|orc| {
        orc.entities
            .get(&qualified)
            .or_else(|| orc.entities.get(&uc.id))
            .or_else(|| orc.entities.get(&uc.label))
    });
    let (rx, ry) = if let Some(rect) = orc_rect {
        (rect.width / 2.0, rect.height / 2.0)
    } else {
        (dim.rx, dim.ry)
    };
    // Fill precedence: per-element `#color` > stereotype-scoped skinparam
    // (`usecaseBackgroundColor<<stereo>>`) > generic `usecaseBackgroundColor` >
    // default.
    let stereo_fill = uc.stereotype.as_deref().and_then(|s| {
        skin_color(
            &diagram.meta.skinparams,
            &format!("usecaseBackgroundColor<<{s}>>"),
        )
    });
    let fill = uc
        .color
        .as_deref()
        .map(resolve_fill)
        .or(stereo_fill)
        .or_else(|| skin.uc_fill.clone())
        .unwrap_or_else(|| ENTITY_FILL.to_string());
    let stroke = skin.uc_border.as_deref().unwrap_or(STROKE);
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{rx}" ry="{ry}" style="stroke:{stroke};stroke-width:0.5;"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        rx = fc(rx),
        ry = fc(ry),
    ));
    let cx_anchor = round_coord(cx);
    // PlantUML's emitted text x values are captured per line (stereotype first,
    // then label/description lines). Reconstructing them from the 4-dp-rounded
    // ellipse centre loses sub-pixel precision, so prefer the captured value and
    // fall back to the geometric centre only when no oracle x is available.
    let captured_x = orc_rect.map(|r| r.text_x_values.as_slice()).unwrap_or(&[]);
    let captured_y = orc_rect.map(|r| r.text_y_values.as_slice()).unwrap_or(&[]);
    let mut line_idx = 0usize;
    let n_lines = dim.line_count;
    let bottom_y = cy + UC_TEXT_OFFSET_SINGLE + (n_lines as f64 - 1.0) * LINE_H;
    let mut text_y = bottom_y - (n_lines as f64 - 1.0) * LINE_H;
    if dim.line_count >= 2 && uc.stereotype.is_some() {
        text_y = cy + 0.0304;
    }
    if let Some(stereo) = &uc.stereotype {
        let stereo_text = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_x = captured_x
            .get(line_idx)
            .copied()
            .unwrap_or(cx_anchor - dim.stereo_w / 2.0);
        let stereo_y = captured_y.get(line_idx).copied().unwrap_or(text_y);
        line_idx += 1;
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &stereo_text,
            &TextBase {
                x: stereo_x,
                y: stereo_y,
                font_size: STEREO_FONT as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
        text_y += LINE_H;
    }
    if uc.description.is_empty() {
        let label_x = captured_x
            .get(line_idx)
            .copied()
            .unwrap_or(cx_anchor - dim.label_w / 2.0);
        let label_y = captured_y.get(line_idx).copied().unwrap_or(text_y);
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &uc.label,
            &TextBase {
                x: label_x,
                y: label_y,
                font_size: FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    } else {
        // Separator dividers (from `--`/`==`/`..` lines in a multiline label)
        // are captured verbatim from the oracle (full geometry + style, so
        // dashed `..` rules and `==` double rules survive) and interleaved with
        // the text lines by y-position: each divider is flushed before the first
        // text line whose baseline sits below it. Fall back to the geometry-only
        // `sep_lines` when the styled capture is unavailable.
        let sep_styled = orc_rect.map(|r| r.lines.as_slice()).unwrap_or(&[]);
        let sep_geom = orc_rect.map(|r| r.sep_lines.as_slice()).unwrap_or(&[]);
        let use_styled = !sep_styled.is_empty();
        let mut sep_idx = 0usize;
        let flush_seps = |svg: &mut SvgBuilder, sep_idx: &mut usize, before_y: f64| {
            if use_styled {
                while let Some(l) = sep_styled.get(*sep_idx) {
                    let y1: f64 = l.y1.parse().unwrap_or(0.0);
                    if y1 < before_y {
                        let style = l
                            .style
                            .as_deref()
                            .unwrap_or("stroke:#181818;stroke-width:1;");
                        svg.raw(&format!(
                            r#"<line style="{style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            l.x1, l.x2, l.y1, l.y2,
                        ));
                        *sep_idx += 1;
                    } else {
                        break;
                    }
                }
            } else {
                while let Some(&(x1, x2, y1)) = sep_geom.get(*sep_idx) {
                    if y1 < before_y {
                        svg.raw(&format!(
                            r#"<line style="stroke:{STROKE};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            fc(x1),
                            fc(x2),
                            fc(y1),
                            fc(y1),
                        ));
                        *sep_idx += 1;
                    } else {
                        break;
                    }
                }
            }
        };
        for line in &uc.description {
            let lw = text_render::measure(line, FONT_SIZE, false);
            let lx = captured_x
                .get(line_idx)
                .copied()
                .unwrap_or(cx_anchor - lw / 2.0);
            let ly = captured_y.get(line_idx).copied().unwrap_or(text_y);
            flush_seps(svg, &mut sep_idx, ly);
            line_idx += 1;
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                line,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
            text_y += LINE_H;
        }
        // Any trailing separators after the last text line.
        flush_seps(svg, &mut sep_idx, f64::INFINITY);
    }
    svg.raw("</g>");
}

/// Render a note entity, reconstructing the box-plus-leader path locally from
/// the oracle-captured geometry (box rect, leader apex/base, text baselines).
fn emit_note(svg: &mut SvgBuilder, note: &crate::layout_oracle::OracleNoteEntity) {
    use std::fmt::Write;

    let Some(g) = note.box_geom.as_ref() else {
        return;
    };
    let bx = g.x;
    let by = g.y;
    let right = g.x + g.width;
    let bottom = g.y + g.height;
    let rf = right - NOTE_FOLD; // fold inner x
    let yf = by + NOTE_FOLD; // fold inner y

    // The leader sits on whichever box edge the apex points toward.
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

    // PlantUML does not place the leader base points symmetrically about the
    // apex, so emit them verbatim from the captured geometry.
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

    // Walk the outline counter-clockwise from the top-left corner, splicing the
    // leader into the appropriate edge.
    let mut d = String::new();
    let _ = write!(d, "M{},{} ", fc(bx), fc(by));
    if matches!(side, Some(LeaderSide::Left)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(bx), fc(bottom));
    let _ = write!(d, "A0,0 0 0 0 {},{} ", fc(bx), fc(bottom));
    if matches!(side, Some(LeaderSide::Bottom)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(right), fc(bottom));
    let _ = write!(d, "A0,0 0 0 0 {},{} ", fc(right), fc(bottom));
    if matches!(side, Some(LeaderSide::Right)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(right), fc(yf));
    let _ = write!(d, "L{},{} ", fc(rf), fc(by));
    if matches!(side, Some(LeaderSide::Top)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(bx), fc(by));
    let _ = write!(d, "A0,0 0 0 0 {},{}", fc(bx), fc(by));

    let src_attr = note
        .source_line
        .as_deref()
        .map(|sl| format!(r#" data-source-line="{sl}""#))
        .unwrap_or_default();
    let ent_id = note.entity_id.as_deref().unwrap_or("");
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qn}"{src_attr} id="{ent_id}">"#,
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
    // Text lines at their captured baselines.
    for (tx, ty, line) in &g.text_lines {
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            line,
            &TextBase {
                x: *tx,
                y: *ty,
                font_size: NOTE_FONT_SIZE,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    }
    svg.raw("</g>");
}

fn source_line_attr(source_line: usize) -> String {
    if source_line == 0 {
        String::new()
    } else {
        format!(r#" data-source-line="{source_line}""#)
    }
}

fn entity_label<'a>(diagram: &'a UseCaseDiagram, id: &'a str) -> &'a str {
    if let Some(a) = diagram.actors.iter().find(|a| a.id == id) {
        return a.label.as_str();
    }
    if let Some(u) = diagram.use_cases.iter().find(|u| u.id == id) {
        return u.label.as_str();
    }
    id
}

fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &UseCaseDiagram,
    oracle: &OracleLayout,
) {
    // PlantUML emits links sorted by source line. The parser already stores
    // connections in declaration order, but sort defensively.
    let mut conns: Vec<&UseCaseConnection> = diagram.connections.iter().collect();
    conns.sort_by_key(|c| c.source_line);
    for conn in &conns {
        let conn: &UseCaseConnection = conn;
        // Oracle edge ids are built from the label form, not the id form.
        let from_label = entity_label(diagram, &conn.from);
        let to_label = entity_label(diagram, &conn.to);
        let candidates = [
            format!("{from_label}-to-{to_label}"),
            format!("{from_label}-{to_label}"),
            format!("{from_label}-backto-{to_label}"),
            format!("{}-to-{}", conn.from, conn.to),
            format!("{}-{}", conn.from, conn.to),
            format!("{}-backto-{}", conn.from, conn.to),
        ];
        let oracle_edge = oracle
            .edges
            .iter()
            .find(|e| candidates.iter().any(|c| c == &e.id));
        let Some(oracle_edge) = oracle_edge else {
            continue;
        };
        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("association");
        let source_line = oracle_edge.source_line.as_deref();
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");
        let source_attr = source_line
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();
        svg.raw(&format!("<!--link {from_label} to {to_label}-->"));
        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}"{source_attr} id="{link_id}">"#,
        ));
        let path_style = oracle_edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let code_line_attr = oracle_edge
            .code_line
            .as_ref()
            .map(|c| format!(r#" codeLine="{c}""#))
            .unwrap_or_default();
        svg.raw(&format!(
            r#"<path{code_line_attr} d="{}" fill="none" id="{}" style="{path_style}"/>"#,
            oracle_edge.d, oracle_edge.id,
        ));
        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }
        if let Some(ref points) = oracle_edge.second_arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }
        if let Some((lx, ly, ref text)) = oracle_edge.label {
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                text,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: 13,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
        }
        svg.raw("</g>");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nactor User\nusecase \"Login\" as UC1\nUser --> UC1\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("User"));
        assert!(svg.contains("Login"));
    }
}
