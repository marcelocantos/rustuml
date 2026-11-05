#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round7-claude-checker
echo "sweep start $(date)" >> $C/logs/run_all.txt
bash $C/tools/sweep3.sh > $C/logs/sweep3.log 2>&1
echo "sweep done $(date)" >> $C/logs/run_all.txt
bash $C/tools/gates_all.sh > $C/logs/gates_all.log 2>&1
echo "gates done $(date)" >> $C/logs/run_all.txt
bash $C/tools/clippy_json.sh > $C/logs/clippy_json.log 2>&1
python3 -I $C/tools/clippy_list.py $C/gates $C/gates/clippy-diags.json > $C/logs/clippy_list.txt 2>&1
echo "clippy done $(date)" >> $C/logs/run_all.txt
python3 -I $C/tools/drift_drill7.py $C/src/b0ca0a7dc $C/src/drift-b0ca0a7dc $C/target/drift $C/drift ~/Library/Caches/rustuml-relay-parent/shared-svg-round7-mm-font-hash/output.txt > $C/logs/drift.log 2>&1
echo "drift done $(date)" >> $C/logs/run_all.txt
