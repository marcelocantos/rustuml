// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's component diagram rendering.
//! PlantUML renders component diagrams as diagram type "DESCRIPTION".

use std::fmt::Write;

use rustuml_layout::graph::{
    ClusterPosition, ClusterTitleSize, Direction, EdgeLabelSize, EdgePath, EdgePorts, GraphSpacing,
    LayoutGraph,
};
use rustuml_parser::diagram::component::*;
use rustuml_parser::diagram::style::StyleScheme;
use rustuml_parser::diagram::{LegendHorizontalAlignment, LegendVerticalAlignment};

use crate::layout_oracle::{
    CrowMark, EntityRect, OracleLayout, emit_oracle_cluster_children, emit_oracle_note_entity,
    wrap_oracle_envelope,
};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::style_cascade::{ResolvedStyle, StyleBoxSides, StyleCascade, StyleSignature};
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

    fn dash(self) -> Option<(f64, f64)> {
        match self {
            Self::Solid => None,
            Self::Dashed => Some((7.0, 7.0)),
            // `FromSkinparamToStyle.convertNow` rewrites dotted to `1;3`,
            // then its complex-value branch retains only `1`;
            // `Style#getStroke` duplicates that lone token.
            Self::Dotted => Some((1.0, 1.0)),
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

#[derive(Clone)]
struct ComponentEntityRenderStyle {
    fill: String,
    stroke: String,
    stroke_width: f64,
    dash: Option<(f64, f64)>,
    round_corner: f64,
    shadow: f64,
    font_color: String,
    font_family: String,
    font_size: f64,
    font_bold: bool,
    font_italic: bool,
}

#[derive(Clone)]
struct ComponentLinkRenderStyle {
    stroke: String,
    stroke_width: f64,
    dash: Option<(f64, f64)>,
    font_color: String,
    font_family: String,
    font_size: f64,
}

#[derive(Debug, PartialEq, Eq)]
struct ComponentGradient {
    color1: String,
    color2: String,
    policy: char,
    id: String,
}

fn register_component_gradient(source: &str, gradients: &mut Vec<ComponentGradient>, value: &str) {
    let Some((raw1, raw2, policy)) = crate::sequence::split_gradient_colors(value) else {
        return;
    };
    let color1 = crate::sequence::resolve_color(raw1);
    let color2 = crate::sequence::resolve_color(raw2);
    if gradients.iter().any(|gradient| {
        gradient.color1 == color1 && gradient.color2 == color2 && gradient.policy == policy
    }) {
        return;
    }
    gradients.push(ComponentGradient {
        color1,
        color2,
        policy,
        id: crate::filter_registry::gradient_id_for(source, gradients.len()),
    });
}

fn component_gradients(diagram: &ComponentDiagram) -> Vec<ComponentGradient> {
    let source = diagram.meta.source.as_deref().unwrap_or("");
    let mut gradients = Vec::new();

    // Java preserves gradient values through style resolution and registers
    // only the tuple selected when an entity is painted. Inventory every
    // background declaration here; `finalize_painted_resources` removes
    // unselected tuples and imposes first-paint order on the retained set.
    for skinparam in &diagram.meta.skinparams {
        let key = skinparam.key.to_ascii_lowercase();
        if key == "backgroundcolor" || key.ends_with("backgroundcolor") {
            register_component_gradient(source, &mut gradients, &skinparam.value);
        }
    }
    for declaration in &diagram.meta.style_program.declarations {
        if declaration.property.eq_ignore_ascii_case("backgroundcolor") {
            register_component_gradient(source, &mut gradients, &declaration.value);
        }
    }
    for component in &diagram.components {
        if let Some(color) = component.color.as_deref() {
            register_component_gradient(source, &mut gradients, color);
        }
    }

    gradients
}

fn component_gradient_defs(gradients: &[ComponentGradient]) -> String {
    let mut defs = String::new();
    for gradient in gradients {
        let (x1, x2, y1, y2) = crate::sequence::gradient_endpoints(gradient.policy);
        write!(
            defs,
            r#"<linearGradient id="{}" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient>"#,
            gradient.id, gradient.color1, gradient.color2,
        )
        .unwrap();
    }
    defs
}

fn component_dash_suffix(dash: Option<(f64, f64)>) -> String {
    dash.map(|(visible, space)| {
        format!(
            "stroke-dasharray:{},{};",
            pm::fmt_coord(visible),
            pm::fmt_coord(space)
        )
    })
    .unwrap_or_default()
}

fn component_element_style_name(kind: ComponentElementKind) -> &'static str {
    match kind {
        ComponentElementKind::Component => "component",
        ComponentElementKind::Actor => "actor",
        ComponentElementKind::Artifact => "artifact",
        ComponentElementKind::Collections => "collections",
        ComponentElementKind::Database => "database",
        ComponentElementKind::Node => "node",
        ComponentElementKind::Queue => "queue",
        ComponentElementKind::Storage => "storage",
        ComponentElementKind::Cloud => "cloud",
    }
}

fn component_shadow_value(value: &str) -> f64 {
    // Java `Style#getShadowing` delegates non-numeric values to
    // `Value#asDoubleDefaultTo(1.5)`; zero and negative deltas do not paint a
    // shadow in `SvgGraphics#addFilterShadowId`.
    match value.trim().to_ascii_lowercase().as_str() {
        "false" | "no" => 0.0,
        "true" | "yes" => 1.5,
        _ => value.trim().parse::<f64>().unwrap_or(1.5).max(0.0),
    }
}

fn apply_component_entity_style(
    target: &mut ComponentEntityRenderStyle,
    resolved: &ResolvedStyle<'_>,
    gradient_defs: Option<&str>,
) {
    // Java `EntityImageDescription` resolves title paint, stroke, shadow, and
    // font from the concrete symbol signature against
    // `Entity#getCurrentStyleBuilder`.
    if let Some(value) = resolved.property("backgroundColor") {
        target.fill = crate::sequence::gradient_fill_or(value, gradient_defs);
    }
    if let Some(value) = resolved.property("lineColor") {
        target.stroke = crate::sequence::resolve_color(value);
    }
    let stroke = resolved.stroke(target.stroke_width);
    target.stroke_width = stroke.thickness;
    if resolved.property("lineStyle").is_some() {
        target.dash = stroke.dash;
    }
    if let Some(value) = resolved.property("roundCorner")
        && let Ok(value) = value.parse::<f64>()
    {
        // `EntityImageDescription` passes the diameter to
        // `URectangle#rounded`; SVG serializes its half-radius.
        target.round_corner = value / 2.0;
    }
    if let Some(value) = resolved.property("shadowing") {
        target.shadow = component_shadow_value(value);
    }
    if let Some(value) = resolved.property("fontColor") {
        target.font_color = crate::sequence::resolve_color(value);
    }
    if let Some(value) = resolved.property("fontName") {
        target.font_family = canonical_font_family(value);
    }
    if let Some(value) = resolved.property("fontSize")
        && let Ok(value) = value.parse::<f64>()
    {
        target.font_size = value;
    }
    if let Some(value) = resolved.property("fontStyle") {
        let value = value.to_ascii_lowercase();
        target.font_bold = value.contains("bold");
        target.font_italic = value.contains("italic");
    }
}

fn apply_component_link_style(target: &mut ComponentLinkRenderStyle, resolved: &ResolvedStyle<'_>) {
    // Java `SvekEdge#getCurrentStyleBuilder` returns `Link#getStyleBuilder`;
    // line, head, and label channels therefore share the link's creation
    // snapshot even after a legacy skinparam refreshes entities.
    if let Some(value) = resolved.property("lineColor") {
        target.stroke = crate::sequence::resolve_color(value);
    }
    let stroke = resolved.stroke(target.stroke_width);
    target.stroke_width = stroke.thickness;
    if resolved.property("lineStyle").is_some() {
        target.dash = stroke.dash;
    }
    if let Some(value) = resolved.property("fontColor") {
        target.font_color = crate::sequence::resolve_color(value);
    }
    if let Some(value) = resolved.property("fontName") {
        target.font_family = canonical_font_family(value);
    }
    if let Some(value) = resolved.property("fontSize")
        && let Ok(value) = value.parse::<f64>()
    {
        target.font_size = value;
    }
}

fn component_stereotype_skinparam<'a>(key: &'a str, property: &str) -> Option<&'a str> {
    key.strip_prefix(property)?
        .strip_prefix("<<")?
        .strip_suffix(">>")
        .filter(|stereotype| !stereotype.is_empty())
}

fn skinparam_stereotype_identity(stereotype: &str) -> String {
    stereotype.to_ascii_lowercase().replace(['_', '.'], "")
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

fn component_svek_owner<'a>(
    id: &str,
    qualified_names: &'a std::collections::HashMap<String, String>,
    package_qualified_names: &'a std::collections::HashMap<String, String>,
) -> Option<&'a str> {
    qualified_names
        .get(id)
        .and_then(|qualified| qualified.rsplit_once('.').map(|(owner, _)| owner))
        .or_else(|| package_qualified_names.get(id).map(String::as_str))
}

fn horizontal_link_crosses_component_cluster_scope(
    connection: &Connection,
    qualified_names: &std::collections::HashMap<String, String>,
    package_qualified_names: &std::collections::HashMap<String, String>,
) -> bool {
    if !matches!(
        connection.direction,
        Some(ConnectionDirection::Left | ConnectionDirection::Right)
    ) {
        return false;
    }
    let from_owner =
        component_svek_owner(&connection.from, qualified_names, package_qualified_names);
    let to_owner = component_svek_owner(&connection.to, qualified_names, package_qualified_names);
    from_owner != to_owner && (from_owner.is_some() || to_owner.is_some())
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
        NamedNote {
            index: usize,
            id: String,
        },
        RemovedAttachedNote,
        Link {
            index: usize,
            consumes_inverse: bool,
        },
        RemovedLink {
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
    let named_note_ids = component_named_note_ids(diagram);
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
    for component in &diagram.removed_components {
        events.push((
            component.source_line,
            ordinal,
            Event::Entity(vec![component.id.clone()]),
        ));
        ordinal += 1;
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
        } else if let Some(id) = named_note_ids.get(&index) {
            events.push((
                note.source_line,
                ordinal,
                Event::NamedNote {
                    index,
                    id: id.clone(),
                },
            ));
            ordinal += 1;
        }
    }
    for note in &diagram.removed_notes {
        if note.target.is_some() {
            events.push((note.source_line, ordinal, Event::RemovedAttachedNote));
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
    for connection in &diagram.removed_connections {
        events.push((
            connection.source_line,
            ordinal,
            Event::RemovedLink {
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
            Event::NamedNote { index, id } => {
                // `CommandFactoryNote.executeArg` creates the explicitly
                // named note entity directly; its link consumes the next UID.
                let entity_id = format!("ent{next_uid:04}");
                entity_ids.insert(id.clone(), entity_id.clone());
                note_ids.insert(
                    index,
                    NoOracleNoteUid {
                        qualified_name: id,
                        entity_id,
                        link_id: 0,
                    },
                );
                next_uid += 1;
            }
            Event::RemovedAttachedNote => {
                // The generated GMN name, note entity, and hidden attachment
                // link are allocated while parsing, before removal is applied.
                next_uid += 3;
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
            Event::RemovedLink { consumes_inverse } => {
                if consumes_inverse {
                    next_uid += 1;
                }
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
        // `CommandCreateElementFull.executeArg` reuses the first quark-backed
        // Entity, whose parent cannot change on a compatible redeclaration.
        map.entry(cid.clone())
            .or_insert_with(|| format!("{path}.{cid}"));
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
/// Trailing width retained by the SVG envelope outside decorated content.
const CHROME_RIGHT_PAD: f64 = 7.0;
/// `DisplayPositioned.createRibbon` adds one pixel below caption text.
const CAPTION_BOTTOM_PAD: f64 = 1.0;
// `EntityImageLegend.create` merges the document legend style from
// `plantuml.skin`: 5px/7px table padding, 12px outer margin, 15px round
// corner, and 1.5 font descents of horizontal cell padding.
const LEGEND_FONT_SIZE: f64 = 14.0;
const LEGEND_RECT_PAD_X: f64 = 5.0;
const LEGEND_RECT_PAD_Y: f64 = 7.0;
const LEGEND_OUTER_MARGIN: f64 = 12.0;
const LEGEND_RECT_RX: f64 = 7.5;
const LEGEND_CELL_PAD_DESCENT_FACTOR: f64 = 1.5;
// `TextBlockBordered.calculateDimension` adds one pixel below the drawn
// legend rectangle when `DecorateEntityImage` stacks the bottom band.
const LEGEND_BORDERED_HEIGHT_DELTA: f64 = 1.0;
// `SvekResult.calculateDimension` normalizes its painted top-left to (6, 6).
const SVEK_ENVELOPE_ORIGIN: f64 = 6.0;
/// Font size for arrow/link labels.
const LINK_FONT: f64 = 13.0;
// Java `SvekEdge.addVisibilityModifier` wraps center labels with
// `TextBlockUtils.withMargin(block, 1, 1)`. `TextBlockMarged.drawU` then
// paints a full-size `UEmpty`, so the margin participates in SVEK bounds.
const LINK_LABEL_MARGIN: f64 = 1.0;
// `SvekEdge` reserves this shield around every non-NONE
// `LinkMiddleDecor` before it asks Graphviz to place the center label.
const MIDDLE_LABEL_SHIELD: f64 = 7.0;
// `SvekEdge.addVisibilityModifier` expands an autolink label by six pixels on
// every side because the label shares the loop's compact routing envelope.
const SELF_LINK_LABEL_MARGIN: f64 = 6.0;
// `SvekEdge.appendTable` truncates a 13-point one-line label plus its margins
// to 17px; Graphviz's solved same-rank HTML box is 20px high.
const HORIZONTAL_LINK_LABEL_SOLVED_HEIGHT: f64 = 20.0;
/// Line height per text line in a component box.
const LINE_HEIGHT: f64 = 16.4883;
// Java `USymbolComponent2.getMargin()` contributes 20px above the merged
// stereotype/label block and 10px below it.
const COMPONENT_MARGIN_TOP: f64 = 20.0;
const COMPONENT_MARGIN_BOTTOM: f64 = 10.0;
/// Base component box height (padding around one line of text).
const COMPONENT_BASE_H: f64 = COMPONENT_MARGIN_TOP + COMPONENT_MARGIN_BOTTOM;
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
// `USymbolQueue.getMargin()` returns (5, 15, 5, 5).
const QUEUE_MARGIN_LEFT: f64 = 5.0;
const QUEUE_MARGIN_RIGHT: f64 = 15.0;
const QUEUE_MARGIN_Y: f64 = 5.0;
// `USymbolStorage.getMargin()` returns ten pixels on every side and
// `drawStorage()` applies `URectangle.rounded(70)`.
const STORAGE_MARGIN: f64 = 10.0;
const STORAGE_RADIUS: f64 = 35.0;
// Java `USymbolArtifact.getMargin()` returns
// `new Margin(10, 20, 13, 10)`.
const ARTIFACT_MARGIN_LEFT: f64 = 10.0;
const ARTIFACT_MARGIN_RIGHT: f64 = 20.0;
const ARTIFACT_MARGIN_TOP: f64 = 13.0;
const ARTIFACT_MARGIN_BOTTOM: f64 = 10.0;
// `SvekNode.appendShape` submits the artifact as a non-fixed, empty-label
// Graphviz rectangle. Fresh renamed Java renders show that its solved graph
// envelope extends six pixels beyond the painted `USymbolArtifact` block.
const ARTIFACT_GRAPH_ENVELOPE_RIGHT: f64 = 6.0;
// `USymbolDatabase.drawDatabase()` appends `UEmpty(10, 10)` at (width, height),
// extending the rendered envelope without changing the Graphviz node size.
// `LimitFinder` includes the UEmpty origin on X and its ten-pixel extent on Y.
const DATABASE_RENDER_OVERFLOW_X: f64 = 11.0;
const DATABASE_RENDER_OVERFLOW_Y: f64 = 10.0;
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
// Java `EntityImageClass` uses this fixed header/body envelope for an
// interface rendered inside a component group.
const INTERFACE_CLASS_BOX_HEIGHT: f64 = 48.0;
const INTERFACE_CLASS_BOX_WIDTH_PAD: f64 = 32.0;
// `SvekNode`'s `RECTANGLE_WITH_CIRCLE_INSIDE` HTML cell contributes eight
// transparent pixels per side that `LimitFinder` excludes from the cluster.
const INTERFACE_CLASS_CLUSTER_WIDTH_TRIM: f64 = 16.0;
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
// Java provenance: `EntityImageNoteLink` delegates to `ComponentRoseNote`,
// whose preferred size adds Rose's five-pixel padding on every side.
const LINK_NOTE_PADDING: f64 = 5.0;
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

#[derive(Clone, Copy, Debug)]
struct ComponentLabelRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct ComponentEndpointLabelLayout {
    tail: Option<ComponentLabelRect>,
    head: Option<ComponentLabelRect>,
}

fn component_note_layout_id(
    index: usize,
    named_note_ids: &std::collections::HashMap<usize, String>,
) -> String {
    named_note_ids
        .get(&index)
        .cloned()
        .unwrap_or_else(|| format!("__component_note_{index}"))
}

fn component_named_note_ids(
    diagram: &ComponentDiagram,
) -> std::collections::HashMap<usize, String> {
    let qualified_components = build_qualified_names(&diagram.packages);
    let package_names = build_package_qualified_names(&diagram.packages);
    let known_endpoint = |id: &str| {
        diagram
            .components
            .iter()
            .any(|component| component.id == id)
            || diagram
                .interfaces
                .iter()
                .any(|interface| interface.id == id)
            || qualified_components.contains_key(id)
            || package_names.contains_key(id)
    };

    let mut pending_notes = Vec::new();
    let mut assigned = std::collections::HashMap::new();
    let mut events = Vec::new();
    for (index, note) in diagram.notes.iter().enumerate() {
        if note.target.is_none() && note.connection.is_none() {
            events.push((note.source_line, 0_u8, index, None));
        }
    }
    for (index, connection) in diagram.connections.iter().enumerate() {
        events.push((connection.source_line, 1_u8, index, Some(connection)));
    }
    events.sort_by_key(|(line, kind, index, _)| (*line, *kind, *index));

    for (_, kind, index, connection) in events {
        if kind == 0 {
            pending_notes.push(index);
            continue;
        }
        let Some(connection) = connection else {
            continue;
        };
        let unknown = [&connection.from, &connection.to]
            .into_iter()
            .filter(|endpoint| !known_endpoint(endpoint))
            .find(|endpoint| !assigned.values().any(|id| id == *endpoint));
        let Some(id) = unknown else {
            continue;
        };
        let Some(note_index) = pending_notes.pop() else {
            continue;
        };
        // `CommandFactoryNote.executeArg` creates the named note entity in the
        // source stream; the later link resolves that otherwise-unknown id.
        assigned.insert(note_index, id.clone());
    }
    assigned
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

fn component_note_on_connection(
    diagram: &ComponentDiagram,
    connection: usize,
) -> Option<(usize, &ComponentNote)> {
    diagram
        .notes
        .iter()
        .enumerate()
        .find(|(_, note)| note.connection == Some(connection))
}

fn component_opalized_floating_note<'a>(
    diagram: &'a ComponentDiagram,
    named_note_ids: &std::collections::HashMap<usize, String>,
    connection_index: usize,
) -> Option<(usize, &'a ComponentNote)> {
    let connection = diagram.connections.get(connection_index)?;
    diagram.notes.iter().enumerate().find(|(index, _)| {
        let Some(id) = named_note_ids.get(index) else {
            return false;
        };
        if connection.from != *id && connection.to != *id {
            return false;
        }
        // `GraphvizImageBuilder.isOpalisable` folds a named note's connector
        // into its outline only when exactly one SVEK edge touches the note.
        diagram
            .connections
            .iter()
            .filter(|candidate| candidate.from == *id || candidate.to == *id)
            .count()
            == 1
    })
}

fn component_link_note_label_size(
    note: &ComponentNote,
    note_dim: &CompDim,
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

    match note.position {
        ComponentNotePosition::Left | ComponentNotePosition::Right => EdgeLabelSize {
            width: note_width + label_size.width,
            height: note_height.max(label_size.height),
        },
        ComponentNotePosition::Top | ComponentNotePosition::Bottom => EdgeLabelSize {
            width: note_width.max(label_size.width),
            height: note_height + label_size.height,
        },
    }
}

fn component_link_note_blocks(
    x: f64,
    y: f64,
    note: &ComponentNote,
    note_dim: &CompDim,
    label_size: Option<EdgeLabelSize>,
) -> (Option<(f64, f64)>, (f64, f64)) {
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let Some(label_size) = label_size else {
        return (None, (x + LINK_NOTE_PADDING, y + LINK_NOTE_PADDING));
    };

    let (label_origin, note_origin) = match note.position {
        ComponentNotePosition::Left => {
            let merged_height = note_height.max(label_size.height);
            (
                (
                    x + note_width,
                    y + (merged_height - label_size.height) / 2.0,
                ),
                (x, y + (merged_height - note_height) / 2.0),
            )
        }
        ComponentNotePosition::Right => {
            let merged_height = note_height.max(label_size.height);
            (
                (x, y + (merged_height - label_size.height) / 2.0),
                (
                    x + label_size.width,
                    y + (merged_height - note_height) / 2.0,
                ),
            )
        }
        ComponentNotePosition::Top => {
            let merged_width = note_width.max(label_size.width);
            (
                (x + (merged_width - label_size.width) / 2.0, y + note_height),
                (x + (merged_width - note_width) / 2.0, y),
            )
        }
        ComponentNotePosition::Bottom => {
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

fn laid_out_note_indices(
    diagram: &ComponentDiagram,
    package_qualified_names: &std::collections::HashMap<String, String>,
    named_note_ids: &std::collections::HashMap<usize, String>,
) -> Vec<usize> {
    diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| {
            if named_note_ids.contains_key(&index) {
                return Some(index);
            }
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
// `SvekEdge.manageCollision` expands each SVEK node by eight pixels before
// moving intersecting endpoint labels through `PositionableUtils`.
const SVEK_ENDPOINT_COLLISION_MARGIN: f64 = 8.0;

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
    let hidden_components: std::collections::HashSet<&str> = diagram
        .hidden_components
        .iter()
        .map(String::as_str)
        .collect();

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
    let mut component_arrow_stroke_width = 1.0;
    let mut component_arrow_line_style = ComponentLineStyle::Solid;
    let mut component_padding = 0.0;
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
    let generated_gradients = oracle.is_none().then(|| component_gradients(diagram));
    let generated_gradient_defs = generated_gradients
        .as_deref()
        .map(component_gradient_defs)
        .filter(|defs| !defs.is_empty());
    let gradient_defs = oracle
        .map(|o| o.defs_inner_xml.as_str())
        .filter(|d| !d.is_empty())
        .or(generated_gradient_defs.as_deref());
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
                .entry(skinparam_stereotype_identity(stereotype))
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
            "padding" => {
                component_padding = val.parse::<f64>().unwrap_or(component_padding);
            }
            _ => {}
        }
    }
    let cascade = StyleCascade::new(&diagram.meta.style_program);
    let component_signature =
        StyleSignature::from_selectors(["root", "element", "componentDiagram", "component"]);
    let arrow_signature =
        StyleSignature::from_selectors(["root", "element", "componentDiagram", "arrow"]);
    let document_signature = StyleSignature::from_selectors(["root", "document"]);
    let component_style = cascade.resolve(&component_signature, StyleScheme::Regular);
    let arrow_style = cascade.resolve(&arrow_signature, StyleScheme::Regular);
    let document_style = cascade.resolve(&document_signature, StyleScheme::Regular);
    // Java `TextBlockExporter12026.Builder#calculateMargin` resolves
    // `root.document` through the final skin builder, then
    // `ClockwiseTopRightBottomLeft#read` preserves all four sides.
    let component_document_margin = document_style.box_sides("margin").unwrap_or_default();
    if let Some(value) = component_style.property("backgroundColor") {
        component_fill = crate::sequence::gradient_fill_or(value, gradient_defs);
    }
    if let Some(value) = component_style.property("lineColor") {
        component_stroke = crate::sequence::resolve_color(value);
    }
    if let Some(value) = component_style.property("lineThickness")
        && let Ok(value) = value.parse::<f64>()
    {
        component_stroke_width = value;
    }
    if let Some(value) = component_style.property("roundCorner")
        && let Ok(value) = value.parse::<f64>()
    {
        // Java `EntityImageDescription` passes the corner diameter through
        // `URectangle.rounded`; the SVG rectangle stores half as its radius.
        component_round_corner = Some(value / 2.0);
    }
    if let Some(value) = component_style.property("lineStyle") {
        component_line_style =
            ComponentLineStyle::from_skinparam(value).unwrap_or(component_line_style);
    }
    if let Some(value) = component_style.property("fontColor") {
        component_font_color_sp = Some(crate::sequence::resolve_color(value));
    }
    if let Some(value) = component_style.property("fontName") {
        component_font_family_sp = Some(canonical_font_family(value));
    }
    if let Some(value) = component_style.property("fontSize") {
        component_font_size_sp = value.parse::<f64>().ok();
    }
    if let Some(value) = component_style.property("fontStyle") {
        let value = value.to_ascii_lowercase();
        component_font_bold = value.contains("bold");
        component_font_italic = value.contains("italic");
    }
    if let Some(value) = arrow_style.property("lineColor") {
        component_arrow_stroke = crate::sequence::resolve_color(value);
    }
    if let Some(value) = arrow_style.property("lineThickness")
        && let Ok(value) = value.parse::<f64>()
    {
        component_arrow_stroke_width = value;
    }
    if let Some(value) = arrow_style.property("lineStyle") {
        component_arrow_line_style =
            ComponentLineStyle::from_skinparam(value).unwrap_or(component_arrow_line_style);
    }
    if let Some(value) = arrow_style.property("fontColor") {
        component_arrow_font_color_sp = Some(crate::sequence::resolve_color(value));
    }
    if let Some(value) = arrow_style.property("fontName") {
        component_arrow_font_family_sp = Some(canonical_font_family(value));
    }
    if let Some(value) = arrow_style.property("fontSize") {
        component_arrow_font_size_sp = value.parse::<f64>().ok();
    }
    if let Some(value) = document_style.property("backgroundColor") {
        bg_value = Some(value.to_string());
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

    let component_entity_styles: Vec<ComponentEntityRenderStyle> = diagram
        .components
        .iter()
        .map(|component| {
            let stereotype_style = matches!(component.kind, ComponentElementKind::Component)
                .then(|| {
                    component.stereotypes.iter().find_map(|stereotype| {
                        component_stereotype_styles.get(&skinparam_stereotype_identity(stereotype))
                    })
                })
                .flatten();
            // `Component#source_line` uses serde's zero default only for
            // legacy serialized models; parser-created entities are 1-based.
            // Preserve the former final-style behavior for that compatibility
            // sentinel without changing valid-source snapshot ownership.
            let use_final_fallback = component.source_line == 0;
            let mut style = ComponentEntityRenderStyle {
                fill: stereotype_style
                    .and_then(|style| style.fill.clone())
                    .unwrap_or_else(|| {
                        if use_final_fallback {
                            component_fill.clone()
                        } else {
                            COMP_FILL.to_string()
                        }
                    }),
                stroke: stereotype_style
                    .and_then(|style| style.stroke.clone())
                    .unwrap_or_else(|| {
                        if use_final_fallback {
                            component_stroke.clone()
                        } else {
                            STROKE.to_string()
                        }
                    }),
                stroke_width: stereotype_style
                    .and_then(|style| style.stroke_width)
                    .unwrap_or(if use_final_fallback {
                        component_stroke_width
                    } else {
                        0.5
                    }),
                dash: stereotype_style
                    .and_then(|style| style.line_style)
                    .unwrap_or(if use_final_fallback {
                        component_line_style
                    } else {
                        ComponentLineStyle::Solid
                    })
                    .dash(),
                round_corner: stereotype_style
                    .and_then(|style| style.round_corner)
                    .or(if use_final_fallback {
                        component_round_corner
                    } else {
                        None
                    })
                    .unwrap_or(ROUND_R),
                shadow: 0.0,
                font_color: if use_final_fallback {
                    component_font_color.clone()
                } else {
                    TEXT_COLOR.to_string()
                },
                font_family: if use_final_fallback {
                    component_font_family.clone()
                } else {
                    "sans-serif".to_string()
                },
                font_size: if use_final_fallback {
                    component_font_size
                } else {
                    FONT_SIZE
                },
                font_bold: use_final_fallback && component_font_bold,
                font_italic: use_final_fallback && component_font_italic,
            };
            // `EntityImageDescription` resolves the concrete `USymbol`
            // signature. A database declared in a component diagram therefore
            // consumes `database`, not the sibling `component` style.
            let base_signature = StyleSignature::from_selectors([
                "root",
                "element",
                "componentDiagram",
                component_element_style_name(component.kind),
            ]);
            let signature = component
                .stereotypes
                .iter()
                .fold(base_signature, |signature, stereotype| {
                    signature.with_stereotype(stereotype)
                });
            // `EntityImageDescription` asks the entity for its builder at
            // consumption time. `Entity#getCurrentStyleBuilder` preserves a
            // pure-CSS creation snapshot, but refreshes every entity to the
            // final builder after any legacy skinparam command.
            let resolved = if component.source_line == 0 {
                cascade.resolve(&signature, StyleScheme::Regular)
            } else {
                cascade.resolve_entity_at_source_line(
                    &signature,
                    StyleScheme::Regular,
                    component.source_line,
                )
            };
            apply_component_entity_style(&mut style, &resolved, gradient_defs);
            style
        })
        .collect();
    let component_link_styles: Vec<ComponentLinkRenderStyle> = diagram
        .connections
        .iter()
        .map(|connection| {
            // `Connection#source_line` has the same zero-only serde
            // compatibility sentinel as component entities.
            let use_final_fallback = connection.source_line == 0;
            let mut style = ComponentLinkRenderStyle {
                stroke: if use_final_fallback {
                    component_arrow_stroke.clone()
                } else {
                    STROKE.to_string()
                },
                stroke_width: if use_final_fallback {
                    component_arrow_stroke_width
                } else {
                    1.0
                },
                dash: if use_final_fallback {
                    component_arrow_line_style.dash()
                } else {
                    None
                },
                font_color: if use_final_fallback {
                    component_arrow_font_color.clone()
                } else {
                    TEXT_COLOR.to_string()
                },
                font_family: if use_final_fallback {
                    component_arrow_font_family.clone()
                } else {
                    "sans-serif".to_string()
                },
                font_size: if use_final_fallback {
                    component_arrow_font_size
                } else {
                    LINK_FONT
                },
            };
            // `Link#getStyleBuilder` always returns the builder captured by
            // the link constructor; unlike entities, legacy skinparams do not
            // refresh an already-created link.
            let resolved = if connection.source_line == 0 {
                cascade.resolve(&arrow_signature, StyleScheme::Regular)
            } else {
                cascade.resolve_link_at_source_line(
                    &arrow_signature,
                    StyleScheme::Regular,
                    connection.source_line,
                )
            };
            apply_component_link_style(&mut style, &resolved);
            style
        })
        .collect();
    let component_shadows: Vec<f64> = component_entity_styles
        .iter()
        .map(|style| style.shadow)
        .collect();

    // Compute the same merged stereotype/label block that
    // `EntityImageDescription` passes to `USymbolComponent2.asSmall`.
    let component_text_metrics: Vec<ComponentTextMetrics> = diagram
        .components
        .iter()
        .zip(&component_entity_styles)
        .map(|(component, style)| {
            component_text_metrics(
                component,
                style.font_size,
                &style.font_family,
                style.font_bold,
            )
        })
        .collect();
    let comp_dims: Vec<CompDim> = diagram
        .components
        .iter()
        .zip(&component_text_metrics)
        .map(|(component, metrics)| {
            let mut dim =
                calc_component_dim_with_symbol_style(component, metrics, component_style_rectangle);
            dim.width += component_padding * 2.0;
            dim.height += component_padding * 2.0;
            dim
        })
        .collect();
    let note_dims: Vec<CompDim> = diagram.notes.iter().map(component_note_dim).collect();
    let qualified_names = build_qualified_names(&diagram.packages);
    let package_qualified_names = build_package_qualified_names(&diagram.packages);
    let named_note_ids = component_named_note_ids(diagram);
    let laid_out_note_indices =
        laid_out_note_indices(diagram, &package_qualified_names, &named_note_ids);
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
    let (component_node_sep, short_label_compat) =
        component_no_oracle_spacing(diagram, &component_link_styles);

    // Try Sugiyama layout (skip when oracle is available).
    let layout_result = if use_oracle {
        None
    } else if !diagram.components.is_empty() || !diagram.interfaces.is_empty() {
        // Java provenance: `AbstractEntityDiagram.getRankdir()` carries
        // `left to right direction` into SVEK's DOT `rankdir`.
        let layout_direction = match diagram.direction {
            ComponentLayoutDirection::TopToBottom => Direction::TopToBottom,
            ComponentLayoutDirection::LeftToRight => Direction::LeftToRight,
        };
        let mut layout = LayoutGraph::new(layout_direction)
            .with_spacing_pixels(
                component_node_sep,
                GraphSpacing::PLANTUML_SVEK_DEFAULTS.rank_sep_px,
            )
            .with_plantuml_svek_node_order();
        for (component, dim) in diagram.components.iter().zip(&comp_dims) {
            layout.add_node(&component.id, &component.label, dim.width, dim.height);
        }
        for interface in &diagram.interfaces {
            if component_interface_uses_class_box(diagram, interface) {
                let dim = component_interface_class_dim(interface);
                layout.add_node(&interface.id, &interface.label, dim.width, dim.height);
            } else if let Some((shield_x, shield_y)) =
                component_interface_shield(diagram, interface)
            {
                layout.add_svek_shielded_node(
                    &interface.id,
                    IFACE_NODE_SIZE,
                    IFACE_NODE_SIZE,
                    shield_x,
                    shield_y,
                );
            } else {
                layout.add_node(
                    &interface.id,
                    &interface.label,
                    IFACE_NODE_SIZE,
                    IFACE_NODE_SIZE,
                );
            }
        }
        for &note_index in &laid_out_note_indices {
            let dim = &note_dims[note_index];
            layout.add_node(
                &component_note_layout_id(note_index, &named_note_ids),
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
        add_component_single_strategy_to_layout(&mut layout, diagram);
        for &note_index in &laid_out_note_indices {
            let note = &diagram.notes[note_index];
            let Some(target) = note.target.as_deref() else {
                continue;
            };
            let note_id = component_note_layout_id(note_index, &named_note_ids);
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
                // Java `Bibliotekon.lines0` serializes every one-rank hidden
                // note link before ordinary nodes.
                layout.add_plantuml_svek_line0_edge(from, to);
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
        for (connection_index, conn) in diagram.connections.iter().enumerate() {
            let link_style = &component_link_styles[connection_index];
            let (logical_from, logical_to, layout_reversed) = no_oracle_layout_edge_ends(conn);
            let class_socket_reversed = component_class_socket_link(diagram, conn);
            let (layout_logical_from, layout_logical_to) = if class_socket_reversed {
                (logical_to, logical_from)
            } else {
                (logical_from, logical_to)
            };
            let horizontal = matches!(
                conn.direction,
                Some(ConnectionDirection::Left | ConnectionDirection::Right)
            );
            let horizontal_crosses_cluster_scope = horizontal_link_crosses_component_cluster_scope(
                conn,
                &qualified_names,
                &package_qualified_names,
            );
            let horizontal_plain_interface = horizontal
                && matches!(conn.shape, LinkShape::Plain)
                && diagram
                    .interfaces
                    .iter()
                    .any(|interface| interface.id == conn.from || interface.id == conn.to);
            let horizontal_socket_same_owner = horizontal
                && matches!(
                    conn.shape,
                    LinkShape::TargetSocket | LinkShape::TargetBallSocket
                )
                && component_svek_owner(&conn.from, &qualified_names, &package_qualified_names)
                    .is_some_and(|owner| {
                        Some(owner)
                            == component_svek_owner(
                                &conn.to,
                                &qualified_names,
                                &package_qualified_names,
                            )
                    });
            let layout_from = package_qualified_names
                .get(layout_logical_from)
                .and_then(|qname| group_endpoint_node_map.get(qname))
                .map(String::as_str)
                .unwrap_or(layout_logical_from);
            let layout_to = package_qualified_names
                .get(layout_logical_to)
                .and_then(|qname| group_endpoint_node_map.get(qname))
                .map(String::as_str)
                .unwrap_or(layout_logical_to);
            if conn.length == 1 {
                // `Bibliotekon.addLine` places every one-rank link in
                // `lines0`; the edge therefore participates in node order.
                layout.add_plantuml_svek_line0_edge(layout_from, layout_to);
            }
            if layout_reversed {
                // Java `CommandLinkElement.executeArg` calls `Link.getInv`
                // for LEFT/UP links. `Cluster.getNodesOrderedTop` then emits
                // that inverted link's start before ordinary SVEK nodes.
                layout.add_plantuml_svek_inverted_start(layout_from);
            }
            if horizontal {
                layout.add_same_rank(layout_from, layout_to);
            }
            let center_label_margin = svek_link_label_margin(logical_from, logical_to)
                + component_middle_label_shield(conn);
            let ordinary_center_label_size = conn.label.as_deref().map(|label| EdgeLabelSize {
                // `SvekEdge.addVisibilityModifier` wraps ordinary center
                // labels by one pixel and autolink labels by six before
                // `appendLine` emits its fixed HTML table.
                width: component_edge_label_layout_width(
                    label,
                    link_style.font_size,
                    &link_style.font_family,
                    center_label_margin + component_padding,
                    short_label_compat,
                ),
                height: (text_render::label_height(label, link_style.font_size)
                    + (center_label_margin + component_padding) * 2.0)
                    .floor(),
            });
            let center_label_size = if let Some((note_index, note)) =
                component_note_on_connection(diagram, connection_index)
            {
                let exact_label_size = conn.label.as_deref().map(|label| EdgeLabelSize {
                    width: component_edge_label_layout_width(
                        label,
                        link_style.font_size,
                        &link_style.font_family,
                        center_label_margin + component_padding,
                        short_label_compat,
                    ),
                    height: text_render::label_height(label, link_style.font_size)
                        + (center_label_margin + component_padding) * 2.0,
                });
                Some(component_link_note_label_size(
                    note,
                    &note_dims[note_index],
                    exact_label_size,
                ))
            } else {
                ordinary_center_label_size
            };
            let endpoint_size = |label: Option<&str>| {
                label.map(|label| EdgeLabelSize {
                    width: (text_render::measure(label, link_style.font_size, false)
                        + component_padding * 2.0)
                        .floor(),
                    height: (text_render::label_height(label, link_style.font_size)
                        + component_padding * 2.0)
                        .floor(),
                })
            };
            let (tail_label, head_label) = if layout_reversed {
                (conn.to_mult.as_deref(), conn.from_mult.as_deref())
            } else {
                (conn.from_mult.as_deref(), conn.to_mult.as_deref())
            };
            // Java `SvekNode.appendShapeInternal` puts shielded lollipop
            // interfaces in an HTML table whose center cell is `PORT="h"`.
            // `SvekEdge.appendLine` addresses that port on every incident edge;
            // routing to the generic node center changes dot's crossing order
            // in symmetric nested clusters.
            let interface_port = |endpoint: &str| {
                diagram
                    .interfaces
                    .iter()
                    .find(|interface| interface.id == endpoint)
                    .and_then(|interface| component_interface_shield(diagram, interface))
                    .map(|_| "h")
            };
            layout.add_edge_with_ports_and_label_sizes_and_minlen(
                layout_from,
                layout_to,
                EdgePorts {
                    tail: interface_port(layout_logical_from),
                    head: interface_port(layout_logical_to),
                },
                center_label_size,
                endpoint_size(tail_label),
                endpoint_size(head_label),
                if horizontal_crosses_cluster_scope
                    || horizontal_plain_interface
                    || horizontal_socket_same_owner
                {
                    // `Cluster.appendRankSame` can only own links whose two
                    // endpoints are in its direct node set. For sibling
                    // component clusters, Java `SvekEdge.appendLine` retains
                    // the one-step queue as `minlen=0`, letting dot align the
                    // cluster members without reparenting them into a root
                    // rank subgraph.
                    Some(0)
                } else {
                    (!horizontal).then(|| conn.length.saturating_sub(1))
                },
            );
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    } else {
        None
    };

    let n_comp = diagram.components.len();

    // Compute positions from oracle, layout engine, or grid fallback.
    let (mut positions, mut iface_positions, mut cluster_positions, content_w, content_h) =
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
    if component_document_margin != StyleBoxSides::default() {
        for (x, y) in &mut positions {
            *x += component_document_margin.left;
            *y += component_document_margin.top;
        }
        for (x, y) in &mut iface_positions {
            *x += component_document_margin.left;
            *y += component_document_margin.top;
        }
        for cluster in &mut cluster_positions {
            cluster.x += component_document_margin.left;
            cluster.y += component_document_margin.top;
        }
    }
    for (position, interface) in iface_positions.iter_mut().zip(&diagram.interfaces) {
        if let Some((shield_x, _)) = component_interface_shield(diagram, interface) {
            // Java's three fixed table cells round independently. When the
            // left shield cell receives the remainder, the painted `h` cell
            // is one point right of the combined table envelope.
            position.0 = round_svek_input_coord(position.0 + shield_x.round() - shield_x.floor());
            position.1 = round_svek_input_coord(position.1);
        }
    }
    let empty_edge_paths: Vec<EdgePath> = Vec::new();
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };
    let svek_svg_y_axis = layout_result.as_ref().map(|result| {
        result
            .node_positions
            .iter()
            .map(|position| position.y + position.height)
            .chain(
                result
                    .cluster_positions
                    .iter()
                    .map(|position| position.y + position.height),
            )
            .fold(0.0_f64, f64::max)
    });
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
    let (mut svek_edge_dx, mut svek_edge_dy) =
        svek_edge_translation.unwrap_or((MARGIN, MARGIN + title_h));
    let mut note_layouts: Vec<ComponentNoteLayout> = layout_result
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
    let mut endpoint_label_layouts =
        component_endpoint_label_layouts(ComponentEndpointLabelInput {
            diagram,
            edge_paths,
            edge_dx: svek_edge_dx,
            edge_dy: svek_edge_dy,
            link_styles: &component_link_styles,
            positions: &positions,
            comp_dims: &comp_dims,
            iface_positions: &iface_positions,
            note_layouts: &note_layouts,
            padding: component_padding,
        });
    let endpoint_min_x = endpoint_label_layouts
        .iter()
        .flat_map(|layout| [layout.tail, layout.head])
        .flatten()
        .map(|rect| rect.x)
        .min_by(f64::total_cmp);
    let endpoint_min_y = endpoint_label_layouts
        .iter()
        .flat_map(|layout| [layout.tail, layout.head])
        .flatten()
        .map(|rect| rect.y)
        .min_by(f64::total_cmp);
    let endpoint_frame_dx =
        endpoint_min_x.map_or(0.0, |min_x| (SVEK_CLUSTER_ORIGIN - min_x).max(0.0));
    let endpoint_frame_dy = endpoint_min_y.map_or(0.0, |min_y| {
        (title_h + SVEK_CLUSTER_ORIGIN - min_y).max(0.0)
    });
    if endpoint_frame_dx > 0.0 || endpoint_frame_dy > 0.0 {
        // `SvekResult.calculateDimension` includes moved endpoint text, then
        // translates the complete painted envelope to its six-pixel origin.
        for (x, y) in &mut positions {
            *x += endpoint_frame_dx;
            *y += endpoint_frame_dy;
        }
        for (x, y) in &mut iface_positions {
            *x += endpoint_frame_dx;
            *y += endpoint_frame_dy;
        }
        for cluster in &mut cluster_positions {
            cluster.x += endpoint_frame_dx;
            cluster.y += endpoint_frame_dy;
        }
        for note in &mut note_layouts {
            note.x += endpoint_frame_dx;
            note.y += endpoint_frame_dy;
        }
        for layout in &mut endpoint_label_layouts {
            for rect in [&mut layout.tail, &mut layout.head].into_iter().flatten() {
                rect.x += endpoint_frame_dx;
                rect.y += endpoint_frame_dy;
            }
        }
        svek_edge_dx += endpoint_frame_dx;
        svek_edge_dy += endpoint_frame_dy;
    }
    let link_note_unpainted_right_edges: Vec<(String, String)> = diagram
        .connections
        .iter()
        .enumerate()
        .filter_map(|(connection_index, connection)| {
            let (note_index, note) = component_note_on_connection(diagram, connection_index)?;
            if !matches!(
                note.position,
                ComponentNotePosition::Top | ComponentNotePosition::Bottom
            ) {
                return None;
            }
            let margin = svek_link_label_margin(&connection.from, &connection.to);
            let label_width = connection
                .label
                .as_deref()
                .map(|label| {
                    component_edge_label_layout_width(
                        label,
                        component_link_styles[connection_index].font_size,
                        &component_link_styles[connection_index].font_family,
                        margin + component_padding,
                        short_label_compat,
                    )
                })
                .unwrap_or(0.0);
            let note_width = note_dims[note_index].width + LINK_NOTE_PADDING * 2.0;
            if note_width < label_width {
                return None;
            }
            let (logical_from, logical_to, _) = no_oracle_layout_edge_ends(connection);
            let layout_endpoint = |logical: &str| {
                package_qualified_names
                    .get(logical)
                    .and_then(|qname| group_endpoint_node_map.get(qname))
                    .cloned()
                    .unwrap_or_else(|| logical.to_string())
            };
            Some((layout_endpoint(logical_from), layout_endpoint(logical_to)))
        })
        .collect();
    let middle_label_edges: Vec<(String, String)> = diagram
        .connections
        .iter()
        .filter(|connection| {
            connection.label.is_some()
                && matches!(
                    connection.shape,
                    LinkShape::MiddleBallSocket | LinkShape::MiddleFullSocket
                )
        })
        .map(|connection| {
            let (logical_from, logical_to, _) = no_oracle_layout_edge_ends(connection);
            let layout_endpoint = |logical: &str| {
                package_qualified_names
                    .get(logical)
                    .and_then(|qname| group_endpoint_node_map.get(qname))
                    .cloned()
                    .unwrap_or_else(|| logical.to_string())
            };
            (layout_endpoint(logical_from), layout_endpoint(logical_to))
        })
        .collect();

    // Estimate package bounding box.
    let pkg_total_w = estimate_packages_width(&diagram.packages);
    let pkg_total_h = estimate_packages_height(&diagram.packages);

    let (mut total_w, mut total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else if oracle.is_none() {
        compute_no_oracle_canvas(NoOracleCanvas {
            diagram,
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
            endpoint_label_layouts: &endpoint_label_layouts,
            link_note_unpainted_right_edges: &link_note_unpainted_right_edges,
            middle_label_edges: &middle_label_edges,
            component_shadows: &component_shadows,
        })
    } else {
        (
            content_w.max(pkg_total_w).max(100.0),
            (content_h + pkg_total_h + title_h).max(50.0),
        )
    };
    if oracle.is_none() && component_document_margin != StyleBoxSides::default() {
        // The solved body already owns the normal seven-pixel chrome tail
        // and five-pixel positive-axis tail. `TextBlockMarged` replaces
        // those tails with the document's right and bottom sides.
        total_w += component_document_margin.right - CHROME_RIGHT_PAD;
        total_h += component_document_margin.bottom - (SVEK_ENVELOPE_ORIGIN - 1.0);
        // `TextBlockExporter12026` adds the document margin to the SVEK
        // dimension before `SvgGraphics.ensureVisible` records the horizontal
        // frontier as `(int) (x + 1)`.
        total_w = total_w.ceil();
    }
    let chrome = if oracle.is_none() {
        let chrome = component_chrome_layout(diagram, total_w, total_h);
        for (x, y) in &mut positions {
            *x += chrome.body_dx;
            *y += chrome.body_dy;
        }
        for (x, y) in &mut iface_positions {
            *x += chrome.body_dx;
            *y += chrome.body_dy;
        }
        for cluster in &mut cluster_positions {
            cluster.x += chrome.body_dx;
            cluster.y += chrome.body_dy;
        }
        for note in &mut note_layouts {
            note.x += chrome.body_dx;
            note.y += chrome.body_dy;
        }
        svek_edge_dx += chrome.body_dx;
        svek_edge_dy += chrome.body_dy;
        total_w = chrome.canvas_width;
        total_h = chrome.canvas_height;
        chrome
    } else {
        ComponentChromeLayout::identity(total_w, total_h)
    };

    // PlantUML emits `data-diagram-type="DESCRIPTION"` for ordinary component
    // diagrams, but routes a bare `interface` inside a `component {…}` block
    // with a lollipop/socket link through its *class* diagram machinery, which
    // tags the root as `CLASS`. Honour the oracle's captured type when it is
    // present so the lollipop case matches; absent an oracle, keep DESCRIPTION.
    let diagram_type = oracle
        .and_then(|o| o.diagram_type.as_deref())
        .filter(|t| *t == "CLASS")
        .or_else(|| {
            diagram
                .interfaces
                .iter()
                .any(|interface| component_interface_uses_class_box(diagram, interface))
                .then_some("CLASS")
        })
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
    let component_shadow_filter_id =
        component_shadows
            .iter()
            .any(|shadow| *shadow > 0.0)
            .then(|| {
                crate::filter_registry::shadow_id_for(diagram.meta.source.as_deref().unwrap_or(""))
            });
    let mut component_defs = gradient_defs.unwrap_or("").to_string();
    if let Some(filter_id) = component_shadow_filter_id.as_deref()
        && !component_defs.contains(&format!(r#"id="{filter_id}""#))
    {
        // Java `SvgGraphics#manageShadow` lazily creates one source-seeded
        // filter shared by every shape whose delta shadow is positive.
        component_defs.push_str(&crate::filter_registry::shadow_filter_def(filter_id));
    }
    let mut svg = SvgBuilder::new_plantuml_with_background_and_defs(
        total_w,
        total_h,
        diagram_type,
        canvas_background.as_deref(),
        &component_defs,
    );
    if let Some(bg) = canvas_rect.as_deref() {
        svg.raw(&format!(
            r#"<rect fill="{bg}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
            h = total_h as i64,
            w = total_w as i64,
        ));
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
        let source_line = diagram.meta.header_line.unwrap_or(1);
        buf.push_str(&format!(
            r#"<g class="header" data-source-line="{source_line}">"#
        ));
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

    // `DiagramChromeFactory12026.create` applies the title before the outer
    // header/footer decorator, so the header draws first and the title is
    // translated by the header ribbon height.
    if let Some(title) = &diagram.meta.title {
        let oracle_title =
            oracle.and_then(|o| o.decorations.iter().find(|d| d.class_name == "title"));
        let widths: Vec<f64> = title
            .lines()
            .map(|t| text_render::measure(t, TITLE_FONT_SIZE, true))
            .collect();
        let mut buf = String::new();
        let oracle_source_line = oracle_title.and_then(|d| d.source_line.as_deref());
        let source_line = oracle_source_line
            .map(str::to_string)
            .unwrap_or_else(|| diagram.meta.title_line.unwrap_or(1).to_string());
        buf.push_str(&format!(
            r#"<g class="title" data-source-line="{source_line}">"#
        ));
        for (i, tline) in title.lines().enumerate() {
            let oracle_text = oracle_title.and_then(|d| d.texts.get(i));
            let ty = oracle_text.map_or(
                chrome.header_height
                    + TITLE_TOP_PAD
                    + pm::ascent(TITLE_FONT_SIZE)
                    + i as f64 * TITLE_LINE_H,
                |t| t.y,
            );
            let x = oracle_text.map_or(
                TITLE_MARGIN_X.max((chrome.content_width - widths[i]) / 2.0),
                |t| t.x,
            );
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
    // order. Without an oracle, package membership supplies the same qualified
    // depth for both components and interfaces.
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
        let any_nested_all = comp_order.iter().any(|&i| {
            qualified_names
                .get(&diagram.components[i].id)
                .map(|q| q.contains('.'))
                .unwrap_or(false)
        }) || diagram.interfaces.iter().any(|interface| {
            qualified_names
                .get(&interface.id)
                .map(|q| q.contains('.'))
                .unwrap_or(false)
        });
        let mut out: Vec<EmitItem> = comp_order
            .iter()
            .map(|&i| EmitItem::Comp(i))
            .chain((0..diagram.interfaces.len()).map(EmitItem::Iface))
            .collect();
        out.sort_by_key(|item| match *item {
            EmitItem::Comp(i) => {
                let component = &diagram.components[i];
                let depth = qualified_names
                    .get(&component.id)
                    .map(|q| q.matches('.').count())
                    .unwrap_or(0);
                (any_nested_all && depth == 0, depth, component.source_line)
            }
            EmitItem::Iface(i) => {
                let interface = &diagram.interfaces[i];
                let depth = qualified_names
                    .get(&interface.id)
                    .map(|q| q.matches('.').count())
                    .unwrap_or(0);
                (any_nested_all && depth == 0, depth, interface.source_line)
            }
        });
        out
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
                    &qualified_names,
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
        let render_style = &component_entity_styles[i];
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
        if hidden_components.contains(comp.id.as_str()) {
            continue;
        }

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
            // Java `Style#eventuallyOverride(colors)` applies declaration
            // colors after the entity-builder snapshot.
            .or_else(|| {
                comp.color
                    .as_deref()
                    .map(|color| crate::sequence::gradient_fill_or(color, gradient_defs))
            });
        let fill = fill_owned.as_deref().unwrap_or(&render_style.fill);

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
                    "stroke:{};stroke-width:{};{}",
                    render_style.stroke,
                    fc(render_style.stroke_width),
                    component_dash_suffix(render_style.dash),
                )
            });
        // Corner radius: honour the oracle's captured rx/ry when present. A
        // `storage` element renders as a fully-rounded rect (rx=35) rather than
        // a component's slight 2.5 rounding, and the value lives in the golden's
        // body `<rect>`. Fall back to skinparam corner radius, then default.
        let oracle_rx = oracle_rect.and_then(|r| r.rect_rx.as_deref());
        let oracle_ry = oracle_rect.and_then(|r| r.rect_ry.as_deref());
        let round_r = render_style.round_corner;
        let rx_s = oracle_rx.map(String::from).unwrap_or_else(|| fc(round_r));
        let ry_s = oracle_ry.map(String::from).unwrap_or_else(|| fc(round_r));
        let shadow_attr = component_shadow_filter_id
            .as_deref()
            .filter(|_| render_style.shadow > 0.0)
            .map(|id| format!(r#" filter="url(#{id})""#))
            .unwrap_or_default();

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
                | ComponentElementKind::Node
                | ComponentElementKind::Storage => unreachable!(),
            }
            // Skip the rect body and tab-icon block below.
        } else if matches!(comp.kind, ComponentElementKind::Storage) {
            svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" rx="{STORAGE_RADIUS}" ry="{STORAGE_RADIUS}" style="{body_style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(h),
                w_s = fc(w),
                x_s = fc(x),
                y_s = fc(y),
            ));
        } else {
            svg.raw(&format!(
            r#"<rect fill="{fill}"{shadow_attr} height="{h_s}" rx="{rx_s}" ry="{ry_s}" style="{body_style}" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
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
        let model_text_block_x = component_padding
            + match comp.kind {
                ComponentElementKind::Database => x + DATABASE_MARGIN_X,
                ComponentElementKind::Queue => x + QUEUE_MARGIN_LEFT,
                ComponentElementKind::Storage => x + STORAGE_MARGIN,
                ComponentElementKind::Artifact => x + ARTIFACT_MARGIN_LEFT,
                ComponentElementKind::Component if component_style_rectangle => {
                    x + RECTANGLE_MARGIN_X
                }
                _ => x + TEXT_PAD_LEFT,
            };
        let oracle_text_x_default = oracle_rect.and_then(|r| r.name_text_x);
        let n_stereo = comp.stereotypes.len();

        // Java symbol implementations draw the merged stereotype/label
        // TextBlock from their top margin. A Creole line can be taller than
        // the default row and its first run need not use the tallest run's
        // baseline, so preserve both dimensions from `Sea`.
        let model_text_block_y = component_padding
            + y
            + match comp.kind {
                ComponentElementKind::Database => DATABASE_MARGIN_TOP,
                ComponentElementKind::Queue => QUEUE_MARGIN_Y,
                ComponentElementKind::Storage => STORAGE_MARGIN,
                ComponentElementKind::Artifact => ARTIFACT_MARGIN_TOP,
                ComponentElementKind::Cloud => CLOUD_MARGIN,
                ComponentElementKind::Component if component_style_rectangle => RECTANGLE_MARGIN_Y,
                _ => COMPONENT_MARGIN_TOP,
            };
        let model_label_y = model_text_block_y
            + text_metrics.stereotype_heights.iter().sum::<f64>()
            + text_metrics.label_first_baseline_ascent;
        let label_y = oracle_text_y
            .and_then(|v| v.get(n_stereo).copied())
            .unwrap_or(model_label_y);

        let mut model_stereo_top = model_text_block_y;
        let model_stereo_y: Vec<f64> = text_metrics
            .stereotype_heights
            .iter()
            .zip(&text_metrics.stereotype_first_baseline_ascents)
            .map(|(height, first_baseline_ascent)| {
                let baseline = model_stereo_top + first_baseline_ascent;
                model_stereo_top += height;
                baseline
            })
            .collect();

        // Stereotypes first (italic in PlantUML).
        for (si, stereo) in comp.stereotypes.iter().enumerate() {
            let ty = oracle_text_y
                .and_then(|v| v.get(si).copied())
                .unwrap_or(model_stereo_y[si]);
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
                    font_size: render_style.font_size as u32,
                    font_family: &render_style.font_family,
                    fill: &render_style.font_color,
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
                font_size: render_style.font_size as u32,
                font_family: &render_style.font_family,
                fill: &render_style.font_color,
                bold: render_style.font_bold,
                italic: render_style.font_italic,
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
            if note.connection.is_some() {
                continue;
            }
            if note
                .target
                .as_deref()
                .is_some_and(|target| hidden_components.contains(target))
            {
                continue;
            }
            let layout = note_layouts
                .iter()
                .find(|layout| layout.note_index == note_index);
            let uid = no_oracle_uids
                .as_ref()
                .and_then(|uids| uids.note_ids.get(&note_index));
            if let (Some(note_id), Some(layout), Some(uid)) =
                (named_note_ids.get(&note_index), layout, uid)
                && let Some((_connection_index, connection)) =
                    diagram.connections.iter().enumerate().find(|(index, _)| {
                        component_opalized_floating_note(diagram, &named_note_ids, *index)
                            .is_some_and(|(candidate, _)| candidate == note_index)
                    })
            {
                let (layout_from, layout_to, _) = no_oracle_layout_edge_ends(connection);
                let edge = edge_paths
                    .iter()
                    .find(|edge| edge.from == layout_from && edge.to == layout_to);
                let note_is_first = edge
                    .map(|edge| edge.from == *note_id)
                    .unwrap_or(connection.from == *note_id);
                let position = edge
                    .and_then(|edge| edge.points.first().zip(edge.points.last()))
                    .map(|(first, last)| {
                        let (note_point, target_point) = if note_is_first {
                            (first, last)
                        } else {
                            (last, first)
                        };
                        let dx = target_point.0 - note_point.0;
                        let dy = target_point.1 - note_point.1;
                        if dx.abs() > dy.abs() {
                            if dx > 0.0 {
                                ComponentNotePosition::Left
                            } else {
                                ComponentNotePosition::Right
                            }
                        } else if dy > 0.0 {
                            ComponentNotePosition::Top
                        } else {
                            ComponentNotePosition::Bottom
                        }
                    })
                    .unwrap_or(ComponentNotePosition::Top);
                render_attached_component_note(
                    note,
                    layout,
                    uid,
                    edge.map(|edge| (edge, svek_edge_dx, svek_edge_dy)),
                    (position, Some(note_is_first)),
                    &mut svg,
                );
                continue;
            }
            if let (Some(layout), Some(uid), Some(target)) = (layout, uid, note.target.as_deref()) {
                let note_id = component_note_layout_id(note_index, &named_note_ids);
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
                        edge.map(|edge| (edge, svek_edge_dx, svek_edge_dy)),
                        (note.position, None),
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
            if hidden_components.contains(target) {
                continue;
            }
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
            let note_id = component_note_layout_id(note_index, &named_note_ids);
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
                (svek_edge_dx, svek_edge_dy),
                svek_svg_y_axis,
                tail_cluster,
                head_cluster,
                0.0,
                0.0,
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
            &component_link_styles,
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
            let link_style = &component_link_styles[connection_index];
            let component_arrow_stroke = link_style.stroke.clone();
            let component_arrow_stroke_width = link_style.stroke_width;
            let component_arrow_font_color = link_style.font_color.clone();
            let component_arrow_font_family = link_style.font_family.clone();
            let component_arrow_font_size = link_style.font_size;
            let (logical_from, logical_to, layout_reversed) = no_oracle_layout_edge_ends(conn);
            let class_socket_reversed = component_class_socket_link(diagram, conn);
            let (layout_logical_from, layout_logical_to) = if class_socket_reversed {
                (logical_to, logical_from)
            } else {
                (logical_from, logical_to)
            };
            let layout_from = package_qualified_names
                .get(layout_logical_from)
                .and_then(|qname| group_endpoint_nodes.get(qname))
                .map(String::as_str)
                .unwrap_or(layout_logical_from);
            let layout_to = package_qualified_names
                .get(layout_logical_to)
                .and_then(|qname| group_endpoint_nodes.get(qname))
                .map(String::as_str)
                .unwrap_or(layout_logical_to);
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
            if component_opalized_floating_note(diagram, &named_note_ids, connection_index)
                .is_some()
            {
                // `EntityImageNote` paints the sole named-note edge as its
                // Opale connector mouth instead of a separate link group.
                continue;
            }

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
                "stroke-dasharray:7,7;".to_string()
            } else {
                component_dash_suffix(link_style.dash)
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
            if hidden_components.contains(logical_from) || hidden_components.contains(logical_to) {
                continue;
            }

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
                let reversed_edge_points;
                let edge_points_input = if class_socket_reversed {
                    reversed_edge_points = ep.points.iter().rev().copied().collect::<Vec<_>>();
                    reversed_edge_points.as_slice()
                } else {
                    ep.points.as_slice()
                };
                let (arrow_at_start, arrow_at_end) = no_oracle_effective_arrow_ends(conn);
                let (extension_at_start, extension_at_end) =
                    no_oracle_effective_extension_ends(conn);
                let end_decoration_length = if extension_at_end {
                    18.0
                } else if arrow_at_end {
                    6.0
                } else if matches!(
                    conn.shape,
                    LinkShape::TargetSocket | LinkShape::TargetBallSocket
                ) {
                    10.0
                } else {
                    0.0
                };
                let edge_points = component_svek_edge_points(
                    edge_points_input,
                    (svek_edge_dx, svek_edge_dy),
                    svek_svg_y_axis,
                    from_package,
                    to_package,
                    if extension_at_start {
                        18.0
                    } else if arrow_at_start {
                        6.0
                    } else {
                        0.0
                    },
                    end_decoration_length,
                );
                let path_d = build_path_d(&edge_points);
                let path_id = no_oracle_path_id(conn);
                let code_line_attr = if class_socket_reversed && conn.source_line > 0 {
                    format!(r#" codeLine="{}""#, conn.source_line)
                } else {
                    String::new()
                };
                svg.raw(&format!(
                r#"<path{code_line_attr} d="{path_d}" fill="none" id="{path_id}" style="stroke:{component_arrow_stroke};stroke-width:{component_arrow_stroke_width};{dash_attr}"/>"#,
            ));

                let raw_edge_points = component_svek_edge_points(
                    edge_points_input,
                    (svek_edge_dx, svek_edge_dy),
                    svek_svg_y_axis,
                    from_package,
                    to_package,
                    0.0,
                    0.0,
                );
                if arrow_at_start {
                    let first = raw_edge_points.first().unwrap();
                    let next = raw_edge_points.get(1).unwrap_or(first);
                    if extension_at_start {
                        render_extension_head(
                            &mut svg,
                            next,
                            first,
                            &component_arrow_stroke,
                            component_arrow_stroke_width,
                        );
                    } else {
                        render_arrowhead(
                            &mut svg,
                            next,
                            first,
                            &component_arrow_stroke,
                            component_arrow_stroke_width,
                        );
                    }
                }
                if arrow_at_end {
                    let last = raw_edge_points.last().unwrap();
                    let prev = if raw_edge_points.len() >= 2 {
                        &raw_edge_points[raw_edge_points.len() - 2]
                    } else {
                        last
                    };
                    if extension_at_end {
                        render_extension_head(
                            &mut svg,
                            prev,
                            last,
                            &component_arrow_stroke,
                            component_arrow_stroke_width,
                        );
                    } else {
                        render_arrowhead(
                            &mut svg,
                            prev,
                            last,
                            &component_arrow_stroke,
                            component_arrow_stroke_width,
                        );
                    }
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
                let link_note = component_note_on_connection(diagram, connection_index);
                let center_label_margin = svek_link_label_margin(logical_from, logical_to)
                    + component_middle_label_shield(conn)
                    + component_padding;
                let exact_center_label_size = conn.label.as_deref().map(|label| EdgeLabelSize {
                    width: component_edge_label_layout_width(
                        label,
                        component_arrow_font_size,
                        &component_arrow_font_family,
                        center_label_margin,
                        short_label_compat,
                    ),
                    height: text_render::label_height(label, component_arrow_font_size)
                        + center_label_margin * 2.0,
                });
                if let Some(label) = &conn.label {
                    let label_margin = center_label_margin;
                    let label_ascent = text_render::label_ascent_with_family(
                        label,
                        component_arrow_font_size,
                        &component_arrow_font_family,
                    );
                    let horizontal = matches!(
                        conn.direction,
                        Some(ConnectionDirection::Left | ConnectionDirection::Right)
                    );
                    let measured_layout_width = text_render::measure_with_family(
                        label,
                        component_arrow_font_size,
                        false,
                        &component_arrow_font_family,
                    ) + label_margin * 2.0;
                    let placeholder_width = component_edge_label_layout_width(
                        label,
                        component_arrow_font_size,
                        &component_arrow_font_family,
                        label_margin,
                        short_label_compat,
                    );
                    let (x, y) = ep
                        .label
                        .map(|position| {
                            let solved_origin =
                                (position.x + svek_edge_dx, position.y + svek_edge_dy);
                            let label_origin = if let Some((note_index, note)) = link_note {
                                component_link_note_blocks(
                                    solved_origin.0,
                                    solved_origin.1,
                                    note,
                                    &note_dims[note_index],
                                    exact_center_label_size,
                                )
                                .0
                                .unwrap_or(solved_origin)
                            } else {
                                solved_origin
                            };
                            (
                                label_origin.0
                                    + label_margin
                                    + (placeholder_width - measured_layout_width) / 2.0,
                                if short_label_compat && horizontal {
                                    first.1 - HORIZONTAL_LINK_LABEL_SOLVED_HEIGHT
                                        + label_margin
                                        + label_ascent
                                } else {
                                    label_origin.1 + label_margin + label_ascent
                                },
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
                if let Some((note_index, note)) = link_note
                    && let Some(position) = ep.label
                {
                    let (_, (note_x, note_y)) = component_link_note_blocks(
                        position.x + svek_edge_dx,
                        position.y + svek_edge_dy,
                        note,
                        &note_dims[note_index],
                        exact_center_label_size,
                    );
                    render_component_link_note(
                        note,
                        note_x,
                        note_y,
                        &note_dims[note_index],
                        &mut svg,
                    );
                }
                let (tail_mult, head_mult) = if layout_reversed {
                    (conn.to_mult.as_deref(), conn.from_mult.as_deref())
                } else {
                    (conn.from_mult.as_deref(), conn.to_mult.as_deref())
                };
                if let Some(tail_mult) = tail_mult {
                    let mw = text_render::measure(tail_mult, component_arrow_font_size, false);
                    let (x, y) = endpoint_label_layouts
                        .get(connection_index)
                        .and_then(|layout| layout.tail)
                        .map(|position| {
                            (
                                position.x + component_padding,
                                position.y
                                    + component_padding
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
                    let (x, y) = endpoint_label_layouts
                        .get(connection_index)
                        .and_then(|layout| layout.head)
                        .map(|position| {
                            (
                                position.x + component_padding,
                                position.y
                                    + component_padding
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
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{component_arrow_stroke};stroke-width:{component_arrow_stroke_width};{dash_attr}"/>"#,
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
                        component_arrow_stroke_width,
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
                        component_arrow_stroke_width,
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
                if let Some(from_mult) = &conn.from_mult {
                    let mw = text_render::measure(from_mult, component_arrow_font_size, false);
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        from_mult,
                        &TextBase {
                            x: from_cx - mw - 1.0,
                            y: from_cy + component_arrow_font_size + 2.0,
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
                if let Some(to_mult) = &conn.to_mult {
                    let mw = text_render::measure(to_mult, component_arrow_font_size, false);
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        to_mult,
                        &TextBase {
                            x: to_cx - mw - 1.0,
                            y: to_cy - 4.0,
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
        let source_line = diagram.meta.footer_line.unwrap_or(1);
        buf.push_str(&format!(
            r#"<g class="footer" data-source-line="{source_line}">"#
        ));
        text_render::emit_text(
            &mut buf,
            footer,
            &text_render::TextBase {
                x,
                // The SVG exporter serializes the decorated dimension as an
                // integer canvas before the footer ribbon is pinned to it.
                y: total_h.floor() - FOOTER_BOTTOM_GAP,
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
        } else if component_legend_rows(Some(legend)).is_empty() {
            svg.render_legend(MARGIN, total_h / 2.0, legend, SMALL_FONT);
        } else {
            emit_component_legend(&mut svg, diagram, &chrome);
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

fn emit_component_legend(
    svg: &mut SvgBuilder,
    diagram: &ComponentDiagram,
    chrome: &ComponentChromeLayout,
) {
    let rows = component_legend_rows(diagram.meta.legend.as_deref());
    let (Some((rect_width, rect_height)), Some(rect_x), Some(rect_y)) = (
        component_legend_rect_size(&rows),
        chrome.legend_rect_x,
        chrome.legend_rect_y,
    ) else {
        return;
    };

    let column_widths = component_legend_column_widths(&rows);
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
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                cell,
                &TextBase {
                    x: x + cell_pad_x,
                    y: baseline,
                    font_size: LEGEND_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&text_buf);
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
        Some(_) => fallback_x - path.min_xy().0,
        // `USymbolCloud.asSmall` retains the generated frontier's local
        // coordinate frame when there is no oracle rectangle.
        None => fallback_x,
    };
    let ty = match first_text_y {
        Some(text_y) => text_y - CLOUD_MARGIN - pm::ascent(FONT_SIZE),
        None => fallback_y,
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

fn emit_no_oracle_interface_class_box(
    svg: &mut SvgBuilder,
    iface: &Interface,
    center_x: f64,
    center_y: f64,
) {
    let dim = component_interface_class_dim(iface);
    let x = round_svek_input_coord(center_x - dim.width / 2.0);
    let y = round_svek_input_coord(center_y - dim.height / 2.0);
    svg.raw(&format!(
        r#"<rect fill="{COMP_FILL}" height="{}" rx="2.5" ry="2.5" style="stroke:{STROKE};stroke-width:0.5;" width="{}" x="{}" y="{}"/>"#,
        fc(dim.height),
        fc(dim.width),
        fc(x),
        fc(y),
    ));

    let icon_cx = x + 15.0;
    let icon_cy = y + INTERFACE_ICON_TOP_INSET + INTERFACE_ICON_RX;
    svg.raw(&format!(
        r#"<ellipse cx="{}" cy="{}" fill="{INTERFACE_ICON_FILL}" rx="11" ry="11" style="stroke:{STROKE};stroke-width:1;"/>"#,
        fc(icon_cx),
        fc(icon_cy),
    ));

    // Extracted from Java `CircledCharacter`'s Liberation Sans Bold "I"
    // outline at its fixed 11px interface spot. The same outline translates
    // with every label and enclosing component size.
    svg.raw(&format!(
        r##"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} Z " fill="#000000"/>"##,
        fc(icon_cx - 3.5723),
        fc(icon_cy - 3.7349),
        fc(icon_cx - 3.5723),
        fc(icon_cy - 5.8931),
        fc(icon_cx + 3.8071),
        fc(icon_cy - 5.8931),
        fc(icon_cx + 3.8071),
        fc(icon_cy - 3.7349),
        fc(icon_cx + 1.3418),
        fc(icon_cy - 3.7349),
        fc(icon_cx + 1.3418),
        fc(icon_cy + 4.3418),
        fc(icon_cx + 3.8071),
        fc(icon_cy + 4.3418),
        fc(icon_cx + 3.8071),
        fc(icon_cy + 6.5),
        fc(icon_cx - 3.5723),
        fc(icon_cy + 6.5),
        fc(icon_cx - 3.5723),
        fc(icon_cy + 4.3418),
        fc(icon_cx - 1.1069),
        fc(icon_cy + 4.3418),
        fc(icon_cx - 1.1069),
        fc(icon_cy - 3.7349),
    ));

    let mut text_buf = String::new();
    text_render::emit_text(
        &mut text_buf,
        &iface.label,
        &TextBase {
            x: icon_cx + INTERFACE_ICON_RX + 3.0,
            // Extracted `EntityImageClass` baseline for the fixed 48px
            // class-style interface header produced by `BodyFactory`.
            y: y + 21.291,
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
    for line_y in [y + 32.0, y + 40.0] {
        svg.raw(&format!(
            r#"<line style="stroke:{STROKE};stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            fc(x + 1.0),
            fc(x + dim.width - 1.0),
            fc(line_y),
            fc(line_y),
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
    qualified_names: &std::collections::HashMap<String, String>,
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
    let qualified_name = resolved
        .map(|(k, _)| k)
        .or_else(|| qualified_names.get(&iface.id).map(String::as_str))
        .unwrap_or(iface.id.as_str());
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
    } else if oracle_iface.is_none() && component_interface_uses_class_box(diagram, iface) {
        emit_no_oracle_interface_class_box(svg, iface, ix, iy);
    } else {
        // Lollipop circle.
        svg.raw(&format!(
            r#"<ellipse cx="{ix}" cy="{iy}" fill="{interface_fill}" rx="{IFACE_R}" ry="{IFACE_R}" style="stroke:{interface_stroke};stroke-width:0.5;"/>"#,
        ));

        // Label below. Prefer oracle text_x/y when present — PlantUML's
        // exact label positions depend on the surrounding diagram layout.
        let label_y = oracle_iface
            .and_then(|r| r.text_y_values.first().copied())
            .unwrap_or_else(|| {
                iy + (IFACE_NODE_SIZE - IFACE_CENTER_OFFSET)
                    + IFACE_LABEL_GAP
                    + text_render::label_first_baseline_ascent_with_family(
                        &iface.label,
                        FONT_SIZE,
                        "sans-serif",
                    )
            });
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
    label_height: f64,
    label_first_baseline_ascent: f64,
    stereotype_widths: Vec<f64>,
    stereotype_heights: Vec<f64>,
    stereotype_first_baseline_ascents: Vec<f64>,
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
    let label_height = text_render::label_height_with_family(&comp.label, font_size, font_family);
    let label_first_baseline_ascent =
        text_render::label_first_baseline_ascent_with_family(&comp.label, font_size, font_family);
    let stereotype_labels: Vec<String> = comp
        .stereotypes
        .iter()
        .map(|stereotype| format!("\u{00AB}{stereotype}\u{00BB}"))
        .collect();
    let stereotype_widths: Vec<f64> = stereotype_labels
        .iter()
        .map(|label| text_render::measure_with_family(label, font_size, false, font_family))
        .collect();
    let stereotype_heights: Vec<f64> = stereotype_labels
        .iter()
        .map(|label| text_render::label_height_with_family(label, font_size, font_family))
        .collect();
    let stereotype_first_baseline_ascents: Vec<f64> = stereotype_labels
        .iter()
        .map(|label| {
            text_render::label_first_baseline_ascent_with_family(label, font_size, font_family)
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
        label_height,
        label_first_baseline_ascent,
        stereotype_widths,
        stereotype_heights,
        stereotype_first_baseline_ascents,
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
    let text_block_height =
        text_metrics.label_height + text_metrics.stereotype_heights.iter().sum::<f64>();

    let (width, height) = match comp.kind {
        ComponentElementKind::Database => (
            text_metrics.content_width + DATABASE_MARGIN_X * 2.0,
            DATABASE_MARGIN_TOP + text_block_height + DATABASE_MARGIN_BOTTOM,
        ),
        ComponentElementKind::Queue => (
            text_metrics.content_width + QUEUE_MARGIN_LEFT + QUEUE_MARGIN_RIGHT,
            text_block_height + QUEUE_MARGIN_Y * 2.0,
        ),
        ComponentElementKind::Storage => (
            text_metrics.content_width + STORAGE_MARGIN * 2.0,
            text_block_height + STORAGE_MARGIN * 2.0,
        ),
        ComponentElementKind::Artifact => (
            text_metrics.content_width + ARTIFACT_MARGIN_LEFT + ARTIFACT_MARGIN_RIGHT,
            text_block_height + ARTIFACT_MARGIN_TOP + ARTIFACT_MARGIN_BOTTOM,
        ),
        ComponentElementKind::Cloud => (
            text_metrics.content_width + CLOUD_MARGIN * 2.0,
            text_block_height + CLOUD_MARGIN * 2.0,
        ),
        ComponentElementKind::Component if component_style_rectangle => (
            text_metrics.content_width + RECTANGLE_MARGIN_X * 2.0,
            text_block_height + RECTANGLE_MARGIN_Y * 2.0,
        ),
        _ => (
            (text_metrics.content_width + TEXT_PAD_LEFT + TEXT_PAD_RIGHT).max(COMPONENT_MIN_W),
            COMPONENT_BASE_H + text_block_height,
        ),
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
            // Java `USymbolStorage` and `USymbolArtifact` inherit the base
            // zero ClusterHeader supplement; their symbol detail is paint-only.
            ComponentPackageKind::Storage | ComponentPackageKind::Artifact => (0.0, 0.0),
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

struct ComponentMagma {
    owner: String,
    nodes: Vec<String>,
    branch: usize,
}

fn component_square_branch(size: usize) -> usize {
    let mut branch = 1;
    while branch * branch < size {
        branch += 1;
    }
    branch
}

fn add_component_invisible_edge(layout: &mut LayoutGraph, from: &str, to: &str, minlen: usize) {
    layout.add_invisible_edge_with_minlen(from, to, minlen);
    if minlen == 0 {
        // `Bibliotekon.lines0` emits horizontal invisible links before nodes.
        layout.add_plantuml_svek_line0_edge(from, to);
    }
}

fn add_component_single_strategy_to_layout(layout: &mut LayoutGraph, diagram: &ComponentDiagram) {
    let qualified_names = build_qualified_names(&diagram.packages);
    let package_qualified_names = build_package_qualified_names(&diagram.packages);
    let mut linked = std::collections::HashSet::new();
    for connection in &diagram.connections {
        linked.insert(connection.from.as_str());
        linked.insert(connection.to.as_str());
    }
    for note in &diagram.notes {
        if let Some(target) = note.target.as_deref() {
            linked.insert(target);
        }
    }

    let mut groups: std::collections::BTreeMap<String, Vec<(usize, usize, String)>> =
        std::collections::BTreeMap::new();
    for (ordinal, component) in diagram.components.iter().enumerate() {
        if linked.contains(component.id.as_str()) {
            continue;
        }
        let owner = component_svek_owner(&component.id, &qualified_names, &package_qualified_names)
            .unwrap_or("")
            .to_string();
        groups.entry(owner).or_default().push((
            component.source_line,
            ordinal,
            component.id.clone(),
        ));
    }
    let component_count = diagram.components.len();
    for (ordinal, interface) in diagram.interfaces.iter().enumerate() {
        if linked.contains(interface.id.as_str()) {
            continue;
        }
        let owner = component_svek_owner(&interface.id, &qualified_names, &package_qualified_names)
            .unwrap_or("")
            .to_string();
        groups.entry(owner).or_default().push((
            interface.source_line,
            component_count + ordinal,
            interface.id.clone(),
        ));
    }

    let mut magmas = Vec::new();
    for (owner, mut leaves) in groups {
        if leaves.len() < 3 {
            continue;
        }
        leaves.sort_by_key(|(source_line, ordinal, _)| (*source_line, *ordinal));
        let nodes: Vec<String> = leaves.into_iter().map(|(_, _, id)| id).collect();
        let branch = component_square_branch(nodes.len());

        // Java `CucaDiagram.applySingleStrategy` delegates each group of
        // standalone leaves to `Magma.putInSquare`. `SquareMaker.putInSquare`
        // creates only invisible links; dot remains responsible for all
        // coordinates and accommodates renamed labels and heterogeneous sizes.
        let mut head_branch = 0;
        for index in 1..nodes.len() {
            if index - head_branch == branch {
                add_component_invisible_edge(layout, &nodes[head_branch], &nodes[index], 1);
                head_branch = index;
            } else {
                add_component_invisible_edge(layout, &nodes[index - 1], &nodes[index], 0);
            }
        }
        magmas.push(ComponentMagma {
            owner,
            nodes,
            branch,
        });
    }

    let mut by_container: std::collections::BTreeMap<String, Vec<&ComponentMagma>> =
        std::collections::BTreeMap::new();
    for magma in &magmas {
        if magma.owner.is_empty() {
            continue;
        }
        let container = magma
            .owner
            .rsplit_once('.')
            .map_or("", |(parent, _)| parent);
        by_container
            .entry(container.to_string())
            .or_default()
            .push(magma);
    }
    for child_magmas in by_container.values() {
        if child_magmas.len() < 3 {
            continue;
        }
        let branch = component_square_branch(child_magmas.len());
        let mut head_branch = 0;
        for index in 1..child_magmas.len() {
            let starts_new_row = index - head_branch == branch;
            let from = if starts_new_row {
                let top = child_magmas[head_branch];
                let bottom_left = ((top.nodes.len() - 1) / top.branch) * top.branch;
                head_branch = index;
                &top.nodes[bottom_left]
            } else {
                let left = child_magmas[index - 1];
                &left.nodes[left.branch - 1]
            };
            let to = &child_magmas[index].nodes[0];
            let minlen = usize::from(starts_new_row);
            add_component_invisible_edge(layout, from, to, minlen);
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

fn component_cluster_painted_max(
    kind: Option<ComponentPackageKind>,
    width: f64,
    height: f64,
) -> (f64, f64, f64) {
    if matches!(kind, Some(ComponentPackageKind::Cloud)) {
        let (_, _, path_max_x, path_max_y) = crate::cloud_shape::generate(width, height).bounds();
        return (path_max_x, path_max_y, SVEK_CANVAS_PAD);
    }

    let pad = match kind {
        Some(ComponentPackageKind::Package | ComponentPackageKind::Folder) => 15.0,
        Some(ComponentPackageKind::Node | ComponentPackageKind::Database) => 25.0,
        _ => SVEK_CANVAS_PAD,
    };
    let shape_max_x = if matches!(kind, Some(ComponentPackageKind::Artifact)) {
        // Java `LimitFinder.drawUPolygon` expands the folded-page polygon ten
        // pixels in X, ending five pixels past the solved body rectangle.
        width + 5.0
    } else {
        width
    };
    (shape_max_x, height, pad)
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
            ComponentPackageKind::Storage | ComponentPackageKind::Artifact => {
                // `LimitFinder` expands both symbols' outer URectangle to
                // (-1,-1), so `SvekResult` translates their body origin to 7.
                (MARGIN, MARGIN)
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
        component_svek_translation(
            diagram,
            comp_dims,
            node_positions,
            attached_note_count,
            title_h,
        )
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
    if !raw_cluster_positions.is_empty() {
        // Java `SvekResult.calculateDimension` asks `TextBlockUtils.getMinMax`
        // for one painted envelope spanning clusters and ordinary root
        // entities before `DotStringFactory.moveDelta` translates the graph.
        // A cluster must therefore not hide a root leaf that paints farther
        // left or above its frame.
        let node_translation = component_svek_translation(
            diagram,
            comp_dims,
            node_positions,
            attached_note_count,
            title_h,
        );
        layout_dx = layout_dx.max(node_translation.0);
        layout_dy = layout_dy.max(node_translation.1);
    }
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
    for (i, iface) in diagram.interfaces.iter().enumerate() {
        let p = &node_positions[n_comp + i];
        let class_dim = component_interface_uses_class_box(diagram, iface)
            .then(|| component_interface_class_dim(iface));
        let (image_dx, image_dy, center_x, center_y) = if let Some(dim) = class_dim {
            (0.0, 0.0, dim.width / 2.0, dim.height / 2.0)
        } else if component_interface_shield(diagram, iface).is_some() {
            (
                (p.width - IFACE_NODE_SIZE) / 2.0,
                (p.height - IFACE_NODE_SIZE) / 2.0,
                IFACE_CENTER_OFFSET,
                IFACE_CENTER_OFFSET,
            )
        } else {
            (0.0, 0.0, IFACE_CENTER_OFFSET, IFACE_CENTER_OFFSET)
        };
        iface_positions.push((
            p.x + image_dx + layout_dx + center_x,
            p.y + image_dy + layout_dy + center_y,
        ));
    }

    let max_x = node_positions
        .iter()
        .take(n_comp + diagram.interfaces.len())
        .enumerate()
        .map(|(i, p)| {
            p.x + if i < n_comp {
                comp_dims[i].width
            } else {
                let interface = &diagram.interfaces[i - n_comp];
                if component_interface_uses_class_box(diagram, interface) {
                    component_interface_class_dim(interface).width
                } else {
                    IFACE_NODE_SIZE
                }
            }
        })
        .fold(0.0_f64, f64::max);
    let max_y = node_positions
        .iter()
        .take(n_comp + diagram.interfaces.len())
        .enumerate()
        .map(|(i, p)| {
            p.y + if i < n_comp {
                comp_dims[i].height
            } else {
                let interface = &diagram.interfaces[i - n_comp];
                if component_interface_uses_class_box(diagram, interface) {
                    component_interface_class_dim(interface).height
                } else {
                    IFACE_NODE_SIZE
                }
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

    let qualified_names = build_qualified_names(&diagram.packages);
    let class_interface_owners: std::collections::HashSet<&str> = diagram
        .interfaces
        .iter()
        .filter(|interface| component_interface_uses_class_box(diagram, interface))
        .filter_map(|interface| {
            qualified_names
                .get(&interface.id)?
                .rsplit_once('.')
                .map(|(owner, _)| owner)
        })
        .collect();
    let cluster_positions = raw_cluster_positions
        .iter()
        .map(|p| ClusterPosition {
            id: p.id.clone(),
            x: p.x + layout_dx,
            y: p.y + layout_dy,
            width: if class_interface_owners.contains(p.id.as_str()) {
                p.width - INTERFACE_CLASS_CLUSTER_WIDTH_TRIM
            } else {
                p.width
            },
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
    comp_dims: &[CompDim],
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
                // `USymbolDatabase.drawDatabase` and
                // `USymbolQueue.drawQueue` paint UPath primitives whose
                // minimum is their declared image origin.
                ComponentElementKind::Database | ComponentElementKind::Queue => (0.0, 0.0),
                // `USymbolCloud.getSpecificFrontierForCloudNew` generates a
                // seeded UPath whose Bezier controls protrude beyond the
                // nominal image. `LimitFinder.drawUPath` includes those
                // controls in the SVEK painted envelope.
                ComponentElementKind::Cloud => {
                    crate::cloud_shape::generate(comp_dims[index].width, comp_dims[index].height)
                        .min_xy()
                }
                // `USymbolComponent2.drawComponent2` and the remaining leaf
                // symbols retain the established `URectangle` top-left
                // envelope until their primitive models are split out.
                _ => (-1.0, -1.0),
            }
        } else if index < note_start {
            let interface = &diagram.interfaces[index - component_count];
            if component_interface_uses_class_box(diagram, interface) {
                (-1.0, -1.0)
            // `CircleInterface2.drawU` translates its ellipse by the one-pixel
            // image margin, so `SvekResult.calculateDimension` sees the
            // interface's painted minimum one pixel inside the image origin.
            } else if component_interface_shield(diagram, interface).is_some() {
                (
                    (position.width - IFACE_NODE_SIZE) / 2.0 + IFACE_MARGIN,
                    (position.height - IFACE_NODE_SIZE) / 2.0 + IFACE_MARGIN,
                )
            } else {
                (IFACE_MARGIN, IFACE_MARGIN)
            }
        } else if (note_start..note_end).contains(&index) {
            // Notes paint their polygon directly to the Graphviz node bounds.
            (0.0, 0.0)
        } else {
            // Protected package endpoints retain the pre-existing point guard.
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

#[derive(Clone, Copy)]
struct ComponentChromeLayout {
    body_dx: f64,
    body_dy: f64,
    canvas_width: f64,
    canvas_height: f64,
    content_width: f64,
    header_height: f64,
    legend_rect_x: Option<f64>,
    legend_rect_y: Option<f64>,
}

impl ComponentChromeLayout {
    fn identity(canvas_width: f64, canvas_height: f64) -> Self {
        Self {
            body_dx: 0.0,
            body_dy: 0.0,
            canvas_width,
            canvas_height,
            content_width: (canvas_width - CHROME_RIGHT_PAD).max(0.0),
            header_height: 0.0,
            legend_rect_x: None,
            legend_rect_y: None,
        }
    }
}

fn component_legend_rows(text: Option<&str>) -> Vec<Vec<&str>> {
    text.map(|text| {
        text.lines()
            .filter(|line| line.trim().contains('|'))
            .map(|line| {
                line.trim()
                    .trim_matches('|')
                    .split('|')
                    .map(str::trim)
                    .collect::<Vec<_>>()
            })
            .filter(|row| row.iter().any(|cell| !cell.is_empty()))
            .collect()
    })
    .unwrap_or_default()
}

fn component_legend_column_widths(rows: &[Vec<&str>]) -> Vec<f64> {
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

fn component_legend_rect_size(rows: &[Vec<&str>]) -> Option<(f64, f64)> {
    if rows.is_empty() {
        return None;
    }
    let grid_width = component_legend_column_widths(rows).iter().sum::<f64>();
    let row_height = pm::text_height(LEGEND_FONT_SIZE);
    Some((
        grid_width + 2.0 * LEGEND_RECT_PAD_X,
        rows.len() as f64 * row_height + 2.0 * LEGEND_RECT_PAD_Y,
    ))
}

fn component_chrome_layout(
    diagram: &ComponentDiagram,
    body_canvas_width: f64,
    body_canvas_height: f64,
) -> ComponentChromeLayout {
    let legend_rows = component_legend_rows(diagram.meta.legend.as_deref());
    let legend_rect_size = component_legend_rect_size(&legend_rows);
    let (
        decorated_canvas_width,
        decorated_canvas_height,
        legend_body_dx,
        legend_body_dy,
        base_legend_rect_x,
        base_legend_rect_y,
    ) = if let Some((rect_width, rect_height)) = legend_rect_size {
        // `DiagramChromeFactory12026.create` wraps SVEK's normalized envelope
        // in `DecorateEntityImage`. The legend's outer block widens/centers
        // the body and stacks above or below it; the bordered block's extra
        // dimension is vertical because its drawn rectangle already owns the
        // full horizontal table width.
        let body_width = (body_canvas_width - CHROME_RIGHT_PAD).max(0.0);
        let body_height = (body_canvas_height - SVEK_ENVELOPE_ORIGIN).max(0.0);
        let block_width = rect_width + 2.0 * LEGEND_OUTER_MARGIN;
        let block_height = rect_height + LEGEND_BORDERED_HEIGHT_DELTA + 2.0 * LEGEND_OUTER_MARGIN;
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
            decorated_width + CHROME_RIGHT_PAD,
            body_height + block_height + SVEK_ENVELOPE_ORIGIN,
            (decorated_width - body_width) / 2.0,
            body_dy,
            Some(block_x + LEGEND_OUTER_MARGIN),
            Some(block_y + LEGEND_OUTER_MARGIN),
        )
    } else {
        (body_canvas_width, body_canvas_height, 0.0, 0.0, None, None)
    };

    // Java `DiagramChromeFactory12026.create` composes the raw `SvekResult`
    // through `DecorateEntityImage.addTitle` and `addHeaderAndFooter`. Each
    // wrapper takes the widest child, centers the narrower body, and stacks
    // ribbon heights above or below it.
    let body_width = (decorated_canvas_width - CHROME_RIGHT_PAD).max(0.0);
    let title_width = diagram
        .meta
        .title
        .as_deref()
        .map(|title| {
            title
                .lines()
                .map(|line| text_render::measure(line, TITLE_FONT_SIZE, true))
                .fold(0.0_f64, f64::max)
                + TITLE_MARGIN_X * 2.0
        })
        .unwrap_or(0.0);
    let caption_width = [&diagram.meta.header, &diagram.meta.footer]
        .into_iter()
        .filter_map(|caption| caption.as_deref())
        .flat_map(str::lines)
        .map(|line| text_render::measure(line, HEADER_FOOTER_FONT, false))
        .fold(0.0_f64, f64::max);
    let content_width = body_width.max(title_width).max(caption_width);
    let ribbon_height = |caption: Option<&str>| {
        caption
            .map(|text| {
                text.lines().count().max(1) as f64 * pm::text_height(HEADER_FOOTER_FONT)
                    + CAPTION_BOTTOM_PAD
            })
            .unwrap_or(0.0)
    };
    let header_height = ribbon_height(diagram.meta.header.as_deref());
    let footer_height = ribbon_height(diagram.meta.footer.as_deref());

    ComponentChromeLayout {
        body_dx: legend_body_dx + (content_width - body_width) / 2.0,
        body_dy: legend_body_dy + header_height,
        canvas_width: content_width + CHROME_RIGHT_PAD,
        canvas_height: decorated_canvas_height + header_height + footer_height,
        content_width,
        header_height,
        legend_rect_x: base_legend_rect_x.map(|x| x + (content_width - body_width) / 2.0),
        legend_rect_y: base_legend_rect_y.map(|y| y + header_height),
    }
}

struct NoOracleCanvas<'a> {
    diagram: &'a ComponentDiagram,
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
    endpoint_label_layouts: &'a [ComponentEndpointLabelLayout],
    link_note_unpainted_right_edges: &'a [(String, String)],
    middle_label_edges: &'a [(String, String)],
    component_shadows: &'a [f64],
}

fn compute_no_oracle_canvas(input: NoOracleCanvas<'_>) -> (f64, f64) {
    let mut max_x = 0.0_f64;
    let mut max_y = input.title_h;

    for ((((x, y), dim), comp), shadow) in input
        .positions
        .iter()
        .zip(input.comp_dims)
        .zip(input.components)
        .zip(input.component_shadows)
    {
        let (painted_max_x, painted_max_y) = match comp.kind {
            // Java `LimitFinder.drawUPolygon` measures ten pixels beyond both
            // horizontal sides of `USymbolNode`'s body. Its lower-edge
            // `UEmpty(10,10)` also extends the vertical envelope by ten. Add
            // one here because the shared tail below is 14px: rectangles end
            // at `dimension - 1`, making that equivalent to SVEK's 15px
            // dimension delta, while polygon/UEmpty maxima do not.
            ComponentElementKind::Node => (dim.width + 11.0, dim.height + 11.0),
            // `EntityImageDescription` passes the style shadow through
            // `Fashion#withShadow` to `USymbolComponent2#drawComponent2`.
            // `SvgGraphics#svgRectangle` records the same painted rectangle at
            // width/height plus twice its delta shadow.
            ComponentElementKind::Component => {
                (dim.width + *shadow * 2.0, dim.height + *shadow * 2.0)
            }
            ComponentElementKind::Database => (
                dim.width + DATABASE_RENDER_OVERFLOW_X,
                dim.height + DATABASE_RENDER_OVERFLOW_Y,
            ),
            ComponentElementKind::Cloud => {
                let (_, _, max_x, max_y) =
                    crate::cloud_shape::generate(dim.width, dim.height).bounds();
                (max_x, max_y)
            }
            // `LimitFinder.drawUPath` includes the queue's rightmost path
            // boundary when converting the painted SVEK bounds to dimensions.
            ComponentElementKind::Queue => (dim.width + 1.0, dim.height),
            ComponentElementKind::Artifact => {
                (dim.width + ARTIFACT_GRAPH_ENVELOPE_RIGHT, dim.height)
            }
            _ => (dim.width, dim.height),
        };
        max_x = max_x.max(x + painted_max_x);
        max_y = max_y.max(y + painted_max_y);
    }
    for ((cx, cy), interface) in input.iface_positions.iter().zip(input.interfaces) {
        if component_interface_uses_class_box(input.diagram, interface) {
            let dim = component_interface_class_dim(interface);
            max_x = max_x.max(cx + dim.width / 2.0);
            max_y = max_y.max(cy + dim.height / 2.0);
        } else {
            let label_width = text_render::measure(&interface.label, FONT_SIZE, false);
            max_x = max_x.max(cx + IFACE_R.max(label_width / 2.0));
            // `EntityImageDescription.drawU` paints the hidden description
            // below the circle. Java `LimitFinder.drawText` ends each run at
            // baseline + 1.5; the SVEK node outline's half-pixel stroke also
            // participates in the normalized `minMax` dimension. Preserve
            // the historical default row floor for ordinary labels.
            let painted_label_height = (text_render::label_limit_finder_height_with_family(
                &interface.label,
                FONT_SIZE,
                "sans-serif",
            ) + 0.5)
                .max(LINE_HEIGHT);
            max_y = max_y.max(
                cy + (IFACE_NODE_SIZE - IFACE_CENTER_OFFSET)
                    + IFACE_LABEL_GAP
                    + painted_label_height,
            );
        }
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
    for edge in input.edge_paths {
        let Some(position) = edge.label else {
            continue;
        };
        // `SvekEdge.addVisibilityModifier` wraps only the center label. For an
        // autolink, Graphviz's label box already contains that six-pixel
        // wrapper; `SvekResult.calculateDimension` therefore sees only the
        // usual one-pixel painted tail beyond the reported box.
        let margin = svek_link_label_margin(&edge.from, &edge.to);
        let painted_tail = if input
            .link_note_unpainted_right_edges
            .iter()
            .any(|(from, to)| from == &edge.from && to == &edge.to)
        {
            // The fixed HTML table reserves Rose's trailing five-pixel
            // padding, but `LimitFinder` sees only the folded Opale body.
            -LINK_NOTE_PADDING
        } else if input
            .middle_label_edges
            .iter()
            .any(|(from, to)| from == &edge.from && to == &edge.to)
        {
            // The Graphviz placeholder includes the middle-decoration shield;
            // `LimitFinder` sees only the inset painted label at its right.
            -MIDDLE_LABEL_SHIELD
        } else if edge.from == edge.to {
            LINK_LABEL_MARGIN
        } else {
            margin * 2.0
        };
        max_x = max_x.max(position.x + input.edge_dx + position.width + painted_tail);
        max_y = max_y.max(position.y + input.edge_dy + position.height + painted_tail);
    }
    for position in input
        .endpoint_label_layouts
        .iter()
        .flat_map(|layout| [layout.tail, layout.head].into_iter().flatten())
    {
        max_x = max_x.max(position.x + position.width);
        max_y = max_y.max(position.y + position.height);
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
        let (shape_max_x, shape_max_y, pad) =
            component_cluster_painted_max(kind, cluster.width, cluster.height);
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
    link_styles: &[ComponentLinkRenderStyle],
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
    for (connection_index, conn) in diagram.connections.iter().enumerate() {
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
        let link_style = link_styles.get(connection_index);
        emit_oracle_edge(
            svg,
            oracle_edge,
            &conn.from,
            &conn.to,
            link_style
                .map(|style| style.font_size)
                .unwrap_or(arrow_font_size),
            link_style
                .map(|style| style.font_family.as_str())
                .unwrap_or(arrow_font_family),
            link_style
                .map(|style| style.font_color.as_str())
                .unwrap_or(arrow_font_color),
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

fn render_arrowhead(
    svg: &mut SvgBuilder,
    prev: &(f64, f64),
    tip: &(f64, f64),
    stroke: &str,
    stroke_width: f64,
) {
    let dx = tip.0 - prev.0;
    let dy = tip.1 - prev.1;
    let angle = dy.atan2(dx);
    render_arrow_at(svg, tip.0, tip.1, angle, stroke, stroke_width);
}

fn render_extension_head(
    svg: &mut SvgBuilder,
    prev: &(f64, f64),
    tip: &(f64, f64),
    stroke: &str,
    stroke_width: f64,
) {
    // Java provenance: `LinkDecor.EXTENDS.getExtremityFactoryComplete`
    // constructs `ExtremityFactoryTriangle(null, 18, 6, 18)`.
    let angle = (tip.1 - prev.1).atan2(tip.0 - prev.0);
    let cos = angle.cos();
    let sin = angle.sin();
    let mut points = String::new();
    for (index, (x, y)) in [(0.0, 0.0), (-18.0, -6.0), (-18.0, 6.0), (0.0, 0.0)]
        .iter()
        .enumerate()
    {
        if index > 0 {
            points.push(',');
        }
        write!(
            points,
            "{},{}",
            fc(tip.0 + x * cos - y * sin),
            fc(tip.1 + x * sin + y * cos),
        )
        .unwrap();
    }
    svg.raw(&format!(
        r#"<polygon fill="none" points="{points}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
    ));
}

fn render_arrowhead_from_coords(
    svg: &mut SvgBuilder,
    fx: f64,
    fy: f64,
    tx: f64,
    ty: f64,
    stroke: &str,
    stroke_width: f64,
) {
    render_arrow_at(svg, tx, ty, (ty - fy).atan2(tx - fx), stroke, stroke_width);
}

fn no_oracle_link_type_attr(conn: &Connection) -> String {
    let (arrow_at_start, arrow_at_end) = no_oracle_arrow_ends(conn);
    if conn.extension_at_start || conn.extension_at_end {
        r#" data-link-type="extension""#.to_string()
    } else if arrow_at_start || arrow_at_end {
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

fn svek_link_label_margin(from: &str, to: &str) -> f64 {
    if from == to {
        SELF_LINK_LABEL_MARGIN
    } else {
        LINK_LABEL_MARGIN
    }
}

fn component_middle_label_shield(connection: &Connection) -> f64 {
    if matches!(
        connection.shape,
        LinkShape::MiddleBallSocket | LinkShape::MiddleFullSocket
    ) {
        MIDDLE_LABEL_SHIELD
    } else {
        0.0
    }
}

fn component_interface_uses_class_box(diagram: &ComponentDiagram, interface: &Interface) -> bool {
    fn owning_component_group<'a>(
        packages: &'a [ComponentPackage],
        interface_id: &str,
    ) -> Option<&'a str> {
        for package in packages {
            if matches!(package.kind, ComponentPackageKind::Component)
                && package
                    .components
                    .iter()
                    .any(|member| member == interface_id)
            {
                return Some(package.name.as_str());
            }
            if let Some(owner) = owning_component_group(&package.packages, interface_id) {
                return Some(owner);
            }
        }
        None
    }

    let Some(owner) = owning_component_group(&diagram.packages, &interface.id) else {
        return false;
    };
    diagram.connections.iter().any(|connection| {
        ((connection.from == owner && connection.to == interface.id)
            || (connection.to == owner && connection.from == interface.id))
            && matches!(
                connection.shape,
                LinkShape::TargetSocket | LinkShape::TargetBallSocket
            )
    })
}

fn component_class_socket_link(diagram: &ComponentDiagram, connection: &Connection) -> bool {
    matches!(
        connection.shape,
        LinkShape::TargetSocket | LinkShape::TargetBallSocket
    ) && diagram.interfaces.iter().any(|interface| {
        component_interface_uses_class_box(diagram, interface)
            && (connection.from == interface.id || connection.to == interface.id)
    })
}

fn component_interface_class_dim(interface: &Interface) -> CompDim {
    // `EntityImageClass` lays out a 22px interface spot, a 3px gap, and the
    // italic display text in a header with one-pixel side borders.
    CompDim {
        width: text_render::measure(&interface.label, FONT_SIZE, false)
            + INTERFACE_CLASS_BOX_WIDTH_PAD,
        height: INTERFACE_CLASS_BOX_HEIGHT,
    }
}

fn component_interface_shield(
    diagram: &ComponentDiagram,
    interface: &Interface,
) -> Option<(f64, f64)> {
    let mut linked_entities = std::collections::HashSet::new();
    for connection in diagram
        .connections
        .iter()
        .filter(|connection| connection.from == interface.id || connection.to == interface.id)
    {
        let other = if connection.from == interface.id {
            &connection.to
        } else {
            &connection.from
        };
        // Java `EntityImageDescription.getShield` disables the label shield
        // for duplicate links to the same peer and for visible one-rank links.
        if !linked_entities.insert(other.as_str()) || connection.length == 1 {
            return None;
        }
    }

    // `EntityImageDescription.getShield` compares the hidden description
    // block against `CircleInterface2`'s 18px image, then reserves the same
    // description height above and below the center cell. `SvekNode.appendHtml`
    // serializes these values directly into its three-row HTML table.
    let label_width = text_render::measure(&interface.label, FONT_SIZE, false);
    // Java `EntityImageDescription.getShield` returns the fractional
    // half-margin and `SvekNode.appendHtml` serializes it unchanged.
    let shield_x = ((label_width - IFACE_NODE_SIZE).max(1.0)) / 2.0;
    let shield_y = text_render::label_height(&interface.label, FONT_SIZE).max(1.0);
    Some((shield_x, shield_y))
}

fn component_no_oracle_spacing(
    diagram: &ComponentDiagram,
    link_styles: &[ComponentLinkRenderStyle],
) -> (f64, bool) {
    let default = GraphSpacing::PLANTUML_SVEK_DEFAULTS.node_sep_px;
    let qualified_names = build_qualified_names(&diagram.packages);
    let has_root_mixed_interface_rank = diagram.interfaces.iter().any(|interface| {
        if qualified_names.contains_key(&interface.id) {
            return false;
        }
        let touches = |connection: &&Connection| {
            connection.from == interface.id || connection.to == interface.id
        };
        let has_horizontal_socket = diagram
            .connections
            .iter()
            .filter(touches)
            .any(|connection| {
                connection.label.is_none()
                    && matches!(
                        connection.direction,
                        Some(ConnectionDirection::Left | ConnectionDirection::Right)
                    )
                    && matches!(
                        connection.shape,
                        LinkShape::TargetSocket | LinkShape::TargetBallSocket
                    )
            });
        let has_vertical_label = diagram
            .connections
            .iter()
            .filter(touches)
            .any(|connection| {
                connection.label.is_some()
                    && matches!(
                        connection.direction,
                        Some(ConnectionDirection::Down | ConnectionDirection::Up)
                    )
            });
        has_horizontal_socket && has_vertical_label
    });
    if has_root_mixed_interface_rank {
        // Java `Cluster.appendRankSame` solves root ranks independently of
        // vertical edge labels. The vendored Graphviz build includes that
        // label's side clearance in the root nodesep; 18px restores Java's
        // 35px solved gap. Cluster-owned ranks do not take this compatibility
        // path because their cluster envelope already supplies the clearance.
        return (18.0, false);
    }
    let shortest_horizontal_table = diagram
        .connections
        .iter()
        .zip(link_styles)
        .filter(|(connection, _)| {
            connection.from != connection.to
                && matches!(
                    connection.direction,
                    Some(ConnectionDirection::Left | ConnectionDirection::Right)
                )
        })
        .filter_map(|(connection, style)| connection.label.as_deref().map(|label| (label, style)))
        .map(|(label, style)| {
            // Java `SvekEdge.addVisibilityModifier` adds one pixel on each
            // side, then `appendTable` truncates the fixed table width.
            (text_render::measure_with_family(label, style.font_size, false, &style.font_family)
                + LINK_LABEL_MARGIN * 2.0)
                .floor()
        })
        .min_by(f64::total_cmp);

    // Extracted Graphviz compatibility table for PlantUML's
    // `Cluster.appendRankSame` + `SvekEdge.appendTable` path:
    //
    // fixed table width | Java solved node gap | vendored inputs
    // 29                | 64                   | nodesep 32, width 29
    // 30                | 65                   | nodesep 32, width 33
    // 52                | 87                   | nodesep 35, width 52
    //
    // The vendored engine collapses Java's distinct 29px and 30px table
    // solutions into the same 70px gap. Its fixed-table quantization needs a
    // three-pixel input increment to retain Java's one-pixel solved-gap step.
    let short_label_compat =
        shortest_horizontal_table.is_some_and(|table_width| (29.0..=30.0).contains(&table_width));
    (
        if short_label_compat {
            default - 3.0
        } else {
            default
        },
        short_label_compat,
    )
}

fn component_edge_label_layout_width(
    label: &str,
    arrow_font_size: f64,
    arrow_font_family: &str,
    margin: f64,
    short_label_compat: bool,
) -> f64 {
    // `GraphvizImageBuilder#buildImage` passes the resolved arrow
    // FontConfiguration to `SvekEdge`; both the visible TextBlock and the
    // fixed HTML table call calculateDimension on that same face.
    let measured =
        text_render::measure_with_family(label, arrow_font_size, false, arrow_font_family)
            + margin * 2.0;
    if !short_label_compat {
        return measured;
    }

    let table_width = measured.floor();
    measured + (table_width - 29.0).clamp(0.0, 1.0) * 3.0
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

fn no_oracle_effective_extension_ends(conn: &Connection) -> (bool, bool) {
    if no_oracle_layout_edge_ends(conn).2 {
        (conn.extension_at_end, conn.extension_at_start)
    } else {
        (conn.extension_at_start, conn.extension_at_end)
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

struct ComponentEndpointLabelInput<'a> {
    diagram: &'a ComponentDiagram,
    edge_paths: &'a [EdgePath],
    edge_dx: f64,
    edge_dy: f64,
    link_styles: &'a [ComponentLinkRenderStyle],
    positions: &'a [(f64, f64)],
    comp_dims: &'a [CompDim],
    iface_positions: &'a [(f64, f64)],
    note_layouts: &'a [ComponentNoteLayout],
    padding: f64,
}

fn component_endpoint_label_layouts(
    input: ComponentEndpointLabelInput<'_>,
) -> Vec<ComponentEndpointLabelLayout> {
    let package_qualified_names = build_package_qualified_names(&input.diagram.packages);
    let group_endpoint_nodes: std::collections::HashMap<String, String> = input
        .diagram
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
    let mut fixed_nodes: Vec<ComponentLabelRect> = input
        .positions
        .iter()
        .zip(input.comp_dims)
        .map(|(&(x, y), dim)| ComponentLabelRect {
            x,
            y,
            width: dim.width,
            height: dim.height,
        })
        .collect();
    fixed_nodes.extend(
        input
            .diagram
            .interfaces
            .iter()
            .zip(input.iface_positions)
            .map(|(interface, &(cx, cy))| {
                let dim = if component_interface_uses_class_box(input.diagram, interface) {
                    component_interface_class_dim(interface)
                } else {
                    CompDim {
                        width: IFACE_NODE_SIZE,
                        height: IFACE_NODE_SIZE,
                    }
                };
                ComponentLabelRect {
                    x: cx - dim.width / 2.0,
                    y: cy - dim.height / 2.0,
                    width: dim.width,
                    height: dim.height,
                }
            }),
    );
    fixed_nodes.extend(input.note_layouts.iter().map(|note| ComponentLabelRect {
        x: note.x,
        y: note.y,
        width: note.width,
        height: note.height,
    }));

    input
        .diagram
        .connections
        .iter()
        .enumerate()
        .map(|(connection_index, connection)| {
            let arrow_font_size = input
                .link_styles
                .get(connection_index)
                .map(|style| style.font_size)
                .unwrap_or(LINK_FONT);
            let (logical_from, logical_to, layout_reversed) =
                no_oracle_layout_edge_ends(connection);
            let (layout_logical_from, layout_logical_to) =
                if component_class_socket_link(input.diagram, connection) {
                    (logical_to, logical_from)
                } else {
                    (logical_from, logical_to)
                };
            let layout_from = package_qualified_names
                .get(layout_logical_from)
                .and_then(|qname| group_endpoint_nodes.get(qname))
                .map(String::as_str)
                .unwrap_or(layout_logical_from);
            let layout_to = package_qualified_names
                .get(layout_logical_to)
                .and_then(|qname| group_endpoint_nodes.get(qname))
                .map(String::as_str)
                .unwrap_or(layout_logical_to);
            let Some(edge) = input
                .edge_paths
                .iter()
                .find(|edge| edge.from == layout_from && edge.to == layout_to)
            else {
                return ComponentEndpointLabelLayout::default();
            };
            let (tail_text, head_text) = if layout_reversed {
                (
                    connection.to_mult.as_deref(),
                    connection.from_mult.as_deref(),
                )
            } else {
                (
                    connection.from_mult.as_deref(),
                    connection.to_mult.as_deref(),
                )
            };
            let make_rect =
                |text: Option<&str>, position: Option<rustuml_layout::graph::EdgeLabelPosition>| {
                    text.zip(position)
                        .map(|(text, position)| ComponentLabelRect {
                            x: position.x + input.edge_dx,
                            y: position.y + input.edge_dy,
                            width: text_render::measure(text, arrow_font_size, false)
                                + input.padding * 2.0,
                            height: text_render::label_height(text, arrow_font_size)
                                + input.padding * 2.0,
                        })
                };
            let mut layout = ComponentEndpointLabelLayout {
                tail: make_rect(tail_text, edge.tail_label),
                head: make_rect(head_text, edge.head_label),
            };
            for fixed_node in &fixed_nodes {
                let expanded = ComponentLabelRect {
                    x: fixed_node.x - SVEK_ENDPOINT_COLLISION_MARGIN,
                    y: fixed_node.y - SVEK_ENDPOINT_COLLISION_MARGIN,
                    width: fixed_node.width + SVEK_ENDPOINT_COLLISION_MARGIN * 2.0,
                    height: fixed_node.height + SVEK_ENDPOINT_COLLISION_MARGIN * 2.0,
                };
                if layout
                    .tail
                    .is_some_and(|label| component_label_rects_intersect(expanded, label))
                {
                    layout.tail = layout
                        .tail
                        .map(|label| component_move_label_away(expanded, label));
                }
                if layout
                    .head
                    .is_some_and(|label| component_label_rects_intersect(expanded, label))
                {
                    layout.head = layout
                        .head
                        .map(|label| component_move_label_away(expanded, label));
                }
            }
            layout
        })
        .collect()
}

fn component_label_rects_intersect(a: ComponentLabelRect, b: ComponentLabelRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn component_move_label_away(
    fixed: ComponentLabelRect,
    label: ComponentLabelRect,
) -> ComponentLabelRect {
    let delta_x = label.x + label.width / 2.0 - (fixed.x + fixed.width / 2.0);
    let delta_y = label.y + label.height / 2.0 - (fixed.y + fixed.height / 2.0);
    let moved = |coefficient: f64| ComponentLabelRect {
        x: label.x + delta_x * coefficient,
        y: label.y + delta_y * coefficient,
        ..label
    };

    // Direct port of `PositionableUtils.moveAwayFrom`, called by
    // `SvekEdge.manageCollision`: bracket the first non-intersecting position
    // in 0.1 doublings, then retain Java's five binary-search iterations.
    let mut min = 0.0;
    let mut max = 0.1;
    while component_label_rects_intersect(fixed, moved(max)) {
        max *= 2.0;
    }
    for _ in 0..5 {
        let candidate = (min + max) / 2.0;
        if component_label_rects_intersect(fixed, moved(candidate)) {
            min = candidate;
        } else {
            max = candidate;
        }
    }
    moved((min + max) / 2.0)
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
        // `MiddleCircleCircled.drawU()` uses its 10px outer radius and a
        // 90-degree arc for the combined ball/parenthesis endpoint.
        render_socket_arc(svg, tip, prev, 10.0, 45.0, "#FFFFFF", stroke);
    } else {
        // `ExtremityParenthesis.drawU()` paints a 140-degree arc around the
        // endpoint (`radius2 = 9`, `ang = 70`).
        render_socket_arc(svg, tip, prev, 9.0, 70.0, "none", stroke);
    }
}

fn render_middle_socket_decoration(
    svg: &mut SvgBuilder,
    shape: LinkShape,
    points: &[(f64, f64)],
    stroke: &str,
) {
    let Some((center, tangent)) = component_dot_path_middle(points) else {
        return;
    };
    let forward = (center.0 + tangent.0, center.1 + tangent.1);
    let backward = (center.0 - tangent.0, center.1 - tangent.1);
    if matches!(shape, LinkShape::MiddleFullSocket) {
        svg.raw(&format!(
            r##"<ellipse cx="{}" cy="{}" fill="#FFFFFF" rx="10" ry="10" style="stroke:#FFFFFF;stroke-width:1;"/>"##,
            fc(center.0),
            fc(center.1),
        ));
        render_socket_arc(svg, center, forward, 10.0, 45.0, "none", stroke);
    }
    render_socket_arc(svg, center, backward, 10.0, 45.0, "none", stroke);
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
    half_angle_degrees: f64,
    fill: &str,
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
    let half_angle = half_angle_degrees.to_radians();
    let along = half_angle.cos();
    let spread = half_angle.sin();
    let px = -uy;
    let py = ux;
    let start = (
        center.0 + (ux * along + px * spread) * radius,
        center.1 + (uy * along + py * spread) * radius,
    );
    let end = (
        center.0 + (ux * along - px * spread) * radius,
        center.1 + (uy * along - py * spread) * radius,
    );
    svg.raw(&format!(
        r##"<path d="M{},{} A{},{} 0 0 0 {} {}" fill="{fill}" style="stroke:{stroke};stroke-width:1.5;"/>"##,
        fc(start.0),
        fc(start.1),
        fc(radius),
        fc(radius),
        fc(end.0),
        fc(end.1),
    ));
}

struct DotPathMiddleCandidate {
    point: (f64, f64),
    tangent: (f64, f64),
    cost: f64,
}

/// Port of `DotPath.getMiddle()`: subdivide each cubic at `t = 0.5`, then
/// choose the candidate minimizing squared distance to the complete path's
/// endpoints. The associated angle is `BezierUtils.getEndingAngle(left)`.
fn component_dot_path_middle(points: &[(f64, f64)]) -> Option<((f64, f64), (f64, f64))> {
    let (&start, &end) = points.first().zip(points.last())?;
    if points.len() < 4 {
        let tangent = (end.0 - start.0, end.1 - start.1);
        return (tangent.0 != 0.0 || tangent.1 != 0.0)
            .then_some((((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0), tangent));
    }

    let cost = |point: (f64, f64)| {
        let start_dx = point.0 - start.0;
        let start_dy = point.1 - start.1;
        let end_dx = point.0 - end.0;
        let end_dy = point.1 - end.1;
        start_dx * start_dx + start_dy * start_dy + end_dx * end_dx + end_dy * end_dy
    };
    let mut best: Option<DotPathMiddleCandidate> = None;
    let mut index = 0;
    while index + 3 < points.len() {
        let p0 = points[index];
        let p1 = points[index + 1];
        let p2 = points[index + 2];
        let p3 = points[index + 3];
        let q0 = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
        let q1 = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        let q2 = ((p2.0 + p3.0) / 2.0, (p2.1 + p3.1) / 2.0);
        let r0 = ((q0.0 + q1.0) / 2.0, (q0.1 + q1.1) / 2.0);
        let r1 = ((q1.0 + q2.0) / 2.0, (q1.1 + q2.1) / 2.0);
        let middle = ((r0.0 + r1.0) / 2.0, (r0.1 + r1.1) / 2.0);

        for (point, tangent) in [
            (p0, (q0.0 - p0.0, q0.1 - p0.1)),
            (middle, (middle.0 - r0.0, middle.1 - r0.1)),
            (p3, (p3.0 - q2.0, p3.1 - q2.1)),
        ] {
            let candidate_cost = cost(point);
            if best
                .as_ref()
                .is_none_or(|candidate| candidate_cost < candidate.cost)
            {
                best = Some(DotPathMiddleCandidate {
                    point,
                    tangent,
                    cost: candidate_cost,
                });
            }
        }
        index += 3;
    }

    best.and_then(|candidate| {
        (candidate.tangent.0 != 0.0 || candidate.tangent.1 != 0.0)
            .then_some((candidate.point, candidate.tangent))
    })
}

fn render_arrow_at(
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    angle: f64,
    stroke: &str,
    stroke_width: f64,
) {
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
        r#"<polygon fill="{stroke}" points="{pts}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
    ));
}

// ---------------------------------------------------------------------------
// Path building
// ---------------------------------------------------------------------------

fn component_svek_edge_points(
    points: &[(f64, f64)],
    translation: (f64, f64),
    svg_y_axis: Option<f64>,
    tail_cluster: Option<&ClusterPosition>,
    head_cluster: Option<&ClusterPosition>,
    start_decoration_length: f64,
    end_decoration_length: f64,
) -> Vec<(f64, f64)> {
    // `SvekEdge.solveLine` reads the spline back from Graphviz's SVG through
    // `SvgResult.toDotPath`; Graphviz serializes those path coordinates at two
    // decimal places. Its SVG transform serializes the Y axis and the
    // mathematical Y coordinate independently, so reproduce
    // `round(axis) - round(axis - y)` rather than rounding the flipped result.
    // The vendored C API gives us the pre-serialization doubles.
    let quantize = |value: f64| (value * 100.0).round() / 100.0;
    let (dx, dy) = translation;
    let serialized_y_axis = svg_y_axis.map(quantize);
    let mut out: Vec<(f64, f64)> = points
        .iter()
        .map(|(x, y)| {
            let serialized_y = svg_y_axis
                .zip(serialized_y_axis)
                .map(|(axis, serialized_axis)| serialized_axis - quantize(axis - *y))
                .unwrap_or_else(|| quantize(*y));
            (quantize(*x) + dx, serialized_y + dy)
        })
        .collect();
    out = simulate_compound(out, tail_cluster, head_cluster);

    if start_decoration_length > 0.0 && out.len() >= 2 {
        let (tip_x, tip_y) = out[0];
        let (next_x, next_y) = out[1];
        let vx = next_x - tip_x;
        let vy = next_y - tip_y;
        let len = (vx * vx + vy * vy).sqrt();
        if len > f64::EPSILON {
            let ux = vx / len;
            let uy = vy / len;
            let trim = start_decoration_length;
            out[0] = (tip_x + ux * trim, tip_y + uy * trim);
            if out.len() >= 4 {
                let (cx, cy) = out[1];
                out[1] = (cx + ux * trim, cy + uy * trim);
            }
        }
    }

    if end_decoration_length > 0.0 && out.len() >= 2 {
        let n = out.len();
        let (tip_x, tip_y) = out[n - 1];
        let (prev_x, prev_y) = out[n - 2];
        let vx = tip_x - prev_x;
        let vy = tip_y - prev_y;
        let len = (vx * vx + vy * vy).sqrt();
        if len > f64::EPSILON {
            let ux = vx / len;
            let uy = vy / len;
            let trim = end_decoration_length;
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

fn render_component_link_note(
    note: &ComponentNote,
    x: f64,
    y: f64,
    dim: &CompDim,
    svg: &mut SvgBuilder,
) {
    // `ComponentRoseNote.drawInternalU` truncates the text-box dimensions
    // before delegating the folded rectangle to `Opale`.
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

    let mut baseline = y + NOTE_MARGIN_Y + pm::ascent(LINK_FONT);
    for line in note.text.lines() {
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            line,
            &TextBase {
                x: x + NOTE_MARGIN_X1,
                y: baseline,
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
        baseline += text_render::label_height(line, LINK_FONT);
    }
}

fn render_attached_component_note(
    note: &ComponentNote,
    layout: &ComponentNoteLayout,
    uid: &NoOracleNoteUid,
    edge: Option<(&EdgePath, f64, f64)>,
    geometry: (ComponentNotePosition, Option<bool>),
    svg: &mut SvgBuilder,
) {
    let (position, note_is_first) = geometry;
    let center = (
        layout.x + layout.width / 2.0,
        layout.y + layout.height / 2.0,
    );
    let fallback = match position {
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
        .and_then(|(path, edge_dx, edge_dy)| {
            path.points
                .first()
                .zip(path.points.last())
                .map(|(first, last)| (first, last, edge_dx, edge_dy))
        })
        .map(|(first, last, edge_dx, edge_dy)| {
            let first = (
                round_svek_input_coord(first.0 + edge_dx),
                round_svek_input_coord(first.1 + edge_dy),
            );
            let last = (
                round_svek_input_coord(last.0 + edge_dx),
                round_svek_input_coord(last.1 + edge_dy),
            );
            match note_is_first {
                Some(true) => (first, last),
                Some(false) => (last, first),
                None => match position {
                    ComponentNotePosition::Top | ComponentNotePosition::Left => (first, last),
                    ComponentNotePosition::Bottom | ComponentNotePosition::Right => (last, first),
                },
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
    let path = match position {
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
        ComponentPackageKind::Component => {
            emit_layout_component_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
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
        ComponentPackageKind::Storage => {
            emit_layout_storage_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
        }
        ComponentPackageKind::Artifact => {
            emit_layout_artifact_cluster(svg, pos, &pkg.label, pkg.stereotype.as_deref(), &fill);
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
                | ComponentPackageKind::Storage
                | ComponentPackageKind::Artifact
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

fn emit_layout_component_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // PlantUML `ClusterDecoration.getTextBlock` delegates component groups to
    // `USymbolComponent2.asBig`. Its `drawComponent2` method paints the same
    // three-tab UML component mark used by a component leaf, while `asBig`
    // places stereotype/title content 13px below the solved cluster top.
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        fc(pos.height),
        fc(pos.width),
        fc(pos.x),
        fc(pos.y),
    ));
    let tab_x = pos.x + pos.width - ICON_TAB_RIGHT_OFFSET;
    let tab_y = pos.y + ICON_TAB_TOP_OFFSET;
    svg.raw(&format!(
        r#"<rect fill="{fill}" height="{}" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        fc(ICON_TAB_H),
        fc(ICON_TAB_W),
        fc(tab_x),
        fc(tab_y),
    ));
    for bar_top in [ICON_BAR_TOP_OFFSET_1, ICON_BAR_TOP_OFFSET_2] {
        svg.raw(&format!(
            r#"<rect fill="{fill}" height="{}" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
            fc(ICON_BAR_H),
            fc(ICON_BAR_W),
            fc(tab_x - ICON_BAR_LEFT_OFFSET),
            fc(tab_y + bar_top),
        ));
    }

    let stereotype_height = if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + (pos.width - width) / 2.0,
                y: pos.y + 13.0 + pm::ascent(FONT_SIZE),
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
            y: pos.y + 13.0 + stereotype_height + pm::ascent(FONT_SIZE),
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

fn emit_layout_usymbol_cluster_text(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    stereotype_top: f64,
    title_top: f64,
) {
    let stereotype_height = if let Some(stereotype) = stereotype {
        let text = format!("\u{00AB}{stereotype}\u{00BB}");
        let width = text_render::measure_no_underline(&text, FONT_SIZE, false);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &text,
            &TextBase {
                x: pos.x + (pos.width - width) / 2.0,
                y: pos.y + stereotype_top + pm::ascent(FONT_SIZE),
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
            y: pos.y + title_top + stereotype_height + pm::ascent(FONT_SIZE),
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

fn emit_layout_storage_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // Java `USymbolStorage.asBig` paints a round-70 rectangle and places
    // stereotype/title blocks at y=5 and y=7+stereotype-height.
    crate::deployment::emit_storage_with_stroke_width(
        svg, pos.x, pos.y, pos.width, pos.height, fill, "#181818", 1.0,
    );
    emit_layout_usymbol_cluster_text(svg, pos, label, stereotype, 5.0, 7.0);
}

fn emit_layout_artifact_cluster(
    svg: &mut SvgBuilder,
    pos: &ClusterPosition,
    label: &str,
    stereotype: Option<&str>,
    fill: &str,
) {
    // Java `USymbolArtifact.asBig` paints the folded-page symbol over the
    // solved cluster body and starts both text blocks at y=2.
    crate::deployment::emit_artifact_with_stroke_width(
        svg, pos.x, pos.y, pos.width, pos.height, fill, "#181818", 1.0,
    );
    emit_layout_usymbol_cluster_text(svg, pos, label, stereotype, 2.0, 2.0);
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
    fn numeric_attr(tag: &str, name: &str) -> f64 {
        let marker = format!(r#" {name}=""#);
        let value = tag
            .split_once(&marker)
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(value, _)| value)
            .unwrap_or_else(|| panic!("missing {name} in {tag}"));
        value
            .trim_end_matches("px")
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric {name} in {tag}"))
    }

    fn entity_rect<'a>(svg: &'a str, qualified_name: &str) -> &'a str {
        let marker = format!(r#"data-qualified-name="{qualified_name}""#);
        let entity = svg
            .split_once(&marker)
            .map(|(_, entity)| entity)
            .unwrap_or_else(|| panic!("missing entity {qualified_name}: {svg}"));
        entity
            .split("<rect ")
            .nth(1)
            .and_then(|tag| tag.split_once("/>"))
            .map(|(tag, _)| tag)
            .unwrap_or_else(|| panic!("missing rectangle for {qualified_name}: {svg}"))
    }

    fn path_data<'a>(svg: &'a str, qualified_name: &str) -> &'a str {
        let marker = format!(r#"data-qualified-name="{qualified_name}""#);
        let entity = svg
            .split_once(&marker)
            .map(|(_, entity)| entity)
            .unwrap_or_else(|| panic!("missing entity {qualified_name}: {svg}"));
        entity
            .split_once(r#"<path d=""#)
            .and_then(|(_, path)| path.split_once('"'))
            .map(|(path, _)| path)
            .unwrap_or_else(|| panic!("missing path for {qualified_name}: {svg}"))
    }

    fn path_min_x(path: &str) -> f64 {
        let coordinates: Vec<f64> = path
            .split(|character: char| {
                character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
            })
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .parse()
                    .unwrap_or_else(|_| panic!("non-numeric path coordinate {value} in {path}"))
            })
            .collect();
        coordinates
            .chunks_exact(2)
            .map(|point| point[0])
            .fold(f64::INFINITY, f64::min)
    }

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
    fn global_left_to_right_direction_controls_fresh_component_chain() {
        let input = concat!(
            "@startuml\n",
            "left to right direction\n",
            "component CopperRelay\n",
            "component VioletRelay\n",
            "component SaffronRelay\n",
            "component TealRelay\n",
            "CopperRelay --> VioletRelay\n",
            "VioletRelay --> SaffronRelay\n",
            "SaffronRelay --> TealRelay\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let root = svg.split_once('>').map_or(svg.as_str(), |(root, _)| root);

        assert!(
            numeric_attr(root, "width") > numeric_attr(root, "height"),
            "left-to-right rank direction must make a four-node chain wider than tall: {svg}"
        );
    }

    #[test]
    fn extension_link_uses_hollow_triangle_and_semantic_type() {
        let input = concat!(
            "@startuml\n",
            "interface RenamedPort\n",
            "component FreshAdapter\n",
            "FreshAdapter ..|> RenamedPort\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"data-link-type="extension""#), "{svg}");
        assert!(
            svg.contains(r#"<polygon fill="none" points=""#),
            "extension must use PlantUML's hollow 18x6 triangle: {svg}"
        );
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
    fn no_oracle_interface_keeps_renamed_java_group_context() {
        let input = "@startuml\ncomponent RenamedShell9701 {\n  interface \"Renamed Audit Port 9703\" as Port9703\n  component \"Renamed Worker 9709\" as Worker9709\n  Worker9709 - Port9703\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // `CommandCreateElementFull.executeArg` creates the interface through
        // `AbstractEntityDiagram.reallyCreateLeaf` in the current quark group.
        assert!(
            svg.contains(r#"data-qualified-name="RenamedShell9701.Port9703""#),
            "the interface must retain its Java group-qualified identity: {svg}"
        );
        assert!(
            svg.contains(r#"data-qualified-name="RenamedShell9701.Worker9709""#),
            "the component peer must share the same Java group context: {svg}"
        );
    }

    #[test]
    fn no_oracle_vertical_interface_reserves_hidden_label_shield_for_renamed_port() {
        let input = "@startuml\ncomponent \"Renamed Telemetry Sink 7301\" as Sink7301\ninterface \"Renamed Audit Port 7303\" as Port7303\nSink7301 -- Port7303\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference. `EntityImageDescription.getShield`
        // measures the hidden interface label, `SvekNode.appendHtml` reserves
        // its table envelope, and `Bibliotekon.getNodeUid` targets port `h`.
        assert!(svg.contains(r#"viewBox="0 0 268 189""#), "{svg}");
        let interface = svg
            .split("<ellipse")
            .nth(1)
            .and_then(|tag| tag.split_once("/>"))
            .map(|(tag, _)| tag)
            .expect("renamed interface ellipse");
        assert!((numeric_attr(interface, "cx") - 130.78).abs() < 0.01);
        assert!((numeric_attr(interface, "cy") - 142.49).abs() < 0.01);
        assert!(
            svg.contains(r#"d="M130.78,53.82 C130.78,78.64 130.78,117.18 130.78,133.62""#),
            "{svg}"
        );
    }

    #[test]
    fn no_oracle_leaf_cloud_normalizes_generated_path_envelope() {
        let input = "@startuml\ninterface I7307\ncloud \"Fresh Telemetry Archive 7309\" as Cloud7309\nI7307 --> Cloud7309\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let cloud_path = path_data(&svg, "Cloud7309");

        // Java `USymbolCloud.getSpecificFrontierForCloudNew` lets Bezier
        // controls protrude outside the nominal box. `LimitFinder.drawUPath`
        // includes that generated minimum before `SvekResult.calculateDimension`
        // moves the complete painted envelope to (6, 6).
        assert!(
            (path_min_x(cloud_path) - super::SVEK_CLUSTER_ORIGIN).abs() < 0.0001,
            "cloud path must define the left SVEK envelope: {svg}"
        );
    }

    #[test]
    fn no_oracle_single_strategy_uses_graphviz_for_heterogeneous_five_and_six_leaf_sets() {
        for (suffix, extra) in [
            ("Five", ""),
            (
                "Six",
                "component \"Sixth exceptionally wide archive 977\" as Leaf6\n",
            ),
        ] {
            let input = format!(
                "@startuml\n\
                 component \"Short relay 911\" as Leaf1\n\
                 component \"A much wider renamed broker 919\" as Leaf2\n\
                 component \"Medium sink 929\" as Leaf3\n\
                 component \"Tiny 937\" as Leaf4\n\
                 component \"Heterogeneous storage boundary 947\" as Leaf5\n\
                 {extra}\
                 @enduml"
            );
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            let svg = crate::render_svg(&diagram);
            let leaf_count = if suffix == "Five" { 5 } else { 6 };
            let positions: Vec<(f64, f64)> = (1..=leaf_count)
                .map(|index| {
                    let rect = entity_rect(&svg, &format!("Leaf{index}"));
                    (numeric_attr(rect, "x"), numeric_attr(rect, "y"))
                })
                .collect();

            // Java `SquareMaker.computeBranch` yields three columns for both
            // cardinalities. Invisible `Magma` links leave widths and final
            // coordinates to Graphviz.
            assert!(
                positions[..3]
                    .windows(2)
                    .all(|pair| (pair[0].1 - pair[1].1).abs() < 0.01 && pair[0].0 < pair[1].0),
                "{suffix} first rank should contain three ordered leaves: {svg}"
            );
            assert!(
                positions[3..]
                    .windows(2)
                    .all(|pair| (pair[0].1 - pair[1].1).abs() < 0.01 && pair[0].0 < pair[1].0),
                "{suffix} second rank should preserve declaration order: {svg}"
            );
            assert!(
                positions[3..].iter().all(|(_, y)| *y > positions[0].1),
                "{suffix} should have a second Graphviz rank: {svg}"
            );
        }
    }

    #[test]
    fn no_oracle_single_strategy_constrains_renamed_together_leaves_without_rewriting_spline() {
        let input = "@startuml\n\
                     together {\n\
                       component \"Renamed Anchor 1201\" as Anchor1201\n\
                       component \"Short peer 1213\" as Peer1213\n\
                       component \"A substantially wider peer 1217\" as Peer1217\n\
                       component \"Medium peer 1223\" as Peer1223\n\
                       component \"Tiny peer 1229\" as Peer1229\n\
                     }\n\
                     component \"External heterogeneous sink 1231\" as Sink1231\n\
                     Anchor1201 --> Sink1231\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let peer_positions: Vec<(f64, f64)> = [1213, 1217, 1223, 1229]
            .into_iter()
            .map(|suffix| {
                let rect = entity_rect(&svg, &format!("Peer{suffix}"));
                (numeric_attr(rect, "x"), numeric_attr(rect, "y"))
            })
            .collect();
        let first_rank_y = peer_positions[0].1;
        let second_rank_y = peer_positions[2].1;

        assert!((peer_positions[1].1 - first_rank_y).abs() < 0.01, "{svg}");
        assert!(second_rank_y > first_rank_y, "{svg}");
        assert!((peer_positions[3].1 - second_rank_y).abs() < 0.01, "{svg}");
        let link = svg
            .split(r#"id="Anchor1201-to-Sink1231""#)
            .next()
            .and_then(|prefix| prefix.rsplit("<path ").next())
            .expect("renamed crossing spline");
        assert!(
            link.contains(" C"),
            "Graphviz should supply the crossing cubic spline: {svg}"
        );
    }

    #[test]
    fn shield_center_offsets_follow_display_metrics_not_aliases_or_owners() {
        fn offsets(grouped: bool) -> Vec<f64> {
            let wrapper = if grouped {
                ("component RenamedShell9701 {\n", "}\n")
            } else {
                ("", "")
            };
            let source = format!(
                "@startuml\n{}\
                 interface \"IStorage\" as RenamedArchivePort9703\n\
                 interface \"IMessaging\" as RenamedDispatchPort9709\n\
                 {}\
                 component RenamedClient9719\n\
                 RenamedClient9719 --> RenamedArchivePort9703\n\
                 RenamedClient9719 --> RenamedDispatchPort9709\n\
                 @enduml",
                wrapper.0, wrapper.1
            );
            let rustuml_parser::diagram::Diagram::Component(diagram) =
                rustuml_parser::parse::parse(&source).unwrap()
            else {
                panic!("expected component diagram");
            };
            diagram
                .interfaces
                .iter()
                .map(|interface| {
                    let (shield_x, _) =
                        super::component_interface_shield(&diagram, interface).unwrap();
                    shield_x.round() - shield_x.floor()
                })
                .collect()
        }

        assert_eq!(offsets(false), [1.0, 0.0]);
        assert_eq!(offsets(true), [1.0, 0.0]);
    }

    #[test]
    fn mixed_interface_spacing_depends_on_rank_owner_not_interface_count() {
        let root_input = "@startuml\n\
                          component \"Root Producer A 9803\" as ProducerA\n\
                          component \"Root Subscriber A 9811\" as SubscriberA\n\
                          component \"Root Producer B 9817\" as ProducerB\n\
                          component \"Root Subscriber B 9829\" as SubscriberB\n\
                          interface \"Root Port A 9833\" as PortA\n\
                          interface \"Root Port B 9839\" as PortB\n\
                          ProducerA -( PortA\n\
                          SubscriberA --> PortA : heterogeneous stream A\n\
                          ProducerB -( PortB\n\
                          SubscriberB --> PortB : wider heterogeneous stream B\n\
                          @enduml";
        let clustered_input = "@startuml\n\
                               component RenamedBoundary9851 {\n\
                                 component \"Cluster Producer A 9857\" as ProducerA\n\
                                 component \"Cluster Subscriber A 9859\" as SubscriberA\n\
                                 component \"Cluster Producer B 9871\" as ProducerB\n\
                                 component \"Cluster Subscriber B 9883\" as SubscriberB\n\
                                 interface \"Cluster Port A 9887\" as PortA\n\
                                 interface \"Cluster Port B 9901\" as PortB\n\
                                 ProducerA -( PortA\n\
                                 SubscriberA --> PortA : heterogeneous stream A\n\
                                 ProducerB -( PortB\n\
                                 SubscriberB --> PortB : wider heterogeneous stream B\n\
                               }\n\
                               @enduml";
        let rustuml_parser::diagram::Diagram::Component(root) =
            rustuml_parser::parse::parse(root_input).unwrap()
        else {
            panic!("expected root component diagram");
        };
        let rustuml_parser::diagram::Diagram::Component(clustered) =
            rustuml_parser::parse::parse(clustered_input).unwrap()
        else {
            panic!("expected clustered component diagram");
        };

        let link_style = super::ComponentLinkRenderStyle {
            stroke: super::STROKE.to_string(),
            stroke_width: 1.0,
            dash: None,
            font_color: super::TEXT_COLOR.to_string(),
            font_family: "sans-serif".to_string(),
            font_size: super::LINK_FONT,
        };
        let root_styles = vec![link_style.clone(); root.connections.len()];
        let clustered_styles = vec![link_style; clustered.connections.len()];
        let root_spacing = super::component_no_oracle_spacing(&root, &root_styles);
        let clustered_spacing = super::component_no_oracle_spacing(&clustered, &clustered_styles);
        assert_eq!(root_spacing, (18.0, false));
        assert_ne!(clustered_spacing.0, root_spacing.0);
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
    fn mixed_creole_uses_measured_text_block_height_and_first_run_baselines() {
        let input = "@startuml\n\
                     component \"<size:19>Renamed Quartz 101</size> tail\" as MixedNode101\n\
                     interface \"\"\"RenamedMono103\"\" tail\" as MixedPort103\n\
                     MixedNode101 -- MixedPort103\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let component = component_diagram
            .components
            .iter()
            .find(|component| component.id == "MixedNode101")
            .expect("renamed component");
        let metrics =
            super::component_text_metrics(component, super::FONT_SIZE, "sans-serif", false);
        let dim = super::calc_component_dim_with_metrics(component, &metrics);
        assert!(metrics.label_height > super::LINE_HEIGHT);
        assert_eq!(dim.height, super::COMPONENT_BASE_H + metrics.label_height);

        let svg = crate::render_svg(&diagram);
        let component_entity = svg
            .split_once(r#"data-qualified-name="MixedNode101""#)
            .map(|(_, entity)| entity)
            .expect("renamed component entity");
        let component_rect = component_entity
            .split("<rect ")
            .nth(1)
            .and_then(|tag| tag.split_once("/>"))
            .map(|(tag, _)| tag)
            .expect("component rectangle");
        let component_text = component_entity
            .split("<text ")
            .nth(1)
            .and_then(|tag| tag.split_once('>'))
            .map(|(tag, _)| tag)
            .expect("component text");
        let component_y = numeric_attr(component_rect, "y");
        let component_text_y = numeric_attr(component_text, "y");
        let expected_component_text_y =
            component_y + super::COMPONENT_MARGIN_TOP + metrics.label_first_baseline_ascent;
        assert!((component_text_y - expected_component_text_y).abs() < 0.01);

        let interface = component_diagram
            .interfaces
            .iter()
            .find(|interface| interface.id == "MixedPort103")
            .expect("renamed interface");
        let interface_entity = svg
            .split_once(r#"data-qualified-name="MixedPort103""#)
            .map(|(_, entity)| entity)
            .expect("renamed interface entity");
        let interface_ellipse = interface_entity
            .split("<ellipse ")
            .nth(1)
            .and_then(|tag| tag.split_once("/>"))
            .map(|(tag, _)| tag)
            .expect("interface ellipse");
        let interface_text = interface_entity
            .split("<text ")
            .nth(1)
            .and_then(|tag| tag.split_once('>'))
            .map(|(tag, _)| tag)
            .expect("interface text");
        let interface_cy = numeric_attr(interface_ellipse, "cy");
        let interface_text_y = numeric_attr(interface_text, "y");
        let expected_interface_text_y = interface_cy
            + (super::IFACE_NODE_SIZE - super::IFACE_CENTER_OFFSET)
            + super::IFACE_LABEL_GAP
            + crate::text_render::label_first_baseline_ascent_with_family(
                &interface.label,
                super::FONT_SIZE,
                "sans-serif",
            );
        assert!((interface_text_y - expected_interface_text_y).abs() < 0.01);
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
    fn no_oracle_mixed_root_and_cluster_share_svek_painted_envelope() {
        let input = "@startuml\nfolder \"Renamed Processing Vault 4103\" {\n  component \"Primary Relay 4111\" as Relay4111\n  component \"Audit Relay 4127\" as Audit4127\n  component \"Cold Archive 4133\" as Archive4133\n  Relay4111 --> Audit4127 : mirrors\n  Audit4127 --> Archive4133 : seals\n}\ncomponent \"External Coordinator 4153\" as Coordinator4153\nCoordinator4153 --> Relay4111 : dispatches\nCoordinator4153 --> Archive4133 : verifies\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference. `SvekResult.calculateDimension`
        // measures the root coordinator together with the folder and moves
        // their shared painted minimum to the six-pixel SVEK frame.
        assert!(svg.contains(r#"viewBox="0 0 367 475""#), "{svg}");
        let coordinator = svg
            .split("<!--entity Coordinator4153-->")
            .nth(1)
            .and_then(|tail| tail.split_once("</g>"))
            .map(|(entity, _)| entity)
            .expect("renamed root coordinator entity");
        assert!(
            coordinator.contains(r#"data-qualified-name="Coordinator4153""#)
                && coordinator.contains(r#"y="7""#),
            "the root leaf must remain inside the shared painted envelope: {svg}"
        );
        assert!(
            svg.contains(r#"data-qualified-name="Renamed Processing Vault 4103""#)
                && svg.contains(r#"M8.5,116.48"#),
            "the cluster must keep its solved position after normalization: {svg}"
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
    fn pure_css_component_entities_keep_creation_snapshots() {
        let input = "@startuml\n\
                     <style>\n\
                     component {\n\
                       BackgroundColor #E1F5FE\n\
                       LineColor #0277BD\n\
                       FontColor #01579B\n\
                     }\n\
                     </style>\n\
                     component \"Early Relay 3109\" as EarlyRelay3109\n\
                     <style>\n\
                     component {\n\
                       BackgroundColor #F3E5F5\n\
                       LineColor #6A1B9A\n\
                       FontColor #4A148C\n\
                     }\n\
                     </style>\n\
                     component \"Late Relay 3119\" as LateRelay3119\n\
                     EarlyRelay3109 --> LateRelay3119\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let early = entity_rect(&svg, "EarlyRelay3109");
        let late = entity_rect(&svg, "LateRelay3119");

        assert!(early.contains(r##"fill="#E1F5FE""##), "{svg}");
        assert!(early.contains("stroke:#0277BD;"), "{svg}");
        assert!(late.contains(r##"fill="#F3E5F5""##), "{svg}");
        assert!(late.contains("stroke:#6A1B9A;"), "{svg}");
    }

    #[test]
    fn component_css_stereotypes_use_normalized_entity_signatures() {
        let input = "@startuml\n\
                     <style>\n\
                     component {\n\
                       BackgroundColor #ECEFF1\n\
                       LineColor #455A64\n\
                       .Critical.Port {\n\
                         BackgroundColor #FFCDD2\n\
                         LineColor #B71C1C\n\
                         LineThickness 2\n\
                       }\n\
                     }\n\
                     </style>\n\
                     component \"Qualified Relay 3163\" as QualifiedRelay3163 <<critical_port>>\n\
                     component \"Plain Relay 3167\" as PlainRelay3167\n\
                     QualifiedRelay3163 --> PlainRelay3167\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let qualified = entity_rect(&svg, "QualifiedRelay3163");
        let plain = entity_rect(&svg, "PlainRelay3167");

        assert!(qualified.contains(r##"fill="#FFCDD2""##), "{svg}");
        assert!(
            qualified.contains("stroke:#B71C1C;stroke-width:2;"),
            "{svg}"
        );
        assert!(plain.contains(r##"fill="#ECEFF1""##), "{svg}");
        assert!(plain.contains("stroke:#455A64;"), "{svg}");
    }

    #[test]
    fn legacy_refreshes_entities_but_links_keep_their_own_builders() {
        let input = "@startuml\n\
                     <style>\n\
                     component {\n\
                       BackgroundColor #E3F2FD\n\
                       LineColor #1565C0\n\
                     }\n\
                     arrow {\n\
                       LineColor #C62828\n\
                       FontColor #C62828\n\
                       LineThickness 2\n\
                     }\n\
                     </style>\n\
                     component \"Early Intake 3203\" as EarlyIntake3203\n\
                     component \"Middle Queue 3209\" as MiddleQueue3209\n\
                     EarlyIntake3203 --> MiddleQueue3209 : before refresh\n\
                     skinparam componentBorderColor #00695C\n\
                     <style>\n\
                     component {\n\
                       BackgroundColor #E0F2F1\n\
                       LineColor #00695C\n\
                     }\n\
                     arrow {\n\
                       LineColor #4527A0\n\
                       FontColor #4527A0\n\
                       LineThickness 3\n\
                     }\n\
                     </style>\n\
                     component \"Late Archive 3217\" as LateArchive3217\n\
                     MiddleQueue3209 --> LateArchive3217 : after refresh\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        for id in ["EarlyIntake3203", "MiddleQueue3209", "LateArchive3217"] {
            let rect = entity_rect(&svg, id);
            assert!(rect.contains(r##"fill="#E0F2F1""##), "{svg}");
            assert!(rect.contains("stroke:#00695C;"), "{svg}");
        }
        assert!(
            svg.contains(
                r#"id="EarlyIntake3203-to-MiddleQueue3209" style="stroke:#C62828;stroke-width:2;"#
            ),
            "{svg}"
        );
        assert!(
            svg.contains(
                r#"id="MiddleQueue3209-to-LateArchive3217" style="stroke:#4527A0;stroke-width:3;"#
            ),
            "{svg}"
        );
    }

    #[test]
    fn component_shadow_and_open_dash_pair_share_painted_style_values() {
        let input = "@startuml\n\
                     left to right direction\n\
                     <style>\n\
                     document {\n\
                       BackgroundColor #E8EAF6\n\
                       Margin 3 7 11 13\n\
                     }\n\
                     component {\n\
                       BackgroundColor #E0F2F1\n\
                       LineColor #00695C\n\
                       Shadowing 4\n\
                     }\n\
                     arrow {\n\
                       LineColor #7B1FA2\n\
                       LineThickness 3\n\
                       LineStyle 7-4\n\
                     }\n\
                     </style>\n\
                     component \"Shadow Source 3301\" as ShadowSource3301\n\
                     component \"Shadow Sink 3307\" as ShadowSink3307\n\
                     ShadowSource3301 --> ShadowSink3307 : weighted route\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let seed_source = component_diagram.meta.source.as_deref().unwrap_or("");
        let filter_id = crate::filter_registry::shadow_id_for(seed_source);
        let svg = crate::render_svg(&diagram);
        let control = input.replace("Margin 3 7 11 13\n", "");
        let control_diagram = rustuml_parser::parse::parse(&control).unwrap();
        let control_svg = crate::render_svg(&control_diagram);

        assert!(
            svg.contains(&crate::filter_registry::shadow_filter_def(&filter_id)),
            "{svg}"
        );
        assert_eq!(
            svg.matches(&format!(r#"filter="url(#{filter_id})""#))
                .count(),
            2,
            "{svg}"
        );
        assert!(
            svg.contains("stroke:#7B1FA2;stroke-width:3;stroke-dasharray:7,4;"),
            "{svg}"
        );
        assert!(svg.contains("background:#E8EAF6;"), "{svg}");
        let styled_rect = entity_rect(&svg, "ShadowSource3301");
        let control_rect = entity_rect(&control_svg, "ShadowSource3301");
        assert_eq!(
            numeric_attr(styled_rect, "x") - numeric_attr(control_rect, "x"),
            13.0
        );
        assert_eq!(
            numeric_attr(styled_rect, "y") - numeric_attr(control_rect, "y"),
            3.0
        );
        let styled_root = svg.split_once('>').map(|(root, _)| root).unwrap();
        let control_root = control_svg.split_once('>').map(|(root, _)| root).unwrap();
        assert_eq!(
            numeric_attr(styled_root, "width") - numeric_attr(control_root, "width"),
            14.0
        );
        assert_eq!(
            numeric_attr(styled_root, "height") - numeric_attr(control_root, "height"),
            9.0
        );
    }

    #[test]
    fn component_inline_gradients_preserve_every_policy_and_reuse() {
        let input = r##"@startuml
component VerticalFresh3401 #102030-#405060
component HorizontalFresh3407 #A0B0C0|#D0E0F0
component RisingFresh3413 #123456\#ABCDEF
component FallingFresh3419 #654321/#FEDCBA
component VerticalReuse3421 #102030-#405060
VerticalFresh3401 --> HorizontalFresh3407
HorizontalFresh3407 --> RisingFresh3413
RisingFresh3413 --> FallingFresh3419
FallingFresh3419 --> VerticalReuse3421
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let source = component_diagram.meta.source.as_deref().unwrap_or("");
        let first_id = crate::filter_registry::gradient_id_for(source, 0);
        let svg = crate::render_svg(&diagram);

        assert_eq!(svg.matches("<linearGradient ").count(), 4, "{svg}");
        assert_eq!(svg.matches(r#"fill="url(#"#).count(), 20, "{svg}");
        assert!(svg.contains(&format!(r#"id="{first_id}" x1="50%" x2="50%""#)));
        for endpoints in [
            r#"x1="50%" x2="50%" y1="0%" y2="100%""#,
            r#"x1="0%" x2="100%" y1="50%" y2="50%""#,
            r#"x1="0%" x2="100%" y1="100%" y2="0%""#,
            r#"x1="0%" x2="100%" y1="0%" y2="100%""#,
        ] {
            assert!(svg.contains(endpoints), "missing {endpoints}: {svg}");
        }
        assert_eq!(
            svg.matches(&format!(r#"fill="url(#{first_id})""#)).count(),
            8
        );
    }

    #[test]
    fn component_gradient_inventory_follows_selected_snapshots_and_drops_dead_values() {
        let input = r##"@startuml
<style>
component {
  BackgroundColor #112233-#445566
}
document {
  BackgroundColor #010203|#040506
}
</style>
component "Early Relay 3433" as EarlyRelay3433
component "Early Sink 3439" as EarlySink3439
<style>
component {
  BackgroundColor #778899|#AABBCC
}
</style>
component "Late Relay 3449" as LateRelay3449
component "Inline Relay 3457" as InlineRelay3457 #DDEEFF/#102132
<style>
component {
  BackgroundColor #314253\#647586
}
</style>
EarlyRelay3433 --> EarlySink3439
EarlySink3439 --> LateRelay3449
LateRelay3449 --> InlineRelay3457
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(svg.matches("<linearGradient ").count(), 3, "{svg}");
        assert_eq!(svg.matches(r#"fill="url(#"#).count(), 16, "{svg}");
        for selected in [
            "#112233", "#445566", "#778899", "#AABBCC", "#DDEEFF", "#102132",
        ] {
            assert!(
                svg.contains(selected),
                "missing selected stop {selected}: {svg}"
            );
        }
        for dead in ["#010203", "#040506", "#314253", "#647586"] {
            assert!(
                !svg.contains(dead),
                "unpainted inventory value survived: {dead}: {svg}"
            );
        }
    }

    #[test]
    fn component_stereotype_skinparam_separator_aliases_follow_source_order() {
        for (first, second, expected, rejected) in [
            ("Red", "Blue", "#0000FF", "#FF0000"),
            ("Blue", "Red", "#FF0000", "#0000FF"),
        ] {
            let input = format!(
                "@startuml\n\
                 skinparam componentBackgroundColor<<relay_71>> {first}\n\
                 skinparam componentBackgroundColor<<relay.71>> {second}\n\
                 component \"Scoped Relay\" as Scoped <<relay_71>>\n\
                 component \"Other Relay\" as Other <<relay_72>>\n\
                 @enduml"
            );
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            let svg = crate::render_svg(&diagram);

            assert_eq!(
                svg.matches(&format!(r##"fill="{expected}""##)).count(),
                4,
                "{svg}"
            );
            assert!(!svg.contains(&format!(r##"fill="{rejected}""##)), "{svg}");
            assert_eq!(svg.matches(r##"fill="#F1F1F1""##).count(), 4, "{svg}");
            assert!(svg.contains("relay_71"), "{svg}");
            assert!(svg.contains("relay_72"), "{svg}");
        }
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
            metrics.label_height
                + metrics.stereotype_heights.iter().sum::<f64>()
                + super::RECTANGLE_MARGIN_Y * 2.0
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
        let text_height = crate::text_render::label_height(&component.label, super::FONT_SIZE);

        assert_eq!(dim.width, text_width + super::DATABASE_MARGIN_X * 2.0);
        assert_eq!(
            dim.height,
            super::DATABASE_MARGIN_TOP + text_height + super::DATABASE_MARGIN_BOTTOM
        );

        let expected_w =
            super::MARGIN + dim.width + super::DATABASE_RENDER_OVERFLOW_X + super::SVEK_CANVAS_PAD;
        let expected_h =
            super::MARGIN + dim.height + super::DATABASE_RENDER_OVERFLOW_Y + super::SVEK_CANVAS_PAD;
        let positions = [(super::MARGIN, super::MARGIN)];
        let dimensions = [dim];
        let (canvas_w, canvas_h) = super::compute_no_oracle_canvas(super::NoOracleCanvas {
            diagram: component_diagram,
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
            endpoint_label_layouts: &[],
            link_note_unpainted_right_edges: &[],
            middle_label_edges: &[],
            component_shadows: &[0.0],
        });
        assert_eq!(canvas_w, expected_w);
        assert_eq!(canvas_h, expected_h);
    }

    #[test]
    fn artifact_symbol_uses_java_margins_and_svek_envelope_for_renamed_label() {
        let input = "@startuml\ncomponent RenamedGateway\nartifact RenamedBuildManifest\nRenamedGateway --> RenamedBuildManifest\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component_diagram) = &diagram else {
            panic!("expected component diagram");
        };
        let artifact = component_diagram
            .components
            .iter()
            .find(|component| matches!(component.kind, super::ComponentElementKind::Artifact))
            .expect("renamed artifact");
        let dim = super::calc_component_dim(artifact);
        let text_width = crate::text_render::measure(&artifact.label, super::FONT_SIZE, false);
        let text_height = crate::text_render::label_height(&artifact.label, super::FONT_SIZE);

        assert_eq!(
            dim.width,
            text_width + super::ARTIFACT_MARGIN_LEFT + super::ARTIFACT_MARGIN_RIGHT
        );
        assert_eq!(
            dim.height,
            text_height + super::ARTIFACT_MARGIN_TOP + super::ARTIFACT_MARGIN_BOTTOM
        );

        // Fresh Java 1.2026.3beta6 render of this renamed perturbation.
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(r#"width="211px""#), "{svg}");
        assert!(svg.contains(r#"height="166px""#), "{svg}");
        assert!(svg.contains(">RenamedBuildManifest</text>"), "{svg}");
    }

    #[test]
    fn no_oracle_queue_and_storage_use_java_leaf_symbol_margins() {
        // Fresh Java PlantUML reference. `USymbolQueue.asSmall/drawQueue`
        // applies (5,15,5,5) margins; `USymbolStorage.asSmall/drawStorage`
        // applies ten-pixel margins and a 70px rounded diameter.
        let input = "@startuml\n\
                     queue \"Telemetry Buffer 8803\" as Buffer8803\n\
                     storage \"Archive Capsule 8819\" as Archive8819\n\
                     component \"Relay 8831\" as Relay8831\n\
                     Buffer8803 --> Archive8819\n\
                     Archive8819 --> Relay8831\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"width="193px""#) && svg.contains(r#"height="249px""#),
            "the renamed leaf-symbol chain must retain Java's envelope: {svg}"
        );
        assert!(
            svg.contains(r#"M11,6 L173"#)
                && svg.contains(r##"<rect fill="#F1F1F1" height=""##)
                && svg.contains(r#"rx="35" ry="35""#),
            "queue and storage primitives must follow their USymbol models: {svg}"
        );
        assert!(
            svg.contains(r#"d="M92.24,32.68 C92.24,48.15 92.24,68.63 92.24,86.16""#)
                && svg.contains(r#"d="M92.24,129.06 C92.24,145.54 92.24,164.42 92.24,182.48""#),
            "the SVEK layout must consume the symbol-specific dimensions: {svg}"
        );
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
    fn no_oracle_center_label_margin_extends_svek_frame_for_renamed_link() {
        let label = "Renamed telemetry exchange 967";
        let input = format!(
            "@startuml\nskinparam componentArrowFontSize 17\ncomponent \"Ingress 947\" as Ingress947\ncomponent \"Archive 953\" as Archive953\nIngress947 --> Archive953 : {label}\n@enduml"
        );
        let diagram = rustuml_parser::parse::parse(&input).unwrap();
        let svg = crate::render_svg(&diagram);

        let root = svg.split_once('>').map(|(root, _)| root).expect("SVG root");
        let label_tag = svg
            .split("<text ")
            .find(|tag| tag.contains(&format!(">{label}</text>")))
            .expect("renamed center label");
        let canvas_width = numeric_attr(root, "width");
        let text_right = numeric_attr(label_tag, "x") + numeric_attr(label_tag, "textLength");
        let required_width =
            (text_right + super::LINK_LABEL_MARGIN + super::SVEK_CANVAS_PAD).ceil();

        assert_eq!(
            canvas_width, required_width,
            "SvekEdge's one-pixel right label margin must participate in the canvas: {svg}"
        );
    }

    #[test]
    fn resolved_arrow_font_sizes_the_same_graphviz_placeholder_it_paints() {
        let input = "@startuml\n\
                     <style>\n\
                     root {\n\
                       Margin 10\n\
                       Padding 6\n\
                     }\n\
                     </style>\n\
                     skinparam Padding 5\n\
                     skinparam defaultFontName Verdana\n\
                     skinparam defaultFontSize 12\n\
                     skinparam dpi 100\n\
                     component AxisSource\n\
                     component AxisTarget\n\
                     AxisSource --> AxisTarget : axis label\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"style="width:169px;height:240px;background:#FFFFFF;""#)
                && svg.contains(r#"width="169.7917px""#),
            "the Verdana label width must drive the hidden table and root frontier: {svg}"
        );
    }

    #[test]
    fn no_oracle_mixed_rank_labels_keep_java_fixed_table_spacing_in_a_renamed_chain() {
        // Fresh Java PlantUML reference. `Cluster.appendRankSame` places both
        // edges on one rank; `SvekEdge.addVisibilityModifier/appendTable`
        // serializes the short and long labels as fixed HTML tables.
        let input = "@startuml\n\
                     component \"Renamed Intake 7301\" as Intake7301\n\
                     component \"Renamed Broker 7303\" as Broker7303\n\
                     component \"Renamed Archive 7307\" as Archive7307\n\
                     Intake7301 -> Broker7303 : sync\n\
                     Broker7303 -> Archive7307 : publishes audit events\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"width="840px""#) && svg.contains(r#"height="67px""#),
            "renamed three-node chain must retain Java's solved envelope: {svg}"
        );
        assert!(
            svg.contains(r#"d="M195.93,30.25 C216.66,30.25 232.63,30.25 253.42,30.25""#)
                && svg.contains(r#"d="M451.15,30.25 C505.5,30.25 567.42,30.25 622.31,30.25""#),
            "both fixed-table splines must retain Java's same-rank spacing: {svg}"
        );
        for (text, expected_x) in [("sync", 213.74), ("publishes audit events", 469.29)] {
            let label = svg
                .split("<text ")
                .find(|tag| tag.contains(&format!(">{text}</text>")))
                .unwrap_or_else(|| panic!("renderer-owned {text:?} label missing: {svg}"));
            assert!(
                (numeric_attr(label, "x") - expected_x).abs() < 0.01
                    && (numeric_attr(label, "y") - 23.8184).abs() < 0.01,
                "renderer-owned text must stay centered in each solved table: {svg}"
            );
        }
    }

    #[test]
    fn no_oracle_mixed_interface_rank_ignores_vertical_label_width() {
        // Fresh Java PlantUML reference. `DotStringFactory.getHorizontalDzeta`
        // asks each `SvekEdge` for horizontal separation, and the vertically
        // labelled edge returns zero while the socket edge retains rank=same.
        let input = "@startuml\n\
                     component \"Renamed Producer 9107\" as Producer9107 #PaleGreen\n\
                     component \"Renamed Subscriber 9133\" as Subscriber9133 #Wheat\n\
                     interface \"Renamed Service Port 9173\" as Port9173\n\
                     Producer9107 -( Port9173\n\
                     Subscriber9133 --> Port9173 : consumes telemetry stream\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"width="450px""#),
            "the vertical label must not widen the horizontal socket rank: {svg}"
        );
        assert!(
            svg.contains(r#"d="M214.71,153.74 C226.26,153.74 227.82,153.74 239.38,153.74""#)
                && svg.contains(r#"d="M258.64,53.81 C258.64,81.12 258.64,119.75 258.64,138.27""#),
            "the renamed mixed-rank perturbation must retain Java's routed splines: {svg}"
        );
    }

    #[test]
    fn no_oracle_renamed_self_link_uses_svek_autolink_label_margin() {
        let label = "renamed feedback circuit 971";
        let input = format!(
            "@startuml\ncomponent \"Relay 967\" as Relay967\nRelay967 --> Relay967 : {label}\n@enduml"
        );
        let diagram = rustuml_parser::parse::parse(&input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(
            super::svek_link_label_margin("Relay967", "Relay967"),
            super::SELF_LINK_LABEL_MARGIN
        );
        assert_eq!(
            super::svek_link_label_margin("Relay967", "Archive977"),
            super::LINK_LABEL_MARGIN
        );

        let root = svg.split_once('>').map(|(root, _)| root).expect("SVG root");
        let label_tag = svg
            .split("<text ")
            .find(|tag| tag.contains(&format!(">{label}</text>")))
            .expect("renamed self-link label");
        let canvas_width = numeric_attr(root, "width");
        let text_right = numeric_attr(label_tag, "x") + numeric_attr(label_tag, "textLength");
        let required_width =
            (text_right + super::SELF_LINK_LABEL_MARGIN + super::SVEK_CANVAS_PAD).floor();

        assert_eq!(
            canvas_width, required_width,
            "SvekEdge's six-pixel autolink margin must own the label envelope: {svg}"
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
    fn no_oracle_socket_extremities_follow_java_geometry_for_renamed_links() {
        // Coordinates below were captured from the Java PlantUML oracle:
        // `DotPath.getMiddle`, `MiddleCircleCircled.drawU`, and
        // `ExtremityParenthesis.drawU/getDecorationLength`.
        let middle_input = "@startuml\ncomponent \"Renamed Socket Producer 1009\" as Producer1009\ncomponent \"Renamed Socket Consumer 1013\" as Consumer1013\nProducer1009 -(0)- Consumer1013\n@enduml";
        let middle_diagram = rustuml_parser::parse::parse(middle_input).unwrap();
        let middle_svg = crate::render_svg(&middle_diagram);

        assert!(
            middle_svg.contains(r#"width="286px""#) && middle_svg.contains(r#"height="173px""#),
            "renamed middle-eye probe must retain Java's canvas: {middle_svg}"
        );
        assert!(
            middle_svg.contains(r#"M132.6189,90.5898 A10,10 0 0 0 146.7611 90.5898"#)
                && middle_svg.contains(r#"M146.7611,76.4477 A10,10 0 0 0 132.6189 76.4477"#),
            "MiddleCircleCircled arcs must follow DotPath.getMiddle(): {middle_svg}"
        );

        let endpoint_input = "@startuml\ncomponent \"Renamed Parenthesis Source 1021\" as Source1021\ncomponent \"Renamed Parenthesis Sink 1031\" as Sink1031\nSource1021 -( Sink1031\n@enduml";
        let endpoint_diagram = rustuml_parser::parse::parse(endpoint_input).unwrap();
        let endpoint_svg = crate::render_svg(&endpoint_diagram);

        assert!(
            endpoint_svg.contains(r#"width="588px""#) && endpoint_svg.contains(r#"height="67px""#),
            "renamed endpoint-parenthesis probe must retain Java's canvas: {endpoint_svg}"
        );
        assert!(
            endpoint_svg.contains(r#"M281.92,30.25 C293.31,30.25 294.69,30.25 306.08,30.25"#)
                && endpoint_svg.contains(r#"M313.0018,21.7928 A9,9 0 0 0 313.0018 38.7072"#),
            "ExtremityParenthesis must trim by 10px and paint its 140-degree arc: {endpoint_svg}"
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
    fn no_oracle_left_link_prioritizes_renamed_inverted_cluster_start() {
        let input = "@startuml\n\
                     package \"Renamed Intake Boundary 701\" as Intake701 {\n\
                       component \"Telemetry Worker 709\" as Worker709\n\
                     }\n\
                     package \"Renamed Archive Boundary 719\" as Archive719 {\n\
                       component \"Audit Worker 727\" as Worker727\n\
                     }\n\
                     Worker709 -left-> Worker727\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML 1.2026.3beta6 reference. The complete SVG is
        // structurally equivalent; these values pin `CommandLinkElement.executeArg`
        // plus `Cluster.getNodesOrderedTop/getNodesOrderedWithoutTop`.
        assert!(
            svg.contains(r#"viewBox="0 0 527 118""#)
                && svg.contains(r#"d="M242.2,64.25 C260.88,64.25 273.56,64.25 292.24,64.25""#),
            "the inverted start must place Archive719 left of Intake701: {svg}"
        );
        assert!(
            svg.contains(r#"id="Worker727-backto-Worker709""#),
            "the renamed perturbation must retain Java's inverted link identity: {svg}"
        );
    }

    #[test]
    fn no_oracle_cluster_rank_same_wraps_a_renamed_four_node_chain() {
        // Fresh Java PlantUML reference. `Cluster.appendRankSame/getRankSame`
        // emits these horizontal constraints inside the owning cluster.
        let input = "@startuml\nfolder \"Renamed Integration Fleet 2026\" as Fleet2026 {\n  component \"Ingress North 17\" as Ingress17\n  component \"Policy Middle 23\" as Policy23\n  component \"Transform Middle 31\" as Transform31\n  component \"Archive South 43\" as Archive43\n}\nIngress17 -right-> Policy23\nPolicy23 -right-> Transform31\nTransform31 -right-> Archive43\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"width="802px""#) && svg.contains(r#"height="118px""#),
            "the owning folder must wrap the complete horizontal rank: {svg}"
        );
        assert!(
            svg.contains(r#"d="M176.48,"#) && svg.contains(r#"d="M580.64,"#),
            "the four-node perturbation must retain Java's in-cluster routing: {svg}"
        );
    }

    #[test]
    fn no_oracle_cross_cluster_horizontal_link_uses_svek_zero_minlen() {
        // Fresh Java PlantUML reference. `SvekEdge.appendLine` emits
        // `minlen=0` when a one-step horizontal link crosses sibling
        // `Cluster.appendRankSame` scopes.
        let input = "@startuml\ncloud \"Renamed Intake Cloud 271\" as Intake271 {\n  component \"Ingress Relay 277\" as Relay277 #PaleGreen\n}\nnode \"Renamed Archive Node 281\" as Archive281 {\n  component \"Audit Sink 283\" as Sink283 #LightBlue\n}\nRelay277 -right-> Sink283\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"width="548px""#) && svg.contains(r#"height="133px""#),
            "sibling clusters must retain Java's horizontal SVEK canvas: {svg}"
        );
        assert!(
            svg.contains(r#"data-qualified-name="Intake271.Relay277""#)
                && svg.contains(r#"data-qualified-name="Archive281.Sink283""#),
            "renamed members must remain owned by their source clusters: {svg}"
        );
        assert!(
            svg.contains(r#"id="Relay277-to-Sink283""#),
            "the cross-cluster spline must retain Java's logical endpoints: {svg}"
        );
    }

    #[test]
    fn no_oracle_hide_and_remove_follow_distinct_svek_lifecycles() {
        let input = "@startuml\n\
                     component \"Renamed Hidden Relay 9101\" as Hidden9101 <<retired_9101>>\n\
                     component \"Renamed Visible Broker 9103\" as Broker9103\n\
                     component \"Renamed Removed Sink 9109\" as Removed9109\n\
                     component \"Renamed Visible Archive 9113\" as Archive9113\n\
                     Hidden9101 --> Broker9103\n\
                     Broker9103 --> Removed9109\n\
                     Broker9103 --> Archive9113\n\
                     hide <<retired_9101>>\n\
                     remove Removed9109\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML beta reference. `GraphvizImageBuilder`
        // excludes removed nodes and links before layout. `SvekResult.drawU`
        // instead paints hidden nodes and inherited-hidden links through
        // `UHidden`, which `LimitFinder` deliberately ignores.
        assert!(svg.contains(r#"viewBox="0 0 267 296""#), "{svg}");
        assert!(
            svg.contains("<!--entity Hidden9101-->")
                && svg.contains("<!--entity Broker9103-->")
                && !svg.contains("Renamed Hidden Relay 9101")
                && !svg.contains("Removed9109"),
            "{svg}"
        );
        assert!(
            svg.contains(r#"data-qualified-name="Broker9103" data-source-line="2" id="ent0003""#)
                && svg.contains(
                    r#"data-qualified-name="Archive9113" data-source-line="4" id="ent0005""#
                ),
            "{svg}"
        );
        assert!(
            svg.contains("<!--link Hidden9101 to Broker9103-->")
                && svg.contains("<!--link Broker9103 to Archive9113-->")
                && svg.contains(r#"data-source-line="7" id="lnk8""#),
            "{svg}"
        );
    }

    #[test]
    fn no_oracle_page_chrome_centers_a_renamed_pair_inside_header_and_title() {
        let input = "@startuml\n\
                     header Observatory transport channel 421 for the southern relay\n\
                     title\n\
                       Fresh Relay Catalogue 431\n\
                       Revision 433\n\
                     end title\n\
                     component \"Ingress 443\" as Ingress443\n\
                     component \"Archive 449\" as Archive449\n\
                     Ingress443 --> Archive449 : forwards\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference. `DiagramChromeFactory12026.create`
        // applies the title decorator before the outer header/footer decorator;
        // `DecorateEntityImage` uses the widest block and centers the raw body.
        assert!(svg.contains(r#"viewBox="0 0 285 257""#), "{svg}");
        let header = svg
            .find(r#"<g class="header" data-source-line="1">"#)
            .unwrap();
        let title = svg
            .find(r#"<g class="title" data-source-line="2">"#)
            .unwrap();
        let ingress = svg.find(r#"data-qualified-name="Ingress443""#).unwrap();
        assert!(header < title && title < ingress, "{svg}");
        assert!(svg.contains(r#"id="Ingress443-to-Archive449""#), "{svg}");
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
    fn component_backed_storage_and_artifact_clusters_keep_usymbols() {
        let input = "@startuml\n\
                     left to right direction\n\
                     storage \"Durable Vault 5101\" as Vault5101 <<durable_5103>> #LightBlue {\n\
                       artifact \"Receipt Bundle 5107\" as Receipts5107 {\n\
                         component \"Inner Worker 5113\" as Inner5113\n\
                       }\n\
                     }\n\
                     component \"Audit Peer 5119\" as Peer5119\n\
                     Vault5101 --> Peer5119 : grants\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(component) = &diagram else {
            panic!("expected the component-backed DESCRIPTION path");
        };
        assert_eq!(
            component.packages[0].kind,
            super::ComponentPackageKind::Storage
        );
        assert_eq!(
            component.packages[0].packages[0].kind,
            super::ComponentPackageKind::Artifact
        );

        let svg = crate::render_svg(&diagram);
        let storage_start = svg.find("<!--cluster Vault5101-->").unwrap();
        let artifact_start = svg.find("<!--cluster Receipts5107-->").unwrap();
        let storage = &svg[storage_start..artifact_start];
        let artifact = &svg[artifact_start..svg.find("<!--entity Inner5113-->").unwrap()];

        assert!(
            storage.contains(r#"rx="35" ry="35" style="stroke:#181818;stroke-width:1;""#),
            "storage must use USymbolStorage's rounded body: {storage}"
        );
        assert!(
            !storage.contains("<path "),
            "storage must not inherit the database cylinder: {storage}"
        );
        assert!(
            artifact.contains("<polygon ") && artifact.matches("<line ").count() == 2,
            "artifact must retain the folded-page glyph: {artifact}"
        );
    }

    #[test]
    fn artifact_cluster_canvas_uses_limitfinder_polygon_overscan() {
        assert_eq!(
            super::component_cluster_painted_max(
                Some(super::ComponentPackageKind::Storage),
                120.0,
                80.0
            ),
            (120.0, 80.0, super::SVEK_CANVAS_PAD)
        );
        assert_eq!(
            super::component_cluster_painted_max(
                Some(super::ComponentPackageKind::Artifact),
                120.0,
                80.0
            ),
            (125.0, 80.0, super::SVEK_CANVAS_PAD)
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

    #[test]
    fn no_oracle_link_note_uses_renamed_svek_label_component() {
        let input = "@startuml\n\
                     component \"Ingress Relay 941\" as Ingress941\n\
                     component \"Archive Sink 947\" as Archive947\n\
                     Ingress941 --> Archive947 : streams batches\n\
                     note on link\n\
                       Validates renamed stream 953\n\
                       before durable handoff 967\n\
                     end note\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML beta reference. In
        // `CommandFactoryNoteOnLink.executeInternal`, the note is owned by the
        // preceding Link; `SvekEdge.appendLine` solves its merged label table.
        assert!(
            svg.contains(r#"viewBox="0 0 320 240""#) || svg.contains(r#"viewBox="0 0 321 240""#),
            "debug and release layout builds may round the right edge to adjacent pixels: {svg}"
        );
        let label = svg.find(">streams batches</text>").unwrap();
        let note_body = svg.find(r#"<path d="M92."#).unwrap();
        let note_text = svg.find(">Validates renamed stream 953</text>").unwrap();
        assert!(
            label < note_body && note_body < note_text,
            "the link label, folded note, and note text must share Java's link group order: {svg}"
        );
        assert_eq!(
            svg.matches(r##"fill="#FEFFDD" style="stroke:#181818;stroke-width:0.5;"/>"##)
                .count(),
            2,
            "the link-owned note must paint one body and one folded corner: {svg}"
        );
        assert_eq!(
            svg.matches(r#"<g class="entity""#).count(),
            2,
            "a link-owned note must not become a standalone SVEK entity: {svg}"
        );
    }

    #[test]
    fn renamed_bracket_interface_and_container_share_component_symbol() {
        let input = "@startuml\n\
                     component RenamedShell9803 {\n\
                       interface [Renamed Audit Store 9817] as Store9817\n\
                     }\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r#"data-qualified-name="RenamedShell9803.Store9817""#),
            "{svg}"
        );
        assert!(svg.contains(">Renamed Audit Store 9817</text>"));
        assert_eq!(
            svg.matches(r#"height="10" style="stroke:#181818;"#).count(),
            2,
            "the group and bracket-selected component leaf each paint one large tab: {svg}"
        );
        assert_eq!(
            svg.matches(r#"height="2" style="stroke:#181818;"#).count(),
            4,
            "the group and leaf each paint two small tab bars: {svg}"
        );
    }

    #[test]
    fn renamed_quoted_link_label_uses_java_labels_normalization() {
        let input = "@startuml\n\
                     component \"Renamed Ingress 9803\" as Ingress9803\n\
                     component \"Renamed Archive 9811\" as Archive9811\n\
                     Ingress9803 --> Archive9811 : \"renamed-link-9817\"\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML reference generated from this non-corpus source.
        assert!(svg.contains(r#"textLength="126.4771""#), "{svg}");
        assert!(svg.contains(">renamed-link-9817</text>"), "{svg}");
        assert!(!svg.contains("&quot;renamed-link-9817&quot;"), "{svg}");
    }

    #[test]
    fn renamed_right_legend_composes_a_bordered_table_band() {
        let input = "@startuml\n\
                     component RenamedIngress9803\n\
                     component RenamedArchive9811\n\
                     RenamedIngress9803 --> RenamedArchive9811\n\
                     legend right\n\
                       | Signal 9817 | Meaning 9829 |\n\
                       | ==> | Fresh route 9833 |\n\
                     endlegend\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML reference generated from this non-corpus source.
        assert!(svg.contains(r#"viewBox="0 0 256 245""#), "{svg}");
        assert!(
            svg.contains(r#"<g class="legend" data-source-line="4">"#),
            "{svg}"
        );
        assert!(
            svg.contains(
                r##"<rect fill="#DDDDDD" height="46.9766" rx="7.5" ry="7.5" style="stroke:#000000;stroke-width:1;" width="225.5234" x="12" y=""##
            ),
            "{svg}"
        );
        assert_eq!(
            svg.matches(r##"<line style="stroke:#000000;stroke-width:1;""##)
                .count(),
            6,
            "{svg}"
        );
    }

    #[test]
    fn renamed_plain_legend_keeps_the_existing_fallback() {
        let input = "@startuml\n\
                     component RenamedGateway9851\n\
                     legend\n\
                       Renamed plain key 9857\n\
                     endlegend\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(">Renamed plain key 9857</text>"), "{svg}");
    }

    #[test]
    fn qualified_name_projection_never_reparents_a_materialized_leaf() {
        let input = "@startuml\n\
                     node OuterAlpha {\n\
                       component SharedLeaf\n\
                     }\n\
                     node OuterBeta {\n\
                       component BetaLeaf\n\
                     }\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Component(mut diagram) = diagram else {
            panic!("expected component diagram");
        };
        // Simulate an older serialized AST carrying the parser's former
        // duplicate membership. Java's quark parent remains first-writer-wins.
        diagram.packages[1]
            .components
            .push("SharedLeaf".to_string());

        let qualified = super::build_qualified_names(&diagram.packages);
        assert_eq!(qualified["SharedLeaf"], "OuterAlpha.SharedLeaf");
    }
}
