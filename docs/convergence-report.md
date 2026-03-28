# Convergence Report

*Evaluated: 2026-03-27*

## Standing invariants

- **Tests**: PASSING (unit tests + oracle tests green)
- **CI**: FAILING — `cargo fmt --check` in Check & Lint job (`golden_pairs.rs` formatting)
- **Release workflow**: FAILING (v0.2.0 release asset upload issues)

## Movement

- 🎯T1: not started → converging (2/5 sub-targets achieved, 2 near-achieved)
- 🎯T1.1: not started → **achieved** (12,568 golden pairs, 0 failures)
- 🎯T1.2: not started → converging (layout engine with timeout wrapper)
- 🎯T1.3: not started → near-achieved (18 diagram types, full TIM preprocessor)
- 🎯T1.4: not started → near-achieved (18 types render to SVG, theme system)
- 🎯T1.5: not started → **achieved** (rustuml-math crate, 50 golden math tests)
- 🎯T1.7: (new) converging (SVG+PNG+PDF output working)

## Gap Report

### 🎯T1.2 Hierarchical graph layout engine in Rust  [weight: 4]
Gap: **significant**
Layout engine exists in `rustuml-layout` crate (2 source files). Timeout/fallback logic lives in the render crate (`class.rs`, `sequence.rs`, `gantt.rs`), not the layout crate itself. Infinite loop mitigation is a timeout wrapper, not a proper fix. Edge routing quality on dense graphs remains poor. Layout not yet extended to component/deployment/object diagrams.

### 🎯T1.3 PlantUML parser and TIM preprocessor ported to Rust  [weight: 2.5]
Gap: **close**
18 diagram types parsed with full TIM preprocessor (variables, functions, control flow, includes, themes, JSON). 9 references to stdlib/import/archimate in parser source — these are the main remaining gaps. Complex nested macro edge cases outstanding.

### 🎯T1.4 Diagram model and rendering pipeline ported to Rust  [weight: 2.5]
Gap: **close**
All 18 diagram types render to SVG. Theme system with 38 skinparam references across 8 files. ~40% of skinparam keys not yet applied. Zero sprite rendering code. Creole markup comprehensive but edge cases remain. 11,104 of 12,568 golden pairs pass.

### 🎯T1.7 Multi-format output (PNG, PDF, EPS)  [weight: 2]
Gap: **significant**
SVG, PNG (-tpng), and PDF (svg2pdf) output working. No format-parameterized test framework — golden files are SVG-only. EPS not implemented.

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies  [weight: 1]
Gap: **converging** (2/5 sub-targets achieved)

  - [x] 🎯T1.1 Oracle-based test framework — achieved
  - [ ] 🎯T1.2 Layout engine — significant
  - [ ] 🎯T1.3 Parser/TIM — close
  - [ ] 🎯T1.4 Rendering pipeline — close
  - [x] 🎯T1.5 KaTeX math rendering — achieved
  - [ ] 🎯T1.7 Multi-format output — significant

## Recommendation

Work on: **Fix CI first** (standing invariant violation)

`cargo fmt --check` is failing on `crates/rustuml-oracle/tests/golden_pairs.rs`. This blocks all convergence — fix it before target work.

After CI is green, work on: **🎯T1.2 Hierarchical graph layout engine in Rust**

Reason: Highest effective weight (4) among unblocked targets. Layout quality is the biggest gap between RustUML and usable output. The layout crate is minimal (2 files) while the timeout/fallback logic is scattered across render modules. Consolidating layout concerns and extending to more diagram types has the highest leverage.

## Suggested action

1. Run `cargo fmt` to fix the formatting in `golden_pairs.rs` and commit
2. Investigate the release workflow failure (contents:write permission was just added — may need further CI fixes)
3. Then assess layout-rs usage: read `crates/rustuml-layout/src/lib.rs` and `crates/rustuml-render/src/class.rs` to understand the current timeout wrapper and plan improvements

<!-- convergence-deps
evaluated: 2026-03-27T11:00:00Z
sha: 4447e7e6

🎯T1:
  gap: converging
  assessment: "2/5 sub-targets achieved (T1.1, T1.5). T1.3 and T1.4 near-achieved. T1.2 and T1.7 significant."
  read:
    - docs/targets.md

🎯T1.2:
  gap: significant
  assessment: "Layout crate minimal (2 files). Timeout wrapper in render crate. No infinite loop fix. Edge routing poor. Not extended to component/deployment/object."
  read:
    - crates/rustuml-layout/src/lib.rs
    - crates/rustuml-layout/src/graph.rs
    - crates/rustuml-render/src/class.rs
    - crates/rustuml-render/src/sequence.rs
    - crates/rustuml-render/src/gantt.rs

🎯T1.3:
  gap: close
  assessment: "18 diagram types parsed, full TIM preprocessor. Missing: stdlib includes, !import, archimate, nested macro edge cases."
  read:
    - crates/rustuml-parser/src/preprocess/mod.rs

🎯T1.4:
  gap: close
  assessment: "18 types render to SVG. 38 skinparam refs across 8 files. ~40% skinparams not applied. No sprite rendering. 11,104/12,568 golden pairs pass."
  read:
    - crates/rustuml-render/src/skinparam.rs
    - crates/rustuml-render/src/class.rs
    - crates/rustuml-render/src/lib.rs

🎯T1.7:
  gap: significant
  assessment: "SVG+PNG+PDF working. No format-parameterized tests. EPS not implemented."
  read:
    - docs/targets.md
-->
