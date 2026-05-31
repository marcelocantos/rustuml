<!-- Plan of record generated 2026-05-31 by the parity-challenge-map workflow (30 agents, adversarially verified). Baseline 9,399/12,546 (75.0%). -->

# RustUML → Full Parity: Challenge Map

*Plan of record — branch `strict-xml-parity-renderers`, 2026-05-31. Baseline: 9,399 / 12,546 golden pairs pass (75.0%); **1,848 real failures** in `test-diagrams/golden_failure_names.txt`. All per-bucket numbers below are reconciled against that file, not the MAP's slightly-higher pre-skip estimates.*

## 1. Where we are

We are at 75.0% byte-exact strict-XML parity. The cheap vein is exhausted: this session and its predecessors harvested everything that was a single attribute echo, a source-line off-by-one, a missing color token, or a near-miss cluster sharing one max-deviation (deployment title-centering +40, class colours +8, hex normalization, seq title-line). What remains is **structural**: deep self-layout geometry in the two hand-written engines (activity 574, sequence 398 — together **53% of all failures**), a cross-cutting font/theme-metric cascade that no renderer currently consumes, and a long tail of contained per-renderer features. The harness already correctly *excludes* the unmatchable Java-error/welcome goldens via `golden_has_syntax_error()` (golden_pairs.rs:68-83) — verified: every syntax-error example case the MAP listed is absent from the 1,848 real failures, so large swaths of the MAP's "T2/T5 error-diagram" categories are **already handled** and contribute zero open failures. The honest shape of the remaining 1,848 is two giant engine-geometry problems, one font/theme subsystem, and ~600 cases of genuine per-renderer feature work.

## 2. Challenge taxonomy

Nine cross-bucket categories, ordered by impact (case count × unblocking leverage). Counts are summed across buckets from the verified MAP; they total to the failing set with overlap noted (combination/kitchen-sink cases are counted once under their primary cause).

---

### C1 — Self-layout engine geometry (activity + sequence + state tiles) — ~720 cases — Tier T3, deep

**The core problem.** RustUML hand-writes the activity and sequence layout engines (`crates/rustuml-render/src/activity.rs`, `sequence.rs`); they do **not** route through the oracle for node positions (unlike class/component/deployment, which consume `extract_oracle_layout`). Every tile metric must be computed from scratch and currently drifts from Java's FTile / Tile-Z geometry by a few pixels, and that drift cascades into every downstream x/y and the canvas size.

**What's inside (by failing-set count):**
- Activity `if`/`while` diamond width + spine centering (~130): `diamond_inner_w()` and branch-balancing off Java's `FTileDiamond` by 2-10px (`act_color_blue_in_while` +2, `act_fork3br_ifdepth1` +9.7px).
- Activity swimlane vertical title-band (~67): every element shifted down exactly 21.64px (`act_color_blue_in_swimlane`) — content origin omits the header band height.
- Activity while/repeat loop spine + break edge routing (~80): `FTileWhile`/`FTileRepeat` loop-back and post-loop landing point (`act_break_repeat_late` 62px short).
- Activity fork/detach join-bar suppression + branch spread (~55), elseif diamond-cascade vs flat branches (~22).
- Sequence self-message inter-participant gap (~30): `max_self_msg_right` (sequence.rs:3317-3338) folds only into the **canvas edge** at line 3717, never into the column gap — confirmed live bug.
- Sequence nested-group / break frame ordering + width-feedback (~30): frames emit inner-before-outer and widths don't max over enclosed frames.
- Sequence note-box / activation-bar sub-pixel metrics (~60 across sequence + preprocessing + creole): the documented "±1 note-width, floor right / sub-pixel measure off"; self-message activation +6px (preproc `define_participant_*`).
- State composite/history/concurrent pseudostate shapes + entry/exit ordering (~10).

**What it takes.** Port Java's exact tile geometry per tile type (diamond insets, label gaps, swimlane band, loop-back routing, join-bar rules, note-box padding). This is the **single highest-leverage** work item — it is bounded (no new subsystem, no external data), but it is many sequential, individually-verifiable metric fixes inside two files, each with layout-feedback ripple. No oracle shortcut: these engines deliberately compute from source.

---

### C2 — Font & theme cascade (per-font metrics + theme `<style>` propagation) — ~290 cases — Tier T4, dedicated subsystem

**The core problem.** Verified across six VERIFY verdicts (kept T4 every time, reachable=true every time). `defaultFontName`/`defaultFontSize`/`activityFontName`/per-element FontName blocks and bundled-theme directives are **parsed into the skin model but never consumed by renderers**. `font-family:"sans-serif"` is a hard-coded string literal at **~150 sites across 22 render files**; `text_render::measure_inner` (line 93) hard-codes the family; there is only one sans-serif ratio table (`plantuml_metrics.rs`) plus mono/serif. Matching Arial/Verdana/Courier/Times requires **new per-named-font AWT metric tables that do not exist**, and font changes ripple through every measured width → box → position → canvas (true layout feedback).

**What's inside:**
- skinparam activity font block not consumed (~65): activity.rs hard-codes `FONT_SIZE=12.0`/`SMALL_FONT=11.0` at ~60 sites; `measure()` takes no family arg.
- Bundled-theme cascade (~112 preprocessing): `flatten_theme_output` drops `<style>` blocks **and** `!procedure`-injected directives inside grouped skinparam blocks never expand (verified: reddress/mars have no `<style>` at all — the real bug there is procedure-in-block expansion, not style-stripping).
- Sequence DefaultFontName/Size (~10), class defaultFontName→Courier monospace + nbsp substitution (~2), edge-case theme cascade (~9), state/combo FontName blocks (~7), creole title/footer font-size in band (subset of C3).

**What it takes.** A three-layer subsystem: (1) fix preprocessor procedure-in-block expansion + parse theme `<style>` blocks; (2) thread resolved font name/size/style/color through the skin model into all ~150 hard-coded sites and the `measure()` API; (3) **reverse-engineer per-font metric tables** (Arial, Verdana, Courier, Times, Helvetica) from the golden corpus the same way the sans-serif table (125,710 elements) was built. Reachable — fully deterministic, the reference JAR and `java_awt_metrics.txt` are in-tree — but cross-cutting and the second-biggest single investment.

---

### C3 — Width/height calc with layout feedback (titles, captions, footers, legends, boxes) — ~80 cases — Tier T3/T4

**The core problem.** Sequence `svg_width_exact` (sequence.rs:3768-3775) is computed purely from participant/group geometry; the measured width of title/caption/footer/legend/box-title is **never folded in**. When a title is wider than the participant span, Java widens the canvas and re-centers participants; RustUML keeps the narrow width and emits title at x=0 or negative-x (off-canvas). Verified on `creole_allctx_title`: constant 15.8px shift, all textLengths already correct.

**What's inside:** creole title/caption/footer width-feedback + `<size:N>` title height-band (~30 verified, 13 currently failing in-bucket but the underlying `svg_width_exact` fix reaches ~537 sequence-title goldens corpus-wide); sequence caption/box-title widen-and-recenter (~12); sequence legend block as bordered band (~6); deployment legend table (~1); header/footer centering and source-line (~13, partly T1).

**What it takes.** Make `svg_width`/`svg_height`/`title_band_h` take `max(participant_span, title, caption, footer, legend)`, then re-center all lifeline/box/message/note x-coords and re-flow y. A self-contained feedback loop in sequence.rs, but it touches the computation every element depends on. Partly entangles with C2 (creole-segmented measurement).

---

### C4 — Per-renderer feature gaps (contained T2/T3 features) — ~330 cases — Tier T2/T3

**The core problem.** Discrete, mostly-independent missing features, each a few elements per case. High aggregate yield, low individual risk, parallelizable.

**What's inside (representative):**
- **Sequence advanced features**: teoz engine (~32, T4 second render path — see C5-adjacent), delay/spacer `...`/`|||` (~24), `ref over` box (~15), newpage page-split (~13), found/lost `[->`/`->]` margins (~11), `order N` keyword (~6, T1 — regex never captures the int).
- **Class/object**: labeled body separators `== X ==` (~8+4), note-on-link folded-corner paths (~9), special node shapes diamond/circle/**lollipop required/provided** (~5+ — VERIFY downgraded this T5→T3: `class_lollipop_required` is a real white-bg diagram, the error-page lollipops are already skipped), multi-dash `--->` edges (~2), FQN/namespace-separator dedup + `::`→`..` mangling (~12), edge emission order (~4, T3 dot-traversal), multiline title (~1), hide-empty-members (~1).
- **Deployment**: cloud-as-cluster bezier `<path>` (~30 — `emit_cluster_shape` has no Cloud arm), node 3D-box decomposition order at nesting depth (~11), queue/element +5px (~5).
- **Creole-in-context**: lists/tables/strike in notes & dividers (sequence ~28, class hline, mindmap), `\n`/`\t` escape line-splitting (~9 edge + 6 creole), `<latex>`→`<image>` (~1), `<code>` literal pass-through (~2).
- **State**: choice/fork/join/history shapes (~6).
- **Other diagram types**: salt table `{#`/group `{^`/tabs `{/` (~11), nwdiag (~4), mindmap two-sided (~1), timing clock-period scale + constraint annotations (~9), gantt printscale/resources/holidays (~6), JSON/YAML empty/anchors/multiline (~6), meta legend/header `%page%` (~8).
- **Type-detection mis-routing** (~11+5): bare-id relations → CLASS not SEQUENCE; leading `note :` → CLASS; component-keyword mix → DESCRIPTION; header+footer+msg → SEQUENCE not CLASS.
- **Scale fit-to-box** `W*H`/`WxH`/`max N` (~6, T3 — currently returns 1.0).

**What it takes.** One feature at a time; many are quick passes (order keyword, scale, type-detection, lollipop, separators). Several share the creole-block-layout dependency. Highly delegatable / fan-out-friendly.

---

### C5 — Encoder/asset-dependent + whole-subsystem rebuilds — ~96 cases — Tier T4/T5-effort, all reachable

**The core problem.** Either a deterministic-but-large algorithm port, not an attribute fix.

**What's inside:**
- **ArchiMate** (48 real fails): VERIFY **downgraded T5→T4, reachable=true**. The bespoke `ArchimateDiagram` path (parser + 7.6k-byte renderer) is structurally incommensurable with Java's `DESCRIPTION` + stdlib-macro pipeline (octagon `<path>`, Verdana/12, vectorized sprite glyphs, Graphviz layout). Proven reachable: sprite path data traces byte-exact through stdlib `.spm` files + `%.4f` formatting; edge `d`/arrowheads are already in the oracle. Requires retiring the bespoke path, routing `<archimate/Archimate>` through the macro preprocessor, modeling elements as DESCRIPTION entities, and **granular** oracle geometry consumption (NOT inner_xml replay — see §5). Activity/sequence-class effort.
- **Handwritten mode** (~17 across skinparam/sequence/deployment/state): VERIFY **kept T4, reachable=true, PROVEN**. `Random(424242L)` fixed-seed LCG + `HandJiggle`; reimplemented the LCG and matched golden point `5.5,8.2102` exactly. Needs a shared shape-jitter layer + bounds-enlargement; deterministic.
- **Sprite inline `<$name>` wiring** (~44): VERIFY-adjacent — the full pipeline (`sprite.rs`, `text_with_sprites`) **already exists with zero callers**; svg.rs `text()`/`text_class_label()` only route `<&` icon refs, never `<$`. This is a **wiring fix**, not a subsystem — arguably belongs in C4 as a quick pass, but the downstream label-width layout feedback (`sprite_multiple`) makes the tail T3.
- **Welcome-screen text block + logo** (~9): VERIFY confirms these are **already skipped** by `golden_has_syntax_error("Welcome to PlantUML")`. The base64 logo PNG is the one genuinely external asset, but it never enters the comparator. **Zero open failures** — listed only to record disposition.

---

### C6 — Residual sub-pixel / 1-off geometry on otherwise-correct diagrams — ~40 cases — Tier T1/T3

Note-path origin +1px, vertical-if diamond y-rounding (0.7754), note-color x-base, duplicate-stereotype last-wins, participant-head 1px centering, ellipse `cy` after `\n`-split. Many resolve as side effects of C1/C3; the true standalone residue is small. Quick passes, but regression-sensitive (a "fix" that moves a coordinate can break a passing neighbor).

---

### C7 — Source-line attribution through the preprocessor — ~45 cases — Tier T1/T3

Macro/procedure/while/foreach body lines inherit the call-site/iteration line instead of the construct-body line (preprocessing ~39); absolute line numbering across pre-`@startuml` lines (~5, pure off-by-N); conditional-body element ordering (~9 edge). The deep version replaces the positional-placeholder line model with per-output-line `LineLocation` origin tracking (matching Java's `LineLocationImpl`); the shallow version (pre-@startuml count) is a one-spot fix. Contained to `preprocess/mod.rs`.

---

### C8 — Graphviz edge emission order & dot-version sensitivity — ~6 cases — Tier T3, deepest-layout

`class_edge_dense_relationships` etc.: golden emits `<g class="link">` in dot-traversal order, RustUML in source order; path-`d` geometry differs because dot lays edges out in that grouped order (vendored Graphviz 14.1.5 vs golden dot 15.0.0). Lowest yield, highest depth, possibly version-sensitive — defer.

---

### C9 — Already-handled / no-op (recorded for honesty) — 0 open failures

The MAP listed ~300+ "error-diagram" cases across activity (~170 over-accepted-syntax), sequence (107), preprocessing (~85 foreach/welcome/rejected-construct), edge-cases (~30), deployment (6), class (~10 lollipop error-pages), sprites (1), archimate (2). **VERIFY proved, and I re-confirmed against the failing set, that these are already skipped** by `golden_has_syntax_error()`. They are **not in the 1,848**. They require no work. (One subtlety: the MAP's activity bucket showed 606, the failing set shows 574 — the ~32 difference is exactly these pre-skipped error goldens. Do not re-introduce them by "implementing the error-diagram renderer" — Java's error banner embeds a build git-hash + timestamp and is genuinely unmatchable; matching them would also require making RustUML *worse* by rejecting valid syntax.)

---

**Taxonomy total:** C1 ~720 + C2 ~290 + C3 ~80 + C4 ~330 + C5 ~96 + C6 ~40 + C7 ~45 + C8 ~6 ≈ 1,607 distinctly-attributed + ~240 combination/kitchen-sink cases that resolve once their contributing causes land = ~1,848.

## 3. Reachability verdict

**Is literal "0 / 12,546 failing" achievable? Yes — and it largely already is, by design.** Every VERIFY verdict that examined a "T5 unmatchable" claim resolved to one of two outcomes:

1. **Already skipped, contributes zero failures** (the honest reading of "unmatchable"). The harness pins/skips exactly the cases that embed external non-deterministic state, via two mechanisms already in place:
   - `golden_has_syntax_error()` (golden_pairs.rs:68-83) skips any golden containing `Welcome to PlantUML`, `[From string`, `(Assumed diagram type:`, or `Syntax Error`. **Proof of genuine non-reproducibility for these**: the Java error/welcome banner embeds `PlantUML version 1.2026.3beta6 / 71806a2 [2026-03-21 13:49:45 UTC]` — a build git-hash and build timestamp with **no harness pin** (unlike `%date()`, which *is* pinned via `RUSTUML_DEBUG`/`GOLDEN_DEBUG`). No correct renderer can reproduce a foreign build's commit hash, and RustUML correctly *accepts* the syntax Java rejects, so it would never emit the error image anyway. Skipping is correct; these are out of the scored set. (Sequence "107 error-renders", deployment 6, sprites 1, class 10 lollipop-error, archimate 2, and the activity/preproc over-accepted-syntax categories all fall here — confirmed absent from `golden_failure_names.txt`.)
   - The base64 welcome logo PNG is a static brand asset inside an already-skipped golden — never compared.

2. **Reachable after all** — the "T5" label was wrong. **ArchiMate (48)** was downgraded T5→T4/reachable (sprite paths trace byte-exact through stdlib `.spm` + `%.4f`). **class lollipop (10)** downgraded T5→T3 (9 are pre-skipped error pages; the 1 real case is an ordinary diagram). **Handwritten (17)** is deterministic (seed `424242L` LCG, point `5.5,8.2102` reproduced).

**Genuinely unreachable, with proof — handle by the existing skip, do nothing else:** only the syntax-error/welcome goldens embedding the build version banner (git-hash + UTC build timestamp). These are **already excluded** and are not in the 1,848. There is **no remaining "date ceiling"** — `%date()` is pinned and matchable; the earlier framing that treated a date as an unreachable ceiling was mistaken. The version-banner cases are unreachable for a *different and narrower* reason (foreign build identity, not time), and the harness already handles them.

**Verdict: the reachable ceiling is 12,546 − (already-skipped error/welcome goldens) = the full 1,848 are all reachable.** 100% of the *scored* set is attainable by a correct renderer. No new pins are required; the two existing mechanisms (`golden_has_syntax_error` skip + `RUSTUML_DEBUG` date pin) are sufficient and correctly scoped.

## 4. Roadmap (75% → 100% of the reachable ceiling)

Sequenced so that unblockers land first and shared infra is built once. Yields are best-estimate from the failing-set counts.

| # | Package | Attacks | Yield | Shape |
|---|---------|---------|-------|-------|
| **WP0** | **Quick passes / 1-spot fixes** — sequence `order N` regex capture; scale fit-to-box (`W*H`/`max N`); type-detection re-route (bare-id→CLASS, `note :`→CLASS, comp-mix→DESCRIPTION, header+footer→SEQUENCE); sprite `<$>` wiring into `text()`/`text_class_label()`; footer/header centering + source-line; pre-`@startuml` absolute line count; `font_metrics.rs` dead-code removal | C4, C5(sprite), C7(shallow) | ~80-110 | Quick pass — days, mostly independent, fan-out-friendly |
| **WP1** | **Activity engine geometry** — diamond width/centering, swimlane band, while/repeat spine + break routing, fork/detach join-bar, elseif cascade | C1 (activity) | ~350 | Focused multi-step — the largest single yield; sequential metric ports inside `activity.rs`, each verifiable against one tile family. **Do first among the big rocks** (largest bucket, no shared-infra dependency) |
| **WP2** | **Sequence engine geometry** — self-message column gap (sequence.rs:3717), nested-group frame order + width-feedback, note-box/activation sub-pixel, delay/spacer/ref/newpage/found-lost features | C1 (sequence) + C4 (seq features) | ~250 | Focused multi-step — second-largest; the self-message gap and frame-ordering fixes also unblock the kitchen-sink combo cases |
| **WP3** | **Width/height layout-feedback loop** — fold title/caption/footer/legend/box width into `svg_width_exact`, re-center participants, height-band for `<size:N>` | C3 | ~30 in-bucket, ripples to ~500+ title goldens corpus-wide | Focused — self-contained but touches the shared width computation; do after WP2 so it builds on corrected sequence geometry |
| **WP4** | **Font & theme cascade subsystem** — (a) preprocessor procedure-in-block expansion + `<style>` parsing; (b) thread font name/size/style through ~150 sites + `measure()` API; (c) reverse-engineer per-font metric tables (Arial/Verdana/Courier/Times/Helvetica) | C2 | ~290 | **Dedicated subsystem build** — the biggest infra investment. Multi-step, spans preprocessor + skin model + 22 render files + new metric tables. Genuinely cross-cutting; needs its own multi-turn effort (per the deferred spec in `project_parity_gaps.md`). Do after WP1-3 so the engine geometry is stable before fonts perturb it |
| **WP5** | **Per-renderer feature fan-out** — class (separators, note-on-link, shapes, FQN, multidash), deployment (cloud-cluster bezier, 3D-box order), creole-in-context (lists/tables/`\n`-split/`<latex>`/`<code>`), state shapes, salt/nwdiag/mindmap/timing/gantt/json-yaml/meta | C4 (remainder) + C5(handwritten, teoz) | ~250 | Focused features + 2 subsystems (teoz second-render-path, handwritten jitter layer). Highly parallelizable; teoz/handwritten each warrant a dedicated agent |
| **WP6** | **Source-line origin tracking** — per-output-line `LineLocation` through macro/procedure/while/foreach | C7 (deep) | ~40 | Focused — rewrites the preprocessor line model; contained to `preprocess/mod.rs` |
| **WP7** | **ArchiMate rebuild** — retire bespoke path, route through DESCRIPTION + stdlib macros + granular oracle geometry | C5 (archimate) | 48 | Dedicated subsystem build — a whole diagram-type reimplementation. Sequence late; high effort, isolated bucket |
| **WP8** | **Residual sub-pixel + Graphviz edge order** — 1-off cleanups (mostly auto-resolved by WP1-3), dot-traversal edge emission | C6, C8 | ~46 | Quick pass + 1 deep-defer (C8 may be dot-version-sensitive; accept if intractable) |

**Sequencing logic:** WP0 banks easy wins immediately. WP1→WP2 attack the 53%-of-failures engine geometry before anything else, because (a) they are the largest yield and (b) WP3/WP4 perturb the same coordinates — fixing fonts/widths on top of *wrong* geometry would mask which fix did what. WP3 builds on WP2's corrected sequence layout. WP4 (font cascade) lands after the geometry is stable so its layout-feedback ripple is debuggable. WP5 fans out independently throughout. WP7 (ArchiMate) is isolated and can run any time after WP4 (it depends on DESCRIPTION-renderer maturity and per-font metrics).

## 5. Risks & honesty notes

- **Oracle-consumption honesty boundary (the T4.11 / inner_xml line).** The render path has a `root_g_inner_xml` verbatim-replay early-return in every renderer, but its only population site is gated `if false` at `extract.rs:217` **by deliberate policy** ("PlantUML is the North Star, not a moving target" — turning goldens into both reference and substrate is forbidden). **Do not re-enable it.** The sanctioned compromise is *granular* oracle consumption — constructing the `<g>` yourself and filling **scalar geometry** (node positions, edge bezier `d`, arrowhead points) from the oracle, as class/component/deployment already do — because the bezier router is infeasible to byte-recompute. The forbidden act is *attribute-echo*: parroting an opaque inner-XML subtree. ArchiMate (WP7) must use granular geometry consumption + real shape drawing; a prior 48→0 ArchiMate patch was **rejected** as attribute-echo self-validation. This distinction is load-bearing for every "0→full" claim.

- **Regression risk — the geometry/font perturbation zone (WP1-4).** Any width/metric change ripples to every downstream coordinate. The corpus has thousands of *passing* neighbors; a "fix" that corrects one case by shifting a shared constant can break ten. Mitigation: run the full golden suite (not `--lib`) after every metric change; never tune a sub-pixel constant without confirming net-positive delta. The font cascade (WP4) is the highest-risk because `font-family` is hard-coded at ~150 sites and the metric tables don't yet exist — changing the family attribute *without* the matching per-font table will regress currently-matching geometry. **Per project memory, visibility-icon scaling (`classAttributeIconSize`) and theme-cascade are flagged regression-risky; treat them as solo, careful, full-suite-verified work, not fan-out.**

- **Where my prior analysis (the MAP) was wrong — corrected here:**
  - **Per-bucket totals were inflated by pre-skipped error goldens.** MAP said activity 606 / sequence 399 / edge-cases 215 / preprocessing 242; the real failing set is 574 / 398 / 197 / 185. The deltas are exactly the `golden_has_syntax_error`-skipped cases. The MAP's "over-accepted invalid syntax" (activity ~110+60), "error-render goldens" (sequence 107), "foreach/welcome/rejected-construct" (preproc ~85), and most "Java emits Syntax-Error" categories are **not open failures** — they require no work and must not be "fixed" by adding an error-diagram renderer (the version banner is unmatchable; matching would require degrading RustUML to reject valid syntax).
  - **edge-cases non-ASCII metrics: T4→T2.** VERIFY proved `font_metrics.rs` (the file the MAP cited) is **dead code** — I confirmed zero `font_metrics::` importers in the render path; everything uses `plantuml_metrics`, which **already has** the per-codepoint AWT non-ASCII table the MAP said was missing. The real defect is a 1-3 line arithmetic bug: `plantuml_metrics.rs:183` recovers font size as `table[0] / 0.31640625` using the **bold** space width for bold tables, inflating every bold non-ASCII glyph by exactly 14.58333/14 = 1.041667 (Greek `54.0324` vs golden `51.8711`, etc.). That's a quick pass (folds into WP0), not a font-table subsystem. Caveat: the non-ASCII *class-diagram* cases in that bucket are mis-attributed — their textLengths already match; they fail on entity ordering/layout (a separate WP5 item).
  - **class lollipop: T5→T3, deployment nav-arrow: T4→T1(no-op), archimate: T5→T4.** All confirmed; reflected in the taxonomy and roadmap.
  - **teoz activation-model claim was wrong** (the cited "completely different activation model" is the lifeline-area rect, not the activation bar — RustUML's bar model already matches). Teoz remains T4 (separate render path) but the per-case delta is small (margins + DOM-wrapper shape), not a wholesale geometry rewrite.

- **Lowest-confidence / accept-as-ceiling candidate:** C8 (Graphviz edge-emission order, ~6 cases) is dot-version-sensitive (vendored 14.1.5 vs golden 15.0.0). If matching dot's traversal order proves intractable without re-vendoring, accept these 6 as a residual rather than chase a version-pinned ceiling.
