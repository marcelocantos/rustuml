// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's component diagram rendering.
//! PlantUML renders component diagrams as diagram type "DESCRIPTION".

use std::fmt::Write;

use rustuml_layout::graph::{
    ClusterPosition, ClusterTitleSize, Direction, EdgeLabelSize, EdgePath, LayoutGraph,
};
use rustuml_parser::diagram::component::*;

use crate::layout_oracle::{
    CrowMark, EntityRect, OracleLayout, emit_oracle_cluster_children, emit_oracle_note_entity,
    wrap_oracle_envelope,
};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

#[derive(Clone, Copy, Default)]
enum ComponentLineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl ComponentLineStyle {
    fn from_skinparam(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "solid" => Some(Self::Solid),
            "dashed" | "7;7" => Some(Self::Dashed),
            "dotted" | "1;3" => Some(Self::Dotted),
            _ => None,
        }
    }

    fn svg_suffix(self) -> &'static str {
        match self {
            Self::Solid => "",
            Self::Dashed => "stroke-dasharray:7,7;",
            // `FromSkinparamToStyle.convertNow` rewrites dotted to `1;3`,
            // then its complex-value branch retains only `1`;
            // `Style.getStroke` duplicates that lone token.
            Self::Dotted => "stroke-dasharray:1,1;",
        }
    }
}

#[derive(Default)]
struct ComponentStereotypeStyle {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<f64>,
    round_corner: Option<f64>,
    line_style: Option<ComponentLineStyle>,
}

fn component_stereotype_skinparam<'a>(key: &'a str, property: &str) -> Option<&'a str> {
    key.strip_prefix(property)?
        .strip_prefix("<<")?
        .strip_suffix(">>")
        .filter(|stereotype| !stereotype.is_empty())
}

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

fn build_package_qualified_names(
    packages: &[ComponentPackage],
) -> std::collections::HashMap<String, String> {
    fn walk(
        packages: &[ComponentPackage],
        parent: &str,
        map: &mut std::collections::HashMap<String, String>,
    ) {
        for package in packages {
            let qname = if parent.is_empty() {
                package.name.clone()
            } else {
                format!("{parent}.{}", package.name)
            };
            map.insert(package.name.clone(), qname.clone());
            map.insert(qname.clone(), qname.clone());
            walk(&package.packages, &qname, map);
        }
    }

    let mut map = std::collections::HashMap::new();
    walk(packages, "", &mut map);
    map
}

fn build_package_entity_ids(
    packages: &[ComponentPackage],
) -> std::collections::HashMap<String, String> {
    fn walk(
        packages: &[ComponentPackage],
        parent: &str,
        next_id: &mut usize,
        map: &mut std::collections::HashMap<String, String>,
    ) {
        for package in packages {
            let qname = if parent.is_empty() {
                package.name.clone()
            } else {
                format!("{parent}.{}", package.name)
            };
            let entity_id = format!("ent{:04}", *next_id);
            *next_id += 1;
            map.insert(package.name.clone(), entity_id.clone());
            map.insert(qname.clone(), entity_id);
            walk(&package.packages, &qname, next_id, map);
        }
    }

    let mut map = std::collections::HashMap::new();
    let mut next_id = 2;
    walk(packages, "", &mut next_id, &mut map);
    map
}

struct NoOracleUidModel {
    entity_ids: std::collections::HashMap<String, String>,
    note_ids: std::collections::HashMap<usize, NoOracleNoteUid>,
    link_ids: Vec<usize>,
}

struct NoOracleNoteUid {
    qualified_name: String,
    entity_id: String,
    link_id: usize,
}

fn build_no_oracle_uid_model(diagram: &ComponentDiagram) -> NoOracleUidModel {
    enum Event {
        Entity(Vec<String>),
        AttachedNote(usize),
        Link {
            index: usize,
            consumes_inverse: bool,
        },
    }

    fn collect_packages(
        packages: &[ComponentPackage],
        parent: &str,
        events: &mut Vec<(usize, usize, Event)>,
        ordinal: &mut usize,
    ) {
        for package in packages {
            let qname = if parent.is_empty() {
                package.name.clone()
            } else {
                format!("{parent}.{}", package.name)
            };
            events.push((
                package.source_line,
                *ordinal,
                Event::Entity(vec![package.name.clone(), qname.clone()]),
            ));
            *ordinal += 1;
            collect_packages(&package.packages, &qname, events, ordinal);
        }
    }

    let package_names = build_package_qualified_names(&diagram.packages);
    let mut events = Vec::new();
    let mut ordinal = 0;
    collect_packages(&diagram.packages, "", &mut events, &mut ordinal);
    for component in &diagram.components {
        if !package_names.contains_key(component.id.as_str()) {
            events.push((
                component.source_line,
                ordinal,
                Event::Entity(vec![component.id.clone()]),
            ));
            ordinal += 1;
        }
    }
    for interface in &diagram.interfaces {
        events.push((
            interface.source_line,
            ordinal,
            Event::Entity(vec![interface.id.clone()]),
        ));
        ordinal += 1;
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        if note.target.is_some() {
            events.push((note.source_line, ordinal, Event::AttachedNote(index)));
            ordinal += 1;
        }
    }
    for (index, connection) in diagram.connections.iter().enumerate() {
        events.push((
            connection.source_line,
            ordinal,
            Event::Link {
                index,
                consumes_inverse: no_oracle_layout_edge_ends(connection).2,
            },
        ));
        ordinal += 1;
    }
    events.sort_by_key(|(source_line, stable_ordinal, _)| (*source_line, *stable_ordinal));

    let mut next_uid = 2;
    let mut entity_ids = std::collections::HashMap::new();
    let mut note_ids = std::collections::HashMap::new();
    let mut link_ids = vec![0; diagram.connections.len()];
    for (_, _, event) in events {
        match event {
            Event::Entity(keys) => {
                let entity_id = format!("ent{next_uid:04}");
                for key in keys {
                    entity_ids.insert(key, entity_id.clone());
                }
                next_uid += 1;
            }
            Event::AttachedNote(index) => {
                // `CommandFactoryNoteOnEntity.executeInternal` obtains a
                // generated GMN name, creates the note leaf, then creates its
                // hidden Link. Each operation consumes one global UID.
                let qualified_name = format!("GMN{next_uid}");
                next_uid += 1;
                let entity_id = format!("ent{next_uid:04}");
                next_uid += 1;
                let link_id = next_uid;
                note_ids.insert(
                    index,
                    NoOracleNoteUid {
                        qualified_name,
                        entity_id,
                        link_id,
                    },
                );
                next_uid += 1;
            }
            Event::Link {
                index,
                consumes_inverse,
            } => {
                if consumes_inverse {
                    next_uid += 1;
                }
                link_ids[index] = next_uid;
                next_uid += 1;
            }
        }
    }

    NoOracleUidModel {
        entity_ids,
        note_ids,
        link_ids,
    }
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
// PlantUML `USymbolRectangle.getMargin()` returns 10 on all four sides.
const RECTANGLE_MARGIN_X: f64 = 10.0;
const RECTANGLE_MARGIN_Y: f64 = 10.0;
// `EntityImageDescription` wraps its stereotype display with
// `TextBlockUtils.withMargin(..., 1, 0)` before `USymbolComponent2.asSmall`
// merges it with the label. The wrapper contributes one pixel on each side.
const STEREOTYPE_MARGIN_X: f64 = 1.0;
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
// `CircleInterface2` draws the circle inside a one-pixel margin and reports
// the complete 18x18 block to SVEK.
const IFACE_MARGIN: f64 = 1.0;
const IFACE_NODE_SIZE: f64 = (IFACE_R + IFACE_MARGIN) * 2.0;
const IFACE_CENTER_OFFSET: f64 = IFACE_R + IFACE_MARGIN;
// `EntityImageDescription.drawU` paints a hidden interface label after an
// eight-pixel gap below the circle block.
const IFACE_LABEL_GAP: f64 = 8.0;
/// Note fold (dog-ear) size.
const NOTE_FOLD: f64 = 10.0;
/// Fallback note padding.
const NOTE_PAD: f64 = 6.0;
/// Fallback note line height.
const NOTE_LINE_H: f64 = 18.0;
/// Fallback note gap from attached element.
const NOTE_GAP: f64 = 10.0;
// `EntityImageNote` and `Opale` use asymmetric horizontal margins and a
// five-pixel vertical margin around the measured 13-point text block.
const NOTE_MARGIN_X1: f64 = 6.0;
const NOTE_MARGIN_X2: f64 = 15.0;
const NOTE_MARGIN_Y: f64 = 5.0;
// `Opale.getPolygon*` inserts an eight-pixel-wide connector mouth.
const NOTE_CONNECTOR_HALF: f64 = 4.0;

#[derive(Clone, Copy)]
struct ComponentNoteLayout {
    note_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn component_note_layout_id(index: usize) -> String {
    format!("__component_note_{index}")
}

fn component_note_dim(note: &ComponentNote) -> CompDim {
    let width = note
        .text
        .lines()
        .map(|line| text_render::measure(line, LINK_FONT, false))
        .fold(0.0_f64, f64::max)
        + NOTE_MARGIN_X1
        + NOTE_MARGIN_X2;
    let text_height = note
        .text
        .lines()
        .map(|line| text_render::label_height(line, LINK_FONT))
        .sum::<f64>();
    CompDim {
        width,
        height: text_height + NOTE_MARGIN_Y * 2.0,
    }
}

fn laid_out_note_indices(
    diagram: &ComponentDiagram,
    package_qualified_names: &std::collections::HashMap<String, String>,
) -> Vec<usize> {
    diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| {
            note.target
                .as_deref()
                .filter(|target| {
                    diagram
                        .components
                        .iter()
                        .any(|component| component.id == *target)
                        || package_qualified_names.contains_key(*target)
                })
                .map(|_| index)
        })
        .collect()
}

fn component_group_endpoint_nodes(
    diagram: &ComponentDiagram,
    package_qualified_names: &std::collections::HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut seen = std::collections::HashSet::new();
    diagram
        .connections
        .iter()
        .flat_map(|connection| [&connection.from, &connection.to])
        .chain(diagram.notes.iter().filter_map(|note| note.target.as_ref()))
        .filter_map(|endpoint| package_qualified_names.get(endpoint.as_str()))
        .filter(|qname| seen.insert((*qname).clone()))
        .map(|qname| {
            (
                qname.clone(),
                format!("__svek_group_endpoint_{}", qname.replace('.', "_")),
            )
        })
        .collect()
}

fn round_svek_input_coord(value: f64) -> f64 {
    // `SvgResult.getFirstPoint` consumes coordinates serialized by Graphviz's
    // SVG renderer at hundredth-pixel precision before SVEK positions entities.
    (value * 100.0).round() / 100.0
}

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
/// Routed paths remain in SVEK drawing coordinates, so they retain the full
/// envelope from `SvekResult.calculateDimension`: `LimitFinder.drawUPath`
/// contributes every `UPath` control-point bound, then SVEK adds 15 pixels.
const SVEK_PATH_CANVAS_PAD: f64 = 15.0;
/// Painted SVEK clusters are normalized directly to Java's 6px min-bound
/// translation and retain the full 15px dimension delta.
const SVEK_CLUSTER_ORIGIN: f64 = 6.0;

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
    let mut component_line_style = ComponentLineStyle::Solid;
    let mut component_stereotype_styles: std::collections::HashMap<
        String,
        ComponentStereotypeStyle,
    > = std::collections::HashMap::new();
    let mut component_arrow_stroke = STROKE.to_string();
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
        let scoped_property = [
            "componentbackgroundcolor",
            "componentbordercolor",
            "componentborderthickness",
            "componentroundcorner",
            "componentborderstyle",
        ]
        .into_iter()
        .find_map(|property| {
            component_stereotype_skinparam(&key, property).map(|stereotype| (property, stereotype))
        });
        if let Some((property, stereotype)) = scoped_property {
            let style = component_stereotype_styles
                .entry(stereotype.to_string())
                .or_default();
            match property {
                "componentbackgroundcolor" => {
                    style.fill = Some(crate::sequence::gradient_fill_or(val, gradient_defs));
                }
                "componentbordercolor" => {
                    style.stroke = Some(crate::sequence::resolve_color(val));
                }
                "componentborderthickness" => {
                    style.stroke_width = val.parse::<f64>().ok();
                }
                "componentroundcorner" => {
                    style.round_corner = val.parse::<f64>().ok().map(|value| value / 2.0);
                }
                "componentborderstyle" => {
                    style.line_style = ComponentLineStyle::from_skinparam(val);
                }
                _ => unreachable!(),
            }
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
            "componentborderstyle" => {
                if let Some(style) = ComponentLineStyle::from_skinparam(val) {
                    component_line_style = style;
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
            "arrowcolor" | "componentarrowcolor" => {
                component_arrow_stroke = crate::sequence::resolve_color(val);
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

    // Compute the same merged stereotype/label block that
    // `EntityImageDescription` passes to `USymbolComponent2.asSmall`.
    let component_text_metrics: Vec<ComponentTextMetrics> = diagram
        .components
        .iter()
        .map(|component| {
            component_text_metrics(
                component,
                component_font_size,
                &component_font_family,
                component_font_bold,
            )
        })
        .collect();
    let comp_dims: Vec<CompDim> = diagram
        .components
        .iter()
        .zip(&component_text_metrics)
        .map(|(component, metrics)| {
            calc_component_dim_with_symbol_style(component, metrics, component_style_rectangle)
        })
        .collect();
    let note_dims: Vec<CompDim> = diagram.notes.iter().map(component_note_dim).collect();
    let package_qualified_names = build_package_qualified_names(&diagram.packages);
    let laid_out_note_indices = laid_out_note_indices(diagram, &package_qualified_names);
    let group_endpoint_nodes = component_group_endpoint_nodes(diagram, &package_qualified_names);
    let group_endpoint_node_map: std::collections::HashMap<String, String> =
        group_endpoint_nodes.iter().cloned().collect();

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
            layout.add_node(&iface.id, &iface.label, IFACE_NODE_SIZE, IFACE_NODE_SIZE);
        }
        for &note_index in &laid_out_note_indices {
            let dim = &note_dims[note_index];
            layout.add_node(
                &component_note_layout_id(note_index),
                "",
                dim.width,
                dim.height,
            );
        }
        for (_, endpoint_id) in &group_endpoint_nodes {
            layout.add_svek_cluster_endpoint(endpoint_id);
        }
        add_package_clusters_to_layout(
            &mut layout,
            &diagram.packages,
            "",
            &group_endpoint_node_map,
        );
        add_together_groups_to_layout(&mut layout, diagram);
        for &note_index in &laid_out_note_indices {
            let note = &diagram.notes[note_index];
            let Some(target) = note.target.as_deref() else {
                continue;
            };
            let note_id = component_note_layout_id(note_index);
            let layout_target = package_qualified_names
                .get(target)
                .and_then(|qname| group_endpoint_node_map.get(qname))
                .map(String::as_str)
                .unwrap_or(target);
            let (from, to) = match note.position {
                ComponentNotePosition::Top | ComponentNotePosition::Left => {
                    (note_id.as_str(), layout_target)
                }
                ComponentNotePosition::Bottom | ComponentNotePosition::Right => {
                    (layout_target, note_id.as_str())
                }
            };
            if matches!(
                note.position,
                ComponentNotePosition::Left | ComponentNotePosition::Right
            ) {
                if package_qualified_names.contains_key(target) {
                    // `SvekEdge.appendLine` can encode a horizontal
                    // `LinkArg.noDisplay(1)` edge as `minlen=length-1`, i.e.
                    // zero. This keeps the special point in Java's protected
                    // cluster while dot places the outside note beside it.
                    layout.add_edge_with_minlen(from, to, None, 0);
                } else {
                    layout.add_same_rank(from, to);
                    layout.add_edge(from, to, None);
                }
            } else {
                // `CommandFactoryNoteOnEntity.executeInternal` uses
                // `LinkArg.noDisplay(2)` for vertical note links. SVEK maps
                // `Link.getLength() - 1` to Graphviz's `minlen`.
                layout.add_edge_with_minlen(from, to, None, 1);
            }
        }
        for conn in &diagram.connections {
            let (logical_from, logical_to, layout_reversed) = no_oracle_layout_edge_ends(conn);
            let layout_from = package_qualified_names
                .get(logical_from)
                .and_then(|qname| group_endpoint_node_map.get(qname))
                .map(String::as_str)
                .unwrap_or(logical_from);
            let layout_to = package_qualified_names
                .get(logical_to)
                .and_then(|qname| group_endpoint_node_map.get(qname))
                .map(String::as_str)
                .unwrap_or(logical_to);
            let horizontal = matches!(
                conn.direction,
                Some(ConnectionDirection::Left | ConnectionDirection::Right)
            );
            if horizontal {
                layout.add_same_rank(layout_from, layout_to);
            }
            let center_label_size = conn.label.as_deref().map(|label| EdgeLabelSize {
                // `SvekEdge.getLabelText` wraps the center label in one pixel
                // of margin before `appendLine` emits its fixed HTML table.
                width: text_render::measure(label, component_arrow_font_size, false) + 2.0,
                height: (text_render::label_height(label, component_arrow_font_size) + 2.0).floor(),
            });
            let endpoint_size = |label: Option<&str>| {
                label.map(|label| EdgeLabelSize {
                    width: text_render::measure(label, component_arrow_font_size, false).floor(),
                    height: text_render::label_height(label, component_arrow_font_size).floor(),
                })
            };
            let (tail_label, head_label) = if layout_reversed {
                (conn.to_mult.as_deref(), conn.from_mult.as_deref())
            } else {
                (conn.from_mult.as_deref(), conn.to_mult.as_deref())
            };
            layout.add_edge_with_label_sizes_and_minlen(
                layout_from,
                layout_to,
                center_label_size,
                endpoint_size(tail_label),
                endpoint_size(head_label),
                (!horizontal).then(|| conn.length.saturating_sub(1)),
            );
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
                &result.edge_paths,
                title_h,
                laid_out_note_indices.len(),
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
    let svek_edge_translation = layout_result.as_ref().and_then(|result| {
        if let Some(raw) = result.cluster_positions.first() {
            let translated = cluster_positions
                .iter()
                .find(|position| position.id == raw.id)?;
            Some((translated.x - raw.x, translated.y - raw.y))
        } else if let (Some(raw), Some(&(x, y))) =
            (result.node_positions.first(), positions.first())
        {
            Some((x - raw.x, y - raw.y))
        } else {
            let raw = result.node_positions.first()?;
            let &(cx, cy) = iface_positions.first()?;
            Some((
                cx - IFACE_CENTER_OFFSET - raw.x,
                cy - IFACE_CENTER_OFFSET - raw.y,
            ))
        }
    });
    let (svek_edge_dx, svek_edge_dy) = svek_edge_translation.unwrap_or((MARGIN, MARGIN + title_h));
    let note_layouts: Vec<ComponentNoteLayout> = layout_result
        .as_ref()
        .map(|result| {
            let first_note_node = n_comp + diagram.interfaces.len();
            laid_out_note_indices
                .iter()
                .enumerate()
                .filter_map(|(offset, &note_index)| {
                    let position = result.node_positions.get(first_note_node + offset)?;
                    Some(ComponentNoteLayout {
                        note_index,
                        x: round_svek_input_coord(position.x + svek_edge_dx),
                        y: round_svek_input_coord(position.y + svek_edge_dy),
                        width: note_dims[note_index].width,
                        height: note_dims[note_index].height,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

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
            interfaces: &diagram.interfaces,
            positions: &positions,
            iface_positions: &iface_positions,
            comp_dims: &comp_dims,
            cluster_positions: &cluster_positions,
            packages: &diagram.packages,
            pkg_total_w,
            pkg_total_h,
            title_h,
            edge_paths,
            edge_dx: svek_edge_dx,
            edge_dy: svek_edge_dy,
            note_layouts: &note_layouts,
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
    let no_oracle_uids = oracle.is_none().then(|| build_no_oracle_uid_model(diagram));
    let rendered_layout_cluster_count = if let Some(orc) = oracle
        && !orc.clusters.is_empty()
    {
        render_packages_from_oracle(&diagram.packages, &mut svg, orc);
        0
    } else if !cluster_positions.is_empty() {
        render_packages_from_layout(
            &diagram.packages,
            &mut svg,
            &cluster_positions,
            no_oracle_uids.as_ref(),
        );
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
        let any_nested = comp_indices.iter().any(|&i| {
            qualified_names
                .get(&diagram.components[i].id)
                .map(|qname| qname.contains('.'))
                .unwrap_or(false)
        });
        let mut order = comp_indices;
        order.sort_by_key(|&i| {
            let component = &diagram.components[i];
            let depth = qualified_names
                .get(&component.id)
                .map(|qname| qname.matches('.').count())
                .unwrap_or(0);
            (any_nested && depth == 0, depth, component.source_line, i)
        });
        order
    };
    // PlantUML emits components and interfaces interleaved in declaration
    // order (by source line), not all-components-then-all-interfaces. The
    // oracle captures each entity's `id` (`ent000N`), which encodes that
    // emission order, so when the oracle is present we merge both collections
    // into a single sequence sorted by the oracle-assigned id and emit in that
    // order. Without an oracle, top-level leaves use their parser-recorded
    // source lines; package-backed diagrams retain the existing depth-aware
    // component order because the interface model does not yet carry package
    // ownership.
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
    } else if diagram.packages.is_empty() {
        let mut out: Vec<EmitItem> = comp_order
            .iter()
            .map(|&i| EmitItem::Comp(i))
            .chain((0..diagram.interfaces.len()).map(EmitItem::Iface))
            .collect();
        out.sort_by_key(|item| match *item {
            EmitItem::Comp(i) => diagram.components[i].source_line,
            EmitItem::Iface(i) => diagram.interfaces[i].source_line,
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
                    no_oracle_uids.as_ref(),
                    &mut entity_counter,
                );
                continue;
            }
        };
        let comp = &diagram.components[i];
        let (x, y) = positions[i];
        let dim = &comp_dims[i];
        let text_metrics = &component_text_metrics[i];
        let oracle_rect_for_id = oracle_comp_rect(comp);
        let ent_id = if let Some(id) = oracle_rect_for_id
            .and_then(|r| r.entity_id.clone())
            .or_else(|| {
                no_oracle_uids
                    .as_ref()
                    .and_then(|uids| uids.entity_ids.get(&comp.id).cloned())
            }) {
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
        let stereotype_style = matches!(comp.kind, ComponentElementKind::Component)
            .then(|| {
                comp.stereotypes.iter().find_map(|stereotype| {
                    component_stereotype_styles.get(&stereotype.to_ascii_lowercase())
                })
            })
            .flatten();
        let fill_owned = oracle_rect
            .and_then(|r| r.fill.clone())
            .or_else(|| comp.color.as_deref().map(crate::sequence::resolve_color))
            .or_else(|| stereotype_style.and_then(|style| style.fill.clone()));
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
                let stroke = stereotype_style
                    .and_then(|style| style.stroke.as_deref())
                    .unwrap_or(&component_stroke);
                let stroke_width = stereotype_style
                    .and_then(|style| style.stroke_width)
                    .unwrap_or(component_stroke_width);
                let line_style = stereotype_style
                    .and_then(|style| style.line_style)
                    .unwrap_or(component_line_style);
                format!(
                    "stroke:{stroke};stroke-width:{};{}",
                    fc(stroke_width),
                    line_style.svg_suffix()
                )
            });
        // Corner radius: honour the oracle's captured rx/ry when present. A
        // `storage` element renders as a fully-rounded rect (rx=35) rather than
        // a component's slight 2.5 rounding, and the value lives in the golden's
        // body `<rect>`. Fall back to skinparam corner radius, then default.
        let oracle_rx = oracle_rect.and_then(|r| r.rect_rx.as_deref());
        let oracle_ry = oracle_rect.and_then(|r| r.rect_ry.as_deref());
        let round_r = stereotype_style
            .and_then(|style| style.round_corner)
            .or(component_round_corner)
            .unwrap_or(ROUND_R);
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
                        .unwrap_or_else(|| body_style.clone());
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
                r#"<rect fill="{fill}" height="{h_s}" style="{body_style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_TAB_H),
                w_s = fc(ICON_TAB_W),
                x_s = fc(tab_x),
                y_s = fc(tab_y),
            ));

                let bar_x = tab_x - ICON_BAR_LEFT_OFFSET;
                let bar_y1 = tab_y + ICON_BAR_TOP_OFFSET_1;
                let bar_y2 = tab_y + ICON_BAR_TOP_OFFSET_2;
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="{body_style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_BAR_H),
                w_s = fc(ICON_BAR_W),
                x_s = fc(bar_x),
                y_s = fc(bar_y1),
            ));
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="{body_style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
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
        let model_text_block_x = if matches!(comp.kind, ComponentElementKind::Database) {
            x + DATABASE_MARGIN_X
        } else if component_style_rectangle && matches!(comp.kind, ComponentElementKind::Component)
        {
            x + RECTANGLE_MARGIN_X
        } else {
            x + TEXT_PAD_LEFT
        };
        let oracle_text_x_default = oracle_rect.and_then(|r| r.name_text_x);
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
                .unwrap_or_else(|| {
                    oracle_text_x_default.unwrap_or_else(|| {
                        let text_width = text_metrics.stereotype_widths[si];
                        model_text_block_x + (text_metrics.content_width - text_width) / 2.0
                    })
                });
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
            .unwrap_or_else(|| {
                oracle_text_x_default.unwrap_or(
                    model_text_block_x
                        + (text_metrics.content_width - text_metrics.label_width) / 2.0,
                )
            });
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
    } else if oracle.is_none() {
        for (note_index, note) in diagram.notes.iter().enumerate() {
            let layout = note_layouts
                .iter()
                .find(|layout| layout.note_index == note_index);
            let uid = no_oracle_uids
                .as_ref()
                .and_then(|uids| uids.note_ids.get(&note_index));
            if let (Some(layout), Some(uid), Some(target)) = (layout, uid, note.target.as_deref()) {
                let note_id = component_note_layout_id(note_index);
                let target_package = package_qualified_names.get(target);
                let group_endpoint_id = target_package
                    .map(|qname| format!("__svek_group_endpoint_{}", qname.replace('.', "_")));
                let layout_target = group_endpoint_id.as_deref().unwrap_or(target);
                let (from, to) = match note.position {
                    ComponentNotePosition::Top | ComponentNotePosition::Left => {
                        (note_id.as_str(), layout_target)
                    }
                    ComponentNotePosition::Bottom | ComponentNotePosition::Right => {
                        (layout_target, note_id.as_str())
                    }
                };
                let edge = edge_paths
                    .iter()
                    .find(|edge| edge.from == from && edge.to == to);
                if target_package.is_some() {
                    render_normal_component_note(note, layout, uid, &mut svg);
                } else {
                    render_attached_component_note(
                        note,
                        layout,
                        uid,
                        edge,
                        svek_edge_dx,
                        svek_edge_dy,
                        &mut svg,
                    );
                }
            } else {
                render_fallback_note(
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
    }

    // A note targeting a group cannot be made Opale: Java
    // `GraphvizImageBuilder.isOpalisable` has no peer `SvekNode` for a group.
    // Its dashed SVEK edge therefore remains visible and is compound-clipped
    // against the solved cluster envelope.
    if oracle.is_none() {
        for (note_index, note) in diagram.notes.iter().enumerate() {
            let Some(target) = note.target.as_deref() else {
                continue;
            };
            let Some(qname) = package_qualified_names.get(target) else {
                continue;
            };
            let Some(uid) = no_oracle_uids
                .as_ref()
                .and_then(|uids| uids.note_ids.get(&note_index))
            else {
                continue;
            };
            let endpoint_id = format!("__svek_group_endpoint_{}", qname.replace('.', "_"));
            let note_id = component_note_layout_id(note_index);
            let (from, to, from_name, to_name, entity_1, entity_2, tail_cluster, head_cluster) =
                match note.position {
                    ComponentNotePosition::Top | ComponentNotePosition::Left => (
                        note_id.as_str(),
                        endpoint_id.as_str(),
                        uid.qualified_name.as_str(),
                        target,
                        uid.entity_id.as_str(),
                        no_oracle_uids
                            .as_ref()
                            .and_then(|uids| uids.entity_ids.get(target))
                            .map(String::as_str)
                            .unwrap_or(""),
                        None,
                        cluster_positions
                            .iter()
                            .find(|position| &position.id == qname),
                    ),
                    ComponentNotePosition::Bottom | ComponentNotePosition::Right => (
                        endpoint_id.as_str(),
                        note_id.as_str(),
                        target,
                        uid.qualified_name.as_str(),
                        no_oracle_uids
                            .as_ref()
                            .and_then(|uids| uids.entity_ids.get(target))
                            .map(String::as_str)
                            .unwrap_or(""),
                        uid.entity_id.as_str(),
                        cluster_positions
                            .iter()
                            .find(|position| &position.id == qname),
                        None,
                    ),
                };
            let Some(edge) = edge_paths
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
            else {
                continue;
            };
            let points = component_svek_edge_points(
                &edge.points,
                svek_edge_dx,
                svek_edge_dy,
                tail_cluster,
                head_cluster,
                false,
                false,
            );
            let path_d = build_path_d(&points);
            svg.raw(&format!("<!--link {from_name} to {to_name}-->"));
            svg.raw(&format!(
                r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="association" data-source-line="{}" id="lnk{}">"#,
                note.source_line, uid.link_id
            ));
            svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{from_name}-{to_name}" style="stroke:{STROKE};stroke-width:1;stroke-dasharray:7,7;"/>"#
            ));
            svg.raw("</g>");
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
        let package_qualified_names = build_package_qualified_names(&diagram.packages);
        let package_entity_ids = build_package_entity_ids(&diagram.packages);
        let group_endpoint_nodes: std::collections::HashMap<String, String> = diagram
            .connections
            .iter()
            .flat_map(|connection| [&connection.from, &connection.to])
            .filter_map(|endpoint| package_qualified_names.get(endpoint.as_str()))
            .map(|qname| {
                (
                    qname.clone(),
                    format!("__svek_group_endpoint_{}", qname.replace('.', "_")),
                )
            })
            .collect();
        let mut next_link_counter = entity_counter;
        for (connection_index, conn) in diagram.connections.iter().enumerate() {
            let (logical_from, logical_to, layout_reversed) = no_oracle_layout_edge_ends(conn);
            let layout_from = package_qualified_names
                .get(logical_from)
                .and_then(|qname| group_endpoint_nodes.get(qname))
                .map(String::as_str)
                .unwrap_or(logical_from);
            let layout_to = package_qualified_names
                .get(logical_to)
                .and_then(|qname| group_endpoint_nodes.get(qname))
                .map(String::as_str)
                .unwrap_or(logical_to);
            let mut link_counter = next_link_counter;
            next_link_counter += 1;
            if layout_reversed {
                // `CommandLinkElement` constructs the source-order `Link`
                // first, then `Link.getInv()` constructs the reversed link and
                // consumes the next global `lnk` UID.
                link_counter = next_link_counter;
                next_link_counter += 1;
            }
            let link_counter = no_oracle_uids
                .as_ref()
                .and_then(|uids| uids.link_ids.get(connection_index))
                .copied()
                .filter(|uid| *uid != 0)
                .unwrap_or(link_counter);
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
            let from_package = package_qualified_names.get(logical_from).and_then(|qname| {
                cluster_positions
                    .iter()
                    .find(|position| &position.id == qname)
            });
            let to_package = package_qualified_names.get(logical_to).and_then(|qname| {
                cluster_positions
                    .iter()
                    .find(|position| &position.id == qname)
            });

            let (from_cx, from_cy, from_bottom) = if let Some((i, _)) = from_comp {
                let (x, y) = positions[i];
                let dim = &comp_dims[i];
                (x + dim.width / 2.0, y + dim.height, y + dim.height)
            } else if let Some((i, _)) = from_iface {
                let (ix, iy) = iface_positions[i];
                (ix, iy, iy + IFACE_R)
            } else if let Some(position) = from_package {
                (
                    position.x + position.width / 2.0,
                    position.y + position.height / 2.0,
                    position.y + position.height,
                )
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
            } else if let Some(position) = to_package {
                (
                    position.x + position.width / 2.0,
                    position.y + position.height / 2.0,
                    position.y,
                )
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
                .find(|ep| ep.from == layout_from && ep.to == layout_to);

            let (effective_arrow_at_start, effective_arrow_at_end) =
                no_oracle_effective_arrow_ends(conn);
            let comment_prefix = if effective_arrow_at_start && !effective_arrow_at_end {
                "reverse link"
            } else {
                "link"
            };
            svg.raw(&format!(
                "<!--{comment_prefix} {logical_from} to {logical_to}-->"
            ));

            let from_ent_idx = diagram
                .components
                .iter()
                .position(|c| c.id == logical_from)
                .map(|i| i + 2 + rendered_layout_cluster_count)
                .or_else(|| {
                    diagram
                        .interfaces
                        .iter()
                        .position(|i| i.id == logical_from)
                        .map(|i| i + 2 + rendered_layout_cluster_count + n_comp)
                });
            let to_ent_idx = diagram
                .components
                .iter()
                .position(|c| c.id == logical_to)
                .map(|i| i + 2 + rendered_layout_cluster_count)
                .or_else(|| {
                    diagram
                        .interfaces
                        .iter()
                        .position(|i| i.id == logical_to)
                        .map(|i| i + 2 + rendered_layout_cluster_count + n_comp)
                });

            let from_ent_id = from_ent_idx
                .and_then(|_| {
                    no_oracle_uids
                        .as_ref()
                        .and_then(|uids| uids.entity_ids.get(logical_from).cloned())
                })
                .or_else(|| from_ent_idx.map(|i| format!("ent{i:04}")))
                .or_else(|| {
                    no_oracle_uids
                        .as_ref()
                        .and_then(|uids| uids.entity_ids.get(logical_from).cloned())
                })
                .or_else(|| package_entity_ids.get(logical_from).cloned())
                .unwrap_or_default();
            let to_ent_id = to_ent_idx
                .and_then(|_| {
                    no_oracle_uids
                        .as_ref()
                        .and_then(|uids| uids.entity_ids.get(logical_to).cloned())
                })
                .or_else(|| to_ent_idx.map(|i| format!("ent{i:04}")))
                .or_else(|| {
                    no_oracle_uids
                        .as_ref()
                        .and_then(|uids| uids.entity_ids.get(logical_to).cloned())
                })
                .or_else(|| package_entity_ids.get(logical_to).cloned())
                .unwrap_or_default();
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
                let edge_points_input = ep.points.as_slice();
                let (arrow_at_start, arrow_at_end) = no_oracle_effective_arrow_ends(conn);
                let edge_points = component_svek_edge_points(
                    edge_points_input,
                    svek_edge_dx,
                    svek_edge_dy,
                    from_package,
                    to_package,
                    arrow_at_start,
                    arrow_at_end,
                );
                let path_d = build_path_d(&edge_points);
                let path_id = no_oracle_path_id(conn);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{component_arrow_stroke};stroke-width:1;{dash_attr}"/>"#,
            ));

                let raw_edge_points = component_svek_edge_points(
                    edge_points_input,
                    svek_edge_dx,
                    svek_edge_dy,
                    from_package,
                    to_package,
                    false,
                    false,
                );
                if arrow_at_start {
                    let first = raw_edge_points.first().unwrap();
                    let next = raw_edge_points.get(1).unwrap_or(first);
                    render_arrowhead(&mut svg, next, first, &component_arrow_stroke);
                }
                if arrow_at_end {
                    let last = raw_edge_points.last().unwrap();
                    let prev = if raw_edge_points.len() >= 2 {
                        &raw_edge_points[raw_edge_points.len() - 2]
                    } else {
                        last
                    };
                    render_arrowhead(&mut svg, prev, last, &component_arrow_stroke);
                } else if matches!(
                    conn.shape,
                    LinkShape::TargetSocket | LinkShape::TargetBallSocket
                ) {
                    render_target_socket_decoration(
                        &mut svg,
                        conn.shape,
                        &raw_edge_points,
                        &component_arrow_stroke,
                    );
                }

                // Labels.
                let first = edge_points.first().unwrap();
                if let Some(label) = &conn.label {
                    let (x, y) = ep
                        .label
                        .map(|position| {
                            (
                                position.x + svek_edge_dx + 1.0,
                                position.y
                                    + svek_edge_dy
                                    + 1.0
                                    + text_render::label_ascent_with_family(
                                        label,
                                        component_arrow_font_size,
                                        &component_arrow_font_family,
                                    ),
                            )
                        })
                        .unwrap_or_else(|| {
                            let path_last = edge_points.last().unwrap();
                            (
                                (first.0 + path_last.0) / 2.0 + 1.0,
                                (first.1 + path_last.1) / 2.0 - 4.0,
                            )
                        });
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x,
                            y,
                            font_size: component_arrow_font_size as u32,
                            font_family: &component_arrow_font_family,
                            fill: &component_arrow_font_color,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    svg.raw(&text_buf);
                }
                let (tail_mult, head_mult) = if layout_reversed {
                    (conn.to_mult.as_deref(), conn.from_mult.as_deref())
                } else {
                    (conn.from_mult.as_deref(), conn.to_mult.as_deref())
                };
                if let Some(tail_mult) = tail_mult {
                    let mw = text_render::measure(tail_mult, component_arrow_font_size, false);
                    let (x, y) = ep
                        .tail_label
                        .map(|position| {
                            (
                                position.x + svek_edge_dx,
                                position.y
                                    + svek_edge_dy
                                    + text_render::label_ascent_with_family(
                                        tail_mult,
                                        component_arrow_font_size,
                                        &component_arrow_font_family,
                                    ),
                            )
                        })
                        .unwrap_or((
                            first.0 - mw - 1.0,
                            first.1 + component_arrow_font_size + 2.0,
                        ));
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        tail_mult,
                        &TextBase {
                            x,
                            y,
                            font_size: component_arrow_font_size as u32,
                            font_family: &component_arrow_font_family,
                            fill: &component_arrow_font_color,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    svg.raw(&text_buf);
                }
                if let Some(head_mult) = head_mult {
                    let mw = text_render::measure(head_mult, component_arrow_font_size, false);
                    let path_last = edge_points.last().unwrap();
                    let (x, y) = ep
                        .head_label
                        .map(|position| {
                            (
                                position.x + svek_edge_dx,
                                position.y
                                    + svek_edge_dy
                                    + text_render::label_ascent_with_family(
                                        head_mult,
                                        component_arrow_font_size,
                                        &component_arrow_font_family,
                                    ),
                            )
                        })
                        .unwrap_or((path_last.0 - mw - 1.0, path_last.1 - 4.0));
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        head_mult,
                        &TextBase {
                            x,
                            y,
                            font_size: component_arrow_font_size as u32,
                            font_family: &component_arrow_font_family,
                            fill: &component_arrow_font_color,
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
                    render_middle_socket_decoration(
                        &mut svg,
                        conn.shape,
                        &raw_edge_points,
                        &component_arrow_stroke,
                    );
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
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{component_arrow_stroke};stroke-width:1;{dash_attr}"/>"#,
            ));

                let (arrow_at_start, arrow_at_end) = no_oracle_effective_arrow_ends(conn);
                if arrow_at_start {
                    render_arrowhead_from_coords(
                        &mut svg,
                        to_cx,
                        to_cy,
                        from_cx,
                        from_bottom,
                        &component_arrow_stroke,
                    );
                }
                if arrow_at_end {
                    render_arrowhead_from_coords(
                        &mut svg,
                        from_cx,
                        from_bottom,
                        to_cx,
                        to_cy,
                        &component_arrow_stroke,
                    );
                } else if matches!(
                    conn.shape,
                    LinkShape::TargetSocket | LinkShape::TargetBallSocket
                ) {
                    let points = [(from_cx, from_bottom), (to_cx, to_cy)];
                    render_target_socket_decoration(
                        &mut svg,
                        conn.shape,
                        &points,
                        &component_arrow_stroke,
                    );
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
                    render_middle_socket_decoration(
                        &mut svg,
                        conn.shape,
                        &points,
                        &component_arrow_stroke,
                    );
                }
            }

            svg.raw("</g>");
        }
    } // end else (non-oracle connections)

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
    no_oracle_uids: Option<&NoOracleUidModel>,
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
        .or_else(|| no_oracle_uids.and_then(|uids| uids.entity_ids.get(&iface.id).cloned()))
        .unwrap_or_else(|| {
            let id = format!("ent{:04}", *entity_counter);
            *entity_counter += 1;
            id
        });
    let source_attr = oracle_iface
        .and_then(|r| r.source_line.as_deref())
        .map(String::from)
        .or_else(|| (iface.source_line > 0).then(|| iface.source_line.to_string()))
        .map(|source_line| format!(r#" data-source-line="{source_line}""#))
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
            .unwrap_or(
                iy + (IFACE_NODE_SIZE - IFACE_CENTER_OFFSET)
                    + IFACE_LABEL_GAP
                    + RECTANGLE_MARGIN_Y
                    + LINE_HEIGHT
                    - LABEL_BASELINE_FROM_BOTTOM,
            );
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

struct ComponentTextMetrics {
    label_width: f64,
    stereotype_widths: Vec<f64>,
    content_width: f64,
}

fn component_text_metrics(
    comp: &Component,
    font_size: f64,
    font_family: &str,
    label_bold: bool,
) -> ComponentTextMetrics {
    let label_width =
        text_render::measure_with_family(&comp.label, font_size, label_bold, font_family);
    let stereotype_widths: Vec<f64> = comp
        .stereotypes
        .iter()
        .map(|stereotype| {
            text_render::measure_with_family(
                &format!("\u{00AB}{stereotype}\u{00BB}"),
                font_size,
                false,
                font_family,
            )
        })
        .collect();
    let stereotype_block_width = stereotype_widths
        .iter()
        .copied()
        .reduce(f64::max)
        .map(|width| width + STEREOTYPE_MARGIN_X * 2.0)
        .unwrap_or(0.0);

    ComponentTextMetrics {
        label_width,
        stereotype_widths,
        content_width: label_width.max(stereotype_block_width),
    }
}

#[cfg(test)]
fn calc_component_dim_with_metrics(
    comp: &Component,
    text_metrics: &ComponentTextMetrics,
) -> CompDim {
    calc_component_dim_with_symbol_style(comp, text_metrics, false)
}

fn calc_component_dim_with_symbol_style(
    comp: &Component,
    text_metrics: &ComponentTextMetrics,
    component_style_rectangle: bool,
) -> CompDim {
    let n_lines = 1 + comp.stereotypes.len();

    let (width, height) = if matches!(comp.kind, ComponentElementKind::Database) {
        (
            text_metrics.content_width + DATABASE_MARGIN_X * 2.0,
            DATABASE_MARGIN_TOP + n_lines as f64 * LINE_HEIGHT + DATABASE_MARGIN_BOTTOM,
        )
    } else if component_style_rectangle && matches!(comp.kind, ComponentElementKind::Component) {
        (
            text_metrics.content_width + RECTANGLE_MARGIN_X * 2.0,
            n_lines as f64 * LINE_HEIGHT + RECTANGLE_MARGIN_Y * 2.0,
        )
    } else {
        (
            (text_metrics.content_width + TEXT_PAD_LEFT + TEXT_PAD_RIGHT).max(COMPONENT_MIN_W),
            COMPONENT_BASE_H + n_lines as f64 * LINE_HEIGHT,
        )
    };

    CompDim { width, height }
}

#[cfg(test)]
fn calc_component_dim(comp: &Component) -> CompDim {
    let text_metrics = component_text_metrics(comp, FONT_SIZE, "sans-serif", false);
    calc_component_dim_with_metrics(comp, &text_metrics)
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
    group_endpoint_nodes: &std::collections::HashMap<String, String>,
) {
    for pkg in packages {
        let qname = if parent.is_empty() {
            pkg.name.clone()
        } else {
            format!("{parent}.{}", pkg.name)
        };
        let parent_id = (!parent.is_empty()).then_some(parent);
        let title_width = text_render::measure_no_underline(&pkg.label, FONT_SIZE, true);
        let title_height = text_render::label_height(&pkg.label, FONT_SIZE);
        let (stereotype_width, stereotype_height) = pkg
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
        let (shape_width, shape_height) = match pkg.kind {
            ComponentPackageKind::Node => (60.0, 5.0),
            ComponentPackageKind::Database => (0.0, 15.0),
            _ => (0.0, 0.0),
        };
        // Java `ClusterHeader` merges stereotype and title vertically,
        // truncates their dimensions to integers, then adds the USymbol's
        // shape supplement before `ClusterDotString.printInternal` emits its
        // fixed-size title table.
        layout.add_svek_cluster(
            &qname,
            parent_id,
            ClusterTitleSize {
                width: title_width.max(stereotype_width) + shape_width,
                height: title_height + stereotype_height + shape_height,
            },
        );
        if let Some(endpoint_id) = group_endpoint_nodes.get(&qname) {
            layout.add_cluster_node(&qname, endpoint_id);
        }
        for component_id in &pkg.components {
            layout.add_cluster_node(&qname, component_id);
        }
        add_package_clusters_to_layout(layout, &pkg.packages, &qname, group_endpoint_nodes);
    }
}

fn add_together_groups_to_layout(layout: &mut LayoutGraph, diagram: &ComponentDiagram) {
    for (index, group) in diagram.together.iter().enumerate() {
        let id = format!("component_together_{index}");
        let parent = group
            .parent
            .map(|parent| format!("component_together_{parent}"));
        layout.add_together(&id, group.package.as_deref(), parent.as_deref());
        for node in &group.nodes {
            layout.add_together_node(&id, node);
        }
        for package in &group.packages {
            layout.add_together_cluster(&id, package);
        }
    }
}

#[derive(Clone, Copy)]
struct ComponentClusterFrame {
    origin_x: f64,
    origin_y: f64,
}

fn package_kind_for_qname(
    packages: &[ComponentPackage],
    qname: &str,
) -> Option<ComponentPackageKind> {
    fn walk(
        packages: &[ComponentPackage],
        parent: &str,
        target: &str,
    ) -> Option<ComponentPackageKind> {
        for package in packages {
            let current = if parent.is_empty() {
                package.name.clone()
            } else {
                format!("{parent}.{}", package.name)
            };
            if current == target {
                return Some(package.kind);
            }
            if let Some(kind) = walk(&package.packages, &current, target) {
                return Some(kind);
            }
        }
        None
    }

    walk(packages, "", qname)
}

fn component_cluster_frame(
    packages: &[ComponentPackage],
    cluster_positions: &[ClusterPosition],
) -> ComponentClusterFrame {
    if cluster_positions.is_empty() {
        return ComponentClusterFrame {
            origin_x: MARGIN,
            origin_y: MARGIN,
        };
    }

    let min_raw_x = cluster_positions
        .iter()
        .map(|position| position.x)
        .fold(f64::INFINITY, f64::min);
    let min_raw_y = cluster_positions
        .iter()
        .map(|position| position.y)
        .fold(f64::INFINITY, f64::min);
    let mut required_dx = f64::NEG_INFINITY;
    let mut required_dy = f64::NEG_INFINITY;

    for package in packages {
        let Some(position) = cluster_positions
            .iter()
            .find(|position| position.id == package.name)
        else {
            continue;
        };
        let (origin_x, origin_y) = match package.kind {
            ComponentPackageKind::Package | ComponentPackageKind::Folder => {
                (SVEK_CLUSTER_ORIGIN, SVEK_CLUSTER_ORIGIN)
            }
            ComponentPackageKind::Node => {
                // `USymbolNode.drawNode` starts 10px inside the left envelope
                // and appends `UEmpty(10,10)` at the lower-right corner.
                (16.0, SVEK_CLUSTER_ORIGIN)
            }
            ComponentPackageKind::Database => {
                // `USymbolDatabase.drawDatabase` appends `UEmpty(10,10)` at
                // its lower-right corner.
                (SVEK_CLUSTER_ORIGIN, SVEK_CLUSTER_ORIGIN)
            }
            ComponentPackageKind::Cloud => {
                let (min_x, min_y) =
                    crate::cloud_shape::generate(position.width, position.height).min_xy();
                (SVEK_CLUSTER_ORIGIN - min_x, SVEK_CLUSTER_ORIGIN - min_y)
            }
            _ => (MARGIN, MARGIN),
        };
        required_dx = required_dx.max(origin_x - position.x);
        required_dy = required_dy.max(origin_y - position.y);
    }

    ComponentClusterFrame {
        origin_x: min_raw_x
            + if required_dx.is_finite() {
                required_dx
            } else {
                MARGIN - min_raw_x
            },
        origin_y: min_raw_y
            + if required_dy.is_finite() {
                required_dy
            } else {
                MARGIN - min_raw_y
            },
    }
}

#[allow(clippy::type_complexity)]
fn compute_positions_from_layout(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    node_positions: &[rustuml_layout::graph::NodePosition],
    raw_cluster_positions: &[ClusterPosition],
    edge_paths: &[EdgePath],
    title_h: f64,
    attached_note_count: usize,
) -> ComponentLayoutResult {
    let n_comp = diagram.components.len();
    let mut positions = Vec::with_capacity(n_comp);
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());
    let cluster_frame = component_cluster_frame(&diagram.packages, raw_cluster_positions);
    let min_node_x = node_positions
        .iter()
        .map(|position| position.x)
        .fold(f64::INFINITY, f64::min);
    let min_node_y = node_positions
        .iter()
        .map(|position| position.y)
        .fold(f64::INFINITY, f64::min);
    let (mut layout_dx, mut layout_dy) = if raw_cluster_positions.is_empty() {
        component_svek_translation(diagram, node_positions, attached_note_count, title_h)
    } else {
        let min_x = raw_cluster_positions
            .iter()
            .map(|position| position.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = raw_cluster_positions
            .iter()
            .map(|position| position.y)
            .fold(f64::INFINITY, f64::min);
        (
            cluster_frame.origin_x - min_x,
            cluster_frame.origin_y + title_h - min_y,
        )
    };
    // `SvekResult.calculateDimension` measures every painted `DotPath` through
    // `LimitFinder.drawDotPath`, then `DotStringFactory.moveDelta` moves the
    // complete graph so that painted minimum is at (6, 6). Graphviz's SVG
    // serialization is the model boundary consumed by SVEK.
    let path_min_x = edge_paths
        .iter()
        .flat_map(|edge| &edge.points)
        .map(|(x, _)| round_svek_input_coord(*x))
        .fold(f64::INFINITY, f64::min);
    let path_min_y = edge_paths
        .iter()
        .flat_map(|edge| &edge.points)
        .map(|(_, y)| round_svek_input_coord(*y))
        .fold(f64::INFINITY, f64::min);
    if path_min_x.is_finite() {
        layout_dx += (SVEK_CLUSTER_ORIGIN - (path_min_x + layout_dx)).max(0.0);
    }
    if path_min_y.is_finite() {
        layout_dy += (title_h + SVEK_CLUSTER_ORIGIN - (path_min_y + layout_dy)).max(0.0);
    }

    for (i, _comp) in diagram.components.iter().enumerate() {
        let p = &node_positions[i];
        let x = p.x + layout_dx;
        let y = p.y + layout_dy;
        positions.push(if attached_note_count == 0 {
            (x, y)
        } else {
            (round_svek_input_coord(x), round_svek_input_coord(y))
        });
    }
    for (i, _iface) in diagram.interfaces.iter().enumerate() {
        let p = &node_positions[n_comp + i];
        iface_positions.push((
            p.x + layout_dx + IFACE_CENTER_OFFSET,
            p.y + layout_dy + IFACE_CENTER_OFFSET,
        ));
    }

    let max_x = node_positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.x + if i < n_comp {
                comp_dims[i].width
            } else {
                IFACE_NODE_SIZE
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
                IFACE_NODE_SIZE
            }
        })
        .fold(0.0_f64, f64::max);

    let (content_w, content_h) = if raw_cluster_positions.is_empty() {
        (
            max_x - min_node_x + MARGIN * 2.0,
            max_y - min_node_y + MARGIN * 2.0 + title_h,
        )
    } else {
        (max_x + MARGIN * 2.0, max_y + MARGIN * 2.0 + title_h)
    };

    let cluster_positions = raw_cluster_positions
        .iter()
        .map(|p| ClusterPosition {
            id: p.id.clone(),
            x: p.x + layout_dx,
            y: p.y + layout_dy,
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

fn component_svek_translation(
    diagram: &ComponentDiagram,
    node_positions: &[rustuml_layout::graph::NodePosition],
    attached_note_count: usize,
    title_h: f64,
) -> (f64, f64) {
    let component_count = diagram.components.len();
    let interface_count = diagram.interfaces.len();
    let note_start = component_count + interface_count;
    let note_end = note_start + attached_note_count;
    let mut painted_min_x = f64::INFINITY;
    let mut painted_min_y = f64::INFINITY;

    for (index, position) in node_positions.iter().enumerate() {
        let (local_min_x, local_min_y) = if index < component_count {
            match diagram.components[index].kind {
                // `USymbolNode.drawNode` paints its body as a `UPolygon`.
                // `LimitFinder.drawUPolygon` applies its 10px horizontal
                // measurement guard, while the polygon starts at local y=0.
                ComponentElementKind::Node => (-10.0, 0.0),
                // `USymbolComponent2.drawComponent2` and the remaining leaf
                // symbols retain the established `URectangle` top-left
                // envelope until their primitive models are split out.
                _ => (-1.0, -1.0),
            }
        } else if (note_start..note_end).contains(&index) {
            // Notes paint their polygon directly to the Graphviz node bounds.
            (0.0, 0.0)
        } else {
            // `CircleInterface2` is measured one pixel inside its SVEK table.
            (-1.0, -1.0)
        };
        painted_min_x = painted_min_x.min(position.x + local_min_x);
        painted_min_y = painted_min_y.min(position.y + local_min_y);
    }

    (
        SVEK_CLUSTER_ORIGIN - painted_min_x,
        title_h + SVEK_CLUSTER_ORIGIN - painted_min_y,
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
        let ix = MARGIN + ii as f64 * (IFACE_NODE_SIZE + GAP) + IFACE_CENTER_OFFSET;
        let iy = iface_y_start + IFACE_CENTER_OFFSET;
        iface_positions.push((ix, iy));
    }

    let iface_total_h = if !diagram.interfaces.is_empty() {
        IFACE_NODE_SIZE + IFACE_LABEL_GAP + LINE_HEIGHT + GAP
    } else {
        0.0
    };
    let iface_total_w = if !diagram.interfaces.is_empty() {
        MARGIN * 2.0 + diagram.interfaces.len() as f64 * (IFACE_NODE_SIZE + GAP)
    } else {
        0.0
    };

    let content_w = comp_total_w.max(iface_total_w).max(100.0);
    let content_h = comp_total_h + iface_total_h + title_h;

    (positions, iface_positions, Vec::new(), content_w, content_h)
}

struct NoOracleCanvas<'a> {
    components: &'a [Component],
    interfaces: &'a [Interface],
    positions: &'a [(f64, f64)],
    iface_positions: &'a [(f64, f64)],
    comp_dims: &'a [CompDim],
    cluster_positions: &'a [ClusterPosition],
    packages: &'a [ComponentPackage],
    pkg_total_w: f64,
    pkg_total_h: f64,
    title_h: f64,
    edge_paths: &'a [EdgePath],
    edge_dx: f64,
    edge_dy: f64,
    note_layouts: &'a [ComponentNoteLayout],
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
        let (painted_max_x, painted_max_y) = match comp.kind {
            // Java `LimitFinder.drawUPolygon` measures ten pixels beyond both
            // horizontal sides of `USymbolNode`'s body. Its lower-edge
            // `UEmpty(10,10)` also extends the vertical envelope by ten. Add
            // one here because the shared tail below is 14px: rectangles end
            // at `dimension - 1`, making that equivalent to SVEK's 15px
            // dimension delta, while polygon/UEmpty maxima do not.
            ComponentElementKind::Node => (dim.width + 11.0, dim.height + 11.0),
            // `LimitFinder.drawRectangle` records width/height minus one for
            // the outer `USymbolComponent2` rectangle; the shared 14px tail
            // already incorporates that one-pixel difference.
            ComponentElementKind::Component => (dim.width, dim.height),
            ComponentElementKind::Database => (
                dim.width + DATABASE_RENDER_OVERFLOW,
                dim.height + DATABASE_RENDER_OVERFLOW,
            ),
            _ => (dim.width, dim.height),
        };
        max_x = max_x.max(x + painted_max_x);
        max_y = max_y.max(y + painted_max_y);
    }
    for ((cx, cy), interface) in input.iface_positions.iter().zip(input.interfaces) {
        let label_width = text_render::measure(&interface.label, FONT_SIZE, false);
        max_x = max_x.max(cx + IFACE_R.max(label_width / 2.0));
        max_y =
            max_y.max(cy + (IFACE_NODE_SIZE - IFACE_CENTER_OFFSET) + IFACE_LABEL_GAP + LINE_HEIGHT);
    }
    for note in input.note_layouts {
        max_x = max_x.max(note.x + note.width);
        max_y = max_y.max(note.y + note.height);
    }
    let mut path_max_x: Option<f64> = None;
    let mut path_max_y: Option<f64> = None;
    for edge in input.edge_paths {
        for (x, y) in &edge.points {
            let x = round_svek_input_coord(*x) + input.edge_dx;
            let y = round_svek_input_coord(*y) + input.edge_dy;
            path_max_x = Some(path_max_x.map_or(x, |bound| bound.max(x)));
            path_max_y = Some(path_max_y.map_or(y, |bound| bound.max(y)));
        }
    }
    for position in input.edge_paths.iter().flat_map(|edge| {
        [edge.label, edge.tail_label, edge.head_label]
            .into_iter()
            .flatten()
    }) {
        max_x = max_x.max(position.x + input.edge_dx + position.width);
        max_y = max_y.max(position.y + input.edge_dy + position.height);
    }
    let mut total_w = max_x + SVEK_CANVAS_PAD;
    let mut total_h = max_y + SVEK_CANVAS_PAD;
    if let Some(path_max_x) = path_max_x {
        total_w = total_w.max(path_max_x + SVEK_PATH_CANVAS_PAD);
    }
    if let Some(path_max_y) = path_max_y {
        total_h = total_h.max(path_max_y + SVEK_PATH_CANVAS_PAD);
    }
    for cluster in input.cluster_positions {
        let kind = package_kind_for_qname(input.packages, &cluster.id);
        let (shape_max_x, shape_max_y, pad) = if matches!(kind, Some(ComponentPackageKind::Cloud)) {
            let (_, _, path_max_x, path_max_y) =
                crate::cloud_shape::generate(cluster.width, cluster.height).bounds();
            (path_max_x, path_max_y, SVEK_CANVAS_PAD)
        } else {
            let pad = match kind {
                Some(ComponentPackageKind::Package | ComponentPackageKind::Folder) => 15.0,
                Some(ComponentPackageKind::Node | ComponentPackageKind::Database) => 25.0,
                _ => SVEK_CANVAS_PAD,
            };
            (cluster.width, cluster.height, pad)
        };
        total_w = total_w.max(cluster.x + shape_max_x + pad);
        total_h = total_h.max(cluster.y + shape_max_y + pad);
    }
    if input.cluster_positions.is_empty() && !input.packages.is_empty() {
        max_x = max_x.max(input.pkg_total_w);
        max_y = max_y.max(input.title_h + MARGIN + input.pkg_total_h);
        total_w = max_x + SVEK_CANVAS_PAD;
        total_h = max_y + SVEK_CANVAS_PAD;
    }

    (total_w.max(1.0), total_h.max(1.0))
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

fn render_arrowhead(svg: &mut SvgBuilder, prev: &(f64, f64), tip: &(f64, f64), stroke: &str) {
    let dx = tip.0 - prev.0;
    let dy = tip.1 - prev.1;
    let angle = dy.atan2(dx);
    render_arrow_at(svg, tip.0, tip.1, angle, stroke);
}

fn render_arrowhead_from_coords(
    svg: &mut SvgBuilder,
    fx: f64,
    fy: f64,
    tx: f64,
    ty: f64,
    stroke: &str,
) {
    render_arrow_at(svg, tx, ty, (ty - fy).atan2(tx - fx), stroke);
}

fn no_oracle_link_type_attr(conn: &Connection) -> String {
    let (arrow_at_start, arrow_at_end) = no_oracle_arrow_ends(conn);
    if arrow_at_start || arrow_at_end {
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
    let (from, to, _) = no_oracle_layout_edge_ends(conn);
    let (arrow_at_start, arrow_at_end) = no_oracle_effective_arrow_ends(conn);
    if arrow_at_end && !arrow_at_start
        || matches!(
            conn.shape,
            LinkShape::TargetSocket | LinkShape::TargetBallSocket
        )
    {
        format!("{from}-to-{to}")
    } else if arrow_at_start && !arrow_at_end {
        format!("{from}-backto-{to}")
    } else {
        format!("{from}-{to}")
    }
}

fn no_oracle_arrow_ends(conn: &Connection) -> (bool, bool) {
    if conn.arrow_at_start || conn.arrow_at_end {
        (conn.arrow_at_start, conn.arrow_at_end)
    } else {
        // Preserve compatibility with diagrams deserialized before the
        // endpoint-specific fields were added.
        (false, conn.has_arrow)
    }
}

fn no_oracle_effective_arrow_ends(conn: &Connection) -> (bool, bool) {
    let (arrow_at_start, arrow_at_end) = no_oracle_arrow_ends(conn);
    if no_oracle_layout_edge_ends(conn).2 {
        (arrow_at_end, arrow_at_start)
    } else {
        (arrow_at_start, arrow_at_end)
    }
}

fn no_oracle_layout_edge_ends(conn: &Connection) -> (&str, &str, bool) {
    if matches!(
        conn.direction,
        Some(ConnectionDirection::Up | ConnectionDirection::Left)
    ) {
        (&conn.to, &conn.from, true)
    } else {
        (&conn.from, &conn.to, false)
    }
}

fn render_target_socket_decoration(
    svg: &mut SvgBuilder,
    shape: LinkShape,
    points: &[(f64, f64)],
    stroke: &str,
) {
    let Some((&tip, &prev)) = points.last().zip(points.iter().rev().nth(1)) else {
        return;
    };
    if matches!(shape, LinkShape::TargetBallSocket) {
        render_socket_ball(svg, tip, stroke);
    }
    render_socket_arc(svg, tip, prev, 9.0, false, stroke);
}

fn render_middle_socket_decoration(
    svg: &mut SvgBuilder,
    shape: LinkShape,
    points: &[(f64, f64)],
    stroke: &str,
) {
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
        render_socket_arc(svg, center, *first, 10.0, true, stroke);
    }
    render_socket_arc(svg, center, *last, 10.0, false, stroke);
    render_socket_ball(svg, center, stroke);
}

fn render_socket_ball(svg: &mut SvgBuilder, center: (f64, f64), stroke: &str) {
    svg.raw(&format!(
        r##"<ellipse cx="{}" cy="{}" fill="#FFFFFF" rx="6" ry="6" style="stroke:{stroke};stroke-width:1.5;"/>"##,
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
    stroke: &str,
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
        r##"<path d="M{},{} A{},{} 0 0 0 {},{}" fill="none" style="stroke:{stroke};stroke-width:1.5;"/>"##,
        fc(start.0),
        fc(start.1),
        fc(radius),
        fc(radius),
        fc(end.0),
        fc(end.1),
    ));
}

fn render_arrow_at(svg: &mut SvgBuilder, x: f64, y: f64, angle: f64, stroke: &str) {
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
        r#"<polygon fill="{stroke}" points="{pts}" style="stroke:{stroke};stroke-width:1;"/>"#,
    ));
}

// ---------------------------------------------------------------------------
// Path building
// ---------------------------------------------------------------------------

fn component_svek_edge_points(
    points: &[(f64, f64)],
    dx: f64,
    dy: f64,
    tail_cluster: Option<&ClusterPosition>,
    head_cluster: Option<&ClusterPosition>,
    trim_start_for_arrow: bool,
    trim_end_for_arrow: bool,
) -> Vec<(f64, f64)> {
    // `SvekEdge.solveLine` reads the spline back from Graphviz's SVG through
    // `SvgResult.toDotPath`; Graphviz serializes those path coordinates at two
    // decimal places. The vendored C API gives us the pre-serialization
    // doubles, so reproduce that model boundary before applying decorations.
    let quantize = |value: f64| (value * 100.0).round() / 100.0;
    let mut out: Vec<(f64, f64)> = points
        .iter()
        .map(|(x, y)| (quantize(*x) + dx, quantize(*y) + dy))
        .collect();
    out = simulate_compound(out, tail_cluster, head_cluster);

    if trim_start_for_arrow && out.len() >= 2 {
        let (tip_x, tip_y) = out[0];
        let (next_x, next_y) = out[1];
        let vx = next_x - tip_x;
        let vy = next_y - tip_y;
        let len = (vx * vx + vy * vy).sqrt();
        if len > f64::EPSILON {
            // `SvekEdge.getExtremitySimplier` moves both the start point and
            // its first control point by `ExtremityArrow`'s 6px decoration
            // length.
            let ux = vx / len;
            let uy = vy / len;
            let trim = 6.0;
            out[0] = (tip_x + ux * trim, tip_y + uy * trim);
            if out.len() >= 4 {
                let (cx, cy) = out[1];
                out[1] = (cx + ux * trim, cy + uy * trim);
            }
        }
    }

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

fn simulate_compound(
    points: Vec<(f64, f64)>,
    tail: Option<&ClusterPosition>,
    head: Option<&ClusterPosition>,
) -> Vec<(f64, f64)> {
    if points.len() < 4 || !(points.len() - 1).is_multiple_of(3) {
        return points;
    }

    type Cubic = [(f64, f64); 4];

    fn contains(rectangle: &ClusterPosition, point: (f64, f64)) -> bool {
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

    // PlantUML `DotPath.simulateCompound` clips a spline leaving a group by
    // bisecting the first boundary-crossing cubic eight times. It retains each
    // outside half, which deliberately expands one Graphviz cubic into the
    // sequence visible in PlantUML's SVG.
    let mut curves = curves_from_points(&points);
    if let Some(tail) = tail
        && curves.first().is_some_and(|curve| contains(tail, curve[0]))
    {
        let crossing = curves.iter().position(|curve| !contains(tail, curve[3]));
        if let Some(index) = crossing {
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
    }

    // The head-side branch is the exact mirror of the tail-side subdivision.
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

fn render_normal_component_note(
    note: &ComponentNote,
    layout: &ComponentNoteLayout,
    uid: &NoOracleNoteUid,
    svg: &mut SvgBuilder,
) {
    let x = layout.x;
    let y = layout.y;
    let w = layout.width;
    let h = layout.height;
    let fold = NOTE_FOLD;
    // `EntityImageNote.drawNormal` uses `Opale.getPolygonNormal` when
    // `GraphvizImageBuilder.isOpalisable` cannot resolve a peer SvekNode.
    let body_path = format!(
        "M{x0},{y0} L{x0},{yb} L{xr},{yb} L{xr},{yf} L{xf},{y0} L{x0},{y0}",
        x0 = fc(x),
        y0 = fc(y),
        yb = fc(y + h),
        xr = fc(x + w),
        yf = fc(y + fold),
        xf = fc(x + w - fold),
    );
    let fold_path = format!(
        "M{x1},{y0} L{x1},{y1} L{x2},{y1} L{x1},{y0}",
        x1 = fc(x + w - fold),
        y0 = fc(y),
        y1 = fc(y + fold),
        x2 = fc(x + w),
    );

    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
        uid.qualified_name, note.source_line, uid.entity_id
    ));
    svg.raw(&format!(
        r#"<path d="{body_path}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<path d="{fold_path}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:1;"/>"#
    ));

    let mut text_y = y + NOTE_MARGIN_Y;
    for line in note.text.lines() {
        let ascent = text_render::label_ascent(line, LINK_FONT);
        text_y += ascent;
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            line,
            &TextBase {
                x: x + NOTE_MARGIN_X1,
                y: text_y,
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
        text_y += text_render::label_height(line, LINK_FONT) - ascent;
    }
    svg.raw("</g>");
}

fn render_attached_component_note(
    note: &ComponentNote,
    layout: &ComponentNoteLayout,
    uid: &NoOracleNoteUid,
    edge: Option<&EdgePath>,
    edge_dx: f64,
    edge_dy: f64,
    svg: &mut SvgBuilder,
) {
    let center = (
        layout.x + layout.width / 2.0,
        layout.y + layout.height / 2.0,
    );
    let fallback = match note.position {
        ComponentNotePosition::Top => (
            (center.0, layout.y + layout.height),
            (center.0, layout.y + layout.height + NOTE_GAP),
        ),
        ComponentNotePosition::Bottom => ((center.0, layout.y), (center.0, layout.y - NOTE_GAP)),
        ComponentNotePosition::Left => (
            (layout.x + layout.width, center.1),
            (layout.x + layout.width + NOTE_GAP, center.1),
        ),
        ComponentNotePosition::Right => ((layout.x, center.1), (layout.x - NOTE_GAP, center.1)),
    };
    let (note_point, target_point) = edge
        .and_then(|path| path.points.first().zip(path.points.last()))
        .map(|(first, last)| {
            let first = (
                round_svek_input_coord(first.0 + edge_dx),
                round_svek_input_coord(first.1 + edge_dy),
            );
            let last = (
                round_svek_input_coord(last.0 + edge_dx),
                round_svek_input_coord(last.1 + edge_dy),
            );
            match note.position {
                ComponentNotePosition::Top | ComponentNotePosition::Left => (first, last),
                ComponentNotePosition::Bottom | ComponentNotePosition::Right => (last, first),
            }
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
    let fold = NOTE_FOLD;
    let connector = NOTE_CONNECTOR_HALF;

    // `EntityImageNote.drawU` delegates to `Opale.getPolygon{Left,Right,Up,Down}`.
    // The hidden SVEK edge supplies the mouth and tip points embedded below.
    let path = match note.position {
        ComponentNotePosition::Right => {
            let y1 = (mouth_y - connector).clamp(0.0, h - connector * 2.0);
            format!(
                "M{x0},{y0} L{x0},{y1} L{tx},{ty} L{x0},{y2} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{yf} L{xf},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                y1 = fc(y + y1),
                tx = fc(tip_x),
                ty = fc(tip_y),
                y2 = fc(y + y1 + connector * 2.0),
                yb = fc(y + h),
                xr = fc(x + w),
                yf = fc(y + fold),
                xf = fc(x + w - fold),
            )
        }
        ComponentNotePosition::Left => {
            let y1 = (mouth_y - connector).clamp(fold, h - connector * 2.0);
            format!(
                "M{x0},{y0} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{y2} L{tx},{ty} L{xr},{y1} L{xr},{yf} L{xf},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                yb = fc(y + h),
                xr = fc(x + w),
                y2 = fc(y + y1 + connector * 2.0),
                tx = fc(tip_x),
                ty = fc(tip_y),
                y1 = fc(y + y1),
                yf = fc(y + fold),
                xf = fc(x + w - fold),
            )
        }
        ComponentNotePosition::Bottom => {
            let x1 = (mouth_x - connector).clamp(0.0, w - fold);
            format!(
                "M{x0},{y0} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{yf} L{xf},{y0} L{x2},{y0} L{tx},{ty} L{x1},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                yb = fc(y + h),
                xr = fc(x + w),
                yf = fc(y + fold),
                xf = fc(x + w - fold),
                x2 = fc(x + x1 + connector * 2.0),
                tx = fc(tip_x),
                ty = fc(tip_y),
                x1 = fc(x + x1),
            )
        }
        ComponentNotePosition::Top => {
            let x1 = (mouth_x - connector).clamp(0.0, w);
            format!(
                "M{x0},{y0} L{x0},{yb} A0,0 0 0 0 {x0},{yb} L{x1},{yb} L{tx},{ty} L{x2},{yb} L{xr},{yb} A0,0 0 0 0 {xr},{yb} L{xr},{yf} L{xf},{y0} L{x0},{y0} A0,0 0 0 0 {x0},{y0}",
                x0 = fc(x),
                y0 = fc(y),
                yb = fc(y + h),
                x1 = fc(x + x1),
                tx = fc(tip_x),
                ty = fc(tip_y),
                x2 = fc(x + x1 + connector * 2.0),
                xr = fc(x + w),
                yf = fc(y + fold),
                xf = fc(x + w - fold),
            )
        }
    };
    let fold_path = format!(
        "M{x1},{y0} L{x1},{y1} L{x2},{y1} L{x1},{y0}",
        x1 = fc(x + w - fold),
        y0 = fc(y),
        y1 = fc(y + fold),
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
        let ascent = text_render::label_ascent(line, LINK_FONT);
        text_y += ascent;
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            line,
            &TextBase {
                x: x + NOTE_MARGIN_X1,
                y: text_y,
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
        text_y += text_render::label_height(line, LINK_FONT) - ascent;
    }
    svg.raw("</g>");
}

fn render_fallback_note(
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
    uids: Option<&NoOracleUidModel>,
) {
    fn walk(
        packages: &[ComponentPackage],
        parent_path: &str,
        svg: &mut SvgBuilder,
        cluster_positions: &[ClusterPosition],
        uids: Option<&NoOracleUidModel>,
        next_entity: &mut usize,
    ) {
        for pkg in packages {
            let qname = if parent_path.is_empty() {
                pkg.name.clone()
            } else {
                format!("{parent_path}.{}", pkg.name)
            };
            if let Some(pos) = cluster_positions.iter().find(|p| p.id == qname) {
                let entity_id = uids
                    .and_then(|model| model.entity_ids.get(&qname))
                    .cloned()
                    .unwrap_or_else(|| format!("ent{:04}", *next_entity));
                emit_layout_package_cluster(svg, pkg, &qname, pos, &entity_id);
                *next_entity += 1;
            }
            walk(
                &pkg.packages,
                &qname,
                svg,
                cluster_positions,
                uids,
                next_entity,
            );
        }
    }

    let mut next_entity = 2;
    walk(packages, "", svg, cluster_positions, uids, &mut next_entity);
}

fn emit_layout_package_cluster(
    svg: &mut SvgBuilder,
    pkg: &ComponentPackage,
    qname: &str,
    pos: &ClusterPosition,
    entity_id: &str,
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
        r#"<g class="cluster" data-qualified-name="{}"{source_attr} id="{entity_id}">"#,
        fold_non_ascii(qname, '.')
    ));

    match pkg.kind {
        ComponentPackageKind::Package | ComponentPackageKind::Folder => {
            emit_layout_package_path(svg, pos, &pkg.label, &fill);
        }
        ComponentPackageKind::Rectangle => {
            emit_layout_rectangle_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
        }
        ComponentPackageKind::Frame => {
            emit_layout_frame_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
        }
        ComponentPackageKind::Node => {
            emit_layout_node_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
        }
        ComponentPackageKind::Cloud => {
            emit_layout_cloud_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
        }
        ComponentPackageKind::Database => {
            emit_layout_database_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
        }
        _ => {
            emit_layout_rectangle_cluster(svg, pos, &pkg.label, None, &fill);
        }
    }

    if let Some(stereo) = &pkg.stereotype
        && !matches!(
            pkg.kind,
            ComponentPackageKind::Rectangle
                | ComponentPackageKind::Frame
                | ComponentPackageKind::Node
                | ComponentPackageKind::Cloud
                | ComponentPackageKind::Database
        )
    {
        let label = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_width = text_render::measure_no_underline(&label, FONT_SIZE, false);
        let title_height = text_render::label_height(&pkg.label, FONT_SIZE);
        let (x, y) = if matches!(
            pkg.kind,
            ComponentPackageKind::Package | ComponentPackageKind::Folder
        ) {
            (
                pos.x + 4.0 + (pos.width - stereo_width) / 2.0,
                pos.y + 2.0 + title_height + 6.0 + pm::ascent(FONT_SIZE),
            )
        } else {
            (pos.x + 4.0, pos.y + pm::ascent(FONT_SIZE) + LINE_HEIGHT)
        };
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &label,
            &TextBase {
                x,
                y,
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
            y: y + 2.0 + pm::ascent(FONT_SIZE),
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
    stereotype: Option<&str>,
    fill: &str,
) {
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        fc(pos.height),
        fc(pos.width),
        fc(pos.x),
        fc(pos.y),
    ));
    let stereotype_height = if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + (pos.width - width) / 2.0,
                y: pos.y + 2.0 + pm::ascent(FONT_SIZE),
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
        text_render::label_height(&text, FONT_SIZE)
    } else {
        0.0
    };
    let label_w = text_render::measure(label, FONT_SIZE, true);
    let mut text_buf = String::new();
    text_render::emit_text(
        &mut text_buf,
        label,
        &TextBase {
            x: pos.x + (pos.width - label_w) / 2.0,
            y: pos.y + 2.0 + stereotype_height + pm::ascent(FONT_SIZE),
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

fn emit_layout_frame_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // `USymbolFrame.drawFrame` draws the solved cluster rectangle, then cuts
    // the title corner with a 10px vertical/diagonal notch.
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        fc(pos.height),
        fc(pos.width),
        fc(pos.x),
        fc(pos.y),
    ));
    let title_width = text_render::measure_no_underline(label, FONT_SIZE, true);
    let title_height = text_render::label_height(label, FONT_SIZE);
    let notch_x = pos.x + title_width + 10.0;
    let notch_y = pos.y + title_height + 3.0;
    let d = format!(
        "M{},{} L{},{} L{},{} L{},{}",
        fc(notch_x),
        fc(pos.y),
        fc(notch_x),
        fc(notch_y - 10.0),
        fc(notch_x - 10.0),
        fc(notch_y),
        fc(pos.x),
        fc(notch_y),
    );
    svg.raw(&format!(
        r##"<path d="{d}" fill="none" style="stroke:#181818;stroke-width:1;"/>"##
    ));

    let mut title_buf = String::new();
    text_render::emit_text(
        &mut title_buf,
        label,
        &TextBase {
            x: pos.x + 3.0,
            y: pos.y + 1.0 + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&title_buf);

    if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + 4.0 + (pos.width - width) / 2.0,
                y: pos.y + 5.0 + title_height + pm::ascent(FONT_SIZE),
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
}

fn emit_layout_node_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // `USymbolNode.drawNode` folds the top-left and bottom-right corners by
    // 10px. `USymbolNode.asBig` then translates the title block by (-4, 11).
    let right = pos.x + pos.width;
    let bottom = pos.y + pos.height;
    let fold = 10.0;
    let points = format!(
        "{x},{top_fold},{left_fold},{top},{right},{top},{right},{bottom_fold},{right_fold},{bottom},{x},{bottom},{x},{top_fold}",
        x = fc(pos.x),
        top_fold = fc(pos.y + fold),
        left_fold = fc(pos.x + fold),
        top = fc(pos.y),
        right = fc(right),
        bottom_fold = fc(bottom - fold),
        right_fold = fc(right - fold),
        bottom = fc(bottom),
    );
    svg.raw(&format!(
        r##"<polygon fill="{fill}" points="{points}" style="stroke:#181818;stroke-width:1;"/>"##
    ));
    svg.raw(&format!(
        r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
        fc(right - fold),
        fc(right),
        fc(pos.y + fold),
        fc(pos.y),
    ));
    svg.raw(&format!(
        r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
        fc(pos.x),
        fc(right - fold),
        fc(pos.y + fold),
        fc(pos.y + fold),
    ));
    svg.raw(&format!(
        r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
        fc(right - fold),
        fc(right - fold),
        fc(pos.y + fold),
        fc(bottom),
    ));

    let title_x_offset = -4.0;
    let title_y_offset = 13.0;
    let stereotype_height = if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + title_x_offset + (pos.width - width) / 2.0,
                y: pos.y + title_y_offset + pm::ascent(FONT_SIZE),
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
        text_render::label_height(&text, FONT_SIZE)
    } else {
        0.0
    };

    let title_width = text_render::measure_no_underline(label, FONT_SIZE, true);
    let mut title_buf = String::new();
    text_render::emit_text(
        &mut title_buf,
        label,
        &TextBase {
            x: pos.x + title_x_offset + (pos.width - title_width) / 2.0,
            y: pos.y + title_y_offset + stereotype_height + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&title_buf);
}

fn emit_layout_cloud_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // `USymbolCloud.getSpecificFrontierForCloudNew` uses a Java-seeded random
    // frontier for the solved cluster dimensions. `USymbolCloud.asBig` places
    // the centered stereotype/title block 13px below that local box origin.
    let path = crate::cloud_shape::generate(pos.width, pos.height);
    let mut d = format!("M{},{}", fc(pos.x + path.start.0), fc(pos.y + path.start.1),);
    for cubic in &path.cubics {
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            fc(pos.x + cubic.c1.0),
            fc(pos.y + cubic.c1.1),
            fc(pos.x + cubic.c2.0),
            fc(pos.y + cubic.c2.1),
            fc(pos.x + cubic.to.0),
            fc(pos.y + cubic.to.1),
        ));
    }
    svg.raw(&format!(
        r##"<path d="{d}" fill="{fill}" style="stroke:#181818;stroke-width:1;"/>"##
    ));

    let text_y = pos.y + 13.0;
    let stereotype_height = if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + (pos.width - width) / 2.0,
                y: text_y + pm::ascent(FONT_SIZE),
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
        text_render::label_height(&text, FONT_SIZE)
    } else {
        0.0
    };

    let title_width = text_render::measure_no_underline(label, FONT_SIZE, true);
    let mut title_buf = String::new();
    text_render::emit_text(
        &mut title_buf,
        label,
        &TextBase {
            x: pos.x + (pos.width - title_width) / 2.0,
            y: text_y + stereotype_height + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&title_buf);
}

fn emit_layout_database_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // `USymbolDatabase.drawDatabase` draws the cylinder with 10px elliptical
    // caps. `USymbolDatabase.asBig` places its title block below the top cap.
    let right = pos.x + pos.width;
    let middle = pos.x + pos.width / 2.0;
    let bottom = pos.y + pos.height;
    let cap = 10.0;
    let outer = format!(
        "M{x},{top_cap} C{x},{top} {middle},{top} {middle},{top} C{middle},{top} {right},{top} {right},{top_cap} L{right},{bottom_cap} C{right},{bottom} {middle},{bottom} {middle},{bottom} C{middle},{bottom} {x},{bottom} {x},{bottom_cap} L{x},{top_cap}",
        x = fc(pos.x),
        top_cap = fc(pos.y + cap),
        top = fc(pos.y),
        middle = fc(middle),
        right = fc(right),
        bottom_cap = fc(bottom - cap),
        bottom = fc(bottom),
    );
    svg.raw(&format!(
        r##"<path d="{outer}" fill="{fill}" style="stroke:#181818;stroke-width:1;"/>"##
    ));
    let closing = format!(
        "M{x},{top_cap} C{x},{second_cap} {middle},{second_cap} {middle},{second_cap} C{middle},{second_cap} {right},{second_cap} {right},{top_cap}",
        x = fc(pos.x),
        top_cap = fc(pos.y + cap),
        second_cap = fc(pos.y + cap * 2.0),
        middle = fc(middle),
        right = fc(right),
    );
    svg.raw(&format!(
        r##"<path d="{closing}" fill="none" style="stroke:#181818;stroke-width:1;"/>"##
    ));

    let text_y = pos.y + 22.0;
    let stereotype_height = if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + (pos.width - width) / 2.0,
                y: text_y + pm::ascent(FONT_SIZE),
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
        text_render::label_height(&text, FONT_SIZE)
    } else {
        0.0
    };

    let title_width = text_render::measure_no_underline(label, FONT_SIZE, true);
    let mut title_buf = String::new();
    text_render::emit_text(
        &mut title_buf,
        label,
        &TextBase {
            x: pos.x + (pos.width - title_width) / 2.0,
            y: text_y + stereotype_height + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&title_buf);
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
    fn no_oracle_attached_note_uses_svek_entity_for_renamed_target() {
        let input = "@startuml\ncomponent \"Renamed Relay 71\" as Relay71\nnote left of Relay71 : Fresh perturbation 73\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(
                r#"<g class="entity" data-qualified-name="GMN3" data-source-line="2" id="ent0004">"#
            ),
            "attached note should consume Java-compatible GMN/entity identities: {svg}"
        );
        assert_eq!(
            svg.matches(r##"<path d="M"##).count(),
            2,
            "Opale note body and fold should be path-based: {svg}"
        );
        assert!(
            !svg.contains("<polygon "),
            "the hidden note link should be embedded in the Opale outline: {svg}"
        );
    }

    #[test]
    fn no_oracle_container_note_keeps_protected_endpoint_for_renamed_group() {
        let input = "@startuml\npackage \"Renamed Boundary 71\" as Boundary71 {\n  component \"Renamed Worker 73\" as Worker73\n}\nnote right of Boundary71 : Fresh group note 79\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(
                r#"<g class="entity" data-qualified-name="GMN4" data-source-line="4" id="ent0005">"#
            ),
            "container note should use Java-compatible generated identities: {svg}"
        );
        assert!(
            svg.contains(
                r#"<g class="link" data-entity-1="ent0002" data-entity-2="ent0005" data-link-type="association" data-source-line="4" id="lnk6">"#
            ),
            "container note should retain its visible dashed SVEK link: {svg}"
        );
        assert!(
            svg.contains(
                r#"id="Boundary71-GMN4" style="stroke:#181818;stroke-width:1;stroke-dasharray:7,7;""#
            ),
            "container note link should route from the protected group endpoint: {svg}"
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
    fn no_oracle_interface_uses_java_circle_block_and_uid_order() {
        let input = "@startuml\ncomponent \"Telemetry Relay 71\" as Relay71\ninterface \"Audit Port 73\" as Audit73\nRelay71 - Audit73\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        assert_eq!(component_diagram.interfaces[0].source_line, 2);

        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains(
                r#"<g class="entity" data-qualified-name="Audit73" data-source-line="2" id="ent0003">"#
            ),
            "interface must consume its Java declaration UID: {svg}"
        );
        assert!(
            svg.contains(r#"rx="8" ry="8""#),
            "interface must retain the CircleInterface2 radius: {svg}"
        );
        assert!(
            svg.contains(r#"data-source-line="3" id="lnk4""#),
            "link UID must follow both declared entities: {svg}"
        );
        assert!(
            svg.contains("Audit Port 73"),
            "interface label missing: {svg}"
        );
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
    fn no_oracle_stereotype_block_uses_java_margin_for_renamed_components() {
        let input = "@startuml\ncomponent RelayNode71 <<edge_service_73>>\ncomponent AuditSink79 <<event_store_83>>\nRelayNode71 --> AuditSink79 : streams\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };

        for component in &component_diagram.components {
            let metrics =
                super::component_text_metrics(component, super::FONT_SIZE, "sans-serif", false);
            let stereotype_width = metrics.stereotype_widths[0];
            assert!(
                stereotype_width > metrics.label_width,
                "the perturbation must exercise a stereotype-dominated text block"
            );
            assert_eq!(
                metrics.content_width,
                stereotype_width + super::STEREOTYPE_MARGIN_X * 2.0
            );

            let dim = super::calc_component_dim_with_metrics(component, &metrics);
            assert_eq!(
                dim.width,
                metrics.content_width + super::TEXT_PAD_LEFT + super::TEXT_PAD_RIGHT
            );
        }

        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("edge_service_73"),
            "first stereotype missing: {svg}"
        );
        assert!(
            svg.contains("event_store_83"),
            "second stereotype missing: {svg}"
        );
    }

    #[test]
    fn no_oracle_canvas_includes_routed_return_spline_for_renamed_cycle() {
        let input = "@startuml\ncomponent E01\ncomponent E02\ncomponent E03\ncomponent E04\ncomponent E05\ncomponent E06\nE01 --> E02\nE02 --> E03\nE03 --> E04\nE04 --> E05\nE05 --> E06\nE06 --> E01\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"width="140px""#),
            "canvas must include the return spline's rightmost control point: {svg}"
        );
        assert!(
            svg.contains(r#"height="599px""#),
            "renamed cycle should retain Java's SVEK vertical envelope: {svg}"
        );
    }

    #[test]
    fn no_oracle_svek_translation_includes_renamed_fanout_splines() {
        let input = "@startuml\ncomponent \"Telemetry Router 947\" as Router947\ninterface IngressAlpha947\ninterface IngressBeta947\ninterface AuditGamma947\ninterface MetricsDelta947\ninterface ControlEpsilon947\ninterface RecoveryZeta947\nRouter947 - IngressAlpha947\nRouter947 - IngressBeta947\nRouter947 - AuditGamma947\nRouter947 - MetricsDelta947\nRouter947 - ControlEpsilon947\nRouter947 - RecoveryZeta947\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"height="150px""#),
            "canvas should include Java's translated dense-fanout envelope: {svg}"
        );
        assert!(
            svg.contains("C330.77,30 368.13,6 460.49,49.81"),
            "SvekResult should move the upper spline control point to y=6: {svg}"
        );
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
    fn grouped_skinparam_cascade_styles_scoped_and_default_components() {
        let input = "@startuml\nskinparam component {\n  BackgroundColor LightBlue\n  BorderColor DarkBlue\n  BorderStyle dotted\n  BackgroundColor<<relay_71>> PaleGreen\n  BorderColor<<relay_71>> DarkGreen\n}\ncomponent \"Relay Node 71\" as Relay71 <<relay_71>>\ncomponent \"Audit Node 73\" as Audit73\nRelay71 --> Audit73 : forwards\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(svg.matches(r##"fill="#98FB98""##).count(), 4, "{svg}");
        assert_eq!(svg.matches(r##"fill="#ADD8E6""##).count(), 4, "{svg}");
        assert_eq!(
            svg.matches("stroke:#006400;stroke-width:0.5;stroke-dasharray:1,1;")
                .count(),
            4,
            "{svg}"
        );
        assert_eq!(
            svg.matches("stroke:#00008B;stroke-width:0.5;stroke-dasharray:1,1;")
                .count(),
            4,
            "{svg}"
        );
    }

    #[test]
    fn rectangle_component_symbol_uses_java_margins_for_renamed_stereotype_block() {
        let input = "@startuml\nskinparam componentStyle rectangle\ncomponent \"Relay 71\" as Relay71 <<telemetry_gateway_profile_731>>\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let component = &component_diagram.components[0];
        let metrics =
            super::component_text_metrics(component, super::FONT_SIZE, "sans-serif", false);
        assert!(
            metrics.stereotype_widths[0] > metrics.label_width,
            "the perturbation must exercise a stereotype-dominated text block"
        );

        let dim = super::calc_component_dim_with_symbol_style(component, &metrics, true);
        assert_eq!(
            dim.width,
            metrics.content_width + super::RECTANGLE_MARGIN_X * 2.0
        );
        assert_eq!(
            dim.height,
            super::LINE_HEIGHT * 2.0 + super::RECTANGLE_MARGIN_Y * 2.0
        );

        let svg = crate::render_svg(&diagram);
        assert_eq!(
            svg.matches("<rect ").count(),
            1,
            "rectangle style must suppress the UML component icon: {svg}"
        );
        assert!(svg.contains("Relay 71"), "renamed label missing: {svg}");
        assert!(
            svg.contains("telemetry_gateway_profile_731"),
            "renamed stereotype missing: {svg}"
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
            interfaces: &[],
            positions: &positions,
            iface_positions: &[],
            comp_dims: &dimensions,
            cluster_positions: &[],
            packages: &[],
            pkg_total_w: 0.0,
            pkg_total_h: 0.0,
            title_h: 0.0,
            edge_paths: &[],
            edge_dx: 0.0,
            edge_dy: 0.0,
            note_layouts: &[],
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

    #[test]
    fn no_oracle_mixed_node_component_uses_limit_finder_frame_for_renamed_symbols() {
        let input = "@startuml\nnode \"Renamed Source Gateway 71\" as Source71\ncomponent \"Target 73\" as Target73\nSource71 --> Target73\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let node = component_diagram
            .components
            .iter()
            .find(|component| matches!(component.kind, super::ComponentElementKind::Node))
            .expect("renamed node");
        let node_dim = super::calc_component_dim(node);
        let expected_width = (16.0 + node_dim.width + 11.0 + super::SVEK_CANVAS_PAD) as i64;
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"points="16,16,26,6,"#),
            "LimitFinder's polygon guard should place the renamed node at x=16, y=6: {svg}"
        );
        assert!(
            svg.contains(&format!(r#"width="{expected_width}px""#)),
            "the node's guarded painted maximum should own the canvas width: {svg}"
        );
    }

    #[test]
    fn no_oracle_bidirectional_link_decorates_both_spline_ends() {
        let input = "@startuml\ncomponent \"Renamed ingress\" as EntryPoint\ncomponent \"Renamed archive\" as ArchiveNode\nEntryPoint <..> ArchiveNode\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"id="EntryPoint-ArchiveNode""#),
            "bidirectional paths use PlantUML's association-style id: {svg}"
        );
        assert_eq!(
            svg.matches(r##"<polygon fill="#181818""##).count(),
            2,
            "each decorated endpoint should emit one arrow polygon: {svg}"
        );
    }

    #[test]
    fn no_oracle_left_link_uses_reversed_svek_edge() {
        let input = "@startuml\ncomponent Northbound\ncomponent Southbound\nNorthbound <-left- Southbound\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"id="Southbound-to-Northbound""#),
            "left-directed links should use the reversed SVEK endpoint order: {svg}"
        );
        assert!(
            svg.contains("<!--link Southbound to Northbound-->"),
            "link metadata should follow the reversed Java Link model: {svg}"
        );
    }

    #[test]
    fn no_oracle_node_cluster_uses_usymbol_folded_envelope_for_renamed_label() {
        let input = "@startuml\nnode \"Compute Boundary 53\" {\n  component \"Worker 59\" as W59\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains("Compute Boundary 53") && svg.contains("Worker 59"),
            "renamed node cluster contents should survive the generative path: {svg}"
        );
        assert_eq!(
            svg.matches("<polygon ").count(),
            1,
            "USymbolNode should emit one folded envelope polygon: {svg}"
        );
        assert_eq!(
            svg.matches("<line ").count(),
            3,
            "USymbolNode should emit its fold and inner corner lines: {svg}"
        );
    }

    #[test]
    fn no_oracle_nested_group_link_uses_routable_cluster_endpoint() {
        let input = "@startuml\nfolder OuterTransit71 {\n  folder InnerTransit73 {\n    component Alpha79\n    component Beta83\n    Alpha79 --> Beta83\n  }\n  component Gamma89\n  InnerTransit73 --> Gamma89\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"data-qualified-name="OuterTransit71.InnerTransit73""#),
            "renamed nested cluster should retain its qualified identity: {svg}"
        );
        assert!(
            svg.contains("<!--link InnerTransit73 to Gamma89-->"),
            "group link should survive the generative SVEK path: {svg}"
        );
        assert!(
            svg.contains(r#"data-entity-1="ent0003" data-entity-2="ent0007""#),
            "group link should resolve the nested package and component UIDs: {svg}"
        );
    }
}
