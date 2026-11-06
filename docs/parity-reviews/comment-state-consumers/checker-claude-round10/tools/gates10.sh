#!/bin/bash
# Round-10 checker: each broad gate once. tip: all gates; base: perturbations/guards/golden_pairs (attribution); 9834a7194: no-oracle.
C=~/Library/Caches/rustuml-shared-svg-round10-claude-checker
bash $C/tools/gates.sh $C/gsrc/e24240b35 e24240b35 $C/target/gates-e24240b35 $C/gates
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
for r in 95291dc6b 9834a7194; do
  export CARGO_TARGET_DIR=$C/target/gates-$r; cd $C/gsrc/$r
  tests="parity_perturbations harness_guards golden_pairs"; [ $r = 9834a7194 ] && tests="golden_no_oracle"
  for t in $tests; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/$r.$t.log 2>&1; echo "$t exit $?" >> $C/gates/$r.exit; done
done
echo GATES DONE
