"""Final round-6 checker records after relays BJ (K6A) and the Monkey Man HarfBuzz diagnostic: tim-builtin-value-boundaries
review-7, comment-state-consumers review-8 (the pending records review-6 / review-7 of commit 409c512eb are kept). Refreshes the
shared checker-claude-round6/ evidence (byte-identical in both mechanism directories, SHA256SUMS.json).
Usage: python3 -I write_reviews6b.py <repo> <evidence_dir>"""
import hashlib, json, pathlib, shutil, sys
REPO = pathlib.Path(sys.argv[1]); EV = pathlib.Path(sys.argv[2]); SUB = "checker-claude-round6"
pairs = json.load(open(EV / "pairs.json")); bj = json.load(open(EV / "relay-results/bj/batch-summary-BJ.json"))["BJ"]
prev = json.load(open(REPO / "docs/parity-reviews/tim-builtin-value-boundaries/review-6.json"))
KEEP = ["schema_version", "reviewer", "java_revision", "jar_sha256", "base_revision", "base_code_identical_to", "merge", "merge_code_identical_to",
        "tested_revision", "answers_rejection", "no_production_changes", "no_shipping", "account_chronology", "frozen_items", "do_not_regress_check"]
common = {k: prev[k] for k in KEEP}
common["commands"] = prev["commands"] + [
    "python3 -I tools/batch.py work /Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round6-k6a BJ bin/rustuml-8317cf006=base bin/rustuml-fc44ee58c=merge bin/rustuml-7e9b16f52=tip; tools/batchsum.py; tools/bj_texts.py (text/viewBox decomposition); exact viewBox census",
    "python3 -I tools/register.py <repo> pairs.json BI=...=432b4184... BJ=...=1ba3bf60...  # error-page markers now include YAML/JSON 'does not sound like'",
    "HarfBuzz 15.0.0: git show 15.0.0:src/{hb-ot-shape.cc,hb-ot-shape-fallback.cc,hb-unicode.hh,hb-ot-shaper*.{hh,cc},hb-ot-shape-normalize.cc} vs 10.0.1; UnicodeData 16.0.0 vs 18.0.0; font tables of Lucida Grande, Helvetica, Menlo .ttc checked for trak/STAT/kern",
    "python3 -I tools/write_reviews6b.py <repo> evidence"]
common["supersedes_pending_records"] = {"commit": "409c512ebac284703eed577196a235580b8bd2c3", "records": ["tim-builtin-value-boundaries/review-6.json", "comment-state-consumers/review-7.json"]}
common["relays_answered"] = {
  "BJ (K6A, held out from the maker)": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round6-k6a/",
     "reference_manifest_sha256": "d8010b01957ffb7e2302fc111b4267f0ead659143be056cd86ece45ef32e59d6", "reference_input_bundle_sha256": "a2170453d1b9239a15b7c67378d57104a0a9bfcabaa50844f1318585be62db95",
     "reference_results_sha256": "1ba3bf60163841bd6a6e4540aa762e09e2587f4f8af431e9a1657afe8b9077cd", "reference_transport_sha256": "e247d6886292a3bfc677b5ada8690e5c38f334902d64038b8bd83740546b4588",
     "parent_relay_validation_sha256": "d76ee71b4d78ab2cdd7b27a0c78a8904da62211652a09ec967f29f5b34b6f4f3",
     "rust_predictions_sha256_registered_before_results": "31b7fcf5693f788ebc2d56530979e8edfbd94a0bce3334a4328ce834a6122264"},
  "Monkey Man HarfBuzz diagnostic": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round6-monkeyman-hb/out/",
     "SHA256SUMS_sha256": "c5d00e3cd71cd98cfb546c940ac747ef1a9281fc33cf0a10d1c010a63314adc8", "run_sh_sha256": "f013f976055c62d938a26ca2cbf5b0503c7ead0297dcc889592016d0e52455da",
     "loaded": "/opt/homebrew/Cellar/harfbuzz/15.0.0_1/lib/libharfbuzz.0.dylib (current_version 61500 = 15.0.0, sha256 b04ab259a343ba51a50db8d1df637160f31d342a66c4192e18dba63ea42ed04a), FreeType 2.15.0; DYLD_PRINT_LIBRARIES and lsof/vmmap of a live reference JVM agree; receipt stable 15.0.0 poured 2026-10-28 10:37 UTC (opt symlink Oct 28 21:37 local); kegs 10.1.0 and 13.2.1 also installed",
     "methodA_output": "byte-identical to BJ mk8_lucida_arabic's Java output (textLength 53.8811): the BJ run used the same library",
     "font_files": "Helvetica.ttc, LucidaGrande.ttc, Menlo.ttc, GeezaPro.ttc, Times New Roman.ttf, Verdana.ttf, Arial.ttf, Courier New.ttf: sha256 identical on Monkey Man and colossus"}}
common["evidence"] = dict(prev["evidence"], relay_results=f"{SUB}/relay-results (bj: batch-BJ.json, batch-summary-BJ.json, bj-viewbox.json, tip-failures-texts.txt, java-error-pages/; monkeyman-hb: static.txt, methodA.*, methodB.*, SHA256SUMS)",
                          pairs=f"{SUB}/pairs.json (BI + BJ; pairs-BI.json kept)", harfbuzz=f"{SUB}/harfbuzz (+ planner-15.0.0.txt, default-ignorable-and-mn-15.0.0.txt, diff-10.0.1-vs-15.0.0.txt)")
common["counts"] = dict(prev["counts"], **{
  "BJ_K6A_held_out": f"120 cases; Java exit 0 x117, exit 200 x3 (s8_chr10_seq, s8_chr13_seq, s8_chr13_class: Java syntax error 'Assumed diagram type: sequence', line 1); 2 more are Java error pages with exit 0 (y8_folded: UnsupportedOperationException wip4, y8_list_key_block: wip3, 'Your data does not sound like YAML data'); 115 Java-valid registered. Strict base/merge/tip 25/42/80 of 120 (strict+comments 15/42/80); registered pass at tip 80/115. Lost vs merge: mk8_lucida_arabic, mk8_seq_arabic; lost vs base: none; 40 gained vs merge; 0 new valid-input errors, 0 lost rejections, 0 panics, 0 comments lost; stderr changed vs merge only y8_blank_lines_everywhere, y8_block_whitespace_lines (merge errored, tip renders). Exact viewBox over the 117 exit-0 cases 24/36/84; lost exact viewBox merge->tip: mk8_lucida_arabic, mk8_seq_arabic, mk8_menlo_hebrew; 51 gained.",
  "intent_disagreements_classified": "y8_folded, y8_list_key_block: intended invalid, Java exit 0 but an error page (YamlParser wip4/wip3): invalid, as intended; Rust renders at base, merge and tip (pre-existing lost rejection). s8_chr10_seq, s8_chr13_seq, s8_chr13_class: intended valid, Java syntax error at line 1 (exit 200): invalid; Rust renders at all three (pre-existing lost rejection). All five are evidence only."})
def perts(mech):
    return [{k: r[k] for k in ["name", "batch", "reference_batch", "axes", "intended_valid", "java_exit", "source", "golden", "cwd", "source_sha256", "golden_sha256"]} |
            {t: f"{r[t+'_cli_exit']}/{r[t+'_strict_compare']}/{r[t+'_difference_count']}/comments={r[t+'_comment_list_exact']}" for t in ["base_8317cf006", "merge_fc44ee58c", "tip_7e9b16f52"]} |
            {"result": r["result"]} for r in pairs[mech] if r["registered"]]
def evidence_only(mech):
    return [{k: r.get(k) for k in ["name", "batch", "java_exit", "reason_not_registered", "tip_outcome_vs_java", "source_sha256"]} for r in pairs[mech] if not r["registered"]]
HB15 = ("HarfBuzz 15.0.0 is what the reference JVM loads; 10.0.1 is only libfontmanager's link-time compatibility version. Consequences (official tag 15.0.0 vs 10.0.1; harfbuzz/): "
  "(1) Unicode data 18.0.0, not 16.0.0: 70 more Mn scalars (39 from 17.0, 31 from 18.0; none reclassified). Java zeroes them (BJ mk8_u17_1ad0 Java 49.7861 = 'Vault ' + U+10FFFD only; mk8_u17_1e6e3 39.2861; mk8_u18_05c8 39.2861); the tip's NONSPACING_MARKS (16.0.0) draws .notdef: 3 fresh failures, pre-existing outputs (tip = merge) but the packet's declared provenance ('HarfBuzz 10.0.1's UCD') is falsified. "
  "(2) Planner: the rules the port uses are unchanged in effect: script_zero_marks and script_fallback_position (renamed from script_fallback_mark_positioning) are still taken before the morx swap to dumber; zero_marks, adjust_mark_positioning_when_zeroing and fallback_mark_positioning keep the same form; hb_synthesize_glyph_classes, zero_mark_widths_by_gdef, hb_ot_position and hb_ot_hide_default_ignorables are identical; default/arabic/hebrew keep BY_GDEF_LATE and fallback_position true, dumber NONE/false. "
  "(3) Changed in 15.0.0 and not modelled: legacy kern (and fallback kern) applies only for fallback-position shapers (affects kerning in other runs on kern faces such as Helvetica, not mark advances); trak applies by default when a font has trak and STAT (Lucida Grande, Helvetica and Menlo .ttc have neither, so no effect on these faces); position_cluster skips hidden and default-ignorable glyphs when looking for a base (a mark after such a glyph in a Hebrew/Arabic run); is_default_ignorable adds U+180F; hb_ot_shaper_categorize adds Unicode 17/18 scripts (unreachable here: ScriptRun maps them to Common); normalization switches to normalizer_glyph_index and a variation-selector flag (no advance effect found). "
  "(4) The oracle itself moved: the opt/harfbuzz link changed on 2026-10-28 with 10.1.0 and 13.2.1 still installed, so relay batches run before that date were shaped by an older library than BH, BI and BJ. That is a T14.6.8 environment fact, not a packet defect.")
MARKS_BUFFER = ("REJECT (fresh held-out, packet-caused). The mark rule's after_base walks the script run; JDK shapes one HarfBuzz buffer per script run x font-slot run (FontRunIterator.next splits at every slot change; CCompositeGlyphMapper puts a scalar no slot maps in slot 0). "
  "BJ, decomposed against controls: mk8_lucida_arabic Java 53.8811 = control 43.3811 + 10.5 (Lucida Grande .notdef 1536/2048 x 14): the mark keeps its .notdef because alef U+0627 is in the Geeza Pro slot; merge 53.8811 MATCH, tip 43.3811 DIFF5. "
  "mk8_seq_arabic (sequence): Java 50.0324, merge MATCH, tip 40.2824 DIFF10. mk8_helv_hebrew: Java 53.7031 - control 44.8301 = 8.873 = Helvetica .notdef 1298/2048 x 14 (alef from the Lucida Grande slot); tip zeroes it (43.5313; merge 52.4043 differed only by the alef advance), DIFF4 -> DIFF5. "
  "mk8_menlo_hebrew: Menlo does not map U+05D0 (CoreText cmap), Java 67.4023 keeps the mark's 8.4287 cell; tip 59.001; exact viewBox lost (119 -> 111). Same-buffer cases match: mk8_lucida_syriac (Syriac unmapped everywhere, zero), mk8_helv_hebrew_pres (U+FB1D unmapped everywhere, zero), mk8_cjk_base (default shaper), BI mk7_after_hebrew (mark zero; alef residue).")
tim_findings = [
  "REJECT (fresh held-out, packet-caused, Java-confirmed). The YAML '|' rewrite (bd0f31ce1 -> 9aead23f0, keep_block_line_breaks) appends '+' to every line ending in ': |', block bodies included. Java (YamlParser.getBlockStyleString 133-149, cleanBlockStyle 151-154) copies those lines trimmed: BJ y8_block_inner_pipe Java 'example: |\\nplain line' (138.7012), tip 'example: |+\\nplain line'; y8_block_inner_pipe_end Java 'Moss\\nratio: |' 84.793, tip 'Moss\\nratio: |+' 95.9219 (without the '+', the tip's LF metric would give Java's text and width; base and merge drew 'ratio: |' at 88.0947, the old LF advance); y8_block_inner_pipe_nested Java 'a: |\\nb', tip 'a: |+\\nb'. Difference counts rise at the tip (5->6, 4->6, 7->8).",
  "Correction to review-6: 'note:   |' and 'note: | # tail' are not missed. serde_yml's default clip keeps the final line break, as Java's block does; BJ y8_block_spaced_pipe and y8_block_comment_pipe match at the tip. The '+' is needed only where serde would not end the body with a break; the rewrite's defect is its scope (any line, body lines included), not the header forms.",
  "cleanBlockStyle's per-line trim is not ported: BJ y8_block_relative_indent Java 'Moss\\nFern\\nReed' (106.7773), tip 'Moss\\n  Fern\\nReed' (pre-existing; base and merge also kept the indentation).",
  "StyleExtractor blank-line drop (ad554600c): confirmed. BJ y8_block_whitespace_lines (base and merge exit 1) and y8_blank_lines_everywhere (merge exit 1) match at the tip; y8_block_two_blocks_end matches.",
  "Display.getWithNewlines and AtomText tabs (85968418f): confirmed on 25 fresh cases (y8 escapes, alignment, nesting, whitespace, CRLF; j8 decoded TAB/LF/CR LF, backslash pairs, bold tab keys, arrays, uneven nested boxes, highlighted rows). Two scoping defects where Java transforms the text before Display: YamlLine.build replaces a literal TAB with four spaces (BJ y8_real_tab_value: Java 'a    b' 34.2617; tip draws AtomText tab stops, DIFF6 -> DIFF12) and YamlLine.toList makes the character after a backslash literal in flow sequences (y8_flow_seq_escapes: Java 'anb', 'etf'; tip splits 'a'/'b' and tabs 'e'/'f', DIFF11 -> DIFF18). Both failed at base and merge too (not lost matches), but the packet moved them further from Java. Java honours 'skinparam tabSize 4' in @startyaml/@startjson (y8_tabsize4, j8_tabsize4 Java exit 0); Rust exits 1 at all three trees (pre-existing valid-input error), so the fixed eight-space TAB_STRING cannot yet be tested against it. y8_escape_trailing is a Java line continuation (trailing backslash joins the next line: 'abcm: \\'), outside this packet.",
  "CR/LF advance (11a9b1d0a): confirmed on the reference host. BJ j8_decoded_cr Java textLength 20.9727 = tip (a + CR as Lucida Grande glyph 2, 648/2048 em + b); j8_unicode_lf, j8_decoded_crlf_tail and y8_crlf match; LucidaGrande.ttc has the same sha256 on Monkey Man and colossus, so the CoreText glyph-2 measurement on colossus is the reference font's. j8_decoded_cr still fails on serialization only: Java writes 'a&#13;b', the tip a raw CR (pre-existing). The non-YAML probes (s8_chr10_seq, s8_chr13_seq, s8_chr13_class) are Java syntax errors, so the rule's effect on other families is not measured; it changes their widths (sequence 67.2153 -> 64.1494, class 72.3857 -> 69.084) where Java rejects the input.",
  MARKS_BUFFER,
  "Marks, confirmed parts: unmapped Unicode-16 Mn in new blocks (mk8_u16_113bb, mk8_u16_10d69, mk8_u16_1612d), Me and Mc not zeroed (mk8_me_1abe, mk8_mc_11f03), leading/indic/Arial/Verdana .notdef kept (mk8_lucida_devanagari, mk8_arial_9/23, mk8_verdana_9/23), and the zero rule at sizes 9 and 23 for SansSerif, Helvetica and Lucida Grande Bold (mk8_sans_9/23, mk8_helv_9/23, mk8_sans_bold_23) match whole-output. Monospaced widths match (mk8_mono_9/23, family name and NBSP differ, pre-existing). Serif: Java keeps both marks (Times New Roman has GPOS; 40.7461), Rust measures Serif with Lucida Grande and now zeroes them (44.4199 -> 30.9199); pre-existing family mapping, failing at merge too. Courier New unchanged and failing (pre-existing). mk8_hebrew_lead_mark fails identically at merge and tip.",
  HB15,
  "Font table and drift gate: fresh PingFang-asset and user-font scalars on the complex path match at the tip for 11 of 12 (fe8_31b8, 324a, 1f105, 327e, f830, fa2e, f881, 218a, f851, f8c0, f8fe complex); fe8_fe22_complex is the declared mapped-mark residue (Java zero; tip 63.7861). Simple path: fe8_31b8, 324a, fe22, f830, f881 simple match; fe8_327e, fa2e, 218a, f851, f8c0, f8fe simple fail on the declared substituted-glyph advance (e.g. f851 Java 47.7148, tip 53.2861). The Helvetica font file is identical on both hosts, so the colossus cross-stream kern bit is the reference host's; the gate still does not check it (drill M5). The table is regenerable and consistent; it records HarfBuzz 10.0.1 as provenance, which is wrong for the reference host.",
  "BI (given to the maker): unchanged, 39/48 at the tip; the 9 failures are as recorded in review-6 and none is lost.",
  prev["findings"][-1],
]
tim_obligations = [
  "Blocking: port YamlParser.getBlockStyleString instead of rewriting text (header from YamlLine.build: trimmed value '|' after comment removal; body until a non-text, non-empty line at indent <= the key's; each line trimmed; '\\n' after each). Acceptance: BJ y8_block_inner_pipe, y8_block_inner_pipe_end, y8_block_inner_pipe_nested, y8_block_relative_indent, the block pairs that match now (y8_block_spaced_pipe, y8_block_comment_pipe, y8_block_two_blocks_end, y8_block_dash_body, y8_block_nested_key, y8_block_whitespace_lines, BI yd7_block_mid_document, BH dc6_yaml_block), plus a renamed variant with a body line ending ': |' at depth 3.",
  "Blocking: shape marks per HarfBuzz buffer = ScriptRun run x font-slot run (FontRunIterator), so a base from another slot does not count. Acceptance: BJ mk8_lucida_arabic, mk8_seq_arabic, mk8_helv_hebrew (its width minus mk8_helv_hebrew_ctrl's), mk8_menlo_hebrew exact viewBox, and the same-buffer cases mk8_lucida_syriac, mk8_helv_hebrew_pres, mk8_cjk_base, BI mk7_after_hebrew; plus a fresh Thai- or Devanagari-slot base before a mark in a default run.",
  "Re-base the mark model on HarfBuzz 15.0.0: NONSPACING_MARKS/UNICODE_MARKS from UnicodeData 18.0.0 (BJ mk8_u17_1ad0, mk8_u17_1e6e3, mk8_u18_05c8), U+180F default-ignorable, position_cluster's hidden/default-ignorable skip; correct the provenance in tools/font_chain_unicode.py, font_chain.rs and font_chain_coverage.py.",
  "T14.6.8: declare HarfBuzz 15.0.0 (and FreeType 2.15.0) as part of the reference environment in docs/reference-bootstrap.md, with the date the link moved (2026-10-28) and its effect on earlier relay batches; gate the Helvetica / Helvetica-Bold kern bit (drill M5).",
  "Apply YamlLine.build's TAB -> four spaces and YamlLine.toList's backslash rule before Display (BJ y8_real_tab_value, y8_flow_seq_escapes); accept 'skinparam' in @startyaml/@startjson and take the tab stop from skinParam.getTabSize() (BJ y8_tabsize4, j8_tabsize4).",
  "Serialize CR in SVG text as &#13; (BJ j8_decoded_cr).",
  "Clippy -D warnings: the 13 rustuml-render diagnostics introduced by the packet line (gates/clippy-introduced.json).",
]
cs_findings = [
  "Outer image canvas (0861a51af): confirmed exactly on fresh held-out cases. BJ cv8_outer_pier_jetty_mm, cv8_outer_harbour_ko_pier, cv8_outer_mm_pier_il (integral width): Java and tip 108 x 482, merge 107 x 482; cv8_outer_dock_mm_harbour, cv8_outer_buoy_mm_lighthouse (fractional): Java = merge = tip. The comparator's frozen +-1 canvas slack lets merge pass strict, so the gain shows in the exact viewBox census only.",
  "Root cluster path canvas (ef2f8c39e): confirmed exactly. BJ cv8_root_pier_jetty_mm 254, cv8_root_quay_dock_harbour 267, cv8_root_wharf_anchorage_pier 233, cv8_rootlate_quay_mooring_wharf 195, cv8_rootlate_mooring_pier_ko 172 wide: Java = tip, merge one wider each; heights equal.",
  "Flat state path and other families (cv8_flat_*, cv8_class_note, cv8_usecase, cv8_component, cv8_activity): exact at base, merge and tip.",
  "The state mechanisms cause no failure on any batch; this record is rejected only because the packet is (tim-builtin-value-boundaries review-7).",
  prev["findings"][-1],
]
cs_obligations = ["None for the canvas mechanisms (accept-ready). Carry forward: second same-side note link, note on a link in an image diagram, above/below/floating outer notes.",
                  "Clippy -D warnings: state.rs:7644, 7836, 7878 (needless_borrow) among the 13 introduced rustuml-render diagnostics."]
PACKET = ("Packet 7e9b16f52 rejected on fresh held-out Java evidence (BJ = K6A). (1) The YAML '|' rewrite (9aead23f0) draws 'example: |+' where Java draws 'example: |' (y8_block_inner_pipe, _end, _nested). "
          "(2) The mark rule (41bf60196) zeroes marks whose base is in another font slot: mk8_lucida_arabic and mk8_seq_arabic matched at merge and fail at the tip; mk8_helv_hebrew and mk8_menlo_hebrew gain the same error. "
          "(3) Its premise is wrong for the reference host: the JVM loads HarfBuzz 15.0.0 (Unicode 18), confirmed by the diagnostic and by mk8_u17_*/mk8_u18_* zero advances. "
          "Confirmed and accept-ready on their own: outer and root canvas (exact on 10 fresh cases), StyleExtractor blank lines, CR/LF advance, Display escapes and AtomText tabs (with two scoping obligations). Broad gates and the ten earlier batches show 0 losses vs merge.")
def write(mech, number, account, impl, other, scope, verdicts, findings, obligations, regressions, failures):
    d = REPO / "docs/parity-reviews" / mech
    rec = common | {"mechanism": mech, "account_commit": account, "implementation_commit": impl, "other_commits_reviewed": other, "scope": scope,
                    "verdict": "rejected", "packet_verdict": PACKET, "per_mechanism_verdicts": verdicts, "findings": findings,
                    "attributed_regressions": regressions, "held_out_failures": failures, "bounded_next_obligations": obligations,
                    "evidence_only_cases": evidence_only(mech), "perturbations": perts(mech)}
    (d / f"review-{number}.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False) + "\n")
    tgt = d / SUB
    if tgt.exists(): shutil.rmtree(tgt)
    shutil.copytree(EV, tgt)
    sums = {str(p.relative_to(tgt)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(tgt.rglob("*")) if p.is_file()}
    (tgt / "SHA256SUMS.json").write_text(json.dumps(sums, indent=1) + "\n")
    print(mech, number, len(rec["perturbations"]), "pairs,", sum(p["result"] == "pass" for p in rec["perturbations"]), "pass,", len(rec["evidence_only_cases"]), "evidence-only,", len(sums), "files")
H = {k: prev["other_commits_reviewed"][k] for k in prev["other_commits_reviewed"]}
write("tim-builtin-value-boundaries", 7, prev["account_commit"], prev["implementation_commit"], prev["other_commits_reviewed"], prev["scope"],
  {"YAML | block line breaks (bd0f31ce1 -> 9aead23f0)": "rejected: '+' injected into block bodies; Java-confirmed on BJ y8_block_inner_pipe, _end, _nested",
   "unmapped marks, HarfBuzz rules (56a67759b -> 5ceb530db, 9bca8ab35 -> 41bf60196)": "rejected: buffer is the script run only; BJ mk8_lucida_arabic and mk8_seq_arabic lost vs merge, mk8_helv_hebrew and mk8_menlo_hebrew worsened; loaded HarfBuzz is 15.0.0 (UCD 18), not 10.0.1",
   "StyleExtractor blank-line drop (9bca8ab35 -> ad554600c)": "confirmed (BJ y8_block_whitespace_lines, y8_blank_lines_everywhere)",
   "Display.getWithNewlines + AtomText tabs (56a67759b -> 85968418f)": "confirmed on 25 fresh cases; obligations: YAML literal TAB and flow-sequence backslash happen before Display in Java; tab size from skinparam",
   "CR/LF advance (51687f9ad -> 11a9b1d0a)": "confirmed (BJ j8_decoded_cr width 20.9727 = Java; same Lucida Grande file on both hosts); CR serialization residue",
   "font table + drift gate (9267aa105, 5ceb530db, 41bf60196)": "table consistent and regenerable; fresh complex-path scalars 11/12 match; gate misses the Helvetica kern bit; HarfBuzz provenance wrong"},
  tim_findings, tim_obligations,
  [{"case": "BJ mk8_lucida_arabic", "commit": prev["other_commits_reviewed"]["marks_final"], "java_exit": 0, "merge": "match", "tip": "DIFF5 (mark zeroed; Java keeps .notdef 10.5)"},
   {"case": "BJ mk8_seq_arabic", "commit": prev["other_commits_reviewed"]["marks_final"], "java_exit": 0, "merge": "match", "tip": "DIFF10 (mark zeroed)"},
   {"case": "BJ mk8_helv_hebrew", "commit": prev["other_commits_reviewed"]["marks_final"], "java_exit": 0, "merge": "DIFF4 (alef advance)", "tip": "DIFF5 (alef advance + mark zeroed; Java keeps 8.873)"},
   {"case": "BJ mk8_menlo_hebrew", "commit": prev["other_commits_reviewed"]["marks_final"], "java_exit": 0, "merge": "DIFF5, exact viewBox 119", "tip": "DIFF6, viewBox 111 (mark zeroed)"},
   {"case": "BJ y8_block_inner_pipe", "commit": prev["other_commits_reviewed"]["block_draft"], "java_exit": 0, "merge": "DIFF5", "tip": "DIFF6 ('example: |+')"},
   {"case": "BJ y8_block_inner_pipe_end", "commit": prev["other_commits_reviewed"]["block_draft"], "java_exit": 0, "merge": "DIFF4 (old LF advance)", "tip": "DIFF6 ('ratio: |+'; only error)"},
   {"case": "BJ y8_block_inner_pipe_nested", "commit": prev["other_commits_reviewed"]["block_draft"], "java_exit": 0, "merge": "DIFF7", "tip": "DIFF8 ('a: |+')"}],
  {"packet-changed, still failing (pre-existing cause)": ["y8_real_tab_value", "y8_flow_seq_escapes", "mk8_serif_9", "mk8_serif_23"],
   "HarfBuzz 15.0.0 UCD (tip = merge)": ["mk8_u17_1ad0", "mk8_u17_1e6e3", "mk8_u18_05c8"],
   "pre-existing, unchanged by round 6": ["y8_block_relative_indent", "y8_escape_trailing", "y8_tabsize4", "j8_tabsize4", "j8_decoded_cr (serialization only)", "m8_json_class", "m8_json_crlf", "m8_json_usecase", "mk8_couriernew_9", "mk8_couriernew_23", "mk8_mono_9", "mk8_mono_23", "mk8_hebrew_lead_mark", "mk8_helv_hebrew_ctrl"],
   "declared residue (substituted-glyph advance, mapped mark)": ["fe8_327e_simple", "fe8_fa2e_simple", "fe8_218a_simple", "fe8_f851_simple", "fe8_f8c0_simple", "fe8_f8fe_simple", "fe8_fe22_complex"]})
write("comment-state-consumers", 8, prev["account_commit"], "ef2f8c39e729fedd582c795c9c76b91728a78413",
  {"outer_canvas_draft": "0861a51af70aecad1661610043b5646820bd3a70", "root_canvas_addendum": "c19eeac56f510b100e8d18f25691d1968f94beb6", "root_canvas_draft": "ef2f8c39e729fedd582c795c9c76b91728a78413"},
  "round-6 state obligations: outer image canvas (int)(x + 1), root cluster path canvas (SvekResult dimension, Cuca margin, ensureVisible)",
  {"outer image canvas (56a67759b -> 0861a51af)": "confirmed exactly on fresh BJ cv8_outer_* (3 integral, 2 fractional)",
   "root cluster path canvas (c19eeac56 -> ef2f8c39e)": "confirmed exactly on fresh BJ cv8_root_*, cv8_rootlate_* (5)"},
  cs_findings, cs_obligations, [], {})
