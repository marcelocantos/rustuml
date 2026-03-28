# TODO

## High Priority
- [ ] Remaining skinparam keys — ~60% applied, ~40% silently ignored (see golden test `skinparam/` directory)
- [ ] Sprite rendering — `sprite` blocks parsed but not rendered; sprites referenced via `<$sprite>` not drawn
- [ ] stdlib includes — `!include <C4/...>`, `!include <awslib/...>` etc. not resolved
- [ ] `!import` directive not implemented
- [ ] Archimate diagram support (`@startuml` with archimate keywords)
- [ ] CI release workflow still failing (v0.2.0 asset upload)

## Medium Priority
- [ ] @startdot — Graphviz DOT pass-through (26 golden tests)
- [ ] @startebnf — EBNF grammar diagrams (26 golden tests)
- [ ] @startboard — Kanban boards (25 golden tests)
- [ ] @startgit — Git log visualisation (25 golden tests)
- [ ] Multi-diagram files — Java PlantUML renders all blocks; we render only the first
- [ ] ASCII art output improvements (-ttxt)
- [ ] EPS output format
- [ ] Hyperlinks in SVG output
- [ ] Layout engine: improve edge routing quality on dense graphs (layout-rs limitation)
- [ ] Layout engine: extend Sugiyama layout to component/deployment renderers

## Low Priority
- [ ] SCXML output for state diagrams
- [ ] Responsive SVG with proper viewBox
- [ ] Format-parameterized test framework (validate PNG/PDF golden files)
- [ ] Complex nested TIM macro edge cases

## Done
- [x] 12,568 golden test pairs (11,104 pass, 0 fail)
- [x] 18 diagram types parsed and rendered
- [x] Full TIM preprocessor (!while, !function, !foreach, !include, !theme, etc.)
- [x] SVG, PNG (resvg), PDF (svg2pdf) output
- [x] KaTeX math rendering (rustuml-math crate)
- [x] YAML/JSON input with serde on all model types
- [x] Score-based diagram type detection
- [x] Layout engine with timeout guard and content-aware node sizing
- [x] Creole markup (bold, italic, underline, strike, color, size, font, nested lists, tree)
- [x] Theme system with 17+ skinparam keys wired into renderers
- [x] Embedded Liberation Sans font for font metrics
- [x] Notes, stereotypes, legends, headers/footers across all types
- [x] --help-agent flag
- [x] Systematic test matrix framework
- [x] CI lint fix (cargo fmt)
