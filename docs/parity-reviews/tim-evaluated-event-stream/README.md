# TIM evaluated-stream maker evidence

This is an implementation report, not an independent checker review or an
acceptance verdict. The four accounts were committed before implementation in
`22c023397bc1422d21d24e37e800ed939be3d841`. The measured production revision is
`cefd22f9d047cf4ab1f2bb515aa869266b91aff6`, based on
`eebd74cebc3a3cf4b1dbc0d8b63bd06d163972cd`. The parent must arrange a fresh
adversarial checker for all four mechanisms.

## Implementation and evidence boundary

TIM now produces typed evaluated events once per preprocessing operation.
`project_events` derives the complete identity and parser representation from
those events. Sprite extraction follows projection, so sprite pixels remain
in the identity while the parser receives extracted sprites. Theme bodies and
user subblocks execute against the real memory; identity no longer replays
them. Diagnostics do not add synthetic identity comments. Theme selector and
local `from` expressions execute, and inactive directives retain origins.

Separate accounts cover sibling procedure memory rooted at global memory,
literal/computed names in `%variable_exists`, and explicit `%date` timestamps.
Java's legacy `!define` also populates variable memory: the fresh literal-name
control exposed and corrected an initial misconception about that boundary.
The existing shared-preamble parser extension retains its one-based API
contract; the direct Java reader rejects that maker input, so it is not Java
parity evidence.

This does not claim complete TIM equivalence. In particular, the unowned
sequence dispatcher can invoke preprocessing twice while choosing a parser;
each result now has one evaluated stream, but no cross-call cache was added.
Expression-valued functions and the existing includesub loader retain their
broader architecture. Fresh checker exploration remains necessary.

## Results

| Measurement | Result |
| --- | --- |
| Prepared 907 inputs, isolated shipped CLI processes, repository cwd | 394 SVG passes, 503 mismatches, 8 invalid references, 2 existing panics; baseline 372 passes; 22 gains, zero regressions or new CLI errors |
| All 33 preserved theme inputs | 33 exact Java `UmlSource` matches (including the invalid reference's source), 28 SVG passes, 4 valid SVG mismatches, 1 invalid reference; all CLI invocations exit 0 |
| 22 new maker/control inputs | 20 valid Java inputs: all 20 exact source matches, 19 SVG passes; 2 invalid Java inputs retained; all CLI invocations exit 0 |
| File-base control | Explicit-base preprocessor source matches Java; shipped CLI SVG matches from source cwd; both baseline and candidate lose the base when invoked from repository cwd |
| Parser units | 569 pass, 1 origin-expectation failure |
| CLI pipe tests | 13 pass |
| Main product tier | 11179/11251, versus baseline 11181; red |
| Main assisted tier | 10497 pass, 754 fail, 1299 skipped out of 12550; red |
| Global guards | 4/6; decimal and historical review completeness guards red |
| Built-in perturbation suite | Red; prepared checkout retains same-thread registry cascade |
| Format and clippy | Owned Rust files format cleanly; workspace formatting and pre-existing component dead code remain red |

The three main-tier losses are preserved-reference conflicts:

* `activity/act_skinparam2_shadowing_true` and
  `activity/act_skinparam_shadowing`: initial blank lines affect filter IDs.
* `edge-cases/edge_whitespace_multiple_blank_lines`: initial blanks affect
  source-line attributes (Java 4/8/12, preserved reference 1/5/9).

Fresh pinned Java SVGs match the candidate for all three. Their unchanged
inputs, fresh outputs and preserved-versus-fresh diffs are in `maker-cases/`.
The parser test `leading_blank_lines_do_not_shift_locations_but_internal_blanks_do`
expects line 1 where the candidate and the fresh renamed Java control produce
line 5. These are still red gates, not waivers. Neither references nor baselines
nor the unowned assertion were edited.

Remaining valid preserved theme mismatches are repeated-theme class spot
palettes (`checker_repeated_reverse`, `maker_repeated_themes`, owned by the
class worker), handwritten shapes (`checker_option_true`), and sequence
continuation geometry (`checker_line_continuation`). The new `aws_activity_seed`
has the existing activity diamond text-style/font and resulting geometry
divergence; its baseline and candidate SVGs are structurally equivalent, and
its candidate evaluated source matches Java exactly.

The two isolated CLI panics remain
`sequence-grouped-note-origin/checker_live_stack_2.puml` and
`checker_live_stack_3.puml`. The parent has independently accepted registry
lifetime cleanup at `22cd748c2ea2b5880c1dc5e4f7796c81a950d08b`; it was not
cherry-picked into this prepared checkout. Accepted review
`bc9e9463ef4e21aa238517d9c3726d39c859f8ae` was read. This packet measures 907
sources, not the parent's later 913. The existing `checker_include_theme`
passes from repository cwd; its crate-cwd failure is T14.6.2 path dependence,
not a geometry claim.

The additional file-base control identifies an existing separate boundary:
`crates/rustuml/src/main.rs` calls `parse_block` for delimited inputs and loses
`base_dir`; `parse_auto_with_base` only receives it for inputs without a block.
`file-base/` preserves baseline/candidate wrong-cwd SVGs and exact sources,
same-cwd SVGs, Java outputs, and explicit-base preprocessor source. No CLI or
parse-dispatch code was changed.

## Durable artifacts and reproduction

`maker-results-summary.json` records revisions, binary hashes, counts, test
results and pending dependencies. Each of the four account directories has
its own `maker-results.json` and readable `maker-cases/` containing every new
input and the Java/baseline/candidate SVG, source, diagnostics and statuses.
The invalid `guard_changed_theme` and shared-preamble controls remain present.
The local saffron theme dependency is in `maker-inputs/themes/`.

`measurement-artifacts.tar.gz` contains all new and preserved diagnostic
measurements at every retained phase, all 907 baseline/candidate isolated SVG
outputs and diffs, and full build/test logs. Intermediate names (`stream`,
`after`, `final`) denote diagnostic snapshots; only `candidate` denotes the
final measured production revision. Per-phase JSON results are also readable
beside this file. `preserved-inventory.json` hashes all 907 unchanged source
and reference files. `artifact-manifest.json` hashes this evidence bundle.

The exact measurement programs are in `tools/`. They deliberately write to
the recorded scratch directory, `/tmp/rustuml-tim-repair`, and must be run
from the repository root. They are diagnostics, not modified acceptance
harnesses. `probe.rs` uses the unchanged `compare_svg_strict` and existing
Java-error detection; only its `compare` command is used for SVG measurements.
`SourceProbe.java` obtains `Diagram.getSource().getPlainString("\n")`, including
an explicit base directory when supplied. `--yaml` source is extracted from
the shipped CLI, including its sequence dispatch, rather than from `-preproc`.

To repeat the final measurements, extract the archive into an empty scratch
directory at the recorded path, restore the scripts from `tools/`, and build
the candidate CLI plus the Rust comparator probe. Preserve existing scratch
evidence instead of overwriting it when rerunning. The build environment is:

```text
CARGO_TARGET_DIR=/tmp/rustuml-tim-repair-target
CARGO_BUILD_JOBS=4
RAYON_NUM_THREADS=4
PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib
RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100
```

The commands exercised were:

```text
cargo build --release -p rustuml
cargo test --release -p rustuml-parser --lib
cargo test --release --no-fail-fast --test pipe_mode --test golden_no_oracle --test golden_pairs --test harness_guards --test parity_perturbations
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
rustfmt --edition 2024 --check crates/rustuml-parser/src/preprocess/mod.rs crates/rustuml-parser/src/preprocess/debug.rs
uv run --with pyyaml /tmp/rustuml-tim-repair/measure.py candidate new
uv run --with pyyaml /tmp/rustuml-tim-repair/measure.py candidate preserved
uv run --with pyyaml /tmp/rustuml-tim-repair/measure.py candidate isolated
```

Copy the built CLI to scratch as `candidate-rustuml`. Compile `probe.rs` with
`rustc --edition=2024`, `-L dependency=<target>/release/deps`, and `--extern`
arguments pointing to the parser and oracle release rlibs from that target.
Compile `SourceProbe.java` with the absolute pinned JDK's `javac` and the
reference JAR on its classpath. `file_base.py` records the original positional
CLI experiment; the additional explicit-base Rust probe is `base_probe.rs`.
Its same-cwd follow-up results and outputs are preserved in `file-base/` and
`file-base-results.json`.

Use the absolute Java executable in `reference-environment.json`. The JAR is
source-pinned, not historically byte-identical; historical font and Graphviz
equivalence remains unproven. There is no runtime Java dependency in RustUML.
