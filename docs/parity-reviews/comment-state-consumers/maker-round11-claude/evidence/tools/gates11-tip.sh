#!/bin/bash
C=/Users/marcelo/Library/Caches/rustuml-shared-svg-round11-claude-maker
rm -f $C/gates/tip.exit
bash $C/tools/gates.sh $C/gsrc/tip tip $C/target/gates-tip $C/gates
echo GATES-TIP DONE
