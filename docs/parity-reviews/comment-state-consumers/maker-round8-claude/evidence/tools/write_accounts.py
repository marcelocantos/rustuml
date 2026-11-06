"""Write the round-8 maker accounts (identical copies in both mechanism directories).
Usage: python3 -I write_accounts.py <repo>"""
import json, pathlib, sys

repo = pathlib.Path(sys.argv[1])
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"
JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
JDK = ("OpenJDK jdk21u tag jdk-21.0.5-ga (https://github.com/openjdk/jdk21u; the reference JVM is Homebrew "
       "openjdk@21 21.0.5), raw files, sha256: TextLine.java 4568ae07, TextLayout.java 3a1f3346, Font.java 06e4691d, "
       "java/text/Bidi.java e80d3188, jdk/internal/icu/text/BidiBase.java df973a5f, BidiLine.java 15910df5, "
       "sun/font/BidiUtils.java 1ebac3a8, TextLabelFactory.java f22871a7, ExtendedTextSourceLabel.java 82e68fbd, "
       "GlyphLayout.java cb95ae86, ScriptRun.java ae8c4d2f, HBShaper.c 2fbbde51, hb-jdk-font.cc 8027d83a, "
       "AWTFont.m 8f834825, CFont.java 6ab8feb0, UBiDiProps.java 61ba59a6, Trie2.java 1dc311fe, Trie2_16.java a5a4d9bf, "
       "scriptMapping.c 0a329fea, icudt72b/ubidi.icu 3d394ff3 (data version 15.0.0.0)")
HB = ("HarfBuzz tag 15.0.0 (https://github.com/harfbuzz/harfbuzz, commit 35f656a1a2bab548ea7f54a6dd3fba2b09680d14; the "
      "library the reference JVM loads, /opt/homebrew/Cellar/harfbuzz/15.0.0_1, T14.6.8), raw files, sha256: "
      "hb-ot-layout.cc 3c4e62bc, GDEF.hh ef1a744e, hb-ot-shaper.hh 3eb84b0a, hb-ot-tag.cc 6bcd8c41, hb-ot-shape.cc c50cabe8")
COMMON = {"schema_version": 1, "java_revision": JAVA, "jar_sha256": JAR,
          "status": "maker causal account before draft; independent review required",
          "mechanism": "tim-builtin-value-boundaries",
          "answers": "docs/parity-reviews/tim-builtin-value-boundaries/review-8.json and comment-state-consumers/review-9.json (98ddcfc9d) bounded_next_obligations"}

bidi = dict(COMMON, **{
 "sub_mechanism": "HarfBuzz buffers are bidi component x script run x font-slot run (round-8 obligation 1, BLOCKING)",
 "sources": [JDK],
 "java_locations": [
  "Font.getStringBounds(char[],int,int,FontRenderContext) 2629-2650: complex text (FontUtilities.isComplexText) is measured as new TextLayout(str, this, frc).getAdvance().",
  "TextLayout(String, Font, FontRenderContext) 371-399: a font without layout attributes passes attributes null; sameBaselineUpTo is the whole string, so fastInit (589-612) -> TextLine.fastCreateTextLine(frc, chars, font, cm, null).",
  "TextLine.fastCreateTextLine 897-980: with attributes null, requiresBidi = Bidi.requiresBidi(chars, 0, chars.length); if so, bidi = new Bidi(chars, 0, null, 0, len, Bidi.DIRECTION_DEFAULT_LEFT_TO_RIGHT) and, when !bidi.isLeftToRight(), levels = BidiUtils.getLevels(bidi); createComponentsOnRun(0, len, ...).",
  "TextLine.createComponentsOnRun 994-1039 and firstVisualChunk 1296-1305: one TextLineComponent per maximal run of equal levels (one for the whole string when levels is null).",
  "TextLabelFactory.createExtended 116-134: the component's flags carry LAYOUT_RIGHT_TO_LEFT for an odd level; ExtendedTextSourceLabel.createGV 325-335 lays out [start, start+length) with GlyphLayout.layout.",
  "GlyphLayout.layout 353-431: _scriptRuns.init(text, offset, count) and _fontRuns.init(font, text, offset, offset+count): ScriptRun (init 103-117 resets parenSP) and FontRunIterator run over the component only; one engine record (HarfBuzz buffer) per script run x slot run inside it.",
  "HBShaper.c 253-293: buffer script = getHBScriptCode(script) (scriptMapping.c), direction RTL iff TYPO_RTL, no buffer flags (no BOT, so no dotted circle), features -kern -liga.",
  "BidiBase.requiresBidi 4534-4561: true only if some code point is R, AL, RLE, RLO or AN (an isolate initiator alone does not require bidi).",
  "BidiBase(char[],int,byte[],int,int,int) 4278-4325: DIRECTION_DEFAULT_LEFT_TO_RIGHT -> paraLevel LEVEL_DEFAULT_LTR; setPara(char[]) 3526-3710 with REORDER_DEFAULT and no options: getDirProps 1476-1677 (P2-P3, FSI resolution, paragraphs at B, CR LF), resolveExplicitLevels 2090-2340 (X1-X8 with the overflow counters and the bracket pairing N0 of bracketProcessChar/Closing 1708-2020), the same-level-run loop of setPara with sor/eor, resolveImplicitLevels 3014-3165 (impTabProps 2470-2497, impTab_DEFAULT 2565-2613, processPropertySeq actions 1-4 2819-3012), adjustWSLevels 3167-3199 (L1; BN and explicit codes take the level of the following character).",
  "BidiBase.getCustomizedClass 3971-3978 (class >= 23 -> ON) over UBiDiProps.getClass (Trie2_16.get on icudt72b/ubidi.icu); UCharacter.getBidiPairedBracket/getIntPropertyValue(BIDI_PAIRED_BRACKET_TYPE) -> UBiDiProps.getPairedBracket/getPairedBracketType.",
  "BidiBase.isLeftToRight 4354-4368 and BidiLine.getRuns 475-590: for direction LTR/RTL all levels are paraLevel; for MIXED the runs are the maximal runs of levels[] (trailingWSStart = length), so BidiUtils.getLevels returns levels[]."
 ],
 "rust_locations": [
  "crates/rustuml-render/src/font_chain.rs ChainFace::complexity_per_token: script runs over the whole string (script_run_codes(&codes)), then slot runs; no bidi."
 ],
 "planned_change": [
  "tools/java_bidi_data.py: decode JDK 21.0.5's ubidi.icu (Trie2_16, mirrors[]) into BIDI_CLASSES and BRACKETS (crates/rustuml-render/src/java_bidi_data.rs); checked equal to Unicode 15.0.0 DerivedBidiClass.txt and BidiBrackets.txt for every code point; ubidi.icu stored under tools/font-chain-reference/jdk-21.0.5/ with SHA256SUMS.",
  "crates/rustuml-render/src/java_bidi.rs: port of BidiBase.setPara for REORDER_DEFAULT without options (getDirProps, resolveExplicitLevels with brackets, resolveImplicitLevels/processPropertySeq with impTab_DEFAULT, adjustWSLevels), requiresBidi and isLeftToRight; text_line_levels(units) -> Option<levels> as TextLine.fastCreateTextLine.",
  "font_chain.rs complexity_per_token: split complex text into components at level changes; ScriptRun and slot runs per component; the planner and fallback per buffer as before.",
  "Unit tests: the JDK paths above on the BK/BL/BM strings; conformance of java_bidi against a sample of Unicode 15.0.0 BidiCharacterTest.txt lines (stored with provenance), levels compared where the file gives one (not 'x')."
 ],
 "precommitted_measurement": {
  "oracle": "BK (L7A), BL (C7A), BM (C7B) Java references (HarfBuzz 15.0.0); BJ..AR; goldens; fresh M8A probes (relay)",
  "expect": [
   "Keep the mark (Verdana .notdef 1 em, text width = control + 1 em; whole-output back to the merge result, failing only by the control's residue): BK mk9_verdana_hebrew_space; BL c7m_heb_verdana_digit, c7m_heb_verdanabold_digit, c7m_heb_verdana_usecase; BM c7b_verdana_tail_space, c7b_verdana_digit_rtl.",
   "Keep zeroing (one RTL component, hebrew shaper): BM c7b_verdana_rtl_space, c7b_verdanabold_rtl_space.",
   "Zero (RLM splits; the EN+NSM component is COMMON: default shaper, Lucida Grande GDEF-synthesized MARK zeroed): BM c7b_thai_rlm_digit, c7b_deva_rlm_digit; their text width equals the control's.",
   "No other BK/BL/BM/BJ..AR case or golden changes result unless its measured string is complex and has a bidi level change; each such change is listed and traced.",
   "Fresh probes (staged for relay, predictions committed first): AL + EN (W2), RLE/LRE/RLO/LRO/PDF, RLI/LRI/FSI/PDI (requiresBidi ignores isolates), trailing whitespace (L1), brackets (N0)."
  ]
 }
})

gdef = dict(COMMON, **{
 "sub_mechanism": "HarfBuzz 15.0.0 GDEF blocklist and the GSUB chosen script in hb_ot_shaper_categorize (round-8 obligation 2, BLOCKING)",
 "sources": [HB, JDK],
 "java_locations": [
  "hb-ot-layout.cc 152-266 OT::GDEF::is_blocklisted (15.0.0): switch on HB_CODEPOINT_ENCODE3(GDEF blob length, GSUB table length, GPOS table length) over 45 entries; 251-254: (588, 5078, 14418) 'Courier New.ttf from macOS 15' and (588, 5078, 14238) 'Courier New Bold.ttf from macOS 15'.",
  "GDEF.hh 974-985: a blocklisted GDEF is replaced by the empty blob; hb-ot-layout.cc 299-302 hb_ot_layout_has_glyph_classes is then false; hb-ot-shape.cc 142-143 fallback_glyph_classes; 856-879 hb_synthesize_glyph_classes: MARK iff general category Mn and not default-ignorable.",
  "hb-jdk-font.cc 320-356 reference_table: tables come from Font2D.getTableBytes; for the slot-0 CFont, CFont.java 81-85 -> AWTFont.m 367-410 Java_sun_font_CFont_getTableBytesNative: CTFontCopyTable(ctfont, tag, kCTFontTableOptionNoOptions); the blob length is that CFData's length.",
  "hb-ot-shape.cc 83-101: shaper = hb_ot_shaper_categorize(script, direction, map.chosen_script[0]) (the GSUB chosen script); script_zero_marks and script_fallback_position from it; morx and a non-default shaper -> dumber.",
  "hb-ot-layout.cc 563-611 hb_ot_layout_table_select_script: the script's tags (hb-ot-tag.cc 157-181 hb_ot_all_tags_from_script: new tag|'3' (not mym2), new tag, old tag; 44-110 old/new tag tables), then 'DFLT', 'dflt', 'latn', else HB_TAG_NONE.",
  "hb-ot-shaper.hh 176-421 hb_ot_shaper_categorize: Arabic/Syriac -> arabic if (gsub_script != 'DFLT' || script == Arabic) and horizontal, else default; Thai/Lao thai; Hangul; Hebrew; nine Indic -> default if gsub_script is 'DFLT' or 'latn', USE if its last byte is '3', else indic; Khmer; Myanmar -> default for 'DFLT', 'latn', 'mymr', else myanmar; USE scripts -> default for 'DFLT'/'latn', else use; everything else default."
 ],
 "rust_locations": [
  "font_chain.rs PlanFacts.gdef_glyph_classes is the raw GDEF presence (true for Courier New); Shaper::for_script assumes no GSUB (chosen script NONE); the drift gate asserts that GSUB faces zero only under the hebrew shaper."
 ],
 "planned_change": [
  "tools/PrimaryFaceExtract.swift: also print the CTFontCopyTable lengths of GDEF, GSUB and GPOS per face (the JDK path); re-run on colossus for all faces plus CourierNewPS-BoldMT (stored output replaced, SHA256SUMS regenerated).",
  "tools/font_chain_coverage.py and tools/font_chain_unicode.py: generate GDEF_BLOCKLIST (all 45 triples parsed from the stored hb-ot-layout.cc 15.0.0), the per-face table lengths, gdef_blocklisted, and the GSUB ScriptList tags; SCRIPT_SHAPERS becomes the hb_ot_shaper_categorize case group of each ICU script, parsed from the stored hb-ot-shaper.hh with scriptMapping.c's ICU -> HB mapping and hb-common.h's script tags.",
  "font_chain.rs: glyph classes = GDEF has classes and is not blocklisted; Shaper::categorize(group, chosen GSUB script) ports hb_ot_shaper_categorize with hb_ot_layout_table_select_script over the face's GSUB tags.",
  "Addendum: the round-7 results' attribution of mk8_couriernew_* to 'Courier New and Monospaced family-name handling' is wrong; it is the GDEF blocklist."
 ],
 "precommitted_measurement": {
  "oracle": "BJ (K6A), BK, BL, BM Java references; goldens; fresh M8A probes",
  "expect": [
   "Zeroed (Courier New glyph 0 is a synthesized MARK for an unmapped Mn): BJ mk8_couriernew_9 and mk8_couriernew_23 (text widths equal Java's; Java 37.8062 for mk8_couriernew_9), BK mk9_couriernew_hebrew_space, BL c7m_heb_cnew_letter, c7m_heb_cnew_digit, BM c7b_cnew_latin_mark, c7b_cnew_leading_mark, c7b_cnew_thai_run, c7b_cnewbold_latin_mark. Whole-output MATCH where the control matches (c7b_cnew_latin_mark, c7b_cnewbold_latin_mark, mk9_couriernew_hebrew_space, c7m_heb_cnew_letter, c7m_heb_cnew_digit); c7b_cnew_thai_run keeps only its control's residue.",
   "Kept: BM c7b_cnew_mc (Mc is not a synthesized MARK), MATCH as before.",
   "No face other than Courier New (and its bold, measured as plain) is blocklisted; no non-Courier-New output changes from obligation 2.",
   "Fresh probes: Courier New marks in Devanagari, Myanmar, Khmer, Hangul, Arabic, Syriac and Thai runs (chosen GSUB script 'latn' or 'arab'), Mc/Me controls."
  ]
 }
})

drift = dict(COMMON, **{
 "sub_mechanism": "Drift gate for the shaping tables (round-8 obligation 3)",
 "sources": [HB, JDK, "Unicode 18.0.0 UnicodeData.txt sha256 0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950 (https://www.unicode.org/Public/18.0.0/ucd/)",
             "Monkey Man font-hash record /Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round7-mm-font-hash/output.txt sha256 a5591bab8d3dc44a1dc54b2e9ca7868538eda6733b43a87c7124bdb488c31152"],
 "planned_change": [
  "Store the pinned upstream files under tools/font-chain-reference/ with SHA256SUMS: UnicodeData-18.0.0.txt.gz (the gate checks the decompressed SHA256 against the official one), hb-ot-shaper.hh, hb-ot-layout.cc, hb-common.h (15.0.0), scriptMapping.c and ubidi.icu (jdk-21.0.5-ga), and the Monkey Man record (byte-identical copy).",
  "Gate (font_chain tests): re-derive NONSPACING_MARKS and UNICODE_MARKS from the stored UCD 18; re-derive SCRIPT_SHAPERS from the stored hb-ot-shaper.hh, scriptMapping.c and hb-common.h; re-derive GDEF_BLOCKLIST from the stored hb-ot-layout.cc; re-decode BIDI_CLASSES and BRACKETS from the stored ubidi.icu; assert each face's blocklist decision from its stored table lengths; every face whose plan can zero a mark has the same font file hash on Monkey Man (either record) as on colossus; PENDING_REFERENCE_HASH becomes empty (Verdana Bold 480f66b6... in the MM record equals colossus).",
  "Mutation drill: the checker's D14 (Unicode 18 run removed), D15 (hangul -> indic), a blocklist triple changed, a face's table length changed, a bidi class run changed, a pending entry re-added; each must fail the gate."
 ],
 "precommitted_measurement": {"expect": ["Every drill mutation fails coverage_table_is_the_stored_reference_host_extraction or the new pinned-table tests; the unmodified tree passes."]}
})

for d in ["tim-builtin-value-boundaries", "comment-state-consumers"]:
    out = repo / "docs/parity-reviews" / d / "maker-round8-claude"
    out.mkdir(parents=True, exist_ok=True)
    for name, obj in [("bidi-component-script-account.json", bidi), ("gdef-blocklist-account.json", gdef),
                      ("drift-gate-account.json", drift)]:
        (out / name).write_text(json.dumps(obj, indent=1, ensure_ascii=False) + "\n")
print("ok")
