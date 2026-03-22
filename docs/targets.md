# Targets

<!-- last-evaluated: 0ec04e7 -->

## Active

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies
- **Weight**: 1 (value 20 / cost 20)
- **Estimated-cost**: 20
- **Acceptance**:
  - A single statically-linked Rust binary accepts PlantUML text input and produces SVG/PNG output for all diagram types
  - No JVM, no Graphviz binary, no external font files required at runtime
  - Output is structurally equivalent to Java PlantUML for the same inputs
- **Context**: PlantUML's JVM dependency makes deployment painful. The current Java codebase has weak test coverage (~12%) and a tangled architecture. A Rust port solves deployment (single binary, cross-platform, WASM-ready) while enabling clean architecture. External dependencies (Graphviz layout, KaTeX math rendering) are ported into the binary. The current Java version serves as the oracle for synthetic test generation.
- **Status**: converging — 7.6MB release binary with 7 diagram types, SVG+PNG output, themes, TIM preprocessor. 37 PRs merged, all CI green.
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
- **Status**: close — 145 golden files, 126 matrix cases, 1,280 proptest random, 107 fixed oracle, 19 error path tests, 5 preprocessor oracle tests. Systematic matrix framework with composable features. Golden file generator for offline testing. Remaining: scale golden files to thousands, add more diagram type matrices.
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
- **Status**: converging — layout engine called from class renderer, produces SVG. Currently using grid fallback for positioning. Remaining: extract actual Sugiyama coordinates from layout-rs output for proper hierarchical positioning.
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
- **Status**: close — 7 diagram types parsed (sequence, class, state, activity, component, use case, deployment). TIM preprocessor with !define, !ifdef/!ifndef/!if/!else, !include, !theme, !undef, comments. Score-based diagram type detection. YAML/JSON input. 88 parser tests. Remaining: !function/!procedure, !foreach/!while, built-in TIM functions, ~33 more diagram types (timing, gantt, mindmap, WBS, etc.).
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
- **Status**: close — All 7 parsed diagram types render to SVG and PNG. Theme system with 2 built-in + custom YAML themes. Skinparams override theme at render time. Creole markup (bold/italic/underline/strikethrough). Embedded Liberation Sans font for accurate text metrics. Proportional text sizing. 145 golden file tests. Remaining: layout coordinate extraction for class diagrams, additional diagram types.
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
- **Status**: identified — not started. Low priority since math in diagrams is rare.
- **Discovered**: 2026-03-22

### 🎯T1.6 YAML input format as alternative to PlantUML text syntax
- **Weight**: 1 (value 5 / cost 5)
- **Estimated-cost**: 5
- **Parent**: 🎯T1
- **Acceptance**:
  - Diagrams can be specified in YAML as an alternative to PlantUML text syntax
  - YAML representation maps cleanly to the diagram model (participants, messages, classes, etc.)
  - Generators (AI, scripts) can produce YAML instead of wrestling with PlantUML string syntax
  - Both formats produce identical diagrams
- **Context**: PlantUML's text syntax is designed for humans writing by hand. For machine generators (AI tools, CI pipelines, code-gen), a structured YAML format is easier to produce correctly — no escaping, no ambiguous syntax, no need to understand arrow notation. YAML deserializes directly into the diagram model types.
- **Status**: achieved — all model types derive Serialize/Deserialize. `rustuml --yaml` exports diagrams as YAML. YAML/JSON input auto-detected and parsed. Round-trip tests pass.
- **Discovered**: 2026-03-22

### 🎯T1.7 Multi-format output (PNG, PDF, EPS)
- **Weight**: 1 (value 10 / cost 5)
- **Estimated-cost**: 5
- **Parent**: 🎯T1
- **Acceptance**:
  - SVG is the primary/default output format (already implemented)
  - PNG output via resvg/tiny-skia (rasterize SVG)
  - Oracle test framework supports validating all output formats
  - Test suite runs against all supported formats, not just SVG
- **Context**: SVG is the development and testing format. PNG is needed for embedding in documents and wikis. The test framework should be parameterized by output format so every test case validates every format.
- **Status**: converging — SVG and PNG output working (-tsvg, -tpng). Golden files are SVG-only. Remaining: PDF output, PNG golden file comparison, format-parameterized test framework.
- **Discovered**: 2026-03-22

## Achieved

### 🎯T1.6 YAML input format ✓
Achieved 2026-03-22. All diagram model types serialize/deserialize via serde. Three input formats auto-detected.
