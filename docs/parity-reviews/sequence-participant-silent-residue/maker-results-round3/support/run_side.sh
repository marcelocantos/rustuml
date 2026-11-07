set -u
# Usage: run_side.sh SIDE  (SIDE names bin/rustuml-SIDE)
R=/Users/marcelo/Library/Caches/rustuml-sprite-round3-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-9ff076c0; S=$1
python3 -I $R/support/replay.py fresh-$S $R/fresh-manifest.json $R/bin/rustuml-$S $R/bin/compare_cli $R/runs/fresh-$S $R/fresh
python3 -I $R/support/replay.py at-$S $R/fresh-at-manifest.json $R/bin/rustuml-$S $R/bin/compare_cli $R/runs/at-$S $R/fresh-at
python3 -I $R/support/replay.py retained-$S $R/retained-manifest.json $R/bin/rustuml-$S $R/bin/compare_cli $R/runs/retained-$S $REPO
python3 -I $R/support/replay.py golden-$S $R/golden-manifest.json $R/bin/rustuml-$S $R/bin/compare_cli $R/runs/golden-$S $REPO
echo SIDE-DONE $S
