#!/bin/bash
# Broad gates once each at the worktree (tip code), plus golden_no_oracle/golden_pairs at a 9834a7194 export
# (failure-name diff) and parity_perturbations at the merged-base export (pre-existing-directory baseline).
set -u
M=$HOME/Library/Caches/rustuml-tim-continuation-round2-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-fc5b3c96
G=$M/gates; mkdir -p $G
(
  cd $M/src/9834a7194 && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  for t in golden_no_oracle golden_pairs; do
    CARGO_TARGET_DIR=$M/target-9834a7194 cargo test --release -j 3 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $G/base9834-$t.log 2>&1; echo "base9834 $t exit $?"
  done
  cd $M/src/base && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  CARGO_TARGET_DIR=$M/target-base cargo test --release -j 3 -p rustuml-oracle --test parity_perturbations --no-fail-fast -- --nocapture > $G/mergedbase-parity_perturbations.log 2>&1; echo "mergedbase parity_perturbations exit $?"
) &
cd $REPO; export CARGO_TARGET_DIR=$M/target-gates
git rev-parse HEAD > $G/head.txt; git status --porcelain > $G/status-before.txt
cargo test --release -j 5 --workspace --lib > $G/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 5 -p rustuml --test pipe_mode > $G/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
  cargo test --release -j 5 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $G/$t.log 2>&1; echo "$t exit $?"
done
git status --porcelain > $G/status-after.txt
wait
echo GATES-DONE
