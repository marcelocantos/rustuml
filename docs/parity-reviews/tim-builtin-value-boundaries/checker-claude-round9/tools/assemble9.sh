#!/bin/bash
set -e
C=~/Library/Caches/rustuml-shared-svg-round9-claude-checker; E=$C/ev; W=/Users/marcelo/.t3/worktrees/rustuml/t3-7fb30d5d; G=$C/gates
mkdir -p $E/{batches,bidi,gates,drift/logs,maker-drill,regen,oracle,tools,remeasure}
for L in AR AU AW BB BC BE BF BG BH BI BJ BK BL BM BN BO BP BQ BR; do cp $C/work/batch-$L.json $E/batches/; done
cp $C/work/batch-summary.json $C/work/viewbox-census.json $C/work/deltas-BQ.json $C/work/deltas-BR.json $C/work/cohorts.json $E/batches/
cp $G/review.guard_cats.txt $G/review.pert_fail.txt $G/14dd5efae.exit $G/9d6af0a0e.exit $G/9834a7194.exit $G/14dd5efae.guard_cats.txt $G/9d6af0a0e.guard_cats.txt $G/14dd5efae.pert_fail.txt $G/9d6af0a0e.pert_fail.txt $G/14dd5efae.decimal.txt $G/9d6af0a0e.decimal.txt $G/clippy-diags.json $E/gates/
diff $C/src/9834a7194/test-diagrams/no_oracle_failure_names.txt $C/src/14dd5efae/test-diagrams/no_oracle_failure_names.txt > $E/gates/no_oracle_failure_names.9834a7194-vs-14dd5efae.diff || true
for r in 14dd5efae 9834a7194; do sed -n '/golden_no_oracle: /,/wire/p' $G/$r.golden_no_oracle.log > $E/gates/$r.no_oracle_families.txt; done
grep "golden_pairs:" $G/14dd5efae.golden_pairs.log $G/9d6af0a0e.golden_pairs.log | sed "s#$G/##" > $E/gates/golden_pairs_summary.txt
grep "test result" $G/14dd5efae.lib.log $G/14dd5efae.pipe_mode.log $G/14dd5efae.doc_claims.log | sed "s#$G/##" > $E/gates/tip.results.txt
cp $C/drill-logs/drill.json $E/drift/; cp $C/drill-logs/*.log $E/drift/logs/; cp $C/maker-drill/logs/drill.json $E/maker-drill/
cp $C/work/pairs.json $E/pairs.json; cp $C/work/tbvb-remeasure.json $C/work/csc-remeasure.json $E/remeasure/
for t in c9a_sources.py predict.py stage.py batch.py batchsum.py vbcensus9.py deltas9.py sweep.py sweep9.sh cohorts.py gates.sh gates9.sh gatelists.py clippy9.sh clippy_list.py bidi_compare9.py drift_drill9c.py register9.py remeasure9.py write_reviews9.py run_all9.sh run_batches9.sh cwd_map.py build.sh assemble9.sh; do cp $C/tools/$t $E/tools/; done
{ echo "// Scratch tests appended to a private copy of 14dd5efae (never committed to a product tree)"; sed -n '/checker_round9_dump/,$p' $C/scratch/tip/crates/rustuml-render/src/java_bidi.rs; sed -n '/checker_round9_faces/,$p' $C/scratch/tip/crates/rustuml-render/src/text_render.rs; } > $E/tools/scratch-tests.rs.txt
printf '%s\n' "254fc2599471562f2fb83e99cd89f7d2e5bdf10bd3861a826e5f8423a27aedf0  rustuml-dcf6d9076 (base)" "5b3370aad125a532149c50ed8269b23ef3bb481b4c51e77d87bd5756094dbdfc  rustuml-9d6af0a0e (merge)" "3514d83528639bc496902a6646732b71295af914eaceef7ed311f258d9b9302d  rustuml-14dd5efae (tip)" > $E/binaries.sha256
rm -f $E/SHA256SUMS.json
python3 -I -c "
import hashlib, json, pathlib, sys
E=pathlib.Path(sys.argv[1]); s={str(p.relative_to(E)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(E.rglob('*')) if p.is_file()}
(E/'SHA256SUMS.json').write_text(json.dumps(s, indent=1)+'\n'); print(len(s))" $E
for m in tim-builtin-value-boundaries comment-state-consumers; do cp -R $E/. $W/docs/parity-reviews/$m/checker-claude-round9/; done
du -sh $W/docs/parity-reviews/tim-builtin-value-boundaries/checker-claude-round9
