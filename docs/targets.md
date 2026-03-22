# Targets

<!-- last-evaluated: (none) -->

## Active

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies
- **Weight**: 1 (value 20 / cost 20)
- **Estimated-cost**: 20
- **Acceptance**:
  - A single statically-linked Rust binary accepts PlantUML text input and produces SVG/PNG output for all diagram types
  - No JVM, no Graphviz binary, no external font files required at runtime
  - Output is structurally equivalent to Java PlantUML for the same inputs
- **Context**: PlantUML's JVM dependency makes deployment painful. The current Java codebase has weak test coverage (~12%) and a tangled architecture. A Rust port solves deployment (single binary, cross-platform, WASM-ready) while enabling clean architecture. External dependencies (Graphviz layout, KaTeX math rendering) are ported into the binary. The current Java version serves as the oracle for synthetic test generation.
- **Status**: identified
- **Discovered**: 2026-03-22

### 🎯T1.1 Oracle-based test framework validates Rust output against Java PlantUML
- **Weight**: 20 (value 20 / cost 1)
- **Estimated-cost**: 1
- **Gates**: 🎯T1.3, 🎯T1.4, 🎯T1.5
- **Parent**: 🎯T1
- **Acceptance**:
  - A generator produces valid PlantUML inputs across all diagram types (random valid diagrams, edge cases, combinatorial feature coverage)
  - The Java PlantUML JAR runs each input and captures reference output (SVG, metadata)
  - A comparator checks Rust output against Java output with two tiers: exact match (parsing, preprocessing, metadata) and structural equivalence (layout — topologically correct, not pixel-identical)
  - The framework runs in CI, is fully automated, and scales to 10k+ test cases
  - New diagram types or features can be added to the generator without framework changes
- **Context**: The existing Java test suite has only 12% coverage and is mostly smoke tests. Synthetic generation with oracle comparison gives unbounded coverage and makes every porting step immediately verifiable. This is the first thing to build — all other sub-targets depend on it.
- **Status**: converging — 1,387 test cases (107 fixed + 1,280 proptest random) across 12 diagram types. CI green (GitHub Actions). Server-based runner. Remaining: scale fixed cases further as more diagram features are ported.
- **Discovered**: 2026-03-22

### 🎯T1.2 Hierarchical graph layout engine in Rust
- **Weight**: 4 (value 20 / cost 5)
- **Estimated-cost**: 5
- **Parent**: 🎯T1
- **Acceptance**:
  - Hierarchical (Sugiyama) layout produces node positions and edge routes from graph descriptions
  - Output is topologically correct for the graph inputs PlantUML generates (same nodes, edges, relative ordering — not pixel-identical to Graphviz)
  - Code is in its own crate (`rustuml-layout`), licensed Apache-2.0
  - No external Graphviz binary required
- **Context**: PlantUML uses Graphviz DOT for entity diagrams (class, component, object, deployment). Rather than porting ~43k lines of C/Java, we use layout-rs (MIT, 726 stars, full Sugiyama pipeline with spline routing) as the foundation and wrap it for PlantUML's needs. This gives us a working layout engine quickly and positions us to improve layout quality (line disambiguation, modern aesthetics) beyond what Graphviz provides.
- **Status**: converging — rustuml-layout crate created wrapping layout-rs (Sugiyama algorithm). Produces SVG with node positions and edge routes. Needs PlantUML-specific shape/style support and integration with rendering pipeline.
- **Discovered**: 2026-03-22

### 🎯T1.3 PlantUML parser and TIM preprocessor ported to Rust
- **Weight**: 2 (value 20 / cost 8)
- **Estimated-cost**: 8
- **Parent**: 🎯T1
- **Acceptance**:
  - TIM preprocessor handles variables, functions, control flow, includes, themes, JSON
  - Parser recognizes all diagram types and produces equivalent ASTs to Java version
  - Command pattern for each diagram type parses source lines into diagram models
  - Exact match with Java version on preprocessing and parsing (verified by oracle tests)
- **Context**: The parser is the largest component — ~40 diagram factories, ~79 command definitions, multi-pass parsing. The TIM preprocessor is a separate subsystem handling macros and includes. Parser correctness is verifiable by exact-match oracle tests (no layout involved).
- **Status**: converging — 4 diagram types parsed (sequence, class, state, activity, 54 tests). Preprocessor is pass-through stub. Binary accepts .puml files, outputs SVG. Next: TIM preprocessor, component/use-case parsers.
- **Discovered**: 2026-03-22

### 🎯T1.4 Diagram model and rendering pipeline ported to Rust
- **Weight**: 2 (value 20 / cost 8)
- **Estimated-cost**: 8
- **Parent**: 🎯T1
- **Acceptance**:
  - UGraphic abstraction supports SVG and PNG output
  - Diagram models (entities, links, groups, notes) render correctly via layout engine
  - Style/skin system applies themes and formatting
  - Output is structurally equivalent to Java version for all diagram types
- **Context**: The rendering pipeline has a clean abstraction (UGraphic interface with backends). SVG output is the primary target. PNG can use resvg/tiny-skia. The style system (CSS-like) and skin parameters need porting.
- **Status**: converging — SVG rendering for all 4 parsed diagram types (sequence, class, state, activity). SVG builder with rect, circle, polygon, diamond, text, line, groups. Grid-based class layout. Next: layout engine coordinate integration, PNG output, style/skin system.
- **Discovered**: 2026-03-22

### 🎯T1.5 KaTeX math rendering ported to Rust
- **Weight**: 2 (value 5 / cost 3)
- **Estimated-cost**: 3
- **Parent**: 🎯T1
- **Acceptance**:
  - LaTeX math notation in diagrams renders to SVG paths
  - Font metrics from KaTeX are embedded (no external font files)
  - Output visually matches KaTeX reference rendering
- **Context**: PlantUML uses JLatexMath for math notation. KaTeX (MIT, ~30k lines TypeScript) is the best porting source: clean lexer→parser→AST→renderer architecture, self-contained font metrics, renders to SVG. The KaTeX fonts (Computer Modern + AMS) need to be embedded in the binary.
- **Status**: identified
- **Discovered**: 2026-03-22

## Achieved
