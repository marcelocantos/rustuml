# Convergence Report

*Evaluated: 2026-03-29*

## Standing invariants

- **Tests**: PASSING (517 total: 8 + 10 + 6 + 296 + 197 across 5 crates)
- **CI**: PASSING (last run on master: success, 1m43s)
- **Release**: v0.3.0 released successfully

Standing invariants: all green.

## Movement

- 🎯T1.2: significant → **close** — layout-rs replaced with vendored Graphviz, bezier edge routing, extended to 8 renderers
- 🎯T1.3: close → **close** (closer) — stdlib includes bundled, archimate parsing, `!import` directive added
- 🎯T1.4: close → **close** (closer) — hyperlinks wired, creole tables/trees, archimate renderer, ASCII renderers, 183 skinparam keys
- 🎯T1.7: significant → **close** — EPS output added, format-parameterized smoke tests implemented
- 🎯T1: converging (2/5 achieved) → converging (2/5 achieved, 3 close)

## Gap Report

### 🎯T1.2 Hierarchical graph layout engine in Rust  [weight: 4]
Gap: **close**
Massive progress in PR #49: layout-rs entirely replaced with vendored Graphviz C libraries (dot algorithm), statically linked via `build.rs`. Cubic bezier edge routing with spline extraction. Layout now used by 8 renderers (class, object, component, deployment, usecase, state, activity, dot) — previously only class and object. Timeout guard retained (5s) with grid fallback. Remaining gap: edge routing quality tuning on dense graphs (the converge/diverge line problem from acceptance criteria). All acceptance criteria met except edge routing quality on dense graphs.

### 🎯T1.3 PlantUML parser and TIM preprocessor ported to Rust  [weight: 2.5]
Gap: **close**
22 diagram types now parsed (was 18). Stdlib includes (`!include <C4/...>`, `!include <awslib/...>`) are bundled and resolved via `stdlib.rs` (177 lines). Archimate parsing added. `!import` directive implemented. Only 1 parse error in golden tests (mindmap edge case). Remaining gap: complex nested TIM macro edge cases.

### 🎯T1.4 Diagram model and rendering pipeline ported to Rust  [weight: 2.5]
Gap: **close**
22 diagram types render to SVG. Hyperlinks (`[[url]]`) wired into SVG output with `open_link`/`close_link` for class, sequence, component diagrams. Creole markup extended with tables, tree lists, nested lists, horizontal rules (creole.rs now 1,230 lines). Archimate renderer (173 lines). ASCII renderers for class, state, activity. 183 skinparam keys wired. Remaining gap: ~15% of skinparam keys not applied, deeper creole edge cases.

### 🎯T1.7 Multi-format output (PNG, PDF, EPS)  [weight: 2]
Gap: **close**
All 4 output formats working: SVG, PNG (-tpng), PDF (svg2pdf), EPS (-teps, 204 lines). Format-parameterized golden smoke tests (`golden_formats.rs`) validate PNG/PDF/EPS conversion produces correct file headers without crashing. Remaining gap: golden comparison for non-SVG formats (currently smoke-only, not structural comparison).

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies  [weight: 1]
Gap: **converging** (2/5 sub-targets achieved, 3 close)

  - [x] 🎯T1.1 Oracle-based test framework — achieved
  - [ ] 🎯T1.2 Layout engine — close: vendored Graphviz with bezier routing, dense graph quality remaining
  - [ ] 🎯T1.3 Parser/TIM — close: 22 types, stdlib, archimate; nested macro edge cases remaining
  - [ ] 🎯T1.4 Rendering pipeline — close: 22 types, hyperlinks, creole tables; ~15% skinparams remaining
  - [x] 🎯T1.5 KaTeX math rendering — achieved
  - [ ] 🎯T1.7 Multi-format output — close: all 4 formats working, smoke tests; golden comparison remaining

## Recommendation

Work on: **🎯T1.2 Hierarchical graph layout engine in Rust**

Reason: Highest effective weight (4) among all targets. With vendored Graphviz now in place and 8 renderers using it, the remaining gap is edge routing quality on dense graphs — the converge/diverge line problem. This is the last acceptance criterion not met. Closing this would achieve the target and unlock the parent target's highest-weight dependency.

## Suggested action

Test edge routing quality on dense graph inputs. Run `test-diagrams/wide-shallow-dense.puml` through the current renderer and visually inspect the output. If the converge/diverge problem persists, investigate Graphviz `splines` attribute settings (`splines=ortho`, `splines=polyline`) and `overlap` removal to improve edge separation. The vendored Graphviz gives full control over layout parameters — experiment with `nodesep`, `ranksep`, and edge weight attributes.

<!-- convergence-deps
evaluated: 2026-03-29T07:00:00Z
sha: d7d3facb

🎯T1:
  gap: converging
  assessment: "2/5 sub-targets achieved (T1.1, T1.5). T1.2, T1.3, T1.4, T1.7 all close. Major progress from PR #49."
  read:
    - docs/targets.md
    - docs/TODO.md

🎯T1.2:
  gap: close
  assessment: "Vendored Graphviz replaces layout-rs. Bezier edge routing. 8 renderers use it. Remaining: dense graph edge quality."
  read:
    - crates/rustuml-layout/src/lib.rs
    - crates/rustuml-layout/src/graph.rs
    - crates/rustuml-layout/src/graphviz_ffi.rs
    - crates/rustuml-render/src/class.rs
    - crates/rustuml-render/src/component.rs
    - crates/rustuml-render/src/deployment.rs
    - crates/rustuml-render/src/usecase.rs
    - crates/rustuml-render/src/state.rs
    - crates/rustuml-render/src/activity.rs
    - crates/rustuml-render/src/object.rs
    - crates/rustuml-render/src/dot_diagram.rs

🎯T1.3:
  gap: close
  assessment: "22 diagram types parsed. Stdlib includes bundled (stdlib.rs). Archimate. !import. 1 mindmap parse failure remaining."
  read:
    - crates/rustuml-parser/src/preprocess/stdlib.rs
    - crates/rustuml-parser/src/preprocess/mod.rs
    - crates/rustuml-parser/src/parse/archimate.rs
    - crates/rustuml-parser/src/diagram/archimate.rs

🎯T1.4:
  gap: close
  assessment: "22 types render to SVG. Hyperlinks wired. Creole tables/trees (1230 lines). Archimate renderer. ASCII renderers. 183 skinparams. ~15% skinparams remaining."
  read:
    - crates/rustuml-render/src/svg.rs
    - crates/rustuml-render/src/creole.rs
    - crates/rustuml-render/src/archimate.rs
    - crates/rustuml-render/src/skinparam.rs
    - crates/rustuml-render/src/ascii.rs
    - crates/rustuml-render/src/eps.rs

🎯T1.7:
  gap: close
  assessment: "SVG+PNG+PDF+EPS all working. Format smoke tests (golden_formats.rs). Remaining: golden comparison for non-SVG."
  read:
    - crates/rustuml-oracle/tests/golden_formats.rs
    - crates/rustuml-render/src/eps.rs
-->
