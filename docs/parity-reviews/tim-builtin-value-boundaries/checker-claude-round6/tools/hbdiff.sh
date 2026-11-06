#!/bin/bash
# Count changed lines vs 10.0.1 in the files the maker traced, per HarfBuzz tag.
H=$1; shift
for t in "$@"; do
  line="== $t:"
  for f in hb-ot-shape-fallback.cc hb-ot-shape.cc hb-ot-shaper-default.cc hb-ot-shaper-hebrew.cc hb-ot-shaper-arabic.cc hb-ot-shaper.hh hb-unicode.hh; do
    n=$(diff <(git -C "$H" show "10.0.1:src/$f") <(git -C "$H" show "${t}:src/$f" 2>/dev/null) | grep -c '^[<>]')
    line="$line $f:$n"
  done
  u=$(git -C "$H" show "${t}:src/hb-ucd-table.hh" 2>/dev/null | grep -m1 -oE 'ucd.py .*|UnicodeData|[0-9]+\.[0-9]+\.[0-9]+' | head -1)
  echo "$line ucd=$u"
done
