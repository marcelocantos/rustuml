#!/bin/bash
# Replay cohorts through one build (unchanged replay.py/compare_cli; 4 modes; 60 s per-case timeout; each case from its recorded cwd).
# Usage: run_chain.sh BUILD GROUP   GROUP = fresh | retained | golden
set -u
V=$HOME/Library/Caches/rustuml-opus-tim-r7-recheck; REPO=/Users/marcelo/.t3/worktrees/rustuml/t3-8d92c211; S=$1
mkdir -p $V/runs $V/logs
case $2 in
 fresh) for c in dn dm dl dk dj dh df dd cc cb ca bz by bw bu bd ba ay az ax av at; do python3 -I $V/support/replay.py $c-$S $V/fresh-$c-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/$c-$S $V/fresh-$c || echo FAIL $c; done
        python3 -I $V/support/replay.py fresh-$S $V/fresh-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/fresh-$S $V/fresh || echo FAIL fresh ;;
 retained) python3 -I $V/support/replay.py retained-$S $V/retained-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/retained-$S $REPO || echo FAIL retained ;;
 golden) python3 -I $V/support/replay.py golden-$S $V/golden-manifest.json $V/bin/rustuml-$S $V/bin/compare_cli $V/runs/golden-$S $REPO || echo FAIL golden ;;
esac
