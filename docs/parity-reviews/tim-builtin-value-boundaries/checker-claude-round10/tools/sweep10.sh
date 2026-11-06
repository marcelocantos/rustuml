#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round10-claude-checker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-51190f64
for s in d06667af0 e24240b35; do
  rm -rf $C/work/sweep-$s; mkdir -p $C/work/sweep-$s
  python3 -I $C/tools/sweep.py $C/bin/rustuml-$s $C/work/sweep-$s $R $C/work/cwd_map.json test-diagrams/golden test-diagrams/perturbations
done
python3 - <<'PY'
import json, pathlib
C = pathlib.Path.home()/"Library/Caches/rustuml-shared-svg-round10-claude-checker"; R = pathlib.Path("/Users/marcelo/.t3/worktrees/rustuml/t3-51190f64")
with open(C/"work/cmp-list.tsv","w") as f:
    for s in ["95291dc6b","d06667af0","e24240b35"]:
        for k in sorted(json.load(open(C/f"work/sweep-{s}/status.json"))):
            f.write(f"{(R/k).with_suffix('.svg')}\t{(C/f'work/sweep-{s}'/k).with_suffix('.svg')}\n")
PY
$C/cmp/target/release/checker-cmp $C/work/cmp-list.tsv > $C/work/cmp.tsv
python3 -I $C/tools/cohorts.py $C/work $R $C/work/cohorts.json > $C/work/cohorts.txt
echo SWEEP DONE
