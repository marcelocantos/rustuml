// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! JSON/YAML visualization SVG renderer.
//!
//! Renders the data tree as a nested 2-column table: keys on the left
//! (blue-grey background), values on the right (white background).  Container
//! values (objects and arrays) expand into a sub-table in the value cell.
//! Highlighted nodes use a yellow background.

use rustuml_parser::diagram::json_diagram::{JsonDiagram, JsonNode, JsonNodeValue};

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ──────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 12.0;
const ROW_H: f64 = 24.0;
const PAD_X: f64 = 8.0;
const BORDER: f64 = 1.0;
const MARGIN: f64 = 16.0;
const MIN_KEY_W: f64 = 40.0;
const MIN_VAL_W: f64 = 60.0;

// ── Colours ───────────────────────────────────────────────────────────────────

const KEY_FILL: &str = "#B8D0E8";
const VAL_FILL: &str = "#F8F8F8";
const BORDER_COLOR: &str = "#808080";
const HIGHLIGHT_FILL: &str = "#FFEF99";

// ── Measurement types ─────────────────────────────────────────────────────────

struct Table {
    /// Width of the key column (0 if the table has no key column, e.g. arrays).
    key_w: f64,
    /// Width of the value column.
    val_w: f64,
    /// Total outer width including all borders.
    total_w: f64,
    /// Total outer height including all borders.
    total_h: f64,
    rows: Vec<Row>,
}

struct Row {
    key_text: String,
    value: RowValue,
    /// Height of this row (equals ROW_H for leaf values, subtable height for containers).
    row_h: f64,
    highlighted: bool,
}

enum RowValue {
    Leaf(String),
    Subtable(Box<Table>),
}

// ── Measurement ───────────────────────────────────────────────────────────────

/// Build a measured Table for a JsonNode.
///
/// For objects and arrays the node's children become rows.
/// For primitives a single-row table is returned.
fn measure_node(node: &JsonNode) -> Table {
    match &node.value {
        JsonNodeValue::Object { fields } if !fields.is_empty() => {
            let rows = fields
                .iter()
                .map(|f| {
                    let rv = node_as_row_value(f);
                    let row_h = row_value_h(&rv);
                    Row {
                        key_text: f.key.clone().unwrap_or_default(),
                        value: rv,
                        row_h,
                        highlighted: f.highlighted,
                    }
                })
                .collect();
            build_table(rows)
        }
        JsonNodeValue::Array { items } if !items.is_empty() => {
            let rows = items
                .iter()
                .map(|item| {
                    let rv = node_as_row_value(item);
                    let row_h = row_value_h(&rv);
                    Row {
                        key_text: String::new(),
                        value: rv,
                        row_h,
                        highlighted: item.highlighted,
                    }
                })
                .collect();
            build_table(rows)
        }
        _ => {
            // Primitive root or empty container — single-value display.
            let text = leaf_text(&node.value);
            let val_w = (metrics::text_width(&text, FONT_SIZE) + 2.0 * PAD_X).max(MIN_VAL_W);
            Table {
                key_w: 0.0,
                val_w,
                total_w: BORDER + val_w + BORDER,
                total_h: BORDER + ROW_H + BORDER,
                rows: vec![Row {
                    key_text: String::new(),
                    value: RowValue::Leaf(text),
                    row_h: ROW_H,
                    highlighted: node.highlighted,
                }],
            }
        }
    }
}

/// Build a RowValue for a node that appears as the value in a parent row.
fn node_as_row_value(node: &JsonNode) -> RowValue {
    match &node.value {
        JsonNodeValue::Object { fields } if !fields.is_empty() => {
            RowValue::Subtable(Box::new(measure_node(node)))
        }
        JsonNodeValue::Array { items } if !items.is_empty() => {
            RowValue::Subtable(Box::new(measure_node(node)))
        }
        _ => RowValue::Leaf(leaf_text(&node.value)),
    }
}

fn row_value_h(rv: &RowValue) -> f64 {
    match rv {
        RowValue::Leaf(_) => ROW_H,
        RowValue::Subtable(t) => t.total_h,
    }
}

fn build_table(rows: Vec<Row>) -> Table {
    let has_keys = rows.iter().any(|r| !r.key_text.is_empty());

    let key_w = if has_keys {
        rows.iter()
            .map(|r| {
                if r.key_text.is_empty() {
                    0.0
                } else {
                    metrics::text_width(&r.key_text, FONT_SIZE) + 2.0 * PAD_X
                }
            })
            .fold(MIN_KEY_W, f64::max)
    } else {
        0.0
    };

    let val_w = rows
        .iter()
        .map(|r| match &r.value {
            RowValue::Leaf(s) => (metrics::text_width(s, FONT_SIZE) + 2.0 * PAD_X).max(MIN_VAL_W),
            RowValue::Subtable(sub) => sub.total_w,
        })
        .fold(MIN_VAL_W, f64::max);

    let total_h = BORDER + rows.iter().map(|r| r.row_h + BORDER).sum::<f64>();
    let total_w = if has_keys {
        BORDER + key_w + BORDER + val_w + BORDER
    } else {
        BORDER + val_w + BORDER
    };

    Table {
        key_w,
        val_w,
        total_w,
        total_h,
        rows,
    }
}

/// The display text for a leaf value.
fn leaf_text(v: &JsonNodeValue) -> String {
    match v {
        JsonNodeValue::Null => String::from("␀"),
        JsonNodeValue::Bool { val } => {
            if *val {
                String::from("☑ true")
            } else {
                String::from("☐ false")
            }
        }
        JsonNodeValue::Number { val } => val.clone(),
        JsonNodeValue::Str { val } => val.clone(),
        JsonNodeValue::Array { items } if items.is_empty() => String::from("[ ]"),
        JsonNodeValue::Object { fields } if fields.is_empty() => String::from("{ }"),
        _ => String::new(),
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw_table(svg: &mut SvgBuilder, table: &Table, x: f64, y: f64) {
    let has_key_col = table.key_w > 0.0;
    let mut cur_y = y + BORDER;

    for row in &table.rows {
        let val_x = if has_key_col {
            // Draw key cell.
            let key_x = x + BORDER;
            let key_fill = if row.highlighted {
                HIGHLIGHT_FILL
            } else {
                KEY_FILL
            };
            svg.rect(key_x, cur_y, table.key_w, row.row_h, key_fill, BORDER_COLOR);
            if !row.key_text.is_empty() {
                let text_y = cur_y + (row.row_h + FONT_SIZE) / 2.0 - 2.0;
                svg.text(key_x + PAD_X, text_y, &row.key_text, "start", FONT_SIZE);
            }
            x + BORDER + table.key_w + BORDER
        } else {
            x + BORDER
        };

        // Draw value cell.
        match &row.value {
            RowValue::Leaf(text) => {
                let val_fill = if row.highlighted {
                    HIGHLIGHT_FILL
                } else {
                    VAL_FILL
                };
                svg.rect(val_x, cur_y, table.val_w, row.row_h, val_fill, BORDER_COLOR);
                if !text.is_empty() {
                    let text_y = cur_y + (row.row_h + FONT_SIZE) / 2.0 - 2.0;
                    svg.text(val_x + PAD_X, text_y, text, "start", FONT_SIZE);
                }
            }
            RowValue::Subtable(sub) => {
                draw_table(svg, sub, val_x, cur_y);
            }
        }

        cur_y += row.row_h + BORDER;
    }

    // Draw outer border on top of the cells.
    svg.raw(&format!(
        r#"<rect x="{x}" y="{y}" width="{}" height="{}" fill="none" stroke="{BORDER_COLOR}" stroke-width="1"/>"#,
        table.total_w,
        table.total_h,
    ));
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn render(diagram: &JsonDiagram, _theme: &Theme) -> String {
    let table = measure_node(&diagram.root);

    let total_w = table.total_w + 2.0 * MARGIN;
    let total_h = table.total_h + 2.0 * MARGIN;

    let mut svg = SvgBuilder::new(total_w, total_h);
    draw_table(&mut svg, &table, MARGIN, MARGIN);
    svg.finalize()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn renders_json_object() {
        let input = "@startjson\n{\"name\": \"Alice\", \"age\": 30}\n@endjson";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("name"), "svg should contain key 'name'");
        assert!(svg.contains("Alice"), "svg should contain value 'Alice'");
        assert!(svg.contains("age"), "svg should contain key 'age'");
        assert!(svg.contains("30"), "svg should contain value '30'");
    }

    #[test]
    fn renders_yaml_list() {
        let input = "@startyaml\n- apple\n- banana\n- cherry\n@endyaml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("apple"));
        assert!(svg.contains("banana"));
        assert!(svg.contains("cherry"));
    }

    #[test]
    fn renders_nested_json() {
        let input = "@startjson\n{\"user\": {\"name\": \"Bob\", \"role\": \"admin\"}}\n@endjson";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("user"));
        assert!(svg.contains("Bob"));
        assert!(svg.contains("admin"));
    }

    #[test]
    fn renders_highlight() {
        let input = "@startjson\n#highlight \"name\"\n{\"name\": \"Alice\", \"age\": 30}\n@endjson";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        // Highlighted cell should use the highlight colour.
        assert!(svg.contains(super::HIGHLIGHT_FILL));
    }

    #[test]
    fn empty_object_renders_placeholder() {
        let input = "@startjson\n{}\n@endjson";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("{ }"));
    }

    #[test]
    fn empty_array_renders_placeholder() {
        let input = "@startjson\n[]\n@endjson";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("[ ]"));
    }
}
