// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sprite rendering — converts PlantUML sprite pixel data to inline PNG images.
//!
//! PlantUML sprites are small bitmaps defined inline with hex-digit pixel rows:
//!
//! ```plantuml
//! sprite $disk [8x8/16] {
//!   00000000
//!   0FFFFFF0
//!   0F8F8F80
//!   0FFFFFF0
//!   00000000
//! }
//! ```
//!
//! Each hex digit represents a grayscale level between the background and
//! foreground colours. The sprite is rendered as a small PNG embedded in the
//! SVG via a data URI.

use std::collections::HashMap;

use resvg::tiny_skia::{self, FilterQuality, PixmapPaint, Transform};
use rustuml_parser::diagram::SpriteData;

const PLANTUML_GRAY_LEVELS: u32 = 16;
const PLANTUML_MAX_CHANNEL: u8 = 255;
const PLANTUML_ALPHA_RAMP_DIVISOR: f64 = 4.0;

/// Render a sprite's pixel data to a raw RGBA pixel buffer.
///
/// This follows PlantUML's `SpriteMonochrome.toUImage`: pixels are a
/// white-to-foreground gradient, and low gray values ramp alpha relative to
/// the maximum gray coefficient present in the sprite.
fn sprite_to_rgba(sprite: &SpriteData) -> (u32, u32, Vec<u8>) {
    let rows = &sprite.rows;
    let height = rows.len() as u32;
    let width = rows.first().map(|r| r.len()).unwrap_or(0) as u32;

    // Use declared width/height if valid, otherwise infer from data.
    let w = if sprite.width > 0 {
        sprite.width
    } else {
        width
    };
    let h = if sprite.height > 0 {
        sprite.height
    } else {
        height
    };

    let mut gray = vec![0u32; (w * h) as usize];
    let mut max_gray = 0u32;

    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx >= h as usize {
            break;
        }
        for (col_idx, ch) in row.chars().enumerate() {
            if col_idx >= w as usize {
                break;
            }
            let digit = ch.to_digit(16).unwrap_or(0);
            let idx = (row_idx as u32 * w + col_idx as u32) as usize;
            gray[idx] = digit;
            max_gray = max_gray.max(digit);
        }
    }

    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let max_coef = max_gray as f64 / (PLANTUML_GRAY_LEVELS - 1) as f64;
    for (idx, gray_value) in gray.iter().copied().enumerate() {
        let coef = gray_value as f64 / (PLANTUML_GRAY_LEVELS - 1) as f64;
        let alpha = if max_gray == 0 {
            0
        } else if coef > max_coef / PLANTUML_ALPHA_RAMP_DIVISOR {
            PLANTUML_MAX_CHANNEL
        } else {
            (PLANTUML_MAX_CHANNEL as f64 * (coef * PLANTUML_ALPHA_RAMP_DIVISOR / max_coef)) as u8
        };
        let channel = PLANTUML_MAX_CHANNEL - (coef * PLANTUML_MAX_CHANNEL as f64).trunc() as u8;
        let base_idx = idx * 4;
        rgba[base_idx] = channel;
        rgba[base_idx + 1] = channel;
        rgba[base_idx + 2] = channel;
        rgba[base_idx + 3] = alpha;
    }

    (w, h, rgba)
}

fn rgba_to_pixmap(w: u32, h: u32, rgba: &[u8]) -> Result<tiny_skia::Pixmap, String> {
    if w == 0 || h == 0 {
        return Err("sprite has no pixel data".to_string());
    }

    let mut pixmap =
        tiny_skia::Pixmap::new(w, h).ok_or_else(|| format!("failed to create pixmap {w}x{h}"))?;

    // tiny-skia stores premultiplied RGBA; PlantUML's PortableImage stores
    // straight ARGB, so premultiply here before handing pixels to the encoder.
    let dst = pixmap.data_mut();
    for (i, chunk) in rgba.chunks_exact(4).enumerate() {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];
        // Premultiply: tiny_skia stores premultiplied alpha.
        let pm_r = ((r as u32 * a as u32 + 127) / 255) as u8;
        let pm_g = ((g as u32 * a as u32 + 127) / 255) as u8;
        let pm_b = ((b as u32 * a as u32 + 127) / 255) as u8;
        dst[i * 4] = pm_r;
        dst[i * 4 + 1] = pm_g;
        dst[i * 4 + 2] = pm_b;
        dst[i * 4 + 3] = a;
    }

    Ok(pixmap)
}

fn scale_pixmap(
    pixmap: &tiny_skia::Pixmap,
    target_w: u32,
    target_h: u32,
) -> Result<tiny_skia::Pixmap, String> {
    if pixmap.width() == target_w && pixmap.height() == target_h {
        return Ok(pixmap.clone());
    }
    if target_w == 0 || target_h == 0 {
        return Err("sprite scale produced an empty image".to_string());
    }

    let mut scaled = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| format!("failed to create scaled pixmap {target_w}x{target_h}"))?;
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    let sx = target_w as f32 / pixmap.width() as f32;
    let sy = target_h as f32 / pixmap.height() as f32;
    scaled.draw_pixmap(
        0,
        0,
        pixmap.as_ref(),
        &paint,
        Transform::from_scale(sx, sy),
        None,
    );
    Ok(scaled)
}

/// Encode a sprite to a PNG byte vector.
pub fn sprite_to_png(sprite: &SpriteData) -> Result<Vec<u8>, String> {
    let (w, h, rgba) = sprite_to_rgba(sprite);
    let pixmap = rgba_to_pixmap(w, h, &rgba)?;
    pixmap
        .encode_png()
        .map_err(|e| format!("PNG encoding error: {e}"))
}

/// Encode a sprite to a PNG byte vector after PlantUML-style image scaling.
pub fn sprite_to_png_scaled(sprite: &SpriteData, scale: f64) -> Result<Vec<u8>, String> {
    let (w, h, rgba) = sprite_to_rgba(sprite);
    let pixmap = rgba_to_pixmap(w, h, &rgba)?;
    let (target_w, target_h) = scaled_sprite_dimensions(sprite, scale);
    let scaled = scale_pixmap(&pixmap, target_w, target_h)?;
    scaled
        .encode_png()
        .map_err(|e| format!("PNG encoding error: {e}"))
}

/// Encode a sprite to a base64-encoded PNG data URI suitable for `xlink:href`.
pub fn sprite_to_data_uri(sprite: &SpriteData) -> Result<String, String> {
    let png = sprite_to_png(sprite)?;
    let encoded = encode_base64(&png);
    Ok(format!("data:image/png;base64,{encoded}"))
}

/// Encode a scaled sprite to a base64-encoded PNG data URI.
pub fn sprite_to_data_uri_scaled(sprite: &SpriteData, scale: f64) -> Result<String, String> {
    let png = sprite_to_png_scaled(sprite, scale)?;
    let encoded = encode_base64(&png);
    Ok(format!("data:image/png;base64,{encoded}"))
}

/// A cache of pre-computed sprite data URIs.
pub struct SpriteCache {
    uris: HashMap<String, Option<String>>,
}

impl SpriteCache {
    /// Build a cache from a sprite map.  Sprites that fail to encode get `None`.
    pub fn from_sprites(sprites: &HashMap<String, SpriteData>) -> Self {
        let uris = sprites
            .iter()
            .map(|(name, data)| {
                let uri = sprite_to_data_uri(data).ok();
                (name.clone(), uri)
            })
            .collect();
        Self { uris }
    }

    /// Build a cache whose PNG payloads use PlantUML-style image scaling.
    pub fn from_sprites_scaled(sprites: &HashMap<String, SpriteData>, scale: f64) -> Self {
        let uris = sprites
            .iter()
            .map(|(name, data)| {
                let uri = sprite_to_data_uri_scaled(data, scale).ok();
                (name.clone(), uri)
            })
            .collect();
        Self { uris }
    }

    /// Return the data URI for a sprite, or `None` if not found / encode failed.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.uris.get(name)?.as_deref()
    }

    /// Return true if any sprites are cached.
    pub fn is_empty(&self) -> bool {
        self.uris.is_empty()
    }
}

/// Return the pixel dimensions of a sprite as `(width, height)`.
pub fn sprite_dimensions(sprite: &SpriteData) -> (u32, u32) {
    let rows = &sprite.rows;
    let h = rows.len() as u32;
    let w = rows.first().map(|r| r.len()).unwrap_or(0) as u32;
    let width = if sprite.width > 0 { sprite.width } else { w };
    let height = if sprite.height > 0 { sprite.height } else { h };
    (width, height)
}

/// Return sprite dimensions after Java `PortableImageAwt.scale` rounding.
pub fn scaled_sprite_dimensions(sprite: &SpriteData, scale: f64) -> (u32, u32) {
    let (width, height) = sprite_dimensions(sprite);
    if scale <= 0.0 {
        return (width, height);
    }
    (
        (width as f64 * scale).round() as u32,
        (height as f64 * scale).round() as u32,
    )
}

/// A segment of text that may contain sprite or OpenIconic references.
#[derive(Debug, Clone, PartialEq)]
pub enum TextSegment {
    /// Plain text (no sprite/icon reference).
    Text(String),
    /// A sprite reference `<$name>`.
    Sprite(String),
    /// An OpenIconic icon reference `<&name>`.
    OpenIcon(String),
}

/// Parse a string into alternating text, sprite, and OpenIconic segments.
///
/// `<$name>` references are split out as `TextSegment::Sprite(name)`.
/// `<&name>` references are split out as `TextSegment::OpenIcon(name)`.
/// Everything else (including leading/trailing spaces) is `TextSegment::Text`.
pub fn parse_sprite_segments(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut rest = text;

    while !rest.is_empty() {
        // Find the earliest sprite `<$` or OpenIconic `<&` reference.
        let sprite_pos = rest.find("<$");
        let icon_pos = rest.find("<&");

        // Pick whichever comes first, or None if neither exists.
        let (start, prefix_len, is_icon) = match (sprite_pos, icon_pos) {
            (Some(s), Some(i)) if i < s => (i, 2, true),
            (Some(s), _) => (s, 2, false),
            (None, Some(i)) => (i, 2, true),
            (None, None) => {
                segments.push(TextSegment::Text(rest.to_string()));
                break;
            }
        };

        // Text before the reference.
        if start > 0 {
            segments.push(TextSegment::Text(rest[..start].to_string()));
        }
        let after = &rest[start + prefix_len..]; // skip "<$" or "<&"
        if let Some(end) = after.find('>') {
            let name = &after[..end];
            if is_icon {
                segments.push(TextSegment::OpenIcon(name.to_string()));
            } else {
                segments.push(TextSegment::Sprite(name.to_string()));
            }
            rest = &after[end + 1..];
            // Skip a single trailing space after the reference (matches PlantUML behaviour).
            if rest.starts_with(' ') {
                rest = &rest[1..];
            }
        } else {
            // No closing '>'; treat from the marker onwards as text.
            segments.push(TextSegment::Text(rest[start..].to_string()));
            break;
        }
    }

    segments
}

/// Measure the total pixel width of a sequence of text segments at the given
/// font size.  Sprite dimensions are looked up from `sprites`.
pub fn measure_segments(
    segments: &[TextSegment],
    font_size: f64,
    sprites: &HashMap<String, SpriteData>,
) -> f64 {
    use crate::metrics;

    let mut total = 0.0_f64;
    for seg in segments {
        match seg {
            TextSegment::Text(t) => {
                total += metrics::text_width(t, font_size);
            }
            TextSegment::Sprite(name) => {
                if let Some(sd) = sprites.get(name) {
                    let (w, _h) = sprite_dimensions(sd);
                    total += w as f64 + 1.0; // 1px gap
                }
            }
            TextSegment::OpenIcon(name) => {
                if let Some(icon) = crate::openiconic::lookup(name) {
                    // Scale icon to match text size.  The icon viewBox is 8x8;
                    // PlantUML scales it proportionally to the font size.
                    let scale = font_size / icon.height;
                    total += icon.width * scale + 1.0; // 1px gap
                }
            }
        }
    }
    total
}

/// Compute the pixel width of text that may contain `<$name>` sprite references.
///
/// Sprite references are replaced by their pixel width.  Unknown sprite names
/// contribute 0 width.
pub fn text_width_with_sprites(
    text: &str,
    font_size: f64,
    sprites: &HashMap<String, SpriteData>,
) -> f64 {
    if !text.contains("<$") && !text.contains("<&") {
        return crate::metrics::text_width(text, font_size);
    }
    let segments = parse_sprite_segments(text);
    measure_segments(&segments, font_size, sprites)
}

/// Simple base64 encoder (no external dependency needed).
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

    fn make_sprite(rows: &[&str]) -> SpriteData {
        let rows: Vec<String> = rows.iter().map(|s| s.to_string()).collect();
        let height = rows.len() as u32;
        let width = rows.first().map(|r| r.len()).unwrap_or(0) as u32;
        SpriteData {
            width,
            height,
            rows,
        }
    }

    #[test]
    fn encode_simple_sprite() {
        let sprite = make_sprite(&["F0", "0F"]);
        let result = sprite_to_png(&sprite);
        assert!(result.is_ok(), "PNG encoding failed: {:?}", result.err());
        let png = result.unwrap();
        // PNG magic bytes: 137 80 78 71 13 10 26 10
        assert_eq!(&png[..4], &[137, 80, 78, 71]);
    }

    #[test]
    fn data_uri_starts_correctly() {
        let sprite = make_sprite(&["FF", "FF"]);
        let uri = sprite_to_data_uri(&sprite).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn transparent_pixel_is_zero_alpha() {
        let sprite = make_sprite(&["0F"]);
        let (_, _, rgba) = sprite_to_rgba(&sprite);
        // First pixel (digit 0) → alpha = 0.
        assert_eq!(rgba[3], 0);
        // Second pixel (digit F) → fully opaque foreground black.
        assert_eq!(rgba[4], 0);
        assert_eq!(rgba[5], 0);
        assert_eq!(rgba[6], 0);
        assert_eq!(rgba[7], 255);
    }

    #[test]
    fn scaled_dimensions_use_java_rounding() {
        let sprite = SpriteData {
            width: 8,
            height: 8,
            rows: vec!["0".repeat(8); 8],
        };
        assert_eq!(scaled_sprite_dimensions(&sprite, 14.0 / 13.0), (9, 9));
    }

    #[test]
    fn sprite_cache_lookup() {
        let sprite = make_sprite(&["FF"]);
        let mut map = HashMap::new();
        map.insert("test".to_string(), sprite);
        let cache = SpriteCache::from_sprites(&map);
        assert!(cache.get("test").is_some());
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn parse_openiconic_segment() {
        let segs = parse_sprite_segments("Hello <&heart> world");
        assert_eq!(
            segs,
            vec![
                TextSegment::Text("Hello ".to_string()),
                TextSegment::OpenIcon("heart".to_string()),
                TextSegment::Text("world".to_string()),
            ]
        );
    }

    #[test]
    fn parse_mixed_sprite_and_icon() {
        let segs = parse_sprite_segments("<$disk> <&check> done");
        assert_eq!(
            segs,
            vec![
                TextSegment::Sprite("disk".to_string()),
                TextSegment::OpenIcon("check".to_string()),
                TextSegment::Text("done".to_string()),
            ]
        );
    }

    #[test]
    fn parse_icon_only() {
        let segs = parse_sprite_segments("<&folder>");
        assert_eq!(segs, vec![TextSegment::OpenIcon("folder".to_string()),]);
    }

    #[test]
    fn parse_icon_no_closing_bracket() {
        let segs = parse_sprite_segments("text <&broken");
        assert_eq!(
            segs,
            vec![
                TextSegment::Text("text ".to_string()),
                TextSegment::Text("<&broken".to_string()),
            ]
        );
    }

    #[test]
    fn measure_includes_icon_width() {
        let segs = vec![
            TextSegment::Text("Hi ".to_string()),
            TextSegment::OpenIcon("heart".to_string()),
        ];
        let empty = HashMap::new();
        let w = measure_segments(&segs, 13.0, &empty);
        let text_w = crate::metrics::text_width("Hi ", 13.0);
        // Icon should add positive width beyond just the text.
        assert!(w > text_w, "width {w} should exceed text-only {text_w}");
    }
}
