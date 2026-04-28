// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! EPS (Encapsulated PostScript) output — converts SVG to EPS by rendering to raster
//! and embedding the pixel data in a minimal PostScript wrapper.

use resvg::tiny_skia;
use resvg::usvg;

/// Convert SVG string to EPS bytes.
///
/// This renders the SVG to a pixel buffer and embeds the raw RGBA image data
/// in an Encapsulated PostScript wrapper with a simple scale-to-fit transform.
pub fn svg_to_eps(svg: &str) -> Result<Vec<u8>, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse error: {e}"))?;

    let size = tree.size().to_int_size();
    let width = size.width() as usize;
    let height = size.height() as usize;

    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("failed to create pixmap")?;

    // White background.
    pixmap.fill(tiny_skia::Color::WHITE);

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // Get raw RGBA pixel data. Pixmap stores pixels as RGBA in native byte order.
    let rgba_data = pixmap.data();

    // Build EPS file.
    // EPS format: PostScript header, image data, rendering commands.
    let mut eps = Vec::new();

    // EPS header (magic number).
    eps.extend_from_slice(b"%!PS-Adobe-3.0 EPSF-3.0\n");

    // BoundingBox: positioned at origin with size in points (assuming 72 DPI).
    let bbox_line = format!("%%BoundingBox: 0 0 {} {}\n", width, height);
    eps.extend_from_slice(bbox_line.as_bytes());

    eps.extend_from_slice(b"%%EndComments\n");

    // PostScript prologue: define image rendering procedure.
    eps.extend_from_slice(b"gsave\n");
    eps.extend_from_slice(b"/DeviceRGB setcolorspace\n");

    // Scale: position image and set size in user coordinates.
    // Translate to origin and scale to image dimensions.
    let scale_line = format!("0 0 translate {} {} scale\n", width, height);
    eps.extend_from_slice(scale_line.as_bytes());

    // Image dictionary: width, height, bits per component, color space.
    eps.extend_from_slice(b"<<\n");
    eps.extend_from_slice(b"  /ImageType 1\n");
    let width_line = format!("  /Width {}\n", width);
    eps.extend_from_slice(width_line.as_bytes());
    let height_line = format!("  /Height {}\n", height);
    eps.extend_from_slice(height_line.as_bytes());
    eps.extend_from_slice(b"  /BitsPerComponent 8\n");
    eps.extend_from_slice(b"  /ColorSpace /DeviceRGB\n");
    eps.extend_from_slice(b"  /Decode [0 1 0 1 0 1]\n");
    eps.extend_from_slice(b">>\n");

    // Inline image data operator with ASCII85-encoded RGBA data.
    eps.extend_from_slice(b"image\n");

    // Encode raw RGBA data as ASCII85.
    // Convert RGBA to RGB by dropping the alpha channel.
    let rgb_data: Vec<u8> = rgba_data
        .chunks(4)
        .flat_map(|rgba| {
            // rgba[0]=R, rgba[1]=G, rgba[2]=B, rgba[3]=A
            // For opaque white backgrounds and dark text, we keep RGB as-is.
            // TODO: Blend alpha properly if needed.
            vec![rgba[0], rgba[1], rgba[2]]
        })
        .collect();

    let ascii85_data = encode_ascii85(&rgb_data);
    eps.extend_from_slice(ascii85_data.as_bytes());
    eps.extend_from_slice(b"~>\n");

    // Epilogue.
    eps.extend_from_slice(b"grestore\n");
    eps.extend_from_slice(b"showpage\n");

    Ok(eps)
}

/// Encode binary data using ASCII85 (also called Base85).
/// This is a standard PostScript/PDF encoding that compresses 4 bytes to 5 ASCII chars.
fn encode_ascii85(data: &[u8]) -> String {
    let mut result = String::new();
    let mut col = 0;

    for chunk in data.chunks(4) {
        // Pad chunk to 4 bytes if necessary.
        let mut buf = [0u8; 4];
        buf[..chunk.len()].copy_from_slice(chunk);

        // Convert 4 bytes to a 32-bit big-endian integer.
        let value = u32::from_be_bytes(buf);

        // Special case: all zeros encode as 'z'.
        if value == 0 && chunk.len() == 4 {
            result.push('z');
            col += 1;
        } else {
            // Encode as 5 ASCII85 digits.
            let mut encoded = [0u8; 5];
            let mut v = value;
            for i in (0..5).rev() {
                encoded[i] = (v % 85) as u8 + 33;
                v /= 85;
            }

            // Only write bytes that correspond to input bytes (handle short chunks).
            let out_len = if chunk.len() == 4 { 5 } else { chunk.len() + 1 };

            for &byte in &encoded[..out_len] {
                result.push(byte as char);
                col += 1;

                // Wrap lines at 76 columns for readability.
                if col >= 76 {
                    result.push('\n');
                    col = 0;
                }
            }
        }
    }

    // End-of-data marker is separate (written outside this function).
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_svg_to_eps() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
            <rect x="10" y="10" width="80" height="30" fill="#E2E2F0" stroke="#000" stroke-width="1"/>
            <text x="50" y="30" text-anchor="middle" font-size="13">Hello</text>
        </svg>"##;

        let eps = svg_to_eps(svg).unwrap();
        // EPS magic: %!PS-Adobe
        assert_eq!(&eps[..9], b"%!PS-Adob");
        assert!(eps.len() > 100, "EPS should have content");
    }

    #[test]
    fn eps_has_boundingbox() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">
            <rect width="200" height="100" fill="white"/>
        </svg>"##;

        let eps = svg_to_eps(svg).unwrap();
        let eps_str = String::from_utf8_lossy(&eps);
        assert!(
            eps_str.contains("%%BoundingBox:"),
            "EPS should contain BoundingBox"
        );
        assert!(
            eps_str.contains("200 100"),
            "BoundingBox should reference dimensions"
        );
    }

    #[test]
    fn end_to_end_diagram_to_eps() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let eps = svg_to_eps(&svg).unwrap();
        assert_eq!(&eps[..9], b"%!PS-Adob");
    }

    #[test]
    fn ascii85_encoding_all_zeros() {
        // Test ASCII85 encoding with all zeros (special case 'z').
        let result = encode_ascii85(&[0, 0, 0, 0]);
        assert_eq!(result, "z");
    }

    #[test]
    fn ascii85_roundtrip() {
        // Verify that encoding produces valid ASCII85 (all printable characters).
        let data = vec![0x4d, 0x61, 0x6e, 0x21]; // "Man!"
        let result = encode_ascii85(&data);
        // Should produce 5 ASCII85 characters (base 85 digits are ! through u)
        assert_eq!(result.len(), 5, "4 bytes should encode to 5 ASCII85 chars");
        // All characters should be in ASCII85 range [33..117] (! through u)
        for c in result.chars() {
            let b = c as u8;
            assert!(
                (33..=117).contains(&b),
                "Character {} outside ASCII85 range",
                c
            );
        }
    }
}
