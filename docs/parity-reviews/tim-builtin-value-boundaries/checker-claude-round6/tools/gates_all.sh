#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round6-claude-checker
for r in 7e9b16f52 8317cf006; do
  bash $C/tools/gates.sh $C/src/$r $r $C/target/$r $C/gates
done
# merge: perturbations + guards only (packet-line attribution)
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib CARGO_TARGET_DIR=$C/target/fc44ee58c
cd $C/src/fc44ee58c
for t in parity_perturbations harness_guards; do cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/fc44ee58c.$t.log 2>&1; echo "$t exit $?" >> $C/gates/fc44ee58c.exit; done
# clippy per crate, no deps, -D warnings
for r in 8317cf006 fc44ee58c 7e9b16f52; do
  cd $C/src/$r
  for k in rustuml-render rustuml-parser rustuml-layout rustuml-oracle rustuml rustuml-math; do
    CARGO_TARGET_DIR=$C/target/clippy-$r cargo clippy -j 6 -p $k --no-deps -- -D warnings > $C/gates/$r.clippy-$k.log 2>&1; echo "$k exit $? errors $(grep -c '^error' $C/gates/$r.clippy-$k.log)" >> $C/gates/$r.clippy-counts.txt
  done
done
echo GATES DONE
