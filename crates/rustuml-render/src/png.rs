// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! PNG output — rasterizes SVG to PNG using resvg.

use resvg::tiny_skia;
use resvg::usvg;

/// Convert SVG string to PNG bytes.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse error: {e}"))?;

    let size = tree.size().to_int_size();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("failed to create pixmap")?;

    // White background.
    pixmap.fill(tiny_skia::Color::WHITE);

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| format!("PNG encoding error: {e}"))
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
    fn end_to_end_diagram_to_png() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let png = svg_to_png(&svg).unwrap();
        assert_eq!(&png[..4], &[137, 80, 78, 71]);
    }
}
