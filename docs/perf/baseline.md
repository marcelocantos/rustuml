# Runtime performance baseline

Locked numbers for the shipped render path (`rustuml file.puml`: parse via
`split_blocks`/`parse_block`/`parse_auto_with_base`, render via
`render_svg_with_theme` with the default theme, no oracle layout), measured
over the golden corpus `test-diagrams/golden` (submodule `9efdc12f`).

## Harness

| Command | What it measures |
|---|---|
| `cargo run -p rustuml-bench --release -- run [--serial] [--family F] [--top N]` | Whole corpus (12,449 files): wall, per-family totals, slowest files |
| `cargo run -p rustuml-bench --release -- check [--write]` | The ratchet below: deterministic sample, allocation counts, wall |
| `cargo run -p rustuml-bench --release -- profile <family\|name> --iters N` | Loop one family for `sample`/`samply` |
| `cargo bench -p rustuml-bench` | criterion: `parse/`, `render/`, `e2e/` per family, `layout/` synthetic |
| `make perf` | Runs `check`; part of `make bullseye` |

## Ratchet (locked both directions)

The sample is derived from the corpus by rule (every `n/6`-th file per
family in sorted order, first file at each stride point that renders
cleanly under 1 s), so it cannot be hand-picked. The `dot` family is
excluded: its shipped path pipes the graph through an external `dot`
binary when one is on PATH, so its numbers are host-dependent.

Gated metrics are **allocs** and **bytes** (counts from a wrapping global
allocator; Graphviz's C mallocs are not counted). They are deterministic,
so the gate is ±1 % in either direction: a regression fails, and an
improvement that is not re-locked with `check --write` on the same change
also fails. Wall times (`parse_us`, `render_us`, single-threaded minimum
of three repeats) are informational unless `RUSTUML_PERF_WALL_GATE=1`
(±25 %); they are host- and load-dependent.

Reference host: Apple M4 Max, 16 cores, macOS 26, rustc 1.96.0.

**The wall columns below were taken with sibling agents building in
parallel (load average ≈ 90–120).** They are not a quiet-machine
measurement and should not be read as one. The allocation columns are
load-independent: they reproduced to the digit across separate runs on
separate worktrees, apart from a four-byte wobble in `type-detection`
from hash-seed-dependent container growth, which is what the ±1 % band is
for.

<!-- perf-baseline:begin -->
| family | files | allocs | bytes | parse_us | render_us |
|---|---:|---:|---:|---:|---:|
| activity | 6 | 12583 | 911993 | 116 | 694 |
| archimate | 6 | 3611 | 217676 | 144 | 76 |
| chart | 6 | 7075 | 425884 | 105 | 134 |
| class | 6 | 8632 | 477377 | 138 | 376 |
| combo | 6 | 141176 | 33185001 | 352 | 10268 |
| component | 6 | 4597 | 294020 | 98 | 287 |
| creole | 6 | 6394 | 273018 | 60 | 295 |
| deployment | 6 | 2743 | 110914 | 92 | 15 |
| ebnf | 6 | 8489 | 786047 | 60 | 321 |
| edge-cases | 6 | 57990 | 3232658 | 835 | 2196 |
| er | 6 | 31705 | 1774614 | 454 | 1465 |
| gantt | 6 | 21226 | 1374653 | 179 | 666 |
| git | 6 | 2110 | 73353 | 44 | 15 |
| json-yaml | 6 | 2816 | 293026 | 50 | 49 |
| links | 6 | 7213 | 405243 | 124 | 353 |
| math | 6 | 1502 | 71441 | 17 | 29 |
| mindmap | 6 | 5783 | 289689 | 61 | 129 |
| multi-diagram | 6 | 6106 | 278318 | 92 | 177 |
| nwdiag | 6 | 8942 | 516189 | 222 | 140 |
| object | 6 | 5358 | 302415 | 127 | 333 |
| preprocessing | 6 | 75369 | 4080098 | 2673 | 516 |
| regex | 6 | 3179 | 158319 | 18 | 96 |
| rendering | 6 | 8196 | 419279 | 161 | 655 |
| salt | 6 | 5122 | 270325 | 71 | 108 |
| sequence | 6 | 11740 | 622179 | 182 | 358 |
| skinparam | 6 | 9614 | 585225 | 153 | 438 |
| sprites | 6 | 2116 | 103684 | 54 | 55 |
| state | 6 | 7896 | 632950 | 141 | 1109 |
| timing | 6 | 9918 | 511016 | 212 | 205 |
| type-detection | 6 | 7068 | 442502 | 108 | 358 |
| usecase | 6 | 6414 | 327211 | 151 | 109 |
| wbs | 6 | 3888 | 197014 | 58 | 71 |
| TOTAL | 192 | 496571 | 53643331 | 7352 | 22096 |
<!-- perf-baseline:end -->

## Whole-corpus throughput (informational)

`rustuml-bench run --serial` over all 12,449 files, binaries alternated
three rounds each so that the load drift hits both sides equally:

| Round | Before | After |
|---|---|---|
| 1 | 5.40 s (2305 files/s) | 4.01 s (3102 files/s) |
| 2 | 5.20 s (2396 files/s) | 4.49 s (2771 files/s) |
| 3 | 7.15 s (1742 files/s) | 5.59 s (2228 files/s) |

Both binaries render 12,373 of 12,449 files, fail to parse the same 76,
panic on none, and emit 71.8 MB of SVG.

Per-family render totals, serial, before the fixes: `dot` 2444 ms (25
files, external `dot` process), `activity` 1142 ms (1497 files), `state`
334 ms (1085), `class` 189 ms (2140), `preprocessing` 150 ms render plus
291 ms parse (394).

## Profile findings and fixes (2026-09-06)

Profiled with `sample` on the `profiling` build (release codegen plus
line tables), driven by `rustuml-bench profile <family> --iters N`.

| Family | Share of render | Mechanism | Fix |
|---|---|---|---|
| activity | 53 % | `compress::attr_val`/`set_attr`/`rewrite_attr` compiled a fresh `Regex` per attribute per element | thread-local compiled-regex cache keyed by pattern |
| class / state | 48 % / 31 % | `LayoutGraph::layout_full` spawned an OS thread per layout, though Graphviz is serialised behind one lock anyway | one persistent layout worker thread fed by a channel |
| all | 7–10 % | `fmt_coord` formatted through `format!("{:.4}")`, hitting the exact float formatter (Grisu falling back to Dragon) | integer tick split, proven byte-identical by `fmt_coord_matches_float_formatter` |

Effect on the ratchet sample: 764,177 → 496,571 allocations (−35 %) and
315 MB → 53.6 MB (−83 %). The `combo` family carries most of the bytes;
its 294 MB → 33 MB is the regex cache no longer rebuilding an automaton
per attribute on the largest diagrams in the corpus.

## The layout curve, and why the budget is not the fix (2026-09-07)

Graphviz layout time is superlinear in node count, and the criterion
`layout/200` bench exceeds the 5 s budget. That prompted 🎯T18. What the
investigation actually found:

**The cost is not monotonic in node count.** Repeating each size three
times back to back in one process, on the synthetic tree-plus-cross-edge
graph the bench uses:

| Nodes | Edges | Seconds (three runs) |
|---|---|---|
| 150 | 293 | 5.39, 7.19, 7.82 |
| 175 | 348 | 1.54, 1.44, 1.44 |
| 200 | 397 | 30.28, 28.18, 21.62 |

175 nodes is five times faster than 150, reproducibly. So **no node count
predicts whether a layout will finish**, and neither a pre-emptive size
guard nor a size-scaled budget can be founded on one. That is the case
for a budget with a loud fallback, which is what was built.

**All of the cost is structure, not size.** The same generator with the
cross-edges removed lays out a 300-node tree in 5 ms. The blow-up comes
entirely from the long-range back-edges, which make the graph dense and
cyclic.

**Where the time goes.** `sample` on the `profiling` build, 200 nodes:
9,530 of 9,856 samples are inside `dot_position`, and within it `rank2`
and `enter_edge` — Graphviz's network simplex for x-coordinate
assignment. Nothing in rustuml's own wrapper appears; edge construction
is a hash lookup per edge.

**Graphviz's documented effort limits did not move it.** `mclimit` was
swept over 0.01, 0.1, 0.5 and 1, and `nslimit` over 0.1, 0.5, 1 and 2.
Neither produced a trend at any size; the smallest `nslimit` values
measured slower, not faster. These runs were taken at load average 90–120
and are individually noisy, so the honest statement is that no effect was
found, not that none exists.

**No real diagram is anywhere near this.** Instrumenting every
`layout_full` call across the whole golden corpus: 5,612 layouts, of
which 5,422 are 10 nodes or fewer, 157 are 11–25, 33 are 26–50, and the
largest is 50. The 200-node case is untested territory rather than a
regression, which is why the silent-degradation fix was done first and
unconditionally and the curve was left as 🎯T18.

## Output equivalence

These are meant to be pure performance changes, and `fmt_coord` in
particular decides the digits in every SVG coordinate. Two checks:

- `cargo test -p rustuml-oracle --test golden_pairs` renders the corpus
  and compares against the checked-in golden SVGs. Before and after:
  12,550 total, 11,250 passed, 1 failed (a `deployment` XML diff that
  predates this branch), 1,299 skipped, 0 panics. Identical on both.
- `fmt_coord_matches_float_formatter` asserts the new integer split
  against the old float formatter over 200,000 pseudo-random magnitudes
  from 1e-5 to 1e7 in both signs, plus every 1e-4 tick boundary in
  [0, 2) nudged seven ways.

## History

| Date | Change | TOTAL allocs | TOTAL bytes | TOTAL render_us |
|---|---|---|---|---|
| 2026-09-06 | harness added, pre-fix lock | 764177 | 314988542 | 44433 |
| 2026-09-06 | regex cache, layout worker, `fmt_coord` integer split | 496571 | 53643331 | 22096 |
