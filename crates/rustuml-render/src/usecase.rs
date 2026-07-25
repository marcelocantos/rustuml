// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's use-case diagram rendering.
//! PlantUML emits use-case diagrams as `data-diagram-type="DESCRIPTION"` —
//! the same envelope used by component and deployment diagrams.

use std::collections::HashMap;
use std::fmt::Write as _;

use rustuml_layout::graph::{ClusterPosition, Direction, EdgeLabelSize, EdgePath, LayoutGraph};
use rustuml_parser::diagram::usecase::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

const FONT_SIZE: f64 = 14.0;
const STROKE: &str = "#181818";
const ENTITY_FILL: &str = "#F1F1F1";
const TEXT_COLOR: &str = "#000000";

const ACTOR_HEAD_R: f64 = 8.0;
const ACTOR_BODY_LEN: f64 = 27.0;
const ACTOR_ARM_HALF: f64 = 13.0;
const ACTOR_ARM_OFFSET: f64 = 8.0;
const ACTOR_LEG_RUN: f64 = 13.0;
const ACTOR_LEG_DROP: f64 = 15.0;
/// `EntityImageDescription` wraps actor stereotypes with
/// `TextBlockUtils.withMargin(stereotype, 1, 0)`.
const ACTOR_STEREOTYPE_MARGIN_X: f64 = 1.0;
/// Java provenance: `skin.ActorStickMan.getPreferredHeight()` adds twice the
/// current stroke thickness to this 58px geometry plus its final 1px guard.
const ACTOR_STICKMAN_BASE_HEIGHT: f64 = 59.0;
/// Vertical offset from head centre to stereotype baseline (measured).
const ACTOR_STEREO_OFFSET: f64 = 11.4531;
const LINE_H: f64 = 16.4883;

const MARGIN: f64 = 7.0;
const GAP: f64 = 40.0;
const BODY_MARGIN: f64 = 6.0;
/// Java's `EntityImageDegenerated` wraps a lone non-state entity in 7px.
const DEGENERATED_MARGIN: f64 = 7.0;
const SVEK_CANVAS_PAD: f64 = 14.0;
const LAYOUT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const DEPENDENCY_ARROW_BACK: f64 = 9.0;
const DEPENDENCY_ARROW_NOTCH: f64 = 5.0;
const DEPENDENCY_ARROW_WING: f64 = 4.0;
const DEPENDENCY_ARROW_PATH_GAP: f64 = 6.0;

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_FOLD: f64 = 10.0;
const NOTE_FONT_SIZE: u32 = 13;
/// Java provenance: `EntityImageNote` and `Opale` use 6px left, 15px right,
/// and 5px vertical text margins.
const NOTE_MARGIN_LEFT: f64 = 6.0;
const NOTE_MARGIN_RIGHT: f64 = 15.0;
const NOTE_MARGIN_Y: f64 = 5.0;
/// Java provenance: `Opale.delta` makes every leader base eight pixels wide.
const NOTE_LEADER_HALF: f64 = 4.0;
/// Java provenance: `Rose` gives link-owned note components 5px padding.
const LINK_NOTE_PADDING: f64 = 5.0;

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
    let normalized = crate::sequence::resolve_color(raw);
    if normalized.starts_with('#') {
        normalized
    } else {
        format!("#{normalized}")
    }
}

fn skin_value<'a>(
    skinparams: &'a [rustuml_parser::diagram::SkinParam],
    keys: &[&str],
) -> Option<&'a str> {
    skinparams
        .iter()
        .rev()
        .find(|p| keys.iter().any(|k| p.key.eq_ignore_ascii_case(k)))
        .map(|p| p.value.trim())
}

/// Look up a skinparam value case-insensitively (PlantUML convention) and
/// resolve it to a fill string. The parser flattens block skinparams like
/// `skinparam usecase { BackgroundColor X }` to the key `usecaseBackgroundColor`.
fn skin_color(skinparams: &[rustuml_parser::diagram::SkinParam], key: &str) -> Option<String> {
    skin_value(skinparams, &[key]).map(resolve_fill)
}

fn skin_fill(
    skinparams: &[rustuml_parser::diagram::SkinParam],
    key: &str,
    gradient_defs: Option<&str>,
) -> Option<String> {
    skin_value(skinparams, &[key]).map(|v| crate::sequence::gradient_fill_or(v, gradient_defs))
}

fn skin_font_size(
    skinparams: &[rustuml_parser::diagram::SkinParam],
    keys: &[&str],
    default: u32,
) -> u32 {
    skin_value(skinparams, keys)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.round() as u32)
        .unwrap_or(default)
}

fn skin_thickness(
    skinparams: &[rustuml_parser::diagram::SkinParam],
    keys: &[&str],
    default: f64,
) -> String {
    skin_value(skinparams, keys)
        .and_then(|v| v.parse::<f64>().ok())
        .map(fc)
        .unwrap_or_else(|| fc(default))
}

fn canonical_usecase_font_family(value: &str) -> String {
    let raw = value.trim();
    let quoted = (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''));
    let trimmed = raw.trim_matches('"').trim_matches('\'');
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("sansserif")
        || trimmed.eq_ignore_ascii_case("sans-serif")
    {
        "sans-serif".to_string()
    } else if quoted {
        format!("'{trimmed}'")
    } else {
        trimmed.to_string()
    }
}

/// Per-kind background/border/text defaults derived from `skinparam`
/// directives. Keep this opt-in: absent skinparams preserve the renderer's
/// existing PlantUML defaults instead of inheriting RustUML's UI theme.
struct SkinColors {
    actor_fill: Option<String>,
    actor_border: Option<String>,
    actor_border_thickness: String,
    actor_font_color: String,
    actor_font_family: String,
    actor_font_size: u32,
    actor_stereo_font_color: String,
    uc_fill: Option<String>,
    uc_border: Option<String>,
    uc_border_thickness: String,
    uc_font_color: String,
    uc_font_family: String,
    uc_font_size: u32,
    uc_stereo_font_color: String,
    arrow_font_color: String,
    arrow_font_family: String,
    arrow_font_size: u32,
    canvas_background: Option<String>,
    canvas_rect: Option<String>,
    gradient_defs: Option<String>,
}

impl SkinColors {
    fn from_meta(
        skinparams: &[rustuml_parser::diagram::SkinParam],
        gradient_defs: Option<&str>,
    ) -> Self {
        let default_font_family = skin_value(skinparams, &["defaultFontName", "fontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| "sans-serif".to_string());
        let default_font_size = skin_font_size(skinparams, &["defaultFontSize"], FONT_SIZE as u32);
        let root_font_color = skin_color(skinparams, "__styleRootFontColor");
        let default_font_color = skin_color(skinparams, "defaultFontColor");
        let fallback_font_color = || {
            root_font_color
                .clone()
                .or_else(|| default_font_color.clone())
                .unwrap_or_else(|| TEXT_COLOR.to_string())
        };
        let actor_font_color = skin_color(skinparams, "actorFontColor")
            .or_else(|| Some(fallback_font_color()))
            .unwrap_or_else(|| TEXT_COLOR.to_string());
        let uc_font_color = skin_color(skinparams, "usecaseFontColor")
            .or_else(|| Some(fallback_font_color()))
            .unwrap_or_else(|| TEXT_COLOR.to_string());
        let actor_font_family = skin_value(skinparams, &["actorFontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| default_font_family.clone());
        let uc_font_family = skin_value(skinparams, &["usecaseFontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| default_font_family.clone());
        let arrow_font_family = skin_value(skinparams, &["arrowFontName", "usecaseArrowFontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| default_font_family.clone());
        let root_line = ["__styleRootLineThickness", "borderThickness"];
        let bg_value = skin_value(skinparams, &["backgroundColor"]);
        let canvas_background = match bg_value {
            Some(v) if v.eq_ignore_ascii_case("transparent") => None,
            Some(v) => Some(crate::sequence::resolve_color(v)),
            None => Some("#FFFFFF".to_string()),
        };
        let canvas_rect = canvas_background
            .as_ref()
            .filter(|c| *c != "#FFFFFF")
            .cloned();
        SkinColors {
            actor_fill: skin_fill(skinparams, "actorBackgroundColor", gradient_defs),
            actor_border: skin_color(skinparams, "actorBorderColor")
                .or_else(|| skin_color(skinparams, "__styleRootLineColor")),
            actor_border_thickness: skin_thickness(
                skinparams,
                &[
                    "actorBorderThickness",
                    "__styleRootLineThickness",
                    "borderThickness",
                ],
                0.5,
            ),
            actor_font_color: actor_font_color.clone(),
            actor_font_family,
            actor_font_size: skin_font_size(
                skinparams,
                &["actorFontSize", "defaultFontSize"],
                default_font_size,
            ),
            actor_stereo_font_color: skin_color(skinparams, "actorStereotypeFontColor")
                .unwrap_or(actor_font_color),
            uc_fill: skin_fill(skinparams, "usecaseBackgroundColor", gradient_defs),
            uc_border: skin_color(skinparams, "usecaseBorderColor")
                .or_else(|| skin_color(skinparams, "__styleRootLineColor")),
            uc_border_thickness: skin_thickness(
                skinparams,
                &["usecaseBorderThickness", root_line[0], root_line[1]],
                0.5,
            ),
            uc_font_color: uc_font_color.clone(),
            uc_font_family,
            uc_font_size: skin_font_size(
                skinparams,
                &["usecaseFontSize", "defaultFontSize"],
                default_font_size,
            ),
            uc_stereo_font_color: skin_color(skinparams, "usecaseStereotypeFontColor")
                .unwrap_or(uc_font_color),
            arrow_font_color: skin_color(skinparams, "usecaseArrowFontColor")
                .or_else(|| skin_color(skinparams, "arrowFontColor"))
                .unwrap_or_else(|| TEXT_COLOR.to_string()),
            arrow_font_family,
            arrow_font_size: skin_font_size(
                skinparams,
                &["usecaseArrowFontSize", "arrowFontSize"],
                13,
            ),
            canvas_background,
            canvas_rect,
            gradient_defs: gradient_defs.map(str::to_string),
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

    let gradient_defs = oracle
        .map(|o| o.defs_inner_xml.as_str())
        .filter(|d| !d.is_empty());
    let skin = SkinColors::from_meta(&diagram.meta.skinparams, gradient_defs);
    let actor_dims: Vec<ActorDim> = diagram.actors.iter().map(|a| actor_dim(a, &skin)).collect();
    let uc_dims: Vec<UseCaseDim> = diagram
        .use_cases
        .iter()
        .map(|u| use_case_dim(u, &skin))
        .collect();
    let note_dims: Vec<NoteDim> = diagram.notes.iter().map(note_dim).collect();
    let positions = resolve_positions(diagram, &actor_dims, &uc_dims, &note_dims, &skin, oracle);
    let id_map = build_entity_id_map(diagram);

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else {
        compute_canvas(diagram, &positions, &actor_dims, &uc_dims, &note_dims)
    };

    let mut svg = SvgBuilder::new_plantuml_with_background_and_defs(
        total_w,
        total_h,
        "DESCRIPTION",
        skin.canvas_background.as_deref(),
        gradient_defs.unwrap_or(""),
    );
    if let Some(bg) = skin.canvas_rect.as_deref() {
        svg.raw(&format!(
            r#"<rect fill="{bg}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
            h = total_h as i64,
            w = total_w as i64,
        ));
    }

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
        render_package_group(&mut svg, pkg, oracle, &positions.cluster_positions, &id_map);
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
    if oracle.is_none() {
        for (i, note) in diagram.notes.iter().enumerate() {
            if !matches!(note.kind, UseCaseNoteKind::OnLink { .. }) {
                top.push((note.source_line, 3, i));
            }
        }
    }
    // Stable sort by source line; on ties keep declaration order (notes after
    // their target on the same conceptual line never collide in practice).
    top.sort_by_key(|m| m.0);
    for (_, kind, i) in top {
        match kind {
            0 => render_actor_i(&mut svg, i),
            1 => render_uc_i(&mut svg, i),
            2 => emit_note(&mut svg, top_notes[i]),
            _ => {
                if let Some(placement) = positions.notes.get(i).and_then(Option::as_ref) {
                    emit_model_note(
                        &mut svg,
                        &diagram.notes[i],
                        &note_dims[i],
                        placement,
                        i,
                        &id_map,
                    );
                }
            }
        }
    }

    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc, &skin);
    } else {
        render_no_oracle_connections(&mut svg, diagram, &id_map, &positions.edge_paths, &skin);
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

/// Assign PlantUML-compatible entity IDs by sorting declarations and links by
/// `source_line` and numbering sequentially from `ent0002`.
///
/// PlantUML draws entity *and* link uids from a single monotonic counter in
/// source-line order. `CommandFactoryNoteOnEntity` additionally consumes one
/// uid for the generated `GMN*` quark, one for the note entity, and one for its
/// hidden opale link.
fn build_entity_id_map(diagram: &UseCaseDiagram) -> HashMap<String, String> {
    enum EntryKind {
        Entity(String),
        AttachedNote(usize),
        FloatingNote(usize),
        Connection,
    }
    struct Entry {
        kind: EntryKind,
        line: usize,
    }
    let mut entries: Vec<Entry> = Vec::new();
    for a in &diagram.actors {
        entries.push(Entry {
            kind: EntryKind::Entity(format!("actor::{}", a.id)),
            line: a.source_line,
        });
    }
    for uc in &diagram.use_cases {
        entries.push(Entry {
            kind: EntryKind::Entity(format!("uc::{}", uc.id)),
            line: uc.source_line,
        });
    }
    for c in &diagram.connections {
        entries.push(Entry {
            kind: EntryKind::Connection,
            line: c.source_line,
        });
    }
    for p in &diagram.packages {
        entries.push(Entry {
            kind: EntryKind::Entity(format!("pkg::{}", p.name)),
            line: p.source_line,
        });
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        let kind = match note.kind {
            UseCaseNoteKind::Attached { .. } => EntryKind::AttachedNote(index),
            UseCaseNoteKind::Floating { .. } => EntryKind::FloatingNote(index),
            UseCaseNoteKind::OnLink { .. } => continue,
        };
        entries.push(Entry {
            kind,
            line: note.source_line,
        });
    }
    entries.sort_by_key(|e| e.line);
    let mut map = HashMap::new();
    let mut counter = 2usize;
    for entry in entries {
        match entry.kind {
            EntryKind::Entity(key) => {
                map.insert(key, format!("ent{counter:04}"));
                counter += 1;
            }
            EntryKind::AttachedNote(index) => {
                map.insert(format!("note-qname::{index}"), format!("GMN{counter}"));
                map.insert(format!("note::{index}"), format!("ent{:04}", counter + 1));
                counter += 3;
            }
            EntryKind::FloatingNote(index) => {
                map.insert(format!("note::{index}"), format!("ent{counter:04}"));
                counter += 1;
            }
            EntryKind::Connection => counter += 1,
        }
    }
    map
}

struct ActorDim {
    label_w: f64,
    stereo_w: f64,
    stereo_h: f64,
    stroke_thickness: f64,
    label_gap: f64,
    paint_min_y: f64,
    width: f64,
    height: f64,
}

struct UseCaseDim {
    label_w: f64,
    stereo_w: f64,
    footprint_center_y: f64,
    rx: f64,
    ry: f64,
}

struct NoteDim {
    width: f64,
    height: f64,
}

#[derive(Clone)]
struct NotePlacement {
    x: f64,
    y: f64,
    apex: Option<(f64, f64)>,
    leader_base: Option<((f64, f64), (f64, f64))>,
}

fn note_dim(note: &UseCaseNote) -> NoteDim {
    let lines: Vec<&str> = note.text.split('\n').collect();
    let text_width = lines
        .iter()
        .map(|line| text_render::measure(line, NOTE_FONT_SIZE as f64, false))
        .fold(0.0_f64, f64::max);
    let line_count = lines.len().max(1);
    NoteDim {
        width: text_width + NOTE_MARGIN_LEFT + NOTE_MARGIN_RIGHT,
        height: line_count as f64 * pm::text_height(NOTE_FONT_SIZE as f64) + NOTE_MARGIN_Y * 2.0,
    }
}

fn actor_dim(actor: &Actor, skin: &SkinColors) -> ActorDim {
    let label_w = text_render::measure_with_family(
        &actor.label,
        skin.actor_font_size as f64,
        false,
        &skin.actor_font_family,
    );
    let stereo_w = actor
        .stereotype
        .as_ref()
        .map(|s| {
            text_render::measure_with_family(
                &format!("\u{00AB}{s}\u{00BB}"),
                skin.actor_font_size as f64,
                false,
                &skin.actor_font_family,
            ) + ACTOR_STEREOTYPE_MARGIN_X * 2.0
        })
        .unwrap_or(0.0);
    let stroke_thickness = skin.actor_border_thickness.parse::<f64>().unwrap_or(0.5);
    let text_block_h = pm::text_height(skin.actor_font_size as f64);
    let stereo_h = if actor.stereotype.is_some() {
        text_block_h
    } else {
        0.0
    };
    let stickman_width = ACTOR_ARM_HALF * 2.0 + stroke_thickness * 2.0;
    let width = label_w.max(stereo_w).max(stickman_width);
    let stickman_height = ACTOR_STICKMAN_BASE_HEIGHT + stroke_thickness * 2.0;
    let label_gap = pm::ascent(skin.actor_font_size as f64) + 1.0 + stroke_thickness;
    // `SvekResult.calculateDimension` normalizes from the minimum painted
    // bound, not the node box. A stereotype's AWT line box overhangs the image
    // origin by the remainder after its baseline; without one, the stickman's
    // first painted point is one stroke thickness below the image origin.
    let paint_min_y = if actor.stereotype.is_some() {
        -(text_block_h - label_gap)
    } else {
        stroke_thickness
    };
    let height = stickman_height + text_block_h + stereo_h;
    ActorDim {
        label_w,
        stereo_w,
        stereo_h,
        stroke_thickness,
        label_gap,
        paint_min_y,
        width,
        height,
    }
}

fn use_case_dim(uc: &UseCase, skin: &SkinColors) -> UseCaseDim {
    let font_size = skin.uc_font_size as f64;
    let label_w =
        text_render::measure_with_family(&uc.label, font_size, false, &skin.uc_font_family);
    let stereo_w = uc
        .stereotype
        .as_ref()
        .map(|s| {
            text_render::measure_with_family(
                &format!("\u{00AB}{s}\u{00BB}"),
                font_size,
                false,
                &skin.uc_font_family,
            )
        })
        .unwrap_or(0.0);
    let body_widths: Vec<f64> = if uc.description.is_empty() {
        vec![label_w]
    } else {
        uc.description
            .iter()
            .map(|d| text_render::measure_with_family(d, font_size, false, &skin.uc_font_family))
            .collect()
    };
    let mut footprint_widths = Vec::with_capacity(body_widths.len() + 1);
    if uc.stereotype.is_some() {
        footprint_widths.push(stereo_w);
    }
    footprint_widths.extend(body_widths.iter().copied());
    // Java `Display.getCreole` builds the standalone stereotype through
    // `SheetBlock1`, whose one-pixel horizontal padding contributes to the
    // merged block dimension (and therefore alpha). `Footprint` records only
    // the painted text corners inside that padding.
    let stereo_block_w = if uc.stereotype.is_some() {
        stereo_w + 2.0
    } else {
        0.0
    };
    let body_block_w = body_widths.iter().copied().fold(0.0_f64, f64::max);
    let block_w = stereo_block_w.max(body_block_w);
    let line_count = uc.description.len().max(1) + if uc.stereotype.is_some() { 1 } else { 0 };
    let (rx, ry, footprint_center_y) =
        use_case_ellipse_radii(&footprint_widths, block_w, line_count as f64 * LINE_H);
    UseCaseDim {
        label_w,
        stereo_w,
        footprint_center_y,
        rx,
        ry,
    }
}

#[derive(Clone, Copy)]
struct FootprintCircle {
    center: (f64, f64),
    radius: f64,
}

impl FootprintCircle {
    fn at(center: (f64, f64)) -> Self {
        Self {
            center,
            radius: 0.0,
        }
    }

    fn through_two(p1: (f64, f64), p2: (f64, f64)) -> Self {
        let center = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        Self {
            center,
            radius: (p1.0 - center.0).hypot(p1.1 - center.1),
        }
    }

    fn through_three(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Self {
        if p3.1 == p2.1 {
            return Self::through_three(p2, p1, p3);
        }
        let num_x = p3.0 * p3.0 * (p1.1 - p2.1)
            + (p1.0 * p1.0 + (p1.1 - p2.1) * (p1.1 - p3.1)) * (p2.1 - p3.1)
            + p2.0 * p2.0 * (-p1.1 + p3.1);
        let den_x = 2.0 * (p3.0 * (p1.1 - p2.1) + p1.0 * (p2.1 - p3.1) + p2.0 * (-p1.1 + p3.1));
        let x = num_x / den_x;
        let y = (p2.1 + p3.1) / 2.0 - (p3.0 - p2.0) / (p3.1 - p2.1) * (x - (p2.0 + p3.0) / 2.0);
        Self {
            center: (x, y),
            radius: (p1.0 - x).hypot(p1.1 - y),
        }
    }

    fn is_outside(self, point: (f64, f64)) -> bool {
        (point.0 - self.center.0).hypot(point.1 - self.center.1) > self.radius
    }
}

fn smallest_enclosing_circle(
    count: usize,
    points: &[(f64, f64)],
    boundary_count: usize,
    boundary: &mut [(f64, f64)],
) -> FootprintCircle {
    let mut circle = match boundary_count {
        0 => FootprintCircle::at((0.0, 0.0)),
        1 => FootprintCircle::at(boundary[0]),
        2 => FootprintCircle::through_two(boundary[0], boundary[1]),
        3 => {
            return FootprintCircle::through_three(boundary[0], boundary[1], boundary[2]);
        }
        _ => unreachable!(),
    };
    for index in 0..count {
        if circle.is_outside(points[index]) {
            boundary[boundary_count] = points[index];
            circle = smallest_enclosing_circle(index, points, boundary_count + 1, boundary);
        }
    }
    circle
}

fn use_case_ellipse_radii(line_widths: &[f64], text_w: f64, text_h: f64) -> (f64, f64, f64) {
    // Java provenance: `svek.image.EntityImageUseCase.calculateDimensionSlow`
    // wraps the merged stereotype/body `TextBlock` in `TextBlockInEllipse`.
    // `Footprint.getEllipse` records every painted text corner after scaling
    // y by alpha, then `SmallestEnclosingCircle.findSec` computes the circle
    // before `getUEllipse().bigger(6)` adds three pixels to each radius.
    let w = text_w.max(1.0);
    let h = text_h.max(1.0);
    let alpha = (h / w).clamp(0.2, 0.8);
    let line_h = h / line_widths.len().max(1) as f64;
    let ascent = pm::ascent(FONT_SIZE);
    let mut points = Vec::with_capacity(line_widths.len() * 4);
    for (index, &line_w) in line_widths.iter().enumerate() {
        let x = (w - line_w) / 2.0;
        let baseline = index as f64 * line_h + ascent;
        // Java `Footprint.MyUGraphic.drawText` shifts the measured line box
        // upward by `height - 1.5` before recording its four corners.
        let top = baseline - line_h + 1.5;
        let bottom = top + line_h;
        points.extend([
            (x, top / alpha),
            (x, bottom / alpha),
            (x + line_w, top / alpha),
            (x + line_w, bottom / alpha),
        ]);
    }
    let mut boundary = points.clone();
    let circle = smallest_enclosing_circle(points.len(), &points, 0, &mut boundary);
    (
        circle.radius + 3.0,
        circle.radius * alpha + 3.0,
        circle.center.1 * alpha,
    )
}

struct Positions {
    actors: Vec<(f64, f64)>,
    use_cases: Vec<(f64, f64)>,
    notes: Vec<Option<NotePlacement>>,
    cluster_positions: Vec<ClusterPosition>,
    edge_paths: Vec<EdgePath>,
}

fn resolve_positions(
    diagram: &UseCaseDiagram,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_dims: &[NoteDim],
    skin: &SkinColors,
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
        return Positions {
            actors,
            use_cases,
            notes: vec![None; diagram.notes.len()],
            cluster_positions: Vec::new(),
            edge_paths: Vec::new(),
        };
    }
    layout_usecase_positions(diagram, actor_dims, uc_dims, note_dims, skin)
        .unwrap_or_else(|| fallback_positions(actor_dims, uc_dims, diagram.notes.len()))
}

fn layout_usecase_positions(
    diagram: &UseCaseDiagram,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_dims: &[NoteDim],
    skin: &SkinColors,
) -> Option<Positions> {
    // Java path: `CucaDiagramFileMakerSvek` builds measured SVEK nodes,
    // `DotStringFactory` serialises fixed-size nodes/clusters to dot, and
    // `GeneralImageBuilder` paints the returned positions. This mirrors that
    // flow with the vendored Graphviz wrapper rather than the old hand-stacked
    // fallback.
    if diagram.actors.is_empty()
        && diagram.use_cases.is_empty()
        && !diagram
            .notes
            .iter()
            .any(|note| !matches!(note.kind, UseCaseNoteKind::OnLink { .. }))
    {
        return None;
    }
    let direction = match diagram.direction {
        UseCaseLayoutDirection::TopToBottom => Direction::TopToBottom,
        UseCaseLayoutDirection::LeftToRight => Direction::LeftToRight,
    };
    let mut layout = LayoutGraph::new(direction).with_plantuml_svek_spacing();
    for (actor, dim) in diagram.actors.iter().zip(actor_dims) {
        layout.add_node(&actor.id, &actor.label, dim.width, dim.height);
    }
    for (uc, dim) in diagram.use_cases.iter().zip(uc_dims) {
        // Java `EntityImageUseCase.getShapeType` returns `ShapeType.OVAL`;
        // `SvekNode.appendShapeInternal` therefore gives Graphviz
        // `shape=ellipse`, so diagonal splines meet the painted oval rather
        // than its rectangular bounding box.
        layout.add_ellipse_node(&uc.id, &uc.label, dim.rx * 2.0, dim.ry * 2.0);
    }
    let entity_note_indices: Vec<usize> = diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| {
            (!matches!(note.kind, UseCaseNoteKind::OnLink { .. })).then_some(index)
        })
        .collect();
    for &note_index in &entity_note_indices {
        let note_id = note_node_id(&diagram.notes[note_index], note_index);
        let dim = &note_dims[note_index];
        layout.add_node(
            &note_id,
            &diagram.notes[note_index].text,
            dim.width,
            dim.height,
        );
    }
    for pkg in &diagram.packages {
        layout.add_cluster(&pkg.name, &pkg.name, None);
        for member in &pkg.elements {
            layout.add_cluster_node(&pkg.name, member);
        }
    }

    // `CommandFactoryNoteOnEntity.executeInternal` creates a real note leaf and
    // a hidden Link. Feed both hidden and visible links to dot in source order
    // so note nodes participate in the same rank/routing model as Java SVEK.
    enum LayoutEdge {
        Connection(usize),
        AttachedNote(usize),
    }
    let mut layout_edges: Vec<(usize, LayoutEdge)> = diagram
        .connections
        .iter()
        .enumerate()
        .map(|(index, connection)| (connection.source_line, LayoutEdge::Connection(index)))
        .collect();
    layout_edges.extend(
        diagram
            .notes
            .iter()
            .enumerate()
            .filter_map(|(index, note)| {
                matches!(note.kind, UseCaseNoteKind::Attached { .. })
                    .then_some((note.source_line, LayoutEdge::AttachedNote(index)))
            }),
    );
    layout_edges.sort_by_key(|(line, _)| *line);

    for (_, edge) in layout_edges {
        match edge {
            LayoutEdge::Connection(index) => {
                let conn = &diagram.connections[index];
                if let Some((note_index, note)) = note_on_connection(diagram, index) {
                    let size = link_note_label_size(conn, note, &note_dims[note_index], skin);
                    layout.add_edge_with_label_sizes(&conn.from, &conn.to, Some(size), None, None);
                } else {
                    layout.add_edge(
                        &conn.from,
                        &conn.to,
                        conn.label.as_deref().or(conn.stereotype.as_deref()),
                    );
                }
            }
            LayoutEdge::AttachedNote(index) => {
                let note = &diagram.notes[index];
                let UseCaseNoteKind::Attached { target } = &note.kind else {
                    continue;
                };
                let note_id = note_node_id(note, index);
                match effective_note_position(note.position, diagram.direction) {
                    UseCaseNotePosition::Right => {
                        layout.add_same_rank(target, &note_id);
                        layout.add_edge(target, &note_id, None);
                    }
                    UseCaseNotePosition::Left => {
                        layout.add_same_rank(&note_id, target);
                        layout.add_edge(&note_id, target, None);
                    }
                    UseCaseNotePosition::Bottom => layout.add_edge(target, &note_id, None),
                    UseCaseNotePosition::Top => layout.add_edge(&note_id, target, None),
                }
            }
        }
    }
    let mut result = layout.layout_full(LAYOUT_TIMEOUT)?;
    let degenerated = diagram.actors.len() + diagram.use_cases.len() + entity_note_indices.len()
        == 1
        && diagram.packages.is_empty()
        && diagram.connections.is_empty();
    let origin_x = if degenerated {
        DEGENERATED_MARGIN
    } else {
        BODY_MARGIN
    };
    let base_origin_y = origin_x;
    let actor_count = diagram.actors.len();
    let min_painted_y = result
        .node_positions
        .iter()
        .take(actor_count)
        .zip(actor_dims)
        .map(|(p, dim)| p.y + dim.paint_min_y)
        .chain(
            result
                .node_positions
                .iter()
                .skip(actor_count)
                .take(diagram.use_cases.len())
                .map(|p| p.y),
        )
        .chain(
            result
                .node_positions
                .iter()
                .skip(actor_count + diagram.use_cases.len())
                .take(entity_note_indices.len())
                .map(|p| p.y),
        )
        .chain(result.cluster_positions.iter().map(|p| p.y))
        .fold(f64::INFINITY, f64::min);
    let origin_y = if min_painted_y.is_finite() {
        base_origin_y - min_painted_y
    } else {
        base_origin_y
    };
    for edge in &mut result.edge_paths {
        for point in &mut edge.points {
            point.0 += origin_x;
            point.1 += origin_y;
        }
        if let Some(point) = &mut edge.start_point {
            point.0 += origin_x;
            point.1 += origin_y;
        }
        if let Some(point) = &mut edge.end_point {
            point.0 += origin_x;
            point.1 += origin_y;
        }
        if let Some(label) = &mut edge.label {
            label.x += origin_x;
            label.y += origin_y;
        }
        if let Some(label) = &mut edge.tail_label {
            label.x += origin_x;
            label.y += origin_y;
        }
        if let Some(label) = &mut edge.head_label {
            label.x += origin_x;
            label.y += origin_y;
        }
    }
    for cluster in &mut result.cluster_positions {
        cluster.x += origin_x;
        cluster.y += origin_y;
    }
    let actors = result
        .node_positions
        .iter()
        .take(actor_count)
        .zip(actor_dims)
        .map(|(p, dim)| {
            (
                p.x + origin_x + p.width / 2.0,
                p.y + origin_y + dim.stereo_h + dim.stroke_thickness + ACTOR_HEAD_R,
            )
        })
        .collect();
    let use_cases = result
        .node_positions
        .iter()
        .skip(actor_count)
        .take(diagram.use_cases.len())
        .zip(uc_dims)
        .map(|(p, dim)| (p.x + origin_x + dim.rx, p.y + origin_y + dim.ry))
        .collect();

    let note_ids: Vec<String> = entity_note_indices
        .iter()
        .map(|&index| note_node_id(&diagram.notes[index], index))
        .collect();
    let mut notes = vec![None; diagram.notes.len()];
    let note_node_offset = actor_count + diagram.use_cases.len();
    for (slot, &note_index) in entity_note_indices.iter().enumerate() {
        let note_id = &note_ids[slot];
        let p = &result.node_positions[note_node_offset + slot];
        let edge = result
            .edge_paths
            .iter()
            .find(|edge| edge.from == *note_id || edge.to == *note_id);
        notes[note_index] = Some(note_placement(
            p.x + origin_x,
            p.y + origin_y,
            &note_dims[note_index],
            note_id,
            edge,
        ));
    }
    let edge_paths = result
        .edge_paths
        .into_iter()
        .filter(|edge| {
            !note_ids
                .iter()
                .any(|note_id| edge.from == *note_id || edge.to == *note_id)
        })
        .collect();
    Some(Positions {
        actors,
        use_cases,
        notes,
        cluster_positions: result.cluster_positions,
        edge_paths,
    })
}

fn fallback_positions(
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_count: usize,
) -> Positions {
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
    Positions {
        actors,
        use_cases,
        notes: vec![None; note_count],
        cluster_positions: Vec::new(),
        edge_paths: Vec::new(),
    }
}

fn note_node_id(note: &UseCaseNote, index: usize) -> String {
    match &note.kind {
        UseCaseNoteKind::Floating { id } => id.clone(),
        UseCaseNoteKind::Attached { .. } => format!("__rustuml_note_{index}"),
        UseCaseNoteKind::OnLink { .. } => format!("__rustuml_link_note_{index}"),
    }
}

fn effective_note_position(
    position: UseCaseNotePosition,
    direction: UseCaseLayoutDirection,
) -> UseCaseNotePosition {
    if direction == UseCaseLayoutDirection::TopToBottom {
        return position;
    }
    // Java provenance: `Position.withRankdir` rotates entity-note placement
    // when DESCRIPTION diagrams use `left to right direction`.
    match position {
        UseCaseNotePosition::Right => UseCaseNotePosition::Bottom,
        UseCaseNotePosition::Left => UseCaseNotePosition::Top,
        UseCaseNotePosition::Bottom => UseCaseNotePosition::Right,
        UseCaseNotePosition::Top => UseCaseNotePosition::Left,
    }
}

fn note_on_connection(
    diagram: &UseCaseDiagram,
    connection: usize,
) -> Option<(usize, &UseCaseNote)> {
    diagram.notes.iter().enumerate().find(|(_, note)| {
        matches!(
            note.kind,
            UseCaseNoteKind::OnLink {
                connection: owner
            } if owner == connection
        )
    })
}

fn link_note_label_size(
    connection: &UseCaseConnection,
    note: &UseCaseNote,
    note_dim: &NoteDim,
    skin: &SkinColors,
) -> EdgeLabelSize {
    // `EntityImageNoteLink` delegates to `ComponentRoseNote`, whose preferred
    // size includes the EntityImageNote text margins plus Rose's 5px padding.
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let label = connection
        .label
        .as_deref()
        .or(connection.stereotype.as_deref());
    let Some(label) = label else {
        return EdgeLabelSize {
            width: note_width,
            height: note_height,
        };
    };
    let label_width = text_render::measure_with_family(
        label,
        skin.arrow_font_size as f64,
        false,
        &skin.arrow_font_family,
    ) + 2.0;
    let label_height = pm::text_height(skin.arrow_font_size as f64) + 2.0;
    match note.position {
        UseCaseNotePosition::Left | UseCaseNotePosition::Right => EdgeLabelSize {
            width: note_width + label_width,
            height: note_height.max(label_height),
        },
        UseCaseNotePosition::Top | UseCaseNotePosition::Bottom => EdgeLabelSize {
            width: note_width.max(label_width),
            height: note_height + label_height,
        },
    }
}

fn note_placement(
    x: f64,
    y: f64,
    dim: &NoteDim,
    note_id: &str,
    edge: Option<&EdgePath>,
) -> NotePlacement {
    let Some(edge) = edge else {
        return NotePlacement {
            x,
            y,
            apex: None,
            leader_base: None,
        };
    };
    let Some(start) = edge.points.first().copied() else {
        return NotePlacement {
            x,
            y,
            apex: None,
            leader_base: None,
        };
    };
    let Some(end) = edge.points.last().copied() else {
        return NotePlacement {
            x,
            y,
            apex: None,
            leader_base: None,
        };
    };
    let (contact, apex) = if edge.from == note_id {
        (start, end)
    } else {
        (end, start)
    };
    let right = x + dim.width;
    let bottom = y + dim.height;
    let leader_base = if apex.1 < y {
        let x1 = (contact.0 - x - NOTE_LEADER_HALF).clamp(0.0, dim.width - NOTE_FOLD);
        Some(((x + x1 + NOTE_LEADER_HALF * 2.0, y), (x + x1, y)))
    } else if apex.1 > bottom {
        let x1 = (contact.0 - x - NOTE_LEADER_HALF).clamp(0.0, dim.width);
        Some(((x + x1, bottom), (x + x1 + NOTE_LEADER_HALF * 2.0, bottom)))
    } else if apex.0 < x {
        let y1 = (contact.1 - y - NOTE_LEADER_HALF).clamp(0.0, dim.height - NOTE_LEADER_HALF * 2.0);
        Some(((x, y + y1), (x, y + y1 + NOTE_LEADER_HALF * 2.0)))
    } else {
        let y1 = (contact.1 - y - NOTE_LEADER_HALF)
            .clamp(NOTE_FOLD, dim.height - NOTE_LEADER_HALF * 2.0);
        Some(((right, y + y1 + NOTE_LEADER_HALF * 2.0), (right, y + y1)))
    };
    NotePlacement {
        x,
        y,
        apex: Some(apex),
        leader_base,
    }
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
    if uc.explicit_id {
        &uc.id
    } else if uc.id == label_to_id(&uc.label) {
        &uc.label
    } else {
        &uc.id
    }
}

/// PlantUML sanitises `data-qualified-name` (and the entity key it stores in
/// the oracle map) by replacing every non-ASCII character with `.`. The visible
/// label text keeps the original unicode; only the identifier attribute is
/// folded.
fn sanitize_qname(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { '.' })
        .collect()
}

/// The form PlantUML uses in the `<!--entity/cluster …-->` comments: non-ASCII
/// characters are replaced with `?` (distinct from the `.` used in
/// `data-qualified-name`).
fn sanitize_comment(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect()
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
            return sanitize_qname(&format!("{}.{display}", pkg.name));
        }
    }
    sanitize_qname(display)
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
    diagram: &UseCaseDiagram,
    positions: &Positions,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_dims: &[NoteDim],
) -> (f64, f64) {
    let degenerated = actor_dims.len() + uc_dims.len() == 1
        && positions.edge_paths.is_empty()
        && positions.cluster_positions.is_empty();
    // `EntityImageDegenerated` owns the 7px entity inset; the surrounding
    // image builder contributes the remaining 12px on the far edges.
    let canvas_pad = if degenerated {
        SVEK_CANVAS_PAD - 2.0
    } else {
        SVEK_CANVAS_PAD
    };
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (i, (cx, cy)) in positions.actors.iter().enumerate() {
        let half = actor_dims[i].width / 2.0;
        let leg_y = cy + ACTOR_HEAD_R + ACTOR_BODY_LEN + ACTOR_LEG_DROP;
        // Java provenance: `LimitFinder.drawText` moves a `UText` up by
        // `height - 1.5`, so its measured maximum is the emitted baseline plus
        // 1.5 rather than the baseline plus a full line box.
        let label_max_y = leg_y + actor_dims[i].label_gap + 1.5;
        let stereotype_max_y = if actor_dims[i].stereo_h > 0.0 {
            cy - ACTOR_STEREO_OFFSET + 1.5
        } else {
            f64::NEG_INFINITY
        };
        max_x = max_x.max(cx + half + canvas_pad);
        max_y = max_y.max(leg_y.max(label_max_y).max(stereotype_max_y) + canvas_pad);
    }
    for (i, (cx, cy)) in positions.use_cases.iter().enumerate() {
        // Java provenance: `LimitFinder.drawEllipse` records the far corner at
        // `origin + dimension - 1`, so the painted oval's maximum is one pixel
        // inside its geometric bounding box on both axes.
        max_x = max_x.max(cx + uc_dims[i].rx - 1.0 + canvas_pad);
        max_y = max_y.max(cy + uc_dims[i].ry - 1.0 + canvas_pad);
    }
    for (index, note) in positions.notes.iter().enumerate() {
        let Some(note) = note else { continue };
        max_x = max_x.max(note.x + note_dims[index].width + SVEK_CANVAS_PAD);
        max_y = max_y.max(note.y + note_dims[index].height + SVEK_CANVAS_PAD);
    }
    for edge in &positions.edge_paths {
        for (x, y) in &edge.points {
            max_x = max_x.max(x + SVEK_CANVAS_PAD);
            max_y = max_y.max(y + SVEK_CANVAS_PAD);
        }
        if let Some(label) = edge.label {
            let rose_note_trailing_pad = diagram
                .connections
                .iter()
                .enumerate()
                .filter(|(_, connection)| {
                    connection.from == edge.from
                        && connection.to == edge.to
                        && connection.label.is_none()
                        && connection.stereotype.is_none()
                })
                .find_map(|(connection_index, _)| {
                    let (note_index, _) = note_on_connection(diagram, connection_index)?;
                    let expected_width = note_dims[note_index].width + LINK_NOTE_PADDING * 2.0;
                    let expected_height = note_dims[note_index].height + LINK_NOTE_PADDING * 2.0;
                    // Graphviz's SVG label rectangle serializes these
                    // component dimensions at whole-pixel precision.
                    ((label.width - expected_width.floor()).abs() < 0.01
                        && (label.height - expected_height.floor()).abs() < 0.01)
                        .then_some(LINK_NOTE_PADDING)
                })
                .unwrap_or(0.0);
            // Java provenance: `ComponentRoseNote` reports a label box with
            // five pixels of padding on every side, then paints the Opale note
            // at `(5,5)`. For a pure note-on-link label, `LimitFinder` sees the
            // painted note but not the unused trailing padding.
            max_x = max_x.max(label.x + label.width - rose_note_trailing_pad + SVEK_CANVAS_PAD);
            max_y = max_y.max(label.y + label.height - rose_note_trailing_pad + SVEK_CANVAS_PAD);
        }
    }
    for cluster in &positions.cluster_positions {
        max_x = max_x.max(cluster.x + cluster.width + SVEK_CANVAS_PAD);
        max_y = max_y.max(cluster.y + cluster.height + SVEK_CANVAS_PAD);
    }
    (max_x.ceil().max(1.0), max_y.ceil().max(1.0))
}

fn render_package_group(
    svg: &mut SvgBuilder,
    pkg: &UseCasePackage,
    oracle: Option<&OracleLayout>,
    cluster_positions: &[ClusterPosition],
    id_map: &HashMap<String, String>,
) {
    // PlantUML keys clusters by the sanitised qualified name (non-ASCII → `.`).
    let qname = sanitize_qname(&pkg.name);
    let (rect_x, rect_y, rect_w, rect_h, captured_x, captured_y): (
        f64,
        f64,
        f64,
        f64,
        &[f64],
        &[f64],
    ) = if let Some(orc) = oracle {
        let Some(rect) = orc
            .entities
            .get(&qname)
            .or_else(|| orc.entities.get(&pkg.name))
        else {
            return;
        };
        (
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.text_x_values.as_slice(),
            rect.text_y_values.as_slice(),
        )
    } else {
        let Some(cluster) = cluster_positions.iter().find(|p| p.id == pkg.name) else {
            return;
        };
        (
            cluster.x,
            cluster.y,
            cluster.width,
            cluster.height,
            &[],
            &[],
        )
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
    svg.raw(&format!("<!--cluster {}-->", sanitize_comment(&pkg.name)));
    svg.raw(&format!(
        r#"<g class="cluster" data-qualified-name="{qname}"{src_attr} id="{ent_id}">"#,
    ));
    let label_w = text_render::measure(&pkg.name, FONT_SIZE, true);
    let (label_x, label_y) = match pkg.kind {
        PackageKind::Rectangle => {
            // Plain rounded rect, centred bold label.
            svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
                h = fc(rect_h),
                w = fc(rect_w),
                x = fc(rect_x),
                y = fc(rect_y),
            ));
            (rect_x + (rect_w - label_w) / 2.0, rect_y + 15.5352)
        }
        PackageKind::Package => {
            // Folder-tab outline: a notched top-left "tab" carrying the label,
            // a diagonal slope down to the body's top edge, then a rounded
            // rectangle body. Reconstructed from the oracle box rect and label
            // width (HALF_UP coords).
            let x = rect_x;
            let y = rect_y;
            let xr = rect_x + rect_w;
            let yb = rect_y + rect_h;
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
    let label_x = captured_x.first().copied().unwrap_or(label_x);
    let label_y = captured_y.first().copied().unwrap_or(label_y);
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
    let display_raw = if actor.id == label_to_id(&actor.label) {
        actor.label.as_str()
    } else {
        actor.id.as_str()
    };
    svg.raw(&format!("<!--entity {}-->", sanitize_comment(display_raw)));
    let display = sanitize_qname(display_raw);
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
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{ACTOR_HEAD_R}" ry="{ACTOR_HEAD_R}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        stroke_width = skin.actor_border_thickness,
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
        r#"<path d="M{cx},{body_top_y} L{cx},{body_bot_y} M{arm_left_x},{arm_y} L{arm_right_x},{arm_y} M{cx},{body_bot_y} L{leg_x_left},{leg_y} M{cx},{body_bot_y} L{leg_x_right},{leg_y}" fill="none" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
        cx = fc(cx),
        body_top_y = fc(body_top_y),
        body_bot_y = fc(body_bot_y),
        arm_left_x = fc(arm_left_x),
        arm_right_x = fc(arm_right_x),
        arm_y = fc(arm_y),
        leg_x_left = fc(leg_x_left),
        leg_x_right = fc(leg_x_right),
        leg_y = fc(leg_y),
        stroke_width = skin.actor_border_thickness,
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
    let label_y = captured_y.first().copied().unwrap_or(leg_y + dim.label_gap);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        &actor.label,
        &TextBase {
            x: label_x,
            y: label_y,
            font_size: skin.actor_font_size,
            font_family: &skin.actor_font_family,
            fill: &skin.actor_font_color,
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
            .unwrap_or(cx_anchor - (dim.stereo_w - ACTOR_STEREOTYPE_MARGIN_X * 2.0) / 2.0);
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
                font_size: skin.actor_font_size,
                font_family: &skin.actor_font_family,
                fill: &skin.actor_stereo_font_color,
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
    svg.raw(&format!(
        "<!--entity {}-->",
        sanitize_comment(display_name(uc))
    ));
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
        skin_fill(
            &diagram.meta.skinparams,
            &format!("usecaseBackgroundColor<<{s}>>"),
            skin.gradient_defs.as_deref(),
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
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{rx}" ry="{ry}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        rx = fc(rx),
        ry = fc(ry),
        stroke_width = skin.uc_border_thickness,
    ));
    let cx_anchor = round_coord(cx);
    // PlantUML's emitted text x values are captured per line (stereotype first,
    // then label/description lines). Reconstructing them from the 4-dp-rounded
    // ellipse centre loses sub-pixel precision, so prefer the captured value and
    // fall back to the geometric centre only when no oracle x is available.
    let captured_x = orc_rect.map(|r| r.text_x_values.as_slice()).unwrap_or(&[]);
    let captured_y = orc_rect.map(|r| r.text_y_values.as_slice()).unwrap_or(&[]);
    let mut line_idx = 0usize;
    // Java `TextBlockInEllipse.drawU` translates the merged text by
    // `(ellipseHalfHeight - footprintCenterY - 2)`. Since `cy` already
    // includes the half-height, each baseline is relative to the computed
    // painted footprint center rather than a fixed one-line offset.
    let mut text_y = cy - dim.footprint_center_y - 2.0 + pm::ascent(FONT_SIZE);
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
                font_size: skin.uc_font_size,
                font_family: &skin.uc_font_family,
                fill: &skin.uc_stereo_font_color,
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
                font_size: skin.uc_font_size,
                font_family: &skin.uc_font_family,
                fill: &skin.uc_font_color,
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
            let lw = text_render::measure_with_family(
                line,
                skin.uc_font_size as f64,
                false,
                &skin.uc_font_family,
            );
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
                    font_size: skin.uc_font_size,
                    font_family: &skin.uc_font_family,
                    fill: &skin.uc_font_color,
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
    let Some(g) = note.box_geom.as_ref() else {
        return;
    };
    let d = note_outline(g.x, g.y, g.width, g.height, g.apex, g.leader_base);
    let right = g.x + g.width;
    let rf = right - NOTE_FOLD;
    let yf = g.y + NOTE_FOLD;

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
        by_s = fc(g.y),
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

fn note_outline(
    bx: f64,
    by: f64,
    width: f64,
    height: f64,
    apex: Option<(f64, f64)>,
    leader_base: Option<((f64, f64), (f64, f64))>,
) -> String {
    let right = bx + width;
    let bottom = by + height;
    let rf = right - NOTE_FOLD;
    let yf = by + NOTE_FOLD;
    let side = apex.map(|(ax, ay)| {
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
        if let (Some((ax, ay)), Some((b0, b1))) = (apex, leader_base) {
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

    // `Opale.getPolygon*` walks the outline counter-clockwise and splices the
    // hidden link into the nearest box edge.
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
    d
}

fn emit_model_note(
    svg: &mut SvgBuilder,
    note: &UseCaseNote,
    dim: &NoteDim,
    placement: &NotePlacement,
    index: usize,
    id_map: &HashMap<String, String>,
) {
    let qname = match &note.kind {
        UseCaseNoteKind::Attached { .. } => id_map
            .get(&format!("note-qname::{index}"))
            .map(String::as_str)
            .unwrap_or("GMN"),
        UseCaseNoteKind::Floating { id } => id.as_str(),
        UseCaseNoteKind::OnLink { .. } => return,
    };
    let entity_id = id_map
        .get(&format!("note::{index}"))
        .map(String::as_str)
        .unwrap_or("");
    let d = note_outline(
        placement.x,
        placement.y,
        dim.width,
        dim.height,
        placement.apex,
        placement.leader_base,
    );
    let right = placement.x + dim.width;
    let fold_x = right - NOTE_FOLD;
    let fold_y = placement.y + NOTE_FOLD;
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qname}"{} id="{entity_id}">"#,
        source_line_attr(note.source_line),
    ));
    svg.raw(&format!(
        r#"<path d="{d}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<path d="M{fold_x},{top} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{top}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        fold_x = fc(fold_x),
        top = fc(placement.y),
        fold_y = fc(fold_y),
        right = fc(right),
    ));
    let mut baseline = placement.y + NOTE_MARGIN_Y + pm::ascent(NOTE_FONT_SIZE as f64);
    for line in note.text.split('\n') {
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            line,
            &TextBase {
                x: placement.x + NOTE_MARGIN_LEFT,
                y: baseline,
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
        baseline += pm::text_height(NOTE_FONT_SIZE as f64);
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
    skin: &SkinColors,
) {
    // PlantUML emits links sorted by source line. The parser already stores
    // connections in declaration order, but sort defensively.
    // Iterate the oracle's edges in their captured DOM order — PlantUML does
    // not always emit links in source-line order (e.g. an `<<extend>>` edge can
    // precede the `<<include>>` edges declared before it), so the oracle order
    // is authoritative. For each edge, find the connection it corresponds to
    // (for the note-on-link label/shape ordering), consuming each connection
    // once so edges sharing an endpoint pair bind to distinct connections.
    let mut used_conns: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for oracle_edge in &oracle.edges {
        let conn_idx = diagram.connections.iter().enumerate().position(|(i, c)| {
            if used_conns.contains(&i) {
                return false;
            }
            let from_label = entity_label(diagram, &c.from);
            let to_label = entity_label(diagram, &c.to);
            let candidates = [
                format!("{from_label}-to-{to_label}"),
                format!("{from_label}-{to_label}"),
                format!("{from_label}-backto-{to_label}"),
                format!("{}-to-{}", c.from, c.to),
                format!("{}-{}", c.from, c.to),
                format!("{}-backto-{}", c.from, c.to),
            ];
            candidates.iter().any(|cand| cand == &oracle_edge.id)
        });
        let Some(conn_idx) = conn_idx else {
            continue;
        };
        used_conns.insert(conn_idx);
        let conn = &diagram.connections[conn_idx];
        let from_label = entity_label(diagram, &conn.from);
        let to_label = entity_label(diagram, &conn.to);
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
        // Edge labels and any `note on link` shape. PlantUML emits, in document
        // order: the link label (e.g. `«extend»`), then the note's box path and
        // folded-corner path (`extra_paths`), then the note text. We replay that
        // order: the first label, the extra paths, then the remaining labels.
        // Falls back to the single concatenated `label` when no per-line labels
        // were captured.
        let emit_label = |svg: &mut SvgBuilder, lx: f64, ly: f64, text: &str| {
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                text,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: skin.arrow_font_size,
                    font_family: &skin.arrow_font_family,
                    fill: &skin.arrow_font_color,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
        };
        let emit_extra_paths = |svg: &mut SvgBuilder| {
            for (d, style) in &oracle_edge.extra_paths {
                let style = style
                    .as_deref()
                    .unwrap_or("stroke:#181818;stroke-width:0.5;");
                svg.raw(&format!(
                    r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#
                ));
            }
        };
        if oracle_edge.labels.is_empty() {
            emit_extra_paths(&mut *svg);
            if let Some((lx, ly, ref text)) = oracle_edge.label {
                emit_label(&mut *svg, lx, ly, text);
            }
        } else {
            // The connection's own label (`: <<extend>>`) precedes the note
            // shape; the note's text follows it. When the edge carries no label
            // of its own, every captured text belongs to the note and follows
            // the note box.
            let mut labels = oracle_edge.labels.iter();
            let has_edge_label = conn.label.is_some() || conn.stereotype.is_some();
            if has_edge_label && let Some((lx, ly, text)) = labels.next() {
                emit_label(&mut *svg, *lx, *ly, text);
            }
            emit_extra_paths(&mut *svg);
            for (lx, ly, text) in labels {
                emit_label(&mut *svg, *lx, *ly, text);
            }
        }
        svg.raw("</g>");
    }
}

fn link_note_blocks(
    x: f64,
    y: f64,
    connection: &UseCaseConnection,
    note: &UseCaseNote,
    note_dim: &NoteDim,
    skin: &SkinColors,
) -> ((f64, f64), (f64, f64)) {
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let label = connection
        .label
        .as_deref()
        .or(connection.stereotype.as_deref());
    let Some(label) = label else {
        return ((x, y), (x, y));
    };
    // `SvekEdge.getLabel` wraps the edge label in a one-pixel margin before
    // merging it with `EntityImageNoteLink`.
    let label_width = text_render::measure_with_family(
        label,
        skin.arrow_font_size as f64,
        false,
        &skin.arrow_font_family,
    ) + 2.0;
    let label_height = pm::text_height(skin.arrow_font_size as f64) + 2.0;
    let ascent = pm::ascent(skin.arrow_font_size as f64);
    match note.position {
        UseCaseNotePosition::Left => {
            let merged_height = note_height.max(label_height);
            let note_y = y + (merged_height - note_height) / 2.0;
            let label_y = y + (merged_height - label_height) / 2.0;
            ((x + note_width + 1.0, label_y + 1.0 + ascent), (x, note_y))
        }
        UseCaseNotePosition::Right => {
            let merged_height = note_height.max(label_height);
            let note_y = y + (merged_height - note_height) / 2.0;
            let label_y = y + (merged_height - label_height) / 2.0;
            ((x + 1.0, label_y + 1.0 + ascent), (x + label_width, note_y))
        }
        UseCaseNotePosition::Top => {
            let merged_width = note_width.max(label_width);
            let note_x = x + (merged_width - note_width) / 2.0;
            let label_x = x + (merged_width - label_width) / 2.0;
            ((label_x + 1.0, y + note_height + 1.0 + ascent), (note_x, y))
        }
        UseCaseNotePosition::Bottom => {
            let merged_width = note_width.max(label_width);
            let note_x = x + (merged_width - note_width) / 2.0;
            let label_x = x + (merged_width - label_width) / 2.0;
            (
                (label_x + 1.0, y + 1.0 + ascent),
                (note_x, y + label_height),
            )
        }
    }
}

fn emit_link_note(
    svg: &mut SvgBuilder,
    connection: &UseCaseConnection,
    note: &UseCaseNote,
    dim: &NoteDim,
    label_x: f64,
    label_y: f64,
    skin: &SkinColors,
) {
    let (_, (component_x, component_y)) =
        link_note_blocks(label_x, label_y, connection, note, dim, skin);
    let x = component_x + LINK_NOTE_PADDING;
    let y = component_y + LINK_NOTE_PADDING;
    // `ComponentRoseNote.drawInternalU` truncates its text-box dimensions
    // before asking Opale for the folded rectangle.
    let width = dim.width.floor();
    let height = dim.height.floor();
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;
    svg.raw(&format!(
        r#"<path d="M{x},{y} L{x},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{y} L{x},{y}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        x = fc(x),
        y = fc(y),
        bottom = fc(bottom),
        right = fc(right),
        fold_y = fc(fold_y),
        fold_x = fc(fold_x),
    ));
    svg.raw(&format!(
        r#"<path d="M{fold_x},{y} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{y}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        fold_x = fc(fold_x),
        y = fc(y),
        fold_y = fc(fold_y),
        right = fc(right),
    ));
    let mut baseline = y + NOTE_MARGIN_Y + pm::ascent(NOTE_FONT_SIZE as f64);
    for line in note.text.split('\n') {
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            line,
            &TextBase {
                x: x + NOTE_MARGIN_LEFT,
                y: baseline,
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
        baseline += pm::text_height(NOTE_FONT_SIZE as f64);
    }
}

fn render_no_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &UseCaseDiagram,
    id_map: &HashMap<String, String>,
    edge_paths: &[EdgePath],
    skin: &SkinColors,
) {
    for (connection_index, conn) in diagram.connections.iter().enumerate() {
        let Some(edge) = edge_paths
            .iter()
            .find(|edge| edge.from == conn.from && edge.to == conn.to)
        else {
            continue;
        };
        let Some(ent1) = usecase_entity_id(diagram, id_map, &conn.from) else {
            continue;
        };
        let Some(ent2) = usecase_entity_id(diagram, id_map, &conn.to) else {
            continue;
        };
        let link_id = no_oracle_link_id(diagram, conn.source_line);
        let from_label = link_comment_name(diagram, &conn.from);
        let to_label = link_comment_name(diagram, &conn.to);
        let start_decoration = connection_start_decoration(conn);
        let end_decoration = connection_end_decoration(conn);
        let link_type = if conn.extension {
            "extension"
        } else if start_decoration.is_some() || end_decoration.is_some() {
            "dependency"
        } else {
            "association"
        };
        svg.raw(&format!("<!--link {from_label} to {to_label}-->"));
        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{ent1}" data-entity-2="{ent2}" data-link-type="{link_type}" data-source-line="{line}" id="{link_id}">"#,
            line = conn.source_line,
        ));
        let raw_points = quantized_svek_edge_points(edge);
        let mut path_points = raw_points.clone();
        if let Some(decoration) = start_decoration {
            trim_svek_edge_endpoint(&mut path_points, true, decoration.path_gap());
        }
        if let Some(decoration) = end_decoration {
            trim_svek_edge_endpoint(&mut path_points, false, decoration.path_gap());
        }
        if let Some(d) = edge_path_d(&path_points) {
            let path_style = if conn.dashed {
                format!("stroke:{STROKE};stroke-width:1;stroke-dasharray:7,7;")
            } else {
                format!("stroke:{STROKE};stroke-width:1;")
            };
            let path_id = if start_decoration.is_some() || end_decoration.is_some() {
                format!("{from_label}-to-{to_label}")
            } else {
                format!("{from_label}-{to_label}")
            };
            svg.raw(&format!(
                r#"<path d="{d}" fill="none" id="{path_id}" style="{path_style}"/>"#,
            ));
        }
        if let Some(decoration) = start_decoration
            && raw_points.len() >= 2
        {
            render_usecase_extremity(svg, decoration, raw_points[1], raw_points[0]);
        }
        if let Some(decoration) = end_decoration
            && raw_points.len() >= 2
        {
            let endpoint = raw_points.len() - 1;
            render_usecase_extremity(
                svg,
                decoration,
                raw_points[endpoint - 1],
                raw_points[endpoint],
            );
        }
        let label_text = conn
            .label
            .as_deref()
            .or(conn.stereotype.as_deref())
            .map(|s| {
                if s.starts_with("<<") {
                    s.replace("<<", "\u{00AB}").replace(">>", "\u{00BB}")
                } else {
                    s.to_string()
                }
            });
        let link_note = note_on_connection(diagram, connection_index);
        if let Some(label) = label_text {
            let position = if let Some((note_index, note)) = link_note {
                edge.label.map(|label_box| {
                    link_note_blocks(
                        label_box.x,
                        label_box.y,
                        conn,
                        note,
                        &note_dim(&diagram.notes[note_index]),
                        skin,
                    )
                    .0
                })
            } else {
                edge.points
                    .get(edge.points.len() / 2)
                    .map(|(x, y)| (*x + 4.0, *y - 4.0))
            };
            let Some((x, y)) = position else {
                svg.raw("</g>");
                continue;
            };
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                &label,
                &TextBase {
                    x,
                    y,
                    font_size: skin.arrow_font_size,
                    font_family: &skin.arrow_font_family,
                    fill: &skin.arrow_font_color,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
        }
        if let Some((note_index, note)) = link_note
            && let Some(label_box) = edge.label
        {
            emit_link_note(
                svg,
                conn,
                note,
                &note_dim(&diagram.notes[note_index]),
                label_box.x,
                label_box.y,
                skin,
            );
        }
        svg.raw("</g>");
    }
}

fn usecase_entity_id<'a>(
    diagram: &UseCaseDiagram,
    id_map: &'a HashMap<String, String>,
    id: &str,
) -> Option<&'a str> {
    if diagram.actors.iter().any(|a| a.id == id) {
        return id_map.get(&format!("actor::{id}")).map(String::as_str);
    }
    if diagram.use_cases.iter().any(|u| u.id == id) {
        return id_map.get(&format!("uc::{id}")).map(String::as_str);
    }
    None
}

fn link_comment_name<'a>(diagram: &'a UseCaseDiagram, id: &'a str) -> &'a str {
    if let Some(uc) = diagram.use_cases.iter().find(|u| u.id == id) {
        return display_name(uc);
    }
    if let Some(actor) = diagram.actors.iter().find(|a| a.id == id) {
        if actor.id == label_to_id(&actor.label) {
            return actor.label.as_str();
        }
        return actor.id.as_str();
    }
    id
}

fn no_oracle_link_id(diagram: &UseCaseDiagram, source_line: usize) -> String {
    let mut counter = 2usize;
    let mut items: Vec<(usize, usize, bool)> = Vec::new();
    items.extend(diagram.actors.iter().map(|a| (a.source_line, 1, false)));
    items.extend(diagram.use_cases.iter().map(|u| (u.source_line, 1, false)));
    items.extend(diagram.packages.iter().map(|p| (p.source_line, 1, false)));
    items.extend(diagram.notes.iter().filter_map(|note| {
        let slots = match note.kind {
            UseCaseNoteKind::Attached { .. } => 3,
            UseCaseNoteKind::Floating { .. } => 1,
            UseCaseNoteKind::OnLink { .. } => return None,
        };
        Some((note.source_line, slots, false))
    }));
    items.extend(diagram.connections.iter().map(|c| (c.source_line, 1, true)));
    items.sort_by_key(|(line, _, _)| *line);
    for (line, slots, is_link) in items {
        if is_link && line == source_line {
            return format!("lnk{counter}");
        }
        counter += slots;
    }
    format!("lnk{counter}")
}

#[derive(Clone, Copy)]
enum UseCaseExtremity {
    Dependency,
    Extension,
}

impl UseCaseExtremity {
    fn path_gap(self) -> f64 {
        match self {
            Self::Dependency => DEPENDENCY_ARROW_PATH_GAP,
            // `LinkDecor.EXTENDS.getExtremityFactoryComplete` constructs an
            // `ExtremityTriangle` with an 18px decoration length.
            Self::Extension => 18.0,
        }
    }
}

fn connection_start_decoration(conn: &UseCaseConnection) -> Option<UseCaseExtremity> {
    conn.arrow_at_start.then_some(if conn.extension {
        UseCaseExtremity::Extension
    } else {
        UseCaseExtremity::Dependency
    })
}

fn connection_end_decoration(conn: &UseCaseConnection) -> Option<UseCaseExtremity> {
    conn.arrow.then_some(if conn.extension {
        UseCaseExtremity::Extension
    } else {
        UseCaseExtremity::Dependency
    })
}

fn quantized_svek_edge_points(edge: &EdgePath) -> Vec<(f64, f64)> {
    // `SvekEdge.solveLine` parses Graphviz's SVG path after Graphviz has
    // serialized every coordinate to two decimal places.
    edge.points
        .iter()
        .map(|(x, y)| ((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0))
        .collect()
}

fn trim_svek_edge_endpoint(points: &mut [(f64, f64)], start: bool, gap: f64) {
    if points.len() < 2 {
        return;
    }
    let endpoint = if start { 0 } else { points.len() - 1 };
    let adjacent = if start { 1 } else { endpoint - 1 };
    let dx = points[endpoint].0 - points[adjacent].0;
    let dy = points[endpoint].1 - points[adjacent].1;
    let len = dx.hypot(dy);
    if len <= f64::EPSILON {
        return;
    }
    let shift = (dx / len * gap, dy / len * gap);
    points[endpoint].0 -= shift.0;
    points[endpoint].1 -= shift.1;
    if points.len() >= 4 {
        points[adjacent].0 -= shift.0;
        points[adjacent].1 -= shift.1;
    }
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

fn render_usecase_extremity(
    svg: &mut SvgBuilder,
    decoration: UseCaseExtremity,
    control: (f64, f64),
    endpoint: (f64, f64),
) {
    match decoration {
        UseCaseExtremity::Dependency => {
            let points = dependency_arrow_points(control, endpoint);
            svg.raw(&format!(
                r##"<polygon fill="#181818" points="{points}" style="stroke:#181818;stroke-width:1;"/>"##,
            ));
        }
        UseCaseExtremity::Extension => {
            let points = extension_arrow_points(control, endpoint);
            svg.raw(&format!(
                r##"<polygon fill="none" points="{points}" style="stroke:#181818;stroke-width:1;"/>"##,
            ));
        }
    }
}

fn dependency_arrow_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;
    let p1 = endpoint;
    let left_wing = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK + px * DEPENDENCY_ARROW_WING,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK + py * DEPENDENCY_ARROW_WING,
    );
    let notch = (
        endpoint.0 - ux * DEPENDENCY_ARROW_NOTCH,
        endpoint.1 - uy * DEPENDENCY_ARROW_NOTCH,
    );
    let right_wing = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK - px * DEPENDENCY_ARROW_WING,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK - py * DEPENDENCY_ARROW_WING,
    );
    // Java's `ExtremityArrow.getDecorationPolygon()` emits the right wing,
    // inset notch, and left wing in that order; SVG points are comma-delimited.
    format!(
        "{},{},{},{},{},{},{},{},{},{}",
        fc(p1.0),
        fc(p1.1),
        fc(right_wing.0),
        fc(right_wing.1),
        fc(notch.0),
        fc(notch.1),
        fc(left_wing.0),
        fc(left_wing.1),
        fc(p1.0),
        fc(p1.1),
    )
}

fn extension_arrow_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let len = dx.hypot(dy).max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;
    // `LinkDecor.EXTENDS.getExtremityFactoryComplete` supplies
    // `ExtremityTriangle` with xWing=18 and yAperture=6.
    let back = (endpoint.0 - ux * 18.0, endpoint.1 - uy * 18.0);
    let left = (back.0 + px * 6.0, back.1 + py * 6.0);
    let right = (back.0 - px * 6.0, back.1 - py * 6.0);
    format!(
        "{},{},{},{},{},{},{},{}",
        fc(endpoint.0),
        fc(endpoint.1),
        fc(right.0),
        fc(right.1),
        fc(left.0),
        fc(left.1),
        fc(endpoint.0),
        fc(endpoint.1),
    )
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

    #[test]
    fn no_oracle_usecase_routes_renamed_actor_link() {
        let input = "@startuml\nactor \"Reader\" as R\nusecase \"Browse Catalog\" as Browse\nR --> Browse\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(r#"<!--link R to Browse-->"#));
        assert!(svg.contains(r#"<path d="M"#));
        assert!(svg.contains(r##"<polygon fill="#181818""##));
    }

    #[test]
    fn actor_stereotype_is_a_measured_top_tile_for_renamed_actor() {
        let input = "@startuml\nactor \"Renamed Portal\" as Portal <<externalized>>\nusecase \"Fresh Flow\" as Flow\nPortal --> Flow\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta.skinparams, None);
        let dim = super::actor_dim(&usecase.actors[0], &skin);
        let bare_stereo_w = crate::text_render::measure_with_family(
            "\u{00AB}externalized\u{00BB}",
            skin.actor_font_size as f64,
            false,
            &skin.actor_font_family,
        );

        assert_eq!(
            dim.stereo_w,
            bare_stereo_w + super::ACTOR_STEREOTYPE_MARGIN_X * 2.0
        );
        let text_block_h = super::pm::text_height(skin.actor_font_size as f64);
        assert_eq!(dim.stereo_h, text_block_h);
        assert_eq!(
            dim.height,
            super::ACTOR_STICKMAN_BASE_HEIGHT + dim.stroke_thickness * 2.0 + text_block_h * 2.0
        );
        assert_eq!(dim.paint_min_y, -(text_block_h - dim.label_gap));

        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(">Renamed Portal</text>"));
        assert!(svg.contains("\u{00AB}externalized\u{00BB}</text>"));
        assert!(svg.contains(r#"id="Portal-to-Flow""#));
        assert!(svg.contains(r##"<polygon fill="#181818" points=""##));
    }

    #[test]
    fn dependency_arrow_uses_extremity_arrow_point_order() {
        assert_eq!(
            super::dependency_arrow_points((0.0, 0.0), (0.0, 10.0)),
            "0,10,4,1,0,5,-4,1,0,10"
        );
    }

    #[test]
    fn reversed_renamed_generalization_keeps_its_hollow_start_triangle() {
        let input = "@startuml\n\
                     actor \"Policy Parent 449\" as Parent\n\
                     actor \"Policy Child 457\" as Child\n\
                     Parent <|-- Child\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"data-link-type="extension""#));
        assert!(svg.contains(r#"<polygon fill="none""#));
        assert_eq!(
            super::extension_arrow_points((0.0, 0.0), (0.0, 20.0)),
            "0,20,6,2,-6,2,0,20"
        );
    }

    #[test]
    fn renamed_generalization_canvas_uses_limit_finder_primitive_bounds() {
        let input = "@startuml\n\
                     actor \"Renamed Principal 701\" as Principal701\n\
                     actor \"Renamed Specialist 709\" as Specialist709\n\
                     usecase \"Renamed Capability 719\" as Capability719\n\
                     Specialist709 --|> Principal701\n\
                     Specialist709 --> Capability719\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Independent PlantUML oracle result for this renamed perturbation.
        // The dimensions exercise `LimitFinder.drawText` for the lower actor
        // and `drawEllipse` for the rightmost use case.
        assert!(svg.contains(r#"viewBox="0 0 402 232""#), "{svg}");
        assert!(svg.contains(r#"width="402px""#), "{svg}");
        assert!(svg.contains(r#"height="232px""#), "{svg}");
    }

    #[test]
    fn renamed_link_note_canvas_excludes_rose_trailing_padding() {
        let input = "@startuml\n\
                     actor \"Renamed Note Source 811\" as Source811\n\
                     usecase \"Renamed Note Target 821\" as Target821\n\
                     Source811 --> Target821\n\
                     note on link : Renamed approval gate 823\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Independent PlantUML oracle result for this renamed perturbation.
        assert!(svg.contains(r#"viewBox="0 0 325 236""#), "{svg}");
        assert!(svg.contains(r#"width="325px""#), "{svg}");
        assert!(svg.contains(r#"height="236px""#), "{svg}");
    }

    #[test]
    fn renamed_stereotype_uses_painted_text_footprint() {
        let input = "@startuml\n\
                     actor \"Renamed Observer 901\" as Observer901\n\
                     usecase \"Renamed Approval Path 907\" as Approval907 <<automated-variant-911>>\n\
                     Observer901 --> Approval907\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML oracle result for this renamed perturbation. The
        // stereotype is narrower than the body, so a bounding-box shortcut
        // cannot reproduce `Footprint.getEllipse`.
        assert!(svg.contains(r#"viewBox="0 0 273 211""#), "{svg}");
        assert!(svg.contains(r#"rx="126.9332" ry="27.7866""#), "{svg}");
    }

    #[test]
    fn diagonal_links_meet_the_renamed_usecase_oval() {
        let input = "@startuml\n\
                     actor \"Audit Reader 431\" as Reader\n\
                     actor \"Policy Writer 433\" as Writer\n\
                     usecase \"Review Unseen Policy 439\" as Review\n\
                     Reader --> Review\n\
                     Writer --> Review\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta.skinparams, None);
        let actor_dims: Vec<_> = usecase
            .actors
            .iter()
            .map(|actor| super::actor_dim(actor, &skin))
            .collect();
        let usecase_dims: Vec<_> = usecase
            .use_cases
            .iter()
            .map(|item| super::use_case_dim(item, &skin))
            .collect();
        let note_dims: Vec<_> = usecase.notes.iter().map(super::note_dim).collect();
        let positions =
            super::resolve_positions(usecase, &actor_dims, &usecase_dims, &note_dims, &skin, None);
        let (cx, cy) = positions.use_cases[0];
        let oval = &usecase_dims[0];
        let mut diagonal_edges = 0;

        for edge in &positions.edge_paths {
            let endpoint = edge
                .end_point
                .or_else(|| edge.points.last().copied())
                .unwrap();
            let normalized =
                ((endpoint.0 - cx) / oval.rx).powi(2) + ((endpoint.1 - cy) / oval.ry).powi(2);
            assert!(
                (normalized - 1.0).abs() < 0.08,
                "diagonal endpoint {endpoint:?} must meet oval centered at ({cx}, {cy})"
            );
            let source_x = usecase
                .actors
                .iter()
                .position(|actor| actor.id == edge.from)
                .map(|index| positions.actors[index].0)
                .unwrap();
            if (source_x - cx).abs() > 1.0 {
                diagonal_edges += 1;
            }
        }
        assert!(diagonal_edges > 0);
    }

    #[test]
    fn lone_renamed_usecase_uses_degenerated_entity_inset() {
        let input = "@startuml\nusecase \"Fresh Singleton\" as Singleton\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta.skinparams, None);
        let dim = super::use_case_dim(&usecase.use_cases[0], &skin);
        let expected_cx = super::fc(super::DEGENERATED_MARGIN + dim.rx);
        let expected_cy = super::fc(super::DEGENERATED_MARGIN + dim.ry);
        assert!(svg.contains(&format!(
            r#"<ellipse cx="{expected_cx}" cy="{expected_cy}""#
        )));
    }

    #[test]
    fn skinparams_style_actor_and_usecase_text() {
        let input = r##"@startuml
skinparam backgroundColor transparent
skinparam defaultFontName "Verdana"
skinparam defaultFontSize 12
skinparam actorFontColor #fff
skinparam actorBorderColor #78c2ad
skinparam actorBackgroundColor #86c8b5
skinparam __styleRootLineThickness 1
skinparam usecaseFontColor #fff
skinparam usecaseBorderColor #78c2ad
skinparam usecaseBackgroundColor #86c8b5
skinparam usecaseBorderThickness 2
actor User
usecase "Login" as UC1
User --> UC1
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(r#"style="width:"#));
        assert!(!svg.contains("background:#FFFFFF;"));
        assert!(svg.contains(r##"fill="#86C8B5""##));
        assert!(svg.contains(r#"stroke:#78C2AD;stroke-width:1;"#));
        assert!(svg.contains(r#"stroke:#78C2AD;stroke-width:2;"#));
        assert!(svg.contains(r#"font-family="'Verdana'""#));
        assert!(svg.contains(r#"font-size="12""#));
        assert!(svg.contains(r##"fill="#FFFFFF""##));
    }
}
