# Broad gates at the review branch (tip code 594c2b3c5 + registered pairs), once each; plus no-oracle/golden_pairs at 9834a7194 for the name diff.
set -u
V=$HOME/Library/Caches/rustuml-tim-continuation-claude-checker-v2; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-a69f53d7
(
  cd $V/src/9834a7194 && rm -rf test-diagrams/golden && ln -s $REPO/test-diagrams/golden test-diagrams/golden
  for t in golden_no_oracle golden_pairs; do
    CARGO_TARGET_DIR=$V/target-98 cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/base9834-$t.log 2>&1; echo "base9834 $t exit $?"
  done
) &
cd $REPO; export CARGO_TARGET_DIR=$V/target-gates
git rev-parse HEAD > $V/gates/head.txt; git status --porcelain > $V/gates/status-before.txt
cargo test --release -j 8 --workspace --lib > $V/gates/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 8 -p rustuml --test pipe_mode > $V/gates/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
  cargo test --release -j 8 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $V/gates/$t.log 2>&1; echo "$t exit $?"
done
git status --porcelain > $V/gates/status-after.txt
wait
echo GATES-DONE
