"""Write the round-7 maker accounts (before the drafts)."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1])
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"
JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
HEAD = dict(schema_version=1, java_revision=JAVA, jar_sha256=JAR,
            status="maker causal account before draft; independent review required",
            mechanism="tim-builtin-value-boundaries")
SRC_JDK = ("OpenJDK jdk21u tag jdk-21.0.5-ga (https://github.com/openjdk/jdk21u; the reference JVM is Homebrew "
           "openjdk@21 21.0.5): sha256 GlyphLayout.java cb95ae86, FontRunIterator.java 1cb09161, ScriptRun.java "
           "ae8c4d2f, CCompositeGlyphMapper.java 33a73e47, CoreTextSupport.m 9bb54645, hb-jdk-font.cc 8027d83a, "
           "HBShaper.c 2fbbde51, scriptMapping.c 0a329fea, ToStream.java 55794276, CharInfo.java 8743d68a")
SRC_HB = ("HarfBuzz tag 15.0.0 (https://github.com/harfbuzz/harfbuzz, commit 35f656a1a2bab548ea7f54a6dd3fba2b09680d14; "
          "the library the reference JVM loads: /opt/homebrew/Cellar/harfbuzz/15.0.0_1, T14.6.8): sha256 "
          "hb-ot-shape.cc c50cabe8, hb-ot-shape-fallback.cc ba8af8e7, hb-unicode.hh 8c15f338, hb-ot-layout.hh aaa549c0, "
          "hb-ot-shaper.hh 3eb84b0a, hb-ucd-table.hh 4bcf2ec3 ('Unicode 18.0.0'); "
          "UnicodeData.txt 18.0.0 sha256 0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950 "
          "(https://www.unicode.org/Public/18.0.0/ucd/)")
Y = "src/main/java/net/sourceforge/plantuml/"

accounts = {
"yaml-block-style-account.json": dict(HEAD,
  sub_mechanism="YAML '|' block values: a port of YamlParser.getBlockStyleString with cleanBlockStyle's per-line trim (round-7 obligation 1, BLOCKING)",
  answers="docs/parity-reviews/tim-builtin-value-boundaries/review-7.json (attributed_regressions y8_block_inner_pipe, _end, _nested; findings[0..2]; bounded_next_obligations[0])",
  java_locations=[
    Y+"yaml/parser/YamlLine.java 52-97 build: TAB -> four spaces (54); indent = leading spaces; removeYamlComment of the trimmed rest (63, 199-221: '#' first or ' #' outside quotes); '- ' list prefix adds 2 to the indent (73-76); key/value split at findColonSeparator (the first ':' outside quotes, 101-115); rawValue (trimmed) equal to '|' is KEY_AND_BLOCK_STYLE (87-88); unquote strips one pair of matching quotes from the key (180-191).",
    Y+"yaml/parser/YamlParser.java 84-85: a non-list KEY_AND_BLOCK_STYLE line takes getBlockStyleString(indent); a list-item one reaches 72-73 ('wip3', unchanged here).",
    Y+"yaml/parser/YamlParser.java 132-149 getBlockStyleString: following lines are the body until one whose YamlLine type is neither NO_KEY_ONLY_TEXT nor EMPTY_LINE and whose indent is <= the key's (139-141); every body line is appended as cleanBlockStyle(indent, line) followed by '\\n' (143-144), so text lines at any indent, deeper key lines, list lines and comment lines are all body.",
    Y+"yaml/parser/YamlParser.java 151-154 cleanBlockStyle: String.trim() of the raw line (characters <= U+0020 at both ends; TABs inside are kept).",
    Y+"jsondiagram/StyleExtractor.java 59-64: whitespace-only lines never reach YamlParser (ported in round 6)."],
  rust_locations=["crates/rustuml-parser/src/parse/json_diagram.rs keep_block_line_breaks (round 6) rewrites every line ending ': |' to '|+', block bodies included; serde_yml then applies YAML block semantics (relative indentation kept, body ends by YAML rules)."],
  cause="Rust delegates the block to serde_yml and patches its chomping by text rewriting; Java builds the value itself with a different body rule and a per-line trim.",
  planned_change=[
    "Remove keep_block_line_breaks.",
    "Before serde, classify each YAML line as YamlLine.build does (TAB replacement, indent, comment removal, list prefix, colon separator). A KEY_AND_BLOCK_STYLE line and its body (the getBlockStyleString rule) become one line: the same indentation and list prefix, the key as a single-quoted scalar (unquote's text, literal), and the value as a double-quoted scalar whose text is the concatenation of java_trim(line) + '\\n' over the body lines.",
    "keep_quoted_backslashes no longer guesses block bodies from ': |' / ': >' suffixes for '|' blocks; it uses the same classification (folded '>' is unchanged: Java throws 'wip4' there)."],
  precommitted_measurement=dict(
    oracle="BJ (K6A) Java references, plus the earlier batches and goldens",
    expect=["BJ y8_block_inner_pipe, y8_block_inner_pipe_end, y8_block_inner_pipe_nested and y8_block_relative_indent match whole-output.",
            "Block cases that match at the merge keep matching: BJ y8_block_spaced_pipe, y8_block_comment_pipe, y8_block_two_blocks_end, y8_block_dash_body, y8_block_nested_key, y8_block_whitespace_lines; BI yd7_block_mid_document; BH dc6_yaml_block.",
            "No lost match, lost rejection, new valid-input error or panic in any cohort.",
            "A fresh perturbation (staged for relay) with a body line ending ': |' at depth 3 and renamed labels."])),

"yaml-line-preprocessing-account.json": dict(HEAD,
  sub_mechanism="YAML lines before Display: YamlLine's TAB replacement and flow-sequence escapes; StyleExtractor's preamble skinparam in @startyaml/@startjson (round-7 obligation 4)",
  answers="docs/parity-reviews/tim-builtin-value-boundaries/review-7.json (findings[4]; held_out_failures y8_real_tab_value, y8_flow_seq_escapes, y8_tabsize4, j8_tabsize4; bounded_next_obligations[4])",
  java_locations=[
    Y+"yaml/parser/YamlLine.java 54: line.replaceAll(\"\\t\", \"    \") before any parsing, so a TAB in a key, value or indentation is four spaces (block bodies use the raw line: YamlParser 143).",
    Y+"yaml/parser/YamlLine.java 91-93: a rawValue that starts with '[' and ends with ']' is KEY_AND_FLOW_SEQUENCE with toList(inner); 117-170 toList: fields split at ',' outside quotes; a field that starts (after spaces) with a quote is kept as quoted text, others are trimmed; outside and inside quotes a backslash appends the next character literally; the last field is always added (so '[]' is one empty field).",
    Y+"yaml/parser/MonomorphToJson.java 44-80 and YamlBuilder.onKeyAndFlowSequence 129-131: every value is a JSON string.",
    Y+"jsondiagram/StyleExtractor.java 59-98: lines Java trims to empty are skipped; while at most the @start line is in the list (list.size() <= 1), lines starting 'skinparam ' are consumed (a '{' block up to a '}' line), only 'handwritten ... true' is kept as a flag; '!assume ', '!pragma ' and 'hide ' are ignored; 'scale ', 'title ' and 'skin ' set fields; '<style>' blocks are consumed anywhere.",
    Y+"yaml/YamlDiagramFactory.java 70-90 and jsondiagram/JsonDiagramFactory.java 69-91 read StyleExtractor's list; neither applies a skinparam other than handwritten, so 'skinparam tabSize 4' leaves AtomText's eight-space tab (BJ y8_tabsize4 / j8_tabsize4: 'b' and 'z' at text x + 35.4375 = 8 x 4.4297)."],
  planned_change=[
    "In the YAML preprocessing pass (shared with obligation 1): every non-block line has each TAB replaced by four spaces before serde sees it.",
    "A KEY_AND_FLOW_SEQUENCE line becomes the same key and a flow sequence of single-quoted scalars holding toList's fields (all strings, as MonomorphToJson).",
    "extract_body (JSON and YAML): before the first data line, a line whose Java trim starts with 'skinparam ' is consumed (with its '{' ... '}' block), as are '!assume ', '!pragma ' and 'hide ' lines. Not ported (residue, recorded): handwritten, title, scale, skin and <style> effects; those inputs keep their present behaviour."],
  precommitted_measurement=dict(
    oracle="BJ Java references",
    expect=["BJ y8_real_tab_value and y8_flow_seq_escapes match whole-output.",
            "BJ y8_tabsize4 and j8_tabsize4 render (exit 0) and match whole-output with the existing eight-space AtomText tab.",
            "perturbations/shared-svg-attributes/checker-independent-astra/family_json (skinparam defaultFontName in @startjson, Java ignores it) renders instead of exiting 1.",
            "No lost match, lost rejection, new valid-input error or panic."])),

"harfbuzz-buffer-mark-account.json": dict(HEAD,
  sub_mechanism="Unmapped nonspacing marks shaped per HarfBuzz buffer (script run x font-slot run) under HarfBuzz 15.0.0 (round-7 obligations 2, 3 and 7, BLOCKING)",
  answers="docs/parity-reviews/tim-builtin-value-boundaries/review-7.json (attributed_regressions mk8_lucida_arabic, mk8_seq_arabic, mk8_helv_hebrew, mk8_menlo_hebrew; findings[7..9]; held_out_failures mk8_u17_1ad0, mk8_u17_1e6e3, mk8_u18_05c8; bounded_next_obligations[1..3])",
  sources=[SRC_JDK, SRC_HB],
  java_locations=[
    "GlyphLayout.java 408-431: a composite font's text is split by ScriptRun, then each script run by FontRunIterator.next(script, limit); nextEngineRecord shapes each piece with its slot's physical font: one HarfBuzz buffer per script run x slot run.",
    "FontRunIterator.java 115-126: a run continues while charToGlyph(ch) & SLOTMASK equals the first character's slot.",
    "CCompositeGlyphMapper.java 54-67: the first slot whose mapper returns a code > 0, else missingGlyph (0, slot 0). Slot 0 is the CFont itself; CoreTextSupport.m 99-133 CTS_GetGlyphsAsIntsForCharacters gives > 0 only for the font's own glyph (CTFontGetGlyphsForCharacters); a cascade substitution is negative. So a character is in slot 0 when the primary face's own cmap maps it or when no slot maps it.",
    "hb-jdk-font.cc 199-207: hb_jdk_get_glyph_extents returns false.",
    "hb-ot-shape.cc 85-103 (15.0.0): the shaper is hb_ot_shaper_categorize(script); script_zero_marks and script_fallback_position are read from it before morx swaps any non-default shaper for _hb_ot_shaper_dumber (zero_width_marks NONE, fallback_position false).",
    "hb-ot-shape.cc 152-221 (15.0.0) compile: apply_kerx/apply_gpos as before; legacy kern now applies only when script_fallback_position (182: 'Not all shapers apply legacy kern'); apply_fallback_kern needs script_fallback_position (187); zero_marks = script_zero_marks && !apply_kerx && (!apply_kern || !machine kerning) (189-195); adjust_mark_positioning_when_zeroing = !apply_gpos && !apply_kerx && (!apply_kern || !cross kerning) (198-204); fallback_mark_positioning = adjust && script_fallback_position (206-207); trak applies when trak and STAT exist (218).",
    "Shapers (15.0.0): default BY_GDEF_LATE/fallback true; dumber NONE/false; arabic BY_GDEF_LATE/true (Arabic and, with no GSUB script, Syriac); hebrew BY_GDEF_LATE/true; hangul NONE/true (true since 12.2.0, commit 63e8c13e1 '[hangul] Enable fallback position / kern table'); thai BY_GDEF_LATE/false; indic, khmer NONE/false; myanmar, use BY_GDEF_EARLY/false.",
    "hb-ot-shape.cc 856-879 hb_synthesize_glyph_classes: without GDEF glyph classes, a glyph is MARK when its character is Mn and not default-ignorable; zero_mark_widths_by_gdef zeroes MARK glyphs when zero_marks and the plan's shaper zeroes by GDEF.",
    "hb-ot-shape-fallback.cc 470-497 _hb_ot_shape_fallback_mark_position (15.0.0): cluster boundaries at characters that are not Unicode marks (Mn, Mc, Me) and not hidden or default-ignorable; 426-467 position_cluster(_impl): in a cluster of two or more, the first non-mark is the base and its marks run up to the next non-mark, skipping hidden and default-ignorable glyphs; 324-342 position_around_base: extents fail (JDK), so zero_mark_advances (189-208) zeroes every Mn after the base in that span.",
    "hb-ot-layout.hh 216-260: hidden is set only on default-ignorables (Mongolian FVS, TAG characters, CGJ), so 'hidden or default-ignorable' is the default-ignorable set; hb-unicode.hh 167-197 is_default_ignorable (15.0.0) adds U+180F; hb-ot-shape.cc 1193-1194 positions before hb_ot_substitute_post hides default-ignorables, so they are in the buffer during fallback positioning.",
    "hb-ucd-table.hh (15.0.0): Unicode 18.0.0 general categories: 70 Mn more than 16.0.0 (39 from 17.0, 31 from 18.0)."],
  rust_locations=["crates/rustuml-render/src/font_chain.rs complexity_per_token: after_base walks the script run only; resolve zeroes by precomputed per-face booleans; NONSPACING_MARKS/UNICODE_MARKS from Unicode 16; U+180F not default-ignorable; hangul not fallback-positioned."],
  environment="The primary faces' own cmaps are not in the Monkey Man extraction. tools/PrimaryFaceExtract.swift reads them (and the kern cross-stream bit, trak and STAT) on colossus. The six font files the zeroing plan can reach (LucidaGrande.ttc, Helvetica.ttc, Menlo.ttc, Arial.ttf, Verdana.ttf, Courier New.ttf) hash identically on Monkey Man (checker monkeyman-hb static.txt) and colossus; the drift gate checks those hashes against a byte-identical copy of the Monkey Man record. Arial Bold and Verdana Bold are not hashed on Monkey Man; the gate asserts their plan never zeroes, so their own cmap cannot affect output.",
  planned_change=[
    "tools/PrimaryFaceExtract.swift; output stored at tools/font-chain-reference/colossus-primary-faces/ with SHA256SUMS, the colossus font-file hashes and the copied Monkey Man record.",
    "tools/font_chain_coverage.py: per face the own-cmap runs (primary_mapped) and the raw plan facts (GDEF glyph classes, class of glyph 0, GSUB, GPOS, morx, kerx, kern, machine kern, cross kern, trak, STAT) instead of two precomputed booleans.",
    "tools/font_chain_unicode.py: Mn and Mn/Mc/Me from UnicodeData 18.0.0; the per-script shaper choice (no generated complex/fallback lists).",
    "font_chain.rs: ChainFace::complexity_per_token(text) splits complex text into buffers (ScriptRun run x slot run, slot 0 = own glyph or unmapped everywhere), ports the 15.0.0 planner per buffer and _hb_ot_shape_fallback_mark_position over the buffer, and marks each unmapped Mn token zeroed or not; is_default_ignorable gains U+180F; provenance comments name 15.0.0.",
    "Drift gate (obligation 7): the generated table equals the stored primary-face extraction (runs and every plan fact, the Helvetica and Helvetica-Bold kern bit included); stored file hashes; the Monkey Man font hashes equal the colossus ones for every face whose plan can zero a mark; mutation drill including the Helvetica kern bit."],
  precommitted_measurement=dict(
    oracle="BJ (K6A) Java references (shaped with HarfBuzz 15.0.0); BI/BH where shaping-dependent; goldens",
    expect=["BJ mk8_lucida_arabic and mk8_seq_arabic match whole-output (mark keeps Lucida Grande .notdef 10.5: alef is in the Geeza Pro slot).",
            "BJ mk8_helv_hebrew: text width 52.4043 = the Rust control's 43.5313 + Helvetica .notdef 8.873 (Java 53.7031 - 44.8301); whole-output stays DIFF only by the pre-existing alef advance that mk8_helv_hebrew_ctrl also fails on.",
            "BJ mk8_menlo_hebrew: mark keeps its 8.4287 cell, width 67.4297, exact viewBox 119 x 68 (Java 119); NBSP/family-name/alef-advance residue stays (pre-existing, as mk8_mono_9/23).",
            "Same-buffer controls keep matching: BJ mk8_lucida_syriac, mk8_helv_hebrew_pres, mk8_cjk_base; BI mk7_after_hebrew unchanged.",
            "BJ mk8_u17_1ad0, mk8_u17_1e6e3, mk8_u18_05c8 match whole-output.",
            "Every other BJ/BI/BH/BG/BF/BE/BC/BB/AW/AU/AR case and every golden keeps its result.",
            "Fresh probes (staged for relay): Unicode 17/18 marks not in BJ; U+180F after a Thai base (default-ignorable now: zero); marks after slot-0 and other-slot bases in Hebrew, Arabic, Syriac, Thai and Devanagari runs in Lucida Grande and Helvetica."])),

"svg-text-cr-account.json": dict(HEAD,
  sub_mechanism="CR in SVG character data serialized as &#13; (round-7 obligation 5)",
  answers="docs/parity-reviews/tim-builtin-value-boundaries/review-7.json (findings[5]: j8_decoded_cr serialization; bounded_next_obligations[5])",
  sources=[SRC_JDK],
  java_locations=[
    "CharInfo.java 201/302: set(S_CARRIAGERETURN) after loading the XML entities; 326-334: a character set in the table is not text-clean ASCII.",
    "ToStream.java 1438-1520 outputCharacters: a non-clean character goes to processDirty; 1619-1650 processDirty: LF in a text node writes the line separator, any other dirty character goes to accumDefaultEscape; 1696-1776: CR is neither C0/C1-range (1589) nor needing an entity, but isSpecialTextChar(CR) holds in a text node (1758), so writeCharRef writes '&#13;' (1781-1788). Attribute values already follow writeAttrString (svg.rs escape_attribute)."],
  rust_locations=["crates/rustuml-render/src/svg_export.rs serialize: the shared ToStream boundary; it rewrites unpaired surrogates only. Family escapers (json_diagram.rs escape_text and others) leave a raw CR in text."],
  planned_change=["svg_export::serialize: a raw CR in a text node or attribute value (not a comment or processing instruction) is written '&#13;'; the fast path also checks for a CR."],
  precommitted_measurement=dict(oracle="BJ j8_decoded_cr", expect=["BJ j8_decoded_cr matches whole-output.", "No other output changes in the sweep except documents with a raw CR in text."])),

"clippy-account.json": dict(HEAD,
  sub_mechanism="The 13 rustuml-render clippy diagnostics introduced by earlier rounds (round-7 obligation 6)",
  answers="docs/parity-reviews/tim-builtin-value-boundaries/review-7.json (findings[11]; checker-claude-round6/gates/clippy-introduced.json)",
  planned_change=["useless_borrows_in_formatting (activity.rs x5, archimate.rs, class.rs x2, object.rs): drop the '&' in the format! arguments.",
                  "needless_borrow (state.rs x3): drop the borrow.",
                  "too_many_arguments (deployment.rs): an #[allow] would hide it; instead pass the related arguments in one struct only if that is local; otherwise allow with a reason. Decided at draft time by reading the function."],
  precommitted_measurement=dict(expect=["cargo clippy -p rustuml-render --no-deps -- -D warnings: no diagnostic at the tip that is absent at base 460570ba2 (same per-crate rule for every crate).",
                                        "Sweep outputs byte-identical for these commits (checked with the tip build against a build without them)."]))
}
for name, data in accounts.items():
    (D / name).write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
print(len(accounts), "accounts")
