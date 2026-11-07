#!/bin/bash
# Tip gates in the maker worktree (code c4b5a8436); merged-base 9dc1af5f9 gates in a git-archive export (golden symlinked).
set -u
V=$HOME/Library/Caches/rustuml-opus-tim-r5; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-bc3abe07
mkdir -p $V/gates
(
  r=9dc1af5f9; rm -rf $V/src/g$r; mkdir -p $V/src/g$r; git -C $REPO archive $(git -C $REPO rev-parse $r) | tar -x -C $V/src/g$r
  rm -rf $V/src/g$r/test-diagrams/golden; ln -s $REPO/test-diagrams/golden $V/src/g$r/test-diagrams/golden; cd $V/src/g$r
  for t in golden_no_oracle golden_pairs parity_perturbations harness_guards doc_claims; do
    CARGO_TARGET_DIR=$V/target-gbase cargo test --release -j 4 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base-$t.log 2>&1; echo "base $t exit $?"
  done
  CARGO_TARGET_DIR=$V/target-gbase cargo test --release -j 4 -p rustuml --test pipe_mode > $V/gates/base-pipe.log 2>&1; echo "base pipe exit $?"
  rm -rf $V/target-gbase $V/src/g$r
) &
cd $REPO; export CARGO_TARGET_DIR=$V/target-wt
git rev-parse HEAD > $V/gates/head.txt; git status --porcelain > $V/gates/status-before.txt
cargo test --release -j 6 --workspace --lib > $V/gates/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 6 -p rustuml --test pipe_mode > $V/gates/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards doc_claims; do
  cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$t.log 2>&1; echo "$t exit $?"
done
git status --porcelain > $V/gates/status-after.txt
wait
echo GATES-DONE
