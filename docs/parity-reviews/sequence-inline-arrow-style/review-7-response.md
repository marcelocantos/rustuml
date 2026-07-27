# Sequence review-7 response

## Rejected boundary

Review record `review-7.json`, committed as `bc038cb8`, rejects implementation
commit `e490ea6f`. The implementation preserved all 37 historical perturbations
and all 16 `review6_` perturbations, but passed only 10 of 17 `review7_`
perturbations and 1 of 8 fresh `review8_` perturbations.

The rejection is accepted. `e490ea6f` implemented selected lifecycle
projections and made several consumers share final numeric values, but did not
implement the two mechanisms promised by `review-6-response.md`:

1. one accepted `LifeLine` variation history used by every lifecycle consumer;
2. one collection of concrete graphical elements used by group membership,
   missing-space translation, final width, and paint.

Passing the older matrices while failing seven of eight fresh rotations is
evidence that the remaining consumer-specific formulas are structurally
overfit. Their observed coordinate differences are not correction constants.

## Pinned Java mechanisms

The lifecycle source of truth remains PlantUML revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711`:

- `LifeLine#addSegmentVariation`
- `LifeLine#finish`
- `LivingParticipantBox#getLiveThicknessAt`
- `DrawableSetInitializer#prepareLiveEvent`
- `DrawableSetInitializer#prepareMessage`
- `DrawableSetInitializer#prepareGroupingStart`
- `DrawableSetInitializer#prepareGroupingLeaf`
- `DrawableSetInitializer#prepareNote`

The horizontal source of truth is:

- `InGroupablesStack#addElement`
- `InGroupableList#getMinX`
- `InGroupableList#getMaxX`
- `GraphicalElement#getStartingX`
- `GraphicalElement#getPreferredWidth`
- `GraphicalElement#getActualWidth`
- `NoteBox#getSegment`
- `NoteBox#getStartingX`
- `MessageArrow#getLeftStartInternal`
- `MessageArrow#getRightEndInternal`
- `MessageExoArrow#getLeftStartInternal`
- `MessageExoArrow#getRightEndInternal`
- `MessageExoArrow#getActualWidth`
- `DrawableSetInitializer#prepareMissingSpace`
- `GraphicalDivider#drawInternalU`
- `DrawableSet#drawPlaygroundU`

## Lifecycle correction

`SequenceDepthSnapshots` and `SequenceLifeLines` currently replay the same raw
events into separate return stacks. They can disagree because each embeds its
own projection and acceptance policy. Sharing helper functions between those
replays would still leave two histories.

The replacement will build rows and their graphical ordinates first, then feed
each projected variation exactly once into one append-only participant
timeline. That timeline applies Java's arrival-order rules immediately:

1. reject an ordinate below the last accepted ordinate;
2. reject an opposite variation at the same ordinate;
3. otherwise append the variation and update the stair depth;
4. close unfinished segments at page end through the same acceptance method.

Pre-row spacing snapshots, message endpoints, note segments, activation bars,
group boundaries, returns, destroys, and paint will be immutable queries over
that accepted timeline. No second raw-event return stack or mutable paint-time
activation depth remains.

## Horizontal correction

The current renderer independently calculates left-note missing space, note
right bounds, reference bounds, lost-arrow bounds, group extrema, divider
widths, and final SVG width. Taking a maximum of those answers is not Java's
constraint model because nested membership and left translation are lost
before the maximum is taken.

The replacement will create a typed horizontal scene containing concrete
elements for:

- ordinary and self messages;
- external messages;
- standalone and message-owned notes;
- group headers, leaves, and enclosing frames;
- references;
- dividers.

Each element exposes its start, preferred width, and actual width at the
accepted live segments it owns. In source preparation order, each element is
registered with every active group list. Group frames derive their bounds from
those concrete members, including nested members, rather than reconstructing
membership from event ranges.

One envelope pass then derives left missing space and right missing space from
all elements, applies one participant/element translation, and publishes one
final width. Frames, external arrows, dividers, notes, and the SVG canvas
consume that solved scene. Consumer-specific right-bound scans and topology
floors are deleted as their consumers migrate.

The uncommitted preferred-width correction for left notes is valid Java
evidence from `NoteBox#getStartingX`, but it is not sufficient independently.
It will survive only as the `NoteBox` element's preferred-width definition
inside the shared scene.

## Acceptance

1. The 37 historical, 16 `review6_`, 17 `review7_`, and 8 `review8_`
   perturbations are replayed without changing their sources or Java SVGs.
2. Fresh review inputs rotate activation ownership, equal-ordinate arrival
   order, nested group membership, external-arrow direction, note side, and
   divider/frame ownership of the widest extent.
3. A structural audit finds one accepted lifecycle timeline. No spacing,
   grouping, bar, or paint consumer replays raw lifecycle events.
4. A structural audit finds one typed element collection, one nested-group
   registration path, and one two-sided envelope solve.
5. The sequence family and global no-oracle ratchets do not regress.
6. No fixture label, fixture filename, observed-delta branch, per-depth
   constant ladder, comparator change, baseline relaxation, or post-hoc SVG
   surgery is introduced.

