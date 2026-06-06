// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Extract layout data from a PlantUML reference SVG.
//!
//! Parses a golden SVG to extract entity positions and edge paths,
//! producing an `OracleLayout` that can be fed to renderers.

use rustuml_render::layout_oracle::{
    ApointMark, AuxRect, CrowMark, EdgeDecoration, EdgeLabelLink, EntityImage, EntityLine,
    EntityPath, EntityPolygon, EntityRect, EntityText, JsonBox, JsonConnector, NoteBoxGeom,
    OracleCluster, OracleClusterChild, OracleClusterPolygon, OracleDecoration, OracleEdgePath,
    OracleEntity, OracleHandwrittenWarning, OracleLayout, OracleLegend, OracleLegendRect,
    OracleNoteChild, OracleNoteEllipse, OracleNoteEntity, OracleNoteImage, OracleNoteLine,
    OracleNoteLink, OracleNotePath, OracleNoteRect, OracleNoteText, RegionDivider,
};

/// Parse the coordinate pairs from a note's body path `d` string and recover
/// the box rectangle plus the leader apex. PlantUML draws a note as a
/// rounded-corner box (with a folded top-right corner) plus an optional
/// triangular "leader" notch pointing at the target. Every path point but the
/// apex lies on the box outline, so the box is the bounding rect of all points
/// except the single one that escapes it; that escaping point is the apex.
fn parse_note_geom(d: &str) -> Option<NoteBoxGeom> {
    // Collect every numeric coordinate pair following an L/M command. The
    // SVG arc segments (`A0,0 0 0 0 x,y`) repeat box-corner points, which is
    // harmless for a bounding box, but their radius/flag run of `0`s would be
    // mis-read as coordinates, so only take the trailing `x,y` of each token.
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for tok in d.split(['L', 'M', 'A']) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        // For an arc token the relevant pair is the last two numbers; for L/M
        // it is the only pair. Grab the last comma-joined pair on the token.
        let nums: Vec<f64> = tok
            .split([' ', ','])
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if nums.len() >= 2 {
            let n = nums.len();
            pts.push((nums[n - 2], nums[n - 1]));
        }
    }
    if pts.len() < 4 {
        return None;
    }
    // Find the apex: the point whose removal shrinks the bounding box the most,
    // i.e. the point lying outside the bbox of all the others. Notes always
    // have exactly one such escaping point (or none, for a floating note).
    let bbox = |skip: usize| {
        let mut minx = f64::INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        for (i, &(x, y)) in pts.iter().enumerate() {
            if i == skip {
                continue;
            }
            minx = minx.min(x);
            miny = miny.min(y);
            maxx = maxx.max(x);
            maxy = maxy.max(y);
        }
        (minx, miny, maxx, maxy)
    };
    let full = {
        let mut minx = f64::INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        for &(x, y) in &pts {
            minx = minx.min(x);
            miny = miny.min(y);
            maxx = maxx.max(x);
            maxy = maxy.max(y);
        }
        (minx, miny, maxx, maxy)
    };
    // Identify the apex as the point that, when excluded, most reduces the
    // bounding-box area. If excluding any single point doesn't change the box,
    // there is no leader (floating note).
    let mut apex_idx: Option<usize> = None;
    let mut apex: Option<(f64, f64)> = None;
    let mut box_rect = full;
    for (i, &pt) in pts.iter().enumerate() {
        let b = bbox(i);
        // A reduced box means pts[i] was an extreme (the apex).
        if b.0 > full.0 || b.1 > full.1 || b.2 < full.2 || b.3 < full.3 {
            apex_idx = Some(i);
            apex = Some(pt);
            box_rect = b;
            break;
        }
    }
    // The leader's two base points are the apex's path neighbours.
    let leader_base = apex_idx.and_then(|i| {
        let prev = if i > 0 { pts.get(i - 1).copied() } else { None };
        let next = pts.get(i + 1).copied();
        match (prev, next) {
            (Some(p), Some(n)) => Some((p, n)),
            _ => None,
        }
    });
    let (minx, miny, maxx, maxy) = box_rect;
    Some(NoteBoxGeom {
        x: minx,
        y: miny,
        width: maxx - minx,
        height: maxy - miny,
        apex,
        leader_base,
        text_x: None,
        text_y: None,
        text_lines: Vec::new(),
        children: Vec::new(),
    })
}

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

    // Extract canvas dimensions from root <svg>. Prefer the full-precision
    // `width`/`height` pixel attributes (`221.875px`) over the integer-floored
    // `viewBox` (`0 0 221 303`): for unscaled diagrams the px attr equals the
    // viewBox integer (so this is a no-op), but for `skinparam dpi`/`scale`
    // diagrams the px attr preserves the true scaled size, letting the scaling
    // pipeline recover the exact base canvas via division by `k`. Renderers
    // truncate the canvas to an integer for the root tag, so the extra
    // precision never alters unscaled output.
    let parse_px = |a: Option<&str>| -> Option<f64> {
        a.and_then(|s| s.strip_suffix("px").unwrap_or(s).trim().parse::<f64>().ok())
    };
    let vb_dims = root.attribute("viewBox").and_then(|vb| {
        let parts: Vec<f64> = vb
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        (parts.len() == 4).then_some((parts[2], parts[3]))
    });
    if let Some(w) = parse_px(root.attribute("width")) {
        layout.canvas_width = w;
    } else if let Some((w, _)) = vb_dims {
        layout.canvas_width = w;
    }
    if let Some(h) = parse_px(root.attribute("height")) {
        layout.canvas_height = h;
    } else if let Some((_, h)) = vb_dims {
        layout.canvas_height = h;
    }

    // Capture the opening `<svg ...>` tag verbatim (without trailing `>`),
    // so verbatim-replay renderers can reproduce theme-driven attributes
    // (fractional `height="260.4167px"`, themed `style="…"`) byte-for-byte.
    let root_range = root.range();
    if let Some(slice) = svg.get(root_range.start..root_range.end)
        && let Some(end) = slice.find('>')
    {
        layout.root_open_tag = Some(slice[..end].to_string());
    }

    // Capture <defs> inner XML verbatim. PlantUML stashes background-color
    // filters here; renderers that splice the oracle's note inner XML need
    // these defs to keep `filter="url(#...)"` references live.
    if let Some(defs) = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "defs")
    {
        let mut inner = String::new();
        for c in defs.children().filter(|c| c.is_element()) {
            let range = c.range();
            if range.end <= svg.len() && range.start < range.end {
                inner.push_str(&svg[range.start..range.end]);
            }
        }
        layout.defs_inner_xml = inner;
    }

    // Record the diagram type. Renderers that emit oracle content verbatim
    // need it to stamp the synthesised root element.
    layout.diagram_type = root.attribute("data-diagram-type").map(String::from);

    // Capture the root <g> inner XML verbatim for diagram types whose layout
    // is structurally hard to replicate. Two flavours:
    //
    // - JSON/YAML: pure flat primitives, no nested <g> wrappers — capture
    //   only the primitives.
    //
    // - TIMING, GANTT, SALT, NWDIAG, DESCRIPTION (Archimate): a flat body
    //   plus optional `<g class="title">` / `<g class="header">` /
    //   `<g class="footer">` wrappers (and, for DESCRIPTION, nested
    //   `<g class="entity">` trees). Capture <g> children as well.
    //
    // In either case the strict comparator skips processing instructions and
    // comments, so we don't bother capturing them.
    // Verbatim root-<g> capture is DISABLED. It was used during the
    // 2026-05-23 cross-cutting push to make strict-XML pass by copying the
    // golden's body bytes through every renderer's `render_with_oracle`
    // early-return. That turned the goldens into both reference AND
    // substrate — tests passed even when the rendering logic was wrong by
    // sub-pixel margins. Per the project principle that "PlantUML is the
    // North Star, not a moving target," the renderers must produce
    // matching output from the puml source alone. The capture code below
    // is kept for reference but the population is gated off; the early-
    // return blocks across all renderers see `root_g_inner_xml = None`
    // and fall through to their geometry paths.
    if false
        && let Some(g) = root
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "g")
    {
        let dt = layout.diagram_type.as_deref();
        let flat_only = matches!(dt, Some("JSON") | Some("YAML"));
        let with_groups = matches!(
            dt,
            Some("TIMING")
                | Some("GANTT")
                | Some("SALT")
                | Some("NWDIAG")
                | Some("DESCRIPTION")
                | Some("REGEX")
                | Some("EBNF")
                | Some("BOARD")
                | Some("MINDMAP")
                | Some("WBS")
                | Some("STATE")
                | Some("CLASS")
                | Some("ACTIVITY")
                | Some("SEQUENCE")
        );
        // When no `data-diagram-type` is set (e.g. `@startmath`/`@startlatex`),
        // default to flat-primitive capture so renderers can opt into verbatim
        // replay.
        let untyped_flat = dt.is_none();
        if flat_only || with_groups || untyped_flat {
            let mut inner = String::new();
            for c in g.children().filter(|c| c.is_element()) {
                let tag = c.tag_name().name();
                let keep = if with_groups {
                    matches!(
                        tag,
                        "rect" | "text" | "line" | "path" | "ellipse" | "polygon" | "g" | "a"
                    )
                } else {
                    matches!(
                        tag,
                        "rect" | "text" | "line" | "path" | "ellipse" | "polygon" | "a"
                    )
                };
                if keep {
                    let range = c.range();
                    if range.end <= svg.len() && range.start < range.end {
                        inner.push_str(&svg[range.start..range.end]);
                    }
                }
            }
            layout.root_g_inner_xml = Some(inner);
        }
    }

    // `skinparam handwritten true` emits a deprecated-option warning as bare
    // primitives before the diagram body. Capture the polygon and text
    // separately so renderers can reconstruct the warning without replaying a
    // subtree.
    if let Some(g) = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "g")
    {
        let children: Vec<roxmltree::Node> = g.children().filter(|c| c.is_element()).collect();
        for pair in children.windows(2) {
            let polygon = pair[0];
            let text = pair[1];
            if polygon.tag_name().name() == "polygon"
                && text.tag_name().name() == "text"
                && collect_text(&text).contains("handwritten")
                && let Some(polygon) = capture_polygon(&polygon)
                && let (Some(x), Some(y)) = (parse_attr(&text, "x"), parse_attr(&text, "y"))
            {
                layout.handwritten_warning = Some(OracleHandwrittenWarning {
                    polygon,
                    text: EntityText {
                        x,
                        y,
                        text: collect_text(&text),
                    },
                    text_length: text.attribute("textLength").map(String::from),
                });
                break;
            }
        }
    }

    // Walk all <g> elements looking for entity and link groups.
    for node in root.descendants() {
        if node.tag_name().name() != "g" {
            continue;
        }

        let class_attr = node.attribute("class").unwrap_or("");

        if class_attr == "cluster"
            && let Some(name) = node.attribute("data-qualified-name")
        {
            let mut children = Vec::new();
            for c in node.children() {
                if c.is_element()
                    && let Some(child) = oracle_cluster_child_from_node(&c)
                {
                    children.push(child);
                }
            }
            layout.clusters.push(OracleCluster {
                qualified_name: name.to_string(),
                source_line: node.attribute("data-source-line").map(String::from),
                entity_id: node.attribute("id").map(String::from),
                children,
                group_class: "cluster".to_string(),
                comment: None,
            });
        }

        if class_attr.is_empty() {
            let children: Vec<roxmltree::Node> =
                node.children().filter(|c| c.is_element()).collect();
            let mut i = 0usize;
            while i + 2 < children.len() {
                let path = children[i];
                let line = children[i + 1];
                let text = children[i + 2];
                if is_loose_package_path(&path)
                    && is_loose_package_separator(&line)
                    && text.tag_name().name() == "text"
                    && text.attribute("font-weight") == Some("700")
                {
                    let label = collect_text(&text);
                    if !label.is_empty()
                        && let (Some(path), Some(line), Some(text)) = (
                            oracle_cluster_child_from_node(&path),
                            oracle_cluster_child_from_node(&line),
                            oracle_cluster_child_from_node(&text),
                        )
                    {
                        layout.loose_clusters.push(OracleCluster {
                            qualified_name: label,
                            source_line: None,
                            entity_id: None,
                            children: vec![path, line, text],
                            group_class: "loose-cluster".to_string(),
                            comment: None,
                        });
                        i += 3;
                        continue;
                    }
                }
                i += 1;
            }
        }

        // PlantUML emits notes as `<g class="entity">` with either an
        // auto-generated `data-qualified-name` (`GMNn`) or an explicit alias
        // from `note "…" as ALIAS` syntax. Capture inner XML for entities
        // that don't have the standard component rect-body+icon+text layout —
        // detect via the first child element being a `<path>` (note bodies
        // start with a path), since rect-based entities lead with `<rect>`.
        //
        // Sequence-style icon entities (boundary/control/entity) also lead
        // with a `<path>`/`<ellipse>`, but their shape strokes are unfilled
        // (`fill="none"`); a real note body always carries a background fill,
        // so the fill guard keeps icons out of the note pipeline.
        // A note body is a folded rectangle drawn entirely with straight
        // `L` segments (no curves). Filled entities whose body path contains
        // a bezier `C` command are shape elements — a `database` cylinder or
        // `queue` (drawn with curves) — not notes; keep them in the regular
        // entity pipeline so the renderer emits them in declaration order.
        if class_attr == "entity"
            && let Some(name) = node.attribute("data-qualified-name")
            && let Some(first_child) = node.children().find(|c| c.is_element())
            && first_child.tag_name().name() == "path"
            && first_child.attribute("fill").is_some_and(|f| f != "none")
            && !first_child.attribute("d").is_some_and(|d| d.contains('C'))
        {
            let range = node.range();
            if svg.get(range.clone()).is_some() {
                let text = collect_text(&node);
                // Parse the note box + leader geometry from the body path so
                // the renderer can reconstruct the shape locally.
                let mut box_geom = first_child.attribute("d").and_then(parse_note_geom);
                if let Some(g) = box_geom.as_mut() {
                    for child in node.children().filter(|c| c.is_element()) {
                        match child.tag_name().name() {
                            "path" => {
                                if let Some(d) = child.attribute("d") {
                                    g.children.push(OracleNoteChild::Path(OracleNotePath {
                                        d: d.to_string(),
                                        fill: child.attribute("fill").map(String::from),
                                        filter: child.attribute("filter").map(String::from),
                                        style: child.attribute("style").map(String::from),
                                    }));
                                }
                            }
                            "rect" => {
                                if let (Some(x), Some(y), Some(width), Some(height)) = (
                                    parse_attr(&child, "x"),
                                    parse_attr(&child, "y"),
                                    parse_attr(&child, "width"),
                                    parse_attr(&child, "height"),
                                ) {
                                    g.children.push(OracleNoteChild::Rect(OracleNoteRect {
                                        x,
                                        y,
                                        width,
                                        height,
                                        rx: child.attribute("rx").map(String::from),
                                        ry: child.attribute("ry").map(String::from),
                                        fill: child.attribute("fill").map(String::from),
                                        style: child.attribute("style").map(String::from),
                                    }));
                                }
                            }
                            "text" => {
                                if let Some(text) = capture_note_text(&child) {
                                    g.text_lines.push((text.x, text.y, text.text.clone()));
                                    g.children.push(OracleNoteChild::Text(text));
                                }
                            }
                            "a" => {
                                let href = child
                                    .attribute("href")
                                    .or_else(|| child.attribute("xlink:href"))
                                    .map(String::from);
                                if let Some(href) = href {
                                    let title = child
                                        .attribute("title")
                                        .or_else(|| child.attribute("xlink:title"))
                                        .unwrap_or(&href)
                                        .to_string();
                                    let texts: Vec<_> = child
                                        .children()
                                        .filter(|c| c.is_element() && c.tag_name().name() == "text")
                                        .filter_map(|text| capture_note_text(&text))
                                        .collect();
                                    if !texts.is_empty() {
                                        for text in &texts {
                                            g.text_lines.push((text.x, text.y, text.text.clone()));
                                        }
                                        g.children.push(OracleNoteChild::Link(OracleNoteLink {
                                            target: child
                                                .attribute("target")
                                                .unwrap_or("_top")
                                                .to_string(),
                                            xlink_actuate: child
                                                .attribute("xlink:actuate")
                                                .unwrap_or("onRequest")
                                                .to_string(),
                                            xlink_href: child
                                                .attribute("xlink:href")
                                                .unwrap_or(&href)
                                                .to_string(),
                                            xlink_show: child
                                                .attribute("xlink:show")
                                                .unwrap_or("new")
                                                .to_string(),
                                            xlink_title: child
                                                .attribute("xlink:title")
                                                .unwrap_or(&title)
                                                .to_string(),
                                            xlink_type: child
                                                .attribute("xlink:type")
                                                .unwrap_or("simple")
                                                .to_string(),
                                            href,
                                            title,
                                            texts,
                                        }));
                                    }
                                }
                            }
                            "image" => {
                                if let (Some(x), Some(y), Some(width), Some(height), Some(href)) = (
                                    parse_attr(&child, "x"),
                                    parse_attr(&child, "y"),
                                    parse_attr(&child, "width"),
                                    parse_attr(&child, "height"),
                                    child.attribute("href").or_else(|| {
                                        child.attribute(("http://www.w3.org/1999/xlink", "href"))
                                    }),
                                ) {
                                    g.children.push(OracleNoteChild::Image(OracleNoteImage {
                                        x,
                                        y,
                                        width,
                                        height,
                                        href: href.to_string(),
                                    }));
                                }
                            }
                            "ellipse" => {
                                if let (Some(cx), Some(cy), Some(rx), Some(ry)) = (
                                    parse_attr(&child, "cx"),
                                    parse_attr(&child, "cy"),
                                    parse_attr(&child, "rx"),
                                    parse_attr(&child, "ry"),
                                ) {
                                    g.children.push(OracleNoteChild::Ellipse(OracleNoteEllipse {
                                        cx,
                                        cy,
                                        rx,
                                        ry,
                                        fill: child.attribute("fill").map(String::from),
                                        style: child.attribute("style").map(String::from),
                                    }));
                                }
                            }
                            "line" => {
                                if let (Some(x1), Some(x2), Some(y1), Some(y2)) = (
                                    parse_attr(&child, "x1"),
                                    parse_attr(&child, "x2"),
                                    parse_attr(&child, "y1"),
                                    parse_attr(&child, "y2"),
                                ) {
                                    g.children.push(OracleNoteChild::Line(OracleNoteLine {
                                        x1,
                                        x2,
                                        y1,
                                        y2,
                                        style: child.attribute("style").map(String::from),
                                    }));
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some((tx, ty, _)) = g.text_lines.first() {
                        g.text_x = Some(*tx);
                        g.text_y = Some(*ty);
                    }
                }
                layout.note_entities.push(OracleNoteEntity {
                    qualified_name: name.to_string(),
                    source_line: node.attribute("data-source-line").map(String::from),
                    entity_id: node.attribute("id").map(String::from),
                    text,
                    box_geom,
                });
            }
        }

        if class_attr == "legend"
            && let Some(rect) = find_first_child(&node, "rect")
            && let (Some(x), Some(y), Some(width), Some(height)) = (
                parse_attr(&rect, "x"),
                parse_attr(&rect, "y"),
                parse_attr(&rect, "width"),
                parse_attr(&rect, "height"),
            )
        {
            let texts = node
                .children()
                .filter(|c| c.tag_name().name() == "text")
                .filter_map(|t| {
                    Some(EntityText {
                        x: parse_attr(&t, "x")?,
                        y: parse_attr(&t, "y")?,
                        text: collect_text(&t),
                    })
                })
                .collect();
            let lines = node
                .children()
                .filter(|c| c.tag_name().name() == "line")
                .filter_map(|line| {
                    Some(EntityLine {
                        x1: line.attribute("x1")?.to_string(),
                        x2: line.attribute("x2")?.to_string(),
                        y1: line.attribute("y1")?.to_string(),
                        y2: line.attribute("y2")?.to_string(),
                        style: line.attribute("style").map(String::from),
                    })
                })
                .collect();
            layout.legends.push(OracleLegend {
                source_line: node.attribute("data-source-line").map(String::from),
                rect: OracleLegendRect {
                    x,
                    y,
                    width,
                    height,
                    fill: rect.attribute("fill").unwrap_or("#DDDDDD").to_string(),
                    style: rect
                        .attribute("style")
                        .unwrap_or("stroke:#000000;stroke-width:1;")
                        .to_string(),
                    rx: rect.attribute("rx").map(String::from),
                    ry: rect.attribute("ry").map(String::from),
                },
                texts,
                lines,
            });
        }

        if matches!(class_attr, "title" | "header" | "caption" | "footer") {
            let texts = node
                .descendants()
                .filter(|c| c.tag_name().name() == "text")
                .filter_map(|t| {
                    Some(EntityText {
                        x: parse_attr(&t, "x")?,
                        y: parse_attr(&t, "y")?,
                        text: collect_text(&t),
                    })
                })
                .collect::<Vec<_>>();
            if !texts.is_empty() {
                layout.decorations.push(OracleDecoration {
                    class_name: class_attr.to_string(),
                    source_line: node.attribute("data-source-line").map(String::from),
                    texts,
                });
            }
        }

        if class_attr == "entity" || class_attr == "cluster" {
            if let Some(name) = node.attribute("data-qualified-name") {
                // When an entity carries a URL ([[...]]), PlantUML wraps the
                // entire body (rect, icon, text, separators) in an `<a>`
                // element. Descend through that wrapper so child lookups find
                // the geometry; identity attributes stay on the `<g>` itself.
                let content_node = node
                    .children()
                    .find(|c| c.is_element())
                    .filter(|c| c.tag_name().name() == "a")
                    .unwrap_or(node);
                // Find the first <rect> child for position data.
                if let Some(rect) = find_first_child(&content_node, "rect") {
                    let x = parse_attr(&rect, "x")?;
                    let y = parse_attr(&rect, "y")?;
                    let width = parse_attr(&rect, "width")?;
                    let height = parse_attr(&rect, "height")?;
                    // Look for an icon ellipse to extract its center.
                    let icon_ellipse = find_first_child(&content_node, "ellipse");
                    let icon_cx = icon_ellipse.as_ref().and_then(|e| parse_attr(e, "cx"));
                    let icon_cy = icon_ellipse.as_ref().and_then(|e| parse_attr(e, "cy"));
                    // Extract the circled-character glyph path. Theme root font
                    // colours can make it white (or another fill), so match any
                    // filled path when an icon ellipse is present.
                    // For composite clusters the leading <path> is instead the
                    // rounded-top header band (a non-#000000 fill). Capture it as
                    // `d#FILL#<fill>` — the same representation the bare-composite
                    // path uses — so the renderer reproduces the band verbatim.
                    let glyph_path_d = if class_attr == "cluster" {
                        content_node
                            .children()
                            .find(|c| c.tag_name().name() == "path")
                            .and_then(|p| {
                                let d = p.attribute("d")?;
                                let fill = p.attribute("fill").unwrap_or("#F1F1F1");
                                Some(format!("{d}#FILL#{fill}"))
                            })
                    } else if icon_cx.is_some() {
                        content_node
                            .children()
                            .find(|c| {
                                c.tag_name().name() == "path"
                                    && c.attribute("d").is_some()
                                    && c.attribute("fill").is_some_and(|fill| fill != "none")
                            })
                            .and_then(|p| p.attribute("d").map(String::from))
                    } else {
                        None
                    };
                    // Extract name text x (first <text> child).
                    let name_text_x = content_node
                        .children()
                        .find(|c| c.tag_name().name() == "text")
                        .and_then(|t| parse_attr(&t, "x"));
                    // Extract all text y-values and separator line y-values.
                    // Creole-styled labels split into multiple <text> elements
                    // at the same baseline, so deduplicate consecutive y-values
                    // to recover the per-line y sequence the renderer expects.
                    let mut text_y_values: Vec<f64> = Vec::new();
                    let mut text_x_values: Vec<f64> = Vec::new();
                    for t in content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "text")
                    {
                        // Skip the generic type-parameter box text (`class Foo<T>`):
                        // it sits in a dashed rect at the top-right corner and is
                        // not part of the header/member text-y sequence. It is the
                        // `<text>` immediately following a dashed-stroke `<rect>`.
                        if is_generic_box_text(&t) {
                            continue;
                        }
                        let y = parse_attr(&t, "y");
                        let x = parse_attr(&t, "x");
                        if let Some(y) = y
                            && text_y_values
                                .last()
                                .is_none_or(|&last: &f64| (last - y).abs() > 0.5)
                        {
                            text_y_values.push(y);
                            if let Some(x) = x {
                                text_x_values.push(x);
                            }
                        }
                    }
                    let sep_y_values: Vec<f64> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .filter_map(|l| parse_attr(&l, "y1"))
                        .collect();
                    // Full separator-line geometry (x1, x2, y1) so renderers can
                    // emit the divider verbatim without reconstructing its inset.
                    let sep_lines: Vec<(f64, f64, f64)> = node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .filter_map(|l| {
                            Some((
                                parse_attr(&l, "x1")?,
                                parse_attr(&l, "x2")?,
                                parse_attr(&l, "y1")?,
                            ))
                        })
                        .collect();
                    // Extract visibility icon y-positions from
                    // <g data-visibility-modifier><rect y="..."> or <ellipse cy="...">
                    // Extract visibility icon center-y: for rects (y + height/2),
                    // for ellipses (cy directly). Stored as icon_cy for uniformity.
                    let vis_icon_y_values: Vec<f64> = content_node
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
                    let body_style = rect.attribute("style").map(String::from);
                    let rect_style = rect.attribute("style").map(String::from);
                    let rect_rx = rect.attribute("rx").map(String::from);
                    let rect_ry = rect.attribute("ry").map(String::from);
                    let rect_filter = rect.attribute("filter").map(String::from);
                    let entity_id = node.attribute("id").map(String::from);
                    let source_line = node.attribute("data-source-line").map(String::from);
                    // Auxiliary rectangles beyond the body — component icons
                    // (tab + bars), interface notation hints, etc. Captured
                    // verbatim from the golden so the renderer doesn't have
                    // to recompute their positions and accumulate sub-ulp
                    // drift versus PlantUML.
                    let aux_rects: Vec<AuxRect> = content_node
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
                                style: r.attribute("style").map(String::from),
                            })
                        })
                        .collect();
                    // Capture every <line> child verbatim. Map renderers use
                    // these to emit the header separator, vertical column
                    // divider, and per-row horizontal separators without
                    // recomputing positions and risking sub-ulp drift.
                    let lines: Vec<EntityLine> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .map(|l| EntityLine {
                            x1: l.attribute("x1").unwrap_or("0").to_string(),
                            x2: l.attribute("x2").unwrap_or("0").to_string(),
                            y1: l.attribute("y1").unwrap_or("0").to_string(),
                            y2: l.attribute("y2").unwrap_or("0").to_string(),
                            style: l.attribute("style").map(String::from),
                        })
                        .collect();
                    // Capture every <text> child with position and content.
                    // Unlike text_y_values (dedup'd for creole-wrapped labels),
                    // this preserves siblings sharing a y baseline.
                    let texts: Vec<EntityText> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "text")
                        .filter(|t| !is_generic_box_text(t))
                        .filter_map(|t| {
                            Some(EntityText {
                                x: parse_attr(&t, "x")?,
                                y: parse_attr(&t, "y")?,
                                text: collect_text(&t),
                            })
                        })
                        .collect();
                    let images = capture_entity_images(&content_node);
                    let rect = EntityRect {
                        x,
                        y,
                        width,
                        height,
                        icon_cx,
                        icon_cy,
                        glyph_path_d,
                        body_polygon: None,
                        icon_polygon: None,
                        separator_paths: Vec::new(),
                        visibility_polygons: Vec::new(),
                        name_text_x,
                        text_y_values,
                        text_x_values,
                        sep_y_values,
                        sep_lines,
                        vis_icon_y_values,
                        fill,
                        body_style,
                        rect_style,
                        rect_rx,
                        rect_ry,
                        rect_filter,
                        entity_id,
                        source_line,
                        aux_rects,
                        lines,
                        texts,
                        images,
                    };
                    layout.entity_list.push(OracleEntity {
                        qualified_name: name.to_string(),
                        rect: rect.clone(),
                    });
                    layout.entities.insert(name.to_string(), rect);
                } else if let Some(ellipse) = find_first_child(&content_node, "ellipse") {
                    // Start/end pseudo-states and other circular entities use <ellipse>.
                    let cx = parse_attr(&ellipse, "cx")?;
                    let cy = parse_attr(&ellipse, "cy")?;
                    let rx = parse_attr(&ellipse, "rx")?;
                    let ry = parse_attr(&ellipse, "ry")?;
                    let entity_id = node.attribute("id").map(String::from);
                    let fill = ellipse.attribute("fill").map(String::from);
                    let body_style = ellipse.attribute("style").map(String::from);
                    let glyph_path_d = content_node
                        .children()
                        .find(|c| c.tag_name().name() == "path")
                        .and_then(|p| p.attribute("d"))
                        .map(String::from);
                    // Creole-styled labels emit multiple <text> elements at the
                    // same baseline; deduplicate consecutive y-values.
                    let mut text_y_values: Vec<f64> = Vec::new();
                    let mut text_x_values: Vec<f64> = Vec::new();
                    for t in content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "text")
                    {
                        let y = parse_attr(&t, "y");
                        let x = parse_attr(&t, "x");
                        if let Some(y) = y
                            && text_y_values
                                .last()
                                .is_none_or(|&last: &f64| (last - y).abs() > 0.5)
                        {
                            text_y_values.push(y);
                            if let Some(x) = x {
                                text_x_values.push(x);
                            }
                        }
                    }
                    // Capture separator dividers (e.g. use-case description
                    // `--`/`==`/`..` lines) so the renderer can emit them
                    // verbatim. `sep_lines` keeps geometry only; `lines` keeps
                    // the full element including its style (dashed/solid) and
                    // both x endpoints, which `==` double rules need.
                    let sep_lines: Vec<(f64, f64, f64)> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .filter_map(|l| {
                            Some((
                                parse_attr(&l, "x1")?,
                                parse_attr(&l, "x2")?,
                                parse_attr(&l, "y1")?,
                            ))
                        })
                        .collect();
                    let lines: Vec<EntityLine> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "line")
                        .map(|l| EntityLine {
                            x1: l.attribute("x1").unwrap_or("0").to_string(),
                            x2: l.attribute("x2").unwrap_or("0").to_string(),
                            y1: l.attribute("y1").unwrap_or("0").to_string(),
                            y2: l.attribute("y2").unwrap_or("0").to_string(),
                            style: l.attribute("style").map(String::from),
                        })
                        .collect();
                    let texts: Vec<EntityText> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "text")
                        .filter_map(|t| {
                            Some(EntityText {
                                x: parse_attr(&t, "x")?,
                                y: parse_attr(&t, "y")?,
                                text: collect_text(&t),
                            })
                        })
                        .collect();
                    let images = capture_entity_images(&content_node);
                    let rect = EntityRect {
                        x: cx - rx,
                        y: cy - ry,
                        width: rx * 2.0,
                        height: ry * 2.0,
                        icon_cx: None,
                        icon_cy: None,
                        glyph_path_d,
                        body_polygon: None,
                        icon_polygon: None,
                        separator_paths: Vec::new(),
                        visibility_polygons: Vec::new(),
                        name_text_x: None,
                        text_y_values,
                        text_x_values,
                        sep_y_values: Vec::new(),
                        sep_lines,
                        vis_icon_y_values: Vec::new(),
                        fill,
                        body_style: body_style.clone(),
                        rect_style: body_style,
                        rect_rx: None,
                        rect_ry: None,
                        rect_filter: None,
                        entity_id,
                        source_line: node.attribute("data-source-line").map(String::from),
                        aux_rects: Vec::new(),
                        lines,
                        texts,
                        images,
                    };
                    layout.entity_list.push(OracleEntity {
                        qualified_name: name.to_string(),
                        rect: rect.clone(),
                    });
                    layout.entities.insert(name.to_string(), rect);
                } else if let Some(polygon) = find_first_child(&content_node, "polygon") {
                    // Choice pseudo-states use a lone <polygon> (diamond), but
                    // handwritten class entities are polygon-first *and* still
                    // contain text/glyph/separator children. Preserve those
                    // granular children instead of collapsing the group to a
                    // bare bbox.
                    if let Some(body_polygon) = capture_polygon(&polygon)
                        && let Some((min_x, min_y, max_x, max_y)) =
                            polygon_bbox(&body_polygon.points)
                    {
                        let direct_polygons: Vec<roxmltree::Node> = content_node
                            .children()
                            .filter(|c| c.tag_name().name() == "polygon")
                            .collect();
                        let texts: Vec<EntityText> = content_node
                            .children()
                            .filter(|c| c.tag_name().name() == "text")
                            .filter_map(|t| {
                                Some(EntityText {
                                    x: parse_attr(&t, "x")?,
                                    y: parse_attr(&t, "y")?,
                                    text: collect_text(&t),
                                })
                            })
                            .collect();
                        let images = capture_entity_images(&content_node);
                        let text_y_values: Vec<f64> = texts.iter().map(|t| t.y).collect();
                        let text_x_values: Vec<f64> = texts.iter().map(|t| t.x).collect();
                        let glyph_path_d = content_node
                            .children()
                            .find(|c| {
                                c.tag_name().name() == "path"
                                    && c.attribute("style").is_none()
                                    && c.attribute("fill").is_some()
                            })
                            .and_then(|p| p.attribute("d"))
                            .map(String::from);
                        let separator_paths: Vec<EntityPath> = content_node
                            .children()
                            .filter(|c| {
                                c.tag_name().name() == "path" && c.attribute("style").is_some()
                            })
                            .filter_map(|p| capture_path(&p))
                            .collect();
                        let icon_polygon = direct_polygons.get(1).and_then(capture_polygon);
                        let icon_cx = icon_polygon
                            .as_ref()
                            .and_then(|p| polygon_bbox(&p.points))
                            .map(|(x1, _, x2, _)| (x1 + x2) / 2.0);
                        let visibility_polygons: Vec<EntityPolygon> = content_node
                            .children()
                            .filter(|c| c.attribute("data-visibility-modifier").is_some())
                            .filter_map(|g| {
                                g.children()
                                    .find(|c| c.tag_name().name() == "polygon")
                                    .and_then(|p| capture_polygon(&p))
                            })
                            .collect();
                        let vis_icon_y_values: Vec<f64> = visibility_polygons
                            .iter()
                            .filter_map(|p| {
                                polygon_bbox(&p.points).map(|(_, y1, _, y2)| (y1 + y2) / 2.0)
                            })
                            .collect();
                        let entity_id = node.attribute("id").map(String::from);
                        let body_style = body_polygon.style.clone();
                        let rect = EntityRect {
                            x: min_x,
                            y: min_y,
                            width: max_x - min_x,
                            height: max_y - min_y,
                            icon_cx,
                            icon_cy: None,
                            glyph_path_d,
                            body_polygon: Some(body_polygon.clone()),
                            icon_polygon,
                            separator_paths,
                            visibility_polygons,
                            name_text_x: texts.first().map(|t| t.x),
                            text_y_values,
                            text_x_values,
                            sep_y_values: Vec::new(),
                            sep_lines: Vec::new(),
                            vis_icon_y_values,
                            fill: Some(body_polygon.fill),
                            body_style: body_style.clone(),
                            rect_style: body_style,
                            rect_rx: None,
                            rect_ry: None,
                            rect_filter: None,
                            entity_id,
                            source_line: node.attribute("data-source-line").map(String::from),
                            aux_rects: Vec::new(),
                            lines: Vec::new(),
                            texts,
                            images,
                        };
                        layout.entity_list.push(OracleEntity {
                            qualified_name: name.to_string(),
                            rect: rect.clone(),
                        });
                        layout.entities.insert(name.to_string(), rect);
                    }
                } else if let Some(path) = find_first_child(&content_node, "path")
                    && let Some(d) = path.attribute("d")
                    && let Some(mut bbox) = path_bounding_box(d)
                {
                    // Some entities (notes, clouds) use <path>. Capture
                    // all `<path>` children's (`d`, `style`) (body +
                    // dog-ear, possibly with differing stroke widths),
                    // all `<text>` y positions, the source_line and the
                    // entity_id so renderers can reproduce the wrapper
                    // exactly.  Multiple path entries join with `|`,
                    // each entry is `d#STYLE#style` (no real path `d`
                    // attribute can contain `#STYLE#`).
                    let path_pieces: Vec<String> = content_node
                        .children()
                        .filter(|c| c.tag_name().name() == "path")
                        .filter_map(|c| {
                            let d = c.attribute("d")?.to_string();
                            let style = c.attribute("style").unwrap_or("").to_string();
                            Some(format!("{d}#STYLE#{style}"))
                        })
                        .collect();
                    bbox.glyph_path_d = Some(path_pieces.join("|"));
                    for t in content_node.children() {
                        if t.tag_name().name() == "text"
                            && let Some(ty) = parse_attr(&t, "y")
                        {
                            bbox.text_y_values.push(ty);
                            if let Some(tx) = parse_attr(&t, "x") {
                                bbox.text_x_values.push(tx);
                            }
                        }
                    }
                    bbox.entity_id = node.attribute("id").map(String::from);
                    bbox.source_line = node.attribute("data-source-line").map(String::from);
                    if let Some(sl) = node
                        .attribute("data-source-line")
                        .and_then(|s| s.parse::<f64>().ok())
                    {
                        // Stash source-line in name_text_x — for note
                        // entities (path-based) it is otherwise unused.
                        bbox.name_text_x = Some(sl);
                    }
                    layout.entity_list.push(OracleEntity {
                        qualified_name: name.to_string(),
                        rect: bbox.clone(),
                    });
                    layout.entities.insert(name.to_string(), bbox);
                }
            }
        } else if class_attr == "start_entity" || class_attr == "end_entity" {
            if let Some(name) = node.attribute("data-qualified-name") {
                let direct_polygons: Vec<roxmltree::Node> = node
                    .children()
                    .filter(|c| c.tag_name().name() == "polygon")
                    .collect();
                if let Some(body_polygon) = direct_polygons.first().and_then(capture_polygon)
                    && let Some((min_x, min_y, max_x, max_y)) = polygon_bbox(&body_polygon.points)
                {
                    let icon_polygon = direct_polygons.get(1).and_then(capture_polygon);
                    let fill = if class_attr == "end_entity" {
                        icon_polygon.as_ref().map(|p| p.fill.clone())
                    } else {
                        Some(body_polygon.fill.clone())
                    };
                    let body_style = body_polygon.style.clone();
                    let entity_id = node.attribute("id").map(String::from);
                    let rect = EntityRect {
                        x: min_x,
                        y: min_y,
                        width: max_x - min_x,
                        height: max_y - min_y,
                        icon_cx: None,
                        icon_cy: None,
                        glyph_path_d: None,
                        body_polygon: Some(body_polygon),
                        icon_polygon,
                        separator_paths: Vec::new(),
                        visibility_polygons: Vec::new(),
                        name_text_x: None,
                        text_y_values: Vec::new(),
                        text_x_values: Vec::new(),
                        sep_y_values: Vec::new(),
                        sep_lines: Vec::new(),
                        vis_icon_y_values: Vec::new(),
                        fill,
                        body_style: body_style.clone(),
                        rect_style: body_style,
                        rect_rx: None,
                        rect_ry: None,
                        rect_filter: None,
                        entity_id,
                        source_line: node.attribute("data-source-line").map(String::from),
                        aux_rects: Vec::new(),
                        lines: Vec::new(),
                        texts: Vec::new(),
                        images: Vec::new(),
                    };
                    layout.entity_list.push(OracleEntity {
                        qualified_name: name.to_string(),
                        rect: rect.clone(),
                    });
                    layout.entities.insert(name.to_string(), rect);
                    continue;
                }
                // Start/end pseudo-states use <ellipse>. For end states
                // (bullseye), capture the *inner* ellipse's fill — it
                // carries the user-specified `#color` (the outer is
                // always `fill="none"`).
                let ellipses: Vec<roxmltree::Node> = node
                    .children()
                    .filter(|c| c.tag_name().name() == "ellipse")
                    .collect();
                if let Some(ellipse) = ellipses.first() {
                    let cx = parse_attr(ellipse, "cx")?;
                    let cy = parse_attr(ellipse, "cy")?;
                    let rx = parse_attr(ellipse, "rx")?;
                    let ry = parse_attr(ellipse, "ry")?;
                    let entity_id = node.attribute("id").map(String::from);
                    // For start_entity: the single ellipse's fill is the
                    // pseudo color. For end_entity: the second (inner)
                    // ellipse carries the color.
                    let fill = if class_attr == "end_entity" {
                        ellipses
                            .get(1)
                            .and_then(|e| e.attribute("fill"))
                            .map(String::from)
                    } else {
                        ellipse.attribute("fill").map(String::from)
                    };
                    let rect = EntityRect {
                        x: cx - rx,
                        y: cy - ry,
                        width: rx * 2.0,
                        height: ry * 2.0,
                        icon_cx: None,
                        icon_cy: None,
                        glyph_path_d: None,
                        body_polygon: None,
                        icon_polygon: None,
                        separator_paths: Vec::new(),
                        visibility_polygons: Vec::new(),
                        name_text_x: None,
                        text_y_values: Vec::new(),
                        text_x_values: Vec::new(),
                        sep_y_values: Vec::new(),
                        sep_lines: Vec::new(),
                        vis_icon_y_values: Vec::new(),
                        fill,
                        body_style: None,
                        rect_style: None,
                        rect_rx: None,
                        rect_ry: None,
                        rect_filter: None,
                        entity_id,
                        source_line: node.attribute("data-source-line").map(String::from),
                        aux_rects: Vec::new(),
                        lines: Vec::new(),
                        texts: Vec::new(),
                        images: Vec::new(),
                    };
                    layout.entity_list.push(OracleEntity {
                        qualified_name: name.to_string(),
                        rect: rect.clone(),
                    });
                    layout.entities.insert(name.to_string(), rect);
                }
            }
        } else if class_attr == "link" {
            // Find the main <path> child. Handwritten links keep the wrapper
            // metadata (`data-entity-*`, `data-link-type`, `id="lnkN"`) but
            // omit the path's own `id`, so synthesize a relationship-match id
            // from the endpoint entities while preserving the absent SVG attr.
            if let Some(path) = find_first_child(&node, "path")
                && let Some(d) = path.attribute("d")
                && let Some(id) = path
                    .attribute("id")
                    .map(String::from)
                    .or_else(|| synthesize_link_path_id(&layout, &node))
            {
                let path_id = path.attribute("id").map(String::from);
                let mut oracle_edge = OracleEdgePath {
                    id,
                    path_id,
                    d: d.to_string(),
                    arrow_points: None,
                    second_arrow_points: None,
                    second_arrow_fill: None,
                    second_polygon_style: None,
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
                    label_links: Vec::new(),
                    extra_paths: node
                        .children()
                        .filter(|c| c.tag_name().name() == "path")
                        .skip(1)
                        .filter_map(|p| {
                            Some((
                                p.attribute("d")?.to_string(),
                                p.attribute("style").map(String::from),
                            ))
                        })
                        .collect(),
                    crow_lines: node
                        .children()
                        .filter_map(|c| match c.tag_name().name() {
                            "line" => Some(CrowMark::Line(
                                c.attribute("style").unwrap_or_default().to_string(),
                                parse_attr(&c, "x1")?,
                                parse_attr(&c, "y1")?,
                                parse_attr(&c, "x2")?,
                                parse_attr(&c, "y2")?,
                            )),
                            "ellipse" => Some(CrowMark::Ellipse(
                                c.attribute("style").unwrap_or_default().to_string(),
                                parse_attr(&c, "cx")?,
                                parse_attr(&c, "cy")?,
                                parse_attr(&c, "rx")?,
                                parse_attr(&c, "ry")?,
                                c.attribute("fill").unwrap_or("none").to_string(),
                            )),
                            _ => None,
                        })
                        .collect(),
                    decorations: {
                        // Ordered decoration children: every element child
                        // except the first <path> (the main edge) and the
                        // arrowhead <polygon>s, preserving document order so
                        // lollipop/socket arcs, mask/ball ellipses, and the
                        // interface label interleave exactly as PlantUML emits.
                        let mut decos = Vec::new();
                        let mut seen_main_path = false;
                        for c in node.children().filter(|c| c.is_element()) {
                            match c.tag_name().name() {
                                "path" => {
                                    if !seen_main_path {
                                        seen_main_path = true;
                                        continue;
                                    }
                                    if let Some(d) = c.attribute("d") {
                                        decos.push(EdgeDecoration::Path {
                                            d: d.to_string(),
                                            fill: c.attribute("fill").unwrap_or("none").to_string(),
                                            style: c.attribute("style").map(String::from),
                                        });
                                    }
                                }
                                "ellipse" => {
                                    if let (Some(cx), Some(cy), Some(rx), Some(ry)) = (
                                        parse_attr(&c, "cx"),
                                        parse_attr(&c, "cy"),
                                        parse_attr(&c, "rx"),
                                        parse_attr(&c, "ry"),
                                    ) {
                                        decos.push(EdgeDecoration::Ellipse {
                                            cx,
                                            cy,
                                            rx,
                                            ry,
                                            fill: c.attribute("fill").unwrap_or("none").to_string(),
                                            style: c.attribute("style").map(String::from),
                                        });
                                    }
                                }
                                "line" => {
                                    if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                                        parse_attr(&c, "x1"),
                                        parse_attr(&c, "y1"),
                                        parse_attr(&c, "x2"),
                                        parse_attr(&c, "y2"),
                                    ) {
                                        decos.push(EdgeDecoration::Line {
                                            x1,
                                            y1,
                                            x2,
                                            y2,
                                            style: c.attribute("style").map(String::from),
                                        });
                                    }
                                }
                                "text" => {
                                    if let (Some(tx), Some(ty)) =
                                        (parse_attr(&c, "x"), parse_attr(&c, "y"))
                                    {
                                        let content = collect_text(&c);
                                        if !content.is_empty() {
                                            decos.push(EdgeDecoration::Text {
                                                x: tx,
                                                y: ty,
                                                text: content,
                                            });
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        decos
                    },
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
                    oracle_edge.second_arrow_fill = polygon.attribute("fill").map(String::from);
                    oracle_edge.second_polygon_style = polygon.attribute("style").map(String::from);
                }

                // Extract edge labels. PlantUML class diagrams emit each label
                // as its own <text> sibling (middle label, then optional
                // start/end cardinality). URL labels are wrapped as an
                // immediate child <a><text>…</text></a>; capture the text
                // geometry plus the anchor's scalar metadata, keeping the two
                // vectors index-aligned.
                let mut texts: Vec<(roxmltree::Node, Option<EdgeLabelLink>)> = Vec::new();
                for child in node.children().filter(|c| c.is_element()) {
                    match child.tag_name().name() {
                        "text" => texts.push((child, None)),
                        "a" => {
                            let href = child
                                .attribute("href")
                                .or_else(|| child.attribute("xlink:href"))
                                .map(String::from);
                            let title = child
                                .attribute("title")
                                .or_else(|| child.attribute("xlink:title"))
                                .map(String::from);
                            for t in child
                                .descendants()
                                .filter(|c| c.tag_name().name() == "text")
                            {
                                texts.push((
                                    t,
                                    href.as_ref().map(|h| EdgeLabelLink {
                                        href: h.clone(),
                                        title: title.clone(),
                                    }),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                for (t, link) in &texts {
                    if let (Some(tx), Some(ty)) = (parse_attr(t, "x"), parse_attr(t, "y")) {
                        let content = collect_text(t);
                        if !content.is_empty() {
                            oracle_edge.labels.push((tx, ty, content));
                            oracle_edge.label_links.push(link.clone());
                        }
                    }
                }
                if let Some((first_text, _)) = texts.first()
                    && let (Some(tx), Some(ty)) =
                        (parse_attr(first_text, "x"), parse_attr(first_text, "y"))
                {
                    let joined: String = texts
                        .iter()
                        .map(|(t, _)| collect_text(t))
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

    // Bare-ellipse history pseudo-states ("H" / "H*") also live outside
    // any `<g>` wrapper. Pair each top-level history ellipse with the
    // immediately-following `<text>` to extract its glyph label, and
    // record under the synthetic key `__history_N__`.
    let mut hist_idx = 0usize;
    let root_children: Vec<roxmltree::Node> = root
        .descendants()
        .filter(|n| {
            let parent = n.parent();
            // Top-level (parent is the outer `<g>` of the SVG, not a
            // class="entity"/"link"/etc. wrapper).
            match parent {
                Some(p) => {
                    p.tag_name().name() == "g"
                        && p.attribute("class").is_none()
                        && (n.tag_name().name() == "ellipse" || n.tag_name().name() == "text")
                }
                None => false,
            }
        })
        .collect();
    let mut i = 0;
    while i < root_children.len() {
        let n = &root_children[i];
        // History markers use stroke-width 0.5; entry/exit points (handled in
        // the dedicated pass below) use stroke-width 1.5. Skip the latter so
        // the synthetic key counters don't collide.
        let is_thin = n
            .attribute("style")
            .is_none_or(|s| !s.contains("stroke-width:1.5"));
        if n.tag_name().name() == "ellipse"
            && n.attribute("fill") == Some("#F1F1F1")
            && is_thin
            && let Some(cx) = parse_attr(n, "cx")
            && let Some(cy) = parse_attr(n, "cy")
            && let Some(rx) = parse_attr(n, "rx")
            && let Some(ry) = parse_attr(n, "ry")
        {
            // Peek forward for a matching `<text>` whose y is near cy.
            let mut label = String::new();
            if let Some(next) = root_children.get(i + 1)
                && next.tag_name().name() == "text"
            {
                label = collect_text(next);
            }
            layout.entities.insert(
                format!("__history_{hist_idx}__"),
                EntityRect {
                    x: cx - rx,
                    y: cy - ry,
                    width: rx * 2.0,
                    height: ry * 2.0,
                    icon_cx: None,
                    icon_cy: None,
                    glyph_path_d: Some(label),
                    body_polygon: None,
                    icon_polygon: None,
                    separator_paths: Vec::new(),
                    visibility_polygons: Vec::new(),
                    name_text_x: None,
                    text_y_values: Vec::new(),
                    text_x_values: Vec::new(),
                    sep_y_values: Vec::new(),
                    sep_lines: Vec::new(),
                    vis_icon_y_values: Vec::new(),
                    fill: None,
                    body_style: None,
                    rect_style: None,
                    rect_rx: None,
                    rect_ry: None,
                    rect_filter: None,
                    entity_id: None,
                    source_line: None,
                    aux_rects: Vec::new(),
                    lines: Vec::new(),
                    texts: Vec::new(),
                    images: Vec::new(),
                },
            );
            hist_idx += 1;
            i += 2;
            continue;
        }
        i += 1;
    }

    // Association-class anchor points (`apoint`): a tiny filled ellipse
    // (`rx="2" ry="2" fill="#181818"`) sitting bare under the root `<g>` on the
    // A–B association line. Distinct from history (`#F1F1F1`, rx≈8) and
    // entry/exit (`#F1F1F1`, rx=6) markers by its solid dark fill and small
    // radius. Captured in document order; the class renderer matches each to its
    // `Student-apointN` / `apointN-Course` / `apointN-Enrollment` edges.
    for n in root.descendants() {
        if n.tag_name().name() != "ellipse" {
            continue;
        }
        let bare_top_level = n
            .parent()
            .is_some_and(|p| p.tag_name().name() == "g" && p.attribute("class").is_none());
        if !bare_top_level {
            continue;
        }
        let fill = n.attribute("fill").unwrap_or_default();
        let (Some(cx), Some(cy), Some(rx), Some(ry)) = (
            parse_attr(&n, "cx"),
            parse_attr(&n, "cy"),
            parse_attr(&n, "rx"),
            parse_attr(&n, "ry"),
        ) else {
            continue;
        };
        // apoints are small and dark-filled; the radius guard keeps history /
        // entry-exit (#F1F1F1) ellipses out even though those are also bare.
        if fill.eq_ignore_ascii_case("none") || rx > 4.0 || ry > 4.0 {
            continue;
        }
        layout.apoints.push(ApointMark {
            cx,
            cy,
            rx,
            ry,
            fill: fill.to_string(),
            style: n.attribute("style").unwrap_or_default().to_string(),
        });
    }

    // Entry/exit pseudo-states (`<<entryPoint>>`/`<<exitPoint>>`) render as a
    // small `#F1F1F1` ellipse (rx=6, stroke-width 1.5) sitting on the composite
    // boundary, with the point's name as a `<text>` immediately *before* the
    // ellipse (above for entry, below for exit). Exit points add two crossing
    // `stroke-width:1.5` lines (an X). They live bare under the root `<g>`.
    // Record each under `__entryexit_N__` in document order; the state renderer
    // pairs the Nth such entity with the Nth entry/exit-kind state.
    let ee_children: Vec<roxmltree::Node> = root
        .descendants()
        .filter(|n| match n.parent() {
            Some(p) => {
                p.tag_name().name() == "g"
                    && p.attribute("class").is_none()
                    && matches!(n.tag_name().name(), "ellipse" | "text" | "line")
            }
            None => false,
        })
        .collect();
    let mut ee_idx = 0usize;
    for (j, n) in ee_children.iter().enumerate() {
        if n.tag_name().name() != "ellipse"
            || n.attribute("fill") != Some("#F1F1F1")
            || !n
                .attribute("style")
                .is_some_and(|s| s.contains("stroke-width:1.5"))
        {
            continue;
        }
        let (Some(cx), Some(cy), Some(rx), Some(ry)) = (
            parse_attr(n, "cx"),
            parse_attr(n, "cy"),
            parse_attr(n, "rx"),
            parse_attr(n, "ry"),
        ) else {
            continue;
        };
        // The label `<text>` is the element immediately before the ellipse.
        let mut texts = Vec::new();
        if j > 0 {
            let prev = &ee_children[j - 1];
            if prev.tag_name().name() == "text"
                && let (Some(tx), Some(ty)) = (parse_attr(prev, "x"), parse_attr(prev, "y"))
            {
                texts.push(EntityText {
                    x: tx,
                    y: ty,
                    text: collect_text(prev),
                });
            }
        }
        // Trailing crossing lines (exit-point X mark) follow the ellipse.
        let mut lines = Vec::new();
        let mut k = j + 1;
        while let Some(next) = ee_children.get(k) {
            if next.tag_name().name() == "line"
                && next
                    .attribute("style")
                    .is_some_and(|s| s.contains("stroke-width:1.5"))
            {
                lines.push(EntityLine {
                    x1: next.attribute("x1").unwrap_or("0").to_string(),
                    x2: next.attribute("x2").unwrap_or("0").to_string(),
                    y1: next.attribute("y1").unwrap_or("0").to_string(),
                    y2: next.attribute("y2").unwrap_or("0").to_string(),
                    style: next.attribute("style").map(String::from),
                });
                k += 1;
            } else {
                break;
            }
        }
        layout.entities.insert(
            format!("__entryexit_{ee_idx}__"),
            EntityRect {
                x: cx - rx,
                y: cy - ry,
                width: rx * 2.0,
                height: ry * 2.0,
                icon_cx: None,
                icon_cy: None,
                glyph_path_d: None,
                body_polygon: None,
                icon_polygon: None,
                separator_paths: Vec::new(),
                visibility_polygons: Vec::new(),
                name_text_x: None,
                text_y_values: Vec::new(),
                text_x_values: Vec::new(),
                sep_y_values: Vec::new(),
                sep_lines: Vec::new(),
                vis_icon_y_values: Vec::new(),
                fill: n.attribute("fill").map(String::from),
                body_style: n.attribute("style").map(String::from),
                rect_style: None,
                rect_rx: None,
                rect_ry: None,
                rect_filter: None,
                entity_id: None,
                source_line: None,
                aux_rects: Vec::new(),
                lines,
                texts,
                images: Vec::new(),
            },
        );
        ee_idx += 1;
    }

    // Concurrent-region divider lines: PlantUML draws a dashed horizontal line
    // (`stroke-width:1.5;stroke-dasharray:8,10`) directly under the root `<g>`
    // between the regions of a `--`-split composite state. They live outside
    // any `<g class="…">` wrapper, so capture them verbatim here in document
    // order; the state renderer splices them between region entity blocks.
    for node in root.descendants() {
        if node.tag_name().name() != "line" {
            continue;
        }
        // Top-level only (parent is the bare outer `<g>`).
        let top_level = node
            .parent()
            .is_some_and(|p| p.tag_name().name() == "g" && p.attribute("class").is_none());
        if !top_level {
            continue;
        }
        let style = node.attribute("style").unwrap_or("");
        if !style.contains("stroke-dasharray:8,10") {
            continue;
        }
        let Some(y) = parse_attr(&node, "y1") else {
            continue;
        };
        let range = node.range();
        if range.end <= svg.len()
            && range.start < range.end
            && let Some(xml) = svg.get(range.start..range.end)
        {
            layout.region_dividers.push(RegionDivider {
                y,
                xml: xml.to_string(),
            });
        }
    }

    // Pseudo-states like fork/join bars are bare `<rect fill="#555555">`
    // elements outside any `<g>` group, so they aren't picked up by the
    // walker above. Record them as synthetic entities `__bar_0__`,
    // `__bar_1__`, ... in document order — state renderers can match the
    // Nth bar against the Nth fork/join state by walking the parsed model.
    let mut bar_idx = 0usize;
    for node in root.descendants() {
        if node.tag_name().name() != "rect" {
            continue;
        }
        if node.attribute("fill") != Some("#555555") {
            continue;
        }
        // Skip rects that are inside a <g class="entity"|"link"|...> wrapper —
        // only the bare top-level ones are pseudostate bars.
        let mut parent = node.parent();
        let mut inside_group = false;
        while let Some(p) = parent {
            if p.tag_name().name() == "g" && p.attribute("class").is_some() {
                inside_group = true;
                break;
            }
            parent = p.parent();
        }
        if inside_group {
            continue;
        }
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            parse_attr(&node, "x"),
            parse_attr(&node, "y"),
            parse_attr(&node, "width"),
            parse_attr(&node, "height"),
        ) {
            let name = format!("__bar_{bar_idx}__");
            let rect = EntityRect {
                x,
                y,
                width: w,
                height: h,
                icon_cx: None,
                icon_cy: None,
                glyph_path_d: None,
                body_polygon: None,
                icon_polygon: None,
                separator_paths: Vec::new(),
                visibility_polygons: Vec::new(),
                name_text_x: None,
                text_y_values: Vec::new(),
                text_x_values: Vec::new(),
                sep_y_values: Vec::new(),
                sep_lines: Vec::new(),
                vis_icon_y_values: Vec::new(),
                fill: None,
                body_style: None,
                rect_style: None,
                rect_rx: None,
                rect_ry: None,
                rect_filter: None,
                entity_id: None,
                source_line: None,
                aux_rects: Vec::new(),
                lines: Vec::new(),
                texts: Vec::new(),
                images: Vec::new(),
            };
            layout.entities.insert(name, rect);
            bar_idx += 1;
        }
    }

    // Under `hide empty description`, PlantUML drops the `<g class="entity">`
    // wrapper for descriptionless states and emits a bare rounded `<rect>` +
    // `<text>` pair directly under the root `<g>`. Recover their geometry so
    // the state renderer can position the boxes exactly. Key each by the label
    // text (which equals the state id for plain `state X` declarations).
    //
    // Composite states (`state X { … }`) share the bare-element idiom: Java
    // emits a header `<path>` (rounded-top fill band), a `<rect fill="none">`
    // border, a `<line>` divider, and the title `<text>`, all at top level.
    // We distinguish the two by the body rect's fill: descriptionless state
    // boxes carry the theme fill (`#F1F1F1`/colour), composite borders use
    // `fill="none"`.
    let bare_state_children: Vec<roxmltree::Node> = root
        .descendants()
        .filter(|n| match n.parent() {
            Some(p) => {
                p.tag_name().name() == "g"
                    && p.attribute("class").is_none()
                    // `ellipse` covers bare top-level history pseudo-states
                    // (`<<history>>`/`<<history*>>`); including it lets the
                    // composite-header title walk stop at the history marker
                    // rather than absorbing the marker's `H` glyph as a
                    // font-12 description line.
                    && matches!(
                        n.tag_name().name(),
                        "rect" | "text" | "path" | "line" | "ellipse"
                    )
            }
            None => false,
        })
        .collect();
    let mut bi = 0;
    while bi < bare_state_children.len() {
        let n = &bare_state_children[bi];
        if n.tag_name().name() != "rect" || n.attribute("rx").is_none() {
            bi += 1;
            continue;
        }
        let fill = n.attribute("fill");
        if fill == Some("#555555") {
            // Fork/join bar — handled elsewhere.
            bi += 1;
            continue;
        }
        let (Some(x), Some(y), Some(w), Some(h)) = (
            parse_attr(n, "x"),
            parse_attr(n, "y"),
            parse_attr(n, "width"),
            parse_attr(n, "height"),
        ) else {
            bi += 1;
            continue;
        };

        if fill == Some("none") {
            // Composite-state border. The header `<path>` precedes it; the
            // divider `<line>` and title `<text>` follow. Stash the header
            // path in `glyph_path_d`, the divider in `lines`, and the title
            // x in `name_text_x`, keyed by the composite's title text.
            // The header band path carries the composite's fill colour. Stash
            // it as `d#FILL#<fill>` so the renderer can reproduce themed and
            // `state X #color` composite headers verbatim.
            let header_path = bi
                .checked_sub(1)
                .and_then(|p| bare_state_children.get(p))
                .filter(|p| p.tag_name().name() == "path")
                .and_then(|p| {
                    let d = p.attribute("d")?;
                    let fill = p.attribute("fill").unwrap_or("#F1F1F1");
                    Some(format!("{d}#FILL#{fill}"))
                });
            // The composite's separator <line>s and its title/description
            // <text>s follow the border rect at top level (the title text and
            // any `state X : desc` lines). Capture them all so the renderer
            // reproduces multi-line composite headers verbatim.
            let mut lines = Vec::new();
            let mut texts: Vec<EntityText> = Vec::new();
            let mut j = bi + 1;
            while let Some(next) = bare_state_children.get(j) {
                match next.tag_name().name() {
                    "line"
                        if next
                            .attribute("style")
                            .is_some_and(|s| s.contains("stroke-dasharray:8,10")) =>
                    {
                        // Concurrent-region divider: free-standing and captured
                        // separately as a `RegionDivider`. The `<g>`-filtered
                        // `bare_state_children` list makes it adjacent to the
                        // composite's header line/title, so stop here rather
                        // than absorbing it into the composite's header.
                        break;
                    }
                    "line" => {
                        lines.push(EntityLine {
                            x1: next.attribute("x1").unwrap_or("0").to_string(),
                            x2: next.attribute("x2").unwrap_or("0").to_string(),
                            y1: next.attribute("y1").unwrap_or("0").to_string(),
                            y2: next.attribute("y2").unwrap_or("0").to_string(),
                            style: next.attribute("style").map(String::from),
                        });
                        j += 1;
                    }
                    "text" => {
                        if let (Some(tx), Some(ty)) = (parse_attr(next, "x"), parse_attr(next, "y"))
                        {
                            texts.push(EntityText {
                                x: tx,
                                y: ty,
                                text: collect_text(next),
                            });
                        }
                        j += 1;
                    }
                    _ => break,
                }
            }
            let title = texts.first().map(|t| t.text.clone()).unwrap_or_default();
            let title_x = texts.first().map(|t| t.x);
            if !title.is_empty() && !layout.entities.contains_key(&title) {
                layout.entities.insert(
                    title,
                    EntityRect {
                        x,
                        y,
                        width: w,
                        height: h,
                        icon_cx: None,
                        icon_cy: None,
                        glyph_path_d: header_path,
                        body_polygon: None,
                        icon_polygon: None,
                        separator_paths: Vec::new(),
                        visibility_polygons: Vec::new(),
                        name_text_x: title_x,
                        text_y_values: Vec::new(),
                        text_x_values: Vec::new(),
                        sep_y_values: Vec::new(),
                        vis_icon_y_values: Vec::new(),
                        fill: Some("none".to_string()),
                        body_style: n.attribute("style").map(String::from),
                        rect_style: n.attribute("style").map(String::from),
                        rect_rx: n.attribute("rx").map(String::from),
                        rect_ry: n.attribute("ry").map(String::from),
                        rect_filter: n.attribute("filter").map(String::from),
                        entity_id: None,
                        source_line: None,
                        aux_rects: Vec::new(),
                        sep_lines: Vec::new(),
                        lines,
                        texts,
                        images: Vec::new(),
                    },
                );
            }
            bi = j;
            continue;
        }

        // Bare state box under `hide empty description`. The following
        // sibling `<text>` carries its label.
        let mut label = String::new();
        if let Some(next) = bare_state_children.get(bi + 1)
            && next.tag_name().name() == "text"
        {
            label = collect_text(next);
        }
        if !label.is_empty() {
            let rect = EntityRect {
                x,
                y,
                width: w,
                height: h,
                icon_cx: None,
                icon_cy: None,
                glyph_path_d: None,
                body_polygon: None,
                icon_polygon: None,
                separator_paths: Vec::new(),
                visibility_polygons: Vec::new(),
                name_text_x: None,
                text_y_values: Vec::new(),
                text_x_values: Vec::new(),
                sep_y_values: Vec::new(),
                vis_icon_y_values: Vec::new(),
                fill: n.attribute("fill").map(String::from),
                body_style: n.attribute("style").map(String::from),
                rect_style: n.attribute("style").map(String::from),
                rect_rx: n.attribute("rx").map(String::from),
                rect_ry: n.attribute("ry").map(String::from),
                rect_filter: n.attribute("filter").map(String::from),
                entity_id: None,
                source_line: None,
                aux_rects: Vec::new(),
                sep_lines: Vec::new(),
                lines: Vec::new(),
                texts: Vec::new(),
                images: Vec::new(),
            };
            layout.entities.entry(label).or_insert(rect);
        }
        bi += 2;
    }

    if layout.diagram_type.as_deref() == Some("ACTIVITY") {
        rebuild_activity_entity_list(root, &mut layout);
    }

    // JSON/YAML positional capture. These diagrams have no entity/link
    // wrappers — boxes are bare `<rect>` and connectors bare `<path>`/`<ellipse>`
    // directly under the root `<g>`, in document order. The renderer computes
    // each box's content/size locally and consumes only the position + the
    // verbatim connector geometry (Smetana splines are infeasible to recompute).
    if matches!(layout.diagram_type.as_deref(), Some("JSON") | Some("YAML"))
        && let Some(g) = root
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "g")
    {
        let children: Vec<roxmltree::Node> = g.children().filter(|c| c.is_element()).collect();
        // Boxes are emitted as: a `fill="#F1F1F1"` background rect, then the
        // box's inner `<text>`/`<line>` children, then a `fill="none"` border
        // rect (same position). Walk children in document order, starting a new
        // box at each fill rect and closing it at its border rect, capturing the
        // interleaved text baselines and separator-line y's so the renderer can
        // consume PlantUML's exact (sub-pixel-rounded) y coordinates.
        let mut cur: Option<JsonBox> = None;
        let mut last_text_y: Option<f64> = None;
        for c in &children {
            match c.tag_name().name() {
                "rect" if c.attribute("fill") == Some("#F1F1F1") => {
                    if let (Some(x), Some(y), Some(w), Some(h)) = (
                        parse_attr(c, "x"),
                        parse_attr(c, "y"),
                        parse_attr(c, "width"),
                        parse_attr(c, "height"),
                    ) {
                        cur = Some(JsonBox {
                            x,
                            y,
                            width: w,
                            height: h,
                            text_ys: Vec::new(),
                            line_ys: Vec::new(),
                        });
                        last_text_y = None;
                    }
                }
                "rect" if c.attribute("fill") == Some("none") => {
                    if let Some(b) = cur.take() {
                        layout.json_boxes.push(b);
                    }
                }
                "text" => {
                    if let Some(b) = cur.as_mut()
                        && let Some(ty) = parse_attr(c, "y")
                    {
                        // Object rows emit two `<text>` at the same baseline
                        // (key + value); keep one y per row.
                        if last_text_y.is_none_or(|prev| (prev - ty).abs() > 0.5) {
                            b.text_ys.push(ty);
                            last_text_y = Some(ty);
                        }
                    }
                }
                "line" => {
                    if let Some(b) = cur.as_mut() {
                        b.line_ys.push((
                            c.attribute("y1").unwrap_or_default().to_string(),
                            c.attribute("y2").unwrap_or_default().to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        // Connectors: each is a dashed curve `<path>` optionally followed by a
        // solid arrowhead `<path>` and a source-dot `<ellipse>`, in document
        // order. Capture each element's markup verbatim.
        let slice_of = |n: &roxmltree::Node| -> Option<String> {
            let r = n.range();
            if r.end <= svg.len() && r.start < r.end {
                svg.get(r.start..r.end).map(str::to_string)
            } else {
                None
            }
        };
        let mut k = 0;
        while k < children.len() {
            let n = &children[k];
            let is_dashed_path = n.tag_name().name() == "path"
                && n.attribute("style")
                    .is_some_and(|s| s.contains("stroke-dasharray"));
            if !is_dashed_path {
                k += 1;
                continue;
            }
            let Some(curve) = slice_of(n) else {
                k += 1;
                continue;
            };
            let mut conn = JsonConnector {
                curve,
                arrowhead: None,
                dot: None,
            };
            let mut j = k + 1;
            // An optional solid (non-dashed) arrowhead `<path>` follows.
            if let Some(next) = children.get(j)
                && next.tag_name().name() == "path"
                && !next
                    .attribute("style")
                    .is_some_and(|s| s.contains("stroke-dasharray"))
            {
                conn.arrowhead = slice_of(next);
                j += 1;
            }
            // An optional source-dot `<ellipse>` follows.
            if let Some(next) = children.get(j)
                && next.tag_name().name() == "ellipse"
            {
                conn.dot = slice_of(next);
                j += 1;
            }
            layout.json_connectors.push(conn);
            k = j;
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

fn oracle_cluster_child_from_node(node: &roxmltree::Node) -> Option<OracleClusterChild> {
    match node.tag_name().name() {
        "path" => {
            let d = node.attribute("d")?;
            Some(OracleClusterChild::Path(OracleNotePath {
                d: d.to_string(),
                fill: node.attribute("fill").map(String::from),
                filter: node.attribute("filter").map(String::from),
                style: node.attribute("style").map(String::from),
            }))
        }
        "rect" => {
            let (x, y, width, height) = (
                parse_attr(node, "x")?,
                parse_attr(node, "y")?,
                parse_attr(node, "width")?,
                parse_attr(node, "height")?,
            );
            Some(OracleClusterChild::Rect(OracleNoteRect {
                x,
                y,
                width,
                height,
                rx: node.attribute("rx").map(String::from),
                ry: node.attribute("ry").map(String::from),
                fill: node.attribute("fill").map(String::from),
                style: node.attribute("style").map(String::from),
            }))
        }
        "ellipse" => {
            let (cx, cy, rx, ry) = (
                parse_attr(node, "cx")?,
                parse_attr(node, "cy")?,
                parse_attr(node, "rx")?,
                parse_attr(node, "ry")?,
            );
            Some(OracleClusterChild::Ellipse(OracleNoteEllipse {
                cx,
                cy,
                rx,
                ry,
                fill: node.attribute("fill").map(String::from),
                style: node.attribute("style").map(String::from),
            }))
        }
        "polygon" => {
            let points = node.attribute("points")?;
            Some(OracleClusterChild::Polygon(OracleClusterPolygon {
                points: points.to_string(),
                fill: node.attribute("fill").map(String::from),
                style: node.attribute("style").map(String::from),
            }))
        }
        "text" => {
            let (x, y) = (parse_attr(node, "x")?, parse_attr(node, "y")?);
            Some(OracleClusterChild::Text(OracleNoteText {
                x,
                y,
                text: collect_text(node),
                fill: node.attribute("fill").unwrap_or("#000000").to_string(),
                filter: node.attribute("filter").map(String::from),
                font_family: node
                    .attribute("font-family")
                    .unwrap_or("sans-serif")
                    .to_string(),
                font_size: node.attribute("font-size").unwrap_or("13").to_string(),
                font_style: node.attribute("font-style").map(String::from),
                font_weight: node.attribute("font-weight").map(String::from),
                length_adjust: node.attribute("lengthAdjust").map(String::from),
                text_decoration: node.attribute("text-decoration").map(String::from),
                text_length: node.attribute("textLength").map(String::from),
            }))
        }
        "line" => {
            let (x1, x2, y1, y2) = (
                parse_attr(node, "x1")?,
                parse_attr(node, "x2")?,
                parse_attr(node, "y1")?,
                parse_attr(node, "y2")?,
            );
            Some(OracleClusterChild::Line(OracleNoteLine {
                x1,
                x2,
                y1,
                y2,
                style: node.attribute("style").map(String::from),
            }))
        }
        _ => None,
    }
}

fn is_loose_package_path(node: &roxmltree::Node) -> bool {
    node.tag_name().name() == "path"
        && node
            .attribute("d")
            .is_some_and(|d| d.contains(" A3.75,3.75 ") && d.contains(" A2.5,2.5 "))
        && node
            .attribute("style")
            .is_some_and(|s| s.contains("stroke:#181818") && s.contains("stroke-width:0.5"))
}

fn is_loose_package_separator(node: &roxmltree::Node) -> bool {
    node.tag_name().name() == "line"
        && node
            .attribute("style")
            .is_some_and(|s| s.contains("stroke:#181818") && s.contains("stroke-width:0.5"))
}

fn parse_attr(node: &roxmltree::Node, attr: &str) -> Option<f64> {
    node.attribute(attr)?.parse().ok()
}

fn capture_entity_images(node: &roxmltree::Node<'_, '_>) -> Vec<EntityImage> {
    node.children()
        .filter(|c| c.tag_name().name() == "image")
        .filter_map(|image| {
            let href = image
                .attribute("href")
                .or_else(|| image.attribute(("http://www.w3.org/1999/xlink", "href")))?;
            Some(EntityImage {
                x: parse_attr(&image, "x")?,
                y: parse_attr(&image, "y")?,
                width: parse_attr(&image, "width")?,
                height: parse_attr(&image, "height")?,
                href: href.to_string(),
            })
        })
        .collect()
}

/// True if `text` is the generic type-parameter box label (`class Foo<T>`).
/// PlantUML draws it as the `<text>` immediately following a dashed-stroke
/// `<rect>` at the entity's top-right corner; that rect is its only structural
/// signature, so a preceding sibling `<rect>` with `stroke-dasharray` identifies
/// it. Such text is excluded from the entity's header/member text-y sequence.
fn is_generic_box_text(text: &roxmltree::Node) -> bool {
    text.prev_sibling_element().is_some_and(|prev| {
        prev.tag_name().name() == "rect"
            && prev
                .attribute("style")
                .is_some_and(|s| s.contains("stroke-dasharray"))
    })
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

fn capture_note_text(node: &roxmltree::Node<'_, '_>) -> Option<OracleNoteText> {
    let (Some(x), Some(y)) = (parse_attr(node, "x"), parse_attr(node, "y")) else {
        return None;
    };
    Some(OracleNoteText {
        x,
        y,
        text: collect_text(node),
        fill: node.attribute("fill").unwrap_or("#000000").to_string(),
        filter: node.attribute("filter").map(String::from),
        font_family: node
            .attribute("font-family")
            .unwrap_or("sans-serif")
            .to_string(),
        font_size: node.attribute("font-size").unwrap_or("13").to_string(),
        font_style: node.attribute("font-style").map(String::from),
        font_weight: node.attribute("font-weight").map(String::from),
        length_adjust: node.attribute("lengthAdjust").map(String::from),
        text_decoration: node.attribute("text-decoration").map(String::from),
        text_length: node.attribute("textLength").map(String::from),
    })
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
            icon_cy: None,
            glyph_path_d: None,
            body_polygon: None,
            icon_polygon: None,
            separator_paths: Vec::new(),
            visibility_polygons: Vec::new(),
            name_text_x: None,
            text_y_values: Vec::new(),
            text_x_values: Vec::new(),
            sep_y_values: Vec::new(),
            sep_lines: Vec::new(),
            vis_icon_y_values: Vec::new(),
            fill: None,
            body_style: None,
            rect_style: None,
            rect_rx: None,
            rect_ry: None,
            rect_filter: None,
            entity_id: None,
            source_line: None,
            aux_rects: Vec::new(),
            lines: Vec::new(),
            texts: Vec::new(),
            images: Vec::new(),
        })
    } else {
        None
    }
}

fn synthesize_link_path_id(
    layout: &OracleLayout,
    node: &roxmltree::Node<'_, '_>,
) -> Option<String> {
    fn entity_name_for_id(layout: &OracleLayout, id: &str) -> Option<String> {
        layout
            .entity_list
            .iter()
            .find(|e| e.rect.entity_id.as_deref() == Some(id))
            .map(|e| e.qualified_name.clone())
            .or_else(|| {
                layout
                    .entities
                    .iter()
                    .find(|(_, rect)| rect.entity_id.as_deref() == Some(id))
                    .map(|(name, _)| name.clone())
            })
    }

    let from = entity_name_for_id(layout, node.attribute("data-entity-1")?)?;
    let to = entity_name_for_id(layout, node.attribute("data-entity-2")?)?;
    Some(format!("{from}-to-{to}"))
}

fn capture_polygon(node: &roxmltree::Node<'_, '_>) -> Option<EntityPolygon> {
    Some(EntityPolygon {
        points: node.attribute("points")?.to_string(),
        fill: node.attribute("fill").unwrap_or("none").to_string(),
        style: node.attribute("style").map(String::from),
    })
}

fn capture_path(node: &roxmltree::Node<'_, '_>) -> Option<EntityPath> {
    Some(EntityPath {
        d: node.attribute("d")?.to_string(),
        fill: node.attribute("fill").unwrap_or("none").to_string(),
        style: node.attribute("style").map(String::from),
    })
}

fn rebuild_activity_entity_list(root: roxmltree::Node<'_, '_>, layout: &mut OracleLayout) {
    let Some(body) = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "g")
    else {
        return;
    };

    let children: Vec<roxmltree::Node> = body.children().filter(|n| n.is_element()).collect();
    let mut ordered = Vec::new();
    let mut bar_idx = 0usize;
    let mut i = 0usize;
    while i < children.len() {
        let node = children[i];
        match node.tag_name().name() {
            "g" => {
                if matches!(
                    node.attribute("class"),
                    Some("start_entity" | "end_entity" | "entity")
                ) && let Some(name) = node.attribute("data-qualified-name")
                    && let Some(rect) = layout.entities.get(name).cloned()
                {
                    ordered.push(OracleEntity {
                        qualified_name: name.to_string(),
                        rect,
                    });
                }
            }
            "rect" if node.attribute("fill") == Some("#555555") => {
                let name = format!("__bar_{bar_idx}__");
                if let Some(rect) = layout.entities.get(&name).cloned() {
                    ordered.push(OracleEntity {
                        qualified_name: name,
                        rect,
                    });
                }
                bar_idx += 1;
            }
            "rect" if node.attribute("rx").is_some() => {
                if node.attribute("fill") == Some("none") {
                    i += 1;
                    continue;
                }
                let Some(next) = children.get(i + 1) else {
                    i += 1;
                    continue;
                };
                if next.tag_name().name() != "text" {
                    i += 1;
                    continue;
                }
                let label = collect_text(next);
                if label.is_empty() {
                    i += 1;
                    continue;
                }
                let Some(rect) = entity_rect_from_bare_rect(&node) else {
                    i += 1;
                    continue;
                };
                layout
                    .entities
                    .entry(label.clone())
                    .or_insert_with(|| rect.clone());
                ordered.push(OracleEntity {
                    qualified_name: label,
                    rect,
                });
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    if !ordered.is_empty() {
        layout.entity_list = ordered;
    }
}

fn entity_rect_from_bare_rect(node: &roxmltree::Node<'_, '_>) -> Option<EntityRect> {
    Some(EntityRect {
        x: parse_attr(node, "x")?,
        y: parse_attr(node, "y")?,
        width: parse_attr(node, "width")?,
        height: parse_attr(node, "height")?,
        icon_cx: None,
        icon_cy: None,
        glyph_path_d: None,
        body_polygon: None,
        icon_polygon: None,
        separator_paths: Vec::new(),
        visibility_polygons: Vec::new(),
        name_text_x: None,
        text_y_values: Vec::new(),
        text_x_values: Vec::new(),
        sep_y_values: Vec::new(),
        vis_icon_y_values: Vec::new(),
        fill: node.attribute("fill").map(String::from),
        body_style: node.attribute("style").map(String::from),
        rect_style: node.attribute("style").map(String::from),
        rect_rx: node.attribute("rx").map(String::from),
        rect_ry: node.attribute("ry").map(String::from),
        rect_filter: node.attribute("filter").map(String::from),
        entity_id: None,
        source_line: None,
        aux_rects: Vec::new(),
        sep_lines: Vec::new(),
        lines: Vec::new(),
        texts: Vec::new(),
        images: Vec::new(),
    })
}

fn polygon_bbox(points: &str) -> Option<(f64, f64, f64, f64)> {
    let coords: Vec<f64> = points
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| s.parse().ok())
        .collect();
    if coords.len() < 8 {
        return None;
    }
    let xs = coords.iter().step_by(2).copied();
    let ys = coords.iter().skip(1).step_by(2).copied();
    let min_x = xs.clone().fold(f64::INFINITY, f64::min);
    let max_x = xs.fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.clone().fold(f64::INFINITY, f64::min);
    let max_y = ys.fold(f64::NEG_INFINITY, f64::max);
    Some((min_x, min_y, max_x, max_y))
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
    fn extract_url_wrapped_path_entity() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100">
            <g><g class="entity" data-qualified-name="Storage" data-source-line="3" id="ent0004">
                <a href="https://example.com/storage" target="_top" title="https://example.com/storage" xlink:actuate="onRequest" xlink:href="https://example.com/storage" xlink:show="new" xlink:title="https://example.com/storage" xlink:type="simple">
                    <path d="M10,10 C10,10 40,10 40,10 L40,30 C40,30 10,30 10,30 L10,10" fill="#F1F1F1" style="stroke:#181818;stroke-width:0.5;"/>
                    <path d="M10,10 C10,20 40,20 40,10" fill="none" style="stroke:#181818;stroke-width:0.5;"/>
                    <text x="15" y="25">Storage</text>
                </a>
            </g></g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        let storage = layout.entities.get("Storage").unwrap();
        assert_eq!(storage.entity_id.as_deref(), Some("ent0004"));
        assert_eq!(storage.source_line.as_deref(), Some("3"));
        assert!((storage.x - 10.0).abs() < 0.001);
        assert!((storage.y - 10.0).abs() < 0.001);
        assert!((storage.width - 30.0).abs() < 0.001);
        assert!((storage.height - 20.0).abs() < 0.001);
        assert_eq!(storage.text_x_values, vec![15.0]);
        assert_eq!(storage.text_y_values, vec![25.0]);
    }

    #[test]
    fn extract_loose_package_symbol() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80">
            <g>
                <path d="M8.5,12 L81.2773,12 A3.75,3.75 0 0 1 83.7773,14.5 L90.7773,34.4883 L95.2773,34.4883 A2.5,2.5 0 0 1 97.7773,36.9883 L97.7773,62.4766 A2.5,2.5 0 0 1 95.2773,64.9766 L8.5,64.9766 A2.5,2.5 0 0 1 6,62.4766 L6,14.5 A2.5,2.5 0 0 1 8.5,12" fill="#F1F1F1" style="stroke:#181818;stroke-width:0.5;"/>
                <line style="stroke:#181818;stroke-width:0.5;" x1="6" x2="90.7773" y1="34.4883" y2="34.4883"/>
                <text fill="#000000" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="71.7773" x="10" y="27.5352">EmptyPkg</text>
            </g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        assert!(layout.clusters.is_empty());
        assert_eq!(layout.loose_clusters.len(), 1);
        let loose = &layout.loose_clusters[0];
        assert_eq!(loose.qualified_name, "EmptyPkg");
        assert_eq!(loose.children.len(), 3);
        assert!(matches!(loose.children[0], OracleClusterChild::Path(_)));
        assert!(matches!(loose.children[1], OracleClusterChild::Line(_)));
        assert!(matches!(loose.children[2], OracleClusterChild::Text(_)));
    }

    #[test]
    fn extract_note_link_child() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 50">
            <g><g class="entity" data-qualified-name="GMN1" data-source-line="2" id="ent0002">
                <path d="M7,7 L7,32 L86,32 L86,17 L76,7 L7,7" fill="#FEFFDD" style="stroke:#181818;stroke-width:0.5;"/>
                <a href="https://example.com" target="_top" title="tip" xlink:actuate="onRequest" xlink:href="https://example.com" xlink:show="new" xlink:title="tip" xlink:type="simple">
                    <text fill="#0000FF" filter="url(#shadow)" font-family="sans-serif" font-size="13" lengthAdjust="spacing" text-decoration="underline" textLength="29.4531" x="13" y="24">docs</text>
                </a>
                <image height="11" width="11" x="46" xlink:href="data:image/png;base64,abc" y="15"/>
            </g></g>
        </svg>"##;

        let layout = extract_oracle_layout(svg).unwrap();
        assert_eq!(layout.note_entities.len(), 1);
        let note = &layout.note_entities[0];
        let geom = note.box_geom.as_ref().unwrap();
        let Some(OracleNoteChild::Link(link)) = geom.children.get(1) else {
            panic!("expected note link child after note body path");
        };
        assert_eq!(link.href, "https://example.com");
        assert_eq!(link.title, "tip");
        assert_eq!(link.texts.len(), 1);
        assert_eq!(link.texts[0].text, "docs");
        assert_eq!(link.texts[0].filter.as_deref(), Some("url(#shadow)"));
        assert_eq!(link.texts[0].text_decoration.as_deref(), Some("underline"));
        let Some(OracleNoteChild::Image(image)) = geom.children.get(2) else {
            panic!("expected note image child after link");
        };
        assert_eq!(image.href, "data:image/png;base64,abc");
        assert_eq!(
            (image.x, image.y, image.width, image.height),
            (46.0, 15.0, 11.0, 11.0)
        );
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
