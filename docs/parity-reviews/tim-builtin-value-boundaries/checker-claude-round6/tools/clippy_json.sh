#!/bin/bash
# Per-crate clippy (-D warnings, --no-deps) with JSON diagnostics; touch the crate root so a cached dep check cannot mask lints.
C=~/Library/Caches/rustuml-shared-svg-round6-claude-checker
for r in 8317cf006 fc44ee58c 7e9b16f52; do
  cd $C/src/$r
  for k in rustuml-render rustuml-parser rustuml-layout rustuml-oracle rustuml rustuml-math; do
    d=crates/$k/src; for f in $d/lib.rs $d/main.rs; do [ -f $f ] && touch $f; done
    CARGO_TARGET_DIR=$C/target/clippy-$r cargo clippy -j 6 -p $k --no-deps --message-format=json -- -D warnings > $C/gates/$r.clippy-$k.json 2>/dev/null
    echo "$r $k exit $?"
  done
done
