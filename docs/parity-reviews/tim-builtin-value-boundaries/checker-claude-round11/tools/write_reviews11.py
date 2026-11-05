"""Write the round-11 checker review records (comment-state-consumers/review-13.json, tim-builtin-value-boundaries/review-12.json)
from the evidence under checker-claude-round11. Usage: python3 -I write_reviews11.py <repo>"""
import json, pathlib, sys
R = pathlib.Path(sys.argv[1]); E = R / "docs/parity-reviews/comment-state-consumers/checker-claude-round11"
J = lambda p: json.load(open(E / p))
pairs = J("batches/pairs.json"); summ = J("batches/batch-summary11.json"); bx = J("batches/bx-eval.json"); bv = J("batches/bv-eval.json")
coh = J("batches/cohorts.json"); units = J("batches/units-relay-all.json"); ug = J("batches/units-goldens.json"); drill = J("drill/drill11c.json")
hm = J("provenance/hmtx-check.json"); mm = J("provenance/mm-ascii-check.json"); f7 = J("batches/f7-batches.json"); ctl = J("batches/bx-controls.json")
ex = J("c11a-request/checker-expectations.json")
import collections
H = collections.defaultdict(collections.Counter)
for r in ex["cases"]:
    p = bx["per_case"][r["name"]]; h = r["hypothesis"]
    H[h]["n"] += 1; H[h]["merge_match"] += p["merge"]["strict"] == "MATCH"; H[h]["tip_match"] += p["tip"]["strict"] == "MATCH"
    H[h]["lost"] += p["merge"]["strict"] == "MATCH" and p["tip"]["strict"] != "MATCH"; H[h]["tip_differs_from_merge"] += r["tip_differs_from_merge"]
hyp = {h: dict(c) for h, c in sorted(H.items())}
batches = {l: {"n": r["cases"], "java_valid": r["java_valid"], "base": r["strict"][0], "merge": r["strict"][1], "tip": r["strict"][2],
               "merge_to_tip": r["merge->tip"], "base_to_tip_lost_match": r["base->tip_lost"], "base_to_tip_viewBox_lost": r["base->tip_vb_lost"],
               "base_to_tip_comments_lost": r["base->tip_comments_lost"], "base_to_tip_new_valid_input_errors": r["base->tip_new_err"],
               "base_to_tip_lost_rejections": r["base->tip_lost_rej"], "panics_tip": r["panics_tip"], "nonempty_stderr_tip": r["stderr_tip"]} for l, r in summ.items()}
sweep = {c: {k: v for k, v in coh[c].items() if not k.endswith("stderr_per_source")} for c in ["goldens", "perturbations", "all"]}
sweep["GET_goldens"] = coh["GET_goldens"]
COMMANDS = [
 "git switch -c review/shared-svg-round11-claude ba15189b2; git -c protocol.file.allow=always submodule update --init --reference .../rustuml/test-diagrams/golden test-diagrams/golden  # 9efdc12f",
 "git archive <rev> | tar -x -C <cache>/src/<rev>; CARGO_TARGET_DIR=<private> cargo build --release -j 6 -p rustuml  # 5ddf7733c 879d166f4 97830bd1e f7c382ff4 (binaries.sha256; targets deleted after copy)",
 "python3 -I tools/hmtx_check.py <tip text_render.rs> <merge text_render.rs> tools/font-chain-reference/ascii-advances-round11 provenance/hmtx-check.json  # fontTools hmtx, independent of the Swift extractor",
 "python3 -I tools/c11a_sources.py c11a; predict.py c11a base=.. merge=.. tip=..; c11a_expect.py; stage.py  # pre-registered in 942870ac0 before any Java run",
 "python3 -I tools/batch.py work <relay dir> <L> bin/rustuml-5ddf7733c=base bin/rustuml-879d166f4=merge bin/rustuml-97830bd1e=tip  # BV, BX and AR..BT (run_batches11.sh); unchanged strict comparator (cmp wraps rustuml-oracle compare_svg_strict at 5ddf7733c, oracle crate identical at tip)",
 "python3 -I tools/eval11.py batch-<L>.json <out> [checker-expectations.json]; controls11.py; units_check.py <tip text_render.rs> <out> '<java svgs>'",
 "bash tools/sweep11.sh  # 12,550 goldens + 5,713 perturbations x {5ddf7733c, 879d166f4, 97830bd1e, f7c382ff4}, recorded cwd (cwd_map.py), whole output; cohorts.py",
 "python3 -I tools/f7_batches.py work f7-batches.json AR=..BX=..  # C3+C5 isolation on every relay batch",
 "bash tools/gates11.sh  # tip ba15189b2 tree: workspace lib, pipe_mode, golden_no_oracle, golden_pairs, parity_perturbations, harness_guards, doc_claims; 9834a7194: golden_no_oracle; 5ddf7733c: perturbations/guards/golden_pairs; 879d166f4 and f7c382ff4: golden_pairs",
 "bash tools/clippy11.sh; python3 -I tools/clippy_list.py gates gates/clippy-diags.json",
 "python3 -I tools/drill11c.py <pristine tip tree> <work tree> maker drill11.json drill/drill11c.json  # maker D0-D8 + checker K1-K9, cargo test -p rustuml-render --lib per mutant",
 "git merge-tree --write-tree 5ddf7733c ba15189b2  # bullseye.yaml union check",
 "python3 -I tools/register11.py <repo> batches/pairs.json BV=<m11a relay>=749bf2c2...=.. BX=<c11a relay>=694d0f73...=..",
]
FINDINGS = [
 "ACCEPT (whole shared-SVG line, rounds 1-11) for local merge into parity-recovery, with the residuals below recorded at merge.",
 "Round-10 blocking obligation 1 is met. The Verdana and Arial printable-ASCII advance tables are the faces' own data: fontTools hmtx of the 8 face files (SHA256 equal to the reference host's monkeyman-round9 font hashes) equals the tip's u16 tables and the maker's TSVs on 95/95 code points per face; the merge tree's upright f64 tables x 2048 equal hmtx exactly; the Monkey Man re-extraction (28 TSVs) is byte-identical to the maker's pre-registered hashes and to the committed files.",
 f"Units x size reproduces Java exactly: {sum(v for k, v in units['stats'].items() if k.endswith('exact'))}/{sum(v for k, v in units['stats'].items() if k.endswith('exact') or k.endswith('miss'))} Verdana/Arial printable-ASCII <text> runs in every relay Java output (23 batches, sizes 9-23) and {sum(ug['stats'].values())}/{sum(ug['stats'].values())} in the goldens; {units['stats'].get('Verdana/0/1/only_own_slot_and_differs_from_upright', 0)} Verdana Italic and {units['stats'].get('Verdana/1/1/only_own_slot_and_differs_from_upright', 0)} Verdana Bold Italic runs match only their own (bold, italic) slot, which confirms the slot selection against Java's face resolution.",
 "Upright output is unchanged: merge -> tip changes 0 of 12,550 golden outputs; every one of the 241 changed outputs (199 relay cases + 42 perturbations) carries a Verdana italic run (or measures one inside creole); Arial, Times, Courier New, Helvetica, SansSerif, Monospaced and Lucida Grande probes are byte-identical merge -> tip.",
 "Gate: the 8 maker mutations reproduce with the same failing tests; checker mutations K1 (swap Verdana italic/bold-italic slots), K2 (swap the Verdana italic and bold-italic rows), K3 (swap the Arial rows), K4 (drop glyph D from the italic const), K5 (drop row D from the TSV), K6 (drop row D and rehash SHA256SUMS plus the pinned hash), K7 (slot arguments swapped) and K8 (chain face drops italic) are all caught. K9 (a consistent forgery of const + TSV + SHA256SUMS + pin) passes: the lib gate pins the committed extraction; face fidelity rests on the extraction record, the Monkey Man relay and this hmtx check (declared residue).",
 "Dead-code removals confirmed: f7c382ff4 (tables only) and the tip are byte-identical in stdout and exit on all 18,263 sweep sources and all 2,637 relay cases; stderr differs only on the 4 shared-svg-corrective dash_* sources (thread id of the pre-existing class.rs panic), exactly as the maker's addendum says. C5: 11 stateFontStyle spellings/forms (case, block, stereotype-keyed, two-word, oblique, attribute, repeated, mono name, size) give byte-identical merge -> tip output in Times and SansSerif; golden_pairs (oracle tier) is identical at base, merge, f7c382ff4 and tip (10,497 / 754 / 1,299; the 746-name park-candidate file is byte-identical).",
 "BX (checker C11A, 515 cases, all Java-valid, exit 0, empty stderr): strict 179 -> 200 -> 237 (base/merge/tip); merge -> tip 0 lost matches, 37 gains, exact viewBox 226 -> 230 (0 lost), comment lists 515/515 at merge and tip, no error, panic or stderr change. BV (maker M11A, 100): 47 -> 50 -> 79; 0 lost, 29 gains; 42 of the maker's 44 'match iff control matches' expectations hold.",
 "Held-out failures with matching controls are attributed, none to the tables: (a) sequence queue participants: Java draws the queue label upright under participantFontStyle italic/bold; Rust draws it italic/bold at base, merge and tip (Times twins fail identically at merge), so the correct italic table exposes the wrong face: BV m11a_kd_queu_vd_{i,bi} and BX c11a_qu_{plain,multi,multist,empty}_vd_i get more differences (2->13 ..), and BV m11a_kd_queu_vd_i loses its exact viewBox (168 -> 167) - an accidental merge-tree agreement, not a lost match; (b) the sequence message layout width (sequence.rs message_label_width) takes bold but never italic at every tree: c11a_fc_smsg_vd_i now has Java's exact textLength 248.2886 but participant B stays 0.3555 right (T14.39 scope); (c) <style> FontWeight bold is not modelled (bold-italic via style fails in every family incl. Times/SansSerif/Lucida); (d) consumers that never draw the skinparam style or family (rectangle, package, object, notes, sequence group/divider, link labels, title, legend, diamond); (e) state text under defaultFontName Arial is measured with a non-Arial metric at every tree (253.0801 vs Java 232.6475, plain included); (f) state names/descriptions draw Creole tags literally while measuring the runs; (g) Helvetica bold (T14.49).",
 "Disclosed non-ASCII residue (T14.55): 13 changed relay cases contain non-ASCII text; none loses a match (3 gain in BR, 10 keep the same difference count); no golden output changes.",
 "Sweep merge -> tip: 18,263 sources (12,550 goldens, 5,713 perturbations); valid strict 14,478 -> 14,496 (+18 perturbations, 0 lost), exits and panics identical, comment lists identical, 4 stderr changes (thread ids). Line base -> tip: goldens 11,181 = 11,181 (lost the 3 GET-provenance goldens recorded in earlier rounds, gained preproc_complex_theme_override, state_history_shallow, state_note_composite), perturbations 2,117 -> 3,315, 2 comment-list losses recorded in round 10.",
 "Gates at the tip tree: workspace lib 1,609/0; pipe_mode 17/17; doc_claims 2/2; golden_no_oracle 11,181/11,251 (name diff vs 9834a7194 as above; the test fails on the recorded GET baseline activity 1299->1297, edge-cases 760->759); golden_pairs identical to base; parity_perturbations 0 newly failing among sources present at base, 117 fixed; harness_guards categories equal to base except: the base decimal-ratchet violation text_render.rs 409->419 is gone (tip 51), svg.rs 0->1 (earlier round, provenance comment) remains, 'No such file' 13->15 (shared-svg-corrective and tim-corrective-execution-maker lack account.json; earlier rounds), review-record categories grow with registered pairs (T14.6). T14.6.7 categories are 98/98/98/98/98 at base and tip (none added by the line). Clippy per crate equals base (render 19, parser 1, oracle 1, others 0; same codes and files).",
 "Review tree (this commit) harness_guards: no new category; 'latest checker review must be accepted' 197 -> 195 and 'latest accepted review cannot contain failing perturbations' 1602 -> 1516 against the packet tip; every registered pair is named ('perturbation is not named' 306 = 306); T14.6.7 categories unchanged (98 each). parity_perturbations failures 2,428 -> 2,727 = exactly the 299 registered BV/BX pairs that fail at tip (316 of 615 pass), as measured from their registered cwd.",
 "Process: accounts precede code (4f959e464 09:29 < f7c382ff4 09:31 < cb3d36bb1 09:35; addendum 0368eee79 09:40 < 97830bd1e 09:41); the addendum's measurement reproduces; the code diff is face data, routing and tests with no fixture-keyed logic; frozen items unchanged (crates/rustuml-oracle, no_oracle_baseline, ulp_parked, decimal baseline, guard allow-lists, strict-XML list; no perturbation edited, deleted or added by the maker); the maker's numbers agree with this review's (sweep +18/42/4, batches, 12 required probes MATCH). bullseye.yaml: merge-tree conflicts only on the regenerated content_hash banner; the bodies union (T14.39 from the line, T14.50-T14.56 from parity-recovery, no ID collision); resolve by keeping both and regenerating the banner with bullseye.",
]
RESIDUALS = [
 "Queue participants take participantFontStyle in Rust (Java: queue style, drawn upright): ndiff worse on 6 relay cases and 1 exact viewBox lost (BV m11a_kd_queu_vd_i); pre-existing face routing exposed by correct italic data (proposed target P1).",
 "Sequence message layout width ignores italic (T14.39 extension P2).",
 "<style> FontWeight bold, style/family routing in non-packet consumers (T14.46 extension P5), state Arial metric (T14.47 extension P3), state Creole drawing (P4), Helvetica (T14.49), non-ASCII advances (T14.55), StateNodeFont.bold (T14.56).",
 "Decimal ratchet: text_render.rs now 51 vs baseline 409 (headroom; baseline frozen for this packet, lowering needs owner sign-off); svg.rs 0 -> 1 (earlier round, provenance) needs its baseline line.",
 "harness_guards 'No such file' +2: docs/parity-reviews/shared-svg-corrective and tim-corrective-execution-maker lack account.json (earlier rounds).",
 "golden_no_oracle baseline: the 3 GET-provenance goldens (activity 1299->1297, edge-cases 760->759) recorded in earlier rounds need the owner's baseline decision.",
 "Gate residue: a consistent forgery (K9) passes the lib gate; a standing hmtx re-check on the reference host closes it (P6).",
 "bullseye.yaml content_hash banner conflict at merge: regenerate through bullseye after the union.",
]
TARGETS = [
 {"id": "P1", "extends": "T14.46", "name": "Sequence queue participants take the queue style's font, not participantFontStyle",
  "acceptance": ["Under skinparam participantFontStyle italic/bold a queue label is drawn and measured as Java draws it (upright, queue style), traced to the sequence queue component's style signature.",
                 "BX c11a_qu_{plain,multi,multist,empty}_{vd,tn}_{i,b} and BV m11a_kd_queu_vd_{i,bi} match, plus renamed held-out queue probes; no pass is lost."]},
 {"id": "P2", "extends": "T14.39", "name": "Sequence message labels are laid out in their drawn face, style included",
  "acceptance": ["sequence.rs message_label_width measures with the arrow font's italic as well as bold (Java measures the drawn TextBlock).",
                 "BX c11a_fc_smsg_vd_i and c11a_cr_smsg_il_b_vd match, plus held-out Times New Roman italic message probes; no pass is lost."]},
 {"id": "P3", "extends": "T14.47", "name": "State text under defaultFontName Arial is measured with Arial's advances",
  "acceptance": ["State names and descriptions in Arial measure 232.6475 for c11a's label at 14 pt (Rust 253.0801 at every tree).",
                 "BX c11a_fc_stat_ar_{p,i,b} and c11a_sz{9,23}_stat_ar_i match; no pass is lost."]},
 {"id": "P4", "name": "State names and descriptions render Creole as Java does, measured and drawn alike",
  "acceptance": ["`state \"a <i>b</i> c\"` and `E1 : <i>..</i>` draw the runs as Java does (Rust draws the tags literally while measuring the runs).",
                 "BX c11a_cr_stat_* and c11a_fc_sdsc_vd_bi match where their controls match."]},
 {"id": "P5", "extends": "T14.46", "name": "Element and label consumers honour <kind>FontStyle and <style> FontWeight",
  "acceptance": ["rectangle, package, object, component/class/sequence/activity notes, sequence group header and divider, component/class/use-case link labels, title, legend and the activity diamond draw and measure the configured family and style; `<style> FontStyle italic + FontWeight bold` yields bold italic.",
                 "BX c11a_fc_{rect,pack,objc,snot,sgrp,sdiv,anot,cnot,knot,clnk,kcls,ulnk,titl,lgnd,diam}_{vd,ar}_{i,bi} and the *_bi style cases match where their controls match."]},
 {"id": "P6", "name": "Table-measured face data is re-verified against the reference-host face files by a standing check",
  "acceptance": ["A gate (skipping where the face files are absent) reads hmtx advances of every table-measured face and compares them with the const tables; wired to the reference-host relay.", "A consistent const+TSV+SHA256SUMS forgery (drill K9) fails it."]},
 {"id": "P7", "extends": "T14.6", "name": "Harness records left by the shared-SVG line are complete",
  "acceptance": ["shared-svg-corrective and tim-corrective-execution-maker carry an account.json (or are folded into a mechanism that has one); the svg.rs decimal literal gets its reviewed baseline line; the text_render.rs headroom (409 vs 51) is locked by the owner."]},
]
for mech, sub, num in [("comment-state-consumers", "comment-state-consumers", 13), ("tim-builtin-value-boundaries", "tim-builtin-value-boundaries", 12)]:
    perts = [{"source": r["source"], "golden": r["golden"], "cwd": r["cwd"], "axes": r["axes"], "batch": r["batch"], "result": r["result"],
              "tip_strict": r["tip_97830bd1e_strict_compare"], "merge_strict": r["merge_879d166f4_strict_compare"], "base_strict": r["base_5ddf7733c_strict_compare"]}
             for r in pairs[mech] if r["registered"]]
    rev = {
     "schema_version": 1,
     "reviewer": {"task_id": "t3-1db06518 independent shared-SVG round-11 checker (claude)", "model": "claude-opus-5-5 / Claude Code in T3 Code; no children; did not implement the candidate; no Java run locally (all Java via the parent relay)"},
     "mechanism": mech, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711", "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
     "base_revision": "5ddf7733c7ef3cbc5371260b050a15cae0e10206", "base_code_identical_to": "9834a719455308120c3289b59a260567f141b724",
     "merge": "879d166f41958d30211ae0ef7a4c343a98c9812e", "merge_code_identical_to": "e24240b3542aaabdaf6286b87c1cb23217898ddc",
     "tested_revision": "97830bd1e455d007430aa8a35d9a05d6a669dcd4", "packet_tip": "ba15189b2c8893725afb1bda5ef0bb18ef60e9d8",
     "answers_rejection": "a7bd62c861e849bb3ddc3835147557676705cea7",
     "account_commit": "4f959e46415d81173de15e4316b72284ceba940a", "implementation_commit": "97830bd1e455d007430aa8a35d9a05d6a669dcd4",
     "other_commits_reviewed": ["4f959e464", "f7c382ff4", "cb3d36bb1", "0368eee79", "97830bd1e", "37381782a", "ac840a3da", "ba15189b2"],
     "no_production_changes": True, "no_shipping": True,
     "scope": "round-11 obligations: Verdana/Arial per-face ASCII advance tables selected by (bold, italic), own-table gate; queue pill dead face arguments (C3); StateNodeFont italic removal (C5). Whole shared-SVG packet line (rounds 1-11) for local merge readiness.",
     "commands": COMMANDS,
     "relays_answered": {"BX (checker C11A)": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round11-c11a", "reference_results_sha256": "694d0f73af805bf6179cc65fc4f38fa9594d4c2b1a648ccb99d02c29e9387938", "pre_registration": "942870ac0c0407dab9824b902e711d5b378eb175"},
                         "BV (maker M11A, held out from the maker)": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round11-m11a", "reference_results_sha256": "749bf2c2a9eab29b6b4855527cbc267fa09f80a1b9f6293ad5cc7718694ba082"},
                         "Monkey Man ASCII advances": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round11-mm-ascii/out", "SHA256SUMS_sha256": "f5def121617419f6d55ed7361fdba5b1d0e0b90433766f1e662a0c81586573d1"}},
     "evidence": {"dir": "checker-claude-round11 (byte-identical copy under docs/parity-reviews/comment-state-consumers/ and tim-builtin-value-boundaries/; SHA256SUMS.json)", "private_cache": "/Users/marcelo/Library/Caches/rustuml-shared-svg-round11-claude-checker"},
     "frozen_items": "unchanged 5ddf7733c..ba15189b2 and 879d166f4..ba15189b2: crates/rustuml-oracle (GEOM_EPS, syntax markers, park list, error census, guard allow-lists, 14-entry strict-XML list, decimal_literal_baseline.txt), test-diagrams/no_oracle_baseline.txt and ulp_parked.txt; no perturbation edited, deleted or added by the maker",
     "account_chronology": {"account (docs + extraction data)": "4f959e464 09:29:14", "tables + gate": "f7c382ff4 09:31:57", "C3 queue": "cb3d36bb1 09:35:38", "C5 addendum (docs only, measured first)": "0368eee79 09:40:36", "C5 code": "97830bd1e 09:41:02", "relay pre-registration": "37381782a 09:43:05", "evidence": "ac840a3da 09:52:46, ba15189b2 09:54:55", "Monkey Man extraction run": "2026-11-05 22:55:58 UTC (after the pre-registration)"},
     "counts": {"batches": batches, "bx_hypotheses": hyp, "bx_controlled_failures": {g: len(v) for g, v in ctl["controlled_failures"].items()},
                "bx_trees": bx["trees"], "bv_trees": bv["trees"], "sweep": sweep, "units_x_size": {"relay": units["stats"], "goldens": ug["stats"]},
                "hmtx": {k: {kk: vv for kk, vv in v.items() if kk in ("psname", "file_sha256", "hmtx_eq_const", "tsv_eq_hmtx", "old_f64_x2048_eq_hmtx", "differs_from_upright_slot")} for k, v in hm.items()},
                "monkeyman": {k: v for k, v in mm.items() if k != "rows"},
                "drill": {k: {"caught": v["caught"], "failed": v["failed"], "counts": v["counts"]} for k, v in drill.items()},
                "c3_c5_isolation_batches": {l: {"cases": v["cases"], "differ": len(v["differ"])} for l, v in f7.items()},
                "registered_pairs": {m: {"registered": sum(r["registered"] for r in rows), "pass_at_tip": sum(r["result"] == "pass" for r in rows)} for m, rows in pairs.items()}},
     "attributed_regressions": {
       "round 11": {"lost matches": "none on any cohort",
                    "exact viewBox": ["BV m11a_kd_queu_vd_i (168 -> 167): queue label drawn italic by Rust at every tree, upright in Java; merge agreed by measuring that italic text upright (P1)"],
                    "difference count worse (still failing at merge)": ["BV m11a_kd_queu_vd_{i,bi}", "BX c11a_qu_{plain,multi,multist,empty}_vd_i (same cause, P1)"]},
       "earlier rounds (already at merge)": {"goldens (base->tip)": "3 GET-provenance goldens act_skinparam_shadowing, act_skinparam2_shadowing_true, edge_whitespace_multiple_blank_lines",
         "exact viewBox only (case fails at every tree)": ["BO c8a_bl_lucida_thai_digit_after_r", "BR c9a_sp_seqnote_pdf_N_ctrl, c9a_sv_actmid_N_sp_ctrl, c9a_sv_classnote_N_ctrl", "BS m10a_acts_cn_i, m10a_css_actv_cn_i, m10a_css_part_cn_i, m10a_dbse_cn_i"],
         "comment lists (perturbations, base->tip)": ["shared-svg-corrective/checker-independent-astra/unicode_activity", "tim-sub-marker-layers/checker5/sub_suffix_continuation"]}},
     "verdict": "accepted",
     "packet_verdict": "Packet ba15189b2 (code 97830bd1e) accepted, and with it the whole shared-SVG line (rounds 1-11) is accept-ready for local merge into parity-recovery. The Verdana/Arial tables are face data (hmtx, Monkey Man, units x size 1,911/1,911 relay runs), selected by the JDK's (bold, italic) slot, upright output byte-identical, both removals byte-identical everywhere, the round-10 blocking probes match, no match is lost on any cohort. Residuals are recorded below; none is caused by the tables.",
     "per_mechanism_verdicts": {
       "per-face ASCII tables (f7c382ff4)": "CONFIRMED: provenance, slot selection, Java widths at sizes 9-23, upright byte-identical, gate catches 8/8 maker + 8/9 checker mutations (K9 declared residue)",
       "queue pill without dead face arguments (cb3d36bb1)": "CONFIRMED: byte-identical on 18,263 sources and 2,637 relay cases (pill height takes no face)",
       "StateNodeFont italic removed (97830bd1e)": "CONFIRMED: 11 stateFontStyle forms byte-identical merge -> tip in Times/SansSerif; golden_pairs identical at f7c382ff4 and tip",
       "process": "CONFIRMED: accounts before code, addendum measured first and accurate, frozen items unchanged, clippy equal, T14.6.7 categories unchanged, bullseye union clean apart from the regenerated banner"},
     "findings": FINDINGS,
     "residuals_to_record_at_merge": RESIDUALS,
     "held_out_failures": {"attributed (control matches, case fails at tip)": {g: [x["name"] for x in v] for g, v in ctl["controlled_failures"].items()}},
     "bounded_next_obligations": [],
     "proposed_targets": TARGETS,
     "perturbations": perts,
    }
    p = R / "docs/parity-reviews" / sub / f"review-{num}.json"; assert not p.exists(), p
    p.write_text(json.dumps(rev, indent=1, ensure_ascii=False) + "\n")
    print(p, len(perts), "pairs", sum(x["result"] == "pass" for x in perts), "pass")
