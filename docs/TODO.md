# TODO

## High Priority
- [ ] Layout engine: improve edge routing quality on dense graphs (bezier routing via Graphviz works, but dense graph quality needs tuning)
- [ ] Complex nested TIM macro edge cases (1 remaining mindmap parse failure)
- [ ] Remaining ~15% of skinparam keys not applied to renderers

## Medium Priority
- [ ] Golden comparison for non-SVG formats (currently smoke-only — validates no crash + correct headers)
- [ ] Deeper creole edge cases beyond tables/trees/nested lists

## Low Priority
- [ ] SCXML output for state diagrams
- [ ] Responsive SVG with proper viewBox
- [ ] PlantUML server mode (HTTP API)
- [ ] WASM compilation target
- [ ] Windows native binaries
- [ ] Performance benchmarks
- [ ] Complex nested TIM macro edge cases

## Done (PR #49 — v0.4.0-dev)
- [x] Replace layout-rs with vendored Graphviz (dot algorithm, statically linked)
- [x] Bezier edge routing via cubic spline extraction from Graphviz
- [x] Layout extended to 8 renderers: class, object, component, deployment, usecase, state, activity, dot
- [x] Archimate diagram support (parser + renderer)
- [x] Stdlib includes (`!include <C4/...>`, `!include <awslib/...>`) bundled and resolved
- [x] Hyperlinks (`[[url]]`) wired into SVG for class, sequence, component diagrams
- [x] ASCII renderers for class, state, activity diagrams
- [x] Creole tables, tree lists, nested lists, horizontal rules
- [x] Activity diagram legacy v1 syntax
- [x] `serde_yaml` deprecation — migrated to `serde_yml`
- [x] Format-parameterized golden smoke tests (PNG/PDF/EPS no-crash validation)

## Done (v0.3.0)
- [x] 183 skinparam keys wired (was 34)
- [x] Sprite rendering (parsing + `<$name>` inline images)
- [x] `!import`, `!includeurl`, `!dump_memory` directives
- [x] `%float`, `%dec2hex`, `%hex2dec`, `%dirpath`, `%feature` built-in functions
- [x] Multi-diagram files (`parse_all`, `parse_block`, `parse_named`, `--block`, `--block-name`)
- [x] EPS output format (`-teps`)
- [x] DOT, EBNF, Git, Board diagram types
- [x] CI release workflow fixed (v0.3.0 released successfully)
- [x] NOTICES file for third-party attribution

## Done (v0.2.0 and earlier)
- [x] 12,568 golden test pairs
- [x] 22 diagram types parsed and rendered
- [x] Full TIM preprocessor
- [x] SVG, PNG, PDF, EPS, ASCII output
- [x] KaTeX math rendering
- [x] YAML/JSON input
- [x] Score-based diagram type detection
- [x] Layout engine with timeout guard
- [x] Creole markup
- [x] Theme system (2 built-in + custom YAML)
- [x] Embedded Liberation Sans font
- [x] -pipe mode
