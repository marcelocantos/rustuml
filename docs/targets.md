# Targets

<!-- last-evaluated: d7d3facb -->

## Active

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies
- **Weight**: 1 (value 20 / cost 20)
- **Estimated-cost**: 20
- **Acceptance**:
  - A single statically-linked Rust binary accepts PlantUML text input and produces SVG/PNG output for all diagram types
  - No JVM, no Graphviz binary, no external font files required at runtime
  - Output is structurally equivalent to Java PlantUML for the same inputs
- **Context**: PlantUML's JVM dependency makes deployment painful. The current Java codebase has weak test coverage (~12%) and a tangled architecture. A Rust port solves deployment (single binary, cross-platform, WASM-ready) while enabling clean architecture. External dependencies (Graphviz layout, KaTeX math rendering) are ported into the binary. The current Java version serves as the oracle for synthetic test generation.
- **Status**: converging — 22 diagram types parsed and rendered, 12,568 golden test pairs (0 failures), full TIM preprocessor, SVG+PNG+PDF+EPS output. 49 PRs merged, all CI green. Graphviz layout engine with bezier edge routing. Stdlib includes, archimate, hyperlinks, creole tables, ASCII renderers added.
- **Discovered**: 2026-03-22

### 🎯T1.2 Hierarchical graph layout engine in Rust
- **Weight**: 4 (value 20 / cost 5)
- **Estimated-cost**: 5
- **Parent**: 🎯T1
- **Acceptance**:
  - Hierarchical (Sugiyama) layout produces node positions and edge routes from graph descriptions
  - Output is topologically correct for the graph inputs PlantUML generates (same nodes, edges, relative ordering — not pixel-identical to Graphviz)
  - Edge routing avoids the converge/diverge line problem (lines that merge into a corridor and fan out, making connections untraceable — see `test-diagrams/wide-shallow-dense.puml`)
  - Code is in its own crate (`rustuml-layout`), licensed Apache-2.0
  - No external Graphviz binary required
- **Context**: PlantUML uses Graphviz DOT for entity diagrams (class, component, object, deployment). Vendored Graphviz C libraries (dot algorithm) are statically linked for layout, replacing the old layout-rs dependency. Edge routing uses cubic bezier splines extracted from Graphviz.
- **Status**: near-achieved — layout-rs replaced with vendored Graphviz (dot algorithm). Cubic bezier edge routing with proper spline extraction. Used by 8 renderers: class, object, component, deployment, usecase, state, activity, dot. Timeout guard (5s) with grid fallback. Remaining: edge routing quality tuning on dense graphs.
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
- **Context**: The parser is the largest component. The TIM preprocessor is a separate subsystem handling macros and includes. Parser correctness is verifiable by exact-match oracle tests.
- **Status**: near-achieved — 22 diagram types parsed. Full TIM preprocessor. Stdlib includes (`!include <C4/...>`, `!include <awslib/...>`) bundled and resolved. Archimate parsing. `!import` directive. Lenient JSON parser. Only 1 parse error in golden tests (mindmap edge case). Remaining: complex nested TIM macro edge cases.
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
- **Context**: The rendering pipeline has a clean abstraction. SVG output is the primary target. PNG via resvg/tiny-skia. The style system and skin parameters need full porting.
- **Status**: near-achieved — 22 diagram types render to SVG including archimate. Hyperlinks (`[[url]]`) wired into SVG output for class, sequence, component diagrams. Creole tables, tree lists, nested lists, horizontal rules. 183 skinparam keys wired. Sprite rendering. ASCII renderers for class, state, activity. Remaining: ~15% of skinparam keys not applied, deeper creole edge cases.
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
- **Context**: SVG is the development and testing format. PNG is needed for embedding in documents and wikis.
- **Status**: near-achieved — SVG, PNG (-tpng), PDF (svg2pdf), and EPS (-teps) output all working. Format-parameterized golden smoke tests added (`golden_formats.rs`) — validates PNG/PDF/EPS conversion does not crash and produces correct file headers. Remaining: golden comparison for non-SVG formats (currently smoke-only).
- **Discovered**: 2026-03-22

## Achieved

### 🎯T1.1 Oracle-based test framework ✓
Achieved 2026-03-26. 12,568 golden pairs across 30 categories and 18 diagram types. 11,104 pass, 0 fail, 1,464 skip (Java errors/unsupported). Parallel via rayon (~8s on 16 cores). Golden files in separate repo (rustuml-golden) as git submodule. Per-directory failure reporting. Layout timeout wrapper for infinite loops.

### 🎯T1.5 KaTeX math rendering ✓
Achieved 2026-03-26. `rustuml-math` crate wired into parser/renderer. `@startmath`/`@endmath` and `@startlatex`/`@endlatex` dispatch to math renderer. 50 golden math tests pass.

### 🎯T1.6 YAML input format ✓
Achieved 2026-03-22. All diagram model types serialize/deserialize via serde. Three input formats auto-detected.
