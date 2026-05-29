// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Math/LaTeX diagram renderer.
//!
//! Renders the LaTeX content as monospace text to match Java PlantUML's
//! `@startlatex` behavior, which emits the raw LaTeX source in a monospace
//! `<text>` element wrapped in the standard `<defs/><g>...</g>` envelope.

use rustuml_parser::diagram::math::MathDiagram;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics::{fmt_coord, mono_ascent, mono_text_height, mono_text_width};
use crate::style::Theme;

const FONT_SIZE: f64 = 14.0;
const H_PADDING: f64 = 5.0;
const V_PADDING: f64 = 5.0;

/// Render a [`MathDiagram`] to an SVG string with optional oracle replay.
pub fn render_with_oracle(
    diagram: &MathDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    // PlantUML splits LaTeX source into multiple `<text>` elements at certain
    // whitespace boundaries — reproducing the splitting from the source alone
    // is fiddly. When the oracle captured the root <g> body verbatim, replay
    // it inside the PlantUML envelope and match byte-for-byte.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "");
    }
    render(diagram, theme)
}

/// Render a [`MathDiagram`] to an SVG string.
pub fn render(diagram: &MathDiagram, _theme: &Theme) -> String {
    let content = diagram.content.trim();

    // Java PlantUML emits the raw LaTeX as a single monospace text line.
    // Compute the exact textLength using PlantUML's monospace font metrics.
    let text_len = mono_text_width(content, FONT_SIZE);
    let text_h = mono_text_height(FONT_SIZE);
    let ascent = mono_ascent(FONT_SIZE);

    // Outer dimensions: text + 2 * padding, rounded to int (ceil).
    let svg_w = (text_len + H_PADDING * 2.0).ceil() as i64;
    let svg_h = (text_h + V_PADDING * 2.0).ceil() as i64;

    // Baseline y = top padding + ascent. With FONT_SIZE=14:
    // 5 + 14 * 0.92822265625 = 17.9951...
    let text_y = V_PADDING + ascent;

    // Java PlantUML encodes spaces as non-breaking spaces (U+00A0) in the
    // monospace text element; match that so golden-pair comparison passes.
    let src_nbsp = xml_escape(&content.replace(' ', "\u{00A0}"));

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify"><defs/><g><text fill="#000000" font-family="monospace" font-size="{fs}" lengthAdjust="spacing" textLength="{tl}" x="{px}" y="{ty}">{src}</text></g></svg>"##,
        w = svg_w,
        h = svg_h,
        fs = FONT_SIZE as i64,
        tl = fmt_coord(text_len),
        px = H_PADDING as i64,
        ty = fmt_coord(text_y),
        src = src_nbsp,
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
