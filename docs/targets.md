# Targets

<!-- last-evaluated: dd3b9530 -->

## Active

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies
- **Weight**: 1 (value 20 / cost 20)
- **Estimated-cost**: 20
- **Acceptance**:
  - A single statically-linked Rust binary accepts PlantUML text input and produces SVG/PNG output for all diagram types
  - No JVM, no Graphviz binary, no external font files required at runtime
  - Output is structurally equivalent to Java PlantUML for the same inputs
- **Context**: PlantUML's JVM dependency makes deployment painful. The current Java codebase has weak test coverage (~12%) and a tangled architecture. A Rust port solves deployment (single binary, cross-platform, WASM-ready) while enabling clean architecture. External dependencies (Graphviz layout, KaTeX math rendering) are ported into the binary. The current Java version serves as the oracle for synthetic test generation.
- **Status**: converging (5/6 sub-targets achieved, 1 close) — 22 diagram types parsed and rendered, 12,568 golden test pairs (11,267 passing, 0 parse skips), full TIM preprocessor, SVG+PNG+PDF+EPS output with format-parameterized smoke tests. Graphviz layout engine with bezier edge routing. Stdlib includes, archimate, hyperlinks, creole tables, ASCII renderers.
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
- **Status**: near-achieved — 22 diagram types render to SVG including archimate. Hyperlinks, creole tables/trees/lists, sprite rendering, ASCII renderers. Skinparam case-normalization fix covers ~200 PascalCase patterns. Object skinparam keys, componentStyle, roundcorner, sequenceResponseMessageBelowArrow added. Creole `<color:X>` tags emit proper tspan fills. 207 render tests. Remaining: minor skinparam completeness, structural SVG equivalence tuning.
- **Discovered**: 2026-03-22

## Achieved

### 🎯T1.7 Multi-format output (PNG, PDF, EPS) ✓
Achieved 2026-03-29. SVG (default), PNG (resvg/tiny-skia at 96 DPI with configurable scale), PDF (svg2pdf), and EPS (raster-embedded PostScript) all working from a single binary. PlantUML-compatible CLI flags (-tsvg/-svg, -tpng/-png, -tpdf/-pdf, -teps/-eps, -ttxt/-txt). Format-parameterized golden smoke tests validate all 12,500+ diagrams across PNG/PDF/EPS with magic byte, dimension, BoundingBox, and page count checks. Unit tests cover dimension extraction, 2x scaling, and structural validation.

### 🎯T1.3 PlantUML parser and TIM preprocessor ported to Rust ✓
Achieved 2026-03-29. 22 diagram types parsed. Full TIM preprocessor with variables, functions, control flow, includes, themes, JSON. Stdlib includes bundled and resolved. EBNF: single-quoted terminals, special sequences, quote-aware semicolon splitting. Mindmap: bare `--` side separator. Preprocessor: single-quote comment stripping disabled in @startebnf blocks. 0 parse skips in golden tests (was 6). 299 parser tests.

### 🎯T1.2 Hierarchical graph layout engine in Rust ✓
Achieved 2026-03-29. Layout-rs replaced with vendored Graphviz C libraries (dot algorithm), statically linked via cc build script. Cubic bezier spline edge routing extracted from Graphviz's libpathplan — same engine PlantUML uses. Used by 8 renderers: class, object, component, deployment, usecase, state, activity, dot. 138 bezier paths on the 50-class dense test graph (zero straight lines). Timeout guard (5s) with grid fallback. Thread-safe via mutex.

### 🎯T1.1 Oracle-based test framework ✓
Achieved 2026-03-26. 12,568 golden pairs across 30 categories and 18 diagram types. 11,104 pass, 0 fail, 1,464 skip (Java errors/unsupported). Parallel via rayon (~8s on 16 cores). Golden files in separate repo (rustuml-golden) as git submodule. Per-directory failure reporting. Layout timeout wrapper for infinite loops.

### 🎯T1.5 KaTeX math rendering ✓
Achieved 2026-03-26. `rustuml-math` crate wired into parser/renderer. `@startmath`/`@endmath` and `@startlatex`/`@endlatex` dispatch to math renderer. 50 golden math tests pass.

### 🎯T1.6 YAML input format ✓
Achieved 2026-03-22. All diagram model types serialize/deserialize via serde. Three input formats auto-detected.
