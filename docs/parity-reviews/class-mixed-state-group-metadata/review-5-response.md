# Class metadata review-5 response

## Rejected boundary

Review record `review-5.json`, committed as `3f9c04a9`, rejects parser
implementation `36916037` on one of 17 fresh scoped identity inputs.
`review5_ambiguous_duplicate_short_names` proves that declaration lookup and
relationship lookup are distinct PlantUML operations.

## Pinned Java model

At PlantUML revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`:

- `CucaDiagram#quarkInContextSafe(false, name)` returns the child of the
  current group for an unqualified name.
- `CucaDiagram#quarkInContextSafe(true, name)` reuses an existing quark only
  when `Plasma#countByName(name) == 1`; ambiguity falls back to the child of
  the current group.
- `Plasma#countByName` counts quarks, including group and intermediate quarks,
  rather than only materialized entities.
- `CommandCreateClass` and `CommandCreateClassMultilines` pass `false`.
- `CommandCreateElementFull2` and `CommandLinkClass` pass `true`.
- `ClassDiagramFactory#initCommandsList` registers the ordinary class commands
  before the mixed-element command, so `class Shared` uses current-context
  lookup even in an allowmixing diagram.

The failing input therefore creates `R5Left.Shared`, then
`R5Right.Shared`. Its later root relationship endpoint sees two matching
quarks, cannot uniquely reuse either, and creates root `Shared`.

## Rust divergence

`ClassParser::resolve_quark_path` already represents the boolean lookup
distinction, and relationship endpoints request unique reuse. However,
`ClassParser::try_entity_decl` sends every declaration through unique reuse
because the parser does not retain the Java command family that matched it.
The second ordinary class declaration consequently reuses
`R5Left.Shared`; the root endpoint then also appears unique and reuses the
same entity.

## Planned correction

Replace the boolean call-site contract with an explicit lookup mode:

- ordinary class and multiline-class declarations use `CurrentContext`;
- mixed element declarations use `ReuseUnique`;
- relationship endpoints use `ReuseUnique`.

Unique-name counting must ultimately cover canonical quark paths for groups,
phantom parents, and leaves, not only `entity_by_path`. `namespaceSeparator
none` retains Java's separate first-name behavior.

The correction must preserve all 28 prior scoped controls and the 16 passing
review5 inputs. Fresh review must include sibling and nested duplicate class
names, endpoint creation before and after ambiguity, fallback inside a
non-root package, mixed-element reuse controls, custom separators, and
package/entity short-name collisions.
