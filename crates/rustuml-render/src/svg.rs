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
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        )
        .unwrap();
        Self { buf, indent: 1, group_depth: 0 }
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
        // Check for creole markup: markers must appear at least twice (balanced pair)
        // or HTML tags must be present, to avoid false positives in arrow labels.
        let has_creole = (content.matches("**").count() >= 2)
            || (content.matches("//").count() >= 2)
            || (content.matches("__").count() >= 2)
            || (content.matches("--").count() >= 2)
            || (content.matches("\"\"").count() >= 2)
            || content.contains('`')
            || content.contains("<b>")
            || content.contains("<i>")
            || content.contains("<u>")
            || content.contains("<s>")
            || content.contains("<del>")
            || content.contains("<color:")
            || content.contains("<size:")
            || content.contains("<font")
            || content.contains("<back:")
            || content.contains("<mono>");
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

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        // Encode guillemets as numeric entities to match PlantUML SVG output.
        .replace('\u{00ab}', "&#171;")
        .replace('\u{00bb}', "&#187;")
}
