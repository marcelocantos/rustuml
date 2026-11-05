#!/bin/bash
# Sweep each named binary over goldens + perturbations, then compare every output with the strict comparator.
C=~/Library/Caches/rustuml-shared-svg-round6-claude-maker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-0998c624
cd $C/work
for t in "$@"; do
  rm -rf sweep-$t; mkdir -p sweep-$t
  python3 -I $C/tools/sweep.py $C/bin/rustuml-$t sweep-$t $R cwd_map.json test-diagrams/golden test-diagrams/perturbations > sweep-$t.log 2>&1
  python3 -I - "$t" <<'PY' > cmp-$t.lst
import json,sys,pathlib
t=sys.argv[1]; R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-0998c624')
st=json.load(open(f'sweep-{t}/status.json'))
for k in sorted(st):
    print(f"{R/pathlib.Path(k).with_suffix('.svg')}\tsweep-{t}/{pathlib.Path(k).with_suffix('.svg')}")
PY
  $C/cmp/target/release/checker-cmp cmp-$t.lst > cmp-$t.tsv
  echo "$t done $(wc -l < cmp-$t.tsv) $(grep -c "	MATCH	" cmp-$t.tsv)"
done
