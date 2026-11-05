#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round9-claude-checker
bash $C/tools/gates.sh $C/src/14dd5efae 14dd5efae $C/target/gates-14dd5efae $C/gates
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
for r in 9d6af0a0e 9834a7194; do
  export CARGO_TARGET_DIR=$C/target/gates-$r; cd $C/src/$r
  tests="parity_perturbations harness_guards golden_pairs"; [ $r = 9834a7194 ] && tests="golden_no_oracle"
  for t in $tests; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/$r.$t.log 2>&1; echo "$t exit $?" >> $C/gates/$r.exit; done
done
echo GATES DONE
