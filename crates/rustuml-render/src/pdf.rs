// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! PDF output — converts SVG to PDF using svg2pdf.

/// Convert SVG string to PDF bytes.
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let opt = svg2pdf::usvg::Options::default();
    let tree =
        svg2pdf::usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse error: {e}"))?;
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| format!("PDF conversion error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_svg_to_pdf() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
            <rect x="10" y="10" width="80" height="30" fill="#E2E2F0" stroke="#000" stroke-width="1"/>
            <text x="50" y="30" text-anchor="middle" font-size="13">Hello</text>
        </svg>"##;

        let pdf = svg_to_pdf(svg).unwrap();
        // PDF magic bytes: %PDF
        assert_eq!(&pdf[..5], b"%PDF-");
        assert!(pdf.len() > 100, "PDF should have content");
    }

    #[test]
    fn end_to_end_diagram_to_pdf() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let pdf = svg_to_pdf(&svg).unwrap();
        assert_eq!(&pdf[..5], b"%PDF-");
    }
}
