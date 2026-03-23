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

pub fn render(diagram: &ComponentDiagram, theme: &Theme) -> String {
    if diagram.components.is_empty() && diagram.packages.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n = diagram.components.len();
    let cols = if n == 0 { 1 } else { (n as f64).sqrt().ceil() as usize };

    let widths: Vec<f64> = diagram
        .components
        .iter()
        .map(|c| {
            let label_w = metrics::text_width(&c.label, FONT_SIZE) + PADDING * 2.0;
            let stereo_w = c
                .stereotype
                .as_deref()
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

    // Compute a bounding box that also covers all packages.
    let pkg_total_w = estimate_packages_width(&diagram.packages);
    let pkg_total_h = estimate_packages_height(&diagram.packages);

    let total_w = comp_total_w.max(pkg_total_w).max(100.0);
    let total_h = (comp_total_h + pkg_total_h).max(50.0);

    let mut svg = SvgBuilder::new(total_w, total_h);
    let cs = &theme.class;

    // Render packages (containers) starting at y_offset=0.
    let mut pkg_y = MARGIN;
    render_packages(&diagram.packages, &mut svg, MARGIN, &mut pkg_y, theme);

    // Render flat components below the packages.
    let y_start = pkg_y + if pkg_y > MARGIN { GAP } else { 0.0 };
    let mut positions = Vec::new();
    for (i, comp) in diagram.components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MARGIN + col_w[..col].iter().sum::<f64>() + GAP * col as f64;
        let y = y_start + row as f64 * (COMPONENT_H + GAP);
        let w = col_w[col];

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
        if let Some(stereo) = &comp.stereotype {
            svg.text(
                x + w / 2.0,
                y + COMPONENT_H / 2.0 - 4.0,
                &format!("«{stereo}»"),
                "middle",
                SMALL_FONT,
            );
            svg.text(
                x + w / 2.0,
                y + COMPONENT_H / 2.0 + 10.0,
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
        positions.push((x, y, w));
    }

    for conn in &diagram.connections {
        let fi = diagram.components.iter().position(|c| c.id == conn.from);
        let ti = diagram.components.iter().position(|c| c.id == conn.to);
        if let (Some(fi), Some(ti)) = (fi, ti) {
            let (fx, fy, fw) = positions[fi];
            let (tx, ty, tw) = positions[ti];
            svg.line_segment(
                fx + fw / 2.0,
                fy + COMPONENT_H,
                tx + tw / 2.0,
                ty,
                &cs.border_color,
                conn.dashed,
            );
            svg.arrow_head(tx + tw / 2.0, ty, 90.0);
            if let Some(label) = &conn.label {
                let mx = (fx + fw / 2.0 + tx + tw / 2.0) / 2.0;
                let my = (fy + COMPONENT_H + ty) / 2.0;
                svg.text(mx, my - 4.0, label, "middle", SMALL_FONT);
            }
            // Render from_mult near the source endpoint (slightly offset).
            if let Some(from_mult) = &conn.from_mult {
                let mx = fx + fw / 2.0 + 6.0;
                let my = fy + COMPONENT_H + 14.0;
                svg.text(mx, my, from_mult, "start", SMALL_FONT);
            }
            // Render to_mult near the target endpoint (slightly offset).
            if let Some(to_mult) = &conn.to_mult {
                let mx = tx + tw / 2.0 + 6.0;
                let my = ty - 4.0;
                svg.text(mx, my, to_mult, "start", SMALL_FONT);
            }
        }
    }

    svg.finalize()
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
        .map(|p| estimate_package_width(p))
        .fold(0.0_f64, f64::max)
        + MARGIN * 2.0
}

fn estimate_packages_height(packages: &[ComponentPackage]) -> f64 {
    packages.iter().map(|p| estimate_package_height(p)).sum::<f64>()
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
        .map(|p| estimate_package_width(p))
        .fold(0.0_f64, f64::max);
    let leaf_w = if pkg.components.is_empty() {
        0.0
    } else {
        COMPONENT_MIN_W
    };
    nested_w.max(leaf_w)
}

fn estimate_package_height(pkg: &ComponentPackage) -> f64 {
    let nested_h: f64 = pkg
        .packages
        .iter()
        .map(|p| estimate_package_height(p))
        .sum();
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
}
