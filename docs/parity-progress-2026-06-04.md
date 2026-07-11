# RustUML 🎯T4 (strict-XML golden parity) — Progress Report

_2026-06-04 · master `1e103c98`_

## The goal

🎯T4 = `cargo test --test golden_pairs --release` reporting **0 failures across all
12,546 golden pairs** (exact-XML match against Java PlantUML).

## Headline result this session

**1,734 → 1,553 real failures (−181, ~10.4% of the remaining gap closed), zero
regressions throughout.** Pass count 9,513 → **9,694**. Master is at `1e103c98`,
working tree clean, all work committed locally (unpushed), golden submodule intact.

| Metric | Start | Now |
|---|---|---|
| Failing | 1,734 | **1,553** |
| Passing | 9,513 | **9,694** |
| Commits landed | — | 19, across 11 features |

## What got done (by feature)

All full-suite-verified, zero-regression, semantic ports from the Java oracle.

| Area | Δ | Essence of the fix |
|---|---|---|
| timing clkperiod | +5 | Period-aware ruler tick-unit + clock square-wave (`PanelsClock`) |
| elseif / FtileIfLong | +6 | Multi-branch if/elseif/else: diamond chain + ON_X compaction port |
| salt `{#` tables | +9 | Root cause was the `Header` chrome command consuming the first row |
| class separators/namespace | +10 (+bonus) | Document-order separator blocks; `Quark.getQualifiedName` |
| sequence newpage | +12 | Single-page render + dashed page-separator rule |
| state composite/history | +5 | Pseudo-state emission ordering (cross-boundary, orphan-history) |
| component lollipop/cloud | +3 | Lollipop→CLASS interface box; leaf cloud shape |
| er crow's-foot | +1 | Floating note shares the `ent000N` counter (declaration order) |
| gantt notes/holidays | +3 | Opale note box; title as fixed band above body; `is closed` dates |
| **sequence note-width** | **+39** | **The gateway fix** — see below |
| sequence note siblings | +22 | OVER_SEVERAL alignment, hnote/rnote gaps, raw-width across-shift |
| creole mixed-size | +1 | PlantUML's 10px atom-height floor in baseline alignment |
| sequence group+note | +8 | Enclosed note's extent folded into alt/opt/loop frame width |

### The keystone — the sequence note-width metric (+39, cascading)

RustUML was `ceil()`-ing a note's text width and reusing that one value for *both* the
drawn box *and* all layout/spacing/canvas reservations. Java keeps the full-precision
`double` for layout and only snaps the *drawn* polygon to integers. Splitting "raw width
for layout / ceiled width for the box" fixed sub-pixel participant drift across the entire
28-case `seq_types` matrix plus newpage/hnote/rnote cases. It was the keystone that
reopened the whole sequence bucket and unblocked the +22 sibling-note and +8 group-frame
fixes that followed.

## How it was done — the method (a proven, reusable pipeline)

Parallel **opus subagents in isolated worktrees**, each taking **one feature on a disjoint
file**, semantic-porting from the Java source, committing-and-stopping. The parent then
cherry-picks the disjoint commits onto one integration branch, runs **one full-suite
gate**, and fast-forwards master only if clean.

Operational lessons (all recorded in memory):

- **Stale-base trap:** worktrees spawn off the old v0.7.0 tag; agents must pin to the
  literal current-master SHA as step 0 (sandbox blocks `reset --hard`, so
  `checkout --detach` + `branch -f`).
- **Golden self-loop:** a stray `ln -s` created `golden/golden`, inflating a gate 33×
  — `rm -f` it before every gate.
- **Cross-bucket regression:** an agent's per-bucket gate is blind to other buckets
  (gantt's height fix passed `gantt/` but broke a gantt diagram living in the
  `multi-diagram/` bucket). The parent's **full-suite gate is mandatory**, and
  **quarantine-and-retask** (drop the one bad commit, bank the rest) preserves
  zero-regression without losing the wave.
- **Exact regression check:** snapshot `golden_failure_names.txt` per FF, then `comm`
  the new list against it — provably empty regressions, no histogram blind spots.
- **Stalled-agent recovery:** two agents hit the 600s watchdog *at wrap-up*; their work
  was salvaged from the worktree (committed or staged) and gated normally.

## Current state

- Master `1e103c98`, **1,553 failing**, clean tree, 0 regressions, nothing in flight.
- The contained per-renderer/per-feature vein is now **heavily harvested** — the easy,
  isolated wins are largely taken.

## Work ahead (the remaining 1,553, by tractability)

### 1. Theme cascade — biggest single lever (~219): `preproc_theme` 111 + skinparam 108

Themes are *partially* built (theme files bundled in `crates/rustuml-parser/themes/`;
`!theme X` expands to skinparam lines via `theme_tail` in `preprocess/mod.rs`), but
`!theme` currently renders with **zero effect** (default background/dimensions, no
gradient defs). **Strong untested hypothesis:** the expanded theme skinparams aren't
actually reaching the renderers — if that's one broken link, fixing it could cascade
across many theme cases at once. **This is the next priority.** It's **cross-cutting**
(shared skinparam/style/SVG + every renderer), so it must run **solo** (no concurrent
renderer agents) with a full-suite gate confirming non-themed diagrams are unaffected.
Gradient-fill (`<linearGradient>`) rendering is a separate add.

### 2. Activity ON_Y engine (~546 — the largest bucket)

Genuinely back-loaded: the remaining elseif/while/repeat/fork/switch cases are blocked on
porting PlantUML's whole-diagram vertical compaction (`CompressionXorYBuilder(ON_Y)`). The
catch is that the legacy emitter's hard-coded gaps *pass today* precisely because they
equal the compacted result, so a real ON_Y pass would churn all ~750 currently-passing
activity cases — high-risk, multi-turn, must be done as one holistic rewrite, not
piecemeal.

### 3. Remaining sequence (~248)

Teoz alt-engine (25, deep), the `*_over_two` cluster (12, a coupled trunc-vs-floor rounding
pipeline needing holistic re-derivation), plus smaller participant/create/divider clusters.

### 4. Creole markup (~84)

`combo`/`in`/`html`/`escape`/`bold`/`img` clusters — contained but touch the shared
creole/text-render engine, so each needs the three-bucket (creole+sequence+class) gate.

### 5. Diverse tail (~200)

edge-cases (163, chips away as renderers improve), combo (19), class residual (25),
assorted 1-offs.

### 6. Ceilings (~148) — may be genuinely unreachable by exact match

math/LaTeX (10 — Java's JLaTeXMath vs RustUML's katex are different engines, so
byte-identical SVG is unlikely without a math-engine rewrite), plus sprites (48),
archimate (48), and deployment cloud-shape floating-point cases (42).

## Honest bottom line

T4=0 is real but **structurally multi-session** from here — the cheap wins are spent, and
the bulk of the remainder is two large subsystems (theme cascade, activity ON_Y engine)
plus a ceiling tier that may require accepting some cases as unreachable (notably the
math-engine mismatch). The strong, fast-moving phase is the **theme cascade** — start
there next, solo, testing the "skinparams not reaching renderers" hypothesis first.

Detailed continuation notes live in the auto-memory: `project_parity_gaps.md` (top dated
section) and `feedback_fanout_worktree_gotchas.md`.
