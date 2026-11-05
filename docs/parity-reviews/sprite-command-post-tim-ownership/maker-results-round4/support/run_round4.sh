# Round-4 replay as run (maker). C = ~/Library/Caches/rustuml-sprite-round4-claude-maker; REPO = this worktree.
# Before = rustuml-044f73657 (merged base, code == 607890d0d). After = rustuml-d322dec66 (round-4 tip code).
# An earlier after build (df1c3f096, first draft) was replayed on AX/AV/AT/fresh/retained and superseded by
# d322dec66 (theme seed source); its runs are not used.
set -u
C=$HOME/Library/Caches/rustuml-sprite-round4-claude-maker; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-f9352ba8
for S in 044f73657 d322dec66; do
  for c in ax av at; do python3 -I $C/support/replay.py $c-$S $C/fresh-$c-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/$c-$S $C/fresh-$c; done
  python3 -I $C/support/replay.py fresh-$S $C/fresh-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/fresh-$S $C/fresh
  python3 -I $C/support/replay.py retained-$S $C/retained-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/retained-$S $REPO
  python3 -I $C/support/replay.py golden-$S $C/golden-manifest.json $C/bin/rustuml-$S $C/bin/compare_cli $C/runs/golden-$S $REPO
done
# Classification: fresh_classify.py (AX/AV/AT/fresh), classify.py + model_changes.py (retained/golden),
# rejection_contract.py on fresh, fresh-at, fresh-av, fresh-ax (before 044f73657, after d322dec66).
