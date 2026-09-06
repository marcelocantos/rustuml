# Build performance audit — 2026-09-06

Build system: cargo workspace of seven crates, driven locally by
`make bullseye` and in CI by three GitHub Actions jobs. Host: Apple M4
Max, 16 cores, macOS 26, rustc 1.96.0, cargo 1.96.0.

**Measurement caveat.** Every number here was taken with a dozen sibling
agents compiling and testing on the same machine, load average 70–120.
Absolute wall times are therefore pessimistic and noisy. Where a claim
depends on a comparison, the two sides were measured alternately, in
several rounds, so the drift lands on both.

## Summary

| Mode | Before | After |
|---|---|---|
| Clean debug build, whole workspace | 25–33 s | 25–33 s (no separable change under saturation) |
| Graphviz build script alone (`cargo build -p rustuml-layout` from clean) | 11.76 / 12.15 / 12.57 s | 3.61 / 3.20 / 4.86 s |
| No-op incremental | 0.06 s | unchanged |
| One-file incremental, whole workspace | 1.6 s | unchanged |
| Clean test build (`cargo test --workspace --lib --no-run`) | 41.4 s | unchanged |

One fix was applied. It makes the vendored Graphviz compile 3.3× faster
in isolation and reproducibly so. It does not show up in the
whole-workspace clean build on this machine right now, because with load
average 90 there are no idle cores for it to spread into and the build
script overlaps with other crates anyway. On an idle machine or a CI
runner where the build script sits alone at the head of the critical
path, the isolated number is the one that applies.

## Baseline

`cargo build --timings`, clean debug build: 208 units, 159.9 unit-seconds
of work compressed into 25.1 s of wall. Longest units:

| Duration | Starts at | Unit |
|---|---|---|
| 14.51 s | 3.98 s | `rustuml-layout` build script (vendored Graphviz C) |
| 13.91 s | 1.42 s | `libquickjs-sys` build script (QuickJS C) |
| 6.57 s | 6.77 s | `rustuml-parser` |
| 5.88 s | 18.58 s | `rustuml-render` |
| 4.71 s | 4.46 s | `read-fonts` |
| 4.56 s | 8.23 s | `write-fonts` |

Clean test build: 258 units, 263.9 unit-seconds, 41.3 s wall. The same
two build scripts lead, at 24.5 s and 20.7 s, followed by `rustuml-parser`
(14.2 s) and `rustuml-render` (11.6 s).

Two C build scripts are 28 s of the 25 s clean build's critical path
region and 45 s of the test build's. Nothing else comes close.

## Findings

### Applied — `cc` compiled Graphviz serially (High, low risk)

`crates/rustuml-layout/build.rs` hands about a hundred vendored Graphviz
`.c` files to `cc::Build`. The `parallel` feature of `cc` is opt-in, and
`crates/rustuml-layout/Cargo.toml` asked for plain `cc = "1"`, so every
file was compiled one at a time on one core while fifteen sat idle.

Fix: `cc = { version = "1", features = ["parallel"] }`. `cc` then draws
tokens from cargo's jobserver, so it respects `-j` rather than
oversubscribing.

Measured in isolation, alternating three rounds:

| Round | Serial | Parallel |
|---|---|---|
| 1 | 11.76 s | 3.61 s |
| 2 | 12.15 s | 3.20 s |
| 3 | 12.57 s | 4.86 s |

Correctness: `cargo test -p rustuml-layout` passes, `make perf` is green,
and `golden_pairs` renders the corpus to the same result as before the
change (12,550 total, 11,250 passed, 1 pre-existing `deployment` XML diff,
0 panics). Graphviz drives layout for class, state, component and
deployment diagrams, so an archive built wrongly would show up there.

### Deferred — `rustuml-render` is a 92,790-line crate (Medium, high risk)

Cargo's compilation unit is the crate, so every edit anywhere in
`rustuml-render` recompiles all of it. `activity.rs` alone is 29,133
lines, most of it golden-fixture tests pulled in with `include_str!`,
which is also why the crate's test build (11.6 s) costs more than twice
its lib build (5.9 s).

A one-file incremental rebuild is 1.6 s today, comfortably under the ~5 s
mark where a split starts to pay for itself, so this is not yet urgent.
It will become urgent as the crate grows. Splitting per diagram family
(`activity`, `sequence`, `class`, …) behind the shared `plantuml_metrics`
and `compress` modules is the natural seam. Not attempted here: it is an
architectural change, not a build-flag change, and it would touch every
renderer.

### Deferred — two copies of the resvg stack (Low, medium risk)

`rustuml-render` depends on `resvg` 0.47 directly and on `svg2pdf` 0.13,
which pins `resvg` 0.45.1. Both stacks build: two `resvg`, two `usvg`,
two `tiny-skia`, two `png`, two `roxmltree`, three `kurbo`, and so on.
The whole SVG stack is 24.1 of the clean build's 159.9 unit-seconds, of
which roughly 8 s is the duplicated older half.

Unifying means moving to a `svg2pdf` that tracks `resvg` 0.47, or pinning
`resvg` back to 0.45. Either changes a rendering dependency version, so it
belongs in a change whose oracle is the golden corpus, not in a build
tweak. Eight unit-seconds spread over sixteen cores is a fraction of a
second here; on a two-core CI runner it is worth more.

### Deferred — `libquickjs-sys` build script, 14–21 s (Low, no fix)

`rustuml-math` uses `katex`, which runs the real KaTeX JavaScript through
`quick-js`, so a JavaScript engine gets compiled from C on every clean
build. It is the second-largest unit in every profile. There is no
cheaper way to run KaTeX; the alternative backends are also JS engines.
Recorded so the next person does not spend an afternoon rediscovering it.

### Deferred — CI builds the workspace three times (Medium, not applied)

`.github/workflows/ci.yml` has three jobs — `check`, `test-unit`,
`test-oracle` — each on its own runner with its own `Swatinem/rust-cache`
entry, so each pays for its own compile of the workspace and its
dependency graph. `check` additionally runs `cargo clippy` and then
`cargo build`, which do not share artifacts.

Merging `check` and `test-unit` into one job would remove one full
workspace build per CI run. That is a change to shared CI, whose blast
radius is everyone, so per this audit's rules it is proposed rather than
applied.

### Checked, not a problem

- No `[profile.dev]` release knobs. There is no `codegen-units = 1`, no
  `lto`, no `opt-level = 3` in dev. The only added profile is
  `profiling`, which inherits release and exists for `sample`.
- `resolver = "2"` is set on the workspace.
- CI already uses `Swatinem/rust-cache@v2` in all three jobs, and caches
  the PlantUML jar separately.
- `split-debuginfo` needs no override: the debug build produces no `.dSYM`
  bundles, so macOS is already using the unpacked default.
- No `cargo clean` in any script or workflow.
- No `--test-threads=1` anywhere.

## Method

```
cargo build --workspace --timings          # clean, into a scratch CARGO_TARGET_DIR
cargo test --workspace --lib --no-run --timings
cargo build --workspace                    # no-op, then after touching one leaf file
cargo build -p rustuml-layout              # build script in isolation, alternating rounds
cargo tree --duplicates
cargo tree --workspace -i <crate>
```

Clean builds were measured into a scratch `CARGO_TARGET_DIR` rather than
by `cargo clean`, so the worktree's own incremental cache was never
destroyed. Timing reports were parsed out of
`target/cargo-timings/cargo-timing-*.html`.
