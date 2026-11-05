set -u
C=/Users/marcelo/Library/Caches/rustuml-sprite-round4-claude-checker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-388b0a83
chain() {
  S=$1
  for c in ay az ax av at; do python3 -I $C/support/replay.py $c-$S $C/fresh-$c-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/$c-$S $C/fresh-$c; done
  python3 -I $C/support/replay.py fresh-$S $C/fresh-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/fresh-$S $C/fresh
  python3 -I $C/support/replay.py retained-$S $C/retained-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/retained-$S $REPO
  python3 -I $C/support/replay.py golden-$S $C/golden-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/golden-$S $REPO
}
for S in 93bf53e18 044f73657 c9e11493c; do chain $S > $C/logs/replay-$S.log 2>&1 & done
wait
echo ALL-DONE
