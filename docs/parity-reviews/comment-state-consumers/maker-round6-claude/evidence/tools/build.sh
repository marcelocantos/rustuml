#!/bin/bash
# Private release build of a revision. Usage: build.sh <rev> [name]
set -e
C=~/Library/Caches/rustuml-shared-svg-round6-claude-maker
R=/Users/marcelo/.t3/worktrees/rustuml/t3-0998c624
rev=$1; name=${2:-$1}
d=$C/src/$name; rm -rf $d; mkdir -p $d
git -C $R archive $rev | tar -x -C $d
(cd $d && CARGO_TARGET_DIR=$C/target/$name cargo build --release -j 6 -p rustuml 2>&1 | tail -2)
cp $C/target/$name/release/rustuml $C/bin/rustuml-$name
echo "built $rev as $name $(shasum -a 256 $C/bin/rustuml-$name)"
