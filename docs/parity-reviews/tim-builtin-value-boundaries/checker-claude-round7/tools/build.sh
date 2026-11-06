#!/bin/bash
set -u
C=~/Library/Caches/rustuml-shared-svg-round7-claude-checker
for r in b0ca0a7dc 716762056 460570ba2; do
  (cd $C/src/$r && CARGO_TARGET_DIR=$C/target/$r cargo build --release -j 6 -p rustuml) > $C/logs/build-$r.log 2>&1
  echo "$r exit $?" >> $C/logs/build-status.txt
  cp $C/target/$r/release/rustuml $C/bin/rustuml-$r && shasum -a 256 $C/bin/rustuml-$r >> $C/logs/build-status.txt
done
