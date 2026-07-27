# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

RustUML is a Rust port of PlantUML — a tool that generates UML and non-UML diagrams from plain text descriptions. The goal is a single statically-linked binary with no runtime dependencies (no JVM, no Graphviz, no external fonts).

This is a **semantic rewrite**, not a line-by-line transliteration of the Java code. Use idiomatic Rust (enums with data, traits, pattern matching, ownership). The Java code is the oracle for *behavior*, not a template for *architecture*.

## Reference Implementation

The Java PlantUML at `~/work/github.com/plantuml/plantuml` serves as the oracle for testing. Build it with `gradle build -Pfast` to get a JAR, then use it to generate reference output for synthetic tests.

## Licensing

- RustUML own code: Apache 2.0
- Layout engine (`rustuml-layout`): Apache 2.0 Rust wrapper around vendored Graphviz layout code (EPL 2.0)
- KaTeX math (`rustuml-math`): MIT (wraps katex crate via QuickJS)
- Embedded font (Liberation Sans): SIL OFL

## Workspace Structure

```
crates/
  rustuml/          — binary (CLI entry point)
  rustuml-parser/   — PlantUML/YAML/JSON parsing, TIM preprocessor
  rustuml-render/   — SVG/PNG/PDF/EPS rendering, themes, creole markup
  rustuml-layout/   — hierarchical graph layout (wraps vendored Graphviz layout code)
  rustuml-math/     — LaTeX math rendering (wraps katex)
  rustuml-oracle/   — oracle test framework (generator, runner, comparator)
```

## Build and Test

```bash
cargo build
cargo test --lib          # unit tests only (fast, no server needed)
```

Golden pair tests require the PlantUML picoweb server running on port 8787:

```bash
scripts/plantuml-server.sh &   # starts on :8787
cargo test                     # includes golden pair validation
```

Override with `PLANTUML_URL=http://host:port` if needed.

Golden pairs live in `test-diagrams/golden/` (12,500+ .puml + .svg pairs).
Generate new ones with `scripts/generate-golden.sh` or `gen_*.py` scripts.

## Architecture Principles

- Single binary, no runtime dependencies
- Semantic rewrite using idiomatic Rust — not a Java transliteration
- Oracle-based testing: 12,500+ golden .puml/.svg pairs from Java PlantUML
- Two comparison tiers: exact match (parsing, preprocessing) and structural equivalence (layout — topologically correct, not pixel-identical)
- Layout via vendored Graphviz layout code with timeout guard for degenerate graphs
- 22 diagram types, 16 @start dispatch types, 6 output formats

## Parity Honesty Rules (🎯T14)

The project metric is **no-oracle product parity**: `cargo test --test golden_no_oracle --release` — what the CLI renders from source alone, ratcheted per family in `test-diagrams/no_oracle_baseline.txt`. The strict tier (`golden_pairs`) remains as a regression net for the oracle-assisted path. History note: both a May 2026 fan-out (verbatim golden replay, +5,990 tautological passes) and a June 2026 endgame (fixture-echo `include_str!` guards) gamed the strict tier under zero-failures pressure. These rules exist so that cannot recur.

1. **Port the generative model, not the output.** An SVG diff is a symptom, never the explanation. Before implementation, describe the causal divergence in terms of the Java model, including the named class/method, relevant data flow or invariant, and why Rust differs. If that account is missing, continue investigating; never tune variables merely because doing so shrinks a diff.
2. **Forbidden patterns**: embedding or reading golden data from shipping crates; conditionals keyed to fixture label text; per-branch-count constant ladders; post-hoc SVG string surgery keyed to coordinates. `tests/harness_guards.rs` enforces the mechanical subset; reviewers enforce the rest.
3. **Constant provenance**: every new or changed parity-affecting layout constant, threshold, metrics-table entry, or branch condition requires an adjacent Java-source citation or checked-in derivation, regardless of numeric precision. High-precision constants (3+ decimal places) additionally trigger the decimal-literal ratchet; its baseline update and the provenance comment are reviewed together.
4. **Ratchet workflow**: baseline changes ride the same change as the work that caused them (`HARNESS_WRITE_BASELINES=1` regenerates). Improvements not locked in fail the suite by design.
5. **Target-writing**: no per-fixture targets. Acceptance criteria name an algorithm or behavior and include at least one perturbation input NOT in the golden corpus (renamed labels, different branch counts, deeper nesting).
6. **Attestation**: completion claims cite the two tier numbers and green guards, never the executor's say-so.
7. **Frozen without user sign-off**: comparator slack (`GEOM_EPS`), `golden_has_syntax_error` markers, the park list, the error-golden census, and the guard allow-lists.
8. **Durable causal account**: before code changes begin, commit `docs/parity-reviews/<mechanism>/account.json` with the Java and Rust locations, claimed invariant, causal divergence, planned model change, and at least two relevant perturbation axes. The implementation diff references that earlier commit. A post-hoc explanation does not satisfy this gate.
9. **Adversarial maker/checker gate**: the implementer does not validate their own mechanism claim or author/edit its attestation. Before a parity change is accepted, a separate reviewer task that did not implement it, preferably using a different model family, is instructed to disprove it: inspect the Java provenance and Rust data flow, search for fixture-shaped logic and unexplained constants, and generate counterexamples the implementer did not use. The checker records its identity, model/tool, exact Rust and Java revisions, commands, inputs, findings, and verdict in `docs/parity-reviews/<mechanism>/review.json`.
10. **Held-out perturbations**: after implementation, the checker creates fresh valid PlantUML inputs exercising at least two independent axes relevant to the claimed invariant, such as label changes plus topology, declaration order, branch count, nesting, direction, aliasing, or style. Every generated input and same-stem Java SVG, passing or failing, is preserved under `test-diagrams/perturbations/<mechanism>/` and named in the review record. A failing counterexample rejects the change and remains executable; changing that rule requires explicit user sign-off and a convergence target. The no-oracle perturbation test must pass before acceptance.
11. **Mechanism-shaped evidence**: a legitimate fix should generalize across the mechanism's cluster or survive its perturbation matrix. Isolated gains receive extra scrutiny, and agents are assigned behavioral mechanisms rather than fixture names or pass-count quotas. Every parity-improving change gets its own checker record; T14's final attestation indexes those records rather than replacing them with retrospective review.

## Agent Guidance

- **Model policy**: Opus for cross-crate and layout-engine work (renderer geometry, FTile porting, de-oracling families — Sonnet compile-loops on these). Sonnet is fine for bounded single-crate mechanical tasks: harness/test code, fixture generation, lint scripts, docs, license notices. Fable handles phase gates, honesty audits, metric definitions, and comparator/skip-rule changes.
- The preprocessor (`preprocess/mod.rs`) is ~2900 lines — read it before editing.
- Test with `cargo test --lib` for fast iteration; golden tests (`cargo test --test golden_pairs`) for full validation; product truth via `cargo test --test golden_no_oracle`.

## Code Style

Standard Rust conventions. Use `cargo fmt` and `cargo clippy -- -D warnings`.

## Delivery

Merged to master.
