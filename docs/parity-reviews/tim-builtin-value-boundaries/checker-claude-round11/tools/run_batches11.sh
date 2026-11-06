#!/bin/bash
W=~/Library/Caches/rustuml-shared-svg-round11-claude-checker; P=~/Library/Caches/rustuml-relay-parent
for pair in AR:shared-svg-claude-batch1 AU:shared-svg-checker-c1 AW:shared-svg-round2-r2a BB:shared-svg-round3-r3a BC:shared-svg-round3-c3a BE:shared-svg-round4-r4a BF:shared-svg-round4-c4a BG:shared-svg-round5-m5a BH:shared-svg-round5-c5a BI:shared-svg-round6-r6a BJ:shared-svg-round6-k6a BK:shared-svg-round7-l7a BL:shared-svg-round7-c7a BM:shared-svg-round7-c7b BN:shared-svg-round8-m8a BO:shared-svg-round8-c8a BP:shared-svg-round9-m9a BQ:shared-svg-round9-m9b BR:shared-svg-round9-c9a BS:shared-svg-round10-m10a BT:shared-svg-round10-c10a; do
  l=${pair%%:*}; d=${pair#*:}
  python3 -I $W/tools/batch.py $W/work $P/$d $l $W/bin/rustuml-5ddf7733c=base $W/bin/rustuml-879d166f4=merge $W/bin/rustuml-97830bd1e=tip > $W/work/batch-$l.txt 2>&1; echo "$l exit $?"
done
echo BATCHES DONE
