#!/bin/bash
C=$HOME/Library/Caches/rustuml-opus-tim-r6-recheck
b() { # name srcdir
  ( cd "$2" && CARGO_TARGET_DIR=$C/target-$1 cargo build --release -j 6 -p rustuml --bin rustuml > $C/build-$1.log 2>&1 && cp $C/target-$1/release/rustuml $C/bin/rustuml-$1 && echo "built $1" )
}
b 7e13632cd /Users/marcelo/.t3/worktrees/rustuml/t3-dfbdde11 &
b c25ccbf3c $C/src/gc25ccbf3c &
wait
b 9ed52f435 $C/src/g9ed52f435 &
b 9fc567332 $C/src/g9fc567332 &
wait
ls -la $C/bin
