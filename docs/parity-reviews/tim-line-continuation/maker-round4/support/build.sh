#!/bin/bash
# Fresh git-archive builds, each with its OWN private CARGO_TARGET_DIR; compare_cli (unchanged source)
# linked against 93bf53e18's rustuml_oracle. Each target and export is deleted once its binary is saved.
# Usage: build.sh REV [REV ...]
set -e
V=$HOME/Library/Caches/rustuml-tim-continuation-round4-claude-maker
R=/Users/marcelo/.t3/worktrees/rustuml/t3-94686399
build_one() {
  r=$1
  full=$(git -C $R rev-parse $r)
  rm -rf $V/src/$r; mkdir -p $V/src/$r
  git -C $R archive $full -- Cargo.toml Cargo.lock crates vendor | tar -x -C $V/src/$r
  export CARGO_TARGET_DIR=$V/target-$r
  (cd $V/src/$r && cargo build --release -j 5 -p rustuml 2>&1 | tail -2)
  cp $CARGO_TARGET_DIR/release/rustuml $V/bin/rustuml-$r
  if [ $r = 93bf53e18 ]; then
    (cd $V/src/$r && cargo build --release -j 5 -p rustuml-oracle --lib 2>&1 | tail -1)
    lib=$(ls -t $CARGO_TARGET_DIR/release/deps/librustuml_oracle-*.rlib | head -1)
    rustc --edition=2024 -O $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs \
      --extern rustuml_oracle=$lib -L dependency=$CARGO_TARGET_DIR/release/deps -o $V/bin/compare_cli
  fi
  rm -rf $CARGO_TARGET_DIR $V/src/$r
  echo "built $r $full"
}
for r in "$@"; do build_one $r > $V/logs/build-$r.log 2>&1 & done
wait
cat $V/logs/build-*.log | grep built
