#!/bin/bash
# cmp_run.sh <repo> <outdir> <tsv> : strict-compare every swept source against its Java golden
R=$1; O=$2; T=$3
python3 -I -c "
import json
st=json.load(open('$O/status.json'))
rels=sorted(st)
parts=[open('$O/cmp.part.%d'%i,'w') for i in range(6)]
for i,rel in enumerate(rels):
    parts[i%6].write('$R/'+rel[:-5]+'.svg\t'+'$O/'+rel[:-5]+'.svg\n')
"
for i in 0 1 2 3 4 5; do ~/Library/Caches/rustuml-shared-svg-round3-claude-maker/checker-cmp $O/cmp.part.$i > $O/cmp.part.$i.tsv & done; wait
cat $O/cmp.part.*.tsv > $T; rm -f $O/cmp.part.*
wc -l < $T
