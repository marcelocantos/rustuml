# Theme/style cascade mechanism analysis

Static analysis only. No Rust build or test command was run.

## Revisions and scope

- RustUML revision inspected: `e264cc2ebd0e762635b213e9a462840db05381f3`
- PlantUML revision inspected: `71806a23780b04a5ccde2f8ceb5121edad5eb711`
- Current no-oracle theme-related failures: 42
  - 19 state
  - 12 class
  - 4 component
  - 4 use case
  - 1 preprocessing theme-plus-user override
  - 2 edge-case class diagrams

The concentration in state/class/component/use-case is significant: those are
SVEK-style graph families in the Java model. Sequence and activity theme
fixtures currently pass, so this account does not claim that every remaining
SVG difference is caused solely by style resolution. A correct cascade is a
prerequisite; graph construction and layout may remain as independent causes.

`edge_theme_plain` is also a separate availability case. RustUML deliberately
does not bundle the unlicensed/GPL-inheriting `plain` theme. It cannot be fixed
by cascade semantics alone without an external-theme source or a licensing
decision.

## Java model

### 1. Themes execute at their source position

`TContext.executeOneLineNotSafe` dispatches a `THEME` line immediately to
`TContext.executeTheme`. `executeTheme` reads the selected `Theme` and calls
`executeLines` on its body before the next user line is processed.
`TContext.buildCodeIterator` includes `CodeIteratorSub`, which replays
`!startsub` bodies in order, so family blocks in bundled themes contribute at
the theme directive rather than forming a detached final palette.

The resulting order is therefore:

1. built-in `plantuml.skin`;
2. user declarations before `!theme`;
3. the theme body, in theme-file order;
4. user declarations after `!theme`;
5. later themes and later declarations in their source positions.

A second theme is not a wholesale reset. It overwrites properties it declares;
properties it does not declare remain in the builder.

### 2. One ordered builder receives modern styles and legacy skinparams

`SkinParam.getCurrentStyleBuilderInternal` clones the cached builder loaded by
`StyleLoader.loadSkin`. Both modern and legacy declarations then mutate that
same logical cascade:

- `CommandStyleMultilinesCSS.executeNow` parses `<style>` with
  `StyleParser.parse` and calls `SkinParam.muteStyle`.
- `CommandSkinParam.executeArg` and `CommandSkinParamMultilines.execute` call
  `TitledDiagram.setParam`.
- `SkinParam.setParam` first applies `cleanForKey`, stores the canonical
  parameter, converts it through `FromSkinparamToStyle.convertNow`, and calls
  `muteStyle`.

`StyleBuilder.muteStyle` returns a copy-on-write builder. `StyleParser` and
`FromSkinparamToStyle` allocate monotonically increasing property priorities
from that builder. `Style.mergeWith`/`ValueImpl.mergeWith` use those priorities,
so later declarations of the same property win independently of whether the
new declaration came from CSS-style syntax or legacy `skinparam`.

Stereotype-qualified declarations receive
`StyleLoader.DELTA_PRIORITY_FOR_STEREOTYPE` through
`StyleLoader.addPriorityForStereotype`. This is an explicit priority tier, not
ordinary CSS specificity.

### 3. The cascade stores sparse selector/property declarations

`StyleParser.Context` turns nested selectors into `StyleSignatureBasic`
instances. `StyleSignatureBasic.matchAll` matches a declaration when the
element signature contains all declaration `SName`s, stereotype identities,
and any depth constraint. `StyleStorage.computeMergedStyle` merges every
matching sparse style.

Consequences:

- A `root` property is inherited by every matching element unless a later
  matching declaration overrides that property.
- A family selector such as
  `root.element.stateDiagram.state.header` does not create a complete state
  palette. It contributes only its declared properties.
- Missing values remain missing until selector inheritance or built-in styles
  supply them.
- Stereotype identities are lowercase and underscore/dot-insensitive through
  `StyleSignatureBasic.clean`.

Representative consumer signatures are:

- `EntityImageClass.getStyle`:
  `root.element.classDiagram.class`
- `EntityImageClass.getStyleHeader`:
  `root.element.classDiagram.class.header`
- `EntityImageStateCommon.getStyleState`:
  `root.element.stateDiagram.state`
- `EntityImageStateCommon.getStyleStateHeader`:
  `root.element.stateDiagram.state.header`
- `EntityImageDescription`:
  `root.element.<diagram-style>.<symbol>[.title]`
- `GraphvizImageBuilder.getDefaultStyleDefinitionArrow` and
  `SvekEdge.getDefaultStyleDefinition`:
  `root.element.<diagram-style>.arrow`

This selector composition is why a single root or arrow declaration can change
component, class, state, and use-case output without a target-family key.

### 4. Legacy key identity and selector identity are distinct

`SkinParam.cleanForKeySlow` canonicalizes ordinary parameter-map identity:
case, underscores, and dots are removed; several compatibility aliases are
rewritten; family arrow prefixes collapse to `arrow`. The canonical key is
then converted by `FromSkinparamToStyle` into one or more selector/property
declarations.

This means `stateArrowColor`, `UseCase.Arrow_Color`, and `ArrowColor` are not
three independent family slots in an executed theme. They are ordered writes
to the shared arrow style property. The component control in the rejected
`state-arrow-style-isolation` review demonstrates the result: a theme with no
component-specific arrow declaration can still give component links `#000000`
through the shared arrow cascade.

Stereotype-qualified keys use the same cleaned identity for storage and
lookup. The displayed stereotype remains unchanged.

### 5. Temporal snapshots are part of style semantics

The builder is versioned, not merely final:

- `CucaDiagram.createLeaf` and `createGroup` pass the current builder into
  `Entity`.
- `Entity.getCurrentStyleBuilder` normally returns that creation-time builder.
  If any legacy `skinparam` command marked the diagram with
  `TitledDiagram.setSkinParamUsed(true)`, it intentionally returns the latest
  diagram builder for backward compatibility.
- Link commands pass the current builder into `Link`; `Link.getStyleBuilder`
  always returns that captured builder. `SvekEdge` resolves against it.
- Inline colors/styles are applied after merged style resolution through
  `Style.eventuallyOverride`, `Link.getColors`, or the corresponding
  family-specific override.

Thus source position cannot be reduced to a final map. Pure `<style>` changes
can affect only subsequently created objects, while legacy skinparams make
entities consult the latest builder; links retain their creation snapshot.

## Rust ownership divergence

### Preprocessing loses selector structure

`preprocess::PreprocessContext::try_theme` appends theme output to
`theme_tail`. `preprocess::themes::flatten_theme_output`:

- drops almost the entire `<style>` block;
- preserves only three root properties as synthetic skinparams
  (`__styleRootLineThickness`, `__styleRootLineColor`,
  `__styleRootFontColor`);
- flattens grouped skinparams into raw family-prefixed keys.

The comment that dropping `<style>` is harmless is no longer true for parity.
Root background, margin, padding, shadowing, family selectors, nested
selectors, dark variants, and selector/property sparsity are discarded.

### Parsing stores a flat compatibility list

`parse::collapse_reassigned_skinparams` reconstructs theme placement, but its
output remains `DiagramMeta.skinparams: Vec<SkinParam>`.

Ordinary declarations use a partial `cleanForKey` model. Synthetic theme-body
records are deliberately keyed as `theme:<segment>:<raw-key>`, preserving
family spellings as independent compatibility scopes. That contradicts Java's
execution model: theme body skinparams are ordinary commands and pass through
the same `SkinParam.setParam` and `FromSkinparamToStyle` path as user
declarations.

The previous attempt to canonicalize theme records caused regressions because
the same flat key is also being used as renderer API. That is evidence that
declaration identity and resolved family style must be separated, not evidence
that Java gives theme records different key semantics.

### Rendering has four competing style owners

`render_svg_with_theme` first folds every metadata skinparam into one dense
`style::Theme` via `skinparam::apply_skinparams`, before rendering any element.
This erases temporal snapshots and missing-property inheritance.

Consumers then disagree about which representation is authoritative:

- class rendering uses `theme.class` for some values and independently scans
  `DiagramMeta.skinparams` through `ClassFontOverrides`;
- state explicitly ignores the passed `Theme` and reverse-scans metadata in
  `StateSkin::from_diagram`;
- component scans metadata in source order and mutates local variables;
- use case ignores the passed `Theme` and builds `SkinColors` from metadata.

Some scans use the first matching key, some use the last, and
`apply_skinparams` folds forward. Multiple themes, aliases, and inherited root
properties therefore have no single ordering rule.

Finally, dense fields such as `Theme.component.arrow_color` always contain a
value. They cannot distinguish “component arrow explicitly declared” from
“absent, inherit the matching root/arrow declaration”, which is the precise
boundary exposed by the rejected component theme heldout.

## Proposed durable mechanism account

### Invariant

Theme CSS, theme skinparams, user CSS, and user skinparams form one ordered
stream of sparse style declarations. Every declaration has a canonical
selector, property, value, source epoch, and Java-compatible priority.
Resolution for an element merges declarations whose selector matches the
element signature; the highest property priority wins, stereotype-qualified
properties retain their explicit priority tier, and inline element/link
colors are final overrides. Theme bodies execute at the directive position.
Creation-time style epochs and PlantUML's `skinParamUsed` entity compatibility
rule are preserved.

### Causal divergence

RustUML currently converts a theme into flattened key/value compatibility
records, discards selector structure, isolates theme aliases that Java
canonicalizes, and materializes one final dense palette. Family renderers then
partly bypass that palette with inconsistent raw-key scans. This conflates
declaration identity, selector inheritance, temporal ordering, and renderer
defaults. It explains why isolated theme values appear correct while
cross-family fallback, repeated themes, user overrides, and SVEK family
geometry still diverge.

### Likely minimal structural change

1. Add a renderer-neutral style IR to parser metadata:
   `StyleProgram { declarations: Vec<StyleDeclaration> }`, where a declaration
   carries an epoch, selector/signature tokens, property, value, origin
   (built-in/theme/user; CSS/skinparam), dark value if present, and stereotype
   priority.
2. Preserve and parse `<style>` blocks instead of reducing them to three
   synthetic keys. Convert grouped and ordinary skinparams through one
   table-driven equivalent of `cleanForKeySlow` plus
   `FromSkinparamToStyle`; do not expose raw family keys as the semantic style
   model.
3. Add a `StyleCascade::resolve(signature, epoch)` resolver in
   `rustuml-render`. Initially adapt its result into the existing `Theme`
   family structs to limit renderer churn, but make the adapter query exact
   Java signatures and preserve absence/inheritance before applying built-in
   defaults.
4. Replace direct metadata scans in class/state/component/use-case with that
   resolver or the adapter. Keep raw skinparams only for non-style behavioral
   options.
5. Record a style epoch on style-bearing AST entities and links. Use the
   diagram-final epoch for entities after any legacy skinparam has set the
   compatibility flag; otherwise use their creation epoch. Always use the
   link creation epoch. Apply inline colors after resolution.

For the current 42 fixtures, themes occur before diagram objects, so steps
1-4 should expose most of the gain. Step 5 is still part of the mechanism gate:
without it, a top-of-file-only implementation would overfit the existing
corpus and fail source-position perturbations.

Do not solve this by adding more `__styleRoot*` keys, family-specific reverse
scans, hard-coded theme names, or complete per-theme palettes. Those reproduce
symptoms while retaining the wrong ownership model.

## Adversarial perturbation categories

At least the following independent categories should be generated with fresh
labels and changed topology:

1. **Temporal theme placement:** declarations and entities before/between/after
   two themes; links created on both sides of each theme.
2. **CSS versus skinparam order:** the same property written by `<style>` and
   legacy skinparam in both orders, with an unrelated property that must
   survive.
3. **Sparse repeated themes:** theme A sets root/font/arrow, theme B sets only
   state background, proving B does not reset A's undeclared properties.
4. **Selector inheritance:** root, element, diagram-family, entity, header,
   note, and arrow selectors each omit different properties so fallback is
   observable.
5. **Cross-family canonical aliases:** mixed-case and separator variants of
   class/state/component/use-case/sequence arrow keys, including a target
   family with no explicit arrow declaration.
6. **Stereotype identity and priority:** underscore/dot aliases, unrelated
   stereotypes, multiple stereotype labels, and source-order reversal against
   unqualified declarations.
7. **Creation snapshots:** pure style blocks around entities and links, then
   the same matrix with one legacy skinparam to trigger Java's entity
   compatibility rule.
8. **Inline final overrides:** local gradient/color/dashed link overrides and
   per-entity colors after inherited theme values.
9. **Geometry feedback:** font family/size, padding, margin, border thickness,
   and shadowing changes on renamed labels of short/long lengths and on
   different branch counts.
10. **Availability negative:** unknown and intentionally unbundled themes must
    not inherit values from the previous theme accidentally.

The checker should inspect both style attributes and geometry. A color-only
match is insufficient when font and padding resolution feed SVEK node sizes
and therefore edge routes and canvas bounds.

## Exact source locations

Java:

- `net.sourceforge.plantuml.tim.TContext#executeOneLineNotSafe`
- `net.sourceforge.plantuml.tim.TContext#executeTheme`
- `net.sourceforge.plantuml.tim.TContext#buildCodeIterator`
- `net.sourceforge.plantuml.tim.iterator.CodeIteratorSub#peek`
- `net.sourceforge.plantuml.style.CommandStyleMultilinesCSS#executeNow`
- `net.sourceforge.plantuml.command.CommandSkinParam#executeArg`
- `net.sourceforge.plantuml.command.CommandSkinParamMultilines#execute`
- `net.sourceforge.plantuml.skin.SkinParam#getCurrentStyleBuilderInternal`
- `net.sourceforge.plantuml.skin.SkinParam#setParam`
- `net.sourceforge.plantuml.skin.SkinParam#cleanForKeySlow`
- `net.sourceforge.plantuml.style.FromSkinparamToStyle#convertNow`
- `net.sourceforge.plantuml.style.StyleBuilder#muteStyle`
- `net.sourceforge.plantuml.style.StyleStorage#computeMergedStyle`
- `net.sourceforge.plantuml.style.StyleSignatureBasic#matchAll`
- `net.sourceforge.plantuml.style.parser.StyleParser#parse`
- `net.sourceforge.plantuml.style.parser.Context#toStyles`
- `net.atmp.CucaDiagram#createLeaf`
- `net.atmp.CucaDiagram#createGroup`
- `net.sourceforge.plantuml.abel.Entity#getCurrentStyleBuilder`
- `net.sourceforge.plantuml.abel.Link#getStyleBuilder`
- `net.sourceforge.plantuml.svek.GraphvizImageBuilder#getDefaultStyleDefinitionArrow`
- `net.sourceforge.plantuml.svek.SvekEdge#getDefaultStyleDefinition`
- `net.sourceforge.plantuml.svek.SvekEdge#drawU`
- `net.sourceforge.plantuml.svek.image.EntityImageClass#getStyle`
- `net.sourceforge.plantuml.svek.image.EntityImageClass#getStyleHeader`
- `net.sourceforge.plantuml.svek.image.EntityImageStateCommon#getStyleState`
- `net.sourceforge.plantuml.svek.image.EntityImageStateCommon#getStyleStateHeader`
- `net.sourceforge.plantuml.svek.image.EntityImageDescription#EntityImageDescription`

Rust:

- `crates/rustuml-parser/src/preprocess/mod.rs::PreprocessContext::try_theme`
- `crates/rustuml-parser/src/preprocess/themes.rs::flatten_theme_output`
- `crates/rustuml-parser/src/parse/mod.rs::canonical_skinparam_key`
- `crates/rustuml-parser/src/parse/mod.rs::collapse_reassigned_skinparams`
- `crates/rustuml-parser/src/diagram/mod.rs::DiagramMeta`
- `crates/rustuml-render/src/lib.rs::render_svg_with_theme`
- `crates/rustuml-render/src/skinparam.rs::apply_skinparams`
- `crates/rustuml-render/src/style.rs::Theme`
- `crates/rustuml-render/src/class.rs::ClassFontOverrides::from_skinparams`
- `crates/rustuml-render/src/state.rs::StateSkin::from_diagram`
- `crates/rustuml-render/src/component.rs::render_with_oracle`
- `crates/rustuml-render/src/usecase.rs::SkinColors::from_meta`
