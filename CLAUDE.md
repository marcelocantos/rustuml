# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RustUML is a Rust port of PlantUML — a tool that generates UML and non-UML diagrams from plain text descriptions. The goal is a single statically-linked binary with no runtime dependencies (no JVM, no Graphviz, no external fonts).

This is a **semantic rewrite**, not a line-by-line transliteration of the Java code. Use idiomatic Rust (enums with data, traits, pattern matching, ownership). The Java code is the oracle for *behavior*, not a template for *architecture*.

## Reference Implementation

The Java PlantUML at `~/work/github.com/plantuml/plantuml` serves as the oracle for testing. Build it with `gradle build -Pfast` to get a JAR, then use it to generate reference output for synthetic tests.

## Licensing

- RustUML own code: Apache 2.0
- Layout engine (`rustuml-layout`): Apache 2.0 (wraps layout-rs, MIT)
- KaTeX-derived math code: MIT (when ported)

## Workspace Structure

```
crates/
  rustuml/          — binary (CLI entry point)
  rustuml-parser/   — PlantUML/YAML/JSON parsing, TIM preprocessor
  rustuml-render/   — SVG/PNG rendering, themes, creole markup
  rustuml-layout/   — hierarchical graph layout (wraps layout-rs)
  rustuml-oracle/   — oracle test framework (generator, runner, comparator)
```

## Build and Test

```bash
cargo build
```

Integration tests require the PlantUML picoweb server running on port 8787:

```bash
scripts/plantuml-server.sh &   # starts on :8787
cargo test
```

Override with `PLANTUML_URL=http://host:port` if needed.

## Architecture Principles

- Single binary, no runtime dependencies
- Semantic rewrite using idiomatic Rust — not a Java transliteration
- Oracle-based testing: synthetic PlantUML inputs run through Java PlantUML for reference output, compared against Rust output
- Two comparison tiers: exact match (parsing, preprocessing) and structural equivalence (layout — topologically correct, not pixel-identical)
- Layout via layout-rs (Sugiyama algorithm), not a Graphviz C port

## Code Style

Standard Rust conventions. Use `cargo fmt` and `cargo clippy`.

## Delivery

Merged to master.
