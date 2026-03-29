// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram SVG renderer.

use rustuml_parser::diagram::component::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const COMPONENT_MIN_W: f64 = 100.0;
const COMPONENT_H: f64 = 40.0;
const MARGIN: f64 = 30.0;
const GAP: f64 = 40.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const PADDING: f64 = 12.0;
const CONTAINER_PADDING: f64 = 16.0;
const LABEL_H: f64 = 22.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
const IFACE_R: f64 = 8.0;
const IFACE_H: f64 = IFACE_R * 2.0 + 20.0; // circle + label below
const NOTE_FONT: f64 = 13.0;
const NOTE_PAD: f64 = 8.0;
const NOTE_LINE_H: f64 = 18.0;
const NOTE_GAP: f64 = 10.0;
const NOTE_FOLD: f64 = 10.0;

pub fn render(diagram: &ComponentDiagram, theme: &Theme) -> String {
    if diagram.components.is_empty() && diagram.packages.is_empty() && diagram.interfaces.is_empty()
    {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n = diagram.components.len();
    let cols = if n == 0 {
        1
    } else {
        (n as f64).sqrt().ceil() as usize
    };

    let widths: Vec<f64> = diagram
        .components
        .iter()
        .map(|c| {
            let label_w = metrics::text_width(&c.label, FONT_SIZE) + PADDING * 2.0;
            let stereo_w = c
                .stereotypes
                .first()
                .map(|s| metrics::text_width(&format!("«{s}»"), SMALL_FONT) + PADDING * 2.0)
                .unwrap_or(0.0);
            label_w.max(stereo_w).max(COMPONENT_MIN_W)
        })
        .collect();

    let col_w: Vec<f64> = {
        let mut cw = vec![0.0_f64; cols];
        for (i, w) in widths.iter().enumerate() {
            cw[i % cols] = cw[i % cols].max(*w);
        }
        cw
    };
    let rows = if n == 0 { 0 } else { n.div_ceil(cols) };
    let comp_total_w = if n > 0 {
        MARGIN * 2.0 + col_w.iter().sum::<f64>() + GAP * (cols.max(1) - 1) as f64
    } else {
        0.0
    };
    let comp_total_h = if n > 0 {
        MARGIN * 2.0 + rows as f64 * (COMPONENT_H + GAP)
    } else {
        0.0
    };

    // Estimate space needed for interfaces (rendered below components).
    let iface_count = diagram.interfaces.len();
    let iface_total_h = if iface_count > 0 { IFACE_H + GAP } else { 0.0 };
    let iface_total_w = if iface_count > 0 {
        MARGIN * 2.0 + iface_count as f64 * (IFACE_R * 2.0 + GAP)
    } else {
        0.0
    };

    // Compute a bounding box that also covers all packages.
    let pkg_total_w = estimate_packages_width(&diagram.packages);
    let pkg_total_h = estimate_packages_height(&diagram.packages);

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let total_w = comp_total_w.max(pkg_total_w).max(iface_total_w).max(100.0);
    let total_h = (comp_total_h + pkg_total_h + iface_total_h + title_h).max(50.0);

    let mut svg = SvgBuilder::new(total_w, total_h);
    if let Some(title) = &diagram.meta.title {
        // Multi-line title: render each line.
        for (i, tline) in title.lines().enumerate() {
            let ty = TITLE_HEIGHT - 4.0 + i as f64 * (TITLE_FONT_SIZE + 2.0);
            svg.text(total_w / 2.0, ty, tline, "middle", TITLE_FONT_SIZE);
        }
    }
    let cs = &theme.class;

    // Render packages (containers) starting after title.
    let mut pkg_y = title_h + MARGIN;
    render_packages(&diagram.packages, &mut svg, MARGIN, &mut pkg_y, theme);

    // Render flat components below the packages.
    let y_start = pkg_y
        + if !diagram.packages.is_empty() {
            GAP
        } else {
            0.0
        };
    let mut positions = Vec::new();
    for (i, comp) in diagram.components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (COMPONENT_H + GAP);
        let w = col_w[col];

        if let Some(ref url) = comp.url {
            svg.open_link(url);
        }
        svg.rect(x, y, w, COMPONENT_H, &cs.class_background, &cs.border_color);
        svg.rect(
            x - 5.0,
            y + 8.0,
            10.0,
            6.0,
            &cs.class_background,
            &cs.border_color,
        );
        svg.rect(
            x - 5.0,
            y + 20.0,
            10.0,
            6.0,
            &cs.class_background,
            &cs.border_color,
        );
        // Render stereotypes (all of them, stacked).
        let stereo_lines: Vec<&str> = comp.stereotypes.iter().map(|s| s.as_str()).collect();
        if !stereo_lines.is_empty() {
            let total_stereo_h = stereo_lines.len() as f64 * (SMALL_FONT + 2.0);
            let label_offset = total_stereo_h + 4.0;
            for (si, stereo) in stereo_lines.iter().enumerate() {
                svg.text(
                    x + w / 2.0,
                    y + COMPONENT_H / 2.0 - label_offset / 2.0 + si as f64 * (SMALL_FONT + 2.0),
                    &format!("«{stereo}»"),
                    "middle",
                    SMALL_FONT,
                );
            }
            svg.text(
                x + w / 2.0,
                y + COMPONENT_H / 2.0 + label_offset / 2.0 + 4.0,
                &comp.label,
                "middle",
                FONT_SIZE,
            );
        } else {
            svg.text(
                x + w / 2.0,
                y + COMPONENT_H / 2.0 + 5.0,
                &comp.label,
                "middle",
                FONT_SIZE,
            );
        }
        if comp.url.is_some() {
            svg.close_link();
        }
        positions.push((x, y, w));
    }

    // Render interfaces below components.
    let iface_y_start = if n > 0 {
        y_start + rows as f64 * (COMPONENT_H + GAP)
    } else {
        y_start
    };
    let mut iface_positions: Vec<(f64, f64)> = Vec::new();
    for (ii, iface) in diagram.interfaces.iter().enumerate() {
        let ix = MARGIN + ii as f64 * (IFACE_R * 2.0 + GAP) + IFACE_R;
        let iy = iface_y_start + IFACE_R;
        // Draw circle.
        svg.circle(ix, iy, IFACE_R, &cs.class_background, &cs.border_color);
        // Label below circle.
        svg.text(
            ix,
            iy + IFACE_R + NOTE_LINE_H,
            &iface.label,
            "middle",
            FONT_SIZE,
        );
        iface_positions.push((ix, iy));
    }

    // Render connections.
    for conn in &diagram.connections {
        let fi = diagram.components.iter().position(|c| c.id == conn.from);
        let ti = diagram.components.iter().position(|c| c.id == conn.to);
        // Also check interfaces as source/target.
        let fi_iface = diagram.interfaces.iter().position(|i| i.id == conn.from);
        let ti_iface = diagram.interfaces.iter().position(|i| i.id == conn.to);

        let (from_cx, from_cy) = if let Some(fi) = fi {
            let (fx, fy, fw) = positions[fi];
            (fx + fw / 2.0, fy + COMPONENT_H)
        } else if let Some(fi) = fi_iface {
            let (ix, iy) = iface_positions[fi];
            (ix, iy)
        } else {
            continue;
        };

        let (to_cx, to_cy) = if let Some(ti) = ti {
            let (tx, ty, tw) = positions[ti];
            (tx + tw / 2.0, ty)
        } else if let Some(ti) = ti_iface {
            let (ix, iy) = iface_positions[ti];
            (ix, iy)
        } else {
            continue;
        };

        svg.line_segment(
            from_cx,
            from_cy,
            to_cx,
            to_cy,
            &cs.border_color,
            conn.dashed,
        );
        // Only draw arrowhead if connecting to a component (not an interface circle).
        if ti.is_some() {
            svg.arrow_head(to_cx, to_cy, 90.0);
        }
        if let Some(label) = &conn.label {
            let mx = (from_cx + to_cx) / 2.0;
            let my = (from_cy + to_cy) / 2.0;
            svg.text(mx + 6.0, my - 4.0, label, "start", SMALL_FONT);
        }
        if let Some(from_mult) = &conn.from_mult {
            let mx = from_cx + 6.0;
            let my = from_cy + 14.0;
            svg.text(mx, my, from_mult, "start", SMALL_FONT);
        }
        if let Some(to_mult) = &conn.to_mult {
            let mx = to_cx + 6.0;
            let my = to_cy - 4.0;
            svg.text(mx, my, to_mult, "start", SMALL_FONT);
        }
    }

    // Render notes.
    for note in &diagram.notes {
        render_note(
            note,
            &positions,
            &diagram.components,
            &mut svg,
            total_w,
            total_h,
        );
    }

    // Render header/footer/legend from diagram meta.
    if let Some(header) = &diagram.meta.header {
        svg.text(
            total_w / 2.0,
            SMALL_FONT + 2.0,
            header,
            "middle",
            SMALL_FONT,
        );
    }
    if let Some(footer) = &diagram.meta.footer {
        svg.text(total_w / 2.0, total_h - 4.0, footer, "middle", SMALL_FONT);
    }
    if let Some(legend) = &diagram.meta.legend {
        svg.render_legend(MARGIN, total_h / 2.0, legend, SMALL_FONT);
    }

    svg.finalize()
}

/// Render a single note as a sticky shape.
fn render_note(
    note: &ComponentNote,
    positions: &[(f64, f64, f64)],
    components: &[Component],
    svg: &mut SvgBuilder,
    canvas_w: f64,
    canvas_h: f64,
) {
    let lines: Vec<&str> = note.text.lines().collect();
    let note_w = lines
        .iter()
        .map(|l| metrics::text_width(l, NOTE_FONT) + NOTE_PAD * 2.0)
        .fold(60.0_f64, f64::max);
    let note_h = (lines.len() as f64).max(1.0) * NOTE_LINE_H + NOTE_PAD * 2.0;

    // Determine placement based on target.
    let (nx, ny) = if let Some(target) = &note.target {
        if let Some(idx) = components.iter().position(|c| c.id == *target) {
            let (ox, oy, ow) = positions[idx];
            // Place to the right of the component.
            let nx = ox + ow + NOTE_GAP;
            let ny = oy + COMPONENT_H / 2.0 - note_h / 2.0;
            (nx, ny)
        } else {
            // Target not found — place at right edge.
            let nx = canvas_w - note_w - NOTE_GAP * 2.0;
            let ny = canvas_h / 2.0 - note_h / 2.0;
            (nx, ny)
        }
    } else {
        // Floating note.
        let nx = NOTE_GAP;
        let ny = canvas_h - note_h - NOTE_GAP;
        (nx, ny)
    };

    let nx = nx.max(NOTE_GAP);
    let ny = ny.max(NOTE_GAP);

    // Note body (rectangle with dog-ear).
    svg.note_box(nx, ny, note_w, note_h, NOTE_FOLD, "#FEFFDD", "#181818");

    for (i, line) in lines.iter().enumerate() {
        let ty = ny + NOTE_PAD + (i as f64 + 1.0) * NOTE_LINE_H - 2.0;
        svg.text(nx + NOTE_PAD, ty, line, "start", NOTE_FONT);
    }
}

/// Render a list of packages starting at (x, *y). Updates *y to point past
/// the last package rendered.
fn render_packages(
    packages: &[ComponentPackage],
    svg: &mut SvgBuilder,
    x: f64,
    y: &mut f64,
    theme: &Theme,
) {
    let cs = &theme.class;
    for pkg in packages {
        let name_w = metrics::text_width(&pkg.label, FONT_SIZE) + PADDING * 2.0;
        let stereo_w = pkg
            .stereotype
            .as_deref()
            .map(|s| metrics::text_width(&format!("«{s}»"), SMALL_FONT) + PADDING * 2.0)
            .unwrap_or(0.0);
        let pkg_label_w = name_w.max(stereo_w).max(COMPONENT_MIN_W);
        let inner_w = estimate_package_inner_width(pkg).max(pkg_label_w);
        let pkg_w = inner_w + CONTAINER_PADDING * 2.0;

        let pkg_y_start = *y;
        // Draw label at the top.
        let label_y = pkg_y_start + LABEL_H;
        *y = label_y + CONTAINER_PADDING;

        // Render nested packages.
        if !pkg.packages.is_empty() {
            render_packages(&pkg.packages, svg, x + CONTAINER_PADDING, y, theme);
        }

        // Advance y for any leaf components in this package.
        let leaf_count = pkg.components.len();
        if leaf_count > 0 {
            *y += leaf_count as f64 * (COMPONENT_H + GAP);
        }

        // Clamp inner height to at least one component height.
        let pkg_inner_h = (*y - label_y - CONTAINER_PADDING).max(COMPONENT_H);
        let pkg_h = pkg_inner_h + CONTAINER_PADDING * 2.0 + LABEL_H;

        // Draw container box.
        svg.rect(
            x,
            pkg_y_start,
            pkg_w,
            pkg_h,
            &cs.class_background,
            &cs.border_color,
        );
        // Draw label.
        svg.text(
            x + CONTAINER_PADDING,
            pkg_y_start + LABEL_H - 4.0,
            &pkg.label,
            "start",
            FONT_SIZE,
        );
        // Draw stereotype below label if present.
        if let Some(stereo) = &pkg.stereotype {
            svg.text(
                x + CONTAINER_PADDING,
                pkg_y_start + LABEL_H + 9.0,
                &format!("«{stereo}»"),
                "start",
                SMALL_FONT,
            );
        }

        *y = pkg_y_start + pkg_h + GAP;
    }
}

/// Estimate total width needed for a list of packages side-by-side (simplified: stacked).
fn estimate_packages_width(packages: &[ComponentPackage]) -> f64 {
    packages
        .iter()
        .map(estimate_package_width)
        .fold(0.0_f64, f64::max)
        + MARGIN * 2.0
}

fn estimate_packages_height(packages: &[ComponentPackage]) -> f64 {
    packages.iter().map(estimate_package_height).sum::<f64>()
        + MARGIN * 2.0
        + GAP * packages.len().saturating_sub(1) as f64
}

fn estimate_package_width(pkg: &ComponentPackage) -> f64 {
    let name_w = metrics::text_width(&pkg.label, FONT_SIZE) + PADDING * 2.0;
    let stereo_w = pkg
        .stereotype
        .as_deref()
        .map(|s| metrics::text_width(&format!("«{s}»"), SMALL_FONT) + PADDING * 2.0)
        .unwrap_or(0.0);
    let label_w = name_w.max(stereo_w).max(COMPONENT_MIN_W);
    let inner_w = estimate_package_inner_width(pkg);
    (label_w.max(inner_w) + CONTAINER_PADDING * 2.0).max(COMPONENT_MIN_W)
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
    LABEL_H + CONTAINER_PADDING * 2.0 + nested_h + leaf_h
}

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
}
