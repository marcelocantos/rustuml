# TIM corrective maker packet (checker3 continuation)

This is implementation evidence, not an independent review or acceptance record.
All prior rejected claims retain their verdicts. Fresh reference capture is
**incomplete**: Monkey Man SSH timed out, and the owner explicitly prohibited
local Java fallback. The parent confirmed the address remains 192.168.1.112 and
has also confirmed reverse SSH is blocked and is arranging checksummed byte relay
over the working T3 channel. `remote-request.json` inventories the immutable batch. No Java or javac ran during this continuation.

Production candidate: `6c550246b458eb5f37539713510dc7d919843421`.
Evidence import: `7bb210e9f7578705638c8f3b7c3b43d8b4b906c3` (cherry-pick of
checker `463fc9bd`). The root's parser handoffs and raw-reader assertion remain.

Precommitted corrective accounts:

- `8765ccc2ea4c2574fe90d25c2ba9921ad520682e`: typed builtin arguments, operand
  snapshots, literal substitution boundaries, return unwinding, sub directives.
- `3020a0aaffba609cf48859ba3d65d21e7000a082`: JSON foreach values and global bindings.
- `56a954287757fd23000d756b2fb483d43d7dcda2`: registered builtin recognition and
  filtered environment lookup.

The implementation prepares expression operands before executing calls, preserving
Java's variable snapshots and eager operand order. Values cross argument/default/
memory boundaries without display-and-reparse conversion. Builtins use their own
integer/string contracts; substring and indexing retain UTF-16 code units,
including split surrogate pairs. Loop returns unwind without subsequent effects.
Procedures cannot be used as expression values. Legacy defines capture expansion
at declaration, and substituted values are not rescanned as source. JSON paths and
foreach bindings follow the Java value and memory model. External sub selectors
expand once, accept directive whitespace, and validate raw markers before executing
only selected lines. Unknown expression calls and invalid substring bounds preserve
the first fatal diagnostic.

Making missing builtins fatal exposed a transient valid-input regression in
`checker_outline_state`: the bundled theme calls `%getenv`. The final implementation
adds the actual filtered lookup contract and distinguishes registered plain-text
calls from unknown expression calls. Iteration3's lost pass and stderr are retained;
the final replay restores it. The signed arithmetic and timezone-width modules from
the prior candidate were not changed.

## Measured scope

Every number below uses existing Java references and the unchanged strict SVG
comparator. The headline path is the shipped CLI. No baselines or references moved.

| Cohort present in this checkout | SVG matches | Invalid references | New gains / lost passes |
| --- | ---: | ---: | ---: |
| 1,038 of checker's frozen 1,449 | 489 | 32 | 0 / 0 |
| Fresh checker3 42 | 32 | 10 | 14 / 0 |
| Retained maker/root controls 21 | 13 | 7 | 0 / 0 |
| Supplement controls 4 | 2 | 2 | 0 / 0 |
| Combined local 1,105 | 536 | 51 | 14 / 0 |

There are no new valid CLI errors. The frozen cohort retains two sequence panics.
The three new CLI rejections are preserved invalid procedure-as-expression,
substring-only, and unknown-builtin-only controls; these are error-status evidence,
not SVG successes. Three invalid/empty Java controls remain accepted locally:
`checker3_shift_scope`, `checker3_unary_variable`, and
`checker3_no_return_extension`. They are not positive parity evidence.

This is **not** the parent's 1,516 scope (frozen1,449 +67). Missing411 are recorded
in `frozen-missing.json`. The historical955 subset is only949 here (425 matches),
and historical1044 only1038 (489 matches); six cleanup controls are absent.
`deltas.json` labels these measured subsets explicitly. All prepared907 inputs are
within the replay. No absent input is counted as passing.

All 32 valid fresh cases match exact Java UmlSource and SVG, including the previously
lost `substr_json` pass. Retained controls have14 exact valid source agreements;
one SVG mismatch remains in the class renderer despite exact source. Supplement
named-zone controls both pass with their declared matching environments. Original 33
source comparisons remain33/33; prior 42 retain40 valid matches and two invalid date
rejections. Six raw-reader file-mode controls and all five class/TIM handoff controls
pass. The selected second-block error still exits1 with empty stdout.

Final validation (`validation.json`, full logs retained):

- Parser590/590; CLI15/15; owned rustfmt and diff whitespace pass.
- Main product11179/11251, assisted10497/11251 (754 failures,1299 skips), unchanged.
  The three old GET/raw reference losses remain; approved raw behavior is preserved.
- Guards4/6. Existing decimal/review-evidence failures remain, including missing
  historical account metadata. No historical records were rewritten.
- Clippy fails only on unowned `parse/component.rs:72` dead code.
- Built-in1105 suite with the fixed gate environment:513 matches, two sequence
  panics, overall failure. Its23 extra failures relative to isolated CLI runs are
  named in `builtin-gate-versus-isolated.json`: cwd-dependent includes/themes and
  named zones run under the suite's fixed offset. This is not a registry cascade.
  The initial run omitted the fixed debug setting and produced499 matches; that
  run is preserved too. Only the built-in suite was repeated to correct the
  environment; the main tiers were run once.

## Pending fresh references

`inputs/` preserves all 17 new maker inputs and both include files byte-for-byte.
They exercise independent axes for each correction: typed/JSON arguments and UTF-16;
variable snapshots inside nested calls and JSON access; nested while/foreach returns;
define declaration order and effect capture; JSON global binding and object order;
dynamic/local sub selectors and invalid unselected structure; builtin text versus
expression registration and permitted/denied environment names.

`pending-reference-rust/` preserves the actual CLI SVG/YAML/stderr. Fourteen positive
inputs render and three intended negative controls exit1. Those statuses are **not**
Java comparisons. No SVG reference or source diagnostic has been fabricated for them.
The exact originals remain staged at `/tmp/rustuml-tim4/new/`.

After the parent supplies recovered transport, stage `inputs/` and the unchanged
`tim-corrective-execution-maker/SourceProbe.java` within the worker's own remote
`/Users/marcelo/rustuml-reference/scratch/worker.Bab5N6zr/tim4/` hierarchy. Use only
the pinned remote wrapper, preserving input bytes and include-relative cwd:

```
reference --cwd <private>/tim4/inputs --timezone GMT+11:00 -- -pipe -tsvg
reference --cwd <private>/tim4/inputs --timezone GMT+11:00 --probe <private>/tim4/SourceProbe.java --classpath <private>/tim4 -- <private>/tim4/inputs/<case>.puml <private>/tim4/inputs
```

Collect stdout, stderr and status separately for both commands. Wrapper runtime is
OpenJDK21.0.5/Graphviz12.2.0, exact campaign JAR, headless AWT. These differ from the
origin runtime; seven transport controls establish scoped agreement only. No raw
trimming, GET substitution or fixture rewriting is permitted. Each new reference
must carry that environment provenance. Existing references need no rerun.

## Reproduction and preserved artifacts

From the checkout root, build with
`CARGO_TARGET_DIR=/tmp/rustuml-tim-repair-target CARGO_BUILD_JOBS=4 cargo build --release -p rustuml -p rustuml-oracle`.
Compile the unchanged checker3 `probe.rs` against that target's release
`rustuml_oracle` rlib. Put the candidate binary at `<scratch>/replay-rustuml` and the
probe at `<scratch>/probe`. Extract the original checker3 measurement archive's
`reference/` directory into scratch; this reuses captured Java source diagnostics.
Run `TIM_MAKER_SCRATCH=<scratch> python3 <this-directory>/measure.py replay frozen`
(and `fresh`, `retained`, `supplement`). It runs only local CLI/comparator processes.
Run from the checkout root and use a fresh scratch directory to retain prior outputs.
PyYAML is required for supplemental source extraction.

`measurement-artifacts.tar.gz` contains all intermediate and final measured output,
including failures. `logs/`, `deltas.json`, `error-statuses.json`, and the per-case
result files record commands, statuses, losses and errors. `provenance.json` records
binary/toolchain hashes and runtime scope. `preservation.json` verifies all 5,674
pre-existing tracked evidence/fixture files against the checker-evidence import;
no historical bytes changed. The golden gitlink remains pinned.

The 48 valid checker3 diagnostic pairs with preserved before/after YAML have no
entity-count drops (`model-observations.json`). This is a scoped model check, not
a claim about unmeasured models in the broader frozen cohort.

## Residue and handoff

No acceptance claim is made. A new independent checker remains required after
fresh remote references are available. No review or attestation was authored.

The exact-source retained sub case still exposes the class renderer's replacement
of source_line0 with a fallback, plus entity order. Existing class alias/style,
repeated palette, handwritten/activity and sequence continued-line dependencies
remain unowned. Combined class/sequence integration is the parent's responsibility.
General JVM system-property overrides have no Rust configuration equivalent for
`%getenv`; only native separator properties and permitted process environment are
implemented. Isolated-surrogate final text projection remains unverified; internal
UTF-16 round trips have unit coverage and a pending fresh Java control. Existing
Rust floating/unary/no-return extensions are not claimed as Java parity.
