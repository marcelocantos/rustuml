# Independent registry lifetime review

This checker assesses only T14.6.1. It does not repair or attest sequence
geometry. `review.json` gives the verdict; `results.json` preserves each old
corpus outcome and the complete focused lifecycle results. All six fresh
source/Java pairs are in `test-diagrams/perturbations/render-registry-unwind-scope/`
and are picked up by the unchanged standing perturbation test.

The original two valid `sequence-grouped-note-origin/checker_live_stack_{2,3}`
inputs remain at their original paths. They must still produce their sequence
index panic. Each is followed, on the **same thread**, by six successful
controls, then repeated once more. Controls span sequence/activity, changed
labels and topology, ordinary text, multiple background colors, and color
alias deduplication. Every recovered SVG must equal a fresh CLI result byte
for byte and pass the existing strict Java comparator. Separate registry
checks cover normal ownership transfer, a populated body panic with a
non-string payload checked by pointer identity, repeated caught nesting,
and uncaught nesting.

## Reproduce

Use this checkout, its real pinned golden submodule, and a private external
scratch directory. Do not write baselines or change filters. Set
`PLANTUML_STDLIB` and `RUSTUML_DEBUG` as documented in
[`../../reference-bootstrap.md`](../../reference-bootstrap.md).
The example scratch directory is specific to this review:

```sh
export CARGO_TARGET_DIR=/tmp/rustuml-registry-checker-bd87/target
export CARGO_BUILD_JOBS=4
export PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib
export RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100
cargo build --release -p rustuml -p rustuml-oracle --message-format=json \
  > /tmp/rustuml-registry-checker-bd87/build.jsonl
uv run --no-project docs/parity-reviews/render-registry-unwind-scope/checker.py \
  compile /tmp/rustuml-registry-checker-bd87/build.jsonl \
  /tmp/rustuml-registry-checker-bd87/checker
/tmp/rustuml-registry-checker-bd87/checker recovery . \
  "$CARGO_TARGET_DIR/release/rustuml"
/tmp/rustuml-registry-checker-bd87/checker corpus \
  test-diagrams/perturbations "$CARGO_TARGET_DIR/release/rustuml"
cargo test --release -p rustuml-render --lib -- --test-threads=4
cargo test --release -p rustuml-oracle --test golden_no_oracle \
  --test golden_pairs --test parity_perturbations --test harness_guards \
  --no-fail-fast -- --nocapture
```

The corpus command intentionally fails while any preserved failure remains.
It records invalid Java references as failures, not successful comparisons.
For every valid reference it runs both the in-process renderer and an actual
CLI child. The oracle dependency enables oracle-layout APIs at compile time;
neither this client nor the CLI calls those APIs or extracts oracle layout.
Only `render_svg` and the default CLI source-to-SVG route produce the measured
output. `compare_svg_strict` supplies the unchanged comparison policy.

The generator is retained for provenance. It invokes the absolute pinned
Java executable and JAR from the reference JSON. It refuses to overwrite an
existing source or SVG; **do not regenerate the preserved references**.

## Scope and residuals

This is the account's explicit journey exception for an in-process library
lifetime: a subprocess-only test cannot observe a leaked thread-local. Real
CLI executions complement the same-process client; there is no live provider
or service in RustUML. Maker unit tests are already wired into the library
test gate, and the six new Java pairs into the existing perturbation gate.
The external checker is additional adversarial evidence, not a replacement
for those standing gates.

No destructor guarantee is claimed for `panic=abort`, process termination,
allocation abort, unsafe access to private state, or a user-supplied destructor
that itself panics during another unwind. The examined registry destructor
contains no callbacks, custom element destructors, or allocations. Its slot
cannot be borrowed by a client across the body: the only stateful client API
returns an owned `String`, and its `RefMut` ends before the call returns.

Global failures remain executable and unwaived. Acceptance here applies
only to releasing a registry after unwinding, not to T14, the campaign, or
any other sequence mechanism.

The built-in suite's working directory is `crates/rustuml-oracle`, whereas
the independent replay and CLI run from the repository root. The existing
`tim-theme-post-expansion-source-identity/checker_include_theme.puml` uses a
relative `docs/...` include and differs between those directories. The
targeted crate-directory replay preserves that discrepancy in `results.json`:
the built-in suite has 377/913 passes; the root-directory replay has 378/913.
Both retain the two initiating panics and all other failures. No path or
fixture was changed to reconcile them.
