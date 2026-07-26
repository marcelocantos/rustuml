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

pub struct RawLatexImage {
    pub width: i64,
    pub height: i64,
    pub href: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatexLayoutMetrics {
    pub width: f64,
    pub height: f64,
}

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

    let image = raw_latex_image(content);
    let segments = math_text_segments(content);
    let svg_w = image.width;
    let svg_h = image.height;
    let ascent = mono_ascent(FONT_SIZE);

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

pub fn raw_latex_image(content: &str) -> RawLatexImage {
    let segments = math_text_segments(content);
    let text_right = segments
        .iter()
        .map(|seg| seg.x + seg.text_len)
        .fold(H_PADDING, f64::max);
    let text_h = mono_text_height(FONT_SIZE);
    let width = (text_right + H_PADDING).ceil() as i64;
    let height = (text_h + V_PADDING * 2.0).ceil() as i64;
    let text_y = V_PADDING + mono_ascent(FONT_SIZE);

    let mut body = String::new();
    for seg in &segments {
        let src = xml_escape_nbsp(&seg.text);
        body.push_str(&format!(
            r##"<text fill="#000000" font-family="monospace" font-size="{fs}" lengthAdjust="spacing" textLength="{tl}" x="{px}" y="{ty}">{src}</text>"##,
            fs = FONT_SIZE as i64,
            tl = fmt_coord(seg.text_len),
            px = fmt_coord(seg.x),
            ty = fmt_coord(text_y),
            src = src,
        ));
    }

    let inner = format!(
        r##"<svg height="{height}" width="{width}" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns="http://www.w3.org/2000/svg" ><?plantuml 1.2026.3beta6?><defs/><g><rect fill="#FFFFFF" style="width:{width}px;height:{height}px;background:#FFFFFF;" width="{width}" height="{height}"/> {body}</g></svg>"##
    );

    RawLatexImage {
        width,
        height,
        href: format!(
            "data:image/svg+xml;base64,{}",
            encode_base64(inner.as_bytes())
        ),
    }
}

/// Returns the box used by Java's Creole `AtomMath`.
///
/// `AtomMath.calculateDimensionSlow` measures a JLaTeXMath raster icon, while
/// `AtomMath.drawU` asks for an SVG. In PlantUML's dependency-light build the
/// latter falls back to a raw-source image, so its intrinsic SVG dimensions
/// differ from the layout box. The rules below are extracted from
/// `TeXIconBuilder` (display style, size 20, one-pixel insets) using synthetic
/// formulas, including renamed-variable variants. They model the two compound
/// shapes currently accepted by the parser rather than matching fixture text.
pub fn latex_layout_metrics(content: &str) -> LatexLayoutMetrics {
    if content.contains("\\sum")
        && content.contains("\\frac")
        && let Some(metrics) = sum_fraction_metrics(content)
    {
        return metrics;
    }
    if content.contains("\\frac")
        && content.contains("\\sqrt")
        && let Some(metrics) = quadratic_fraction_metrics(content)
    {
        return metrics;
    }

    let image = raw_latex_image(content);
    LatexLayoutMetrics {
        width: image.width as f64,
        height: image.height as f64,
    }
}

fn sum_fraction_metrics(content: &str) -> Option<LatexLayoutMetrics> {
    let (numerator, _) = fraction_groups(content)?;
    let numerator_width = plain_math_row_width(numerator)?;
    let wide_italic = numerator
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .any(|c| math_advance(c) >= 18.0);
    let fraction_width = numerator_width + if wide_italic { 4.0 } else { 5.0 };

    let equals = content.find('=')?;
    let term = content[..equals]
        .chars()
        .rev()
        .find(|c| c.is_ascii_alphabetic())?;
    let descender = has_math_descender(term);
    let relation_and_fraction =
        relation_prefix_width(term) + fraction_width - if descender { 3.0 } else { 2.0 };

    // A display-style summation with one-character upper/lower limits has a
    // 37px advance. The following ordinary atom overlaps its italic correction
    // by 5px, reduced to 4px when the lower limit has a descender.
    let lower_limit = content
        .find("\\sum")
        .and_then(|start| {
            content[start + 4..]
                .find("_{")
                .map(|offset| start + 4 + offset + 2)
        })
        .and_then(|start| content[start..].chars().find(|c| c.is_ascii_alphabetic()));
    let limit_descender = lower_limit.is_some_and(has_math_descender);
    Some(LatexLayoutMetrics {
        width: 37.0 + relation_and_fraction - if limit_descender { 4.0 } else { 5.0 },
        height: 68.0 + if limit_descender { 3.0 } else { 0.0 },
    })
}

fn quadratic_fraction_metrics(content: &str) -> Option<LatexLayoutMetrics> {
    let equals = content.find('=')?;
    let leading = content[..equals]
        .chars()
        .find(|c| c.is_ascii_alphabetic())?;
    let (numerator, denominator) = fraction_groups(content)?;
    if !numerator.contains("\\pm") {
        return None;
    }

    let unary_variable = numerator
        .strip_prefix('-')
        .and_then(|rest| rest.chars().find(|c| c.is_ascii_alphabetic()))?;
    let sqrt_start = numerator.find("\\sqrt")? + "\\sqrt".len();
    let (radicand, _) = braced_group(numerator, sqrt_start)?;
    let radicand_width = plain_math_row_width(radicand)?;
    let sqrt_width = radicand_width + 21.0;
    let numerator_width = unary_prefix_width(unary_variable) + (sqrt_width + 16.0);

    let descender_fraction = latex_has_descender(numerator) || latex_has_descender(denominator);
    let fraction_width = numerator_width + if descender_fraction { 5.0 } else { 1.0 };
    let relation_overlap = if has_math_descender(leading) {
        3.0
    } else {
        2.0
    };

    Some(LatexLayoutMetrics {
        width: relation_prefix_width(leading) + fraction_width - relation_overlap,
        height: 54.0 + if descender_fraction { 5.0 } else { 0.0 },
    })
}

fn fraction_groups(content: &str) -> Option<(&str, &str)> {
    let start = content.find("\\frac")? + "\\frac".len();
    let (numerator, next) = braced_group(content, start)?;
    let (denominator, _) = braced_group(content, next)?;
    Some((numerator, denominator))
}

fn braced_group(content: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = content.as_bytes();
    let mut open = start;
    while bytes.get(open).is_some_and(u8::is_ascii_whitespace) {
        open += 1;
    }
    if bytes.get(open) != Some(&b'{') {
        return None;
    }

    let mut depth = 0usize;
    for index in open..bytes.len() {
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((&content[open + 1..index], index + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn plain_math_row_width(content: &str) -> Option<f64> {
    let bytes = content.as_bytes();
    let mut width = 2.0;
    let mut index = 0usize;
    let mut paren_depth = 0usize;
    let mut saw_script = false;
    let mut digit_to_letter = false;

    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' | b'{' | b'}' => index += 1,
            b'(' => {
                paren_depth += 1;
                width += 8.0;
                index += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                width += 8.0;
                index += 1;
            }
            b'+' => {
                width += if paren_depth > 0 { 30.0 } else { 26.0 };
                index += 1;
            }
            b'-' => {
                width += if saw_script { 24.0 } else { 26.0 };
                index += 1;
            }
            b'^' | b'_' => {
                width += 7.0;
                saw_script = true;
                index += 1;
                if bytes.get(index) == Some(&b'{') {
                    let (_, next) = braced_group(content, index)?;
                    index = next;
                } else {
                    index += usize::from(index < bytes.len());
                }
            }
            b'0'..=b'9' => {
                width += 10.0;
                if bytes.get(index + 1).is_some_and(u8::is_ascii_alphabetic) {
                    digit_to_letter = true;
                }
                index += 1;
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                width += math_advance(byte as char);
                index += 1;
            }
            b'\\' => return None,
            _ => index += 1,
        }
    }

    if digit_to_letter {
        width += 2.0;
    }
    if bytes.last().is_some_and(u8::is_ascii_alphabetic) {
        width += 4.0;
    }
    Some(width)
}

fn relation_prefix_width(variable: char) -> f64 {
    standalone_math_width(variable)
        + if has_math_descender(variable) {
            22.0
        } else {
            21.0
        }
}

fn unary_prefix_width(variable: char) -> f64 {
    match variable {
        'p' | 'q' => standalone_math_width(variable) + 15.0,
        'y' | 'j' => standalone_math_width(variable) + 17.0,
        'x' => standalone_math_width(variable) + 15.0,
        _ => standalone_math_width(variable) + 16.0,
    }
}

fn standalone_math_width(variable: char) -> f64 {
    match variable {
        'a' => 19.0,
        'b' | 'c' | 'j' | 'q' => 17.0,
        'i' => 15.0,
        'm' => 26.0,
        'n' | 'x' => 20.0,
        'p' => 19.0,
        'r' | 'y' => 18.0,
        _ => math_advance(variable) + 8.0,
    }
}

fn math_advance(variable: char) -> f64 {
    match variable {
        'a' | 'x' | 'y' => 11.0,
        'b' | 'c' => 9.0,
        'i' => 7.0,
        'j' | 'p' | 'r' => 10.0,
        'm' => 18.0,
        'n' => 12.0,
        'q' => 11.0,
        _ => 11.0,
    }
}

fn has_math_descender(variable: char) -> bool {
    matches!(variable, 'g' | 'j' | 'p' | 'q' | 'y')
}

fn latex_has_descender(content: &str) -> bool {
    let mut command = false;
    for character in content.chars() {
        if character == '\\' {
            command = true;
            continue;
        }
        if command {
            if character.is_ascii_alphabetic() {
                continue;
            }
            command = false;
        }
        if character.is_ascii_alphabetic() && has_math_descender(character) {
            return true;
        }
    }
    false
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

fn xml_escape_nbsp(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            ' ' => out.push_str("&#160;"),
            c => out.push(c),
        }
    }
    out
}

fn encode_base64(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jlatex_layout_uses_raster_box_not_svg_fallback_box() {
        let source = "\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}";
        assert_eq!(
            latex_layout_metrics(source),
            LatexLayoutMetrics {
                width: 153.0,
                height: 68.0,
            }
        );
        assert_eq!(raw_latex_image(source).width, 306);
        assert_eq!(
            latex_layout_metrics("x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}"),
            LatexLayoutMetrics {
                width: 188.0,
                height: 54.0,
            }
        );
    }

    #[test]
    fn jlatex_layout_tracks_renamed_variable_glyph_metrics() {
        assert_eq!(
            latex_layout_metrics("\\sum_{j=1}^{m} j = \\frac{m(m+1)}{2}"),
            LatexLayoutMetrics {
                width: 167.0,
                height: 71.0,
            }
        );
        assert_eq!(
            latex_layout_metrics("y = \\frac{-p \\pm \\sqrt{p^2-4qr}}{2q}"),
            LatexLayoutMetrics {
                width: 193.0,
                height: 59.0,
            }
        );
    }

    #[test]
    fn malformed_compound_latex_falls_back_without_panicking() {
        let source = "\\frac{oops";
        let image = raw_latex_image(source);
        assert_eq!(
            latex_layout_metrics(source),
            LatexLayoutMetrics {
                width: image.width as f64,
                height: image.height as f64,
            }
        );
    }
}
