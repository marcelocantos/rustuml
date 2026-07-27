# Review 6 Response: Ordered Quarks and Live Groups

Review 6 rejects `b7b00d50` on two valid parser-owned counterexamples. The
ordinary declaration versus unique-reuse split is correct, but the model is
still incomplete at two adjacent boundaries.

## Separator-None Lookup

`CucaDiagram#quarkInContextSafe` has a separate branch when
`getNamespaceSeparator()` returns `null`. It calls
`Plasma#firstWithName(full)` and reuses that quark whenever one exists. The
lookup mode and duplicate count are irrelevant in this branch.

`Plasma#firstWithName` returns the first quark registered for the short name.
Its result therefore depends on quark creation order. Rust currently derives
uniqueness from `HashMap` keys collected into a `HashSet`; that representation
cannot answer the Java query.

The parser must retain one creation-ordered quark path index shared by group
and entity quarks. Register each path exactly once when its quark is first
materialized. Separator-none lookup selects the first path whose final
component equals the requested name. Non-null-separator unique reuse continues
to count distinct registered group and entity paths and reuses only when that
count is one.

## Group Liveness

`CommandPackage` must still create and enter a group quark before its body is
parsed. A mixed declaration inside that scope may then reuse a unique leaf
owned by another group through `CommandCreateElementFull2`. In that case the
new scope owns no leaf.

Java preserves the group quark for lookup and scope restoration but does not
emit an empty package cluster. This is visible independently in
`edge_boundary_empty_package`: `EmptyPkg` is absent while `NonEmpty` and its
leaf render.

Rust must therefore separate group existence from rendered-group liveness. A
group renders only when it owns an entity or has a rendered descendant.
Filtering cannot alter quark lookup, package-stack behavior, source ordering,
or the parentage of retained descendants.

## Planned Change

1. Add a creation-ordered quark path index to `ClassParser`.
2. Register package and entity paths exactly once at first materialization.
3. Use first-created lookup unconditionally when the namespace separator is
   disabled.
4. Keep the existing current-context and unique-reuse modes for non-null
   separators.
5. Exclude groups with no owned entity and no live descendant from class SVG
   cluster emission while retaining them in the parser's quark and scope
   model.

## Acceptance

1. All 45 prior scoped identity controls remain exact.
2. All six `review6_` inputs match Java for ordered entity/cluster qualified
   names and source lines.
3. `review6_namespace_none_duplicate_first_match` reuses
   `SepNoneAlpha.EchoToken` and creates no root `EchoToken`.
4. `review6_unique_mixed_reuses_remote_class` emits no `BridgeActors` cluster
   while preserving `BridgeClasses.TransitBridge`.
5. Parser unit tests, class renderer tests, and the no-oracle ratchet show no
   regression.
6. A fresh independent review varies creation order, duplicate counts, empty
   nested groups, and remote mixed-element reuse.
