#!/bin/bash
# Re-run lib, parity_perturbations and harness_guards for a tree. Usage: gates2.sh <tree> <label> <target> <out>
set -u
T=$1; L=$2; TD=$3; O=$4
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
export CARGO_TARGET_DIR=$TD
cd "$T"
: > $O/$L.exit
[ "$L" = "tip" ] && { cargo test --release -j 6 --workspace --lib --no-fail-fast > $O/$L.lib.log 2>&1; echo "lib exit $?" >> $O/$L.exit; }
for t in parity_perturbations harness_guards; do
  cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $O/$L.$t.log 2>&1; echo "$t exit $?" >> $O/$L.exit
done
