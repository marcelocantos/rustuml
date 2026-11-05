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
    /// GSUB's ScriptList tags (`hb_ot_layout_table_select_script` chooses
    /// the shaper's `gsub_script` from them).
    pub(crate) gsub_scripts: &'static [[u8; 4]],
    /// GDEF, GSUB and GPOS table lengths as JDK hands them to HarfBuzz
    /// (`CTFontCopyTable`); 0 when absent.
    pub(crate) gdef_length: u32,
    pub(crate) gsub_length: u32,
    pub(crate) gpos_length: u32,
}

impl PlanFacts {
    /// `OT::GDEF::is_blocklisted` (hb-ot-layout.cc 152-266, 15.0.0): the
    /// (GDEF, GSUB, GPOS) lengths are a listed broken font's.
    pub(crate) fn gdef_blocklisted(&self) -> bool {
        unicode::GDEF_BLOCKLIST.contains(&(self.gdef_length, self.gsub_length, self.gpos_length))
    }

    /// `hb_ot_layout_has_glyph_classes` (hb-ot-layout.cc 299-302): GDEF has
    /// glyph classes and is not blocklisted (GDEF.hh 974-985 replaces a
    /// blocklisted GDEF by the empty blob); otherwise
    /// `hb_synthesize_glyph_classes`.
    pub(crate) fn has_glyph_classes(&self) -> bool {
        self.gdef_glyph_classes && !self.gdef_blocklisted()
    }

    /// `hb_ot_layout_table_select_script` (hb-ot-layout.cc 563-611) over
    /// GSUB for `script` (an ICU code): the first of the script's OT tags
    /// GSUB lists, else `DFLT`, `dflt`, `latn`, else none.
    fn gsub_chosen_script(&self, script: u8) -> Option<[u8; 4]> {
        if !self.gsub {
            return None;
        }
        let hb_script = unicode::HB_SCRIPTS
            .get(usize::from(script))
            .copied()
            .unwrap_or(*b"Zyyy");
        ot_script_tags(hb_script)
            .into_iter()
            .flatten()
            .chain([*b"DFLT", *b"dflt", *b"latn"])
            .find(|tag| self.gsub_scripts.contains(tag))
    }

    /// The shaper `hb_ot_shape_planner_t` categorizes for `script` (an ICU
    /// code) on this face (hb-ot-shape.cc 83-93).
    pub(crate) fn shaper(&self, script: u8) -> Shaper {
        let group = unicode::SCRIPT_SHAPERS
            .binary_search_by_key(&script, |(code, _)| *code)
            .map_or(ShaperGroup::Default, |index| {
                unicode::SCRIPT_SHAPERS[index].1
            });
        group.categorize(script, self.gsub_chosen_script(script))
    }
}

/// `hb_ot_all_tags_from_script` (hb-ot-tag.cc 157-181, 15.0.0): the new
/// Indic/Myanmar tags (`hb_ot_new_tag_from_script` 87-104; `mym2` has no
/// `'3'` form), then the old tag (`hb_ot_old_tag_from_script` 44-68).
fn ot_script_tags(script: [u8; 4]) -> [Option<[u8; 4]>; 3] {
    let new = match &script {
        b"Beng" => Some(*b"bng2"),
        b"Deva" => Some(*b"dev2"),
        b"Gujr" => Some(*b"gjr2"),
        b"Guru" => Some(*b"gur2"),
        b"Knda" => Some(*b"knd2"),
        b"Mlym" => Some(*b"mlm2"),
        b"Orya" => Some(*b"ory2"),
        b"Taml" => Some(*b"tml2"),
        b"Telu" => Some(*b"tel2"),
        b"Mymr" => Some(*b"mym2"),
        _ => None,
    };
    let three = new
        .filter(|tag| tag != b"mym2")
        .map(|[a, b, c, _]| [a, b, c, b'3']);
    let old = match &script {
        b"Hira" | b"Hrkt" => *b"kana",
        b"Laoo" => *b"lao ",
        b"Yiii" => *b"yi  ",
        b"Nkoo" => *b"nko ",
        b"Vaii" => *b"vai ",
        [first, rest @ ..] => [first.to_ascii_lowercase(), rest[0], rest[1], rest[2]],
    };
    [three, new, Some(old)]
}

/// The case groups of `hb_ot_shaper_categorize` (hb-ot-shaper.hh 176-421,
/// 15.0.0); the generated `SCRIPT_SHAPERS` puts each ICU script in one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShaperGroup {
    Default,
    Arabic,
    Thai,
    Hangul,
    Hebrew,
    Indic,
    Khmer,
    Myanmar,
    Use,
}

/// ICU ARABIC (`hb_ot_shaper_categorize` uses the arabic shaper for Arabic
/// even when GSUB has only `DFLT`).
const ARABIC_SCRIPT: u8 = 2;

impl ShaperGroup {
    /// The shaper of the group for the GSUB chosen script (horizontal text).
    fn categorize(self, script: u8, gsub_script: Option<[u8; 4]>) -> Shaper {
        let generic = matches!(gsub_script, Some(tag) if &tag == b"DFLT" || &tag == b"latn");
        match self {
            ShaperGroup::Default => Shaper::Default,
            ShaperGroup::Arabic => {
                if gsub_script != Some(*b"DFLT") || script == ARABIC_SCRIPT {
                    Shaper::Arabic
                } else {
                    Shaper::Default
                }
            }
            ShaperGroup::Thai => Shaper::Thai,
            ShaperGroup::Hangul => Shaper::Hangul,
            ShaperGroup::Hebrew => Shaper::Hebrew,
            ShaperGroup::Indic => {
                if generic {
                    Shaper::Default
                } else if matches!(gsub_script, Some([_, _, _, b'3'])) {
                    Shaper::Use
                } else {
                    Shaper::Indic
                }
            }
            ShaperGroup::Khmer => Shaper::Khmer,
            ShaperGroup::Myanmar => {
                if generic || gsub_script == Some(*b"mymr") {
                    Shaper::Default
                } else {
                    Shaper::Myanmar
                }
            }
            ShaperGroup::Use => {
                if generic {
                    Shaper::Default
                } else {
                    Shaper::Use
                }
            }
        }
    }
}

/// GDEF glyph class MARK.
const GDEF_MARK_CLASS: u8 = 3;

/// The modelled font families (`SansSerif` resolves to Lucida Grande and
/// `Monospaced` to Menlo on macOS, `CFontManager.loadFonts` 228-232).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChainFamily {
    LucidaGrande,
    Arial,
    Helvetica,
    Verdana,
    Menlo,
    CourierNew,
}

/// The physical faces JDK measures for the modelled families: one per
/// (family, bold, italic) slot (see [`ChainFace::for_style`]). Every fact of
/// the face (own cmap, cascade, `.notdef`, HarfBuzz plan) is its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChainFace {
    LucidaGrande,
    LucidaGrandeBold,
    Arial,
    ArialBold,
    ArialItalic,
    ArialBoldItalic,
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
    Verdana,
    VerdanaBold,
    VerdanaItalic,
    VerdanaBoldItalic,
    Menlo,
    MenloBold,
    MenloItalic,
    MenloBoldItalic,
    CourierNew,
    CourierNewBold,
    CourierNewItalic,
    CourierNewBoldItalic,
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
    #[cfg(test)]
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
    for (index, &code) in codes.iter().enumerate().skip(1) {
        if !mark(code) && !is_default_ignorable(code) {
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
    /// The face JDK measures for `family` in a style: the family's slot
    /// face (`FontFamily.getFont` 320-360; slots by full name,
    /// `Font2D.setStyle` 151-175), whose native font is
    /// `AWTFont.awtFontForName` (71-111). Lucida Grande has no italic face:
    /// `CFontManager.registerItalicDerived` (174-203) adds derived italics
    /// of its plain and bold faces, and `NSFontManager` finds no italic
    /// trait for them, so they measure as those faces.
    pub(crate) fn for_style(family: ChainFamily, bold: bool, italic: bool) -> ChainFace {
        use ChainFace::*;
        match (family, bold, italic) {
            (ChainFamily::LucidaGrande, false, _) => LucidaGrande,
            (ChainFamily::LucidaGrande, true, _) => LucidaGrandeBold,
            (ChainFamily::Arial, false, false) => Arial,
            (ChainFamily::Arial, true, false) => ArialBold,
            (ChainFamily::Arial, false, true) => ArialItalic,
            (ChainFamily::Arial, true, true) => ArialBoldItalic,
            (ChainFamily::Helvetica, false, false) => Helvetica,
            (ChainFamily::Helvetica, true, false) => HelveticaBold,
            (ChainFamily::Helvetica, false, true) => HelveticaOblique,
            (ChainFamily::Helvetica, true, true) => HelveticaBoldOblique,
            (ChainFamily::Verdana, false, false) => Verdana,
            (ChainFamily::Verdana, true, false) => VerdanaBold,
            (ChainFamily::Verdana, false, true) => VerdanaItalic,
            (ChainFamily::Verdana, true, true) => VerdanaBoldItalic,
            (ChainFamily::Menlo, false, false) => Menlo,
            (ChainFamily::Menlo, true, false) => MenloBold,
            (ChainFamily::Menlo, false, true) => MenloItalic,
            (ChainFamily::Menlo, true, true) => MenloBoldItalic,
            (ChainFamily::CourierNew, false, false) => CourierNew,
            (ChainFamily::CourierNew, true, false) => CourierNewBold,
            (ChainFamily::CourierNew, false, true) => CourierNewItalic,
            (ChainFamily::CourierNew, true, true) => CourierNewBoldItalic,
        }
    }

    fn coverage(self) -> &'static FaceCoverage {
        match self {
            Self::LucidaGrande => &coverage::LUCIDA_GRANDE,
            Self::LucidaGrandeBold => &coverage::LUCIDA_GRANDE_BOLD,
            Self::Arial => &coverage::ARIAL,
            Self::ArialBold => &coverage::ARIAL_BOLD,
            Self::ArialItalic => &coverage::ARIAL_ITALIC,
            Self::ArialBoldItalic => &coverage::ARIAL_BOLD_ITALIC,
            Self::Helvetica => &coverage::HELVETICA,
            Self::HelveticaBold => &coverage::HELVETICA_BOLD,
            Self::HelveticaOblique => &coverage::HELVETICA_OBLIQUE,
            Self::HelveticaBoldOblique => &coverage::HELVETICA_BOLD_OBLIQUE,
            Self::Verdana => &coverage::VERDANA,
            Self::VerdanaBold => &coverage::VERDANA_BOLD,
            Self::VerdanaItalic => &coverage::VERDANA_ITALIC,
            Self::VerdanaBoldItalic => &coverage::VERDANA_BOLD_ITALIC,
            Self::Menlo => &coverage::MENLO,
            Self::MenloBold => &coverage::MENLO_BOLD,
            Self::MenloItalic => &coverage::MENLO_ITALIC,
            Self::MenloBoldItalic => &coverage::MENLO_BOLD_ITALIC,
            Self::CourierNew => &coverage::COURIER_NEW,
            Self::CourierNewBold => &coverage::COURIER_NEW_BOLD,
            Self::CourierNewItalic => &coverage::COURIER_NEW_ITALIC,
            Self::CourierNewBoldItalic => &coverage::COURIER_NEW_BOLD_ITALIC,
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
    /// buffers: `TextLine` components (runs of equal bidi level,
    /// [`java_bidi::text_line_levels`]), then `ScriptRun` runs and slot runs
    /// inside each component (`GlyphLayout.layout` 408-431 runs both over
    /// the component only). In a slot-0 buffer the plan for the run's
    /// script decides which unmapped glyphs it zeroes: MARK glyphs by GDEF
    /// (glyph 0's class, or Mn and not default-ignorable when classes are
    /// synthesized, hb-ot-shape.cc 856-879), and Mn after a base by fallback
    /// positioning.
    pub(crate) fn complexity_per_token(self, text: &str) -> Vec<Complexity> {
        let tokens: Vec<rustuml_parser::java_utf16::Token> =
            rustuml_parser::java_utf16::tokens(text).collect();
        let codes: Vec<u32> = tokens
            .iter()
            .map(|token| match *token {
                rustuml_parser::java_utf16::Token::Unit(unit) => u32::from(unit),
                rustuml_parser::java_utf16::Token::Char(ch) => ch as u32,
            })
            .collect();
        if !is_complex_text(text) {
            return vec![Complexity::Simple; codes.len()];
        }
        let mut units = Vec::with_capacity(codes.len());
        let mut first_unit = Vec::with_capacity(codes.len());
        for token in &tokens {
            first_unit.push(units.len());
            match *token {
                rustuml_parser::java_utf16::Token::Unit(unit) => units.push(unit),
                rustuml_parser::java_utf16::Token::Char(ch) => {
                    units.extend_from_slice(ch.encode_utf16(&mut [0; 2]));
                }
            }
        }
        // Both units of a pair share a level (the lead is BN, adjustWSLevels).
        let levels = crate::java_bidi::text_line_levels(&units);
        let level = |token: usize| levels.as_ref().map_or(0, |l| l[first_unit[token]]);
        let coverage = self.coverage();
        let slot0: Vec<bool> = codes
            .iter()
            .map(|&code| {
                in_runs(coverage.primary_mapped, code) || !in_runs(coverage.complex_mapped, code)
            })
            .collect();
        let mut result = vec![Complexity::Complex { zeroed: false }; codes.len()];
        let mut component = 0;
        while component < codes.len() {
            let mut component_end = component + 1;
            while component_end < codes.len() && level(component_end) == level(component) {
                component_end += 1;
            }
            let scripts = script_run_codes(&codes[component..component_end]);
            let mut start = component;
            while start < component_end {
                let script = scripts[start - component];
                let mut end = start + 1;
                while end < component_end
                    && scripts[end - component] == script
                    && slot0[end] == slot0[start]
                {
                    end += 1;
                }
                if slot0[start] {
                    let plan = plan(&coverage.plan, coverage.plan.shaper(script), false);
                    let fallback = if plan.fallback_mark_positioning {
                        fallback_zeroed(&codes[start..end])
                    } else {
                        vec![false; end - start]
                    };
                    for index in start..end {
                        let code = codes[index];
                        let glyph_is_mark = if coverage.plan.has_glyph_classes() {
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
            component = component_end;
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

/// SHA256 of `tools/font-chain-reference/shaping-sources/SHA256SUMS`, the
/// pinned upstream files the drift gate re-derives the shaping tables from.
#[cfg(test)]
const SHAPING_SOURCES_SHA256SUMS_SHA256: &str =
    "b7e96abae07facf76b6f70a6b477d75e516fbea41cedff25ab60d882b54a3b81";

/// A pinned upstream file, checked against the stored SHA256SUMS, itself
/// checked against [`SHAPING_SOURCES_SHA256SUMS_SHA256`].
#[cfg(test)]
pub(crate) fn pinned_shaping_source(relative: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let hex = |bytes: &[u8]| -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/font-chain-reference/shaping-sources");
    let sums = std::fs::read(dir.join("SHA256SUMS")).unwrap();
    assert_eq!(
        hex(&sums),
        SHAPING_SOURCES_SHA256SUMS_SHA256,
        "shaping-sources SHA256SUMS"
    );
    let data = std::fs::read(dir.join(relative)).unwrap();
    let line = format!("{}  {relative}\n", hex(&data));
    assert!(
        String::from_utf8(sums).unwrap().contains(&line),
        "{relative}"
    );
    data
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
        for face in [ChainFace::Arial, ChainFace::Verdana] {
            assert_eq!(
                last_glyph(face, "Vault \u{1D17B}"),
                ChainGlyph::Missing,
                "{face:?}"
            );
        }
        // HarfBuzz 15.0.0 blocklists the macOS Courier New GDEF (588, 5078,
        // 14418): classes are synthesized, so an unmapped Mn is a MARK the
        // default shaper zeroes, leading or not; Mc and Me are not.
        let courier = ChainFace::CourierNew;
        assert!(courier.coverage().plan.gdef_blocklisted());
        assert_eq!(zeroing(courier, "Wharf x\u{1D167}"), "-------z");
        assert_eq!(zeroing(courier, "\u{1D167}Wharf"), "z-----");
        assert_eq!(zeroing(courier, "Wharf x\u{11F03}"), "--------");
        assert_eq!(zeroing(courier, "Wharf x\u{1ABE}\u{10FFFD}"), "---------");
        // The blocklist is decided per font file: Courier New Bold (588,
        // 5078, 14238) is listed, Courier New Italic (162, 914, 8502) and Bold
        // Italic (114, 928, 8322) are not, so their GDEF classes make glyph 0
        // a base and the mark keeps its cell (BO c8a_ci_*).
        assert_eq!(
            zeroing(ChainFace::CourierNewBold, "Wharf x\u{1D167}"),
            "-------z"
        );
        for face in [ChainFace::CourierNewItalic, ChainFace::CourierNewBoldItalic] {
            assert!(!face.coverage().plan.gdef_blocklisted(), "{face:?}");
            assert_eq!(zeroing(face, "Wharf x\u{1D167}"), "--------", "{face:?}");
            assert_eq!(zeroing(face, "\u{1D167}Wharf"), "------", "{face:?}");
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
        // GPOS faces: Arial lists 'hebr', so GPOS stays on and no fallback
        // runs; Verdana's GPOS lists only 'latn', so the hebrew shaper's
        // gpos_tag disables it and fallback positioning zeroes. (Courier New
        // also lists 'hebr', but its blocklisted GDEF zeroes the MARK.)
        assert_eq!(zeroing(Arial, "\u{05D0} \u{1D17B}"), "---");
        assert_eq!(zeroing(CourierNew, "\u{05D0} \u{1D17B}"), "--z");
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
            gsub_scripts: &[],
            gdef_length: 0,
            gsub_length: 0,
            gpos_length: 0,
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
                .map(|script| match coverage::LUCIDA_GRANDE.plan.shaper(script) {
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
    /// host's records do not hash yet. The gate fails if this list is stale.
    const PENDING_REFERENCE_HASH: &[&str] = &[];

    /// `tools/font-chain-reference/colossus-primary-faces`: the earlier
    /// Monkey Man hash records (`monkeyman-*.txt`) and the colossus font-file
    /// hashes, with the SHA256 of its SHA256SUMS.
    const HASH_RECORDS_DIR: &str = "tools/font-chain-reference/colossus-primary-faces";
    const HASH_RECORDS_SHA256SUMS_SHA256: &str =
        "b8895935d746cb127cd52c8f1b39e431031066a4352a8c9df66a954d12eabb08";

    /// The faces whose GDEF HarfBuzz 15.0.0 blocklists (their stored table
    /// lengths are listed in `GDEF_BLOCKLIST`).
    const GDEF_BLOCKLISTED: &[&str] = &["CourierNewPSMT", "CourierNewPS-BoldMT"];

    /// Every modelled (family, style) slot: the slot's native font name and
    /// Java style (0 plain, 1 bold, 2 italic, 3 bold italic), and the face
    /// [`ChainFace::for_style`] measures, with its PostScript name.
    const SLOTS: [(ChainFamily, &str, u8, ChainFace, &str); 24] = {
        use ChainFace::*;
        [
            (
                ChainFamily::LucidaGrande,
                "LucidaGrande",
                0,
                LucidaGrande,
                "LucidaGrande",
            ),
            (
                ChainFamily::LucidaGrande,
                "LucidaGrande-Bold",
                1,
                LucidaGrandeBold,
                "LucidaGrande-Bold",
            ),
            (
                ChainFamily::LucidaGrande,
                "LucidaGrande",
                2,
                LucidaGrande,
                "LucidaGrande",
            ),
            (
                ChainFamily::LucidaGrande,
                "LucidaGrande-Bold",
                3,
                LucidaGrandeBold,
                "LucidaGrande-Bold",
            ),
            (ChainFamily::Arial, "ArialMT", 0, Arial, "ArialMT"),
            (
                ChainFamily::Arial,
                "Arial-BoldMT",
                1,
                ArialBold,
                "Arial-BoldMT",
            ),
            (
                ChainFamily::Arial,
                "Arial-ItalicMT",
                2,
                ArialItalic,
                "Arial-ItalicMT",
            ),
            (
                ChainFamily::Arial,
                "Arial-BoldItalicMT",
                3,
                ArialBoldItalic,
                "Arial-BoldItalicMT",
            ),
            (
                ChainFamily::Helvetica,
                "Helvetica",
                0,
                Helvetica,
                "Helvetica",
            ),
            (
                ChainFamily::Helvetica,
                "Helvetica-Bold",
                1,
                HelveticaBold,
                "Helvetica-Bold",
            ),
            (
                ChainFamily::Helvetica,
                "Helvetica-Oblique",
                2,
                HelveticaOblique,
                "Helvetica-Oblique",
            ),
            (
                ChainFamily::Helvetica,
                "Helvetica-BoldOblique",
                3,
                HelveticaBoldOblique,
                "Helvetica-BoldOblique",
            ),
            (ChainFamily::Verdana, "Verdana", 0, Verdana, "Verdana"),
            (
                ChainFamily::Verdana,
                "Verdana-Bold",
                1,
                VerdanaBold,
                "Verdana-Bold",
            ),
            (
                ChainFamily::Verdana,
                "Verdana-Italic",
                2,
                VerdanaItalic,
                "Verdana-Italic",
            ),
            (
                ChainFamily::Verdana,
                "Verdana-BoldItalic",
                3,
                VerdanaBoldItalic,
                "Verdana-BoldItalic",
            ),
            (
                ChainFamily::Menlo,
                "Menlo-Regular",
                0,
                Menlo,
                "Menlo-Regular",
            ),
            (ChainFamily::Menlo, "Menlo-Bold", 1, MenloBold, "Menlo-Bold"),
            (
                ChainFamily::Menlo,
                "Menlo-Italic",
                2,
                MenloItalic,
                "Menlo-Italic",
            ),
            (
                ChainFamily::Menlo,
                "Menlo-BoldItalic",
                3,
                MenloBoldItalic,
                "Menlo-BoldItalic",
            ),
            (
                ChainFamily::CourierNew,
                "CourierNewPSMT",
                0,
                CourierNew,
                "CourierNewPSMT",
            ),
            (
                ChainFamily::CourierNew,
                "CourierNewPS-BoldMT",
                1,
                CourierNewBold,
                "CourierNewPS-BoldMT",
            ),
            (
                ChainFamily::CourierNew,
                "CourierNewPS-ItalicMT",
                2,
                CourierNewItalic,
                "CourierNewPS-ItalicMT",
            ),
            (
                ChainFamily::CourierNew,
                "CourierNewPS-BoldItalicMT",
                3,
                CourierNewBoldItalic,
                "CourierNewPS-BoldItalicMT",
            ),
        ]
    };

    /// The stored own-cmap extraction of a slot.
    fn own_file(native: &str, style: u8) -> String {
        let style = ["plain", "bold", "italic", "bolditalic"][usize::from(style)];
        format!("{native}.{style}.own.tsv")
    }

    /// Drift gate (T14.6.8): the generated table is the stored extractions.
    /// Each SHA256SUMS hashes to the recorded value and every stored file
    /// matches it; each face's runs, `.notdef` advance, own-cmap runs and
    /// every HarfBuzz plan fact (the kern cross-stream bit included) equal
    /// the stored TSVs, and the own-cmap extraction's planner facts equal the
    /// chain extraction's; every Java slot resolves to its face (derived
    /// italics to their upright face, with identical facts); the
    /// unextracted GPOS `kern` feature cannot change any plan; and every
    /// face whose plan can zero a glyph has its font file hashed by the
    /// reference host's records (Monkey Man), equal to colossus's.
    #[test]
    fn coverage_table_is_the_stored_reference_host_extraction() {
        use sha2::{Digest, Sha256};
        use std::collections::HashMap;
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
        let records = verify(HASH_RECORDS_DIR, HASH_RECORDS_SHA256SUMS_SHA256);
        let font_hashes = |path: std::path::PathBuf| -> HashMap<String, String> {
            std::fs::read_to_string(path)
                .unwrap()
                .lines()
                .filter_map(|line| {
                    let (hash, path) = line.split_once("  ")?;
                    (hash.len() == 64 && path.starts_with("/System/Library/Fonts/"))
                        .then(|| (path.to_string(), hash.to_string()))
                })
                .collect()
        };
        // Every Monkey Man record (the extraction's own font-hashes.txt and
        // the earlier relays'); where two hash a file they must agree.
        let mut reference_host = HashMap::new();
        for record in [
            primary.join("font-hashes.txt"),
            records.join("monkeyman-hb-static.txt"),
            records.join("monkeyman-round7-font-hash.txt"),
            records.join("monkeyman-round8-font-hash.txt"),
        ] {
            for (path, hash) in font_hashes(record) {
                if let Some(previous) = reference_host.insert(path.clone(), hash.clone()) {
                    assert_eq!(previous, hash, "{path}: Monkey Man records disagree");
                }
            }
        }
        let colossus = font_hashes(records.join("colossus-font-files.sha256"));
        // A stored TSV: header facts (key=value fields of #hb_plan and
        // #plan_extra), other header lines, and the S/C/P runs.
        struct Stored {
            facts: HashMap<String, String>,
            header: HashMap<String, Vec<String>>,
            runs: HashMap<&'static str, Vec<(u32, u32)>>,
        }
        let read = |path: std::path::PathBuf| {
            let mut stored = Stored {
                facts: HashMap::new(),
                header: HashMap::new(),
                runs: HashMap::new(),
            };
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                let fields: Vec<&str> = line.split('\t').collect();
                match fields[0] {
                    "#hb_plan" | "#plan_extra" => {
                        for fact in &fields[1..] {
                            let (key, value) = fact.split_once('=').unwrap();
                            stored.facts.insert(key.to_string(), value.to_string());
                        }
                    }
                    kind @ ("S" | "C" | "P") => stored
                        .runs
                        .entry(match kind {
                            "S" => "S",
                            "C" => "C",
                            _ => "P",
                        })
                        .or_default()
                        .push((
                            u32::from_str_radix(fields[1], 16).unwrap(),
                            u32::from_str_radix(fields[2], 16).unwrap(),
                        )),
                    _ => {}
                }
                if fields[0].starts_with('#') {
                    stored.header.insert(
                        fields[0].to_string(),
                        fields[1..].iter().map(|f| f.to_string()).collect(),
                    );
                }
            }
            for runs in stored.runs.values_mut() {
                runs.sort_unstable();
            }
            stored
        };
        let header_line =
            |stored: &Stored, key: &str| -> Vec<String> { stored.header[key].clone() };
        let mut pending = Vec::new();
        let mut faces_seen = Vec::new();
        for (family, native, style, face, ps) in SLOTS {
            assert_eq!(
                ChainFace::for_style(family, style & 1 != 0, style & 2 != 0),
                face,
                "{native} style {style}"
            );
            let own = read(primary.join(own_file(native, style)));
            // The slot's native font, through awtFontForName's traits, is
            // this face (a derived italic finds no italic trait).
            assert_eq!(
                header_line(&own, "#java_face"),
                [native.to_string(), style.to_string(), ps.to_string()],
                "{native} style {style}"
            );
            if faces_seen.contains(&face) {
                // A derived italic: identical facts to its upright face.
                let (_, upright_native, upright_style, _, _) =
                    SLOTS.iter().find(|slot| slot.3 == face).copied().unwrap();
                let upright = read(primary.join(own_file(upright_native, upright_style)));
                assert_eq!(own.facts, upright.facts, "{native} style {style}");
                assert_eq!(own.runs, upright.runs, "{native} style {style}");
                continue;
            }
            faces_seen.push(face);
            let chain = read(dir.join(format!("{ps}.tsv")));
            let coverage = face.coverage();
            assert_eq!(header_line(&chain, "#family")[1], ps, "{ps}");
            let notdef = |stored: &Stored| {
                stored.header["#notdef_advance_units"][0]
                    .parse::<f64>()
                    .unwrap()
            };
            assert_eq!(notdef(&chain), f64::from(coverage.notdef_units), "{ps}");
            assert_eq!(notdef(&own), f64::from(coverage.notdef_units), "{ps}");
            let empty = Vec::new();
            let runs =
                |stored: &Stored, kind: &str| stored.runs.get(kind).unwrap_or(&empty).clone();
            assert_eq!(runs(&chain, "S"), coverage.simple_missing, "{ps} simple");
            assert_eq!(runs(&chain, "C"), coverage.complex_mapped, "{ps} complex");
            assert_eq!(runs(&own, "P"), coverage.primary_mapped, "{ps} own cmap");
            // Both extractions read the planner facts of the same face.
            for key in [
                "gdef_glyph_classes",
                "notdef_gdef_class",
                "gsub",
                "gpos",
                "morx",
                "kerx",
                "kern",
                "kern_formats",
                "machine_kern",
            ] {
                assert_eq!(chain.facts[key], own.facts[key], "{ps} {key}");
            }
            let facts = &own.facts;
            let flag = |key: &str| facts[key] == "1";
            let length = |key: &str| facts[key].parse::<u32>().unwrap();
            let stored = &coverage.plan;
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
                    stored
                        .gsub_scripts
                        .iter()
                        .map(|tag| String::from_utf8(tag.to_vec()).unwrap())
                        .collect::<Vec<_>>(),
                    (stored.gdef_length, stored.gsub_length, stored.gpos_length),
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
                    facts["gsub_scripts"]
                        .split(',')
                        .filter(|tag| !tag.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                    (
                        length("gdef_length"),
                        length("gsub_length"),
                        length("gpos_length")
                    ),
                ),
                "{ps} plan facts"
            );
            assert!(!(flag("trak") && flag("STAT")), "{ps}: trak applies");
            // The blocklist decision, from this face's own table lengths.
            assert_eq!(
                stored.gdef_blocklisted(),
                GDEF_BLOCKLISTED.contains(&ps),
                "{ps}: GDEF blocklist decision"
            );
            let glyph0_can_be_mark =
                !stored.has_glyph_classes() || stored.notdef_gdef_class == GDEF_MARK_CLASS;
            for shaper in Shaper::ALL {
                assert_eq!(
                    plan(stored, shaper, false),
                    plan(stored, shaper, true),
                    "{ps} {shaper:?}: GPOS kern feature"
                );
            }
            // Every ICU script JDK maps (scriptMapping.c 0..45), categorized
            // with this face's GSUB chosen script.
            let zeroes = (0..unicode::HB_SCRIPTS.len() as u8).any(|script| {
                let plan = plan(stored, stored.shaper(script), false);
                (plan.zero_by_gdef && glyph0_can_be_mark) || plan.fallback_mark_positioning
            });
            if zeroes {
                let font_path = &header_line(&own, "#face")[2];
                let here = &colossus[font_path];
                match reference_host.get(font_path) {
                    Some(there) => assert_eq!(here, there, "{ps}: {font_path}"),
                    None => pending.push(ps),
                }
            }
        }
        assert_eq!(faces_seen.len(), 22);
        assert_eq!(
            pending, PENDING_REFERENCE_HASH,
            "faces awaiting reference hashes"
        );
        // plantuml_metrics counts monospaced cells: every Menlo and Courier
        // New face's .notdef is its cell (1233 and 1229 units).
        for (family, cell) in [(ChainFamily::Menlo, 1233), (ChainFamily::CourierNew, 1229)] {
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let face = ChainFace::for_style(family, bold, italic);
                assert_eq!(face.coverage().notdef_units, cell, "{face:?}");
            }
        }
        // Courier New Italic and Bold Italic (HarfBuzz 15.0.0 does not list
        // their lengths): GDEF classes kept, glyph 0 a base.
        for face in [ChainFace::CourierNewItalic, ChainFace::CourierNewBoldItalic] {
            let plan = &face.coverage().plan;
            assert!(
                plan.has_glyph_classes() && plan.notdef_gdef_class == 1,
                "{face:?}"
            );
        }
    }

    #[test]
    fn harfbuzz_buffers_split_at_bidi_level_changes() {
        use ChainFace::*;
        // The mark's zeroing in each string ('z' zeroed, '-' kept).
        let mark = |face: ChainFace, text: &str| {
            let at = text
                .chars()
                .position(|c| c == '\u{1D167}' || c == '\u{1D17B}');
            zeroing(face, text).chars().nth(at.unwrap()).unwrap()
        };
        // A neutral or digit and the mark after the last Hebrew letter are an
        // LTR component of COMMON (or Latin) script: default shaper, Verdana
        // GPOS on, mark kept (BK mk9_verdana_hebrew_space, BL
        // c7m_heb_verdana_digit/verdanabold_digit/verdana_usecase, BM
        // c7b_verdana_tail_space/digit_rtl).
        for (face, text) in [
            (Verdana, "Reef \u{05D0} \u{1D17B}z"),
            (Verdana, "Quay \u{05D0}1\u{1D167}"),
            (VerdanaBold, "Quay \u{05D0}2\u{1D167}"),
            (Verdana, "Quay \u{05D0}.\u{1D167}"),
            (Verdana, "Wharf \u{05D0} \u{1D167}"),
            (Verdana, "Wharf \u{05D0}7\u{1D167}\u{05D1}"),
        ] {
            assert_eq!(mark(face, text), '-', "{face:?} {text:?}");
        }
        // Inside one RTL component the hebrew shaper still zeroes (BM
        // c7b_verdana_rtl_space, c7b_verdanabold_rtl_space).
        for face in [Verdana, VerdanaBold] {
            assert_eq!(mark(face, "Wharf \u{05D0} \u{1D167}\u{05D1}"), 'z');
        }
        // RLM (R) splits off the EN + NSM component: COMMON, default shaper,
        // Lucida Grande's synthesized MARK zeroed (BM c7b_thai_rlm_digit,
        // c7b_deva_rlm_digit); without it the Thai/Devanagari run keeps it.
        for letter in ['\u{0E01}', '\u{0915}'] {
            assert_eq!(
                mark(LucidaGrande, &format!("Wharf {letter}\u{200F}7\u{1D167}")),
                'z'
            );
            assert_eq!(
                mark(LucidaGrande, &format!("Wharf {letter}7\u{1D167}")),
                '-'
            );
            // ALM is AL: W2 makes the digit AN, the same level 2 component.
            assert_eq!(
                mark(LucidaGrande, &format!("Wharf {letter}\u{061C}7\u{1D167}")),
                'z'
            );
            // An explicit RLE embedding: the digit and mark are level 2.
            assert_eq!(
                mark(
                    LucidaGrande,
                    &format!("Wharf {letter}\u{202B}7\u{1D167}\u{202C}")
                ),
                'z'
            );
            // An isolate initiator alone does not make requiresBidi true.
            assert_eq!(
                mark(
                    LucidaGrande,
                    &format!("Wharf {letter}\u{2067}7\u{1D167}\u{2069}")
                ),
                '-'
            );
        }
    }

    #[test]
    fn gsub_chosen_script_categorizes_the_shaper() {
        use ChainFace::*;
        let shaper = |face: ChainFace, script: u8| face.coverage().plan.shaper(script);
        // ICU DEVANAGARI 10, MYANMAR 28, TIBETAN 39, SYRIAC 34, ARABIC 2,
        // THAI 38, HANGUL 18, KHMER 23.
        // No GSUB: chosen script NONE.
        assert_eq!(shaper(LucidaGrande, 10), Shaper::Indic);
        assert_eq!(shaper(LucidaGrande, 28), Shaper::Myanmar);
        assert_eq!(shaper(LucidaGrande, 39), Shaper::Use);
        assert_eq!(shaper(LucidaGrande, 34), Shaper::Arabic);
        // Courier New's GSUB lists arab, hebr, latn: Indic, Myanmar and USE
        // scripts fall back to 'latn' and the default shaper; Syriac gets
        // 'latn' (not 'DFLT'), so arabic.
        assert_eq!(shaper(CourierNew, 10), Shaper::Default);
        assert_eq!(shaper(CourierNew, 28), Shaper::Default);
        assert_eq!(shaper(CourierNew, 39), Shaper::Default);
        assert_eq!(shaper(CourierNew, 34), Shaper::Arabic);
        assert_eq!(shaper(CourierNew, 2), Shaper::Arabic);
        for face in [CourierNew, Verdana, LucidaGrande] {
            assert_eq!(shaper(face, 38), Shaper::Thai);
            assert_eq!(shaper(face, 18), Shaper::Hangul);
            assert_eq!(shaper(face, 23), Shaper::Khmer);
        }
        // Courier New: a mark after Devanagari is zeroed by the default
        // shaper's GDEF-synthesized MARK; Lucida Grande's indic shaper becomes
        // dumber under morx and keeps it.
        assert_eq!(zeroing(CourierNew, "Wharf \u{0915}\u{1D167}"), "-------z");
        assert_eq!(
            zeroing(LucidaGrande, "Wharf \u{0915} \u{1D167}"),
            "---------"
        );
        assert_eq!(
            ot_script_tags(*b"Deva"),
            [Some(*b"dev3"), Some(*b"dev2"), Some(*b"deva")]
        );
        assert_eq!(
            ot_script_tags(*b"Mymr"),
            [None, Some(*b"mym2"), Some(*b"mymr")]
        );
        assert_eq!(ot_script_tags(*b"Laoo"), [None, None, Some(*b"lao ")]);
    }

    fn pinned(relative: &str) -> Vec<u8> {
        pinned_shaping_source(relative)
    }

    /// Drift gate: the Mn and Mn/Mc/Me tables are those of the official
    /// Unicode 18.0.0 UnicodeData.txt (stored gzipped; its decompressed
    /// SHA256 is the official file's).
    #[test]
    fn unicode_mark_tables_are_ucd_18() {
        use sha2::{Digest, Sha256};
        use std::io::Read;
        let mut text = String::new();
        flate2::read::GzDecoder::new(&pinned("unicode-18.0.0/UnicodeData.txt.gz")[..])
            .read_to_string(&mut text)
            .unwrap();
        let hash: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            hash,
            "0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950"
        );
        let runs = |categories: &[&str]| {
            let mut runs: Vec<(u32, u32)> = Vec::new();
            for line in text.lines() {
                let fields: Vec<&str> = line.split(';').collect();
                if !categories.contains(&fields[2]) {
                    continue;
                }
                let code = u32::from_str_radix(fields[0], 16).unwrap();
                match runs.last_mut() {
                    Some(run) if run.1 + 1 == code => run.1 = code,
                    _ => runs.push((code, code)),
                }
            }
            runs
        };
        assert_eq!(runs(&["Mn"]), unicode::NONSPACING_MARKS);
        assert_eq!(runs(&["Mn", "Mc", "Me"]), unicode::UNICODE_MARKS);
    }

    /// Drift gate: `HB_SCRIPTS`, `SCRIPT_SHAPERS` and `GDEF_BLOCKLIST` are
    /// re-derived from the stored HarfBuzz 15.0.0 sources.
    #[test]
    fn shaper_groups_and_gdef_blocklist_are_harfbuzz_15() {
        let text = |relative: &str| String::from_utf8(pinned(relative)).unwrap();
        // JDK scriptMapping.c ICU_to_Harfbuzz_ScriptCode, ICU codes 0..45.
        const HB_NAMES: [&str; 46] = [
            "COMMON",
            "INHERITED",
            "ARABIC",
            "ARMENIAN",
            "BENGALI",
            "BOPOMOFO",
            "CHEROKEE",
            "COPTIC",
            "CYRILLIC",
            "DESERET",
            "DEVANAGARI",
            "ETHIOPIC",
            "GEORGIAN",
            "GOTHIC",
            "GREEK",
            "GUJARATI",
            "GURMUKHI",
            "HAN",
            "HANGUL",
            "HEBREW",
            "HIRAGANA",
            "KANNADA",
            "KATAKANA",
            "KHMER",
            "LAO",
            "LATIN",
            "MALAYALAM",
            "MONGOLIAN",
            "MYANMAR",
            "OGHAM",
            "OLD_ITALIC",
            "ORIYA",
            "RUNIC",
            "SINHALA",
            "SYRIAC",
            "TAMIL",
            "TELUGU",
            "THAANA",
            "THAI",
            "TIBETAN",
            "CANADIAN_SYLLABICS",
            "YI",
            "TAGALOG",
            "HANUNOO",
            "BUHID",
            "TAGBANWA",
        ];
        let list = text("harfbuzz-15.0.0/hb-script-list.h");
        for (code, name) in HB_NAMES.iter().enumerate() {
            let key = format!("  HB_SCRIPT_{name}");
            let line = list
                .lines()
                .find(|line| {
                    line.starts_with(&key) && line[key.len()..].trim_start().starts_with('=')
                })
                .unwrap_or_else(|| panic!("{name}"));
            let tag: Vec<u8> = line
                .split("HB_TAG (")
                .nth(1)
                .unwrap()
                .split(')')
                .next()
                .unwrap()
                .split(',')
                .map(|c| c.trim().as_bytes()[1])
                .collect();
            assert_eq!(unicode::HB_SCRIPTS[code], tag[..], "{name}");
        }
        let shaper = text("harfbuzz-15.0.0/hb-ot-shaper.hh");
        let body = &shaper[shaper
            .find("hb_ot_shaper_categorize (hb_script_t script")
            .unwrap()..];
        let body = &body[..body.find("\n}\n").unwrap()];
        let mut group_of = std::collections::HashMap::new();
        let (mut labels, mut returns): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
        let mut close = |labels: &mut Vec<&str>, returns: &mut Vec<&str>| {
            let group = match returns.as_slice() {
                ["arabic", "default"] => Some(ShaperGroup::Arabic),
                ["thai"] => Some(ShaperGroup::Thai),
                ["hangul"] => Some(ShaperGroup::Hangul),
                ["hebrew"] => Some(ShaperGroup::Hebrew),
                ["default", "use", "indic"] => Some(ShaperGroup::Indic),
                ["khmer"] => Some(ShaperGroup::Khmer),
                ["default", "myanmar"] => Some(ShaperGroup::Myanmar),
                ["default", "use"] => Some(ShaperGroup::Use),
                ["default"] => Some(ShaperGroup::Default),
                ["myanmar_zawgyi"] => None,
                other => panic!("unknown case group {other:?}"),
            };
            for label in labels.drain(..) {
                group_of.insert(label.to_string(), group);
            }
            returns.clear();
        };
        for line in body.lines() {
            let trimmed = line.trim();
            let case = trimmed
                .strip_prefix("case HB_SCRIPT_")
                .and_then(|r| r.strip_suffix(':'));
            if (case.is_some() || trimmed == "default:") && !returns.is_empty() {
                close(&mut labels, &mut returns);
            }
            if let Some(name) = case {
                labels.push(name);
            }
            if let Some(rest) = trimmed.split("return &_hb_ot_shaper_").nth(1) {
                returns.push(rest.trim_end_matches(';'));
            }
        }
        close(&mut labels, &mut returns);
        let derived: Vec<(u8, ShaperGroup)> = HB_NAMES
            .iter()
            .enumerate()
            .filter_map(|(code, name)| match group_of.get(*name) {
                Some(Some(group)) if *group != ShaperGroup::Default => Some((code as u8, *group)),
                _ => None,
            })
            .collect();
        assert_eq!(derived, unicode::SCRIPT_SHAPERS);
        let layout = text("harfbuzz-15.0.0/hb-ot-layout.cc");
        let body = &layout[layout.find("OT::GDEF::is_blocklisted").unwrap()..];
        let body = &body[..body.find("\n}\n").unwrap()];
        let blocklist: Vec<(u32, u32, u32)> = body
            .split("case HB_CODEPOINT_ENCODE3 (")
            .skip(1)
            .map(|rest| {
                let numbers: Vec<u32> = rest
                    .split(')')
                    .next()
                    .unwrap()
                    .split(',')
                    .map(|n| n.trim().parse().unwrap())
                    .collect();
                (numbers[0], numbers[1], numbers[2])
            })
            .collect();
        assert_eq!(blocklist, unicode::GDEF_BLOCKLIST);
        assert!(blocklist.contains(&(588, 5078, 14418)) && blocklist.contains(&(588, 5078, 14238)));
    }

    #[test]
    fn complexity_follows_java_units() {
        assert!(!is_complex_text("Vault"));
        assert!(is_complex_text("Vault \u{10FFFD}"));
        assert!(is_complex_text("a\u{0301}"));
        assert!(!is_complex_text("\u{A500}"));
    }
}
