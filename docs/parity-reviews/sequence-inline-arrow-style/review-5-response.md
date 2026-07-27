# Sequence review-5 response

## Rejected boundary

Review record `review-5.json`, committed as `32b71af3`, rejects implementation
commit `d01dd24e05ef41fbcb2303d45dab96fab9efacba` against PlantUML revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711`. The implementation preserves all
37 scoped pre-existing perturbations, apart from the already declared physical
Serif exclusion, but only 6 of 16 fresh valid perturbations pass.

The fresh failures disprove the claim in causal-gate commit `99666502` that the
renderer has one message-owned lifecycle timeline. The problem is not the
observed 5-pixel and 30-pixel deltas. RustUML reconstructs attachment, row
allocation, activation depth, endpoint geometry, and horizontal extents in
separate scans. Those scans do not implement the same event semantics and can
therefore disagree.

## Pinned Java model

The relevant Java source is PlantUML revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711`.

Lifecycle ownership and row allocation are defined by:

- `net.sourceforge.plantuml.sequencediagram.SequenceDiagram#addMessage`
- `net.sourceforge.plantuml.sequencediagram.SequenceDiagram#activate`
- `net.sourceforge.plantuml.sequencediagram.SequenceDiagram#grouping`
- `net.sourceforge.plantuml.sequencediagram.AbstractMessage#addLifeEvent`
- `net.sourceforge.plantuml.sequencediagram.AbstractMessage#isActivateAndDeactive`
- `net.sourceforge.plantuml.sequencediagram.GroupingLeaf#addLifeEvent`
- `net.sourceforge.plantuml.sequencediagram.graphic.Step1Message#prepareMessage`
- `net.sourceforge.plantuml.sequencediagram.graphic.Step1MessageExo#prepareMessage`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareLiveEvent`

Live geometry and shared extents are defined by:

- `net.sourceforge.plantuml.sequencediagram.graphic.LivingParticipantBox#getLiveThicknessAt`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageArrow#getLeftStartInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageArrow#getRightEndInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageExoArrow#getLeftStartInternal`
- `net.sourceforge.plantuml.sequencediagram.graphic.MessageExoArrow#getRightEndInternal`
- `net.sourceforge.plantuml.skin.rose.ComponentRoseArrow#drawInternalU`
- `net.sourceforge.plantuml.skin.rose.ComponentRoseArrow#drawDressing1`
- `net.sourceforge.plantuml.skin.rose.ComponentRoseArrow#drawDressing2`
- `net.sourceforge.plantuml.sequencediagram.graphic.NoteBox#getSegment`
- `net.sourceforge.plantuml.sequencediagram.graphic.NoteBox#getStartingX`
- `net.sourceforge.plantuml.sequencediagram.graphic.InGroupablesStack#addElement`
- `net.sourceforge.plantuml.sequencediagram.InGroupableList#getMinX`
- `net.sourceforge.plantuml.sequencediagram.InGroupableList#getMaxX`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSetInitializer#prepareMissingSpace`
- `net.sourceforge.plantuml.sequencediagram.graphic.GraphicalDivider#drawInternalU`
- `net.sourceforge.plantuml.sequencediagram.graphic.DrawableSet#drawPlaygroundU`

## Invariant

PlantUML has two related but distinct facts for every lifecycle command:

1. The command has an attachment owner that determines its ordinate.
2. The command may or may not contribute a lifecycle type to an
   `AbstractMessage` for combined-row classification.

Messages replace `SequenceDiagram.lastEventWithDeactivate`. Ordinary
intervening events, including notes, dividers, autonumber commands, and group
starts, do not. A group end replaces it with the `GroupingLeaf`. A self-message
still attaches an unrelated participant's lifecycle event to itself, but
`AbstractMessage#addLifeEvent` returns before recording that lifecycle type.

For an `AbstractMessage`, the extra row is present only when the first accepted
lifecycle type is `ACTIVATE` and a later accepted type is `DEACTIVATE` or
`DESTROY`. `Step1Message` and `Step1MessageExo` derive the message start and end
ordinates from that classification. `DrawableSetInitializer#prepareLiveEvent`
then installs all activation variations on the shared `LifeLine`, and every
later geometry consumer queries that same line at its own ordinate.

The Rust implementation must therefore normalize ownership, accepted
lifecycle-type order, message start/end ordinates, and live segments once.
Rendering and extent consumers must query that model rather than replaying raw
events independently.

## GroupEnd ownership

Java `SequenceDiagram#addMessage` assigns the message to
`lastEventWithDeactivate`. `SequenceDiagram#grouping` deliberately leaves that
owner unchanged for group starts, but assigns the closing `GroupingLeaf` for
`GroupingType.END`. A later standalone deactivation is consequently offered to
`GroupingLeaf#addLifeEvent`, which accepts it without associating it with the
earlier message. In `DrawableSetInitializer#prepareLiveEvent`, that event has
no message ordinate and uses the current group-end flow position.

At `d01dd24e`, `crates/rustuml-render/src/sequence.rs:7796-7827` tracks
`last_message_owner`, but changes it only for `Event::Message` and
`Event::Return`. `Event::GroupEnd` is ignored. A deactivation after the end is
therefore attached to the pre-group-end message and can incorrectly extend
that message by the combined-row height. The same bad owner is then consumed
by right-border depth logic.

The model change is to represent the Java owner domain explicitly, including
message, group-end leaf, and no owner. Group starts preserve the current owner;
group ends replace it. Only an `AbstractMessage` owner can accumulate accepted
types or reserve a combined row.

Relevant perturbation axes are:

- group start versus group end between a message and lifecycle command;
- left-boundary versus right-boundary external messages;
- nested groups and mirrored participant declaration order;
- deactivation and destruction after the ownership boundary.

## Activation order

Java `AbstractMessage#addLifeEvent` records lifecycle types in
`lifeEventsType`. It sets `firstIsActivate` only when the accepted type set has
size one and that type is `ACTIVATE`. `isActivateAndDeactive` requires that
historical fact plus a recorded `DEACTIVATE` or `DESTROY`. Seeing an activation
at any later point is insufficient.

At `d01dd24e`, `sequence.rs:7799-7822` reduces this history to
`owner_has_activation: bool`. Any later activation sets the boolean, including
one following a deactivation. A subsequent deactivation then reserves the
row, even though Java's first accepted type was `DEACTIVATE`.

The normalized message record must retain accepted lifecycle-type order, or an
equivalent state machine with `first_accepted_is_activate` and closing-type
presence. It must never infer Java's predicate from unordered "has activation"
and "has deactivation" flags.

Relevant perturbation axes are:

- activate then deactivate versus deactivate then activate;
- repeated lifecycle types versus mixed types;
- unrelated participants interleaved on one owner;
- notes, dividers, and autonumber commands between lifecycle commands.

## Self-message filtering

Java `AbstractMessage#addLifeEvent` first sets the lifecycle event's message.
For a self-message, it then returns before adding the type when the lifecycle
participant differs from the self participant. This distinction is essential:
the event remains attached for ordinate placement, but it does not influence
`firstIsActivate`, `isDeactivateOrDestroy`, or the combined-row predicate.

At `d01dd24e`, the owner scan at `sequence.rs:7811-7823` records every
standalone activation, deactivation, and destruction against the last message.
It has no self-participant filter and no separation between attachment and
row-classification eligibility. Unrelated lifecycle commands can therefore
turn a self-message into a false combined row.

The normalized lifecycle entry must carry both its attachment owner and
whether Java accepts its type into that owner's lifecycle-type state. The
self-message participant check controls only the latter. It must not be
implemented by dropping the event or moving its ordinate.

Relevant perturbation axes are:

- self-message versus non-self owner;
- matching versus unrelated lifecycle participant;
- participant declaration order and renamed aliases;
- activation/deactivation pairs followed by ordinary and bidirectional
  messages.

## Active-source cross placement

Java does not place a source cross by shifting a semantic sender coordinate.
`MessageArrow#getLeftStartInternal` and `#getRightEndInternal` first construct
the complete spatial arrow area from the left and right participants'
`LivingParticipantBox#getLiveThicknessAt` segments at the message ordinate.
`ComponentRoseArrow#drawInternalU` then paints dressing 1 and dressing 2 at
fixed local positions within that area. `#drawDressing1` and
`#drawDressing2` apply the cross geometry in that local coordinate system.

At `d01dd24e`, `sequence.rs:10885-10943` derives a direction-specific
`from_x_shifted`, and `sequence.rs:11030-11044` and
`sequence.rs:11244-11252` derive the cross center from that semantic-source
coordinate. The inactive control happens to agree because the lifeline segment
collapses to a point. Once the spatial source has a live segment, the semantic
shift and the component-area endpoint are no longer interchangeable.

The correction is to compute one arrow area from the left and right live
segments at the message ordinate, then transform dressing-local geometry
through that area. Source/target semantics select a dressing; they do not
select a second endpoint-shift formula. Existing Java-provenanced component
dimensions remain unchanged, and no active-source correction constant is
introduced.

Relevant perturbation axes are:

- inactive, singly active, and nested active source segments;
- semantic source on the spatial left versus spatial right;
- solid, dotted, and thin half heads;
- source cross combined with each target half-head orientation.

## Fragmented lifecycle and extent consumers

The reviewer found that correct lifecycle ordinates in some cases did not make
notes, group frames, dividers, and shared external borders correct. That is
expected from the current architecture:

- `sequence.rs:7702-7770` computes note shifts with its own `active_depth`
  replay and stores only a one-half-width active flag.
- `sequence.rs:7796-7827` separately computes lifecycle ownership and row
  offsets.
- `sequence.rs:8396-8483` separately evolves `lost_scan_activation` for
  external-width candidates.
- `sequence.rs:8780-8923` separately constructs activation bars with
  `ActivationTracker` and `open_activations`.
- `sequence.rs:9234-9576` separately evolves `group_act_depth` while
  reconstructing group extents.
- `sequence.rs:10423-10541` and `sequence.rs:12853-12865` separately evolve
  `render_activation` for painted endpoints.
- `sequence.rs:11683-11733` gives dividers a participant/group-derived span,
  even when the final shared external envelope is wider.

Java's flow is different. `Step1Message` and `Step1MessageExo` register the
actual arrow and living participant boxes with `InGroupablesStack`.
`DrawableSetInitializer#prepareNote` registers the actual `NoteBox`.
`NoteBox#getSegment` merges the shared `LifeLine` segment at the note's top and
bottom ordinates. Group frames obtain their range from those same registered
`InGroupable` objects. `DrawableSetInitializer#prepareMissingSpace` then
evaluates the actual graphical elements, including arrow actual widths, to
produce the final horizontal envelope. Finally, `DrawableSet#drawPlaygroundU`
passes that one final width to every element, and
`GraphicalDivider#drawInternalU` spans it.

The Rust correction must introduce a single normalized sequence layout model
with:

- lifecycle attachment and accepted-type state per event;
- message start and end ordinates plus combined-row allocation;
- participant live segments at message, note-top, note-bottom, and lifecycle
  ordinates;
- arrow areas and actual extents derived from those segments;
- group membership over actual element extents;
- one final horizontal envelope consumed by dividers, notes, group frames, and
  all four external-message directions.

Activation bars, source and target endpoints, note anchors, group ranges,
external preferred/actual widths, and the paint pass must consume this model.
The independent raw-event depth maps listed above must be removed or reduced
to views over it. A new helper that performs another event replay would
preserve the divergence and does not satisfy this response.

Relevant perturbation axes are:

- owner-preserving notes, dividers, autonumber directives, and group starts;
- owner-resetting group ends at multiple activation depths;
- FROM_LEFT, TO_LEFT, FROM_RIGHT, and TO_RIGHT sharing each boundary;
- notes spanning an activation transition over their vertical height;
- nested group membership with internal and external arrows;
- final divider width with participant, group, note, and external-message
  envelopes taking turns as the widest element.

## Planned model change

1. Build a typed lifecycle attachment pass that follows
   `SequenceDiagram#addMessage`, `#activate`, and `#grouping`, including
   `GroupingLeaf` ownership and the self-message accepted-type filter.
2. Derive each message's ordered lifecycle summary, combined-row predicate,
   start ordinate, and end ordinate from that pass.
3. Apply lifecycle variations once to a queryable participant timeline
   equivalent to Java's `LifeLine`.
4. Derive message and external arrow areas, note segments, activation bars,
   and group element extents from that participant timeline.
5. Resolve one horizontal constraint envelope from actual element extents and
   use it for final drawing, including dividers.
6. Delete the competing lifecycle/depth replays as their consumers migrate.

This is a model replacement, not a coordinate patch. It forbids branches keyed
to review fixture names, labels, participant counts, or observed SVG
coordinates. It also forbids adding 5-pixel, 6-pixel, or 30-pixel
counterexample corrections. Existing dimensions may be used only where they
already follow the cited Java component contract.

## Acceptance

1. All 37 scoped pre-existing perturbations remain strict matches; the
   declared physical Serif case remains separately excluded.
2. All 16 `review6_` perturbations become strict matches without editing their
   inputs or Java SVGs.
3. Fresh checker inputs independently vary ownership boundary, lifecycle
   order, self-participant filtering, source-side activity, and an intervening
   extent consumer.
4. A structural audit finds one lifecycle attachment model and one
   participant live-segment model, with no independent note, group, external,
   activation-bar, or paint-pass depth replay.
5. No fixture-shaped branch, unexplained parity constant, comparator change,
   baseline relaxation, or pass-for-fail ratchet swap is introduced.
