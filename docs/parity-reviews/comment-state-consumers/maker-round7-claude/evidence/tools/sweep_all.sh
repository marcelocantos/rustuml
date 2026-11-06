#!/bin/bash
# Whole-output sweep of goldens + perturbations for each tree, from recorded cwds; strict comparator; cohorts.
# Usage: sweep_all.sh <repo> <tree>... (last = tip). Binaries: $C/bin/rustuml-<tree>.
C=~/Library/Caches/rustuml-shared-svg-round7-claude-maker; R=$1; shift
python3 -I $C/tools/cwd_map.py $R $C/work/cwd_map.json
for s in "$@"; do
  if [ ! -f $C/work/sweep-$s/status.json ]; then
    rm -rf $C/work/sweep-$s; mkdir -p $C/work/sweep-$s
    python3 -I $C/tools/sweep.py $C/bin/rustuml-$s $C/work/sweep-$s $R $C/work/cwd_map.json test-diagrams/golden test-diagrams/perturbations
  fi
done
python3 -I - "$R" "$@" <<'PY'
import json, pathlib, sys
C = pathlib.Path.home()/"Library/Caches/rustuml-shared-svg-round7-claude-maker"; R = pathlib.Path(sys.argv[1])
with open(C/"work/cmp-list.tsv","w") as f:
    for s in sys.argv[2:]:
        for k in sorted(json.load(open(C/f"work/sweep-{s}/status.json"))):
            f.write(f"{(R/k).with_suffix('.svg')}\t{(C/f'work/sweep-{s}'/k).with_suffix('.svg')}\n")
PY
$C/cmp/target/release/maker-cmp $C/work/cmp-list.tsv > $C/work/cmp.tsv
python3 -I $C/tools/cohorts.py $C/work $R $C/work/cohorts.json "$@" > $C/work/cohorts.txt
echo SWEEP DONE
