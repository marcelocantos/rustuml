# TODO

## High Priority
- [ ] stdlib includes — `!include <C4/...>`, `!include <awslib/...>` not resolved (need to bundle or fetch PlantUML stdlib)
- [ ] Archimate diagram support (`@startuml` with archimate keywords)
- [ ] Layout engine: extend Sugiyama to component/deployment/usecase/state renderers (currently grid)
- [ ] Layout engine: improve edge routing quality on dense graphs

## Medium Priority
- [ ] ASCII art output for non-sequence diagram types
- [ ] Hyperlinks in SVG output (SVG builder has `open_link`/`close_link` but not wired into renderers)
- [ ] Creole edge cases: tables, complex nesting, tree lists
- [ ] Activity diagram legacy v1 syntax
- [ ] `serde_yaml` deprecation — migrate to maintained alternative
- [ ] Format-parameterized golden test framework (validate PNG/PDF output)

## Low Priority
- [ ] SCXML output for state diagrams
- [ ] Responsive SVG with proper viewBox
- [ ] PlantUML server mode (HTTP API)
- [ ] WASM compilation target
- [ ] Windows native binaries
- [ ] Performance benchmarks
- [ ] Complex nested TIM macro edge cases

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
