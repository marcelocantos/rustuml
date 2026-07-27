# Sequence review-4 response

## Rejected boundary

Review commit `b85ba7e1` confirms that `dcdafc78` preserves all 27 scoped
earlier perturbations and fixes the required-zero autonumber, ordinary
half-head, and three external-depth matrices it targeted. Six fresh valid
counterexamples expose four remaining model boundaries.

## DecimalFormat field

Java passes the complete autonumber pattern to `DecimalFormat`. Its numeric
field begins at the first contiguous `#` or `0` placeholder and consumes the
whole mixed run. `0` positions define minimum zero padding; `#` positions are
optional digits. Literal prefix and suffix text, including parsed Creole style
runs, remain outside that field.

RustUML currently searches for `0` before `#` and consumes only the following
zero run. For `##0`, the two optional positions leak into output as literals.
The replacement operation must identify one contiguous `[#0]+` field, derive
minimum width from its zero count, and splice the formatted number across style
run boundaries without changing those runs' presentation.

## Decoration composition

Java `CommandArrow` determines semantic participant order, `ArrowPart`, circle
decoration, and cross head independently before `reverseDefine`. A slash or
backslash half-head can therefore coexist with a source cross. The parser
already retains `source_cross`, but RustUML's half-arrow writer accepts only a
source circle and silently drops the two cross strokes.

All ordinary and dotted half-head branches must compose the retained source
circle/cross with the selected half polygon or open stroke. Primitive ordering
follows semantic source side, exactly as for the filled-arrow writer; it is not
derived from participant labels or spatial index.

## Message-owned lifecycle timeline

Java inserts a message before applying the activation suffix parsed from that
command. `SequenceDiagram.addMessage` also makes it the owner for following
standalone lifecycle commands until another message replaces that owner.
`AbstractMessage.addLifeEvent` therefore retains ordered activation and
deactivation events on one message, including:

```plantuml
[--> Worker ++
deactivate Worker
```

`AbstractMessage.isActivateAndDeactive` exposes that combined ownership.
`Step1MessageExo` reserves a 30-pixel margin for it and moves the message end
ordinate down by the same amount. `DrawableSetInitializer.prepareLiveEvent`
then places activation at the arrow start and deactivation at that extended
end. Only after those distinct ordinates are established does `LifeLine`
receive its variations; its same-ordinate rejection rule is not the source of
this case.

RustUML's `Message` stores only one optional activation, while the standalone
deactivation remains an unowned event. Standalone lifecycle events consume no
vertical flow, so both changes are paired at the message's single ordinate,
creating a zero-height nested bar and placing every later message 30 pixels
early.

The correction is a normalized lifecycle timeline that retains ordered life
events and their owning message. It derives each message's start/end ordinates
and the Java-provenanced combined-event margin once. Row allocation,
activation bars, message endpoint depth, rendering state, and external extent
scans must consume that same timeline. A local "next event is deactivate"
condition is insufficient because Java ownership survives intervening
non-message events.

## External area ownership

Java builds the right border in two stages. `Step1MessageExo` first constrains
one shared virtual border for every FROM_RIGHT and TO_RIGHT message using the
arrow component's preferred width. `DrawableSetInitializer.prepareMissingSpace`
then expands the envelope for each arrow's actual starting position.
`MessageExoArrow` obtains that position from
`LivingParticipantBox.getLiveThicknessAt(messageY)`.

RustUML's width prepass includes live-depth displacement for TO_RIGHT but not
FROM_RIGHT, and it evolves that depth independently from later endpoint logic.
In the fresh staircase the deepest FROM_RIGHT candidate is at depth three,
while the final deactivating TO_RIGHT candidate is at depth two. The omitted
activation half-width is exactly the observed five-pixel deficit.

The correction must preserve Java's two stages: establish the preferred-width
constraint, then compute maximum actual overflow using the shared lifecycle
timeline for both right-border directions. Every right exo message renders
against the resulting common edge; no canvas constant is added.

## Acceptance

1. All 27 scoped earlier perturbations remain strict matches; the separately
   declared physical Serif gap remains excluded rather than hidden.
2. All ten `review5_` fixtures are strict matches, including `##0` formatting,
   source-cross half-heads, right staircases, and same-ordinate variation order.
3. Unit tests vary mixed `#`/`0` fields, left/right and dotted half-heads, and
   same-ordinate opposite/same variation sequences using renamed participants.
4. Every activation consumer uses the same message-owned lifecycle timeline;
   no consumer reconstructs a competing ownership or depth history.
5. The complete no-oracle ratchet has no pass-for-fail swaps.
