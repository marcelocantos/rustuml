#!/bin/bash
# Private release builds: build_all.sh <tree>...  -> $C/bin/rustuml-<tree> (sha256 in $C/bin/binaries.sha256)
C=~/Library/Caches/rustuml-shared-svg-round8-claude-maker
for n in "$@"; do
  (cd $C/src/$n && CARGO_TARGET_DIR=$C/target/$n cargo build --release -j 6 -p rustuml > $C/work/build-$n.log 2>&1) || { echo "build $n FAILED"; exit 1; }
  cp $C/target/$n/release/rustuml $C/bin/rustuml-$n
done
(cd $C/cmp && CARGO_TARGET_DIR=$C/cmp/target cargo build --release -j 6 > $C/work/build-cmp.log 2>&1) || echo "cmp FAILED"
(cd $C/bin && shasum -a 256 rustuml-* > binaries.sha256 && cat binaries.sha256)
echo BUILDS DONE
