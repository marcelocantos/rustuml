// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Creole-aware `<text>` emission.
//!
//! PlantUML emits styled text by **splitting on style transitions and
//! producing one `<text>` element per uniform run** (no `<tspan>` wrappers
//! anywhere in the golden corpus). Each text element advertises its
//! `textLength` and gets a calculated `x` offset, so the runs line up
//! seamlessly. This module owns that emission shape so every renderer
//! routes through one path.
//!
//! Width calculation currently uses PlantUML's sans-serif metrics for all
//! segments — monospace runs will mismatch on `textLength` until those
//! metrics land. The structural shape (text content, font-family/style
//! attributes, NBSP conversion, per-segment positioning) is correct
//! regardless.

use std::fmt::Write;

use crate::creole::{self, Segment, Style};
use crate::plantuml_metrics as pm;

/// Effective styling for a base font that the caller controls. Each call to
/// [`emit_text`] starts from this base; segment-level styles add on top.
#[derive(Debug, Clone)]
pub struct TextBase<'a> {
    pub x: f64,
    pub y: f64,
    pub font_size: u32,
    pub font_family: &'a str,
    pub fill: &'a str,
    /// Whether the base text is bold (e.g. class entity names emit bold by
    /// default; creole inside still toggles bold normally).
    pub bold: bool,
    /// Whether the base text is italic.
    pub italic: bool,
    /// When true, treat `__` as literal (class-entity labels).
    pub skip_underline: bool,
}

/// Emit one or more `<text>` elements covering `content` with creole markup
/// resolved. Writes to `buf`. Returns the total advance width.
pub fn emit_text(buf: &mut String, content: &str, base: &TextBase<'_>) -> f64 {
    let segments = if base.skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    emit_segments(buf, &segments, base)
}

/// Pre-computed widths for the segments. Useful when the caller needs the
/// total advance for layout before deciding `x`.
pub fn total_width(content: &str, base: &TextBase<'_>) -> f64 {
    let segments = if base.skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    segments
        .iter()
        .map(|seg| segment_width(seg, base))
        .sum::<f64>()
}

/// Shared emission body: walks segments, writes one `<text>` per segment,
/// advancing `x` by each segment's width. Returns the sum of widths.
fn emit_segments(buf: &mut String, segments: &[Segment], base: &TextBase<'_>) -> f64 {
    if segments.is_empty() {
        // Nothing to emit. Still produce an empty <text> with zero width so
        // surrounding layout stays consistent — matches PlantUML behaviour
        // for empty labels.
        write_text_element(buf, "", base, &Style::default(), base.x, 0.0);
        return 0.0;
    }

    let mut x = base.x;
    let mut total = 0.0;
    for seg in segments {
        let w = segment_width(seg, base);
        write_text_element(buf, &seg.text, base, &seg.style, x, w);
        x += w;
        total += w;
    }
    total
}

/// Width of one segment under the effective styling. Uses sans-serif
/// metrics; monospace and custom font sizes will need a richer metric
/// surface to be byte-for-byte accurate.
fn segment_width(seg: &Segment, base: &TextBase<'_>) -> f64 {
    let raw = unescape_for_metrics(&seg.text);
    let bold = base.bold || seg.style.bold;
    let font_size = seg.style.size.map(|s| s as f64).unwrap_or(base.font_size as f64);
    pm::text_width(&raw, font_size, bold)
}

/// Reverse XML escaping for metric calculation — PlantUML measures text
/// against the source string, not its escaped form.
fn unescape_for_metrics(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn write_text_element(
    buf: &mut String,
    content: &str,
    base: &TextBase<'_>,
    style: &Style,
    x: f64,
    width: f64,
) {
    let bold = base.bold || style.bold;
    let italic = base.italic || style.italic;
    let font_family = if style.monospace {
        "monospace"
    } else if let Some(f) = style.font_family.as_deref() {
        f
    } else {
        base.font_family
    };
    let font_size = style.size.unwrap_or(base.font_size);
    let fill = style.fill.as_deref().unwrap_or(base.fill);

    let mut decorations: Vec<&str> = Vec::new();
    if style.underline {
        decorations.push("underline");
    }
    if style.line_through {
        decorations.push("line-through");
    }
    if style.wavy_underline {
        decorations.push("wavy underline");
    }
    let text_decoration = if decorations.is_empty() {
        String::new()
    } else {
        format!(r#" text-decoration="{}""#, decorations.join(" "))
    };

    let weight_attr = if bold { r#" font-weight="700""# } else { "" };
    let style_attr = if italic { r#" font-style="italic""# } else { "" };
    let baseline_attr = match style.baseline_shift {
        Some(shift) => format!(r#" baseline-shift="{shift}" font-size="0.7em""#),
        None => String::new(),
    };

    // Build a deterministic attribute order matching PlantUML's golden
    // output: fill, font-family, font-size, font-style, font-weight,
    // lengthAdjust, text-decoration, textLength, x, y.
    write!(
        buf,
        r#"<text fill="{fill}" font-family="{font_family}" font-size="{font_size}"{style_attr}{weight_attr}{baseline_attr} lengthAdjust="spacing"{text_decoration} textLength="{tl}" x="{x_s}" y="{y_s}">{content}</text>"#,
        tl = pm::fmt_coord(width),
        x_s = pm::fmt_coord(x),
        y_s = pm::fmt_coord(base.y),
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(x: f64, y: f64) -> TextBase<'static> {
        TextBase {
            x,
            y,
            font_size: 12,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            skip_underline: false,
        }
    }

    #[test]
    fn plain_text_emits_one_element() {
        let mut buf = String::new();
        emit_text(&mut buf, "hello", &base(10.0, 20.0));
        assert!(buf.starts_with(r##"<text fill="#000000" font-family="sans-serif" font-size="12""##));
        assert!(buf.contains(">hello</text>"));
        assert_eq!(buf.matches("<text").count(), 1);
    }

    #[test]
    fn uniform_bold_lifts_to_text_element() {
        let mut buf = String::new();
        emit_text(&mut buf, "**bold**", &base(0.0, 0.0));
        assert_eq!(buf.matches("<text").count(), 1);
        assert!(buf.contains(r#"font-weight="700""#));
        assert!(buf.contains(">bold</text>"));
        assert!(!buf.contains("**"));
    }

    #[test]
    fn mixed_styles_split_into_multiple_elements() {
        let mut buf = String::new();
        emit_text(&mut buf, "<color:blue>**field**</color>: String", &base(0.0, 0.0));
        assert_eq!(buf.matches("<text").count(), 2);
        // First element: blue + bold + "field"
        assert!(buf.contains(r#"fill="blue""#));
        assert!(buf.contains(r#"font-weight="700""#));
        assert!(buf.contains(">field</text>"));
        // Second element: plain + ": String"
        assert!(buf.contains(">: String</text>"));
    }

    #[test]
    fn no_tspans_anywhere() {
        // PlantUML's golden corpus never contains <tspan>. Our output must
        // not either, regardless of how complex the input is.
        let inputs = [
            "**bold**",
            "//italic//",
            "__under__",
            "--strike--",
            r#"""mono"""#,
            "<color:red>**bold red**</color>",
            "**//bold italic//**",
            "before **bold** after",
        ];
        for input in inputs {
            let mut buf = String::new();
            emit_text(&mut buf, input, &base(0.0, 0.0));
            assert!(!buf.contains("<tspan"), "tspan in output for {input:?}: {buf}");
        }
    }
}
