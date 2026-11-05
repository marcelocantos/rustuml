#!/bin/bash
# Round-10 gates over exported trees (golden submodule symlinked). Usage: gates10.sh
C=~/Library/Caches/rustuml-shared-svg-round10-claude-maker
export PLANTUML_STDLIB=$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib
run() { # tree label test...
  local T=$1; shift
  for t in "$@"; do
    case $t in
      lib) (cd $C/src/$T && CARGO_TARGET_DIR=$C/target/$T cargo test --release -j 6 --workspace --lib --no-fail-fast > $C/gates/$T.lib.log 2>&1); echo "lib exit $?" >> $C/gates/$T.exit ;;
      pipe_mode) (cd $C/src/$T && CARGO_TARGET_DIR=$C/target/$T cargo test --release -j 6 -p rustuml --test pipe_mode -- --nocapture > $C/gates/$T.pipe_mode.log 2>&1); echo "pipe_mode exit $?" >> $C/gates/$T.exit ;;
      *) (cd $C/src/$T && CARGO_TARGET_DIR=$C/target/$T cargo test --release -j 6 -p rustuml-oracle --test $t -- --nocapture > $C/gates/$T.$t.log 2>&1); echo "$t exit $?" >> $C/gates/$T.exit ;;
    esac
  done
}
rm -f $C/gates/{tip,merge,base}.exit
run tip lib pipe_mode golden_no_oracle golden_pairs parity_perturbations harness_guards doc_claims
for n in no_oracle_failure_names golden_failure_names; do cp $C/src/tip/test-diagrams/$n.txt $C/gates/tip.$n.txt 2>/dev/null; done
run merge golden_no_oracle golden_pairs parity_perturbations harness_guards
for n in no_oracle_failure_names golden_failure_names; do cp $C/src/merge/test-diagrams/$n.txt $C/gates/merge.$n.txt 2>/dev/null; done
run base golden_no_oracle
for n in no_oracle_failure_names golden_failure_names; do cp $C/src/base/test-diagrams/$n.txt $C/gates/base.$n.txt 2>/dev/null; done
for r in tip merge; do python3 -I $C/tools/gatelists.py $C/gates $r; done
echo GATES DONE
