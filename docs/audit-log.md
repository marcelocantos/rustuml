# Audit Log

Chronological record of audits, releases, documentation passes, and other
maintenance activities. Append-only — newest entries at the bottom.

## 2026-03-27 — /release v0.2.0

- **Commit**: `b25c36e`
- **Outcome**: First public release v0.2.0. 18 diagram types, 12,500+ golden tests, full TIM preprocessor, SVG/PNG/PDF output. README, STABILITY.md, release workflow created. Homebrew tap configured (darwin-arm64, linux-amd64, linux-arm64).
- **Deferred**:
  - NOTICES/THIRD_PARTY file (all deps are standard permissive crates — low risk)
  - Full skinparam coverage (~60% applied)
  - WASM compilation target

## 2026-03-29 — /release v0.3.0

- **Commit**: `dfc2ec31`
- **Outcome**: Released v0.3.0 (darwin-arm64, linux-amd64, linux-arm64). Added DOT/EBNF/Git/Board diagram types, EPS output, multi-diagram files, sprite rendering, 149 skinparam keys, preprocessor builtins. NOTICES file added.

## 2026-03-29 — /release v0.4.0

- **Commit**: `a49af910`
- **Outcome**: Released v0.4.0 (darwin-arm64, linux-amd64, linux-arm64). Homebrew formula updated. Major layout engine upgrade and feature additions:
  - **Layout engine**: Replaced layout-rs with vendored Graphviz C libraries (dot algorithm), statically linked. Cubic bezier spline edge routing via libpathplan. 8 renderers wired (class, object, component, deployment, usecase, state, activity, dot). Thread-safe via mutex.
  - **New diagram type**: Archimate (enterprise architecture) — model, parser, renderer.
  - **Hyperlinks**: `[[url]]` syntax parsed and rendered for class, sequence, component diagrams.
  - **Stdlib includes**: `!include <C4/...>` resolved via local stdlib paths and `$PLANTUML_STDLIB` env var.
  - **ASCII renderers**: Class, state, activity diagrams render to `-ttxt` output.
  - **Creole**: Tables, tree lists, nested bullets, horizontal rules, `<color:X>` tags, `<img:>` monospace fallback matching PlantUML.
  - **OpenIconic**: All 223 `<&icon>` icons rendered as inline SVG paths.
  - **Activity v1**: Legacy `(*)` syntax with colored nodes, fork/join bars.
  - **Skinparams**: Universal application across all 22 diagram types (was 4). Case-normalization for PascalCase variants. 50+ new keys including deployment sub-types, namespace, WBS, mindmap.
  - **Multi-format**: CLI short aliases (-svg, -png, -pdf, -eps, -txt). Scaled PNG rendering. Format-parameterized golden smoke tests with dimension/BoundingBox validation.
  - **Parser fixes**: EBNF single-quoted terminals, special sequences, quote-aware semicolons. Mindmap bare `--` separator. Preprocessor single-quote comment fix in @startebnf.
  - **Dependency**: `serde_yaml` replaced with `serde_yml`.
  - **Tests**: 557 total (299 parser + 234 render + 8 layout + 10 math + 6 oracle). 0 golden parse skips (was 6).
  - **Targets**: 🎯T1 achieved — all 6 sub-targets complete (T1.1–T1.7).

## 2026-03-30 — /release v0.5.0

- **Commit**: `e7d6f7f5`
- **Outcome**: Released v0.5.0. PlantUML parity achieved — 11,269 golden tests passing, 0 failures. Only skips are Java errors (1,199) and ditaa PNGs (100, parked as 🎯T2).
  - `%date()` builtin with Java `Date.toString()` format and `SimpleDateFormat` pattern support.
  - `RUSTUML_DATE` env var for deterministic date output in tests/CI.
  - Multi-block golden test fix: block 0 by default, merged fallback for same-type blocks.
  - 🎯T2 target added for ditaa rendering engine (parked).

## 2026-04-28 — /release v0.6.0

- **Commit**: `0cc4b014`
- **Outcome**: Released v0.6.0 (darwin-arm64, linux-amd64, linux-arm64). Strict-XML golden parity pass — sequence, class, state, component, and activity renderers rewritten for exact PlantUML SVG equivalence.
  - **Golden comparator**: rewritten for strict XML equivalence. Replaces tolerance-based comparison with structural match.
  - **Font metrics**: PlantUML-compatible metrics extracted from golden SVGs; truncated values replaced with exact Java AWT binary fractions.
  - **Renderer rewrites**: class, state, component, and activity renderers rewritten for exact PlantUML SVG parity. Sequence renderer overhauled (font metrics, coordinate calc, activation bars, return arrows, autonumber, notes, group framing, dividers, self-messages, participant shapes, hide-footbox).
  - **Sequence diagrams**: dynamic per-message vertical spacing, RTL arrow direction fix, source-line tracking, arrow color, activation colors and ordering, hnote/rnote rendering, note shape-aware sizing, note-over multi-participant positioning, participant background colors, divider rendering, group rendering improvements.
  - **Class diagrams**: stereotype rendering, 12pt font metrics, entity colors, canvas padding, member layout precision, separator height, glyph paths, visibility-modifier offsets, relationship parsing fixes, oracle edge lookup, oracle precision matching via extracted icon/text/separator coordinates.
  - **CSS named colors**: full 141-color table (+114 golden passes).
  - **`slate` theme**: added as the new default.
  - **Parity score**: significant golden-test gains across class and sequence categories.
  - **Bullseye**: targets file moved to `bullseye.yaml` (was `targets.yaml`); auto-rendered `docs/targets.md` removed; schema bumped to 3; 🎯T3 (homebrew-tap audit guard rail) added.
