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
//! reference host's fonts.
//!
//! HarfBuzz (10.0.1, which the reference JDK's libfontmanager links) also
//! zeroes the advance of an unmapped nonspacing mark (Unicode 16 Mn) in two
//! ways. With no GDEF glyph classes, `hb_synthesize_glyph_classes` makes the
//! mark a MARK glyph and the default shaper's `zero_mark_widths_by_gdef`
//! (BY_GDEF_LATE) zeroes it; with morx, any other shaper becomes
//! `_hb_ot_shaper_dumber`, which zeroes nothing here (hb-ot-shape.cc 94-96).
//! Without GPOS, kerx or cross-stream kern, a shaper with `fallback_position`
//! (default, arabic, hebrew) also runs `_hb_ot_shape_fallback_mark_position`,
//! which zeroes every Mn that follows a base in its cluster
//! (hb-ot-shape-fallback.cc 188-205, 412-462). The run's script comes from
//! JDK's `ScriptRun` over `ScriptRunData` (Unicode 3.2).

use crate::font_chain_coverage as coverage;
use crate::font_chain_unicode as unicode;

/// One face's coverage, as generated into `font_chain_coverage.rs`.
pub(crate) struct FaceCoverage {
    /// Advance of glyph 0 (`.notdef`) in 2048ths of an em.
    pub(crate) notdef_units: u16,
    /// BMP runs the simple path cannot map.
    pub(crate) simple_missing: &'static [(u32, u32)],
    /// Runs the complex path maps.
    pub(crate) complex_mapped: &'static [(u32, u32)],
    /// HarfBuzz zeroes the `.notdef` of an unmapped nonspacing mark in a
    /// default-shaped run (glyph 0 is a mark and the plan zeroes marks).
    pub(crate) zeroes_unmapped_marks: bool,
    /// `adjust_mark_positioning_when_zeroing` holds (no GPOS, kerx or
    /// cross-stream kern), so fallback-positioning shapers zero marks that
    /// follow a base.
    pub(crate) fallback_positions_marks: bool,
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
    /// HarfBuzz gives it zero advance in complex text: a hidden
    /// default-ignorable, or an unmapped nonspacing mark it zeroes.
    Hidden,
}

/// Which JDK measuring path a character takes and, for complex text, how
/// HarfBuzz shapes its script run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Complexity {
    /// `FontUtilities.isComplexText` is false for the measured string.
    Simple,
    /// `TextLayout` path: the run's shaper and whether a base (a character
    /// that is not a Unicode mark) precedes this one in its run with only
    /// marks between.
    Complex { shaper: RunShaper, after_base: bool },
}

/// The class of shaper `hb_ot_shaper_categorize` picks for a run's script on
/// a face without GSUB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunShaper {
    /// The default shaper (zeroes marks late, positions them by fallback).
    Default,
    /// arabic or hebrew: positions marks by fallback.
    FallbackPositioned,
    /// Any other complex shaper (no fallback positioning).
    Other,
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

    /// How the chain draws `ch` at a position whose complexity is `complexity`
    /// (see [`complexity_per_token`]).
    pub(crate) fn resolve(self, ch: char, complexity: Complexity) -> ChainGlyph {
        let code = ch as u32;
        let coverage = self.coverage();
        let (shaper, after_base) = match complexity {
            Complexity::Simple => (RunShaper::Default, false),
            Complexity::Complex { shaper, after_base } => (shaper, after_base),
        };
        if code >= FIRST_SUPPLEMENTARY || complexity != Complexity::Simple {
            let mark = in_runs(unicode::NONSPACING_MARKS, code);
            let zeroed_by_gdef = coverage.zeroes_unmapped_marks && shaper == RunShaper::Default;
            let zeroed_by_fallback =
                coverage.fallback_positions_marks && shaper != RunShaper::Other && after_base;
            return if in_runs(coverage.complex_mapped, code) {
                ChainGlyph::Mapped
            } else if is_hidden_default_ignorable(code)
                || (mark && (zeroed_by_gdef || zeroed_by_fallback))
            {
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

/// The [`Complexity`] of each Java token of `text` (one per
/// `java_utf16::tokens` item): `Simple` throughout unless the string is
/// complex, then each token's `ScriptRun` run decides its shaper.
pub(crate) fn complexity_per_token(text: &str) -> Vec<Complexity> {
    let codes: Vec<u32> = rustuml_parser::java_utf16::tokens(text)
        .map(|token| match token {
            rustuml_parser::java_utf16::Token::Unit(unit) => u32::from(unit),
            rustuml_parser::java_utf16::Token::Char(ch) => ch as u32,
        })
        .collect();
    if !is_complex_text(text) {
        return vec![Complexity::Simple; codes.len()];
    }
    let scripts = script_run_codes(&codes);
    (0..codes.len())
        .map(|index| {
            let script = scripts[index];
            let shaper = if !unicode::COMPLEX_SHAPED_SCRIPTS.contains(&script) {
                RunShaper::Default
            } else if unicode::FALLBACK_POSITIONED_SCRIPTS.contains(&script) {
                RunShaper::FallbackPositioned
            } else {
                RunShaper::Other
            };
            // `_hb_ot_shape_fallback_mark_position`: a cluster is a base and
            // the Unicode marks after it, inside the run's buffer.
            // Walking back inside the run over marks, the first non-mark is
            // the base.
            let after_base = (0..index)
                .rev()
                .take_while(|&before| scripts[before] == script)
                .find(|&before| !in_runs(unicode::UNICODE_MARKS, codes[before]))
                .is_some();
            Complexity::Complex { shaper, after_base }
        })
        .collect()
}

/// ICU `UScriptCode` of a code point in JDK's `ScriptRunData` (Unicode 3.2):
/// 0 (COMMON) for Common, Inherited and unassigned code points.
fn script_code(code: u32) -> u8 {
    let index = unicode::SCRIPTS.partition_point(|(start, _, _)| *start <= code);
    match index.checked_sub(1).map(|i| unicode::SCRIPTS[i]) {
        Some((_, end, script)) if code <= end => script,
        _ => COMMON_SCRIPT,
    }
}

const COMMON_SCRIPT: u8 = 0;
/// ICU INHERITED; codes up to it are weak (`ScriptRun.sameScript`).
const INHERITED_SCRIPT: u8 = 1;

/// `ScriptRun.pairedChars` (JDK 21): open/close pairs at even/odd indices.
const PAIRED_CHARS: [u32; 34] = [
    0x0028, 0x0029, 0x003c, 0x003e, 0x005b, 0x005d, 0x007b, 0x007d, 0x00ab, 0x00bb, 0x2018, 0x2019,
    0x201c, 0x201d, 0x2039, 0x203a, 0x3008, 0x3009, 0x300a, 0x300b, 0x300c, 0x300d, 0x300e, 0x300f,
    0x3010, 0x3011, 0x3014, 0x3015, 0x3016, 0x3017, 0x3018, 0x3019, 0x301a, 0x301b,
];

/// Port of `sun.font.ScriptRun.next` (JDK 21, 150-232) over the whole string:
/// the script code of the run holding each code point. Weak (COMMON and
/// INHERITED) code points join the current run, and a run's script is its
/// first strong script; a closing paired character takes the script recorded
/// with its opener (popping the paren stack as the JDK does).
fn script_run_codes(codes: &[u32]) -> Vec<u8> {
    let mut result = vec![COMMON_SCRIPT; codes.len()];
    let mut stack: Vec<i32> = Vec::new();
    let mut paren_sp: i32 = 0;
    let mut limit = 0;
    while limit < codes.len() {
        let start = limit;
        let mut start_sp = paren_sp;
        let mut run_script = i32::from(COMMON_SCRIPT);
        while limit < codes.len() {
            let ch = codes[limit];
            let mut sc = i32::from(script_code(ch));
            let pair_index = if sc == i32::from(COMMON_SCRIPT) {
                PAIRED_CHARS
                    .iter()
                    .position(|&p| p == ch)
                    .map_or(-1, |i| i as i32)
            } else {
                -1
            };
            if pair_index >= 0 {
                if pair_index & 1 == 0 {
                    stack.truncate(paren_sp as usize);
                    stack.push(pair_index);
                    stack.push(run_script);
                    paren_sp += 2;
                } else if paren_sp > 0 {
                    let pi = pair_index & !1;
                    loop {
                        paren_sp -= 2;
                        if paren_sp < 0 || stack[paren_sp as usize] == pi {
                            break;
                        }
                    }
                    if paren_sp >= 0 {
                        sc = stack[paren_sp as usize + 1];
                    } else {
                        paren_sp = 0;
                    }
                    if paren_sp < start_sp {
                        start_sp = paren_sp;
                    }
                }
            }
            let weak = |code: i32| code <= i32::from(INHERITED_SCRIPT);
            if run_script == sc || weak(run_script) || weak(sc) {
                if weak(run_script) && !weak(sc) {
                    run_script = sc;
                    while start_sp < paren_sp {
                        stack[start_sp as usize + 1] = run_script;
                        start_sp += 2;
                    }
                }
                if pair_index > 0 && pair_index & 1 != 0 && paren_sp > 0 {
                    paren_sp -= 2;
                }
                limit += 1;
            } else {
                break;
            }
        }
        for slot in &mut result[start..limit] {
            *slot = run_script as u8;
        }
    }
    result
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

    const COMPLEX: Complexity = Complexity::Complex {
        shaper: RunShaper::Default,
        after_base: true,
    };

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
            assert_eq!(lucida.resolve(ch, COMPLEX), ChainGlyph::Missing, "{ch:?}");
        }
        assert_eq!(
            lucida.resolve('\u{0378}', Complexity::Simple),
            ChainGlyph::Missing
        );
        assert_eq!(lucida.resolve('a', Complexity::Simple), ChainGlyph::Mapped);
        assert_eq!(lucida.notdef_per_em(), 0.75);
        assert_eq!(ChainFace::Verdana.notdef_per_em(), 1.0);
    }

    #[test]
    fn harfbuzz_hides_unmapped_supplementary_format_characters() {
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{E0041}', COMPLEX),
            ChainGlyph::Hidden
        );
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1D173}', COMPLEX),
            ChainGlyph::Hidden
        );
        // U+1BCA0 is excluded from HarfBuzz's set.
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1BCA0}', COMPLEX),
            ChainGlyph::Missing
        );
    }

    #[test]
    fn harfbuzz_zeroes_unmapped_nonspacing_marks_of_default_shaped_runs() {
        // Synthesized glyph classes (no GDEF): Mn U+180F, U+1D17B, U+16FE4 and
        // the Unicode 16 U+1E5EE are marks; the default shaper zeroes them,
        // with or without a base before them.
        let leading = Complexity::Complex {
            shaper: RunShaper::Default,
            after_base: false,
        };
        for face in [
            ChainFace::LucidaGrande,
            ChainFace::LucidaGrandeBold,
            ChainFace::Helvetica,
            ChainFace::Menlo,
        ] {
            for ch in [
                '\u{180F}',
                '\u{1D17B}',
                '\u{16FE4}',
                '\u{1DFA}',
                '\u{1E5EE}',
            ] {
                assert_eq!(
                    face.resolve(ch, COMPLEX),
                    ChainGlyph::Hidden,
                    "{face:?} {ch:?}"
                );
                assert_eq!(
                    face.resolve(ch, leading),
                    ChainGlyph::Hidden,
                    "{face:?} {ch:?}"
                );
            }
            // Hebrew (fallback-positioned, dumber under morx): zeroed only
            // after a base.
            let hebrew = |after_base| Complexity::Complex {
                shaper: RunShaper::FallbackPositioned,
                after_base,
            };
            assert_eq!(face.resolve('\u{1D17B}', hebrew(true)), ChainGlyph::Hidden);
            assert_eq!(
                face.resolve('\u{1D17B}', hebrew(false)),
                ChainGlyph::Missing
            );
            // Thai/Indic/USE: never.
            let other = Complexity::Complex {
                shaper: RunShaper::Other,
                after_base: true,
            };
            assert_eq!(face.resolve('\u{1D17B}', other), ChainGlyph::Missing);
        }
        // GDEF classes glyph 0 as a base and GPOS disables fallback marks.
        for face in [ChainFace::Arial, ChainFace::Verdana, ChainFace::CourierNew] {
            assert_eq!(
                face.resolve('\u{1D17B}', COMPLEX),
                ChainGlyph::Missing,
                "{face:?}"
            );
        }
        // Simple text never reaches HarfBuzz.
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1AB0}', Complexity::Simple),
            ChainGlyph::Missing
        );
        // Mc (U+1D165 is mapped elsewhere; U+11F03 Kawi Mc) is not zeroed.
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{11F03}', COMPLEX),
            ChainGlyph::Missing
        );
    }

    #[test]
    fn script_runs_follow_jdk_script_run() {
        let runs = |text: &str| {
            complexity_per_token(text)
                .into_iter()
                .map(|c| match c {
                    Complexity::Simple => 's',
                    Complexity::Complex {
                        shaper: RunShaper::Default,
                        ..
                    } => 'd',
                    Complexity::Complex {
                        shaper: RunShaper::FallbackPositioned,
                        ..
                    } => 'f',
                    Complexity::Complex {
                        shaper: RunShaper::Other,
                        ..
                    } => 'o',
                })
                .collect::<String>()
        };
        let bases = |text: &str| {
            complexity_per_token(text)
                .into_iter()
                .map(|c| match c {
                    Complexity::Complex {
                        after_base: true, ..
                    } => 'b',
                    _ => '-',
                })
                .collect::<String>()
        };
        assert_eq!(runs("Vault"), "sssss");
        // Weak characters (unassigned in Unicode 3.2) join the Latin run.
        assert_eq!(runs("Vault \u{1D17B}"), "ddddddd");
        // Hebrew is fallback-positioned; the mark after it joins its run.
        assert_eq!(runs("a \u{05D0}\u{1D17B}"), "ddff");
        assert_eq!(bases("a \u{05D0}\u{1D17B}"), "-b-b");
        // A leading mark takes the first strong script after it, and has no base.
        assert_eq!(runs("\u{1D17B}\u{05D0}"), "ff");
        assert_eq!(bases("\u{1D17B}\u{05D0}"), "--");
        // Thai is neither default nor fallback-positioned.
        assert_eq!(runs("\u{0E01}\u{1D17B}"), "oo");
        // A closing bracket takes its opener's run script.
        assert_eq!(runs("\u{05D0}(\u{1D17B})"), "ffff");
        // When that script ends the current run, the opener is already popped:
        // the bracket starts a weak (COMMON, default-shaped) run.
        assert_eq!(runs("\u{05D0}(a)\u{1D17B}"), "ffddd");
        // Marks between the base and the mark keep the cluster.
        assert_eq!(bases("\u{05D0}\u{0301}\u{1D17B}"), "-bb");
    }

    /// Drift gate (T14.6.8): the generated table is the stored reference-host
    /// extraction. Its SHA256SUMS hashes to the recorded value, every stored
    /// file matches SHA256SUMS, and each face's runs, `.notdef` advance and
    /// HarfBuzz plan fact equal the stored TSV.
    #[test]
    fn coverage_table_is_the_stored_reference_host_extraction() {
        use sha2::{Digest, Sha256};
        let hex = |bytes: &[u8]| {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(coverage::SOURCE_DIR);
        let sums = std::fs::read(dir.join("SHA256SUMS")).expect("stored SHA256SUMS");
        assert_eq!(hex(&sums), coverage::SOURCE_SHA256SUMS_SHA256);
        for line in String::from_utf8(sums).unwrap().lines() {
            let (hash, name) = line.split_once("  ").expect("shasum line");
            let data = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(hex(&data), hash, "{name}");
        }
        let faces = [
            (ChainFace::LucidaGrande, "LucidaGrande.tsv"),
            (ChainFace::LucidaGrandeBold, "LucidaGrande-Bold.tsv"),
            (ChainFace::Arial, "ArialMT.tsv"),
            (ChainFace::ArialBold, "Arial-BoldMT.tsv"),
            (ChainFace::Helvetica, "Helvetica.tsv"),
            (ChainFace::HelveticaBold, "Helvetica-Bold.tsv"),
            (ChainFace::Verdana, "Verdana.tsv"),
            (ChainFace::VerdanaBold, "Verdana-Bold.tsv"),
            (ChainFace::Menlo, "Menlo-Regular.tsv"),
            (ChainFace::CourierNew, "CourierNewPSMT.tsv"),
        ];
        for (face, name) in faces {
            let text = std::fs::read_to_string(dir.join(name)).unwrap();
            let (mut simple, mut complex) = (Vec::new(), Vec::new());
            for line in text.lines() {
                let fields: Vec<&str> = line.split('\t').collect();
                match fields[0] {
                    "#notdef_advance_units" => assert_eq!(
                        fields[1].parse::<f64>().unwrap(),
                        f64::from(face.coverage().notdef_units),
                        "{name}"
                    ),
                    "#hb_plan" => {
                        let fact = |key: &str| {
                            fields
                                .iter()
                                .find_map(|f| f.strip_prefix(key)?.strip_prefix('='))
                                .unwrap()
                        };
                        let notdef_is_mark =
                            fact("gdef_glyph_classes") == "0" || fact("notdef_gdef_class") == "3";
                        let zero_marks = fact("kerx") == "0"
                            && !(fact("kern") == "1" && fact("machine_kern") == "1");
                        assert_eq!(
                            notdef_is_mark && zero_marks,
                            face.coverage().zeroes_unmapped_marks,
                            "{name}"
                        );
                        // Fallback marks need no GPOS and no kerx; with a
                        // kern table the cross-stream bit (not extracted)
                        // also decides, so only the extracted part is gated.
                        if fact("gpos") == "1" || fact("kerx") == "1" {
                            assert!(!face.coverage().fallback_positions_marks, "{name}");
                        } else if fact("kern") == "0" {
                            assert!(face.coverage().fallback_positions_marks, "{name}");
                        }
                    }
                    "S" | "C" => {
                        let run = (
                            u32::from_str_radix(fields[1], 16).unwrap(),
                            u32::from_str_radix(fields[2], 16).unwrap(),
                        );
                        if fields[0] == "S" {
                            &mut simple
                        } else {
                            &mut complex
                        }
                        .push(run);
                    }
                    _ => {}
                }
            }
            simple.sort_unstable();
            complex.sort_unstable();
            assert_eq!(simple, face.coverage().simple_missing, "{name} simple");
            assert_eq!(complex, face.coverage().complex_mapped, "{name} complex");
        }
    }

    #[test]
    fn complexity_follows_java_units() {
        assert!(!is_complex_text("Vault"));
        assert!(is_complex_text("Vault \u{10FFFD}"));
        assert!(is_complex_text("a\u{0301}"));
        assert!(!is_complex_text("\u{A500}"));
    }
}
