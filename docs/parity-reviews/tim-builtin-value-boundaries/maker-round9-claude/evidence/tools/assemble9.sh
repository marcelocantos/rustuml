#!/bin/bash
# Copy round-9 maker evidence into tim-builtin-value-boundaries/maker-round9-claude/evidence, then mirror the
# whole maker-round9-claude directory byte-identically into comment-state-consumers (T14.6.7).
set -e
C=~/Library/Caches/rustuml-shared-svg-round9-claude-maker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-8ee2b4ee
D=$R/docs/parity-reviews/tim-builtin-value-boundaries/maker-round9-claude/evidence
rm -rf $D; mkdir -p $D/m9b-request $D/batches $D/sweep $D/gates $D/clippy $D/drift $D/bidi $D/scan $D/m9a-request $D/mm-styles-request $D/face-probe-request $D/tools
for L in AR AU AW BB BC BE BF BG BH BI BJ BK BL BM BN BO BP; do cp $C/work/batch-$L.json $D/batches/; done
cp $C/work/batch-summary.json $C/work/viewbox-census.json $C/work/widths-BO.json $D/batches/
cp $C/work/cohorts.json $C/work/cohorts.txt $D/sweep/
for r in base merge tip; do cp $C/gates/$r.exit $C/gates/$r.guard_cats.txt $C/gates/$r.pert_fail.txt $D/gates/; done
for r in base tip; do
  grep -E "^golden_pairs:" $C/gates/$r.golden_pairs.log > $D/gates/$r.golden_pairs_summary.txt
  sed -n '/families\|^  [a-z-]* /p' $C/gates/$r.golden_no_oracle.log | tail -40 > $D/gates/$r.no_oracle_families.txt
  grep -E "test result" $C/gates/$r.lib.log > $D/gates/$r.lib_results.txt
  grep -E "^test |test result" $C/gates/$r.pipe_mode.log > $D/gates/$r.pipe_mode.txt
  cp $C/src/$r/test-diagrams/no_oracle_failure_names.txt $D/gates/$r.no_oracle_failure_names.txt
done
diff $D/gates/base.no_oracle_failure_names.txt $D/gates/tip.no_oracle_failure_names.txt > $D/gates/no_oracle_failure_names.base-vs-tip.diff || true
cp $C/work/clippy-diags.json $D/clippy/
cp $C/drift/*.log $C/drift/drill.json $D/drift/
cp $C/work/bidi/bidi-compare.json $C/work/bidi/textline-compare.json $C/work/bidi/textline-compare-round8tip.json $C/work/bidi/maker-probe.patch $D/bidi/
cp $C/work/scan/*.puml $C/work/scan/scan-summary.txt $D/scan/
for f in cases.json m9a_sources.py reference-manifest.json reference-input-bundle.json.gz rust-predictions.json SHA256SUMS; do cp $C/relay/m9a/$f $D/m9a-request/; done
cp $C/relay/mm-styles/* $D/mm-styles-request/; cp $C/relay/face-probe/* $D/face-probe-request/
cp $C/work/mmtest/work/out/SHA256SUMS $D/mm-styles-request/colossus-dry-run-SHA256SUMS
for f in cases.json m9b_sources.py reference-manifest.json reference-input-bundle.json.gz rust-predictions.json SHA256SUMS; do cp $C/relay/m9b/$f $D/m9b-request/; done
cp $C/work/relay-results.json $D/relay-results.json
chmod -R u+w $D
cp $C/tools/*.py $C/tools/*.sh $C/tools/maker-cmp-* $D/tools/
cp $C/bin/binaries.sha256 $D/binaries.sha256
python3 -I - "$D" <<'PY'
import hashlib, json, pathlib, sys
D = pathlib.Path(sys.argv[1])
sums = {str(p.relative_to(D)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(D.rglob("*")) if p.is_file()}
(D / "SHA256SUMS.json").write_text(json.dumps(sums, indent=1, sort_keys=True) + "\n")
print(len(sums), "files")
PY
S=$R/docs/parity-reviews/tim-builtin-value-boundaries/maker-round9-claude
T=$R/docs/parity-reviews/comment-state-consumers/maker-round9-claude
rm -rf $T; cp -R $S $T; diff -r $S $T && echo mirrored
