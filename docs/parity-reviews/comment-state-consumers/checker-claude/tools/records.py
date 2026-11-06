import gzip, hashlib, json, pathlib, shutil, sys
W = pathlib.Path("/Users/marcelo/Library/Caches/rustuml-shared-svg-claude-checker")
REPO = pathlib.Path(sys.argv[1])
AR = pathlib.Path("/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-claude-batch1")
C1 = pathlib.Path("/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-checker-c1")
pairs = json.load(open(W / "pairs.json"))
ev = json.load(open(W / "sweep_events.json")); evr = json.load(open(W / "sweep_events_root.json"))
cm = json.load(open(W / "comments_out.json"))
sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"; JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
BASE = "9834a719455308120c3289b59a260567f141b724"; TIP = "55c92692dd7b589bde814a6fc34ed246746007c6"
MERGED = "5e8dbd6881e49a63aa28922705e93305607fe0f0"
REVIEWER = {"task_id": "t3-28a2cee5-independent-shared-svg-corrective-checker-claude",
            "model": "claude-opus-5-5 / Claude Code in T3 Code; no children; did not implement candidate"}
CK = "/Users/marcelo/Library/Caches/rustuml-shared-svg-claude-checker"
COMMANDS = [
 f"git archive 9834a7194 | tar -x -C {CK}/src-before; git archive 55c92692d | tar -x -C {CK}/src-after; git archive 5e8dbd688 | tar -x -C {CK}/src-merged",
 f"CARGO_TARGET_DIR={CK}/tgt-<tree> cargo build --release -j 6 -p rustuml (before, after, merged)",
 "PLANTUML_STDLIB=~/work/github.com/plantuml/plantuml-stdlib/stdlib CARGO_TARGET_DIR=<tree target> cargo test --release -j 6 -p rustuml-oracle --test golden_no_oracle|golden_pairs|parity_perturbations|harness_guards -- --nocapture (both trees, golden submodule 9efdc12f cloned into each export)",
 "cargo test --release -j 6 -p rustuml --test pipe_mode; cargo test --release -j 6 --workspace --lib (both trees)",
 "python3 -I checker-claude/tools/sweep.py <bin> <out> <test-diagrams> golden perturbations  (16348 sources, cwd=source dir; SWEEP_CWD=<export root> variant for repo-root cwd)",
 "checker-claude/tools/checker-cmp (unchanged rustuml_oracle::compare::compare_svg_strict) over every pair; python3 -I checker-claude/tools/classify.py; python3 -I checker-claude/tools/comments.py",
 "Java references: parent relay of pinned Monkey Man wrapper '/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg' (batch AR worker.1o0kbNgn; batch AU worker.EOk7EEcc for checker staging C1)",
]

def refcopy(dst):
    d = dst / "references"; d.mkdir(parents=True, exist_ok=True); rec = {}
    for tag, src in (("ar", AR), ("au-checker-c1", C1)):
        for f in ("reference-manifest.json", "reference-results.json.gz", "reference-transport.json", "parent-relay-validation.json"):
            out = d / f"{tag}-{f}"; shutil.copyfile(src / f, out); rec[out.name] = {"sha256": sha(out), "origin": str(src / f)}
    (d / "manifest.json").write_text(json.dumps(rec, indent=1, sort_keys=True) + "\n")

def tools(dst):
    d = dst / "tools"; d.mkdir(parents=True, exist_ok=True)
    for f in ("sweep.py", "sweep_cwd.py", "classify.py", "comments.py", "stage.py", "pairs.py", "records.py"):
        shutil.copyfile(W / "scripts" / f, d / f)
    shutil.copyfile(W / "cmp/src/main.rs", d / "checker-cmp-main.rs"); shutil.copyfile(W / "cmp/Cargo.toml", d / "checker-cmp-Cargo.toml")

def pert(rows): return [{k: v for k, v in r.items()} for r in rows]

# ---------- packet-wide measurements (comment-state-consumers) ----------
CS = REPO / "docs/parity-reviews/comment-state-consumers/checker-claude"; CS.mkdir(parents=True, exist_ok=True)
refcopy(CS); tools(CS)
G = W / "gates"
for f in ("before.no_oracle_failure_names.txt", "after.no_oracle_failure_names.txt", "before.guard_cats.txt", "after.guard_cats.txt",
          "before.pert_fail.txt", "after.pert_fail.txt"):
    shutil.copyfile(G / f, CS / f)
shutil.copyfile(W / "src-after/test-diagrams/golden_failure_names.txt", CS / "golden_pairs_failure_names.after.txt")
shutil.copyfile(W / "src-before/test-diagrams/golden_failure_names.txt", CS / "golden_pairs_failure_names.before.txt")
(CS / "cli-status-events.cwd-source-dir.json").write_text(json.dumps(ev, indent=1, sort_keys=True) + "\n")
(CS / "cli-status-events.cwd-repo-root.json").write_text(json.dumps(evr, indent=1, sort_keys=True) + "\n")
(CS / "comment-lists.json").write_text(json.dumps(cm, indent=1, sort_keys=True) + "\n")
cwd_resolution = {
 "rule": "A case is a new valid-input error only if it newly exits nonzero under the cwd its reference was generated with (recorded in that mechanism's review). The maker's tool (cwd = source dir) and a repo-root sweep disagree on 23 sources; each is resolved against its recorded cwd.",
 "maker_reported_10": {
  "tim-external-sub-selection/checker3_dynamic_selector": "recorded cwd '.'; after exit 0 and strict MATCH at repo root (before DIFF). Not an error.",
  "tim-external-sub-selection/checker3_nested": "recorded cwd '.'; after MATCH at repo root. Not an error.",
  "tim-external-sub-selection/checker3_relative": "recorded cwd '.'; after MATCH at repo root. Not an error.",
  "tim-external-sub-selection/checker3_repeated": "recorded cwd '.'; after MATCH at repo root. Not an error.",
  "tim-external-sub-selection/checker3_tabs": "recorded cwd '.'; after MATCH at repo root. Not an error.",
  "tim-function-execution-kind/checker3_include_from_return": "recorded cwd '.'; after MATCH at repo root. Not an error.",
  "tim-parser-error-propagation/checker2_sub_ignored_prelude": "recorded cwd '.'; after MATCH at repo root. Not an error.",
  "tim-external-sub-selection/checker3_retained_sub_caller_relative": "recorded cwd checker3-maker-resources; before and after exit 0 there, both strict DIFF (18 -> 32 differences). Pre-existing mismatch, not a new error.",
  "tim-intval-conversion/checker6/intval_unicode_digits": "GENUINE new valid-input error in every cwd: Java Integer.parseInt (Character.digit) gives 123/-42; Rust str::parse::<i32> is ASCII-only (f663c083d).",
  "tim-intval-conversion/checker6/intval_unicode_json": "GENUINE new valid-input error in every cwd: Java gives 123; Rust rejects Devanagari digit (f663c083d)."},
 "repo_root_only_errors_resolved": {
  "tim-empty-sub-iterator-termination/checker8/empty_selected": "no recorded cwd; after MATCH at source dir (the only cwd at which its relative include resolves).",
  "tim-sub-directive-selection/checker4_sub_function_selector": "recorded cwd = its directory; after MATCH there.",
  "tim-sub-marker-layers/checker5/sub_repeated_comments": "recorded staged cwd mirrors its directory; after MATCH there.",
  "golden/preprocessing/preproc_include_{main,startsub,dark_theme,light_theme}": "Java golden is the Welcome error page; after exit 1 at repo root is a rejection, not a valid-input error."},
 "genuine_new_valid_input_errors": ["perturbations/tim-intval-conversion/checker6/intval_unicode_digits.puml", "perturbations/tim-intval-conversion/checker6/intval_unicode_json.puml"],
}
(CS / "cwd-resolution.json").write_text(json.dumps(cwd_resolution, indent=1) + "\n")

measurements = {
 "trees": {"before": BASE, "after": TIP, "merged_predraft": MERGED, "golden_submodule": "9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a"},
 "no_oracle": {"before": "11181/11251", "after": "11179/11251",
   "lost": ["activity/act_skinparam2_shadowing_true", "activity/act_skinparam_shadowing", "edge-cases/edge_whitespace_multiple_blank_lines"],
   "gained": ["preprocessing/preproc_complex_theme_override"],
   "note": "Both trees fail the ratchet: the base has unlocked progress (class +1, preprocessing +1, rendering +4); the after tree regresses activity 1299->1297 and edge-cases 760->759 against the unchanged baseline."},
 "golden_pairs": {"before": "10497 passed / 754 failed / 1299 skipped of 12550", "after": "10497 / 754 / 1299 of 12550", "failure_names_identical": True},
 "parity_perturbations": {"before_sources": 3008, "before_fail": 1365, "after_sources": 3798, "after_fail": 1602,
   "pre_existing_sources_newly_failing": 0, "pre_existing_sources_gained": 105, "added_sources": 790,
   "added_sources_in_pre_existing_dirs": 9, "added_sources_in_pre_existing_dirs_failing": 9,
   "note": "Counts are before this checker registered its pairs."},
 "harness_guards": {"before": "4/6 (decimal_literal_ratchet, parity_review_records_are_complete fail)", "after": "4/6 (same two)",
   "decimal_ratchet_new_vs_base": {"crates/rustuml-render/src/svg.rs": "0 -> 1 (unit-test literal 0.46667 = 0x77/255 at {:.5}, no adjacent provenance comment)"},
   "review_record_category_growth": {"latest accepted review cannot contain failing perturbations": "1012 -> 1300",
     "perturbation is not named in any checker review": "198 -> 306", "latest checker review must be accepted": "151 -> 198",
     "No such file or directory": "13 -> 15"},
   "t14_6_7_categories": "escape/same-stem/.puml-under: 98 each before and after (no growth)"},
 "pipe_mode": {"before": "16/16", "after": "17/17 (strict-XML census green; EXPECTED_MALFORMED 14 entries, byte-identical to 7ed71f3e5)"},
 "workspace_lib": {"before": "all pass (parser 586, renderer 800)", "after": "parser 649/649; renderer 825/827",
   "after_failures": {"filter_registry::tests::parser_source_identity_matches_java_shadow_id_after_initial_blanks": "expects GET seed fbdz2fx1jlqnt; after emits f1jczin02njk7, which pinned Java -pipe emits for the exact input (AU raw_unit_test_input)",
                      "class::tests::plain_theme_uses_legacy_class_icon_and_font_defaults": "bdda66260 dropped the skinparam __theme marker for unbundled themes"}},
 "cli_whole_output_16348_sources_cwd_source_dir": {
   "lost_matches_valid_refs": 3, "gained_matches_valid_refs": len(ev["gained_match"]), "lost_rejections": ev["lost_rejection"],
   "rejections_gained": len(ev["rejection_gained"]), "new_nonzero_on_valid_refs_raw": len(ev["new_valid_input_error"]),
   "new_valid_input_errors_under_recorded_cwd": cwd_resolution["genuine_new_valid_input_errors"],
   "base_panics_now_exit0_valid_ref": ev["error_now_renders_valid_ref"], "panics": "8 -> 4", "new_panics": 0, "timeouts": 0},
 "comment_lists_16348_pairs": {"exact_before": cm["exact_before"], "exact_after": cm["exact_after"], "gained": len(cm["gained"]),
   "lost": cm["lost"], "lost_note": "all three references are nonpositive: two Java crash SVGs (T14.20) and one Java syntax-error page"},
}
(CS / "measurements.json").write_text(json.dumps(measurements, indent=1) + "\n")

def write_review(mech, name, body):
    p = REPO / "docs/parity-reviews" / mech / name
    assert not p.exists(), p
    p.write_text(json.dumps(body, indent=1, ensure_ascii=False) + "\n")

common = lambda mech: {"schema_version": 1, "mechanism": mech, "reviewer": REVIEWER, "java_revision": JAVA, "jar_sha256": JAR,
                       "base_revision": BASE, "tested_revision": TIP, "commands": COMMANDS,
                       "no_production_changes": True, "no_shipping": True}

# ---------- delay ----------
DL = REPO / "docs/parity-reviews/sequence-delay-label-allocation/checker-claude"; DL.mkdir(parents=True, exist_ok=True); refcopy(DL)
write_review("sequence-delay-label-allocation", "review-2.json", {**common("sequence-delay-label-allocation"),
 "account_commit": "258995f33b259a2ccb21f85b932b7011003dc58b", "implementation_commit": "6830bd056c70fa025fd46897ae0b066c1b2d8931",
 "format_only_followup": "00a5f9ae90a585d9d115e06db4a32ac9728488ed", "merged_predraft_revision": MERGED,
 "scope": "delay leading-spacer atom-run measurement (maker account 258995f33) only; the earlier delay packet record review.json is preserved",
 "verdict": "rejected",
 "spacer_draft_verdict": "confirmed: the atom-run measurement fixes the merge interaction and is Java-traced",
 "findings": [
  "Chronology: account 258995f33 precedes draft 6830bd056 (rustfmt 00a5f9ae9). No constants, no label conditions; the only new code is measure_atom_run_with_family, which omits StripeSimple.getAtoms' empty-stripe space atom (StripeSimple.java:124-129).",
  "Spacer draft confirmed from three trees: the merged predraft 5e8dbd688 fails 4 of the 10 single-line pairs (golden copies edge_seq_delay_texts and edge_sequence_delay, held-out delay_legacy_plain0 and delay_teoz_medium, all with no leading spacer); 55c92692d and base 9834a7194 match Java on all 10. Coverage: leading spacer runs of 0, 1 and 3 in legacy and teoz, plus empty labels in both engines; exact comment lists.",
  "Whole-output (CLI sweep of 16348 sources, predraft vs tip, checker-claude/merged-predraft-to-tip-output-changes.json): 16 outputs change; the 4 sequence outputs are exactly edge_seq_delay_texts, edge_sequence_delay and the existing perturbations checker_d382_e0_trailing / checker_d382_e1_trailing, all DIFF 1 at the predraft and strict matches at the tip and base. No exit status changes.",
  "Mechanism still rejected: the binding finding of review.json (multi-line delay labels: Java Display.getWithNewlines and TextBlock dimensions versus one scalar Rust label) persists. Fresh delay_legacy_multiline (18 differences) and delay_teoz_lead3_multiline (17) fail identically in base, predraft and tip. delay_legacy_lead2_creole (a creole run after leading spacers, 9 differences) and delay_legacy_wide (Java writes no delay text for '...after a considerably longer pause...' yet widens the canvas, 27 differences) also fail unchanged. These are not regressions of the spacer draft, and this record does not waive them.",
  "delay_legacy_short ('...ok...') is a family-dispatch failure (Rust routes it to CLASS) and is registered under comment-state-consumers as packet residue."],
 "perturbations": pert(pairs["sequence-delay-label-allocation"])})

# ---------- T14.24 / evaluated event stream ----------
TE = REPO / "docs/parity-reviews/tim-evaluated-event-stream/checker-claude"; TE.mkdir(parents=True, exist_ok=True); refcopy(TE)
write_review("tim-evaluated-event-stream", "review-4.json", {**common("tim-evaluated-event-stream"),
 "account_commit": "22c023397bc1422d21d24e37e800ed939be3d841", "implementation_commit": "bdda6626040e9c2ecca77d7dc1fc089e9b8ef3d1",
 "scope": "T14.24 raw-reader source identity (owner contract b62d8ce96) and the theme lowering of the same commit; reviews review.json, review-2.json and review-3.json are preserved",
 "verdict": "rejected",
 "raw_reader_interpretation_holds": True,
 "unit_test_seed_confirmed": {"input": "test-diagrams/perturbations/tim-evaluated-event-stream/checker-claude-shared-svg/raw_unit_test_input.puml",
   "java_pipe_shadow_id": "f1jczin02njk7", "after": "f1jczin02njk7", "before_and_test_expectation": "fbdz2fx1jlqnt (GET seed)"},
 "findings": [
  "Java trace verified at 71806a2: Pipe.readSingleDiagram appends every line from the @start line on (blank body lines included); BlockUmlBuilder and UmlSource.loadInternal keep them; StringUtils.seed hashes UmlSource.getPlainString. Only DiagramSourceCompressor.decompress/clean (picoweb GET) trims after removing @startuml. The three lost no-oracle goldens are GET-provenance mismatches documented by b62d8ce96.",
  "Fresh raw-pipe pairs pass on the after tree for every blank-line shape inside the block: 0/1/3 leading blanks in activity and class (AR), the exact unit-test input, trailing blanks before @enduml, CRLF terminators, space/tab-only lines, and an !include whose file starts with blanks (11/11 shadow ids and data-source-line lists exact; 9 of them fail on the base).",
  "Pinned Java -pipe emits f1jczin02njk7 for the exact input of parser_source_identity_matches_java_shadow_id_after_initial_blanks. The test still expects the GET seed fbdz2fx1jlqnt and fails; changing it needs owner sign-off under T14.24's acceptance (the checker did not edit it).",
  "Remaining raw-reader mismatch 1 (pre-existing, identical in base): blank lines BEFORE @startuml. Pipe.readSingleDiagram discards everything before the start mark (state NO_CONTENT, then sb.setLength(0)), so Java numbers from @startuml (source lines 2/3/4, seed fe1qw2wng4r90). Rust keeps the prefix (lines 6/7/8, seed frwspn3gohq7o).",
  "Remaining raw-reader mismatch 2 (pre-existing): multi-diagram stdin. Java Pipe renders each block in turn (block 0 empty -> Welcome page with exit 200; block 1 class with seed fw017dwy8pf55 and lines 3/4/4 relative to its own @startuml). Rust -pipe renders block 0 only and exits 0; --block=1 in file mode gives seed f1d39f5xyy5tlj and lines 7/8/8.",
  "raw_sequence_two_blanks: identity (seed, source lines, comments) matches Java; the remaining 16 differences are sequence geometry, outside T14.24.",
  "REGRESSION attributed to this mechanism (bdda66260): the evaluated theme path pushes a directive placeholder when the theme source is not bundled, and no longer emits 'skinparam __theme NAME' (base preprocess/mod.rs line 1890). Four renderer consumers (class.rs:2972, skinparam.rs:36, sequence.rs:7005, activity.rs:877) still key on __theme, so unbundled themes (plain, mono, amiga, blueprint, ...) lose their renderer emulation. Evidence: unit test class::tests::plain_theme_uses_legacy_class_icon_and_font_defaults fails only on the after tree; golden edge_theme_plain loses Java's Verdana; fresh theme_plain_class_note loses the Verdana fonts and #FFFFFF icon fill that base produced. All three fresh theme pairs fail in both trees, so no strict pass is lost, but whole-output moves away from Java and a valid renderer input is dropped.",
  "Correction to the checker's interim report: the __theme drop is bdda66260, not c1730d656 (a -S search had matched test text in c1730d656)."],
 "bounded_next_obligations": [
  "Emit the __theme marker (or an equivalent parser event) for unbundled theme names exactly as the base did, without bundling GPL theme files; pairs theme_* must not regress against base.",
  "Port Pipe.readSingleDiagram's pre-@start discard for -pipe input (raw_blank_before_start).",
  "Owner decision on the renderer unit-test seed (f1jczin02njk7 confirmed by pinned Java -pipe).",
  "Multi-diagram -pipe output per block (raw_multi_blank_first_block), with a defined exit status for an empty block."],
 "perturbations": pert(pairs["tim-evaluated-event-stream"])})

# ---------- packet / comment-state-consumers ----------
write_review("comment-state-consumers", "review-2.json", {**common("comment-state-consumers"),
 "account_commit": "7704a34c9d88f935e7838ca28868488b36a510a8", "implementation_commit": "b27411107523cb9cd6bb2f26b36ea7b5c61df9fe",
 "account_chronology_record": {"cluster/history": ["e2431c84ed64a3dddeb89844588a44fb1d9ef564", "2bb3b829c2a9500b8b63cf5c171aee9f0ff7f4ca", "00a5f9ae90a585d9d115e06db4a32ac9728488ed"],
   "deployment": ["e349cb2d716e1196ba33a076d8f7507cd986f17e", "db75de35478951e31e5b0966d8015d66a3c496d8"],
   "stroke": ["7704a34c9d88f935e7838ca28868488b36a510a8", "b27411107523cb9cd6bb2f26b36ea7b5c61df9fe"],
   "delay": ["258995f33b259a2ccb21f85b932b7011003dc58b", "6830bd056c70fa025fd46897ae0b066c1b2d8931"],
   "every account precedes its draft": True},
 "scope": "whole packet 9834a7194..55c92692d (shared-SVG corrective 3875fee3b, comment/state 2a791353c, rejections 075f5095 and 7ed71f3e5, this round); review.json (7ed71f3e5) is preserved",
 "verdict": "rejected",
 "per_mechanism_verdicts": {
  "sequence-delay-label-allocation (spacer atom run)": "spacer draft confirmed; mechanism record rejected because the earlier multi-line delay finding persists on fresh pairs, see sequence-delay-label-allocation/review-2.json",
  "T14.24 raw reader (tim-evaluated-event-stream)": "interpretation holds; record rejected for the bdda66260 theme-marker regression and two pre-existing raw shapes, see tim-evaluated-event-stream/review-4.json",
  "state cluster/history comment selection": "rejected: the account invariant (cluster comment iff Java's consumer is Cluster.drawU) fails on 6 of 7 fresh Java non-autarkic groups; history and link endpoint naming are confirmed",
  "deployment Quark identity and reversal": "rejected as complete: reversal (LinkType.looksLikeRevertedForSvg after getInv) and alias identity are confirmed on 3 fresh pairs, but the precommitted bracket-declaration axis fails (deploy_unaliased_forms)",
  "StateSkin stroke": "accepted within its effect: every fresh stroke-colour case reachable by asColor matches Java; the declared boundary is inaccurate (stereotype <style> and stereotype skinparam strokes changed too, toward Java)",
  "inherited TIM ancestry": "blocking: 2 new valid-input errors (tim-intval-conversion f663c083d), 1 lost rejection (tim-builtin-value-boundaries 92647b126 lineage, T14.20), 1 dropped renderer input (tim-evaluated-event-stream bdda66260)"},
 "findings": [
  "Measurements reproduce the maker's tier numbers (measurements.json): no-oracle 11181 -> 11179 of 11251 (3 GET-provenance losses, 1 gain); golden_pairs 10497/754/1299 with identical failure names; 0 newly failing pre-existing perturbations; comment lists exact 14270 -> 14952 of 16348 (+685, -3, all nonpositive references).",
  "The maker's '10 new valid-input errors' overcounts: its comment_lists.py and status sweep run every source with cwd = source dir, but seven of those sources use repo-root-relative includes and their reviews record cwd '.'. At that cwd all seven match Java exactly (they were silent wrong renders in the base). retained_sub_caller_relative exits 0 at its recorded cwd (pre-existing strict DIFF). Two are genuine: intval_unicode_digits and intval_unicode_json. Java IntVal.executeReturnFunction calls Integer.parseInt, which accepts any Unicode Nd digit through Character.digit; f663c083d replaced the base f64 parse with ASCII-only str::parse::<i32>. tim-intval-conversion/review.json already rejected exactly this. See cwd-resolution.json.",
  "Lost rejection: surrogate_display_residue. Base panicked (preprocess/mod.rs:2587 byte slice); the UTF-16 %substr port (92647b126 lineage, tim-builtin-value-boundaries) leaves a lone low surrogate that Rust renders as U+FFFD in a normal exit-0 diagram, where Java exports the SAXException crash SVG (invalid XML character 0xde80). T14.20 has no error contract to carry it.",
  "Inherited dropped renderer input: bdda66260 stopped emitting 'skinparam __theme NAME' for unbundled themes; see tim-evaluated-event-stream/review-4.json.",
  "Cluster comment selection: java_state_group_is_autarkic matches Entity.isAutarkic (CONCURRENT_STATE, isPureInnerLink3, direct-leaf EntityPosition with equalsIgnoreCase stereotypes) and suppresses the comment only inside emit_root_state_cluster. But Rust's consumer dispatch does not follow Entity.isAutarkic. Fresh Java non-autarkic groups drawn by Rust as entity images with no cluster comment: cluster_entry_point (Mill), quark_reuse_crossing (Sluice), cluster_inputpin_mixed_case (Press), cluster_root_note_on_inner (Kiln, a root note link makes it non-autarkic), cluster_entry_inside_autarkic (Shed), history_external_suffix_form (Vault). Only history_depth2_crossing is right. The account's premise that Cluster.drawU runs only for the root SVEK image is also incomplete: Java emits 'cluster Shed' from inside the autarkic Works image (GroupMakerState builds a nested SVEK). The maker's residual list names only the opposite direction (Java-autarkic groups painted as Rust clusters).",
  "History identity: [H]/[H*] endpoints now keep *historical*X/*deephistory*X and their group scope (history_depth1_autarkic and history_depth2_crossing comment lists exact; history_root_marker passes). The suffix form Vault[H] / Vault[H*] (StateDiagram.getHistorical(location, idShort)) is unsupported in both trees: Rust drops Guard and both links.",
  "Deployment: deploy_alias_reverse_up comments exact; deploy_alias_direction_matrix (left/right heads under LEFT and DOWN, both heads, undecorated) and deploy_nested_alias_reverse are full strict matches after (both DIFF in base). deploy_unaliased_forms still splits bare '[Grain Store]' into Grain_Store plus a second node 'Grain Store' (the parse path for a keywordless bracket declaration does not set quark_name), identical to base.",
  "Stroke: ValueImpl.asColor and SvgGraphics.fixColor are traced correctly (HColorSimple.isTransparent is alpha == 0, so any zero-alpha colour is written as none; stroke_zero_alpha_nonblack matches). autonomous_state_style resolves stereotype <style> lineColor through the new rule, so stereotype strokes changed although declared out of scope; both fresh stereotype cases (style and skinparam) now match Java. Per-arrow -[#RebeccaPurple]-> stays #663399 where Java writes #FFFFFF (declared residual). With ArrowColor transparent, Java omits the edge path entirely; Rust writes stroke:none correctly but still emits the path (consumer residual). stroke_per_state_css4 is a Java 'No such color' error that Rust renders (T14.20 class).",
  "Intent/Java disagreements kept as staged: cluster_concurrent_crossing (Java: 'The state Blow cannot be used here', exit 200), raw_multi_blank_first_block (exit 200 for an empty first block), stroke_per_state_css4 (exit 200, 'No such color'). Rust renders all three as normal diagrams in both trees.",
  "Other checks: no fixture-keyed logic in shipping code (all label-like literals added by the packet are in #[cfg(test)]); frozen items unchanged against base (oracle crate, harness markers, ulp_parked, census, guard allow-lists; EXPECTED_MALFORMED unchanged since 7ed71f3e5); svg.rs gains one unit-test decimal literal 0.46667 without an adjacent provenance comment; T14.6.7 escape categories do not grow and every pair in this record lies in its own mechanism directory."],
 "attributed_regressions": [
  {"case": "test-diagrams/perturbations/tim-intval-conversion/checker6/intval_unicode_digits.puml", "kind": "new valid-input error", "commit": "f663c083df56b736b7d8fef693a9b9a3c106c7a5", "mechanism": "tim-intval-conversion"},
  {"case": "test-diagrams/perturbations/tim-intval-conversion/checker6/intval_unicode_json.puml", "kind": "new valid-input error", "commit": "f663c083df56b736b7d8fef693a9b9a3c106c7a5", "mechanism": "tim-intval-conversion"},
  {"case": "test-diagrams/perturbations/tim-builtin-value-boundaries/checker5/surrogate_display_residue.puml", "kind": "lost rejection (Java crash SVG -> Rust normal diagram)", "commit": "92647b126f6dc1c83ebdc6213d0c2c0a5b3211b6", "mechanism": "tim-builtin-value-boundaries", "target": "T14.20"},
  {"case": "!theme <unbundled> (unit test plain_theme_uses_legacy_class_icon_and_font_defaults; golden edge_theme_plain)", "kind": "dropped renderer input", "commit": "bdda6626040e9c2ecca77d7dc1fc089e9b8ef3d1", "mechanism": "tim-evaluated-event-stream"}],
 "bounded_next_obligations": [
  "tim-intval-conversion: Integer.parseInt digit semantics (Character.digit over UTF-16 units, optional sign) so both intval_unicode pairs match without new errors.",
  "tim-builtin-value-boundaries / T14.20: a defined non-normal outcome for a lone surrogate reaching SVG text (surrogate_display_residue).",
  "tim-evaluated-event-stream: restore the __theme marker for unbundled themes (see its review-4.json).",
  "state: route groups through Entity.isAutarkic (cluster vs GroupMakerState image) including nested clusters inside autarkic images; the seven fresh pairs here are the acceptance set. Support the X[H]/X[H*] suffix form.",
  "deployment: keywordless bracket declarations record their Quark code (deploy_unaliased_forms).",
  "stroke: restate the boundary (stereotype <style>/skinparam strokes now follow asColor); per-arrow inline colour through asColor; omit transparent edge paths as SvekEdge does.",
  "harness: run CLI status sweeps at each case's recorded cwd; add a provenance comment for the svg.rs 0.46667 literal."],
 "perturbations": pert(pairs["comment-state-consumers"])})
print("ok")
