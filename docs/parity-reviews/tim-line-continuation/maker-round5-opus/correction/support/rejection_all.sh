#!/bin/bash
# Unchanged rejection_contract.py per fresh cohort: tip against the merged base and both parents.
V=$HOME/Library/Caches/rustuml-opus-tim-r5; TIP=$1; mkdir -p $V/rejection
for c in dd df cc cb ca bz by bw bu bd ba ay az ax av at fresh; do
  if [ $c = fresh ]; then M=$V/fresh-manifest.json; D=$V/fresh; else M=$V/fresh-$c-manifest.json; D=$V/fresh-$c; fi
  for B in 1ad869dae 9ed52f435 28af44ab3; do
    python3 -I $V/support/rejection_contract.py $D $M $V/bin/rustuml-$B $V/bin/rustuml-$TIP $V/rejection/$c-$B.json > $V/rejection/$c-$B.stdout 2>&1
    python3 -I -c "import json,sys;d=json.load(open('$V/rejection/$c-$B.json'));print('$c','$B',d['entries'],d['before_rejections'],'->',d['after_rejections'],'introduced',d['introduced_losses'])"
  done
done
