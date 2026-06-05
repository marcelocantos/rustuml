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
const TAB_COLUMNS: usize = 8;

struct MathTextSegment {
    x: f64,
    text: String,
    text_len: f64,
}

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

    let segments = math_text_segments(content);
    let text_right = segments
        .iter()
        .map(|seg| seg.x + seg.text_len)
        .fold(H_PADDING, f64::max);
    let text_h = mono_text_height(FONT_SIZE);
    let ascent = mono_ascent(FONT_SIZE);

    // Outer dimensions: text + 2 * padding, rounded to int (ceil).
    let svg_w = (text_right + H_PADDING).ceil() as i64;
    let svg_h = (text_h + V_PADDING * 2.0).ceil() as i64;

    // Baseline y = top padding + ascent. With FONT_SIZE=14:
    // 5 + 14 * 0.92822265625 = 17.9951...
    let text_y = V_PADDING + ascent;

    let mut body = String::new();
    for seg in &segments {
        let src_nbsp = xml_escape(&seg.text.replace(' ', "\u{00A0}"));
        body.push_str(&format!(
            r##"<text fill="#000000" font-family="monospace" font-size="{fs}" lengthAdjust="spacing" textLength="{tl}" x="{px}" y="{ty}">{src}</text>"##,
            fs = FONT_SIZE as i64,
            tl = fmt_coord(seg.text_len),
            px = fmt_coord(seg.x),
            ty = fmt_coord(text_y),
            src = src_nbsp,
        ));
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify"><defs/><g>{body}</g></svg>"##,
        w = svg_w,
        h = svg_h,
    )
}

fn math_text_segments(content: &str) -> Vec<MathTextSegment> {
    let char_w = mono_text_width("0", FONT_SIZE);
    let mut segments = Vec::new();
    let mut col = 0usize;
    let mut segment_col = 0usize;
    let mut segment = String::new();
    let chars = content.chars().collect::<Vec<_>>();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1).is_some_and(|c| *c == 't') {
            push_math_segment(&mut segments, segment_col, char_w, &segment);
            col = ((col / TAB_COLUMNS) + 1) * TAB_COLUMNS;
            segment_col = col;
            segment.clear();
            i += 2;
        } else {
            segment.push(chars[i]);
            col += 1;
            i += 1;
        }
    }
    push_math_segment(&mut segments, segment_col, char_w, &segment);

    if segments.is_empty() {
        segments.push(MathTextSegment {
            x: H_PADDING,
            text: String::new(),
            text_len: 0.0,
        });
    }
    segments
}

fn push_math_segment(
    segments: &mut Vec<MathTextSegment>,
    segment_col: usize,
    char_w: f64,
    text: &str,
) {
    let visible = text.trim_end_matches(' ');
    if visible.is_empty() {
        return;
    }
    segments.push(MathTextSegment {
        x: H_PADDING + char_w * segment_col as f64,
        text: visible.to_string(),
        text_len: mono_text_width(visible, FONT_SIZE),
    });
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
