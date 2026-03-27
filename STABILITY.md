# Stability

RustUML follows semantic versioning. Version 1.0 will represent a
backwards-compatibility commitment: after 1.0, breaking changes to the
CLI interface, output format, or configuration require a major version
bump. The pre-1.0 period exists to get these right.

## Interaction surface catalogue

*Snapshot as of v0.2.0*

### CLI interface

| Item | Stability |
|------|-----------|
| `rustuml <file>` — render file to SVG | Stable |
| `rustuml -pipe` — stdin to stdout | Stable |
| `-tsvg` output format | Stable |
| `-tpng` output format | Stable |
| `-tpdf` output format | Needs review — svg2pdf quality |
| `-ttxt` ASCII art output | Needs review — sequence only |
| `--ast` debug output | Fluid — debug tool, not user-facing |
| `--yaml` YAML output | Fluid — serialization format may change |
| `--theme=NAME` built-in themes | Needs review — theme names not finalised |
| `--theme-file=PATH` custom themes | Needs review — theme YAML schema not stable |
| `--version` | Stable |
| `--help` | Stable |
| `--help-agent` | Stable |

### Input format

| Item | Stability |
|------|-----------|
| PlantUML text syntax (`@startuml` etc.) | Stable — follows PlantUML spec |
| YAML input format | Fluid — schema not documented |
| JSON input format | Fluid — schema not documented |
| TIM preprocessor (`!define`, `!if`, etc.) | Needs review — mostly complete |

### Output format

| Item | Stability |
|------|-----------|
| SVG structure (elements, attributes) | Fluid — layout and rendering actively evolving |
| PNG rasterization | Stable (via resvg) |
| PDF generation | Needs review (via svg2pdf) |

### Diagram types

| Type | Stability |
|------|-----------|
| Sequence, Class, State, Activity | Needs review — core types, mostly complete |
| Component, Deployment, Use Case | Needs review — parsers recently rewritten |
| Object, Timing, Gantt, Mindmap, WBS | Needs review — newer implementations |
| JSON/YAML, Salt, Nwdiag, Regex, Ditaa, Math | Fluid — recently added, less tested |
| ER (crow's foot notation) | Fluid — piggybacks on class parser |

## Gaps and prerequisites for 1.0

### Features
- [ ] Full skinparam coverage (~60% of keys applied)
- [ ] Sprite/icon rendering
- [ ] Improved layout engine (layout-rs infinite loop fixes, edge routing quality)
- [ ] Creole markup edge cases (tables, complex nesting)
- [ ] `!include` with URLs
- [ ] stdlib theme/icon library support
- [ ] Activity diagram legacy v1 syntax (full support)
- [ ] Archimate diagram support

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
- [ ] `serde_yaml` deprecation — migrate to maintained alternative
- [ ] `layout-rs` stability — infinite loop and panic issues

## Out of scope for 1.0

- Windows native binaries (CI builds Linux/macOS only for now)
- PlantUML server mode (HTTP API)
- Graphviz DOT input format
- PlantUML `.iuml` standard library bundling
- Interactive/real-time rendering
- Custom font embedding (beyond Liberation Sans)
