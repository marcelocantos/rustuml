#!/bin/bash
# Broad gates once each. Tip gates run in the review checkout (code d28dd64a9; HEAD carries only docs on top).
# golden_no_oracle/golden_pairs at 9834a7194 for the failure-name diff; parity_perturbations at the merged base
# a5720e095 for the new/fixed diff. One private CARGO_TARGET_DIR per tree; exports and targets deleted afterwards.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-round3-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-a7fabc1d
mkdir -p $V/gates
export_tree() { r=$1; rm -rf $V/src/$r; mkdir -p $V/src/$r; git -C $REPO archive $(git -C $REPO rev-parse $r) | tar -x -C $V/src/$r; rm -rf $V/src/$r/test-diagrams/golden; ln -s $REPO/test-diagrams/golden $V/src/$r/test-diagrams/golden; }
(
  export_tree 9834a7194; cd $V/src/9834a7194
  for t in golden_no_oracle golden_pairs; do
    CARGO_TARGET_DIR=$V/target-g9834 cargo test --release -j 4 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base9834-$t.log 2>&1; echo "base9834 $t exit $?"
  done
  rm -rf $V/target-g9834 $V/src/9834a7194
) &
(
  export_tree a5720e095; cd $V/src/a5720e095
  CARGO_TARGET_DIR=$V/target-ga572 cargo test --release -j 4 -p rustuml-oracle --test parity_perturbations --no-fail-fast -- --nocapture > $V/gates/basea572-parity_perturbations.log 2>&1; echo "basea572 pert exit $?"
  CARGO_TARGET_DIR=$V/target-ga572 cargo test --release -j 4 -p rustuml-oracle --test harness_guards --no-fail-fast -- --nocapture > $V/gates/basea572-harness_guards.log 2>&1; echo "basea572 guards exit $?"
  rm -rf $V/target-ga572 $V/src/a5720e095
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
