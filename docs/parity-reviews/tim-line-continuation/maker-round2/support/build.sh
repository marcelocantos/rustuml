#!/bin/bash
# Private release builds: git archive REV into $M/src/REV, one CARGO_TARGET_DIR per tree.
# Usage: build.sh LABEL REV   (compare_cli is linked against 93bf53e18's rustuml_oracle)
set -e
M=$HOME/Library/Caches/rustuml-tim-continuation-round2-claude-maker
R=/Users/marcelo/.t3/worktrees/rustuml/t3-fc5b3c96
label=$1; full=$(git -C $R rev-parse $2)
rm -rf $M/src/$label; mkdir -p $M/src/$label
git -C $R archive $full | tar -x -C $M/src/$label
export CARGO_TARGET_DIR=$M/target-$label
(cd $M/src/$label && cargo build --release -j 4 -p rustuml 2>&1 | tail -2)
cp $CARGO_TARGET_DIR/release/rustuml $M/bin/rustuml-$label
if [ $label = 93bf53e18 ]; then
  (cd $M/src/$label && cargo build --release -j 4 -p rustuml-oracle --lib 2>&1 | tail -1)
  lib=$(ls -t $CARGO_TARGET_DIR/release/deps/librustuml_oracle-*.rlib | head -1)
  rustc --edition=2024 -O $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs \
    --extern rustuml_oracle=$lib -L dependency=$CARGO_TARGET_DIR/release/deps -o $M/bin/compare_cli
fi
echo "built $label $full $(shasum -a 256 $M/bin/rustuml-$label | cut -c1-64)"
