"""Write checker-claude-round2 evidence and new review records into each mechanism directory.

Usage: python3 -I records.py <repo>
Shared evidence is copied byte-identically into every mechanism directory that cites it (T14.6.7),
with SHA256s in checker-claude-round2/manifest.json.
"""
import hashlib, json, pathlib, shutil, sys

REPO = pathlib.Path(sys.argv[1])
W = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round2-claude-checker"
AW = pathlib.Path.home() / "Library/Caches/rustuml-relay-parent/shared-svg-round2-r2a"
SUB = "checker-claude-round2"
H = {k: v for k, v in (l.split() for l in """
50f4f5414 50f4f5414f9b879e629a70f6072c13978b990539
9834a7194 9834a719455308120c3289b59a260567f141b724
55c92692d 55c92692dd7b589bde814a6fc34ed246746007c6
04cf5b4d5 04cf5b4d515fe9cbab6fb117fb2f2fa013c37ecc
a15b09dde a15b09ddefd87a173a664ccf3ca29c7b7cf2857c
1537ada52 1537ada52cf04c5f6b748415ce2a1ca54296bac7
441452686 4414526868d903ae3bbb8a0ad0dddf0526837d46
61f3883ff 61f3883ffb0f41be652951f257fb1eaa57dbf16d
33c7f2d49 33c7f2d49550c1eb8ab483074b31903873522edf
7c9f23f20 7c9f23f2010a783252ba54800d5ab2c2d480fd4f
2546ba755 2546ba7554dc2aca3dba00f49dedb2ae0d9632a0
7dd3e5419 7dd3e541907382648b4c063b1e83e0d0fc24226b
b0df8ed5d b0df8ed5d90c4830982b3ae7e6aee35de7f68f83
8f3c59efd 8f3c59efd0ae629235e633e99c5b61d85296a51f
e01f9a6b9 e01f9a6b98a2794a771285974132667f503bf808
066603e48 066603e48dc06a013d29ebb7667452420544d77d
e86a4635d e86a4635d37763d6f71251bab259fb504b3e955d
ebdfd6e7f ebdfd6e7f93f5a6bb98b7767a8ecd5c9a349f364
1603cc54e 1603cc54ed8d53f75bf52ac40828870af2a764c6
47cc04b28 47cc04b28cdd9d16c429b35e0ec2a03a0a29fa24
6c52f2cfd 6c52f2cfd073be4c904a88db283ff2694cb0b8f0
d539f9a08 d539f9a08aab7ab7c2b4f085b8703733f25f1999
b62d8ce96 b62d8ce967b6dda7d85bb6dc28ff92509f6066f9
7bb210e9f 7bb210e9f7578705638c8f3b7c3b43d8b4b906c3
""".strip().splitlines())}
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"
JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
REVIEWER = {"task_id": "t3-9f1c28b6-independent-shared-svg-round2-checker-claude",
            "model": "claude-opus-5-5 / Claude Code in T3 Code; no children; did not implement candidate; no Java run locally"}
sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
pairs = json.load(open(W / "work/pairs.json"))
ev_bt = json.load(open(W / "gates/events-base-tip.json")); ev_pt = json.load(open(W / "gates/events-prev-tip.json"))
cohort = {b: json.load(open(W / f"work/batch-{b}.json")) for b in ("AW", "AU", "AR")}

def cohort_summary(b):
    rows = cohort[b]
    out = {"cases": len(rows)}
    for t in ("base", "prev", "tip"):
        out[t] = {"strict_match": sum(r[f"{t}_strict"] == "MATCH" for r in rows.values()),
                  "comment_list_exact": sum(r[f"{t}_comments_exact"] for r in rows.values()),
                  "nonzero_exit": sorted(n for n, r in rows.items() if r[f"{t}_exit"] != 0),
                  "panics": sorted(n for n, r in rows.items() if r[f"{t}_panic"])}
    out["java_exit_nonzero"] = sorted(n for n, r in rows.items() if r["java_exit"] != 0)
    out["java_crash_svg_exit0"] = sorted(n for n, r in rows.items() if r["java_exit"] == 0 and r["java_crash"])
    out["tip_failures_on_java_valid"] = sorted(n for n, r in rows.items() if not r["java_error_marker"] and r["tip_strict"] != "MATCH")
    out["tip_new_errors_on_java_valid"] = sorted(n for n, r in rows.items() if not r["java_error_marker"] and r["tip_exit"] != 0 and r["base_exit"] == 0)
    out["per_case"] = {n: {k: r[k] for k in ("java_exit", "java_crash", "base_exit", "base_strict", "base_ndiff", "base_comments_exact",
                                             "prev_exit", "prev_strict", "prev_ndiff", "prev_comments_exact",
                                             "tip_exit", "tip_strict", "tip_ndiff", "tip_comments_exact")} for n, r in rows.items()}
    return out

measurements = {
    "schema_version": 1,
    "scope": "independent checker measurements for maker round 2 (claude), tip 6c52f2cfd, versus merged base 50f4f5414 (code = 9834a7194) and previous maker tip 55c92692d",
    "identities": {"base": H["50f4f5414"], "base_code_identical_to": H["9834a7194"], "previous_maker_tip": H["55c92692d"],
                   "maker_merge": H["04cf5b4d5"], "tip": H["6c52f2cfd"], "golden_submodule": "9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a",
                   "java_revision": JAVA, "jar_sha256": JAR,
                   "binaries": "git archive <rev> | tar -x into ~/Library/Caches/rustuml-shared-svg-round2-claude-checker/src/<rev>; CARGO_TARGET_DIR=<private>/target/<rev> cargo build --release -j 6 -p rustuml",
                   "comparator": "rustuml_oracle::compare::compare_svg_strict from the 50f4f5414 export (crates/rustuml-oracle/src unchanged 9834a7194..6c52f2cfd; GEOM_EPS 0.02) via tools/checker-cmp-main.rs"},
    "cli_whole_output_sweep": {
        "sources": 16402, "goldens": 12550, "perturbations": 3852,
        "cwd": "each case from its recorded cwd: tools/cwd_map.py scans every docs/parity-reviews record naming a source with a cwd ('.'/worktree root -> repo root, private dirs -> source dir); unrecorded -> source dir. Independently derived; agrees with the maker's recorded-cwd-map.json on all 853 mapped repo sources (one maker entry names a stale private cwd that both resolve to the source dir).",
        "env": "RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100, PLANTUML_STDLIB=~/work/github.com/plantuml/plantuml-stdlib/stdlib, -pipe -tsvg, 30 s timeout",
        "exit_status_counts": {"50f4f5414": {"0": 16230, "1": 164, "101": 8}, "55c92692d": {"0": 16162, "1": 236, "101": 4},
                               "6c52f2cfd": {"0": 16162, "1": 236, "101": 4}},
        "timeouts": {"50f4f5414": 0, "55c92692d": 0, "6c52f2cfd": 0},
        "base_to_tip": {"valid_strict_matches": "12903 -> 13453", "gained": len(ev_bt["events"].get("gained_match", [])),
                        "lost_matches": ev_bt["events"].get("lost_match", []),
                        "lost_get_provenance_documented": ev_bt["events"].get("lost_match_GET_provenance", []),
                        "new_valid_input_errors": ev_bt["events"].get("new_valid_input_error", []),
                        "lost_rejections": ev_bt["events"].get("lost_rejection", []),
                        "new_panics": ev_bt["events"].get("new_panic", []), "panics_removed": ev_bt["events"].get("panic_removed", []),
                        "new_timeouts": [], "rejections_gained": len(ev_bt["events"].get("rejection_gained", [])),
                        "base_error_now_renders_valid_reference": ev_bt["events"].get("error_now_renders_valid_ref", []),
                        "outputs_changed": len(ev_bt["events"].get("outputs_changed", [])),
                        "comment_lists_exact": "14308 -> 15014", "comment_lists_gained": len(ev_bt["comments"].get("gained", [])),
                        "comment_lists_lost": ev_bt["comments"].get("lost", [])},
        "previous_tip_to_tip": {"valid_strict_matches": "13433 -> 13453", "gained": len(ev_pt["events"].get("gained_match", [])),
                                "lost_matches": ev_pt["events"].get("lost_match", []), "new_valid_input_errors": ev_pt["events"].get("new_valid_input_error", []),
                                "lost_rejections": ev_pt["events"].get("lost_rejection", []), "new_panics": ev_pt["events"].get("new_panic", []),
                                "rejections_gained": ev_pt["events"].get("rejection_gained", []),
                                "previous_tip_error_now_renders_valid_reference": ev_pt["events"].get("error_now_renders_valid_ref", []),
                                "outputs_changed": len(ev_pt["events"].get("outputs_changed", [])),
                                "comment_lists_exact": "15003 -> 15014", "comment_lists_gained": len(ev_pt["comments"].get("gained", [])),
                                "comment_lists_lost": ev_pt["comments"].get("lost", [])},
        "inherited_panics_in_all_three_trees": ["test-diagrams/perturbations/shared-svg-corrective/checker-independent-astra/dash_01.puml",
            "test-diagrams/perturbations/shared-svg-corrective/checker-independent-astra/dash_03.puml",
            "test-diagrams/perturbations/shared-svg-corrective/checker-independent-astra/dash_06.puml",
            "test-diagrams/perturbations/shared-svg-corrective/checker-independent-astra/dash_07.puml"],
        "inherited_panic_site": "crates/rustuml-parser/src/parse/class.rs:285:57 index out of bounds (base, previous tip and tip)",
        "get_provenance_note": "the three losses are the documented historical GET-provenance goldens (owner raw-input contract b62d8ce96, docs/reference-bootstrap.md); they are reported separately and are not raw-reader regressions",
        "full_event_lists": "comment-state-consumers/checker-claude-round2/cli-events.base-to-tip.json and cli-events.previous-tip-to-tip.json"},
    "cohorts": {"AW": cohort_summary("AW"), "AU": cohort_summary("AU"), "AR": cohort_summary("AR")},
    "gates_run_once_each": {
        "golden_no_oracle": {"base_50f4f5414": "11181/11251 (ratchet fails on unlocked progress: class 1962->1963, preprocessing 284->285, rendering 91->95)",
                             "tip": "11179/11251 (ratchet fails: activity 1299->1297, edge-cases 760->759)",
                             "failure_names_vs_9834a7194_code": {"newly_failing": ["activity/act_skinparam2_shadowing_true", "activity/act_skinparam_shadowing", "edge-cases/edge_whitespace_multiple_blank_lines"],
                                                                 "newly_passing": ["preprocessing/preproc_complex_theme_override"]},
                             "preproc_complex_theme_override": "gain genuine: the base differs only in linearGradient ids; tip emits Java's g5uh6twufn3b00/01 (SVG resource seed over the expanded source, TIM evaluated-event-stream lineage); previous tip and tip are byte-identical; cerulean is a shipped theme, so the round-2 marker change does not reach it"},
        "golden_pairs": {"base": "12550 total, 10497 passed, 754 failed, 1299 skipped", "tip": "12550 total, 10497 passed, 754 failed, 1299 skipped",
                         "failure_names_identical": True, "failure_names_sha256": "b92f592c7910f5fa5aaed1d6ddc6df7b7e2e7d79d1404c7e242091b06659ae1b"},
        "parity_perturbations": {"base_sources": 3008, "base_failing": 1365, "tip_sources": 3852, "tip_failing": 1608,
                                 "pre_existing_sources_newly_failing": 0, "pre_existing_sources_newly_passing": 111,
                                 "added_sources": 844, "added_sources_failing": 354,
                                 "added_failing_in_pre_existing_directories": "9, all added by 7bb210e9f (typed-TIM rejection record, lineage): tim-explicit-date-instant/checker3_* (3) and tim-parser-error-propagation/checker3_* (6)",
                                 "note": "measured before this review registered its own AW pairs"},
        "harness_guards": {"base": "4/6 (decimal_literal_ratchet, parity_review_records_are_complete fail)", "tip": "4/6 (same two)",
                           "decimal_literal_ratchet_tip_vs_base": "identical except crates/rustuml-render/src/svg.rs 0 -> 1 (the 0.46667 unit-test literal, provenance comment SvgGraphics.fillMe lines 894-895)",
                           "review_record_categories_tip_before_this_review": {"latest accepted review cannot contain failing perturbations": 1235, "perturbation is not named in any checker review": 306,
                               "latest checker review must be accepted": 198, "source must be a .puml under": 98, "golden must be the source's same-stem .svg under": 98,
                               "referenced source escapes the mechanism directory": 98, "referenced golden escapes the mechanism directory": 98,
                               "checker source is outside the executable perturbation walk": 98, "account.json missing (No such file)": 15},
                           "review_record_categories_base": {"latest accepted review cannot contain failing perturbations": 1012, "perturbation is not named in any checker review": 198,
                               "latest checker review must be accepted": 151, "T14.6.7 escape / same-stem / .puml-under / outside-walk": "98 each", "account.json missing (No such file)": 13},
                           "account_json_missing_added_by_lineage": ["shared-svg-corrective", "tim-corrective-execution-maker"]},
        "pipe_mode": {"base": "13/13", "tip": "17/17 (14-entry strict-XML malformed list unchanged since 55c92692d)"},
        "workspace_lib": {"base": "1468 passed, 0 failed", "tip": "1561 passed, 0 failed"}},
    "frozen_items": "unchanged by every round-2 maker commit (a15b09dde..6c52f2cfd, merge excluded): crates/rustuml-oracle/src (GEOM_EPS), test-diagrams/no_oracle_baseline.txt, test-diagrams/ulp_parked.txt, golden_has_syntax_error markers, error_golden_census_is_frozen (passes), harness_guards allow-lists, pipe_mode EXPECTED_MALFORMED (14 entries, unchanged 55c92692d..6c52f2cfd). No round-2 maker commit touches test-diagrams/.",
}

COMMANDS = [
    "git switch -c review/shared-svg-round2-claude 6c52f2cfd; git -c protocol.file.allow=always submodule update --init --reference <main checkout>/test-diagrams/golden test-diagrams/golden  (-> 9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a)",
    "for r in 50f4f5414 55c92692d 6c52f2cfd: git archive $r | tar -x -C <private>/src/$r; CARGO_TARGET_DIR=<private>/target/$r cargo build --release -j 6 -p rustuml",
    "cargo build --release (tools/checker-cmp-main.rs against the 50f4f5414 rustuml-oracle crate; compare_svg_strict, unchanged)",
    "python3 -I tools/batch.py <work> ~/Library/Caches/rustuml-relay-parent/shared-svg-round2-r2a AW base=rustuml-50f4f5414 prev=rustuml-55c92692d tip=rustuml-6c52f2cfd  (also AU = shared-svg-checker-c1 with its resource, AR = shared-svg-claude-batch1)",
    "python3 -I tools/cwd_map.py <repo> cwd_map.json; python3 -I tools/sweep.py <bin> <out> <repo> cwd_map.json test-diagrams/golden test-diagrams/perturbations  (each of the three trees)",
    "checker-cmp over 3 x 16402 pairs; python3 -I tools/classify.py <work> <repo> 50f4f5414 6c52f2cfd and 55c92692d 6c52f2cfd",
    "PLANTUML_STDLIB=... CARGO_TARGET_DIR=<private> cargo test --release -j 6 --workspace --lib; -p rustuml --test pipe_mode; -p rustuml-oracle --test golden_no_oracle|golden_pairs|parity_perturbations|harness_guards -- --nocapture  (tip worktree and the 50f4f5414 export with the golden submodule linked)",
    "python3 -I tools/register.py <repo> pairs.json  (Java-valid AW pairs registered under test-diagrams/perturbations/<mechanism>/checker-claude-round2, rerun from that cwd)",
    "python3 -I tools/intval_diff.py <bin> 400 (seeded 20261105); Rust-only probes: tools/geomcheck.py over probes/state, probes/sur_*, probes/theme",
]

def copy(src, dst):
    dst.parent.mkdir(parents=True, exist_ok=True); shutil.copyfile(src, dst)

MECHS = ["tim-evaluated-event-stream", "tim-intval-conversion", "tim-builtin-value-boundaries", "comment-state-consumers"]
TOOLS = ["batch.py", "sweep.py", "classify.py", "cwd_map.py", "register.py", "records.py", "intval_diff.py", "geomcheck.py", "review_bodies.py"]
for mech in MECHS:
    E = REPO / "docs/parity-reviews" / mech / SUB
    if E.exists(): shutil.rmtree(E)
    E.mkdir(parents=True)
    for f in ("reference-manifest.json", "reference-results.json.gz", "reference-transport.json", "parent-relay-validation.json",
              "reference-input-bundle.json.gz"):
        copy(AW / f, E / "references" / f"aw-{f}")
    for t in TOOLS: copy(W / "tools" / t, E / "tools" / t)
    copy(W / "cmp/src/main.rs", E / "tools/checker-cmp-main.rs"); copy(W / "cmp/Cargo.toml", E / "tools/checker-cmp-Cargo.toml")
    (E / "measurements.json").write_text(json.dumps(measurements, indent=1, ensure_ascii=False) + "\n")
    (E / "pairs.json").write_text(json.dumps(pairs[mech], indent=1, ensure_ascii=False) + "\n")

E = lambda m: REPO / "docs/parity-reviews" / m / SUB
copy(W / "gates/theme-probes.json", E("tim-evaluated-event-stream") / "theme-probes.json")
copy(W / "gates/intval-differential.tip.txt", E("tim-intval-conversion") / "intval-differential.tip.txt")
copy(W / "gates/intval-differential.base.txt", E("tim-intval-conversion") / "intval-differential.base.txt")
copy(W / "gates/surrogate-probes.json", E("tim-builtin-value-boundaries") / "surrogate-probes.json")
cs = E("comment-state-consumers")
copy(W / "gates/state-probes.json", cs / "state-probes.json")
copy(W / "gates/events-base-tip.json", cs / "cli-events.base-to-tip.json")
copy(W / "gates/events-prev-tip.json", cs / "cli-events.previous-tip-to-tip.json")
for f in ("base.no_oracle_failure_names.txt", "tip.no_oracle_failure_names.txt", "tip.golden_failure_names.txt", "base.pert_fail.txt",
          "tip.pert_fail.txt", "base.guard_cats.txt", "tip.guard_cats.txt", "added_failing_in_preexisting_dirs.txt"):
    copy(W / "gates" / f, cs / "gates" / f)

PERT = lambda mech: [{k: r[k] for k in r if k not in ("java_comments", "tip_comments", "registered", "name")}
                     for r in pairs[mech] if r["registered"]]

def write_review(mech, number, body):
    review = {"schema_version": 1, "mechanism": mech, "reviewer": REVIEWER, "java_revision": JAVA, "jar_sha256": JAR,
              "base_revision": H["50f4f5414"], "base_code_identical_to": H["9834a7194"], "previous_maker_tip": H["55c92692d"],
              "tested_revision": H["6c52f2cfd"], "maker_merge": H["04cf5b4d5"], "commands": COMMANDS,
              "no_production_changes": True, "no_shipping": True}
    review.update(body)
    review["perturbations"] = PERT(mech)
    review["evidence"] = f"docs/parity-reviews/{mech}/{SUB}/ (measurements.json, pairs.json, references/aw-*, tools/; manifest.json lists SHA256s)"
    path = REPO / "docs/parity-reviews" / mech / f"review-{number}.json"
    assert not path.exists(), path
    path.write_text(json.dumps(review, indent=1, ensure_ascii=False) + "\n")
    files = sorted(p for p in E(mech).rglob("*") if p.is_file() and p.name != "manifest.json")
    (E(mech) / "manifest.json").write_text(json.dumps({"schema_version": 1, "files": {str(p.relative_to(E(mech))): sha(p) for p in files},
        "shared_byte_identical_across": MECHS,
        "aw_relay_hashes": {"reference-results.json": "ca51ecb2cfffccfa2f9c9de8822c686844f7d0982f48f16edc1198f1103eeed7",
                            "reference-results.json.gz": "1d983971016d2eb534b150289457c3fdbcb813becb764c2980188a40d77ca651"}}, indent=1) + "\n")
    print("wrote", path)

exec(open(W / "tools/review_bodies.py").read())
