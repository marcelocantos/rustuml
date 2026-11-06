#!/bin/bash
# Copy round-10 evidence into the maker directory of both mechanisms (byte-identical), with SHA256SUMS.
C=~/Library/Caches/rustuml-shared-svg-round10-claude-maker; W=/Users/marcelo/.t3/worktrees/rustuml/t3-839e94d7
D=$W/docs/parity-reviews/tim-builtin-value-boundaries/maker-round10-claude/evidence
rm -rf $D; mkdir -p $D/{tools,batches,sweep,gates,clippy,drill/logs,scan,m10a-request,process}
cp $C/tools/*.py $C/tools/*.sh $D/tools/; cp $C/cmp/Cargo.toml $D/tools/maker-cmp-Cargo.toml; cp $C/cmp/src/main.rs $D/tools/maker-cmp-main.rs
cp $C/batches/batch-*.json $C/batches/viewbox-census.json $D/batches/
cp $C/work/cohorts.json $C/work/cohorts.txt $D/sweep/
for r in tip merge base; do
  cp $C/gates/$r.exit $D/gates/ 2>/dev/null
  for f in pert_fail.txt guard_cats.txt no_oracle_failure_names.txt golden_failure_names.txt; do cp $C/gates/$r.$f $D/gates/ 2>/dev/null; done
  grep -E "^golden_pairs:" $C/gates/$r.golden_pairs.log > $D/gates/$r.golden_pairs_summary.txt 2>/dev/null
  grep -E "^golden_no_oracle:|^  [a-z][a-z0-9-]* +[0-9]+/[0-9]+" $C/gates/$r.golden_no_oracle.log > $D/gates/$r.no_oracle_families.txt 2>/dev/null
  grep -E "^test result|FAILED|panicked" $C/gates/$r.lib.log > $D/gates/$r.lib_results.txt 2>/dev/null
  grep -E "^test |^test result" $C/gates/$r.pipe_mode.log > $D/gates/$r.pipe_mode.txt 2>/dev/null
  grep -E "^test |^test result" $C/gates/$r.doc_claims.log > $D/gates/$r.doc_claims.txt 2>/dev/null
done
find $D/gates -size 0 -delete
diff $C/gates/base.no_oracle_failure_names.txt $C/gates/tip.no_oracle_failure_names.txt > $D/gates/no_oracle_failure_names.9834a7194-vs-tip.diff
diff $C/gates/merge.no_oracle_failure_names.txt $C/gates/tip.no_oracle_failure_names.txt > $D/gates/no_oracle_failure_names.merge-vs-tip.diff
cp $C/gates/clippy-diags.json $C/gates/clippy-summary.txt $C/gates/clippy.out $D/clippy/
cp $C/work/drill10t/drill.json $D/drill/; cp $C/work/drill10t/*.log $D/drill/logs/
cp $C/work/scan-tip/scan.txt $C/work/scan-tip/style-scan.json $D/scan/; cp $C/work/scan-merge.txt $D/scan/scan-merge.txt
for f in cases.json maker-expectations.json reference-manifest.json reference-input-bundle.json.gz; do cp $C/m10a/$f $D/m10a-request/; done
gzip -9 -n -c $C/m10a/rust-predictions.json > $D/m10a-request/rust-predictions.json.gz
(cd $C/m10a && shasum -a 256 cases.json maker-expectations.json reference-manifest.json reference-input-bundle.json.gz rust-predictions.json) > $D/m10a-request/SHA256SUMS.cache
cp $C/work/c6413-font_chain.log $D/process/
(cd $C/bin && shasum -a 256 rustuml-base rustuml-merge rustuml-tip) > $D/binaries.sha256
(cd $D && find . -type f ! -name SHA256SUMS.json | sort | while read f; do printf '%s  %s\n' "$(shasum -a 256 "$f" | cut -c1-64)" "${f#./}"; done | python3 -c "import sys,json;print(json.dumps(dict(reversed(l.rstrip().split('  ',1)) for l in sys.stdin),indent=1))" > SHA256SUMS.json)
du -sh $D
