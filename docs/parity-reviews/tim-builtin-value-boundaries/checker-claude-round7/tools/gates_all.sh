#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round7-claude-checker
for r in b0ca0a7dc 460570ba2; do
  bash $C/tools/gates.sh $C/src/$r $r $C/target/$r $C/gates
done
# merge: perturbations + guards only (packet-line attribution)
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib CARGO_TARGET_DIR=$C/target/716762056
cd $C/src/716762056
for t in parity_perturbations harness_guards; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/716762056.$t.log 2>&1; echo "$t exit $?" >> $C/gates/716762056.exit; done
# clippy per crate, no deps, -D warnings
for r in 460570ba2 716762056 b0ca0a7dc; do
  cd $C/src/$r
  for k in rustuml-render rustuml-parser rustuml-layout rustuml-oracle rustuml rustuml-math; do
    CARGO_TARGET_DIR=$C/target/clippy-$r cargo clippy -j 6 -p $k --no-deps -- -D warnings > $C/gates/$r.clippy-$k.log 2>&1; echo "$k exit $? errors $(grep -c '^error' $C/gates/$r.clippy-$k.log)" >> $C/gates/$r.clippy-counts.txt
  done
done
echo GATES DONE
