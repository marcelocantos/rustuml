# Independent checker evidence

This packet rejects the candidate. `../review.json` records exact revisions,
commands, the complete per-case verdicts, source audit and repair dependencies.
The executable source/reference pairs are in
`test-diagrams/perturbations/tim-theme-post-expansion-source-identity/`.

There are 24 fresh checker inputs and nine preserved maker inputs. The latter
comprise all eight fresh maker cases plus its existing-corpus diagnostic.
One fresh checker input unexpectedly produced a Java error page; it remains a
failing executable pair. No input was discarded after seeing its result.

The shipped binaries were freshly compiled in this isolated checkout at the
recorded baseline and candidate revisions, with their own Cargo target under
`/tmp/rustuml-tim-checker/target` and `CARGO_BUILD_JOBS=4`. Every isolated render
used a new CLI process. `probe.rs` invokes the unchanged strict comparator;
its generic expansion mode is diagnostic only. No production adapter or test
harness was changed.

`cases/*/before.svg` and `after.svg` are actual CLI output. `*.diff.txt` files
are comparator output. `*.uml-source` contains Java's actual `Diagram` source
and the CLI's `--yaml` metadata source; `*.source.diff.txt` compares those exact
strings. These differ from generic `*.expand` diagnostics for the sequence
continuation case, whose CLI identity is correct despite a failing SVG.

Java SVGs were generated using the pinned JAR in `-pipe -tsvg` mode from the
repository root. Maker `fresh-java.svg` files are strict-equivalent to their
byte-for-byte preserved references. Initial Java `-preproc` file-mode captures
for the include cases had a different resolution context; those captures are
retained as `file-context.expanded`. The authoritative `java.expanded` for
these cases was regenerated with `-pipe -preproc` from the repository root.
`SourceProbe.java` independently reads the actual Java `UmlSource` for all
cases. Rust metadata came from each built CLI, using YAML node inspection
without interpreting application tags.

`before-isolated-ledger.json` and `after-isolated-ledger.json` enumerate every
preserved corpus case, process status, comparator status, stderr and comparison
digest. Their denominators are 874 and 907. The original 810, class 23, sequence
41 and this review's 33 pairs are counted separately in the review. The built-in
suite remains red and suffers the documented shared-registry panic cascade;
its collateral failures are not substituted for these isolated CLI results.

The scripts are the executed machine-local reproduction recipes, retaining
absolute reference, maker and scratch paths. Copy them to the recorded scratch
directory before replaying and use the recorded revision/environment. They are
not a new standing gate. The existing `parity_perturbations` suite discovers all
new pairs automatically. Keep intentional source-significant whitespace in
the expansion captures.

Parser tests passed 564/564 and CLI tests 13/13. Both main corpus tiers and
guards were run before/after; guards were run again after authoring the rejected
review. Final guards remain 4/6, including rejection of non-accepted evidence.
No baseline, comparator, exclusion or previous evidence was changed.
