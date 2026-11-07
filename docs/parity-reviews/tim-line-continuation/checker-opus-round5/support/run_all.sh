#!/bin/bash
# Replay every cohort through each fresh build (unchanged replay.py/compare_cli; 4 modes; each case from its recorded cwd).
set -u
V=$HOME/Library/Caches/rustuml-opus-tim-r5-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-7758f731
chain() {
  S=$1
  for c in dd cc cb ca bz by bw bu bd ba ay az ax av at; do python3 -I $V/support/replay.py $c-$S $V/fresh-$c-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/$c-$S $V/fresh-$c; done
  python3 -I $V/support/replay.py fresh-$S $V/fresh-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/fresh-$S $V/fresh
  python3 -I $V/support/replay.py retained-$S $V/retained-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/retained-$S $REPO
  python3 -I $V/support/replay.py golden-$S $V/golden-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/golden-$S $REPO
}
mkdir -p $V/runs
for S in "$@"; do chain $S > $V/logs/replay-$S.log 2>&1 & done
wait
echo ALL-DONE
