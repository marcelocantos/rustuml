# Mixed metadata review-3 response

## Rejected boundary

Review commit `37c4763b` confirms that `0b06536b` repairs the declared
description-leaf kinds and preserves all six earlier metadata matrices. Two
independent parser ownership gaps remain: class dispatch for description-only
`allowmixing` sources, and explicit code ownership when a declaration refines
an endpoint first created by a relationship.

## Dispatch ownership

PlantUML handles `allowmixing` as a class-diagram command. Its presence is
therefore sufficient to select the class parser even when every concrete leaf
uses description syntax (`actor`, `usecase`, `component`, `queue`, and related
symbols). It is not merely a modifier applied after another class token has
already won subtype detection.

RustUML's subtype detector currently requires a separate positive class score
before `allowmixing` promotes the source. That sends a valid mixed source to the
description parser, where the directive itself is tokenized as bogus endpoint
text. The correction belongs in UML subtype detection: a standalone,
case-insensitive `allowmixing` command is direct class evidence. It must not
depend on fixture leaf names or relationship count.

## Quark identity

Java `CommandCreateElementFull2` resolves the declaration code or alias to a
quark before deciding whether a leaf already exists. If a relationship created
that quark first, the later declaration refines the existing entity's display
and concrete symbol; it does not replace the quark or its original creation
location. `EntityImageDescription` subsequently emits the stable quark code as
`data-qualified-name`.

RustUML already retains the first relationship source line, but
`ClassParser::try_entity_decl` updates an existing implicit entity without
recording that the declaration supplied an explicit alias. Qualification then
falls back to the quoted display label. The correction is to promote the
existing entity to explicit-code ownership when the declaration has an alias,
while retaining its original source line and relationship identity. Display
text and `EntityKind` remain refinable presentation fields.

## Acceptance

1. All six `review2_` controls and all eight `review3_` matrices retain exact
   entity/cluster qualified-name and source-line metadata.
2. Description-only valid `allowmixing` sources dispatch directly to CLASS and
   never synthesize tokens from the directive.
3. Relationships before declarations preserve first-creation source lines,
   while later aliases become the stable qualified code for actor, usecase,
   component, queue, and renamed variants.
4. A fresh matrix varies declaration order, quoted display text, aliases, and
   repeated/reversed relationships without changing the result.
5. The complete no-oracle ratchet has no pass-for-fail swaps.
