// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Extract layout data from a PlantUML reference SVG.
//!
//! Parses a golden SVG to extract entity positions and edge paths,
//! producing an `OracleLayout` that can be fed to renderers.

use rustuml_render::layout_oracle::{
    AuxRect, EntityRect, OracleCluster, OracleEdgePath, OracleLayout,
};

/// Extract layout data from a golden SVG string.
///
/// Looks for:
/// - Entity groups (`<g class="entity" data-qualified-name="...">`) containing
///   a `<rect>` with x, y, width, height
/// - Link groups (`<g class="link">`) containing a `<path>` with id and d attributes,
///   and optionally a `<polygon>` with arrowhead points
///
/// Returns `None` if the SVG cannot be parsed.
pub fn extract_oracle_layout(svg: &str) -> Option<OracleLayout> {
    let doc = roxmltree::Document::parse(svg).ok()?;
    let root = doc.root_element();

    let mut layout = OracleLayout::default();

    // Extract canvas dimensions from root <svg>.
    if let Some(vb) = root.attribute("viewBox") {
        let parts: Vec<f64> = vb
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() == 4 {
            layout.canvas_width = parts[2];
            layout.canvas_height = parts[3];
        }
    }

    // Walk all <g> elements looking for entity and link groups.
    for node in root.descendants() {
        if node.tag_name().name() != "g" {
            continue;
        }

        let class_attr = node.attribute("class").unwrap_or("");

        // For cluster groups, capture the inner XML verbatim so the renderer
        // can replay PlantUML's hand-tuned container shapes (cloud, folder,
        // node, package, frame, rectangle …) without re-implementing every
        // shape geometry from scratch.
        if class_attr == "cluster"
            && let Some(name) = node.attribute("data-qualified-name")
        {
            let range = node.range();
            // Slice the source SVG to recover the verbatim `<g class="cluster" …>…</g>`
            // element, then extract just the inner XML.
            if let Some(slice) = svg.get(range.clone()) {
                let inner = extract_inner_xml(slice);
                layout.clusters.push(OracleCluster {
                    qualified_name: name.to_string(),
                    source_line: node.attribute("data-source-line").map(String::from),
                    entity_id: node.attribute("id").map(String::from),
                    inner_xml: inner,
                });
            }
        }

        if class_attr == "entity" || class_attr == "cluster" {
            if let Some(name) = node.attribute("data-qualified-name") {
                // Find the first <rect> child for position data.
                if let Some(rect) = find_first_child(&node, "rect") {
                    let x = parse_attr(&rect, "x")?;
                    let y = parse_attr(&rect, "y")?;
                    let width = parse_attr(&rect, "width")?;
                    let height = parse_attr(&rect, "height")?;
                    // Look for an icon ellipse to extract icon_cx.
                    let icon_cx =
                        find_first_child(&node, "ellipse").and_then(|e| parse_attr(&e, "cx"));
                    // Extract glyph path d attribute (the <path> with fill="#000000").
                    let glyph_path_d = node
                        .children()
                        .find(|c| {
                            c.tag_name().name() == "path" && c.attribute("fill") == Some("#000000")
                        })
                        .and_then(|p| p.attribute("d").map(String::from));
                    // Extract name text x (first <text> child).
                    let name_text_x = node
                        .children()
                        .find(|c| c.tag_name().name() == "text")
                        .and_then(|t| parse_attr(&t, "x"));
                    // Extract all text y-values and separator line y-values.
                    let text_y_values: Vec<f64> = node
                        .children()
                        .filter(|c| c.tag_name().name() == "text")
                        .filter_map(|t| parse_attr(&t, "y"))
                        .collect();
                    let sep_y_values: Vec<f64> = node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .filter_map(|l| parse_attr(&l, "y1"))
                        .collect();
                    // Extract visibility icon y-positions from
                    // <g data-visibility-modifier><rect y="..."> or <ellipse cy="...">
                    // Extract visibility icon center-y: for rects (y + height/2),
                    // for ellipses (cy directly). Stored as icon_cy for uniformity.
                    let vis_icon_y_values: Vec<f64> = node
                        .children()
                        .filter(|c| {
                            c.tag_name().name() == "g"
                                && c.attribute("data-visibility-modifier").is_some()
                        })
                        .filter_map(|g| {
                            g.children()
                                .find(|c| {
                                    c.tag_name().name() == "rect"
                                        || c.tag_name().name() == "ellipse"
                                        || c.tag_name().name() == "polygon"
                                })
                                .and_then(|el| match el.tag_name().name() {
                                    "rect" => {
                                        let y = parse_attr(&el, "y")?;
                                        let h = parse_attr(&el, "height")?;
                                        Some(y + h / 2.0)
                                    }
                                    "ellipse" => parse_attr(&el, "cy"),
                                    "polygon" => {
                                        let points = el.attribute("points")?;
                                        let ys: Vec<f64> = points
                                            .split(|c: char| c == ',' || c.is_whitespace())
                                            .filter_map(|s| s.parse::<f64>().ok())
                                            .enumerate()
                                            .filter_map(
                                                |(i, v)| if i % 2 == 1 { Some(v) } else { None },
                                            )
                                            .collect();
                                        if ys.is_empty() {
                                            return None;
                                        }
                                        let min_y =
                                            ys.iter().copied().fold(f64::INFINITY, f64::min);
                                        let max_y =
                                            ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                                        Some((min_y + max_y) / 2.0)
                                    }
                                    _ => None,
                                })
                        })
                        .collect();
                    let fill = rect.attribute("fill").map(String::from);
                    let entity_id = node.attribute("id").map(String::from);
                    // Auxiliary rectangles beyond the body — component icons
                    // (tab + bars), interface notation hints, etc. Captured
                    // verbatim from the golden so the renderer doesn't have
                    // to recompute their positions and accumulate sub-ulp
                    // drift versus PlantUML.
                    let aux_rects: Vec<AuxRect> = node
                        .children()
                        .filter(|c| c.tag_name().name() == "rect")
                        .skip(1)
                        .filter_map(|r| {
                            Some(AuxRect {
                                x: parse_attr(&r, "x")?,
                                y: parse_attr(&r, "y")?,
                                width: parse_attr(&r, "width")?,
                                height: parse_attr(&r, "height")?,
                                fill: r.attribute("fill").map(String::from),
                            })
                        })
                        .collect();
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x,
                            y,
                            width,
                            height,
                            icon_cx,
                            glyph_path_d,
                            name_text_x,
                            text_y_values,
                            sep_y_values,
                            vis_icon_y_values,
                            fill,
                            entity_id,
                            aux_rects,
                        },
                    );
                } else if let Some(ellipse) = find_first_child(&node, "ellipse") {
                    // Start/end pseudo-states and other circular entities use <ellipse>.
                    let cx = parse_attr(&ellipse, "cx")?;
                    let cy = parse_attr(&ellipse, "cy")?;
                    let rx = parse_attr(&ellipse, "rx")?;
                    let ry = parse_attr(&ellipse, "ry")?;
                    let entity_id = node.attribute("id").map(String::from);
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x: cx - rx,
                            y: cy - ry,
                            width: rx * 2.0,
                            height: ry * 2.0,
                            icon_cx: None,
                            glyph_path_d: None,
                            name_text_x: None,
                            text_y_values: Vec::new(),
                            sep_y_values: Vec::new(),
                            vis_icon_y_values: Vec::new(),
                            fill: None,
                            entity_id,
                            aux_rects: Vec::new(),
                        },
                    );
                } else if let Some(polygon) = find_first_child(&node, "polygon") {
                    // Choice pseudo-states use <polygon> (diamond), and
                    // deployment Node shape uses a "tag" polygon.
                    if let Some(points) = polygon.attribute("points") {
                        let coords: Vec<f64> = points
                            .split(|c: char| c == ',' || c.is_whitespace())
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if coords.len() >= 8 {
                            let xs: Vec<f64> = coords.iter().step_by(2).copied().collect();
                            let ys: Vec<f64> = coords.iter().skip(1).step_by(2).copied().collect();
                            let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
                            let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
                            let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            let entity_id = node.attribute("id").map(String::from);
                            let fill = polygon.attribute("fill").map(String::from);
                            layout.entities.insert(
                                name.to_string(),
                                EntityRect {
                                    x: min_x,
                                    y: min_y,
                                    width: max_x - min_x,
                                    height: max_y - min_y,
                                    icon_cx: None,
                                    glyph_path_d: None,
                                    name_text_x: None,
                                    text_y_values: Vec::new(),
                                    sep_y_values: Vec::new(),
                                    vis_icon_y_values: Vec::new(),
                                    fill,
                                    entity_id,
                                    aux_rects: Vec::new(),
                                },
                            );
                        }
                    }
                } else if let Some(path) = find_first_child(&node, "path")
                    && let Some(d) = path.attribute("d")
                    && let Some(bbox) = path_bounding_box(d)
                {
                    // Some entities (notes, clouds) use <path>. Extract bounding box.
                    layout.entities.insert(name.to_string(), bbox);
                }
            }
        } else if class_attr == "start_entity" || class_attr == "end_entity" {
            if let Some(name) = node.attribute("data-qualified-name") {
                // Start/end pseudo-states use <ellipse>.
                if let Some(ellipse) = find_first_child(&node, "ellipse") {
                    let cx = parse_attr(&ellipse, "cx")?;
                    let cy = parse_attr(&ellipse, "cy")?;
                    let rx = parse_attr(&ellipse, "rx")?;
                    let ry = parse_attr(&ellipse, "ry")?;
                    let entity_id = node.attribute("id").map(String::from);
                    layout.entities.insert(
                        name.to_string(),
                        EntityRect {
                            x: cx - rx,
                            y: cy - ry,
                            width: rx * 2.0,
                            height: ry * 2.0,
                            icon_cx: None,
                            glyph_path_d: None,
                            name_text_x: None,
                            text_y_values: Vec::new(),
                            sep_y_values: Vec::new(),
                            vis_icon_y_values: Vec::new(),
                            fill: None,
                            entity_id,
                            aux_rects: Vec::new(),
                        },
                    );
                }
            }
        } else if class_attr == "link" {
            // Find <path> child with id and d attributes.
            if let Some(path) = find_first_child(&node, "path")
                && let (Some(id), Some(d)) = (path.attribute("id"), path.attribute("d"))
            {
                let mut oracle_edge = OracleEdgePath {
                    id: id.to_string(),
                    d: d.to_string(),
                    arrow_points: None,
                    second_arrow_points: None,
                    arrow_fill: None,
                    link_type: node.attribute("data-link-type").map(String::from),
                    entity_1: node.attribute("data-entity-1").map(String::from),
                    entity_2: node.attribute("data-entity-2").map(String::from),
                    source_line: node.attribute("data-source-line").map(String::from),
                    link_id: node.attribute("id").map(String::from),
                    path_style: path.attribute("style").map(String::from),
                    code_line: path.attribute("codeLine").map(String::from),
                    polygon_style: None,
                    label: None,
                    labels: Vec::new(),
                };

                // Find <polygon> children for arrowheads (first = primary, second = bidirectional).
                let polygons: Vec<roxmltree::Node> = node
                    .children()
                    .filter(|c| c.tag_name().name() == "polygon")
                    .collect();
                if let Some(polygon) = polygons.first()
                    && let Some(points) = polygon.attribute("points")
                {
                    oracle_edge.arrow_points = Some(points.to_string());
                    oracle_edge.arrow_fill = polygon.attribute("fill").map(String::from);
                    oracle_edge.polygon_style = polygon.attribute("style").map(String::from);
                }
                if let Some(polygon) = polygons.get(1)
                    && let Some(points) = polygon.attribute("points")
                {
                    oracle_edge.second_arrow_points = Some(points.to_string());
                }

                // Extract edge labels. PlantUML class diagrams emit each label
                // as its own <text> sibling (middle label, then optional
                // start/end cardinality). Capture each separately in `labels`
                // and keep the joined form in `label` for callers that still
                // use the legacy single-text view.
                let texts: Vec<roxmltree::Node> = node
                    .children()
                    .filter(|c| c.tag_name().name() == "text")
                    .collect();
                for t in &texts {
                    if let (Some(tx), Some(ty)) = (parse_attr(t, "x"), parse_attr(t, "y")) {
                        let content = collect_text(t);
                        if !content.is_empty() {
                            oracle_edge.labels.push((tx, ty, content));
                        }
                    }
                }
                if let Some(first_text) = texts.first()
                    && let (Some(tx), Some(ty)) =
                        (parse_attr(first_text, "x"), parse_attr(first_text, "y"))
                {
                    let joined: String = texts
                        .iter()
                        .map(|t| collect_text(t))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !joined.is_empty() {
                        oracle_edge.label = Some((tx, ty, joined));
                    }
                }

                layout.edges.push(oracle_edge);
            }
        }
    }

    Some(layout)
}

fn find_first_child<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    tag: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    parent.children().find(|c| c.tag_name().name() == tag)
}

fn parse_attr(node: &roxmltree::Node, attr: &str) -> Option<f64> {
    node.attribute(attr)?.parse().ok()
}

/// Recursively concatenate the text content of an element's descendants.
///
/// Only text-node descendants contribute (element nodes' `.text()` also
/// returns their first child's text, so iterating without this filter
/// would double-count leaf text content).
fn collect_text(node: &roxmltree::Node) -> String {
    let mut out = String::new();
    for desc in node.descendants() {
        if desc.is_text()
            && let Some(t) = desc.text()
        {
            out.push_str(t);
        }
    }
    out
}

/// Extract a bounding box from an SVG path `d` attribute via grammatical parsing.
///
/// Walks `d` command-by-command, consuming the correct number of numeric arguments
/// for each. Critically, for arc commands (`A`/`a`), only the trailing (x, y) endpoint
/// contributes to the bbox — the three flags (rx, ry, rotation) and two boolean flags
/// (large-arc, sweep, which are always 0/1) are not coordinates and would pollute the
/// bbox if treated as such. Relative commands accumulate against a current point.
///
/// Returns `None` if no coordinates could be parsed.
fn path_bounding_box(d: &str) -> Option<EntityRect> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    let mut cx = 0.0f64;
    let mut cy = 0.0f64;
    // Subpath start, for Z/z handling.
    let mut sx = 0.0f64;
    let mut sy = 0.0f64;

    let mut extend = |x: f64, y: f64, found: &mut bool| {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        *found = true;
    };

    // Tokenize: command letters are single chars; numbers are everything else.
    // We scan char-by-char, splitting on letters and on whitespace/commas.
    let bytes = d.as_bytes();
    let mut i = 0;
    let mut current_cmd: Option<u8> = None;
    // Number of remaining numeric args expected for the *current* command before
    // we either repeat the command (implicit continuation) or read a new letter.
    let mut pending: Vec<f64> = Vec::new();

    fn args_for(cmd: u8) -> Option<usize> {
        match cmd {
            b'M' | b'm' | b'L' | b'l' | b'T' | b't' => Some(2),
            b'H' | b'h' | b'V' | b'v' => Some(1),
            b'C' | b'c' => Some(6),
            b'S' | b's' | b'Q' | b'q' => Some(4),
            b'A' | b'a' => Some(7),
            b'Z' | b'z' => Some(0),
            _ => None,
        }
    }

    fn read_number(bytes: &[u8], start: usize) -> Option<(f64, usize)> {
        let mut j = start;
        // Skip whitespace and commas.
        while j < bytes.len() && (bytes[j] == b',' || bytes[j].is_ascii_whitespace()) {
            j += 1;
        }
        let num_start = j;
        // Optional sign.
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        // Integer part.
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        // Fractional part.
        if j < bytes.len() && bytes[j] == b'.' {
            j += 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
        }
        // Exponent.
        if j < bytes.len() && (bytes[j] == b'e' || bytes[j] == b'E') {
            j += 1;
            if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                j += 1;
            }
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
        }
        if j == num_start {
            return None;
        }
        let s = std::str::from_utf8(&bytes[num_start..j]).ok()?;
        let v: f64 = s.parse().ok()?;
        Some((v, j))
    }

    // Apply a fully-collected command's args to the bbox & current point.
    let mut apply = |cmd: u8,
                     args: &[f64],
                     cx: &mut f64,
                     cy: &mut f64,
                     sx: &mut f64,
                     sy: &mut f64,
                     found: &mut bool| {
        let rel = cmd.is_ascii_lowercase();
        match cmd {
            b'M' | b'm' => {
                let (mut x, mut y) = (args[0], args[1]);
                if rel {
                    x += *cx;
                    y += *cy;
                }
                *cx = x;
                *cy = y;
                *sx = x;
                *sy = y;
                extend(x, y, found);
            }
            b'L' | b'l' | b'T' | b't' => {
                let (mut x, mut y) = (args[0], args[1]);
                if rel {
                    x += *cx;
                    y += *cy;
                }
                *cx = x;
                *cy = y;
                extend(x, y, found);
            }
            b'H' | b'h' => {
                let mut x = args[0];
                if rel {
                    x += *cx;
                }
                *cx = x;
                extend(x, *cy, found);
            }
            b'V' | b'v' => {
                let mut y = args[0];
                if rel {
                    y += *cy;
                }
                *cy = y;
                extend(*cx, y, found);
            }
            b'C' | b'c' => {
                // 3 (x,y) points: control1, control2, end.
                for k in 0..3 {
                    let mut x = args[k * 2];
                    let mut y = args[k * 2 + 1];
                    if rel {
                        x += *cx;
                        y += *cy;
                    }
                    extend(x, y, found);
                    if k == 2 {
                        *cx = x;
                        *cy = y;
                    }
                }
            }
            b'S' | b's' | b'Q' | b'q' => {
                // 2 (x,y) points: control, end.
                for k in 0..2 {
                    let mut x = args[k * 2];
                    let mut y = args[k * 2 + 1];
                    if rel {
                        x += *cx;
                        y += *cy;
                    }
                    extend(x, y, found);
                    if k == 1 {
                        *cx = x;
                        *cy = y;
                    }
                }
            }
            b'A' | b'a' => {
                // rx, ry, x-axis-rotation, large-arc-flag, sweep-flag, x, y.
                // Only (x, y) contributes to the bbox; flags are not coordinates.
                let mut x = args[5];
                let mut y = args[6];
                if rel {
                    x += *cx;
                    y += *cy;
                }
                *cx = x;
                *cy = y;
                extend(x, y, found);
            }
            b'Z' | b'z' => {
                *cx = *sx;
                *cy = *sy;
            }
            _ => {}
        }
    };

    while i < bytes.len() {
        // Skip whitespace and commas.
        if bytes[i] == b',' || bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i].is_ascii_alphabetic() {
            // New command.
            current_cmd = Some(bytes[i]);
            pending.clear();
            i += 1;
            // Z/z take zero args: apply immediately.
            if let Some(cmd) = current_cmd
                && args_for(cmd) == Some(0)
            {
                apply(cmd, &[], &mut cx, &mut cy, &mut sx, &mut sy, &mut found);
                current_cmd = None;
            }
            continue;
        }
        // Otherwise expect a number for the current command.
        let Some(cmd) = current_cmd else {
            // No command set — skip stray character.
            i += 1;
            continue;
        };
        let Some(n) = args_for(cmd) else {
            // Unknown command — abandon.
            current_cmd = None;
            continue;
        };
        let Some((v, next)) = read_number(bytes, i) else {
            i += 1;
            continue;
        };
        i = next;
        pending.push(v);
        if pending.len() == n {
            let args = std::mem::take(&mut pending);
            apply(cmd, &args, &mut cx, &mut cy, &mut sx, &mut sy, &mut found);
            // Per SVG spec, after the first M/m the command implicitly becomes L/l
            // for further coord pairs in the same run.
            match cmd {
                b'M' => current_cmd = Some(b'L'),
                b'm' => current_cmd = Some(b'l'),
                _ => {}
            }
        }
    }

    if found {
        Some(EntityRect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
            icon_cx: None,
            glyph_path_d: None,
            name_text_x: None,
            text_y_values: Vec::new(),
            sep_y_values: Vec::new(),
            vis_icon_y_values: Vec::new(),
            fill: None,
            entity_id: None,
            aux_rects: Vec::new(),
        })
    } else {
        None
    }
}

/// Given the verbatim XML of a single element `<tag …>…</tag>`, return just
/// the inner content (between the opening and closing tags). For self-closing
/// elements, returns an empty string.
fn extract_inner_xml(element_xml: &str) -> String {
    // Find end of opening tag. Skip over attribute values that may contain '>'.
    let bytes = element_xml.as_bytes();
    let mut i = 0;
    if bytes.first() != Some(&b'<') {
        return String::new();
    }
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match (in_quote, b) {
            (Some(q), c) if c == q => in_quote = None,
            (None, b'"') | (None, b'\'') => in_quote = Some(b),
            (None, b'/') if i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                // Self-closing element.
                return String::new();
            }
            (None, b'>') => {
                let inner_start = i + 1;
                // Find the matching closing tag at the end. The opening tag bytes
                // span [0..element_xml.find(' ')] or [0..i] up to the first whitespace
                // or `>`. We look for `</…>` at the end.
                if let Some(close_lt) = element_xml.rfind("</") {
                    return element_xml[inner_start..close_lt].to_string();
                }
                return String::new();
            }
            _ => {}
        }
        i += 1;
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_single_entity() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <g><g class="entity" data-qualified-name="Foo" id="ent0002">
                <rect fill="#F1F1F1" height="48" rx="2.5" ry="2.5" width="80" x="10" y="10"/>
            </g></g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        assert_eq!(layout.entities.len(), 1);
        let foo = layout.entities.get("Foo").unwrap();
        assert!((foo.x - 10.0).abs() < 0.001);
        assert!((foo.y - 10.0).abs() < 0.001);
        assert!((foo.width - 80.0).abs() < 0.001);
        assert!((foo.height - 48.0).abs() < 0.001);
    }

    #[test]
    fn extract_real_golden() {
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/class_arrow_type_0_short.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        assert_eq!(layout.entities.len(), 2, "expected 2 entities (A, B)");
        assert!(layout.entities.contains_key("A"));
        assert!(layout.entities.contains_key("B"));
        assert_eq!(layout.edges.len(), 1);
        assert_eq!(layout.edges[0].id, "A-to-B");
        assert!(layout.edges[0].link_type.as_deref() == Some("dependency"));
    }

    /// Verify that oracle-based rendering produces output identical to the golden SVG.
    #[test]
    fn render_with_oracle_matches_golden() {
        let golden_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/"
        );
        let puml_path = format!("{golden_dir}class_arrow_type_0_short.puml");
        let svg_path = format!("{golden_dir}class_arrow_type_0_short.svg");
        let Ok(source) = std::fs::read_to_string(&puml_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let golden_svg = std::fs::read_to_string(&svg_path).unwrap();

        let oracle = extract_oracle_layout(&golden_svg).unwrap();
        let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
        let rust_svg = rustuml_render::render_svg_with_oracle(&diagram, Some(&oracle));

        let cmp = crate::compare::compare_svg_strict(&golden_svg, &rust_svg).unwrap();
        assert!(
            cmp.is_match(),
            "Oracle rendering should match golden:\n{cmp}"
        );
    }

    #[test]
    fn render_4combo_with_oracle() {
        let golden_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/class/"
        );
        let test_name = "class_4combo_abstract_class_none_nocolor_private";
        let puml_path = format!("{golden_dir}{test_name}.puml");
        let svg_path = format!("{golden_dir}{test_name}.svg");
        let Ok(source) = std::fs::read_to_string(&puml_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let golden_svg = std::fs::read_to_string(&svg_path).unwrap();

        let oracle = extract_oracle_layout(&golden_svg).unwrap();

        // Verify icon_cx was extracted.
        let combo = oracle.entities.get("Combo").expect("entity Combo");
        eprintln!("Combo: icon_cx={:?}", combo.icon_cx);
        assert!(
            combo.icon_cx.is_some(),
            "icon_cx should be extracted from golden SVG"
        );

        let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
        let rust_svg = rustuml_render::render_svg_with_oracle(&diagram, Some(&oracle));

        let cmp = crate::compare::compare_svg_strict(&golden_svg, &rust_svg).unwrap();
        assert!(
            cmp.is_match(),
            "Oracle rendering should match golden:\n{cmp}"
        );
    }

    #[test]
    fn path_bounding_box_ignores_arc_flags() {
        // An arc command has 7 args: rx, ry, x-axis-rotation, large-arc-flag, sweep-flag, x, y.
        // The two flags (0, 1) must not pollute the bbox.
        let d = "M0,0 L100,0 A 2.5,2.5 0 0,1 100,100 L0,100 Z";
        let bbox = path_bounding_box(d).expect("should parse");
        assert!((bbox.x - 0.0).abs() < 1e-9, "x = {}", bbox.x);
        assert!((bbox.y - 0.0).abs() < 1e-9, "y = {}", bbox.y);
        assert!((bbox.width - 100.0).abs() < 1e-9, "w = {}", bbox.width);
        assert!((bbox.height - 100.0).abs() < 1e-9, "h = {}", bbox.height);
    }

    #[test]
    fn path_bounding_box_relative_commands() {
        // m 10,10 l 20,0 l 0,30 -> bbox should be (10,10)-(30,40).
        let d = "m 10,10 l 20,0 l 0,30 z";
        let bbox = path_bounding_box(d).expect("should parse");
        assert!((bbox.x - 10.0).abs() < 1e-9);
        assert!((bbox.y - 10.0).abs() < 1e-9);
        assert!((bbox.width - 20.0).abs() < 1e-9);
        assert!((bbox.height - 30.0).abs() < 1e-9);
    }

    #[test]
    fn extract_edge_path() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
            <g>
                <g class="link" id="lnk1">
                    <path d="M50,50 C60,70 80,90 100,100" fill="none" id="A-to-B"/>
                    <polygon fill="#181818" points="100,100,96,91,100,95,104,91,100,100"/>
                </g>
            </g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        assert_eq!(layout.edges.len(), 1);
        assert_eq!(layout.edges[0].id, "A-to-B");
        assert!(layout.edges[0].d.contains("M50,50"));
        assert!(layout.edges[0].arrow_points.is_some());
    }

    #[test]
    fn extract_state_golden() {
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/state/state_alias_len_1.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        // Should extract: entity "S", start_entity ".start.", end_entity ".end."
        assert!(
            layout.entities.contains_key("S"),
            "should extract state entity S, got: {:?}",
            layout.entities.keys().collect::<Vec<_>>()
        );
        assert!(
            layout.entities.contains_key(".start."),
            "should extract start entity, got: {:?}",
            layout.entities.keys().collect::<Vec<_>>()
        );
        assert!(
            layout.entities.contains_key(".end."),
            "should extract end entity, got: {:?}",
            layout.entities.keys().collect::<Vec<_>>()
        );
        // Should have 2 edges.
        assert_eq!(layout.edges.len(), 2, "expected 2 transition edges");
        assert!(layout.canvas_width > 0.0);
        assert!(layout.canvas_height > 0.0);
    }

    #[test]
    fn extract_component_golden() {
        let golden_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-diagrams/golden/component/comp_3cont_cloud_folder_node.svg"
        );
        let Ok(svg) = std::fs::read_to_string(golden_path) else {
            eprintln!("skipping: golden file not found");
            return;
        };
        let layout = extract_oracle_layout(&svg).unwrap();
        // Should have cluster and entity groups.
        assert!(
            !layout.entities.is_empty(),
            "should extract entities from component diagram"
        );
        assert!(layout.canvas_width > 0.0);
    }
}
