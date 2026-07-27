# Theme/style cascade review-2 response

## Rejected boundary

Review commit `4621ef5a` rejects implementation commit `99fe75cf`. The earlier
ten heldouts improve to nine strict matches and ten scoped style matches, but
only one of fourteen fresh valid perturbations is strict. Eleven failures are
direct style semantics, one is a style-derived component envelope, and one is
an independently declared state direction-layout gap.

The sparse winner-selection engine is not the rejected mechanism. The mixed
theme/style/skinparam ordering control selects every child property correctly.
The rejected boundary is the renderer adapter: it projects Java's
consumer-specific style-builder ownership and open value types into a few
diagram-wide scalar structs.

## Builder ownership

Java does not have one temporal rule for every style consumer.

- `Entity#getCurrentStyleBuilder` returns the builder captured when the entity
  was created. If any legacy skinparam command was used, it returns
  `SkinParam#getCurrentStyleBuilder` instead for compatibility.
- `EntityImageDescription` resolves its symbol, title, stereotype, colors,
  stroke, and shadow through `Entity#getCurrentStyleBuilder`. Component and
  use-case DESCRIPTION leaves therefore need per-entity creation snapshots,
  stereotype-qualified signatures, and legacy refresh semantics.
- `Link#getStyleBuilder` always returns the builder captured by the link.
  `SvekEdge#getCurrentStyleBuilder`, line stroke, arrow colors, and label font
  consume that builder. Legacy skinparams do not refresh older links.
- `EntityImageState2#getStyle` resolves through
  `SkinParam#getCurrentStyleBuilder`. Flat state entities therefore use the
  final builder, while still adding each concrete entity's stereotype to the
  signature.
- `EntityImageClass#getStyle` and `getStyleHeader` use the entity builder for
  class body paint and compartments. `EntityImageClassHeader` separately
  resolves class-name fonts through `SkinParam#getCurrentStyleBuilder`.
  RustUML must not snapshot those header font channels with the body.
- `TextBlockExporter12026.Builder#calculateMargin` and document chrome resolve
  `root.document` through the final skin builder.

The correction is an explicit consumer policy, not a general
`resolve_entity_at_source_line` call at every entity draw site. Each renderer
must request the builder ownership of the Java consumer it models.

## Signatures

Java's `EntityImageDescription` calls `withTOBECHANGED(stereotype)` on the
concrete symbol signature before merging. Component and use-case entities must
therefore resolve independently with all of their own stereotypes. Flat state
entities likewise need a stereotype-qualified final signature.

`StyleSignatureBasic#clean` normalizes source stereotype spellings, including
dot and underscore forms. RustUML's existing `StyleSignature::with_stereotype`
already provides that normalization; the missing behavior is passing each
entity's stereotypes into the signature at the consuming draw site.

## Generative value types

Java style values retain structure until a consumer interprets them.

- `ClockwiseTopRightBottomLeft#read` accepts one value as all sides, two as
  vertical/horizontal, three as top/horizontal/bottom, and four as
  top/right/bottom/left. Component, state, use-case, and document adapters must
  use one shared four-side value type rather than taking the first scalar.
- `Style#getStroke` tokenizes `LineStyle` on `-`, `;`, or `,`. One number means
  equal visible and space lengths; two numbers preserve an arbitrary dash
  pair. `LineThickness` remains independent. A solid/dotted enum cannot model
  this grammar.
- `EntityImageDescription` obtains `Style#getShadowing`, carries it in
  `Fashion.withShadow`, and symbol painters such as
  `USymbolComponent2#drawComponent2` apply it with
  `URectangle#setDeltaShadow`. RustUML must paint the shadow/filter and derive
  the envelope from the same value. The unproven
  `COMPONENT_DEFAULT_SHADOW_FRONTIER` estimate and boolean fallback are not a
  model.

Padding is still consumer-specific. The fresh component and use-case controls
show valid padding declarations that their Java symbol painters do not consume.
Parsing a four-side value does not authorize applying it to every symbol.

## Renderer projections

The implementation must make these channels concrete:

1. Component entities: per-component creation/legacy-refresh style,
   stereotype signature, fill, line, font, corner, stroke, and painted shadow.
2. Component links: per-connection creation style for line/head and label
   channels, including arbitrary dash pairs and thickness.
3. Use-case entities: per-use-case creation/legacy-refresh style and
   stereotype signature. Actors retain their own actor signature.
4. Use-case links: per-connection creation style with line thickness/style and
   existing font/color channels.
5. Flat states: final-builder entity style with concrete stereotypes, final
   state-header style, final document background/margin, and per-link captured
   arrow font/line channels.
6. Classes: entity-snapshot body and compartment channels, but final-builder
   class-name header font channels.

The component envelope must then be derived from what these consumers paint.
The mixed-order review fixture already proves winner ordering; a canvas-only
constant cannot repair the remaining width difference.

## Acceptance

1. All ten original `review1_` fixtures retain scoped style equivalence; the
   separately named actor-alias identity gap remains separate.
2. All fourteen `review2_` fixtures pass scoped style comparison. Strict
   comparison may exclude only the already declared state direction-layout
   case.
3. Fresh heldouts vary entity/link declaration order, legacy refresh,
   stereotypes, one/two/three/four-side values, arbitrary dash pairs, and
   numeric shadows with renamed labels and different graph topology.
4. Unit tests identify each Java consumer policy rather than asserting a
   universal temporal rule.
5. No shipping code reads fixture data, branches on labels/source lines, or
   adjusts coordinates to satisfy a heldout.
6. `cargo test --test golden_no_oracle --release`, renderer tests, clippy, and
   honesty guards pass with no pass-for-fail swaps.
7. A fresh independent reviewer accepts an immutable implementation commit.
