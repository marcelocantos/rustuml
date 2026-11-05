#!/bin/bash
# Per-crate clippy (-D warnings, --no-deps) with JSON diagnostics over exported trees; the crate root is
# touched first so a cached dependency check cannot mask lints. Usage: clippy_json.sh <tree>...
C=~/Library/Caches/rustuml-shared-svg-round8-claude-maker
for r in "$@"; do
  cd $C/src/$r || exit 1
  for k in rustuml-render rustuml-parser rustuml-layout rustuml-oracle rustuml rustuml-math; do
    d=crates/$k/src; for f in $d/lib.rs $d/main.rs; do [ -f $f ] && touch $f; done
    CARGO_TARGET_DIR=$C/target/clippy-$r cargo clippy -j 6 -p $k --no-deps --message-format=json -- -D warnings > $C/gates/$r.clippy-$k.json 2>/dev/null
    echo "$r $k exit $?"
  done
done
