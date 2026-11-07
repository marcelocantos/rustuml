# Usage: bash run_chain.sh BUILD  (replays every cohort through rustuml-BUILD; each from its private cwd)
set -u
C=$HOME/Library/Caches/rustuml-tim-continuation-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-eb7e6a90; S=$1
for c in ba ay az ax av at; do python3 -I $C/support/replay.py $c-$S $C/fresh-$c-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/$c-$S $C/fresh-$c; done
python3 -I $C/support/replay.py fresh-$S $C/fresh-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/fresh-$S $C/fresh
python3 -I $C/support/replay.py retained-$S $C/retained-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/retained-$S $REPO
python3 -I $C/support/replay.py golden-$S $C/golden-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/golden-$S $REPO
echo CHAIN-DONE $S
