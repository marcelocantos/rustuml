#!/bin/bash
# Copy round-7 maker evidence into the mechanism directory (then results.json via write_results.py).
set -e
C=~/Library/Caches/rustuml-shared-svg-round9-claude-maker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-ddc6d2de
D=$R/docs/parity-reviews/tim-builtin-value-boundaries/maker-round7-claude/evidence
rm -rf $D; mkdir -p $D/batches $D/sweep $D/gates $D/clippy $D/drift $D/l7a-request $D/mm-font-hash-request $D/sources $D/tools
for L in BJ BI BH BG BF BE BC BB AW AU AR; do cp $C/work/batch-$L.json $D/batches/; done
cp $C/work/batch-summary.json $C/work/viewbox-census.json $D/batches/
cp $C/work/cohorts.json $C/work/cohorts.txt $D/sweep/
python3 -I - "$C" "$D" <<'PY'
import json, pathlib, sys
C, D = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
st = {t: json.load(open(C / f"work/sweep-{t}/status.json")) for t in ["base", "merge", "c1", "tip1"]}
# per-source stdout/stderr hashes and exit for every tree (compact, sorted)
# one line per source: path, then exit:stdout_sha256[:16]:stderr_sha256[:16] for base, merge, c1, tip1
lines = ["# source\t" + "\t".join(st)] + [k + "\t" + "\t".join(f"{st[t][k]['status']}:{st[t][k]['stdout_sha256'][:16]}:{st[t][k]['stderr_sha256'][:16]}" for t in st) for k in sorted(st["tip1"])]
(D / "sweep/status-by-tree.tsv").write_text("\n".join(lines) + "\n")
same = sum(st["c1"][k]["stdout_sha256"] == st["tip1"][k]["stdout_sha256"] for k in st["tip1"])
(D / "sweep/clippy-commit-byte-identity.json").write_text(json.dumps({
  "trees": {"c1": "9f33bc3b1 (before the clippy commit)", "tip1": "c77fa20a2"}, "sources": len(st["tip1"]),
  "stdout_identical": same, "exit_identical": sum(st["c1"][k]["status"] == st["tip1"][k]["status"] for k in st["tip1"]),
  "stderr_differences": [k for k in st["tip1"] if st["c1"][k]["stderr_sha256"] != st["tip1"][k]["stderr_sha256"]],
  "stderr_difference_cause": "thread ids in four pre-existing panic messages (crates/rustuml-parser/src/parse/class.rs:285), present at base and merge"}, indent=1))
PY
for t in base merge tip1; do for f in exit guard_cats.txt pert_fail.txt; do [ -f $C/gates/$t.$f ] && cp $C/gates/$t.$f $D/gates/; done
  for g in golden_no_oracle golden_pairs pipe_mode doc_claims; do [ -f $C/gates/$t.$g.log ] && grep -vE "^Warning: |Compiling|^\s+Finished|^\s+Running" $C/gates/$t.$g.log > $D/gates/$t.$g.txt; done
  [ -f $C/gates/$t.lib.log ] && grep "test result" $C/gates/$t.lib.log > $D/gates/$t.lib_results.txt
  awk '/decimal_literal_ratchet/{f=1} f' $C/gates/$t.harness_guards.log | grep -E "decimal|rustuml-render/src|FAILED" | head -12 > $D/gates/$t.decimal_ratchet.txt
done
cp $C/gates/pert_summary.json $D/gates/
cp $C/gates/clippy-diags.json $C/gates/clippy-run.log $D/clippy/
cp $C/drift/drill-summary.txt $D/drift/; for f in $C/drift/M*.log; do grep -E "panicked|assertion|left:|right:|test result" $f | head -6 > $D/drift/$(basename $f .log).txt; done
cp -R $C/l7a/cases.json $C/l7a/sources $C/l7a/reference-manifest.json $C/l7a/reference-input-bundle.json.gz $C/l7a/rust-predictions.json $C/l7a/SHA256SUMS $C/l7a/l7a_sources.py $C/l7a/predict.py $C/l7a/stage.py $D/l7a-request/
cp $C/mm-font-hash/run.sh $C/mm-font-hash/SHA256SUMS $D/mm-font-hash-request/
cp $C/work/sources-sha256.txt $C/work/regeneration.txt $D/sources/
cp $C/tools/*.py $C/tools/*.sh $D/tools/; cp $C/cmp/src/main.rs $D/tools/maker-cmp-main.rs; cp $C/cmp/Cargo.toml $D/tools/maker-cmp-Cargo.toml
shasum -a 256 $C/bin/rustuml-* | sed "s#$C/##" > $D/binaries.sha256
echo ASSEMBLED
