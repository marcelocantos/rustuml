set -u
C=/Users/marcelo/Library/Caches/rustuml-sprite-round4-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-388b0a83
cd $REPO; export CARGO_TARGET_DIR=$C/target-gates
git rev-parse HEAD > $C/gates/head.txt; git status --porcelain > $C/gates/status-before.txt
cargo test --release -j 6 --workspace --lib > $C/gates/lib.log 2>&1; echo "lib exit $?"
cargo test --release -j 6 -p rustuml --test pipe_mode > $C/gates/pipe.log 2>&1; echo "pipe exit $?"
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards; do
  cargo test --release -j 6 -p rustuml-oracle --test $t --no-fail-fast -- --nocapture > $C/gates/$t.log 2>&1; echo "$t exit $?"
done
echo GATES-DONE
