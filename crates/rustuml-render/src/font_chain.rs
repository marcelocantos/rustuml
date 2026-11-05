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
//! (`tools/FontChainExtract.swift`, `tools/PrimaryFaceExtract.swift`,
//! `tools/font_chain_coverage.py`) of the reference host's fonts.
//!
//! HarfBuzz 15.0.0 (the library the reference JVM loads, T14.6.8) can also
//! zero the advance of an unmapped character. JDK shapes one HarfBuzz buffer
//! per `ScriptRun` run and composite slot run (`GlyphLayout.layout` 408-431,
//! `FontRunIterator.next` 115-126). An unmapped character is in slot 0, the
//! primary face, as is every character the primary face's own cmap maps
//! (`CCompositeGlyphMapper.convertToGlyph` 54-67; `CTS_GetGlyphsAsIntsForCharacters`
//! gives a positive code only for the font's own glyph). For each slot-0
//! buffer, [`plan`] ports the planner (hb-ot-shape.cc 85-221) and
//! [`fallback_zeroed`] ports `_hb_ot_shape_fallback_mark_position`
//! (hb-ot-shape-fallback.cc 426-497).

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
    /// Runs the face's own cmap maps (composite slot 0).
    pub(crate) primary_mapped: &'static [(u32, u32)],
    /// The sfnt facts HarfBuzz's planner reads.
    pub(crate) plan: PlanFacts,
}

/// The sfnt facts HarfBuzz 15.0.0's planner reads for a face (horizontal text).
pub(crate) struct PlanFacts {
    /// GDEF has glyph classes (else `hb_synthesize_glyph_classes`).
    pub(crate) gdef_glyph_classes: bool,
    /// GDEF class of glyph 0 (`.notdef`), when there are classes.
    pub(crate) notdef_gdef_class: u8,
    pub(crate) gsub: bool,
    pub(crate) gpos: bool,
    pub(crate) morx: bool,
    pub(crate) kerx: bool,
    pub(crate) kern: bool,
    /// A kern subtable is a state machine (`hb_ot_layout_has_machine_kerning`).
    pub(crate) machine_kern: bool,
    /// A kern subtable is cross-stream (`hb_ot_layout_has_cross_kerning`).
    pub(crate) cross_kern: bool,
    /// GPOS's ScriptList has `hebr`, the hebrew shaper's `gpos_tag`.
    pub(crate) gpos_hebrew_script: bool,
}

/// GDEF glyph class MARK.
const GDEF_MARK_CLASS: u8 = 3;

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
    /// default-ignorable, or an unmapped character its plan zeroes.
    Hidden,
}

/// Which JDK measuring path a character takes and, for complex text,
/// whether HarfBuzz zeroes the advance of its glyph when it is unmapped
/// (glyph 0 of slot 0) in its buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Complexity {
    /// `FontUtilities.isComplexText` is false for the measured string.
    Simple,
    /// `TextLayout` path.
    Complex { zeroed: bool },
}

/// HarfBuzz 15.0.0's `hb_ot_shaper_t`s that `hb_ot_shaper_categorize`
/// (hb-ot-shaper.hh) picks for the ICU scripts JDK maps, plus `dumber`,
/// which replaces any non-default shaper when morx applies (hb-ot-shape.cc
/// 99-102).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shaper {
    Default,
    Dumber,
    Arabic,
    Hebrew,
    Hangul,
    Thai,
    Indic,
    Khmer,
    Myanmar,
    Use,
}

/// `hb_ot_shape_zero_width_marks_type_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ZeroWidthMarks {
    None,
    ByGdefEarly,
    ByGdefLate,
}

impl Shaper {
    pub(crate) const ALL: [Shaper; 10] = [
        Shaper::Default,
        Shaper::Dumber,
        Shaper::Arabic,
        Shaper::Hebrew,
        Shaper::Hangul,
        Shaper::Thai,
        Shaper::Indic,
        Shaper::Khmer,
        Shaper::Myanmar,
        Shaper::Use,
    ];

    /// `hb_ot_shaper_categorize` for an ICU script on a face without GSUB
    /// (`chosen_script` NONE): the generated `SCRIPT_SHAPERS`, else default.
    fn for_script(script: u8) -> Self {
        unicode::SCRIPT_SHAPERS
            .binary_search_by_key(&script, |(code, _)| *code)
            .map_or(Shaper::Default, |index| unicode::SCRIPT_SHAPERS[index].1)
    }

    /// `zero_width_marks` (hb-ot-shaper-*.cc, 15.0.0).
    fn zero_width_marks(self) -> ZeroWidthMarks {
        match self {
            Shaper::Default | Shaper::Arabic | Shaper::Hebrew | Shaper::Thai => {
                ZeroWidthMarks::ByGdefLate
            }
            Shaper::Myanmar | Shaper::Use => ZeroWidthMarks::ByGdefEarly,
            Shaper::Dumber | Shaper::Hangul | Shaper::Indic | Shaper::Khmer => ZeroWidthMarks::None,
        }
    }

    /// `fallback_position` (hb-ot-shaper-*.cc, 15.0.0; hangul's is true since
    /// HarfBuzz 12.2.0).
    fn fallback_position(self) -> bool {
        matches!(
            self,
            Shaper::Default | Shaper::Arabic | Shaper::Hebrew | Shaper::Hangul
        )
    }
}

/// What a HarfBuzz plan does to the marks of one buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MarkPlan {
    /// `zero_mark_widths_by_gdef` runs (hb-ot-shape.cc 1051-1077): every
    /// MARK glyph gets zero advance.
    pub(crate) zero_by_gdef: bool,
    /// `_hb_ot_shape_fallback_mark_position` runs (1086-1090).
    pub(crate) fallback_mark_positioning: bool,
}

/// Port of `hb_ot_shape_planner_t` (hb-ot-shape.cc 85-103) and `compile`
/// (152-221), HarfBuzz 15.0.0, for horizontal text and the plan facts that
/// decide mark advances. `has_gpos_kern` (a GPOS `kern` feature for the
/// chosen script) is not extracted; the drift gate checks that it cannot
/// change the result for any face and shaper.
pub(crate) fn plan(facts: &PlanFacts, categorized: Shaper, has_gpos_kern: bool) -> MarkPlan {
    let script_zero_marks = categorized.zero_width_marks() != ZeroWidthMarks::None;
    let script_fallback_position = categorized.fallback_position();
    let apply_morx = facts.morx;
    let shaper = if apply_morx && categorized != Shaper::Default {
        Shaper::Dumber
    } else {
        categorized
    };
    // disable_gpos (133-134): only hebrew has a gpos_tag ('hebr'); GPOS's
    // chosen script is 'hebr' only when its ScriptList has it.
    let disable_gpos = shaper == Shaper::Hebrew && !facts.gpos_hebrew_script;
    let has_gsub = !apply_morx && facts.gsub;
    let has_gpos = !disable_gpos && facts.gpos;
    let mut apply_kerx = facts.kerx && !(has_gsub && has_gpos);
    let apply_gpos = !apply_kerx && has_gpos;
    let mut apply_kern = false;
    if !apply_kerx && (!has_gpos_kern || !apply_gpos) {
        if facts.kerx {
            apply_kerx = true;
        } else if facts.kern {
            // 15.0.0: "Not all shapers apply legacy `kern`".
            apply_kern = script_fallback_position;
        }
    }
    let zero_marks = script_zero_marks && !apply_kerx && (!apply_kern || !facts.machine_kern);
    let adjust_mark_positioning_when_zeroing =
        !apply_gpos && !apply_kerx && (!apply_kern || !facts.cross_kern);
    MarkPlan {
        zero_by_gdef: zero_marks && shaper.zero_width_marks() != ZeroWidthMarks::None,
        fallback_mark_positioning: adjust_mark_positioning_when_zeroing && script_fallback_position,
    }
}

/// Port of `_hb_ot_shape_fallback_mark_position` (hb-ot-shape-fallback.cc
/// 470-497) over one buffer's code points: which of them get zero advance.
/// Clusters split at characters that are neither Unicode marks nor hidden
/// or default-ignorable (hidden is set only on default-ignorables,
/// hb-ot-layout.hh 216-260). In a cluster of two or more,
/// `position_cluster_impl` (426-453) takes each non-mark as a base and its
/// marks up to the next non-mark, skipping default-ignorables;
/// `position_around_base` (324-342) cannot read glyph extents through JDK's
/// font functions (hb-jdk-font.cc 199-207), so `zero_mark_advances`
/// (189-208) zeroes every Mn after the base in that span.
fn fallback_zeroed(codes: &[u32]) -> Vec<bool> {
    let mark = |code: u32| in_runs(unicode::UNICODE_MARKS, code);
    let mut zeroed = vec![false; codes.len()];
    let mut position_cluster = |start: usize, end: usize| {
        if end - start < 2 {
            return;
        }
        let mut base = start;
        while base < end {
            if mark(codes[base]) {
                base += 1;
                continue;
            }
            let mut next = base + 1;
            while next < end && (is_default_ignorable(codes[next]) || mark(codes[next])) {
                next += 1;
            }
            for index in base + 1..next {
                if in_runs(unicode::NONSPACING_MARKS, codes[index]) {
                    zeroed[index] = true;
                }
            }
            base = next;
        }
    };
    let mut start = 0;
    for index in 1..codes.len() {
        if !mark(codes[index]) && !is_default_ignorable(codes[index]) {
            position_cluster(start, index);
            start = index;
        }
    }
    position_cluster(start, codes.len());
    zeroed
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
    /// (see [`ChainFace::complexity_per_token`]).
    pub(crate) fn resolve(self, ch: char, complexity: Complexity) -> ChainGlyph {
        let code = ch as u32;
        let coverage = self.coverage();
        if code >= FIRST_SUPPLEMENTARY || complexity != Complexity::Simple {
            let zeroed = complexity == Complexity::Complex { zeroed: true };
            return if in_runs(coverage.complex_mapped, code) {
                ChainGlyph::Mapped
            } else if is_default_ignorable(code) || zeroed {
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

    /// The [`Complexity`] of each Java token of `text` (one per
    /// `java_utf16::tokens` item) measured in this face: `Simple` throughout
    /// unless the string is complex. Complex text is split into HarfBuzz
    /// buffers, `ScriptRun` runs by slot runs; in a slot-0 buffer the plan
    /// for the run's script decides which unmapped glyphs it zeroes: MARK
    /// glyphs by GDEF (glyph 0's class, or Mn and not default-ignorable when
    /// classes are synthesized, hb-ot-shape.cc 856-879), and Mn after a base
    /// by fallback positioning.
    pub(crate) fn complexity_per_token(self, text: &str) -> Vec<Complexity> {
        let codes: Vec<u32> = rustuml_parser::java_utf16::tokens(text)
            .map(|token| match token {
                rustuml_parser::java_utf16::Token::Unit(unit) => u32::from(unit),
                rustuml_parser::java_utf16::Token::Char(ch) => ch as u32,
            })
            .collect();
        if !is_complex_text(text) {
            return vec![Complexity::Simple; codes.len()];
        }
        let coverage = self.coverage();
        let scripts = script_run_codes(&codes);
        let slot0: Vec<bool> = codes
            .iter()
            .map(|&code| {
                in_runs(coverage.primary_mapped, code) || !in_runs(coverage.complex_mapped, code)
            })
            .collect();
        let mut result = vec![Complexity::Complex { zeroed: false }; codes.len()];
        let mut start = 0;
        while start < codes.len() {
            let mut end = start + 1;
            while end < codes.len() && scripts[end] == scripts[start] && slot0[end] == slot0[start]
            {
                end += 1;
            }
            if slot0[start] {
                let plan = plan(&coverage.plan, Shaper::for_script(scripts[start]), false);
                let fallback = if plan.fallback_mark_positioning {
                    fallback_zeroed(&codes[start..end])
                } else {
                    vec![false; end - start]
                };
                for index in start..end {
                    let code = codes[index];
                    let glyph_is_mark = if coverage.plan.gdef_glyph_classes {
                        coverage.plan.notdef_gdef_class == GDEF_MARK_CLASS
                    } else {
                        in_runs(unicode::NONSPACING_MARKS, code) && !is_default_ignorable(code)
                    };
                    result[index] = Complexity::Complex {
                        zeroed: (plan.zero_by_gdef && glyph_is_mark) || fallback[index - start],
                    };
                }
            }
            start = end;
        }
        result
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

/// HarfBuzz 15.0.0 `is_default_ignorable` (hb-unicode.hh 167-197; U+180F
/// joined in 15.0.0's set): the code points TextLayout hides with zero
/// advance, and that fallback mark positioning skips.
fn is_default_ignorable(code: u32) -> bool {
    matches!(
        code,
        0x00AD
            | 0x034F
            | 0x061C
            | 0x17B4..=0x17B5
            | 0x180B..=0x180F
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

    const UNZEROED: Complexity = Complexity::Complex { zeroed: false };

    /// For each token of `text` in `face`: Simple, zeroed, or kept.
    fn zeroing(face: ChainFace, text: &str) -> String {
        face.complexity_per_token(text)
            .into_iter()
            .map(|c| match c {
                Complexity::Simple => 's',
                Complexity::Complex { zeroed: true } => 'z',
                Complexity::Complex { zeroed: false } => '-',
            })
            .collect()
    }

    /// Whether the last token of `text` draws as `glyph` in `face`.
    fn last_glyph(face: ChainFace, text: &str) -> ChainGlyph {
        let complexity = *face.complexity_per_token(text).last().unwrap();
        face.resolve(text.chars().last().unwrap(), complexity)
    }

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
            assert_eq!(lucida.resolve(ch, UNZEROED), ChainGlyph::Missing, "{ch:?}");
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
    fn harfbuzz_hides_unmapped_default_ignorables() {
        for ch in ['\u{E0041}', '\u{1D173}', '\u{180F}'] {
            assert_eq!(
                ChainFace::LucidaGrande.resolve(ch, UNZEROED),
                ChainGlyph::Hidden,
                "{ch:?}"
            );
        }
        // U+1BCA0 is excluded from HarfBuzz's set.
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1BCA0}', UNZEROED),
            ChainGlyph::Missing
        );
        // U+180F is default-ignorable in 15.0.0, so even a Thai run (thai ->
        // dumber, which zeroes nothing) hides it.
        assert_eq!(
            last_glyph(ChainFace::LucidaGrande, "Vault \u{0E01}\u{180F}"),
            ChainGlyph::Hidden
        );
    }

    #[test]
    fn default_shaped_runs_zero_unmapped_marks_by_gdef() {
        // Synthesized glyph classes (no GDEF): unmapped Mn are MARK glyphs and
        // the default shaper zeroes them with or without a base, including
        // the Unicode 17 and 18 marks of HarfBuzz 15.0.0's UCD.
        for face in [
            ChainFace::LucidaGrande,
            ChainFace::LucidaGrandeBold,
            ChainFace::Helvetica,
            ChainFace::HelveticaBold,
            ChainFace::Menlo,
        ] {
            for mark in [
                '\u{1D17B}',
                '\u{16FE4}',
                '\u{1E5EE}',
                '\u{1AD0}',
                '\u{1E6E3}',
                '\u{05C8}',
                '\u{0B53}',
                '\u{10EFA}',
            ] {
                for text in [
                    format!("Vault \u{10FFFD}{mark}"),
                    format!("{mark}Vault \u{10FFFD}"),
                ] {
                    let tokens = face.complexity_per_token(&text);
                    let at = text.chars().position(|c| c == mark).unwrap();
                    assert_eq!(
                        face.resolve(mark, tokens[at]),
                        ChainGlyph::Hidden,
                        "{face:?} {text:?}"
                    );
                }
            }
            // Mc and Me are not MARK glyphs under synthesized classes.
            for other in ['\u{11F03}', '\u{1ABE}'] {
                assert_eq!(
                    last_glyph(face, &format!("Vault \u{10FFFD}{other}")),
                    ChainGlyph::Missing
                );
            }
        }
        // GDEF classes make glyph 0 a base: never zeroed by GDEF.
        for face in [ChainFace::Arial, ChainFace::Verdana, ChainFace::CourierNew] {
            assert_eq!(
                last_glyph(face, "Vault \u{1D17B}"),
                ChainGlyph::Missing,
                "{face:?}"
            );
        }
        // Simple text never reaches HarfBuzz.
        assert_eq!(
            ChainFace::LucidaGrande.resolve('\u{1AB0}', Complexity::Simple),
            ChainGlyph::Missing
        );
    }

    #[test]
    fn fallback_positioning_needs_a_base_in_the_same_buffer() {
        use ChainFace::*;
        // BJ decompositions: Lucida Grande does not map alef U+0627 (Geeza
        // Pro slot), so the mark is alone in its slot-0 buffer.
        assert_eq!(zeroing(LucidaGrande, "Vault \u{0627}\u{1D17B}"), "--------");
        // Lucida Grande maps Hebrew alef itself: same buffer, zeroed.
        assert_eq!(zeroing(LucidaGrande, "Vault \u{05D0}\u{1D17B}"), "-------z");
        // Helvetica and Menlo do not (Lucida Grande slot): kept.
        for face in [Helvetica, Menlo] {
            assert_eq!(zeroing(face, "Vault \u{05D0}\u{1D17B}"), "--------");
            // A slot-0 space in the Hebrew run is a base.
            assert_eq!(zeroing(face, "Vault \u{05D0} \u{1D17B}"), "--------z");
        }
        // Syriac and U+FB1D are unmapped everywhere: slot 0, zeroed.
        assert_eq!(zeroing(Helvetica, "Vault \u{0710}\u{1D17B}"), "-------z");
        assert_eq!(zeroing(Helvetica, "Vault \u{FB1D}\u{1D17B}"), "-------z");
        // A leading mark has no base.
        assert_eq!(zeroing(LucidaGrande, "\u{1D17B}\u{05D0}"), "--");
        // Thai and Devanagari shapers do not position by fallback.
        assert_eq!(zeroing(LucidaGrande, "\u{0E01} \u{1D17B}"), "---");
        assert_eq!(zeroing(LucidaGrande, "\u{0915} \u{1D17B}"), "---");
        // A default-ignorable between the base and the mark is skipped.
        assert_eq!(zeroing(LucidaGrande, "\u{05D0}\u{2060}\u{1D17B}"), "--z");
        // GPOS faces: Arial and Courier New list 'hebr', so GPOS stays on and
        // no fallback runs; Verdana's GPOS lists only 'latn', so the hebrew
        // shaper's gpos_tag disables it and fallback positioning zeroes.
        assert_eq!(zeroing(Arial, "\u{05D0} \u{1D17B}"), "---");
        assert_eq!(zeroing(CourierNew, "\u{05D0} \u{1D17B}"), "---");
        assert_eq!(zeroing(Verdana, "\u{05D0} \u{1D17B}"), "--z");
    }

    #[test]
    fn fallback_port_follows_position_cluster() {
        let zeroed = |codes: &[u32]| {
            fallback_zeroed(codes)
                .into_iter()
                .map(|z| if z { 'z' } else { '-' })
                .collect::<String>()
        };
        // Base, Mn, Mc, Mn: every Mn after the base; Mc keeps its advance.
        assert_eq!(zeroed(&[0x5D0, 0x1D17B, 0x903, 0x1D17B]), "-z-z");
        // Marks before any base are not zeroed.
        assert_eq!(zeroed(&[0x1D17B, 0x1D17B, 0x5D0, 0x1D17B]), "---z");
        // A non-mark default-ignorable at the start is a base (position_cluster_impl).
        assert_eq!(zeroed(&[0x200B, 0x1D17B]), "-z");
        // A lone base and mark buffer of one character does nothing.
        assert_eq!(zeroed(&[0x1D17B]), "-");
    }

    #[test]
    fn planner_ports_harfbuzz_15() {
        let facts = |gpos, kern, cross_kern, morx, hebr| PlanFacts {
            gdef_glyph_classes: gpos,
            notdef_gdef_class: if gpos { 1 } else { 0 },
            gsub: gpos,
            gpos,
            morx,
            kerx: false,
            kern,
            machine_kern: false,
            cross_kern,
            gpos_hebrew_script: hebr,
        };
        let lucida = facts(false, false, false, true, false);
        // morx: non-default shapers become dumber (no GDEF zeroing) but keep
        // their fallback_position.
        assert_eq!(
            plan(&lucida, Shaper::Default, false),
            MarkPlan {
                zero_by_gdef: true,
                fallback_mark_positioning: true
            }
        );
        for shaper in [Shaper::Arabic, Shaper::Hebrew, Shaper::Hangul] {
            assert_eq!(
                plan(&lucida, shaper, false),
                MarkPlan {
                    zero_by_gdef: false,
                    fallback_mark_positioning: true
                }
            );
        }
        for shaper in [Shaper::Thai, Shaper::Indic, Shaper::Use, Shaper::Myanmar] {
            assert_eq!(
                plan(&lucida, shaper, false),
                MarkPlan {
                    zero_by_gdef: false,
                    fallback_mark_positioning: false
                }
            );
        }
        // Legacy kern applies only to fallback-positioning shapers: a
        // cross-stream kern then blocks fallback marks.
        let cross = facts(false, true, true, true, false);
        assert!(!plan(&cross, Shaper::Default, false).fallback_mark_positioning);
        assert!(!plan(&cross, Shaper::Hebrew, false).fallback_mark_positioning);
        // GPOS: only the hebrew gpos_tag can disable it.
        let verdana = facts(true, true, false, false, false);
        assert!(plan(&verdana, Shaper::Hebrew, false).fallback_mark_positioning);
        assert!(!plan(&verdana, Shaper::Arabic, false).fallback_mark_positioning);
        let arial = facts(true, true, false, false, true);
        assert!(!plan(&arial, Shaper::Hebrew, false).fallback_mark_positioning);
    }

    #[test]
    fn script_runs_follow_jdk_script_run() {
        let runs = |text: &str| {
            let codes: Vec<u32> = text.chars().map(|c| c as u32).collect();
            script_run_codes(&codes)
                .into_iter()
                .map(|script| match Shaper::for_script(script) {
                    Shaper::Default => 'd',
                    Shaper::Hebrew | Shaper::Arabic => 'f',
                    _ => 'o',
                })
                .collect::<String>()
        };
        // Weak characters (unassigned in Unicode 3.2) join the Latin run.
        assert_eq!(runs("Vault \u{1D17B}"), "ddddddd");
        // The mark after Hebrew joins its run.
        assert_eq!(runs("a \u{05D0}\u{1D17B}"), "ddff");
        // A leading mark takes the first strong script after it.
        assert_eq!(runs("\u{1D17B}\u{05D0}"), "ff");
        assert_eq!(runs("\u{0E01}\u{1D17B}"), "oo");
        // A closing bracket takes its opener's run script.
        assert_eq!(runs("\u{05D0}(\u{1D17B})"), "ffff");
        // When that script ends the current run, the opener is already popped:
        // the bracket starts a weak (COMMON, default-shaped) run.
        assert_eq!(runs("\u{05D0}(a)\u{1D17B}"), "ffddd");
    }

    /// Faces whose plan can zero a glyph but whose font file the reference
    /// host's record does not hash yet (relay request in
    /// maker-round7-claude). The gate fails if this list is stale.
    const PENDING_REFERENCE_HASH: &[&str] = &["Verdana-Bold"];

    /// Drift gate (T14.6.8): the generated table is the stored extractions.
    /// Each SHA256SUMS hashes to the recorded value and every stored file
    /// matches it; each face's runs, `.notdef` advance, own-cmap runs and
    /// every HarfBuzz plan fact (the kern cross-stream bit included) equal
    /// the stored TSVs; the unextracted GPOS `kern` feature cannot change any
    /// plan; and every face whose plan can zero a glyph has the same font
    /// file on the reference host (Monkey Man record) as where its own cmap
    /// was read.
    #[test]
    fn coverage_table_is_the_stored_reference_host_extraction() {
        use sha2::{Digest, Sha256};
        let hex = |bytes: &[u8]| {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let verify = |relative: &str, expected: &str| {
            let dir = root.join(relative);
            let sums = std::fs::read(dir.join("SHA256SUMS")).expect("stored SHA256SUMS");
            assert_eq!(hex(&sums), expected, "{relative}");
            for line in String::from_utf8(sums).unwrap().lines() {
                let (hash, name) = line.split_once("  ").expect("shasum line");
                let data = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(hex(&data), hash, "{name}");
            }
            dir
        };
        let dir = verify(coverage::SOURCE_DIR, coverage::SOURCE_SHA256SUMS_SHA256);
        let primary = verify(coverage::PRIMARY_DIR, coverage::PRIMARY_SHA256SUMS_SHA256);
        let font_hashes = |name: &str| -> std::collections::HashMap<String, String> {
            std::fs::read_to_string(primary.join(name))
                .unwrap()
                .lines()
                .filter_map(|line| {
                    let (hash, path) = line.split_once("  ")?;
                    (hash.len() == 64 && path.starts_with("/System/Library/Fonts/"))
                        .then(|| (path.to_string(), hash.to_string()))
                })
                .collect()
        };
        let reference_host = font_hashes("monkeyman-hb-static.txt");
        let colossus = font_hashes("colossus-font-files.sha256");
        let faces = [
            (ChainFace::LucidaGrande, "LucidaGrande"),
            (ChainFace::LucidaGrandeBold, "LucidaGrande-Bold"),
            (ChainFace::Arial, "ArialMT"),
            (ChainFace::ArialBold, "Arial-BoldMT"),
            (ChainFace::Helvetica, "Helvetica"),
            (ChainFace::HelveticaBold, "Helvetica-Bold"),
            (ChainFace::Verdana, "Verdana"),
            (ChainFace::VerdanaBold, "Verdana-Bold"),
            (ChainFace::Menlo, "Menlo-Regular"),
            (ChainFace::CourierNew, "CourierNewPSMT"),
        ];
        let mut pending = Vec::new();
        for (face, name) in faces {
            let mut facts = std::collections::HashMap::new();
            let (mut simple, mut complex, mut own) = (Vec::new(), Vec::new(), Vec::new());
            let mut font_path = String::new();
            for file in [format!("{name}.tsv"), format!("{name}.own.tsv")] {
                let base = if file.ends_with(".own.tsv") {
                    &primary
                } else {
                    &dir
                };
                let text = std::fs::read_to_string(base.join(&file)).unwrap();
                for line in text.lines() {
                    let fields: Vec<&str> = line.split('\t').collect();
                    match fields[0] {
                        "#notdef_advance_units" => assert_eq!(
                            fields[1].parse::<f64>().unwrap(),
                            f64::from(face.coverage().notdef_units),
                            "{name}"
                        ),
                        "#hb_plan" | "#plan_extra" => {
                            for fact in &fields[1..] {
                                let (key, value) = fact.split_once('=').unwrap();
                                facts.insert(key.to_string(), value.to_string());
                            }
                        }
                        "#face" => font_path = fields[3].to_string(),
                        "S" | "C" | "P" => {
                            let run = (
                                u32::from_str_radix(fields[1], 16).unwrap(),
                                u32::from_str_radix(fields[2], 16).unwrap(),
                            );
                            match fields[0] {
                                "S" => &mut simple,
                                "C" => &mut complex,
                                _ => &mut own,
                            }
                            .push(run);
                        }
                        _ => {}
                    }
                }
            }
            simple.sort_unstable();
            complex.sort_unstable();
            own.sort_unstable();
            assert_eq!(simple, face.coverage().simple_missing, "{name} simple");
            assert_eq!(complex, face.coverage().complex_mapped, "{name} complex");
            assert_eq!(own, face.coverage().primary_mapped, "{name} own cmap");
            let flag = |key: &str| facts[key] == "1";
            let stored = &face.coverage().plan;
            assert_eq!(
                (
                    stored.gdef_glyph_classes,
                    stored.notdef_gdef_class,
                    stored.gsub,
                    stored.gpos,
                    stored.morx,
                    stored.kerx,
                    stored.kern,
                    stored.machine_kern,
                    stored.cross_kern,
                    stored.gpos_hebrew_script,
                ),
                (
                    flag("gdef_glyph_classes"),
                    facts["notdef_gdef_class"].parse::<u8>().unwrap(),
                    flag("gsub"),
                    flag("gpos"),
                    flag("morx"),
                    flag("kerx"),
                    flag("kern"),
                    flag("machine_kern"),
                    flag("cross_kern"),
                    facts["gpos_scripts"].split(',').any(|tag| tag == "hebr"),
                ),
                "{name} plan facts"
            );
            assert!(!(flag("trak") && flag("STAT")), "{name}: trak applies");
            let glyph0_can_be_mark =
                !stored.gdef_glyph_classes || stored.notdef_gdef_class == GDEF_MARK_CLASS;
            let zeroes = |shaper: Shaper| {
                let plan = plan(stored, shaper, false);
                (plan.zero_by_gdef && glyph0_can_be_mark) || plan.fallback_mark_positioning
            };
            for shaper in Shaper::ALL {
                assert_eq!(
                    plan(stored, shaper, false),
                    plan(stored, shaper, true),
                    "{name} {shaper:?}: GPOS kern feature"
                );
                // With GSUB the categorized shaper depends on the chosen
                // script; only hebrew (chosen regardless) may zero there.
                if stored.gsub && shaper != Shaper::Hebrew {
                    assert!(!zeroes(shaper), "{name} {shaper:?}");
                }
            }
            if Shaper::ALL.into_iter().any(zeroes) {
                let here = &colossus[&font_path];
                match reference_host.get(&font_path) {
                    Some(there) => assert_eq!(here, there, "{name}: {font_path}"),
                    None => pending.push(name),
                }
            }
        }
        assert_eq!(
            pending, PENDING_REFERENCE_HASH,
            "faces awaiting reference hashes"
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
