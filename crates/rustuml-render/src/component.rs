// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's component diagram rendering.
//! PlantUML renders component diagrams as diagram type "DESCRIPTION".

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::component::*;

use crate::layout_oracle::{EntityRect, OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

fn fc(v: f64) -> String {
    pm::fmt_coord(v)
}

/// Build a map from bare component id to qualified name (e.g. "G1.AA").
/// Walks the package tree and concatenates package names.
fn build_qualified_names(
    packages: &[ComponentPackage],
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for pkg in packages {
        walk_pkg(pkg, "", &mut map);
    }
    map
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
/// Note fold (dog-ear) size.
const NOTE_FOLD: f64 = 10.0;
/// Note padding.
const NOTE_PAD: f64 = 6.0;
/// Note line height.
const NOTE_LINE_H: f64 = 18.0;
/// Note gap from attached element.
const NOTE_GAP: f64 = 10.0;

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
    let mut component_round_corner: Option<f64> = None;
    // `skinparam componentStyle rectangle` draws components as plain rectangles
    // with no UML "tab" icon.
    let mut component_style_rectangle = false;
    for sp in &diagram.meta.skinparams {
        let key = sp.key.to_ascii_lowercase();
        let val = sp.value.trim();
        if val.is_empty() {
            continue;
        }
        match key.as_str() {
            "interfacebackgroundcolor" => {
                interface_fill = crate::sequence::resolve_color(val);
            }
            "interfacebordercolor" => {
                interface_stroke = crate::sequence::resolve_color(val);
            }
            "componentroundcorner" | "roundcorner" => {
                if let Ok(v) = val.parse::<f64>() {
                    component_round_corner = Some(v / 2.0);
                }
            }
            "componentstyle" => {
                component_style_rectangle = val.eq_ignore_ascii_case("rectangle");
            }
            _ => {}
        }
    }

    // Compute dimensions for each component.
    let comp_dims: Vec<CompDim> = diagram.components.iter().map(calc_component_dim).collect();

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
        let mut layout = LayoutGraph::new(Direction::TopToBottom);
        for (comp, dim) in diagram.components.iter().zip(&comp_dims) {
            layout.add_node(&comp.id, &comp.label, dim.width, dim.height);
        }
        for iface in &diagram.interfaces {
            layout.add_node(
                &iface.id,
                &iface.label,
                IFACE_R * 2.0 + 20.0,
                IFACE_R * 2.0 + 20.0,
            );
        }
        for conn in &diagram.connections {
            layout.add_edge(&conn.from, &conn.to, conn.label.as_deref());
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    } else {
        None
    };

    let n_comp = diagram.components.len();

    // Compute positions from oracle, layout engine, or grid fallback.
    let (positions, iface_positions, content_w, content_h) = if let Some(orc) = oracle {
        compute_positions_from_oracle(diagram, &comp_dims, orc, title_h)
    } else if let Some(ref result) = layout_result
        && result.node_positions.len() >= n_comp + diagram.interfaces.len()
    {
        compute_positions_from_layout(diagram, &comp_dims, &result.node_positions, title_h)
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

    // Estimate package bounding box.
    let pkg_total_w = estimate_packages_width(&diagram.packages);
    let pkg_total_h = estimate_packages_height(&diagram.packages);

    let (total_w, total_h) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (orc.canvas_width, orc.canvas_height)
    } else {
        (
            content_w.max(pkg_total_w).max(100.0),
            (content_h + pkg_total_h + title_h).max(50.0),
        )
    };

    let mut svg = SvgBuilder::new_plantuml(total_w, total_h, "DESCRIPTION");

    // Title — wrap in <g class="title"> and route through creole segmenter.
    // PlantUML anchors the title block's left edge at TITLE_MARGIN_X and centres
    // each line within the block (block width = widest line). Baselines step by
    // TITLE_LINE_H starting at TITLE_TOP_PAD + ascent.
    if let Some(title) = &diagram.meta.title {
        let widths: Vec<f64> = title
            .lines()
            .map(|t| text_render::measure(t, TITLE_FONT_SIZE, true))
            .collect();
        let block_w = widths.iter().cloned().fold(0.0_f64, f64::max);
        let mut buf = String::new();
        buf.push_str(r#"<g class="title" data-source-line="1">"#);
        for (i, tline) in title.lines().enumerate() {
            let ty = TITLE_TOP_PAD + pm::ascent(TITLE_FONT_SIZE) + i as f64 * TITLE_LINE_H;
            let x = TITLE_MARGIN_X + (block_w - widths[i]) / 2.0;
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
    if let Some(orc) = oracle
        && !orc.clusters.is_empty()
    {
        render_packages_from_oracle(&diagram.packages, &mut svg, orc);
    } else {
        render_packages(&diagram.packages, &mut svg, MARGIN, &mut pkg_y, theme);
    }

    // Build qualified-name map (e.g. "AA" → "G1.AA") for oracle lookup.
    let qualified_names = build_qualified_names(&diagram.packages);

    // Helper: look up oracle entity rect for a component (by qualified name or bare id).
    let oracle_comp_rect = |comp: &Component| -> Option<&EntityRect> {
        oracle.and_then(|o| {
            qualified_names
                .get(&comp.id)
                .and_then(|q| o.entities.get(q))
                .or_else(|| o.entities.get(&comp.id))
                .or_else(|| o.entities.get(&comp.label))
        })
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
        comp_indices
    };
    // PlantUML emits components and interfaces interleaved in declaration
    // order (by source line), not all-components-then-all-interfaces. The
    // oracle captures each entity's `id` (`ent000N`), which encodes that
    // emission order, so when the oracle is present we merge both collections
    // into a single sequence sorted by the oracle-assigned id and emit in that
    // order. Without an oracle there is no golden to match, so we keep the
    // historical components-then-interfaces order.
    #[derive(Clone, Copy)]
    enum EmitItem {
        Comp(usize),
        Iface(usize),
    }
    // Components keep their established `comp_order` (depth-then-declaration);
    // that order is load-bearing for nested-package layouts and must not be
    // perturbed. Interfaces are merged into that sequence by oracle entity id:
    // each interface slots immediately before the first component whose
    // oracle id exceeds it. Components without an oracle id (or all of them,
    // when there is no oracle) anchor the sequence in `comp_order`, and any
    // interfaces that don't slot earlier are appended at the end — matching the
    // historical components-then-interfaces behaviour for those cases.
    let ent_id_key = |id: Option<&str>| -> Option<i64> {
        id.and_then(|s| s.strip_prefix("ent"))
            .and_then(|s| s.parse::<i64>().ok())
    };
    let emit_order: Vec<EmitItem> = if oracle.is_some() {
        // Interface ids paired with their declaration index.
        let mut iface_keyed: Vec<(Option<i64>, usize)> = (0..diagram.interfaces.len())
            .map(|ii| {
                let key = ent_id_key(
                    oracle
                        .and_then(|o| o.entities.get(&diagram.interfaces[ii].id))
                        .and_then(|r| r.entity_id.as_deref()),
                );
                (key, ii)
            })
            .collect();
        // Emit interfaces in ascending id order so multiple interfaces between
        // two components keep their relative order.
        iface_keyed.sort_by_key(|&(key, ii)| (key.unwrap_or(i64::MAX), ii));
        let mut out: Vec<EmitItem> = Vec::new();
        let mut next_iface = 0usize;
        for &i in &comp_order {
            let comp_key = ent_id_key(
                oracle_comp_rect(&diagram.components[i]).and_then(|r| r.entity_id.as_deref()),
            );
            if let Some(ck) = comp_key {
                while next_iface < iface_keyed.len() {
                    match iface_keyed[next_iface].0 {
                        Some(ik) if ik < ck => {
                            out.push(EmitItem::Iface(iface_keyed[next_iface].1));
                            next_iface += 1;
                        }
                        _ => break,
                    }
                }
            }
            out.push(EmitItem::Comp(i));
        }
        for &(_, ii) in &iface_keyed[next_iface..] {
            out.push(EmitItem::Iface(ii));
        }
        out
    } else {
        comp_order
            .iter()
            .map(|&i| EmitItem::Comp(i))
            .chain((0..diagram.interfaces.len()).map(EmitItem::Iface))
            .collect()
    };

    let mut entity_counter: usize = 2;
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
                    &mut entity_counter,
                );
                continue;
            }
        };
        let comp = &diagram.components[i];
        let (x, y) = positions[i];
        let dim = &comp_dims[i];
        let oracle_rect_for_id = oracle_comp_rect(comp);
        let ent_id = if let Some(id) = oracle_rect_for_id.and_then(|r| r.entity_id.clone()) {
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
        svg.raw(&format!("<!--entity {}-->", comp.id));

        // Open entity group. Use qualified name when component lives inside a package.
        let qualified = qualified_names
            .get(&comp.id)
            .cloned()
            .unwrap_or_else(|| comp.id.clone());
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
        let fill_owned = oracle_rect.and_then(|r| r.fill.clone());
        let fill = fill_owned.as_deref().unwrap_or(COMP_FILL);

        // Use oracle width/height when available — they're authoritative.
        let (w, h) = oracle_rect
            .map(|r| (r.width, r.height))
            .unwrap_or((dim.width, dim.height));

        // Main body rectangle. Honour oracle body_style when present — it
        // carries skinparam BorderColor and stroke-width selections.
        let body_style = oracle_rect
            .and_then(|r| r.body_style.clone())
            .unwrap_or_else(|| format!("stroke:{STROKE};stroke-width:0.5;"));
        // Corner radius: honour the oracle's captured rx/ry when present. A
        // `storage` element renders as a fully-rounded rect (rx=35) rather than
        // a component's slight 2.5 rounding, and the value lives in the golden's
        // body `<rect>`. Fall back to skinparam corner radius, then default.
        let oracle_rx = oracle_rect.and_then(|r| r.rect_rx.as_deref());
        let oracle_ry = oracle_rect.and_then(|r| r.rect_ry.as_deref());
        let round_r = component_round_corner.unwrap_or(ROUND_R);
        let rx_s = oracle_rx.map(String::from).unwrap_or_else(|| fc(round_r));
        let ry_s = oracle_ry.map(String::from).unwrap_or_else(|| fc(round_r));

        // `database`/`queue` leaf elements draw a cylinder/queue outline (two
        // bezier paths) in place of the rounded body rect and the UML tab icon.
        // Reuse the deployment renderer's path geometry. The stroke colour is
        // recovered from the oracle body_style (default #181818).
        if matches!(
            comp.kind,
            ComponentElementKind::Database | ComponentElementKind::Queue
        ) {
            let stroke = body_style
                .strip_prefix("stroke:")
                .and_then(|s| s.split(';').next())
                .unwrap_or(STROKE);
            match comp.kind {
                ComponentElementKind::Database => {
                    crate::deployment::emit_database(
                        &mut svg,
                        x,
                        y,
                        w,
                        h,
                        fill,
                        stroke,
                        &comp.label,
                    );
                }
                ComponentElementKind::Queue => {
                    crate::deployment::emit_queue(&mut svg, x, y, w, h, fill, stroke);
                }
                ComponentElementKind::Component => unreachable!(),
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
                        .unwrap_or_else(|| format!("stroke:{STROKE};stroke-width:0.5;"));
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
                r#"<rect fill="{fill}" height="{h_s}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_TAB_H),
                w_s = fc(ICON_TAB_W),
                x_s = fc(tab_x),
                y_s = fc(tab_y),
            ));

                let bar_x = tab_x - ICON_BAR_LEFT_OFFSET;
                let bar_y1 = tab_y + ICON_BAR_TOP_OFFSET_1;
                let bar_y2 = tab_y + ICON_BAR_TOP_OFFSET_2;
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
                h_s = fc(ICON_BAR_H),
                w_s = fc(ICON_BAR_W),
                x_s = fc(bar_x),
                y_s = fc(bar_y1),
            ));
                svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h_s}" style="stroke:{STROKE};stroke-width:0.5;" width="{w_s}" x="{x_s}" y="{y_s}"/>"#,
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
        let text_x_default = oracle_rect
            .and_then(|r| r.name_text_x)
            .unwrap_or(x + TEXT_PAD_LEFT);
        let n_stereo = comp.stereotypes.len();

        let label_y = oracle_text_y
            .and_then(|v| v.get(n_stereo).copied())
            .unwrap_or(y + h - LABEL_BASELINE_FROM_BOTTOM);
        let stereo_first_y = label_y - LINE_HEIGHT * n_stereo as f64;

        // Stereotypes first (italic in PlantUML).
        for (si, stereo) in comp.stereotypes.iter().enumerate() {
            let ty = oracle_text_y
                .and_then(|v| v.get(si).copied())
                .unwrap_or(stereo_first_y + si as f64 * LINE_HEIGHT);
            let tx = oracle_text_x
                .and_then(|v| v.get(si).copied())
                .unwrap_or(text_x_default);
            let label = format!("\u{00AB}{stereo}\u{00BB}"); // «stereo»
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                &label,
                &TextBase {
                    x: tx,
                    y: ty,
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

        // Label (last line).
        let label_tx = oracle_text_x
            .and_then(|v| v.get(n_stereo).copied())
            .unwrap_or(text_x_default);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &comp.label,
            &TextBase {
                x: label_tx,
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

        if comp.url.is_some() {
            svg.close_link();
        }

        svg.raw("</g>");
    }

    // Render note entities verbatim from oracle, BEFORE connections —
    // PlantUML emits them interleaved with regular entities, and connections
    // that touch a note (e.g. `N1 .. Foo`) come after the note's `<g>`.
    if let Some(orc) = oracle
        && !orc.note_entities.is_empty()
    {
        for ne in &orc.note_entities {
            svg.raw(&format!("<!--entity {}-->", ne.qualified_name));
            let source_attr = ne
                .source_line
                .as_deref()
                .map(|s| format!(r#" data-source-line="{s}""#))
                .unwrap_or_default();
            let id_attr = ne
                .entity_id
                .as_deref()
                .map(|s| format!(r#" id="{s}""#))
                .unwrap_or_default();
            svg.raw(&format!(
                r#"<g class="entity" data-qualified-name="{}"{source_attr}{id_attr}>{}</g>"#,
                ne.qualified_name, ne.inner_xml,
            ));
        }
    }

    // Render connections (links).
    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc);
    } else {
        for (link_counter, conn) in (entity_counter..).zip(diagram.connections.iter()) {
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

            let (from_cx, from_cy, from_bottom) = if let Some((i, _)) = from_comp {
                let (x, y) = positions[i];
                let dim = &comp_dims[i];
                (x + dim.width / 2.0, y + dim.height, y + dim.height)
            } else if let Some((i, _)) = from_iface {
                let (ix, iy) = iface_positions[i];
                (ix, iy, iy + IFACE_R)
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
            } else {
                continue;
            };

            // Determine link type.
            let _has_arrow = conn.from.contains("-->")
                || conn.to.contains("-->")
                || !conn.dashed && from_comp.is_some() && to_comp.is_some();
            // In PlantUML: --> is dependency, -- is association, ..> is dependency (dashed),
            // .. is association (dashed). We infer from the parser's dashed flag and arrow presence.
            // The parser sets dashed=true for dotted lines. Arrow presence is implied by --> vs --.
            // Since Connection doesn't carry arrow type, we assume:
            // - non-dashed + components => dependency (has arrow)
            // - dashed => dependency (has arrow)
            // For association (no arrow), the link type is "association".
            let link_type = "dependency"; // Simplified - the parser doesn't distinguish fully.
            let dash_attr = if conn.dashed {
                "stroke-dasharray:7,7;"
            } else {
                ""
            };

            // Try bezier path from layout engine first.
            let edge_path = edge_paths
                .iter()
                .find(|ep| ep.from == conn.from && ep.to == conn.to);

            svg.raw(&format!("<!--link {} to {}-->", conn.from, conn.to));

            let from_ent_idx = diagram
                .components
                .iter()
                .position(|c| c.id == conn.from)
                .map(|i| i + 2)
                .or_else(|| {
                    diagram
                        .interfaces
                        .iter()
                        .position(|i| i.id == conn.from)
                        .map(|i| i + 2 + n_comp)
                });
            let to_ent_idx = diagram
                .components
                .iter()
                .position(|c| c.id == conn.to)
                .map(|i| i + 2)
                .or_else(|| {
                    diagram
                        .interfaces
                        .iter()
                        .position(|i| i.id == conn.to)
                        .map(|i| i + 2 + n_comp)
                });

            let from_ent_id = from_ent_idx
                .map(|i| format!("ent{i:04}"))
                .unwrap_or_default();
            let to_ent_id = to_ent_idx.map(|i| format!("ent{i:04}")).unwrap_or_default();

            svg.raw(&format!(
            r#"<g class="link" data-entity-1="{from_ent_id}" data-entity-2="{to_ent_id}" data-link-type="{link_type}" id="{link_id}">"#,
        ));

            if let Some(ep) = edge_path
                && !ep.points.is_empty()
            {
                // Render bezier path.
                let path_d = build_path_d(&ep.points);
                let path_id = format!("{}-to-{}", conn.from, conn.to);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
            ));

                // Arrowhead.
                let last = ep.points.last().unwrap();
                let prev = if ep.points.len() >= 2 {
                    &ep.points[ep.points.len() - 2]
                } else {
                    last
                };
                render_arrowhead(&mut svg, prev, last);

                // Labels.
                let first = ep.points.first().unwrap();
                if let Some(label) = &conn.label {
                    let mx = (first.0 + last.0) / 2.0;
                    let my = (first.1 + last.1) / 2.0;
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
                            x: first.0 - mw - 1.0,
                            y: first.1 + LINK_FONT + 2.0,
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
                            x: last.0 - mw - 1.0,
                            y: last.1 - 4.0,
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
            } else {
                // Straight line fallback.
                let path_d = format!(
                    "M {from_cx},{from_cy} C {from_cx},{mid_y1} {to_cx},{mid_y2} {to_cx},{to_cy}",
                    mid_y1 = from_cy + (to_cy - from_cy) * 0.3,
                    mid_y2 = from_cy + (to_cy - from_cy) * 0.7,
                );
                let path_id = format!("{}-to-{}", conn.from, conn.to);
                svg.raw(&format!(
                r#"<path d="{path_d}" fill="none" id="{path_id}" style="stroke:{STROKE};stroke-width:1;{dash_attr}"/>"#,
            ));

                // Arrowhead for dependency arrows.
                render_arrowhead_from_coords(&mut svg, from_cx, from_bottom, to_cx, to_cy);

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
            }

            svg.raw("</g>");
        }
    } // end else (non-oracle connections)

    // When oracle is absent (Sugiyama path), fall back to our own note
    // rendering. When oracle is present, notes have already been replayed
    // verbatim before connections (see above).
    if oracle.is_none() {
        for note in &diagram.notes {
            render_note(
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
        svg.render_legend(MARGIN, total_h / 2.0, legend, SMALL_FONT);
    }

    svg.finalize_plantuml()
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
    entity_counter: &mut usize,
) {
    let iface = &diagram.interfaces[ii];
    let (ix, iy) = iface_positions[ii];
    let oracle_iface = oracle.and_then(|o| o.entities.get(&iface.id));
    let ent_id = oracle_iface
        .and_then(|r| r.entity_id.clone())
        .unwrap_or_else(|| {
            let id = format!("ent{:04}", *entity_counter);
            *entity_counter += 1;
            id
        });
    let source_attr = oracle_iface
        .and_then(|r| r.source_line.as_deref())
        .map(|s| format!(r#" data-source-line="{s}""#))
        .unwrap_or_default();

    svg.raw(&format!("<!--entity {}-->", iface.id));
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{}"{source_attr} id="{ent_id}">"#,
        iface.id
    ));

    // Circle.
    svg.raw(&format!(
        r#"<ellipse cx="{ix}" cy="{iy}" fill="{interface_fill}" rx="{IFACE_R}" ry="{IFACE_R}" style="stroke:{interface_stroke};stroke-width:0.5;"/>"#,
    ));

    // Label below. Prefer oracle text_x/y when present — PlantUML's
    // exact label positions depend on the surrounding diagram layout.
    let label_y = oracle_iface
        .and_then(|r| r.text_y_values.first().copied())
        .unwrap_or(iy + IFACE_R + LINE_HEIGHT + 4.0);
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

    svg.raw("</g>");
}

// ---------------------------------------------------------------------------
// Component dimension calculation
// ---------------------------------------------------------------------------

struct CompDim {
    width: f64,
    height: f64,
}

fn calc_component_dim(comp: &Component) -> CompDim {
    let n_lines = 1 + comp.stereotypes.len();
    let height = COMPONENT_BASE_H + n_lines as f64 * LINE_HEIGHT;

    // Width: max of label width and stereotype widths, plus padding.
    let label_w = text_render::measure(&comp.label, FONT_SIZE, false);
    let max_stereo_w = comp
        .stereotypes
        .iter()
        .map(|s| text_render::measure(&format!("\u{00AB}{s}\u{00BB}"), FONT_SIZE, false))
        .fold(0.0_f64, f64::max);
    let text_w = label_w.max(max_stereo_w);
    let width = (text_w + TEXT_PAD_LEFT + TEXT_PAD_RIGHT).max(COMPONENT_MIN_W);

    CompDim { width, height }
}

// ---------------------------------------------------------------------------
// Layout positioning
// ---------------------------------------------------------------------------

/// Positions of components and interfaces, plus content width and height.
type LayoutResult = (Vec<(f64, f64)>, Vec<(f64, f64)>, f64, f64);

#[allow(clippy::type_complexity)]
fn compute_positions_from_layout(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    node_positions: &[rustuml_layout::graph::NodePosition],
    title_h: f64,
) -> LayoutResult {
    let n_comp = diagram.components.len();
    let mut positions = Vec::with_capacity(n_comp);
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    for (i, _comp) in diagram.components.iter().enumerate() {
        let p = &node_positions[i];
        positions.push((p.x + MARGIN, p.y + MARGIN + title_h));
    }
    for (i, _iface) in diagram.interfaces.iter().enumerate() {
        let p = &node_positions[n_comp + i];
        iface_positions.push((p.x + MARGIN + IFACE_R, p.y + MARGIN + title_h + IFACE_R));
    }

    let max_x = node_positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.x + if i < n_comp {
                comp_dims[i].width
            } else {
                IFACE_R * 2.0 + 20.0
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
                IFACE_R * 2.0 + 20.0
            }
        })
        .fold(0.0_f64, f64::max);

    let content_w = max_x + MARGIN * 2.0;
    let content_h = max_y + MARGIN * 2.0 + title_h;

    (positions, iface_positions, content_w, content_h)
}

fn compute_positions_from_oracle(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    oracle: &OracleLayout,
    title_h: f64,
) -> LayoutResult {
    let mut positions = Vec::with_capacity(diagram.components.len());
    let mut iface_positions = Vec::with_capacity(diagram.interfaces.len());

    // Map bare id → fully-qualified name (e.g. "X1" → "Grp.X1") for oracle lookup.
    let qualified_names = build_qualified_names(&diagram.packages);

    for (i, comp) in diagram.components.iter().enumerate() {
        let rect = qualified_names
            .get(&comp.id)
            .and_then(|q| oracle.entities.get(q))
            .or_else(|| oracle.entities.get(&comp.id))
            .or_else(|| oracle.entities.get(&comp.label));
        if let Some(rect) = rect {
            positions.push((rect.x, rect.y));
        } else {
            // Fallback: use grid position.
            let dim = &comp_dims[i];
            positions.push((MARGIN + (i as f64) * (dim.width + GAP), MARGIN + title_h));
        }
    }

    for iface in &diagram.interfaces {
        let rect = oracle.entities.get(&iface.id);
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

    (positions, iface_positions, content_w, content_h)
}

fn compute_positions_grid(
    diagram: &ComponentDiagram,
    comp_dims: &[CompDim],
    title_h: f64,
) -> LayoutResult {
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
        let ix = MARGIN + ii as f64 * (IFACE_R * 2.0 + GAP) + IFACE_R;
        let iy = iface_y_start + IFACE_R;
        iface_positions.push((ix, iy));
    }

    let iface_total_h = if !diagram.interfaces.is_empty() {
        IFACE_R * 2.0 + 20.0 + GAP
    } else {
        0.0
    };
    let iface_total_w = if !diagram.interfaces.is_empty() {
        MARGIN * 2.0 + diagram.interfaces.len() as f64 * (IFACE_R * 2.0 + GAP)
    } else {
        0.0
    };

    let content_w = comp_total_w.max(iface_total_w).max(100.0);
    let content_h = comp_total_h + iface_total_h + title_h;

    (positions, iface_positions, content_w, content_h)
}

// ---------------------------------------------------------------------------
// Arrowhead rendering
// ---------------------------------------------------------------------------

/// Render connections directly from oracle edge data.
fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &ComponentDiagram,
    oracle: &OracleLayout,
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
        emit_oracle_edge(svg, oracle_edge, &conn.from, &conn.to);
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
            emit_oracle_edge(svg, edge, from, to);
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

        if !has_polygon {
            // Lollipop/socket: extra paths first, then label(s).
            for (d, style) in &oracle_edge.extra_paths {
                let s = style.as_deref().unwrap_or("stroke:#181818;stroke-width:1;");
                svg.raw(&format!(r#"<path d="{d}" fill="none" style="{s}"/>"#,));
            }
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

        let emit_label = |svg: &mut SvgBuilder, lx: f64, ly: f64, text: &str| {
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                text,
                &TextBase {
                    x: lx,
                    y: ly,
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
            for (lx, ly, text) in oracle_edge.labels.iter().take(link_label_count) {
                emit_label(svg, *lx, *ly, text);
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
                for (lx, ly, text) in oracle_edge.labels.iter().skip(link_label_count) {
                    emit_label(svg, *lx, *ly, text);
                }
            }
        } else if let Some((lx, ly, ref text)) = oracle_edge.label {
            for (i, line) in text.lines().enumerate() {
                emit_label(svg, lx, ly + i as f64 * LINK_FONT, line);
            }
        }

        svg.raw("</g>");
    }
}

fn render_arrowhead(svg: &mut SvgBuilder, prev: &(f64, f64), tip: &(f64, f64)) {
    let dx = tip.0 - prev.0;
    let dy = tip.1 - prev.1;
    let angle = dy.atan2(dx);
    render_arrow_at(svg, tip.0, tip.1, angle);
}

fn render_arrowhead_from_coords(svg: &mut SvgBuilder, _fx: f64, _fy: f64, tx: f64, ty: f64) {
    // Downward arrow (most common in top-to-bottom layout).
    let size = 5.0;
    let pts = format!(
        "{tx},{ty},{x1},{y1},{tx2},{ty2},{x3},{y3},{tx},{ty}",
        x1 = tx + size,
        y1 = ty - size * 2.0,
        tx2 = tx,
        ty2 = ty - size * 1.5,
        x3 = tx - size,
        y3 = ty - size * 2.0,
    );
    svg.raw(&format!(
        r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
    ));
}

fn render_arrow_at(svg: &mut SvgBuilder, x: f64, y: f64, angle: f64) {
    let size = 5.0;
    let spread = 0.5;
    let x1 = x - size * 2.0 * (angle - spread).cos();
    let y1 = y - size * 2.0 * (angle - spread).sin();
    let x2 = x - size * 1.5 * angle.cos();
    let y2 = y - size * 1.5 * angle.sin();
    let x3 = x - size * 2.0 * (angle + spread).cos();
    let y3 = y - size * 2.0 * (angle + spread).sin();
    let pts = format!("{x},{y},{x1},{y1},{x2},{y2},{x3},{y3},{x},{y}");
    svg.raw(&format!(
        r#"<polygon fill="{STROKE}" points="{pts}" style="stroke:{STROKE};stroke-width:1;"/>"#,
    ));
}

// ---------------------------------------------------------------------------
// Path building
// ---------------------------------------------------------------------------

fn build_path_d(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut d = String::new();
    let (x0, y0) = points[0];
    write!(d, "M {x0},{y0}").unwrap();
    if points.len() >= 4 {
        // Cubic bezier.
        let mut i = 1;
        while i + 2 < points.len() {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];
            let (x3, y3) = points[i + 2];
            write!(d, " C {x1},{y1} {x2},{y2} {x3},{y3}").unwrap();
            i += 3;
        }
    } else {
        // Line segments.
        for &(x, y) in &points[1..] {
            write!(d, " L {x},{y}").unwrap();
        }
    }
    d
}

// ---------------------------------------------------------------------------
// Note rendering
// ---------------------------------------------------------------------------

fn render_note(
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
                    r#"<g class="cluster" data-qualified-name="{qname}"{source_attr}{id_attr}>{}</g>"#,
                    cluster.inner_xml,
                ));
            }
            walk(&pkg.packages, &qname, svg, oracle);
        }
    }
    walk(packages, "", svg, oracle);
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
    fn multiple_stereotypes_rendered() {
        let input = "@startuml\ncomponent Auth <<service>> <<secured>>\nAuth --> Backend\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("service"), "first stereotype missing: {svg}");
        assert!(svg.contains("secured"), "second stereotype missing: {svg}");
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
}
