"""Write maker-round5-claude/results.json and SHA256SUMS.json for both mechanisms."""
import json, hashlib, pathlib
R = pathlib.Path("/Users/marcelo/.t3/worktrees/rustuml/t3-b381b8c8")
P = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round5-claude-maker"
co = json.load(open(P / "work/cohorts.json"))
def sha(p): return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
bins = {t: {"path": str(P / f"bin/rustuml-{t}"), "sha256": sha(P / f"bin/rustuml-{t}")} for t in ["fbb2e59ed", "7ec2e5af8", "5149e8ecb"]}
batches = {}
for b in ["BF", "BE", "BC", "BB", "AW", "AU", "AR"]:
    d = json.load(open(P / f"work/batch-{b}.json"))
    svg = [r for r in d.values() if r["format"] == "-tsvg"]
    valid = [r for r in svg if r["java_exit"] == 0 and not r["java_crash"]]
    row = {"cases": len(d), "svg_cases": len(svg), "java_valid_svg": len(valid)}
    for t in ["base", "before", "tip"]:
        row[f"{t}_java_valid_svg_strict_match"] = sum(r[f"{t}_strict"] == "MATCH" for r in valid)
        row[f"{t}_comment_lists_exact"] = sum(bool(r.get(f"{t}_comments_exact")) for r in valid)
    row["before_to_tip"] = {
        "lost_match": [n for n, r in d.items() if r in valid and r["before_strict"] == "MATCH" and r["tip_strict"] != "MATCH"],
        "gained_match": [n for n, r in d.items() if r in valid and r["before_strict"] != "MATCH" and r["tip_strict"] == "MATCH"],
        "new_valid_input_error": [n for n, r in d.items() if r in valid and r["before_exit"] == 0 and r["tip_exit"] != 0],
        "lost_rejection": [n for n, r in d.items() if r not in valid and r["before_exit"] != 0 and r["tip_exit"] == 0],
        "new_panic": [n for n, r in d.items() if r["tip_panic"] and not r["before_panic"]],
        "outputs_changed": sum(r["before_sha256"] != r["tip_sha256"] for r in d.values()),
        "stderr_changed": [n for n, r in d.items() if r["before_stderr"] != r["tip_stderr"]]}
    row["tip_failing_java_valid_svg"] = {n: f"{r['tip_strict']} {r['tip_ndiff']} (before {r['before_strict']} {r['before_ndiff']}, base {r['base_strict']} {r['base_ndiff']})"
                                          for n, r in d.items() if r in valid and r["tip_strict"] != "MATCH"}
    batches[b] = row
sweep = {c: {k: v for k, v in co[c].items() if not k.endswith("_to_ce1e52e30")} for c in ["goldens", "perturbations", "all"]}
for c in sweep:
    for a in ["fbb2e59ed", "7ec2e5af8"]:
        sweep[c][f"{a}_to_tip_counts"] = co[c][f"{a}_to_ce1e52e30_counts"]
        sweep[c][f"{a}_to_tip"] = co[c][f"{a}_to_ce1e52e30"]
common = {
 "schema_version": 1, "status": "maker packet for independent review; Java relay m5a pending",
 "identities": {"java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
                "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
                "golden_submodule": "9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a",
                "base": "fbb2e59ed (parity-recovery-2026-11-03; crates identical to 9834a7194)",
                "before": "d6f63a8aa merge (crates identical to round-4 maker tip 7ec2e5af8)",
                "tip_code": "5149e8ecb", "private_release_builds": bins,
                "tip_build_note": "the sweep and gates ran on ce1e52e30; 5149e8ecb only rewrites one vec! for clippy and its build is byte-identical on all 16,508 sweep outputs, the 7 batches and m5a"},
 "measurement_contract": "Whole CLI output (-pipe, the case's -t flag) from the recorded cwd, RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100, TZ=GMT+11:00, PLANTUML_STDLIB pinned; unchanged strict comparator rustuml_oracle::compare::compare_svg_strict built from base (tools/checker-cmp-*); exact SVG comment lists.",
 "sweep": sweep,
 "sweep_note": "16,508 sources = 12,550 goldens + 3,958 perturbation pairs in the final tree (the round-4 checker's registered BE/BC pairs included). No new pairs were registered by this maker: maker-registered pairs would be unnamed in any checker review. Before->tip: 298 outputs changed, 234 MATCH->MATCH (154 of them now have Java's canvas size exactly; the comparator tolerates the 1 px root size), 49 DIFF->DIFF, 8 DIFF->MATCH, 7 ERR->ERR (Java crash pages; stderr equal). One DIFF->DIFF perturbation, state-themed-pseudostate-paint/checker_history_font_state_channel, loses Java's canvas height: Rust draws its history labels in the wrong font (pre-existing), and the painted bounds follow the wrong label.",
 "GET_goldens": co["GET_goldens"],
 "stderr": "before->tip: 4 sweep sources change stderr, each only in the thread id of the pre-existing class.rs panic line. Batches: no stderr change apart from the BF YAML/JSON cases that no longer print an export error.",
 "batches": batches,
 "gates": {
  "exit_codes": "base, before, tip identical: lib 0, pipe_mode 0, doc_claims 0; golden_no_oracle, golden_pairs, parity_perturbations, harness_guards 101 (measurements/gates/*.exit)",
  "golden_no_oracle": "11181/11251 in all three trees. before and tip fail the ratchet on the 3 GET goldens only (activity 1299->1297, edge-cases 760->759), as in round 4; base fails on unlocked improvements (class, preprocessing, rendering).",
  "no_oracle_failure_names_vs_9834a7194": "tip vs base: +3 GET goldens (act_skinparam2_shadowing_true, act_skinparam_shadowing, edge_whitespace_multiple_blank_lines), -3 (preproc_complex_theme_override, state_history_shallow, state_note_composite now match). tip vs before: identical (measurements/gates/*.golden_nonmatch_names.txt).",
  "golden_pairs": "12550 total, 10497 passed, 754 failed, 1299 skipped in base, before and tip.",
  "parity_perturbations": "merge tree 1622 failing, tip 1614: 0 new failures, 8 now pass (BE no_outer_note_left_of_image, no_outer_two_notes_on_image, no_root_cluster_note, no_root_two_clusters_notes, pua_b_upper_escape_point; state-autonomous-root-painted-normalization checker_lr_concurrent_nested_labels, checker_lr_root_ellipse_nested_chain; state-parallel-transition-route-identity review_compound_endpoint_parallel_order).",
  "harness_guards": "guard categories and counts identical between the merge tree and tip (measurements/gates/merge.guard_cats.txt, tip.guard_cats.txt); decimal_literal_ratchet output identical (the round-5 drafts add no high-precision literal and remove the 0.75 constant); parity_review_records_are_complete categories identical.",
  "pipe_mode": "17 passed (before, tip); base 13 passed.",
  "workspace_lib": "tip 1584 passed, 0 failed (before 1574, base 1468).",
  "fmt_clippy": "cargo fmt --check clean. clippy findings (warning mode, toolchain 1.99) identical between the merge tree and tip (23 distinct pre-existing lines, measurements/gates/*.clippy.txt); -D warnings fails on both on the pre-existing dead looks_like_description_bracket_command."},
 "frozen_items": "Unchanged: comparator slack, golden_has_syntax_error markers, park list, error census, guard allow-lists and the 14-entry strict-XML list (no file under crates/rustuml-oracle, test-diagrams, Makefile or scripts changed by the round-5 commits).",
 "relay_request": "relay-request-m5a.json: 53 held-out -tsvg probes staged immutable at ~/Library/Caches/rustuml-shared-svg-round5-claude-maker/m5a with Rust predictions of a18a76980, equal to the outputs of ce1e52e30 and 5149e8ecb.",
}
cs = dict(common, mechanism="comment-state-consumers", obligations={
 "1 outer image origin (BLOCKING)": {"status": "implemented f3ebc92f3 (account outer-image-origin-account.json, f5442f4e5) + ce1e52e30 (addendum note-anchor-pass-three-addendum.json, d80bee422)",
   "java_trace": "SvekResult.calculateDimension 128-136 (moveDelta(6 - minX, 6 - minY), dimension delta 15); LimitFinder.drawRectangle 175-179 (x-1,y-1) vs drawUPath 155-158 (Opale note path, no adjustment); CucaDiagram.getDefaultMargins 803-806 (0,5,5,0). Pass three: StateDiagramFactory 97-100 registers CommandFactoryNoteOnEntity for ParserPass.THREE; executeInternal 224-240 finds the anchor created by a pass-two link.",
   "acceptance": "BE no_outer_note_left_of_image MATCH, no_outer_two_notes_on_image MATCH; BF st_np_outer_note_left_multiline MATCH, st_np_outer_note_left_on_leaf_with_image MATCH (and st_np_note_before_target, st_np_root_note_before_states). Held-out: m5a no5_* (8 probes) pending relay.",
   "still_failing_BF": "st_np_outer_note_top_of_image (note top/bottom of an image falls through), st_np_outer_floating_with_image (floating note), st_np_outer_note_in_live_cluster_on_image, st_np_concurrent_regions, st_np_link_note_in_cluster, st_np_three_notes_one_cluster (top/bottom notes in a cluster), st_bp_multiline_label, st_bp_root_wide_multiline; all DIFF in before as well."},
 "5 root-path note qualified name": {"status": "implemented 3ebcb763e (account root-path-note-names-account.json)",
   "java_trace": "CommandFactoryNoteOnEntity.executeInternal 258-260 (getUniqueSequence(GMN), quarkInContext(true, tmp)); CucaDiagram.quarkInContextSafe 249-275 (getCurrentGroup().getQuark().child(full)).",
   "acceptance": "BE no_root_cluster_note MATCH, no_root_two_clusters_notes MATCH, BF st_np_root_three_clusters MATCH; held-out m5a rp5_root_note_depth2, rp5_two_groups_sides pending."},
 "6 routing guard f6b7f66f0": {"status": "addendum routing-guard-addendum.json (f5442f4e5)",
   "java_trace": "CucaDiagramSimplifierState.simplify 57-70 decides whether any GroupMakerState image exists; GraphvizImageBuilder.buildImage is the one root builder; with no image, render_non_autarkic_root_clusters is its port."}},
 residue=[
  "Notes above/below images (note top/bottom of) and floating notes keep falling through.",
  "A note anchor resolved at its own line to a quark a later line makes ambiguous keeps the earlier resolution.",
  "checker_history_font_state_channel: the history label font is wrong (pre-existing), so the painted bounds and canvas height follow it."],
 commits={"f5442f4e5": "accounts (all round-5 obligations), docs only",
          "f3ebc92f3": "draft: outer image origin and size from LimitFinder bounds",
          "3ebcb763e": "draft: note qualified names on every path",
          "d80bee422": "post-measurement addendum: note anchors in pass three, docs only",
          "ce1e52e30": "draft: pass-three note anchor resolution",
          "02fa4159f": "relay request m5a (before any Java result), docs only",
          "5149e8ecb": "clippy-only rewrite in the YAML pre-pass (no output change)"})
tim = dict(common, mechanism="tim-builtin-value-boundaries", obligations={
 "2 missing-glyph advance (BLOCKING)": {"status": "implemented 4a16eaffd (account missing-glyph-account.json, f5442f4e5)",
   "java_trace": "FileFormat.getJavaDimension 220-238 -> Font.getStringBounds 2614-2650 (isComplexText -> TextLayout, else FontDesignMetrics); FontUtilities.isComplexText 190-201, isNonSimpleChar 214-218, isComplexCharCode 239-309; CCharToGlyphMapper (missingGlyph 0, nativeCharsToGlyphs CoreText substitution); CFont.createCompositeFont 203-240 + CFontManager.getOrCreateFallbackFont 328-338 + CCompositeGlyphMapper.convertToGlyph 54-68; HarfBuzz default-ignorable hiding.",
   "font_metric_source": ".notdef (glyph 0) advances read through CoreText at 2048 units/em on the origin Mac (macOS 27.2): Lucida Grande 1536 (regular, bold), Arial 1536, Helvetica 1298 / bold 1479, Verdana 2048, Menlo 1233, Courier New 1229; recorded in measurements/font-chain-extract-headers.json and in the generated crates/rustuml-render/src/font_chain_coverage.rs.",
   "unmapped_set": "derived per face from the cmap coverage of the chain as the JDK resolves it (tools/FontChainExtract.swift, tools/font_chain_coverage.py), not from a code-point range. Validation: 286,520 of 286,591 Java-defined scalars agree with the Java AWT Times New Roman table (measurements/font-chain-validation.json); 407 of 409 NON_ASCII_WIDTHS entries agree (U+FE0F is 0.75 em in Java while CoreText substitutes Apple Color Emoji, and U+043C bold is a mapped glyph whose advance happens to be 0.75 em; the account said 408 by a looser test). Both table entries keep their Java values (tables take precedence).",
   "acceptance": "BE pua_b_upper_escape_point MATCH; BF w_sans_nonchar_10ffff, w_sans_nonchar_10fffe, w_sans_unassigned_3fffd, w_sans_tag_e0041, w_sans_bmp_pua_f000, w_sans_escape_10f7ff, w_arial_nonchar_10ffff MATCH; BC 18/19 as before (pua_b_usecase is T14.21 dispatch). Held-out m5a w5_* pending."},
 "3 every width path decodes escapes (BLOCKING)": {"status": "implemented 4a16eaffd (account width-token-decoding-account.json)",
   "audit": "plantuml_metrics text_width, mono_text_width, courier_new_text_width, times_new_roman_text_width, serif_text_width; text_render family_table_text_width (Arial, Verdana), helvetica_text_width; salt charLen (Java String.length()).",
   "acceptance": "BF w_arial_pua_b MATCH, w_times_pua_b MATCH; w_courier_pua_b and w_monospaced_pua_b now carry Java's textLength (59.001) and fail only on pre-existing SVG emission (SvgGraphics.text 700-710: monospace/courier spaces become U+00A0 and 'Monospaced' is written 'monospace'), as in base (DIFF 1 and DIFF 2)."},
 "4 post-entry decoders (BLOCKING)": {"status": "implemented 673548fb9 (account decode-site-escape-account.json) + a18a76980 (addendum yaml-unquote-addendum.json, a4e895aaf)",
   "java_trace": "JSON: Java's decoder yields the ordinary pair (BF cr_json_u_escape). YAML: yaml/YamlDiagramFactory 89-90 uses yaml/parser/YamlParser; YamlLine.build 51-94 / unquote 189-198 strip quotes only, so '\\U0010FFFD' stays literal.",
   "decoders": "@startjson, @startyaml, model YAML/JSON input, parse_auto JSON fallback, TIM JSON literals, member/index access and %str2json.",
   "acceptance": "BF cr_json_u_escape MATCH, cr_yaml_u_escape MATCH; renamed variants checked in Rust (unit tests) and staged as m5a dc5_* (6 probes) pending."},
 "6 '?'-in-comment rule": {"status": "kept, accounted in comment-unit-addendum.json; Java probes cm_unit_* staged in m5a",
   "java_trace": "SvgGraphics.getTransformer 769-779 (ENCODING us-ascii); ToStream.setOutputStreamInternal ('US-ASCII'.equals fails) -> Encodings.getWriter 83-97 (OutputStreamWriter) -> StreamEncoder 265-268 (REPLACE); ToStream.comment 2275-2340 writes chars to that writer. Golden comments show '?' for every non-ASCII scalar.",
   "note": "Rust's class parser drops an unpaired unit from an entity code (cm_unit_* predictions render 'Q'); Java's CODE regex [^%s{}%g<>]+ would keep it. The relay shows Java's outcome."},
 "out of scope": "T14.21 dispatch (BC pua_b_usecase) and T14.34 PDF: BE fmt_high_pdf and fmt_low_pdf are reference-unavailable (TitledDiagram.java:345 PDF route commented out; Java exit 1 UnsupportedOperationException: PDF), not counted."},
 residue=[
  "Font coverage is the origin Mac's (macOS 27.2, system fonts only); Monkey Man's macOS and JDK 21.0.5 may cover more or fewer scalars; m5a measures several.",
  "BMP characters inside complex text that the chain maps but HarfBuzz would hide keep their table or approximate advance; mapped characters without Java tables keep their approximations; Serif has no chain table.",
  "SansSerif complex-text effect: a BMP scalar mapped only by CoreText substitution becomes Missing inside complex text; safe_unicode_* perturbations (U+20000 now 0.75 em) gain one difference each (DIFF->DIFF); m5a w5_sans_20000 and w5_sans_a500_complex measure it.",
  "YAML flow sequences keep serde's escape decoding; single-quoted '' and multi-line quoted scalars keep YAML semantics.",
  "Text that carries an unpaired unit and also a decoder escape keeps the round-4 decoding.",
  "Creole numeric entities are not decoded by the class renderer (BF cr_ent_dec_10fffd, pre-existing); maxMessageSize wrapping is not implemented (cr_wrap_seq_pua, pre-existing); stereotype guillemets in complex text (cr_stereotype_pua) differ from Java by 2.32 px."],
 commits={"f5442f4e5": "accounts (all round-5 obligations), docs only",
          "4a16eaffd": "draft: font-chain missing glyph + token decoding in every width path (tools/FontChainExtract.swift, tools/font_chain_coverage.py, font_chain.rs, font_chain_coverage.rs)",
          "673548fb9": "draft: decode-site escaping (YAML, JSON, TIM JSON)",
          "a4e895aaf": "post-measurement addendum: YAML unquote, docs only",
          "a18a76980": "draft: YAML quoted backslashes literal",
          "02fa4159f": "relay request m5a, docs only",
          "5149e8ecb": "clippy-only rewrite in the YAML pre-pass (no output change)"})
for m, rec in [("comment-state-consumers", cs), ("tim-builtin-value-boundaries", tim)]:
    D = R / "docs/parity-reviews" / m / "maker-round5-claude"
    (D / "results.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False) + "\n")
    files = sorted(p for p in D.rglob("*") if p.is_file() and p.name != "SHA256SUMS.json")
    (D / "SHA256SUMS.json").write_text(json.dumps({str(p.relative_to(D)): sha(p) for p in files}, indent=1) + "\n")
    print(m, len(files), "files")
