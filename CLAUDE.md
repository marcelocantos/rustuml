# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RustUML is a Rust port of PlantUML — a tool that generates UML and non-UML diagrams from plain text descriptions. The goal is a single statically-linked binary with no runtime dependencies (no JVM, no Graphviz, no external fonts).

The port includes:
- PlantUML parser, preprocessor (TIM), and rendering pipeline
- Graphviz DOT layout algorithm (ported from C, licensed EPL-2.0)
- KaTeX math rendering (ported from TypeScript, licensed MIT)

## Reference Implementation

The Java PlantUML at `~/work/github.com/plantuml/plantuml` serves as the oracle for testing. Build it with `gradle build -Pfast` to get a JAR, then use it to generate reference output for synthetic tests.

## Licensing

- RustUML own code: Apache 2.0
- Graphviz-derived layout code: EPL-2.0 (must remain EPL-2.0, kept in separate modules)
- KaTeX-derived math code: MIT
- KaTeX fonts: MIT (upstream sources: public domain Computer Modern + SIL OFL AMS fonts)

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
- Oracle-based testing: synthetic PlantUML inputs are generated, run through Java PlantUML for reference output, then compared against Rust output
- Two comparison tiers: exact match (parsing, preprocessing, metadata) and structural equivalence (layout)
- Graphviz and KaTeX ports are isolated in their own crates with their upstream licenses

## Code Style

Standard Rust conventions. Use `cargo fmt` and `cargo clippy`.
