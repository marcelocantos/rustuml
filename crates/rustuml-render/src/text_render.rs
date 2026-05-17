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
    /// Whether the base text is underlined (e.g. static class members).
    /// OR-merged with any creole-driven underline on the segment style.
    pub underline: bool,
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

/// Width of `content` after creole resolution — the value a renderer needs
/// to size boxes around a label. Per-segment styling (monospace vs sans-
/// serif, bold, custom size) is honoured by routing through `total_width`.
pub fn measure(content: &str, font_size: f64, bold: bool) -> f64 {
    let base = TextBase {
        x: 0.0,
        y: 0.0,
        font_size: font_size as u32,
        font_family: "sans-serif",
        fill: "#000000",
        bold,
        italic: false,
        underline: false,
        skip_underline: false,
    };
    total_width(content, &base)
}

/// Height of `content` after creole resolution — the value a renderer
/// needs to size boxes around a label vertically. Returns the max of
/// per-segment heights (mono vs sans-serif) at the resolved font size.
pub fn label_height(content: &str, font_size: f64) -> f64 {
    let segments = creole::parse_segments(content);
    if segments.is_empty() {
        return pm::text_height(font_size);
    }
    segments
        .iter()
        .map(|seg| {
            let size = seg.style.size.map(|s| s as f64).unwrap_or(font_size);
            if seg.style.monospace {
                pm::mono_text_height(size)
            } else {
                pm::text_height(size)
            }
        })
        .fold(0.0f64, f64::max)
}

/// Ascent for vertical positioning of the text baseline within a label box.
/// Picks the max of per-segment ascents (matches PlantUML's behaviour for
/// mixed-font lines).
pub fn label_ascent(content: &str, font_size: f64) -> f64 {
    let segments = creole::parse_segments(content);
    if segments.is_empty() {
        return pm::ascent(font_size);
    }
    segments
        .iter()
        .map(|seg| {
            let size = seg.style.size.map(|s| s as f64).unwrap_or(font_size);
            if seg.style.monospace {
                pm::mono_ascent(size)
            } else {
                pm::ascent(size)
            }
        })
        .fold(0.0f64, f64::max)
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

/// Width of one segment under the effective styling. Routes to the
/// monospace metric path when the segment is monospaced; otherwise uses
/// sans-serif.
fn segment_width(seg: &Segment, base: &TextBase<'_>) -> f64 {
    let raw = unescape_for_metrics(&seg.text);
    let bold = base.bold || seg.style.bold;
    let font_size = seg
        .style
        .size
        .map(|s| s as f64)
        .unwrap_or(base.font_size as f64);
    if seg.style.monospace {
        pm::mono_text_width(&raw, font_size)
    } else {
        pm::text_width(&raw, font_size, bold)
    }
}

/// Reverse XML escaping for metric calculation — PlantUML measures text
/// against the source string, not its escaped form.
fn unescape_for_metrics(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

/// Normalise a colour spec to PlantUML's preferred form: lower-case hex
/// (`#RRGGBB`) for known named colours, pass-through otherwise.
///
/// PlantUML emits resolved hex colours in SVG output even when the source
/// uses HTML colour names — `<color:blue>` becomes `fill="#0000FF"`.
fn normalize_color(s: &str) -> String {
    if s.starts_with('#') {
        return s.to_string();
    }
    let lower = s.to_ascii_lowercase();
    match css_color_hex(&lower) {
        Some(hex) => hex.to_string(),
        None => s.to_string(),
    }
}

/// CSS3 named colours, lowercased keys. PlantUML accepts the same names
/// (Java AWT's `Color.decode` plus extended palette).
fn css_color_hex(name: &str) -> Option<&'static str> {
    // Listed in case-insensitive lookup order; values are uppercase
    // `#RRGGBB` to match PlantUML's golden SVG output.
    match name {
        "aliceblue" => Some("#F0F8FF"),
        "antiquewhite" => Some("#FAEBD7"),
        "aqua" | "cyan" => Some("#00FFFF"),
        "aquamarine" => Some("#7FFFD4"),
        "azure" => Some("#F0FFFF"),
        "beige" => Some("#F5F5DC"),
        "bisque" => Some("#FFE4C4"),
        "black" => Some("#000000"),
        "blanchedalmond" => Some("#FFEBCD"),
        "blue" => Some("#0000FF"),
        "blueviolet" => Some("#8A2BE2"),
        "brown" => Some("#A52A2A"),
        "burlywood" => Some("#DEB887"),
        "cadetblue" => Some("#5F9EA0"),
        "chartreuse" => Some("#7FFF00"),
        "chocolate" => Some("#D2691E"),
        "coral" => Some("#FF7F50"),
        "cornflowerblue" => Some("#6495ED"),
        "cornsilk" => Some("#FFF8DC"),
        "crimson" => Some("#DC143C"),
        "darkblue" => Some("#00008B"),
        "darkcyan" => Some("#008B8B"),
        "darkgoldenrod" => Some("#B8860B"),
        "darkgray" | "darkgrey" => Some("#A9A9A9"),
        "darkgreen" => Some("#006400"),
        "darkkhaki" => Some("#BDB76B"),
        "darkmagenta" => Some("#8B008B"),
        "darkolivegreen" => Some("#556B2F"),
        "darkorange" => Some("#FF8C00"),
        "darkorchid" => Some("#9932CC"),
        "darkred" => Some("#8B0000"),
        "darksalmon" => Some("#E9967A"),
        "darkseagreen" => Some("#8FBC8F"),
        "darkslateblue" => Some("#483D8B"),
        "darkslategray" | "darkslategrey" => Some("#2F4F4F"),
        "darkturquoise" => Some("#00CED1"),
        "darkviolet" => Some("#9400D3"),
        "deeppink" => Some("#FF1493"),
        "deepskyblue" => Some("#00BFFF"),
        "dimgray" | "dimgrey" => Some("#696969"),
        "dodgerblue" => Some("#1E90FF"),
        "firebrick" => Some("#B22222"),
        "floralwhite" => Some("#FFFAF0"),
        "forestgreen" => Some("#228B22"),
        "fuchsia" | "magenta" => Some("#FF00FF"),
        "gainsboro" => Some("#DCDCDC"),
        "ghostwhite" => Some("#F8F8FF"),
        "gold" => Some("#FFD700"),
        "goldenrod" => Some("#DAA520"),
        "gray" | "grey" => Some("#808080"),
        "green" => Some("#008000"),
        "greenyellow" => Some("#ADFF2F"),
        "honeydew" => Some("#F0FFF0"),
        "hotpink" => Some("#FF69B4"),
        "indianred" => Some("#CD5C5C"),
        "indigo" => Some("#4B0082"),
        "ivory" => Some("#FFFFF0"),
        "khaki" => Some("#F0E68C"),
        "lavender" => Some("#E6E6FA"),
        "lavenderblush" => Some("#FFF0F5"),
        "lawngreen" => Some("#7CFC00"),
        "lemonchiffon" => Some("#FFFACD"),
        "lightblue" => Some("#ADD8E6"),
        "lightcoral" => Some("#F08080"),
        "lightcyan" => Some("#E0FFFF"),
        "lightgoldenrodyellow" => Some("#FAFAD2"),
        "lightgray" | "lightgrey" => Some("#D3D3D3"),
        "lightgreen" => Some("#90EE90"),
        "lightpink" => Some("#FFB6C1"),
        "lightsalmon" => Some("#FFA07A"),
        "lightseagreen" => Some("#20B2AA"),
        "lightskyblue" => Some("#87CEFA"),
        "lightslategray" | "lightslategrey" => Some("#778899"),
        "lightsteelblue" => Some("#B0C4DE"),
        "lightyellow" => Some("#FFFFE0"),
        "lime" => Some("#00FF00"),
        "limegreen" => Some("#32CD32"),
        "linen" => Some("#FAF0E6"),
        "maroon" => Some("#800000"),
        "mediumaquamarine" => Some("#66CDAA"),
        "mediumblue" => Some("#0000CD"),
        "mediumorchid" => Some("#BA55D3"),
        "mediumpurple" => Some("#9370DB"),
        "mediumseagreen" => Some("#3CB371"),
        "mediumslateblue" => Some("#7B68EE"),
        "mediumspringgreen" => Some("#00FA9A"),
        "mediumturquoise" => Some("#48D1CC"),
        "mediumvioletred" => Some("#C71585"),
        "midnightblue" => Some("#191970"),
        "mintcream" => Some("#F5FFFA"),
        "mistyrose" => Some("#FFE4E1"),
        "moccasin" => Some("#FFE4B5"),
        "navajowhite" => Some("#FFDEAD"),
        "navy" => Some("#000080"),
        "oldlace" => Some("#FDF5E6"),
        "olive" => Some("#808000"),
        "olivedrab" => Some("#6B8E23"),
        "orange" => Some("#FFA500"),
        "orangered" => Some("#FF4500"),
        "orchid" => Some("#DA70D6"),
        "palegoldenrod" => Some("#EEE8AA"),
        "palegreen" => Some("#98FB98"),
        "paleturquoise" => Some("#AFEEEE"),
        "palevioletred" => Some("#DB7093"),
        "papayawhip" => Some("#FFEFD5"),
        "peachpuff" => Some("#FFDAB9"),
        "peru" => Some("#CD853F"),
        "pink" => Some("#FFC0CB"),
        "plum" => Some("#DDA0DD"),
        "powderblue" => Some("#B0E0E6"),
        "purple" => Some("#800080"),
        "rebeccapurple" => Some("#663399"),
        "red" => Some("#FF0000"),
        "rosybrown" => Some("#BC8F8F"),
        "royalblue" => Some("#4169E1"),
        "saddlebrown" => Some("#8B4513"),
        "salmon" => Some("#FA8072"),
        "sandybrown" => Some("#F4A460"),
        "seagreen" => Some("#2E8B57"),
        "seashell" => Some("#FFF5EE"),
        "sienna" => Some("#A0522D"),
        "silver" => Some("#C0C0C0"),
        "skyblue" => Some("#87CEEB"),
        "slateblue" => Some("#6A5ACD"),
        "slategray" | "slategrey" => Some("#708090"),
        "snow" => Some("#FFFAFA"),
        "springgreen" => Some("#00FF7F"),
        "steelblue" => Some("#4682B4"),
        "tan" => Some("#D2B48C"),
        "teal" => Some("#008080"),
        "thistle" => Some("#D8BFD8"),
        "tomato" => Some("#FF6347"),
        "turquoise" => Some("#40E0D0"),
        "violet" => Some("#EE82EE"),
        "wheat" => Some("#F5DEB3"),
        "white" => Some("#FFFFFF"),
        "whitesmoke" => Some("#F5F5F5"),
        "yellow" => Some("#FFFF00"),
        "yellowgreen" => Some("#9ACD32"),
        _ => None,
    }
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
    let raw_fill = style.fill.as_deref().unwrap_or(base.fill);
    let fill = normalize_color(raw_fill);

    let mut decorations: Vec<&str> = Vec::new();
    if style.underline || base.underline {
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
    let style_attr = if italic {
        r#" font-style="italic""#
    } else {
        ""
    };
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
            underline: false,
            skip_underline: false,
        }
    }

    #[test]
    fn plain_text_emits_one_element() {
        let mut buf = String::new();
        emit_text(&mut buf, "hello", &base(10.0, 20.0));
        assert!(
            buf.starts_with(r##"<text fill="#000000" font-family="sans-serif" font-size="12""##)
        );
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
        emit_text(
            &mut buf,
            "<color:blue>**field**</color>: String",
            &base(0.0, 0.0),
        );
        assert_eq!(buf.matches("<text").count(), 2);
        // First element: blue + bold + "field"
        assert!(buf.contains(r##"fill="#0000FF""##));
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
            assert!(
                !buf.contains("<tspan"),
                "tspan in output for {input:?}: {buf}"
            );
        }
    }
}
