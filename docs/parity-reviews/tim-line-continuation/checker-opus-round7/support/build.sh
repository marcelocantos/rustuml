#!/bin/bash
# One private cargo-created CARGO_TARGET_DIR per tree; binary saved to bin/, target deleted after.
C=$HOME/Library/Caches/rustuml-opus-tim-r7-recheck
TIP=/Users/marcelo/.t3/worktrees/rustuml/t3-8d92c211
b() { # name srcdir
  ( cd "$2" && CARGO_TARGET_DIR=$C/target-$1 cargo build --release -j 6 -p rustuml --bin rustuml > $C/logs/build-$1.log 2>&1 && cp $C/target-$1/release/rustuml $C/bin/rustuml-$1 && echo "built $1" || echo "BUILD FAIL $1" )
}
b df078f7d7 $TIP &
b 6d195c178 $C/src/g6d195c178 &
wait
b 9ed52f435 $C/src/g9ed52f435 &
b f6a738af1 $C/src/gf6a738af1 &
wait
# compare_cli: unchanged sequence-dependency-closure adapter, linked against f6a738af1's rustuml_oracle rlib
( cd $C/src/gf6a738af1 && CARGO_TARGET_DIR=$C/target-f6a738af1 cargo build --release -j 6 -p rustuml-oracle --lib >> $C/logs/build-f6a738af1.log 2>&1 )
RLIB=$(ls -t $C/target-f6a738af1/release/deps/librustuml_oracle-*.rlib | head -1)
rustc --edition=2024 -C opt-level=2 $C/src/gf6a738af1/docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs -L dependency=$C/target-f6a738af1/release/deps --extern rustuml_oracle=$RLIB -o $C/bin/compare_cli && echo built compare_cli
ls -la $C/bin; shasum -a 256 $C/bin/* > $C/binaries.sha256
