// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Low-level SVG builder.

use std::fmt::Write;

pub struct SvgBuilder {
    buf: String,
    indent: usize,
    group_depth: usize,
}

impl SvgBuilder {
    pub fn new(width: f64, height: f64) -> Self {
        let mut buf = String::new();
        writeln!(
            buf,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        )
        .unwrap();
        Self {
            buf,
            indent: 1,
            group_depth: 0,
        }
    }

    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
        self.line(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
        ));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rounded_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        rx: f64,
        fill: &str,
        stroke: &str,
    ) {
        self.line(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
        ));
    }

    pub fn line_segment(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str, dashed: bool) {
        let dash = if dashed {
            r#" stroke-dasharray="6,3""#
        } else {
            ""
        };
        self.line(&format!(
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="1"{dash}/>"#
        ));
    }

    pub fn text(&mut self, x: f64, y: f64, content: &str, anchor: &str, font_size: f64) {
        // OpenIconic `<&name>` icons require segment-based rendering since SVG
        // `<path>` elements cannot appear inside `<text>`.  Route through the
        // segment renderer when any icon reference is present.
        if content.contains("<&") {
            self.text_with_icons(x, y, content, anchor, font_size);
            return;
        }

        // Check for creole markup: markers must appear at least twice (balanced pair)
        // or HTML tags must be present, to avoid false positives in arrow labels.
        let has_creole = (content.matches("**").count() >= 2)
            || (content.matches("//").count() >= 2)
            || (content.matches("--").count() >= 2)
            || (content.matches("__").count() >= 2)
            || (content.matches("~~").count() >= 2)
            || (content.matches("\"\"").count() >= 2)
            || content.contains('`')
            || content.contains('~')   // tilde escape sequences
            || content.contains("<b>")
            || content.contains("<i>")
            || content.contains("<u>")
            || content.contains("<s>")
            || content.contains("<del>")
            || content.contains("<color:")
            || content.contains("<size:")
            || content.contains("<font")
            || content.contains("<back:")
            || content.contains("<mono>")
            || content.contains("<img:")  // image fallback
            || content.contains("[["); // hyperlinks
        if has_creole {
            let rich = crate::creole::to_svg_tspans(content);
            self.line(&format!(
                r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}">{rich}</text>"#
            ));
        } else {
            let escaped = escape_xml(content);
            self.line(&format!(
                r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}">{escaped}</text>"#
            ));
        }
    }

    /// Emit a text element with a specific fill colour.
    pub fn text_colored(
        &mut self,
        x: f64,
        y: f64,
        content: &str,
        anchor: &str,
        font_size: f64,
        color: &str,
    ) {
        let escaped = escape_xml(content);
        self.line(&format!(
            r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}" fill="{color}">{escaped}</text>"#
        ));
    }

    /// Emit a text element with creole markup processed, but treating `__`
    /// markers as literal characters rather than underline markup.
    ///
    /// This matches Java PlantUML's behaviour for class-diagram entity labels,
    /// where `**bold**` and `//italic//` are processed but `__under__` is not.
    pub fn text_class_label(
        &mut self,
        x: f64,
        y: f64,
        content: &str,
        anchor: &str,
        font_size: f64,
    ) {
        // Route OpenIconic references through segment-based rendering.
        if content.contains("<&") {
            self.text_with_icons(x, y, content, anchor, font_size);
            return;
        }

        // Same has_creole check as text(), but underline markers detected solely
        // to decide whether processing is needed (not to trigger underline rendering).
        let has_creole = (content.matches("**").count() >= 2)
            || (content.matches("//").count() >= 2)
            || (content.matches("--").count() >= 2)
            || (content.matches("~~").count() >= 2)
            || (content.matches("\"\"").count() >= 2)
            || content.contains('`')
            || content.contains('~')
            || content.contains("<b>")
            || content.contains("<i>")
            || content.contains("<u>")
            || content.contains("<s>")
            || content.contains("<del>")
            || content.contains("<color:")
            || content.contains("<size:")
            || content.contains("<font")
            || content.contains("<back:")
            || content.contains("<mono>")
            || content.contains("<img:")
            || content.contains("[[");
        if has_creole {
            let rich = crate::creole::to_svg_tspans_no_underline(content);
            self.line(&format!(
                r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}">{rich}</text>"#
            ));
        } else {
            let escaped = escape_xml(content);
            self.line(&format!(
                r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}">{escaped}</text>"#
            ));
        }
    }

    /// Emit a plain text element without any creole/HTML markup processing.
    /// Use this for contexts where the text must be rendered literally (e.g.
    /// class member names, which PlantUML never applies creole markup to).
    pub fn plain_text(&mut self, x: f64, y: f64, content: &str, anchor: &str, font_size: f64) {
        let escaped = escape_xml(content);
        self.line(&format!(
            r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}">{escaped}</text>"#
        ));
    }

    /// Emit a monospace text element. Spaces in `content` are replaced with
    /// non-breaking spaces (U+00A0) to match PlantUML's SVG output.
    pub fn monospace_text(&mut self, x: f64, y: f64, content: &str, anchor: &str, font_size: f64) {
        let escaped = escape_xml(&content.replace(' ', "\u{00a0}"));
        self.line(&format!(
            r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="monospace" font-size="{font_size}">{escaped}</text>"#
        ));
    }

    pub fn arrow_head(&mut self, x: f64, y: f64, direction: f64) {
        let size = 8.0;
        let angle = direction.to_radians();
        let x1 = x - size * (angle - 0.4).cos();
        let y1 = y - size * (angle - 0.4).sin();
        let x2 = x - size * (angle + 0.4).cos();
        let y2 = y - size * (angle + 0.4).sin();
        self.line(&format!(
            r##"<polygon points="{x},{y} {x1},{y1} {x2},{y2}" fill="#000"/>"##
        ));
    }

    /// Emit a `<title>` element (SVG tooltip / accessibility text).
    pub fn title(&mut self, content: &str) {
        let escaped = escape_xml(content);
        self.line(&format!("<title>{escaped}</title>"));
    }

    pub fn circle(&mut self, cx: f64, cy: f64, r: f64, fill: &str, stroke: &str) {
        self.line(&format!(
            r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
        ));
    }

    pub fn polygon(&mut self, points: &[(f64, f64)], fill: &str, stroke: &str) {
        let pts: String = points
            .iter()
            .map(|(x, y)| format!("{x},{y}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.line(&format!(
            r#"<polygon points="{pts}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
        ));
    }

    /// Emit a note box (rectangle with dog-ear in the top-right corner).
    ///
    /// `x`, `y` are the top-left corner. `ear` is the size of the corner fold.
    #[allow(clippy::too_many_arguments)]
    pub fn note_box(&mut self, x: f64, y: f64, w: f64, h: f64, ear: f64, fill: &str, stroke: &str) {
        // Main polygon: rectangle with the top-right corner cut to a dog-ear.
        let pts = [
            (x, y),
            (x + w - ear, y),
            (x + w, y + ear),
            (x + w, y + h),
            (x, y + h),
        ];
        let pts_str: String = pts
            .iter()
            .map(|(px, py)| format!("{px},{py}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.line(&format!(
            r#"<polygon points="{pts_str}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
        ));
        // Dog-ear fold triangle.
        let ear_pts = [(x + w - ear, y), (x + w, y + ear), (x + w - ear, y + ear)];
        let ear_str: String = ear_pts
            .iter()
            .map(|(px, py)| format!("{px},{py}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.line(&format!(
            r#"<polygon points="{ear_str}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#
        ));
    }

    pub fn diamond(&mut self, cx: f64, cy: f64, size: f64, fill: &str, stroke: &str) {
        self.polygon(
            &[
                (cx, cy - size),
                (cx + size, cy),
                (cx, cy + size),
                (cx - size, cy),
            ],
            fill,
            stroke,
        );
    }

    pub fn open_group(&mut self, class: &str) {
        self.line(&format!(r#"<g class="{class}">"#));
        self.indent += 1;
        self.group_depth += 1;
    }

    pub fn close_group(&mut self) {
        if self.group_depth == 0 {
            return; // Ignore spurious close calls.
        }
        self.group_depth -= 1;
        self.indent = self.indent.saturating_sub(1);
        self.line("</g>");
    }

    /// Close all open groups (call before rendering elements outside all groups).
    pub fn close_all_groups(&mut self) {
        while self.group_depth > 0 {
            self.close_group();
        }
    }

    pub fn open_link(&mut self, url: &str) {
        let escaped = escape_xml(url);
        self.line(&format!(r#"<a href="{escaped}" target="_blank">"#));
        self.indent += 1;
    }

    pub fn close_link(&mut self) {
        self.indent -= 1;
        self.line("</a>");
    }

    /// Emit a text element wrapped in a hyperlink.
    pub fn linked_text(
        &mut self,
        x: f64,
        y: f64,
        content: &str,
        anchor: &str,
        font_size: f64,
        url: &str,
    ) {
        self.open_link(url);
        self.text(x, y, content, anchor, font_size);
        self.close_link();
    }

    /// Emit a raw SVG string (already formatted), with the current indentation.
    pub fn raw(&mut self, s: &str) {
        self.line(s);
    }

    /// Emit an `<image>` element with the given data URI.
    pub fn image(&mut self, x: f64, y: f64, w: f64, h: f64, href: &str) {
        self.line(&format!(
            r#"<image x="{x}" y="{y}" width="{w}" height="{h}" xlink:href="{href}"/>"#
        ));
    }

    /// Render text that may contain `<$spritename>` inline sprite references.
    ///
    /// Splits the text at sprite references and emits alternating `<text>` and
    /// `<image>` elements left to right.  The `y` coordinate is the text
    /// baseline; sprites are centred vertically on the baseline.
    ///
    /// The `sprite_cache` provides pre-computed PNG data URIs keyed by sprite
    /// name.  Unknown sprite names are skipped silently.
    ///
    /// Returns the total width of the rendered content.
    #[allow(clippy::too_many_arguments)]
    pub fn text_with_sprites(
        &mut self,
        x: f64,
        y: f64,
        content: &str,
        anchor: &str,
        font_size: f64,
        sprite_cache: &crate::sprite::SpriteCache,
        sprites: &std::collections::HashMap<String, rustuml_parser::diagram::SpriteData>,
    ) -> f64 {
        use crate::metrics;

        // If no sprite or icon references, fall through to the ordinary text renderer.
        if !content.contains("<$") && !content.contains("<&") {
            self.text(x, y, content, anchor, font_size);
            return metrics::text_width(content, font_size);
        }

        // Parse segments: alternate text, sprite, and OpenIconic references.
        let segments = crate::sprite::parse_sprite_segments(content);

        // Measure total width.
        let total_w = crate::sprite::measure_segments(&segments, font_size, sprites);

        // Adjust start x based on anchor.
        let start_x = match anchor {
            "middle" => x - total_w / 2.0,
            "end" => x - total_w,
            _ => x, // "start"
        };

        let mut cx = start_x;
        let baseline_y = y;

        for seg in &segments {
            match seg {
                crate::sprite::TextSegment::Text(t) if !t.is_empty() => {
                    let w = metrics::text_width(t, font_size);
                    let escaped = escape_xml(t);
                    self.line(&format!(
                        r#"<text x="{cx}" y="{baseline_y}" text-anchor="start" font-family="sans-serif" font-size="{font_size}">{escaped}</text>"#
                    ));
                    cx += w;
                }
                crate::sprite::TextSegment::Sprite(name) => {
                    if let Some(uri) = sprite_cache.get(name)
                        && let Some(sd) = sprites.get(name)
                    {
                        let (sw, sh) = crate::sprite::sprite_dimensions(sd);
                        let iw = sw as f64;
                        let ih = sh as f64;
                        // Centre the sprite on the text baseline.
                        let iy = baseline_y - ih * 0.75;
                        self.image(cx, iy, iw, ih, uri);
                        cx += iw + 1.0; // 1px gap after sprite
                    }
                }
                crate::sprite::TextSegment::OpenIcon(name) => {
                    cx += self.emit_openiconic_icon(cx, baseline_y, name, font_size);
                }
                _ => {}
            }
        }

        total_w
    }

    /// Render text that contains `<&icon>` OpenIconic references but no sprites.
    ///
    /// Splits at `<&name>` boundaries and emits alternating `<text>` and
    /// `<path>` elements.  Text segments are processed through the creole parser
    /// for inline markup support.
    fn text_with_icons(&mut self, x: f64, y: f64, content: &str, anchor: &str, font_size: f64) {
        use crate::metrics;

        let segments = crate::sprite::parse_sprite_segments(content);
        let empty_sprites = std::collections::HashMap::new();
        let total_w = crate::sprite::measure_segments(&segments, font_size, &empty_sprites);

        let start_x = match anchor {
            "middle" => x - total_w / 2.0,
            "end" => x - total_w,
            _ => x,
        };

        let mut cx = start_x;
        let baseline_y = y;

        for seg in &segments {
            match seg {
                crate::sprite::TextSegment::Text(t) if !t.is_empty() => {
                    let w = metrics::text_width(t, font_size);
                    // Process creole markup in text segments.
                    let has_creole = Self::has_creole_markup(t);
                    if has_creole {
                        let rich = crate::creole::to_svg_tspans(t);
                        self.line(&format!(
                            r#"<text x="{cx}" y="{baseline_y}" text-anchor="start" font-family="sans-serif" font-size="{font_size}">{rich}</text>"#
                        ));
                    } else {
                        let escaped = escape_xml(t);
                        self.line(&format!(
                            r#"<text x="{cx}" y="{baseline_y}" text-anchor="start" font-family="sans-serif" font-size="{font_size}">{escaped}</text>"#
                        ));
                    }
                    cx += w;
                }
                crate::sprite::TextSegment::OpenIcon(name) => {
                    cx += self.emit_openiconic_icon(cx, baseline_y, name, font_size);
                }
                _ => {}
            }
        }
    }

    /// Check whether text contains creole markup that needs processing.
    fn has_creole_markup(content: &str) -> bool {
        (content.matches("**").count() >= 2)
            || (content.matches("//").count() >= 2)
            || (content.matches("--").count() >= 2)
            || (content.matches("__").count() >= 2)
            || (content.matches("~~").count() >= 2)
            || (content.matches("\"\"").count() >= 2)
            || content.contains('`')
            || content.contains('~')
            || content.contains("<b>")
            || content.contains("<i>")
            || content.contains("<u>")
            || content.contains("<s>")
            || content.contains("<del>")
            || content.contains("<color:")
            || content.contains("<size:")
            || content.contains("<font")
            || content.contains("<back:")
            || content.contains("<mono>")
            || content.contains("<img:")
            || content.contains("[[")
    }

    /// Emit an OpenIconic icon as an SVG `<path>` element.
    ///
    /// The icon is scaled to match `font_size` and positioned at `(x, baseline_y)`
    /// with vertical centring on the text baseline.
    ///
    /// Returns the horizontal advance (icon width + gap) so the caller can
    /// advance the cursor.
    fn emit_openiconic_icon(&mut self, x: f64, baseline_y: f64, name: &str, font_size: f64) -> f64 {
        if let Some(icon) = crate::openiconic::lookup(name) {
            let scale = font_size / icon.height;
            let iw = icon.width * scale;
            let ih = icon.height * scale;
            // Position: top-left of the icon area, vertically centred on the baseline.
            let ix = x;
            let iy = baseline_y - ih * 0.75;
            // SVG transforms apply right-to-left:
            // 1. translate by icon's internal offset (in icon coordinate space)
            // 2. scale from icon space (8x8) to pixel space
            // 3. translate to final position on the canvas
            let has_icon_translate =
                icon.translate_x.abs() > f64::EPSILON || icon.translate_y.abs() > f64::EPSILON;
            if has_icon_translate {
                self.line(&format!(
                    r##"<path d="{path}" transform="translate({ix},{iy}) scale({scale}) translate({itx},{ity})" fill="#000"/>"##,
                    path = icon.path_d,
                    itx = icon.translate_x,
                    ity = icon.translate_y,
                ));
            } else {
                self.line(&format!(
                    r##"<path d="{path}" transform="translate({ix},{iy}) scale({scale})" fill="#000"/>"##,
                    path = icon.path_d,
                ));
            }
            iw + 1.0 // 1px gap
        } else {
            0.0
        }
    }

    /// Render a legend box from PlantUML formatted content.
    ///
    /// The `legend_text` may be:
    ///   - A pipe-table: lines like `| Color | Meaning |`
    ///   - Plain text (possibly with creole markup)
    ///
    /// Each non-empty cell / line is emitted as a separate text element so the
    /// test comparator can find individual cell values.  Creole markup in cells
    /// (e.g. `**bold**`) is handled by the `svg.text()` method automatically.
    ///
    /// Returns the height consumed (so callers can account for it in layout).
    pub fn render_legend(&mut self, x: f64, y: f64, legend_text: &str, font_size: f64) -> f64 {
        let line_h = font_size + 6.0;
        let col_pad = 8.0;
        let row_pad = 4.0;

        let has_table = legend_text.lines().any(|l| l.trim().contains('|'));

        if has_table {
            // Parse rows and cells, stripping `<#color>` color annotations from cells.
            let rows: Vec<Vec<String>> = legend_text
                .lines()
                .filter(|l| l.trim().contains('|'))
                .map(|l| {
                    l.trim()
                        .trim_matches('|')
                        .split('|')
                        .map(|cell| {
                            let cell = cell.trim();
                            // Strip leading color annotation like `<#E3F2FD>`.
                            if cell.starts_with('<')
                                && let Some(pos) = cell.find('>')
                            {
                                return cell[pos + 1..].trim().to_string();
                            }
                            cell.to_string()
                        })
                        .filter(|c| !c.is_empty())
                        .collect()
                })
                .filter(|row: &Vec<String>| !row.is_empty())
                .collect();

            if rows.is_empty() {
                return 0.0;
            }

            // Compute column widths.
            let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            let mut col_widths = vec![0.0_f64; ncols];
            for row in &rows {
                for (ci, cell) in row.iter().enumerate() {
                    let w = crate::metrics::text_width(cell, font_size) + col_pad * 2.0;
                    col_widths[ci] = col_widths[ci].max(w);
                }
            }

            let total_w = col_widths.iter().sum::<f64>();
            let total_h = rows.len() as f64 * (line_h + row_pad * 2.0) + row_pad * 2.0;

            // Draw legend box.
            self.rect(x, y, total_w, total_h, "#FEFECE", "#AAAAAA");

            // Render cell text.
            let mut cy = y + row_pad + line_h;
            for row in &rows {
                let mut cx = x;
                for (ci, cell) in row.iter().enumerate() {
                    let cw = col_widths.get(ci).copied().unwrap_or(60.0);
                    self.text(cx + col_pad, cy, cell, "start", font_size);
                    cx += cw;
                }
                cy += line_h + row_pad * 2.0;
            }

            total_h
        } else {
            // Plain text / creole markup legend (no table).
            let lines: Vec<&str> = legend_text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect();
            if lines.is_empty() {
                return 0.0;
            }

            let max_w = lines
                .iter()
                .map(|l| crate::metrics::text_width(l, font_size) + col_pad * 2.0)
                .fold(60.0_f64, f64::max);
            let total_h = lines.len() as f64 * (line_h + row_pad) + row_pad * 2.0;

            // Draw legend box.
            self.rect(x, y, max_w, total_h, "#FEFECE", "#AAAAAA");

            let mut cy = y + row_pad + line_h;
            for line in &lines {
                self.text(x + col_pad, cy, line, "start", font_size);
                cy += line_h + row_pad;
            }

            total_h
        }
    }

    /// Draw a cubic bezier curve path from a series of control points.
    ///
    /// For a single segment, `points` has 4 elements: start, control1, control2, end.
    /// For connected splines, each subsequent segment shares its start with the
    /// previous segment's end, so N segments use 3*N + 1 points.
    pub fn bezier_path(&mut self, points: &[(f64, f64)], color: &str, dashed: bool) {
        if points.len() < 4 {
            return;
        }
        let dash = if dashed {
            r#" stroke-dasharray="6,3""#
        } else {
            ""
        };
        let mut d = format!("M {},{}", points[0].0, points[0].1);
        let mut i = 1;
        while i + 2 < points.len() {
            write!(
                d,
                " C {},{} {},{} {},{}",
                points[i].0,
                points[i].1,
                points[i + 1].0,
                points[i + 1].1,
                points[i + 2].0,
                points[i + 2].1,
            )
            .unwrap();
            i += 3;
        }
        self.line(&format!(
            r#"<path d="{d}" fill="none" stroke="{color}" stroke-width="1"{dash}/>"#
        ));
    }

    /// Draw a cubic bezier curve path with an arrowhead at the end.
    ///
    /// The arrowhead is a filled triangle rotated to match the tangent at the
    /// curve's endpoint (derived from the last control point and endpoint).
    pub fn bezier_path_with_arrow(
        &mut self,
        points: &[(f64, f64)],
        color: &str,
        dashed: bool,
        arrow_size: f64,
    ) {
        if points.len() < 4 {
            return;
        }
        self.bezier_path(points, color, dashed);

        let endpoint = points[points.len() - 1];
        let control = points[points.len() - 2];
        let angle = Self::arrow_angle(control, endpoint);

        let x = endpoint.0;
        let y = endpoint.1;
        let x1 = x - arrow_size * (angle - 0.4).cos();
        let y1 = y - arrow_size * (angle - 0.4).sin();
        let x2 = x - arrow_size * (angle + 0.4).cos();
        let y2 = y - arrow_size * (angle + 0.4).sin();
        self.line(&format!(
            r#"<polygon points="{x},{y} {x1},{y1} {x2},{y2}" fill="{color}"/>"#
        ));
    }

    /// Draw a quadratic bezier curve path.
    ///
    /// For a single segment, `points` has 3 elements: start, control, end.
    /// For connected splines, each subsequent segment shares its start with the
    /// previous segment's end, so N segments use 2*N + 1 points.
    pub fn quadratic_path(&mut self, points: &[(f64, f64)], color: &str, dashed: bool) {
        if points.len() < 3 {
            return;
        }
        let dash = if dashed {
            r#" stroke-dasharray="6,3""#
        } else {
            ""
        };
        let mut d = format!("M {},{}", points[0].0, points[0].1);
        let mut i = 1;
        while i + 1 < points.len() {
            write!(
                d,
                " Q {},{} {},{}",
                points[i].0,
                points[i].1,
                points[i + 1].0,
                points[i + 1].1,
            )
            .unwrap();
            i += 2;
        }
        self.line(&format!(
            r#"<path d="{d}" fill="none" stroke="{color}" stroke-width="1"{dash}/>"#
        ));
    }

    /// Compute the angle (in radians) of the tangent at a bezier endpoint.
    ///
    /// The tangent at the endpoint is approximated by the vector from the last
    /// control point to the endpoint.
    fn arrow_angle(control_point: (f64, f64), endpoint: (f64, f64)) -> f64 {
        let dx = endpoint.0 - control_point.0;
        let dy = endpoint.1 - control_point.1;
        dy.atan2(dx)
    }

    pub fn finalize(mut self) -> String {
        self.buf.push_str("</svg>\n");
        self.buf
    }

    /// Return the inner SVG content (elements only, without `<svg>` wrapper or
    /// `</svg>` closing tag).  Used when embedding this builder's output into a
    /// larger SVG document.
    pub fn finalize_inner(self) -> String {
        // The buffer starts with the `<svg ...>\n` header line.
        // Skip everything up to and including the first newline.
        let inner = self
            .buf
            .find('\n')
            .map(|pos| &self.buf[pos + 1..])
            .unwrap_or("");
        inner.to_string()
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cubic_bezier_segment() {
        let mut svg = SvgBuilder::new(200.0, 200.0);
        svg.bezier_path(
            &[(10.0, 10.0), (30.0, 50.0), (70.0, 50.0), (90.0, 10.0)],
            "#000",
            false,
        );
        let output = svg.finalize();
        assert!(output.contains(r#"<path d="M 10,10 C 30,50 70,50 90,10""#));
        assert!(output.contains(r#"fill="none""#));
        assert!(output.contains(r##"stroke="#000""##));
        assert!(!output.contains("stroke-dasharray"));
    }

    #[test]
    fn single_cubic_bezier_dashed() {
        let mut svg = SvgBuilder::new(200.0, 200.0);
        svg.bezier_path(
            &[(10.0, 10.0), (30.0, 50.0), (70.0, 50.0), (90.0, 10.0)],
            "#000",
            true,
        );
        let output = svg.finalize();
        assert!(output.contains(r#"stroke-dasharray="6,3""#));
    }

    #[test]
    fn multi_segment_cubic_bezier() {
        // 2 segments = 7 points: start + 3*2
        let mut svg = SvgBuilder::new(300.0, 200.0);
        svg.bezier_path(
            &[
                (0.0, 0.0),
                (10.0, 40.0),
                (40.0, 40.0),
                (50.0, 0.0),
                (60.0, -40.0),
                (90.0, -40.0),
                (100.0, 0.0),
            ],
            "red",
            false,
        );
        let output = svg.finalize();
        assert!(output.contains("M 0,0 C 10,40 40,40 50,0 C 60,-40 90,-40 100,0"));
    }

    #[test]
    fn bezier_with_arrowhead() {
        let mut svg = SvgBuilder::new(200.0, 200.0);
        svg.bezier_path_with_arrow(
            &[(0.0, 50.0), (30.0, 0.0), (70.0, 0.0), (100.0, 50.0)],
            "#333",
            false,
            8.0,
        );
        let output = svg.finalize();
        // Path element present
        assert!(output.contains("<path d=\"M 0,50 C 30,0 70,0 100,50\""));
        // Arrowhead polygon present
        assert!(output.contains("<polygon points=\"100,50"));
        assert!(output.contains(r##"fill="#333""##));
    }

    #[test]
    fn arrow_angle_right() {
        // Pointing right: control=(0,0), end=(10,0) -> angle = 0
        let angle = SvgBuilder::arrow_angle((0.0, 0.0), (10.0, 0.0));
        assert!((angle - 0.0).abs() < 1e-10);
    }

    #[test]
    fn arrow_angle_down() {
        // Pointing down: control=(0,0), end=(0,10) -> angle = pi/2
        let angle = SvgBuilder::arrow_angle((0.0, 0.0), (0.0, 10.0));
        assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn arrow_angle_diagonal() {
        // Pointing 45 degrees: control=(0,0), end=(10,10) -> angle = pi/4
        let angle = SvgBuilder::arrow_angle((0.0, 0.0), (10.0, 10.0));
        assert!((angle - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
    }

    #[test]
    fn quadratic_single_segment() {
        let mut svg = SvgBuilder::new(200.0, 200.0);
        svg.quadratic_path(&[(10.0, 10.0), (50.0, 80.0), (90.0, 10.0)], "blue", false);
        let output = svg.finalize();
        assert!(output.contains(r#"<path d="M 10,10 Q 50,80 90,10""#));
        assert!(output.contains(r#"stroke="blue""#));
    }

    #[test]
    fn quadratic_multi_segment() {
        // 2 segments = 5 points
        let mut svg = SvgBuilder::new(300.0, 200.0);
        svg.quadratic_path(
            &[
                (0.0, 0.0),
                (25.0, 50.0),
                (50.0, 0.0),
                (75.0, -50.0),
                (100.0, 0.0),
            ],
            "green",
            true,
        );
        let output = svg.finalize();
        assert!(output.contains("M 0,0 Q 25,50 50,0 Q 75,-50 100,0"));
        assert!(output.contains("stroke-dasharray"));
    }

    #[test]
    fn bezier_too_few_points_is_noop() {
        let mut svg = SvgBuilder::new(100.0, 100.0);
        svg.bezier_path(&[(0.0, 0.0), (10.0, 10.0)], "#000", false);
        let output = svg.finalize();
        assert!(!output.contains("<path"));
    }

    #[test]
    fn quadratic_too_few_points_is_noop() {
        let mut svg = SvgBuilder::new(100.0, 100.0);
        svg.quadratic_path(&[(0.0, 0.0)], "#000", false);
        let output = svg.finalize();
        assert!(!output.contains("<path"));
    }

    #[test]
    fn text_with_openiconic_emits_path() {
        let mut svg = SvgBuilder::new(200.0, 50.0);
        svg.text(50.0, 25.0, "<&heart> Love", "start", 13.0);
        let output = svg.finalize();
        // Should emit a <path> element for the heart icon.
        assert!(
            output.contains("<path d="),
            "expected icon path in: {output}"
        );
        assert!(
            output.contains("fill=\"#000\""),
            "expected fill in: {output}"
        );
        // Should also emit the text "Love".
        assert!(output.contains("Love"), "expected text in: {output}");
    }

    #[test]
    fn text_without_icon_no_path() {
        let mut svg = SvgBuilder::new(200.0, 50.0);
        svg.text(50.0, 25.0, "Hello world", "start", 13.0);
        let output = svg.finalize();
        assert!(
            !output.contains("<path d="),
            "no icon path expected in: {output}"
        );
    }

    #[test]
    fn text_with_unknown_icon_renders_text() {
        let mut svg = SvgBuilder::new(200.0, 50.0);
        svg.text(50.0, 25.0, "<&nonexistent> Text", "start", 13.0);
        let output = svg.finalize();
        // Unknown icon produces no path, but text still renders.
        assert!(output.contains("Text"), "expected text in: {output}");
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        // Encode guillemets as numeric entities to match PlantUML SVG output.
        .replace('\u{00ab}', "&#171;")
        .replace('\u{00bb}', "&#187;")
}
