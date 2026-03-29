// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram SVG renderer.

use std::collections::{HashMap, HashSet};

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::SpriteData;
use rustuml_parser::diagram::deployment::*;

use crate::layout_oracle::OracleLayout;
use crate::metrics;
use crate::sprite::SpriteCache;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const NODE_MIN_W: f64 = 100.0;
const NODE_H: f64 = 40.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 12.0;
const LABEL_H: f64 = 22.0;
const CONTAINER_PADDING: f64 = 16.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
const NOTE_W: f64 = 100.0;

fn node_fill(kind: DeploymentNodeKind) -> &'static str {
    match kind {
        DeploymentNodeKind::Cloud => "#E8F4FD",
        DeploymentNodeKind::Database => "#D5F5E3",
        DeploymentNodeKind::Storage => "#FEF9E7",
        _ => "#F8F9FA",
    }
}

fn node_label_w(node: &DeploymentNode, sprites: &HashMap<String, SpriteData>) -> f64 {
    // For multi-line labels, use the longest line's width.
    let lw = node
        .label
        .split('\n')
        .map(|line| {
            crate::sprite::text_width_with_sprites(line, FONT_SIZE, sprites) + PADDING * 2.0
        })
        .fold(0.0_f64, f64::max);
    let sw = node
        .stereotype
        .as_deref()
        .map(|s| metrics::text_width(&format!("«{s}»"), SMALL_FONT) + PADDING * 2.0)
        .unwrap_or(0.0);
    lw.max(sw).max(NODE_MIN_W)
}

/// Returns (width, height) for this node including all children.
fn node_size(
    id: &str,
    nodes: &[DeploymentNode],
    sprites: &HashMap<String, SpriteData>,
) -> (f64, f64) {
    let node = match nodes.iter().find(|n| n.id == id) {
        Some(n) => n,
        None => return (NODE_MIN_W, NODE_H),
    };
    if node.children.is_empty() {
        return (node_label_w(node, sprites), NODE_H);
    }
    let mut inner_h = 0.0_f64;
    let mut max_child_w = 0.0_f64;
    for (i, child_id) in node.children.iter().enumerate() {
        let (cw, ch) = node_size(child_id, nodes, sprites);
        if i > 0 {
            inner_h += GAP;
        }
        inner_h += ch;
        max_child_w = max_child_w.max(cw);
    }
    let w = node_label_w(node, sprites).max(max_child_w + CONTAINER_PADDING * 2.0);
    let h = LABEL_H + CONTAINER_PADDING + inner_h + CONTAINER_PADDING;
    (w, h)
}

fn render_leaf_label(
    node: &DeploymentNode,
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    w: f64,
    sprite_cache: &SpriteCache,
    sprites: &HashMap<String, SpriteData>,
) {
    let lines: Vec<&str> = node.label.split('\n').collect();
    let line_h = FONT_SIZE + 2.0;
    let total_text_h = lines.len() as f64 * line_h;
    let cx = x + w / 2.0;
    if let Some(stereo) = &node.stereotype {
        svg.text(
            cx,
            y + NODE_H / 2.0 - 4.0,
            &format!("«{stereo}»"),
            "middle",
            SMALL_FONT,
        );
        let text_start_y = y + NODE_H / 2.0 + 6.0 - total_text_h / 2.0;
        for (i, line) in lines.iter().enumerate() {
            svg.text_with_sprites(
                cx,
                text_start_y + i as f64 * line_h,
                line,
                "middle",
                FONT_SIZE,
                sprite_cache,
                sprites,
            );
        }
    } else {
        let text_start_y = y + NODE_H / 2.0 + FONT_SIZE / 2.0 - total_text_h / 2.0 + 3.0;
        for (i, line) in lines.iter().enumerate() {
            svg.text_with_sprites(
                cx,
                text_start_y + i as f64 * line_h,
                line,
                "middle",
                FONT_SIZE,
                sprite_cache,
                sprites,
            );
        }
    }
}

fn render_container_label(
    node: &DeploymentNode,
    svg: &mut SvgBuilder,
    x: f64,
    y: f64,
    sprite_cache: &SpriteCache,
    sprites: &HashMap<String, SpriteData>,
) {
    if let Some(stereo) = &node.stereotype {
        svg.text(
            x + CONTAINER_PADDING,
            y + LABEL_H - 4.0,
            &format!("«{stereo}»"),
            "start",
            SMALL_FONT,
        );
        svg.text_with_sprites(
            x + CONTAINER_PADDING,
            y + LABEL_H + 9.0,
            &node.label,
            "start",
            FONT_SIZE,
            sprite_cache,
            sprites,
        );
    } else {
        svg.text_with_sprites(
            x + CONTAINER_PADDING,
            y + LABEL_H - 4.0,
            &node.label,
            "start",
            FONT_SIZE,
            sprite_cache,
            sprites,
        );
    }
}

#[allow(clippy::too_many_arguments)]
/// Render connections directly from oracle edge data.
fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &DeploymentDiagram,
    oracle: &OracleLayout,
) {
    for conn in &diagram.connections {
        let expected_id = format!("{}-to-{}", conn.from, conn.to);

        let oracle_edge = match oracle.edges.iter().find(|e| e.id == expected_id) {
            Some(e) => e,
            None => continue,
        };

        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("dependency");
        let source_line = oracle_edge.source_line.as_deref();
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");

        let source_attr = source_line
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();

        svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));
        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}"{source_attr} id="{link_id}">"#,
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

        svg.raw("</g>");
    }
}

fn render_node(
    id: &str,
    nodes: &[DeploymentNode],
    x: f64,
    y: f64,
    w: f64,
    svg: &mut SvgBuilder,
    theme: &Theme,
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
    sprite_cache: &SpriteCache,
    sprites: &HashMap<String, SpriteData>,
) {
    let node = match nodes.iter().find(|n| n.id == id) {
        Some(n) => n,
        None => return,
    };
    let (natural_w, h) = node_size(id, nodes, sprites);
    let w = w.max(natural_w);
    let fill = node_fill(node.kind);
    let gs = &theme.global;
    positions.insert(id.to_string(), (x, y, w, h));

    if node.children.is_empty() {
        svg.rounded_rect(x, y, w, NODE_H, 5.0, fill, &gs.border_color);
        render_leaf_label(node, svg, x, y, w, sprite_cache, sprites);
    } else {
        svg.rounded_rect(x, y, w, h, 5.0, fill, &gs.border_color);
        render_container_label(node, svg, x, y, sprite_cache, sprites);
        let inner_x = x + CONTAINER_PADDING;
        let inner_w = w - CONTAINER_PADDING * 2.0;
        let mut child_y = y + LABEL_H + CONTAINER_PADDING;
        // Clone children to avoid borrow conflict while mutating positions.
        let children: Vec<String> = node.children.clone();
        for child_id in children {
            let (_, ch) = node_size(&child_id, nodes, sprites);
            render_node(
                &child_id,
                nodes,
                inner_x,
                child_y,
                inner_w,
                svg,
                theme,
                positions,
                sprite_cache,
                sprites,
            );
            child_y += ch + GAP;
        }
    }
}

pub fn render(diagram: &DeploymentDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a deployment diagram to SVG, optionally using pre-computed layout from an oracle.
pub fn render_with_oracle(
    diagram: &DeploymentDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if diagram.nodes.is_empty() && diagram.notes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Build sprite cache from diagram metadata.
    let sprites = &diagram.meta.sprites;
    let sprite_cache = SpriteCache::from_sprites(sprites);

    // Check skinparams.
    let is_handwritten = diagram
        .meta
        .skinparams
        .iter()
        .any(|sp| sp.key.to_lowercase() == "handwritten" && sp.value.to_lowercase() == "true");

    // Find root nodes (not listed as children of any other node).
    let all_children: HashSet<&str> = diagram
        .nodes
        .iter()
        .flat_map(|n| n.children.iter().map(|s| s.as_str()))
        .collect();
    let roots: Vec<&DeploymentNode> = diagram
        .nodes
        .iter()
        .filter(|n| !all_children.contains(n.id.as_str()))
        .collect();

    let n = roots.len();

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let header_h = if diagram.meta.header.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let footer_h = if diagram.meta.footer.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    // Estimate note space: each note gets one row per text line plus padding.
    let note_line_h = SMALL_FONT + 4.0;
    let note_extra_h: f64 = diagram
        .notes
        .iter()
        .map(|note| {
            let nlines = note.text.lines().count().max(1);
            PADDING * 2.0 + (nlines as f64) * note_line_h + 10.0
        })
        .sum();

    let use_oracle = oracle.is_some();

    // Try Sugiyama layout for root nodes (skip when oracle is available).
    let layout_result = if use_oracle {
        None
    } else if n > 0 {
        let mut layout = LayoutGraph::new(Direction::TopToBottom);
        for root in &roots {
            let (w, h) = node_size(&root.id, &diagram.nodes, sprites);
            layout.add_node(&root.id, &root.label, w, h);
        }
        for conn in &diagram.connections {
            // Only add edges between root nodes (layout handles top-level positioning).
            let from_is_root = roots.iter().any(|r| r.id == conn.from);
            let to_is_root = roots.iter().any(|r| r.id == conn.to);
            if from_is_root && to_is_root {
                layout.add_edge(&conn.from, &conn.to, conn.label.as_deref());
            }
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    } else {
        None
    };

    let layout_positions = layout_result.as_ref().map(|r| &r.node_positions[..]);
    let empty_edge_paths: Vec<EdgePath> = Vec::new();
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };

    let use_sugiyama = !use_oracle && layout_positions.is_some_and(|p| p.len() >= n);

    // Compute root node positions (oracle, Sugiyama, or grid fallback).
    let (root_xy, nodes_w, nodes_h) = if let Some(orc) = oracle {
        // Oracle mode: extract positions from oracle entity data.
        let mut rxy = Vec::new();
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for root in &roots {
            let (default_w, default_h) = node_size(&root.id, &diagram.nodes, sprites);
            let rect = orc
                .entities
                .get(&root.id)
                .or_else(|| orc.entities.get(&root.label));
            if let Some(rect) = rect {
                rxy.push((rect.x, rect.y, rect.width));
                max_x = max_x.max(rect.x + rect.width);
                max_y = max_y.max(rect.y + rect.height);
            } else {
                let x = MARGIN + rxy.len() as f64 * (default_w + GAP);
                let y = MARGIN;
                rxy.push((x, y, default_w));
                max_x = max_x.max(x + default_w);
                max_y = max_y.max(y + default_h);
            }
        }
        (rxy, max_x, max_y)
    } else if use_sugiyama {
        let lp = layout_positions.unwrap();
        let mut rxy = Vec::new();
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (i, root) in roots.iter().enumerate() {
            let (w, h) = node_size(&root.id, &diagram.nodes, sprites);
            rxy.push((lp[i].x + MARGIN, lp[i].y + MARGIN, w));
            max_x = max_x.max(lp[i].x + w);
            max_y = max_y.max(lp[i].y + h);
        }
        (rxy, max_x, max_y)
    } else {
        // Grid fallback.
        let cols = if n == 0 {
            1
        } else {
            (n as f64).sqrt().ceil() as usize
        };
        let rows = if n == 0 { 0 } else { n.div_ceil(cols) };
        let col_w: Vec<f64> = {
            let mut cw = vec![0.0_f64; cols];
            for (i, root) in roots.iter().enumerate() {
                let (w, _) = node_size(&root.id, &diagram.nodes, sprites);
                cw[i % cols] = cw[i % cols].max(w);
            }
            cw
        };
        let row_h: Vec<f64> = {
            let mut rh = vec![0.0_f64; rows];
            for (i, root) in roots.iter().enumerate() {
                let (_, h) = node_size(&root.id, &diagram.nodes, sprites);
                rh[i / cols] = rh[i / cols].max(h);
            }
            rh
        };
        let mut rxy = Vec::new();
        for (i, _root) in roots.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
            let y = MARGIN + row_h[..row].iter().sum::<f64>() + GAP * row as f64;
            let w = col_w[col];
            rxy.push((x, y, w));
        }
        let nw = if col_w.is_empty() {
            NODE_MIN_W
        } else {
            col_w.iter().sum::<f64>() + GAP * cols.saturating_sub(1) as f64
        };
        let nh = if row_h.is_empty() {
            0.0
        } else {
            row_h.iter().sum::<f64>() + GAP * rows.saturating_sub(1) as f64
        };
        (rxy, nw, nh)
    };

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else {
        (
            MARGIN * 2.0 + nodes_w.max(NOTE_W + 20.0),
            MARGIN * 2.0 + nodes_h + title_h + header_h + footer_h + note_extra_h,
        )
    };

    let mut svg = SvgBuilder::new(total_w, total_h);

    // Header.
    if let Some(header) = &diagram.meta.header {
        svg.text(total_w / 2.0, header_h - 4.0, header, "middle", SMALL_FONT);
    }

    // Footer.
    if let Some(footer) = &diagram.meta.footer {
        svg.text(total_w / 2.0, total_h - 4.0, footer, "middle", SMALL_FONT);
    }

    // Handwritten warning.
    if is_handwritten {
        let nbsp = '\u{00a0}';
        let msg = format!(
            "Please{n}use{n}'!option{n}handwritten{n}true'{n}to{n}enable{n}handwritten",
            n = nbsp
        );
        svg.text(
            total_w / 2.0,
            header_h + title_h + MARGIN + FONT_SIZE,
            &msg,
            "middle",
            SMALL_FONT,
        );
    }

    // Title.
    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_w / 2.0,
            header_h + TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }

    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let content_top = header_h + title_h + MARGIN;

    for (i, root) in roots.iter().enumerate() {
        let (x, y_off, w) = root_xy[i];
        let y = content_top + y_off - MARGIN; // adjust since root_xy already includes MARGIN
        render_node(
            &root.id,
            &diagram.nodes,
            x,
            y,
            w,
            &mut svg,
            theme,
            &mut positions,
            &sprite_cache,
            sprites,
        );
    }

    let gs = &theme.global;

    // Render connections.
    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc);
    } else {
        for conn in &diagram.connections {
            // Try bezier path from layout engine first.
            let edge_path = edge_paths
                .iter()
                .find(|ep| ep.from == conn.from && ep.to == conn.to);

            if let Some(ep) = edge_path
                && !ep.points.is_empty()
            {
                svg.bezier_path_with_arrow(&ep.points, &gs.border_color, false, 8.0);
                if let Some(label) = &conn.label {
                    let first = ep.points.first().unwrap();
                    let last = ep.points.last().unwrap();
                    let mx = (first.0 + last.0) / 2.0;
                    let my = (first.1 + last.1) / 2.0;
                    svg.text(mx, my - 4.0, label, "middle", SMALL_FONT);
                }
                continue;
            }

            // Fallback to straight lines.
            if let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, _))) =
                (positions.get(&conn.from), positions.get(&conn.to))
            {
                svg.line_segment(
                    fx + fw / 2.0,
                    fy + fh,
                    tx + tw / 2.0,
                    ty,
                    &gs.border_color,
                    false,
                );
                svg.arrow_head(tx + tw / 2.0, ty, 90.0);
                if let Some(label) = &conn.label {
                    let mx = (fx + fw / 2.0 + tx + tw / 2.0) / 2.0;
                    let my = (fy + fh + ty) / 2.0;
                    svg.text(mx, my - 4.0, label, "middle", SMALL_FONT);
                }
            }
        }
    }

    // Render notes.
    let note_fill = "#FEFECE";
    let mut note_y = content_top + nodes_h + if nodes_h > 0.0 { GAP } else { 0.0 };

    for note in &diagram.notes {
        let lines: Vec<&str> = note.text.lines().collect();
        let nlines = lines.len().max(1);
        let nh = PADDING * 2.0 + (nlines as f64) * note_line_h;
        let nw = lines
            .iter()
            .map(|l| metrics::text_width(l, SMALL_FONT) + PADDING * 2.0)
            .fold(NOTE_W, |a, b| a.max(b))
            .min(total_w - MARGIN * 2.0);
        let nx = MARGIN;
        let ny = note_y;

        // Determine where to place the note box.
        let (box_x, box_y, has_target, tgt_cx, tgt_top) = if let Some(target_id) = &note.target {
            if let Some(&(tgt_x, tgt_y, tgt_w, _)) = positions.get(target_id) {
                let note_cx = (tgt_x + tgt_w / 2.0).min(total_w - MARGIN - nw);
                (note_cx, ny, true, tgt_x + tgt_w / 2.0, tgt_y)
            } else {
                (nx, ny, false, 0.0, 0.0)
            }
        } else {
            (nx, ny, false, 0.0, 0.0)
        };

        // Draw the note box.
        svg.rect(box_x, box_y, nw, nh, note_fill, &gs.border_color);

        // Draw each line of text.
        for (i, line) in lines.iter().enumerate() {
            let ly = box_y + PADDING + (i as f64) * note_line_h + SMALL_FONT;
            svg.text(box_x + PADDING, ly, line, "start", SMALL_FONT);
        }

        // If attached, draw a dashed connector to the target element.
        if has_target {
            svg.line_segment(
                box_x + nw / 2.0,
                box_y,
                tgt_cx,
                tgt_top,
                &gs.border_color,
                true,
            );
        }

        note_y += nh + 10.0;
    }

    // Render legend.
    if let Some(legend) = &diagram.meta.legend {
        let legend_x = MARGIN;
        let mut legend_y = note_y + 10.0;
        for line in legend.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            // Strip leading/trailing `|` from table rows.
            let t = t.trim_matches('|').trim();
            if t.is_empty() {
                continue;
            }
            // Split by `|` for table cells.
            let cells: Vec<&str> = t
                .split('|')
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            let mut cell_x = legend_x;
            for cell in cells {
                svg.text(cell_x, legend_y, cell, "start", SMALL_FONT);
                let cell_w = metrics::text_width(cell, SMALL_FONT) + 20.0;
                cell_x += cell_w;
            }
            legend_y += 14.0;
        }
    }

    svg.finalize()
}
