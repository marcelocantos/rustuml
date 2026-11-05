#!/bin/bash
# Fresh git-archive builds of the four measured trees plus compare_cli (linked against 93bf53e18's rustuml_oracle).
set -e
V=$HOME/Library/Caches/rustuml-tim-continuation-claude-checker-v2
R=/Users/marcelo/.t3/worktrees/rustuml/t3-a69f53d7
export CARGO_TARGET_DIR=$V/target
for r in 93bf53e18 9834a7194 2a9373aca 594c2b3c5; do
  full=$(git -C $R rev-parse $r)
  rm -rf $V/src/$r; mkdir -p $V/src/$r
  git -C $R archive $full | tar -x -C $V/src/$r
  (cd $V/src/$r && cargo build --release -j 8 -p rustuml 2>&1 | tail -2)
  cp $CARGO_TARGET_DIR/release/rustuml $V/bin/rustuml-$r
  if [ $r = 93bf53e18 ]; then
    (cd $V/src/$r && cargo build --release -j 8 -p rustuml-oracle --lib 2>&1 | tail -1)
    lib=$(ls -t $CARGO_TARGET_DIR/release/deps/librustuml_oracle-*.rlib | head -1)
    rustc --edition=2024 -O $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs \
      --extern rustuml_oracle=$lib -L dependency=$CARGO_TARGET_DIR/release/deps -o $V/bin/compare_cli
  fi
  echo "built $r $full"
done
shasum -a 256 $V/bin/* $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs
