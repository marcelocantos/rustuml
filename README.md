# RustUML

A Rust port of [PlantUML](https://plantuml.com) — generate UML and
non-UML diagrams from plain text. Single statically-linked binary, no
JVM, no Graphviz, no external fonts.

## Status

Pre-release. 18 diagram types supported, 12,500+ golden test pairs
passing against Java PlantUML reference output.

## Supported diagram types

| Type | Tag | Status |
|------|-----|--------|
| Sequence | `@startuml` | Good |
| Class | `@startuml` | Good |
| Activity (new syntax) | `@startuml` | Good |
| State | `@startuml` | Good |
| Component | `@startuml` | Good |
| Deployment | `@startuml` | Good |
| Use Case | `@startuml` | Good |
| Object | `@startuml` | Good |
| Timing | `@startuml` | Good |
| ER (crow's foot) | `@startuml` | Good |
| Gantt | `@startgantt` | Good |
| Mindmap | `@startmindmap` | Good |
| WBS | `@startwbs` | Good |
| JSON | `@startjson` | Good |
| YAML | `@startyaml` | Good |
| Salt (wireframes) | `@startsalt` | Good |
| Network (nwdiag) | `@startnwdiag` | Good |
| Regex (railroad) | `@startregex` | Good |
| Ditaa (ASCII art) | `@startditaa` | Basic |
| Math/LaTeX | `@startmath` / `@startlatex` | Good |

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
| `-ttxt` | ASCII art (sequence diagrams) |

### Other options

| Flag | Description |
|------|-------------|
| `--ast` | Print parsed AST |
| `--yaml` | Print diagram as YAML |
| `--theme=NAME` | Use built-in theme |
| `--theme-file=PATH` | Load theme from YAML file |
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

The golden test suite compares RustUML output against Java PlantUML
reference SVGs. The test files live in a separate repo added as a
submodule:

```bash
git submodule update --init    # Fetch golden test files
cargo test --test golden_pairs # Run golden comparison (~8s)
```

## Architecture

```
crates/
  rustuml/          — CLI binary
  rustuml-parser/   — PlantUML/YAML/JSON parsing, TIM preprocessor
  rustuml-render/   — SVG/PNG/PDF rendering, themes, creole markup
  rustuml-layout/   — Hierarchical graph layout (Sugiyama via layout-rs)
  rustuml-math/     — LaTeX math rendering
  rustuml-oracle/   — Oracle test framework
```

## Licence

Apache 2.0. See [LICENSE](LICENSE).

## Agent integration

If you use an agentic coding tool (Claude Code, Cursor, etc.), run
`rustuml --help-agent` for a guide to integrating RustUML into your
workflow.
