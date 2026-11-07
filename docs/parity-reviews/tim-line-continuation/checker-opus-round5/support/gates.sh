#!/bin/bash
# Checker gates. Tip gates in the checker worktree (36843c258 + checker docs; code c4b5a8436). Merged base 9dc1af5f9 and
# parent 32684f81e in git-archive exports (golden symlinked to the hydrated 9efdc12f submodule). One cargo-created
# CARGO_TARGET_DIR per tree, deleted after its run.
set -u
V=$HOME/Library/Caches/rustuml-opus-tim-r5-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-7758f731
mkdir -p $V/gates
export_gates() {
  r=$1; shift
  rm -rf $V/src/g$r; mkdir -p $V/src/g$r; git -C $REPO archive $(git -C $REPO rev-parse $r) | tar -x -C $V/src/g$r
  rm -rf $V/src/g$r/test-diagrams/golden; ln -s $REPO/test-diagrams/golden $V/src/g$r/test-diagrams/golden; cd $V/src/g$r
  for t in "$@"; do
    CARGO_TARGET_DIR=$V/target-g$r cargo test --release -j 4 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$r-$t.log 2>&1; echo "$r $t exit $?"
  done
  rm -rf $V/target-g$r $V/src/g$r
}
( export_gates 9dc1af5f9 golden_no_oracle golden_pairs parity_perturbations harness_guards ) &
( export_gates 32684f81e golden_no_oracle parity_perturbations ) &
cd $REPO; export CARGO_TARGET_DIR=$V/target-wt
git rev-parse HEAD > $V/gates/head.txt; git status --porcelain > $V/gates/status-before.txt
cargo test --release -j 6 --workspace --lib > $V/gates/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 6 -p rustuml --test pipe_mode > $V/gates/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards doc_claims; do
  cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$t.log 2>&1; echo "$t exit $?"
done
for c in rustuml rustuml-parser rustuml-render rustuml-layout rustuml-math rustuml-oracle; do
  cargo clippy --release -j 6 -p $c -- -D warnings > $V/gates/clippy-$c.log 2>&1; echo "clippy $c exit $?"
done
git status --porcelain > $V/gates/status-after.txt
rm -rf $V/target-wt
wait
echo GATES-DONE
