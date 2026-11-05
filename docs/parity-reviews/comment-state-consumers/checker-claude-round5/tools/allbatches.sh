#!/bin/bash
C=~/Library/Caches/rustuml-shared-svg-round5-claude-checker; P=~/Library/Caches/rustuml-relay-parent
cd $C/work
for b in BF:shared-svg-round4-c4a BE:shared-svg-round4-r4a BC:shared-svg-round3-c3a BB:shared-svg-round3-r3a AW:shared-svg-round2-r2a AU:shared-svg-checker-c1 AR:shared-svg-claude-batch1; do
  n=${b%%:*}; d=${b#*:}
  python3 -I $C/tools/batch.py $C/work $P/$d $n $C/bin/rustuml-fbb2e59ed=base $C/bin/rustuml-d6f63a8aa=merge $C/bin/rustuml-8dda18b1b=tip > $C/work/batch-$n.txt 2>&1
  echo "$n done: $(tail -3 $C/work/batch-$n.txt | tr '\n' ' ')"
done
