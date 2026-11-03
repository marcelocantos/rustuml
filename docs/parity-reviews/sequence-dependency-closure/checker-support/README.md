# Independent sequence recovery review

**Rejected.** No mechanism receives whole-source acceptance in this review.
The candidate repairs substantial behavior, including both valid-input parent
panics, but leaves known failures and independently generated counterexamples.
The fourteen new checker records distinguish supported local corrections from
unaccepted dependencies. None changes a historical verdict or acceptance rule.

Reviewer: independent task `t3-b60455c3`, GPT-6 Astra, high reasoning effort,
Codex harness in T3 Code. No children were used. The maker checkout was read-only.

The tested parent is `29642ec630e9902c892162a7081cd8a5e1113dac`. All 26 commits
from `9ef2acc9c72e614bfe4c8ab06147aab8fa80c45b` through
`1800ac48714743863307c82674490c697fcf5709` were cherry-picked in original order,
producing tested candidate `5d3eb9824df6084411c591f90d5b385aac32c1b7`.
[The mapping](evidence/commit-mapping.json) records every original/prepared pair.
Registry commits were already in the parent and were not duplicated. No class
or TIM production was imported. Checker changes after that head are evidence only.

The source-pinned Java revision is `71806a23780b04a5ccde2f8ceb5121edad5eb711`;
JAR SHA256 is `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Both were checked locally. See [reference environment](evidence/reference.json).
This rebuilt JAR is not historically byte-identical and historical fonts remain
unproven. New pairs use the real absolute Temurin 21 executable and raw CLI pipe
input. The owner approved raw CLI/file semantics during review, recorded in
policy commit `b62d8ce967b6dda7d85bb6dc28ff92509f6066f9`. Historical GET sources,
references, eligibility and counts remain unchanged. TIM owns independent
raw-reader verification; it was not duplicated here.

## Observations

- Historical release library tiers, before and after: **11181/11251** product,
  **10497/11251** assisted. Sequence is **1411/1411** in both tiers. These are
  the repository library harnesses, not 11251 fresh CLI subprocesses.
- Real shipped CLI, frozen parent **955**: **379 to 403 matches**, zero
  passing-input regressions; after has 542 mismatches and 10 invalid references.
  The two valid-input renderer panics disappear. Its original **810** subset
  improves **349 to 351**, with 452 remaining mismatches and seven invalid references.
- All **535** maker cases reproduce **513 matches / 22 mismatches**. Our parent
  baseline is 133 matches, 323 mismatches and 79 CLI panics; this differs from
  the maker's staged mechanism baselines. No previously passing case regresses.
- **146** independent checker pair entries: **54 matches / 90 mismatches /
  two intentional invalid controls**, zero passing-input regressions or candidate
  panics. There are **130 distinct source byte sequences**: the three Teoz
  mechanism matrices deliberately share eight programs testing their dependencies.
  The valid-Java denominator is 144, not 146. All valid cases have identical
  CLI and compared product-library SVGs after trimming trailing whitespace.
- The original 41 checker cases reproduce **39 matches**, with both font controls
  still failed. All 22 known valid Teoz maker failures were copied byte-for-byte
  into standing perturbations and are explicitly marked as maker evidence.
- Units: **562 parser + 781 renderer passed**. Touched production rustfmt and
  `git diff --check` pass. Clippy fails on the existing unused function
  `parse/component.rs:72`. Guards remain **4 passed / 2 failed**, with frozen
  decimal and historical/rejected review evidence failures. No sequence decimal
  ratchet increase was reported. Both main suites intentionally remain red:
  unlocked historical improvements and assisted mismatches were not waived.

The parent 955 is frozen as 810 original + 23 class + 41 sequence + 33 theme +
six registry + 42 TIM entries. The later parent class review adds 237 unrelated
entries (parent now 1192); those are not in this checkout or these denominators.
The integrator owns the combined replay. This checkout's final standing corpus
is **1123 = 955 + 146 checker pairs + 22 retained maker failures**.
The final unchanged built-in suite reports **456 matches / 667 nonpasses** at
package cwd: 655 valid mismatches and 12 invalid references (11 error pages plus
one empty reference), with zero renderer panics. New review records have no
schema errors; rejection and failed perturbations correctly keep the guard red.

Nine parent invalid references trigger the existing error classifier. The tenth,
the overflow-date control, is empty: historical `diff_one` panics while parsing
the reference. The CLI adapter classifies it as an invalid reference, retains it
in the denominator, and never calls it a match. This changes no comparator rule.
Two fresh orphan returns produce Java error SVGs but successful Rust SVGs;
they remain negative evidence in the standing suite.

Standalone comparisons run at repository root. Cargo runs the library suites
at `crates/rustuml-oracle`. The same candidate's `checker_include_theme` passes
at the former and fails at the latter, as it did before. Three include controls
were rerun at package cwd to isolate that dependency. Sequence controls contain
no includes. The final built-in result is therefore one match lower than the
root-CLI cohort. No directory-sensitive discrepancy was hidden by a new denominator.

## Source findings and bounded repairs

[Source audit](source-audit.json) records the changed branch/numeric-expression
inventory, Java methods, duplication, provenance and limits. All fourteen
accounts precede code and are unchanged. The new named margins 5 and 15 and
Teoz endpoint depth branches have Java provenance. No new golden replay,
fixture-label discrimination or post-hoc SVG surgery was found. Some changed
composite offsets lack adjacent derivations; unchanged historical heuristics
are explicitly not blessed by this review.

1. **Return parsing/lowering:** parse the optional color as configuration, not
   label text. Java `CommandReturn#executeArg` applies `withColor`. Fresh colored
   self and internal returns fail in both engines. Orphan returns need explicit
   invalid-input behavior, not silently successful SVGs.
2. **Self-arrow direction and components:** `SequenceSelfArrow` must represent
   reverse start/ConstraintBefore as well as forward geometry. A new reversed
   legacy control is 213px wide in Java versus 170px in Rust. Derive attached-note
   vertical centering from `ArrowAndNoteBox` preferred dimensions; padding3
   currently leaves a self note 6px too low.
3. **Shared note and participant dimensions:** propagate the head's full preferred
   width through spanning-note allocation, group membership and paint. Simple
   shadowed groups pass, but adding a spanning note changes a fresh group width
   from Java187.9014 to Rust186.9014 and moves participants by1px. Eliminate the
   inconsistent component reconstructions rather than compensating coordinates.
4. **Page composition:** complete the vertical TextBlock hierarchy and multiline
   captions. Legacy title + legend + caption is Java231px versus Rust212px;
   the caption is placed relative to undecorated tail boxes. Teoz retains a
   separate midpoint heuristic and disagrees on newpage behavior.
5. **Teoz lifecycle/external tiles:** port event-history modes and staircase
   callbacks coherently. The shared legacy combined row adds 30px where Java
   `LifeEventTile` has zero preferred height; future flags, notes resetting
   ownership, and staircase reopening cannot be represented by paired variations.
   Teoz external arrows also retain origin/constraint failures. All 22 maker
   failures and the new history controls remain executable.

The raw-member winner, ordinary return endpoints, right-side one-unit ASYNC
stroke correction, and legacy attached-flag aggregation each have positive
source and full-CLI observations. Their failed whole-source dependencies prevent
independent acceptance here; these observations are not fourteen equivalent
claims that every local formula is wrong.

## Reproduction and evidence

All runtime captures went to `/tmp/rustuml-seq-check-b60455c3`; selected final
evidence was then copied here. Complete diagnostics and logs are gzip-compressed,
not truncated. [Summary](evidence/summary.json), [review index](evidence/review-index.json),
[provenance](evidence/provenance.json), per-pair manifests, binary hashes and
retained-negative hashes identify every input and result. All newly generated
sources and same-stem Java SVGs are under `test-diagrams/perturbations/<mechanism>`.

Use `CARGO_TARGET_DIR=/tmp/rustuml-seq-check-b60455c3/target`,
`CARGO_BUILD_JOBS=4`, and
`PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib`.
Baseline-write variables must remain unset.

The before and after builds each used:

```sh
cargo build --release -p rustuml -p rustuml-oracle --bin rustuml --example diff_one
cargo test --release --no-fail-fast --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards -- --nocapture
```

The built `rustuml` and `examples/diff_one` were copied to scratch `before/` and
`after/`. `compare_cli.rs` was compiled with `rustc --edition=2024`, linking the
tested revision's `librustuml_oracle-*.rlib`; exact commands and binary SHA256s
are in `evidence/{before,after}-binaries.json`. It calls unchanged
`compare_svg_strict` on **actual CLI output**. `diff_one` separately records
library source-only and assisted outcomes; CLI/library byte equivalence is
checked but is not substituted for the actual CLI comparison.

`measure.py` was run once per row below, with scratch root
`/tmp/rustuml-seq-check-b60455c3`, repository-root cwd unless shown:

| Manifest | Binaries | Output |
| --- | --- | --- |
| `parent955.json` | `before` | `parent-before` |
| `parent955.json` | `after` | `parent-after` |
| `maker535.json` | `before` | `maker-before` |
| `maker535.json` | `after` | `maker-after` |
| `generated/generated.json` | `before` | `fresh-before` |
| `generated/generated.json` | `after` | `fresh-after` |
| `focused-generated/generated.json` | `before` | `focused-before` |
| `focused-generated/generated.json` | `after` | `focused-after` |
| `include-controls.json` | `after` | `includes-package-cwd` (cwd `crates/rustuml-oracle`) |

Invocation: `uv run --no-project python docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py MANIFEST BINARIES OUTPUT [CWD]`.
The preserved manifests replace scratch manifests for a replay. `generate.py`
and its `--focused` option document the independent input construction and raw
Java command; generation refuses to overwrite existing evidence. Replay the
preserved inputs instead of regenerating them in this checkout.

Candidate quality commands:

```sh
cargo test --release -p rustuml-parser -p rustuml-render --lib --no-fail-fast
cargo clippy --release -p rustuml-parser -p rustuml-render -- -D warnings
rustfmt --edition 2024 --check crates/rustuml-render/src/sequence.rs crates/rustuml-parser/src/parse/sequence.rs crates/rustuml-parser/src/diagram/sequence.rs
git diff --check
```

After authoring reviews and retaining the 22 failures, the unchanged standing
perturbation suite and guards were run again to verify registration and preserve
their failing result. That is an evidence-registration check, not a repeated
main-corpus run or an attempt to turn red gates green.
The initial before built-in run contained 955 entries and the initial after run
contained 1073 (955 + 118 primary checker pairs). Those aggregate logs therefore
have different denominators; only the explicitly frozen CLI/library manifests
are used for before/after regression comparisons.
