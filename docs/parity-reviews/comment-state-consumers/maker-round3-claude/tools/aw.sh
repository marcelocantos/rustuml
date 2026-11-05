#!/bin/bash
# aw.sh <bin> <outdir> <srcdir> <refdir> [names...] : run cases from srcdir (cwd = srcdir) and strict-compare
B=$1; OUT=$2; SRC=$3; REF=$4; shift 4
mkdir -p $OUT; : > $OUT/list
names="$@"; [ -z "$names" ] && names=$(cd $SRC && ls *.puml | sed 's/\.puml$//')
for n in $names; do
  (cd $SRC && RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100 $B -pipe -tsvg < $n.puml > $OUT/$n.svg 2> $OUT/$n.err; echo $? > $OUT/$n.exit)
  printf "%s\t%s\n" $REF/$n.svg $OUT/$n.svg >> $OUT/list
done
~/Library/Caches/rustuml-shared-svg-round3-claude-maker/checker-cmp $OUT/list | awk -F'\t' '{n=$1; sub(".*/","",n); printf "%-40s %s %s\n", n, $2, $3}'
