#!/bin/bash
# Whole-output CLI sweep of all goldens and perturbations (recorded cwd) for each tree; then strict compare + cohorts.
C=/Users/marcelo/Library/Caches/rustuml-shared-svg-round11-claude-maker; R=/Users/marcelo/.t3/worktrees/rustuml/t3-6998b799
for s in "$@"; do
  rm -rf $C/work/sweep-$s; mkdir -p $C/work/sweep-$s
  python3 -I $C/tools/sweep.py $C/bin/rustuml-$s $C/work/sweep-$s $R $C/work/cwd_map.json test-diagrams/golden test-diagrams/perturbations
done
echo SWEEP DONE
