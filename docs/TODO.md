# TODO

## High Priority
- [ ] Font metrics via ab_glyph — embed Liberation Sans for accurate text sizing
- [ ] Layout coordinate extraction — parse layout-rs SVG to get Sugiyama positions for class diagrams
- [ ] Apply skinparams to theme at render time (currently parsed but not used)
- [ ] Wire `!include` base directory from CLI input file path

## Medium Priority
- [ ] `!function` / `!procedure` in TIM preprocessor
- [ ] Component and use case diagram renderers
- [ ] Deployment diagram parser and renderer
- [ ] Multiline notes (note left\n...\nendnote)
- [ ] `creole` markup in text (bold, italic, underline)
- [ ] Custom theme loading from YAML file (`--theme=path/to/theme.yaml`)
- [ ] `!theme` directive in preprocessor

## Low Priority
- [ ] Timing diagram parser and renderer
- [ ] Gantt chart parser and renderer
- [ ] Mind map parser and renderer
- [ ] WBS parser and renderer
- [ ] ASCII art output (`-ttxt`)
- [ ] PDF output
- [ ] EPS output
- [ ] SCXML output for state diagrams
- [ ] `!foreach` loops in preprocessor
- [ ] `!while` loops in preprocessor
- [ ] Built-in TIM functions (%strlen, %substr, %date, etc.)
- [ ] Embedded sprites
- [ ] Hyperlinks in SVG output
- [ ] Responsive SVG with proper viewBox

## Testing
- [ ] Multi-format oracle tests (PNG comparison)
- [ ] Preprocessor oracle tests against Java PlantUML -preproc
- [ ] Matrix tests for component and use case diagrams
- [ ] Error path testing (malformed input, missing participants)
