// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Low-level SVG builder.

use std::fmt::Write;

pub struct SvgBuilder {
    buf: String,
    indent: usize,
}

impl SvgBuilder {
    pub fn new(width: f64, height: f64) -> Self {
        let mut buf = String::new();
        writeln!(
            buf,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        )
        .unwrap();
        Self { buf, indent: 1 }
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
        let escaped = escape_xml(content);
        self.line(&format!(
            r#"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}">{escaped}</text>"#
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

    pub fn open_group(&mut self, class: &str) {
        self.line(&format!(r#"<g class="{class}">"#));
        self.indent += 1;
    }

    pub fn close_group(&mut self) {
        self.indent -= 1;
        self.line("</g>");
    }

    pub fn finalize(mut self) -> String {
        self.buf.push_str("</svg>\n");
        self.buf
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
}
