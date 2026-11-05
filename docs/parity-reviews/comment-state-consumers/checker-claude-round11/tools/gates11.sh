#!/bin/bash
# Round-11 checker gates. tip (evidence tip ba15189b2, code = 97830bd1e): every gate once. 9834a7194: golden_no_oracle (name diff).
# base 5ddf7733c: perturbations/guards/golden_pairs (attribution). merge 879d166f4 and f7c382ff4: golden_pairs (oracle-tier names, C5).
W=~/Library/Caches/rustuml-shared-svg-round11-claude-checker
T=$(dirname $0)
bash $W/tools/gates.sh $W/gsrc/ba15189b2 tip $W/tgt-gates-tip $W/gates
rm -rf $W/tgt-gates-tip
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
for r in 9834a7194 5ddf7733c 879d166f4 f7c382ff4; do
  export CARGO_TARGET_DIR=$W/tgt-gates-$r; cd $W/gsrc/$r
  case $r in 9834a7194) tests="golden_no_oracle";; 5ddf7733c) tests="parity_perturbations harness_guards golden_pairs";; *) tests="golden_pairs";; esac
  for t in $tests; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $W/gates/$r.$t.log 2>&1; echo "$t exit $?" >> $W/gates/$r.exit; done
  rm -rf $W/tgt-gates-$r
done
echo GATES DONE
