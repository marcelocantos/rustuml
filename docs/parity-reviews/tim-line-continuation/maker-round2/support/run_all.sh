#!/bin/bash
# Replay every cohort through each private build (unchanged replay.py/compare_cli; 4 modes; each case from its recorded cwd).
# Usage: run_all.sh BUILD...   (labels of $M/bin/rustuml-<label>)
set -u
M=$HOME/Library/Caches/rustuml-tim-continuation-round2-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-fc5b3c96
chain() {
  S=$1
  for c in bu bd ba ay az ax av at; do python3 -I $M/support/replay.py $c-$S $M/fresh-$c-manifest.json $M/bin/rustuml-$S $M/bin/compare_cli $M/runs/$c-$S $M/fresh-$c; done
  python3 -I $M/support/replay.py fresh-$S $M/fresh-manifest.json $M/bin/rustuml-$S $M/bin/compare_cli $M/runs/fresh-$S $M/fresh
  python3 -I $M/support/replay.py retained-$S $M/retained-manifest.json $M/bin/rustuml-$S $M/bin/compare_cli $M/runs/retained-$S $REPO
  python3 -I $M/support/replay.py golden-$S $M/golden-manifest.json $M/bin/rustuml-$S $M/bin/compare_cli $M/runs/golden-$S $REPO
}
mkdir -p $M/runs $M/logs
for S in "$@"; do chain $S > $M/logs/replay-$S.log 2>&1 & done
wait
echo ALL-DONE
