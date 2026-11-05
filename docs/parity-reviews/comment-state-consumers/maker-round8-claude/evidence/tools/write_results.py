"""Write maker-round8-claude/results.json from the evidence directory. Usage: python3 -I write_results.py <repo>"""
import json, pathlib, sys
repo = pathlib.Path(sys.argv[1]); M = repo / "docs/parity-reviews/tim-builtin-value-boundaries/maker-round8-claude"; E = M / "evidence"
summary = json.load(open(E / "batches/batch-summary.json")); census = json.load(open(E / "batches/viewbox-census.json"))
widths = {L: json.load(open(E / f"batches/widths-{L}.json")) for L in ["BM", "BL", "BK", "BJ"]}
cohorts = json.load(open(E / "sweep/cohorts.json")); m2t = json.load(open(E / "sweep/merge-to-tip.json"))
drill = json.load(open(E / "drift/drill.json"))
def w(L, n):
    d = widths[L][n]; return {t: d[t][0] for t in ["java", "base", "merge", "tip"]} | {"strict_base_merge_tip": d["strict"]}
def mark(L, n, ctrl):
    a, c = widths[L][n], widths[L][ctrl]
    return {t: round(a[t][0] - c[t][0], 4) for t in ["java", "merge", "tip"]} | {"strict_base_merge_tip": a["strict"]}
batches = {}
for L, s in summary.items():
    batches[L] = {k: s[k] for k in ["cases", "java_valid", "base_strict", "merge_strict", "tip_strict", "tip_strict_and_comments",
                                    "lost_vs_base", "lost_vs_merge", "lost_rejection_vs_base", "lost_rejection_vs_merge",
                                    "new_valid_input_error_vs_base", "new_valid_input_error_vs_merge", "new_panic_vs_base",
                                    "new_panic_vs_merge", "comments_lost_vs_base", "comments_lost_vs_merge", "gained_vs_merge",
                                    "stderr_changed_vs_merge"]}
    batches[L]["stderr_changed_vs_base"] = s["stderr_changed_vs_base"]
    batches[L]["viewbox"] = {k: census[L][k] for k in ["base_exact_viewbox", "merge_exact_viewbox", "tip_exact_viewbox",
                                                       "viewbox_lost_vs_base", "viewbox_lost_vs_merge", "comments_lost_vs_base", "comments_lost_vs_merge"]}
res = {
 "schema_version": 1, "status": "maker results (round 8); independent review required; not self-certified",
 "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711", "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
 "trees": {"base": "5d8b1f7c1 (parity-recovery-2026-11-03; code identical to 9834a7194)", "merge": "d6e9ffd94 (round-7 maker code b0ca0a7dc merged with base)",
           "tip": "9f025704b (code; later commits add records only)"},
 "binaries": (E / "binaries.sha256").read_text().split("\n")[:3],
 "commits": {"7c391218a": "accounts before draft (bidi component, GDEF blocklist + GSUB chosen script, drift gate)",
             "9f025704b": "implementation (java_bidi, blocklist, categorize, pinned sources, gate)",
             "b6646894b": "addendum: mk8_couriernew_* attribution; 40 blocklist triples"},
 "obligations": {
  "1_bidi_component": {"status": "implemented; measured on BK/BL/BM; fresh probes staged (M8A)",
   "trace": "TextLine.fastCreateTextLine 933-965 / createComponentsOnRun 994-1039 / firstVisualChunk 1296-1305; GlyphLayout.layout 408-431 (ScriptRun and FontRunIterator over the component); BidiBase.requiresBidi 4534-4561, setPara 3526-3710, getDirProps 1476-1677, resolveExplicitLevels 2090-2340, bracket N0 1708-2020, resolveImplicitLevels 3014-3165, adjustWSLevels 3167-3199; ubidi.icu (icudt72b, data 15.0.0)",
   "keep_mark (case - control advance; Verdana .notdef = 14 at size 14)": {
     "BK mk9_verdana_hebrew_space": mark("BK", "mk9_verdana_hebrew_space", "mk9_verdana_hebrew_space_ctrl"),
     "BL c7m_heb_verdana_digit": mark("BL", "c7m_heb_verdana_digit", "c7m_heb_verdana_digit_ctrl"),
     "BL c7m_heb_verdanabold_digit": mark("BL", "c7m_heb_verdanabold_digit", "c7m_heb_verdanabold_ctrl"),
     "BL c7m_heb_verdana_usecase": mark("BL", "c7m_heb_verdana_usecase", "c7m_heb_verdana_usecase_ctrl"),
     "BM c7b_verdana_tail_space (control has no trailing space)": w("BM", "c7b_verdana_tail_space"),
     "BM c7b_verdana_digit_rtl": mark("BM", "c7b_verdana_digit_rtl", "c7b_verdana_digit_rtl_ctrl")},
   "keep_zeroing": {"BM c7b_verdana_rtl_space": mark("BM", "c7b_verdana_rtl_space", "c7b_verdana_rtl_space_ctrl"),
                    "BM c7b_verdanabold_rtl_space": mark("BM", "c7b_verdanabold_rtl_space", "c7b_verdanabold_rtl_space_ctrl")},
   "zero": {"BM c7b_thai_rlm_digit": mark("BM", "c7b_thai_rlm_digit", "c7b_thai_rlm_digit_ctrl"),
            "BM c7b_deva_rlm_digit": mark("BM", "c7b_deva_rlm_digit", "c7b_deva_rlm_digit_ctrl")},
   "whole_output": "the Verdana/Thai/Devanagari cases stay DIFF only by their controls' residue (other-slot base advances), as the controls fail identically at every tree",
   "fresh": "AL + EN (incl. ALM W2), RLE/RLO/LRO/LRE, RLI with and without requiresBidi, FSI, RLM, brackets (incl. JDK's NSM-after-bracket), CS/EN tails: M8A request below; Java results pending",
   "bidi_conformance": "Unicode 15.0.0 BidiCharacterTest: 91676/91707 lines equal; the 31 others are where JDK's BidiBase departs from UAX #9 (overridden brackets still pair; NSM after a closing bracket stays ON, no Unicode 8.0 N0 amendment; more than 63 openings still pair; LRE/LRO in otherwise LTR text unresolved; isolate initiators under an override not overridden); the port follows the JDK source there. Sample of 946 lines stored and gated."},
  "2_gdef_blocklist": {"status": "implemented; 9/9 Courier New cases zero the mark; c7b_cnew_mc keeps its Mc",
   "trace": "hb-ot-layout.cc 152-266 (40 triples; 251-254 Courier New 588/5078/14418 and 588/5078/14238); GDEF.hh 974-985; hb-ot-layout.cc 299-302; hb-ot-shape.cc 83-101, 142-143, 856-879; table lengths via hb-jdk-font.cc 320-356 -> CFont.java 81-85 -> AWTFont.m 367-410 (CTFontCopyTable); GSUB chosen script: hb-ot-layout.cc 563-611, hb-ot-tag.cc 44-181, hb-ot-shaper.hh 176-421",
   "cases": {"BJ mk8_couriernew_9": w("BJ", "mk8_couriernew_9"), "BJ mk8_couriernew_23": w("BJ", "mk8_couriernew_23"),
             "BK mk9_couriernew_hebrew_space": w("BK", "mk9_couriernew_hebrew_space"), "BL c7m_heb_cnew_letter": w("BL", "c7m_heb_cnew_letter"),
             "BL c7m_heb_cnew_digit": w("BL", "c7m_heb_cnew_digit"), "BM c7b_cnew_latin_mark": w("BM", "c7b_cnew_latin_mark"),
             "BM c7b_cnew_leading_mark": w("BM", "c7b_cnew_leading_mark"), "BM c7b_cnew_thai_run": w("BM", "c7b_cnew_thai_run"),
             "BM c7b_cnewbold_latin_mark": w("BM", "c7b_cnewbold_latin_mark"), "BM c7b_cnew_mc (Mc kept)": w("BM", "c7b_cnew_mc")},
   "note": "c7b_cnew_thai_run width 67.2109 vs Java 67.1296 is its control's residue (control 58.8096 vs 58.7282); the mark is zeroed in both",
   "attribution_addendum": "maker-round8-claude/attribution-addendum.json (b6646894b)"},
  "3_drift_gate": {"status": "implemented; drill 13/13 caught, unmodified passes",
   "gates": ["blocklist decision asserted per face from stored CTFontCopyTable lengths (Courier New and Courier New Bold blocklisted; Bold's facts checked equal to plain)",
             "NONSPACING_MARKS/UNICODE_MARKS re-derived from the stored official UCD 18.0.0 UnicodeData (decompressed sha256 0736451d...)",
             "SCRIPT_SHAPERS and HB_SCRIPTS re-derived from stored hb-ot-shaper.hh 3eb84b0a / hb-script-list.h 898de25a; GDEF_BLOCKLIST from hb-ot-layout.cc 3c4e62bc",
             "BIDI_CLASSES/BRACKETS re-decoded from stored ubidi.icu 3d394ff3",
             "tools/font-chain-reference/shaping-sources/SHA256SUMS pinned in font_chain.rs",
             "PENDING_REFERENCE_HASH empty: Verdana Bold 480f66b6... from the Monkey Man record (copied byte-identically, sha256 a5591bab...) equals colossus; both Monkey Man records must agree"],
   "drill": {k: {"exit": v["exit"], "failed_tests": v["failed_tests"]} for k, v in drill.items()}}},
 "measurement": {
  "batches": batches,
  "batch_outputs_changed_merge_to_tip": "17, all in the precommitted set (BM 8, BL 5, BK 2, BJ 2); none in BI..AR",
  "sweep": {"sources": m2t["sources"], "cohorts": {c: {k: v for k, v in cohorts[c].items()} for c in cohorts if c in ("goldens", "perturbations", "all")} if isinstance(cohorts, dict) else None,
            "merge_to_tip_stdout_changed": m2t["stdout_changed"], "merge_to_tip_stderr_changed": m2t["stderr_changed"],
            "stderr_change_cause": m2t["stderr_change_cause"], "golden_outputs_changed_merge_to_tip": m2t["golden_outputs_changed"],
            "GET_goldens": "act_skinparam2_shadowing_true, act_skinparam_shadowing, edge_whitespace_multiple_blank_lines: MATCH at base, DIFF at merge and tip (raw-reader contract, unchanged by round 8)"},
  "gates": {"golden_no_oracle": "11181/11251 at base and tip; family table at tip identical to the round-7 checker's b0ca0a7dc table; base -> tip activity 1299->1297, edge-cases 760->759 (GET), preprocessing 285->286, state 895->897; no golden output changes merge -> tip, so the failure-name set equals b0ca0a7dc's and its diff against 9834a7194 is the checker's no_oracle_failure_names.base-vs-tip.diff (-3 GET, +preproc_complex_theme_override, state_history_shallow, state_note_composite)",
            "golden_pairs": "12550 total, 10497 passed, 754 failed, 1299 skipped at base and tip",
            "parity_perturbations": "failing base 1365, merge 1793, tip 1785; new vs merge 0; new vs base on files existing at base 0; fixed merge -> tip 8 (the Courier New cases)",
            "harness_guards": "tip categories identical to merge (decimal_literal_ratchet and parity_review_records_are_complete fail at base, merge and tip; T14.6.7 path categories 4 x 98 unchanged; decimal-ratchet lines identical merge -> tip)",
            "pipe_mode": "13 passed at base, 17 at tip, 0 failed", "workspace_lib": "base 1468 passed, tip 1604 passed, 0 failed (rustuml-render 857 + 1 ignored full-file conformance test)",
            "clippy_per_crate_errors": "render 19, parser 1, oracle 1, layout 0, rustuml 0, math 0 at base, merge and tip; no new diagnostic in any crate"}},
 "relay_request": {"batch": "M8A", "dir": "/Users/marcelo/Library/Caches/rustuml-shared-svg-round8-claude-maker/m8a", "cases": 63,
   "sha256": {l.split()[1]: l.split()[0] for l in (E / "m8a-request/SHA256SUMS").read_text().splitlines()},
   "predictions": "rust-predictions.json (base, merge, tip) recorded before any Java run"},
 "residue": ["Java results for M8A (fresh AL+EN, explicit-embedding, isolate and Courier New GSUB-script probes) are pending; obligation 1's fresh-probe clause is unmet until they arrive.",
             "The 31 JDK-vs-UAX#9 bidi differences are reproduced from the JDK source, not observed from a JVM; M8A m8b_verdana_heb_bracket_nsm probes one (NSM after a closing bracket).",
             "RTL components: HarfBuzz mirroring of bracket glyphs is not modelled (advances assumed equal).",
             "Hebrew/Thai/Devanagari whole-output residue: other-slot base advances (pre-existing; controls fail identically).",
             "Out of scope (filed): T14.35, T14.36."]
}
out = json.dumps(res, indent=1, ensure_ascii=False) + "\n"
for d in ["tim-builtin-value-boundaries", "comment-state-consumers"]:
    (repo / "docs/parity-reviews" / d / "maker-round8-claude/results.json").write_text(out)
print("ok")
