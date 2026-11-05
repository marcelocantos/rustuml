// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Whether Java's macOS font chain can map a character, and the advance of
//! the missing glyph when it cannot.
//!
//! PlantUML measures text with `FontMetrics.getStringBounds`
//! (`FileFormat.getJavaDimension`). JDK 21's `Font.getStringBounds` takes one
//! of two paths:
//!
//! * simple text (`FontUtilities.isComplexText` false) is measured glyph by
//!   glyph through the `CFont`'s `CCharToGlyphMapper`: the font's own glyph,
//!   else the glyph CoreText substitutes; a character no system font maps is
//!   glyph 0, the primary font's `.notdef`;
//! * complex text (any UTF-16 surrogate, or a character with
//!   `FontUtilities.isComplexCharCode`) goes through `TextLayout` over
//!   `CFont.createCompositeFont` (the font plus its CoreText cascade list and
//!   GeezaPro): the first slot whose cmap maps the character, else slot 0's
//!   `.notdef`. HarfBuzz hides default-ignorable code points (zero advance).
//!
//! The coverage tables are an environment extraction
//! (`tools/FontChainExtract.swift`, `tools/font_chain_coverage.py`) of the
//! origin Mac's system fonts.

use crate::font_chain_coverage as coverage;

/// One face's coverage, as generated into `font_chain_coverage.rs`.
pub(crate) struct FaceCoverage {
    /// Advance of glyph 0 (`.notdef`) in 2048ths of an em.
    pub(crate) notdef_units: u16,
    /// BMP runs the simple path cannot map.
    pub(crate) simple_missing: &'static [(u32, u32)],
    /// Runs the complex path maps.
    pub(crate) complex_mapped: &'static [(u32, u32)],
}

/// The physical faces RustUML measures (`SansSerif` resolves to Lucida
/// Grande and `Monospaced` to Menlo on macOS, `CFontManager`). Menlo and
/// Courier New have one face here: RustUML measures their bold like plain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChainFace {
    LucidaGrande,
    LucidaGrandeBold,
    Arial,
    ArialBold,
    Helvetica,
    HelveticaBold,
    Verdana,
    VerdanaBold,
    Menlo,
    CourierNew,
}

/// How the chain draws one character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChainGlyph {
    /// Some font of the chain has a glyph; its advance is measured elsewhere.
    Mapped,
    /// No font maps it: the primary font's `.notdef`.
    Missing,
    /// HarfBuzz hides it in complex text (zero advance).
    Hidden,
}

/// Units per em of the extracted advances.
const UNITS_PER_EM: f64 = 2048.0;
const FIRST_SUPPLEMENTARY: u32 = 0x1_0000;

impl ChainFace {
    fn coverage(self) -> &'static FaceCoverage {
        match self {
            Self::LucidaGrande => &coverage::LUCIDA_GRANDE,
            Self::LucidaGrandeBold => &coverage::LUCIDA_GRANDE_BOLD,
            Self::Arial => &coverage::ARIAL,
            Self::ArialBold => &coverage::ARIAL_BOLD,
            Self::Helvetica => &coverage::HELVETICA,
            Self::HelveticaBold => &coverage::HELVETICA_BOLD,
            Self::Verdana => &coverage::VERDANA,
            Self::VerdanaBold => &coverage::VERDANA_BOLD,
            Self::Menlo => &coverage::MENLO,
            Self::CourierNew => &coverage::COURIER_NEW,
        }
    }

    /// The `.notdef` advance per unit of font size.
    pub(crate) fn notdef_per_em(self) -> f64 {
        f64::from(self.coverage().notdef_units) / UNITS_PER_EM
    }

    /// How the chain draws `ch` inside text whose complexity is `complex_text`
    /// (see [`is_complex_text`]).
    pub(crate) fn resolve(self, ch: char, complex_text: bool) -> ChainGlyph {
        let code = ch as u32;
        let coverage = self.coverage();
        if code >= FIRST_SUPPLEMENTARY || complex_text {
            return if in_runs(coverage.complex_mapped, code) {
                ChainGlyph::Mapped
            } else if is_hidden_default_ignorable(code) {
                ChainGlyph::Hidden
            } else {
                ChainGlyph::Missing
            };
        }
        if in_runs(coverage.simple_missing, code) {
            ChainGlyph::Missing
        } else {
            ChainGlyph::Mapped
        }
    }
}

fn in_runs(runs: &[(u32, u32)], code: u32) -> bool {
    let index = runs.partition_point(|(start, _)| *start <= code);
    index > 0 && code <= runs[index - 1].1
}

/// `sun.font.FontUtilities.isComplexText` over the Java units of carried
/// text: any surrogate unit (an astral scalar or an unpaired unit) or a
/// character with `isComplexCharCode`.
pub(crate) fn is_complex_text(text: &str) -> bool {
    rustuml_parser::java_utf16::tokens(text).any(|token| match token {
        rustuml_parser::java_utf16::Token::Unit(_) => true,
        rustuml_parser::java_utf16::Token::Char(ch) => {
            ch as u32 >= FIRST_SUPPLEMENTARY || is_complex_char_code(ch as u32)
        }
    })
}

/// `sun.font.FontUtilities.isComplexCharCode` (JDK 21, 239-309).
fn is_complex_char_code(code: u32) -> bool {
    matches!(
        code,
        0x0300..=0x036F
            | 0x0590..=0x06FF
            | 0x0900..=0x0E7F
            | 0x0F00..=0x109F
            | 0x1100..=0x11FE
            | 0x1780..=0x17FF
            | 0x200C..=0x200D
            | 0x202A..=0x202E
            | 0x206A..=0x206F
    )
}

/// HarfBuzz `_hb_unicode_is_default_ignorable`, the code points TextLayout
/// hides with zero advance. Only unmapped ones reach this test.
fn is_hidden_default_ignorable(code: u32) -> bool {
    matches!(
        code,
        0x00AD
            | 0x034F
            | 0x061C
            | 0x17B4..=0x17B5
            | 0x180B..=0x180E
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x206F
            | 0xFE00..=0xFE0F
            | 0xFEFF
            | 0xFFF0..=0xFFF8
            | 0x1D173..=0x1D17A
            | 0xE0000..=0xE0FFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmapped_scalars_take_the_primary_notdef_on_both_paths() {
        let lucida = ChainFace::LucidaGrande;
        for ch in [
            '\u{10FFFD}',
            '\u{10FFFF}',
            '\u{3FFFD}',
            '\u{F0000}',
            '\u{0378}',
            '\u{F000}',
        ] {
            assert_eq!(lucida.resolve(ch, true), ChainGlyph::Missing, "{ch:?}");
        }
        assert_eq!(lucida.resolve('\u{0378}', false), ChainGlyph::Missing);
        assert_eq!(lucida.resolve('a', false), ChainGlyph::Mapped);
        assert_eq!(lucida.notdef_per_em(), 0.75);
        assert_eq!(ChainFace::Verdana.notdef_per_em(), 1.0);
    }

    #[test]
    fn harfbuzz_hides_unmapped_supplementary_format_characters() {
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{E0041}', true),
            ChainGlyph::Hidden
        );
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1D173}', true),
            ChainGlyph::Hidden
        );
        // U+1BCA0 is excluded from HarfBuzz's set.
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1BCA0}', true),
            ChainGlyph::Missing
        );
    }

    #[test]
    fn complexity_follows_java_units() {
        assert!(!is_complex_text("Vault"));
        assert!(is_complex_text("Vault \u{10FFFD}"));
        assert!(is_complex_text("a\u{0301}"));
        assert!(!is_complex_text("\u{A500}"));
    }
}
