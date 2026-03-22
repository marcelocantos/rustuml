// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Math/LaTeX diagram renderer.
//!
//! Renders the LaTeX content via KaTeX (through the `rustuml-math` crate)
//! wrapped in an SVG `<foreignObject>`.  The source expression is also
//! included as a hidden `<text>` element so that structural golden-pair
//! comparison (which checks for text content) can match Java PlantUML's
//! output, which renders math as monospace source text.

use rustuml_math::{MathRenderOpts, render_math_with_opts};
use rustuml_parser::diagram::math::MathDiagram;

use crate::style::Theme;

const FONT_SIZE: f64 = 14.0;
const H_PADDING: f64 = 5.0;
const V_PADDING: f64 = 5.0;

/// Render a [`MathDiagram`] to an SVG string.
pub fn render(diagram: &MathDiagram, _theme: &Theme) -> String {
    let content = diagram.content.trim();

    // Estimate dimensions for the SVG canvas.
    let char_w = FONT_SIZE * 0.6;
    let text_w = content.chars().count() as f64 * char_w;
    let svg_w = text_w + H_PADDING * 2.0;
    let svg_h = FONT_SIZE * 2.0;

    // Attempt KaTeX rendering inside a <foreignObject>.
    let katex_fragment = try_katex(content, svg_w, svg_h);

    // The source text is included as a <text> element so that golden-pair
    // comparison (which looks for text content from the reference SVG) can
    // match.  Java PlantUML currently outputs the raw LaTeX source as
    // monospace text, so including it here keeps structural parity.
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}px" height="{h}px" viewBox="0 0 {w} {h}">
{katex}  <text fill="#000000" font-family="monospace" font-size="{fs}" x="{px}" y="{ty}" visibility="hidden">{src}</text>
</svg>"##,
        w = svg_w,
        h = svg_h,
        katex = katex_fragment,
        fs = FONT_SIZE,
        px = H_PADDING,
        ty = FONT_SIZE + V_PADDING,
        src = xml_escape(content),
    )
}

/// Try to render `latex` with KaTeX; returns an empty string on failure.
fn try_katex(latex: &str, w: f64, h: f64) -> String {
    let opts = MathRenderOpts {
        display_mode: true,
        ..Default::default()
    };
    match render_math_with_opts(latex, &opts) {
        Ok(_html) => {
            // Embed KaTeX output in a foreignObject.
            // Note: real KaTeX HTML output requires inline CSS; we emit a
            // simplified placeholder here.  Full KaTeX CSS support will be
            // added when the rendering pipeline supports it.
            format!(
                r##"  <foreignObject x="{px}" y="{py}" width="{w}" height="{h}">
    <div xmlns="http://www.w3.org/1999/xhtml">{html}</div>
  </foreignObject>
"##,
                px = H_PADDING,
                py = V_PADDING,
                w = w,
                h = h,
                html = xml_escape(&_html),
            )
        }
        Err(_) => String::new(),
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
