# Class package-render review-8 response

## Rejected boundary

Review record `review-8.json`, committed as `5125f0ad`, rejects implementation
commit `0caa6d9f`. The class family remains at its `1940/1971` eligible baseline
and the existing scoped identity predicate reports `59/59`, but that predicate
cannot observe PlantUML `EMPTY_PACKAGE` leaves because they are not wrapped in
`g[class=entity|cluster]`.

The rejection identifies three incomplete parts of the ported model:

1. `EmptyLeaf` packages participate in layout but have no SVG painter;
2. null-separator package lookup bypasses the shared creation-ordered quark
   index;
3. the separator command grammar omits Java-valid aliases and case variants.

## Java mechanisms

The oracle revision is
`71806a23780b04a5ccde2f8ceb5121edad5eb711`.

`GraphvizImageBuilder#printGroups` converts a directly empty package to
`LeafType.EMPTY_PACKAGE` and routes it through ordinary entity printing.
`EntityImageEmptyPackage#calculateDimensionSlow` and `#drawU` jointly define
its layout envelope and visible package chrome. Rust must therefore render
`PackageRenderRole::EmptyLeaf` from the same layout node used by the package
graph; it cannot remain an invisible spacing proxy.

`CucaDiagram#quarkInContextSafe` calls creation-ordered
`Plasma#firstWithName` whenever the namespace separator is null. Package
commands use that same method, so entity and package declarations share one
ordered quark namespace. Rust package lookup must use `quark_creation_order`
and may not search `package_by_path` by hash-map iteration or exclude entity
quarks from the first-match decision.

`CommandNamespaceSeparator#getRegexConcat` accepts both `separator` and
`namespaceseparator`. `#executeArg` treats `none` case-insensitively and
replaces the current separator state. Rust will parse both spellings and all
case variants into the existing single `Option<String>` state.

## Planned model change

The renderer will add one `EmptyPackageLayout` record per
`PackageRenderRole::EmptyLeaf`. The record will carry the package identity,
layout rectangle, display label, stereotype, and resolved colors. Both SVG
paint and any package-edge attachment will consume that record. Cluster
painting remains exclusive to `PackageRenderRole::Cluster`; hidden phantom
quarks remain absent from paint.

The parser will replace `resolve_group_path`'s null-separator hash-map search
with the same creation-ordered first-quark lookup used by entity resolution.
The first quark decides identity even when its current rendered role is an
entity. Package commands will then apply their package semantics to that
shared quark instead of selecting a later package-only match.

The separator command parser will use a case-insensitive command/value match
for `set separator` and `set namespaceSeparator`. Every accepted command
replaces the one separator option; no secondary lexical-mode flag is added.

## Acceptance

1. Every existing `review2_` through `review7_` source and Java SVG remains
   unchanged and preserves its prior scoped identity result.
2. A fresh independent checker preserves and validates reversed duplicate
   package creation order, entity/package quark collisions, both separator
   aliases with mixed-case `none`, and repeated transitions back to custom
   separators.
3. The checker asserts visible empty-package chrome for root and nested leaves,
   including renamed labels, stereotypes, colors, empty siblings, and deep
   descendants. Identity-only predicates are insufficient.
4. Parser and class-renderer unit suites pass, and the class no-oracle family
   does not regress from `1940/1971` eligible passes.
5. No fixture label, source line, SVG coordinate, golden read, count ladder, or
   package-name exception influences lookup, role selection, layout, or paint.
