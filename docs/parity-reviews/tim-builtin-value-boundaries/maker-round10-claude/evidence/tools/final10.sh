#!/bin/bash
# Final serial run at the tip: gates (tip all; merge lib/pipe_mode/doc_claims), clippy per crate (merge, tip), routing drill on a tip archive.
C=~/Library/Caches/rustuml-shared-svg-round10-claude-maker; W=/Users/marcelo/.t3/worktrees/rustuml/t3-839e94d7
source <(sed -n '/^run()/,/^}/p' $C/tools/gates10.sh)
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
mkdir -p $C/gates/prev && mv $C/gates/tip.* $C/gates/prev/ 2>/dev/null; rm -f $C/gates/tip.exit
run tip lib pipe_mode golden_no_oracle golden_pairs parity_perturbations harness_guards doc_claims
for n in no_oracle_failure_names golden_failure_names; do cp $C/src/tip/test-diagrams/$n.txt $C/gates/tip.$n.txt; done
run merge lib pipe_mode doc_claims
python3 -I $C/tools/gatelists.py $C/gates tip; python3 -I $C/tools/gatelists.py $C/gates merge
mkdir -p $C/gates && bash $C/tools/clippy_json.sh merge tip > $C/gates/clippy.out 2>&1
python3 -I $C/tools/clippy_list.py $C/gates $C/gates/clippy-diags.json merge tip > $C/gates/clippy-summary.txt
rm -rf $C/src/drill10t && mkdir -p $C/src/drill10t && git -C $W archive HEAD | tar -x -C $C/src/drill10t
python3 -I $C/tools/routing_drill10.py $C/src/drill10t $C/work/drill10t > $C/work/drill10t.out 2>&1
echo FINAL DONE
