# RustUML

A Rust port of [PlantUML](https://plantuml.com) — generate UML and
non-UML diagrams from plain text. Single statically-linked binary, no
JVM, no Graphviz, no external fonts.

## Status

Pre-release. 23 parsed diagram models are supported, but release-readiness
claims are based on the no-oracle product tier: 6,435/11,251 eligible SVG
goldens currently pass when rendered through the same path as the CLI (57.2%).
The strict oracle-assisted tier passes 11,022/11,251 eligible SVG goldens and
remains a regression net, not the headline product metric. The 1,199 Java
PlantUML error-page goldens are skipped by both tiers and are not counted as
passes.

## Supported diagram types

The table below is derived from `test-diagrams/no_oracle_baseline.txt`.

<!-- no-oracle-status:start -->
| Type | Tag | Baseline family | No-oracle product status |
|------|-----|-----------------|--------------------------|
| Sequence | `@startuml` | `sequence` | 1,400/1,411 (99.2%, partial) |
| Class | `@startuml` | `class` | 1,176/1,971 (59.7%, partial) |
| Archimate | `@startuml` | `archimate` | 0/48 (0.0%, none) |
| Activity (new syntax) | `@startuml` | `activity` | 1,266/1,299 (97.5%, partial) |
| State | `@startuml` | `state` | 1/901 (0.1%, partial) |
| Component | `@startuml` | `component` | 56/663 (8.4%, partial) |
| Deployment | `@startuml` | `deployment` | 38/495 (7.7%, partial) |
| Use Case | `@startuml` | `usecase` | 2/315 (0.6%, partial) |
| Object | `@startuml` | `object` | 48/155 (31.0%, partial) |
| Timing | `@startuml` | `timing` | 141/141 (100.0%, exact) |
| ER (crow's foot) | `@startuml` | `er` | 1/157 (0.6%, partial) |
| Gantt | `@startgantt` | `gantt` | 114/114 (100.0%, exact) |
| Mindmap | `@startmindmap` | `mindmap` | 157/157 (100.0%, exact) |
| WBS | `@startwbs` | `wbs` | 138/138 (100.0%, exact) |
| JSON/YAML | `@startjson` / `@startyaml` | `json-yaml` | 56/147 (38.1%, partial) |
| Salt (wireframes) | `@startsalt` | `salt` | 103/103 (100.0%, exact) |
| Network (nwdiag) | `@startnwdiag` | `nwdiag` | 89/89 (100.0%, exact) |
| Regex (railroad) | `@startregex` | `regex` | 45/45 (100.0%, exact) |
| EBNF | `@startebnf` | `ebnf` | 25/25 (100.0%, exact) |
| DOT | `@startdot` | `dot` | 25/25 (100.0%, exact) |
| Git | `@startgit` | `git` | 25/25 (100.0%, exact) |
| Board / Wire | `@startboard` | `wire` | 0/0 (no eligible SVG goldens) |
| Ditaa (ASCII art) | `@startditaa` | excluded | Excluded from SVG parity tiers; raster comparator pending |
| Math/LaTeX | `@startmath` / `@startlatex` | `math` | 50/50 (100.0%, exact) |
<!-- no-oracle-status:end -->

## Install

### From source

```bash
git clone https://github.com/marcelocantos/rustuml.git
cd rustuml
cargo build --release
# Binary at target/release/rustuml
```

### From releases

Download a pre-built binary from the
[releases page](https://github.com/marcelocantos/rustuml/releases).

## Usage

```bash
# File to SVG (writes input.svg alongside input.puml)
rustuml input.puml

# File to PNG
rustuml -tpng input.puml

# Pipe mode (stdin to stdout)
cat input.puml | rustuml -pipe -tsvg

# With theme
rustuml --theme=modern input.puml
```

### Output formats

| Flag | Format |
|------|--------|
| `-tsvg` | SVG (default) |
| `-tpng` | PNG |
| `-tpdf` | PDF |
| `-teps` | EPS |
| `-ttxt` | ASCII art (sequence diagrams) |

### Other options

| Flag | Description |
|------|-------------|
| `--ast` | Print parsed AST |
| `--yaml` | Print diagram as YAML |
| `--theme=NAME` | Use built-in theme |
| `--theme-file=PATH` | Load theme from YAML file |
| `--block=N` | Select block by 0-based index |
| `--block-name=NAME` | Select block by `@start... name` |
| `--version` | Print version |
| `--help` | Print usage |
| `--help-agent` | Print agent integration guide |

## Example

```
@startuml
actor User
participant "Web App" as app
database "User DB" as db

User -> app : Login
app -> db : SELECT user
db --> app : user record
app --> User : Welcome
@enduml
```

## Preprocessor

RustUML supports the PlantUML TIM preprocessor:

- `!define` / `!definelong` — macros with `##` token-paste
- `!ifdef` / `!ifndef` / `!if` / `!elseif` / `!else` / `!endif`
- `!function` / `!procedure` / `!return` / `!local`
- `!$variable` assignments with arithmetic
- `!while` / `!endwhile` loops
- `!include` / `!includesub` / `!startsub` / `!endsub`
- `!theme` — built-in theme loading
- `%strlen`, `%substr`, `%intval`, `%date`, `%is_defined`, and other builtins

## Development

```bash
cargo build          # Build
cargo test           # Run unit tests
cargo clippy         # Lint
cargo fmt            # Format
```

### Golden tests

There are two golden tiers:

- Strict oracle-assisted tier: `cargo test --test golden_pairs`. This compares
  against Java PlantUML reference SVGs while allowing test-only oracle layout
  extraction. Current state: 11,022/11,251 eligible SVG goldens pass, 229 fail,
  and 1,299 are skipped.
- No-oracle product tier: `cargo test --test golden_no_oracle --release`. This
  renders through the same source-only path the CLI uses. Current baseline:
  6,435/11,251 eligible SVG goldens pass.

The 1,199 Java PlantUML error-page goldens are skips, not successes, and do
not contribute to either headline pass count. The test files live in a
separate repo added as a submodule:

```bash
git submodule update --init    # Fetch golden test files
cargo test --test golden_pairs # Run golden comparison (~8s)
cargo test --test golden_no_oracle --release
```

## Architecture

```
crates/
  rustuml/          — CLI binary
  rustuml-parser/   — PlantUML/YAML/JSON parsing, TIM preprocessor
  rustuml-render/   — SVG/PNG/PDF/EPS rendering, themes, creole markup
  rustuml-layout/   — Hierarchical graph layout (vendored Graphviz layout code)
  rustuml-math/     — LaTeX math rendering
  rustuml-oracle/   — Oracle test framework
```

## Licence

RustUML's own code is Apache 2.0. See [LICENSE](LICENSE).
Bundled third-party components and assets are listed in [NOTICES](NOTICES).

## Agent integration

If you use an agentic coding tool (Claude Code, Cursor, etc.), run
`rustuml --help-agent` for a guide to integrating RustUML into your
workflow.
