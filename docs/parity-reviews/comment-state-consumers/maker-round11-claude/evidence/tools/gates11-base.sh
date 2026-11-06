#!/bin/bash
# Round-11 maker: merge-tree gates (attribution) and the 9834a7194 no-oracle failure names.
C=/Users/marcelo/Library/Caches/rustuml-shared-svg-round11-claude-maker
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
for r in merge 9834a7194; do
  export CARGO_TARGET_DIR=$C/target/gates-$r; cd $C/gsrc/$r
  tests="golden_no_oracle golden_pairs parity_perturbations harness_guards"; [ $r = 9834a7194 ] && tests="golden_no_oracle"
  for t in $tests; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/$r.$t.log 2>&1; echo "$t exit $?" >> $C/gates/$r.exit; done
  rm -rf $C/target/gates-$r
done
echo GATES-BASE DONE
