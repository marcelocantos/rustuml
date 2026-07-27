# Mixed metadata review-4 response

## Scope and revisions

This response addresses the rejected parser model in review commit `7fb30ee8`
without changing production code, fixtures, baselines, or the reviewer record.

- PlantUML revision: `71806a23780b04a5ccde2f8ceb5121edad5eb711`
- Rust implementation under review:
  `30902301fa70d9410bcfd7d38be52b654970e29b`
- Rust tree inspected:
  `025bd4ec620fae66823996148449ce63fba18332`
- The current versions of `crates/rustuml-parser/src/parse/mod.rs` and
  `crates/rustuml-parser/src/parse/class.rs` are unchanged from `30902301`.

The implementation is rejected. It repaired lowercase, top-level cases but
did not reproduce PlantUML's command grammar or hierarchical quark ownership.

## Corrected invariant

PlantUML has two connected invariants:

1. `allowmixing` is a complete class-diagram command. It is accepted only when
   the whole line matches `allow_?mixing`, case-insensitively. The same command
   grammar controls factory selection and command execution.
2. Group and leaf identity is a path in a quark tree, not a display string or
   a flat entity ID. A package alias supplies the group's identity path
   segment, while its quoted name is only its display. Relationship endpoints
   resolve through that tree. The first command that materializes an entity
   owns its quark, source location, and leaf type; a later declaration may
   update fields that Java explicitly mutates, such as display, but does not
   replace that identity or type.

Rust currently approximates these invariants with independent string-prefix
tests, flat entity IDs, package membership vectors, and a partial package-name
strip. Those approximations explain all four fresh scoped failures.

## Java model

All Java locations below refer to revision `71806a2`.

### Directive grammar and class dispatch

`ClassDiagramFactory#initCommandsList` registers `CommandAllowMixing`.
`CommandAllowMixing#getRegexConcat` builds:

```text
start + "allow" + "_?" + "mixing" + end
```

`Pattern2#compileInternal` compiles that expression with
`Pattern.CASE_INSENSITIVE`. `CommandAllowMixing#executeArg` then sets the
class diagram's allow-mixing state. `PSystemBuilder#createPSystem` tries the
registered diagram factories against the source, so acceptance of this exact
class command is direct evidence for the class factory.

The consequences are:

- `allowmixing`, `allow_mixing`, and case variants are valid.
- Trailing tokens are invalid because the command is end-anchored.
- Recognition and consumption cannot disagree about spelling.

### Package alias ownership

`CommandPackage#getRegexConcat` parses a package `NAME`, an optional `AS`, the
opening brace, and the end of the line. In `CommandPackage#executeArg`:

- without `AS`, the cleaned name is both the quark identity and display;
- with `AS`, `AS` is passed to `CucaDiagram#quarkInContext` as the identity,
  while `NAME` remains the display;
- `gotoGroup` enters the group represented by that quark.

Therefore:

```text
package "Review Four Domain" as R4Domain {
```

creates a group whose identity segment is `R4Domain` and whose display is
`Review Four Domain`. Children belong below the `R4Domain` quark. The display
text never becomes the qualified identity.

`CommandNamespace#executeArg` follows the same quark discipline for namespace
names: it resolves the namespace through `quarkInContext` and enters that
group. Nested namespaces therefore form a parent/child identity path.

### Endpoint lookup and first-creation ownership

`CommandLinkClass#executeArg` resolves both endpoints with
`CucaDiagram#quarkInContextSafe(true, endpoint)`. In
`CucaDiagram#quarkInContextSafe`:

- an unqualified name may reuse the one existing matching child;
- a leading separator starts at the root;
- a qualified name whose first group exists resolves through
  `root.child(full)`;
- otherwise the name is resolved relative to the current group.

Thus an external reference such as `R4Outer.R4Inner.R4Gateway` resolves to the
already materialized nested quark instead of creating another leaf.

When a relationship first materializes a missing endpoint,
`CommandLinkClass#executeArg` calls `reallyCreateLeaf` with `LeafType.CLASS`
and the relationship location. Later, `CommandCreateElementFull2#executeArg`
calls `quarkInContext(true, idShort)`, reuses the existing entity when the
quark already has data, and calls `setDisplay`. It calls `reallyCreateLeaf`
with the declaration's requested leaf type and symbol only when no entity
exists.

This corrects `review-3-response.md`: `EntityKind` is not a generally
refinable presentation field. A later actor, usecase, component, or other
declaration does not retype a relationship-created Java class leaf. The
original quark, source location, and `LeafType.CLASS` remain owned by the
relationship; only Java's explicitly mutable fields are updated.

## Rust divergence

### Mixed-case and underscored `allowmixing`

At `30902301`, and still in the current parser:

- `parse/mod.rs::detect_uml_subtype` recognizes only lowercase
  `trimmed == "allowmixing"` or `trimmed.starts_with("allowmixing ")`.
- `parse/class.rs::ClassParser::try_meta` independently consumes any lowercase
  line beginning with `allowmixing`.

These checks are both narrower and broader than Java. They reject valid
mixed-case and optional-underscore spellings, but accept invalid trailing
text. On rejected valid spellings, Rust selects the description factory and
then invents entities from pieces of the unconsumed directive.

The causal error is duplicated, lossy command recognition. It is not the
factory score, the labels in the held-out inputs, or the number of mixed
leaves.

### Package alias ownership

`parse/class.rs::ClassParser::try_package` has a start-anchored but not
end-anchored regex with no `as CODE` capture. It matches only the quoted
display prefix, silently ignores the alias suffix, and stores that display in
`Package.name`. `Package.display_name` is not used to separate an explicitly
aliased package's presentation from its identity.

Children are consequently registered under `Review Four Domain` rather than
`R4Domain`. The later `R4Domain.R4DomainComponent` relationship cannot address
the existing group path and materializes a duplicate endpoint.

The causal error is that Rust's package data flow conflates group identity and
display, then permits a partial grammar match. It is not an endpoint-label
special case.

### Nested qualified endpoint duplication

`ClassParser::resolve_relationship_endpoint` first compares the raw string to
flat `ClassEntity.id` values. It then iterates packages and tries to strip one
`Package.name` prefix before checking `Package.entities`. A nested package
stores its own segment and parent index, but the resolver never constructs or
looks up the complete parent path.

For `R4Outer.R4Inner.R4Gateway`, no single package named
`R4Outer.R4Inner` exists for the one-prefix strip. The resolver therefore
falls through to `ensure_entity(raw)` and creates a second entity at the
relationship line, even though the nested `R4Gateway` already exists.

The causal error is the absence of a canonical hierarchical identity lookup.
Adding another separator or another prefix strip would preserve the faulty
flat model and fail at a different nesting depth.

### Existing-entity type mutation

`ClassParser::ensure_entity` creates implicit relationship endpoints as
`EntityKind::Class`. `ClassParser::try_entity_decl` later unconditionally
assigns `entity.kind = kind` when it finds that existing ID. Java does not make
the corresponding leaf-type mutation.

This did not cause the four review-4 scoped metadata failures, but it is a
direct model-account error exposed by the same held-out analysis. It must be
corrected with the identity work rather than retained because the current
renderer masks it.

## Planned generative model change

Before production edits, this analysis must be represented in the mechanism's
required machine-readable `account.json` and committed as the durable causal
account. This response draft is not a substitute for that gate.

The implementation should then make these model changes:

1. Introduce one shared parser for the complete `allow_?mixing` command,
   matched case-insensitively and end-to-end. Subtype detection and class
   command consumption must use the same result. Command-shaped input with
   trailing tokens must remain invalid rather than becoming class evidence or
   an ignored metadata line.
2. Parse package identity and display separately. For `package NAME as CODE`,
   store `CODE` as the canonical group identity and `NAME` as display. Require
   the complete package command grammar, including the opening brace and line
   end, before entering a package.
3. Give package and leaf identities one canonical path representation derived
   from parent identity plus local code. Maintain an index from that canonical
   path to the existing group or entity. Package membership remains layout
   data, not the authority for identity lookup.
4. Route declarations and relationship endpoints through one resolver with
   the relevant Java `reuseExistingChild` behavior. Qualified external
   references must resolve from the root when their first group exists;
   unqualified references may reuse a uniquely existing child as Java does.
5. Preserve first materialization ownership. Reusing an existing entity keeps
   its source location and kind. A later declaration updates only fields that
   the corresponding Java command updates, including display and supported
   decoration, and records an explicit code without replacing the identity.
6. Put the named Java class and method beside every new parity-affecting
   recognizer or branch. No branch may depend on held-out names, line numbers,
   entity counts, relationship counts, coordinates, or fixture paths.

## Passing controls to preserve

The rejection does not invalidate the behavior that generalized:

- All 14 prior review-2/review-3 inputs pass scoped identity.
- Fresh lowercase description-only `allowmixing` inputs pass class dispatch.
- Top-level relationships before declarations preserve first-use source lines
  and later explicit codes across reordered, reversed, and repeated links.
- Multiple top-level implicit chains, including a never-declared endpoint,
  preserve Java identity and source ownership.
- The valid mixed-leaf matrix passes scoped identity across twelve leaf kinds.
- Ordinary class, component, sequence, and usecase controls without
  `allowmixing` retain their prior factory selection.

These controls show that the existing top-level first-use behavior should be
retained while command grammar and hierarchical lookup are replaced.

## Perturbation axes

The next checker should generate fresh valid inputs spanning independent axes:

- directive case crossed with optional underscore;
- valid exact directive crossed with invalid trailing tokens;
- quoted package display crossed with explicit package code;
- package or namespace depth of one, two, and at least three levels;
- declarations before links crossed with links before declarations;
- qualified references from inside and outside the owning group;
- renamed displays with stable codes;
- repeated and reversed relationships;
- multiple packages containing the same short leaf name;
- default and custom namespace separators;
- relationship-created leaves later declared as different description kinds;
- aliased and unaliased package controls.

No production branch should mention values chosen for these perturbations.

## Acceptance

A replacement implementation is not accepted until an independent checker:

1. preserves all 14 review-4 source/SVG pairs, including the four direct
   failures;
2. replays all 14 review-2/review-3 inputs and all 14 review-4 inputs with
   28/28 scoped identity matches;
3. verifies exact valid and invalid `allowmixing` grammar against Java;
4. verifies package code/display separation and nested qualified lookup at
   more than one depth;
5. verifies source location and leaf type remain owned by first
   materialization;
6. finds no fixture-shaped logic, golden reads, unexplained parity branches,
   or unproven constants; and
7. records the fresh perturbations and verdict under the mechanism's normal
   adversarial review gate.

Strict SVG mismatches may be classified as renderer-owned only after the
parser-owned factory, canonical qualified identity, source line, and
first-created leaf type all match.
