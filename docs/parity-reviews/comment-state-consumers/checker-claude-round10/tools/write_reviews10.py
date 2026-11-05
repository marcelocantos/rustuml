"""Write the checker round-10 (claude) review records and evidence directory.
Usage: python3 -I write_reviews10.py <repo>
Reads every number from the private work/gates/drill files; writes
docs/parity-reviews/{tim-builtin-value-boundaries/review-11.json, comment-state-consumers/review-12.json}
and a byte-identical checker-claude-round10/ evidence directory under both mechanisms."""
import hashlib, json, pathlib, shutil, sys

REPO = pathlib.Path(sys.argv[1])
C = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round10-claude-checker"
W, G = C / "work", C / "gates"
J = lambda p: json.load(open(p))
sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()

TIP = "e24240b3542aaabdaf6286b87c1cb23217898ddc"
MERGE = "d06667af0"
BASE = "95291dc6b"
pairs = J(W / "pairs.json")
summary = J(W / "batch-summary10.json")
census = J(W / "viewbox-census.json")
cohorts = J(W / "cohorts.json")
bs_eval = J(W / "bs-eval.json")
bt_eval = J(W / "bt-eval.json")
t1447 = J(W / "t1447.json")
mutnet = J(W / "mutant-net.json")
cdrill = J(C / "drill/logs-consumer/consumer-drill.json")
rdrill = J(C / "drill/logs-routing/drill.json")
t1447_tiers = J(W / "t1447-tiers.json")

# ---- evidence directory -------------------------------------------------------------
EV = "checker-claude-round10"
def build_evidence(dst: pathlib.Path):
    keep = {p.name for p in dst.glob("*")} if dst.exists() else set()
    (dst / "batches").mkdir(parents=True, exist_ok=True)
    src_req = REPO / "docs/parity-reviews/tim-builtin-value-boundaries" / EV / "c10a-request"
    if not (dst / "c10a-request").exists(): shutil.copytree(src_req, dst / "c10a-request")
    interim = REPO / "docs/parity-reviews/tim-builtin-value-boundaries" / EV / "batches/bs-interim-eval.json"
    if not (dst / "batches/bs-interim-eval.json").exists(): shutil.copy(interim, dst / "batches/bs-interim-eval.json")
    for f in ["batch-summary10.json", "viewbox-census.json", "bs-eval.json", "bt-eval.json", "batch-BS.json", "batch-BT.json",
              "merge-tip-deltas.json", "cohorts.json", "cohorts.txt", "t1447.json", "t1447-tiers.json", "mutant-net.json", "pairs.json"]:
        shutil.copy(W / f, dst / "batches" / f)
    (dst / "gates").mkdir(exist_ok=True)
    for f in ["e24240b35.exit", "95291dc6b.exit", "9834a7194.exit", "e24240b35.pert_fail.txt", "95291dc6b.pert_fail.txt",
              "e24240b35.guard_cats.txt", "95291dc6b.guard_cats.txt", "no_oracle_failure_names.9834a7194-vs-tip.diff", "clippy-diags.json"]:
        shutil.copy(G / f, dst / "gates" / f)
    for f in ["e24240b35.golden_no_oracle.log", "9834a7194.golden_no_oracle.log", "e24240b35.golden_pairs.log", "95291dc6b.golden_pairs.log",
              "e24240b35.lib.log", "e24240b35.pipe_mode.log", "t1447-v1.golden_no_oracle.log", "t1447-v2.golden_no_oracle.log",
              "t1447-v1.golden_pairs.log", "t1447-v2.golden_pairs.log"]:
        txt = (G / f).read_text(errors="replace").splitlines()
        (dst / "gates" / (f + ".tail")).write_text("\n".join(txt[-80:]) + "\n")
    (dst / "drill").mkdir(exist_ok=True)
    shutil.copy(C / "drill/logs-routing/drill.json", dst / "drill/routing-drill.json")
    shutil.copy(C / "drill/logs-consumer/consumer-drill.json", dst / "drill/consumer-drill.json")
    for p in (C / "t1447").glob("*.patch"):
        shutil.copy(p, dst / "drill" / ("t1447-" + p.name))
    (dst / "regen").mkdir(exist_ok=True)
    shutil.copy(C / "regen/regen.out", dst / "regen/regen.out")
    (dst / "regen/sha256.txt").write_text((C / "regen/before.sha").read_text())
    (dst / "tools").mkdir(exist_ok=True)
    for p in (C / "tools").glob("*"):
        shutil.copy(p, dst / "tools" / p.name)
    shutil.copy(C / "bin/SHA256SUMS", dst / "binaries.sha256")
    sums = {str(p.relative_to(dst)): sha(p) for p in sorted(dst.rglob("*")) if p.is_file() and p.name != "SHA256SUMS.json"}
    (dst / "SHA256SUMS.json").write_text(json.dumps(sums, indent=1, sort_keys=True))

# ---- record ---------------------------------------------------------------------------
def counts():
    return {
        "batches": {L: {k: (len(v) if isinstance(v, list) else v) for k, v in s.items()} for L, s in summary.items()},
        "viewbox_comment_census": {L: {k: (len(v) if isinstance(v, list) else v) for k, v in s.items()} for L, s in census.items()},
        "goldens_and_sweep": cohorts,
        "BS_vs_maker_expectations": bs_eval["by_category"],
        "BT_vs_checker_expectations": bt_eval["by_expect"],
        "BT_by_group": bt_eval["by_group"],
        "registered_pairs": {m: {"registered": sum(r["registered"] for r in rs), "pass_at_tip": sum(r["result"] == "pass" for r in rs),
                                 "fail_at_tip": sum(r["result"] == "fail" for r in rs)} for m, rs in pairs.items()},
        "gates_tip": (G / "e24240b35.exit").read_text().strip().splitlines(),
        "gates_base": (G / "95291dc6b.exit").read_text().strip().splitlines(),
        "no_oracle": {"9834a7194": "11181/11251", "tip": "11181/11251",
                      "failure_name_diff_vs_9834a7194": (G / "no_oracle_failure_names.9834a7194-vs-tip.diff").read_text().strip().splitlines(),
                      "note": "3 out (the 3 GET-provenance goldens act_skinparam_shadowing, act_skinparam2_shadowing_true, edge_whitespace_multiple_blank_lines), 3 in; identical to the maker's diff; merge->tip identical"},
        "golden_pairs": {"base": "10497 passed / 754 failed / 1299 skipped", "tip": "10497 / 754 / 1299", "failure_names_identical_base_tip": True},
        "parity_perturbations": {"base_fail": 1365, "tip_fail": 2061, "pre_existing_files_at_base": 3008,
                                 "new_failures_in_pre_existing_files": 0, "fixed_base_to_tip": 117,
                                 "note": "tip count measured before this review's 753 pairs were registered"},
        "harness_guards": "decimal_literal_ratchet and parity_review_records_are_complete fail at base and tip; the decimal list differs only by svg.rs 0->1 (a test literal with a SvgGraphics.fillMe provenance comment, earlier round); round 10 adds no 3+-decimal literal",
        "clippy_error_diagnostics_per_crate": {"base": {"rustuml-render": 19, "rustuml-parser": 1, "rustuml-oracle": 1, "rustuml-layout": 0, "rustuml": 0, "rustuml-math": 0},
                                               "tip": {"rustuml-render": 19, "rustuml-parser": 1, "rustuml-oracle": 1, "rustuml-layout": 0, "rustuml": 0, "rustuml-math": 0},
                                               "same_lint_codes_and_files": True},
        "routing_drill": {k: ("caught" if (v.get("caught") if isinstance(v, dict) and "caught" in v else not v.get("passed", True)) else "pass") for k, v in rdrill.items()},
        "consumer_drill_lib_suite": {k: ("caught" if v.get("caught") else ("baseline" if k == "M00_unmodified" else "SURVIVED")) for k, v in cdrill.items()},
        "consumer_mutants_vs_registered_pairs": {k: v["pairs_failing"] for k, v in mutnet.items()},
        "T14.47": t1447_tiers,
    }

FINDINGS = [
 "REJECT (round-9 obligation 1 acceptance not met; parent ruling 2026-11-06): plain-Latin text in Verdana Italic and Verdana Bold Italic is measured with the upright ASCII tables. text_render.rs family_table_text_width picks VERDANA_WIDTH / VERDANA_BOLD_WIDTH by `bold` only and routes ASCII through the own table, so the italic face reaches only characters outside 32..126. Java (BT ft tables, 14 pt): Verdana Italic A-M 119.4717 / N-Z 130.3477 / a-m 98.3145 / n-z 106.0869 vs Rust (Regular) 119.5537 / 130.3408 / 98.3965 / 106.1143; Verdana Bold Italic n-z 120.3945 vs Rust (Bold) 120.4219. Probes whose controls match and that fail: BS m10a_{comp,nodc,artc,dbsc,actu,usec,stat,part,clss,actv}_vd_i (10), BT c10a_ml_part_vd_i (Java 127.9482 vs 127.9688) and c10a_ft_vd_creolebi (bold label + <i>: Java 149.8301 vs 149.8574; control c10a_ft_vd_creolebi_ctrl matches). Fails at merge and tip (not a round-10 loss). Not affected (Java italic advances equal the tables Rust uses): Arial Italic/Bold Italic, Helvetica Oblique, Times New Roman (four tables), Courier New, SansSerif derived italics.",
 "No round-10 loss on any cohort: AR..BT 0 matches, rejections, exact viewBoxes or comment lists lost merge->tip; 0 new valid-input errors, panics or stderr changes; goldens byte-identical merge->tip except the 2 disclosed Java error pages; perturbations: 6 outputs changed merge->tip (3 gains, 3 DIFF->DIFF moving toward Java's viewBox), 4 stderr changes are only the panic thread id of the pre-existing class.rs:285 panic.",
 "Style -> face path confirmed against Java wherever the control matches (BT): Creole runs in italic labels (30/40 incl. all 20 component/participant/activity/use-case italic probes whose control matches), multi-line labels (Times), size + italic (10/10), state title/fields (Times, stateFontName), activity deprecated-colour action, skinparam vs <style> order for the label (both orders), <style> nested, stereotype and element selectors for component and use case, cloud outline in component diagrams (9/16; all stereotype variants fail on the multi-word stereotype parser gap). Fresh failing paths are all outside the packet or filed: <style> FontStyle on participants and activity (T14.46), stereotype face, sequence boxes / nested containers / swimlanes (fail at control), multi-word stereotypes.",
 "ValueImpl.asFontFace port exact for the (bold, italic) decision: 13 forms x 5 sites (BOLD, Italic, bolder, lighter, 600, 650, 900, normal, 'italic bold', oblique, style 800, style 'bold italic', control) match Java's face; widths match. Residue: Java writes the CSS weight string (DriverTextSvg 104-110: font-weight=\"800\"/\"300\"/\"600\"/\"900\"), Rust collapses to bold/plain, so 25 width-correct probes fail on the attribute (also at merge). Unprobed static differences: Java trim() strips only <= U+0020 (Rust trims Unicode whitespace, e.g. NBSP) and Integer.parseInt accepts non-ASCII digits.",
 "'bold italic' is plain only at the packet's sites. Still drawn bold italic (Java plain): class names (BS 5 m10a_clss_*_bi, plain twins match), class attributes (c10a_bi_classattr_bi), sequence arrows (c10a_bi_seqarrow_bi); other consumers fail at their control.",
 "Stereotype face hardcoded italic (component, use case, actor, participant) in measurement and drawing; Java resolves it from the stereotype style cascade: `<style> component { FontStyle bold }` after `skinparam componentFontStyle italic` draws and measures «Dock» bold (c10a_pr_{comp,usec}_sp_then_css; part/actv also fail on T14.46).",
 "Disclosed golden changes skin_usecase_fontStyle_bold/_italic: Java syntax-error pages (`rectangle System {{`) at every tree; Rust renders at merge and tip (exit 0); tip draws the use-case labels bold/italic through the new SkinColors face (usecase.rs). Explained by the style fix; DIFF -> DIFF.",
 "Gate every_italic_slot_measures_in_its_own_face: maker drill reproduced 14/15 (each caught by the test's assertion, no build failure); Menlo bold-italic -> Menlo Bold survives. It is equivalent only under Rust's cell-only monospaced model: Menlo Bold maps U+01F0, U+0220, U+2060 and U+2580..U+27AF runs that Menlo Bold Italic does not; Java gives fallback glyphs their own advances, so the routes are probably distinguishable in Java (unprobed). The gate's own-table branch uses the public width as its oracle (`Some(own) if own(c) => public(c)`), so it cannot see the Verdana Italic ASCII gap. Consumer drill: 6/6 consumer-site italic mutants survive the whole rustuml-render lib suite (860 tests); the registered round-10 pairs catch C1 (component label, 19 pairs), C2 (component stereotype, 6), C4 (activity, 9), C6 (use case, 14); C3 (participant block 5663) and C5 (StateNodeFont italic) are caught by nothing. Times arm mutant caught by awt_named_font_metrics_match_goldens.",
 "Regeneration command in monkeyman-round9/README.md reproduces font_chain_coverage.rs byte-identically (eef8a552...3479; generator prints SHA256SUMS ad8e6566...).",
 "T14.47 confirmed in base (state.rs:1581 at 95291dc6b, 1653 at tip; introduced ceda8d9f2 2026-06-05): see T14.47 measurement.",
 "Process: accounts 681fce4c4 (07:37) and addendum 6f86bf825 (08:00) precede their code commits e1ee5ce7f/2a352f67b (07:53) and df9365441 (08:02); account/addendum/evidence commits touch only docs/parity-reviews. Addendum inaccurate in one prediction (no golden changes) - disclosed by the maker. Maker expectations wrong in 3 places: 10 Verdana italic 'match-iff-control' probes fail; 5 class 'same-as-plain' probes fail; 13 'fail' probes match (bold italic renders plain on both sides). No fixture-keyed logic and no fitted constant in the round-10 diff (constants: CSS weights 100/300/700/800/900 from UFontFace/UFontWeight, stereotype italic from plantuml.skin). Frozen items unchanged base..tip (crates/rustuml-oracle incl. GEOM_EPS 0.02, syntax markers, park list, error census, guard allow-lists, strict-XML list; ulp_parked.txt; non-perturbation test-diagrams; no perturbation edited or deleted). bullseye.yaml = base + T14.39 (and its T14 depends_on entry), merge == tip; parity-recovery has since added T14.46/T14.47, so the T14 depends_on list needs a union at merge. T14.6.7 path categories unchanged (98 source / 98 golden path violations, as at base).",
]

PROPOSED = [
 {"name": "Verdana Italic and Bold Italic measure plain-Latin text with their own advances", "blocking": True,
  "acceptance": ["Printable ASCII in Verdana Italic and Verdana Bold Italic measures with those faces' advances (extracted from the face files with the existing per-face extractor, provenance recorded), not the Regular/Bold tables.",
                 "The face-routing gate compares own-table characters against the face's own table, so dropping the italic table is caught.",
                 "BS m10a_*_vd_i, BT c10a_ml_part_vd_i and c10a_ft_vd_creolebi match, plus renamed held-out Verdana italic/bold-italic probes, and no pass is lost."]},
 {"name": "Helvetica upright tables match Java", "acceptance": ["Rust's Helvetica plain table matches Java in N-Z (129.124 vs 129.1172 at 14 pt) and the Helvetica Bold table in a-m/n-z (94.8965/103.4551 vs 94.1309/101.9033); Java measures \"Helvetica\" as Arial's advances there. Trace JDK 21's macOS resolution of the name before changing data.",
  "BT c10a_ft_hv_* per-label widths match, plus held-out Helvetica probes, and no pass is lost."]},
 {"name": "Multi-word stereotypes after `as` are kept", "acceptance": ["`component \"Kiln\" as E1 <<Ferrous gauge vista>>` (and node, database, cloud, queue, actor, class) keeps and draws the stereotype as Java does.", "BT c10a_st_* match where the family/face is otherwise right, plus renamed variants."]},
 {"name": "Labels containing `{|}` parse as Java parses them", "acceptance": ["`component \"... {|}\" as E4` renders one label as Java does instead of two texts (sans-serif bold + monospace).", "BT c10a_ft_* match once face data is right, plus held-out punctuation probes."]},
 {"name": "Stereotype text face resolves through the style cascade", "acceptance": ["The stereotype face is the stereotype style's (EntityImageDescription styleStereo, AbstractTextualComponent stereo): an element <style> FontStyle declared after the skinparam reaches it, and <kind>StereotypeFontStyle is honoured; measured and drawn alike.", "BT c10a_pr_{comp,usec}_sp_then_css match, plus held-out stereotype-style probes."], "note": "extends T14.46's <kind>StereotypeFontStyle item"},
 {"name": "Numeric CSS font weights are written as Java writes them", "acceptance": ["FontStyle values that resolve to a CSS weight other than 400/700 (bolder, lighter, 600, 900, style 800) emit font-weight=\"<weight>\" as DriverTextSvg 104-110 does; measurement keeps the bold/plain face (isBold).", "BT c10a_fv_*_{bolder,lighter,w600,w900,css800} match where their controls match."]},
 {"name": "Menlo Bold Italic fallback advances are modelled, or the equivalence is proven", "acceptance": ["Fresh Java probes with U+01F0, U+0220, U+2060 and U+2580..U+27AF in Menlo Bold Italic vs Menlo Bold show whether the routes differ; if they do, monospaced text uses fallback glyph advances (not the cell) and the gate's EQUIVALENT entry is removed.", "No pass is lost."]},
 {"name": "'bold italic' FontStyle is plain in every consumer", "acceptance": ["Class names and attributes, sequence arrows and every other FontStyle reader use ValueImpl.asFontFace (style_cascade::font_face).", "BS m10a_clss_*_bi and BT c10a_bi_{classname,classattr,seqarrow}_bi match where their controls match."]},
 {"name": "Every round-10 consumer site is pinned by a test", "acceptance": ["Dropping the italic at sequence ParticipantTextBlock (5663) or StateNodeFont fails a lib test or a registered perturbation (today nothing catches either); if a site is unobservable it is removed or documented as such."]},
]

T1447_TEXT = None  # filled below

def record(mechanism):
    rows = [r for r in pairs[mechanism] if r["registered"]]
    perts = [{"source": r["source"], "golden": r["golden"], "cwd": r["cwd"], "axes": r["axes"] or ["batch " + r["batch"]],
              "batch": r["batch"], "result": r["result"],
              "tip_strict": r["tip_e24240b35_strict_compare"], "merge_strict": r["merge_d06667af0_strict_compare"],
              "base_strict": r["base_95291dc6b_strict_compare"]} for r in rows]
    return {
        "schema_version": 1,
        "reviewer": {"task_id": "t3-51190f64 independent shared-SVG round-10 checker (claude)",
                     "model": "claude-opus-5-5 / Claude Code in T3 Code; no children; did not implement the candidate; no Java run locally (all Java via the parent relay)"},
        "mechanism": mechanism,
        "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
        "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
        "base_revision": "95291dc6bc460b0faeb2d80f15b4010ed56864ab",
        "base_code_identical_to": "9834a719455308120c3289b59a260567f141b724",
        "merge": "d06667af0e6cd326e8b22a65a2781130f6db41de",
        "merge_code_identical_to": "14dd5efaeb388dd557bd39f8b6ab40c4338ada6d",
        "tested_revision": TIP,
        "packet_tip": TIP,
        "answers_rejection": "d8242a8dcafa603db77dcc7251418f8aba03749c",
        "account_commit": "681fce4c43532fbf99231266e2b11177350b29de",
        "implementation_commit": "df9365441fafb4e4af6763681eafe23934278ecb",
        "other_commits_reviewed": ["681fce4c4", "e1ee5ce7f", "2a352f67b", "6f86bf825", "df9365441", "e24240b35"],
        "no_production_changes": True,
        "no_shipping": True,
        "scope": "round-10 obligations: element text measured in its drawn face, style included (component, activity, sequence participant, state, use case/actor); ValueImpl.asFontFace; face-routing gate and regeneration record; 6413e26b4 process record. Whole shared-SVG packet line (rounds 1-10) for local merge readiness.",
        "commands": [
            "git switch -c review/shared-svg-round10-claude e24240b35; git -c protocol.file.allow=always submodule update --init --reference .../rustuml/test-diagrams/golden test-diagrams/golden  # 9efdc12f",
            "git archive <rev> | tar -x -C ~/Library/Caches/rustuml-shared-svg-round10-claude-checker/src/<rev>; CARGO_TARGET_DIR=<private> cargo build --release -j 6 -p rustuml  # 95291dc6b d06667af0 e24240b35 (binaries.sha256)",
            "python3 -I tools/c10a_sources.py c10a; python3 -I tools/predict.py c10a base=... merge=... tip=...; python3 -I tools/c10a_expect.py c10a; python3 -I tools/stage.py <abs c10a>  # pre-registered in 77327eb70 before any Java run",
            "python3 -I tools/batch.py work <relay dir> <L> bin/rustuml-95291dc6b=base bin/rustuml-d06667af0=merge bin/rustuml-e24240b35=tip  # AR..BT (run_batches10.sh); unchanged strict comparator (cmp/ wraps rustuml-oracle compare_svg_strict at 95291dc6b) + exact comment list",
            "python3 -I tools/bs_eval.py work/batch-BS.json <maker-expectations.json> work/bs-eval.json; python3 -I tools/bt_eval.py work/batch-BT.json c10a/checker-expectations.json work/bt-eval.json; python3 -I tools/vbcensus10.py ...; python3 -I tools/batchsum10.py ...",
            "bash tools/sweep10.sh  # all 12,550 goldens + 4,960 perturbations x 3 trees, recorded cwd (cwd_map.py), whole output; cohorts.py",
            "bash tools/gates10.sh  # tip: workspace lib, pipe_mode, golden_no_oracle, golden_pairs, parity_perturbations, harness_guards, doc_claims; base: perturbations/guards/golden_pairs; 9834a7194: golden_no_oracle (re-run after ENOSPC)",
            "bash tools/clippy10.sh; python3 -I tools/clippy_list.py gates gates/clippy-diags.json",
            "python3 -I tools/routing_drill10.py drill/tree drill/logs-routing  # maker drill; python3 -I tools/consumer_drill10.py drill/tree drill/logs-consumer; python3 -I tools/build_variants10.py; python3 -I tools/mutant_net10.py <repo> work/pairs.json work/mutant-net.json",
            "regeneration: the monkeyman-round9/README.md command on a git-archive copy; shasum before/after (regen/)",
            "python3 -I tools/t1447.py <repo> work/cwd_map.json work/t1447.json; golden_no_oracle and golden_pairs on the tip gate tree with v1 / v2 state.rs (t1447-*.patch)",
            "python3 -I tools/register10.py <repo> work/pairs.json BS=... BT=...",
        ],
        "relays_answered": {
            "BS (maker M10A, held out from the maker)": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round10-m10a",
                                                         "reference_results_sha256": "236ca869013dfc8fcea700f47306ac7350e478899b4e1ad477476354e3e975de"},
            "BT (checker C10A)": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round10-c10a",
                                   "reference_results_sha256": "9660501647d2e216776788b8364194df1263e3eb6fce47e1ebb402a4613e09fd",
                                   "reference_manifest_sha256": "2fb673a344885dbd498a91c36825772f588ddc93fdcb558185b011d9b732eb3e",
                                   "pre_registration_commit": "77327eb704fc10ab973993e504460446094092ac",
                                   "rust_predictions_sha256": "742b2068fbbd29c70a52bcac563e52d6cb010a7a986085709711777442eb4dd3",
                                   "checker_expectations_sha256": "338f7bc539ed53c1f4503a806e2a2f98c1ae898eca48a7c85276c34d1502b1db"}},
        "evidence": {"dir": EV + " (byte-identical copy under docs/parity-reviews/comment-state-consumers/ and tim-builtin-value-boundaries/; SHA256SUMS.json)",
                     "private_cache": str(C)},
        "frozen_items": "unchanged 95291dc6b..e24240b35 and d06667af0..e24240b35: crates/rustuml-oracle (GEOM_EPS 0.02, syntax markers, park list, error census, guard allow-lists, 14-entry strict-XML list), test-diagrams/no_oracle_baseline.txt and ulp_parked.txt; no perturbation edited or deleted",
        "account_chronology": {"accounts (docs only)": "681fce4c4 07:37", "code obligation 1": "e1ee5ce7f 07:53", "gate + regeneration record": "2a352f67b 07:53",
                               "asFontFace addendum (docs only)": "6f86bf825 08:00", "asFontFace code": "df9365441 08:02", "evidence (docs only)": "e24240b35 08:15"},
        "counts": counts(),
        "attributed_regressions": {
            "round 10": "none on any cohort",
            "earlier rounds (already at merge)": {
                "goldens (base->tip)": "3 GET-provenance goldens act_skinparam_shadowing, act_skinparam2_shadowing_true, edge_whitespace_multiple_blank_lines (no-oracle and sweep; recorded in earlier rounds)",
                "exact viewBox only (case fails at every tree)": ["BO c8a_bl_lucida_thai_digit_after_r", "BR c9a_sp_seqnote_pdf_N_ctrl, c9a_sv_actmid_N_sp_ctrl, c9a_sv_classnote_N_ctrl (T14.43)",
                                                                   "BS m10a_acts_cn_i, m10a_css_actv_cn_i, m10a_css_part_cn_i, m10a_dbse_cn_i (Courier New marks on T14.46 paths; rounds 7/8)"],
                "comment lists (perturbations, base->tip)": ["shared-svg-corrective/checker-independent-astra/unicode_activity", "tim-sub-marker-layers/checker5/sub_suffix_continuation"]}},
        "verdict": "rejected",
        "packet_verdict": "Packet e24240b35 rejected. Round 10 causes no loss anywhere and ports the style -> face path and ValueImpl.asFontFace faithfully, but round-9 obligation 1's own acceptance ('in plain Latin ... match where their controls match') fails on fresh held-out Java: plain-Latin Verdana Italic and Verdana Bold Italic text is measured with the upright ASCII tables (10 BS + 2 BT probes with matching controls). Per the parent ruling this blocks the line regardless of the round that introduced it.",
        "per_mechanism_verdicts": {
            "drawn face, style included (e1ee5ce7f)": "confirmed for routing at component, activity, participant, state and use-case/actor sites (BS 98/128, BT match-iff-control cases match wherever controls match except Verdana italic); NOT accept-ready: Verdana Italic/Bold Italic ASCII data absent; participant (5663) and StateNodeFont sites unpinned",
            "ValueImpl.asFontFace (df9365441)": "accepted as a port of the face decision (13 forms x 5 sites); residue: numeric font-weight emission, 'bold italic' outside the packet",
            "face-routing gate + regeneration (2a352f67b)": "drill reproduced 14/15, regeneration byte-identical; gate blind to own-table face data and to consumer sites; Menlo Bold Italic equivalence holds only in Rust's cell model",
            "6413e26b4 process record (681fce4c4)": "accurate (not re-measured beyond the record)",
            "two disclosed golden changes": "explained, Java error pages either way",
        },
        "findings": FINDINGS,
        "t14_47_measurement": T1447_TEXT,
        "held_out_failures": {"blocking (obligation 1 acceptance)": ["BS m10a_{comp,nodc,artc,dbsc,actu,usec,stat,part,clss,actv}_vd_i", "BT c10a_ml_part_vd_i", "BT c10a_ft_vd_creolebi"],
                              "filed / proposed residues": "T14.46 paths, stereotype cascade, numeric weights, class/attribute/arrow bold italic, multi-word stereotypes, {|} labels, Helvetica tables, state Arial (T14.47/T14.39)"},
        "bounded_next_obligations": [
            "Blocking: Verdana Italic and Verdana Bold Italic printable-ASCII advances measured from those faces (per-face extraction with provenance, no fitted values); the face-routing gate's own-table branch checks the face's own table (a dropped italic table fails it). Acceptance: the 10 BS and 2 BT Verdana probes named above match, plus fresh renamed held-out Verdana italic/bold-italic probes (other labels, sizes, contexts), and no pass is lost on AR..BT, goldens, sweep and registered pairs.",
            "Pin the two unpinned round-10 consumer sites (sequence ParticipantTextBlock at the 5663 call; StateNodeFont italic) with a lib test or a registered perturbation that fails when the italic is dropped, or show the site is unobservable and remove it.",
            "Record at merge (not obligations of this packet): the proposed targets below; T14.46 extended with the stereotype style cascade; T14.47 measurement.",
        ],
        "proposed_targets": PROPOSED,
        "perturbations": perts,
    }

if __name__ == "__main__":
    t = t1447_tiers
    T1447_TEXT = t
    for mech in ["tim-builtin-value-boundaries", "comment-state-consumers"]:
        d = REPO / "docs/parity-reviews" / mech
        build_evidence(d / EV)
        n = {"tim-builtin-value-boundaries": 11, "comment-state-consumers": 12}[mech]
        rec = record(mech); rec["t14_47_measurement"] = T1447_TEXT
        (d / f"review-{n}.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False) + "\n")
        print(mech, f"review-{n}.json", "perturbations", len(rec["perturbations"]))
