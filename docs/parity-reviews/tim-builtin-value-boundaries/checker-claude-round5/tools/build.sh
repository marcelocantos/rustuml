#!/bin/bash
set -e
C=~/Library/Caches/rustuml-shared-svg-round5-claude-checker
R=/Users/marcelo/.t3/worktrees/rustuml/t3-2bb601af
for rev in fbb2e59ed d6f63a8aa 8dda18b1b; do
  d=$C/src/$rev; rm -rf $d; mkdir -p $d
  git -C $R archive $rev | tar -x -C $d
  (cd $d && CARGO_TARGET_DIR=$C/target/$rev cargo build --release -j 6 -p rustuml 2>&1 | tail -3)
  cp $C/target/$rev/release/rustuml $C/bin/rustuml-$rev
  echo "built $rev $(shasum -a 256 $C/bin/rustuml-$rev)"
done
