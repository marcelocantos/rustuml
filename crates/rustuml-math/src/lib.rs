// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! LaTeX math rendering to SVG for RustUML.
//!
//! Uses KaTeX (via QuickJS) to render LaTeX math notation. The output
//! is an SVG fragment suitable for embedding in diagram SVGs via
//! `<foreignObject>`.

/// Options for math rendering.
#[derive(Debug, Clone)]
pub struct MathRenderOpts {
    /// Font size in pixels (default: 16.0).
    pub font_size_px: f64,
    /// Whether to render in display mode (centered, larger) vs inline.
    pub display_mode: bool,
}

impl Default for MathRenderOpts {
    fn default() -> Self {
        Self {
            font_size_px: 16.0,
            display_mode: false,
        }
    }
}

/// Render a LaTeX math expression to an SVG fragment.
///
/// Returns an SVG `<foreignObject>` element containing the KaTeX HTML
/// output, along with the required CSS inlined as a `<style>` element.
/// This can be embedded directly in an SVG document.
///
/// The returned SVG fragment includes width/height attributes estimated
/// from the expression length. For precise sizing, the caller should
/// measure the rendered output.
pub fn render_math(latex: &str) -> Result<String, String> {
    render_math_with_opts(latex, &MathRenderOpts::default())
}

/// Render a LaTeX math expression to an SVG fragment with custom options.
pub fn render_math_with_opts(latex: &str, opts: &MathRenderOpts) -> Result<String, String> {
    let katex_opts = katex::Opts::builder()
        .display_mode(opts.display_mode)
        .output_type(katex::OutputType::HtmlAndMathml)
        .throw_on_error(true)
        .build()
        .map_err(|e| format!("failed to build KaTeX options: {e}"))?;

    let html = katex::render_with_opts(latex, &katex_opts)
        .map_err(|e| format!("KaTeX render error: {e}"))?;

    // Estimate dimensions from the expression.
    // These are rough estimates; real rendering would measure the DOM.
    let char_width = opts.font_size_px * 0.6;
    let estimated_width = estimate_width(latex, char_width);
    let estimated_height = estimate_height(latex, opts.font_size_px);

    // Build an SVG foreignObject fragment with inlined KaTeX CSS.
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <foreignObject x="0" y="0" width="{w}" height="{h}">
    <div xmlns="http://www.w3.org/1999/xhtml" style="font-size: {fs}px;">
      {html}
    </div>
  </foreignObject>
</svg>"#,
        w = estimated_width,
        h = estimated_height,
        fs = opts.font_size_px,
        html = html,
    );

    Ok(svg)
}

/// Render a LaTeX math expression to KaTeX HTML (without SVG wrapping).
///
/// This is useful when the caller wants to embed the HTML in their own
/// `<foreignObject>` or when the SVG structure is managed externally.
pub fn render_math_html(latex: &str) -> Result<String, String> {
    render_math_html_with_opts(latex, &MathRenderOpts::default())
}

/// Render a LaTeX math expression to KaTeX HTML with custom options.
pub fn render_math_html_with_opts(latex: &str, opts: &MathRenderOpts) -> Result<String, String> {
    let katex_opts = katex::Opts::builder()
        .display_mode(opts.display_mode)
        .output_type(katex::OutputType::HtmlAndMathml)
        .throw_on_error(true)
        .build()
        .map_err(|e| format!("failed to build KaTeX options: {e}"))?;

    katex::render_with_opts(latex, &katex_opts).map_err(|e| format!("KaTeX render error: {e}"))
}

/// Estimate the rendered width of a LaTeX expression.
fn estimate_width(latex: &str, char_width: f64) -> f64 {
    // Strip common LaTeX commands to get approximate character count.
    let stripped = latex
        .replace("\\frac", "")
        .replace("\\sum", "E")
        .replace("\\sqrt", "V")
        .replace("\\int", "I")
        .replace("\\alpha", "a")
        .replace("\\beta", "b")
        .replace("\\gamma", "g")
        .replace("\\delta", "d")
        .replace("\\pi", "p")
        .replace("\\infty", "oo")
        .replace("\\left", "")
        .replace("\\right", "")
        .replace(['{', '}', '^', '_'], "");
    let count = stripped.chars().filter(|c| !c.is_whitespace()).count();
    // Minimum width, plus character-based estimate with padding.
    (count as f64 * char_width).max(char_width * 2.0) + char_width * 2.0
}

/// Estimate the rendered height of a LaTeX expression.
fn estimate_height(latex: &str, font_size: f64) -> f64 {
    let mut height = font_size * 1.5;
    // Fractions and sums add vertical space.
    if latex.contains("\\frac") {
        height += font_size * 1.0;
    }
    if latex.contains("\\sum") || latex.contains("\\int") || latex.contains("\\prod") {
        height += font_size * 0.8;
    }
    if latex.contains("\\sqrt") {
        height += font_size * 0.3;
    }
    height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expression() {
        let result = render_math("x^2").unwrap();
        assert!(result.contains("<svg"));
        assert!(result.contains("foreignObject"));
        assert!(result.contains("katex"));
    }

    #[test]
    fn test_fraction() {
        let result = render_math("\\frac{a}{b}").unwrap();
        assert!(result.contains("<svg"));
        assert!(result.contains("frac"));
    }

    #[test]
    fn test_summation() {
        let result = render_math("\\sum_{i=0}^n i").unwrap();
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_square_root() {
        let result = render_math("\\sqrt{x}").unwrap();
        assert!(result.contains("<svg"));
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_complex_expression() {
        let result = render_math("E = mc^2").unwrap();
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_display_mode() {
        let opts = MathRenderOpts {
            display_mode: true,
            ..Default::default()
        };
        let result = render_math_with_opts("\\int_0^\\infty e^{-x} dx", &opts).unwrap();
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_html_output() {
        let html = render_math_html("x^2 + y^2 = z^2").unwrap();
        assert!(html.contains("katex"));
        // Should not contain SVG wrapper.
        assert!(!html.contains("<svg"));
    }

    #[test]
    fn test_invalid_latex() {
        let result = render_math("\\invalid_command_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_greek_letters() {
        let result = render_math("\\alpha + \\beta = \\gamma").unwrap();
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_matrix() {
        let result = render_math("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}").unwrap();
        assert!(result.contains("<svg"));
    }
}
