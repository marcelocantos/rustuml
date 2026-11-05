"""Write maker-round7-claude/results.json from the stored evidence. Usage: python3 -I write_results.py <maker dir> <tip>"""
import json, pathlib, sys
M = pathlib.Path(sys.argv[1]); E = M / "evidence"; TIP = sys.argv[2]
bs = json.load(open(E / "batches/batch-summary.json")); vb = json.load(open(E / "batches/viewbox-census.json"))
co = json.load(open(E / "sweep/cohorts.json")); pert = json.load(open(E / "gates/pert_summary.json"))
ident = json.load(open(E / "sweep/clippy-commit-byte-identity.json"))
def batch(L):
    s = bs[L]; v = vb[L]
    return {"cases": s["cases"], "java_exit_0": s["java_exit_0"],
            "strict_base_merge_tip": [s["base_strict"], s["merge_strict"], s["tip_strict"]],
            "strict_and_comments_base_merge_tip": [s["base_strict_and_comments"], s["merge_strict_and_comments"], s["tip_strict_and_comments"]],
            "lost_vs_merge": s["lost_vs_merge"], "lost_vs_base": s["lost_vs_base"], "gained_vs_merge": s["gained_vs_merge"],
            "new_valid_input_error_vs_merge": s["new_valid_input_error_vs_merge"], "new_valid_input_error_vs_base": s["new_valid_input_error_vs_base"],
            "lost_rejection_vs_merge": s["lost_rejection_vs_merge"], "lost_rejection_vs_base": s["lost_rejection_vs_base"],
            "new_panic_vs_merge": s["new_panic_vs_merge"], "new_panic_vs_base": s["new_panic_vs_base"],
            "comments_lost_vs_merge": s["comments_lost_vs_merge"], "stderr_changed_vs_merge": s["stderr_changed_vs_merge"],
            "exact_viewbox_base_merge_tip_of_exit0_svg": [v["base_exact_viewbox"], v["merge_exact_viewbox"], v["tip_exact_viewbox"], v["cases_exit0_svg"]],
            "exact_viewbox_lost_vs_merge": v["viewbox_lost_vs_merge"], "exact_viewbox_gained_vs_merge": v["viewbox_gained_vs_merge"],
            "comment_list_exact_base_merge_tip": [v["base_comments_exact"], v["merge_comments_exact"], v["tip_comments_exact"]],
            "tip_failures": s["tip_failures"]}
def cohort(c):
    r = co[c]
    keep = ["sources", "java_valid", "java_error"]
    out = {k: r[k] for k in keep}
    for t in ["base", "merge", "tip1"]:
        out[t] = {k[len(t) + 1:]: r[k] for k in r if k.startswith(t + "_") and "_to_" not in k}
    for b in ["base", "merge", "c1"]:
        out[f"{b}_to_tip"] = r[f"{b}_to_tip1"]
    return out
res = {
 "schema_version": 1, "mechanism": "tim-builtin-value-boundaries", "also_recorded_under": "comment-state-consumers/maker-round7-claude (byte-identical copy)",
 "status": "maker results; independent checker review required", "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
 "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
 "trees": {"base": "460570ba24f3de00d28ef57ee8bf9f476061e585 (parity-recovery-2026-11-03; code identical to 9834a7194)",
           "merge": "7167620568369689aac23d76f67f9d74bb40b1f7 (round-6 review 4689792ca on maker tip 7e9b16f52, merged with base)",
           "c1": "9f33bc3b1 (all round-7 code but the clippy commit)", "tip": TIP + " (code identical to c77fa20a2; later commits add evidence only)"},
 "commits": [
   ["7167620568369689aac23d76f67f9d74bb40b1f7", "merge of parity-recovery-2026-11-03 (460570ba2)"],
   ["0030a27dd", "accounts before the drafts (5 files, precommitted measurements)"],
   ["f74008cd8", "obligations 1 and 4: YAML block port, YamlLine TAB/toList, StyleExtractor preamble"],
   ["adc5ffde4", "obligation 5: CR in character data written &#13;"],
   ["9f33bc3b1", "obligations 2, 3, 7: HarfBuzz buffer (script run x slot run) marks under HarfBuzz 15.0.0; primary-face extraction; drift gate"],
   ["c77fa20a2", "obligation 6: the 13 rustuml-render clippy diagnostics"],
   [TIP, "evidence"]],
 "obligations": {
  "1_yaml_block_style": {"status": "done; BJ acceptance met",
     "trace": "YamlLine.build 52-97 (header: trimmed value '|' after removeYamlComment), YamlParser 84-85, getBlockStyleString 132-149 (body until a keyed/listed line indented <= the key), cleanBlockStyle 151-154 (String.trim per line)",
     "measured": "BJ y8_block_inner_pipe, y8_block_inner_pipe_end, y8_block_inner_pipe_nested, y8_block_relative_indent MATCH; y8_block_spaced_pipe, y8_block_comment_pipe, y8_block_two_blocks_end, y8_block_dash_body, y8_block_nested_key, y8_block_whitespace_lines, BI yd7_block_mid_document, BH dc6_yaml_block still MATCH; the '|+' rewrite is removed",
     "pending": "fresh depth-3 renamed variant y9_block_depth3 and four other block probes staged in L7A"},
  "2_buffer_marks": {"status": "done for the mark rule; two cases keep pre-existing non-mark residue",
     "trace": "GlyphLayout.layout 408-431, FontRunIterator.next 115-126, CCompositeGlyphMapper.convertToGlyph 54-67, CoreTextSupport.m 99-133, hb-jdk-font.cc 199-207",
     "measured": {"mk8_lucida_arabic": "MATCH (lost at merge)", "mk8_seq_arabic": "MATCH (lost at merge)",
                  "mk8_helv_hebrew": "DIFF4: width 52.4043 = Rust control 43.5313 + Helvetica .notdef 8.873, exactly Java's mark decomposition (53.7031 - 44.8301); the remaining difference is the alef advance on which mk8_helv_hebrew_ctrl fails identically at base, merge and tip (Rust measures Helvetica text with SansSerif advances); not a mark defect",
                  "mk8_menlo_hebrew": "DIFF5: exact viewBox 119x68 = Java; mark keeps its 8.4287 cell; residue alef advance (67.4297 vs 67.4023: Java's alef comes from another slot with its own advance, Rust counts a Menlo cell) and the Monospaced family name/NBSP (pre-existing, as mk8_mono_9/23)",
                  "same_buffer_controls": "mk8_lucida_syriac, mk8_helv_hebrew_pres, mk8_cjk_base MATCH; BI mk7_after_hebrew unchanged"},
     "note": "These two cases cannot match whole-output without the Helvetica/Monospaced family metrics and other-slot advances, which are outside this packet."},
  "3_harfbuzz_15": {"status": "done",
     "trace": "hb-ot-shape.cc 85-103, 152-221 (legacy kern only with script_fallback_position; trak needs STAT), 856-879, 988-999, 1051-1090; hb-ot-shape-fallback.cc 189-208, 324-342, 426-497 (hidden/default-ignorable skip); hb-ot-layout.hh 216-260; hb-unicode.hh 167-197 (U+180F); hb-ucd-table.hh Unicode 18.0.0; shapers' zero_width_marks/fallback_position; hangul fallback_position true since 12.2.0 (63e8c13e1, not listed in review-7)",
     "measured": "BJ mk8_u17_1ad0, mk8_u17_1e6e3, mk8_u18_05c8 MATCH; Unicode 18 Mn table: 70 added vs 16.0.0, 0 removed; provenance comments name 15.0.0",
     "legacy_kern": "Ported in plan(): apply_kern = has kern && script_fallback_position. On the modelled faces it changes no mark advance (no machine or cross-stream kern; drift gate asserts the facts); Rust models no kern-pair advances, so no other output is affected.",
     "pending": "fresh Unicode 17/18 marks (U+1AD5, U+10EFA, U+11B60, U+0B53, U+1D127, U+10ECB), U+180F after a Thai base, the hangul fallback change and the base-skip in L7A"},
  "4_yaml_preprocessing": {"status": "done; all four BJ cases match",
     "trace": "YamlLine 54 (TAB -> four spaces), 91-93 and toList 117-170, MonomorphToJson 44-80; StyleExtractor 59-98 (preamble skinparam consumed; only handwritten read)",
     "measured": "BJ y8_real_tab_value, y8_flow_seq_escapes, y8_tabsize4, j8_tabsize4 MATCH (the last two: exit 1 -> MATCH; Java ignores tabSize, eight-space tab stop); perturbation shared-svg-attributes/checker-independent-astra/family_json exit 1 -> MATCH",
     "residue": "StyleExtractor's handwritten flag, title, scale, skin and <style> effects are not ported (those inputs behave as before)"},
  "5_svg_text_cr": {"status": "done", "trace": "CharInfo 201/302, 326-334; ToStream 1438-1520, 1619-1650, 1696-1788 (jdk-21.0.5-ga)", "measured": "BJ j8_decoded_cr MATCH; no golden output changed"},
  "6_clippy": {"status": "done", "measured": "cargo clippy -p <crate> --no-deps -- -D warnings: tip diagnostics equal base per crate (render 19, parser 1 dead_code, oracle 1 useless_format, others 0); merge had render 32. Byte identity of the clippy commit: " + json.dumps({k: ident[k] for k in ["sources", "stdout_identical", "exit_identical"]}) + "; stderr differs only in thread ids of four pre-existing panic messages"},
  "7_drift_gate": {"status": "done; one face pending a reference-host hash",
     "measured": "font_chain::tests::coverage_table_is_the_stored_reference_host_extraction checks both stored extractions' SHA256SUMS, every run table, every plan fact including Helvetica and Helvetica-Bold cross_kern, the GPOS 'hebr' fact, trak/STAT, GPOS-kern invariance, and the Monkey Man font-file hashes; mutation drill 12/12 caught (evidence/drift), M0 green, tree restored",
     "pending": "Verdana Bold.ttf is not hashed on Monkey Man and its plan can zero (GPOS lacks 'hebr'); PENDING_REFERENCE_HASH lists it and the gate fails if the list goes stale; relay request mm-font-hash"}},
 "counts": {
   "goldens": cohort("goldens"), "sweep_all": cohort("all"), "perturbations": cohort("perturbations"),
   "GET_goldens": co["GET_goldens"],
   "batches": {L: batch(L) for L in ["BJ", "BI", "BH", "BG", "BF", "BE", "BC", "BB", "AW", "AU", "AR"]},
   "BJ_java_valid_registered": "115 (5 Java error pages are evidence only: y8_folded, y8_list_key_block, s8_chr10_seq, s8_chr13_seq, s8_chr13_class); registered BJ pairs passing at the tip: 94 - 0 error pages = 94 of 115 (the five error pages fail at every tree)",
   "gates": {
     "golden_no_oracle": "11181/11251 at base and tip; per family base->tip activity 1299->1297, edge-cases 760->759 (the 3 GET goldens), preprocessing 285->286, state 895->897; CLI-sweep names: lost = act_skinparam2_shadowing_true, act_skinparam_shadowing, edge_whitespace_multiple_blank_lines (GET provenance); gained = preproc_complex_theme_override, state_history_shallow, state_note_composite; merge->tip: no golden output changed",
     "golden_pairs": "12550 total, 10497 passed, 754 failed, 1299 skipped at base and tip",
     "parity_perturbations": {"failing": pert["failing"], "new_vs_base_on_files_existing_at_base": pert["new_vs_base_existing_at_base"], "new_vs_merge_on_files_existing_at_merge": pert["new_vs_merge_existing_at_merge"], "new_vs_merge_total": pert["new_vs_merge_total"], "fixed_merge_to_tip": pert["fixed_merge_to_tip"]},
     "harness_guards": "failing tests decimal_literal_ratchet and parity_review_records_are_complete at base, merge and tip; categories identical merge->tip (evidence/gates/*.guard_cats.txt); T14.6.7 categories 98+98 escapes / 98 / 98 / 98 unchanged base->tip; decimal ratchet lines identical merge->tip",
     "pipe_mode": "17/17 at tip (base 13/13)", "workspace_lib": "1598/1598 at tip (base 1468/1468)", "doc_claims": "2/2"},
   "clippy": "see obligation 6; evidence/clippy/clippy-diags.json"},
 "frozen_items": "unchanged: crates/rustuml-oracle (comparator slack, syntax markers, park list, error census, guard allow-lists, 14-entry strict-XML list), test-diagrams/no_oracle_baseline.txt, decimal_literal_baseline.txt, docs/reference-bootstrap.md, test-diagrams/",
 "pending_relay_requests": {
   "L7A": {"dir": "/Users/marcelo/Library/Caches/rustuml-shared-svg-round7-claude-maker/l7a", "cases": 49, "intended_invalid": ["y9_skinparam_after_data"],
           "reference-manifest.json": "cf8071ba477fbede6124889c0f0aba365526aab8f7aa4c9f4951db8ab1ffadc6",
           "reference-input-bundle.json.gz": "65522daff0c8db953b374e385777da6e4f75be7d8d79b55f66afc7dc3bb2df9f",
           "rust-predictions.json": "8b4ce1c0e9ab64fb9095bb07df099f4ddc93537a9cb27f18b06e61ad67f57367 (base, merge, tip builds; registered before Java)",
           "invocation": "-pipe -tsvg from PRIVATE_CWD, timezone GMT+11:00"},
   "mm-font-hash": {"dir": "/Users/marcelo/Library/Caches/rustuml-shared-svg-round7-claude-maker/mm-font-hash", "run.sh": "4cf4b0686a4c1800eb5511be8507635ef4a83f1df848b2bd6ab47138dced7891",
                    "purpose": "SHA256 of the nine primary-face font files on Monkey Man (read-only); resolves PENDING_REFERENCE_HASH (Verdana Bold)"}},
 "out_of_scope_not_fixed": ["T14.6.8 owner decision", "JSON elements inside class/usecase diagrams (m8_json_class, m8_json_usecase, m8_json_crlf)", "Courier New and Monospaced family-name handling (mk8_couriernew_*, mk8_mono_*)", "y8_escape_trailing (Java trailing-backslash continuation)"],
 "other_known_residue": ["mk8_hebrew_lead_mark: HarfBuzz normalization decomposes U+FB1D into yod + hiriq in Lucida Grande (Java 54.2295 = 10.5 + 4.4434 + 39.2861); normalization is not modelled (unchanged at base, merge and tip)",
                         "mk8_serif_9/23: Serif measured with Lucida Grande (pre-existing family mapping)",
                         "BJ s8_chr13_class and s8_chr13_seq (Java syntax errors, exit 200; Rust renders at base, merge and tip: pre-existing lost rejections) change output only by the CR now written &#13;",
                         "Older relay batches (AR..BG) were captured before the 2026-10-28 HarfBuzz switch; none of their shaping-dependent results contradicts 15.0.0 here (0 changes vs merge outside BJ and BG dc5_yaml_flow_escape, a YAML case)"]
}
(M / "results.json").write_text(json.dumps(res, indent=1, ensure_ascii=False) + "\n")
print("results.json written")
