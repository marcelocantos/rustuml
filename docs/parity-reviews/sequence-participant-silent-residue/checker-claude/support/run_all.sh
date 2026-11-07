set -u
R=/Users/marcelo/Library/Caches/rustuml-sprite-ownership-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-991a0cd7
for s in before after; do python3 -I $R/support/replay.py fresh-$s $R/fresh-manifest.json $R/bin/rustuml-$s $R/bin/compare_cli $R/runs/fresh-$s $R/fresh; done
for s in before after; do python3 -I $R/support/replay.py retained-$s $R/retained-manifest.json $R/bin/rustuml-$s $R/bin/compare_cli $R/runs/retained-$s $REPO; done
for s in before after; do python3 -I $R/support/replay.py golden-$s $R/golden-manifest.json $R/bin/rustuml-$s $R/bin/compare_cli $R/runs/golden-$s $REPO; done
echo ALLDONE
