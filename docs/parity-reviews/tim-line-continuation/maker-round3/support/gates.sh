#!/bin/bash
# Broad gates once each at the maker branch (tip code d28dd64a9); golden_no_oracle/golden_pairs at 9834a7194 for
# the failure-name diff; parity_perturbations and harness_guards at the merged base a5720e095 for the new/fixed and
# category diffs. Each tree builds in its own private CARGO_TARGET_DIR.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-round3-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-78d2182a
mkdir -p $V/gates
(
  cd $V/src/9834a7194 && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  for t in golden_no_oracle golden_pairs; do
    CARGO_TARGET_DIR=$V/target-9834a7194 cargo test --release -j 4 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base9834-$t.log 2>&1; echo "base9834 $t exit $?"
  done
) &
(
  cd $V/src/a5720e095 && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  for t in parity_perturbations harness_guards; do
    CARGO_TARGET_DIR=$V/target-a5720e095 cargo test --release -j 4 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/mergedbase-$t.log 2>&1; echo "mergedbase $t exit $?"
  done
) &
cd $REPO; export CARGO_TARGET_DIR=$V/target-gates
git rev-parse HEAD > $V/gates/head.txt; git status --porcelain > $V/gates/status-before.txt
cargo test --release -j 6 --workspace --lib > $V/gates/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 6 -p rustuml --test pipe_mode > $V/gates/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
  cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$t.log 2>&1; echo "$t exit $?"
done
git status --porcelain > $V/gates/status-after.txt
wait
echo GATES-DONE
