# Swimlane Renderer Rewrite — Single-Tree + Per-Lane Translation

**Status:** design (branch `swimlane-single-tree`, off `bb29a924`). Target: the
~40 failing swimlane goldens under 🎯T4, without regressing the ~124 passing
ones. Baseline (163 fails) is never at risk — this branch merges only when the
full golden suite is zero-regression net-positive.

## Why the current (segment) model cannot work

`build_swimlanes` (`activity.rs:1397`) splits the `ActivityStep` stream at every
`|Lane|` marker into `LaneSegment`s (`{lane_index, body}`), each a self-contained
sub-tree, then `emit_swimlanes` (`activity.rs:11392`) stacks them **chronologically/
vertically**, each at its lane's x-column. A construct whose branches live in
different lanes is therefore *fractured* across segments and stacked.

Canonical failure — `act_fork2br_lanes2`:

```
|Lane1|
|Lane2|
start
fork
  |Lane1|
  :Branch 1;
fork again
  |Lane2|
  :Branch 2;
end fork
stop
```

- **Gold:** 232 tall × 259 wide. The fork's two branches sit **side-by-side at the
  same y**, Branch 1 in Lane1's column, Branch 2 in Lane2's column.
- **Mine (segment model):** 307 tall × 210 wide. Branch 1 (Lane1 segment) and
  Branch 2 (Lane2 segment) are **stacked vertically** → +75 height; Lane2 is
  ~49px too narrow because it never sees Branch 1's width.

No amount of geometry tuning fixes this: the segment model has no representation
for "parallel branches, same y, different lane columns." (Contrast the
`act_swimlane*_while_simple` cluster, which the segment model *does* represent
structurally and fails only on sub-pixel geometry — see
`memory/project_parity_gaps.md`. Those become moot once this rewrite lands, so do
**not** patch them in the segment model.)

## PlantUML's model (the oracle), from `Swimlanes.java`

PlantUML builds **ONE** ftile tree for the whole diagram and lays it out
**normally** (as if there were no swimlanes — so a fork's branches already get
their natural side-by-side, same-y layout). Swimlanes are applied at **draw time**
as a per-lane x-translation. Key methods (`activitydiagram3/ftile/Swimlanes.java`):

- `root.createFtile(getFtileFactory(...))` → `full`: the single tree. Every tile
  carries `getSwimlaneIn()` / `getSwimlaneOut()` (the lane active at its entry/exit),
  assigned by the factory as the instruction stream is built; `|Lane|` switches
  `currentSwimlane`.
- `computeDrawingWidths(ug, full)` (line 369): draws `full` through
  `UGraphicInterceptorAllSwimlanes` — a `UGraphic` that routes each drawn shape to
  the **per-swimlane `LimitFinder`** for whichever swimlane is active at that point.
  Result: `swimlane.setMinMax(...)` = the bounding box of *that lane's* shapes in
  the tree's own coordinate space. (Fork branch 1's box → Lane1's MinMax; branch 2's
  → Lane2's, even though both are at the same y.)
- `computeSizeInternal` (line 386): `swimlane.width = max(min, MinMax.width)`; then
  assigns each lane a cumulative x and a translate:
  `xx = xpos + dividerWidth − swimlane.minMax.minX + (actualWidth − widthWithoutTitle)/2`
  `swimlane.setTranslate(UTranslate.dx(xx))`; `xpos += actualWidth + dividerWidth`.
  So a shape at tree-x `tx` in lane `S` is drawn at `tx + S.translate.dx`
  `= laneColStart_S + (tx − S.minMax.minX) + centering` — i.e. **a single uniform
  dx per lane** maps that lane's tree-coords into its on-canvas column.
- `drawWhenSwimlanes` (line 309): for **each** lane, draws the FULL tree through
  `UGraphicInterceptorOneSwimlane(swimlane)` applied with `swimlane.getTranslate()`
  — the interceptor emits only shapes belonging to that lane, the translate shifts
  them into the lane column. Then dividers at
  `swimlane.getTranslate().getDx() + swimlane.getMinMax().getMinX()`.
- **Cross-lane connectors**: drawn ONCE via the `Cross` UGraphic (line 177), which
  only renders a `Connection` when `tile1.getSwimlaneOut() != tile2.getSwimlaneIn()`.

Crucially: **y comes straight from the normal tree layout; x is remapped per-lane
by one uniform dx.** The fork's natural branch x-spread is *replaced* by lane
columns; its y is *kept*.

## Target RustUML architecture

Reuse the existing non-swimlane single-tree layout (the bulk of `activity.rs`) and
add lane tagging + per-lane x-remap + cross-lane connectors. Do **not** reimplement
activity layout.

1. **One tree, lane-annotated.** Stop splitting in `build_swimlanes`. Build the
   normal `LayoutNode` tree (`build_tree_inner`) for the whole activity, and tag
   each node (and ultimately each emitted shape) with the swimlane active at its
   flow position. `|Lane|` markers set the "current lane"; the tag must propagate
   into fork/if/while/switch branches (each branch may switch lanes via its own
   `|Lane|`). Cleanest: thread a `current_lane` while walking steps→tree, storing a
   `lane_index` on each `LayoutNode` (or a side table keyed by node identity).

2. **Normal layout.** Lay the tree out exactly as the non-swimlane path does
   (fork branches side-by-side, same y). Emit shapes at their tree-coordinate x/y —
   but into **per-lane shape buffers** instead of one `shapes` string. The active
   lane during emit selects the buffer. (Mirror of `UGraphicInterceptorOneSwimlane`:
   one buffer per lane.) This requires threading the current lane through the emit
   functions (`emit_sequence`/`emit_if`/`emit_while`/`emit_fork`/`emit_switch`), or
   tagging each `svg.shapes` write and partitioning afterward.

3. **Per-lane MinMax.** For each lane buffer, compute `minX`/`maxX` (and the lane
   width = `maxX − minX`, floored by `skinparam swimlaneWidth` / same-width rule).
   This is the analogue of `computeDrawingWidths`.

4. **Per-lane translate + concat.** `laneColStart` accumulates
   `width + dividerWidth` left to right; `dx_lane = laneColStart − minX + centering`.
   Shift every x in that lane's buffer by `dx_lane`, then concatenate. Dividers at
   `dx_lane + minX`. (Analogue of `computeSizeInternal` + `drawWhenSwimlanes`.)

5. **Cross-lane connectors.** A flow edge whose source lane ≠ target lane (incl. the
   fork bar spanning lanes, and ordinary flow that crosses a `|Lane|`) is drawn once
   as an L-snake between the two lane columns at the appropriate y. Analogue of
   `Cross` / `ConnectionCross`. The existing deferred-cross-lane logic in
   `emit_swimlanes` (the `deferred_cross_lanes` vec) is the seed, but must generalise
   from segment-boundaries to any cross-lane edge (notably fork bars).

## Hard parts / risks

- **Shape→lane tagging through emit.** The emit functions are large and lane-unaware.
  Either thread a `lane: usize` param (invasive but explicit) or push/pop a
  `current_lane` on the `SvgEmitter` and have every `write!` to `shapes` honor it.
  The latter is less invasive; verify every shape-emitting site routes through it.
- **Fork/if with per-branch lanes.** The branch's lane is set by a `|Lane|` at the
  branch head. The fork *bar* spans all involved lanes (cross-lane). Branch bodies
  layout normally (same y) but tag to their lanes → x-remapped to columns.
- **uniform-dx assumption.** PlantUML shifts a whole lane by ONE dx. Confirm
  RustUML's emitted shapes for a lane are internally self-consistent in tree-coords
  so a single dx suffices (it should, since they share the tree layout).
- **The ~124 passing swimlane goldens** (linear, single-lane-at-a-time flows) must
  still pass. They are the segment model's success cases; the single-tree model must
  reproduce them. Gate the FULL suite continuously; this is the merge bar.

## Validation ladder (gate full suite at each rung)

1. `act_fork2br_lanes2` (1 cross-lane fork, 2 lanes) — the minimal case.
2. `act_fork{2..6}br_lanes{2,3}` (12) — fork fan-out × lane count.
3. `act_swimlane_with_fork` / `_with_if` / `_with_while` — single construct per lane.
4. `act_combo_swimlane_fork_{2,3,4}lanes`, `act_combo_swimlane_if_{2,3}`.
5. `act_swimlane*_while_simple` (the geometry cluster — should fall out for free).
6. The real-world composites (`act_complex_swim*`, `act_business_*`, `act_domain_*`,
   `combo_workflow_swimlanes`).
7. Full suite zero-regression vs the 124 passing swimlane goldens → merge bar.

## References

- PlantUML: `activitydiagram3/ftile/Swimlanes.java` (drawWhenSwimlanes /
  computeSizeInternal / computeDrawingWidths / Cross), `Swimlane.java`
  (getTranslate/getMinMax/setWidth), `UGraphicInterceptorOneSwimlane` /
  `UGraphicInterceptorAllSwimlanes`, `ConnectionCross`.
- RustUML: `activity.rs` — `build_swimlanes` (1397), `emit_swimlanes` (11392),
  `Lane`/`LaneSegment` (829/838), `lane_width` (4698), `lane_content_cx` (4707),
  `swimlane_lane_extents` (4724), and the non-swimlane tree emit
  (`emit_sequence`/`emit_fork`/`parallel_layout`).
- `memory/project_parity_gaps.md` — full T4 triage and the swimlane-while geometry
  cluster anatomy (subsumed by this rewrite).

## Cross-lane handling — the architectural core (findings 2026-06-08)

Implemented the partition + per-lane measurement (`partition_lane_buffer`,
`layout_swimlanes_v2`, hooked in `render_inner`, env-gated). Measuring
`act_fork2br_lanes2`'s per-lane natural bounds surfaced the real difficulty:

- **Fork/join bars are cross-lane shapes.** They render as wide, short rects
  (`height="6"`, `fill="#555555"`) spanning every involved lane — e.g. Lane2's
  natural bounds came out `[16, 209.7]` (w 193.7) vs gold 129.9 purely because the
  two bars (fork top + join bottom, both `width="193.7305" x="16"`) are attributed
  to the lane active when `emit_fork` drew them. They must be **excluded from
  per-lane measurement** and **redrawn spanning columns** after remap (PlantUML's
  `Cross`/`ConnectionCross`). The `height≈6` short-rect signature identifies a bar
  structurally (no color dependence).

- **Fork connectors are vertical (single-x) but mis-tagged.** The connectors from
  the bar down to each branch are vertical lines at the branch's natural x
  (`x=63.4326` for branch 1, `162.2979` for branch 2). Each belongs to its
  *branch's* lane, but `emit_fork` draws them under the *fork's entry* lane, so the
  byte-offset/`current_lane` tag is wrong for them.

- **Two lane-assignment regimes.** (a) *Temporal* — a linear `|Lane|` switch
  changes lane at the same x over time; the byte-offset spans capture this
  correctly. (b) *Spatial* — a fork's parallel branches occupy different x at the
  same time; here lane = which natural x-cluster the element sits in, NOT emit
  order. The byte-offset model alone handles (a) but not (b).

**Resolution (next push): make `emit_fork`/`emit_if` lane-aware on the V2 path.**
When a construct's branches span lanes: set `current_lane` to branch *i*'s lane
around emitting branch *i*'s inbound/outbound connectors (so they tag correctly),
and emit the fork/join bar into a dedicated cross-lane buffer (not a lane buffer).
`layout_swimlanes_v2` then: measures lanes from correctly-tagged shapes+connectors
(bars excluded), shifts each lane by its `dx`, and redraws each cross-lane bar
stretched from its leftmost to rightmost connected branch column. This keeps the
byte-offset model for linear switches and adds explicit cross-lane emission for
branch constructs — matching PlantUML's per-tile `swimlaneIn/Out` + `Cross` split.

## BREAKTHROUGH: cross-lane forks need no emit surgery (2026-06-08)

Verified on `act_fork2br_lanes2`: per-lane SHAPE clusters are DISJOINT
(Lane1 `[28, 98.9]`, Lane2 `[101.9, 197.7]`), every vertical fork connector falls
cleanly inside exactly one cluster (x=63.4→L1; 112.9→L2; 162.3→L2), and the
fork/join bars (`height="6"`, span `[16, 209.7]`) straddle both. So the cross-lane
case is handled by a PURE POST-PROCESS in `layout_swimlanes_v2`, NOT by making
`emit_fork`/`emit_if` lane-aware:

- **Shape lane** = its byte-offset `LaneMark` tag (robust even if branches are
  declared out of column order — a box tagged to its lane regardless of natural x).
- **Connector lane** = which lane's shape-cluster its x falls into (geometric).
  Branches are laid side-by-side at disjoint x, so this is unambiguous. (For LINEAR
  `|Lane|` switches the clusters overlap at the shared spine x — but those connectors
  ARE correctly byte-offset-tagged, so use the tag when clusters overlap, geometry
  when disjoint.)
- **Cross-lane element** = a `height≈6` bar, or any connector whose x-span straddles
  two clusters. Excluded from per-lane width; redrawn spanning the relevant columns.

This removes the invasive multi-function emit change from the plan. Remaining:
implement this post-process in `layout_swimlanes_v2` + the column geometry.

### Lane-width calibration gap
Box-cluster widths (L1 70.9, L2 95.9) are NARROWER than gold lane widths
(L1 80.9, L2 129.9). Gold's per-lane `MinMax` includes connector reach and the
in-lane portion of the bar, not just the boxes. So lane width must be measured over
shapes + in-lane connectors (NOT bars), and the column math must replicate
`computeSizeInternal` (dividers via `getHalfMissingSpace`=5, translate.dx). This is
the pixel-exact calibration step, done against the golden ladder.

## Column math (from source) + the per-lane MinMax blocker (2026-06-08)

Exact column geometry from `LaneDivider.java` + `Swimlanes.computeSizeInternal`:
- `LaneDivider(x1,x2,h).getWidth() = x1 + x2`; its vertical line draws at `+x1`
  within the divider region.
- `getHalfMissingSpace(i)`: 5 at the ends and whenever `titleWidth <= actualWidth`
  (the common case), else `max(5, 5 + (titleWidth-actualWidth)/2)`.
- `xpos=0; for lane i: dw = hms(i)+hms(i+1); translate.dx = xpos + dw - minMax.minX
  + (actualWidth - widthWithoutTitle)/2; xpos += actualWidth + dw`.
  `actualWidth = max(swimlaneWidth, MinMax.width)`; with distinct lane widths and
  default `swimlaneWidth`, `actualWidth = MinMax.width` so the last term is 0.
- Divider line i draws at `translate.dx_i + minMax_i.minX - hms(i+1)`.

Calibration data (`act_fork2br_lanes2`, gold): dividers `20 / 100.8652 / 230.7305`;
per-lane shift dx_L1 = −2.0, dx_L2 = +17.0; title-band y-shift = +17.4961
(= titlesHeight 12.4961 + 5); lane widths L1 80.8652, L2 129.8653; canvas 232×259.

**BLOCKER pinned:** dx_L1 = −2 with dw=10 ⇒ `minMax_L1.minX = 12`, which is LEFT of
Branch1's box (natural x=28). So PlantUML's per-lane `MinMax` is NOT just the lane's
boxes — it attributes some of the fork bar / connector geometry to each lane (the
`UGraphicInterceptorAllSwimlanes` routes each drawn shape to the active lane's
`LimitFinder`, and the bar/connectors land in specific lanes). The exact attribution
can't be read off the final SVG. **Next action (faithful, matches the compression
port method): instrument `Swimlanes.computeDrawingWidths` /
`UGraphicInterceptorAllSwimlanes` in the PlantUML checkout to dump each lane's
`MinMax` for `act_fork2br_lanes2`, then implement the column math above + the
validated geometric post-process, and iterate up the ladder.** (Revert the
instrumentation after, as with the compression work.)

## CORRECTION via PlantUML instrumentation (2026-06-08) — bars are NOT excluded

Instrumented `Swimlanes.computeSizeInternal` (printed per-lane MinMax/dx, then
reverted + rebuilt clean JAR). For `act_fork2br_lanes2`:
```
Lane1: minX=13.0  maxX=83.87   width=70.87   actualW=70.87   x1=x2=5 dw=10 dx=-3.0  xpos_before=0
Lane2: minX=-1.0  maxX=196.73  width=197.73  actualW=197.73  x1=x2=5 dw=10 dx=91.87 xpos_before=80.87
last:  empty
```

This OVERTURNS the "exclude bars" idea. The fork/join BAR is attributed to the
fork's ENTRY lane (Lane2) and is INCLUDED in that lane's MinMax — Lane2.width=197.73
is bar-dominated (the bar is ~193.7 wide). Key consequences:

1. **Byte-offset tagging is already correct for the bar.** `emit_fork` draws the bar
   under `current_lane` = the fork's entry lane (Lane2, set by the `|Lane2|` before
   `start`). So our `lane_spans` already put the bar in Lane2 — matching PlantUML.
   Do NOT exclude `height=6` bars; INCLUDE them in per-lane bounds.
2. **Lanes are placed side-by-side WIDE, then ON_X-compressed.** `xpos` accumulates
   `actualWidth + dw`: 0 → 80.87 (L1) → 288.6 (after L2). Internal width ~288, but the
   final canvas is 259 — the diagram-level CompressionXorYBuilder ON_X (which RustUML
   ALREADY has, `compress_activity_buffers`) collapses the ~29px of slack the wide
   bar/lane layout leaves. So V2 must: place lanes side-by-side via the formula, shift
   each lane's shapes+connectors by its `dx`, then run the EXISTING compress pass.
3. **Cross-lane CONNECTIONS (not bars) are the separate-draw case.** The bar stays in
   its lane; it's the connection bar→branch-in-another-lane (`swimlaneOut != swimlaneIn`)
   that PlantUML draws via `Cross`. Those are the elements needing span-redraw, not the
   bar itself.

Revised V2 plan: (a) per-lane MinMax over each lane's shapes+connectors INCLUDING
bars (byte-offset tags), (b) column placement `dx = xpos + dw - minX` with
`dw = hms(i)+hms(i+1)` (hms=5 common), `xpos += actualW + dw`, (c) shift each lane's
fragment by `dx`, (d) identify+redraw cross-lane connections spanning, (e) title band
+ dividers + titles, (f) run the existing ON_X compress pass to collapse slack, (g)
iterate. The instrument-then-revert confirmed the model before mis-implementing it.

## V2 renders lanes (milestone) + cross-lane bar geometry finding (2026-06-08)

`layout_swimlanes_v2` now produces lane-laid-out renders (partition -> measure ->
column placement -> shift_x/shift_y -> dividers + titles -> existing ON_X compress).
On `act_fork2br_lanes2`: **dividers 1&2 EXACT (20, 100.8652), Lane1/Branch1 box within
1px, lane titles rendered, height 230 vs gold 232.** Default path unchanged (zero
regression). Branch `swimlane-single-tree`.

Remaining issue = the fork bar. Gold's bar is `[106.9, 226.7]` (width 119.9) sitting
ENTIRELY within Lane2's region; **Branch1 (Lane1) connects via a separate cross-lane
horizontal connector** (gold lines `x=113.9 -> 61.4` at the bar's y), NOT by the bar
spanning to Lane1. So the earlier "redraw bar spanning both columns" idea is wrong:
- The bar stays in its OWN lane (the fork's entry lane), drawn NARROWER than its
  natural single-tree width (119.9 vs my natural 193.7).
- Each branch in a DIFFERENT lane gets a cross-lane L-snake from the bar's lane to
  that branch's column (the `Cross`/`ConnectionCross` case).

Open question (needs PlantUML instrumentation of the per-lane DRAW, i.e.
`UGraphicInterceptorOneSwimlane` / `ParallelBuilderFork`, not just MinMax): how the
bar's DRAWN width (119.9) is derived in the swimlane case — it is narrower than the
natural fork-bar span, so the fork tile must re-derive the bar to cover only its own
lane's branch reach + the cross-lane connector stubs. Next: instrument that, then
implement bar-stays-in-lane + cross-lane L-snake for out-of-lane branches.

## Fork bar is DECOMPOSED per-lane (2026-06-08, from multi-golden data)

`act_fork3br_lanes2` (3 branches, 2 lanes) has TWO bars in DIFFERENT lanes with
DIFFERENT widths: Lane1 bar `[26, 233.7]` w=207.7 (covers the 2 Lane1 branches at
73.4, 186.3), Lane2 bar `[243.7, 338.6]` w=94.9 (covers the 1 Lane2 branch at 291.2).
`act_fork2br_lanes2` (1 branch/lane) shows ONE bar (in the fork's lane) + a cross-lane
connector to the other lane's single branch.

So the fork/join bar is NOT one element translated — it is **decomposed per lane**:
each lane draws the bar segment spanning ITS branches; lanes are joined by cross-lane
connectors at the bar's y. A lane with multiple fork branches gets a real bar segment
(to split/merge flow among them); a lane with a single branch may get just the
connector. This is `ParallelBuilderFork`'s swimlane-aware construction
(`AbstractParallelFtilesBuilder` + the per-swimlane draw), genuinely case-dependent on
branches-per-lane.

Implication: V2's "bar stays in one lane" is still too simple. The faithful path is to
instrument `ParallelBuilderFork`/`FtileFactoryDelegatorCreateParallel` in the swimlane
case to see how the bar is split + where cross-lane connectors attach, then implement
per-lane bar segments + cross-lane joins. This is the deepest sub-system of the rewrite
and is genuinely multi-session; the ~40-golden ladder then needs per-case iteration.
