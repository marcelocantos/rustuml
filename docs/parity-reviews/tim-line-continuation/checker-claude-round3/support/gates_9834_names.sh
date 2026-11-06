#!/bin/bash
# Rerun of the two 9834a7194 gate tests to keep their failure-name files (written inside the tree), then clean up.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-round3-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-a7fabc1d
r=9834a7194; rm -rf $V/src/$r; mkdir -p $V/src/$r; git -C $REPO archive $(git -C $REPO rev-parse $r) | tar -x -C $V/src/$r
rm -rf $V/src/$r/test-diagrams/golden; ln -s $REPO/test-diagrams/golden $V/src/$r/test-diagrams/golden
cd $V/src/$r
for t in golden_no_oracle golden_pairs; do
  CARGO_TARGET_DIR=$V/target-g9834 cargo test --release -j 8 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base9834-$t-rerun.log 2>&1; echo "base9834 $t exit $?"
done
cp test-diagrams/no_oracle_failure_names.txt $V/gates/no_oracle_failure_names-9834.txt
cp test-diagrams/golden_failure_names.txt $V/gates/golden_failure_names-9834.txt
cd $V; rm -rf $V/target-g9834 $V/src/$r
