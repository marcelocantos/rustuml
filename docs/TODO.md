# TODO

## High Priority
- [ ] Scale golden files to 1,000+ (add component/usecase/deployment matrices, more sequence combinations)
- [ ] Built-in TIM functions (%strlen, %substr, %date, %newline, %dirpath, etc.)
- [ ] Timing diagram parser and renderer
- [ ] Gantt chart parser and renderer

## Medium Priority
- [ ] !while loops in preprocessor
- [ ] Mind map parser and renderer
- [ ] WBS parser and renderer
- [ ] Component/usecase/deployment matrix test coverage
- [ ] ASCII art output (-ttxt)
- [ ] EPS output
- [ ] Hyperlinks in SVG output

## Low Priority
- [ ] SCXML output for state diagrams
- [ ] Embedded sprites
- [ ] Responsive SVG with proper viewBox
- [ ] KaTeX math rendering (🎯T1.5)
- [ ] Format-parameterized test framework (validate PNG/PDF golden files)

## Done
- [x] Layout coordinate extraction (Sugiyama positions for class diagrams)
- [x] !function/!procedure in TIM preprocessor
- [x] !foreach loops in preprocessor
- [x] PDF output via svg2pdf
- [x] Apply skinparams to theme at render time
- [x] Wire !include base directory from CLI input file path
- [x] Multiline notes (note left\n...\nendnote)
- [x] Creole markup wired into SVG text rendering
- [x] Custom theme loading from YAML file (--theme-file=)
- [x] !theme directive in preprocessor
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
- [x] Golden file infrastructure (382 files)
- [x] --help-agent flag
- [x] Systematic test matrix framework
- [x] State quick matrix (20 cases)
- [x] Activity quick matrix (18 cases)
- [x] Class medium matrix (80 cases)
- [x] Sequence medium matrix (208 cases)
