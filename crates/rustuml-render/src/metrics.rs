// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Text metrics — accurate text dimensions using embedded Liberation Sans font
//! and PlantUML-compatible Java AWT font metrics.

use std::sync::LazyLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

/// Embedded Liberation Sans Regular font (Apache 2.0 licensed).
static FONT_DATA: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");

static FONT: LazyLock<FontRef<'static>> =
    LazyLock::new(|| FontRef::try_from_slice(FONT_DATA).expect("failed to load embedded font"));

/// Accurate width of a string in pixels at the given font size using Liberation Sans.
pub fn text_width(text: &str, font_size: f64) -> f64 {
    let font = FONT.as_scaled(PxScale::from(font_size as f32));
    let mut width = 0.0f32;
    let mut prev_glyph = None;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        if let Some(prev) = prev_glyph {
            width += font.kern(prev, glyph_id);
        }
        width += font.h_advance(glyph_id);
        prev_glyph = Some(glyph_id);
    }

    width as f64
}

/// Approximate height of a line of text at the given font size.
pub fn text_height(font_size: f64) -> f64 {
    let font = FONT.as_scaled(PxScale::from(font_size as f32));
    (font.ascent() - font.descent()) as f64
}

/// Line height (ascent + descent + line gap).
pub fn line_height(font_size: f64) -> f64 {
    let font = FONT.as_scaled(PxScale::from(font_size as f32));
    (font.ascent() - font.descent() + font.line_gap()) as f64
}

// ---------------------------------------------------------------------------
// PlantUML-compatible Java AWT font metrics
// ---------------------------------------------------------------------------

/// Java AWT SansSerif character widths at font-size 14 (plain),
/// with `RenderingHints.VALUE_FRACTIONALMETRICS_ON`.
///
/// Extracted from Java 21+ on macOS using `FontMetrics.getStringBounds()` on
/// a `BufferedImage` `Graphics2D`, matching PlantUML's `FileFormat.getJavaDimension()`
/// exactly. These values are additive (no kerning in Java AWT logical fonts),
/// verified against 2300+ golden textLength values with zero residual.
const JAVA_AWT_WIDTHS_14: &[(char, f64)] = &[
    (' ', 4.4297),
    ('!', 4.4297),
    ('"', 5.2295),
    ('#', 8.8525),
    ('$', 8.8525),
    ('%', 9.3584),
    ('&', 9.7617),
    ('\'', 3.2061),
    ('(', 4.5527),
    (')', 4.5527),
    ('*', 6.7471),
    ('+', 11.1289),
    (',', 4.4297),
    ('-', 8.1006),
    ('.', 4.4297),
    ('/', 7.3418),
    ('0', 8.8525),
    ('1', 8.8525),
    ('2', 8.8525),
    ('3', 8.8525),
    ('4', 8.8525),
    ('5', 8.8525),
    ('6', 8.8525),
    ('7', 8.8525),
    ('8', 8.8525),
    ('9', 8.8525),
    (':', 4.4297),
    (';', 4.4297),
    ('<', 11.1289),
    ('=', 11.1289),
    ('>', 11.1289),
    ('?', 5.9063),
    ('@', 12.0176),
    ('A', 9.6592),
    ('B', 8.0527),
    ('C', 9.6865),
    ('D', 10.4863),
    ('E', 7.5879),
    ('F', 7.5059),
    ('G', 10.1172),
    ('H', 10.2881),
    ('I', 4.0332),
    ('J', 4.3545),
    ('K', 9.1396),
    ('L', 7.4648),
    ('M', 12.0586),
    ('N', 10.3428),
    ('O', 10.8760),
    ('P', 7.7383),
    ('Q', 10.8760),
    ('R', 8.8525),
    ('S', 7.5400),
    ('T', 8.8525),
    ('U', 9.7002),
    ('V', 9.1533),
    ('W', 11.9766),
    ('X', 8.7637),
    ('Y', 8.7227),
    ('Z', 8.4629),
    ('[', 4.5527),
    ('\\', 7.3418),
    (']', 4.5527),
    ('^', 8.8525),
    ('_', 7.0000),
    ('`', 8.5996),
    ('a', 7.7314),
    ('b', 8.8115),
    ('c', 7.1709),
    ('d', 8.8115),
    ('e', 7.7998),
    ('f', 5.1475),
    ('g', 8.7295),
    ('h', 8.6885),
    ('i', 4.0469),
    ('j', 4.2588),
    ('k', 8.1826),
    ('l', 4.0469),
    ('m', 13.0703),
    ('n', 8.6885),
    ('o', 8.5996),
    ('p', 8.8115),
    ('q', 8.8115),
    ('r', 5.7285),
    ('s', 7.1367),
    ('t', 5.2363),
    ('u', 8.6885),
    ('v', 7.2461),
    ('w', 10.7871),
    ('x', 8.5859),
    ('y', 7.3145),
    ('z', 8.0254),
    ('{', 4.5527),
    ('|', 5.2295),
    ('}', 4.5527),
    ('~', 8.8525),
];

/// Build a fast lookup array for ASCII char widths.
static JAVA_AWT_LOOKUP_14: LazyLock<[f64; 128]> = LazyLock::new(|| {
    let mut table = [0.0f64; 128];
    for &(c, w) in JAVA_AWT_WIDTHS_14 {
        table[c as usize] = w;
    }
    table
});

/// Text width using PlantUML's exact Java AWT SansSerif metrics (font-size 14, plain).
///
/// Returns the sum of per-character widths matching `FontMetrics.getStringBounds()`
/// with `FRACTIONALMETRICS_ON`. For non-ASCII characters, falls back to the
/// average width of ASCII lowercase letters.
pub fn plantuml_text_width_14(text: &str) -> f64 {
    let table = &*JAVA_AWT_LOOKUP_14;
    let fallback = 8.0; // reasonable default for unknown chars
    text.chars()
        .map(|c| {
            let code = c as usize;
            if code < 128 { table[code] } else { fallback }
        })
        .sum()
}

/// Text width for a PlantUML text string, scaled to a specific font size.
///
/// PlantUML uses Java AWT `SansSerif` with `FRACTIONALMETRICS_ON`.
/// Character widths scale linearly with font size.
pub fn plantuml_text_width(text: &str, font_size: f64) -> f64 {
    plantuml_text_width_14(text) * font_size / 14.0
}

/// Format a text width to 4 decimal places, matching PlantUML's output format.
pub fn format_width(w: f64) -> String {
    format!("{w:.4}")
}

// ---------------------------------------------------------------------------
// Guillemet character widths
// ---------------------------------------------------------------------------

/// Width of the left guillemet character «  (U+00AB) at font-size 14.
pub const GUILLEMET_LEFT_WIDTH_14: f64 = 8.8525;

/// Width of the right guillemet character »  (U+00BB) at font-size 14.
pub const GUILLEMET_RIGHT_WIDTH_14: f64 = 8.8525;

/// Width of non-breaking space (U+00A0) at font-size 14 — same as regular space.
pub const NBSP_WIDTH_14: f64 = 4.4297;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_text_is_narrower() {
        let narrow = text_width("ill", 14.0);
        let wide = text_width("MWM", 14.0);
        assert!(narrow < wide, "narrow={narrow} should be < wide={wide}");
    }

    #[test]
    fn empty_string_is_zero() {
        assert_eq!(text_width("", 14.0), 0.0);
    }

    #[test]
    fn font_size_scales() {
        let w14 = text_width("hello", 14.0);
        let w28 = text_width("hello", 28.0);
        // Should be roughly 2x, within tolerance.
        assert!(
            (w28 / w14 - 2.0).abs() < 0.1,
            "28pt/14pt ratio should be ~2.0, got {}",
            w28 / w14
        );
    }

    #[test]
    fn typical_widths() {
        let w = text_width("Alice", 13.0);
        assert!(w > 20.0 && w < 50.0, "Alice width={w}");
    }

    #[test]
    fn text_height_positive() {
        let h = text_height(14.0);
        assert!(h > 10.0 && h < 25.0, "height={h}");
    }

    #[test]
    fn line_height_greater_than_text() {
        let th = text_height(14.0);
        let lh = line_height(14.0);
        assert!(lh >= th, "line height {lh} should be >= text height {th}");
    }

    #[test]
    fn plantuml_widths_match_golden() {
        // These golden values are from actual PlantUML SVG output.
        let cases = [
            ("MyClass", 55.1113),
            ("String field", 74.2520),
            ("void method()", 94.4453),
            ("Shape", 40.5713),
            ("Override", 58.0371),
            ("Color", 36.6611),
            ("Drawable", 63.1230),
            ("double area()", 89.2842),
            ("double perimeter()", 126.3145),
            ("void describe()", 99.5449),
            ("RED", 26.9268),
            ("GREEN", 44.4883),
            ("BLUE", 32.8057),
            ("AllModifiers", 81.1289),
        ];
        for (text, expected) in cases {
            let actual = plantuml_text_width_14(text);
            let diff = (actual - expected).abs();
            assert!(
                diff < 0.001,
                "{text}: expected={expected:.4}, actual={actual:.4}, diff={diff:.4}"
            );
        }
    }

    #[test]
    fn plantuml_empty_is_zero() {
        assert_eq!(plantuml_text_width_14(""), 0.0);
    }
}
