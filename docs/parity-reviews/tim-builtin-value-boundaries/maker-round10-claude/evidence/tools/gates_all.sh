#!/bin/bash
# Usage: gates_all.sh <tip tree>   (tree dirs under $C/src; base and merge fixed)
C=~/Library/Caches/rustuml-shared-svg-round10-claude-maker; TIP=$1
for r in $TIP base; do rm -f $C/gates/$r.exit; bash $C/tools/gates.sh $C/src/$r $r $C/target/$r $C/gates; done
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib CARGO_TARGET_DIR=$C/target/merge
cd $C/src/merge; rm -f $C/gates/merge.exit
for t in parity_perturbations harness_guards; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/merge.$t.log 2>&1; echo "$t exit $?" >> $C/gates/merge.exit; done
for r in $TIP base merge; do python3 -I $C/tools/gatelists.py $C/gates $r; done
echo GATES DONE
