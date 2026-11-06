#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round9-claude-checker
echo "batches start $(date)" >> $C/logs/run_all.txt
bash $C/tools/run_batches9.sh > $C/logs/batches.log 2>&1
echo "sweep start $(date)" >> $C/logs/run_all.txt
bash $C/tools/sweep9.sh > $C/logs/sweep.log 2>&1
echo "gates start $(date)" >> $C/logs/run_all.txt
bash $C/tools/gates9.sh > $C/logs/gates.log 2>&1
echo "clippy start $(date)" >> $C/logs/run_all.txt
bash $C/tools/clippy9.sh > $C/logs/clippy.log 2>&1
echo "all done $(date)" >> $C/logs/run_all.txt
