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

## Canonical life variations

Java `LifeLine.addSegmentVariation` owns one ordered variation program per
participant. A variation earlier than the last accepted ordinate is ignored.
At the same ordinate, an opposite variation is ignored, while a repeated
variation of the same type is accepted. `finish` appends enough closing
variations at the diagram maximum for levels left open by those rules.

RustUML currently replays activation state separately for bar pairing,
message endpoints, external-width scans, and rendering. Its stack pairing
closes an inline activation with an opposite standalone deactivation at the
same message ordinate, creating a zero-height nested bar that Java never
creates. Later scans then observe a shallower timeline and can disagree with
the bars.

The correction is one renderer-neutral accepted-variation replay, indexed by
participant and event/ordinate. Activation bars, live depth at a message,
endpoint shifts, return state, and extent scans must consume that same replay.
The replay preserves source event order and first-creation ownership; it does
not insert a fixture-specific row or branch on event count.

## External area ownership

For every exo direction, `MessageExoArrow` asks
`LivingParticipantBox.getLiveThicknessAt(messageY)` for the participant segment
at the exact arrow ordinate. On the right, its area end is
`max(sharedMaxX, livePos2 + preferredWidth)`; every FROM_RIGHT and TO_RIGHT
message then draws against that shared area edge.

RustUML's right-envelope scan uses participant centre plus label extent for
FROM_RIGHT and evolves activation depth independently. It therefore omits the
accepted live-segment right shift in the fresh staircase and fixes the shared
border five pixels too far left. Right-border candidates must be derived from
the canonical life replay's `pos2` and the arrow component's preferred width,
then shared by both right exo directions.

## Acceptance

1. All 27 scoped earlier perturbations remain strict matches; the separately
   declared physical Serif gap remains excluded rather than hidden.
2. All ten `review5_` fixtures are strict matches, including `##0` formatting,
   source-cross half-heads, right staircases, and same-ordinate variation order.
3. Unit tests vary mixed `#`/`0` fields, left/right and dotted half-heads, and
   same-ordinate opposite/same variation sequences using renamed participants.
4. Every activation consumer uses the same accepted variation replay; no
   consumer reconstructs a competing depth timeline.
5. The complete no-oracle ratchet has no pass-for-fail swaps.
