#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round7-claude-checker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-5706bf73
for s in 460570ba2 716762056 b0ca0a7dc; do
  rm -rf $C/work/sweep-$s; mkdir -p $C/work/sweep-$s
  python3 -I $C/tools/sweep.py $C/bin/rustuml-$s $C/work/sweep-$s $R $C/work/cwd_map.json test-diagrams/golden test-diagrams/perturbations
done
# comparator list: expected (repo .svg) vs actual
python3 - <<'PY'
import json, pathlib
C = pathlib.Path.home()/"Library/Caches/rustuml-shared-svg-round7-claude-checker"; R = pathlib.Path("/Users/marcelo/.t3/worktrees/rustuml/t3-5706bf73")
with open(C/"work/cmp-list.tsv","w") as f:
    for s in ["460570ba2","716762056","b0ca0a7dc"]:
        for k in sorted(json.load(open(C/f"work/sweep-{s}/status.json"))):
            f.write(f"{(R/k).with_suffix('.svg')}\t{(C/f'work/sweep-{s}'/k).with_suffix('.svg')}\n")
PY
$C/cmp/target/release/checker-cmp $C/work/cmp-list.tsv > $C/work/cmp.tsv
python3 -I $C/tools/cohorts.py $C/work $R $C/work/cohorts.json > $C/work/cohorts.txt
echo SWEEP DONE
