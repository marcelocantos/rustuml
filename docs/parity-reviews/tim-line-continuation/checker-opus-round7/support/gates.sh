#!/bin/bash
# Checker gates as run (each once, foreground, `timeout 590`). Tip = this worktree at df078f7d7 (+ the Q8 pre-registration
# commit c9423fbb9, docs only); merged base 6d195c178 in a git-archive export with test-diagrams/golden symlinked to the
# hydrated 9efdc12f submodule. One cargo-created CARGO_TARGET_DIR per tree, deleted after.
V=$HOME/Library/Caches/rustuml-opus-tim-r7-recheck
# tip (CARGO_TARGET_DIR=$V/target-tip) and base (CARGO_TARGET_DIR=$V/target-base):
#   cargo test --release -j 8 -p rustuml-oracle --test {golden_no_oracle,golden_pairs,harness_guards,parity_perturbations} --no-fail-fast -- --nocapture
# tip only:
#   cargo test --release -j 8 -p rustuml --test pipe_mode
#   cargo test --release -j 8 -p rustuml-oracle --test doc_claims
#   cargo test --release -j 8 --workspace --lib
#   cargo clippy --release -j 8 -p {rustuml,rustuml-parser,rustuml-render,rustuml-layout,rustuml-math,rustuml-oracle} -- -D warnings
# process-slip reproduction: git archive 50adb43e2; cargo test --release -j 8 -p rustuml-parser --lib (CARGO_TARGET_DIR=$V/target-slip)
# rejection contract (unchanged rejection_contract.py), 23 fresh cohorts x {6d195c178,9ed52f435,f6a738af1}:
#   python3 -I support/rejection_contract.py $FRESH_DIR $MANIFEST $V/bin/rustuml-$B $V/bin/rustuml-df078f7d7 rejection/$c-$B.json
# replays: support/run_chain.sh BUILD {fresh,retained,golden}; summary: OVERLAY=1 support/summarize_q8.py; table: support/table_q8.py
