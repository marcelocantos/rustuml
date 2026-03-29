// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Text metrics — accurate text dimensions using embedded Liberation Sans font.

use std::sync::LazyLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

/// Embedded Liberation Sans Regular font (Apache 2.0 licensed).
static FONT_DATA: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");

static FONT: LazyLock<FontRef<'static>> =
    LazyLock::new(|| FontRef::try_from_slice(FONT_DATA).expect("failed to load embedded font"));

/// Accurate width of a string in pixels at the given font size.
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
}
