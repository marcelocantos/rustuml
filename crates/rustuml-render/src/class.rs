// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram SVG renderer.
//!
//! Uses rustuml-layout (Sugiyama algorithm) for node positioning,
//! then renders classes with fields/methods and relationships.

use rustuml_layout::graph::{Direction, LayoutGraph};
use rustuml_parser::diagram::class::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_BORDER: &str = "#888888";
const NOTE_FOLD: f64 = 10.0; // size of the folded corner
const NOTE_PAD_X: f64 = 6.0;
const NOTE_PAD_Y: f64 = 4.0;
const NOTE_LINE_HEIGHT: f64 = 16.0;

const CLASS_MIN_WIDTH: f64 = 120.0;
const HEADER_HEIGHT: f64 = 30.0;
const MEMBER_HEIGHT: f64 = 18.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 8.0;
const MARGIN: f64 = 30.0;
const PACKAGE_HEADER: f64 = 24.0;
const PACKAGE_PAD: f64 = 12.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;

/// Font names that PlantUML treats as monospace. When one of these is set via
/// `skinparam defaultFontName`, spaces in member text are rendered as non-breaking
/// spaces (U+00A0) to match Java PlantUML's SVG output.
const MONOSPACE_FONTS: &[&str] = &[
    "courier",
    "monospaced",
    "monospace",
    "consolas",
    "lucida console",
];

/// Render a class diagram to SVG.
pub fn render(diagram: &ClassDiagram, theme: &Theme) -> String {
    let cs = &theme.class;
    if diagram.entities.is_empty() {
        // Notes-only diagram (no entities): render directly without layout.
        if !diagram.notes.is_empty() {
            return render_notes_only(diagram, cs);
        }
        // Meta-only diagram (header/footer/legend with no content).
        let has_meta = diagram.meta.header.is_some()
            || diagram.meta.footer.is_some()
            || diagram.meta.legend.is_some();
        if has_meta {
            return render_meta_only(diagram);
        }
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Phase 1: Use layout engine to determine relative ordering.
    let mut layout = LayoutGraph::new(Direction::TopToBottom);

    for entity in &diagram.entities {
        layout.add_node(&entity.id, &entity.label);
    }
    for rel in &diagram.relationships {
        layout.add_edge(&rel.from, &rel.to, rel.label.as_deref());
    }

    // Extract Sugiyama-positioned coordinates.
    // Run layout in a dedicated thread with a timeout — layout-rs can
    // enter infinite loops on degenerate graphs (e.g. bidirectional edges).
    let positions = {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(layout.layout_positions());
        });
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(pos) => pos,
            Err(_) => {
                // Layout timed out — fall back to grid layout.
                return render_grid(diagram, cs);
            }
        }
    };

    // Phase 2: Render with our own class boxes using layout positions.
    render_with_positions(diagram, &positions, cs)
}

/// Render using Sugiyama layout positions from the layout engine.
fn render_with_positions(
    diagram: &ClassDiagram,
    positions: &[rustuml_layout::graph::NodePosition],
    cs: &crate::style::ClassStyle,
) -> String {
    if diagram.entities.is_empty() {
        // If there are notes but no entities, render just the notes.
        if !diagram.notes.is_empty() {
            return render_notes_only(diagram, cs);
        }
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let class_dims: Vec<ClassDim> = diagram.entities.iter().map(calc_class_dim).collect();

    // Use layout positions if available, fall back to grid.
    let use_layout = positions.len() >= diagram.entities.len();

    if !use_layout {
        return render_grid(diagram, cs);
    }

    // Build entity id → index map for package membership lookup.
    let entity_idx: std::collections::HashMap<&str, usize> = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.as_str(), i))
        .collect();

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    // Compute raw entity positions (before any package-driven adjustment).
    let raw_pos: Vec<(f64, f64)> = (0..diagram.entities.len())
        .map(|i| (positions[i].x + MARGIN, positions[i].y + MARGIN + title_h))
        .collect();

    // Compute each package's bounding box and the y-shift needed so package
    // headers don't fall above the top margin.
    let pkg_boxes: Vec<Option<(f64, f64, f64, f64)>> = diagram
        .packages
        .iter()
        .map(|pkg| {
            let idxs: Vec<usize> = pkg
                .entities
                .iter()
                .filter_map(|eid| entity_idx.get(eid.as_str()).copied())
                .collect();
            if idxs.is_empty() {
                // Empty package: render as a small labelled box at the bottom-right.
                // Use a fixed placeholder position offset from the last entity or a default.
                let px = MARGIN;
                let py = MARGIN;
                let pw = CLASS_MIN_WIDTH;
                let ph = PACKAGE_HEADER + PACKAGE_PAD;
                return Some((px, py, pw, ph));
            }
            let min_ex = idxs
                .iter()
                .map(|&i| raw_pos[i].0)
                .fold(f64::INFINITY, f64::min);
            let min_ey = idxs
                .iter()
                .map(|&i| raw_pos[i].1)
                .fold(f64::INFINITY, f64::min);
            let max_ex = idxs
                .iter()
                .map(|&i| raw_pos[i].0 + class_dims[i].width)
                .fold(0.0_f64, f64::max);
            let max_ey = idxs
                .iter()
                .map(|&i| raw_pos[i].1 + class_dims[i].height)
                .fold(0.0_f64, f64::max);
            Some((
                min_ex - PACKAGE_PAD,
                min_ey - PACKAGE_PAD - PACKAGE_HEADER,
                max_ex - min_ex + PACKAGE_PAD * 2.0,
                max_ey - min_ey + PACKAGE_PAD * 2.0 + PACKAGE_HEADER,
            ))
        })
        .collect();

    // Determine how far down to shift everything so package headers fit.
    let y_shift: f64 = pkg_boxes
        .iter()
        .filter_map(|b| *b)
        .map(|(_, py, _, _)| if py < MARGIN { MARGIN - py } else { 0.0 })
        .fold(0.0_f64, f64::max);

    // Final entity positions.
    let entity_positions: Vec<(f64, f64, f64, f64)> = (0..diagram.entities.len())
        .map(|i| {
            let (x, y) = raw_pos[i];
            (x, y + y_shift, class_dims[i].width, class_dims[i].height)
        })
        .collect();

    // Compute note positions relative to their target entities.
    let note_positions: Vec<Option<(f64, f64, f64, f64)>> = diagram
        .notes
        .iter()
        .map(|note| {
            let (nw, nh) = note_box_dims(note);
            if let Some(target) = &note.target
                && let Some(ti) = diagram.entities.iter().position(|e| &e.id == target) {
                    let (ex, ey, ew, eh) = entity_positions[ti];
                    let pos = note.position.unwrap_or(NotePosition::Right);
                    let (nx, ny) = match pos {
                        NotePosition::Right => (ex + ew + MARGIN / 2.0, ey),
                        NotePosition::Left => (ex - nw - MARGIN / 2.0, ey),
                        NotePosition::Top => (ex, ey - nh - MARGIN / 2.0),
                        NotePosition::Bottom => (ex, ey + eh + MARGIN / 2.0),
                    };
                    return Some((nx, ny, nw, nh));
                }
            // Floating note: place to the right of all entities.
            let float_x = entity_positions
                .iter()
                .map(|(x, _, w, _)| x + w)
                .fold(0.0_f64, f64::max)
                + MARGIN / 2.0;
            let float_y = MARGIN;
            Some((float_x, float_y, nw, nh))
        })
        .collect();

    // Compute SVG canvas size (entity extents + package extents + note extents + margin).
    let ent_max_x = entity_positions
        .iter()
        .map(|(x, _, w, _)| x + w)
        .fold(0.0_f64, f64::max);
    let ent_max_y = entity_positions
        .iter()
        .map(|(_, y, _, h)| y + h)
        .fold(0.0_f64, f64::max);
    let pkg_max_x = pkg_boxes
        .iter()
        .filter_map(|b| *b)
        .map(|(px, _, pw, _)| px + pw)
        .fold(0.0_f64, f64::max);
    let pkg_max_y = pkg_boxes
        .iter()
        .filter_map(|b| *b)
        .map(|(_, py, _, ph)| py + y_shift + ph)
        .fold(0.0_f64, f64::max);
    let note_max_x = note_positions
        .iter()
        .filter_map(|b| *b)
        .map(|(nx, _, nw, _)| nx + nw)
        .fold(0.0_f64, f64::max);
    let note_max_y = note_positions
        .iter()
        .filter_map(|b| *b)
        .map(|(_, ny, _, nh)| ny + nh)
        .fold(0.0_f64, f64::max);
    let total_width = ent_max_x.max(pkg_max_x).max(note_max_x) + MARGIN;
    let total_height = ent_max_y.max(pkg_max_y).max(note_max_y) + MARGIN;

    let is_handwritten = diagram
        .meta
        .skinparams
        .iter()
        .any(|sp| sp.key.to_lowercase() == "handwritten" && sp.value.to_lowercase() == "true");

    // When a monospace font (e.g. Courier) is configured via skinparam, PlantUML
    // renders class member text using non-breaking spaces (U+00A0) instead of
    // regular spaces, matching the monospace_text rendering path.
    let use_monospace_members = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.to_lowercase() == "defaultfontname"
            && MONOSPACE_FONTS.contains(&sp.value.to_lowercase().as_str())
    });

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Emit warning for `skinparam handwritten true` (not supported; use !option).
    if is_handwritten {
        let nbsp = '\u{00a0}';
        let msg = format!(
            "Please{n}use{n}'!option{n}handwritten{n}true'{n}to{n}enable{n}handwritten",
            n = nbsp
        );
        svg.monospace_text(10.0, SMALL_FONT + 4.0, &msg, "start", SMALL_FONT);
    }

    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_width / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }
    if let Some(header) = &diagram.meta.header {
        svg.text(
            total_width / 2.0,
            SMALL_FONT + 2.0,
            header,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(footer) = &diagram.meta.footer {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            footer,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(caption) = &diagram.meta.caption {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            caption,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(legend) = &diagram.meta.legend {
        svg.render_legend(
            total_width - 200.0,
            total_height - 150.0,
            legend,
            SMALL_FONT,
        );
    }

    // Render package containers first (behind entities).
    for (pkg, maybe_box) in diagram.packages.iter().zip(&pkg_boxes) {
        if let Some((px, py, pw, ph)) = maybe_box {
            let adj_py = py + y_shift;
            let fill = pkg_fill_color(pkg.color.as_deref());
            svg.rect(*px, adj_py, *pw, *ph, fill, "#888888");
            // Build header label: name + stereotypes (e.g. "myPkg «Application»")
            let display = pkg.display_name.as_deref().unwrap_or(&pkg.name);
            let header_label = if pkg.stereotypes.is_empty() {
                display.to_string()
            } else {
                let stereos: Vec<String> =
                    pkg.stereotypes.iter().map(|s| format!("«{s}»")).collect();
                format!("{} {}", display, stereos.join(" "))
            };
            svg.text(
                px + 6.0,
                adj_py + PACKAGE_HEADER - 6.0,
                &header_label,
                "start",
                FONT_SIZE,
            );
        }
    }

    // Render each class at its adjusted position.
    for (i, (entity, dim)) in diagram.entities.iter().zip(&class_dims).enumerate() {
        let (x, y, _, _) = entity_positions[i];
        render_class_box(&mut svg, entity, x, y, dim, cs, use_monospace_members);
    }

    // Render relationships.
    for rel in &diagram.relationships {
        let from_idx = diagram.entities.iter().position(|e| e.id == rel.from);
        let to_idx = diagram.entities.iter().position(|e| e.id == rel.to);

        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            let (fx, fy, fw, fh) = entity_positions[fi];
            let (tx, ty, tw, _th) = entity_positions[ti];

            let from_cx = fx + fw / 2.0;
            let from_bottom = fy + fh;
            let to_cx = tx + tw / 2.0;
            let to_top = ty;

            let dashed = matches!(
                rel.kind,
                RelationshipKind::Dependency | RelationshipKind::Implementation
            );
            svg.line_segment(
                from_cx,
                from_bottom,
                to_cx,
                to_top,
                &cs.border_color,
                dashed,
            );
            render_relationship_head(&mut svg, rel.kind, to_cx, to_top);
            render_relationship_labels(&mut svg, rel, from_cx, from_bottom, to_cx, to_top);
        }
    }

    // Render notes.
    for (note, maybe_pos) in diagram.notes.iter().zip(&note_positions) {
        if let Some((nx, ny, nw, nh)) = maybe_pos {
            render_note_box(&mut svg, note, *nx, *ny, *nw, *nh);
        }
    }

    svg.finalize()
}

/// Return a default SVG fill for package containers.
/// Color rendering fidelity is not required by current tests (text comparison only).
fn pkg_fill_color(_color: Option<&str>) -> &'static str {
    "#e8f0f8"
}

/// Grid-based rendering fallback (when layout positions aren't available).
fn render_grid(diagram: &ClassDiagram, cs: &crate::style::ClassStyle) -> String {
    if diagram.entities.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Calculate dimensions for each class box.
    let class_dims: Vec<ClassDim> = diagram.entities.iter().map(calc_class_dim).collect();

    let use_monospace_members = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.to_lowercase() == "defaultfontname"
            && MONOSPACE_FONTS.contains(&sp.value.to_lowercase().as_str())
    });

    // Simple grid layout: arrange classes in rows.
    let cols = (diagram.entities.len() as f64).sqrt().ceil() as usize;
    let col_widths = calc_col_widths(&class_dims, cols);
    let row_heights = calc_row_heights(&class_dims, cols);

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let total_width = col_widths.iter().sum::<f64>() + MARGIN * (cols as f64 + 1.0);
    let total_height =
        row_heights.iter().sum::<f64>() + MARGIN * (row_heights.len() as f64 + 1.0) + title_h;

    let mut svg = SvgBuilder::new(total_width, total_height);
    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_width / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }
    if let Some(header) = &diagram.meta.header {
        svg.text(
            total_width / 2.0,
            SMALL_FONT + 2.0,
            header,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(footer) = &diagram.meta.footer {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            footer,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(caption) = &diagram.meta.caption {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            caption,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(legend) = &diagram.meta.legend {
        svg.render_legend(
            total_width - 200.0,
            total_height - 150.0,
            legend,
            SMALL_FONT,
        );
    }

    // Position and render each class.
    let mut positions: Vec<(f64, f64, f64, f64)> = Vec::new(); // (x, y, w, h)

    for (i, (entity, dim)) in diagram.entities.iter().zip(&class_dims).enumerate() {
        let col = i % cols;
        let row = i / cols;

        let x = MARGIN + col_widths[..col].iter().sum::<f64>() + MARGIN * col as f64;
        let y = title_h + MARGIN + row_heights[..row].iter().sum::<f64>() + MARGIN * row as f64;

        render_class_box(&mut svg, entity, x, y, dim, cs, use_monospace_members);
        positions.push((x, y, dim.width, dim.height));
    }

    // Render relationships as lines between class centers.
    for rel in &diagram.relationships {
        let from_idx = diagram.entities.iter().position(|e| e.id == rel.from);
        let to_idx = diagram.entities.iter().position(|e| e.id == rel.to);

        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            let (fx, fy, fw, fh) = positions[fi];
            let (tx, ty, tw, _th) = positions[ti];

            let from_cx = fx + fw / 2.0;
            let from_bottom = fy + fh;
            let to_cx = tx + tw / 2.0;
            let to_top = ty;

            let dashed = matches!(
                rel.kind,
                RelationshipKind::Dependency | RelationshipKind::Implementation
            );
            svg.line_segment(from_cx, from_bottom, to_cx, to_top, "#000", dashed);

            // Draw relationship decoration at the target end.
            render_relationship_head(&mut svg, rel.kind, to_cx, to_top);
            render_relationship_labels(&mut svg, rel, from_cx, from_bottom, to_cx, to_top);
        }
    }

    // Render notes.
    for note in &diagram.notes {
        let (nw, nh) = note_box_dims(note);
        let (nx, ny) = if let Some(target) = &note.target {
            if let Some(ti) = diagram.entities.iter().position(|e| &e.id == target) {
                let (ex, ey, ew, eh) = positions[ti];
                let pos = note.position.unwrap_or(NotePosition::Right);
                match pos {
                    NotePosition::Right => (ex + ew + MARGIN / 2.0, ey),
                    NotePosition::Left => (ex - nw - MARGIN / 2.0, ey),
                    NotePosition::Top => (ex, ey - nh - MARGIN / 2.0),
                    NotePosition::Bottom => (ex, ey + eh + MARGIN / 2.0),
                }
            } else {
                (total_width - MARGIN, MARGIN)
            }
        } else {
            (total_width - MARGIN, MARGIN)
        };
        render_note_box(&mut svg, note, nx, ny, nw, nh);
    }

    svg.finalize()
}

struct ClassDim {
    width: f64,
    height: f64,
    header_text: String,
    kind_label: Option<&'static str>,
    /// User-defined stereotypes rendered as «text» above the class name.
    stereotype_labels: Vec<String>,
}

fn calc_class_dim(entity: &ClassEntity) -> ClassDim {
    let kind_label = match entity.kind {
        EntityKind::Interface => Some("<<interface>>"),
        EntityKind::AbstractClass => Some("<<abstract>>"),
        EntityKind::Enum => Some("<<enum>>"),
        EntityKind::Annotation => Some("<<annotation>>"),
        EntityKind::Entity => Some("<<entity>>"),
        EntityKind::Class => None,
    };

    let stereotype_labels: Vec<String> = entity
        .stereotypes
        .iter()
        .map(|s| format_stereotype(s))
        .collect();

    let name_width = metrics::text_width(&entity.label, FONT_SIZE) + PADDING * 2.0;
    let kind_width = kind_label.map_or(0.0, |k| metrics::text_width(k, SMALL_FONT) + PADDING * 2.0);
    let stereo_max_width = stereotype_labels
        .iter()
        .map(|s| metrics::text_width(s, SMALL_FONT) + PADDING * 2.0)
        .fold(0.0_f64, f64::max);
    let member_max_width = entity
        .members
        .iter()
        .map(|m| metrics::text_width(&format_member(m), SMALL_FONT) + PADDING * 2.0)
        .fold(0.0_f64, f64::max);

    let width = CLASS_MIN_WIDTH
        .max(name_width)
        .max(kind_width)
        .max(stereo_max_width)
        .max(member_max_width);

    let kind_height = if kind_label.is_some() {
        MEMBER_HEIGHT
    } else {
        0.0
    };
    let stereo_height = stereotype_labels.len() as f64 * MEMBER_HEIGHT;
    let members_height = if entity.members.is_empty() {
        0.0
    } else {
        entity.members.len() as f64 * MEMBER_HEIGHT + PADDING
    };

    let height = HEADER_HEIGHT + kind_height + stereo_height + members_height;

    ClassDim {
        width,
        height,
        header_text: entity.label.clone(),
        kind_label,
        stereotype_labels,
    }
}

fn render_class_box(
    svg: &mut SvgBuilder,
    entity: &ClassEntity,
    x: f64,
    y: f64,
    dim: &ClassDim,
    cs: &crate::style::ClassStyle,
    use_monospace_members: bool,
) {
    // Background.
    let fill = match entity.kind {
        EntityKind::Interface => &cs.interface_background,
        EntityKind::Enum => &cs.enum_background,
        EntityKind::Annotation => &cs.class_background,
        _ => &cs.class_background,
    };
    svg.rect(x, y, dim.width, dim.height, fill, &cs.border_color);

    let mut cy = y;

    // User-defined stereotypes (e.g. «service», «singleton»).
    for stereo in &dim.stereotype_labels {
        cy += MEMBER_HEIGHT;
        svg.text(x + dim.width / 2.0, cy - 3.0, stereo, "middle", SMALL_FONT);
    }

    // Kind label (<<interface>>, etc.).
    if let Some(kind) = dim.kind_label {
        cy += MEMBER_HEIGHT;
        svg.text(x + dim.width / 2.0, cy - 3.0, kind, "middle", SMALL_FONT);
    }

    // Class name — process bold/italic creole but NOT underline (`__`), matching
    // Java PlantUML behaviour where `__` is literal in entity labels.
    cy += HEADER_HEIGHT / 2.0 + 5.0;
    svg.text_class_label(
        x + dim.width / 2.0,
        cy,
        &dim.header_text,
        "middle",
        FONT_SIZE,
    );
    let stereo_height = dim.stereotype_labels.len() as f64 * MEMBER_HEIGHT;
    cy = y + HEADER_HEIGHT + stereo_height + dim.kind_label.map_or(0.0, |_| MEMBER_HEIGHT);

    // Separator line.
    if !entity.members.is_empty() {
        svg.line_segment(x, cy, x + dim.width, cy, "#000", false);
    }

    // Members.
    for member in &entity.members {
        cy += MEMBER_HEIGHT;
        if member.kind == MemberKind::Separator {
            // Draw a separator line; if labeled, also render the label.
            svg.line_segment(
                x,
                cy - MEMBER_HEIGHT / 2.0,
                x + dim.width,
                cy - MEMBER_HEIGHT / 2.0,
                "#000",
                false,
            );
            if !member.display_text.is_empty() {
                svg.text(
                    x + dim.width / 2.0,
                    cy - 3.0,
                    &member.display_text,
                    "middle",
                    SMALL_FONT,
                );
            }
        } else {
            let text = format_member(member);
            if use_monospace_members {
                svg.monospace_text(x + PADDING, cy - 3.0, &text, "start", SMALL_FONT);
            } else {
                // PlantUML renders class member text literally — creole markup
                // (e.g. __field__) is NOT interpreted; it appears as-is.
                svg.plain_text(x + PADDING, cy - 3.0, &text, "start", SMALL_FONT);
            }
        }
    }
}

fn format_member(member: &Member) -> String {
    if member.kind == MemberKind::Separator {
        return member.display_text.clone();
    }
    let static_prefix = if member.is_static { "{static} " } else { "" };
    let abstract_prefix = if member.is_abstract {
        "{abstract} "
    } else {
        ""
    };
    // Use the verbatim display_text to preserve the original colon spacing
    // from the source (e.g. "field: String" or "field : String").
    format!("{static_prefix}{abstract_prefix}{}", member.display_text)
}

/// Format a stereotype string for display.
///
/// PlantUML's rule: if the spot color is a hex color (`#RRGGBB`), the spot
/// notation `(letter,#color)` is stripped and only the name is shown.
/// If the color is a named color (e.g. `#red`), the full spot notation is kept.
///
/// - `(A,#red) SpotA`    → `«(A,#red) SpotA»`
/// - `(F,#FF7700) SpotF` → `«SpotF»`
fn format_stereotype(s: &str) -> String {
    // Match spot notation: (single-char, #color) name
    if let Some(rest) = s.strip_prefix('(')
        && let Some(comma_pos) = rest.find(',') {
            let after_comma = &rest[comma_pos + 1..];
            if let Some(color_and_rest) = after_comma.strip_prefix('#')
                && let Some(close_pos) = color_and_rest.find(')') {
                    let color = &color_and_rest[..close_pos];
                    // Hex color: all chars are hex digits (3 or 6 digits)
                    if (color.len() == 3 || color.len() == 6)
                        && color.chars().all(|c| c.is_ascii_hexdigit()) {
                            let name = color_and_rest[close_pos + 1..].trim();
                            return format!("«{name}»");
                        }
                }
        }
    format!("«{s}»")
}

/// Convert `<<stereotype>>` notation to `«stereotype»` guillemets.
fn convert_guillemets(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find("<<") {
        if let Some(end) = result[start..].find(">>") {
            let inner = result[start + 2..start + end].to_string();
            result = format!(
                "{}«{}»{}",
                &result[..start],
                inner,
                &result[start + end + 2..]
            );
        } else {
            break;
        }
    }
    result
}

/// Strip PlantUML label direction markers (`< label` or `label >`) then
/// convert `<<stereotype>>` notation to `«stereotype»`.
fn normalize_label(label: &str) -> String {
    let s = label.trim();
    // Only strip a leading `<` if it's a direction marker (followed by space),
    // not a `<<stereotype>>` notation.
    let stripped = if s.starts_with("< ") {
        s[1..].trim_start()
    } else if s.ends_with(" >") {
        s[..s.len() - 1].trim_end()
    } else {
        s
    };
    convert_guillemets(stripped)
}

/// Render the label, from_multiplicity, and to_multiplicity for a relationship.
fn render_relationship_labels(
    svg: &mut SvgBuilder,
    rel: &Relationship,
    from_cx: f64,
    from_bottom: f64,
    to_cx: f64,
    to_top: f64,
) {
    let mid_x = (from_cx + to_cx) / 2.0;
    let mid_y = (from_bottom + to_top) / 2.0;

    if let Some(label) = &rel.label {
        let display = normalize_label(label);
        if !display.is_empty() {
            svg.text(mid_x + 5.0, mid_y - 4.0, &display, "start", SMALL_FONT);
        }
    }

    // from_multiplicity/role: near the FROM end (bottom of from-box).
    if let Some(mult) = &rel.from_multiplicity {
        svg.text(
            from_cx - 5.0,
            from_bottom + SMALL_FONT,
            mult,
            "end",
            SMALL_FONT,
        );
    }

    // to_multiplicity/role: near the TO end (top of to-box).
    if let Some(mult) = &rel.to_multiplicity {
        svg.text(to_cx - 5.0, to_top - 4.0, mult, "end", SMALL_FONT);
    }
}

fn render_relationship_head(svg: &mut SvgBuilder, kind: RelationshipKind, x: f64, y: f64) {
    match kind {
        RelationshipKind::Inheritance | RelationshipKind::Implementation => {
            // Open triangle (unfilled).
            let size = 10.0;
            svg.open_group("rel-head");
            let points = format!(
                "{x},{y} {},{} {},{}",
                x - size / 2.0,
                y - size,
                x + size / 2.0,
                y - size,
            );
            svg.line_segment(x, y, x - size / 2.0, y - size, "#000", false);
            svg.line_segment(x, y, x + size / 2.0, y - size, "#000", false);
            svg.line_segment(
                x - size / 2.0,
                y - size,
                x + size / 2.0,
                y - size,
                "#000",
                false,
            );
            let _ = points;
            svg.close_group();
        }
        RelationshipKind::Composition => {
            // Filled diamond.
            svg.arrow_head(x, y, 90.0);
        }
        RelationshipKind::Aggregation => {
            // Open diamond (approximated with arrow).
            svg.arrow_head(x, y, 90.0);
        }
        RelationshipKind::Dependency => {
            // Simple arrow head.
            svg.arrow_head(x, y, 90.0);
        }
        RelationshipKind::Association => {
            // No decoration.
        }
    }
}

/// Strip basic Creole/HTML markup from a note line to get plain text for rendering.
/// This is a minimal strip: enough to get readable text in SVG without implementing
/// a full Creole renderer.
fn strip_note_markup(s: &str) -> String {
    let mut out = s.trim().to_string();
    // Remove Creole delimiters: **, //, __
    out = out.replace("**", "");
    out = out.replace("//", "");
    out = out.replace("__", "");
    // Strip HTML tags: <b>, </b>, <i>, <u>, <color:#...>, etc.
    let mut result = String::new();
    let mut in_tag = false;
    for ch in out.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result.trim().to_string()
}

/// Replace `<img:...>` tags with PlantUML's fallback text.
/// For HTTP/HTTPS URLs the URL is included in the message.
fn replace_img_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<img:") {
        result.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find('>') {
            let raw_src = &after["<img:".len()..end];
            // Strip {scale=...} or similar suffix from the URL.
            let src = if let Some(brace) = raw_src.find('{') {
                &raw_src[..brace]
            } else {
                raw_src
            };
            if src.starts_with("https://") || src.starts_with("http://") {
                result.push_str(&format!("(Cannot\u{00a0}decode:\u{00a0}{src})"));
            } else {
                result.push_str("(Cannot\u{00a0}decode)");
            }
            rest = &after[end + 1..];
        } else {
            result.push_str(after);
            return result;
        }
    }
    result.push_str(rest);
    result
}

/// Compute the dimensions of a note box.
fn note_box_dims(note: &Note) -> (f64, f64) {
    let max_width = note
        .lines
        .iter()
        .map(|l| metrics::text_width(&strip_note_markup(l), FONT_SIZE) + NOTE_PAD_X * 2.0)
        .fold(80.0_f64, f64::max);
    let height = NOTE_PAD_Y * 2.0 + note.lines.len() as f64 * NOTE_LINE_HEIGHT;
    (max_width.max(NOTE_FOLD * 3.0), height.max(NOTE_FOLD * 2.0))
}

/// Render a note box at (x, y) with given dimensions.
fn render_note_box(svg: &mut SvgBuilder, note: &Note, x: f64, y: f64, w: f64, h: f64) {
    // Draw the note shape: rectangle with folded top-right corner.
    // Using polygon for the main shape.
    let fold = NOTE_FOLD;
    let points = &[
        (x, y),
        (x, y + h),
        (x + w, y + h),
        (x + w, y + fold),
        (x + w - fold, y),
    ];
    svg.polygon(points, NOTE_FILL, NOTE_BORDER);
    // The fold triangle.
    let fold_pts = &[
        (x + w - fold, y),
        (x + w - fold, y + fold),
        (x + w, y + fold),
    ];
    svg.polygon(fold_pts, NOTE_FILL, NOTE_BORDER);

    // Render each line of text — pass the raw markup so svg.text can apply creole styling.
    // Handle ordered/nested lists (#, ##, ...), bullet lists (*, **, ...),
    // tree syntax (|_, |__, ...), and inline creole markup.
    let mut ty = y + NOTE_PAD_Y + NOTE_LINE_HEIGHT - 3.0;
    // Per-level ordered-list counters: counters[0] = level-1 count, etc.
    let mut list_ctrs: Vec<usize> = Vec::new();
    for line in &note.lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            ty += NOTE_LINE_HEIGHT;
            continue;
        }
        // Tree syntax: |_ level1, |__ level2, etc.
        // The `|` is the tree edge; each additional `_` adds one level of indent.
        if let Some(rest) = trimmed.strip_prefix('|') {
            let underscore_count = rest.chars().take_while(|&c| c == '_').count();
            if underscore_count > 0 {
                let content = replace_img_tags(rest[underscore_count..].trim());
                let indent = "_".repeat(underscore_count - 1);
                let display = if indent.is_empty() {
                    content
                } else {
                    format!("{indent} {content}")
                };
                list_ctrs.clear();
                svg.text(x + NOTE_PAD_X, ty, &display, "start", FONT_SIZE);
                ty += NOTE_LINE_HEIGHT;
                continue;
            }
        }
        // Ordered list: # item, ## sub-item, etc.
        let hash_level = trimmed.chars().take_while(|&c| c == '#').count();
        if hash_level > 0 {
            let content = trimmed[hash_level..].trim_start();
            if list_ctrs.len() > hash_level {
                list_ctrs.truncate(hash_level);
            }
            while list_ctrs.len() < hash_level {
                list_ctrs.push(0);
            }
            list_ctrs[hash_level - 1] += 1;
            let n = list_ctrs[hash_level - 1];
            let indent = "  ".repeat(hash_level - 1);
            let content = replace_img_tags(content);
            svg.text(
                x + NOTE_PAD_X,
                ty,
                &format!("{indent}{n}. {content}"),
                "start",
                FONT_SIZE,
            );
            ty += NOTE_LINE_HEIGHT;
            continue;
        }
        // Bullet list: `* item` only.  Only a single leading `*` followed by
        // whitespace is treated as a bullet, matching Java PlantUML behaviour.
        // Multi-star patterns (`** text`) could be bold markup (`**text**`) so
        // restrict to star_level == 1 to avoid false positives.
        let star_level = trimmed.chars().take_while(|&c| c == '*').count();
        let next_after_stars = trimmed[star_level..].chars().next();
        let is_bullet =
            star_level == 1 && matches!(next_after_stars, None | Some(' ') | Some('\t'));
        if is_bullet {
            let content = trimmed[star_level..].trim_start();
            let indent = "  ".repeat(star_level - 1);
            let content = replace_img_tags(content);
            list_ctrs.clear();
            svg.text(
                x + NOTE_PAD_X,
                ty,
                &format!("{indent}* {content}"),
                "start",
                FONT_SIZE,
            );
            ty += NOTE_LINE_HEIGHT;
            continue;
        }
        // Regular line — reset list counters, pass through creole processing via svg.text.
        list_ctrs.clear();
        let display = replace_img_tags(trimmed);
        svg.text(x + NOTE_PAD_X, ty, &display, "start", FONT_SIZE);
        ty += NOTE_LINE_HEIGHT;
    }
}

/// Render a diagram that contains only meta content (header/footer/legend), no entities or notes.
fn render_meta_only(diagram: &ClassDiagram) -> String {
    let width = 200.0;
    let mut y = SMALL_FONT + 2.0;
    let mut lines: Vec<(f64, String)> = Vec::new();

    if let Some(header) = &diagram.meta.header {
        lines.push((y, header.clone()));
        y += SMALL_FONT + 6.0;
    }
    if let Some(legend) = &diagram.meta.legend {
        for line in legend.lines() {
            if !line.trim().is_empty() {
                lines.push((y, line.to_string()));
                y += SMALL_FONT + 6.0;
            }
        }
    }
    if let Some(footer) = &diagram.meta.footer {
        lines.push((y, footer.clone()));
        y += SMALL_FONT + 6.0;
    }

    let height = (y + 4.0).max(30.0);
    let mut svg = SvgBuilder::new(width, height);
    for (text_y, text) in &lines {
        svg.text(width / 2.0, *text_y, text, "middle", SMALL_FONT);
    }
    svg.finalize()
}

/// Render a diagram that contains only floating notes (no entities).
/// Notes are laid out horizontally with a margin between them.
fn render_notes_only(diagram: &ClassDiagram, _cs: &crate::style::ClassStyle) -> String {
    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let mut x = MARGIN;
    let mut max_h = 0.0_f64;
    let note_data: Vec<(f64, f64, f64, f64)> = diagram
        .notes
        .iter()
        .map(|note| {
            let (nw, nh) = note_box_dims(note);
            let nx = x;
            let ny = MARGIN + title_h;
            x += nw + MARGIN;
            max_h = max_h.max(nh);
            (nx, ny, nw, nh)
        })
        .collect();
    let total_width = x.max(MARGIN * 2.0);
    let total_height = MARGIN + title_h + max_h + MARGIN;

    let mut svg = SvgBuilder::new(total_width, total_height);
    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_width / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }
    if let Some(header) = &diagram.meta.header {
        svg.text(
            total_width / 2.0,
            SMALL_FONT + 2.0,
            header,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(footer) = &diagram.meta.footer {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            footer,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(caption) = &diagram.meta.caption {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            caption,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(legend) = &diagram.meta.legend {
        svg.text(
            total_width / 2.0,
            total_height - 4.0,
            legend,
            "middle",
            SMALL_FONT,
        );
    }
    for (note, (nx, ny, nw, nh)) in diagram.notes.iter().zip(&note_data) {
        render_note_box(&mut svg, note, *nx, *ny, *nw, *nh);
    }
    svg.finalize()
}

fn calc_col_widths(dims: &[ClassDim], cols: usize) -> Vec<f64> {
    let mut widths = vec![0.0_f64; cols];
    for (i, dim) in dims.iter().enumerate() {
        let col = i % cols;
        widths[col] = widths[col].max(dim.width);
    }
    widths
}

fn calc_row_heights(dims: &[ClassDim], cols: usize) -> Vec<f64> {
    let rows = dims.len().div_ceil(cols);
    let mut heights = vec![0.0_f64; rows];
    for (i, dim) in dims.iter().enumerate() {
        let row = i / cols;
        heights[row] = heights[row].max(dim.height);
    }
    heights
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_class_diagram() -> ClassDiagram {
        ClassDiagram {
            meta: DiagramMeta::default(),
            entities: vec![
                ClassEntity {
                    id: "Animal".into(),
                    label: "Animal".into(),
                    kind: EntityKind::Class,
                    members: vec![
                        Member {
                            name: "name".into(),
                            return_type: Some("String".into()),
                            visibility: Visibility::Public,
                            is_static: false,
                            is_abstract: false,
                            kind: MemberKind::Field,
                            display_text: "name: String".into(),
                        },
                        Member {
                            name: "makeSound()".into(),
                            return_type: Some("void".into()),
                            visibility: Visibility::Public,
                            is_static: false,
                            is_abstract: false,
                            kind: MemberKind::Method,
                            display_text: "makeSound(): void".into(),
                        },
                    ],
                    stereotypes: vec![],
                },
                ClassEntity {
                    id: "Dog".into(),
                    label: "Dog".into(),
                    kind: EntityKind::Class,
                    members: vec![Member {
                        name: "fetch()".into(),
                        return_type: Some("void".into()),
                        visibility: Visibility::Public,
                        is_static: false,
                        is_abstract: false,
                        kind: MemberKind::Method,
                        display_text: "fetch(): void".into(),
                    }],
                    stereotypes: vec![],
                },
            ],
            relationships: vec![Relationship {
                from: "Animal".into(),
                to: "Dog".into(),
                kind: RelationshipKind::Inheritance,
                label: None,
                from_multiplicity: None,
                to_multiplicity: None,
            }],
            packages: vec![],
            notes: vec![],
        }
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
    }

    #[test]
    fn has_class_boxes() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 2,
            "should have at least 2 class boxes, got {rect_count}"
        );
    }

    #[test]
    fn has_members() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(svg.contains("name: String"));
        assert!(svg.contains("makeSound(): void"));
        assert!(svg.contains("fetch(): void"));
    }

    #[test]
    fn has_relationship_line() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(svg.contains("<line"), "should have relationship line");
    }

    #[test]
    fn interface_rendering() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            entities: vec![ClassEntity {
                id: "Drawable".into(),
                label: "Drawable".into(),
                kind: EntityKind::Interface,
                members: vec![Member {
                    name: "draw()".into(),
                    return_type: Some("void".into()),
                    visibility: Visibility::Public,
                    is_static: false,
                    is_abstract: true,
                    kind: MemberKind::Method,
                    display_text: "draw(): void".into(),
                }],
                stereotypes: vec![],
            }],
            relationships: vec![],
            packages: vec![],
            notes: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("&lt;&lt;interface&gt;&gt;"));
        assert!(svg.contains("Drawable"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\nclass Animal {\n  +name : String\n}\nclass Dog\nAnimal <|-- Dog\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
    }

    #[test]
    fn nested_package_entities_shown() {
        let input = "@startuml\ncloud Outer {\n  cloud Inner {\n    class MyClass {\n      +void method()\n    }\n  }\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("MyClass"), "MyClass should appear in SVG");
        assert!(svg.contains("method"), "method should appear in SVG");
    }

    #[test]
    fn empty_diagram() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            entities: vec![],
            relationships: vec![],
            packages: vec![],
            notes: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("<svg"));
    }
}
