# Allowmixing description-leaf account

## Rejected boundary

Review commit `ef6faef6` showed that the renderer predicate introduced by
`1166e7e3` is correct for every `EntityKind` RustUML currently represents, but
the parser does not represent all valid `allowmixing` leaves. Actor, use-case,
component, database, queue, node, and rectangle declarations are either
discarded and reconstructed from a later relationship or misclassified as
open package containers. Their resulting `source_line` and qualified identity
therefore belong to the wrong syntax event.

## Java mechanism

At PlantUML revision `71806a2`, `CommandCreateElementFull2` creates these
commands as typed leaves. `GeneralImageBuilder` dispatches the corresponding
leaf types to `EntityImageDescription`; that image constructs its `UGroup`
with the entity `LineLocation`, so `DATA_SOURCE_LINE` is the declaration line.

The overlapping database, node, and rectangle keywords have two different
grammar roles:

- a declaration without an opened body creates a description leaf;
- a declaration that opens a body creates a group/container.

Relationship parsing never owns the source location of an explicitly declared
leaf. `ensure_entity` is only the fallback for a genuinely implicit endpoint.

## Rust invariant

The class parser must recognize valid `allowmixing` description declarations
before relationship fallback. It must retain a typed image contract and the
declaration's source line. Overlapping package keywords may enter
`package_stack` only when the syntax opens a container body. The class renderer
then derives source metadata from the represented image contract:

- class/object/description contracts carry the declaration line;
- state/branch contracts omit it.

No renderer branch may recover declaration ownership from fixture names,
relationship lines, package positions, or output coordinates.

## Acceptance

1. The six `review2_` matrices retain valid Java SVGs.
2. All entity-group qualified names and `data-source-line` values match Java.
3. The four previously passing core matrices remain exact at the metadata
   layer.
4. The description and deployment-description matrices no longer synthesize
   ownership from relationships or phantom containers.
5. Parser tests perturb labels, declaration order, relationship order, and the
   leaf-versus-container brace boundary.
