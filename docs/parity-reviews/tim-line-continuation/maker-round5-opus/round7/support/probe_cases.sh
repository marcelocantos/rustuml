#!/bin/bash
# probe_cases.sh BIN case... : compare_cli outcome for named DH/DJ/DF cases (cwd = cohort dir)
V=$HOME/Library/Caches/rustuml-opus-tim-r5; BIN=$1; shift
export RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100"
for n in "$@"; do
  for co in dl dk dj dh df dd; do [ -f $V/fresh-$co/$n.puml ] && break; done
  ( cd $V/fresh-$co && timeout 60 $BIN -tsvg -pipe < $n.puml > /tmp/pc.svg 2>/tmp/pc.err; e=$?; timeout 60 $V/bin/compare_cli $n.svg /tmp/pc.svg > /tmp/pc.cmp 2>&1; c=$?; printf '%-40s %s exit=%s cmp=%s %s\n' $n $co $e $c "$(head -c 90 /tmp/pc.err | tr '\n' ' ')" )
done
