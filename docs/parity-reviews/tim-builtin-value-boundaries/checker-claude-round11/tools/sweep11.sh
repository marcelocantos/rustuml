#!/bin/bash
W=~/Library/Caches/rustuml-shared-svg-round11-claude-checker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-1db06518
for s in 5ddf7733c 879d166f4 97830bd1e f7c382ff4; do
  rm -rf $W/work/sweep-$s; mkdir -p $W/work/sweep-$s
  python3 -I $W/tools/sweep.py $W/bin/rustuml-$s $W/work/sweep-$s $R $W/work/cwd_map.json test-diagrams/golden test-diagrams/perturbations
done
python3 - <<'PY'
import json, pathlib
W = pathlib.Path.home()/"Library/Caches/rustuml-shared-svg-round11-claude-checker"; R = pathlib.Path("/Users/marcelo/.t3/worktrees/rustuml/t3-1db06518")
with open(W/"work/cmp-list.tsv","w") as f:
    for s in ["5ddf7733c","879d166f4","97830bd1e"]:
        for k in sorted(json.load(open(W/f"work/sweep-{s}/status.json"))):
            f.write(f"{(R/k).with_suffix('.svg')}\t{(W/f'work/sweep-{s}'/k).with_suffix('.svg')}\n")
PY
$W/cmp/target/release/checker-cmp $W/work/cmp-list.tsv > $W/work/cmp.tsv
echo SWEEP DONE
