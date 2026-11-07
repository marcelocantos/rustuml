set -u
C=/Users/marcelo/Library/Caches/rustuml-sprite-round3-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-336f6851
for S in 93bf53e18 dea00b295 607890d0d; do
  python3 -I $C/support/replay.py av-$S $C/fresh-av-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/av-$S $C/fresh-av
  python3 -I $C/support/replay.py at-$S $C/fresh-at-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/at-$S $C/fresh-at
  python3 -I $C/support/replay.py fresh-$S $C/fresh-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/fresh-$S $C/fresh
done
for S in 93bf53e18 dea00b295 607890d0d; do
  python3 -I $C/support/replay.py retained-$S $C/retained-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/retained-$S $REPO
  python3 -I $C/support/replay.py golden-$S $C/golden-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/golden-$S $REPO
done
echo ALL-DONE
