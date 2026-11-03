# Sequence dependency closure maker packet

This is maker evidence, not a checker attestation. `summary.json` names full preimplementation account and production hashes. `dependencies.json` retains the two font controls and 22 broader Teoz lifecycle failures. No existing account, review, fixture, baseline, comparator or accepted registry implementation was edited in this continuation.

## Reproduction

Run from the isolated repository root. Set `CARGO_TARGET_DIR=/tmp/rustuml-seq-recovery/target`, `CARGO_BUILD_JOBS=4`, and `PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib`. The Java executable and source-pinned JAR are recorded in `reference.json`; generation uses the absolute Java executable with `-Djava.awt.headless=true -jar <jar> -tsvg -pipe -charset UTF-8`. Do not use the jenv shim. The rebuilt JAR is source-pinned, not historically byte-identical; historical font and Graphviz equivalence remains unproven.

```sh
cargo build --release -p rustuml -p rustuml-oracle --bin rustuml --example diff_one
cargo test --release -p rustuml-parser -p rustuml-render --lib
cargo test --release --no-fail-fast --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards
cargo clippy --release -p rustuml-parser -p rustuml-render -- -D warnings
rustfmt --edition 2024 --check crates/rustuml-render/src/sequence.rs crates/rustuml-parser/src/parse/sequence.rs
git diff --check
```

Copy the freshly built CLI and `examples/diff_one` into an isolated binary directory. Run the unchanged checker measurement entry point:

```sh
python3 docs/parity-reviews/sequence-grouped-note-origin/checker.py measure /tmp/new-sequence-isolated /tmp/sequence-binaries
python3 docs/parity-reviews/sequence-dependency-closure/maker-results/measure_fresh.py INPUT_DIRECTORY /tmp/new-sequence-fresh /tmp/sequence-binaries
```

`fresh/*/index.json` maps each exact source, reference and final CLI SVG and their SHA-256 hashes. The 535 fresh inputs were all generated through real pinned Java, then compared through unchanged `diff_one` in both modes. Every CLI result is byte-equivalent to the source-only path that was compared (after trimming final whitespace). The 874 corpus replay preserves per-input diagnostics, including all 7 pre-existing invalid Java inputs; no failed input was dropped.

`logs/closure-suites.log` is the final complete two-tier, guard and built-in perturbation run. Tests execute in Cargo's `crates/rustuml-oracle` package cwd; standalone CLI and isolated measurements execute in repository root. All sequence fresh inputs are include-free. Parent-only T14.6.2 and the additional 39 parent fixtures remain parent integration work.

The library suites pass (562 parser, 781 render). The aggregate command exits 101: frozen main ratchets and assisted failures remain, guards are 4/6 with 659 review diagnostics, and built-in perturbations retain 484 nonpasses. The decimal guard reports five unrelated files, no sequence addition. Clippy stops at the existing unused function in parser `parse/component.rs:72`, outside ownership. Owned rustfmt checks and `git diff --check` pass.

## Evidence lineage

The final replay supersedes intermediate maker counts while preserving their files. In particular, the Teoz account's initial `maker-results/current` contains intermediate output; use this packet's `fresh/teoz` for the complete 36/36 final replay. The original 78 cases improve from 66/78 at continuation start to 78/78. The original 41 checker cases improve from 31/41 to 39/41. Each new mechanism's before matrix is staged immediately before that mechanism, not an aggregate b1bb1cf benchmark.

`final-isolated-measurements.json` preserves the temporary combined-row regression; `logs/attached-suites.log` preserves the temporary half-head regressions. `isolated-874/measurements.json` and `logs/closure-suites.log` show both repaired. The final 874 replay is 390 product / 350 assisted passes, with 477 / 517 mismatches respectively, 7 invalid references and zero panics. There are no new parse/runtime errors or passing-input regressions against b1bb1cf.

No acceptance is claimed. Root must arrange a fresh independent checker and resolve the unwaived dependencies before accepting the sequence packet.
