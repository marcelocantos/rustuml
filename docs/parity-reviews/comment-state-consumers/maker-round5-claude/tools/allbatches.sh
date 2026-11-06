#!/bin/bash
# Run every relayed batch through the given bins and print each batch's summary lines.
# Usage: allbatches.sh <bin>=<tag> ...   (EXTRA="LABEL:/abs/dir ..." adds batches)
P=$HOME/Library/Caches/rustuml-shared-svg-round5-claude-maker
RP=$HOME/Library/Caches/rustuml-relay-parent
for b in BF:$RP/shared-svg-round4-c4a BE:$RP/shared-svg-round4-r4a BC:$RP/shared-svg-round3-c3a \
         BB:$RP/shared-svg-round3-r3a AW:$RP/shared-svg-round2-r2a AU:$RP/shared-svg-checker-c1 \
         AR:$RP/shared-svg-claude-batch1 $EXTRA; do
  label=${b%%:*}; dir=${b#*:}
  python3 -I "$P/tools/batch.py" "$P/work" "$dir" "$label" "$@" > "$P/work/batch-$label.txt" 2>&1
  tail -n $# "$P/work/batch-$label.txt" | sed "s/^/$label /"
done
