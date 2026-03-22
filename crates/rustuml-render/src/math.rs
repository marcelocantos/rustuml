// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Math/LaTeX diagram renderer.
//!
//! Delegates to the `rustuml-math` crate (KaTeX) to produce an SVG.

use rustuml_math::{MathRenderOpts, render_math_with_opts};
use rustuml_parser::diagram::math::MathDiagram;

use crate::style::Theme;

/// Render a [`MathDiagram`] to an SVG string.
pub fn render(diagram: &MathDiagram, _theme: &Theme) -> String {
    let opts = MathRenderOpts {
        display_mode: true,
        ..Default::default()
    };
    match render_math_with_opts(&diagram.content, &opts) {
        Ok(svg) => svg,
        Err(e) => error_svg(&e),
    }
}

fn error_svg(msg: &str) -> String {
    let escaped = msg
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="60">
  <rect width="400" height="60" fill="#fff0f0" stroke="#cc0000"/>
  <text x="8" y="20" font-family="monospace" font-size="12" fill="#cc0000">Math render error:</text>
  <text x="8" y="40" font-family="monospace" font-size="11" fill="#cc0000">{escaped}</text>
</svg>"##
    )
}
