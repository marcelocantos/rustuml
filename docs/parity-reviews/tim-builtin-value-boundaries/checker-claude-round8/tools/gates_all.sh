#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round8-claude-checker
for r in c3116f0b0 5d8b1f7c1; do bash $C/tools/gates.sh $C/src/$r $r $C/target/gates-$r $C/gates; done
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
for r in d6e9ffd94 9834a7194; do
  export CARGO_TARGET_DIR=$C/target/gates-$r; cd $C/src/$r
  tests="parity_perturbations harness_guards"; [ $r = 9834a7194 ] && tests="golden_no_oracle"
  for t in $tests; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/$r.$t.log 2>&1; echo "$t exit $?" >> $C/gates/$r.exit; done
done
echo GATES DONE
