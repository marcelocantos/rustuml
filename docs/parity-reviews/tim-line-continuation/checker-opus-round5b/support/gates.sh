#!/bin/bash
# Checker gates as run (each once, foreground). Tip 6fd071169 and merged base 1ad869dae in git-archive exports with
# test-diagrams/golden symlinked to the hydrated 9efdc12f submodule; one cargo-created CARGO_TARGET_DIR per tree, deleted after.
V=$HOME/Library/Caches/rustuml-opus-tim-r5-recheck
# tip (src/g6fd071169, CARGO_TARGET_DIR=$V/target-g6fd071169):
#   cargo test --release -j 8 -p rustuml-oracle --test {golden_no_oracle,golden_pairs,doc_claims,harness_guards,parity_perturbations} --no-fail-fast -- --nocapture
#   cargo test --release -j 8 -p rustuml --test pipe_mode
#   cargo test --release -j 8 --workspace --lib
#   cargo clippy --release -j 8 -p {rustuml,rustuml-parser,rustuml-render,rustuml-layout,rustuml-math,rustuml-oracle} -- -D warnings
# base (src/g1ad869dae, CARGO_TARGET_DIR=$V/target-g1ad869dae):
#   cargo test --release -j 8 -p rustuml-oracle --test {parity_perturbations,golden_no_oracle,golden_pairs,harness_guards} --no-fail-fast -- --nocapture
# rejection contract (unchanged rejection_contract.py), every fresh cohort x {1ad869dae,9ed52f435,28af44ab3}:
#   python3 -I support/rejection_contract.py $D $M bin/rustuml-$B bin/rustuml-6fd071169 rejection/$c-$B.json
