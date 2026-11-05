#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round8-claude-checker; P=~/Library/Caches/rustuml-relay-parent
B="$C/bin/rustuml-5d8b1f7c1=base $C/bin/rustuml-d6e9ffd94=merge $C/bin/rustuml-c3116f0b0=tip"
for pair in AR:shared-svg-claude-batch1 AU:shared-svg-checker-c1 AW:shared-svg-round2-r2a BB:shared-svg-round3-r3a BC:shared-svg-round3-c3a BE:shared-svg-round4-r4a BF:shared-svg-round4-c4a BG:shared-svg-round5-m5a BH:shared-svg-round5-c5a BI:shared-svg-round6-r6a BJ:shared-svg-round6-k6a BK:shared-svg-round7-l7a BL:shared-svg-round7-c7a BM:shared-svg-round7-c7b BN:shared-svg-round8-m8a; do
  l=${pair%%:*}; d=${pair#*:}
  python3 -I $C/tools/batch.py $C/work $P/$d $l $B > $C/work/batch-$l.txt 2>&1; echo "$l exit $?"
done
echo BATCHES DONE
