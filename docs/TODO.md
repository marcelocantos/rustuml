# TODO

## High Priority
- [ ] Scale golden files from 145 to 1,000+ (expand matrix feature sets, regenerate)
- [ ] Layout coordinate extraction — parse layout-rs SVG to get Sugiyama positions for class diagrams (currently grid-based)
- [ ] `!function` / `!procedure` in TIM preprocessor

## Medium Priority
- [ ] `!foreach` / `!while` loops in preprocessor
- [ ] Built-in TIM functions (%strlen, %substr, %date, etc.)
- [ ] PDF output
- [ ] Format-parameterized test framework (run golden tests for PNG too)
- [ ] Timing diagram parser and renderer
- [ ] Gantt chart parser and renderer

## Low Priority
- [ ] Mind map parser and renderer
- [ ] WBS parser and renderer
- [ ] ASCII art output (`-ttxt`)
- [ ] EPS output
- [ ] SCXML output for state diagrams
- [ ] Embedded sprites
- [ ] Hyperlinks in SVG output
- [ ] Responsive SVG with proper viewBox
- [ ] KaTeX math rendering (🎯T1.5)

## Done
- [x] Apply skinparams to theme at render time
- [x] Wire `!include` base directory from CLI input file path
- [x] Multiline notes (note left\n...\nendnote)
- [x] Creole markup (bold, italic, underline, strikethrough) wired into SVG
- [x] Custom theme loading from YAML file (--theme-file=)
- [x] `!theme` directive in preprocessor
- [x] Deployment diagram parser and renderer
- [x] Component and use case diagram parsers and renderers
- [x] Note on link
- [x] PNG output via resvg
- [x] Font metrics via ab_glyph with embedded Liberation Sans
- [x] YAML/JSON input with serde on all model types
- [x] Score-based diagram type detection
- [x] Ref-over rendering in sequence diagrams
- [x] Error path tests (19 tests)
- [x] Preprocessor oracle tests (5 tests)
- [x] Golden file infrastructure (145 files, generator script)
- [x] --help-agent flag
- [x] Systematic test matrix framework
