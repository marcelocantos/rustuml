#!/bin/bash
# Fresh git-archive release builds, one private cargo-created CARGO_TARGET_DIR per tree; deleted after the binary is saved.
set -e
V=$HOME/Library/Caches/rustuml-opus-tim-r5-recheck
R=/Users/marcelo/.t3/worktrees/rustuml/t3-18babc2b
build_one() {
  r=$1
  full=$(git -C $R rev-parse $r)
  rm -rf $V/src/$r; mkdir -p $V/src/$r
  paths="Cargo.toml Cargo.lock crates vendor"; git -C $R cat-file -e $full:third_party 2>/dev/null && paths="$paths third_party"
  git -C $R archive $full -- $paths | tar -x -C $V/src/$r
  export CARGO_TARGET_DIR=$V/target-$r
  (cd $V/src/$r && cargo build --release -j 7 -p rustuml 2>&1 | tail -2)
  cp $CARGO_TARGET_DIR/release/rustuml $V/bin/rustuml-$r
  if [ $r = 28af44ab3 ]; then
    (cd $V/src/$r && cargo build --release -j 7 -p rustuml-oracle --lib 2>&1 | tail -1)
    lib=$(ls -t $CARGO_TARGET_DIR/release/deps/librustuml_oracle-*.rlib | head -1)
    rustc --edition=2024 -O $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs \
      --extern rustuml_oracle=$lib -L dependency=$CARGO_TARGET_DIR/release/deps -o $V/bin/compare_cli
  fi
  rm -rf $CARGO_TARGET_DIR $V/src/$r
  echo "built $r $full"
}
( build_one 6fd071169; build_one 9ed52f435 ) > $V/logs/build-a.log 2>&1 &
( build_one 1ad869dae; build_one 28af44ab3 ) > $V/logs/build-b.log 2>&1 &
wait
cat $V/logs/build-a.log $V/logs/build-b.log
shasum -a 256 $V/bin/* $R/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs
echo BUILD-DONE
