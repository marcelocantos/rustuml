# Class quark review-7 response

## Rejected boundary

Review record `review-7.json`, committed as `f09a4be5`, rejects implementation
commit `78b19bf5`. All 51 prior scoped identity controls pass, but only 6 of 8
fresh `review7_` controls pass.

The creation-ordered quark index is retained. The rejection identifies two
separate omissions:

1. namespace separator lexical state is not replaced coherently when changing
   from `none` to a non-null separator;
2. flattened descendant-entity membership cannot represent Java's distinction
   between an empty package leaf and a group cluster containing direct child
   groups.

## Namespace separator state

PlantUML `CommandNamespaceSeparator#executeArg` replaces the active separator
on every command. `none` is not an independent sticky parser mode.

Rust currently stores both `namespace_sep_none` and `namespace_sep`. Setting
`none` updates both, but setting a later non-null separator updates only
`namespace_sep`. The stale boolean keeps selecting the literal-name entity
grammar even though identity splitting already uses the replacement separator.

The correction will use `namespace_sep: Option<String>` as the sole state:

- `None` selects literal-name parsing and creation-ordered
  `Plasma#firstWithName`;
- `Some(separator)` selects qualified-name parsing and the ordinary
  current-context versus reusable-unique lookup modes;
- every separator command replaces that one value.

No second boolean may influence declaration grammar.

## Package render roles

Java's quark tree retains direct child structure. `Entity#isEmpty` is false
when a group has any direct nonremoved child, including a child group that is
itself empty. `GraphvizImageBuilder` therefore distinguishes:

- a scope with direct leaf or group children: render as a cluster;
- an explicit scope with no direct children: render as an `EMPTY_PACKAGE`
  leaf;
- a synthetic quark with no renderable role: keep for identity, omit from
  paint.

Rust's `Package.entities` is intentionally transitive: ancestors contain
descendant entity ids. `entities.is_empty()` therefore cannot answer either
direct-child question. The correction will derive a diagram-wide package role
from:

1. the explicit `parent` relation between package quarks;
2. each entity's innermost owning package;
3. explicit versus phantom package metadata.

A package is a cluster when it owns at least one direct entity or has at least
one direct package child. An explicit childless package is an empty-package
leaf. A childless synthetic phantom remains hidden. Ancestors with live deep
descendants remain clusters through their direct package-child chain.

Cluster filtering, layout ownership, UID ordering, and SVG metadata will
consume this one role calculation. The role is not inferred from labels,
fixture names, source-line values, or eventual flattened leaf count.

## Acceptance

1. All 59 `review2_` through `review7_` sources match the ordered scoped
   identity predicate without editing their sources or Java SVGs.
2. Fresh controls reverse separator transitions repeatedly and combine empty
   siblings, empty nested chains, phantom parents, direct leaves, and deep
   descendants.
3. Parser and class renderer unit suites pass.
4. The class no-oracle family does not regress from `1940/2140` passes with
   `169` intentional error skips.
5. No fixture-shaped branch, golden read, source-line exception, comparator
   change, or invalid-syntax reclassification is introduced.

