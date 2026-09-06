# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

The project metric is **shipped-binary product parity**: `make ratchet` — the built `rustuml` binary run over every eligible golden, scored by `compare_svg_strict`, gated by the honesty ratchet in `test-diagrams/ratchet-baseline.json` (`tools/ratchet/ratchet.py`; doctrine in `~/.claude/skills/oracle-first/honesty-ratchet.md`). It is the headline number and the only one produced by the artefact a user runs; it also recounts the denominator from the corpus tree, poisons the outputs to prove the scorer compares, and refuses a hand-edited baseline. Below it sit two regression nets, in decreasing product-truth: the in-process no-oracle tier (`cargo test --test golden_no_oracle --release`, ratcheted per family in `test-diagrams/no_oracle_baseline.txt`) and the oracle-assisted strict tier (`golden_pairs`). The two no-oracle numbers are close but not equal — at the 2026-09-06 lock the binary read 11,086/11,251 against the in-process tier's 11,088 — which is exactly why only the binary's number may be quoted as parity. History note: both a May 2026 fan-out (verbatim golden replay, +5,990 tautological passes) and a June 2026 endgame (fixture-echo `include_str!` guards) gamed the strict tier under zero-failures pressure. These rules exist so that cannot recur.

1. **Port the generative model, not the output.** Every geometry fix must trace to a named mechanism in the Java source (cite class/method) or an extracted metrics table. Never tune a constant merely because it shrinks a diff.
2. **Forbidden patterns**: embedding or reading golden data from shipping crates; conditionals keyed to fixture label text; per-branch-count constant ladders; post-hoc SVG string surgery keyed to coordinates. `tests/harness_guards.rs` enforces the mechanical subset; reviewers enforce the rest.
3. **High-precision constants** (3+ decimal places) require an adjacent provenance comment. The decimal-literal ratchet makes additions visible; the baseline update and the provenance comment are reviewed together.
4. **Ratchet workflow**: baseline changes ride the same change as the work that caused them. The in-process tier regenerates with `HARNESS_WRITE_BASELINES=1`; the shipped-binary baseline moves only through `python3 tools/ratchet/ratchet.py lock --reason ... --mechanism ...`, which re-runs every guard, refuses to lock while any is red, and hashes the numbers to the stated reason so a hand-edit is reported as PROVENANCE. Improvements not locked in fail by design, in both tiers.
5. **Target-writing**: no per-fixture targets. Acceptance criteria name an algorithm or behavior and include at least one perturbation input NOT in the golden corpus (renamed labels, different branch counts, deeper nesting).
6. **Attestation**: completion claims cite the two tier numbers and green guards, never the executor's say-so.
7. **Frozen without user sign-off**: comparator slack (`GEOM_EPS`), `golden_has_syntax_error` markers, the park list, the error-golden census, and the guard allow-lists.

## Agent Guidance

- **Model policy**: Opus for cross-crate and layout-engine work (renderer geometry, FTile porting, de-oracling families — Sonnet compile-loops on these). Sonnet is fine for bounded single-crate mechanical tasks: harness/test code, fixture generation, lint scripts, docs, license notices. Fable handles phase gates, honesty audits, metric definitions, and comparator/skip-rule changes.
- The preprocessor (`preprocess/mod.rs`) is ~2900 lines — read it before editing.
- Test with `cargo test --lib` for fast iteration; golden tests (`cargo test --test golden_pairs`) for full validation; product truth via `cargo test --test golden_no_oracle`.

## Code Style

Standard Rust conventions. Use `cargo fmt` and `cargo clippy -- -D warnings`.

## Gates

profile: base

## Delivery

Merged to master.
