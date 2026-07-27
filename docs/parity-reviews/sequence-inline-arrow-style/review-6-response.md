# Sequence review-6 response

## Rejected boundary

Review record `review-6.json`, committed as `ad330988`, rejects implementation
commit `6ff6d9a8f069e9c6130580214905b9c7e8813106` against PlantUML revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711`.

The implementation preserves all 37 designated historical perturbations and
all 16 `review6_` perturbations as strict matches, but only 8 of 17 fresh valid
`review7_` perturbations pass. The fresh failures disprove two claims in the
previous account:

1. Rust does not yet have Java's accepted `LifeLine` variation timeline.
2. Rust does not yet have one Java-like horizontal-element and missing-space
   solve shared by notes, groups, external arrows, dividers, and paint.

The rejected behavior must not be repaired with observed 5px, 6px, 7px, 10px,
94px, or 142px corrections. Those deltas are effects of the model gaps below.

## Pinned Java model

The relevant Java source is PlantUML revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711`.

Lifecycle variation acceptance and geometry are defined by:

- `net.sourceforge.plantuml.sequencediagram.graphic.LifeLine#addSegmentVariation`
- `net.sourceforge.plantuml.sequencediagram.graphic.LifeLine#finish`
- `net.sourceforge.plantuml.sequencediagram.graphic.LivingParticipantBox#getLiveThicknessAt`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareLiveEvent`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareMessage`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareGroupingStart`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareGroupingLeaf`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareNote`

Horizontal element ownership and final translation are defined by:

- `net.sourceforge.plantuml.sequencediagram.graphic.InGroupablesStack#addElement`
- `net.sourceforge.plantuml.sequencediagram.InGroupableList#getMinX`
- `net.sourceforge.plantuml.sequencediagram.InGroupableList#getMaxX`
- `net.sourceforge.plantuml.sequencediagram.graphic.NoteBox#getSegment`
- `net.sourceforge.plantuml.sequencediagram.graphic.NoteBox#getStartingX`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageArrow#getLeftStartInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageArrow#getRightEndInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageExoArrow#getLeftStartInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageExoArrow#getRightEndInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageExoArrow#getActualWidth`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareMissingSpace`
- `net.sourceforge.plantuml.sequencediagram.graphic.GraphicalDivider#drawInternalU`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSet#drawPlaygroundU`

## Corrected lifecycle invariant

`LifeLine#addSegmentVariation` is an append-only acceptance state machine.
For each participant:

1. Variations arrive in source/layout preparation order.
2. A variation whose ordinate is lower than the last accepted ordinate is
   ignored.
3. At the same ordinate, a variation of the opposite type is ignored.
4. Otherwise the variation is appended and immediately updates the stair
   level.

Java does not sort equal-ordinate variations, and it does not impose
`SMALLER` before `LARGER`. Arrival order determines which opposite variation
is retained. `LifeLine#finish` appends only the missing closing variations at
the final ordinate through the same acceptance function.

The review7 names `same_ordinate_close_then_open_rejected` and
`same_ordinate_filtered_pair` do not prove that their source-level commands
share an ordinate. `DrawableSetInitializer#prepareLiveEvent` projects attached
self-message lifecycle commands onto distinct graphical ordinates:

- activation is installed at message start plus 8;
- close/destroy is installed at message end minus 7;
- a self-message's arrow-only end is start plus 13;
- a combined lifecycle row extends that end before the close projection.

Consequently close-then-open can produce ordinates `start + 6` and
`start + 8`, both accepted. Open-then-close can produce `start + 8` followed
by `start + 6`, so the close is rejected for moving backward. True
equal-ordinate opposite-type rejection remains part of `LifeLine`, but it
must not be inferred from source-event Y values. The accepted timeline stores
these projected ordinates directly; activation-bar geometry must not add the
projection later during paint.

At `6ff6d9a8`, `SequenceLifeLines` collects raw variations and then sorts them
by ordinate, close-before-open, and event index. The adjacent comment claims
that Java applies closing variations first, but Java source directly
contradicts it. `closing_keeps_depth` then patches one consumer after the
incorrect ordering has already been introduced.

The replacement must install variations once, in arrival order, with Java's
rejection rules. Depth, segment shifts, activation bars, message endpoints,
note top/bottom segments, and page-end closure must query that accepted
timeline. No consumer may reinterpret the rejected raw event.

## Corrected horizontal invariant

Java creates concrete graphical elements first. Messages, external arrows,
notes, and grouping elements implement the horizontal geometry queried by
`InGroupablesStack` and `InGroupableList`. `NoteBox#getSegment` merges the
participant's live segment at the note's top and bottom ordinates, then
`NoteBox#getStartingX` derives the raw position from that merged segment.

`DrawableSetInitializer#prepareMissingSpace` evaluates every actual element:

- negative element starts contribute left missing space;
- actual arrow and grouping-header widths can exceed preferred widths;
- element ends contribute right missing space;
- the constraint set shifts participants and elements to satisfy both sides.

Only after this solve does `DrawableSet#drawPlaygroundU` provide the final
width to dividers and other drawables.

At `6ff6d9a8`, `group_note_left_floor_by_event` replaces grouped left-note
positions with a topology-derived coordinate floor. Other scans independently
derive note depth, group extents, external-arrow boundaries, activation bars,
and paint endpoints. This can leave a note at negative x with an undersized
canvas, shift a grouped note far from Java, widen a frame while shortening its
external arrow, or give a divider a stale width.

The replacement must represent actual horizontal elements and solve one
two-sided envelope. The grouped-note floor is removed, not adjusted.

## Arrow-area consequence

`MessageArrow` and `MessageExoArrow` query the accepted live segments at the
message ordinate. `ComponentRoseArrow` then paints source and target dressings
in the resulting component-local area. A nested active source therefore
changes both the component area's endpoint and spacing constraints.

The fresh mirrored active-source failures show that
`message_area_right_end` plus direction-specific `render_activation` state is
not a general substitute. The correction is to derive both spatial endpoints
from the accepted participant segments and then transform semantic dressings
within that one area.

## Competing histories to remove

The following remain independent state histories at `6ff6d9a8`:

- `activation_depth` during vertical spacing;
- `act_depth` during note precomputation;
- `SequenceLifeLines` during selected segment queries;
- `ActivationTracker` and `open_activations` during bar construction;
- group-specific activation and extent scans;
- `render_activation` during paint;
- `group_note_left_floor_by_event` during grouped-note placement.

They may be retained only as derived, immutable views of the accepted
timeline and concrete horizontal elements. They may not replay raw events or
mutate depth independently.

## Planned model change

1. Extend the normalized lifecycle pass so each participant owns an
   append-only accepted variation timeline with Java's monotonic-ordinate and
   same-ordinate rejection rules.
2. Derive message rows and ordinates first, then feed lifecycle variations to
   that timeline in `DrawableSetInitializer` preparation order.
3. Derive activation bars and every live endpoint from accepted segments,
   including unfinished page-end closure.
4. Represent messages, external arrows, notes, grouping headers/leaves, and
   dividers as typed horizontal elements with preferred, actual, minimum, and
   maximum extents.
5. Register those elements with nested group lists and propagate actual child
   extents through the group stack.
6. Run one two-sided missing-space/envelope solve and translate all consumers
   from its result.
7. Remove the grouped-note coordinate floor and all independent mutable depth
   replays as their consumers migrate.

## Acceptance

1. All 37 designated historical perturbations remain strict matches.
2. All 16 `review6_` and all 17 `review7_` valid perturbations become strict
   matches without editing their sources or Java SVGs.
3. Fresh checker inputs independently reverse projected lifecycle arrival
   order, include a genuinely equal-ordinate opposite-type pair, vary nested
   activation depth, mirror semantic/spatial source sides, and rotate
   note/group/external/divider ownership of the widest extent.
4. A structural audit finds one accepted lifecycle timeline and one
   horizontal-element envelope solve. No note, group, bar, external, or paint
   consumer independently replays activation state.
5. No fixture-shaped branch, observed-delta constant, comparator change,
   baseline relaxation, invalid golden, or post-hoc SVG surgery is introduced.
