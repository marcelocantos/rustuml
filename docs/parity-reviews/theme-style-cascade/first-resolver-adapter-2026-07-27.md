# Theme Style Cascade: First Resolver Adapter

Static sidecar analysis. No Cargo or rustc command was run, and this note does
not change production code.

## Scope

At revision `e264cc2ebd0e762635b213e9a462840db05381f3`, 42 current
no-oracle failures contain a bundled theme: 19 state, 12 class, 4 component,
4 use-case, 1 theme-plus-user preprocessing case, and 2 edge-case class
diagrams. `edge_theme_plain` is outside this mechanism because `plain` is not
bundled for licensing reasons.

The fixtures use simple theme-first graphs: state chains; two-component
links; two use cases plus an actor; and small class inheritance pairs. That
is useful evidence for a resolver-first change, but not proof that all
post-style SVG differences disappear: fonts, padding, and borders change
node dimensions and therefore routed-edge geometry.

## Java Conversion And Resolution

| Input form | Java normalization/conversion | Sparse selector and properties | Required Rust adapter output |
| --- | --- | --- | --- |
| `root { ... }` | Parsed by `style.StyleParser.parse`; merged by `StyleStorage.computeMergedStyle` | `root`: `BackgroundColor`, `FontColor`, `FontName`, `FontSize`, `LineColor`, `LineThickness`, `Shadowing`, `RoundCorner`, `Padding` | document background; inherited font, line, thickness, shadow, corner, and padding defaults |
| `arrow { ... }`, or `skinparam [family]Arrow...` | `SkinParam.cleanForKeySlow` removes case, `_`, `.`, then collapses `activity|class|component|object|sequence|state|usecase + arrow` to `arrow`; `FromSkinparamToStyle.convertNow` maps it | `arrow`: `LineColor`, `LineThickness`, `LineStyle`, `HeadColor`, `FontColor`, `FontName`, `FontSize`, `FontStyle` | edge stroke/head/dash and label font for all four families |
| `class` and `class.header` | `FromSkinparamToStyle`: `classBackgroundColor`, `classBorderColor`, `classBorderThickness`, `classHeaderBackgroundColor`, `classFont*`, `classAttributeFont*` | `root.element.classDiagram.class[.header]` | class fill, border, width, corner, header/member font and header fill |
| `state` and `state.header` | `stateBackgroundColor`, `stateBorderColor`, `stateBorderThickness`, `stateFont*`, `stateAttributeFont*` map directly through `FromSkinparamToStyle` | `root.element.stateDiagram.state[.header]` | state fill/border/width/corner and state/transition-label font |
| `component` | `addMagic(SName.component)` maps `component{Background,Border,BorderThickness,RoundCorner,Shadowing,Font*}` | `root.element.componentDiagram.component` | component fill/border/width/corner/shadow and label font |
| `usecase` | `addMagic(SName.usecase)` maps the same property family | `root.element.usecaseDiagram.usecase` | use-case fill/border/width/shadow and label font |
| `...<<stereotype>>` | `FromSkinparamToStyle.addStyle` adds each stereotype to the signature and calls `StyleLoader.addPriorityForStereotype` | family selector plus stereotype identity; properties above, plus `SName.stereotype` for `*StereotypeFont*` | stereotype-qualified field override after ordinary matching selector resolution |

`FromSkinparamToStyle.addConFont` maps `FontSize`, `FontStyle`, `FontColor`,
and `FontName` consistently. `convertNow` also normalizes `true/false`
shadowing to `3/0`, and dotted/dashed line styles to `1;3`/`7;7`. Preserve
those as values in the adapter; do not manufacture dense palettes.

Exact Java provenance:

- `net/sourceforge/plantuml/skin/SkinParam.java`: `setParam` (lines 210-218),
  `cleanForKeySlow` (265-281).
- `net/sourceforge/plantuml/style/FromSkinparamToStyle.java`:
  conversion registrations (static initializer and `addMagic`), `convertNow`
  (301-355), stereotype priority in `addStyle` (392-404), and `addConFont`
  (420-425).
- `net/sourceforge/plantuml/style/StyleParser.java`: `parse`; and
  `StyleStorage.java`: `computeMergedStyle`; selector matching is
  `StyleSignatureBasic.matchAll`.
- Consumers: `svek/GraphvizImageBuilder.java`
  `getDefaultStyleDefinitionArrow`, `svek/SvekEdge.java`
  `getDefaultStyleDefinition`, `svek/image/EntityImageClass.java` `getStyle`
  and `getStyleHeader`, and `svek/image/EntityImageStateCommon.java`
  `getStyleState` and `getStyleStateHeader`.

## Bundled Source Crosswalk

The failing themes are drawn from the bundled sources for `aws-orange`,
`black-knight`, `cerulean`, `cerulean-outline`, `cloudscape-design`,
`hacker`, `materia`, `metal`, `minty`, all eight `reddress-*` variants,
`silver`, `sketchy`, `sketchy-outline`, `spacelab`, `superhero`, and
`united`.

Their shared theme layout establishes a `root` selector with background,
font color, line color/thickness, margin, padding, and shadowing (for example
`crates/rustuml-parser/themes/puml-theme-metal.puml:97-106`). It also supplies
diagram-local `arrow`, `element`, and family selectors in the expanded
`!startsub` blocks. The current preprocessing code discards those blocks in
`preprocess/themes.rs:140-175`, retaining only three synthetic root keys, then
relocates the remaining flattened skinparams in `preprocess/mod.rs:1810-1885`.

That loss is visible in the current renderer split:

| Rust site | Present consumption | Resolver replacement |
| --- | --- | --- |
| `rustuml-render/src/skinparam.rs:29-370` | Folds every declaration into a dense `Theme` before rendering. | Adapt one resolved sparse view to the existing `Theme` fields only at the renderer boundary. |
| `rustuml-render/src/class.rs:2656-2790` | First-match/raw-key `ClassFontOverrides`, including synthetic `__styleRoot*`. | Resolve class and class-header signatures; use raw metadata only for non-style behaviour. |
| `rustuml-render/src/state.rs:1909-2140` | Reverse scans `DiagramMeta.skinparams` in `StateSkin` helpers. | Resolve state, state-header, and arrow signatures once. |
| `rustuml-render/src/component.rs` | Source-order local mutation of component color values. | Resolve component and arrow signatures once. |
| `rustuml-render/src/usecase.rs` | Builds family colors from raw metadata. | Resolve usecase and arrow signatures once. |

## Smallest First Adapter

Do not introduce a dense `Theme` replacement or a per-theme palette. Add an
ordered sparse program beside `DiagramMeta.skinparams` and a limited adapter:

```rust
pub struct StyleProgram {
    declarations: Vec<StyleDeclaration>,
}

pub struct StyleDeclaration {
    epoch: u32,
    selector: StyleSelector,
    property: StyleProperty,
    value: StyleValue,
    priority: StylePriority,
}

pub struct FamilyStyleView {
    pub background: Option<ColorValue>,
    pub border: Option<ColorValue>,
    pub line_thickness: Option<f64>,
    pub round_corner: Option<f64>,
    pub shadowing: Option<f64>,
    pub padding: Option<InsetsValue>,
    pub font: FontStyleView,
    pub arrow: ArrowStyleView,
}

impl StyleProgram {
    pub fn resolve_family(
        &self,
        diagram: DiagramStyleKind,
        family: FamilyStyleKind,
        stereotype: Option<&str>,
        epoch: StyleEpoch,
    ) -> FamilyStyleView;
}
```

First implementation scope:

1. Parse nested `<style>` blocks into ordered declarations instead of
   `__styleRoot*` records.
2. Translate ordinary and grouped `skinparam` through a table equivalent to
   `cleanForKeySlow` plus the conversion rows above. Theme-body declarations
   are ordinary declarations, not `theme:<segment>:<raw-key>` aliases.
3. Match only `root`, `element`, the four diagram/family signatures, `arrow`,
   and stereotype qualifiers; return `Option` fields so inheritance remains
   observable.
4. Use that view to fill existing class/state/component/use-case fields.
   Apply inline entity/link colors after the view is resolved.

This is the smallest coherent unit likely to unlock most of the 42 fixtures:
the simple state chains and component/use-case links need root inheritance plus
family/arrow colors, while class cases additionally need header/member fonts,
padding, and border thickness. It deliberately excludes temporal entity/link
snapshots for the first patch because all current failing theme fixtures place
the theme before their objects. The program must nevertheless retain epochs;
the next patch must use Java's creation-time builder rules before accepting
mid-diagram-theme behaviour.

## Expected Residuals After Correct Styles

- **Likely style-only or style-dominant:** the state chains and two-node
  component/use-case fixtures. Their topology is small, so correct font,
  fill, border, and arrow resolution should account for most divergence.
- **Likely style plus geometry:** class inheritance fixtures. Correct fonts,
  padding, corner radius, and border thickness change node dimensions; the
  remaining deltas are expected in SVEK-equivalent sizing/routing rather than
  additional color keys.
- **Independent work:** `preproc_complex_theme_override` tests source order;
  `edge_theme_plain` requires a licensing/source decision; the two themed
  edge-case class diagrams retain package/note/relationship layout exposure.

Validation for an implementation must add held-out source-position,
cross-family-arrow, nested-selector, and stereotype inputs. A pass count from
the existing theme fixtures alone would not validate the model.

