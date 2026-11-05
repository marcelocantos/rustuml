"""Write the round-6 checker's two review records and copy the shared evidence byte-identically into each mechanism's
checker-claude-round6/ directory with SHA256SUMS.json. Usage: python3 -I write_reviews6.py <repo> <evidence_dir>"""
import hashlib, json, pathlib, shutil, sys
REPO = pathlib.Path(sys.argv[1]); EV = pathlib.Path(sys.argv[2])
pairs = json.load(open(EV / "pairs-BI.json")); summ = json.load(open(EV / "batch-summary.json")); coh = json.load(open(EV / "cohorts.json"))
H = {"base": "8317cf006a18ba626a75824b13456934dbe8ce6c", "base_code": "9834a719455308120c3289b59a260567f141b724",
     "merge": "fc44ee58cb74817e37bc1af843f2ba549ab63610", "r5tip": "8dda18b1bf819f379c0c13d1cadd158400eed147",
     "tip": "7e9b16f52ab20a00eba367ce5d572025318325fd", "r5review": "50cacfe23402f66536753aec052b93df07fde4a1",
     "accounts": "56a67759b0904ba7bf56b521e1ace5ab5ba1a3bc", "outer": "0861a51af70aecad1661610043b5646820bd3a70",
     "tool": "9267aa10551028c6498578ac97c057886c0aac86", "marks_interim": "5ceb530dbec1f45762390745954182cd44deec5b",
     "display": "85968418f764e8962900ab3af79ccaea13aa1418", "crlf_add": "51687f9ad7dbd237a5e7efd07610516e60f59343",
     "crlf": "11a9b1d0a6bba0bccdb06e7983dc25342b1c4baf", "block_add": "bd0f31ce1360bbb1f551392480134c600ae885d5",
     "block": "9aead23f0b80af4f33e3d57bd600873597762926", "root_add": "c19eeac56f510b100e8d18f25691d1968f94beb6",
     "root": "ef2f8c39e729fedd582c795c9c76b91728a78413", "relay": "cbc69571567d06aede3bc920e9df5a20260feb56",
     "post_add": "9bca8ab35d22727cd7dc3395bc630a69295fb638", "marks_final": "41bf6019690f1e88589a85c066b84186f915f62c",
     "blank": "ad554600c035b23d33d337f26c9fc2a5f47b94ab", "test_only": "6838ac4ea5cdb432f60f927122ebfe34800c31b5"}
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"; JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
SUB = "checker-claude-round6"; CACHE = "~/Library/Caches/rustuml-shared-svg-round6-claude-checker"
K6A = {"path": f"{CACHE}/k6a/ (read-only)", "reference-manifest.json": "d8010b01957ffb7e2302fc111b4267f0ead659143be056cd86ece45ef32e59d6",
       "reference-input-bundle.json.gz": "a2170453d1b9239a15b7c67378d57104a0a9bfcabaa50844f1318585be62db95",
       "rust-predictions.json": "31b7fcf5693f788ebc2d56530979e8edfbd94a0bce3334a4328ce834a6122264", "SHA256SUMS": "fd0cec5bb061e8da008b4887b305e1c74f0c22aa90379a58c7bd649a8371178c",
       "cases": 120, "status": "staged for the parent relay; no Java result exists; Rust predictions for base, merge and tip recorded before any result"}
MM = {"path": f"{CACHE}/monkeyman-hb/ (read-only)", "run.sh": "f013f976055c62d938a26ca2cbf5b0503c7ead0297dcc889592016d0e52455da",
      "input/probe.puml": "f47193bb49e6a5f0845b1598c8799ef18e2e0ccd4f0413b8c25796ae03e9cf7e", "input/long.puml": "af059c4449f0976f71671c6a2319e06e36e6b1c4dbe6ee7e07da8ddc72796f09",
      "colossus-font-sha256.txt": "17bfae57de4fe91b222fd09f65106e04c68b8bc9bfa17a33cbe951b054b951b8",
      "what": "read-only: otool/realpath/receipt/sha256 of the libharfbuzz the reference JDK loads (DYLD_PRINT_LIBRARIES on one reference run; lsof and vmmap on a live reference JVM), and font-file hashes to compare with colossus"}
COMMANDS = [
    f"git switch -c review/shared-svg-round6-claude {H['tip']}; git -c protocol.file.allow=always submodule update --init --reference /Users/marcelo/work/github.com/marcelocantos/rustuml/test-diagrams/golden test-diagrams/golden  # 9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a",
    f"git archive 8317cf006 | fc44ee58c | 7e9b16f52 -> {CACHE}/src/<rev>; CARGO_TARGET_DIR={CACHE}/target/<rev> cargo build --release -j 6 -p rustuml  # binaries sha256 00094ef5.. (base) 15b48398.. (merge) 8cb25352.. (tip)",
    f"checker-cmp (tools/checker-cmp-main.rs) built against the base export's rustuml-oracle (unchanged base->tip): compare::compare_svg_strict",
    f"python3 -I tools/sweep.py <bin> <out> <repo> work/cwd_map.json test-diagrams/golden test-diagrams/perturbations  # 16,701 sources x 3, recorded cwd; tools/sweep3.sh; tools/cohorts.py",
    f"python3 -I tools/batch.py {CACHE}/work <relay dir> <label> bin/rustuml-8317cf006=base bin/rustuml-fc44ee58c=merge bin/rustuml-7e9b16f52=tip  # BI BH BG BF BE BC BB AW AU AR; tools/batchsum.py",
    "bash tools/gates_all.sh  # gates.sh per tree (lib, pipe_mode, golden_no_oracle, golden_pairs, parity_perturbations, harness_guards, doc_claims) for tip and base; merge: parity_perturbations, harness_guards; tools/gatelists.py",
    "bash tools/clippy_json.sh; python3 -I tools/clippy_list.py  # cargo clippy -p <crate> --no-deps --message-format=json -- -D warnings, crate root touched first",
    "harfbuzz: git clone --filter=blob:none https://github.com/harfbuzz/harfbuzz; git show <tag>:src/<file>; tools/hbdiff.sh",
    "font table: tools/font_chain_coverage.py and tools/font_chain_unicode.py regenerated (+ rustfmt) from the stored extraction and official UCD files; tools/drift_drill.sh (7 mutations, restored)",
    "held-out batch K6A: python3 -I tools/k6a_sources.py; tools/canvas_pool.py (integral-extent selection); tools/predict.py base/merge/tip; tools/stage.py; chmod a-w",
    "python3 -I tools/register.py <repo> pairs-BI.json BI=...; python3 -I tools/write_reviews6.py <repo> evidence",
]
REVIEWER = {"task_id": "t3-661d4578 independent shared-SVG round-6 checker (claude)",
            "model": "claude-opus-5-5 / Claude Code in T3 Code; no children; did not implement the candidate; no Java run locally"}
def perts(mech):
    out = []
    for r in pairs[mech]:
        if not r["registered"]: continue
        out.append({k: r[k] for k in ["name", "batch", "reference_batch", "axes", "intended_valid", "java_exit", "source", "golden", "cwd",
                                       "source_sha256", "golden_sha256"]} | {
            t: f"{r[t+'_cli_exit']}/{r[t+'_strict_compare']}/{r[t+'_difference_count']}/comments={r[t+'_comment_list_exact']}"
            for t in ["base_8317cf006", "merge_fc44ee58c", "tip_7e9b16f52"]} | {"result": r["result"]})
    return out
def bline(L):
    s = summ[L]
    return (f"{s['cases']} cases, Java exit 0 x{s['java_exit_0']}; strict base/merge/tip {s['base_strict']}/{s['merge_strict']}/{s['tip_strict']}, "
            f"strict+comments {s['base_strict_and_comments']}/{s['merge_strict_and_comments']}/{s['tip_strict_and_comments']}; lost vs merge {len(s['lost_vs_merge'])}, vs base {len(s['lost_vs_base'])}; "
            f"new valid-input errors {len(s['new_valid_input_error_vs_merge'])}; lost rejections {len(s['lost_rejection_vs_merge'])}; new panics {len(s['new_panic_vs_merge'])}; "
            f"comments lost {len(s['comments_lost_vs_merge'])}; stderr changed vs merge {len(s['stderr_changed_vs_merge'])}")
a = coh["all"]; g = coh["goldens"]
common = {"schema_version": 1, "reviewer": REVIEWER, "java_revision": JAVA, "jar_sha256": JAR,
  "base_revision": H["base"], "base_code_identical_to": H["base_code"], "merge": H["merge"], "merge_code_identical_to": H["r5tip"],
  "tested_revision": H["tip"], "answers_rejection": H["r5review"], "commands": COMMANDS, "no_production_changes": True, "no_shipping": True,
  "account_chronology": {"accounts (docs only)": H["accounts"], "outer canvas draft": H["outer"], "extraction tool": H["tool"],
     "marks + interim table + drift gate": H["marks_interim"], "YAML/JSON Display draft": H["display"], "CR/LF addendum -> draft": [H["crlf_add"], H["crlf"]],
     "YAML | addendum -> draft": [H["block_add"], H["block"]], "root canvas addendum -> draft": [H["root_add"], H["root"]],
     "relay request r6a (predictions sha256 38bc8cc7..., before results)": H["relay"], "post-result addenda (HarfBuzz 10.0.1, StyleExtractor)": H["post_add"],
     "final table + HarfBuzz 10.0.1 rules": H["marks_final"], "blank-line drop": H["blank"], "test only": H["test_only"],
     "every account and addendum precedes its change in linear ancestry; account and addendum commits are docs-only": True,
     "post-BI output changes disclosed (results.json final_tip_changed_from_prediction: yd7_block_mid_document, mk7_0897_u16, mk7_1e5ee_u16, mk7_after_hebrew, fe7_f8b7_simple, fe7_20b7_simple)": True},
  "frozen_items": "unchanged 8317cf006..7e9b16f52: crates/rustuml-oracle (comparator slack, syntax markers, park list, error census, guard allow-lists, 14-entry strict-XML list), test-diagrams/no_oracle_baseline.txt, decimal_literal_baseline.txt, docs/reference-bootstrap.md; round 6 adds no 3+-decimal literal (ratchet lines identical merge -> tip)",
  "evidence": {"dir": f"{SUB} (byte-identical copy in docs/parity-reviews/comment-state-consumers/ and docs/parity-reviews/tim-builtin-value-boundaries/; SHA256SUMS.json)",
     "measurements": f"{SUB}/measurements.json", "batches": f"{SUB}/batches, {SUB}/batch-summary.json", "cohorts": f"{SUB}/cohorts.json",
     "pairs": f"{SUB}/pairs-BI.json", "gates": f"{SUB}/gates (guard categories, perturbation summary, decimal ratchet, no-oracle families, clippy-diags.json, clippy-introduced.json)",
     "harfbuzz": f"{SUB}/harfbuzz", "font_table": f"{SUB}/font-table (regeneration, cmap, drift drill M0-M7)",
     "held_out_request": f"{SUB}/k6a-request (sources, cases, manifest, bundle, Rust predictions, prediction-summary.json, generator)",
     "monkeyman_request": f"{SUB}/monkeyman-hb-request", "tools": f"{SUB}/tools"},
  "staged_requests_pending": {"K6A": K6A, "monkeyman_harfbuzz_diagnostic": MM},
  "do_not_regress_check": "BH/BG/BF/BE/BC/BB/AW/AU/AR: 0 lost vs merge, 0 comments lost, 0 new errors, panics or stderr changes; sweep merge->tip 0 lost matches, 0 lost rejections, 0 new valid-input errors, 0 new panics or timeouts, 0 comments lost (4 stderr changes are thread ids of pre-existing panics dash_01/03/06/07). Outer image origin, root-path note names, pass-three anchors, routing guard, width-token decoding, decode-site escaping, '?' comment rule, .notdef constants, note paint order, border points, stderr discard, node order, leaf corners, nested x, surrogate serializer, %intval, bracket declaration, strokes, X[H], seed, __theme, %strlen/%substr/%upper: unchanged outcomes in their batches and perturbation directories (parity_perturbations: 0 new failures on files existing at base or at merge).",
  "counts": {"BI_r6a_given_to_maker": bline("BI"), "BH": bline("BH"), "BG": bline("BG"), "BF": bline("BF"), "BE": bline("BE"), "BC": bline("BC"),
             "BB": bline("BB"), "AW": bline("AW"), "AU": bline("AU"), "AR": bline("AR"),
             "goldens": f"{g['sources']} sources, {g['java_valid']} Java-valid: strict {g['8317cf006_valid_strict_match']}/{g['fc44ee58c_valid_strict_match']}/{g['7e9b16f52_valid_strict_match']}; tip exits {g['7e9b16f52_exit']}; 0 panics, 0 timeouts; merge->tip 91 outputs changed, 0 match changes",
             "sweep_all": f"{a['sources']} sources, {a['java_valid']} Java-valid: strict {a['8317cf006_valid_strict_match']}/{a['fc44ee58c_valid_strict_match']}/{a['7e9b16f52_valid_strict_match']}; merge->tip {a['fc44ee58c_to_7e9b16f52_counts']}; base->tip {a['8317cf006_to_7e9b16f52_counts']}",
             "GET_goldens": "act_skinparam2_shadowing_true, act_skinparam_shadowing, edge_whitespace_multiple_blank_lines: MATCH at base, DIFF at merge and tip (raw-reader contract, not round 6)",
             "gates": "golden_no_oracle 11181/11251 at base and tip; per family base->tip activity 1299->1297, edge-cases 760->759 (the 3 GET goldens), preprocessing 285->286, state 895->897 (CLI sweep names: lost = the 3 GET goldens; gained preproc_complex_theme_override, state_history_shallow, state_note_composite); golden_pairs 12550 total, 10497 passed, 754 failed, 1299 skipped at base and tip; parity_perturbations failing base/merge/tip 1365/1667/1656, 0 new failures on files that exist at base, 0 vs merge, 11 fixed merge->tip; harness_guards categories identical merge->tip, T14.6.7 categories 98+98 escapes / 98 / 98 unchanged base->tip; decimal ratchet lines identical merge->tip; pipe_mode 17/17 (base 13/13); workspace lib 1591/1591 (base 1468/1468); doc_claims 2/2",
             "clippy": "cargo clippy -p <crate> --no-deps -- -D warnings, distinct error diagnostics base/merge/tip: rustuml-render 19/32/32, rustuml-parser 1/1/1 (dead_code), rustuml-oracle 1/1/1 (useless_format), rustuml-layout 0/0/0, rustuml 0, rustuml-math 0. Introduced by the packet line: 13 in rustuml-render, all before round 6 (merge == tip). The maker's '2 layout errors' does not reproduce (layout re-linted from a touched crate root: 0 at all three trees); its 22/35 render counts include summary lines."}}
CLIPPY = ("Clippy (bounded obligation, not a parity rejection): 13 rustuml-render diagnostics introduced by the packet line before round 6 (gates/clippy-introduced.json): "
          "clippy::useless_borrows_in_formatting activity.rs:27402 (x2), 28762, 28815, 28869, archimate.rs:629, class.rs:13916 (x2), object.rs:1102; clippy::too_many_arguments deployment.rs:3310; "
          "clippy::needless_borrow state.rs:7644, 7836, 7878. Round 6 adds none. Layout: 0 (the maker's 2 do not reproduce).")
tim_findings = [
  "REJECT (source-traced, deterministic; K6A y8_block_inner_pipe, y8_block_inner_pipe_end, y8_block_inner_pipe_nested measure it). The YAML '|' change (bd0f31ce1 -> 9aead23f0) is a text rewrite, not a port of YamlParser.getBlockStyleString: keep_block_line_breaks (crates/rustuml-parser/src/parse/json_diagram.rs) appends '+' to every line whose trimmed end is ': |', including lines inside a block scalar body. Java's getBlockStyleString (133-149) copies such a body line with cleanBlockStyle (151-154: trim only), so the value text is 'example: |'. The tip draws 'example: |+' (pre-registered: y8_block_inner_pipe tip text 'example: |+\\nplain line', base 'example: |\\nplain line\\n'; y8_block_inner_pipe_end 'Moss\\nratio: |+'; y8_block_inner_pipe_nested 'a: |+\\nb'). The rendered text gains a character that is not in the source and that no Java path can produce. The base did not do this.",
  "The same change misses Java's own block headers: YamlLine.build (52-97) trims the value and removes a YAML comment before comparing with '|', so 'note:   |' and 'note: | # tail' are blocks in Java; the rewrite needs the literal ': |' ending (K6A y8_block_spaced_pipe, y8_block_comment_pipe). cleanBlockStyle trims every body line, so deeper indentation is dropped in Java and kept by serde_yml (y8_block_relative_indent: tip 'Moss\\n  Fern\\nReed'). These two are pre-existing (base also differs), but they are inside the function the addendum claims to port.",
  "StyleExtractor blank-line drop (9bca8ab35 -> ad554600c): matches StyleExtractor 59-64 (String.trim, skip empty) as cited; Rust's c <= ' ' test equals Java's trim. Pre-registered: y8_block_whitespace_lines (space-only and tab-only lines inside a | block) errors at base and merge and renders at tip; y8_blank_lines_everywhere errors at merge only.",
  "YAML/JSON Display.getWithNewlines and AtomText tabs (56a67759b -> 85968418f): the escape table matches Display.java 258-342 for \\\\, \\t, \\n, \\r, \\l and unknown pairs (BI yd7/jd7 12 of 13, K6A measures more). Two departures from Java found in source: (1) the tab stop is a hard-coded eight spaces (TAB_STRING) while AtomText takes FontConfiguration.tabSize from skinParam.getTabSize() (K6A y8_tabsize4, j8_tabsize4; Rust exits 1 on 'skinparam' inside @startyaml/@startjson at all three trees, so this is untestable on Rust until that is fixed); (2) YamlLine.build (54) replaces every literal TAB of a YAML line by four spaces before parsing, but the tip treats a literal TAB in a YAML value as an AtomText tab stop (K6A y8_real_tab_value: tip draws 'a' and 'b' as two texts 35.4 px apart; Java's text is 'a    b'). yd7_raw_brackets residue confirmed: Java draws a hyperlink (<a href=\"a\\nb\">) whose label is split into 'a' and 'b'; Rust keeps '[[a\\nb]]' as one text.",
  "CR/LF advance (51687f9ad -> 11a9b1d0a): the metric is traced. CoreText on colossus (macOS 27.2 26B123, same build as Monkey Man) maps U+000A, U+000D and U+0009 to Lucida Grande glyph 2 with advance 648/2048 (Bold 675), the space advance (font-table/regeneration-and-cmap.txt); the Monkey Man diagnostic records the font files' hashes to close the host gap. Scope: the rule sits in literal_char_width, so it changes every family that measures a real CR: pre-registered s8_chr13_seq (sequence message 67.2153 -> 64.1494) and s8_chr13_class (class name 72.3857 -> 69.084). Both emit a raw CR inside <text>, which an XML parser normalizes to LF (Java's serializer writes &#13;): K6A j8_decoded_cr, s8_chr13_* measure it.",
  "Unmapped marks / HarfBuzz rules (56a67759b -> 5ceb530db; 9bca8ab35 -> 41bf60196): the maker's stored HarfBuzz files are byte-identical to official tag 10.0.1 (harfbuzz/maker-copies-vs-official.txt). Between 8.2.2 and 10.0.1 the traced functions are semantically identical: hb-ot-shape-fallback.cc and hb-ot-shaper-default.cc unchanged; hb_ot_shaper_categorize takes (script, direction, gsub_script) instead of the planner and adds the seven Unicode 16 scripts; the kern if/else chain is restructured equivalently; hebrew compose gains a plan null-check; 10.0.0 adds hb_ot_deal_with_variation_selectors, a no-op unless not_found_variation_selector is set (harfbuzz/diff-8.2.2-vs-10.0.1.txt). The plan facts are ported, not fitted: script_zero_marks and script_fallback_mark_positioning are taken before the morx swap to dumber (hb-ot-shape.cc 88-97), zero_marks = script_zero_marks && !kerx && (!kern || !machine_kern), BY_GDEF_LATE zeroing applies only to the unswapped default shaper, fallback_mark_positioning needs no GPOS/kerx/cross-stream kern, hb_synthesize_glyph_classes makes only non-default-ignorable Mn a MARK. font_chain_unicode.rs regenerates byte-identically from Scripts-3.2.0.txt and UnicodeData 16.0.0 with JDK ScriptRunData equal to Scripts-3.2.0 (0 differences).",
  "Marks, buffer boundary (source trace; K6A measures it): JDK GlyphLayout shapes one HarfBuzz buffer per script run AND font-slot run (FontRunIterator.next splits at every slot change; CCompositeGlyphMapper.convertToGlyph puts a scalar no slot maps in slot 0). The port's after_base looks back through the script run only. When the base comes from another slot font, the unmapped mark is alone in its slot-0 buffer, _hb_ot_shape_fallback_mark_position finds no base, and Java keeps the .notdef; the tip zeroes it. Pre-registered: mk8_helv_hebrew (Helvetica; alef from the Lucida Grande slot; tip text 43.5313 = control mk8_helv_hebrew_ctrl, merge 52.4043), mk8_lucida_arabic (Lucida Grande; alef U+0627 from the Geeza Pro slot; tip 43.3811 = control, merge 53.8811), mk8_seq_arabic (sequence). If Java's width minus its control's equals the .notdef advance, the packet caused these failures. Same-buffer controls (expected zero in Java and tip): mk8_lucida_syriac, mk8_helv_hebrew_pres, mk8_cjk_base (default shaper zeroes regardless). The maker declared 'a base from another slot font' as residue; on these inputs it is not residue but a wrong output the base did not produce.",
  "Marks, loaded HarfBuzz version (open): only the link-time version is known (otool -L of libfontmanager: compat 61001 = 10.0.1). dyld loads whatever /opt/homebrew/opt/harfbuzz holds. On colossus the JDK 21.0.13 bottle links 14.6.0 (61460) and loads the installed 15.0.0 (harfbuzz/colossus-link-vs-load.txt). From 10.0.1 to 15.0.0 the planner applies legacy kern and fallback kern only for fallback-position shapers, applies trak by default for fonts with STAT, position_cluster skips hidden and default-ignorable glyphs when finding the base, and the UCD moves to 17.0 (11.5.0) and 18.0 (14.6.0) (harfbuzz/diff-10.0.1-vs-15.0.0.txt). K6A mk8_u17_1ad0, mk8_u17_1e6e3 (Mn only from Unicode 17) and mk8_u18_05c8 (Unicode 18) discriminate: Java zero means the loaded UCD is newer than 10.0.1's. The Monkey Man diagnostic (monkeyman-hb-request/run.sh) records the loaded path, keg, dylib version and hash.",
  "Font table and drift gate: the stored tools/font-chain-reference/monkeyman-round6 is byte-identical to the parent's artifact (24 files, SHA256SUMS sha256 1c345cc3...); font_chain_coverage.rs regenerates byte-identically (after rustfmt) with the colossus cross-kern facts. Mutation drill (font-table/M*.txt): unmodified passes; a changed stored run, a changed run with its SHA256SUMS line, the same plus the recorded hash, a changed generated run, Menlo's zeroes_unmapped_marks and Lucida Grande's fallback_positions_marks each fail; flipping Helvetica's fallback_positions_marks passes (M5): for faces with a kern table the gate leaves the bit unchecked because it comes from colossus, not the extraction. The gate checks the committed table against the committed artifact; it does not detect drift of the reference host itself (T14.6.8 acceptance 2 needs a re-extraction compared on the host).",
  "Font-environment scalars (fe8_*, K6A): 20 never-probed scalars from the same PingFang-asset and user-font ranges in simple and complex text. Pre-registered tip widths: simple-path PingFang/user-font scalars give 53.2861 (one em), complex PingFang-asset scalars 63.7861. BI shows Java uses the substituted glyph's own advance (e.g. fe7_f880_simple Java 40.4341, tip 53.2861) and zeroes a mapped mark in its slot font (fe7_fe20_complex Java 49.7861, tip 63.7861): K6A expects the same residue on the new scalars (fe8_fe22_complex). These are mapped-advance residue the maker declared, not packet-caused.",
  "BI failures (9): yd7_raw_brackets (hyperlink, above); mk7_after_hebrew and mk7_after_bracket differ only by the Hebrew alef advance (Java 9.0303 = Lucida Grande 1321/2048 x 14, Rust 7.7314) with the mark zeroed as in Java; mk7_mono width equal, family name 'monospace' and NBSP differ (pre-existing); fe7_31d0_simple, fe7_f8b7_simple, fe7_20b7_simple, fe7_f880_simple: substituted-glyph advance not modelled; fe7_fe20_complex: mapped mark zeroed by its slot's plan. None is lost vs base or merge.",
  CLIPPY,
]
tim_obligations = [
  "Blocking: replace keep_block_line_breaks with a port of YamlParser.getBlockStyleString (block header after YamlLine.build's trim and comment removal; body = following lines until a non-text, non-empty line at indent <= the key's; each line cleanBlockStyle-trimmed; '\\n' after each). Acceptance: K6A y8_block_inner_pipe, y8_block_inner_pipe_end, y8_block_inner_pipe_nested, y8_block_spaced_pipe, y8_block_comment_pipe, y8_block_relative_indent, the BI/BH YAML block pairs that match, plus a renamed nested variant not in K6A.",
  "Blocking (if K6A confirms): the mark rule must use the HarfBuzz buffer JDK shapes, script run x font-slot run (FontRunIterator), not the script run alone. Acceptance: K6A mk8_helv_hebrew, mk8_lucida_arabic, mk8_seq_arabic with their controls, and the same-buffer cases mk8_lucida_syriac, mk8_helv_hebrew_pres, BI mk7_after_hebrew.",
  "Record the libharfbuzz the reference JVM loads (monkeyman-hb-request) in docs/reference-bootstrap.md (T14.6.8) and re-check the traced functions and the UCD against that version; K6A mk8_u17_*/mk8_u18_* decide the UCD.",
  "YAML literal TAB: YamlLine.build replaces TAB by four spaces before parsing (K6A y8_real_tab_value); AtomText tab size from skinParam.getTabSize() once skinparam is accepted inside @startyaml/@startjson (K6A y8_tabsize4, j8_tabsize4).",
  "Gate the Helvetica / Helvetica-Bold cross-stream kern bit from the reference host (extract it there, or hash the font files on both hosts; monkeyman-hb-request records them) so mutation M5 fails.",
  "CR inside SVG text: emit &#13; as Java's serializer does, or confirm the comparator treats it as Java does (K6A j8_decoded_cr, s8_chr13_seq, s8_chr13_class).",
  "Clippy -D warnings: fix the 13 rustuml-render diagnostics the packet line introduced before merge.",
]
cs_findings = [
  "Outer image canvas (56a67759b -> 0861a51af): SvgGraphics.ensureVisible (142-147) assigns maxX = (int)(x + 1); the tip uses svg_ensure_visible_dimension at both outer-path sites. Confirmed by BH no6_integer_canvas (Java 108) and BI cv7_integer_outer. K6A adds three outer diagrams with integral width and height (cv8_outer_pier_jetty_mm, cv8_outer_harbour_ko_pier, cv8_outer_mm_pier_il: base 107, tip 108) and two with fractional width (unchanged), selected from 312 generated candidates by base (ceil) != tip ((int)(x + 1)) (tools/canvas_pool.py, k6a-request/canvas-pool-classified.json).",
  "Root cluster path canvas (c19eeac56 -> ef2f8c39e): SvekResult.calculateDimension (130-136) returns (max - min) + 15 after moving the cluster by 6 - min; with the Cuca 5 px margin and ensureVisible this is (int)(max - min + 21). Merge computed ceil(6 - min + max + 15), one pixel wider for every fractional width. In 312 generated root-path candidates every width is fractional and every height integral (merge == tip heights, merge = tip + 1 widths), so K6A's five root cases (cv8_root_*, cv8_rootlate_*) test both rounding branches; BI cv7_root_group_note and cv7_root_note_later_inner match.",
  "Flat state path: unchanged by round 6 (base == tip on all 312 flat candidates); K6A cv8_flat_* and four other families (class with note, usecase, component, activity) record Java's canvas for future work; a failure there would not be caused by this packet.",
  "No state output lost: BI 3/3 canvas pairs, BH/BG/BF/BE/BB state pairs unchanged or gained, sweep merge->tip 0 lost; exact viewBox gain reported by the maker (12,970 -> 13,139) is consistent with the 192 outputs that changed merge->tip.",
  CLIPPY,
]
cs_obligations = [
  "None blocking for the state canvas mechanisms pending K6A (cv8_outer_*, cv8_root_*, cv8_rootlate_*). Carry forward: second same-side note link (BG no5_three_notes_image, BH no6_two_left_same, BF st_np_three_notes_one_cluster), note on a link in an image diagram (BH no6_note_on_link_image), above/below/floating outer notes.",
  "Clippy -D warnings: state.rs:7644, 7836, 7878 (needless_borrow) among the 13 introduced rustuml-render diagnostics.",
]
def write(mech, number, account, impl, other, scope, verdict, packet_text, verdicts, findings, obligations, regressions, failures):
    d = REPO / "docs/parity-reviews" / mech
    rec = common | {"mechanism": mech, "account_commit": account, "implementation_commit": impl, "other_commits_reviewed": other, "scope": scope,
                    "verdict": verdict, "packet_verdict": packet_text, "per_mechanism_verdicts": verdicts, "findings": findings,
                    "attributed_regressions": regressions, "held_out_failures": failures, "bounded_next_obligations": obligations,
                    "staged_java_requests_answered": {"BI": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round6-r6a/",
                        "reference_results_sha256": "432b41840a07dc43319a76481b6353814a92601a31b856ccd11f2fb16523c493", "held_out": False}},
                    "evidence_only_cases": [], "perturbations": perts(mech)}
    (d / f"review-{number}.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False) + "\n")
    tgt = d / SUB
    if tgt.exists(): shutil.rmtree(tgt)
    shutil.copytree(EV, tgt)
    sums = {str(p.relative_to(tgt)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(tgt.rglob("*")) if p.is_file()}
    (tgt / "SHA256SUMS.json").write_text(json.dumps(sums, indent=1) + "\n")
    print(mech, number, len(rec["perturbations"]), "pairs,", sum(p["result"] == "pass" for p in rec["perturbations"]), "pass,", len(sums), "evidence files")
PACKET = ("Packet 7e9b16f52 rejected. The YAML '|' rewrite (9aead23f0) puts a '+' into rendered text whenever a block body line ends in ': |' (pre-registered fresh K6A y8_block_inner_pipe*, "
          "source-traced to YamlParser.getBlockStyleString/cleanBlockStyle; the base drew Java's text). The mark rule (41bf60196) ignores JDK's font-slot buffer split and is predicted to zero marks "
          "Java keeps (K6A mk8_helv_hebrew, mk8_lucida_arabic, pending). All broad gates and the ten existing batches show 0 losses vs merge. K6A (120 cases) and the Monkey Man HarfBuzz diagnostic are staged; the verdicts marked pending wait on them.")
write("tim-builtin-value-boundaries", 6, H["accounts"], H["blank"],
      {"display_draft": H["display"], "crlf_addendum": H["crlf_add"], "crlf_draft": H["crlf"], "block_addendum": H["block_add"], "block_draft": H["block"],
       "extraction_tool": H["tool"], "marks_interim": H["marks_interim"], "post_result_addenda": H["post_add"], "marks_final": H["marks_final"], "test_only": H["test_only"], "relay_request": H["relay"]},
      "round-6 text obligations: YAML/JSON display escapes (Display.getWithNewlines, AtomText tabs), YAML | blocks, StyleExtractor blank lines, CR/LF advance, unmapped nonspacing marks (HarfBuzz 10.0.1), reference-host font table and drift gate",
      "rejected", PACKET,
      {"YAML | block line breaks (bd0f31ce1 -> 9aead23f0)": "rejected: text rewrite injects '+' into block bodies (fresh K6A cases; source-traced); headers and per-line trim of getBlockStyleString not ported",
       "StyleExtractor blank-line drop (9bca8ab35 -> ad554600c)": "confirmed by source (StyleExtractor 59-64); K6A measures",
       "Display.getWithNewlines + AtomText tabs (56a67759b -> 85968418f)": "pending K6A; two source departures (fixed tab size; YAML literal TAB)",
       "CR/LF advance (51687f9ad -> 11a9b1d0a)": "metric traced (Lucida Grande glyph 2); applies to all families; pending K6A",
       "unmapped marks, HarfBuzz 10.0.1 rules (56a67759b -> 5ceb530db, 9bca8ab35 -> 41bf60196)": "not accepted: trace and tables verified, but the buffer is the script run only (font-slot split ignored, predicted failures pending K6A) and the loaded HarfBuzz version is unverified",
       "font table + drift gate (9267aa105, 5ceb530db, 41bf60196)": "table and copy verified byte-identical; gate catches 6/7 mutations (Helvetica fallback bit unchecked); it is a stored-artifact gate, not a host-drift gate"},
      tim_findings, tim_obligations,
      [{"cases": ["K6A y8_block_inner_pipe", "K6A y8_block_inner_pipe_end", "K6A y8_block_inner_pipe_nested"], "commit": H["block"], "status": "source-traced; Java pending",
        "mechanism": "keep_block_line_breaks appends '+' to body lines ending ': |'; YamlParser.getBlockStyleString 133-149 + cleanBlockStyle 151-154 copy them trimmed"},
       {"cases": ["K6A mk8_helv_hebrew", "K6A mk8_lucida_arabic", "K6A mk8_seq_arabic"], "commit": H["marks_final"], "status": "predicted; Java pending",
        "mechanism": "after_base ignores the font-slot buffer split (FontRunIterator.next, CCompositeGlyphMapper missing -> slot 0)"}],
      {"BI yd7_raw_brackets": "hyperlink label split; pre-existing", "BI mk7_after_hebrew, mk7_after_bracket": "Hebrew alef advance; mark correct", "BI mk7_mono": "family name / NBSP",
       "BI fe7_31d0_simple, fe7_f8b7_simple, fe7_20b7_simple, fe7_f880_simple": "substituted-glyph advance", "BI fe7_fe20_complex": "mapped mark zeroed by its slot's plan"})
write("comment-state-consumers", 7, H["accounts"], H["root"],
      {"outer_canvas_draft": H["outer"], "root_canvas_addendum": H["root_add"], "root_canvas_draft": H["root"]},
      "round-6 state obligations: outer image canvas (int)(x + 1), root cluster path canvas (SvekResult dimension, Cuca margin, ensureVisible)",
      "rejected", PACKET,
      {"outer image canvas (56a67759b -> 0861a51af)": "confirmed by source and BH/BI; K6A integral/fractional cases pending",
       "root cluster path canvas (c19eeac56 -> ef2f8c39e)": "confirmed by source and BI; K6A pending"},
      cs_findings, cs_obligations, [], {})
