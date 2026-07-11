// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Text metrics — accurate text dimensions using embedded Liberation Sans font
//! and PlantUML-compatible Java AWT font metrics.

use std::fmt::Write;
use std::sync::LazyLock;

use ab_glyph::{Font, FontRef, OutlineCurve, PxScale, ScaleFont, point};

/// Embedded Liberation Sans Regular font (SIL Open Font License 1.1).
static FONT_DATA: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");

static FONT: LazyLock<FontRef<'static>> =
    LazyLock::new(|| FontRef::try_from_slice(FONT_DATA).expect("failed to load embedded font"));

/// DejaVu Sans Mono Bold, the AWT logical `Monospaced` face used by PlantUML's
/// default circled-character font. See `fonts/DEJAVU-LICENSE`.
static CIRCLED_FONT_DATA: &[u8] = include_bytes!("../fonts/DejaVuSansMono-Bold-Powerline.ttf");
static CIRCLED_FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::try_from_slice(CIRCLED_FONT_DATA).expect("failed to load circled-character font")
});

/// `UnusedSpace.getUnusedSpace` centers for printable ASCII in AWT logical
/// Monospaced Bold 17. Extracted from PlantUML `UnusedSpace` on the same JVM
/// build used for the golden corpus. Values are quarter-pixel raster coordinates.
const JAVA_AWT_MONO_BOLD_17_CENTERS: &[(u8, f64, f64)] = &[
    (0x21, 4.5, -7.0),
    (0x22, 4.5, -11.0),
    (0x23, 4.5, -7.0),
    (0x24, 4.5, -6.0),
    (0x25, 5.0, -7.0),
    (0x26, 5.5, -7.0),
    (0x27, 4.5, -11.0),
    (0x28, 6.0, -6.0),
    (0x29, 3.0, -6.0),
    (0x2A, 4.5, -6.0),
    (0x2B, 5.0, -6.0),
    (0x2C, 4.0, -1.0),
    (0x2D, 4.5, -5.5),
    (0x2E, 4.5, -2.5),
    (0x2F, 4.5, -6.0),
    (0x30, 5.0, -7.0),
    (0x31, 5.0, -7.0),
    (0x32, 4.0, -7.0),
    (0x33, 4.0, -7.0),
    (0x34, 5.25, -7.0),
    (0x35, 4.5, -6.75),
    (0x36, 5.5, -7.0),
    (0x37, 5.0, -7.25),
    (0x38, 4.5, -7.0),
    (0x39, 3.5, -7.0),
    (0x3A, 4.5, -5.5),
    (0x3B, 4.5, -4.0),
    (0x3C, 5.75, -6.0),
    (0x3D, 4.5, -6.0),
    (0x3E, 3.25, -6.0),
    (0x3F, 4.5, -7.5),
    (0x40, 5.0, -5.25),
    (0x41, 4.75, -6.0),
    (0x42, 4.0, -7.25),
    (0x43, 6.0, -7.0),
    (0x44, 4.0, -7.0),
    (0x45, 5.0, -7.0),
    (0x46, 5.0, -7.0),
    (0x47, 5.5, -7.0),
    (0x48, 5.0, -7.0),
    (0x49, 4.5, -7.0),
    (0x4A, 4.75, -6.5),
    (0x4B, 5.0, -7.0),
    (0x4C, 5.0, -7.0),
    (0x4D, 4.5, -7.0),
    (0x4E, 4.5, -7.0),
    (0x4F, 5.0, -7.0),
    (0x50, 4.5, -7.0),
    (0x51, 5.5, -6.0),
    (0x52, 5.5, -7.0),
    (0x53, 4.0, -7.0),
    (0x54, 4.5, -7.75),
    (0x55, 4.5, -7.25),
    (0x56, 4.5, -7.5),
    (0x57, 4.5, -7.75),
    (0x58, 5.0, -7.0),
    (0x59, 4.75, -8.0),
    (0x5A, 4.5, -7.0),
    (0x5B, 5.0, -6.0),
    (0x5C, 4.5, -6.0),
    (0x5D, 4.5, -6.0),
    (0x5E, 4.5, -9.0),
    (0x5F, 4.5, 2.0),
    (0x60, 3.5, -13.0),
    (0x61, 4.75, -5.25),
    (0x62, 4.0, -7.0),
    (0x63, 5.5, -5.5),
    (0x64, 5.5, -7.25),
    (0x65, 5.25, -5.25),
    (0x66, 6.0, -7.5),
    (0x67, 5.0, -3.5),
    (0x68, 4.5, -7.5),
    (0x69, 5.0, -7.5),
    (0x6A, 4.0, -6.0),
    (0x6B, 5.0, -7.5),
    (0x6C, 4.5, -7.5),
    (0x6D, 4.5, -5.5),
    (0x6E, 4.5, -5.5),
    (0x6F, 4.5, -5.25),
    (0x70, 4.5, -3.5),
    (0x71, 5.0, -3.5),
    (0x72, 5.5, -5.5),
    (0x73, 4.5, -5.5),
    (0x74, 6.0, -7.0),
    (0x75, 4.5, -5.25),
    (0x76, 4.5, -6.5),
    (0x77, 4.5, -6.5),
    (0x78, 4.5, -5.5),
    (0x79, 4.5, -3.5),
    (0x7A, 5.0, -5.5),
    (0x7B, 6.0, -6.0),
    (0x7C, 5.0, -5.0),
    (0x7D, 3.0, -6.0),
    (0x7E, 4.5, -5.5),
];

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

/// Convert one embedded-font glyph to the SVG path used for PlantUML's
/// `UCenteredCharacter`. Default-size ASCII uses the extracted `UnusedSpace`
/// table; other sizes and Unicode glyphs use the same quarter-pixel minimax
/// algorithm over the embedded font raster. Both paths then apply
/// `DriverCenteredCharacterSvg`'s half-pixel translation.
pub fn centered_character_path(c: char, font_size: f64, cx: f64, cy: f64) -> Option<String> {
    const RASTER_CENTER: f32 = 20.0;
    const QUARTERS_PER_PIXEL: i32 = 4;

    let units_per_em = CIRCLED_FONT.units_per_em()?;
    let awt_scale = font_size as f32 * CIRCLED_FONT.height_unscaled() / units_per_em;
    let scaled = CIRCLED_FONT.as_scaled(PxScale::from(awt_scale));
    let outline = CIRCLED_FONT.outline(scaled.glyph_id(c))?;
    let java_center = (font_size == 17.0)
        .then_some(c as u32)
        .filter(|codepoint| *codepoint <= u8::MAX as u32)
        .and_then(|codepoint| {
            JAVA_AWT_MONO_BOLD_17_CENTERS
                .iter()
                .find(|(candidate, _, _)| *candidate == codepoint as u8)
                .map(|(_, x, y)| (*x as f32, *y as f32))
        });
    let (center_x, center_y) = if let Some(center) = java_center {
        center
    } else {
        let mut glyph = scaled.scaled_glyph(c);
        glyph.position = point(RASTER_CENTER, RASTER_CENTER);
        let outlined = CIRCLED_FONT.outline_glyph(glyph)?;
        let pixel_bounds = outlined.px_bounds();
        let mut pixels = Vec::new();
        outlined.draw(|x, y, coverage| {
            if coverage > 0.0 {
                pixels.push((pixel_bounds.min.x + x as f32, pixel_bounds.min.y + y as f32));
            }
        });
        if pixels.is_empty() {
            let factor = scaled.scale_factor();
            (
                (outline.bounds.min.x + outline.bounds.max.x) * factor.horizontal / 2.0,
                -(outline.bounds.min.y + outline.bounds.max.y) * factor.vertical / 2.0,
            )
        } else {
            let (mut min_x, mut min_y) = pixels[0];
            let (mut max_x, mut max_y) = pixels[0];
            for &(x, y) in &pixels[1..] {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            let mut best = (min_x, min_y);
            let mut best_distance = f32::MAX;
            for ix in (min_x as i32 * QUARTERS_PER_PIXEL)..=(max_x as i32 * QUARTERS_PER_PIXEL) {
                for iy in (min_y as i32 * QUARTERS_PER_PIXEL)..(max_y as i32 * QUARTERS_PER_PIXEL) {
                    let candidate = (
                        ix as f32 / QUARTERS_PER_PIXEL as f32,
                        iy as f32 / QUARTERS_PER_PIXEL as f32,
                    );
                    let farthest = pixels
                        .iter()
                        .map(|&(x, y)| {
                            let dx = candidate.0 - x;
                            let dy = candidate.1 - y;
                            dx * dx + dy * dy
                        })
                        .fold(0.0_f32, f32::max);
                    if farthest < best_distance {
                        best_distance = farthest;
                        best = candidate;
                    }
                }
            }
            (best.0 - RASTER_CENTER, best.1 - RASTER_CENTER)
        }
    };

    let factor = scaled.scale_factor();
    let offset_x = cx - center_x as f64 - 0.5;
    let offset_y = cy - center_y as f64 - 0.5;
    let transform = |p: ab_glyph::Point| {
        (
            p.x as f64 * factor.horizontal as f64 + offset_x,
            -(p.y as f64 * factor.vertical as f64) + offset_y,
        )
    };
    let mut path = String::new();
    let mut current = None;
    let mut contour_start = None;
    for curve in outline.curves {
        let (start, end) = match &curve {
            OutlineCurve::Line(start, end)
            | OutlineCurve::Quad(start, _, end)
            | OutlineCurve::Cubic(start, _, _, end) => (*start, *end),
        };
        let start = transform(start);
        let end = transform(end);
        if current != Some(start) {
            write!(path, "M{},{} ", glyph_num(start.0), glyph_num(start.1)).unwrap();
            contour_start = Some(start);
        }
        match curve {
            OutlineCurve::Line(_, _) if contour_start == Some(end) && current != contour_start => {
                path.push_str("Z ");
            }
            OutlineCurve::Line(_, _) if current == contour_start && start == end => {}
            OutlineCurve::Line(_, _) => {
                write!(path, "L{},{} ", glyph_num(end.0), glyph_num(end.1)).unwrap();
            }
            OutlineCurve::Quad(_, control, _) => {
                let control = transform(control);
                write!(
                    path,
                    "Q{},{} {},{} ",
                    glyph_num(control.0),
                    glyph_num(control.1),
                    glyph_num(end.0),
                    glyph_num(end.1),
                )
                .unwrap();
            }
            OutlineCurve::Cubic(_, control1, control2, _) => {
                let control1 = transform(control1);
                let control2 = transform(control2);
                write!(
                    path,
                    "C{},{} {},{} {},{} ",
                    glyph_num(control1.0),
                    glyph_num(control1.1),
                    glyph_num(control2.0),
                    glyph_num(control2.1),
                    glyph_num(end.0),
                    glyph_num(end.1),
                )
                .unwrap();
            }
        }
        current = Some(end);
    }
    Some(path)
}

fn glyph_num(value: f64) -> String {
    let formatted = format!("{value:.4}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
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
/// Exact Java AWT SansSerif Plain character widths at font-size 14.
///
/// These are exact binary fractions from `FontMetrics.getStringBounds()` with
/// `FRACTIONALMETRICS_ON`. They sum without floating-point error, producing
/// textLength values that match PlantUML's golden SVGs exactly.
#[allow(clippy::excessive_precision)]
const JAVA_AWT_WIDTHS_14: &[(char, f64)] = &[
    (' ', 4.429687500000000),
    ('!', 4.429687500000000),
    ('"', 5.229492187500000),
    ('#', 8.852539062500000),
    ('$', 8.852539062500000),
    ('%', 9.358398437500000),
    ('&', 9.761718750000000),
    ('\'', 3.206054687500000),
    ('(', 4.552734375000000),
    (')', 4.552734375000000),
    ('*', 6.747070312500000),
    ('+', 11.128906250000000),
    (',', 4.429687500000000),
    ('-', 8.100585937500000),
    ('.', 4.429687500000000),
    ('/', 7.341796875000000),
    ('0', 8.852539062500000),
    ('1', 8.852539062500000),
    ('2', 8.852539062500000),
    ('3', 8.852539062500000),
    ('4', 8.852539062500000),
    ('5', 8.852539062500000),
    ('6', 8.852539062500000),
    ('7', 8.852539062500000),
    ('8', 8.852539062500000),
    ('9', 8.852539062500000),
    (':', 4.429687500000000),
    (';', 4.429687500000000),
    ('<', 11.128906250000000),
    ('=', 11.128906250000000),
    ('>', 11.128906250000000),
    ('?', 5.906250000000000),
    ('@', 12.017578125000000),
    ('A', 9.659179687500000),
    ('B', 8.052734375000000),
    ('C', 9.686523437500000),
    ('D', 10.486328125000000),
    ('E', 7.587890625000000),
    ('F', 7.505859375000000),
    ('G', 10.117187500000000),
    ('H', 10.288085937500000),
    ('I', 4.033203125000000),
    ('J', 4.354492187500000),
    ('K', 9.139648437500000),
    ('L', 7.464843750000000),
    ('M', 12.058593750000000),
    ('N', 10.342773437500000),
    ('O', 10.875976562500000),
    ('P', 7.738281250000000),
    ('Q', 10.875976562500000),
    ('R', 8.852539062500000),
    ('S', 7.540039062500000),
    ('T', 8.852539062500000),
    ('U', 9.700195312500000),
    ('V', 9.153320312500000),
    ('W', 11.976562500000000),
    ('X', 8.763671875000000),
    ('Y', 8.722656250000000),
    ('Z', 8.462890625000000),
    ('[', 4.552734375000000),
    ('\\', 7.341796875000000),
    (']', 4.552734375000000),
    ('^', 8.852539062500000),
    ('_', 7.000000000000000),
    ('`', 8.599609375000000),
    ('a', 7.731445312500000),
    ('b', 8.811523437500000),
    ('c', 7.170898437500000),
    ('d', 8.811523437500000),
    ('e', 7.799804687500000),
    ('f', 5.147460937500000),
    ('g', 8.729492187500000),
    ('h', 8.688476562500000),
    ('i', 4.046875000000000),
    ('j', 4.258789062500000),
    ('k', 8.182617187500000),
    ('l', 4.046875000000000),
    ('m', 13.070312500000000),
    ('n', 8.688476562500000),
    ('o', 8.599609375000000),
    ('p', 8.811523437500000),
    ('q', 8.811523437500000),
    ('r', 5.728515625000000),
    ('s', 7.136718750000000),
    ('t', 5.236328125000000),
    ('u', 8.688476562500000),
    ('v', 7.246093750000000),
    ('w', 10.787109375000000),
    ('x', 8.585937500000000),
    ('y', 7.314453125000000),
    ('z', 8.025390625000000),
    ('{', 4.552734375000000),
    ('|', 5.229492187500000),
    ('}', 4.552734375000000),
    ('~', 8.852539062500000),
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

// ---------------------------------------------------------------------------
// Java AWT Bold font metrics
// ---------------------------------------------------------------------------

/// Java AWT SansSerif **Bold** character widths at font-size 14,
/// with `RenderingHints.VALUE_FRACTIONALMETRICS_ON`.
///
/// Extracted from Java 21+ on macOS using `FontMetrics.getStringBounds()`,
/// matching PlantUML's bold text rendering (e.g., autonumber labels, group headers).
/// Widths scale linearly with font size, same as the plain variant.
#[allow(clippy::excessive_precision)]
const JAVA_AWT_BOLD_WIDTHS_14: &[(char, f64)] = &[
    (' ', 4.614257812500000),
    ('!', 4.388671875000000),
    ('"', 6.323242187500000),
    ('#', 9.235351562500000),
    ('$', 9.235351562500000),
    ('%', 10.964843750000000),
    ('&', 10.424804687500000),
    ('\'', 3.458984375000000),
    ('(', 5.188476562500000),
    (')', 5.188476562500000),
    ('*', 6.528320312500000),
    ('+', 11.135742187500000),
    (',', 3.458984375000000),
    ('-', 8.941406250000000),
    ('.', 3.458984375000000),
    ('/', 7.916015625000000),
    ('0', 9.235351562500000),
    ('1', 9.235351562500000),
    ('2', 9.235351562500000),
    ('3', 9.235351562500000),
    ('4', 9.235351562500000),
    ('5', 9.235351562500000),
    ('6', 9.235351562500000),
    ('7', 9.235351562500000),
    ('8', 9.235351562500000),
    ('9', 9.235351562500000),
    (':', 3.458984375000000),
    (';', 3.458984375000000),
    ('<', 11.135742187500000),
    ('=', 11.135742187500000),
    ('>', 11.135742187500000),
    ('?', 6.958984375000000),
    ('@', 12.017578125000000),
    ('A', 10.308593750000000),
    ('B', 8.818359375000000),
    ('C', 9.973632812500000),
    ('D', 11.101562500000000),
    ('E', 8.408203125000000),
    ('F', 8.039062500000000),
    ('G', 10.438476562500000),
    ('H', 10.923828125000000),
    ('I', 4.641601562500000),
    ('J', 5.858398437500000),
    ('K', 9.939453125000000),
    ('L', 8.141601562500000),
    ('M', 12.708007812500000),
    ('N', 10.759765625000000),
    ('O', 11.525390625000000),
    ('P', 8.585937500000000),
    ('Q', 11.532226562500000),
    ('R', 9.659179687500000),
    ('S', 7.998046875000000),
    ('T', 9.659179687500000),
    ('U', 10.308593750000000),
    ('V', 9.782226562500000),
    ('W', 12.653320312500000),
    ('X', 9.337890625000000),
    ('Y', 9.611328125000000),
    ('Z', 9.023437500000000),
    ('[', 5.188476562500000),
    ('\\', 7.916015625000000),
    (']', 5.188476562500000),
    ('^', 9.235351562500000),
    ('_', 7.000000000000000),
    ('`', 8.941406250000000),
    ('a', 8.230468750000000),
    ('b', 9.276367187500000),
    ('c', 7.451171875000000),
    ('d', 9.276367187500000),
    ('e', 8.203125000000000),
    ('f', 5.803710937500000),
    ('g', 9.235351562500000),
    ('h', 9.194335937500000),
    ('i', 4.552734375000000),
    ('j', 4.662109375000000),
    ('k', 8.859375000000000),
    ('l', 4.552734375000000),
    ('m', 13.576171875000000),
    ('n', 9.194335937500000),
    ('o', 8.941406250000000),
    ('p', 9.276367187500000),
    ('q', 9.276367187500000),
    ('r', 6.364257812500000),
    ('s', 7.916015625000000),
    ('t', 5.673828125000000),
    ('u', 9.194335937500000),
    ('v', 8.305664062500000),
    ('w', 12.079101562500000),
    ('x', 8.271484375000000),
    ('y', 8.162109375000000),
    ('z', 8.271484375000000),
    ('{', 5.188476562500000),
    ('|', 5.400390625000000),
    ('}', 5.188476562500000),
    ('~', 11.128906250000000),
];

/// Build a fast lookup array for bold ASCII char widths.
static JAVA_AWT_BOLD_LOOKUP_14: LazyLock<[f64; 128]> = LazyLock::new(|| {
    let mut table = [0.0f64; 128];
    for &(c, w) in JAVA_AWT_BOLD_WIDTHS_14 {
        table[c as usize] = w;
    }
    table
});

/// Bold text width using PlantUML's exact Java AWT SansSerif Bold metrics (font-size 14).
pub fn plantuml_bold_text_width_14(text: &str) -> f64 {
    let table = &*JAVA_AWT_BOLD_LOOKUP_14;
    let fallback = 9.0; // reasonable default for unknown bold chars
    text.chars()
        .map(|c| {
            let code = c as usize;
            if code < 128 { table[code] } else { fallback }
        })
        .sum()
}

/// Bold text width for a PlantUML text string, scaled to a specific font size.
pub fn plantuml_bold_text_width(text: &str, font_size: f64) -> f64 {
    plantuml_bold_text_width_14(text) * font_size / 14.0
}

/// Format a text width to 4 decimal places, matching PlantUML's output format.
pub fn format_width(w: f64) -> String {
    format!("{w:.4}")
}

// ---------------------------------------------------------------------------
// Java AWT Plain font metrics at font-size 12
// ---------------------------------------------------------------------------

/// Java AWT SansSerif character widths at font-size 12 (plain),
/// with `RenderingHints.VALUE_FRACTIONALMETRICS_ON`.
///
/// Extracted from Java 21+ on macOS. Used for stereotype text rendering.
#[allow(clippy::excessive_precision)]
const JAVA_AWT_WIDTHS_12: &[(char, f64)] = &[
    (' ', 3.796875000000000),
    ('!', 3.796875000000000),
    ('"', 4.482421875000000),
    ('#', 7.587890625000000),
    ('$', 7.587890625000000),
    ('%', 8.021484375000000),
    ('&', 8.367187500000000),
    ('\'', 2.748046875000000),
    ('(', 3.902343750000000),
    (')', 3.902343750000000),
    ('*', 5.783203125000000),
    ('+', 9.539062500000000),
    (',', 3.796875000000000),
    ('-', 6.943359375000000),
    ('.', 3.796875000000000),
    ('/', 6.292968750000000),
    ('0', 7.587890625000000),
    ('1', 7.587890625000000),
    ('2', 7.587890625000000),
    ('3', 7.587890625000000),
    ('4', 7.587890625000000),
    ('5', 7.587890625000000),
    ('6', 7.587890625000000),
    ('7', 7.587890625000000),
    ('8', 7.587890625000000),
    ('9', 7.587890625000000),
    (':', 3.796875000000000),
    (';', 3.796875000000000),
    ('<', 9.539062500000000),
    ('=', 9.539062500000000),
    ('>', 9.539062500000000),
    ('?', 5.062500000000000),
    ('@', 10.300781250000000),
    ('A', 8.279296875000000),
    ('B', 6.902343750000000),
    ('C', 8.302734375000000),
    ('D', 8.988281250000000),
    ('E', 6.503906250000000),
    ('F', 6.433593750000000),
    ('G', 8.671875000000000),
    ('H', 8.818359375000000),
    ('I', 3.457031250000000),
    ('J', 3.732421875000000),
    ('K', 7.833984375000000),
    ('L', 6.398437500000000),
    ('M', 10.335937500000000),
    ('N', 8.865234375000000),
    ('O', 9.322265625000000),
    ('P', 6.632812500000000),
    ('Q', 9.322265625000000),
    ('R', 7.587890625000000),
    ('S', 6.462890625000000),
    ('T', 7.587890625000000),
    ('U', 8.314453125000000),
    ('V', 7.845703125000000),
    ('W', 10.265625000000000),
    ('X', 7.511718750000000),
    ('Y', 7.476562500000000),
    ('Z', 7.253906250000000),
    ('[', 3.902343750000000),
    ('\\', 6.292968750000000),
    (']', 3.902343750000000),
    ('^', 7.587890625000000),
    ('_', 6.000000000000000),
    ('`', 7.371093750000000),
    ('a', 6.626953125000000),
    ('b', 7.552734375000000),
    ('c', 6.146484375000000),
    ('d', 7.552734375000000),
    ('e', 6.685546875000000),
    ('f', 4.412109375000000),
    ('g', 7.482421875000000),
    ('h', 7.447265625000000),
    ('i', 3.468750000000000),
    ('j', 3.650390625000000),
    ('k', 7.013671875000000),
    ('l', 3.468750000000000),
    ('m', 11.203125000000000),
    ('n', 7.447265625000000),
    ('o', 7.371093750000000),
    ('p', 7.552734375000000),
    ('q', 7.552734375000000),
    ('r', 4.910156250000000),
    ('s', 6.117187500000000),
    ('t', 4.488281250000000),
    ('u', 7.447265625000000),
    ('v', 6.210937500000000),
    ('w', 9.246093750000000),
    ('x', 7.359375000000000),
    ('y', 6.269531250000000),
    ('z', 6.878906250000000),
    ('{', 3.902343750000000),
    ('|', 4.482421875000000),
    ('}', 3.902343750000000),
    ('~', 7.587890625000000),
];

/// Build a fast lookup array for 12pt ASCII char widths.
static JAVA_AWT_LOOKUP_12: LazyLock<[f64; 128]> = LazyLock::new(|| {
    let mut table = [0.0f64; 128];
    for &(c, w) in JAVA_AWT_WIDTHS_12 {
        table[c as usize] = w;
    }
    table
});

/// Width of left guillemet «  (U+00AB) at font-size 12.
pub const GUILLEMET_LEFT_WIDTH_12: f64 = 6.287_109_375;
/// Width of right guillemet »  (U+00BB) at font-size 12.
pub const GUILLEMET_RIGHT_WIDTH_12: f64 = 6.287_109_375;

/// Text width using PlantUML's exact Java AWT SansSerif metrics (font-size 12, plain).
pub fn plantuml_text_width_12(text: &str) -> f64 {
    let table = &*JAVA_AWT_LOOKUP_12;
    let fallback = 7.0; // reasonable default for unknown chars at 12pt
    text.chars()
        .map(|c| {
            let code = c as usize;
            if code < 128 {
                table[code]
            } else {
                match c {
                    '\u{00AB}' => GUILLEMET_LEFT_WIDTH_12,
                    '\u{00BB}' => GUILLEMET_RIGHT_WIDTH_12,
                    _ => fallback,
                }
            }
        })
        .sum()
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
