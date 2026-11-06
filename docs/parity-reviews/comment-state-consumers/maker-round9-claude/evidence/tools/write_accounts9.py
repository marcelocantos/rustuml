"""Write the round-9 maker accounts (before any draft) into both mechanism directories, byte-identical."""
import json, pathlib, sys
R = pathlib.Path(sys.argv[1])
HDR = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
       "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
       "status": "maker causal account before draft; independent review required",
       "mechanism": "tim-builtin-value-boundaries",
       "answers": "docs/parity-reviews/tim-builtin-value-boundaries/review-9.json and comment-state-consumers/review-10.json (8089ef150) bounded_next_obligations",
       "base": "merge of parity-recovery-2026-11-03 dcf6d9076 (code identical to 9834a7194) into 8089ef150 (code identical to c3116f0b0): 9d6af0a0e"}
JDK = "OpenJDK jdk21u tag jdk-21.0.5-ga raw files (the reference JVM is Homebrew openjdk@21 21.0.5), sha256 prefixes: "
accounts = {}
accounts["per-face-facts-account.json"] = dict(HDR, **{
 "sub_mechanism": "obligation 1 (BLOCKING): HarfBuzz plan facts, GDEF blocklist decision, own cmap, .notdef advance and composite cascade come from the physical face Java measures for family x bold x italic",
 "sources": [JDK + "CFontManager.java 3ace8c4c, FontFamily.java 11323b8b, Font2D.java a2c16f1d, CFont.java 6ab8feb0, AWTFont.m 8f834825, hb-jdk-font.cc 8027d83a; HarfBuzz 15.0.0 hb-ot-layout.cc 3c4e62bc (pinned in tools/font-chain-reference/shaping-sources)"],
 "java_locations": [
  "CFontManager.loadFonts 209-235: loadNativeFonts() registers every face of every NSFontManager family (AWTFont.m GetFilteredFonts) through registerFont 168-172 -> new CFont(name, family); registerItalicDerived 174-203 then adds createItalicVariant() of the plain and bold faces only for families with no italic/bold-italic face.",
  "CFont(String, String) 160-166 -> Font2D.setStyle 151-175: the style slot comes from the face's full name ('bolditalic'/'boldoblique'/... -> BOLD|ITALIC, 'italic'/'oblique' -> ITALIC, 'bold' -> BOLD); FontFamily.setFont 234-289 keeps per slot the face of closest weight; FontFamily.getFont(style) 320-360 returns the slot's face.",
  "CFont.createItalicVariant 177-185: the derived italic keeps nativeFontName = the plain/bold face's full name and sets style |= ITALIC (isFakeItalic).",
  "CFont.getNativeFontPtr 187-191 -> createNativeFont(nativeFontName, style) -> AWTFont.m Java_sun_font_CFont_createNativeFont 429-448 -> awtFontForName 71-111: [NSFont fontWithName:name], then convertFont:toHaveTrait:NSItalicFontMask if ITALIC, then NSBoldFontMask if BOLD.",
  "CFont.getTableBytes 81-85 -> AWTFont.m getTableBytesNative 368-410: CTFontCopyTable of that native font (no options); hb-jdk-font.cc reference_table hands those blobs to HarfBuzz, so OT::GDEF::is_blocklisted (hb-ot-layout.cc 152-266) keys on the measured face's own (GDEF, GSUB, GPOS) lengths, and GDEF classes, GSUB/GPOS script lists come from the same face.",
  "CFont.createCompositeFont 203-239 with getCascadeList(nativeFontPtr) (AWTFont.m 547-580): the composite slots, slot 0's own cmap and .notdef are those of the same native font."
 ],
 "rust_locations": [
  "font_chain.rs ChainFace has ten faces; plantuml_metrics::courier_new_text_width measures plain, bold, italic and bold-italic Courier New with ChainFace::CourierNew (the plain face's facts 588/5078/14418, blocklisted). mono_text_width measures every Menlo style with Menlo-Regular. text_render::family_text_width picks Arial/Helvetica/Verdana faces by bold only.",
  "On colossus (macOS 27.2 26B123, same build as Monkey Man) Courier New Italic is (162, 914, 8502) and Bold Italic (114, 928, 8322): not blocklisted, GDEF classes present, glyph 0 class 1; GSUB/GPOS hebr,latn. Own cmaps and the cascade (complex) coverage differ by style in every family (100-207 code points on colossus)."
 ],
 "planned_change": [
  "tools/PrimaryFaceExtract.swift takes <nativeFontName> <java style> and resolves the face as awtFontForName does (NSFont fontWithName, NSFontManager italic then bold trait), printing the resolved PostScript name and path, the #hb_plan facts and the .notdef advance with the existing own cmap and #plan_extra. Run on colossus for the 24 (family, style) slots of the six modelled families (Lucida Grande italic slots are its derived variants).",
  "tools/FontChainExtract.swift (unchanged) on colossus for the ten italic/bold-italic/oblique faces Monkey Man has no extraction of; stored as a provisional, separately SHA-gated directory; a Monkey Man extraction of every style (both tools, plus font-file hashes) is requested in this round and replaces it.",
  "tools/font_chain_coverage.py: faces may come from the reference-host directory or the provisional one; generated FaceCoverage for 22 physical faces (Courier New Bold and Menlo Bold from monkeyman-round6, already extracted there).",
  "font_chain.rs: ChainFace is the physical face; ChainFace::for_style(family, bold, italic) follows FontFamily.getFont (Lucida Grande italic -> its plain/bold face). Courier New and Monospaced widths take bold/italic; Arial/Helvetica/Verdana chain faces take italic. Advance tables are not changed (Courier New and Menlo cells are style-independent; Arial/Verdana/Helvetica italic advance tables are out of scope).",
  "Drift gate: every face's stored facts (hb_plan, plan_extra, own cmap, chain runs) equal the generated table; the blocklist decision is asserted per face (Courier New plain and bold only); Courier New Italic/Bold Italic not blocklisted with glyph-0 class 1; each (family, style) slot resolves to the expected PostScript name; derived italic slots equal their upright faces; reference-host file hashes for every face whose plan can zero; the provisional-chain faces are an explicit list that fails when stale."
 ],
 "precommitted_measurement": {
  "oracle": "BO (C8A) Java references; BN, BM, BL, BK, BJ and earlier batches; all goldens; fresh relayed italic probes",
  "expect": [
   "BO c8a_ci_abstract_latin, _hebrew, _leading, _size11, c8a_ci_creole_italic, c8a_ci_creole_bolditalic, c8a_ci_usecase_italic: whole-output MATCH (the mark keeps its Courier New .notdef cell, as at merge).",
   "BO c8a_ci_stereo_latin stereotype width 64.8105 (Java; control + 7.2011) and c8a_lt_cnew_thai_stereo keeps the mark (+7.2011 over tip); both still fail on their control residues.",
   "Courier New regular and bold zeroing unchanged (BN/BO c8a_cr_*, c8a_bl_cnew_khmer_split, BM c7b_cnew_* gains kept).",
   "No golden changes (no golden carries an unmapped mark in italic Courier New, Menlo, Arial, Verdana or Helvetica text); no other batch case changes except italic/bold-italic cases in those families, each traced to a per-face fact.",
   "Fresh relayed probes: italic and bold-italic Courier New marks in sequence, activity and note text match Java."
  ]
 }})
accounts["text-line-runs-account.json"] = dict(HDR, **{
 "sub_mechanism": "obligation 2 (BLOCKING): text_line_levels returns BidiUtils.getLevels, i.e. the java.text.Bidi run levels",
 "sources": [JDK + "TextLine.java 4568ae07, BidiUtils.java 1ebac3a8, java/text/Bidi.java e80d3188, BidiBase.java df973a5f, BidiLine.java 15910df5"],
 "java_locations": [
  "TextLine.fastCreateTextLine 933-948: levels = BidiUtils.getLevels(bidi) when !bidi.isLeftToRight().",
  "BidiUtils.getLevels 54-82: fills each run [.., getRunLimit(i)) with getRunLevel(i), i in logical run order.",
  "BidiBase.getRunLevel 4443-4456 / getRunLimit 4502-: BidiLine.getRuns(this) first.",
  "BidiLine.getRuns 475-575: direction != MIXED -> getSingleRun(bidiBase, bidiBase.paraLevel) (one run over the whole text at the FIRST paragraph's level: 'bidiBase.paraLevel is ok even for contextual multiple paragraphs'); MIXED -> runs of equal levels[] up to trailingWSStart, each run's level = levels[run start] (559-566), and a trailing WS run at paraLevel.",
  "BidiBase.setPara 3593: trailingWSStart = length for a paragraph object (java.text.Bidi never calls setLine), so a MIXED paragraph's runs are exactly its levels[]."
 ],
 "rust_locations": ["java_bidi.rs text_line_levels 176-182 returns Paragraph.levels, which for direction LTR/RTL are per-paragraph para_level_at(i) (getLevelAt semantics), not the single run."],
 "planned_change": ["java_bidi.rs: Paragraph keeps getLevelAt levels; text_line_levels returns vec![para_level; n] for a non-MIXED direction and levels for MIXED (BidiLine.getRuns + BidiUtils.getLevels)."],
 "precommitted_measurement": {
  "oracle": "the checker's Java differential bidi-java-levels.txt (e09425f4...) over bidi-input.txt (fb7cd937...): run field for DIRECTION_DEFAULT_LEFT_TO_RIGHT inputs where requiresBidi && !isLeftToRight",
  "expect": ["0 run-level differences (352 now); requiresBidi, isLeftToRight, base level and getLevelAt stay at 0 differences over 242,508 inputs.",
             "No golden or batch case changes (no carried label holds a paragraph separator inside complex text) except relayed U+2029/U+0085 probes.",
             "Relayed probe: a Hebrew label + U+2029 + unmapped U+1D167 in Verdana is one RTL component; the mark is in the slot-0 buffer with U+2029 (hebrew shaper, Verdana GPOS lacks 'hebr' -> fallback positioning) and is zeroed, where the per-paragraph split kept it."]}})
accounts["link-label-font-account.json"] = dict(HDR, **{
 "sub_mechanism": "obligation 3 (BLOCKING, round-7 loss): class relationship center labels are laid out in the font they are drawn in",
 "sources": ["PlantUML 71806a23 src/main/java/net/sourceforge/plantuml/svek/GraphvizImageBuilder.java, SvekEdge.java"],
 "java_locations": [
  "GraphvizImageBuilder 233-241: labelFont = getDefaultStyleDefinitionArrow(link.getStereotype()).getMergedStyle(link.getStyleBuilder()).getFontConfiguration(...) is passed to new SvekEdge(...).",
  "SvekEdge 279-319: labelOnly = link.getLabel().create0(font, ...) (plus the magic arrow), labelText = labelOnly or merged with the note; the same TextBlock is measured for dot (appendTable at 433, strategy.eat 1163/1187) and drawn (drawU 946)."
 ],
 "rust_locations": [
  "class.rs emission (render relationship, 11527-11551) draws center labels with the arrow style's fontName/fontSize; relationship_center_layout (12804-12855) and synthesize_ortho_edge_labels (12900-12925) measure them with text_render::measure_no_underline(label, 13, false), i.e. sans-serif 13, and label_height(label, 13); the arrow block's content height (11853-11860) also uses sans-serif 13.",
  "BO c8a_lt_verdana_deva_link: layout width is Lucida Grande's 31.8459 (its synthesized MARK zeroes U+1D167 since 9f33bc3b1; before, Lucida's .notdef kept the box near Verdana's width by chance: 79 vs Java 80) while the drawn Verdana text is 35.2737 = Java. A private probe measuring the layout width in Verdana gives Java's 80."
 ],
 "planned_change": ["class.rs: one relationship_label_font(diagram, rel) resolving the arrow style fontName/fontSize exactly as emission does; relationship_center_layout, synthesize_ortho_edge_labels and the arrow block height measure width and height with it (endpoint labels are out of scope)."],
 "precommitted_measurement": {
  "oracle": "BO Java reference; goldens; fresh relayed held-out variant",
  "expect": ["BO c8a_lt_verdana_deva_link MATCH (viewBox 0 0 80 195).",
             "Goldens: only class diagrams with center labels under a non-default arrow font family or size can change; predicted no lost match (Java measures with the drawn font); every changed golden listed with its arrow font.",
             "Relayed renamed held-out variant (other Indic script + U+1D167, other class/object link labels under Verdana/Arial/ArrowFontSize) matches."]}})
accounts["bidi-oracle-gate-account.json"] = dict(HDR, **{
 "sub_mechanism": "obligation 4 (gate): Java's own java.text.Bidi results as a committed test oracle for java_bidi",
 "sources": ["/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round8-bidi-probe/bidi-java-levels.txt (sha256 e09425f49fb9f02fe2647c8bcd554ca7f6108d33586f0ed5b04cd7d1494bbf26; header: OpenJDK 64-Bit Server VM 21.0.5, the reference JVM on Monkey Man) over the checker's bidi-input.txt (fb7cd937973956246a1b34bdb9784ee7cf36422d1b8dd398fe47e97c1254d2e2), BidiProbe.java 7e0632e7"],
 "planned_change": [
  "tools/font-chain-reference/java-bidi-oracle/: a deterministic subset of (input, Java output) lines, gzipped, with SHA256SUMS, README (provenance, the full-file hashes, selection rule) and the selection script; selection rule fixed before any mutation run: all 801 stack/overflow lines (part C), every multi-paragraph DEFAULT_LTR line, every line whose only RTL-triggering class is AN, every line ending in whitespace/segment separators, and every 8th remaining line.",
  "java_bidi.rs test: for every stored line, requires_bidi, is_left_to_right, base level, getLevelAt levels and the run levels (text_line_levels for DEFAULT_LTR; the run model for 0/1) equal Java's; the stored SHA256SUMS hash is pinned."
 ],
 "precommitted_measurement": {"oracle": "mutation drill on a private copy",
  "expect": ["M12 (requires_bidi mask without AN), M14 (adjust_ws_levels no-op), M16 (MAX_EXPLICIT_LEVEL 61) each fail the new test; the unmutated tree passes; the earlier 19 drill mutations stay caught."]}})
for d in ["tim-builtin-value-boundaries", "comment-state-consumers"]:
    out = R / "docs/parity-reviews" / d / "maker-round9-claude"; out.mkdir(parents=True, exist_ok=True)
    for name, body in accounts.items():
        (out / name).write_text(json.dumps(body, indent=1, ensure_ascii=False) + "\n")
print("wrote", len(accounts))
