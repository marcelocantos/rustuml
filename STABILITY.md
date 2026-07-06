# Stability

RustUML follows semantic versioning. Version 1.0 will represent a
backwards-compatibility commitment: after 1.0, breaking changes to the
CLI interface, output format, or configuration require a major version
bump. The pre-1.0 period exists to get these right.

## Interaction surface catalogue

*Snapshot as of v0.7.0*

Parity claims in this document use the no-oracle product tier, not the
oracle-assisted strict tier. The current no-oracle baseline is 5,005/11,251
eligible SVG goldens (44.5%). The strict tier is green at 11,251/11,251
eligible SVG goldens but uses test-only oracle layout extraction. The 1,199 Java
PlantUML error-page goldens are skipped by both tiers and are not counted as
passes.

### CLI interface

| Item | Stability |
|------|-----------|
| `rustuml <file>` — render file to SVG | Stable |
| `rustuml -pipe` — stdin to stdout | Stable |
| `-tsvg` output format | Stable |
| `-tpng` output format | Stable |
| `-tpdf` output format | Needs review — svg2pdf quality |
| `-teps` output format | Needs review — raster-based, not vector EPS |
| `-ttxt` ASCII art output | Needs review — sequence only |
| `--ast` debug output | Fluid — debug tool, not user-facing |
| `--yaml` YAML output | Fluid — serialization format may change |
| `--theme=NAME` built-in themes | Needs review — theme names not finalised |
| `--theme-file=PATH` custom themes | Needs review — theme YAML schema not stable |
| `--block=N` multi-diagram selection | Needs review — new in v0.3.0 |
| `--block-name=NAME` named block selection | Needs review — new in v0.3.0 |
| `--version` | Stable |
| `--help` | Stable |
| `--help-agent` | Stable |

### Input format

| Item | Stability |
|------|-----------|
| PlantUML text syntax (`@startuml` etc.) | Stable — follows PlantUML spec |
| All 16 `@startXXX` types dispatched | Stable |
| YAML input format | Fluid — schema not documented |
| JSON input format | Fluid — schema not documented |
| TIM preprocessor (`!define`, `!if`, `!foreach`, `!function`, etc.) | Needs review — feature-complete |
| Multi-diagram files (multiple @start blocks) | Needs review — new in v0.3.0 |
| Sprite definitions (`sprite $name { }`) | Needs review — new in v0.3.0 |

### Output format

| Item | Stability |
|------|-----------|
| SVG structure (elements, attributes) | Fluid — layout and rendering actively evolving |
| PNG rasterization | Stable (via resvg) |
| PDF generation | Needs review (via svg2pdf) |
| EPS generation | Needs review — raster-based (new in v0.3.0) |

### Diagram types

| Type | Stability |
|------|-----------|
| Sequence, Activity | Needs review — high no-oracle coverage, still not exact |
| Timing, Gantt, Mindmap, WBS, Salt, Nwdiag, Regex, EBNF, DOT, Git, Math | Needs review — exact on the current no-oracle baseline |
| Class, Deployment, Use Case, Object, JSON/YAML | Fluid — partial no-oracle coverage |
| Archimate, State, Component, ER (crow's foot notation) | Fluid — no current no-oracle passes |
| Board | Fluid — no eligible SVG goldens in the no-oracle baseline |
| Ditaa | Fluid — excluded from SVG parity tiers pending a raster comparator |

## Gaps and prerequisites for 1.0

### Features
- [ ] stdlib theme/icon library support (`!include <C4/...>`)
- [x] Archimate diagram support
- [ ] Improved layout engine (edge routing quality on dense graphs)
- [ ] Extend Sugiyama layout to component/deployment/usecase/state
- [ ] Creole markup edge cases (tables, complex nesting)
- [ ] Activity diagram legacy v1 syntax

### Documentation
- [ ] Complete PlantUML syntax coverage documentation
- [ ] YAML/JSON input schema documentation
- [ ] Theme YAML schema documentation
- [ ] API documentation for library crates

### Quality
- [ ] SVG output visual fidelity audit against Java PlantUML
- [ ] Cross-platform binary testing (Windows)
- [ ] WASM compilation target
- [ ] Performance benchmarks

### Dependencies
- [x] `serde_yaml` deprecation — migrated to `serde_yml`
- [ ] Vendored Graphviz layout stability and source-availability review

## Out of scope for 1.0

- Windows native binaries (CI builds Linux/macOS only for now)
- PlantUML server mode (HTTP API)
- PlantUML `.iuml` standard library bundling
- Interactive/real-time rendering
- Custom font embedding (beyond Liberation Sans)
