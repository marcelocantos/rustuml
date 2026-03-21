# Convergence Report

*Evaluated: 2026-03-22*

## Standing invariants

No code yet — no tests or CI to evaluate. All green by default.

## Gap Report

### 🎯T1 PlantUML exists as a single Rust binary with no runtime dependencies  [weight: 1]
Gap: **not started** (converging 0/5 sub-targets achieved)

No Rust code exists yet. The project has targets and CLAUDE.md only.

  - [ ] 🎯T1.1 Oracle-based test framework — not started (weight: 20, **highest priority**)
  - [ ] 🎯T1.2 Graphviz DOT layout — not started (weight: 4)
  - [ ] 🎯T1.3 Parser and TIM preprocessor — not started (weight: 2.5)
  - [ ] 🎯T1.4 Rendering pipeline — not started (weight: 2.5)
  - [ ] 🎯T1.5 KaTeX math rendering — not started (weight: 1.7)

## Recommendation

Work on: **🎯T1.1 Oracle-based test framework validates Rust output against Java PlantUML**

Reason: Highest effective weight (20) by a wide margin. This target gates 🎯T1.3, 🎯T1.4, and 🎯T1.5 — without it, there's no way to verify correctness of the parser, renderer, or math ports. Building this first means every subsequent porting step is immediately testable. Cost is low (estimated 1) relative to value (20).

## Suggested action

Bootstrap the Rust workspace and oracle test framework:

1. `cargo init` a workspace with a `rustuml` binary crate and a `rustuml-oracle` test/support crate
2. Write a test generator that produces minimal PlantUML inputs (start with sequence diagrams — simplest syntax)
3. Build a test harness that shells out to the Java PlantUML JAR (`~/work/github.com/plantuml/plantuml`) for reference output
4. Implement exact-match comparison for preprocessing/metadata and structural SVG comparison for layout
5. Wire it up as `cargo test` integration tests

<!-- convergence-deps
evaluated: 2026-03-22T00:48:00Z
sha: no-commits

🎯T1:
  gap: not started
  assessment: "No code exists. 0/5 sub-targets achieved."
  read:
    - docs/targets.md
    - CLAUDE.md

🎯T1.1:
  gap: not started
  assessment: "No test framework code exists."
  read:
    - docs/targets.md

🎯T1.2:
  gap: not started
  assessment: "No Graphviz port code exists."
  read:
    - docs/targets.md

🎯T1.3:
  gap: not started
  assessment: "No parser code exists."
  read:
    - docs/targets.md

🎯T1.4:
  gap: not started
  assessment: "No rendering code exists."
  read:
    - docs/targets.md

🎯T1.5:
  gap: not started
  assessment: "No KaTeX port code exists."
  read:
    - docs/targets.md
-->
