// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Text metrics — approximate text dimensions for layout.
//!
//! Uses proportional character widths based on typical sans-serif fonts
//! (Liberation Sans / DejaVu Sans). This is an approximation; proper
//! font metrics will be added when we embed font data.

/// Approximate width of a string in pixels at the given font size.
pub fn text_width(text: &str, font_size: f64) -> f64 {
    let scale = font_size / 14.0; // Normalize to 14pt baseline.
    text.chars().map(|c| char_width(c) * scale).sum()
}

/// Approximate height of a line of text at the given font size.
pub fn text_height(font_size: f64) -> f64 {
    font_size * 1.2 // Line height ~120% of font size.
}

/// Proportional character width at 14pt (typical sans-serif).
/// Values are approximate relative widths in pixels.
fn char_width(c: char) -> f64 {
    match c {
        // Narrow characters.
        'i' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | '`' => 4.0,
        'I' | 'j' | 'f' | 'r' | 't' => 5.0,
        '1' | '(' | ')' | '[' | ']' | '{' | '}' => 5.5,

        // Medium-narrow.
        'a' | 'c' | 'e' | 's' | 'z' => 7.5,
        'b' | 'd' | 'g' | 'h' | 'k' | 'n' | 'o' | 'p' | 'q' | 'u' | 'v' | 'x' | 'y' => 8.0,

        // Medium.
        'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'K' | 'L' | 'N' | 'P' | 'R' | 'S' | 'T'
        | 'U' | 'V' | 'X' | 'Y' | 'Z' => 9.0,
        '0' | '2'..='9' => 8.0,

        // Wide characters.
        'm' | 'w' => 10.5,
        'M' | 'W' => 11.5,
        'O' | 'Q' => 10.0,

        // Symbols.
        ' ' => 4.0,
        '-' | '+' | '=' | '<' | '>' | '~' | '^' => 7.0,
        '_' => 7.0,
        '#' | '$' | '%' | '&' | '@' => 9.0,
        '*' => 6.0,
        '/' | '\\' => 5.5,
        '"' => 6.0,

        // Default for unknown/wide characters (CJK, emoji, etc.).
        _ => 10.0,
    }
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
    fn font_size_scales_linearly() {
        let w14 = text_width("hello", 14.0);
        let w28 = text_width("hello", 28.0);
        assert!((w28 - w14 * 2.0).abs() < 0.01);
    }

    #[test]
    fn typical_widths() {
        // "Alice" at 13pt should be roughly 35-45px.
        let w = text_width("Alice", 13.0);
        assert!(w > 25.0 && w < 55.0, "Alice width={w}");
    }
}
