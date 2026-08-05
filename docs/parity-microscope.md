# Parity Microscope

The parity microscope reports the earliest divergence visible through its
declared projections between the pinned Java PlantUML oracle and RustUML. It
does not certify causality or complete DOT equivalence. It is diagnostic
evidence, not a completion metric: T14 still closes only on the ratcheted
no-oracle product suite and its standing guards.

## Trace One Case

```bash
cargo run -p rustuml-oracle --example parity_trace -- \
  path/to/case.puml --out target/parity-trace/case
```

The trace captures the Rust semantic model, each exact Rust Graphviz request
and solved graph, Java's exact SVEK Graphviz request and raw Graphviz SVG, both
final SVGs, canonical request projections, and `summary.json`. Generated node
IDs, equivalent box spellings, and effective Graphviz defaults are normalized.
The projection compares graph direction/spacing/routing, node order and
dimensions, edge topology/constraints, label-box dimensions, and cluster count.
It does not prove complete DOT equivalence; the raw requests remain the source
for fields and cluster membership outside that projection.

Java capture uses PlantUML's dormant `-debugsvek` seam. The tool compiles one
class from the pinned Java commit into a temporary classpath overlay; it never
changes the PlantUML checkout or JAR. It refuses any Java revision or JAR other
than the repository's pinned oracle. Reports include the SHA-256 digest of the
executed JAR bytes as well as the Java source revision used for the overlay.

## Reduce A Divergence

```bash
cargo run -p rustuml-oracle --example parity_reduce -- \
  path/to/case.puml --out target/parity-reduce/case
```

The reducer removes lines with delta debugging while protecting `@start` and
`@end` framing. A candidate is accepted only when:

1. RustUML parses and renders it.
2. The traced PlantUML run succeeds and is not an error SVG according to the
   frozen central syntax-error classifier.
3. The complete disagreement set in the canonical layout-request projection is
   unchanged.
4. The first strict final-SVG divergence shape is unchanged.

The output contains the original and reduced valid sources, paired final SVGs,
raw Java/Rust layout requests and solved artifacts, and a report with pinned
Java provenance, both divergence signatures, and probe counts. Reduction is
refused unless the request projection exposes a concrete model difference.
The result is one-minimal by source line under those predicates; it is a
compact lead to investigate, not proof of complete mechanism identity and not
permission to tune output constants.
