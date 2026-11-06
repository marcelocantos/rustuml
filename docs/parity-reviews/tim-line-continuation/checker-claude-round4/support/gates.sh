#!/bin/bash
# Broad gates once each. Tip gates run in the review checkout (code 9ed52f435; HEAD adds only docs).
# Merged base 1f6ffa9f4 export: golden_no_oracle, golden_pairs, parity_perturbations, harness_guards (diffs).
# parity-recovery cb550c0d6 export: pipe_mode (the brief's pipe_mode reference).
# One private CARGO_TARGET_DIR per tree; exports and targets deleted afterwards.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-round4-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-1ec8f45b
mkdir -p $V/gates
export_tree() { r=$1; rm -rf $V/src/g$r; mkdir -p $V/src/g$r; git -C $REPO archive $(git -C $REPO rev-parse $r) | tar -x -C $V/src/g$r; rm -rf $V/src/g$r/test-diagrams/golden; ln -s $REPO/test-diagrams/golden $V/src/g$r/test-diagrams/golden; }
(
  export_tree 1f6ffa9f4; cd $V/src/g1f6ffa9f4
  for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
    CARGO_TARGET_DIR=$V/target-gbase cargo test --release -j 4 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base-$t.log 2>&1; echo "base $t exit $?"
  done
  rm -rf $V/target-gbase $V/src/g1f6ffa9f4
) &
(
  export_tree cb550c0d6; cd $V/src/gcb550c0d6
  CARGO_TARGET_DIR=$V/target-gpr cargo test --release -j 3 -p rustuml --test pipe_mode > $V/gates/pr-pipe.log 2>&1; echo "pr pipe exit $?"
  rm -rf $V/target-gpr $V/src/gcb550c0d6
) &
cd $REPO; export CARGO_TARGET_DIR=$V/target-gtip
git rev-parse HEAD > $V/gates/head.txt; git status --porcelain > $V/gates/status-before.txt
cargo test --release -j 6 --workspace --lib > $V/gates/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 6 -p rustuml --test pipe_mode > $V/gates/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
  cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$t.log 2>&1; echo "$t exit $?"
done
git status --porcelain > $V/gates/status-after.txt
wait
rm -rf $V/target-gtip
echo GATES-DONE
