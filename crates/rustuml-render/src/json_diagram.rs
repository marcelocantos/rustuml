// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! JSON/YAML visualization SVG renderer.
//!
//! PlantUML emits JSON/YAML diagrams as `data-diagram-type="JSON"` (or
//! `"YAML"`) SVGs. A flat object/array is drawn as a single rounded box: a
//! `#F1F1F1` fill rect, then per row a bold key `<text>`, a plain value
//! `<text>`, a vertical `<line>` separating the key and value columns, and a
//! horizontal `<line>` separating consecutive rows. A `fill="none"` border
//! rect closes the box.
//!
//! Nested objects/arrays are laid out by PlantUML via its Smetana (graphviz)
//! engine as detached boxes joined by dashed bezier connectors; that geometry
//! is not reproduced here. Such diagrams fall through to a best-effort render.

use rustuml_parser::diagram::json_diagram::{DataFormat, JsonDiagram, JsonNode, JsonNodeValue};

use crate::layout_oracle::OracleLayout;
use crate::plantuml_metrics::{ascent, fmt_coord, text_height, text_width};
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── PlantUML JSON layout constants ───────────────────────────────────────────

const FONT_SIZE: f64 = 14.0;
/// Outer margin from the SVG edge to the box.
const MARGIN: f64 = 10.0;
/// Horizontal padding either side of text within a column (5px each side).
const CELL_PAD: f64 = 5.0;
/// Extra vertical space added to the text height to form a row.
const ROW_EXTRA: f64 = 4.0;
/// Baseline offset of text below the row top (above the ascent).
const TEXT_TOP_PAD: f64 = 2.0;
/// Corner radius of the rounded box.
const RX: f64 = 5.0;

const FILL: &str = "#F1F1F1";
const BORDER: &str = "#000000";

// ── Public entry points ───────────────────────────────────────────────────────

/// Render a JSON/YAML diagram to SVG (no oracle).
pub fn render(diagram: &JsonDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a JSON/YAML diagram. The oracle parameter is unused — the renderer
/// computes its own geometry from PlantUML-compatible font metrics.
pub fn render_with_oracle(
    diagram: &JsonDiagram,
    _theme: &Theme,
    _oracle: Option<&OracleLayout>,
) -> String {
    let diagram_type = match diagram.format {
        DataFormat::Json => "JSON",
        DataFormat::Yaml => "YAML",
    };

    // A flat single box is only possible when every value is a scalar
    // (a nested object/array spawns a detached box via the Smetana layout).
    if let Some(rows) = flat_rows(&diagram.root) {
        return render_single_box(&rows, diagram_type);
    }

    render_fallback(diagram, diagram_type)
}

// ── Single-box (flat) rendering ───────────────────────────────────────────────

struct FlatRow {
    /// Key text (empty for array items).
    key: String,
    /// Display text for the value (already in PlantUML display form).
    value: String,
    highlighted: bool,
}

/// If `node` is an object or array whose every value is a scalar, return its
/// rows. Otherwise `None` (nested children require the Smetana layout).
fn flat_rows(node: &JsonNode) -> Option<Vec<FlatRow>> {
    match &node.value {
        JsonNodeValue::Object { fields } if !fields.is_empty() => {
            let mut rows = Vec::with_capacity(fields.len());
            for f in fields {
                let value = scalar_display(&f.value)?;
                rows.push(FlatRow {
                    key: f.key.clone().unwrap_or_default(),
                    value,
                    highlighted: f.highlighted,
                });
            }
            Some(rows)
        }
        JsonNodeValue::Array { items } if !items.is_empty() => {
            let mut rows = Vec::with_capacity(items.len());
            for item in items {
                let value = scalar_display(&item.value)?;
                rows.push(FlatRow {
                    key: String::new(),
                    value,
                    highlighted: item.highlighted,
                });
            }
            Some(rows)
        }
        _ => None,
    }
}

/// PlantUML display string for a scalar value, or `None` for nested
/// objects/arrays (including empty ones, which PlantUML draws as detached
/// boxes connected by a dashed link).
fn scalar_display(v: &JsonNodeValue) -> Option<String> {
    match v {
        JsonNodeValue::Null => Some("\u{2400}".to_string()),
        JsonNodeValue::Bool { val } => Some(if *val {
            "\u{2611} true".to_string()
        } else {
            "\u{2610} false".to_string()
        }),
        JsonNodeValue::Number { val } => Some(val.clone()),
        JsonNodeValue::Str { val } => {
            if val.is_empty() {
                // PlantUML renders an empty string as a single non-breaking space.
                Some("\u{00a0}".to_string())
            } else {
                Some(val.clone())
            }
        }
        // Nested (including empty) structures are not flat.
        JsonNodeValue::Array { .. } | JsonNodeValue::Object { .. } => None,
    }
}

fn render_single_box(rows: &[FlatRow], diagram_type: &str) -> String {
    let has_keys = rows.iter().any(|r| !r.key.is_empty());

    // Column widths: 5px padding either side of the widest text in each column.
    let key_text_w = rows
        .iter()
        .map(|r| text_width(&r.key, FONT_SIZE, true))
        .fold(0.0_f64, f64::max);
    let val_text_w = rows
        .iter()
        .map(|r| text_width(&r.value, FONT_SIZE, false))
        .fold(0.0_f64, f64::max);

    let key_col_w = if has_keys {
        key_text_w + 2.0 * CELL_PAD
    } else {
        0.0
    };
    let val_col_w = val_text_w + 2.0 * CELL_PAD;
    let box_w = key_col_w + val_col_w;

    let row_h = text_height(FONT_SIZE) + ROW_EXTRA;
    let box_h = row_h * rows.len() as f64;

    let box_x = MARGIN;
    let box_y = MARGIN;
    let box_right = box_x + box_w;
    let val_col_x = box_x + key_col_w;

    // Canvas dimensions: box plus the outer margin, rounded up and padded by
    // one pixel to match PlantUML's reported integer px in the root <svg>.
    let canvas_w = (box_right + MARGIN).ceil() + 1.0;
    let canvas_h = (box_y + box_h + MARGIN).ceil() + 1.0;

    let mut svg = SvgBuilder::new_plantuml(canvas_w, canvas_h, diagram_type);

    // Background fill rect (stroke matches fill so only the fill shows).
    svg.raw_inline(&rounded_rect(box_x, box_y, box_w, box_h, FILL, FILL, 1.5));

    // The highlight rect for a row is emitted just before that row's top edge:
    // immediately after the fill for the first row, otherwise just before the
    // horizontal separator above the row.
    let highlight_rect = |y: f64| highlight_box(box_x, y, box_w, row_h);
    if rows[0].highlighted {
        svg.raw_inline(&highlight_rect(box_y));
    }

    let mut row_top = box_y;
    for (i, row) in rows.iter().enumerate() {
        let baseline = row_top + TEXT_TOP_PAD + ascent(FONT_SIZE);
        let row_bottom = row_top + row_h;

        if has_keys && !row.key.is_empty() {
            svg.raw_inline(&key_text(
                box_x + CELL_PAD,
                baseline,
                &row.key,
                text_width(&row.key, FONT_SIZE, true),
            ));
        }

        svg.raw_inline(&value_text(
            val_col_x + CELL_PAD,
            baseline,
            &row.value,
            text_width(&row.value, FONT_SIZE, false),
        ));

        // Vertical separator at the key/value column boundary (per row).
        if has_keys {
            svg.raw_inline(&line(val_col_x, row_top, val_col_x, row_bottom));
        }

        // Horizontal separator below this row, except after the last. The next
        // row's highlight rect (if any) is drawn immediately before it.
        if i + 1 < rows.len() {
            if rows[i + 1].highlighted {
                svg.raw_inline(&highlight_rect(row_bottom));
            }
            svg.raw_inline(&line(box_x, row_bottom, box_right, row_bottom));
        }

        row_top = row_bottom;
    }

    // Border rect.
    svg.raw_inline(&rounded_rect(
        box_x, box_y, box_w, box_h, "none", BORDER, 1.5,
    ));

    svg.finalize_plantuml()
}

// ── Element emitters (PlantUML attribute order) ───────────────────────────────

/// Highlight rect drawn behind a `#highlight`-ed row: inset 1.5px from the box
/// sides, `rx`/`ry` 2, fill and stroke both `#CCFF02`.
fn highlight_box(box_x: f64, row_top: f64, box_w: f64, row_h: f64) -> String {
    format!(
        r##"<rect fill="#CCFF02" height="{h}" rx="2" ry="2" style="stroke:#CCFF02;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"##,
        h = fmt_coord(row_h),
        w = fmt_coord(box_w - 2.0),
        x = fmt_coord(box_x + 1.5),
        y = fmt_coord(row_top),
    )
}

fn rounded_rect(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: f64) -> String {
    format!(
        r#"<rect fill="{fill}" height="{h}" rx="{RX}" ry="{RX}" style="stroke:{stroke};stroke-width:{sw};" width="{w}" x="{x}" y="{y}"/>"#,
        h = fmt_coord(h),
        w = fmt_coord(w),
        x = fmt_coord(x),
        y = fmt_coord(y),
    )
}

fn key_text(x: f64, y: f64, content: &str, text_len: f64) -> String {
    format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{c}</text>"##,
        tl = fmt_coord(text_len),
        x = fmt_coord(x),
        y = fmt_coord(y),
        c = escape_text(content),
    )
}

fn value_text(x: f64, y: f64, content: &str, text_len: f64) -> String {
    format!(
        r##"<text fill="#000000" font-family="sans-serif" font-size="14" lengthAdjust="spacing" textLength="{tl}" x="{x}" y="{y}">{c}</text>"##,
        tl = fmt_coord(text_len),
        x = fmt_coord(x),
        y = fmt_coord(y),
        c = escape_text(content),
    )
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    format!(
        r#"<line style="stroke:#000000;stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"/>"#,
        x1 = fmt_coord(x1),
        x2 = fmt_coord(x2),
        y1 = fmt_coord(y1),
        y2 = fmt_coord(y2),
    )
}

/// Escape text for SVG, matching PlantUML's numeric-entity encoding for
/// non-breaking spaces and the special JSON glyphs.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\u{00a0}' => out.push_str("&#160;"),
            '\u{2400}' => out.push_str("&#9216;"),
            '\u{2610}' => out.push_str("&#9744;"),
            '\u{2611}' => out.push_str("&#9745;"),
            _ => out.push(c),
        }
    }
    out
}

// ── Fallback rendering (nested structures, errors) ────────────────────────────

/// Best-effort render for diagrams the single-box path can't handle (nested
/// objects/arrays). Emits a PlantUML envelope so output is well-formed. These
/// cases require the Smetana layout for exact parity and are expected to remain
/// failing until that is implemented.
fn render_fallback(diagram: &JsonDiagram, diagram_type: &str) -> String {
    let rows: Vec<FlatRow> = match &diagram.root.value {
        JsonNodeValue::Object { fields } => fields
            .iter()
            .map(|f| FlatRow {
                key: f.key.clone().unwrap_or_default(),
                value: scalar_display(&f.value).unwrap_or_else(|| fallback_value(&f.value)),
                highlighted: f.highlighted,
            })
            .collect(),
        JsonNodeValue::Array { items } => items
            .iter()
            .map(|item| FlatRow {
                key: String::new(),
                value: scalar_display(&item.value).unwrap_or_else(|| fallback_value(&item.value)),
                highlighted: item.highlighted,
            })
            .collect(),
        _ => vec![FlatRow {
            key: String::new(),
            value: scalar_display(&diagram.root.value).unwrap_or_default(),
            highlighted: diagram.root.highlighted,
        }],
    };

    if rows.is_empty() {
        let svg = SvgBuilder::new_plantuml(20.0, 20.0, diagram_type);
        return svg.finalize_plantuml();
    }

    render_single_box(&rows, diagram_type)
}

/// Placeholder text for a nested value in the fallback path.
fn fallback_value(v: &JsonNodeValue) -> String {
    match v {
        JsonNodeValue::Object { fields } if fields.is_empty() => "\u{00a0}\u{00a0}\u{00a0}".into(),
        JsonNodeValue::Array { items } if items.is_empty() => "\u{00a0}\u{00a0}\u{00a0}".into(),
        JsonNodeValue::Object { .. } => "{...}".into(),
        JsonNodeValue::Array { .. } => "[...]".into(),
        _ => String::new(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rustuml_parser::diagram::Diagram;

    fn render_input(input: &str) -> String {
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        crate::render_svg(&diagram)
    }

    #[test]
    fn renders_json_object() {
        let svg = render_input("@startjson\n{\"name\": \"Alice\", \"age\": 30}\n@endjson");
        assert!(svg.contains("name"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("age"));
        assert!(svg.contains("30"));
    }

    #[test]
    fn emits_plantuml_envelope() {
        let svg = render_input("@startjson\n{\"name\": \"Alice\"}\n@endjson");
        assert!(svg.contains(r#"data-diagram-type="JSON""#));
        assert!(svg.contains("<defs/>"));
        assert!(svg.contains("</g></svg>"));
        assert!(svg.contains("font-weight=\"700\""));
        assert!(svg.contains("lengthAdjust=\"spacing\""));
        assert!(svg.contains("textLength="));
    }

    #[test]
    fn yaml_uses_yaml_diagram_type() {
        let svg = render_input("@startyaml\nname: Alice\nage: 30\n@endyaml");
        assert!(svg.contains(r#"data-diagram-type="YAML""#));
        assert!(svg.contains("Alice"));
    }

    #[test]
    fn renders_yaml_list() {
        let svg = render_input("@startyaml\n- apple\n- banana\n- cherry\n@endyaml");
        assert!(svg.contains("apple"));
        assert!(svg.contains("banana"));
        assert!(svg.contains("cherry"));
    }

    #[test]
    fn renders_bool_glyphs() {
        let svg = render_input("@startjson\n{\"a\": true, \"b\": false}\n@endjson");
        assert!(svg.contains("&#9745; true"));
        assert!(svg.contains("&#9744; false"));
    }

    #[test]
    fn renders_null_glyph() {
        let svg = render_input("@startjson\n{\"a\": null}\n@endjson");
        assert!(svg.contains("&#9216;"));
    }

    #[test]
    fn empty_string_renders_nbsp() {
        let svg = render_input("@startjson\n{\"a\": \"\"}\n@endjson");
        assert!(svg.contains("&#160;"));
    }

    #[test]
    fn parses_diagram_kind() {
        let d = rustuml_parser::parse::parse("@startjson\n{\"a\":1}\n@endjson").unwrap();
        assert!(matches!(d, Diagram::Json(_)));
    }
}
