# Convergence Report

*Evaluated: 2026-03-29*

## Standing invariants

- **Tests**: PASSING (517 total: 8 + 10 + 6 + 296 + 197 across 5 crates)
- **CI**: PASSING (last run on master: success)
- **Release**: v0.3.0 released successfully

Standing invariants: all green.

## Movement

- 🎯T1.2: close → **achieved** — marked achieved after PR #49 merge
- 🎯T1.3: (unchanged) — close, no code changes
- 🎯T1.4: (unchanged) — close, no code changes
- 🎯T1.7: (unchanged) — close, no code changes
- 🎯T1: converging (2/5 achieved) → converging (4/7 achieved, 3 close)

## Gap Report

### 🎯T1.3 PlantUML parser and TIM preprocessor ported to Rust  [weight: 2.5]
Gap: **close**
22 diagram types parsed. Full TIM preprocessor with variables, functions, control flow, includes, themes, JSON. Stdlib includes bundled and resolved. Archimate parsing. `!import` directive. Lenient JSON parser. Only 1 parse error in golden tests (mindmap edge case). Remaining gap: complex nested TIM macro edge cases.

### 🎯T1.4 Diagram model and rendering pipeline ported to Rust  [weight: 2.5]
Gap: **close**
22 diagram types render to SVG including archimate. Hyperlinks (`[[url]]`) wired into SVG output for class, sequence, component diagrams. Creole tables, tree lists, nested lists, horizontal rules. 183 skinparam keys wired. Sprite rendering. ASCII renderers for class, state, activity. Remaining gap: ~15% of skinparam keys not applied, deeper creole edge cases.

### 🎯T1.7 Multi-format output (PNG, PDF, EPS)  [weight: 2]
Gap: **close**
All 4 output formats working: SVG, PNG (-tpng), PDF (svg2pdf), EPS (-teps). Format-parameterized golden smoke tests validate PNG/PDF/EPS conversion produces correct file headers. Remaining gap: golden comparison for non-SVG formats (currently smoke-only).

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies  [weight: 1]
Gap: **converging** (4/7 sub-targets achieved, 3 close)

  - [x] 🎯T1.1 Oracle-based test framework — achieved
  - [x] 🎯T1.2 Layout engine — achieved
  - [ ] 🎯T1.3 Parser/TIM — close: 1 mindmap parse error, nested macro edge cases
  - [ ] 🎯T1.4 Rendering pipeline — close: ~15% skinparams, creole edge cases
  - [x] 🎯T1.5 KaTeX math rendering — achieved
  - [x] 🎯T1.6 YAML input format — achieved
  - [ ] 🎯T1.7 Multi-format output — close: all formats working, golden comparison remaining

## Recommendation

Work on: **🎯T1.3 PlantUML parser and TIM preprocessor ported to Rust** or **🎯T1.4 Diagram model and rendering pipeline ported to Rust**

Reason: Both share the highest effective weight (2.5) and both are close. They are largely independent — parser work (fixing the mindmap parse error, nested TIM macros) is orthogonal to rendering work (wiring remaining skinparams, creole edge cases). Either is high-leverage. Between the two, 🎯T1.3 has a more concrete remaining gap (1 known parse failure, specific macro edge cases) making it slightly more actionable. 🎯T1.4's remaining ~15% skinparams require identifying which keys are missing and wiring them — also actionable but broader in scope.

## Suggested action

For 🎯T1.3: Investigate the 1 remaining golden test parse failure (mindmap edge case). Run the golden tests with a filter for mindmap to identify the specific failing input, then examine the parser to understand what construct it doesn't handle. Fixing this single failure would bring golden parse errors to zero.

For 🎯T1.4: Run `cargo test --lib` with a focus on skinparam coverage — grep for skinparam keys referenced in golden test files that aren't yet in `skinparam.rs` to identify the gap. Alternatively, pick a specific creole edge case from the golden tests and implement support.

Both targets can be parallelised with team agents if desired.

<!-- convergence-deps
evaluated: 2026-03-29T08:00:00Z
sha: dd3b9530

🎯T1:
  gap: converging
  assessment: "4/7 sub-targets achieved (T1.1, T1.2, T1.5, T1.6). T1.3, T1.4, T1.7 all close. No code changes since last eval — movement was T1.2 achievement."
  read:
    - docs/targets.md

🎯T1.3:
  gap: close
  assessment: "22 diagram types parsed. Full TIM preprocessor. Stdlib includes. 1 mindmap parse failure. Nested macro edge cases remaining."
  read:
    - docs/targets.md

🎯T1.4:
  gap: close
  assessment: "22 types render to SVG. Hyperlinks wired. Creole tables/trees. 183 skinparams. ~15% skinparams remaining, creole edge cases."
  read:
    - docs/targets.md

🎯T1.7:
  gap: close
  assessment: "SVG+PNG+PDF+EPS all working. Format smoke tests. Remaining: golden comparison for non-SVG."
  read:
    - docs/targets.md
-->
