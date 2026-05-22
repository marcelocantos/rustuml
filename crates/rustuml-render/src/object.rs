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

use rustuml_layout::graph::{Direction, LayoutGraph};
use rustuml_parser::diagram::object::*;

use crate::layout_oracle::OracleLayout;
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// ---------------------------------------------------------------------------
// PlantUML layout constants (extracted from golden SVGs).
// All offsets are relative to the entity rect's top-left corner.
// ---------------------------------------------------------------------------

/// Margin from SVG edge to entity boxes.
const MARGIN: f64 = 7.0;
/// Name baseline y relative to rect top (no stereotype).
const NAME_BASELINE_Y: f64 = 15.5352;
/// Header separator y relative to rect top (no stereotype).
const HEADER_SEP_Y: f64 = 20.4883;
/// Stereotype baseline y relative to rect top.
const STEREO_BASELINE_Y: f64 = 11.6016;
/// Name baseline y relative to rect top when stereotype is present.
const NAME_BASELINE_Y_WITH_STEREO: f64 = 29.668;
/// Header separator y relative to rect top when stereotype is present.
const HEADER_SEP_Y_WITH_STEREO: f64 = 34.6211;
/// Vertical advance between member baselines.
const MEMBER_SPACING: f64 = 16.4883;
/// First member baseline offset below the header separator.
const FIRST_MEMBER_OFFSET: f64 = 17.5351;
/// Empty body section height (objects with no fields).
const EMPTY_BODY_HEIGHT: f64 = 16.0;
/// Padding below last field to rect bottom.
const BODY_PAD_BOTTOM: f64 = 8.0;
/// X offset of object field text relative to rect left.
const FIELD_TEXT_X_OFFSET: f64 = 6.0;
/// X offset of map field text relative to rect/column left.
const MAP_TEXT_X_OFFSET: f64 = 5.0;
/// Map row baseline advance.
const MAP_ROW_HEIGHT: f64 = 20.4883;
/// Map first row baseline offset below header separator.
const MAP_FIRST_ROW_OFFSET: f64 = 15.5351;

const FONT_SIZE: u32 = 14;
const STEREO_FONT_SIZE: u32 = 12;

const ENTITY_FILL: &str = "#F1F1F1";
const BORDER_COLOR: &str = "#181818";
const BORDER_WIDTH: &str = "0.5";
const MAP_LINE_WIDTH: &str = "1";

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
    _theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if diagram.objects.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Compute intrinsic per-object dimensions. Oracle overrides apply later.
    let mut dims: Vec<ObjDim> = diagram.objects.iter().map(calc_obj_dim).collect();

    // Determine positions: oracle first, layout-rs fallback.
    let positions = if let Some(orc) = oracle {
        oracle_positions(diagram, &mut dims, orc)
    } else {
        layout_positions(diagram, &dims)
    };

    render_plantuml_svg(diagram, &dims, &positions, oracle)
}

// ---------------------------------------------------------------------------
// Object dimensions
// ---------------------------------------------------------------------------

struct ObjDim {
    width: f64,
    height: f64,
    /// Stereotype (with guillemets) text length, if any.
    stereo_width: f64,
    /// Map only: x of the vertical divider (relative to rect left).
    /// For non-map objects, this is unused.
    map_divider_x: f64,
}

fn calc_obj_dim(obj: &ObjectInstance) -> ObjDim {
    let label_w = text_render::measure(&obj.label, FONT_SIZE as f64, false);

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
            .map(|f| text_render::measure(&f.name, FONT_SIZE as f64, false))
            .fold(0.0_f64, f64::max);
        let value_w = obj
            .fields
            .iter()
            .map(|f| {
                let text = f.value.as_deref().unwrap_or("");
                text_render::measure(text, FONT_SIZE as f64, false)
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
            HEADER_SEP_Y
        };
        let body_h = if obj.fields.is_empty() {
            EMPTY_BODY_HEIGHT
        } else {
            obj.fields.len() as f64 * MAP_ROW_HEIGHT
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
                text_render::measure(&text, FONT_SIZE as f64, false)
            })
            .fold(0.0_f64, f64::max);

        // Header width: name (or stereotype) centred with 2 * 7px padding.
        let header_w = label_w.max(stereo_width) + 14.0;
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
            HEADER_SEP_Y
        };
        let body_h = if obj.fields.is_empty() {
            EMPTY_BODY_HEIGHT
        } else {
            obj.fields.len() as f64 * MEMBER_SPACING + BODY_PAD_BOTTOM
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
    match &f.value {
        Some(v) => format!("{} = {}", f.name, v),
        None => f.name.clone(),
    }
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
        let rect = oracle
            .entities
            .get(&obj.id)
            .or_else(|| oracle.entities.get(&obj.label));
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

fn layout_positions(diagram: &ObjectDiagram, dims: &[ObjDim]) -> Vec<(f64, f64)> {
    let mut layout = LayoutGraph::new(Direction::TopToBottom);
    for (obj, dim) in diagram.objects.iter().zip(dims) {
        layout.add_node(&obj.id, &obj.label, dim.width, dim.height);
    }
    for link in &diagram.links {
        let from_base = link.from.split("::").next().unwrap_or(&link.from);
        let to_base = link.to.split("::").next().unwrap_or(&link.to);
        layout.add_edge(from_base, to_base, link.label.as_deref());
    }
    match layout.layout_full(std::time::Duration::from_secs(5)) {
        Some(r) => r
            .node_positions
            .iter()
            .map(|p| (p.x + MARGIN, p.y + MARGIN))
            .collect(),
        None => (0..diagram.objects.len())
            .map(|i| (MARGIN, MARGIN + (i as f64) * 100.0))
            .collect(),
    }
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

fn render_plantuml_svg(
    diagram: &ObjectDiagram,
    dims: &[ObjDim],
    positions: &[(f64, f64)],
    oracle: Option<&OracleLayout>,
) -> String {
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
        (max_x as i64 + 13, max_y as i64 + 13)
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

    let mut ent_id = 2;
    for (i, obj) in diagram.objects.iter().enumerate() {
        let (x, y) = positions[i];
        let dim = &dims[i];
        let current_ent_id = format!("ent{:04}", ent_id);
        ent_id += 1;

        let qname = translate_qualified_name(&obj.label);
        let source_line = if obj.source_line > 0 {
            obj.source_line
        } else {
            i + 1
        };

        // Look up the oracle's per-entity overrides.
        let oracle_rect = oracle.and_then(|orc| {
            orc.entities
                .get(&qname)
                .or_else(|| orc.entities.get(&obj.label))
                .or_else(|| orc.entities.get(&obj.id))
        });

        write!(
            svg,
            r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
            escape_xml(&qname),
            source_line,
            current_ent_id,
        )
        .unwrap();

        render_object_content(&mut svg, obj, x, y, dim, oracle_rect);

        svg.push_str("</g>");
    }

    // Links: prefer oracle data.
    if let Some(orc) = oracle {
        render_oracle_links(&mut svg, diagram, orc, &mut ent_id);
    }

    svg.push_str("</g></svg>");
    svg
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
) {
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
        .unwrap_or_else(|| ENTITY_FILL.to_string());
    let fill = oracle_fill.unwrap_or(&fill_default);
    let style_default = format!("stroke:{BORDER_COLOR};stroke-width:{BORDER_WIDTH};");
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
    let label_w = text_render::measure(&obj.label, FONT_SIZE as f64, false);
    let name_baseline_offset = if has_stereo {
        NAME_BASELINE_Y_WITH_STEREO
    } else {
        NAME_BASELINE_Y
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
            font_size: FONT_SIZE,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );

    // Header separator line. For objects: stroke-width 0.5, inset 1px from
    // rect left/right. For maps: stroke-width 1, flush with rect edges.
    let header_sep_y = oracle_rect
        .and_then(|r| r.sep_y_values.first().copied())
        .unwrap_or(
            y + if has_stereo {
                HEADER_SEP_Y_WITH_STEREO
            } else {
                HEADER_SEP_Y
            },
        );
    let (sep_x1, sep_x2, sep_w) = if is_map {
        (x, x + dim.width, MAP_LINE_WIDTH)
    } else {
        (x + 1.0, x + dim.width - 1.0, BORDER_WIDTH)
    };
    write!(
        svg,
        r#"<line style="stroke:{BORDER_COLOR};stroke-width:{sep_w};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        fmt_tl(sep_x1),
        fmt_tl(sep_x2),
        fmt_tl(header_sep_y),
        fmt_tl(header_sep_y),
    )
    .unwrap();

    // Field rows.
    if is_map {
        render_map_rows(svg, obj, x, y, dim, header_sep_y, oracle_rect);
    } else {
        render_object_rows(svg, obj, x, y, dim, header_sep_y, oracle_rect);
    }
}

/// Emit one `<text>` per field for an object's body section.
fn render_object_rows(
    svg: &mut String,
    obj: &ObjectInstance,
    x: f64,
    _y: f64,
    _dim: &ObjDim,
    header_sep_y: f64,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
) {
    let has_stereo = obj.stereotype.is_some();
    let stereo_offset = if has_stereo { 1 } else { 0 };
    for (i, field) in obj.fields.iter().enumerate() {
        let text = format_field(field);
        // Oracle text y indexing: 0 = name (no stereo) or stereo+1=name, fields follow.
        let text_idx = 1 + stereo_offset + i;
        let field_y = oracle_rect
            .and_then(|r| r.text_y_values.get(text_idx).copied())
            .unwrap_or(header_sep_y + FIRST_MEMBER_OFFSET + (i as f64) * MEMBER_SPACING);
        let field_x = oracle_rect
            .and_then(|r| r.text_x_values.get(text_idx).copied())
            .unwrap_or(x + FIELD_TEXT_X_OFFSET);
        text_render::emit_text(
            svg,
            &text,
            &TextBase {
                x: field_x,
                y: field_y,
                font_size: FONT_SIZE,
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
fn render_map_rows(
    svg: &mut String,
    obj: &ObjectInstance,
    x: f64,
    y: f64,
    dim: &ObjDim,
    header_sep_y: f64,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
) {
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
            .unwrap_or(header_sep_y + MAP_FIRST_ROW_OFFSET + (i as f64) * MAP_ROW_HEIGHT);

        // Key text (left column).
        let key_x = oracle_rect
            .and_then(|r| r.text_x_values.get(1 + stereo_offset + i * 2).copied())
            .unwrap_or(x + MAP_TEXT_X_OFFSET);
        text_render::emit_text(
            svg,
            &field.name,
            &TextBase {
                x: key_x,
                y: row_y,
                font_size: FONT_SIZE,
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
                font_size: FONT_SIZE,
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
            header_sep_y + (i as f64) * MAP_ROW_HEIGHT
        };
        let row_bottom = if i + 1 < obj.fields.len() {
            header_sep_y + ((i + 1) as f64) * MAP_ROW_HEIGHT
        } else {
            rect_bottom
        };

        write!(
            svg,
            r#"<line style="stroke:{BORDER_COLOR};stroke-width:{MAP_LINE_WIDTH};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
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
                r#"<line style="stroke:{BORDER_COLOR};stroke-width:{MAP_LINE_WIDTH};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
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

        let oracle_edge = oracle
            .edges
            .iter()
            .find(|e| e.id == backto_id)
            .or_else(|| oracle.edges.iter().find(|e| e.id == to_id))
            .or_else(|| oracle.edges.iter().find(|e| e.id == assoc_id))
            .or_else(|| oracle.edges.iter().find(|e| e.id == backto_id_rev))
            .or_else(|| oracle.edges.iter().find(|e| e.id == to_id_rev))
            .or_else(|| oracle.edges.iter().find(|e| e.id == assoc_id_rev));

        let Some(edge) = oracle_edge else { continue };

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

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
                label: Some("drives".into()),
                from_multiplicity: None,
                to_multiplicity: None,
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
    fn plantuml_envelope() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        assert!(svg.contains("data-diagram-type=\"CLASS\""));
        assert!(svg.contains("<?plantuml"));
        assert!(svg.contains("<defs/>"));
        assert!(svg.contains("class=\"entity\""));
        assert!(svg.contains("data-qualified-name=\"Car\""));
        assert!(svg.contains("id=\"ent0002\""));
    }
}
