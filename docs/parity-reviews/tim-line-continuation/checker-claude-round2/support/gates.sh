#!/bin/bash
# Broad gates once each at the review branch (tip code f2d12d0ef); no-oracle/golden_pairs at 9834a7194 for the
# failure-name diff and parity_perturbations at the merged base b0fb43c3e for the new/fixed diff.
# Each tree builds in its own private CARGO_TARGET_DIR.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-round2-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-860c09a0
mkdir -p $V/gates
(
  cd $V/src/9834a7194 && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  for t in golden_no_oracle golden_pairs; do
    CARGO_TARGET_DIR=$V/target-9834a7194 cargo test --release -j 5 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base9834-$t.log 2>&1; echo "base9834 $t exit $?"
  done
) &
(
  cd $V/src/b0fb43c3e && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  CARGO_TARGET_DIR=$V/target-b0fb43c3e cargo test --release -j 5 -p rustuml-oracle --test parity_perturbations --no-fail-fast -- --nocapture > $V/gates/baseb0fb-parity_perturbations.log 2>&1; echo "baseb0fb pert exit $?"
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
