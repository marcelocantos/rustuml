# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

RustUML is a Rust port of PlantUML — a tool that generates UML and non-UML diagrams from plain text descriptions. The goal is a single statically-linked binary with no runtime dependencies (no JVM, no Graphviz, no external fonts).

This is a **semantic rewrite**, not a line-by-line transliteration of the Java code. Use idiomatic Rust (enums with data, traits, pattern matching, ownership). The Java code is the oracle for *behavior*, not a template for *architecture*.

## Reference Implementation

The Java PlantUML at `~/work/github.com/plantuml/plantuml` serves as the oracle for testing. Build it with `gradle build -Pfast` to get a JAR, then use it to generate reference output for synthetic tests.

## Licensing

- RustUML own code: Apache 2.0
- Layout engine (`rustuml-layout`): Apache 2.0 (wraps layout-rs, MIT)
- KaTeX math (`rustuml-math`): MIT (wraps katex crate via QuickJS)
- Embedded font (Liberation Sans): SIL OFL

## Workspace Structure

```
crates/
  rustuml/          — binary (CLI entry point)
  rustuml-parser/   — PlantUML/YAML/JSON parsing, TIM preprocessor
  rustuml-render/   — SVG/PNG/PDF/EPS rendering, themes, creole markup
  rustuml-layout/   — hierarchical graph layout (wraps layout-rs)
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
- Layout via layout-rs (Sugiyama algorithm) with timeout guard for degenerate graphs
- 22 diagram types, 16 @start dispatch types, 6 output formats

## Agent Guidance

- **Use opus for all team agents** on this project. Sonnet is too inefficient on this codebase — cross-cutting features touch multiple crates and sonnet gets stuck in compile-error loops.
- The preprocessor (`preprocess/mod.rs`) is ~2900 lines — read it before editing.
- Test with `cargo test --lib` for fast iteration; golden tests (`cargo test --test golden_pairs`) for full validation.

## Code Style

Standard Rust conventions. Use `cargo fmt` and `cargo clippy -- -D warnings`.

## Delivery

Merged to master.
