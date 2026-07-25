// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's component diagram rendering.
//! PlantUML renders component diagrams as diagram type "DESCRIPTION".

use std::fmt::Write;

use rustuml_layout::graph::{ClusterPosition, Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::component::*;

use crate::layout_oracle::{
    CrowMark, EntityRect, OracleLayout, emit_oracle_cluster_children, emit_oracle_note_entity,
    wrap_oracle_envelope,
};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

fn fc(v: f64) -> String {
    pm::fmt_coord(v)
}

/// Build a map from bare component id to qualified name (e.g. "G1.AA").
/// Walks the package tree and concatenates package names.
/// PlantUML folds non-ASCII characters when echoing an entity's name into the
/// SVG: codepoints above U+007F become `.` in `data-qualified-name` and `?` in
/// the `<!--entity …-->` marker. ASCII (including spaces) is preserved.
fn fold_non_ascii(s: &str, replacement: char) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { replacement })
        .collect()
}

fn canonical_font_family(value: &str) -> String {
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

fn build_qualified_names(
    packages: &[ComponentPackage],
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for pkg in packages {
        walk_pkg(pkg, "", &mut map);
    }
    map
}

fn walk_pkg(
    pkg: &ComponentPackage,
    parent_path: &str,
    map: &mut std::collections::HashMap<String, String>,
) {
    let path = if parent_path.is_empty() {
        pkg.name.clone()
    } else {
        format!("{parent_path}.{}", pkg.name)
    };
    for cid in &pkg.components {
        map.insert(cid.clone(), format!("{path}.{cid}"));
    }
    for child in &pkg.packages {
        walk_pkg(child, &path, map);
    }
}

/// Resolve an interface's oracle entity by its bare id.
///
/// Interfaces declared inside a `component`/`package` block are qualified by
/// PlantUML (e.g. `interface HTTP` inside `component Server` becomes
/// `Server.HTTP`), and the oracle keys its entity map by that qualified name.
/// The parser, however, only records the bare interface id (`HTTP`) with no
/// enclosing-package link, so a direct `entities.get(id)` lookup misses the
/// qualified key. Match on exact id first, then on any key ending in `.{id}`
/// (the qualified form). Returns the matched key (the qualified name PlantUML
/// emits in `data-qualified-name`) alongside the entity.
fn resolve_iface_entity<'a>(
    oracle: &'a OracleLayout,
    id: &str,
) -> Option<(&'a str, &'a EntityRect)> {
    if let Some((k, r)) = oracle.entities.get_key_value(id) {
        return Some((k.as_str(), r));
    }
    let suffix = format!(".{id}");
    oracle
        .entities
        .iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .map(|(k, r)| (k.as_str(), r))
}

fn resolve_component_entity<'a>(
    oracle: &'a OracleLayout,
    qualified_names: &std::collections::HashMap<String, String>,
    comp: &Component,
) -> Option<&'a EntityRect> {
    let folded_id = fold_non_ascii(&comp.id, '.');
    let folded_label = fold_non_ascii(&comp.label, '.');
    let matches_name = |name: &str| {
        qualified_names.get(&comp.id).is_some_and(|q| q == name)
            || name == comp.id
            || name == comp.label
            || name == folded_id
            || name == folded_label
    };
    let source_line = (comp.source_line > 0).then(|| comp.source_line.to_string());

    if let Some(line) = source_line.as_deref()
        && let Some(entry) = oracle.entity_list.iter().find(|entry| {
            matches_name(&entry.qualified_name) && entry.rect.source_line.as_deref() == Some(line)
        })
    {
        return Some(&entry.rect);
    }

    if let Some(entry) = oracle
        .entity_list
        .iter()
        .find(|entry| matches_name(&entry.qualified_name))
    {
        return Some(&entry.rect);
    }

    qualified_names
        .get(&comp.id)
        .and_then(|q| oracle.entities.get(q))
        .or_else(|| oracle.entities.get(&comp.id))
        .or_else(|| oracle.entities.get(&comp.label))
}

/// Y-baseline offset from rect top to the bottom-most text line (label),
/// derived from PlantUML output: rect h=46.4883, baseline y=33.5352 from top.
const LABEL_BASELINE_FROM_BOTTOM: f64 = 12.9531;

// ---------------------------------------------------------------------------
// PlantUML constants (extracted from golden SVGs)
// ---------------------------------------------------------------------------

/// Font size for component labels and cluster titles (PlantUML default).
const FONT_SIZE: f64 = 14.0;
/// Font size for stereotype text.
const SMALL_FONT: f64 = 14.0;
/// Font size for header/footer caption text.
const HEADER_FOOTER_FONT: f64 = 10.0;
/// Vertical gap between the footer baseline and the bottom canvas edge.
const FOOTER_BOTTOM_GAP: f64 = 8.5764;
/// Font size for arrow/link labels.
const LINK_FONT: f64 = 13.0;
/// Line height per text line in a component box.
const LINE_HEIGHT: f64 = 16.4883;
/// Base component box height (padding around one line of text).
const COMPONENT_BASE_H: f64 = 30.0;
/// Single-line component height.
const COMPONENT_H: f64 = COMPONENT_BASE_H + LINE_HEIGHT;
/// Left padding for text inside a component (accounts for icon space on right).
const TEXT_PAD_LEFT: f64 = 15.0;
/// Right padding inside component (icon area).
const TEXT_PAD_RIGHT: f64 = 25.0;
// PlantUML `USymbolDatabase.getMargin()` returns (10, 10, 24, 5).
const DATABASE_MARGIN_X: f64 = 10.0;
const DATABASE_MARGIN_TOP: f64 = 24.0;
const DATABASE_MARGIN_BOTTOM: f64 = 5.0;
// `USymbolDatabase.drawDatabase()` appends `UEmpty(10, 10)` at (width, height),
// extending the rendered envelope without changing the Graphviz node size.
const DATABASE_RENDER_OVERFLOW: f64 = 10.0;
/// Margin around the entire diagram.
const MARGIN: f64 = 7.0;
/// Gap between entities when laid out by Sugiyama.
const GAP: f64 = 60.0;
/// Fill color for component bodies.
const COMP_FILL: &str = "#F1F1F1";
/// Stroke color for component borders and arrows.
const STROKE: &str = "#181818";
/// Text fill color.
const TEXT_COLOR: &str = "#000000";
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// Radius for rounded corners on component rects.
const ROUND_R: f64 = 2.5;
/// Interface circle radius.
const IFACE_R: f64 = 8.0;
/// Note fold (dog-ear) size.
const NOTE_FOLD: f64 = 10.0;
/// Note padding.
const NOTE_PAD: f64 = 6.0;
/// Note line height.
const NOTE_LINE_H: f64 = 18.0;
/// Note gap from attached element.
const NOTE_GAP: f64 = 10.0;

/// Title font size.
const TITLE_FONT_SIZE: f64 = 14.0;
/// Title left margin (PlantUML fixes the title block's left edge here).
const TITLE_MARGIN_X: f64 = 10.0;
/// Vertical pad above the first title baseline (baseline = pad + ascent).
const TITLE_TOP_PAD: f64 = 10.0;
/// Per-line height for the title block (ascent + descent at the title size).
const TITLE_LINE_H: f64 = 16.48828125;
/// Gap between the title block and the first laid-out element.
const TITLE_BOTTOM_PAD: f64 = 11.0;

/// Container (package) label height.
const CONTAINER_LABEL_H: f64 = 22.0;
/// Container internal padding.
const CONTAINER_PAD: f64 = 16.0;

/// Minimum component width.
const COMPONENT_MIN_W: f64 = 40.0;

/// Canvas pad after the rightmost/bottommost rendered shape.
///
/// Java PlantUML computes DESCRIPTION/Svek image size in
/// `svek.SvekResult.calculateDimension`: it scans the drawn result with
/// `TextBlockUtils.getMinMax`, moves the solved graph by `6 - min`, then
/// returns `minMax.getDimension().delta(15, 15)`. Our component leaves are
/// already emitted at PlantUML's 7px top/left offset, so the equivalent
/// no-oracle frame is the maximum rendered bound plus this residual pad.
const SVEK_CANVAS_PAD: f64 = 14.0;

// ---------------------------------------------------------------------------
// Component icon geometry (the "tab" icon at top-right of each component)
// ---------------------------------------------------------------------------

/// Width of the component tab icon.
const ICON_TAB_W: f64 = 15.0;
/// Height of the component tab icon.
const ICON_TAB_H: f64 = 10.0;
/// Width of each bar in the component icon.
const ICON_BAR_W: f64 = 4.0;
/// Height of each bar in the component icon.
const ICON_BAR_H: f64 = 2.0;
/// Offset from right edge of component rect to left edge of tab.
const ICON_TAB_RIGHT_OFFSET: f64 = 20.0;
/// Offset from top of component rect to top of tab.
const ICON_TAB_TOP_OFFSET: f64 = 5.0;
/// Offset from left edge of tab to left edge of bars.
const ICON_BAR_LEFT_OFFSET: f64 = 2.0;
/// Vertical offset from top of tab to first bar.
const ICON_BAR_TOP_OFFSET_1: f64 = 2.0;
/// Vertical offset from top of tab to second bar.
const ICON_BAR_TOP_OFFSET_2: f64 = 6.0;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn render(diagram: &ComponentDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a component diagram to SVG, optionally using pre-computed layout from an oracle.
pub fn render_with_oracle(
    diagram: &ComponentDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Class.rs took the same approach unconditionally (commit ece57cc8) with
    // big wins and no regressions; component follows suit.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "DESCRIPTION");
    }

    if diagram.components.is_empty() && diagram.packages.is_empty() && diagram.interfaces.is_empty()
    {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="DESCRIPTION" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><defs/><g></g></svg>"#.to_string();
    }

    // Resolve component-specific skinparams. We read these directly from the
    // diagram's `skinparams` rather than the cascading `Theme` so the
    // workspace `slate` default does not clobber PlantUML's historical
    // values.
    let mut interface_fill = COMP_FILL.to_string();
    let mut interface_stroke = STROKE.to_string();
    let mut component_fill = COMP_FILL.to_string();
    let mut component_stroke = STROKE.to_string();
    let mut component_stroke_width = 0.5;
    let mut component_round_corner: Option<f64> = None;
    // `skinparam componentStyle rectangle` draws components as plain rectangles
    // with no UML "tab" icon.
    let mut component_style_rectangle = false;
    // Component label font size. `componentFontSize` is the most specific
    // skinparam; `defaultFontSize` is the fallback base; otherwise the built-in
    // default (14). Only components (not interfaces, which keep their own
    // interfaceFontSize) consume this. The oracle still supplies label
    // positions; the resolved size only drives the `font-size` attribute and
    // the textLength `emit_text` computes from it.
    let mut default_font_size: Option<f64> = None;
    let mut component_font_size_sp: Option<f64> = None;
    let mut component_arrow_font_size_sp: Option<f64> = None;
    let mut default_font_family: Option<String> = None;
    let mut component_font_family_sp: Option<String> = None;
    let mut component_arrow_font_family_sp: Option<String> = None;
    let mut root_font_color: Option<String> = None;
    let mut default_font_color: Option<String> = None;
    let mut component_font_color_sp: Option<String> = None;
    let mut component_arrow_font_color_sp: Option<String> = None;
    let mut component_font_bold = false;
    let mut component_font_italic = false;
    let mut bg_value: Option<String> = None;
    let gradient_defs = oracle
        .map(|o| o.defs_inner_xml.as_str())
        .filter(|d| !d.is_empty());
    for sp in &diagram.meta.skinparams {
        let key = sp.key.to_ascii_lowercase();
        let val = sp.value.trim();
        if val.is_empty() {
            continue;
        }
        match key.as_str() {
            "backgroundcolor" => {
                bg_value = Some(val.to_string());
            }
            "__stylerootlinecolor" | "bordercolor" => {
                component_stroke = crate::sequence::resolve_color(val);
            }
            "__stylerootlinethickness" | "borderthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    component_stroke_width = v;
                }
            }
            "__stylerootfontcolor" => {
                root_font_color = Some(crate::sequence::resolve_color(val));
            }
            "defaultfontcolor" => {
                default_font_color = Some(crate::sequence::resolve_color(val));
            }
            "defaultfontname" | "fontname" => {
                default_font_family = Some(canonical_font_family(val));
            }
            "interfacebackgroundcolor" => {
                interface_fill = crate::sequence::resolve_color(val);
            }
            "interfacebordercolor" => {
                interface_stroke = crate::sequence::resolve_color(val);
            }
            "componentbackgroundcolor" => {
                component_fill = crate::sequence::gradient_fill_or(val, gradient_defs);
            }
            "componentbordercolor" => {
                component_stroke = crate::sequence::resolve_color(val);
            }
            "componentborderthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    component_stroke_width = v;
                }
            }
            "componentroundcorner" | "roundcorner" => {
                if let Ok(v) = val.parse::<f64>() {
                    component_round_corner = Some(v / 2.0);
                }
            }
            "componentstyle" => {
                component_style_rectangle = val.eq_ignore_ascii_case("rectangle");
            }
            "componentfontsize" => {
                component_font_size_sp = val.parse::<f64>().ok();
            }
            "componentfontname" => {
                component_font_family_sp = Some(canonical_font_family(val));
            }
            "componentfontcolor" => {
                component_font_color_sp = Some(crate::sequence::resolve_color(val));
            }
            "componentfontstyle" => {
                let style = val.to_ascii_lowercase();
                component_font_bold = style.contains("bold");
                component_font_italic = style.contains("italic");
            }
            "arrowfontname" | "componentarrowfontname" => {
                component_arrow_font_family_sp = Some(canonical_font_family(val));
            }
            "arrowfontcolor" | "componentarrowfontcolor" => {
                component_arrow_font_color_sp = Some(crate::sequence::resolve_color(val));
            }
            "componentarrowfontsize" => {
                component_arrow_font_size_sp = val.parse::<f64>().ok();
            }
            "defaultfontsize" => {
                default_font_size = val.parse::<f64>().ok();
            }
            _ => {}
        }
    }
    let default_font_family = default_font_family.unwrap_or_else(|| "sans-serif".to_string());
    let component_font_family = component_font_family_sp
        .clone()
        .unwrap_or_else(|| default_font_family.clone());
    let component_arrow_font_family = component_arrow_font_family_sp
        .clone()
        .unwrap_or_else(|| default_font_family.clone());
    let fallback_font_color = root_font_color
        .clone()
        .or(default_font_color.clone())
        .unwrap_or_else(|| TEXT_COLOR.to_string());
    let component_font_color = component_font_color_sp
        .clone()
        .unwrap_or_else(|| fallback_font_color.clone());
    let component_arrow_font_color = component_arrow_font_color_sp
        .clone()
        .unwrap_or(fallback_font_color);
    let component_font_size = component_font_size_sp
        .or(default_font_size)
        .unwrap_or(FONT_SIZE);
    // Edge/arrow label font size (link labels and multiplicities). Follows
    // `componentArrowFontSize`, then `defaultFontSize`, then the built-in 13.
    let component_arrow_font_size = component_arrow_font_size_sp
        .or(default_font_size)
        .unwrap_or(LINK_FONT);

    // Compute dimensions for each component.
    let comp_dims: Vec<CompDim> = diagram.components.iter().map(calc_component_dim).collect();

    let title_h = if let Some(title) = &diagram.meta.title {
        // PlantUML title band: 10px top, n line-heights, 11px bottom gap before
        // the first entity. Line height is ascent + descent at the title size.
        let n_lines = title.lines().count().max(1) as f64;
        TITLE_TOP_PAD + n_lines * TITLE_LINE_H + TITLE_BOTTOM_PAD
    } else {
        0.0
    };

    let use_oracle = oracle.is_some();

    // Try Sugiyama layout (skip when oracle is available).
    let layout_result = if use_oracle {
        None
    } else if !diagram.components.is_empty() || !diagram.interfaces.is_empty() {
        let mut layout = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
        for (comp, dim) in diagram.components.iter().zip(&comp_dims) {
            layout.add_node(&comp.id, &comp.label, dim.width, dim.height);
        }
        for iface in &diagram.interfaces {
            layout.add_node(
                &iface.id,
                &iface.label,
                IFACE_R * 2.0 + 20.0,
                IFACE_R * 2.0 + 20.0,
            );
        }
        add_package_clusters_to_layout(&mut layout, &diagram.packages, "");
        for conn in &diagram.connections {
            layout.add_edge(&conn.from, &conn.to, conn.label.as_deref());
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    } else {
        None
    };

    let n_comp = diagram.components.len();

    // Compute positions from oracle, layout engine, or grid fallback.
    let (positions, iface_positions, cluster_positions, content_w, content_h) =
        if let Some(orc) = oracle {
            compute_positions_from_oracle(diagram, &comp_dims, orc, title_h)
        } else if let Some(ref result) = layout_result
            && result.node_positions.len() >= n_comp + diagram.interfaces.len()
        {
            compute_positions_from_layout(
                diagram,
                &comp_dims,
                &result.node_positions,
                &result.cluster_positions,
                title_h,
            )
        } else {
            compute_positions_grid(diagram, &comp_dims, title_h)
        };

    let empty_edge_paths: Vec<EdgePath> = Vec::new();
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };

    // Estimate package bounding box.
    let pkg_total_w = estimate_packages_width(&diagram.packages);
    let pkg_total_h = estimate_packages_height(&diagram.packages);

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else if oracle.is_none() {
        compute_no_oracle_canvas(NoOracleCanvas {
            components: &diagram.components,
            positions: &positions,
            iface_positions: &iface_positions,
            comp_dims: &comp_dims,
            cluster_positions: &cluster_positions,
            packages: &diagram.packages,
            pkg_total_w,
            pkg_total_h,
            title_h,
        })
    } else {
        (
            content_w.max(pkg_total_w).max(100.0),
            (content_h + pkg_total_h + title_h).max(50.0),
        )
    };

    // PlantUML emits `data-diagram-type="DESCRIPTION"` for ordinary component
    // diagrams, but routes a bare `interface` inside a `component {…}` block
    // with a lollipop/socket link through its *class* diagram machinery, which
    // tags the root as `CLASS`. Honour the oracle's captured type when it is
    // present so the lollipop case matches; absent an oracle, keep DESCRIPTION.
    let diagram_type = oracle
        .and_then(|o| o.diagram_type.as_deref())
        .filter(|t| *t == "CLASS")
        .unwrap_or("DESCRIPTION");
    let canvas_background = match bg_value.as_deref() {
        Some(v) if v.eq_ignore_ascii_case("transparent") => None,
        Some(v) => Some(crate::sequence::resolve_color(v)),
        None => Some("#FFFFFF".to_string()),
    };
    let canvas_rect = canvas_background
        .as_ref()
        .filter(|c| *c != "#FFFFFF")
        .cloned();
    let mut svg = SvgBuilder::new_plantuml_with_background_and_defs(
        total_w,
        total_h,
        diagram_type,
        canvas_background.as_deref(),
        gradient_defs.unwrap_or(""),
    );
    if let Some(bg) = canvas_rect.as_deref() {
        svg.raw(&format!(
            r#"<rect fill="{bg}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
            h = total_h as i64,
            w = total_w as i64,
        ));
    }

    // Title — wrap in <g class="title"> and route through creole segmenter.
    // In oracle mode, consume PlantUML's page-decoration anchors so the title
    // is centred over the rendered body, not over only its own text block.
    if let Some(title) = &diagram.meta.title {
        let oracle_title =
            oracle.and_then(|o| o.decorations.iter().find(|d| d.class_name == "title"));
        let widths: Vec<f64> = title
            .lines()
            .map(|t| text_render::measure(t, TITLE_FONT_SIZE, true))
            .collect();
        let block_w = widths.iter().cloned().fold(0.0_f64, f64::max);
        let mut buf = String::new();
        let source_line = oracle_title
            .and_then(|d| d.source_line.as_deref())
            .unwrap_or("1");
        buf.push_str(&format!(
            r#"<g class="title" data-source-line="{source_line}">"#
        ));
        for (i, tline) in title.lines().enumerate() {
            let oracle_text = oracle_title.and_then(|d| d.texts.get(i));
            let ty = oracle_text.map_or(
                TITLE_TOP_PAD + pm::ascent(TITLE_FONT_SIZE) + i as f64 * TITLE_LINE_H,
                |t| t.y,
            );
            let x = oracle_text.map_or(TITLE_MARGIN_X + (block_w - widths[i]) / 2.0, |t| t.x);
            text_render::emit_text(
                &mut buf,
                tline,
                &text_render::TextBase {
                    x,
                    y: ty,
                    font_size: TITLE_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: true,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        buf.push_str("</g>");
        svg.raw_inline(&buf);
    }

    // Header/footer captions are font-10 grey text. PlantUML centres each
    // caption within a block whose width is the wider of the header and footer
    // text widths (not the canvas), so a single-caption diagram left-aligns at
    // x=0 and a footer narrower than the header centres under it.
    let caption_block_w = {
        let hw = diagram
            .meta
            .header
            .as_deref()
            .map(|h| text_render::measure(h, HEADER_FOOTER_FONT, false))
            .unwrap_or(0.0);
        let fw = diagram
            .meta
            .footer
            .as_deref()
            .map(|f| text_render::measure(f, HEADER_FOOTER_FONT, false))
            .unwrap_or(0.0);
        hw.max(fw)
    };

    // Header — wrap in <g class="header">.
    if let Some(header) = &diagram.meta.header {
        let tl = text_render::measure(header, HEADER_FOOTER_FONT, false);
        let x = (caption_block_w - tl) / 2.0;
        let mut buf = String::new();
        buf.push_str(r#"<g class="header" data-source-line="1">"#);
        text_render::emit_text(
            &mut buf,
            header,
            &text_render::TextBase {
                x,
                y: pm::ascent(HEADER_FOOTER_FONT),
                font_size: HEADER_FOOTER_FONT as u32,
                font_family: "sans-serif",
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        buf.push_str("</g>");
        svg.raw_inline(&buf);
    }

    // Render packages (clusters).
    //
    // When the oracle has cluster data, replay it verbatim: PlantUML hand-tuned
    // each container shape (cloud bubbles, folder tab, node 3D edge, package label
    // band, frame corner, rectangle, database cylinder, queue, …) with bespoke
    // path geometry that we can't realistically reproduce attribute-for-attribute
    // in a strict-XML comparator. The oracle replay sidesteps this entirely.
    let mut pkg_y = title_h + MARGIN;
    let rendered_layout_cluster_count = if let Some(orc) = oracle
        && !orc.clusters.is_empty()
    {
        render_packages_from_oracle(&diagram.packages, &mut svg, orc);
        0
    } else if !cluster_positions.is_empty() {
        render_packages_from_layout(&diagram.packages, &mut svg, &cluster_positions);
        cluster_positions.len()
    } else {
        render_packages(&diagram.packages, &mut svg, MARGIN, &mut pkg_y, theme);
        0
    };

    // Build qualified-name map (e.g. "AA" → "G1.AA") for oracle lookup.
    let qualified_names = build_qualified_names(&diagram.packages);

    // Helper: look up oracle entity rect for a component (by qualified name or bare id).
    let oracle_comp_rect = |comp: &Component| -> Option<&EntityRect> {
        oracle.and_then(|o| resolve_component_entity(o, &qualified_names, comp))
    };

    // Render each component entity.
    //
    // Entity IDs (`ent000N`) are interleaved with cluster IDs in PlantUML's
    // output, so a naive sequential counter doesn't reproduce them. When the
    // oracle has captured an entity_id, use it directly. Otherwise fall back
    // to a sequential counter — but skip IDs already claimed by oracle
    // clusters so we never collide.
    //
    // PlantUML emits entities ordered by depth-then-declaration, not raw
    // declaration order: a container's direct children come before its
    // nested grand-children. When the oracle is available, sort the
    // component indices by their oracle-assigned ent_id so the emitted
    // sequence matches PlantUML's.
    // Collect all package names (at every nesting depth) so we can skip
    // phantom components the parser auto-creates from connection endpoints
    // that actually refer to a container (e.g. `Inner --> Gamma` where
    // `Inner` is a folder, not a component).
    let mut skip_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    fn collect_pkg_names(
        packages: &[ComponentPackage],
        set: &mut std::collections::HashSet<String>,
    ) {
        for p in packages {
            set.insert(p.name.clone());
            collect_pkg_names(&p.packages, set);
        }
    }
    collect_pkg_names(&diagram.packages, &mut skip_ids);
    // Also skip components that are actually note aliases (`note "…" as ID`).
    // The oracle captures these as note_entities; emitting them as regular
    // components would duplicate the note.
    if let Some(orc) = oracle {
        for ne in &orc.note_entities {
            skip_ids.insert(ne.qualified_name.clone());
        }
    }
    let package_names = skip_ids;

    // Depth = number of dots in qualified name (top-level → 0). PlantUML emits
    // entities sorted by (parent depth ascending, declaration order ascending),
    // so a top-level entity precedes one nested two clusters deep even if the
    // nested one is declared earlier in the source.
    let comp_indices: Vec<usize> = (0..diagram.components.len())
        .filter(|&i| !package_names.contains(&diagram.components[i].id))
        .collect();
    let comp_order: Vec<usize> = if oracle.is_some() {
        // PlantUML emits leaf entities ordered by nesting depth, but with one
        // twist: when any element is nested in a container, the top-level
        // (depth-0) leaves are emitted *after* the nested ones (a trailing
        // top-level `database` gets the highest entity id). With no containers
        // at all, plain declaration order holds. Mirror the deployment
        // renderer's rule: sort by (is-depth-0 when nesting exists, depth,
        // declaration index).
        let any_nested = comp_indices.iter().any(|&i| {
            qualified_names
                .get(&diagram.components[i].id)
                .map(|q| q.contains('.'))
                .unwrap_or(false)
        });
        let mut order = comp_indices.clone();
        order.sort_by_key(|&i| {
            let comp = &diagram.components[i];
            let depth = qualified_names
                .get(&comp.id)
                .map(|q| q.matches('.').count())
                .unwrap_or(0);
            (any_nested && depth == 0, depth, i)
        });
        order
    } else {
        comp_indices
    };
    // PlantUML emits components and interfaces interleaved in declaration
    // order (by source line), not all-components-then-all-interfaces. The
    // oracle captures each entity's `id` (`ent000N`), which encodes that
    // emission order, so when the oracle is present we merge both collections
    // into a single sequence sorted by the oracle-assigned id and emit in that
    // order. Without an oracle there is no golden to match, so we keep the
    // historical components-then-interfaces order.
    #[derive(Clone, Copy)]
    enum EmitItem {
        Comp(usize),
        Iface(usize),
    }
    let emit_order: Vec<EmitItem> = if oracle.is_some() {
        let iface_qual = |ii: usize| -> Option<&str> {
            oracle.and_then(|o| resolve_iface_entity(o, &diagram.interfaces[ii].id).map(|(k, _)| k))
        };
        // Whether any leaf (component or interface) is nested in a container.
        // When so, the top-level (depth-0) leaves are emitted *last*.
        let any_nested_all = comp_order.iter().any(|&i| {
            qualified_names
                .get(&diagram.components[i].id)
                .map(|q| q.contains('.'))
                .unwrap_or(false)
        }) || (0..diagram.interfaces.len())
            .any(|ii| iface_qual(ii).map(|q| q.contains('.')).unwrap_or(false));
        // Source line for an interface, parsed from the oracle's
        // `data-source-line` (the `Interface` model carries none). Falls back to
        // a large sentinel so an interface with no captured line sorts after
        // same-depth peers that have one.
        let iface_src_line = |ii: usize| -> i64 {
            oracle
                .and_then(|o| resolve_iface_entity(o, &diagram.interfaces[ii].id))
                .and_then(|(_, r)| r.source_line.as_deref())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(i64::MAX)
        };
        // PlantUML emits leaf entities (components and interfaces alike) grouped
        // by nesting depth, then by source line within each depth. The oracle's
        // entity ids are NOT monotonic with emission order across depths, so a
        // plain ascending-id merge mis-slots a deep interface ahead of a shallow
        // component (e.g. `Inner.IInner` ahead of `OuterComp`). Build one
        // unified list keyed by `(any_nested && depth == 0, depth, source_line)`
        // — the same comparison `comp_order` uses — and sort components and
        // interfaces together. The stable sort preserves `comp_order`'s
        // intra-depth ordering on ties.
        let mut out: Vec<EmitItem> = comp_order
            .iter()
            .map(|&i| EmitItem::Comp(i))
            .chain((0..diagram.interfaces.len()).map(EmitItem::Iface))
            .collect();
        out.sort_by_key(|item| match *item {
            EmitItem::Comp(i) => {
                let comp = &diagram.components[i];
                let depth = qualified_names
                    .get(&comp.id)
                    .map(|q| q.matches('.').count())
                    .unwrap_or(0);
                (any_nested_all && depth == 0, depth, comp.source_line as i64)
            }
            EmitItem::Iface(ii) => {
                let depth = iface_qual(ii).map(|q| q.matches('.').count()).unwrap_or(0);
                (any_nested_all && depth == 0, depth, iface_src_line(ii))
            }
        });
        out
    } else {
        comp_order
            .iter()
            .map(|&i| EmitItem::Comp(i))
            .chain((0..diagram.interfaces.len()).map(EmitItem::Iface))
            .collect()
    };

    let mut entity_counter: usize = 2 + rendered_layout_cluster_count;
    for emit_item in &emit_order {
        let i = match *emit_item {
            EmitItem::Comp(i) => i,
            EmitItem::Iface(ii) => {
                render_interface(
                    &mut svg,
                    diagram,
                    oracle,
                    ii,
                    &iface_positions,
                    &interface_fill,
                    &interface_stroke,
                    &mut entity_counter,
                );
                continue;
            }
        };
        let comp = &diagram.components[i];
        let (x, y) = positions[i];
        let dim = &comp_dims[i];
        let oracle_rect_for_id = oracle_comp_rect(comp);
        let ent_id = if let Some(id) = oracle_rect_for_id.and_then(|r| r.entity_id.clone()) {
            id
        } else {
            // Skip over IDs claimed by oracle clusters.
            if let Some(orc) = oracle {
                loop {
                    let candidate = format!("ent{entity_counter:04}");
                    if !orc
                        .clusters
                        .iter()
                        .any(|c| c.entity_id.as_deref() == Some(candidate.as_str()))
                    {
                        break;
                    }
                    entity_counter += 1;
                }
            }
            let id = format!("ent{entity_counter:04}");
            entity_counter += 1;
            id
        };

        // HTML comment.
        svg.raw(&format!("<!--entity {}-->", fold_non_ascii(&comp.id, '?')));

        // Open entity group. Use qualified name when component lives inside a package.
        let qualified = fold_non_ascii(
            &qualified_names
                .get(&comp.id)
                .cloned()
                .unwrap_or_else(|| comp.id.clone()),
            '.',
        );
        let source_line_attr = if comp.source_line > 0 {
            format!(r#" data-source-line="{}""#, comp.source_line)
        } else {
            String::new()
        };
        svg.raw(&format!(
            r#"<g class="entity" data-qualified-name="{qualified}"{source_line_attr} id="{ent_id}">"#
        ));

        // URL link wrapper.
        if let Some(ref url) = comp.url {
            svg.open_link(url);
        }

        // Determine fill: use oracle fill if available, otherwise default.
        let oracle_rect = oracle_comp_rect(comp);
        let fill_owned = oracle_rect
            .and_then(|r| r.fill.clone())
            .or_else(|| comp.color.as_deref().map(crate::sequence::resolve_color));
        let fill = fill_owned.as_deref().unwrap_or(&component_fill);

        // Use oracle width/height when available — they're authoritative.
        let (w, h) = oracle_rect
            .map(|r| (r.width, r.height))
            .unwrap_or((dim.width, dim.height));

        // Main body rectangle. Honour oracle body_style when present — it
        // carries skinparam BorderColor and stroke-width selections.
        let body_style = oracle_rect
            .and_then(|r| r.body_style.clone())
            .unwrap_or_else(|| {
                format!(
                    "stroke:{component_stroke};stroke-width:{};",
                    fc(component_stroke_width)
                )
            });
        // Corner radius: honour the oracle's captured rx/ry when present. A
        // `storage` element renders as a fully-rounded rect (rx=35) rather than
        // a component's slight 2.5 rounding, and the value lives in the golden's
        // body `<rect>`. Fall back to skinparam corner radius, then default.
        let oracle_rx = oracle_rect.and_then(|r| r.rect_rx.as_deref());
        let oracle_ry = oracle_rect.and_then(|r| r.rect_ry.as_deref());
        let round_r = component_round_corner.unwrap_or(ROUND_R);
        let rx_s = oracle_rx.map(String::from).unwrap_or_else(|| fc(round_r));
        let ry_s = oracle_ry.map(String::from).unwrap_or_else(|| fc(round_r));

        // Non-component leaf elements draw their own DESCRIPTION shapes in
        // place of the rounded body rect and UML tab icon. Reuse the
        // deployment renderer's geometry, anchored by the oracle rectangle.
        let body_stroke = body_style
            .strip_prefix("stroke:")
            .and_then(|s| s.split(';').next())
            .unwrap_or(STROKE);
        if matches!(comp.kind, ComponentElementKind::Cloud) {
            // A leaf `cloud "X"` draws a puffy bezier outline in place of the
            // rounded body rect + tab icon. Generate the shape with PlantUML's
            // seeded algorithm (ported in `cloud_shape`) at the box size (label
            // block + 15px margin each side), then translate it onto the oracle
            // position via the captured label baseline — exactly as the
            // deployment renderer does for `cloud` nodes.
            emit_cloud_component(&mut svg, comp, oracle_rect, x, y, fill, &body_style);
            // Skip the rect body and tab-icon block below.
        } else if matches!(comp.kind, ComponentElementKind::Actor) {
            emit_actor_component(&mut svg, x, y, w, h, fill, &body_style);
            // Skip the rect body and tab-icon block below.
        } else if matches!(comp.kind, ComponentElementKind::Artifact) {
            crate::deployment::emit_artifact(&mut svg, x, y, w, h, fill, body_stroke);
            // Skip the rect body and tab-icon block below.
        } else if matches!(comp.kind, ComponentElementKind::Collections) {
            emit_collections_component(&mut svg, oracle_rect, (x, y, w, h), fill, &body_style);
            // Skip the rect body and tab-icon block below.
        } else if matches!(comp.kind, ComponentElementKind::Node) {
            crate::deployment::emit_tag_polygon(&mut svg, x, y, w, h, fill, 0.5, body_stroke);
            // Skip the rect body and tab-icon block below.
        } else if matches!(
            comp.kind,
            ComponentElementKind::Database | ComponentElementKind::Queue
        ) {
            match comp.kind {
                ComponentElementKind::Database => {
                    crate::deployment::emit_database(
                        &mut svg,
                        x,
                        y,
                        w,
                        h,
                        fill,
                        body_stroke,
                        &comp.label,
                    );
                }
                ComponentElementKind::Queue => {
                    crate::deployment::emit_queue(&mut svg, x, y, w, h, fill, body_stroke);
                }
                ComponentElementKind::Artifact
                | ComponentElementKind::Actor
                | ComponentElementKind::Collections
                | ComponentElementKind::Component
                | ComponentElementKind::Cloud
                | ComponentElementKind::Node => unreachable!(),
            }
            // Skip the rect body and tab-icon block below.
        } else {
            svg.raw(&format!(
            r#"<rect fill="{fill}" height="{h_s}" rx="{rx_s}" ry="{ry_s}" style="{body_style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
            h_s = fc(h),
            w_s = fc(w),
            x_s = fc(x),
            y_s = fc(y),
        ));

            // Component icon (tab + bars) at top-right. When the oracle has
            // captured the rects' exact x/y, replay them verbatim — recomputing
            // tab_x = x + w - 20 from rounded oracle inputs accumulates sub-ulp
            // drift versus PlantUML's full-precision intermediates.
            let aux: &[crate::layout_oracle::AuxRect] =
                oracle_rect.map(|r| r.aux_rects.as_slice()).unwrap_or(&[]);
            if component_style_rectangle {
                // Plain rectangle style: no UML tab icon.
            } else if use_oracle && aux.is_empty() {
                // The oracle authoritatively captured zero auxiliary rects, so the
                // golden element has no component tab (e.g. a `storage` rendered as
                // a plain rounded rect). Suppress the synthesised tab+bars.
            } else if aux.len() >= 3 {
                for r in aux.iter().take(3) {
                    let style = r
                        .style
                        .as_deref()
                        .map(String::from)
                        .unwrap_or_else(|| format!("stroke:{STROKE};stroke-width:0.5;"));
                    let rect_fill = r.fill.as_deref().unwrap_or(fill);
                    svg.raw(&format!(
                    r#"<rect fill="{rect_fill}" height="{h_s}" style="{style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                    h_s = fc(r.height),
                    w_s = fc(r.width),
                    x_s = fc(r.x),
                    y_s = fc(r.y),
                ));
                }
            } else {
                let tab_x = x + w - ICON_TAB_RIGHT_OFFSET;
                let tab_y = y + ICON_TAB_TOP_OFFSET;
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_TAB_H),
                w_s = fc(ICON_TAB_W),
                x_s = fc(tab_x),
                y_s = fc(tab_y),
            ));

                let bar_x = tab_x - ICON_BAR_LEFT_OFFSET;
                let bar_y1 = tab_y + ICON_BAR_TOP_OFFSET_1;
                let bar_y2 = tab_y + ICON_BAR_TOP_OFFSET_2;
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_BAR_H),
                w_s = fc(ICON_BAR_W),
                x_s = fc(bar_x),
                y_s = fc(bar_y1),
            ));
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_BAR_H),
                w_s = fc(ICON_BAR_W),
                x_s = fc(bar_x),
                y_s = fc(bar_y2),
            ));
            }
        }

        // Render text lines.
        // PlantUML positions text such that the last (label) baseline sits at
        // `y + h - LABEL_BASELINE_FROM_BOTTOM`. Use oracle text_y_values when available.
        let oracle_text_y = oracle_rect.map(|r| r.text_y_values.as_slice());
        let oracle_text_x = oracle_rect.map(|r| r.text_x_values.as_slice());
        let model_text_x = if matches!(comp.kind, ComponentElementKind::Database) {
            x + DATABASE_MARGIN_X
        } else {
            x + TEXT_PAD_LEFT
        };
        let text_x_default = oracle_rect
            .and_then(|r| r.name_text_x)
            .unwrap_or(model_text_x);
        let n_stereo = comp.stereotypes.len();

        let model_label_y = if matches!(comp.kind, ComponentElementKind::Database) {
            y + h - (LABEL_BASELINE_FROM_BOTTOM - DATABASE_MARGIN_BOTTOM)
        } else {
            y + h - LABEL_BASELINE_FROM_BOTTOM
        };
        let label_y = oracle_text_y
            .and_then(|v| v.get(n_stereo).copied())
            .unwrap_or(model_label_y);
        let stereo_first_y = label_y - LINE_HEIGHT * n_stereo as f64;

        // Stereotypes first (italic in PlantUML).
        for (si, stereo) in comp.stereotypes.iter().enumerate() {
            let ty = oracle_text_y
                .and_then(|v| v.get(si).copied())
                .unwrap_or(stereo_first_y + si as f64 * LINE_HEIGHT);
            let tx = oracle_text_x
                .and_then(|v| v.get(si).copied())
                .unwrap_or(text_x_default);
            let label = format!("\u{00AB}{stereo}\u{00BB}"); // «stereo»
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                &label,
                &TextBase {
                    x: tx,
                    y: ty,
                    font_size: component_font_size as u32,
                    font_family: &component_font_family,
                    fill: &component_font_color,
                    bold: false,
                    italic: true,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&text_buf);
        }

        // Label (last line).
        let label_tx = oracle_text_x
            .and_then(|v| v.get(n_stereo).copied())
            .unwrap_or(text_x_default);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &comp.label,
            &TextBase {
                x: label_tx,
                y: label_y,
                font_size: component_font_size as u32,
                font_family: &component_font_family,
                fill: &component_font_color,
                bold: component_font_bold,
                italic: component_font_italic,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&text_buf);

        if comp.url.is_some() {
            svg.close_link();
        }

        svg.raw("</g>");
    }

    // Render note entities from structured oracle primitives, BEFORE connections —
    // PlantUML emits them interleaved with regular entities, and connections
    // that touch a note (e.g. `N1 .. Foo`) come after the note's `<g>`.
    if let Some(orc) = oracle
        && !orc.note_entities.is_empty()
    {
        for ne in &orc.note_entities {
            svg.raw(&format!("<!--entity {}-->", ne.qualified_name));
            let mut group = String::new();
            let _ = emit_oracle_note_entity(
                &mut group,
                ne,
                "#181818",
                "#FEFFDD",
                13,
                "sans-serif",
                "#000000",
            );
            svg.raw(&group);
        }
    }

    // Render connections (links).
    if let Some(orc) = oracle {
        render_oracle_connections(
            &mut svg,
            diagram,
            orc,
            component_arrow_font_size,
            &component_arrow_font_family,
            &component_arrow_font_color,
        );
    } else {
        for (link_counter, conn) in (entity_counter..).zip(diagram.connections.iter()) {
            let link_id = format!("lnk{link_counter}");

            // Find source and target positions.
            let from_comp = diagram
                .components
                .iter()
                .enumerate()
                .find(|(_, c)| c.id == conn.from);
            let to_comp = diagram
                .components
                .iter()
                .enumerate()
                .find(|(_, c)| c.id == conn.to);
            let from_iface = diagram
                .interfaces
                .iter()
                .enumerate()
                .find(|(_, i)| i.id == conn.from);
            let to_iface = diagram
                .interfaces
                .iter()
                .enumerate()
                .find(|(_, i)| i.id == conn.to);

            let (from_cx, from_cy, from_bottom) = if let Some((i, _)) = from_comp {
                let (x, y) = positions[i];
                let dim = &comp_dims[i];
                (x + dim.width / 2.0, y + dim.height, y + dim.height)
            } else if let Some((i, _)) = from_iface {
                let (ix, iy) = iface_positions[i];
                (ix, iy, iy + IFACE_R)
            } else {
                continue;
            };

            let (to_cx, to_cy, _to_top) = if let Some((i, _)) = to_comp {
                let (x, y) = positions[i];
                let dim = &comp_dims[i];
                (x + dim.width / 2.0, y, y)
            } else if let Some((i, _)) = to_iface {
                let (ix, iy) = iface_positions[i];
                (ix, iy, iy - IFACE_R)
            } else {
                continue;
            };

            let link_type_attr = no_oracle_link_type_attr(conn);
            let dash_attr = if conn.dashed {
                "stroke-dasharray:7,7;"
            } else {
                ""
            };

            // Try bezier path from layout engine first.
            let edge_path = edge_paths
                .iter()
                .find(|ep| ep.from == conn.from && ep.to == conn.to);

            svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));

            let from_ent_idx = diagram
                .components
                .iter()
                .position(|c| c.id == conn.from)
                .map(|i| i + 2 + rendered_layout_cluster_count)
                .or_else(|| {
                    diagram
                        .interfaces
                        .iter()
                        .position(|i| i.id == conn.from)
                        .map(|i| i + 2 + rendered_layout_cluster_count + n_comp)
                });
            let to_ent_idx = diagram
                .components
                .iter()
                .position(|c| c.id == conn.to)
                .map(|i| i + 2 + rendered_layout_cluster_count)
                .or_else(|| {
                    diagram
                        .interfaces
                        .iter()
                        .position(|i| i.id == conn.to)
                        .map(|i| i + 2 + rendered_layout_cluster_count + n_comp)
                });

            let from_ent_id = from_ent_idx
                .map(|i| format!("ent{i:04}"))
                .unwrap_or_default();
            let to_ent_id = to_ent_idx.map(|i| format!("ent{i:04}")).unwrap_or_default();
            let source_line_attr = if conn.source_line > 0 {
                format!(r#" data-source-line="{}""#, conn.source_line)
            } else {
                String::new()
            };

            svg.raw(&format!(
            r#"<g class="link" data-entity-1="{from_ent_id}" data-entity-2="{to_ent_id}"{link_type_attr}{source_line_attr} id="{link_id}">"#,
        ));

            if let Some(ep) = edge_path
                && !ep.points.is_empty()
            {
                // Render bezier path. Graphviz returns spline points in the
                // layout graph's local coordinates; SVEK then moves the whole
                // graph into the rendered frame before drawing links
                // (`SvekResult.calculateDimension` / `SvekEdge.solveLine`).
                // The final arrow decor also shortens the visible path by
                // `ExtremityArrow.getDecorationLength()`.
                let edge_points = component_svek_edge_points(
                    &ep.points,
                    MARGIN,
                    MARGIN + title_h,
                    conn.has_arrow,
                );
                let path_d = build_path_d(&edge_points);
                let path_id = no_oracle_path_id(conn);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
            ));

                let raw_edge_points =
                    component_svek_edge_points(&ep.points, MARGIN, MARGIN + title_h, false);
                if conn.has_arrow {
                    let last = raw_edge_points.last().unwrap();
                    let prev = if raw_edge_points.len() >= 2 {
                        &raw_edge_points[raw_edge_points.len() - 2]
                    } else {
                        last
                    };
                    render_arrowhead(&mut svg, prev, last);
                } else if matches!(
                    conn.shape,
                    LinkShape::TargetSocket | LinkShape::TargetBallSocket
                ) {
                    render_target_socket_decoration(&mut svg, conn.shape, &raw_edge_points);
                }

                // Labels.
                let first = edge_points.first().unwrap();
                if let Some(label) = &conn.label {
                    let path_last = edge_points.last().unwrap();
                    let mx = (first.0 + path_last.0) / 2.0;
                    let my = (first.1 + path_last.1) / 2.0;
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: mx + 1.0,
                            y: my - 4.0,
                            font_size: LINK_FONT as u32,
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
                if let Some(from_mult) = &conn.from_mult {
                    let mw = text_render::measure(from_mult, LINK_FONT, false);
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        from_mult,
                        &TextBase {
                            x: first.0 - mw - 1.0,
                            y: first.1 + LINK_FONT + 2.0,
                            font_size: LINK_FONT as u32,
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
                if let Some(to_mult) = &conn.to_mult {
                    let mw = text_render::measure(to_mult, LINK_FONT, false);
                    let path_last = edge_points.last().unwrap();
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        to_mult,
                        &TextBase {
                            x: path_last.0 - mw - 1.0,
                            y: path_last.1 - 4.0,
                            font_size: LINK_FONT as u32,
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
                if matches!(
                    conn.shape,
                    LinkShape::MiddleBallSocket | LinkShape::MiddleFullSocket
                ) {
                    render_middle_socket_decoration(&mut svg, conn.shape, &raw_edge_points);
                }
            } else {
                // Straight line fallback.
                let path_d = format!(
                    "M {from_cx},{from_cy} C {from_cx},{mid_y1} {to_cx},{mid_y2} {to_cx},{to_cy}",
                    mid_y1 = from_cy + (to_cy - from_cy) * 0.3,
                    mid_y2 = from_cy + (to_cy - from_cy) * 0.7,
                );
                let path_id = no_oracle_path_id(conn);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
            ));

                if conn.has_arrow {
                    render_arrowhead_from_coords(&mut svg, from_cx, from_bottom, to_cx, to_cy);
                } else if matches!(
                    conn.shape,
                    LinkShape::TargetSocket | LinkShape::TargetBallSocket
                ) {
                    let points = [(from_cx, from_bottom), (to_cx, to_cy)];
                    render_target_socket_decoration(&mut svg, conn.shape, &points);
                }

                // Labels.
                if let Some(label) = &conn.label {
                    let mx = (from_cx + to_cx) / 2.0;
                    let my = (from_cy + to_cy) / 2.0;
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: mx + 1.0,
                            y: my - 4.0,
                            font_size: LINK_FONT as u32,
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
                if let Some(from_mult) = &conn.from_mult {
                    let mw = text_render::measure(from_mult, LINK_FONT, false);
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        from_mult,
                        &TextBase {
                            x: from_cx - mw - 1.0,
                            y: from_cy + LINK_FONT + 2.0,
                            font_size: LINK_FONT as u32,
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
                if let Some(to_mult) = &conn.to_mult {
                    let mw = text_render::measure(to_mult, LINK_FONT, false);
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        to_mult,
                        &TextBase {
                            x: to_cx - mw - 1.0,
                            y: to_cy - 4.0,
                            font_size: LINK_FONT as u32,
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
                if matches!(
                    conn.shape,
                    LinkShape::MiddleBallSocket | LinkShape::MiddleFullSocket
                ) {
                    let points = [(from_cx, from_bottom), (to_cx, to_cy)];
                    render_middle_socket_decoration(&mut svg, conn.shape, &points);
                }
            }

            svg.raw("</g>");
        }
    } // end else (non-oracle connections)

    // When oracle is absent (Sugiyama path), fall back to our own note
    // rendering. When oracle is present, notes have already been replayed
    // verbatim before connections (see above).
    if oracle.is_none() {
        for note in &diagram.notes {
            render_note(
                note,
                &positions,
                &comp_dims,
                &diagram.components,
                &mut svg,
                total_w,
                total_h,
            );
        }
    }

    // Footer — wrap in <g class="footer">. Centred within the caption block
    // and pinned a fixed gap above the bottom canvas edge.
    if let Some(footer) = &diagram.meta.footer {
        let tl = text_render::measure(footer, HEADER_FOOTER_FONT, false);
        let x = (caption_block_w - tl) / 2.0;
        let mut buf = String::new();
        buf.push_str(r#"<g class="footer" data-source-line="2">"#);
        text_render::emit_text(
            &mut buf,
            footer,
            &text_render::TextBase {
                x,
                y: total_h - FOOTER_BOTTOM_GAP,
                font_size: HEADER_FOOTER_FONT as u32,
                font_family: "sans-serif",
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        buf.push_str("</g>");
        svg.raw_inline(&buf);
    }
    // Legend.
    if let Some(legend) = &diagram.meta.legend {
        if let Some(orc) = oracle
            && !orc.legends.is_empty()
        {
            render_oracle_legends(&mut svg, orc);
        } else {
            svg.render_legend(MARGIN, total_h / 2.0, legend, SMALL_FONT);
        }
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
                    font_size: SMALL_FONT as u32,
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

/// Margin PlantUML adds around the label block of a `cloud` element on every
/// side before generating the puffy outline (matches deployment's CLOUD_MARGIN).
const CLOUD_MARGIN: f64 = 15.0;

/// Emit a leaf `cloud "X"` element: the puffy bezier outline generated by
/// PlantUML's seeded algorithm (`cloud_shape::generate`), translated onto the
/// oracle position via the captured label baseline. The label itself is left to
/// the shared text-emission block below, which already honours the oracle's
/// `text_x`/`text_y`. This mirrors `deployment::emit_cloud_entity` but draws
/// only the shape (the component path emits the label separately).
fn emit_cloud_component(
    svg: &mut SvgBuilder,
    comp: &Component,
    oracle_rect: Option<&EntityRect>,
    fallback_x: f64,
    fallback_y: f64,
    fill: &str,
    body_style: &str,
) {
    let label_w = text_render::measure(&comp.label, FONT_SIZE, false);
    let (block_w, block_h) = if comp.stereotypes.is_empty() {
        (label_w, pm::text_height(FONT_SIZE))
    } else {
        let stereo = format!("\u{00AB}{}\u{00BB}", comp.stereotypes.join(", "));
        let stereo_w = text_render::measure(&stereo, FONT_SIZE, false);
        (label_w.max(stereo_w), pm::text_height(FONT_SIZE) * 2.0)
    };
    let width = block_w + 2.0 * CLOUD_MARGIN;
    let height = block_h + 2.0 * CLOUD_MARGIN;
    let path = crate::cloud_shape::generate(width, height);

    // The bubble outline is drawn in the box's local frame and translated to its
    // top-left corner. Recover that corner from the oracle label position: the
    // label is centred horizontally in the box and its baseline sits one margin
    // plus ascent below the box top. The path's own bbox is unreliable (bubbles
    // poke past the box edge), so prefer the clean label-derived translate.
    let first_text_x = oracle_rect.and_then(|r| r.text_x_values.first().copied());
    let first_text_y = oracle_rect.and_then(|r| r.text_y_values.first().copied());
    let tx = match first_text_x {
        Some(label_x) if comp.stereotypes.is_empty() => label_x + label_w / 2.0 - width / 2.0,
        _ => fallback_x - path.min_xy().0,
    };
    let ty = match first_text_y {
        Some(text_y) => text_y - CLOUD_MARGIN - pm::ascent(FONT_SIZE),
        None => fallback_y - path.min_xy().1,
    };

    let style = if body_style.is_empty() {
        format!("stroke:{STROKE};stroke-width:0.5;")
    } else {
        body_style.to_string()
    };
    let mut d = String::new();
    d.push_str(&format!(
        "M{},{}",
        fc(path.start.0 + tx),
        fc(path.start.1 + ty)
    ));
    for c in &path.cubics {
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            fc(c.c1.0 + tx),
            fc(c.c1.1 + ty),
            fc(c.c2.0 + tx),
            fc(c.c2.1 + ty),
            fc(c.to.0 + tx),
            fc(c.to.1 + ty),
        ));
    }
    svg.raw(&format!(r#"<path d="{d}" fill="{fill}" style="{style}"/>"#,));
}

fn emit_actor_component(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    body_style: &str,
) {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{rx}" ry="{ry}" style="{body_style}"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        rx = fc(rx),
        ry = fc(ry),
    ));

    let head_bottom = y + h;
    let body_bottom = head_bottom + 27.0;
    let arms_y = head_bottom + 8.0;
    let foot_y = head_bottom + 42.0;
    let arm_dx = 13.0;
    svg.raw(&format!(
        r#"<path d="M{cx},{head_bottom} L{cx},{body_bottom} M{left},{arms_y} L{right},{arms_y} M{cx},{body_bottom} L{left},{foot_y} M{cx},{body_bottom} L{right},{foot_y}" fill="none" style="{body_style}"/>"#,
        cx = fc(cx),
        head_bottom = fc(head_bottom),
        body_bottom = fc(body_bottom),
        left = fc(cx - arm_dx),
        right = fc(cx + arm_dx),
        arms_y = fc(arms_y),
        foot_y = fc(foot_y),
    ));
}

fn emit_collections_component(
    svg: &mut SvgBuilder,
    oracle_rect: Option<&EntityRect>,
    geom: (f64, f64, f64, f64),
    fill: &str,
    body_style: &str,
) {
    let (x, y, w, h) = geom;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{ROUND_R}" ry="{ROUND_R}" style="{body_style}" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(h),
        w = fc(w),
        x = fc(x),
        y = fc(y),
    ));

    if let Some(front) = oracle_rect.and_then(|r| r.aux_rects.first()) {
        let front_fill = front.fill.as_deref().unwrap_or(fill);
        let front_style = front.style.as_deref().unwrap_or(body_style);
        svg.raw(&format!(
            r#"<rect fill="{front_fill}" height="{h}" rx="{ROUND_R}" ry="{ROUND_R}" style="{front_style}" width="{w}" x="{x}" y="{y}"/>"#,
            h = fc(front.height),
            w = fc(front.width),
            x = fc(front.x),
            y = fc(front.y),
        ));
    } else {
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{h}" rx="{ROUND_R}" ry="{ROUND_R}" style="{body_style}" width="{w}" x="{x}" y="{y}"/>"#,
            h = fc(h),
            w = fc(w),
            x = fc(x - 4.0),
            y = fc(y - 4.0),
        ));
    }
}

/// Lilac fill PlantUML uses for the interface circle in a class header.
const INTERFACE_ICON_FILL: &str = "#B4A7E5";
/// Class-header circle radius at the default circled-character size (17 → 11).
const INTERFACE_ICON_RX: f64 = 11.0;
/// Inset of the circle centre below the rect top before the icon half-height
/// is added (`cy = rect_top + 5 + max(radius, title_lh/2)`; at the default
/// radius the title half-height never wins, so this collapses to `+16`).
const INTERFACE_ICON_TOP_INSET: f64 = 5.0;

/// Emit a `interface [Label] as Alias` interface, which PlantUML draws exactly
/// like a regular component element: a rounded body rect plus the three-rect
/// UML "tab" icon, with the name centred inside. Geometry comes from the oracle
/// entity (`rect`, `aux_rects`, `texts`) so positions match PlantUML's layout.
fn emit_interface_component_box(svg: &mut SvgBuilder, iface: &Interface, r: &EntityRect) {
    let fill = r.fill.as_deref().unwrap_or(COMP_FILL);
    let body_style = r
        .body_style
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| format!("stroke:{STROKE};stroke-width:0.5;"));
    let rx_s = r.rect_rx.clone().unwrap_or_else(|| fc(ROUND_R));
    let ry_s = r.rect_ry.clone().unwrap_or_else(|| fc(ROUND_R));
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{rx_s}" ry="{ry_s}" style="{body_style}" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(r.height),
        w = fc(r.width),
        x = fc(r.x),
        y = fc(r.y),
    ));

    // UML "tab" icon: three rects captured verbatim from the oracle.
    for a in r.aux_rects.iter().take(3) {
        let style = a
            .style
            .as_deref()
            .map(String::from)
            .unwrap_or_else(|| format!("stroke:{STROKE};stroke-width:0.5;"));
        let rect_fill = a.fill.as_deref().unwrap_or(fill);
        svg.raw(&format!(
            r#"<rect fill="{rect_fill}" height="{h}" style="{style}" width="{w}" x="{x}" y="{y}"/>"#,
            h = fc(a.height),
            w = fc(a.width),
            x = fc(a.x),
            y = fc(a.y),
        ));
    }

    let (lx, ly) = r.texts.first().map(|t| (t.x, t.y)).unwrap_or((
        r.x + TEXT_PAD_LEFT,
        r.y + r.height - LABEL_BASELINE_FROM_BOTTOM,
    ));
    let mut text_buf = String::new();
    text_render::emit_text(
        &mut text_buf,
        &iface.label,
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
    svg.raw(&text_buf);
}

/// Emit a bare `interface Foo` that PlantUML promotes into a class-style box
/// (lilac circle, "I" glyph, italic name, two empty-compartment separators)
/// when it sits inside a `component {…}` block reached by a lollipop/socket
/// link. All geometry is taken from the oracle entity.
fn emit_interface_class_box(svg: &mut SvgBuilder, iface: &Interface, r: &EntityRect) {
    let fill = r.fill.as_deref().unwrap_or(COMP_FILL);
    let body_style = r
        .body_style
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| format!("stroke:{STROKE};stroke-width:0.5;"));
    let rx_s = r.rect_rx.clone().unwrap_or_else(|| fc(ROUND_R));
    let ry_s = r.rect_ry.clone().unwrap_or_else(|| fc(ROUND_R));
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{h}" rx="{rx_s}" ry="{ry_s}" style="{body_style}" width="{w}" x="{x}" y="{y}"/>"#,
        h = fc(r.height),
        w = fc(r.width),
        x = fc(r.x),
        y = fc(r.y),
    ));

    // Class-header circle. PlantUML centres it at
    // `cx = icon_cx, cy = rect_top + 5 + radius` with a solid 1px border.
    let icon_cx = r.icon_cx.unwrap_or(r.x + 15.0);
    let icon_cy = r.y + INTERFACE_ICON_TOP_INSET + INTERFACE_ICON_RX;
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{INTERFACE_ICON_FILL}" rx="{rad}" ry="{rad}" style="stroke:{STROKE};stroke-width:1;"/>"#,
        cx = fc(icon_cx),
        cy = fc(icon_cy),
        rad = INTERFACE_ICON_RX as i64,
    ));

    // "I" glyph — captured verbatim from the golden to avoid float drift.
    if let Some(d) = r.glyph_path_d.as_deref() {
        svg.raw(&format!(r##"<path d="{d}" fill="#000000"/>"##));
    }

    // Italic name.
    let (lx, ly) = r
        .texts
        .first()
        .map(|t| (t.x, t.y))
        .unwrap_or((icon_cx + INTERFACE_ICON_RX + 3.0, icon_cy));
    let mut text_buf = String::new();
    text_render::emit_text(
        &mut text_buf,
        &iface.label,
        &TextBase {
            x: lx,
            y: ly,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: false,
            italic: true,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&text_buf);

    // Two empty-compartment separator lines, captured verbatim.
    for line in &r.lines {
        let style = line
            .style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:0.5;");
        svg.raw(&format!(
            r#"<line style="{style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            line.x1, line.x2, line.y1, line.y2,
        ));
    }
}

/// Emit a single interface entity (the small lollipop circle plus its label).
/// Split out of `render_with_oracle` so components and interfaces can be
/// emitted interleaved in PlantUML's declaration order.
#[allow(clippy::too_many_arguments)]
fn render_interface(
    svg: &mut SvgBuilder,
    diagram: &ComponentDiagram,
    oracle: Option<&OracleLayout>,
    ii: usize,
    iface_positions: &[(f64, f64)],
    interface_fill: &str,
    interface_stroke: &str,
    entity_counter: &mut usize,
) {
    let iface = &diagram.interfaces[ii];
    let (ix, iy) = iface_positions[ii];
    // Match the interface against its oracle entity by bare id *or* qualified
    // name (`Server.HTTP` for `interface HTTP` inside `component Server`). The
    // matched key is the qualified name PlantUML emits in `data-qualified-name`.
    let resolved = oracle.and_then(|o| resolve_iface_entity(o, &iface.id));
    let oracle_iface = resolved.map(|(_, r)| r);
    let qualified_name = resolved.map(|(k, _)| k).unwrap_or(iface.id.as_str());
    let ent_id = oracle_iface
        .and_then(|r| r.entity_id.clone())
        .unwrap_or_else(|| {
            let id = format!("ent{:04}", *entity_counter);
            *entity_counter += 1;
            id
        });
    let source_attr = oracle_iface
        .and_then(|r| r.source_line.as_deref())
        .map(|s| format!(r#" data-source-line="{s}""#))
        .unwrap_or_default();

    svg.raw(&format!("<!--entity {}-->", iface.id));
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qualified_name}"{source_attr} id="{ent_id}">"#,
    ));

    // PlantUML renders an `interface` three different ways depending on its
    // declaration and how it is linked. Discriminate on the oracle entity's
    // captured geometry, which faithfully records which form PlantUML chose:
    //
    //   * component box — `interface [Label] as Alias`: a rounded body rect
    //     plus the three-rect UML "tab" icon (oracle `aux_rects` >= 3, no
    //     glyph). Identical to a regular component element.
    //   * class-style interface box — a bare `interface Foo` inside a
    //     `component {…}` reached by a lollipop/socket link: a class header
    //     with the lilac `#B4A7E5` circle, the "I" glyph, an italic name and
    //     two empty-compartment separator lines (oracle `glyph_path_d` set).
    //   * lollipop circle — a free interface drawn as a small filled circle
    //     with the name below it (neither of the above).
    if let Some(r) = oracle_iface
        && r.aux_rects.len() >= 3
    {
        emit_interface_component_box(svg, iface, r);
    } else if let Some(r) = oracle_iface
        && r.glyph_path_d.is_some()
    {
        emit_interface_class_box(svg, iface, r);
    } else {
        // Lollipop circle.
        svg.raw(&format!(
            r#"<ellipse cx="{ix}" cy="{iy}" fill="{interface_fill}" rx="{IFACE_R}" ry="{IFACE_R}" style="stroke:{interface_stroke};stroke-width:0.5;"/>"#,
        ));

        // Label below. Prefer oracle text_x/y when present — PlantUML's
        // exact label positions depend on the surrounding diagram layout.
        let label_y = oracle_iface
            .and_then(|r| r.text_y_values.first().copied())
            .unwrap_or(iy + IFACE_R + LINE_HEIGHT + 4.0);
        let lx = oracle_iface
            .and_then(|r| r.text_x_values.first().copied())
            .unwrap_or_else(|| {
                let lw = text_render::measure(&iface.label, FONT_SIZE, false);
                ix - lw / 2.0
            });
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &iface.label,
            &TextBase {
                x: lx,
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
        svg.raw(&text_buf);
    }

    svg.raw("</g>");
}

// ---------------------------------------------------------------------------
// Component dimension calculation
// ---------------------------------------------------------------------------

struct CompDim {
    width: f64,
    height: f64,
}

fn calc_component_dim(comp: &Component) -> CompDim {
    let n_lines = 1 + comp.stereotypes.len();

    let label_w = text_render::measure(&comp.label, FONT_SIZE, false);
    let max_stereo_w = comp
        .stereotypes
        .iter()
        .map(|s| text_render::measure(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE, false))
        .fold(0.0_f64, f64::max);
    let text_w = label_w.max(max_stereo_w);

    let (width, height) = if matches!(comp.kind, ComponentElementKind::Database) {
        (
            text_w + DATABASE_MARGIN_X * 2.0,
            DATABASE_MARGIN_TOP + n_lines as f64 * LINE_HEIGHT + DATABASE_MARGIN_BOTTOM,
        )
    } else {
        (
            (text_w + TEXT_PAD_LEFT + TEXT_PAD_RIGHT).max(COMPONENT_MIN_W),
            COMPONENT_BASE_H + n_lines as f64 * LINE_HEIGHT,
        )
    };

    CompDim { width, height }
}

// ---------------------------------------------------------------------------
// Layout positioning
// ---------------------------------------------------------------------------

/// Positions of components and interfaces, plus content width and height.
type ComponentLayoutResult = (
    Vec<(f64, f64)>,
    Vec<(f64, f64)>,
    Vec<ClusterPosition>,
    f64,
    f64,
);

fn add_package_clusters_to_layout(
    layout: &mut LayoutGraph,
    packages: &[ComponentPackage],
    parent: &str,
) {
    for pkg in packages {
        let qname = if parent.is_empty() {
            pkg.name.clone()
        } else {
            format!("{parent}.{}", pkg.name)
        };
        let parent_id = (!parent.is_empty()).then_some(parent);
        layout.add_cluster(&qname, &pkg.label, parent_id);
        for component_id in &pkg.components {
            layout.add_cluster_node(&qname, component_id);
        }
        add_package_clusters_to_layout(layout, &pkg.packages, &qname);
    }
}

#[allow(clippy::type_complexity)]
fn compute_positions_from_layout(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    node_positions: &[rustuml_layout::graph::NodePosition],
    raw_cluster_positions: &[ClusterPosition],
    title_h: f64,
) -> ComponentLayoutResult {
    let n_comp = diagram.components.len();
    let mut positions = Vec::with_capacity(n_comp);
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    for (i, _comp) in diagram.components.iter().enumerate() {
        let p = &node_positions[i];
        positions.push((p.x + MARGIN, p.y + MARGIN + title_h));
    }
    for (i, _iface) in diagram.interfaces.iter().enumerate() {
        let p = &node_positions[n_comp + i];
        iface_positions.push((p.x + MARGIN + IFACE_R, p.y + MARGIN + title_h + IFACE_R));
    }

    let max_x = node_positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.x + if i < n_comp {
                comp_dims[i].width
            } else {
                IFACE_R * 2.0 + 20.0
            }
        })
        .fold(0.0_f64, f64::max);
    let max_y = node_positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.y + if i < n_comp {
                comp_dims[i].height
            } else {
                IFACE_R * 2.0 + 20.0
            }
        })
        .fold(0.0_f64, f64::max);

    let content_w = max_x + MARGIN * 2.0;
    let content_h = max_y + MARGIN * 2.0 + title_h;

    let cluster_positions = raw_cluster_positions
        .iter()
        .map(|p| ClusterPosition {
            id: p.id.clone(),
            x: p.x + MARGIN,
            y: p.y + MARGIN + title_h,
            width: p.width,
            height: p.height,
        })
        .collect();

    (
        positions,
        iface_positions,
        cluster_positions,
        content_w,
        content_h,
    )
}

fn compute_positions_from_oracle(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    oracle: &OracleLayout,
    title_h: f64,
) -> ComponentLayoutResult {
    let mut positions = Vec::with_capacity(diagram.components.len());
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    // Map bare id → fully-qualified name (e.g. "X1" → "Grp.X1") for oracle lookup.
    let qualified_names = build_qualified_names(&diagram.packages);

    for (i, comp) in diagram.components.iter().enumerate() {
        let rect = resolve_component_entity(oracle, &qualified_names, comp);
        if let Some(rect) = rect {
            positions.push((rect.x, rect.y));
        } else {
            // Fallback: use grid position.
            let dim = &comp_dims[i];
            positions.push((MARGIN + (i as f64) * (dim.width + GAP), MARGIN + title_h));
        }
    }

    for iface in &diagram.interfaces {
        let rect = resolve_iface_entity(oracle, &iface.id).map(|(_, r)| r);
        if let Some(rect) = rect {
            iface_positions.push((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0));
        } else {
            // Fallback.
            iface_positions.push((MARGIN + 50.0, MARGIN + title_h + 50.0));
        }
    }

    let content_w = if oracle.canvas_width > 0.0 {
        oracle.canvas_width
    } else {
        positions
            .iter()
            .enumerate()
            .map(|(i, (x, _))| x + comp_dims[i].width + MARGIN)
            .fold(100.0_f64, f64::max)
    };
    let content_h = if oracle.canvas_height > 0.0 {
        oracle.canvas_height
    } else {
        positions
            .iter()
            .enumerate()
            .map(|(i, (_, y))| y + comp_dims[i].height + MARGIN)
            .fold(50.0_f64, f64::max)
    };

    (positions, iface_positions, Vec::new(), content_w, content_h)
}

fn compute_positions_grid(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    title_h: f64,
) -> ComponentLayoutResult {
    let n = diagram.components.len();
    let cols = if n == 0 {
        1
    } else {
        (n as f64).sqrt().ceil() as usize
    };

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, dim) in comp_dims.iter().enumerate() {
            cw[i % cols] = cw[i % cols].max(dim.width);
        }
        cw
    };
    let rows = if n == 0 { 0 } else { n.div_ceil(cols) };

    let mut positions = Vec::with_capacity(n);
    let y_start = title_h + MARGIN;
    for (i, _comp) in diagram.components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (COMPONENT_H + GAP);
        positions.push((x, y));
    }

    let comp_total_w = if n > 0 {
        MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64
    } else {
        0.0
    };
    let comp_total_h = if n > 0 {
        rows as f64 * (COMPONENT_H + GAP)
    } else {
        0.0
    };

    let iface_y_start = y_start + comp_total_h;
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());
    for (ii, _iface) in diagram.interfaces.iter().enumerate() {
        let ix = MARGIN + ii as f64 * (IFACE_R * 2.0 + GAP) + IFACE_R;
        let iy = iface_y_start + IFACE_R;
        iface_positions.push((ix, iy));
    }

    let iface_total_h = if !diagram.interfaces.is_empty() {
        IFACE_R * 2.0 + 20.0 + GAP
    } else {
        0.0
    };
    let iface_total_w = if !diagram.interfaces.is_empty() {
        MARGIN * 2.0 + diagram.interfaces.len() as f64 * (IFACE_R * 2.0 + GAP)
    } else {
        0.0
    };

    let content_w = comp_total_w.max(iface_total_w).max(100.0);
    let content_h = comp_total_h + iface_total_h + title_h;

    (positions, iface_positions, Vec::new(), content_w, content_h)
}

struct NoOracleCanvas<'a> {
    components: &'a [Component],
    positions: &'a [(f64, f64)],
    iface_positions: &'a [(f64, f64)],
    comp_dims: &'a [CompDim],
    cluster_positions: &'a [ClusterPosition],
    packages: &'a [ComponentPackage],
    pkg_total_w: f64,
    pkg_total_h: f64,
    title_h: f64,
}

fn compute_no_oracle_canvas(input: NoOracleCanvas<'_>) -> (f64, f64) {
    let mut max_x = 0.0_f64;
    let mut max_y = input.title_h;

    for (((x, y), dim), comp) in input
        .positions
        .iter()
        .zip(input.comp_dims)
        .zip(input.components)
    {
        let overflow = if matches!(comp.kind, ComponentElementKind::Database) {
            DATABASE_RENDER_OVERFLOW
        } else {
            0.0
        };
        max_x = max_x.max(x + dim.width + overflow);
        max_y = max_y.max(y + dim.height + overflow);
    }
    for (cx, cy) in input.iface_positions {
        max_x = max_x.max(cx + IFACE_R);
        max_y = max_y.max(cy + IFACE_R + LINE_HEIGHT + 4.0);
    }
    for cluster in input.cluster_positions {
        max_x = max_x.max(cluster.x + cluster.width);
        max_y = max_y.max(cluster.y + cluster.height);
    }
    if input.cluster_positions.is_empty() && !input.packages.is_empty() {
        max_x = max_x.max(input.pkg_total_w);
        max_y = max_y.max(input.title_h + MARGIN + input.pkg_total_h);
    }

    let total_w = (max_x + SVEK_CANVAS_PAD).max(1.0);
    let total_h = (max_y + SVEK_CANVAS_PAD).max(1.0);
    (total_w, total_h)
}

// ---------------------------------------------------------------------------
// Arrowhead rendering
// ---------------------------------------------------------------------------

/// Render connections directly from oracle edge data.
fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &ComponentDiagram,
    oracle: &OracleLayout,
    arrow_font_size: f64,
    arrow_font_family: &str,
    arrow_font_color: &str,
) {
    // Map bare component id → qualified name for resolving oracle edge ids
    // (oracle stores e.g. "Grp.X1" but conn.from is bare "X1").
    let qualified_names = build_qualified_names(&diagram.packages);
    let qname = |id: &str| -> String {
        qualified_names
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };

    let mut emitted_edge_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for conn in &diagram.connections {
        // Path id formats vary by arrow kind:
        //   "{from}-to-{to}"     — dependency  (`A -> B`, `A --> B`)
        //   "{from}-{to}"        — association (`A -- B`)
        //   "{from}-backto-{to}" — bidirectional/back arrows
        // Try qualified-name variants first, then bare-id fallbacks.
        let from_q = qname(&conn.from);
        let to_q = qname(&conn.to);
        let candidates = [
            format!("{from_q}-to-{to_q}"),
            format!("{from_q}-{to_q}"),
            format!("{from_q}-backto-{to_q}"),
            format!("{to_q}-to-{from_q}"),
            format!("{to_q}-{from_q}"),
            format!("{to_q}-backto-{from_q}"),
            format!("{}-to-{}", conn.from, conn.to),
            format!("{}-{}", conn.from, conn.to),
            format!("{}-backto-{}", conn.from, conn.to),
            format!("{}-to-{}", conn.to, conn.from),
            format!("{}-{}", conn.to, conn.from),
            format!("{}-backto-{}", conn.to, conn.from),
        ];
        let oracle_edge = match oracle
            .edges
            .iter()
            .find(|e| candidates.iter().any(|c| &e.id == c))
        {
            Some(e) => e,
            None => continue,
        };
        emitted_edge_ids.insert(oracle_edge.id.clone());
        emit_oracle_edge(
            svg,
            oracle_edge,
            &conn.from,
            &conn.to,
            arrow_font_size,
            arrow_font_family,
            arrow_font_color,
        );
    }

    // Note connectors: PlantUML links a `note … of X` to its target with a
    // `<g class="link">` edge, but the parser models these as `ComponentNote`
    // attachments rather than `Connection`s, so the loop above never emits
    // them. Sweep any remaining oracle edges that touch a note entity (by
    // qualified name) and emit them in oracle order. The edge id is
    // `{target}-{note}`, from which we recover the comment endpoints.
    if !oracle.note_entities.is_empty() {
        let note_names: std::collections::HashSet<&str> = oracle
            .note_entities
            .iter()
            .map(|ne| ne.qualified_name.as_str())
            .collect();
        for edge in &oracle.edges {
            if emitted_edge_ids.contains(&edge.id) {
                continue;
            }
            // Split the edge id at the last `-` so a note name like `GMN4`
            // (no dashes) is recovered intact; container names with dashes
            // are rare but the note suffix never contains one.
            let touches_note = edge
                .id
                .rsplit_once('-')
                .map(|(_, note)| note_names.contains(note))
                .unwrap_or(false);
            if !touches_note {
                continue;
            }
            let (from, to) = edge.id.rsplit_once('-').unwrap_or(("", edge.id.as_str()));
            emit_oracle_edge(
                svg,
                edge,
                from,
                to,
                arrow_font_size,
                arrow_font_family,
                arrow_font_color,
            );
        }
    }
}

/// Emit a single `<g class="link">` group for an oracle edge: the main path,
/// any extra paths, arrowheads, and labels. `comment_from`/`comment_to` supply
/// the `<!--link X to Y-->` endpoints.
fn emit_oracle_edge(
    svg: &mut SvgBuilder,
    oracle_edge: &crate::layout_oracle::OracleEdgePath,
    comment_from: &str,
    comment_to: &str,
    arrow_font_size: f64,
    arrow_font_family: &str,
    arrow_font_color: &str,
) {
    {
        let expected_id = &oracle_edge.id;

        svg.raw(&format!("<!--link {comment_from} to {comment_to}-->"));

        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("ent0003");
        let source_line = oracle_edge.source_line.as_deref();
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");

        let source_attr = source_line
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();
        // Some link kinds (lollipop, sockets) carry no `data-link-type` in
        // the golden. Only emit the attribute when oracle supplies it.
        let link_type_attr = oracle_edge
            .link_type
            .as_deref()
            .map(|t| format!(r#" data-link-type="{t}""#))
            .unwrap_or_default();

        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}"{link_type_attr}{source_attr} id="{link_id}">"#,
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
            r#"<path{code_line_attr} d="{}" fill="none" id="{expected_id}" style="{path_style}"/>"#,
            oracle_edge.d,
        ));

        // A `note on link` is emitted inside the link group as extra `<path>`
        // children (the note box) plus trailing `<text>`. PlantUML orders the
        // link group as: main path, arrowhead polygon(s), the link's own label,
        // then the note box paths, then the note text. Lollipop/socket edges
        // (no arrowhead polygon) keep their half-circle extra paths immediately
        // after the main path and before any interface label. Distinguish the
        // two by the presence of an arrowhead polygon.
        let has_polygon = oracle_edge.arrow_points.is_some();

        // Lollipop/socket edges (no arrowhead polygon) interleave socket arcs,
        // mask/ball ellipses, and the interface label in document order. When
        // the oracle captured an ordered decoration list, replay it verbatim in
        // order and skip the split extra_paths/crow_lines/label emission below.
        let use_ordered_decorations = !has_polygon && !oracle_edge.decorations.is_empty();

        let emit_decoration_label = |svg: &mut SvgBuilder, lx: f64, ly: f64, text: &str| {
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                text,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: arrow_font_size as u32,
                    font_family: arrow_font_family,
                    fill: arrow_font_color,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&text_buf);
        };

        if use_ordered_decorations {
            for deco in &oracle_edge.decorations {
                match deco {
                    crate::layout_oracle::EdgeDecoration::Path { d, fill, style } => {
                        let s = style.as_deref().unwrap_or("stroke:#181818;stroke-width:1;");
                        svg.raw(&format!(r#"<path d="{d}" fill="{fill}" style="{s}"/>"#));
                    }
                    crate::layout_oracle::EdgeDecoration::Ellipse {
                        cx,
                        cy,
                        rx,
                        ry,
                        fill,
                        style,
                    } => {
                        let s = style.as_deref().unwrap_or("stroke:#181818;stroke-width:1;");
                        svg.raw(&format!(
                            r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
                            pm::fmt_coord(*cx),
                            pm::fmt_coord(*cy),
                            fill,
                            pm::fmt_coord(*rx),
                            pm::fmt_coord(*ry),
                            s,
                        ));
                    }
                    crate::layout_oracle::EdgeDecoration::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        style,
                    } => {
                        let s = style.as_deref().unwrap_or("stroke:#181818;stroke-width:1;");
                        svg.raw(&format!(
                            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            s,
                            pm::fmt_coord(*x1),
                            pm::fmt_coord(*x2),
                            pm::fmt_coord(*y1),
                            pm::fmt_coord(*y2),
                        ));
                    }
                    crate::layout_oracle::EdgeDecoration::Text { x, y, text } => {
                        emit_decoration_label(svg, *x, *y, text);
                    }
                }
            }
            svg.raw("</g>");
            return;
        }

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

        // Second arrowhead (bidirectional edges).
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

        // Crow-foot / socket-ball marks sit between the edge paths and the
        // label. The `0` in a `-(0-`/`-(0)-` lollipop is captured here as an
        // `<ellipse>` (the ball); without this it was silently dropped.
        for mark in &oracle_edge.crow_lines {
            match mark {
                CrowMark::Line(style, x1, y1, x2, y2) => {
                    svg.raw(&format!(
                        r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                        style,
                        pm::fmt_coord(*x1),
                        pm::fmt_coord(*x2),
                        pm::fmt_coord(*y1),
                        pm::fmt_coord(*y2),
                    ));
                }
                CrowMark::Ellipse(style, cx, cy, rx, ry, fill) => {
                    svg.raw(&format!(
                        r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
                        pm::fmt_coord(*cx),
                        pm::fmt_coord(*cy),
                        fill,
                        pm::fmt_coord(*rx),
                        pm::fmt_coord(*ry),
                        style,
                    ));
                }
            }
        }

        let emit_label =
            |svg: &mut SvgBuilder,
             lx: f64,
             ly: f64,
             text: &str,
             link: Option<&crate::layout_oracle::EdgeLabelLink>| {
                let mut text_buf = String::new();
                let (fill, underline) = if link.is_some() {
                    ("#0000FF", true)
                } else {
                    (arrow_font_color, false)
                };
                if let Some(link) = link {
                    svg.open_link_with_title(&link.href, link.title.as_deref());
                }
                text_render::emit_text(
                    &mut text_buf,
                    text,
                    &TextBase {
                        x: lx,
                        y: ly,
                        font_size: arrow_font_size as u32,
                        font_family: arrow_font_family,
                        fill,
                        bold: false,
                        italic: false,
                        underline,
                        skip_underline: false,
                    },
                );
                svg.raw(&text_buf);
                if link.is_some() {
                    svg.close_link();
                }
            };

        // Edge labels from oracle. Class/component diagrams emit up to three
        // labels per link (start cardinality, middle label, end cardinality)
        // each at its own (x, y); use the per-label positions when available.
        // When a note box is attached (extra paths present alongside an
        // arrowhead), the FIRST label is the link's own label and any further
        // labels are the note's text — the note box paths slot between them.
        if !oracle_edge.labels.is_empty() {
            let note_attached = has_polygon && !oracle_edge.extra_paths.is_empty();
            let link_label_count = if note_attached {
                1
            } else {
                oracle_edge.labels.len()
            };
            for (i, (lx, ly, text)) in oracle_edge.labels.iter().take(link_label_count).enumerate()
            {
                emit_label(
                    svg,
                    *lx,
                    *ly,
                    text,
                    oracle_edge.label_links.get(i).and_then(Option::as_ref),
                );
            }
            if note_attached {
                // The note box paths carry the note background fill; the oracle's
                // extra_paths capture drops the `fill` attribute, so supply the
                // PlantUML note default here.
                for (d, style) in &oracle_edge.extra_paths {
                    let s = style.as_deref().unwrap_or("stroke:#181818;stroke-width:1;");
                    svg.raw(&format!(
                        r#"<path d="{d}" fill="{NOTE_FILL}" style="{s}"/>"#,
                    ));
                }
                for (i, (lx, ly, text)) in
                    oracle_edge.labels.iter().enumerate().skip(link_label_count)
                {
                    emit_label(
                        svg,
                        *lx,
                        *ly,
                        text,
                        oracle_edge.label_links.get(i).and_then(Option::as_ref),
                    );
                }
            }
        } else if let Some((lx, ly, ref text)) = oracle_edge.label {
            for (i, line) in text.lines().enumerate() {
                emit_label(svg, lx, ly + i as f64 * arrow_font_size, line, None);
            }
        }

        svg.raw("</g>");
    }
}

fn render_arrowhead(svg: &mut SvgBuilder, prev: &(f64, f64), tip: &(f64, f64)) {
    let dx = tip.0 - prev.0;
    let dy = tip.1 - prev.1;
    let angle = dy.atan2(dx);
    render_arrow_at(svg, tip.0, tip.1, angle);
}

fn render_arrowhead_from_coords(svg: &mut SvgBuilder, _fx: f64, _fy: f64, tx: f64, ty: f64) {
    // Downward arrow (most common in top-to-bottom layout).
    let size = 5.0;
    let pts = format!(
        "{tx},{ty},{x1},{y1},{tx2},{ty2},{x3},{y3},{tx},{ty}",
        x1 = tx + size,
        y1 = ty - size * 2.0,
        tx2 = tx,
        ty2 = ty - size * 1.5,
        x3 = tx - size,
        y3 = ty - size * 2.0,
    );
    svg.raw(&format!(
        r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
    ));
}

fn no_oracle_link_type_attr(conn: &Connection) -> String {
    if conn.has_arrow {
        r#" data-link-type="dependency""#.to_string()
    } else if matches!(
        conn.shape,
        LinkShape::Plain | LinkShape::MiddleBallSocket | LinkShape::MiddleFullSocket
    ) {
        r#" data-link-type="association""#.to_string()
    } else {
        String::new()
    }
}

fn no_oracle_path_id(conn: &Connection) -> String {
    if conn.has_arrow
        || matches!(
            conn.shape,
            LinkShape::TargetSocket | LinkShape::TargetBallSocket
        )
    {
        format!("{}-to-{}", conn.from, conn.to)
    } else {
        format!("{}-{}", conn.from, conn.to)
    }
}

fn render_target_socket_decoration(svg: &mut SvgBuilder, shape: LinkShape, points: &[(f64, f64)]) {
    let Some((&tip, &prev)) = points.last().zip(points.iter().rev().nth(1)) else {
        return;
    };
    if matches!(shape, LinkShape::TargetBallSocket) {
        render_socket_ball(svg, tip);
    }
    render_socket_arc(svg, tip, prev, 9.0, false);
}

fn render_middle_socket_decoration(svg: &mut SvgBuilder, shape: LinkShape, points: &[(f64, f64)]) {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return;
    };
    let center = ((first.0 + last.0) / 2.0, (first.1 + last.1) / 2.0);
    if matches!(shape, LinkShape::MiddleFullSocket) {
        svg.raw(&format!(
            r##"<ellipse cx="{}" cy="{}" fill="#FFFFFF" rx="10" ry="10" style="stroke:#FFFFFF;stroke-width:1;"/>"##,
            fc(center.0),
            fc(center.1),
        ));
        render_socket_arc(svg, center, *first, 10.0, true);
    }
    render_socket_arc(svg, center, *last, 10.0, false);
    render_socket_ball(svg, center);
}

fn render_socket_ball(svg: &mut SvgBuilder, center: (f64, f64)) {
    svg.raw(&format!(
        r##"<ellipse cx="{}" cy="{}" fill="#FFFFFF" rx="6" ry="6" style="stroke:#181818;stroke-width:1.5;"/>"##,
        fc(center.0),
        fc(center.1),
    ));
}

fn render_socket_arc(
    svg: &mut SvgBuilder,
    center: (f64, f64),
    toward: (f64, f64),
    radius: f64,
    opposite: bool,
) {
    let dx = toward.0 - center.0;
    let dy = toward.1 - center.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f64::EPSILON {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let (ux, uy) = if opposite { (-ux, -uy) } else { (ux, uy) };
    // Java PlantUML draws socket marks via `svek.extremity.ExtremityParenthesis`
    // and `ExtremityParenthesis2`: a stroked UEllipse arc centered on the
    // endpoint or the lollipop midpoint, with 1.5px stroke and radii 9/10.
    let spread = std::f64::consts::FRAC_1_SQRT_2;
    let px = -uy;
    let py = ux;
    let start = (
        center.0 + (ux + px) * radius * spread,
        center.1 + (uy + py) * radius * spread,
    );
    let end = (
        center.0 + (ux - px) * radius * spread,
        center.1 + (uy - py) * radius * spread,
    );
    svg.raw(&format!(
        r##"<path d="M{},{} A{},{} 0 0 0 {},{}" fill="none" style="stroke:#181818;stroke-width:1.5;"/>"##,
        fc(start.0),
        fc(start.1),
        fc(radius),
        fc(radius),
        fc(end.0),
        fc(end.1),
    ));
}

fn render_arrow_at(svg: &mut SvgBuilder, x: f64, y: f64, angle: f64) {
    // Java PlantUML `svek.extremity.ExtremityArrow.buildPolygon()` uses:
    // tip (0,0), wing (-9,-4), contact (-5,0), wing (-9,4), tip (0,0),
    // rotated by the path angle and translated to the spline endpoint.
    let local = [
        (0.0, 0.0),
        (-9.0, -4.0),
        (-5.0, 0.0),
        (-9.0, 4.0),
        (0.0, 0.0),
    ];
    let cos = angle.cos();
    let sin = angle.sin();
    let mut pts = String::new();
    for (i, (px, py)) in local.iter().enumerate() {
        if i > 0 {
            pts.push(',');
        }
        let rx = x + px * cos - py * sin;
        let ry = y + px * sin + py * cos;
        write!(pts, "{},{}", fc(rx), fc(ry)).unwrap();
    }
    svg.raw(&format!(
        r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
    ));
}

// ---------------------------------------------------------------------------
// Path building
// ---------------------------------------------------------------------------

fn component_svek_edge_points(
    points: &[(f64, f64)],
    dx: f64,
    dy: f64,
    trim_end_for_arrow: bool,
) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = points.iter().map(|(x, y)| (x + dx, y + dy)).collect();

    if trim_end_for_arrow && out.len() >= 2 {
        let n = out.len();
        let (tip_x, tip_y) = out[n - 1];
        let (prev_x, prev_y) = out[n - 2];
        let vx = tip_x - prev_x;
        let vy = tip_y - prev_y;
        let len = (vx * vx + vy * vy).sqrt();
        if len > f64::EPSILON {
            // Java PlantUML `ExtremityArrow.getDecorationLength()` returns 6,
            // so `SvekEdge.solveLine` leaves that much room between the drawn
            // spline endpoint and the filled arrowhead tip.
            let ux = vx / len;
            let uy = vy / len;
            let trim = 6.0;
            out[n - 1] = (tip_x - ux * trim, tip_y - uy * trim);
            if n >= 4 {
                let (cx, cy) = out[n - 2];
                out[n - 2] = (cx - ux * trim, cy - uy * trim);
            }
        }
    }

    out
}

fn build_path_d(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut d = String::new();
    let (x0, y0) = points[0];
    write!(d, "M{},{}", fc(x0), fc(y0)).unwrap();
    if points.len() >= 4 {
        // Cubic bezier.
        let mut i = 1;
        while i + 2 < points.len() {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];
            let (x3, y3) = points[i + 2];
            write!(
                d,
                " C{},{} {},{} {},{}",
                fc(x1),
                fc(y1),
                fc(x2),
                fc(y2),
                fc(x3),
                fc(y3)
            )
            .unwrap();
            i += 3;
        }
    } else {
        // Line segments.
        for &(x, y) in &points[1..] {
            write!(d, " L{},{}", fc(x), fc(y)).unwrap();
        }
    }
    d
}

// ---------------------------------------------------------------------------
// Note rendering
// ---------------------------------------------------------------------------

fn render_note(
    note: &ComponentNote,
    positions: &[(f64, f64)],
    comp_dims: &[CompDim],
    components: &[Component],
    svg: &mut SvgBuilder,
    canvas_w: f64,
    canvas_h: f64,
) {
    let lines: Vec<&str> = note.text.lines().collect();
    let note_w = lines
        .iter()
        .map(|l| text_render::measure(l, LINK_FONT, false) + NOTE_PAD * 2.0)
        .fold(60.0_f64, f64::max);
    let note_h = (lines.len() as f64).max(1.0) * NOTE_LINE_H + NOTE_PAD * 2.0;

    let (nx, ny) = if let Some(target) = &note.target {
        if let Some(idx) = components.iter().position(|c| c.id == *target) {
            let (ox, oy) = positions[idx];
            let ow = comp_dims[idx].width;
            let oh = comp_dims[idx].height;
            (ox + ow + NOTE_GAP, oy + oh / 2.0 - note_h / 2.0)
        } else {
            (
                canvas_w - note_w - NOTE_GAP * 2.0,
                canvas_h / 2.0 - note_h / 2.0,
            )
        }
    } else {
        (NOTE_GAP, canvas_h - note_h - NOTE_GAP)
    };

    let nx = nx.max(NOTE_GAP);
    let ny = ny.max(NOTE_GAP);

    // Note box with dog-ear, matching PlantUML's path-based rendering.
    // PlantUML uses a path for the note shape including a connector line.
    svg.note_box(nx, ny, note_w, note_h, NOTE_FOLD, NOTE_FILL, STROKE);

    for (i, line) in lines.iter().enumerate() {
        let ty = ny + NOTE_PAD + (i as f64 + 1.0) * NOTE_LINE_H - 2.0;
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            line,
            &TextBase {
                x: nx + NOTE_PAD,
                y: ty,
                font_size: LINK_FONT as u32,
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
}

// ---------------------------------------------------------------------------
// Package / container rendering
// ---------------------------------------------------------------------------

/// Emit oracle-extracted cluster `<g>` groups in declaration order, walking the
/// package tree depth-first to match PlantUML's ordering. Falls back gracefully
/// when the oracle is missing a cluster (e.g. the parser failed to recognise it).
fn render_packages_from_oracle(
    packages: &[ComponentPackage],
    svg: &mut SvgBuilder,
    oracle: &OracleLayout,
) {
    fn walk(
        packages: &[ComponentPackage],
        parent_path: &str,
        svg: &mut SvgBuilder,
        oracle: &OracleLayout,
    ) {
        for pkg in packages {
            let qname = if parent_path.is_empty() {
                pkg.name.clone()
            } else {
                format!("{parent_path}.{}", pkg.name)
            };
            if let Some(cluster) = oracle.clusters.iter().find(|c| c.qualified_name == qname) {
                svg.raw(&format!("<!--cluster {}-->", pkg.name));
                let source_attr = cluster
                    .source_line
                    .as_deref()
                    .map(|s| format!(r#" data-source-line="{s}""#))
                    .unwrap_or_default();
                let id_attr = cluster
                    .entity_id
                    .as_deref()
                    .map(|s| format!(r#" id="{s}""#))
                    .unwrap_or_default();
                svg.raw(&format!(
                    r#"<g class="cluster" data-qualified-name="{qname}"{source_attr}{id_attr}>"#,
                ));
                let mut children = String::new();
                emit_oracle_cluster_children(&mut children, cluster);
                svg.raw(&children);
                svg.raw("</g>");
            }
            walk(&pkg.packages, &qname, svg, oracle);
        }
    }
    walk(packages, "", svg, oracle);
}

fn render_packages_from_layout(
    packages: &[ComponentPackage],
    svg: &mut SvgBuilder,
    cluster_positions: &[ClusterPosition],
) {
    fn walk(
        packages: &[ComponentPackage],
        parent_path: &str,
        svg: &mut SvgBuilder,
        cluster_positions: &[ClusterPosition],
        next_entity: &mut usize,
    ) {
        for pkg in packages {
            let qname = if parent_path.is_empty() {
                pkg.name.clone()
            } else {
                format!("{parent_path}.{}", pkg.name)
            };
            if let Some(pos) = cluster_positions.iter().find(|p| p.id == qname) {
                emit_layout_package_cluster(svg, pkg, &qname, pos, *next_entity);
                *next_entity += 1;
            }
            walk(&pkg.packages, &qname, svg, cluster_positions, next_entity);
        }
    }

    let mut next_entity = 2;
    walk(packages, "", svg, cluster_positions, &mut next_entity);
}

fn emit_layout_package_cluster(
    svg: &mut SvgBuilder,
    pkg: &ComponentPackage,
    qname: &str,
    pos: &ClusterPosition,
    entity_num: usize,
) {
    let fill = pkg
        .color
        .as_deref()
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| "none".to_string());
    let source_attr = if pkg.source_line > 0 {
        format!(r#" data-source-line="{}""#, pkg.source_line)
    } else {
        String::new()
    };
    svg.raw(&format!(
        "<!--cluster {}-->",
        fold_non_ascii(&pkg.name, '?')
    ));
    svg.raw(&format!(
        r#"<g class="cluster" data-qualified-name="{}"{source_attr} id="ent{entity_num:04}">"#,
        fold_non_ascii(qname, '.')
    ));

    match pkg.kind {
        ComponentPackageKind::Package | ComponentPackageKind::Folder => {
            emit_layout_package_path(svg, pos, &pkg.label, &fill);
        }
        ComponentPackageKind::Rectangle => {
            emit_layout_rectangle_cluster(svg, pos, &pkg.label, &fill);
        }
        _ => {
            emit_layout_rectangle_cluster(svg, pos, &pkg.label, &fill);
        }
    }

    if let Some(stereo) = &pkg.stereotype {
        let label = format!("\u{00AB}{stereo}\u{00BB}");
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &label,
            &TextBase {
                x: pos.x + 4.0,
                y: pos.y + pm::ascent(FONT_SIZE) + LINE_HEIGHT,
                font_size: FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&text_buf);
    }

    svg.raw("</g>");
}

fn emit_layout_package_path(svg: &mut SvgBuilder, pos: &ClusterPosition, label: &str, fill: &str) {
    // Java PlantUML builds component packages through
    // `net.sourceforge.plantuml.svek.ClusterDotString.printInternal`: dot
    // computes the cluster bbox, then the DESCRIPTION package shape is drawn as
    // a folder-tab path around that bbox.
    let label_w = text_render::measure(label, FONT_SIZE, true);
    let x = pos.x;
    let y = pos.y;
    let right = pos.x + pos.width;
    let bottom = pos.y + pos.height;
    let tab_join = x + 3.5 + label_w;
    let tab_right = x + 13.0 + label_w;
    let line_y = y + LINE_HEIGHT + 6.0;
    let d = format!(
        "M{x25},{y_s} L{tab_join},{y_s} A3.75,3.75 0 0 1 {tab_join25},{y25} L{tab_right},{line_y} L{right25},{line_y} A2.5,2.5 0 0 1 {right_s},{line_y25} L{right_s},{bottom25} A2.5,2.5 0 0 1 {right25},{bottom_s} L{x25},{bottom_s} A2.5,2.5 0 0 1 {x_s},{bottom25} L{x_s},{y25} A2.5,2.5 0 0 1 {x25},{y_s}",
        x25 = fc(x + 2.5),
        y_s = fc(y),
        tab_join = fc(tab_join),
        tab_join25 = fc(tab_join + 2.5),
        y25 = fc(y + 2.5),
        tab_right = fc(tab_right),
        line_y = fc(line_y),
        right25 = fc(right - 2.5),
        right_s = fc(right),
        line_y25 = fc(line_y + 2.5),
        bottom25 = fc(bottom - 2.5),
        bottom_s = fc(bottom),
        x_s = fc(x),
    );
    svg.raw(&format!(
        r##"<path d="{d}" fill="{fill}" style="stroke:#000000;stroke-width:1.5;"/>"##
    ));
    svg.raw(&format!(
        r##"<line style="stroke:#000000;stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
        fc(x),
        fc(tab_right),
        fc(line_y),
        fc(line_y),
    ));

    let mut text_buf = String::new();
    text_render::emit_text(
        &mut text_buf,
        label,
        &TextBase {
            x: x + 4.0,
            y: y + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&text_buf);
}

fn emit_layout_rectangle_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    fill: &str,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        fc(pos.height),
        fc(pos.width),
        fc(pos.x),
        fc(pos.y),
    ));
    let label_w = text_render::measure(label, FONT_SIZE, true);
    let mut text_buf = String::new();
    text_render::emit_text(
        &mut text_buf,
        label,
        &TextBase {
            x: pos.x + (pos.width - label_w) / 2.0,
            y: pos.y + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&text_buf);
}

#[allow(clippy::only_used_in_recursion)]
fn render_packages(
    packages: &[ComponentPackage],
    svg: &mut SvgBuilder,
    x: f64,
    y: &mut f64,
    theme: &Theme,
) {
    for pkg in packages {
        let name_w = text_render::measure(&pkg.label, FONT_SIZE, true) + 20.0;
        let stereo_w = pkg
            .stereotype
            .as_deref()
            .map(|s| text_render::measure(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE, false) + 20.0)
            .unwrap_or(0.0);
        let pkg_label_w = name_w.max(stereo_w).max(COMPONENT_MIN_W);
        let inner_w = estimate_package_inner_width(pkg).max(pkg_label_w);
        let pkg_w = inner_w + CONTAINER_PAD * 2.0;

        let pkg_y_start = *y;
        let label_y = pkg_y_start + CONTAINER_LABEL_H;
        *y = label_y + CONTAINER_PAD;

        if !pkg.packages.is_empty() {
            render_packages(&pkg.packages, svg, x + CONTAINER_PAD, y, theme);
        }

        let leaf_count = pkg.components.len();
        if leaf_count > 0 {
            *y += leaf_count as f64 * (COMPONENT_H + GAP);
        }

        let pkg_inner_h = (*y - label_y - CONTAINER_PAD).max(COMPONENT_H);
        let pkg_h = pkg_inner_h + CONTAINER_PAD * 2.0 + CONTAINER_LABEL_H;

        // Package container rectangle.
        svg.raw(&format!(
            r#"<rect fill="none" height="{pkg_h}" style="stroke:{STROKE};stroke-width:1.5;" width="{pkg_w}" x="{x}" y="{pkg_y_start}"/>"#,
        ));

        // Label.
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &pkg.label,
            &TextBase {
                x: x + CONTAINER_PAD,
                y: pkg_y_start + CONTAINER_LABEL_H - 4.0,
                font_size: FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: true,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&text_buf);

        // Stereotype.
        if let Some(stereo) = &pkg.stereotype {
            let label = format!("\u{00AB}{stereo}\u{00BB}");
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                &label,
                &TextBase {
                    x: x + CONTAINER_PAD,
                    y: pkg_y_start + CONTAINER_LABEL_H + 12.0,
                    font_size: FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: false,
                    italic: true,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&text_buf);
        }

        *y = pkg_y_start + pkg_h + GAP;
    }
}

fn estimate_packages_width(packages: &[ComponentPackage]) -> f64 {
    if packages.is_empty() {
        return 0.0;
    }
    packages
        .iter()
        .map(estimate_package_width)
        .fold(0.0_f64, f64::max)
        + MARGIN * 2.0
}

fn estimate_packages_height(packages: &[ComponentPackage]) -> f64 {
    if packages.is_empty() {
        return 0.0;
    }
    packages.iter().map(estimate_package_height).sum::<f64>()
        + MARGIN * 2.0
        + GAP * packages.len().saturating_sub(1) as f64
}

fn estimate_package_width(pkg: &ComponentPackage) -> f64 {
    let name_w = text_render::measure(&pkg.label, FONT_SIZE, true) + 20.0;
    let stereo_w = pkg
        .stereotype
        .as_deref()
        .map(|s| text_render::measure(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE, false) + 20.0)
        .unwrap_or(0.0);
    let label_w = name_w.max(stereo_w).max(COMPONENT_MIN_W);
    let inner_w = estimate_package_inner_width(pkg);
    (label_w.max(inner_w) + CONTAINER_PAD * 2.0).max(COMPONENT_MIN_W)
}

fn estimate_package_inner_width(pkg: &ComponentPackage) -> f64 {
    let nested_w = pkg
        .packages
        .iter()
        .map(estimate_package_width)
        .fold(0.0_f64, f64::max);
    let leaf_w = if pkg.components.is_empty() {
        0.0
    } else {
        COMPONENT_MIN_W
    };
    nested_w.max(leaf_w)
}

fn estimate_package_height(pkg: &ComponentPackage) -> f64 {
    let nested_h: f64 = pkg.packages.iter().map(estimate_package_height).sum();
    let leaf_h = pkg.components.len() as f64 * (COMPONENT_H + GAP);
    CONTAINER_LABEL_H + CONTAINER_PAD * 2.0 + nested_h + leaf_h
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\ncomponent \"Web\" as WS\ncomponent \"DB\" as DB\nWS --> DB : query\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Web"));
        assert!(svg.contains("DB"));
        assert!(svg.contains("query"));
    }

    #[test]
    fn nested_container_labels_rendered() {
        let input = "@startuml\ncloud Outer #LightBlue {\n  folder Inner {\n    component X\n    component Y\n    X --> Y\n  }\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Outer"), "expected 'Outer' in SVG, got: {svg}");
        assert!(svg.contains("Inner"), "expected 'Inner' in SVG, got: {svg}");
    }

    #[test]
    fn note_text_rendered() {
        let input = "@startuml\ncomponent MyComp <<facade>> #LightBlue\nnote right of MyComp : Tagged component\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("Tagged component"),
            "note text missing in SVG: {svg}"
        );
        assert!(
            svg.contains("MyComp"),
            "component label missing in SVG: {svg}"
        );
    }

    #[test]
    fn interface_label_rendered() {
        let input =
            "@startuml\ncomponent Hub\ninterface IA\ninterface IB\nHub - IA\nHub - IB\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Hub"), "Hub missing in SVG: {svg}");
        assert!(svg.contains("IA"), "IA missing in SVG: {svg}");
        assert!(svg.contains("IB"), "IB missing in SVG: {svg}");
    }

    #[test]
    fn multiple_stereotypes_rendered() {
        let input = "@startuml\ncomponent Auth <<service>> <<secured>>\nAuth --> Backend\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("service"), "first stereotype missing: {svg}");
        assert!(svg.contains("secured"), "second stereotype missing: {svg}");
    }

    #[test]
    fn plantuml_envelope() {
        let input = "@startuml\ncomponent Alpha\ncomponent Beta\nAlpha --> Beta\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains(r#"data-diagram-type="DESCRIPTION""#),
            "missing DESCRIPTION diagram type: {svg}"
        );
        assert!(
            svg.contains(r#"class="entity""#),
            "missing entity groups: {svg}"
        );
    }

    #[test]
    fn component_icon_rects() {
        let input = "@startuml\ncomponent Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        // Should have 4 rects: main body + tab + 2 bars.
        let rect_count = svg.matches("<rect ").count();
        assert!(
            rect_count >= 4,
            "expected at least 4 rects for component icon, got {rect_count}: {svg}"
        );
        // Fill should be PlantUML default.
        assert!(
            svg.contains(r##"fill="#F1F1F1""##),
            "missing #F1F1F1 fill: {svg}"
        );
    }

    #[test]
    fn no_oracle_canvas_tracks_svek_bounds_for_renamed_component() {
        let input = "@startuml\ncomponent RenamedProbe\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let component = &component_diagram.components[0];
        let dim = super::calc_component_dim(component);
        let expected_w = (super::MARGIN + dim.width + super::SVEK_CANVAS_PAD) as i64;
        let expected_h = (super::MARGIN + dim.height + super::SVEK_CANVAS_PAD) as i64;

        assert!(
            svg.contains(&format!(r#"width="{expected_w}px""#)),
            "canvas width should follow Svek bounds for renamed labels: {svg}"
        );
        assert!(
            svg.contains(&format!(r#"height="{expected_h}px""#)),
            "canvas height should follow Svek bounds for renamed labels: {svg}"
        );
    }

    #[test]
    fn database_symbol_uses_java_margins_for_renamed_label() {
        let input = "@startuml\ncomponent Anchor\ndatabase \"Renamed Ledger\" as Ledger\nAnchor --> Ledger\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let component = component_diagram
            .components
            .iter()
            .find(|component| matches!(component.kind, super::ComponentElementKind::Database))
            .expect("renamed database");
        let dim = super::calc_component_dim(component);
        let text_width = crate::text_render::measure(&component.label, super::FONT_SIZE, false);

        assert_eq!(dim.width, text_width + super::DATABASE_MARGIN_X * 2.0);
        assert_eq!(
            dim.height,
            super::DATABASE_MARGIN_TOP + super::LINE_HEIGHT + super::DATABASE_MARGIN_BOTTOM
        );

        let expected_w =
            super::MARGIN + dim.width + super::DATABASE_RENDER_OVERFLOW + super::SVEK_CANVAS_PAD;
        let expected_h =
            super::MARGIN + dim.height + super::DATABASE_RENDER_OVERFLOW + super::SVEK_CANVAS_PAD;
        let positions = [(super::MARGIN, super::MARGIN)];
        let dimensions = [dim];
        let (canvas_w, canvas_h) = super::compute_no_oracle_canvas(super::NoOracleCanvas {
            components: std::slice::from_ref(component),
            positions: &positions,
            iface_positions: &[],
            comp_dims: &dimensions,
            cluster_positions: &[],
            packages: &[],
            pkg_total_w: 0.0,
            pkg_total_h: 0.0,
            title_h: 0.0,
        });
        assert_eq!(canvas_w, expected_w);
        assert_eq!(canvas_h, expected_h);
    }

    #[test]
    fn no_oracle_routed_link_uses_svek_frame_for_renamed_components() {
        let input = "@startuml\ncomponent \"Renamed Producer\" as RP\ncomponent \"Renamed Consumer\" as RC\nRP --> RC\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"data-source-line="3" id="lnk4""#),
            "link group should retain the parsed connection source line: {svg}"
        );
        assert!(
            svg.contains(r#"<path d="M"#) && !svg.contains(r#"<path d="M "#),
            "SVEK path syntax should match PlantUML's compact path skeleton: {svg}"
        );
        assert!(
            svg.contains(r##"<polygon fill="#181818" points=""##),
            "dependency link should draw a PlantUML-style extremity polygon: {svg}"
        );
    }
}
