#!/bin/bash
# Run each broad gate once for a tree. Usage: gates.sh <tree_dir> <label> <target_dir> <out_dir>
set -u
T=$1; L=$2; TD=$3; O=$4
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
export CARGO_TARGET_DIR=$TD
cd "$T"
: > $O/$L.exit
cargo test --release -j 4 --workspace --lib --no-fail-fast > $O/$L.lib.log 2>&1; echo "lib exit $?" >> $O/$L.exit
cargo test --release -j 4 -p rustuml --test pipe_mode -- --nocapture > $O/$L.pipe_mode.log 2>&1; echo "pipe_mode exit $?" >> $O/$L.exit
for t in golden_no_oracle golden_pairs parity_perturbations harness_guards doc_claims; do
  cargo test --release -j 4 -p rustuml-oracle --test $t -- --nocapture > $O/$L.$t.log 2>&1; echo "$t exit $?" >> $O/$L.exit
done
