// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's use-case diagram rendering.
//! PlantUML emits use-case diagrams as `data-diagram-type="DESCRIPTION"` —
//! the same envelope used by component and deployment diagrams.

use std::collections::HashMap;

use rustuml_parser::diagram::usecase::*;

use crate::layout_oracle::OracleLayout;
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

/// Round a coordinate to PlantUML's 4-decimal format, dropping trailing zeros.
fn fc(v: f64) -> String {
    pm::fmt_coord(v)
}

pub fn render(diagram: &UseCaseDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

pub fn render_with_oracle(
    diagram: &UseCaseDiagram,
    _theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
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

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else {
        compute_canvas(&positions, &actor_dims, &uc_dims)
    };

    let mut svg = SvgBuilder::new_plantuml(total_w, total_h, "DESCRIPTION");
    render_packages(&mut svg, diagram, oracle, &id_map);

    for (i, actor) in diagram.actors.iter().enumerate() {
        let (cx, cy) = positions.actors[i];
        render_actor(&mut svg, actor, &actor_dims[i], cx, cy, &id_map);
    }
    for (i, uc) in diagram.use_cases.iter().enumerate() {
        let (cx, cy) = positions.use_cases[i];
        render_use_case(&mut svg, uc, &uc_dims[i], diagram, cx, cy, oracle, &id_map);
    }
    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc);
    }
    svg.finalize_plantuml()
}

/// Assign PlantUML-compatible entity IDs by sorting actors, use cases, and
/// packages by `source_line` and numbering sequentially from `ent0002`.
fn build_entity_id_map(diagram: &UseCaseDiagram) -> HashMap<String, String> {
    struct Entry {
        key: String,
        line: usize,
    }
    let mut entries: Vec<Entry> = Vec::new();
    for a in &diagram.actors {
        entries.push(Entry {
            key: format!("actor::{}", a.id),
            line: a.source_line,
        });
    }
    for uc in &diagram.use_cases {
        entries.push(Entry {
            key: format!("uc::{}", uc.id),
            line: uc.source_line,
        });
    }
    // Packages have no source_line in the model; pin them at the lowest line
    // of their first declared child entity (a reasonable proxy for declaration
    // order). Fall back to declaration index when empty.
    for (i, p) in diagram.packages.iter().enumerate() {
        let line = p
            .elements
            .iter()
            .filter_map(|eid| {
                diagram
                    .actors
                    .iter()
                    .find(|a| &a.id == eid)
                    .map(|a| a.source_line)
                    .or_else(|| {
                        diagram
                            .use_cases
                            .iter()
                            .find(|u| &u.id == eid)
                            .map(|u| u.source_line)
                    })
            })
            .min()
            .map(|l| l.saturating_sub(1))
            .unwrap_or(i);
        entries.push(Entry {
            key: format!("pkg::{}", p.name),
            line,
        });
    }
    entries.sort_by_key(|e| e.line);
    let mut map = HashMap::new();
    let mut counter = 2usize;
    for e in entries {
        map.insert(e.key, format!("ent{counter:04}"));
        counter += 1;
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

fn qualified_name(id: &str, diagram: &UseCaseDiagram) -> String {
    for pkg in &diagram.packages {
        if pkg.elements.iter().any(|e| e == id) {
            return format!("{}.{id}", pkg.name);
        }
    }
    id.to_string()
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

fn render_packages(
    svg: &mut SvgBuilder,
    diagram: &UseCaseDiagram,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
) {
    for pkg in &diagram.packages {
        let Some(orc) = oracle else { continue };
        let Some(rect) = orc.entities.get(&pkg.name) else {
            continue;
        };
        let ent_id = id_map
            .get(&format!("pkg::{}", pkg.name))
            .cloned()
            .unwrap_or_else(|| "ent0003".to_string());
        svg.raw(&format!("<!--cluster {}-->", pkg.name));
        svg.raw(&format!(
            r#"<g class="cluster" data-qualified-name="{}" id="{ent_id}">"#,
            pkg.name
        ));
        svg.raw(&format!(
            r#"<rect fill="none" height="{h}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
            h = fc(rect.height),
            w = fc(rect.width),
            x = fc(rect.x),
            y = fc(rect.y),
        ));
        let label_w = text_render::measure(&pkg.name, FONT_SIZE, true);
        let label_x = rect.x + (rect.width - label_w) / 2.0;
        let label_y = rect.y + 15.5352;
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
}

fn render_actor(
    svg: &mut SvgBuilder,
    actor: &Actor,
    dim: &ActorDim,
    cx: f64,
    cy: f64,
    id_map: &HashMap<String, String>,
) {
    let ent_id = id_map
        .get(&format!("actor::{}", actor.id))
        .cloned()
        .unwrap_or_else(|| "ent0002".to_string());
    svg.raw(&format!("<!--entity {}-->", actor.id));
    let src_attr = source_line_attr(actor.source_line);
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{}"{src_attr} id="{ent_id}">"#,
        actor.id
    ));
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{ENTITY_FILL}" rx="{ACTOR_HEAD_R}" ry="{ACTOR_HEAD_R}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
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
        r#"<path d="M{cx},{body_top_y} L{cx},{body_bot_y} M{arm_left_x},{arm_y} L{arm_right_x},{arm_y} M{cx},{body_bot_y} L{leg_x_left},{leg_y} M{cx},{body_bot_y} L{leg_x_right},{leg_y}" fill="none" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
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
    let label_x = cx - dim.label_w / 2.0;
    let label_y = leg_y + ACTOR_LABEL_GAP;
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
        let stereo_x = cx - dim.stereo_w / 2.0;
        let stereo_y = cy - ACTOR_STEREO_OFFSET;
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

fn render_use_case(
    svg: &mut SvgBuilder,
    uc: &UseCase,
    dim: &UseCaseDim,
    diagram: &UseCaseDiagram,
    cx: f64,
    cy: f64,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
) {
    let qualified = qualified_name(&uc.id, diagram);
    let ent_id = id_map
        .get(&format!("uc::{}", uc.id))
        .cloned()
        .unwrap_or_else(|| "ent0003".to_string());
    svg.raw(&format!("<!--entity {}-->", uc.id));
    let src_attr = source_line_attr(uc.source_line);
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qualified}"{src_attr} id="{ent_id}">"#,
    ));
    let (rx, ry) = if let Some(orc) = oracle
        && let Some(rect) = orc
            .entities
            .get(&qualified)
            .or_else(|| orc.entities.get(&uc.id))
            .or_else(|| orc.entities.get(&uc.label))
    {
        (rect.width / 2.0, rect.height / 2.0)
    } else {
        (dim.rx, dim.ry)
    };
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{ENTITY_FILL}" rx="{rx}" ry="{ry}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        rx = fc(rx),
        ry = fc(ry),
    ));
    let n_lines = dim.line_count;
    let bottom_y = cy + UC_TEXT_OFFSET_SINGLE + (n_lines as f64 - 1.0) * LINE_H;
    let mut text_y = bottom_y - (n_lines as f64 - 1.0) * LINE_H;
    if dim.line_count >= 2 && uc.stereotype.is_some() {
        text_y = cy + 0.0304;
    }
    if let Some(stereo) = &uc.stereotype {
        let stereo_text = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_x = cx - dim.stereo_w / 2.0;
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &stereo_text,
            &TextBase {
                x: stereo_x,
                y: text_y,
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
        let label_x = cx - dim.label_w / 2.0;
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &uc.label,
            &TextBase {
                x: label_x,
                y: text_y,
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
        for line in &uc.description {
            let lw = text_render::measure(line, FONT_SIZE, false);
            let lx = cx - lw / 2.0;
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                line,
                &TextBase {
                    x: lx,
                    y: text_y,
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

fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &UseCaseDiagram,
    oracle: &OracleLayout,
) {
    for conn in &diagram.connections {
        let id_arrow = format!("{}-to-{}", conn.from, conn.to);
        let id_plain = format!("{}-{}", conn.from, conn.to);
        let id_back = format!("{}-backto-{}", conn.from, conn.to);
        let oracle_edge = oracle
            .edges
            .iter()
            .find(|e| e.id == id_arrow || e.id == id_plain || e.id == id_back);
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
        svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));
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
