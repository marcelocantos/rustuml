// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! PNG output — rasterizes SVG to PNG using resvg.

use resvg::tiny_skia;
use resvg::usvg;

/// Convert SVG string to PNG bytes at default DPI (96, matching PlantUML).
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    svg_to_png_scaled(svg, 1.0)
}

/// Convert SVG string to PNG bytes with a scale factor.
///
/// A scale of 1.0 renders at 96 DPI (matching PlantUML's default).
/// A scale of 2.0 renders at 192 DPI (retina).
pub fn svg_to_png_scaled(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse error: {e}"))?;

    let size = tree.size().to_int_size();
    let w = ((size.width() as f32) * scale).ceil() as u32;
    let h = ((size.height() as f32) * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("failed to create pixmap")?;

    // White background.
    pixmap.fill(tiny_skia::Color::WHITE);

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| format!("PNG encoding error: {e}"))
}

/// Read PNG width and height from the IHDR chunk (bytes 16..24 of a valid PNG).
pub fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || data[..4] != [137, 80, 78, 71] {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_svg_to_png() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
            <rect x="10" y="10" width="80" height="30" fill="#E2E2F0" stroke="#000" stroke-width="1"/>
            <text x="50" y="30" text-anchor="middle" font-size="13">Hello</text>
        </svg>"##;

        let png = svg_to_png(svg).unwrap();
        // PNG magic bytes.
        assert_eq!(&png[..4], &[137, 80, 78, 71]);
        assert!(png.len() > 100, "PNG should have content");
    }

    #[test]
    fn png_dimensions_match_svg_viewbox() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">
            <rect width="200" height="100" fill="white"/>
        </svg>"##;

        let png = svg_to_png(svg).unwrap();
        let (w, h) = png_dimensions(&png).expect("valid PNG");
        assert_eq!(w, 200);
        assert_eq!(h, 100);
    }

    #[test]
    fn png_scaled_2x() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
            <rect width="100" height="50" fill="white"/>
        </svg>"##;

        let png = svg_to_png_scaled(svg, 2.0).unwrap();
        let (w, h) = png_dimensions(&png).expect("valid PNG");
        assert_eq!(w, 200);
        assert_eq!(h, 100);
    }

    #[test]
    fn end_to_end_diagram_to_png() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let png = svg_to_png(&svg).unwrap();
        assert_eq!(&png[..4], &[137, 80, 78, 71]);
        let (w, h) = png_dimensions(&png).expect("valid PNG");
        assert!(w > 0 && h > 0, "PNG dimensions must be non-zero");
    }
}
