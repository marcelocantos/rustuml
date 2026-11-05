"""Write maker-round9-claude/results.json (both mechanism directories, byte-identical)."""
import hashlib, json, pathlib, subprocess, sys
R = pathlib.Path(sys.argv[1]); C = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round9-claude-maker"
W = C / "work"; sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
git = lambda *a: subprocess.run(["git", "-C", str(R), *a], capture_output=True, text=True).stdout.strip()
bs = json.load(open(W / "batch-summary.json")); vb = json.load(open(W / "viewbox-census.json")); co = json.load(open(W / "cohorts.json"))
drill = json.load(open(C / "drift/drill.json")); tl = json.load(open(W / "bidi/textline-compare.json")); bc = json.load(open(W / "bidi/bidi-compare.json"))
tl8 = json.load(open(W / "bidi/textline-compare-round8tip.json"))
batches = {L: {"cases": s["cases"], "java_valid": s["java_valid"], "strict_base_merge_tip": [s["base_strict"], s["merge_strict"], s["tip_strict"]],
               "strict_and_comments_base_merge_tip": [s["base_strict_and_comments"], s["merge_strict_and_comments"], s["tip_strict_and_comments"]],
               "exact_viewbox_base_merge_tip": [vb[L]["base_exact_viewbox"], vb[L]["merge_exact_viewbox"], vb[L]["tip_exact_viewbox"]],
               "comment_list_exact_base_merge_tip": [vb[L]["base_comments_exact"], vb[L]["merge_comments_exact"], vb[L]["tip_comments_exact"]],
               "lost_vs_merge": s["lost_vs_merge"], "gained_vs_merge": s["gained_vs_merge"], "lost_vs_base": s["lost_vs_base"],
               "gained_vs_base": s["gained_vs_base"],
               "viewbox_lost_vs_merge": vb[L]["viewbox_lost_vs_merge"], "viewbox_lost_vs_base": vb[L]["viewbox_lost_vs_base"],
               "comments_lost_vs_merge": s["comments_lost_vs_merge"], "lost_rejection_vs_merge": s["lost_rejection_vs_merge"],
               "new_valid_input_error_vs_merge": s["new_valid_input_error_vs_merge"], "new_panic_vs_merge": s["new_panic_vs_merge"],
               "stderr_changed_vs_merge": s["stderr_changed_vs_merge"], "tip_panics": s["tip_panics"]} for L, s in bs.items()}
g, p = co["goldens"], co["perturbations"]
m9a = json.load(open(C / "relay/m9a/rust-predictions.json"))
pred = {r["name"]: {"tip_viewBox": r["tip"]["viewBox"], "tip_equals_merge": r["tip"]["stdout_sha256"] == r["merge"]["stdout_sha256"],
                    "tip_stdout_sha256": r["tip"]["stdout_sha256"]} for r in m9a["predictions"]}
out = {
 "schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
 "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
 "status": "maker results; not a verdict (independent review required)", "mechanism": "tim-builtin-value-boundaries",
 "answers": "review-9.json (tim-builtin-value-boundaries) and comment-state-consumers/review-10.json (8089ef150)",
 "revisions": {"start (round-8 final review)": "8089ef15030e92f96609a46498d577cdd61c1dcf",
               "merge of parity-recovery-2026-11-03 dcf6d9076 (code = c3116f0b0)": git("rev-parse", "9d6af0a0e"),
               "base (dcf6d9076, code = 9834a7194)": git("rev-parse", "dcf6d9076"),
               "measured code tip": git("rev-parse", "59c10105c")},
 "commit_roles": {git("rev-parse", "c518a95d5"): "accounts (docs only), before any draft: per-face facts, TextLine runs, link-label font, bidi oracle gate",
                  git("rev-parse", "2d031e595"): "code: obligation 2 (text_line_levels = BidiUtils.getLevels run levels) and obligation 4 (java-bidi-oracle test data + test)",
                  git("rev-parse", "ac3b44fbc"): "code: obligation 3 (class center labels laid out in the arrow-style font)",
                  git("rev-parse", "6413e26b4"): "addendum (docs only) after a dev-build BO measurement, before the italic-measurement change was committed",
                  git("rev-parse", "59c10105c"): "code + data: obligation 1 (per-physical-face facts; italic measured in the drawn face)",
                  "next commit": "evidence + results + target T14.39 (bullseye.yaml)"},
 "binaries": (C / "bin/binaries.sha256").read_text().splitlines(),
 "obligations": {
  "1 per-face facts (BLOCKING)": {
   "status": "implemented; BO acceptance met locally; fresh relayed probes pending (M9A); chain runs of the ten italic faces provisional (colossus) pending the requested Monkey Man extraction",
   "source_trace": "CFontManager.loadFonts 209-235 / registerItalicDerived 174-203, Font2D.setStyle 151-175, FontFamily.getFont 320-360, CFont.createItalicVariant 177-185 / getNativeFontPtr 187-191 / getTableBytes 81-85, AWTFont.m awtFontForName 71-111 / getTableBytesNative 368-410, hb-jdk-font.cc reference_table, hb-ot-layout.cc 152-266 OT::GDEF::is_blocklisted (HarfBuzz 15.0.0)",
   "evidence": {"BO gained vs merge": batches["BO"]["gained_vs_merge"],
                "stereotype widths": "c8a_ci_stereo_latin 64.8105 = Java (control 57.6094 + 7.2011); c8a_lt_cnew_thai_stereo 28.8047 (mark kept, +7.2011 over merge 21.6035; Java 28.7349, the 0.0698 Thai control residue remains)",
                "drift gate": "22 faces x 24 Java slots; Courier New Italic (162, 914, 8502) and Bold Italic (114, 928, 8322) not blocklisted, glyph 0 GDEF class 1; plain and bold blocklisted; colossus primary-face facts equal Monkey Man round-6 hb_plan and .notdef on all 12 shared faces"}},
  "2 text_line_levels (BLOCKING)": {
   "status": "implemented; Java differential met; relayed U+2029/U+0085 probe pending (M9A)",
   "source_trace": "TextLine.fastCreateTextLine 933-948, BidiUtils.getLevels 54-82, BidiBase.getRunLevel 4443-4456, BidiLine.getRuns 475-575 (getSingleRun at paraLevel when not MIXED), BidiBase.setPara 3593",
   "evidence": {"DEFAULT_LTR inputs": tl["default_ltr_inputs"], "inputs where TextLine uses the runs": tl["textline_uses_runs"],
                "run-level differences tip": tl["run_level_differences"], "run-level differences round-8 tip c3116f0b0": tl8["run_level_differences"],
                "other fields (requiresBidi, isLeftToRight, base level, getLevelAt) over 242,508": {k: {f: v[f] for f in ["requiresBidi_diff", "isLTR_diff", "paraLevel_diff", "levelAt_diff"]} for k, v in bc["parts"].items()}}},
  "3 round-7 loss c8a_lt_verdana_deva_link (BLOCKING)": {
   "status": "implemented; matches (viewBox 0 0 80 195); renamed held-out variants pending (M9A)",
   "source_trace": "GraphvizImageBuilder 233-241 (arrow style font configuration) -> SvekEdge 279-319 (label TextBlock measured for dot and drawn)",
   "mechanism": "the layout measured the label in sans-serif 13 (Lucida Grande, whose synthesized MARK zeroes U+1D167 since 9f33bc3b1) while emission draws it in the arrow-style font (Verdana 35.2737 = Java)"},
  "4 gate": {"status": "implemented", "evidence": {m: drill[m]["caught"] for m in drill},
             "caught": f"{sum(v['caught'] for k, v in drill.items() if k != 'M00_unmodified')}/{len(drill) - 1}; unmodified exit {drill['M00_unmodified']['exit']}",
             "oracle": "tools/font-chain-reference/java-bidi-oracle: 93,786 lines of the checker's Java differential (e09425f4...), selection rule fixed in the account"}},
 "counts": {
  "goldens (12,550, recorded cwd, byte-level)": {"merge_to_tip": g.get("merge_to_tip_counts", {}), "base_to_tip": g.get("base_to_tip_counts", {}),
       "valid_strict_match_base_merge_tip": [g["base_valid_strict_match"], g["merge_valid_strict_match"], g["tip_valid_strict_match"]],
       "comments_exact_base_merge_tip": [g["base_comments_exact"], g["merge_comments_exact"], g["tip_comments_exact"]],
       "panics_tip": g["tip_panics"], "GET_goldens": co.get("GET_goldens")},
  "perturbations (4,666 incl. registered pairs)": {"merge_to_tip": p.get("merge_to_tip_counts", {}), "merge_to_tip_lists": p.get("merge_to_tip", {}),
       "valid_strict_match_base_merge_tip": [p["base_valid_strict_match"], p["merge_valid_strict_match"], p["tip_valid_strict_match"]],
       "panics_base_merge_tip": [p["base_panics"], p["merge_panics"], p["tip_panics"]],
       "stderr note": "the 4 stderr changes are thread ids in the same pre-existing parse/class.rs:285 panics (dash_01/03/06/07)"},
  "batches": batches,
  "gates": {"golden_pairs base and tip": "12550 total, 10497 passed, 754 failed, 1299 skipped (identical)",
            "golden_no_oracle": "tip 11181 = merge; failure names vs 9834a7194: +3 GET-provenance goldens (activity/act_skinparam2_shadowing_true, activity/act_skinparam_shadowing, edge-cases/edge_whitespace_multiple_blank_lines; lost at merge, raw-reader contract), -3 (preprocessing/preproc_complex_theme_override, state/state_history_shallow, state/state_note_composite)",
            "parity_perturbations failing": "base 1365, merge 1871, tip 1861; tip failures are a subset of merge's (0 new failures in any directory); 10 fewer",
            "harness_guards categories": "identical merge -> tip (gates/*.guard_cats.txt)",
            "pipe_mode": "tip 17/17 ok", "workspace lib": "tip ok (41, 10, 31, 665, 858 passed; 1 ignored)",
            "clippy per crate (error-level, -D warnings)": {"base": [19, 1, 0, 1, 0, 0], "merge": [19, 1, 0, 1, 0, 0], "tip": [19, 1, 0, 1, 0, 0], "order": "render, parser, layout, oracle, rustuml, math"}},
  "frozen": "crates/rustuml-oracle unchanged (comparator slack, syntax markers, park list, error census, guard allow-lists, 14-entry strict-XML list); test-diagrams/no_oracle_baseline.txt unchanged; no new 3+-decimal literal"},
 "residues": [
  "Ten italic faces' chain runs (simple path, composite cascade) are a colossus extraction (tools/font-chain-reference/colossus-styled-chain), listed in PROVISIONAL_CHAIN; Monkey Man extraction requested.",
  "Verdana Italic / Bold Italic have no reference-host file hash (PENDING_REFERENCE_HASH); requested.",
  "Activity notes ignore noteFontName (drawn sans-serif), and activity if-diamond conditions, component edge labels, state box widths and the sequence title are laid out in sans-serif while drawn in the defaultFontName: pre-existing, filed as T14.39.",
  "Arial/Verdana/Helvetica italic advance tables are the upright tables (not touched).",
  "c8a_bl_lucida_thai_digit_after_r exact viewBox lost base -> merge (round-8 code), unchanged at tip; strict status DIFF at all trees."],
 "pending_relay": {
  "M9A (Java reference batch, 60 cases)": {"path": str(C / "relay/m9a"), "reference_manifest_sha256": sha(C / "relay/m9a/reference-manifest.json"),
       "reference_input_bundle_sha256": sha(C / "relay/m9a/reference-input-bundle.json.gz"), "rust_predictions_sha256": sha(C / "relay/m9a/rust-predictions.json"),
       "cases_json_sha256": sha(C / "relay/m9a/cases.json"), "predictions_summary": pred},
  "Monkey Man per-style font extraction": {"path": str(C / "relay/mm-styles"), "run.sh": sha(C / "relay/mm-styles/run.sh"),
       "README.md": sha(C / "relay/mm-styles/README.md"), "SHA256SUMS": sha(C / "relay/mm-styles/SHA256SUMS"),
       "colossus dry run": "exit 0; 22 chain + 24 own files identical (except #host) to the committed colossus extraction"},
  "Java face-resolution probe": {"path": str(C / "relay/face-probe"), "FaceProbe.java": sha(C / "relay/face-probe/FaceProbe.java"),
       "README.md (predictions inside)": sha(C / "relay/face-probe/README.md")}},
}
for d in ["tim-builtin-value-boundaries", "comment-state-consumers"]:
    (R / "docs/parity-reviews" / d / "maker-round9-claude/results.json").write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
print("ok")
