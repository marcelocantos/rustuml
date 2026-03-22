# TODO

## High Priority
- [ ] Font metrics via ab_glyph — embed Liberation Sans for accurate text sizing (currently using char-width approximation)
- [ ] Layout coordinate extraction — parse layout-rs SVG to get Sugiyama positions for class diagrams (currently grid-based)
- [ ] Wire creole markup into SVG text rendering (module exists but not integrated into renderers)

## Medium Priority
- [ ] `!function` / `!procedure` in TIM preprocessor
- [ ] `!theme` directive in preprocessor
- [ ] Component and use case diagram SVG renderers
- [ ] Deployment diagram SVG renderer
- [ ] `!foreach` / `!while` loops in preprocessor
- [ ] Built-in TIM functions (%strlen, %substr, %date, etc.)

## Low Priority
- [ ] Timing diagram parser and renderer
- [ ] Gantt chart parser and renderer
- [ ] Mind map parser and renderer
- [ ] WBS parser and renderer
- [ ] ASCII art output (`-ttxt`)
- [ ] PDF output
- [ ] EPS output
- [ ] SCXML output for state diagrams
- [ ] Embedded sprites
- [ ] Hyperlinks in SVG output
- [ ] Responsive SVG with proper viewBox

## Testing
- [ ] Multi-format oracle tests (PNG comparison)
- [ ] Preprocessor oracle tests against Java PlantUML -preproc
- [ ] Matrix tests for component and use case diagrams
- [ ] Error path testing (malformed input, missing participants)
- [ ] Creole markup rendering tests against Java PlantUML

## Done
- [x] Apply skinparams to theme at render time
- [x] Wire `!include` base directory from CLI input file path
- [x] Multiline notes (note left\n...\nendnote)
- [x] Creole markup module (bold, italic, underline, strikethrough)
- [x] Custom theme loading from YAML file (--theme-file=)
- [x] Deployment diagram parser
- [x] Component and use case diagram parsers
- [x] Note on link
- [x] PNG output via resvg
