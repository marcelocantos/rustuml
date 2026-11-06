#!/bin/bash
# Broad gates. Usage: gates.sh base|tip
#  base: export of the merged base 1f6ffa9f4 (golden symlinked), golden_no_oracle, golden_pairs,
#        parity_perturbations, harness_guards; failure-name files kept; target and export deleted.
#  tip:  in the maker checkout: workspace lib, pipe_mode, then the same four oracle tests.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-round4-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-94686399
mkdir -p $V/gates
if [ "$1" = base ]; then
  r=1f6ffa9f4; rm -rf $V/src/g$r; mkdir -p $V/src/g$r
  git -C $REPO archive $(git -C $REPO rev-parse $r) | tar -x -C $V/src/g$r
  rm -rf $V/src/g$r/test-diagrams/golden; ln -s $REPO/test-diagrams/golden $V/src/g$r/test-diagrams/golden
  cd $V/src/g$r
  for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
    CARGO_TARGET_DIR=$V/target-gbase cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base-$t.log 2>&1; echo "base $t exit $?"
  done
  cp test-diagrams/no_oracle_failure_names.txt $V/gates/no_oracle_failure_names-base.txt 2>/dev/null
  cp test-diagrams/golden_failure_names.txt $V/gates/golden_failure_names-base.txt 2>/dev/null
  git -C $REPO diff --no-index --stat $REPO/test-diagrams $V/src/g$r/test-diagrams > $V/gates/base-tree-writes.txt 2>&1
  cd $V; rm -rf $V/target-gbase $V/src/g$r
else
  cd $REPO; export CARGO_TARGET_DIR=$V/target-gtip
  git rev-parse HEAD > $V/gates/head.txt; git status --porcelain > $V/gates/status-before.txt
  cargo test --release -j 6 --workspace --lib > $V/gates/lib.log 2>&1; echo "lib exit $?"
  cargo test --release -j 6 -p rustuml --test pipe_mode > $V/gates/pipe.log 2>&1; echo "pipe exit $?"
  for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
    cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$t.log 2>&1; echo "$t exit $?"
  done
  cp test-diagrams/no_oracle_failure_names.txt $V/gates/no_oracle_failure_names-tip.txt 2>/dev/null
  cp test-diagrams/golden_failure_names.txt $V/gates/golden_failure_names-tip.txt 2>/dev/null
  git status --porcelain > $V/gates/status-after.txt
  rm -rf $V/target-gtip
fi
echo GATES-DONE $1
