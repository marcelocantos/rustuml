#!/bin/bash
# Fresh git-archive builds, one private CARGO_TARGET_DIR each; compare_cli (unchanged source) against 32684f81e's rustuml_oracle.
set -e
V=$HOME/Library/Caches/rustuml-opus-tim-r5
R=/Users/marcelo/.t3/worktrees/rustuml/t3-bc3abe07
build_one() {
  r=$1
  full=$(git -C $R rev-parse $r)
  rm -rf $V/src/$r; mkdir -p $V/src/$r
  git -C $R archive $full -- Cargo.toml Cargo.lock crates vendor third_party | tar -x -C $V/src/$r
  export CARGO_TARGET_DIR=$V/target-$r
  (cd $V/src/$r && cargo build --release -j 6 -p rustuml 2>&1 | tail -2)
  cp $CARGO_TARGET_DIR/release/rustuml $V/bin/rustuml-$r
  if [ $r = 32684f81e ]; then
    (cd $V/src/$r && cargo build --release -j 6 -p rustuml-oracle --lib 2>&1 | tail -1)
    lib=$(ls -t $CARGO_TARGET_DIR/release/deps/librustuml_oracle-*.rlib | head -1)
    rustc --edition=2024 -O $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs \
      --extern rustuml_oracle=$lib -L dependency=$CARGO_TARGET_DIR/release/deps -o $V/bin/compare_cli
  fi
  rm -rf $CARGO_TARGET_DIR $V/src/$r
  echo "built $r $full"
}
( build_one ea5375293; build_one 28af44ab3 ) > $V/logs/build2-a.log 2>&1 &
( build_one 1ad869dae ) > $V/logs/build2-b.log 2>&1 &
wait
cat $V/logs/build2-a.log $V/logs/build2-b.log
shasum -a 256 $V/bin/* $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs
echo BUILD-DONE
