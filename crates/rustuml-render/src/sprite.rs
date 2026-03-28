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
//! Each hex digit represents a grayscale level (0 = transparent, F = opaque white).
//! The sprite is rendered as a small PNG embedded in the SVG via a data URI.

use std::collections::HashMap;

use resvg::tiny_skia;
use rustuml_parser::diagram::SpriteData;

/// Render a sprite's pixel data to a raw RGBA pixel buffer.
///
/// Digit 0 → fully transparent (alpha 0).
/// Digit 1–F → grayscale with proportional alpha.  For digit d (1–15),
/// the gray level and alpha are both `round(d / 15.0 * 255)`.
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

    let mut rgba = vec![0u8; (w * h * 4) as usize];

    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx >= h as usize {
            break;
        }
        for (col_idx, ch) in row.chars().enumerate() {
            if col_idx >= w as usize {
                break;
            }
            let digit = ch.to_digit(16).unwrap_or(0);
            let base_idx = (row_idx as u32 * w + col_idx as u32) as usize * 4;
            if digit == 0 {
                // Fully transparent pixel.
                rgba[base_idx] = 0;
                rgba[base_idx + 1] = 0;
                rgba[base_idx + 2] = 0;
                rgba[base_idx + 3] = 0;
            } else {
                // Gray level: digit 1 → 17, digit F → 255.
                let level = ((digit as f64 / 15.0) * 255.0).round() as u8;
                rgba[base_idx] = level;
                rgba[base_idx + 1] = level;
                rgba[base_idx + 2] = level;
                rgba[base_idx + 3] = 255; // fully opaque
            }
        }
    }

    (w, h, rgba)
}

/// Encode a sprite to a PNG byte vector.
pub fn sprite_to_png(sprite: &SpriteData) -> Result<Vec<u8>, String> {
    let (w, h, rgba) = sprite_to_rgba(sprite);
    if w == 0 || h == 0 {
        return Err("sprite has no pixel data".to_string());
    }

    let mut pixmap =
        tiny_skia::Pixmap::new(w, h).ok_or_else(|| format!("failed to create pixmap {w}x{h}"))?;

    // Copy RGBA data into the pixmap.
    // tiny_skia uses premultiplied alpha internally, but Pixmap::data_mut()
    // takes pre-multiplied RGBA. Since our pixels are either fully transparent
    // or fully opaque, this is a direct copy for the opaque case.
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

    pixmap
        .encode_png()
        .map_err(|e| format!("PNG encoding error: {e}"))
}

/// Encode a sprite to a base64-encoded PNG data URI suitable for `xlink:href`.
pub fn sprite_to_data_uri(sprite: &SpriteData) -> Result<String, String> {
    let png = sprite_to_png(sprite)?;
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

/// A segment of text that may contain sprite references.
#[derive(Debug, Clone, PartialEq)]
pub enum TextSegment {
    /// Plain text (no sprite reference).
    Text(String),
    /// A sprite reference `<$name>`.
    Sprite(String),
}

/// Parse a string into alternating text and sprite segments.
///
/// `<$name>` references are split out as `TextSegment::Sprite(name)`.
/// Everything else (including leading/trailing spaces) is `TextSegment::Text`.
pub fn parse_sprite_segments(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut rest = text;

    while !rest.is_empty() {
        if let Some(start) = rest.find("<$") {
            // Text before the sprite ref.
            if start > 0 {
                segments.push(TextSegment::Text(rest[..start].to_string()));
            }
            let after = &rest[start + 2..]; // skip "<$"
            if let Some(end) = after.find('>') {
                let name = &after[..end];
                segments.push(TextSegment::Sprite(name.to_string()));
                rest = &after[end + 1..];
                // Skip a single trailing space after sprite ref (matches PlantUML behaviour).
                if rest.starts_with(' ') {
                    rest = &rest[1..];
                }
            } else {
                // No closing '>'; treat the rest as text.
                segments.push(TextSegment::Text(rest.to_string()));
                break;
            }
        } else {
            segments.push(TextSegment::Text(rest.to_string()));
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
    if !text.contains("<$") {
        return crate::metrics::text_width(text, font_size);
    }
    let segments = parse_sprite_segments(text);
    measure_segments(&segments, font_size, sprites)
}

/// Simple base64 encoder (no external dependency needed).
fn encode_base64(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
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
        // Second pixel (digit F) → fully opaque white.
        assert_eq!(rgba[7], 255);
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
}
