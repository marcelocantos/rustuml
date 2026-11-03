# TIM corrective maker packet

This continues maker `d2d571de0ca551cfd93223d577539eff7c366afe` after independent
rejection `5fc923d08d817d415000d6dc14bd66657f894c65`. It is not an independent
review or acceptance. All four reviews and their failed inputs remain unchanged.
Corrective accounts were committed before code in
`67e53172fa470cdf2cec61c5aa0323dad11ba224`; implementation is
`c1730d656949364dd12f0fa3bcc69b1dc04b1941`.

The statement-scoped event sink preserves function effects through return-value,
argument, selector and diagnostic evaluation. Textual evaluation proceeds left
to right and does not re-evaluate replacements. Inline procedure prefix/suffix
handling retains every event variant. A following statement keeps its source
origin when expression evaluation has emitted events.

Separate corrective accounts cover declaration-time default values,
quote-aware expression tokens and date pattern runs. Defaults capture evaluated
values before later global mutations or sibling calls. Name expressions use
quoted tokens, concatenation, balanced parentheses and byte-safe Unicode
boundaries. Java's quoted tokenizer treats backslashes literally, rather than
as C-style quote escapes; opposite quote forms can contain each other.
Date formatting now handles token-run widths, quoted/doubled apostrophes,
numeric zone formats and the tested English calendar fields.

## Measurements

| Set | Result at the committed implementation |
| --- | --- |
| Original prepared 907, isolated CLI, root cwd | 394 matches, unchanged from prior maker; no new CLI errors or lost passes |
| Expanded 949 = 907 + checker 42 | 425 matches versus 418 before this continuation; 8 gains, 1 lost SVG pass; 512 SVG mismatches, 9 Java error SVGs, 1 empty Java reference, 2 existing panics |
| Checker additions | 40/40 valid exact Java source matches; 31/40 valid SVG matches; 2 deliberately invalid controls retained separately |
| Fresh maker matrix | 14/14 valid exact-source and SVG matches; 2 deliberately invalid patterns retained separately |
| Previous maker 22 | 20 valid exact-source matches, 19 SVG matches; 2 invalid inputs remain |
| Original preserved theme 33 | 33 exact-source matches including the invalid case; 28 SVG matches, 4 valid mismatches, 1 invalid reference |
| Supplemental date zones | 12/12 live IANA-zone controls and 3/3 fixed-zone controls pass shipped SVG comparison |
| Parser units / CLI pipe tests | 575 pass, 1 unchanged parser assertion fails / 13 pass |
| Main product / assisted | 11179/11251 / 10497 pass, 754 fail, 1299 skipped; unchanged from starting maker; both red |
| Guards | 4/6; decimal and historical review completeness remain red |
| Built-in expanded 949 | 424 matches; only 2 genuine sequence panics, no registry cascade |

The parent has 955 inputs because it also has six registry-cleanup controls.
This checkout has 949, not 955. Accepted registry account/implementation
`c6ba01b4453825dd6ec85f849964716ad5fcd961` and
`43f6e7b82c52c6b4ce989926e33f84eec84bcc38` were cherry-picked as validation
prerequisites, not claimed as this maker's mechanism. Their local commits are
recorded in `maker-results-summary.json`.

The built-in package-cwd replay has one fewer match than the isolated root-cwd
replay: the existing `checker_include_theme` path dependence (T14.6.2). This is
not reported as a geometry regression. The remaining genuine panics are
`sequence-grouped-note-origin/checker_live_stack_2` and `_3`.

The lost SVG pass is `checker_selector_nested_theme_event`: the first theme
previously disappeared. It now reaches both identity and parser; Java source
matches exactly, while the class stereotype spot retains the first palette
instead of the later palette. This remains a failed counterexample, owned by
the class worker. The other palette cases remain failed. Additional existing
rendering dependencies are the empty themed class canvas in
`checker_comment_closure_tail`, standalone note styling/placement in
`checker_loop_sprite_comment`, activity diamond text styling/geometry,
handwritten shapes, and sequence continued-line geometry. No reference or
comparator was changed to waive them.

All five previously passing memory controls remain passing. Both lookup grammar
counterexamples, declaration default capture, diagnostic side effects, return
function theme execution and all three valid date-format counterexamples now
match exact Java source and shipped SVG. Nested-selector identity now matches,
with the palette failure stated above.

## Two pending interfaces

**Reader policy (T14.6.3): no owner answer has been received.** The historical
picoweb GET route runs `DiagramSourceCompressor.decompress`, trimming outer body
blanks before TIM. Raw pipe/file/POST retain them. That explains the three main
reference differences and the existing parser assertion. Current candidate
behavior is retained pending the decision; reader normalization, the assertion,
references and baselines were not changed. This evidence does not choose raw
or GET compatibility and does not claim the red main tier is acceptable.

**Date error propagation requires the parse owner.** Owned preprocessing now
returns a typed `PreprocessOutput.error: Option<PreprocessError>`, containing
`line` and `message`, and stops further source emission on invalid format or
signed-second overflow. No synthetic identity line, parser command, SVG patch,
or panic carries the error. The explicit diagnostic probe confirms all four
invalid controls produce an error.

The existing `parse_with_base` ignores this new field. The precise handoff is
to check it immediately after the initial `preprocess_full_for_parse` call,
before style extraction/type detection, and again after the sequence-specific
retry. For each check, the parse owner can use:

```rust
if let Some(error) = preprocess_out.error.take() {
    return Err(ParseError {
        line: error.line,
        message: error.message,
    });
}
```

The CLI already reports `ParseError`. No CLI change is needed for that route.
Legacy convenience APIs returning only `Vec<String>` also cannot communicate
errors without a checked API contract. None of those unowned callers was
changed here. **The supplied invalid checker cases still exit CLI 0 and remain
failures.** The empty overflow reference stays empty; no error-policy change
or fabricated SVG was made. The typed probe is supplementary evidence and is
not substituted for shipped-path rejection.

## Evidence and replay

Each corrective account directory contains every fresh source and all retained
Java/Rust phase outputs in `maker-cases/`, plus `maker-results.json`. The shared
summary records hashes, exact counts, prerequisites and unresolved dependencies.
`inventory-949.json` hashes the complete expanded source/reference set and labels
the original 907 separately. `isolated-changes.json` records all eight gains and
the one regression, including exit status.

`measurement-artifacts.tar.gz` preserves the baseline, intermediate and final
CLI/source outputs for all measured inputs, including temporary regressions,
the main preprocessing investigation, and full build/test logs. Two initial
main-preprocessing diagnostic attempts raced while writing a shared Java-source
status cache and failed in the measurement script; their logs remain. Completed
replays used SVG comparison only for that investigation. These attempts are
not product outcomes. No frozen input/reference was written.

Scripts in `tools/` write to `/tmp/rustuml-tim-correction`. Run from repository
cwd. The inherited `probe.rs` uses unchanged `compare_svg_strict` and the
existing Java-error detection. `SourceProbe.java` calls actual Java
`Diagram.getSource().getPlainString("\n")`; source diagnostics use shipped CLI
`--yaml` output, not `-preproc`. The error probe inspects the typed API and is
explicitly distinct from CLI success. Canonical Java runs specify
`-Duser.timezone=GMT+11:00`; supplemental zone runs record their own environment.

```text
CARGO_TARGET_DIR=/tmp/rustuml-tim-repair-target
CARGO_BUILD_JOBS=4
RAYON_NUM_THREADS=4
PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib
RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100

cargo test --release --no-fail-fast -p rustuml-parser --lib
cargo test --release --no-fail-fast --test pipe_mode --test golden_no_oracle --test golden_pairs --test harness_guards --test parity_perturbations
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
rustfmt --edition 2024 --check crates/rustuml-parser/src/preprocess/mod.rs crates/rustuml-parser/src/preprocess/debug.rs crates/rustuml-parser/src/preprocess/date_format.rs
uv run --with pyyaml /tmp/rustuml-tim-correction/measure.py candidate checker
uv run --with pyyaml /tmp/rustuml-tim-correction/measure.py candidate new
uv run --with pyyaml /tmp/rustuml-tim-correction/measure.py candidate prior
uv run --with pyyaml /tmp/rustuml-tim-correction/measure.py candidate preserved
uv run --with pyyaml /tmp/rustuml-tim-correction/measure.py candidate isolated
uv run --no-project python /tmp/rustuml-tim-correction/timezones.py
uv run --no-project python /tmp/rustuml-tim-correction/fixed_zones.py
```

Restore archived inputs/scripts into a fresh scratch location before replay;
preserve existing scratch evidence instead of overwriting it. Copy the built
CLI as `candidate-rustuml`; supplemental zone scripts use the recorded `fixed`
snapshot, whose date code is identical. Compile the Rust probes with the
matching target's parser/oracle rlibs and `-L dependency=.../release/deps`;
compile `SourceProbe.java` with the pinned JDK and JAR from
`reference-environment.json`. That JAR is source-pinned, not historically
byte-identical; broad historical font/Graphviz equality remains unproven.

Owned Rust formatting passes. Workspace formatting remains red, and clippy
now fails only on pre-existing unowned `parse/component.rs` dead code.
Captured oracle whitespace is deliberately preserved byte-for-byte. English
symbols/week rules match the pinned locale; localization, long display names
for named host zones and the non-Unix fallback are not established by these
controls. No runtime Java dependency was introduced. Fresh independent checker
work is still required for all four corrective accounts.
