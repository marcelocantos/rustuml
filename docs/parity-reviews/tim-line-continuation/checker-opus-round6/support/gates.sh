#!/bin/bash
# Checker gates as run (each once, foreground). Tip = worktree at 7e13632cd (+ this review's pre-registration commit, docs only);
# merged base c25ccbf3c in a git-archive export with test-diagrams/golden symlinked to the hydrated 9efdc12f submodule.
# One cargo-created CARGO_TARGET_DIR per tree, deleted after.
V=$HOME/Library/Caches/rustuml-opus-tim-r6-recheck
# tip (CARGO_TARGET_DIR=$V/target-7e13632cd) and base (CARGO_TARGET_DIR=$V/target-c25ccbf3c):
#   cargo test --release -j 8 -p rustuml-oracle --test {golden_no_oracle,golden_pairs,harness_guards,parity_perturbations} --no-fail-fast -- --nocapture
# tip only:
#   cargo test --release -j 8 -p rustuml --test pipe_mode
#   cargo test --release -j 8 -p rustuml-oracle --test doc_claims
#   cargo test --release -j 8 --workspace --lib
#   cargo clippy --release -j 8 -p {rustuml,rustuml-parser,rustuml-render,rustuml-layout,rustuml-math,rustuml-oracle} -- -D warnings
# rejection contract (unchanged rejection_contract.py), 21 fresh cohorts x {c25ccbf3c,9ed52f435,9fc567332}:
#   python3 -I support/rejection_contract.py $FRESH_DIR $MANIFEST $V/bin/rustuml-$B $V/bin/rustuml-7e13632cd rejection/$c-$B.json
# replays: support/run_chain.sh BUILD {fresh,retained,golden}; summary: OVERLAY=1 support/summarize_q7.py; table: support/table_q7.py
