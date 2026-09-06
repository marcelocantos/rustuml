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
| activity | 6 | 13424 | 918060 | 125 | 872 |
| archimate | 6 | 3611 | 217676 | 147 | 78 |
| chart | 6 | 7525 | 429207 | 120 | 201 |
| class | 6 | 8750 | 479189 | 160 | 554 |
| combo | 6 | 388271 | 294368629 | 404 | 28734 |
| component | 6 | 4837 | 296389 | 86 | 340 |
| creole | 6 | 6484 | 274133 | 66 | 336 |
| deployment | 6 | 2743 | 110914 | 93 | 16 |
| ebnf | 6 | 12124 | 813967 | 70 | 828 |
| edge-cases | 6 | 58844 | 3240233 | 865 | 2490 |
| er | 6 | 32096 | 1778475 | 542 | 1807 |
| gantt | 6 | 26254 | 1411862 | 173 | 1222 |
| git | 6 | 2110 | 73353 | 44 | 15 |
| json-yaml | 6 | 3114 | 295139 | 49 | 83 |
| links | 6 | 7293 | 406790 | 149 | 488 |
| math | 6 | 1532 | 71661 | 17 | 34 |
| mindmap | 6 | 6366 | 294097 | 63 | 210 |
| multi-diagram | 6 | 6225 | 279310 | 118 | 264 |
| nwdiag | 6 | 9870 | 523165 | 245 | 292 |
| object | 6 | 5492 | 304340 | 131 | 449 |
| preprocessing | 6 | 75725 | 4082733 | 2663 | 545 |
| regex | 6 | 3825 | 162825 | 19 | 173 |
| rendering | 6 | 8353 | 421222 | 143 | 726 |
| salt | 6 | 5459 | 272654 | 74 | 156 |
| sequence | 6 | 11898 | 623330 | 163 | 344 |
| skinparam | 6 | 9994 | 588148 | 153 | 514 |
| sprites | 6 | 2199 | 104428 | 67 | 99 |
| state | 6 | 8835 | 641091 | 125 | 1222 |
| timing | 6 | 11768 | 525036 | 237 | 454 |
| type-detection | 6 | 8181 | 451380 | 114 | 614 |
| usecase | 6 | 6590 | 328458 | 154 | 139 |
| wbs | 6 | 4385 | 200648 | 63 | 134 |
| TOTAL | 192 | 764177 | 314988542 | 7642 | 44433 |
<!-- perf-baseline:end -->

## Whole-corpus throughput (informational)

`rustuml-bench run --serial`, three rounds, same load conditions as
above:

| Round | Wall | Files/s |
|---|---|---|
| 1 | 5.40 s | 2305 |
| 2 | 5.20 s | 2396 |
| 3 | 7.15 s | 1742 |

12,449 files, 12,373 rendered, 76 parse failures, 0 panics, 71.8 MB of
SVG. Per-family render totals, serial: `dot` 2444 ms (25 files, external
`dot` process), `activity` 1142 ms (1497 files), `state` 334 ms (1085),
`class` 189 ms (2140), `preprocessing` 150 ms render plus 291 ms parse
(394).

## History

| Date | Change | TOTAL allocs | TOTAL bytes | TOTAL render_us |
|---|---|---|---|---|
| 2026-09-06 | harness added, pre-fix lock | 764177 | 314988542 | 44433 |
