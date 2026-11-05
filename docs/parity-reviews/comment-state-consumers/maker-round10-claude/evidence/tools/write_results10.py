"""Compose maker-round10-claude/results.json from the evidence files. Usage: python3 -I write_results10.py <repo>"""
import json, pathlib, re, sys, subprocess, hashlib
R = pathlib.Path(sys.argv[1]); D = R / "docs/parity-reviews/tim-builtin-value-boundaries/maker-round10-claude"; E = D / "evidence"
C = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round10-claude-maker"
git = lambda *a: subprocess.run(["git", "-C", str(R), *a], capture_output=True, text=True).stdout.strip()
bs = json.load(open(C / "batches/batch-summary10.json")); vb = json.load(open(E / "batches/viewbox-census.json"))
coh = json.load(open(E / "sweep/cohorts.json"))
def rd(p): return (E / p).read_text().strip() if (E / p).exists() else None
drill = json.load(open(E / "drill/drill.json"))
res = {
 "schema_version": 1, "mechanism": "tim-builtin-value-boundaries", "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
 "status": "maker results; independent review required (the maker does not judge acceptance)",
 "base": {"branch": "parity-recovery-2026-11-03", "tip": "95291dc6b", "code_identical_to": "9834a7194"},
 "merge": git("rev-parse", "d06667af0"), "tested_revision": git("rev-parse", "HEAD~0"),
 "binaries": rd("binaries.sha256"),
 "batches": {L: {k: (v if not isinstance(v, list) else v) for k, v in row.items()} for L, row in bs.items()},
 "viewbox_and_comment_census": vb,
 "sweep_cohorts": coh,
 "gates": {t: rd(f"gates/{t}.exit") for t in ("tip", "merge", "base")},
 "golden_pairs": {t: rd(f"gates/{t}.golden_pairs_summary.txt") for t in ("tip", "merge")},
 "no_oracle": {t: rd(f"gates/{t}.no_oracle_families.txt").splitlines()[0] if rd(f"gates/{t}.no_oracle_families.txt") else None for t in ("tip", "merge", "base")},
 "no_oracle_failure_names_diff_vs_9834a7194": rd("gates/no_oracle_failure_names.9834a7194-vs-tip.diff") or "(empty: identical 70 names)",
 "no_oracle_failure_names_diff_vs_merge": rd("gates/no_oracle_failure_names.merge-vs-tip.diff") or "(empty)",
 "parity_perturbations_fail_counts": {t: len((rd(f"gates/{t}.pert_fail.txt") or "").splitlines()) for t in ("tip", "merge")},
 "parity_perturbations_new_failures_tip_vs_merge": sorted(set((rd("gates/tip.pert_fail.txt") or "").splitlines()) - set((rd("gates/merge.pert_fail.txt") or "").splitlines())),
 "parity_perturbations_fixed_tip_vs_merge": sorted(set((rd("gates/merge.pert_fail.txt") or "").splitlines()) - set((rd("gates/tip.pert_fail.txt") or "").splitlines())),
 "harness_guards": {t: rd(f"gates/{t}.guard_cats.txt") for t in ("tip", "merge")},
 "clippy": rd("clippy/clippy-summary.txt"),
 "drill": {k: ("caught" if v.get("caught") else ("pass" if v["passed"] else "fail")) for k, v in drill.items()},
}
(D / "results.json").write_text(json.dumps(res, indent=1, ensure_ascii=False) + "\n")
print(json.dumps({k: res[k] for k in ("gates", "golden_pairs", "no_oracle", "parity_perturbations_fail_counts", "parity_perturbations_new_failures_tip_vs_merge", "parity_perturbations_fixed_tip_vs_merge", "clippy", "drill")}, indent=1))
