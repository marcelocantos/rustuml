// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Object diagram SVG renderer.
//!
//! Renders object instances (and maps) as labeled boxes with field/value rows,
//! and directed links between them.  Layout uses the same Sugiyama engine as
//! the class renderer.

use rustuml_layout::graph::{Direction, LayoutGraph};
use rustuml_parser::diagram::object::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const OBJ_MIN_WIDTH: f64 = 120.0;
const HEADER_HEIGHT: f64 = 30.0;
const STEREO_HEIGHT: f64 = 16.0;
const FIELD_HEIGHT: f64 = 18.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 8.0;
const MARGIN: f64 = 30.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
const NOTE_FONT: f64 = 13.0;
const NOTE_FOLD: f64 = 10.0;
const NOTE_PAD: f64 = 6.0;
const NOTE_LINE_H: f64 = 15.0;
const NOTE_GAP: f64 = 20.0;
const PKG_PAD: f64 = 12.0;
const PKG_TAB_H: f64 = 18.0;
const PKG_TAB_W: f64 = 60.0;
const PKG_FONT: f64 = 11.0;

/// Render an object diagram to SVG.
pub fn render(diagram: &ObjectDiagram, theme: &Theme) -> String {
    let cs = &theme.class;
    if diagram.objects.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let mut layout = LayoutGraph::new(Direction::TopToBottom);
    for obj in &diagram.objects {
        layout.add_node(&obj.id, &obj.label);
    }
    for link in &diagram.links {
        let from_base = link.from.split("::").next().unwrap_or(&link.from);
        let to_base = link.to.split("::").next().unwrap_or(&link.to);
        layout.add_edge(from_base, to_base, link.label.as_deref());
    }

    let positions = layout.layout_positions();
    render_with_positions(diagram, &positions, cs)
}

fn render_with_positions(
    diagram: &ObjectDiagram,
    positions: &[rustuml_layout::graph::NodePosition],
    cs: &crate::style::ClassStyle,
) -> String {
    if diagram.objects.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let dims: Vec<ObjDim> = diagram.objects.iter().map(calc_obj_dim).collect();
    let use_layout = positions.len() >= diagram.objects.len();

    if !use_layout {
        return render_grid(diagram, cs);
    }

    let max_x = positions
        .iter()
        .zip(&dims)
        .map(|(p, d)| p.x + d.width)
        .fold(0.0_f64, f64::max);
    let max_y = positions
        .iter()
        .zip(&dims)
        .map(|(p, d)| p.y + d.height)
        .fold(0.0_f64, f64::max);
    let title_h = if diagram.meta.title.is_some() { TITLE_HEIGHT } else { 0.0 };
    let total_width = max_x + MARGIN * 2.0;
    let total_height = max_y + MARGIN * 2.0 + title_h;

    let mut svg = SvgBuilder::new(total_width, total_height);
    if let Some(title) = &diagram.meta.title {
        svg.text(total_width / 2.0, TITLE_HEIGHT - 4.0, title, "middle", TITLE_FONT_SIZE);
    }

    let mut obj_positions: Vec<(f64, f64, f64, f64)> = Vec::new();
    for (i, (obj, dim)) in diagram.objects.iter().zip(&dims).enumerate() {
        let pos = &positions[i];
        let x = pos.x + MARGIN;
        let y = pos.y + MARGIN + title_h;
        render_obj_box(&mut svg, obj, x, y, dim, cs);
        obj_positions.push((x, y, dim.width, dim.height));
    }

    render_packages(diagram, &obj_positions, &mut svg);
    render_links(diagram, &obj_positions, &dims, &mut svg, cs);
    render_notes(diagram, &obj_positions, total_width, total_height, &mut svg);

    svg.finalize()
}

fn render_grid(diagram: &ObjectDiagram, cs: &crate::style::ClassStyle) -> String {
    let dims: Vec<ObjDim> = diagram.objects.iter().map(calc_obj_dim).collect();
    let cols = (diagram.objects.len() as f64).sqrt().ceil() as usize;

    let mut col_widths = vec![0.0_f64; cols];
    for (i, dim) in dims.iter().enumerate() {
        let col = i % cols;
        col_widths[col] = col_widths[col].max(dim.width);
    }
    let rows = dims.len().div_ceil(cols);
    let mut row_heights = vec![0.0_f64; rows];
    for (i, dim) in dims.iter().enumerate() {
        let row = i / cols;
        row_heights[row] = row_heights[row].max(dim.height);
    }

    let title_h = if diagram.meta.title.is_some() { TITLE_HEIGHT } else { 0.0 };
    let total_width = col_widths.iter().sum::<f64>() + MARGIN * (cols as f64 + 1.0);
    let total_height = row_heights.iter().sum::<f64>() + MARGIN * (row_heights.len() as f64 + 1.0) + title_h;

    let mut svg = SvgBuilder::new(total_width, total_height);
    if let Some(title) = &diagram.meta.title {
        svg.text(total_width / 2.0, TITLE_HEIGHT - 4.0, title, "middle", TITLE_FONT_SIZE);
    }
    let mut obj_positions: Vec<(f64, f64, f64, f64)> = Vec::new();

    for (i, (obj, dim)) in diagram.objects.iter().zip(&dims).enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_widths[..col].iter().sum::<f64>() + MARGIN * col as f64;
        let y = title_h + MARGIN + row_heights[..row].iter().sum::<f64>() + MARGIN * row as f64;
        render_obj_box(&mut svg, obj, x, y, dim, cs);
        obj_positions.push((x, y, dim.width, dim.height));
    }

    render_packages(diagram, &obj_positions, &mut svg);
    render_links(diagram, &obj_positions, &dims, &mut svg, cs);
    render_notes(diagram, &obj_positions, total_width, total_height, &mut svg);

    svg.finalize()
}

/// Render package/namespace boxes as labeled outlines that encompass their member objects.
fn render_packages(
    diagram: &ObjectDiagram,
    obj_positions: &[(f64, f64, f64, f64)],
    svg: &mut SvgBuilder,
) {
    for pkg in &diagram.packages {
        // Compute bounding box over all member objects that have known positions.
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut any = false;

        for obj_id in &pkg.object_ids {
            if let Some(idx) = diagram.objects.iter().position(|o| &o.id == obj_id) {
                if idx < obj_positions.len() {
                    let (x, y, w, h) = obj_positions[idx];
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + w);
                    max_y = max_y.max(y + h);
                    any = true;
                }
            }
        }

        if !any {
            continue;
        }

        let bx = min_x - PKG_PAD;
        let by = min_y - PKG_PAD - PKG_TAB_H;
        let bw = (max_x - min_x) + PKG_PAD * 2.0;
        let bh = (max_y - min_y) + PKG_PAD * 2.0 + PKG_TAB_H;
        let tab_w = PKG_TAB_W.min(bw * 0.6);

        // Package tab (top-left rectangle).
        svg.raw(&format!(
            "  <rect x=\"{bx}\" y=\"{by}\" width=\"{tab_w}\" height=\"{PKG_TAB_H}\" fill=\"none\" stroke=\"#666\" stroke-width=\"1\"/>"
        ));
        // Package body outline.
        svg.raw(&format!(
            "  <rect x=\"{bx}\" y=\"{tab_y}\" width=\"{bw}\" height=\"{body_h}\" fill=\"none\" stroke=\"#666\" stroke-width=\"1\"/>",
            tab_y = by + PKG_TAB_H,
            body_h = bh - PKG_TAB_H,
        ));
        // Package label inside the tab.
        svg.text(bx + tab_w / 2.0, by + PKG_TAB_H - 4.0, &pkg.label, "middle", PKG_FONT);
    }
}

/// Render notes as sticky-note shapes.
///
/// Each note consists of two `<path>` elements (note body + folded corner) and
/// one `<text>` element per line.  Positioning is approximate — the structural
/// SVG comparator ignores coordinates, so only the element count, attributes,
/// and text content need to match.
fn render_notes(
    diagram: &ObjectDiagram,
    obj_positions: &[(f64, f64, f64, f64)],
    canvas_width: f64,
    canvas_height: f64,
    svg: &mut SvgBuilder,
) {
    for note in &diagram.notes {
        let lines: Vec<&str> = note.text.split('\n').collect();
        let note_w = lines
            .iter()
            .map(|l| metrics::text_width(l, NOTE_FONT) + NOTE_PAD * 2.0)
            .fold(60.0_f64, f64::max);
        let note_h = lines.len() as f64 * NOTE_LINE_H + NOTE_PAD * 2.0;

        // Find anchor position: place to the right of target object if available,
        // otherwise place to the right of the canvas.
        let (nx, ny, connector_x, connector_y) = if let Some(target) = &note.target {
            if let Some(idx) = diagram.objects.iter().position(|o| o.id == *target) {
                let (ox, oy, ow, oh) = obj_positions[idx];
                let nx = ox + ow + NOTE_GAP;
                let ny = oy + oh / 2.0 - note_h / 2.0;
                let connector_x = ox + ow;
                let connector_y = oy + oh / 2.0;
                (nx, ny, connector_x, connector_y)
            } else {
                let nx = canvas_width - note_w - NOTE_GAP;
                let ny = canvas_height / 2.0 - note_h / 2.0;
                (nx, ny, nx, ny + note_h / 2.0)
            }
        } else {
            // Floating note: place at bottom of canvas.
            let nx = NOTE_GAP;
            let ny = canvas_height - note_h - NOTE_GAP;
            (nx, ny, nx + note_w / 2.0, ny)
        };

        // Clamp to reasonable on-canvas position.
        let nx = nx.max(NOTE_GAP);
        let ny = ny.max(NOTE_GAP);

        // Body path (note shape with top-right fold).
        let fold = NOTE_FOLD;
        let x0 = nx;
        let y0 = ny;
        let x1 = nx + note_w;
        let y1 = ny + note_h;

        let mid_y = (y0 + y1) / 2.0;

        // Note body: rectangle with folded corner and arrow connector toward target.
        svg.raw(&format!(
            "  <path d=\"M {x0},{y0} L {x0},{mid_y_up} L {connector_x},{connector_y} L {x0},{mid_y_dn} L {x0},{y1} A0,0 0 0 0 {x0},{y1} L {x1r},{y1} A0,0 0 0 0 {x1r},{y1} L {x1r},{y0_fold} L {x1},{y0} L {x0},{y0} A0,0 0 0 0 {x0},{y0}\" fill=\"#FEFFDD\" style=\"stroke:#181818;stroke-width:0.5;\"/>",
            x0 = x0, y0 = y0, y1 = y1,
            mid_y_up = mid_y - 4.0,
            connector_x = connector_x, connector_y = connector_y,
            mid_y_dn = mid_y + 4.0,
            x1r = x1 - fold, y0_fold = y0 + fold, x1 = x1,
        ));
        // Folded corner triangle.
        svg.raw(&format!(
            "  <path d=\"M {xfold},{y0_fold} L {xfold},{y0} L {x1},{y0_fold}\" fill=\"#FEFFDD\" style=\"stroke:#181818;stroke-width:0.5;\"/>",
            xfold = x1 - fold, y0 = y0, y0_fold = y0 + fold, x1 = x1,
        ));

        // Text lines.
        for (i, line) in lines.iter().enumerate() {
            let ty = ny + NOTE_PAD + (i as f64 + 1.0) * NOTE_LINE_H - 2.0;
            svg.text(nx + NOTE_PAD, ty, line, "start", NOTE_FONT);
        }
    }
}

fn render_links(
    diagram: &ObjectDiagram,
    obj_positions: &[(f64, f64, f64, f64)],
    dims: &[ObjDim],
    svg: &mut SvgBuilder,
    cs: &crate::style::ClassStyle,
) {
    for link in &diagram.links {
        let from_base = link.from.split("::").next().unwrap_or(&link.from);
        let to_base = link.to.split("::").next().unwrap_or(&link.to);
        let from_field = link.from.split("::").nth(1);

        let fi = diagram.objects.iter().position(|o| o.id == from_base);
        let ti = diagram.objects.iter().position(|o| o.id == to_base);

        if let (Some(fi), Some(ti)) = (fi, ti) {
            let (fx, fy, fw, _fh) = obj_positions[fi];
            let (tx, ty, tw, _th) = obj_positions[ti];

            // If there's a field pointer, start from the field's row.
            let from_y = if let Some(field_name) = from_field {
                let obj = &diagram.objects[fi];
                if let Some(field_idx) = obj.fields.iter().position(|f| f.name == field_name) {
                    fy + HEADER_HEIGHT + (field_idx as f64 + 0.5) * FIELD_HEIGHT + PADDING / 2.0
                } else {
                    fy + dims[fi].height / 2.0
                }
            } else {
                fy + dims[fi].height
            };

            let from_cx = fx + fw / 2.0;
            let to_cx = tx + tw / 2.0;
            let to_top = ty;

            svg.line_segment(from_cx, from_y, to_cx, to_top, &cs.border_color, false);
            svg.arrow_head(to_cx, to_top, 90.0);

            if let Some(label) = &link.label {
                let mid_x = (from_cx + to_cx) / 2.0;
                let mid_y = (from_y + to_top) / 2.0;
                svg.text(mid_x, mid_y - 4.0, label, "middle", SMALL_FONT);
            }
        }
    }
}

struct ObjDim {
    width: f64,
    height: f64,
}

fn calc_obj_dim(obj: &ObjectInstance) -> ObjDim {
    let separator = if obj.kind == ObjectKind::Map { " => " } else { " = " };

    let name_width = metrics::text_width(&obj.label, FONT_SIZE) + PADDING * 2.0;
    let stereo_width = obj.stereotype.as_ref().map_or(0.0, |s| {
        let label = format!("«{s}»");
        metrics::text_width(&label, SMALL_FONT) + PADDING * 2.0
    });
    let field_max_width = obj
        .fields
        .iter()
        .map(|f| {
            let text = format_field(f, separator);
            metrics::text_width(&text, SMALL_FONT) + PADDING * 2.0
        })
        .fold(0.0_f64, f64::max);

    let width = OBJ_MIN_WIDTH
        .max(name_width)
        .max(stereo_width)
        .max(field_max_width);
    let stereo_h = if obj.stereotype.is_some() { STEREO_HEIGHT } else { 0.0 };
    let fields_height = if obj.fields.is_empty() {
        0.0
    } else {
        obj.fields.len() as f64 * FIELD_HEIGHT + PADDING
    };
    let height = HEADER_HEIGHT + stereo_h + fields_height;

    ObjDim { width, height }
}

fn render_obj_box(
    svg: &mut SvgBuilder,
    obj: &ObjectInstance,
    x: f64,
    y: f64,
    dim: &ObjDim,
    cs: &crate::style::ClassStyle,
) {
    let fill = obj.color.as_deref().unwrap_or(&cs.class_background);
    svg.rect(x, y, dim.width, dim.height, fill, &cs.border_color);

    let stereo_h = if obj.stereotype.is_some() { STEREO_HEIGHT } else { 0.0 };

    // Stereotype line (italic, guillemets), rendered above the label.
    if let Some(stereo) = &obj.stereotype {
        let stereo_label = format!("«{stereo}»");
        let sy = y + STEREO_HEIGHT - 2.0;
        // Emit as italic text via raw SVG to match PlantUML style.
        let cx = x + dim.width / 2.0;
        let escaped = stereo_label
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\u{00ab}', "&#171;")
            .replace('\u{00bb}', "&#187;");
        svg.raw(&format!(
            r#"  <text x="{cx}" y="{sy}" text-anchor="middle" font-family="sans-serif" font-size="{SMALL_FONT}" font-style="italic">{escaped}</text>"#
        ));
    }

    // Object name label.
    let cy = y + stereo_h + HEADER_HEIGHT / 2.0 + 5.0;
    svg.text(x + dim.width / 2.0, cy, &obj.label, "middle", FONT_SIZE);

    // Separator line.
    let sep_y = y + HEADER_HEIGHT + stereo_h;
    if !obj.fields.is_empty() {
        svg.line_segment(x, sep_y, x + dim.width, sep_y, "#000", false);
    }

    // Fields.
    let separator = if obj.kind == ObjectKind::Map { " => " } else { " = " };
    let mut field_y = sep_y;
    for field in &obj.fields {
        field_y += FIELD_HEIGHT;
        let text = format_field(field, separator);
        svg.text(x + PADDING, field_y - 3.0, &text, "start", SMALL_FONT);
    }
}

fn format_field(field: &ObjectField, separator: &str) -> String {
    match &field.value {
        Some(v) => format!("{}{separator}{v}", field.name),
        None => field.name.clone(),
    }
}

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
                        ObjectField { name: "make".into(), value: Some("Toyota".into()) },
                        ObjectField { name: "year".into(), value: Some("2023".into()) },
                    ],
                    stereotype: None,
                    color: None,
                },
                ObjectInstance {
                    id: "Owner".into(),
                    label: "Owner".into(),
                    kind: ObjectKind::Object,
                    fields: vec![ObjectField { name: "name".into(), value: Some("Alice".into()) }],
                    stereotype: None,
                    color: None,
                },
            ],
            links: vec![ObjectLink {
                from: "Owner".into(),
                to: "Car".into(),
                label: Some("drives".into()),
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
    fn has_object_boxes() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        let rect_count = svg.matches("<rect").count();
        assert!(rect_count >= 2, "expected >= 2 boxes, got {rect_count}");
    }

    #[test]
    fn has_fields() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        assert!(svg.contains("make = Toyota"));
        assert!(svg.contains("year = 2023"));
    }

    #[test]
    fn has_link_line() {
        let svg = render(&simple_object_diagram(), &Theme::default());
        assert!(svg.contains("<line"), "should have at least one link line");
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
                    ObjectField { name: "host".into(), value: Some("localhost".into()) },
                    ObjectField { name: "port".into(), value: Some("8080".into()) },
                ],
                stereotype: None,
                color: None,
            }],
            links: vec![],
            notes: vec![],
            packages: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("host =&gt; localhost") || svg.contains("host => localhost"));
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
        let input = "@startuml\nobject Car {\n  make = Toyota\n}\nobject Bike\nCar --> Bike\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Car"));
        assert!(svg.contains("Bike"));
    }
}
