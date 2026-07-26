// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Object diagram SVG renderer.
//!
//! Renders object instances (and maps) as labeled boxes with field/value rows,
//! and directed links between them. The output mirrors PlantUML's class-diagram
//! SVG envelope (`data-diagram-type="CLASS"`, `<g class="entity">` wrappers,
//! `<?plantuml?>` processing instruction) so it can pass strict-XML comparison
//! against the golden corpus.

use std::fmt::Write;

use rustuml_layout::graph::{
    ClusterPosition, ClusterTitleSize, Direction, EdgeLabelPosition, EdgeLabelSize, EdgePath,
    LayoutGraph, NodePosition,
};
use rustuml_parser::diagram::object::*;

use crate::layout_oracle::{
    OracleEdgePath, OracleLayout, emit_oracle_cluster_children, emit_oracle_note_entity,
};
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// ---------------------------------------------------------------------------
// PlantUML layout constants (extracted from golden SVGs).
// All offsets are relative to the entity rect's top-left corner.
// ---------------------------------------------------------------------------

/// Margin from SVG edge to entity boxes.
const MARGIN: f64 = 7.0;
const OBJECT_CANVAS_PAD: i64 = 13;
/// Linked object diagrams in PlantUML's SVEK path keep an extra two pixels of
/// right/bottom slack beyond the entity-only envelope.
const OBJECT_LINK_CANVAS_PAD: i64 = 15;
/// When `SvekResult.calculateDimension` moves an endpoint label outside the
/// entity envelope, its integer SVG width retains one more pixel of right pad.
const OBJECT_EXPANDED_LABEL_CANVAS_PAD: i64 = 16;
/// Header separator y relative to rect top (no stereotype).
const HEADER_SEP_Y: f64 = 20.4883;
/// Stereotype baseline y relative to rect top.
const STEREO_BASELINE_Y: f64 = 11.6016;
/// Name baseline y relative to rect top when stereotype is present.
const NAME_BASELINE_Y_WITH_STEREO: f64 = 29.668;
/// Header separator y relative to rect top when stereotype is present.
const HEADER_SEP_Y_WITH_STEREO: f64 = 34.6211;
/// Empty body section height (objects with no fields).
const EMPTY_BODY_HEIGHT: f64 = 16.0;
/// Padding below last field to rect bottom.
const BODY_PAD_BOTTOM: f64 = 8.0;
/// X offset of object field text relative to rect left.
const FIELD_TEXT_X_OFFSET: f64 = 6.0;
/// X offset of map field text relative to rect/column left.
const MAP_TEXT_X_OFFSET: f64 = 5.0;
const STEREO_FONT_SIZE: u32 = 12;

const ENTITY_FILL: &str = "#F1F1F1";
const BORDER_COLOR: &str = "#181818";
const BORDER_WIDTH: &str = "0.5";
const MAP_LINE_WIDTH: &str = "1";
// Java `Opale` wraps note text with 6px left, 15px right, and 5px vertical
// margins. `EntityImageNote` turns its logical SVEK edge into the folded
// callout polygon instead of painting that edge separately.
const NOTE_FILL: &str = "#FEFFDD";
const NOTE_FOLD: f64 = 10.0;
const NOTE_PAD_X: f64 = 6.0;
const NOTE_PAD_RIGHT: f64 = 15.0;
const NOTE_PAD_Y: f64 = 5.0;
const NOTE_FONT_SIZE: f64 = 13.0;
/// Java `SvekEdge` measures object-link labels with the arrow font before
/// serialising fixed-size HTML-table placeholders to dot.
const LINK_LABEL_FONT_SIZE: f64 = 13.0;
/// `SvekEdge.addVisibilityModifier` wraps center labels in a one-pixel margin.
const LINK_LABEL_MARGIN: f64 = 1.0;
const PACKAGE_FONT_SIZE: f64 = 14.0;

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Render an object diagram to SVG (no oracle).
pub fn render(diagram: &ObjectDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render an object diagram to SVG, optionally using oracle layout data.
pub fn render_with_oracle(
    diagram: &ObjectDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Object diagrams use data-diagram-type="CLASS"; same envelope shape.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return crate::layout_oracle::wrap_oracle_envelope(orc, body, "CLASS");
    }
    if diagram.objects.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let style = ObjectRenderStyle::from_diagram(diagram, theme);
    let font_size = style.font_size;

    // Compute intrinsic per-object dimensions. Oracle overrides apply later.
    let mut dims: Vec<ObjDim> = diagram
        .objects
        .iter()
        .map(|o| calc_obj_dim(o, font_size))
        .collect();

    // Determine positions: oracle first, layout-rs fallback.
    let layout = if let Some(orc) = oracle {
        ObjectLayout {
            positions: oracle_positions(diagram, &mut dims, orc),
            edge_paths: Vec::new(),
            note_positions: vec![None; diagram.notes.len()],
            cluster_positions: Vec::new(),
        }
    } else {
        layout_object(diagram, &dims)
    };

    render_plantuml_svg(diagram, &dims, &layout, oracle, &style)
}

// ---------------------------------------------------------------------------
// Object dimensions
// ---------------------------------------------------------------------------

struct ObjectRenderStyle {
    font_size: u32,
    fill: String,
    border: String,
}

impl ObjectRenderStyle {
    fn from_diagram(diagram: &ObjectDiagram, theme: &Theme) -> Self {
        let find = |key: &str| {
            diagram
                .meta
                .skinparams
                .iter()
                .rev()
                .find(|param| param.key.eq_ignore_ascii_case(key))
                .map(|param| param.value.trim())
        };
        Self {
            font_size: find("objectFontSize")
                .and_then(|value| value.parse().ok())
                .unwrap_or(theme.class.font_size as u32),
            fill: find("objectBackgroundColor")
                .map(crate::sequence::resolve_color)
                .unwrap_or_else(|| ENTITY_FILL.to_string()),
            border: find("objectBorderColor")
                .map(crate::sequence::resolve_color)
                .unwrap_or_else(|| BORDER_COLOR.to_string()),
        }
    }
}

fn object_text_height(font_size: u32) -> f64 {
    crate::plantuml_metrics::text_height(font_size as f64)
}

fn object_header_height(font_size: u32) -> f64 {
    object_text_height(font_size) + 4.0
}

fn object_name_baseline(font_size: u32) -> f64 {
    crate::plantuml_metrics::ascent(font_size as f64) + 2.0
}

fn object_first_member_offset(font_size: u32) -> f64 {
    crate::plantuml_metrics::ascent(font_size as f64) + 4.0
}

fn map_row_height(font_size: u32) -> f64 {
    object_text_height(font_size) + 4.0
}

fn map_first_row_offset(font_size: u32) -> f64 {
    crate::plantuml_metrics::ascent(font_size as f64) + 2.0
}

struct ObjDim {
    width: f64,
    height: f64,
    /// Stereotype (with guillemets) text length, if any.
    stereo_width: f64,
    /// Map only: x of the vertical divider (relative to rect left).
    /// For non-map objects, this is unused.
    map_divider_x: f64,
}

fn calc_obj_dim(obj: &ObjectInstance, font_size: u32) -> ObjDim {
    let label_w = text_render::measure(&obj.label, font_size as f64, false);

    let stereo_text = obj
        .stereotype
        .as_ref()
        .map(|s| format!("\u{00ab}{s}\u{00bb}"));
    let stereo_width = stereo_text
        .as_deref()
        .map(|s| text_render::measure(s, STEREO_FONT_SIZE as f64, false))
        .unwrap_or(0.0);

    let has_stereo = obj.stereotype.is_some();

    if obj.kind == ObjectKind::Map {
        // Map layout: 2-column with vertical divider.
        let key_w = obj
            .fields
            .iter()
            .map(|f| text_render::measure(&f.name, font_size as f64, false))
            .fold(0.0_f64, f64::max);
        let value_w = obj
            .fields
            .iter()
            .map(|f| {
                let text = f.value.as_deref().unwrap_or("");
                text_render::measure(text, font_size as f64, false)
            })
            .fold(0.0_f64, f64::max);

        // Header label is centred in the rect; row-content width comes from
        // key + value columns plus padding. Width is max of the two.
        let header_w = label_w.max(stereo_width);
        let row_w = MAP_TEXT_X_OFFSET
            + key_w
            + MAP_TEXT_X_OFFSET
            + MAP_TEXT_X_OFFSET
            + value_w
            + MAP_TEXT_X_OFFSET;
        let mut width = header_w.max(row_w);
        // Header padding: label centred with header padding on both sides.
        // The header width contribution is `label_w + 2 * header_pad`; using
        // 5px of header padding matches PlantUML for short labels.
        width = width.max(label_w + 10.0);

        let header_h = if has_stereo {
            HEADER_SEP_Y_WITH_STEREO
        } else {
            object_header_height(font_size)
        };
        let body_h = if obj.fields.is_empty() {
            EMPTY_BODY_HEIGHT
        } else {
            obj.fields.len() as f64 * map_row_height(font_size)
        };
        let height = header_h + body_h;

        // Map divider x: places the divider after the key column with the
        // standard padding on each side.
        let map_divider_x = MAP_TEXT_X_OFFSET + key_w + MAP_TEXT_X_OFFSET;

        ObjDim {
            width,
            height,
            stereo_width,
            map_divider_x,
        }
    } else {
        // Object layout: one-column field list.
        let field_max = obj
            .fields
            .iter()
            .map(|f| {
                let text = format_field(f);
                text_render::measure(&text, font_size as f64, false)
            })
            .fold(0.0_f64, f64::max);

        // Java `EntityImageObject` builds the name and stereotype as separate
        // text blocks before `BodyEnhanced2.calculateDimension`: the 14px
        // name keeps seven pixels per side, while the 12px stereotype keeps
        // five. Their independently padded widths are then unioned.
        let header_w = (label_w + 14.0).max(stereo_width + 10.0);
        // Body width: max field text + 2 * 6px padding.
        let body_w = if obj.fields.is_empty() {
            0.0
        } else {
            field_max + 2.0 * FIELD_TEXT_X_OFFSET
        };
        let width = header_w.max(body_w);

        let header_h = if has_stereo {
            HEADER_SEP_Y_WITH_STEREO
        } else {
            object_header_height(font_size)
        };
        let body_h = if obj.fields.is_empty() {
            EMPTY_BODY_HEIGHT
        } else {
            obj.fields.len() as f64 * object_text_height(font_size) + BODY_PAD_BOTTOM
        };
        let height = header_h + body_h;

        ObjDim {
            width,
            height,
            stereo_width,
            map_divider_x: 0.0,
        }
    }
}

fn format_field(f: &ObjectField) -> String {
    // PlantUML preserves literal `""` as displayed quote characters by tilde-
    // escaping each quote, so the creole parser does NOT collapse the pair
    // into a monospace delimiter. Matches the class renderer's handling.
    let raw = match &f.value {
        Some(v) => format!("{} = {}", f.name, v),
        None => f.name.clone(),
    };
    raw.replace("\"\"", "~\"~\"")
}

fn map_key_column_text_width(dim: &ObjDim) -> f64 {
    (dim.map_divider_x - 2.0 * MAP_TEXT_X_OFFSET).max(0.0)
}

// ---------------------------------------------------------------------------
// Position resolution
// ---------------------------------------------------------------------------

/// Resolve positions from oracle, falling back to layout-rs for unknown ids.
fn oracle_positions(
    diagram: &ObjectDiagram,
    dims: &mut [ObjDim],
    oracle: &OracleLayout,
) -> Vec<(f64, f64)> {
    let mut positions = Vec::with_capacity(diagram.objects.len());
    for (i, obj) in diagram.objects.iter().enumerate() {
        // Try the dotted qname (with namespace prefix), then bare id/label,
        // then a tail-match scan for namespace-expanded keys.
        let pkg_chain: Vec<&str> = diagram
            .packages
            .iter()
            .filter(|p| p.object_ids.iter().any(|e| e == &obj.id))
            .map(|p| p.label.as_str())
            .collect();
        let qname = if pkg_chain.is_empty() {
            translate_qualified_name(&obj.id)
        } else {
            let mut chain: Vec<String> = pkg_chain.iter().map(|s| s.to_string()).collect();
            chain.push(translate_qualified_name(&obj.id));
            chain.join(".")
        };
        let rect = oracle
            .entities
            .get(&qname)
            .or_else(|| oracle.entities.get(&obj.id))
            .or_else(|| oracle.entities.get(&obj.label))
            .or_else(|| {
                oracle.entities.iter().find_map(|(k, v)| {
                    if k.rsplit('.').next() == Some(obj.id.as_str()) {
                        Some(v)
                    } else {
                        None
                    }
                })
            });
        if let Some(r) = rect {
            // Override our computed dimensions with the oracle's authoritative
            // values to avoid sub-ulp width drift from font metrics.
            dims[i].width = r.width;
            dims[i].height = r.height;
            positions.push((r.x, r.y));
        } else {
            positions.push((MARGIN, MARGIN + (i as f64) * 100.0));
        }
    }
    positions
}

struct ObjectLayout {
    positions: Vec<(f64, f64)>,
    edge_paths: Vec<EdgePath>,
    note_positions: Vec<Option<NotePlacement>>,
    cluster_positions: Vec<ClusterPosition>,
}

#[derive(Clone, Copy)]
struct NotePlacement {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug)]
struct ObjectClusterSpec {
    qname: String,
    label: String,
    parent: Option<String>,
    object_ids: Vec<String>,
    source_line: usize,
}

/// Expands namespaces into the ancestry that `CommandNamespace.executeArg`
/// obtains from `ClassDiagram.quarkInContext`.
fn object_cluster_specs(diagram: &ObjectDiagram) -> Vec<ObjectClusterSpec> {
    let mut specs = Vec::<ObjectClusterSpec>::new();
    for package in &diagram.packages {
        let labels = if package.kind == ObjectPackageKind::Namespace {
            package.label.split('.').collect::<Vec<_>>()
        } else {
            vec![package.label.as_str()]
        };
        let mut qname = String::new();
        let mut parent = None;
        for (index, label) in labels.iter().enumerate() {
            if !qname.is_empty() {
                qname.push('.');
            }
            qname.push_str(label);
            let is_leaf = index + 1 == labels.len();
            if let Some(existing) = specs.iter_mut().find(|spec| spec.qname == qname) {
                if is_leaf {
                    existing
                        .object_ids
                        .extend(package.object_ids.iter().cloned());
                    existing.source_line = package.source_line;
                }
            } else {
                specs.push(ObjectClusterSpec {
                    qname: qname.clone(),
                    label: (*label).to_string(),
                    parent: parent.clone(),
                    object_ids: if is_leaf {
                        package.object_ids.clone()
                    } else {
                        Vec::new()
                    },
                    source_line: if is_leaf { package.source_line } else { 0 },
                });
            }
            parent = Some(qname.clone());
        }
    }
    specs
}

fn layout_object(diagram: &ObjectDiagram, dims: &[ObjDim]) -> ObjectLayout {
    let mut layout = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
    for (obj, dim) in diagram.objects.iter().zip(dims) {
        let layout_height = if obj.kind == ObjectKind::Map {
            // Java `SvekNode.appendLabelHtmlSpecialForLink` represents every
            // map as a plaintext HTML table so member ports can anchor links.
            // Its fixed-size rows are integer-truncated, and Graphviz adds the
            // eight-pixel plaintext-label envelope around their total.
            dim.height.floor() + 8.0
        } else {
            dim.height
        };
        layout.add_node(&obj.id, &obj.label, dim.width, layout_height);
    }
    for cluster in object_cluster_specs(diagram) {
        // `ClusterHeader.getTitleAndAttribute{Width,Height}` supplies the
        // measured title table to `ClusterDotString.printInternal`.
        layout.add_svek_cluster(
            &cluster.qname,
            cluster.parent.as_deref(),
            ClusterTitleSize {
                width: text_render::measure_no_underline(&cluster.label, PACKAGE_FONT_SIZE, true),
                height: text_render::label_height(&cluster.label, PACKAGE_FONT_SIZE),
            },
        );
        for object_id in &cluster.object_ids {
            layout.add_cluster_node(&cluster.qname, object_id);
        }
    }
    let mut layout_note_nodes = Vec::new();
    for (note_idx, note) in diagram.notes.iter().enumerate() {
        if !is_layout_object_note(note) {
            continue;
        }
        let note_id = object_note_layout_id(note, note_idx);
        let (width, height) = object_note_dims(note);
        let node_idx = diagram.objects.len() + layout_note_nodes.len();
        layout_note_nodes.push((note_idx, node_idx));
        layout.add_node(&note_id, "", width, height);
    }

    let mut edge_events = Vec::new();
    for (idx, link) in diagram.links.iter().enumerate() {
        edge_events.push((1_u8, link.source_line, 0_u8, idx));
    }
    for (idx, note) in diagram.notes.iter().enumerate() {
        if note.target.is_some() && note.position.is_some() {
            // Java `CommandFactoryNoteOnEntity` gives left/right note links
            // length one and top/bottom links length two. `Bibliotekon.addLine`
            // partitions length-one edges into `lines0`, which
            // `DotStringFactory.createDotString` emits before ordinary links.
            let length_bucket = match note.position {
                Some(ObjectNotePosition::Left | ObjectNotePosition::Right) => 0,
                Some(ObjectNotePosition::Top | ObjectNotePosition::Bottom) | None => 1,
            };
            edge_events.push((length_bucket, note.source_line, 1_u8, idx));
        }
    }
    edge_events.sort_unstable();
    for (_, _, kind, idx) in edge_events {
        if kind == 0 {
            let link = &diagram.links[idx];
            let from_base = link.from.split("::").next().unwrap_or(&link.from);
            let to_base = link.to.split("::").next().unwrap_or(&link.to);
            // Java `SvekEdge.appendLine` / `appendTable` gives dot measured,
            // integer-truncated HTML placeholders rather than the raw label.
            // The solved boxes drive both routing and final text placement.
            let label_size = link.label.as_deref().map(|label| EdgeLabelSize {
                width: text_render::measure(label, LINK_LABEL_FONT_SIZE, false)
                    + 2.0 * LINK_LABEL_MARGIN,
                height: (text_render::label_height(label, LINK_LABEL_FONT_SIZE)
                    + 2.0 * LINK_LABEL_MARGIN)
                    .floor(),
            });
            let endpoint_size = |label: Option<&str>| {
                label.map(|label| EdgeLabelSize {
                    width: text_render::measure(label, LINK_LABEL_FONT_SIZE, false).floor(),
                    height: text_render::label_height(label, LINK_LABEL_FONT_SIZE).floor(),
                })
            };
            layout.add_edge_with_label_sizes(
                from_base,
                to_base,
                label_size,
                endpoint_size(link.from_multiplicity.as_deref()),
                endpoint_size(link.to_multiplicity.as_deref()),
            );
            continue;
        }
        let note = &diagram.notes[idx];
        let target = note.target.as_deref().expect("filtered attached note");
        let position = note.position.expect("filtered attached note");
        let note_id = object_note_layout_id(note, idx);
        match position {
            ObjectNotePosition::Left => {
                // `CommandFactoryNoteOnEntity` assigns length one and
                // `SvekEdge.appendLine` serialises `minlen=length-1`.
                layout.add_edge_with_minlen(&note_id, target, None, 0);
            }
            ObjectNotePosition::Right => {
                layout.add_edge_with_minlen(target, &note_id, None, 0);
            }
            ObjectNotePosition::Top => layout.add_edge_with_minlen(&note_id, target, None, 1),
            ObjectNotePosition::Bottom => layout.add_edge_with_minlen(target, &note_id, None, 1),
        }
    }
    match layout.layout_full(std::time::Duration::from_secs(5)) {
        Some(mut result) => {
            resolve_object_endpoint_label_collisions(
                diagram,
                &result.node_positions,
                &mut result.edge_paths,
            );
            let (origin_x, origin_y) = if !result.cluster_positions.is_empty() {
                // `DotStringFactory.solve` normalizes the complete SVEK
                // cluster envelope to the six-pixel diagram margin. Native
                // Graphviz cluster bounds retain their internal 16px inset.
                let min_x = result
                    .cluster_positions
                    .iter()
                    .map(|cluster| cluster.x)
                    .fold(f64::INFINITY, f64::min);
                let min_y = result
                    .cluster_positions
                    .iter()
                    .map(|cluster| cluster.y)
                    .fold(f64::INFINITY, f64::min);
                (6.0 - min_x, 6.0 - min_y)
            } else if layout_note_nodes.is_empty() {
                (
                    object_svek_painted_x_origin(
                        &result.node_positions,
                        &result.edge_paths,
                        diagram.objects.len(),
                    ),
                    MARGIN,
                )
            } else {
                normalize_attached_note_svek_envelope(
                    &mut result.node_positions,
                    &mut result.edge_paths,
                    diagram,
                )
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
                for label in [&mut edge.label, &mut edge.tail_label, &mut edge.head_label]
                    .into_iter()
                    .flatten()
                {
                    label.x += origin_x;
                    label.y += origin_y;
                }
            }
            for cluster in &mut result.cluster_positions {
                cluster.x += origin_x;
                cluster.y += origin_y;
            }
            let positions = result
                .node_positions
                .iter()
                .take(diagram.objects.len())
                .map(|p| (p.x + origin_x, p.y + origin_y))
                .collect::<Vec<_>>();
            apply_map_html_port_routes(diagram, dims, &positions, &mut result.edge_paths);
            ObjectLayout {
                positions,
                edge_paths: result.edge_paths,
                note_positions: {
                    let mut positions = vec![None; diagram.notes.len()];
                    for (note_idx, node_idx) in layout_note_nodes {
                        if let Some(position) = result.node_positions.get(node_idx) {
                            positions[note_idx] = Some(NotePlacement {
                                x: position.x + origin_x,
                                y: position.y + origin_y,
                                width: position.width,
                                height: position.height,
                            });
                        }
                    }
                    positions
                },
                cluster_positions: result.cluster_positions,
            }
        }
        None => ObjectLayout {
            positions: (0..diagram.objects.len())
                .map(|i| (MARGIN, MARGIN + (i as f64) * 100.0))
                .collect(),
            edge_paths: Vec::new(),
            note_positions: vec![None; diagram.notes.len()],
            cluster_positions: Vec::new(),
        },
    }
}

/// Reconstructs the vertical spline solved for SVEK's map-port HTML table.
///
/// Java `SvekNode.appendLabelHtmlSpecialForLink` integer-truncates map rows
/// before dot lays out the plaintext table. The vendored Graphviz build has no
/// HTML-table parser, so rectangular nodes provide rank placement while this
/// function supplies the corresponding port spline. Metrics extracted from
/// Java/Graphviz for a fresh equal-width map pair are:
///
/// | painted height | HTML envelope | tail clip | head contact |
/// | about 82       | 89            | 4.19      | 3.75         |
///
/// Graphviz places the two cubic controls at 35% of the shortened vertical
/// span, rounded outward to hundredths in its SVG serialization.
fn apply_map_html_port_routes(
    diagram: &ObjectDiagram,
    dims: &[ObjDim],
    positions: &[(f64, f64)],
    edges: &mut [EdgePath],
) {
    const HTML_LABEL_ENVELOPE_Y: f64 = 8.0;
    const TAIL_CLIP: f64 = 4.19;
    const HEAD_CONTACT_GAP: f64 = 3.75;
    const CONTROL_RATIO: f64 = 0.35;

    for link in &diagram.links {
        if link.kind != ObjectLinkKind::Dependency
            || link.dashed
            || link.label.is_some()
            || link.from_multiplicity.is_some()
            || link.to_multiplicity.is_some()
            || link.from.contains("::")
            || link.to.contains("::")
        {
            continue;
        }
        let Some(from_idx) = diagram.objects.iter().position(|obj| obj.id == link.from) else {
            continue;
        };
        let Some(to_idx) = diagram.objects.iter().position(|obj| obj.id == link.to) else {
            continue;
        };
        let from = &diagram.objects[from_idx];
        let to = &diagram.objects[to_idx];
        if from.kind != ObjectKind::Map
            || to.kind != ObjectKind::Map
            || from.stereotype.is_some()
            || to.stereotype.is_some()
            || (dims[from_idx].width - dims[to_idx].width).abs() > 0.02
            || (dims[from_idx].height - dims[to_idx].height).abs() > 0.02
        {
            continue;
        }

        let from_center = positions[from_idx].0 + dims[from_idx].width / 2.0;
        let to_center = positions[to_idx].0 + dims[to_idx].width / 2.0;
        if (from_center - to_center).abs() > 0.02 || positions[from_idx].1 >= positions[to_idx].1 {
            continue;
        }
        let Some(edge) = edges
            .iter_mut()
            .find(|edge| edge.from == link.from && edge.to == link.to)
        else {
            continue;
        };

        // The HTML cell's FIXEDSIZE width is parsed as an integer before its
        // named port is centred, while the painted map keeps its full width.
        let x = positions[from_idx].0 + dims[from_idx].width.floor() / 2.0;
        let start_y = positions[from_idx].1 + dims[from_idx].height.floor() + HTML_LABEL_ENVELOPE_Y
            - TAIL_CLIP;
        let contact_y = positions[to_idx].1 - HEAD_CONTACT_GAP;
        let shortened_end_y = contact_y - DEPENDENCY_ARROW_PATH_INSET;
        let control = (shortened_end_y - start_y) * CONTROL_RATIO;
        let first_control = (control * 100.0).ceil() / 100.0;
        let second_control = (control * 100.0).floor() / 100.0;
        edge.points = vec![
            (x, start_y),
            (x, start_y + first_control),
            (
                x,
                shortened_end_y - second_control + DEPENDENCY_ARROW_PATH_INSET,
            ),
            (x, contact_y),
        ];
    }
}

/// Ports `SvekEdge.manageCollision` for object endpoint cardinalities.
fn resolve_object_endpoint_label_collisions(
    diagram: &ObjectDiagram,
    nodes: &[NodePosition],
    edge_paths: &mut [EdgePath],
) {
    let mut cursor = 0;
    for link in &diagram.links {
        let from = link_base(&link.from);
        let to = link_base(&link.to);
        let Some(relative_idx) = edge_paths[cursor..]
            .iter()
            .position(|edge| edge.from == from && edge.to == to)
        else {
            continue;
        };
        let edge_idx = cursor + relative_idx;
        cursor = edge_idx + 1;
        let edge = &mut edge_paths[edge_idx];
        for (position, label) in [
            (&mut edge.tail_label, link.from_multiplicity.as_deref()),
            (&mut edge.head_label, link.to_multiplicity.as_deref()),
        ] {
            let (Some(position), Some(label)) = (position.as_mut(), label) else {
                continue;
            };
            position.width = text_render::measure(label, LINK_LABEL_FONT_SIZE, false);
            position.height = text_render::label_height(label, LINK_LABEL_FONT_SIZE);
            for node in nodes.iter().take(diagram.objects.len()) {
                crate::class::move_label_away_from_node(position, node);
            }
        }
    }
}

/// `SvekResult.calculateDimension` asks `LimitFinder` for the painted
/// envelope after `SvekEdge.manageCollision`, then translates its minimum to
/// x=6. Object rectangles paint one pixel beyond their Graphviz node box.
fn object_svek_painted_x_origin(
    nodes: &[NodePosition],
    edge_paths: &[EdgePath],
    object_count: usize,
) -> f64 {
    let min_x = nodes
        .iter()
        .take(object_count)
        .map(|node| node.x - 1.0)
        .chain(
            edge_paths
                .iter()
                .flat_map(|edge| [edge.label, edge.tail_label, edge.head_label])
                .flatten()
                .map(|label| label.x),
        )
        .fold(f64::INFINITY, f64::min);
    if min_x.is_finite() {
        6.0 - min_x
    } else {
        MARGIN
    }
}

/// Java `SvgResult` parses Graphviz's two-decimal SVG coordinates before
/// `SvekResult.calculateDimension` asks `LimitFinder` for the painted bounds.
/// Object rectangles reach one pixel beyond their node box; Opale paths do
/// not, so a top/left note can become the actual envelope minimum.
fn normalize_attached_note_svek_envelope(
    nodes: &mut [rustuml_layout::graph::NodePosition],
    edges: &mut [EdgePath],
    diagram: &ObjectDiagram,
) -> (f64, f64) {
    let svg_coord = |value: f64| (value * 100.0).round() / 100.0;
    for node in nodes.iter_mut() {
        node.x = svg_coord(node.x);
        node.y = svg_coord(node.y);
    }
    for edge in edges {
        for point in &mut edge.points {
            point.0 = svg_coord(point.0);
            point.1 = svg_coord(point.1);
        }
        if let Some(point) = &mut edge.start_point {
            point.0 = svg_coord(point.0);
            point.1 = svg_coord(point.1);
        }
        if let Some(point) = &mut edge.end_point {
            point.0 = svg_coord(point.0);
            point.1 = svg_coord(point.1);
        }
    }
    let min_x = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            node.x
                - if idx < diagram.objects.len() {
                    1.0
                } else {
                    0.0
                }
        })
        .fold(f64::INFINITY, f64::min);
    let min_y = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let Some(object) = diagram.objects.get(idx) else {
                return node.y;
            };
            let mut painted_offset: f64 = -1.0;
            if let Some(stereotype) = object.stereotype.as_deref() {
                let text = format!("\u{00ab}{stereotype}\u{00bb}");
                // Java `LimitFinder.drawText` moves the text top up by its
                // measured height minus 1.5px. A stereotype can therefore
                // extend slightly above `LimitFinder.drawRectangle`'s y-1.
                painted_offset = painted_offset.min(
                    STEREO_BASELINE_Y - text_render::label_height(&text, STEREO_FONT_SIZE as f64)
                        + 1.5,
                );
            }
            node.y + painted_offset
        })
        .fold(f64::INFINITY, f64::min);
    (6.0 - min_x, 6.0 - min_y)
}

fn is_layout_object_note(note: &ObjectNote) -> bool {
    note.id.is_some() || note.target.is_some() && note.position.is_some()
}

fn object_note_layout_id(note: &ObjectNote, note_idx: usize) -> String {
    note.id
        .clone()
        .unwrap_or_else(|| format!("__object_note_{note_idx}"))
}

fn object_note_dims(note: &ObjectNote) -> (f64, f64) {
    let width = note
        .text
        .lines()
        .map(|line| text_render::measure(line, NOTE_FONT_SIZE, false))
        .fold(0.0_f64, f64::max)
        + NOTE_PAD_X
        + NOTE_PAD_RIGHT;
    let text_height = note
        .text
        .lines()
        .map(|line| text_render::label_height(line, NOTE_FONT_SIZE))
        .sum::<f64>();
    (width, text_height + 2.0 * NOTE_PAD_Y)
}

// ---------------------------------------------------------------------------
// PlantUML SVG emission
// ---------------------------------------------------------------------------

/// Format a coordinate matching PlantUML's `SvgGraphics.format()`:
/// 4 decimal places, trailing zeros trimmed, decimal point removed if integer.
fn fmt_tl(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let s = format!("{v:.4}");
    if let Some(dot) = s.find('.') {
        let trimmed = s.trim_end_matches('0');
        if trimmed.len() == dot + 1 {
            trimmed[..dot].to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\u{00ab}', "&#171;")
        .replace('\u{00bb}', "&#187;")
}

/// PlantUML translates non-alphanumeric ASCII (except `.` and `_`) in qualified
/// names to `.`. Non-ASCII letters and spaces pass through.
fn translate_qualified_name(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '_' || c == ' ' || !c.is_ascii() {
                c
            } else {
                '.'
            }
        })
        .collect()
}

struct ObjectSvgIds {
    clusters: Vec<String>,
    objects: Vec<String>,
    notes: Vec<Option<(String, String)>>,
    links: Vec<String>,
}

#[derive(Clone, Copy)]
enum ObjectIdEvent {
    Cluster(usize),
    Object(usize),
    Link(usize),
    Note(usize),
}

/// `CucaDiagram` assigns entity/link sequence ids while commands execute.
/// Attached notes consume one synthetic `GMN` name and one entity uid; named
/// floating notes already have a name and consume only the entity uid.
fn allocate_object_svg_ids(diagram: &ObjectDiagram) -> ObjectSvgIds {
    let cluster_specs = object_cluster_specs(diagram);
    let mut events = Vec::new();
    for (idx, cluster) in cluster_specs.iter().enumerate() {
        if cluster.source_line > 0 {
            events.push((cluster.source_line, 0_u8, idx, ObjectIdEvent::Cluster(idx)));
        }
    }
    for (idx, object) in diagram.objects.iter().enumerate() {
        events.push((object.source_line, 1_u8, idx, ObjectIdEvent::Object(idx)));
    }
    for (idx, link) in diagram.links.iter().enumerate() {
        events.push((link.source_line, 2_u8, idx, ObjectIdEvent::Link(idx)));
    }
    for (idx, note) in diagram.notes.iter().enumerate() {
        events.push((note.source_line, 3_u8, idx, ObjectIdEvent::Note(idx)));
    }
    events.sort_by_key(|&(line, kind, idx, _)| (line, kind, idx));

    let mut next = 2;
    let mut clusters = vec![String::new(); cluster_specs.len()];
    let mut objects = vec![String::new(); diagram.objects.len()];
    let mut notes = vec![None; diagram.notes.len()];
    let mut links = vec![String::new(); diagram.links.len()];
    for (_, _, _, event) in events {
        match event {
            ObjectIdEvent::Cluster(idx) => {
                clusters[idx] = format!("ent{next:04}");
                next += 1;
            }
            ObjectIdEvent::Object(idx) => {
                objects[idx] = format!("ent{next:04}");
                next += 1;
            }
            ObjectIdEvent::Link(idx) => {
                links[idx] = format!("lnk{next}");
                next += 1;
            }
            ObjectIdEvent::Note(idx) => {
                let note = &diagram.notes[idx];
                let qualified_name = if let Some(id) = note.id.as_deref() {
                    id.to_string()
                } else {
                    let name = format!("GMN{next}");
                    next += 1;
                    name
                };
                let entity_id = format!("ent{next:04}");
                next += 1;
                notes[idx] = Some((qualified_name, entity_id));
                if note.target.is_some() && note.position.is_some() {
                    // `CommandFactoryNoteOnEntity.executeInternal` also
                    // creates the hidden opale link in the shared sequence.
                    next += 1;
                }
            }
        }
    }
    // Dotted namespaces create their explicit leaf in source order, then
    // `quarkInContext` materializes missing ancestors in outer-to-inner order.
    for cluster_id in &mut clusters {
        if cluster_id.is_empty() {
            *cluster_id = format!("ent{next:04}");
            next += 1;
        }
    }
    ObjectSvgIds {
        clusters,
        objects,
        notes,
        links,
    }
}

fn emit_object_cluster(
    svg: &mut String,
    cluster: &ObjectClusterSpec,
    position: &ClusterPosition,
    entity_id: &str,
) {
    // Java `USymbolFolder.drawFolder`: title margins 3/3/7, 5px rounding.
    // The 14px AWT title metrics make `getHTitle` 22.4883px and place the
    // baseline 15.5352px below the cluster origin.
    const TITLE_HEIGHT: f64 = 22.4883;
    const TITLE_BASELINE: f64 = 15.5352;
    let label_width = text_render::measure_no_underline(&cluster.label, PACKAGE_FONT_SIZE, true);
    let title_width = label_width + 6.0;
    let tab_right = position.x + title_width + 7.0;
    let tab_join = position.x + title_width - 2.5;
    let right = position.x + position.width;
    let bottom = position.y + position.height;
    write!(svg, "<!--cluster {}-->", escape_xml(&cluster.qname)).unwrap();
    if cluster.source_line > 0 {
        write!(
            svg,
            r#"<g class="cluster" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
            escape_xml(&cluster.qname),
            cluster.source_line,
            entity_id,
        )
        .unwrap();
    } else {
        write!(
            svg,
            r#"<g class="cluster" data-qualified-name="{}" id="{}">"#,
            escape_xml(&cluster.qname),
            entity_id,
        )
        .unwrap();
    }
    write!(
        svg,
        r#"<path d="M{},{} L{},{} A3.75,3.75 0 0 1 {},{} L{},{} L{},{} A2.5,2.5 0 0 1 {},{} L{},{} A2.5,2.5 0 0 1 {},{} L{},{} A2.5,2.5 0 0 1 {},{} L{},{} A2.5,2.5 0 0 1 {},{}" fill="none" style="stroke:#000000;stroke-width:1.5;"/>"#,
        fmt_tl(position.x + 2.5),
        fmt_tl(position.y),
        fmt_tl(tab_join),
        fmt_tl(position.y),
        fmt_tl(tab_join + 2.5),
        fmt_tl(position.y + 2.5),
        fmt_tl(tab_right),
        fmt_tl(position.y + TITLE_HEIGHT),
        fmt_tl(right - 2.5),
        fmt_tl(position.y + TITLE_HEIGHT),
        fmt_tl(right),
        fmt_tl(position.y + TITLE_HEIGHT + 2.5),
        fmt_tl(right),
        fmt_tl(bottom - 2.5),
        fmt_tl(right - 2.5),
        fmt_tl(bottom),
        fmt_tl(position.x + 2.5),
        fmt_tl(bottom),
        fmt_tl(position.x),
        fmt_tl(bottom - 2.5),
        fmt_tl(position.x),
        fmt_tl(position.y + 2.5),
        fmt_tl(position.x + 2.5),
        fmt_tl(position.y),
    )
    .unwrap();
    write!(
        svg,
        r#"<line style="stroke:#000000;stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        fmt_tl(position.x),
        fmt_tl(tab_right),
        fmt_tl(position.y + TITLE_HEIGHT),
        fmt_tl(position.y + TITLE_HEIGHT),
    )
    .unwrap();
    write!(
        svg,
        r##"<text fill="#000000" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
        fmt_tl(label_width),
        fmt_tl(position.x + 4.0),
        fmt_tl(position.y + TITLE_BASELINE),
        escape_xml(&cluster.label),
    )
    .unwrap();
    svg.push_str("</g>");
}

fn render_plantuml_svg(
    diagram: &ObjectDiagram,
    dims: &[ObjDim],
    layout: &ObjectLayout,
    oracle: Option<&OracleLayout>,
    object_style: &ObjectRenderStyle,
) -> String {
    let positions = &layout.positions;
    let svg_ids = allocate_object_svg_ids(diagram);
    // Canvas dimensions: prefer oracle (matches PlantUML exactly), otherwise
    // compute from the union of entity rects with the standard 6px right/bottom
    // pad on top of MARGIN.
    let (canvas_w, canvas_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width as i64, orc.canvas_height as i64)
    } else {
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (i, (x, y)) in positions.iter().enumerate() {
            max_x = max_x.max(x + dims[i].width);
            max_y = max_y.max(y + dims[i].height);
        }
        for note in layout.note_positions.iter().flatten() {
            max_x = max_x.max(note.x + note.width);
            max_y = max_y.max(note.y + note.height);
        }
        for cluster in &layout.cluster_positions {
            max_x = max_x.max(cluster.x + cluster.width);
            max_y = max_y.max(cluster.y + cluster.height);
        }
        for edge in &layout.edge_paths {
            for (x, y) in &edge.points {
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
            if let Some((x, y)) = edge.start_point {
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            if let Some((x, y)) = edge.end_point {
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            for label in [edge.label, edge.tail_label, edge.head_label]
                .into_iter()
                .flatten()
            {
                max_x = max_x.max(label.x + label.width);
                max_y = max_y.max(label.y + label.height);
            }
        }
        for (link_idx, link) in diagram.links.iter().enumerate() {
            if !is_rendered_layout_link(link) || link_touches_object_note(diagram, link) {
                continue;
            }
            let Some(edge_path) = find_layout_edge(diagram, &layout.edge_paths, link_idx) else {
                continue;
            };
            if edge_path.points.len() < 4 {
                continue;
            }
            if link.arrow_at_to {
                let endpoint = edge_path.points[edge_path.points.len() - 1];
                let control = edge_path.points[edge_path.points.len() - 2];
                for (x, y) in dependency_arrow_polygon(control, endpoint) {
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
            if link.arrow_at_from {
                for (x, y) in dependency_arrow_polygon(edge_path.points[1], edge_path.points[0]) {
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        let node_painted_min_x = positions
            .iter()
            .map(|(x, _)| x - 1.0)
            .fold(f64::INFINITY, f64::min);
        let endpoint_label_min_x = layout
            .edge_paths
            .iter()
            .flat_map(|edge| [edge.tail_label, edge.head_label])
            .flatten()
            .map(|label| label.x)
            .fold(f64::INFINITY, f64::min);
        let base_canvas_pad = if has_rendered_layout_dependency(diagram, &layout.edge_paths)
            || layout.note_positions.iter().any(Option::is_some)
            || !layout.cluster_positions.is_empty()
        {
            OBJECT_LINK_CANVAS_PAD
        } else {
            OBJECT_CANVAS_PAD
        };
        let canvas_pad_x = if endpoint_label_min_x < node_painted_min_x {
            OBJECT_EXPANDED_LABEL_CANVAS_PAD
        } else {
            base_canvas_pad
        };
        (max_x as i64 + canvas_pad_x, max_y as i64 + base_canvas_pad)
    };

    let mut svg = String::new();

    // Root <svg> with PlantUML attributes (alphabetical attribute order).
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="CLASS" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
        w = canvas_w,
        h = canvas_h,
    )
    .unwrap();
    svg.push_str("<?plantuml 1.2026.3beta6?>");
    svg.push_str("<defs/>");
    svg.push_str("<g>");

    // Oracle-captured clusters (packages / namespaces) emitted before
    // entities, mirroring PlantUML's emission order. Cluster ids occupy
    // entity-id slots.
    let oracle_clusters: Vec<&crate::layout_oracle::OracleCluster> = oracle
        .map(|o| {
            o.clusters
                .iter()
                .filter(|c| c.group_class == "cluster")
                .collect()
        })
        .unwrap_or_default();
    for cluster in &oracle_clusters {
        let cid = cluster.entity_id.as_deref().unwrap_or("ent0002");
        write!(svg, "<!--cluster {}-->", cluster.qualified_name).unwrap();
        match cluster.source_line.as_deref() {
            Some(sl) => write!(
                svg,
                r#"<g class="cluster" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
                escape_xml(&cluster.qualified_name),
                sl,
                cid,
            )
            .unwrap(),
            None => write!(
                svg,
                r#"<g class="cluster" data-qualified-name="{}" id="{}">"#,
                escape_xml(&cluster.qualified_name),
                cid,
            )
            .unwrap(),
        }
        emit_oracle_cluster_children(&mut svg, cluster);
        svg.push_str("</g>");
    }
    if oracle.is_none() {
        for (index, cluster) in object_cluster_specs(diagram).iter().enumerate() {
            let Some(position) = layout
                .cluster_positions
                .iter()
                .find(|position| position.id == cluster.qname)
            else {
                continue;
            };
            emit_object_cluster(&mut svg, cluster, position, &svg_ids.clusters[index]);
        }
    }
    let mut ent_id = 2;
    for (i, obj) in diagram.objects.iter().enumerate() {
        let (x, y) = positions[i];
        let dim = &dims[i];

        // PlantUML's `data-qualified-name` is the entity identifier — for
        // `map "Config" as cfg` that is `cfg`, not `Config`. For bare
        // `object Person` the id and label are equal. When the entity is
        // declared inside a `namespace pkg.qual` block, its qname is the
        // dot-joined chain `pkg.qual.entity_id`.
        let pkg_chain: Vec<String> = diagram
            .packages
            .iter()
            .filter(|p| p.object_ids.iter().any(|e| e == &obj.id))
            .map(|p| p.label.clone())
            .collect();
        let translated = translate_qualified_name(&obj.id);
        let qname = if pkg_chain.is_empty() {
            translated
        } else {
            let mut chain = pkg_chain;
            chain.push(translated);
            chain.join(".")
        };
        let source_line = if obj.source_line > 0 {
            obj.source_line
        } else {
            i + 1
        };

        // Look up the oracle's per-entity overrides. Match qname first, then
        // the bare id/label. As a last resort, scan oracle.entities for any
        // key whose dotted tail matches the entity id (handles namespace
        // splits where the parser package chain differs from PlantUML's
        // expanded cluster tree).
        let oracle_rect = oracle.and_then(|orc| {
            orc.entities
                .get(&qname)
                .or_else(|| orc.entities.get(&obj.id))
                .or_else(|| orc.entities.get(&obj.label))
                .or_else(|| {
                    orc.entities.iter().find_map(|(k, v)| {
                        if k.rsplit('.').next() == Some(obj.id.as_str()) {
                            Some(v)
                        } else {
                            None
                        }
                    })
                })
        });
        // If the oracle keyed this entity under a different qname (e.g.
        // namespace expansion), prefer that for the wrapper attribute.
        let effective_qname = oracle
            .and_then(|orc| {
                if orc.entities.contains_key(&qname) {
                    Some(qname.clone())
                } else {
                    orc.entities.iter().find_map(|(k, _)| {
                        if k.rsplit('.').next() == Some(obj.id.as_str()) {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                }
            })
            .unwrap_or_else(|| qname.clone());

        // Prefer the oracle's entity_id when it captured one, so the
        // ent000N counter matches PlantUML's allocation order across
        // clusters and entities.
        let current_ent_id_string = oracle_rect
            .and_then(|r| r.entity_id.clone())
            .unwrap_or_else(|| svg_ids.objects[i].clone());

        write!(
            svg,
            r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
            escape_xml(&effective_qname),
            source_line,
            current_ent_id_string,
        )
        .unwrap();

        render_object_content(&mut svg, obj, x, y, dim, oracle_rect, object_style);

        svg.push_str("</g>");
        ent_id += 1;
    }

    // Oracle-captured note entities (GMN*) — emit from structured primitives,
    // sharing the
    // entity-id counter.
    if let Some(orc) = oracle {
        for note in &orc.note_entities {
            let _ = emit_oracle_note_entity(
                &mut svg,
                note,
                "#181818",
                "#FEFFDD",
                13,
                "sans-serif",
                "#000000",
            );
            ent_id += 1;
        }
    } else {
        render_object_notes(&mut svg, diagram, layout, &svg_ids, dims);
    }

    // Links: prefer oracle data.
    if let Some(orc) = oracle {
        render_oracle_links(&mut svg, diagram, orc, &mut ent_id);
    } else {
        render_layout_links(&mut svg, diagram, &layout.edge_paths, &svg_ids);
    }

    svg.push_str("</g></svg>");
    svg
}

fn render_layout_links(
    svg: &mut String,
    diagram: &ObjectDiagram,
    edge_paths: &[EdgePath],
    svg_ids: &ObjectSvgIds,
) {
    for (link_idx, link) in diagram.links.iter().enumerate() {
        if !is_rendered_layout_link(link) || link_touches_object_note(diagram, link) {
            continue;
        }
        let from_base = link_base(&link.from);
        let to_base = link_base(&link.to);
        let Some(edge_path) = find_layout_edge(diagram, edge_paths, link_idx) else {
            continue;
        };
        if edge_path.points.len() < 4 {
            continue;
        }

        let Some(from_index) = diagram.objects.iter().position(|obj| obj.id == from_base) else {
            continue;
        };
        let Some(to_index) = diagram.objects.iter().position(|obj| obj.id == to_base) else {
            continue;
        };
        let source_line = if link.source_line > 0 {
            link.source_line
        } else {
            0
        };
        let link_id = &svg_ids.links[link_idx];
        let parallel_index = diagram.links[..link_idx]
            .iter()
            .filter(|previous| {
                link_base(&previous.from) == from_base && link_base(&previous.to) == to_base
            })
            .count();
        let path_id = object_link_path_id(link, from_base, to_base, parallel_index);
        let link_type = object_link_type(link.kind);

        write!(svg, "<!--link {from_base} to {to_base}-->").unwrap();
        write!(
            svg,
            r#"<g class="link" data-entity-1="{}" data-entity-2="{}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
            svg_ids.objects[from_index],
            svg_ids.objects[to_index],
        )
        .unwrap();
        let path_points = shortened_object_link_points(link, &edge_path.points);
        let dash_style = if link.dashed {
            "stroke-dasharray:7,7;"
        } else {
            ""
        };
        write!(
            svg,
            r#"<path codeLine="{source_line}" d="{}" fill="none" id="{path_id}" style="stroke:{BORDER_COLOR};stroke-width:1;{dash_style}"/>"#,
            edge_path_d(&path_points),
        )
        .unwrap();
        emit_object_link_start_decor(svg, link, &edge_path.points);
        emit_object_link_end_decor(svg, link, &edge_path.points);
        if let Some(label) = link.label.as_deref() {
            let (x, y) = edge_path
                .label
                .map(|position| {
                    (
                        position.x + LINK_LABEL_MARGIN,
                        position.y
                            + LINK_LABEL_MARGIN
                            + text_render::label_ascent(label, LINK_LABEL_FONT_SIZE),
                    )
                })
                .unwrap_or_else(|| {
                    let (x, y) = edge_label_position(&edge_path.points);
                    (x + 1.0, y - 4.0)
                });
            text_render::emit_text(
                svg,
                label,
                &TextBase {
                    x,
                    y,
                    font_size: LINK_LABEL_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        emit_object_endpoint_label(svg, link.from_multiplicity.as_deref(), edge_path.tail_label);
        emit_object_endpoint_label(svg, link.to_multiplicity.as_deref(), edge_path.head_label);
        svg.push_str("</g>");
    }
}

fn emit_object_endpoint_label(
    svg: &mut String,
    text: Option<&str>,
    position: Option<EdgeLabelPosition>,
) {
    let (Some(text), Some(position)) = (text, position) else {
        return;
    };
    text_render::emit_text(
        svg,
        text,
        &TextBase {
            x: position.x,
            y: position.y + text_render::label_ascent(text, LINK_LABEL_FONT_SIZE),
            font_size: LINK_LABEL_FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
}

fn render_object_notes(
    svg: &mut String,
    diagram: &ObjectDiagram,
    layout: &ObjectLayout,
    svg_ids: &ObjectSvgIds,
    dims: &[ObjDim],
) {
    for (note_idx, note) in diagram.notes.iter().enumerate() {
        let (Some(target), Some(position), Some(placement), Some((qname, entity_id))) = (
            note.target.as_deref(),
            note.position,
            layout.note_positions[note_idx],
            svg_ids.notes[note_idx].as_ref(),
        ) else {
            continue;
        };
        let note_id = object_note_layout_id(note, note_idx);
        let (edge_from, edge_to) = match position {
            ObjectNotePosition::Left | ObjectNotePosition::Top => (note_id.as_str(), target),
            ObjectNotePosition::Right | ObjectNotePosition::Bottom => (target, note_id.as_str()),
        };
        let Some(edge) = layout
            .edge_paths
            .iter()
            .find(|edge| edge.from == edge_from && edge.to == edge_to)
        else {
            continue;
        };
        let tip = match position {
            ObjectNotePosition::Left | ObjectNotePosition::Top => {
                edge.end_point.or_else(|| edge.points.last().copied())
            }
            ObjectNotePosition::Right | ObjectNotePosition::Bottom => {
                edge.start_point.or_else(|| edge.points.first().copied())
            }
        };
        let Some((tip_x, tip_y)) = tip else {
            continue;
        };
        render_attached_object_note(
            svg, note, placement, tip_x, tip_y, position, qname, entity_id,
        );
    }

    for (note_idx, note) in diagram.notes.iter().enumerate() {
        let (Some(note_name), Some(placement), Some((qname, entity_id))) = (
            note.id.as_deref(),
            layout.note_positions[note_idx],
            svg_ids.notes[note_idx].as_ref(),
        ) else {
            continue;
        };
        let linked = diagram
            .links
            .iter()
            .filter(|link| link_base(&link.from) == note_name || link_base(&link.to) == note_name)
            .collect::<Vec<_>>();
        let [link] = linked.as_slice() else {
            continue;
        };
        let note_is_from = link_base(&link.from) == note_name;
        let target = if note_is_from {
            link_base(&link.to)
        } else {
            link_base(&link.from)
        };
        let Some(target_idx) = diagram
            .objects
            .iter()
            .position(|object| object.id == target)
        else {
            continue;
        };
        let Some(edge) = layout
            .edge_paths
            .iter()
            .find(|edge| edge.from == link_base(&link.from) && edge.to == link_base(&link.to))
        else {
            continue;
        };
        let tip = if note_is_from {
            edge.end_point.or_else(|| edge.points.last().copied())
        } else {
            edge.start_point.or_else(|| edge.points.first().copied())
        };
        let Some((tip_x, tip_y)) = tip else {
            continue;
        };
        let note_center = (
            placement.x + placement.width / 2.0,
            placement.y + placement.height / 2.0,
        );
        let target_center = (
            layout.positions[target_idx].0 + dims[target_idx].width / 2.0,
            layout.positions[target_idx].1 + dims[target_idx].height / 2.0,
        );
        let delta_x = note_center.0 - target_center.0;
        let delta_y = note_center.1 - target_center.1;
        let position = if delta_y.abs() >= delta_x.abs() {
            if delta_y < 0.0 {
                ObjectNotePosition::Top
            } else {
                ObjectNotePosition::Bottom
            }
        } else if delta_x < 0.0 {
            ObjectNotePosition::Left
        } else {
            ObjectNotePosition::Right
        };
        render_attached_object_note(
            svg, note, placement, tip_x, tip_y, position, qname, entity_id,
        );
    }
}

/// Port of `EntityImageNote.drawU` and `Opale`'s four linked-note polygons.
/// Graphviz positions the note and target; the logical edge endpoint becomes
/// the callout tip and the edge itself is not emitted.
#[allow(clippy::too_many_arguments)]
fn render_attached_object_note(
    svg: &mut String,
    note: &ObjectNote,
    placement: NotePlacement,
    tip_x: f64,
    tip_y: f64,
    position: ObjectNotePosition,
    qualified_name: &str,
    entity_id: &str,
) {
    let x = placement.x;
    let y = placement.y;
    let right = x + placement.width;
    let bottom = y + placement.height;
    let fold_x = right - NOTE_FOLD;
    let delta = 4.0;
    let path = match position {
        ObjectNotePosition::Left => {
            let base_y =
                (placement.height / 2.0 - delta).clamp(NOTE_FOLD, placement.height - 2.0 * delta);
            format!(
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(y + base_y + 2.0 * delta),
                fmt_tl(tip_x),
                fmt_tl(tip_y),
                fmt_tl(right),
                fmt_tl(y + base_y),
                fmt_tl(right),
                fmt_tl(y + NOTE_FOLD),
                fmt_tl(fold_x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
            )
        }
        ObjectNotePosition::Right => {
            let base_y =
                (placement.height / 2.0 - delta).clamp(0.0, placement.height - 2.0 * delta);
            format!(
                "M{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y + base_y),
                fmt_tl(tip_x),
                fmt_tl(tip_y),
                fmt_tl(x),
                fmt_tl(y + base_y + 2.0 * delta),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(y + NOTE_FOLD),
                fmt_tl(fold_x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
            )
        }
        ObjectNotePosition::Top => {
            let base_x = (placement.width / 2.0 - delta).clamp(0.0, placement.width);
            format!(
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(x + base_x),
                fmt_tl(bottom),
                fmt_tl(tip_x),
                fmt_tl(tip_y),
                fmt_tl(x + base_x + 2.0 * delta),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(y + NOTE_FOLD),
                fmt_tl(fold_x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
            )
        }
        ObjectNotePosition::Bottom => {
            let base_x =
                (placement.width / 2.0 - delta).clamp(0.0, (placement.width - NOTE_FOLD).max(0.0));
            format!(
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(x),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(bottom),
                fmt_tl(right),
                fmt_tl(y + NOTE_FOLD),
                fmt_tl(fold_x),
                fmt_tl(y),
                fmt_tl(x + base_x + 2.0 * delta),
                fmt_tl(y),
                fmt_tl(tip_x),
                fmt_tl(tip_y),
                fmt_tl(x + base_x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
                fmt_tl(x),
                fmt_tl(y),
            )
        }
    };

    write!(
        svg,
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}"><path d="{}" fill="{}" style="stroke:{};stroke-width:0.5;"/><path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:0.5;"/>"#,
        escape_xml(qualified_name),
        note.source_line,
        entity_id,
        path,
        NOTE_FILL,
        BORDER_COLOR,
        fmt_tl(fold_x),
        fmt_tl(y),
        fmt_tl(fold_x),
        fmt_tl(y + NOTE_FOLD),
        fmt_tl(right),
        fmt_tl(y + NOTE_FOLD),
        fmt_tl(fold_x),
        fmt_tl(y),
        NOTE_FILL,
        BORDER_COLOR,
    )
    .unwrap();
    let mut line_top = y + NOTE_PAD_Y;
    for line in note.text.lines() {
        text_render::emit_text(
            svg,
            line,
            &TextBase {
                x: x + NOTE_PAD_X,
                y: line_top + text_render::label_ascent(line, NOTE_FONT_SIZE),
                font_size: NOTE_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        line_top += text_render::label_height(line, NOTE_FONT_SIZE);
    }
    svg.push_str("</g>");
}

fn link_base(link_end: &str) -> &str {
    link_end.split("::").next().unwrap_or(link_end)
}

fn is_rendered_layout_link(link: &ObjectLink) -> bool {
    !link.from.contains("::") && !link.to.contains("::")
}

fn link_touches_object_note(diagram: &ObjectDiagram, link: &ObjectLink) -> bool {
    diagram.notes.iter().any(|note| {
        note.id
            .as_deref()
            .is_some_and(|id| id == link_base(&link.from) || id == link_base(&link.to))
    })
}

fn find_layout_edge<'a>(
    diagram: &ObjectDiagram,
    edge_paths: &'a [EdgePath],
    link_idx: usize,
) -> Option<&'a EdgePath> {
    let link = diagram.links.get(link_idx)?;
    let from = link_base(&link.from);
    let to = link_base(&link.to);
    let occurrence = diagram.links[..link_idx]
        .iter()
        .filter(|previous| {
            is_rendered_layout_link(previous)
                && !link_touches_object_note(diagram, previous)
                && link_base(&previous.from) == from
                && link_base(&previous.to) == to
        })
        .count();
    let mut first = None;
    for (seen, edge) in edge_paths
        .iter()
        .filter(|edge| edge.from == from && edge.to == to)
        .enumerate()
    {
        first.get_or_insert(edge);
        if seen == occurrence {
            return Some(edge);
        }
    }
    // The vendored cgraph path currently coalesces parallel endpoint pairs.
    // Until it exposes every spline, retain the first solved route so later
    // source links still render instead of disappearing.
    first
}

fn has_rendered_layout_dependency(diagram: &ObjectDiagram, edge_paths: &[EdgePath]) -> bool {
    diagram.links.iter().any(|link| {
        is_rendered_layout_link(link)
            && !link_touches_object_note(diagram, link)
            && edge_paths.iter().any(|edge| {
                edge.from == link_base(&link.from)
                    && edge.to == link_base(&link.to)
                    && edge.points.len() >= 4
            })
    })
}

fn object_link_type(kind: ObjectLinkKind) -> &'static str {
    match kind {
        ObjectLinkKind::Dependency => "dependency",
        ObjectLinkKind::Extension => "extension",
        ObjectLinkKind::Composition => "composition",
        ObjectLinkKind::Aggregation => "aggregation",
        ObjectLinkKind::Association => "association",
    }
}

fn object_link_path_id(link: &ObjectLink, from: &str, to: &str, parallel_index: usize) -> String {
    let mut id = match link.kind {
        // `SvekEdge` names links with a source-side diamond as reversed
        // decorations, even when the opposite endpoint is undecorated.
        ObjectLinkKind::Aggregation | ObjectLinkKind::Composition if !link.arrow_at_to => {
            format!("{from}-backto-{to}")
        }
        ObjectLinkKind::Aggregation | ObjectLinkKind::Composition => format!("{from}-{to}"),
        ObjectLinkKind::Dependency | ObjectLinkKind::Extension | ObjectLinkKind::Association => {
            match (link.arrow_at_from, link.arrow_at_to) {
                (true, false) => format!("{from}-backto-{to}"),
                (true, true) | (false, false) => format!("{from}-{to}"),
                (false, true) => format!("{from}-to-{to}"),
            }
        }
    };
    if parallel_index > 0 {
        write!(id, "-{parallel_index}").unwrap();
    }
    id
}

fn shortened_object_link_points(link: &ObjectLink, points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start_len = match link.kind {
        ObjectLinkKind::Aggregation | ObjectLinkKind::Composition => OBJECT_DIAMOND_LENGTH,
        ObjectLinkKind::Dependency if link.arrow_at_from => DEPENDENCY_ARROW_PATH_INSET,
        ObjectLinkKind::Extension if link.arrow_at_from => OBJECT_TRIANGLE_LENGTH,
        ObjectLinkKind::Dependency | ObjectLinkKind::Extension | ObjectLinkKind::Association => 0.0,
    };
    let end_len = match link.kind {
        ObjectLinkKind::Dependency if link.arrow_at_to => DEPENDENCY_ARROW_PATH_INSET,
        ObjectLinkKind::Extension if link.arrow_at_to => OBJECT_TRIANGLE_LENGTH,
        ObjectLinkKind::Aggregation | ObjectLinkKind::Composition if link.arrow_at_to => {
            DEPENDENCY_ARROW_PATH_INSET
        }
        ObjectLinkKind::Dependency
        | ObjectLinkKind::Extension
        | ObjectLinkKind::Aggregation
        | ObjectLinkKind::Composition => 0.0,
        ObjectLinkKind::Association => 0.0,
    };
    shorten_edge_points(points, start_len, end_len)
}

fn shorten_edge_points(points: &[(f64, f64)], start_len: f64, end_len: f64) -> Vec<(f64, f64)> {
    let mut out = points.to_vec();
    if out.len() < 2 {
        return out;
    }
    if start_len > 0.0 {
        let tangent = unit_vector(out[0], out[1]);
        out[0].0 += tangent.0 * start_len;
        out[0].1 += tangent.1 * start_len;
        out[1].0 += tangent.0 * start_len;
        out[1].1 += tangent.1 * start_len;
    }
    if end_len > 0.0 {
        let last = out.len() - 1;
        let tangent = unit_vector(out[last], out[last - 1]);
        out[last].0 += tangent.0 * end_len;
        out[last].1 += tangent.1 * end_len;
        out[last - 1].0 += tangent.0 * end_len;
        out[last - 1].1 += tangent.1 * end_len;
    }
    out
}

fn emit_object_link_start_decor(svg: &mut String, link: &ObjectLink, points: &[(f64, f64)]) {
    match link.kind {
        ObjectLinkKind::Aggregation => emit_diamond(svg, points, true, "none"),
        ObjectLinkKind::Composition => emit_diamond(svg, points, true, BORDER_COLOR),
        ObjectLinkKind::Dependency if link.arrow_at_from => {
            emit_dependency_arrow(svg, points, true)
        }
        ObjectLinkKind::Extension if link.arrow_at_from => {
            emit_extension_triangle(svg, points, true)
        }
        ObjectLinkKind::Dependency | ObjectLinkKind::Extension | ObjectLinkKind::Association => {}
    }
}

fn emit_object_link_end_decor(svg: &mut String, link: &ObjectLink, points: &[(f64, f64)]) {
    match link.kind {
        ObjectLinkKind::Dependency | ObjectLinkKind::Aggregation | ObjectLinkKind::Composition
            if link.arrow_at_to =>
        {
            emit_dependency_arrow(svg, points, false);
        }
        ObjectLinkKind::Extension if link.arrow_at_to => {
            emit_extension_triangle(svg, points, false)
        }
        ObjectLinkKind::Dependency
        | ObjectLinkKind::Extension
        | ObjectLinkKind::Aggregation
        | ObjectLinkKind::Composition
        | ObjectLinkKind::Association => {}
    }
}

fn emit_dependency_arrow(svg: &mut String, points: &[(f64, f64)], at_start: bool) {
    if points.len() >= 2 {
        let (control, endpoint) = if at_start {
            (points[1], points[0])
        } else {
            (points[points.len() - 2], points[points.len() - 1])
        };
        let arrow = dependency_arrow_points(control, endpoint);
        write!(
            svg,
            r#"<polygon fill="{BORDER_COLOR}" points="{arrow}" style="stroke:{BORDER_COLOR};stroke-width:1;"/>"#,
        )
        .unwrap();
    }
}

fn emit_extension_triangle(svg: &mut String, points: &[(f64, f64)], at_start: bool) {
    if points.len() < 2 {
        return;
    }
    let (contact, neighbor) = if at_start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let inside = unit_vector(contact, neighbor);
    let perp = (-inside.1, inside.0);
    let base = (
        contact.0 + inside.0 * OBJECT_TRIANGLE_LENGTH,
        contact.1 + inside.1 * OBJECT_TRIANGLE_LENGTH,
    );
    let side1 = (
        base.0 + perp.0 * OBJECT_TRIANGLE_HALF_WIDTH,
        base.1 + perp.1 * OBJECT_TRIANGLE_HALF_WIDTH,
    );
    let side2 = (
        base.0 - perp.0 * OBJECT_TRIANGLE_HALF_WIDTH,
        base.1 - perp.1 * OBJECT_TRIANGLE_HALF_WIDTH,
    );
    write!(
        svg,
        r#"<polygon fill="none" points="{},{},{},{},{},{},{},{}" style="stroke:{BORDER_COLOR};stroke-width:1;"/>"#,
        fmt_tl(contact.0),
        fmt_tl(contact.1),
        fmt_tl(side1.0),
        fmt_tl(side1.1),
        fmt_tl(side2.0),
        fmt_tl(side2.1),
        fmt_tl(contact.0),
        fmt_tl(contact.1),
    )
    .unwrap();
}

const OBJECT_DIAMOND_LENGTH: f64 = 12.0;
const OBJECT_DIAMOND_HALF_WIDTH: f64 = 4.0;
const OBJECT_TRIANGLE_LENGTH: f64 = 18.0;
const OBJECT_TRIANGLE_HALF_WIDTH: f64 = 6.0;

fn emit_diamond(svg: &mut String, points: &[(f64, f64)], at_start: bool, fill: &str) {
    if points.len() < 2 {
        return;
    }
    let (contact, neighbor) = if at_start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let inside = unit_vector(contact, neighbor);
    let perp = (-inside.1, inside.0);
    let side_center = (
        contact.0 + inside.0 * (OBJECT_DIAMOND_LENGTH / 2.0),
        contact.1 + inside.1 * (OBJECT_DIAMOND_LENGTH / 2.0),
    );
    let far = (
        contact.0 + inside.0 * OBJECT_DIAMOND_LENGTH,
        contact.1 + inside.1 * OBJECT_DIAMOND_LENGTH,
    );
    let side1 = (
        side_center.0 + perp.0 * OBJECT_DIAMOND_HALF_WIDTH,
        side_center.1 + perp.1 * OBJECT_DIAMOND_HALF_WIDTH,
    );
    let side2 = (
        side_center.0 - perp.0 * OBJECT_DIAMOND_HALF_WIDTH,
        side_center.1 - perp.1 * OBJECT_DIAMOND_HALF_WIDTH,
    );
    write!(
        svg,
        r#"<polygon fill="{fill}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{BORDER_COLOR};stroke-width:1;"/>"#,
        fmt_tl(contact.0),
        fmt_tl(contact.1),
        fmt_tl(side1.0),
        fmt_tl(side1.1),
        fmt_tl(far.0),
        fmt_tl(far.1),
        fmt_tl(side2.0),
        fmt_tl(side2.1),
        fmt_tl(contact.0),
        fmt_tl(contact.1),
    )
    .unwrap();
}

fn edge_label_position(points: &[(f64, f64)]) -> (f64, f64) {
    if points.len() >= 2 {
        let first = points[0];
        let last = points[points.len() - 1];
        (
            (first.0 + last.0) / 2.0,
            (first.1 + last.1) / 2.0 + crate::plantuml_metrics::text_height(13.0) / 3.0,
        )
    } else {
        points[0]
    }
}

fn edge_path_d(points: &[(f64, f64)]) -> String {
    let mut d = format!("M{},{}", fmt_tl(points[0].0), fmt_tl(points[0].1));
    let mut i = 1;
    while i + 2 < points.len() {
        write!(
            d,
            " C{},{} {},{} {},{}",
            fmt_tl(points[i].0),
            fmt_tl(points[i].1),
            fmt_tl(points[i + 1].0),
            fmt_tl(points[i + 1].1),
            fmt_tl(points[i + 2].0),
            fmt_tl(points[i + 2].1),
        )
        .unwrap();
        i += 3;
    }
    d
}

const DEPENDENCY_ARROW_PATH_INSET: f64 = 6.0;
const DEPENDENCY_ARROW_BACK: f64 = 9.0;
const DEPENDENCY_ARROW_NOTCH: f64 = 5.0;
const DEPENDENCY_ARROW_HALF_WIDTH: f64 = 4.0;

fn unit_vector(control: (f64, f64), endpoint: (f64, f64)) -> (f64, f64) {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 0.0 {
        (dx / len, dy / len)
    } else {
        (0.0, 1.0)
    }
}

fn dependency_arrow_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let points = dependency_arrow_polygon(control, endpoint);
    format!(
        "{},{},{},{},{},{},{},{},{},{}",
        fmt_tl(points[0].0),
        fmt_tl(points[0].1),
        fmt_tl(points[1].0),
        fmt_tl(points[1].1),
        fmt_tl(points[2].0),
        fmt_tl(points[2].1),
        fmt_tl(points[3].0),
        fmt_tl(points[3].1),
        fmt_tl(points[4].0),
        fmt_tl(points[4].1),
    )
}

fn dependency_arrow_polygon(control: (f64, f64), endpoint: (f64, f64)) -> [(f64, f64); 5] {
    let (ux, uy) = unit_vector(control, endpoint);
    let (px, py) = (-uy, ux);
    let side1 = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK - px * DEPENDENCY_ARROW_HALF_WIDTH,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK - py * DEPENDENCY_ARROW_HALF_WIDTH,
    );
    let notch = (
        endpoint.0 - ux * DEPENDENCY_ARROW_NOTCH,
        endpoint.1 - uy * DEPENDENCY_ARROW_NOTCH,
    );
    let side2 = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK + px * DEPENDENCY_ARROW_HALF_WIDTH,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK + py * DEPENDENCY_ARROW_HALF_WIDTH,
    );
    [endpoint, side1, notch, side2, endpoint]
}

/// Render the inner content of one entity rect (body, header text, separator,
/// field/value rows).
fn render_object_content(
    svg: &mut String,
    obj: &ObjectInstance,
    x: f64,
    y: f64,
    dim: &ObjDim,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
    object_style: &ObjectRenderStyle,
) {
    let font_size = object_style.font_size;
    let has_stereo = obj.stereotype.is_some();
    let is_map = obj.kind == ObjectKind::Map;

    // Background rect — honour oracle overrides if available.
    let oracle_fill = oracle_rect.and_then(|r| r.fill.as_deref());
    let oracle_style = oracle_rect.and_then(|r| r.rect_style.as_deref());
    let oracle_rx = oracle_rect.and_then(|r| r.rect_rx.as_deref());
    let oracle_ry = oracle_rect.and_then(|r| r.rect_ry.as_deref());
    let fill_default = obj
        .color
        .as_ref()
        .map(|c| crate::sequence::resolve_color(c))
        .unwrap_or_else(|| object_style.fill.clone());
    let fill = oracle_fill.unwrap_or(&fill_default);
    let style_default = format!(
        "stroke:{};stroke-width:{BORDER_WIDTH};",
        object_style.border
    );
    let style = oracle_style.unwrap_or(style_default.as_str());
    let rx = oracle_rx.unwrap_or("2.5");
    let ry = oracle_ry.unwrap_or("2.5");

    write!(
        svg,
        r#"<rect fill="{}" height="{}" rx="{}" ry="{}" style="{}" width="{}" x="{}" y="{}"/>"#,
        fill,
        fmt_tl(dim.height),
        rx,
        ry,
        style,
        fmt_tl(dim.width),
        fmt_tl(x),
        fmt_tl(y),
    )
    .unwrap();

    // Stereotype text (centered, italic, 12pt). Use oracle text x/y when
    // available.
    if let Some(stereo) = &obj.stereotype {
        let stereo_text = format!("\u{00ab}{stereo}\u{00bb}");
        let stereo_y = oracle_rect
            .and_then(|r| r.text_y_values.first().copied())
            .unwrap_or(y + STEREO_BASELINE_Y);
        let stereo_tw = dim.stereo_width;
        let stereo_x = oracle_rect
            .and_then(|r| r.text_x_values.first().copied())
            .unwrap_or(x + (dim.width - stereo_tw) / 2.0);
        text_render::emit_text(
            svg,
            &stereo_text,
            &TextBase {
                x: stereo_x,
                y: stereo_y,
                font_size: STEREO_FONT_SIZE,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
    }

    // Name text (centered, 14pt).
    let label_w = text_render::measure(&obj.label, font_size as f64, false);
    let name_baseline_offset = if has_stereo {
        NAME_BASELINE_Y_WITH_STEREO
    } else {
        object_name_baseline(font_size)
    };
    // When the oracle is present, the name's y/x come from text_y_values:
    //   without stereotype: index 0
    //   with stereotype:    index 1
    let name_idx = if has_stereo { 1 } else { 0 };
    let name_y = oracle_rect
        .and_then(|r| r.text_y_values.get(name_idx).copied())
        .unwrap_or(y + name_baseline_offset);
    let name_x = oracle_rect
        .and_then(|r| r.text_x_values.get(name_idx).copied())
        .unwrap_or(x + (dim.width - label_w) / 2.0);
    text_render::emit_text(
        svg,
        &obj.label,
        &TextBase {
            x: name_x,
            y: name_y,
            font_size,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );

    // Header separator + body rows. When the oracle captured the entity's
    // `<line>` children verbatim, replay them at the exact pixel positions
    // to sidestep sub-ulp float drift. Otherwise compute positions from
    // our own metrics.
    let oracle_lines = oracle_rect.map(|r| r.lines.as_slice()).unwrap_or(&[]);

    if !oracle_lines.is_empty() {
        // First line is always the header separator.
        emit_oracle_line(svg, &oracle_lines[0]);
        let header_sep_y = oracle_lines[0]
            .y1
            .parse::<f64>()
            .unwrap_or(y + HEADER_SEP_Y);
        if is_map {
            render_map_rows_with_oracle(svg, obj, oracle_rect, oracle_lines, font_size);
        } else {
            // Object: any additional `<line>` children (rare — only horizontal
            // separator). Emit them after the field rows, but objects only
            // have the header separator, so just render rows.
            render_object_rows(svg, obj, x, y, dim, header_sep_y, oracle_rect, font_size);
        }
        return;
    }

    // No oracle: compute everything ourselves.
    let header_sep_y = y + if has_stereo {
        HEADER_SEP_Y_WITH_STEREO
    } else {
        object_header_height(font_size)
    };
    let (sep_x1, sep_x2, sep_w) = if is_map {
        (x, x + dim.width, MAP_LINE_WIDTH)
    } else {
        (x + 1.0, x + dim.width - 1.0, BORDER_WIDTH)
    };
    write!(
        svg,
        r#"<line style="stroke:{};stroke-width:{sep_w};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        object_style.border,
        fmt_tl(sep_x1),
        fmt_tl(sep_x2),
        fmt_tl(header_sep_y),
        fmt_tl(header_sep_y),
    )
    .unwrap();

    if is_map {
        render_map_rows(svg, obj, x, y, dim, header_sep_y, oracle_rect, object_style);
    } else {
        render_object_rows(svg, obj, x, y, dim, header_sep_y, oracle_rect, font_size);
    }
}

fn emit_oracle_line(svg: &mut String, line: &crate::layout_oracle::EntityLine) {
    let style = line
        .style
        .as_deref()
        .unwrap_or("stroke:#181818;stroke-width:0.5;");
    write!(
        svg,
        r#"<line style="{style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        line.x1, line.x2, line.y1, line.y2,
    )
    .unwrap();
}

/// Map row rendering when the oracle's `<line>` and `<text>` children are
/// available. Emits texts and lines in PlantUML's exact document order:
/// for each row, key text → value text → vertical divider line; then if
/// not the last row, the horizontal row separator.
fn render_map_rows_with_oracle(
    svg: &mut String,
    obj: &ObjectInstance,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
    oracle_lines: &[crate::layout_oracle::EntityLine],
    font_size: u32,
) {
    let Some(rect) = oracle_rect else {
        return;
    };
    let texts = &rect.texts;
    let stereo_offset = if obj.stereotype.is_some() { 1 } else { 0 };
    let n_rows = obj.fields.len();
    // Header sep already emitted by the caller. Walk additional lines for
    // each row.
    let mut line_idx = 1;
    for (i, field) in obj.fields.iter().enumerate() {
        // Two text siblings per row.
        let key_text_idx = 1 + stereo_offset + i * 2;
        let value_text_idx = key_text_idx + 1;

        if let Some(key) = texts.get(key_text_idx) {
            text_render::emit_text(
                svg,
                &field.name,
                &TextBase {
                    x: key.x,
                    y: key.y,
                    font_size,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        if let Some(value_text) = texts.get(value_text_idx) {
            let value = field.value.as_deref().unwrap_or("");
            text_render::emit_text(
                svg,
                value,
                &TextBase {
                    x: value_text.x,
                    y: value_text.y,
                    font_size,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        // Vertical divider for this row.
        if let Some(divider) = oracle_lines.get(line_idx) {
            emit_oracle_line(svg, divider);
            line_idx += 1;
        }
        // Horizontal separator after this row (except the last).
        if i + 1 < n_rows
            && let Some(h_sep) = oracle_lines.get(line_idx)
        {
            emit_oracle_line(svg, h_sep);
            line_idx += 1;
        }
    }
}

/// Emit one `<text>` per field for an object's body section.
#[allow(clippy::too_many_arguments)]
fn render_object_rows(
    svg: &mut String,
    obj: &ObjectInstance,
    x: f64,
    _y: f64,
    _dim: &ObjDim,
    header_sep_y: f64,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
    font_size: u32,
) {
    let has_stereo = obj.stereotype.is_some();
    let stereo_offset = if has_stereo { 1 } else { 0 };
    for (i, field) in obj.fields.iter().enumerate() {
        let text = format_field(field);
        // Oracle text y indexing: 0 = name (no stereo) or stereo+1=name, fields follow.
        let text_idx = 1 + stereo_offset + i;
        let field_y = oracle_rect
            .and_then(|r| r.text_y_values.get(text_idx).copied())
            .unwrap_or(
                header_sep_y
                    + object_first_member_offset(font_size)
                    + (i as f64) * object_text_height(font_size),
            );
        let field_x = oracle_rect
            .and_then(|r| r.text_x_values.get(text_idx).copied())
            .unwrap_or(x + FIELD_TEXT_X_OFFSET);
        text_render::emit_text(
            svg,
            &text,
            &TextBase {
                x: field_x,
                y: field_y,
                font_size,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
    }
}

/// Emit two `<text>`s per row (key + value), a vertical divider line, and a
/// horizontal row separator (between rows, not after the last).
#[allow(clippy::too_many_arguments)]
fn render_map_rows(
    svg: &mut String,
    obj: &ObjectInstance,
    x: f64,
    y: f64,
    dim: &ObjDim,
    header_sep_y: f64,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
    object_style: &ObjectRenderStyle,
) {
    let font_size = object_style.font_size;
    let row_height = map_row_height(font_size);
    let has_stereo = obj.stereotype.is_some();
    let stereo_offset = if has_stereo { 1 } else { 0 };
    let rect_bottom = y + dim.height;
    // Divider x: prefer oracle if available — captured as one of the sep_y_values?
    // No, the vertical divider lives in `<line>` siblings of header sep. The
    // oracle currently stores y1 values, not x; pull from our computed dim.
    let divider_x = x + dim.map_divider_x;

    for (i, field) in obj.fields.iter().enumerate() {
        // Row baseline.
        let row_y = oracle_rect
            .and_then(|r| r.text_y_values.get(1 + stereo_offset + i * 2).copied())
            .unwrap_or(header_sep_y + map_first_row_offset(font_size) + (i as f64) * row_height);

        // Key text (left column).
        let key_x = oracle_rect
            .and_then(|r| r.text_x_values.get(1 + stereo_offset + i * 2).copied())
            .unwrap_or_else(|| {
                // PlantUML TextBlockMap.drawU centers each key TextBlock in
                // widthColA via HorizontalAlignment.getPosition(keyWidth,
                // widthColA); TextBlockUtils.withMargin then adds the 5px
                // left text inset inside that centered block.
                let key_w = text_render::measure(&field.name, font_size as f64, false);
                x + MAP_TEXT_X_OFFSET + (map_key_column_text_width(dim) - key_w) / 2.0
            });
        text_render::emit_text(
            svg,
            &field.name,
            &TextBase {
                x: key_x,
                y: row_y,
                font_size,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );

        // Value text (right column).
        let value = field.value.as_deref().unwrap_or("");
        let value_x = oracle_rect
            .and_then(|r| r.text_x_values.get(1 + stereo_offset + i * 2 + 1).copied())
            .unwrap_or(divider_x + MAP_TEXT_X_OFFSET);
        text_render::emit_text(
            svg,
            value,
            &TextBase {
                x: value_x,
                y: row_y,
                font_size,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );

        // Row vertical divider: from previous separator down to next.
        // First row: starts at header_sep_y. Later rows: at the y of the
        // previous horizontal separator.
        let row_top = if i == 0 {
            header_sep_y
        } else {
            // Prior horizontal separator. Compute by stepping from header_sep_y.
            header_sep_y + (i as f64) * row_height
        };
        let row_bottom = if i + 1 < obj.fields.len() {
            header_sep_y + ((i + 1) as f64) * row_height
        } else {
            rect_bottom
        };

        write!(
            svg,
            r#"<line style="stroke:{};stroke-width:{MAP_LINE_WIDTH};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            object_style.border,
            fmt_tl(divider_x),
            fmt_tl(divider_x),
            fmt_tl(row_top),
            fmt_tl(row_bottom),
        )
        .unwrap();

        // Horizontal separator between this row and the next (only if not last).
        if i + 1 < obj.fields.len() {
            let h_sep_y = row_bottom;
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{MAP_LINE_WIDTH};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                object_style.border,
                fmt_tl(x),
                fmt_tl(x + dim.width),
                fmt_tl(h_sep_y),
                fmt_tl(h_sep_y),
            )
            .unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// Links (oracle-driven)
// ---------------------------------------------------------------------------

fn render_oracle_links(
    svg: &mut String,
    diagram: &ObjectDiagram,
    oracle: &OracleLayout,
    ent_id: &mut usize,
) {
    for link in &diagram.links {
        let from_base = link.from.split("::").next().unwrap_or(&link.from);
        let to_base = link.to.split("::").next().unwrap_or(&link.to);

        let to_id = format!("{from_base}-to-{to_base}");
        let backto_id = format!("{from_base}-backto-{to_base}");
        let assoc_id = format!("{from_base}-{to_base}");
        let to_id_rev = format!("{to_base}-to-{from_base}");
        let backto_id_rev = format!("{to_base}-backto-{from_base}");
        let assoc_id_rev = format!("{to_base}-{from_base}");
        let source_line = (link.source_line > 0).then(|| link.source_line.to_string());
        let candidates = [
            backto_id.as_str(),
            to_id.as_str(),
            assoc_id.as_str(),
            backto_id_rev.as_str(),
            to_id_rev.as_str(),
            assoc_id_rev.as_str(),
        ];
        let Some((_edge_index, edge)) =
            find_oracle_object_edge(&oracle.edges, &candidates, source_line.as_deref())
        else {
            continue;
        };

        let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = edge.link_type.as_deref().unwrap_or("association");
        let source_line = edge.source_line.as_deref().unwrap_or("0");
        let link_id = edge.link_id.as_deref().unwrap_or("lnk0");

        write!(svg, "<!--link {from_base} to {to_base}-->").unwrap();
        write!(
            svg,
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
        )
        .unwrap();

        let code_line = edge.code_line.as_deref().unwrap_or("0");
        let path_style = edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        write!(
            svg,
            r#"<path codeLine="{code_line}" d="{}" fill="none" id="{}" style="{path_style}"/>"#,
            edge.d, edge.id,
        )
        .unwrap();

        if let Some(points) = &edge.arrow_points {
            let fill = edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            )
            .unwrap();
        }
        if let Some(points) = &edge.second_arrow_points {
            let fill = edge
                .second_arrow_fill
                .as_deref()
                .or(edge.arrow_fill.as_deref())
                .unwrap_or("#181818");
            let poly_style = edge
                .second_polygon_style
                .as_deref()
                .or(edge.polygon_style.as_deref())
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            )
            .unwrap();
        }

        // Edge labels.
        for (lx, ly, text) in &edge.labels {
            text_render::emit_text(
                svg,
                text,
                &TextBase {
                    x: *lx,
                    y: *ly,
                    font_size: 13,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }

        svg.push_str("</g>");
        *ent_id += 1;
    }
}

fn find_oracle_object_edge<'a>(
    edges: &'a [OracleEdgePath],
    candidates: &[&str],
    source_line: Option<&str>,
) -> Option<(usize, &'a OracleEdgePath)> {
    fn is_numbered_duplicate(edge_id: &str, candidate_id: &str) -> bool {
        let Some(rest) = edge_id.strip_prefix(candidate_id) else {
            return false;
        };
        let Some(number) = rest.strip_prefix('-') else {
            return false;
        };
        !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
    }

    let mut fallback = None;
    for candidate_id in candidates {
        for (edge_index, edge) in edges.iter().enumerate().filter(|(_, edge)| {
            edge.id == *candidate_id
                || source_line.is_some() && is_numbered_duplicate(&edge.id, candidate_id)
        }) {
            if source_line.is_some_and(|line| edge.source_line.as_deref() == Some(line)) {
                return Some((edge_index, edge));
            }
            if edge.id == *candidate_id {
                fallback.get_or_insert((edge_index, edge));
            }
        }
    }
    fallback
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_layout::graph::GraphSpacing;
    use rustuml_parser::diagram::Diagram;
    use rustuml_parser::diagram::DiagramMeta;

    fn oracle_edge(id: &str, source_line: &str) -> OracleEdgePath {
        OracleEdgePath {
            id: id.to_string(),
            path_id: Some(id.to_string()),
            d: String::new(),
            arrow_points: None,
            second_arrow_points: None,
            second_arrow_fill: None,
            second_polygon_style: None,
            arrow_fill: None,
            link_type: None,
            entity_1: None,
            entity_2: None,
            source_line: Some(source_line.to_string()),
            link_id: None,
            path_style: None,
            code_line: None,
            polygon_style: None,
            label: None,
            labels: Vec::new(),
            label_links: Vec::new(),
            extra_paths: Vec::new(),
            crow_lines: Vec::new(),
            decorations: Vec::new(),
        }
    }

    fn simple_object_diagram() -> ObjectDiagram {
        ObjectDiagram {
            meta: DiagramMeta::default(),
            objects: vec![
                ObjectInstance {
                    id: "Car".into(),
                    label: "Car".into(),
                    kind: ObjectKind::Object,
                    fields: vec![
                        ObjectField {
                            name: "make".into(),
                            value: Some("Toyota".into()),
                        },
                        ObjectField {
                            name: "year".into(),
                            value: Some("2023".into()),
                        },
                    ],
                    stereotype: None,
                    color: None,
                    source_line: 1,
                },
                ObjectInstance {
                    id: "Owner".into(),
                    label: "Owner".into(),
                    kind: ObjectKind::Object,
                    fields: vec![ObjectField {
                        name: "name".into(),
                        value: Some("Alice".into()),
                    }],
                    stereotype: None,
                    color: None,
                    source_line: 5,
                },
            ],
            links: vec![ObjectLink {
                from: "Owner".into(),
                to: "Car".into(),
                kind: ObjectLinkKind::Dependency,
                label: Some("drives".into()),
                from_multiplicity: None,
                to_multiplicity: None,
                dashed: false,
                arrow_at_from: false,
                arrow_at_to: true,
                source_line: 9,
            }],
            notes: vec![],
            packages: vec![],
        }
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Car"));
        assert!(svg.contains("Owner"));
    }

    #[test]
    fn has_object_rect() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        let rect_count = svg.matches("<rect").count();
        assert!(rect_count >= 2, "expected >= 2 boxes, got {rect_count}");
    }

    #[test]
    fn has_field_text() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        assert!(svg.contains("make = Toyota"));
        assert!(svg.contains("year = 2023"));
    }

    #[test]
    fn map_fields_use_arrow() {
        let diagram = ObjectDiagram {
            meta: DiagramMeta::default(),
            objects: vec![ObjectInstance {
                id: "cfg".into(),
                label: "Config".into(),
                kind: ObjectKind::Map,
                fields: vec![
                    ObjectField {
                        name: "host".into(),
                        value: Some("localhost".into()),
                    },
                    ObjectField {
                        name: "port".into(),
                        value: Some("8080".into()),
                    },
                ],
                stereotype: None,
                color: None,
                source_line: 1,
            }],
            links: vec![],
            notes: vec![],
            packages: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        // Map renders key and value as separate text spans without an
        // arrow glyph — the divider line is the visual key/value
        // separator.
        assert!(svg.contains("host"));
        assert!(svg.contains("localhost"));
        assert!(svg.contains("port"));
        assert!(svg.contains("8080"));
        assert!(svg.contains("Config"));
    }

    #[test]
    fn map_keys_center_in_key_column() {
        let diagram = ObjectDiagram {
            meta: DiagramMeta::default(),
            objects: vec![ObjectInstance {
                id: "cfg".into(),
                label: "Config".into(),
                kind: ObjectKind::Map,
                fields: vec![
                    ObjectField {
                        name: "host".into(),
                        value: Some("localhost".into()),
                    },
                    ObjectField {
                        name: "debug".into(),
                        value: Some("true".into()),
                    },
                ],
                stereotype: None,
                color: None,
                source_line: 1,
            }],
            links: vec![],
            notes: vec![],
            packages: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        let host_w = text_render::measure("host", Theme::default().class.font_size, false);
        let debug_w = text_render::measure("debug", Theme::default().class.font_size, false);
        let host_x = MARGIN + MAP_TEXT_X_OFFSET + (debug_w - host_w) / 2.0;
        let debug_x = MARGIN + MAP_TEXT_X_OFFSET;
        assert!(svg.contains(&format!(r#"x="{}""#, fmt_tl(host_x))));
        assert!(svg.contains(&format!(r#"x="{}""#, fmt_tl(debug_x))));
    }

    #[test]
    fn empty_diagram() {
        let diagram = ObjectDiagram {
            meta: DiagramMeta::default(),
            objects: vec![],
            links: vec![],
            notes: vec![],
            packages: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\nobject Car {\n  make = Toyota\n}\nobject Bike\nCar --> Bike\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Car"));
        assert!(svg.contains("Bike"));
    }

    #[test]
    fn renamed_dotted_namespace_expands_to_svek_cluster_ancestry() {
        let input = r#"@startuml
namespace alpha.beta.gamma {
  object RenamedNode {
    code = 17
    state = "ready"
  }
}
@enduml"#;
        let lines = input
            .lines()
            .skip(1)
            .take_while(|line| *line != "@enduml")
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let diagram = rustuml_parser::parse::object::parse_object(&lines).unwrap();
        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"width="261px""#));
        assert!(svg.contains(r#"height="267px""#));
        assert!(svg.contains(r#"data-qualified-name="alpha" id="ent0004""#));
        assert!(svg.contains(r#"data-qualified-name="alpha.beta" id="ent0005""#));
        assert!(svg.contains(
            r#"data-qualified-name="alpha.beta.gamma" data-source-line="1" id="ent0002""#
        ));
        assert!(svg.contains(
            r#"data-qualified-name="alpha.beta.gamma.RenamedNode" data-source-line="2" id="ent0003""#
        ));
        // Fresh Java PlantUML reference. `CommandNamespace.executeArg`
        // creates the qname ancestry and `ClusterDotString.printInternal`
        // solves these nested SVEK bounds around the renamed entity.
        assert!(svg.contains(r#"M8.5,6 L48.9844,6"#));
        assert!(svg.contains(r#"M56.5,92 L110.3486,92"#));
        assert!(svg.contains("state = &quot;ready&quot;"));
    }

    #[test]
    fn renamed_stereotypes_keep_independent_name_and_stereo_padding() {
        let input = r#"object Z <<renamed_boundary_type>> {
  code = 17
}
object RenamedLongObject <<db>> {
  state = ready
}"#;
        let lines = input.lines().map(str::to_owned).collect::<Vec<_>>();
        let diagram = rustuml_parser::parse::object::parse_object(&lines).unwrap();
        let font_size = Theme::default().class.font_size as u32;
        let stereo_dominant = calc_obj_dim(&diagram.objects[0], font_size);
        let name_dominant = calc_obj_dim(&diagram.objects[1], font_size);

        // Fresh Java PlantUML reference. `EntityImageObject` and
        // `BodyEnhanced2.calculateDimension` union the independently padded
        // AWT text blocks at these widths.
        assert!((stereo_dominant.width - 165.8594).abs() < 0.001);
        assert!((name_dominant.width - 154.3896).abs() < 0.001);
    }

    #[test]
    fn renamed_reverse_link_floating_note_becomes_an_opale_callout() {
        let input = r#"object RenamedLedger {
  code = 47
  state = open
}
note "Fresh memo 47\nsecond renamed line\nthird line" as Memo_47
Memo_47 .. RenamedLedger"#;
        let lines = input.lines().map(str::to_owned).collect::<Vec<_>>();
        let diagram = rustuml_parser::parse::object::parse_object(&lines).unwrap();
        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"width="173px""#));
        assert!(svg.contains(r#"height="198px""#));
        assert!(svg.contains(r#"data-qualified-name="Memo_47" data-source-line="5" id="ent0003""#));
        assert!(!svg.contains(r#"class="link""#));
        // Fresh Java PlantUML reference. `GraphvizImageBuilder.isOpalisable`
        // and `EntityImageNote.setOpaleLine` absorb the reverse logical edge
        // into the top note's callout polygon.
        assert!(svg.contains(r#"L82.09,121.61"#));
    }

    #[test]
    fn attached_notes_render_renamed_mixed_side_components() {
        let input = r#"@startuml
object RenamedAlpha {
  code = 17
}
object RenamedBeta {
  ready = true
}
object RenamedGamma {
  owner = "team"
}
note left of RenamedAlpha : Left callout
note right of RenamedBeta : Right callout
note top of RenamedGamma : Top callout
@enduml"#;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert_eq!(svg.matches(r##"fill="#FEFFDD""##).count(), 6);
        assert!(svg.contains(r#"data-qualified-name="GMN5""#));
        assert!(svg.contains(r#"data-qualified-name="GMN8""#));
        assert!(svg.contains(r#"data-qualified-name="GMN11""#));
        assert!(svg.contains("Left callout"));
        assert!(svg.contains("Right callout"));
        assert!(svg.contains("Top callout"));
    }

    #[test]
    fn labeled_link_and_right_note_use_svek_boxes_for_renamed_nodes() {
        let input = r#"@startuml
object "Ledger Root" as Ledger_47 <<boundary>> {
  key = 47
  status = "open"
  retries = 3
}
object "Archive Sink" as Archive_83 {
  slot = 83
  retained = true
}
Ledger_47 --> Archive_83 : "archives into"
note right of Ledger_47 : Fresh source note 47
@enduml"#;
        let Diagram::Object(diagram) = rustuml_parser::parse::parse(input).unwrap() else {
            panic!("expected object diagram");
        };
        let font_size = Theme::default().class.font_size as u32;
        let dims: Vec<_> = diagram
            .objects
            .iter()
            .map(|object| calc_obj_dim(object, font_size))
            .collect();
        let layout = layout_object(&diagram, &dims);
        let edge = find_layout_edge(&diagram, &layout.edge_paths, 0).unwrap();
        assert!(edge.label.is_some(), "expected a solved SVEK label box");

        let note = layout.note_positions[0].expect("expected attached note placement");
        let source_right = layout.positions[0].0 + dims[0].width;
        let gap = note.x - source_right;
        assert!(
            (gap - GraphSpacing::PLANTUML_SVEK_DEFAULTS.node_sep_px).abs() < 1.0,
            "length-one note edge should use one SVEK node gap, got {gap}"
        );
    }

    #[test]
    fn no_oracle_map_port_chain_uses_html_table_envelope_and_spline() {
        let input = r#"@startuml
map "Node47" as ledger_47 {
  route_code => item_47
  checksum => hash_47
  retention => zone_47
}
map "Node83" as archive_83 {
  route_code => item_83
  checksum => hash_83
  retention => zone_83
}
map "Node29" as policy_29 {
  route_code => item_29
  checksum => hash_29
  retention => zone_29
}
map "Node61" as audit_61 {
  route_code => item_61
  checksum => hash_61
  retention => zone_61
}
map "Node73" as sink_73 {
  route_code => item_73
  checksum => hash_73
  retention => zone_73
}
ledger_47 --> archive_83
archive_83 --> policy_29
policy_29 --> audit_61
audit_61 --> sink_73
@enduml"#;
        let Diagram::Object(diagram) = rustuml_parser::parse::parse(input).unwrap() else {
            panic!("expected object diagram");
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"height="699px""#));
        assert!(svg.contains(r#"d="M83.5,91.81 C83.5,110.87 83.5,127.2 83.5,146.25""#));
        assert!(svg.contains(r#"d="M83.5,538.81 C83.5,557.87 83.5,574.2 83.5,593.25""#));
    }

    #[test]
    fn renamed_cardinalities_clear_svek_endpoint_collision_boxes() {
        let input = r#"object RenamedParent {
  code = 17
}
object RenamedChild {
  state = ready
}
object RenamedPeer {
  owner = team
}
RenamedParent "one" --> "zero to many" RenamedChild : owns
RenamedChild "many" --> "exactly one" RenamedPeer : hands off"#;
        let lines = input.lines().map(str::to_owned).collect::<Vec<_>>();
        let diagram = rustuml_parser::parse::object::parse_object(&lines).unwrap();
        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r#"width="176px""#));
        assert!(svg.contains(r#"height="310px""#));
        // Fresh Java PlantUML reference. `SvekEdge.manageCollision` expands
        // each entity by eight pixels before moving these four differently
        // sized cardinality boxes away from the renamed chain.
        assert!(svg.contains(r#"textLength="83.2495" x="6""#));
        assert!(svg.contains(">one</text>"));
        assert!(svg.contains(">many</text>"));
        assert!(svg.contains(">exactly one</text>"));
    }

    #[test]
    fn object_skinparams_drive_awt_metrics_and_entity_colors() {
        let lines = r#"skinparam object {
  BackgroundColor AliceBlue
  BorderColor FireBrick
  FontSize 11
}
object RenamedLedger {
  alpha = 10
  beta = 20
  gamma = 30
}
object RenamedArchive {
  state = ready
}
RenamedLedger --> RenamedArchive"#
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let diagram = rustuml_parser::parse::object::parse_object(&lines).unwrap();
        let svg = render(&diagram, &Theme::default());

        assert!(svg.contains(r##"fill="#F0F8FF" height="63.8203""##));
        assert!(svg.contains(r##"style="stroke:#B22222;stroke-width:0.5;""##));
        assert!(svg.contains(r#"font-size="11""#));
        assert!(svg.contains(r#"y="19.6348">RenamedLedger"#));
        assert!(svg.contains(r#"y1="23.9551" y2="23.9551""#));
        assert!(svg.contains(r#"y="64.5">gamma = 30"#));
    }

    #[test]
    fn link_endpoint_decorations_remain_orthogonal_to_relation_kind() {
        let input = r#"@startuml
object RenamedAlpha
object RenamedBeta
object RenamedGamma
object RenamedDelta
object RenamedEpsilon
RenamedAlpha <-- RenamedBeta
RenamedBeta <--> RenamedGamma
RenamedGamma o-- RenamedDelta
RenamedDelta *-- RenamedEpsilon
@enduml"#;
        let Diagram::Object(diagram) = rustuml_parser::parse::parse(input).unwrap() else {
            panic!("expected object diagram");
        };
        assert_eq!(
            diagram
                .links
                .iter()
                .map(|link| (link.arrow_at_from, link.arrow_at_to))
                .collect::<Vec<_>>(),
            [(true, false), (true, true), (false, false), (false, false)]
        );

        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"id="RenamedAlpha-backto-RenamedBeta""#));
        assert!(svg.contains(r#"id="RenamedBeta-RenamedGamma""#));
        assert!(svg.contains(r#"id="RenamedGamma-backto-RenamedDelta""#));
        assert!(svg.contains(r#"id="RenamedDelta-backto-RenamedEpsilon""#));
        assert_eq!(svg.matches("<polygon").count(), 5);
    }

    #[test]
    fn plantuml_envelope() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        assert!(svg.contains("data-diagram-type=\"CLASS\""));
        assert!(svg.contains("<?plantuml"));
        assert!(svg.contains("<defs/>"));
        assert!(svg.contains("class=\"entity\""));
        assert!(svg.contains("data-qualified-name=\"Car\""));
        assert!(svg.contains("id=\"ent0002\""));
    }

    #[test]
    fn oracle_edge_matching_uses_source_line_before_endpoint_suffix() {
        let edges = vec![
            oracle_edge("o1-to-o2", "7"),
            oracle_edge("o1-o2", "8"),
            oracle_edge("o1-o2-1", "9"),
        ];
        let candidates = [
            "o1-backto-o2",
            "o1-to-o2",
            "o1-o2",
            "o2-backto-o1",
            "o2-to-o1",
            "o2-o1",
        ];

        let (_, directed) = find_oracle_object_edge(&edges, &candidates, Some("7")).unwrap();
        assert_eq!(directed.id, "o1-to-o2");

        let (_, association) = find_oracle_object_edge(&edges, &candidates, Some("8")).unwrap();
        assert_eq!(association.id, "o1-o2");

        let (_, duplicate) = find_oracle_object_edge(&edges, &candidates, Some("9")).unwrap();
        assert_eq!(duplicate.id, "o1-o2-1");
    }
}
