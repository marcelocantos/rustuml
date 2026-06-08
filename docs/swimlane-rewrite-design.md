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
