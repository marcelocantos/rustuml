# TIM checker5 corrective maker packet — acceptance pending

This is implementation evidence, not an independent review. No acceptance record
or prior review was authored or changed. The parent has not integrated this TIM
production. All six corrective claims need a **new independent checker**.

- Exact starting candidate: `1703a2727fa364ee8004ba59a22b4c6934ee5d3d`.
- Independent evidence: `5d37f60cbce90a3f3a9efb199c75285d6b7b859f`, imported as
  `079c1bc1ffc325e8ebfa1719bc0ce2d3c71573c2`.
- Six causal accounts, committed before production:
  `c91103f92e9f7e4e79fd1d1c735a68af0359934a`.
- Implementation: `f663c083df56b736b7d8fef693a9b9a3c106c7a5`.
- Maker: inherited gpt-6-astra/high through Codex/T3, isolated checkout
  `/Users/marcelo/.t3/worktrees/rustuml/t3-2c674563`, no children.

## Changed contracts

The accounts are `tim-rpn-call-state`, `tim-call-operand-resolution`,
`tim-legacy-raw-caller-prefix`, `tim-local-sub-collection`,
`tim-intval-conversion`, and `tim-hsl-typed-alpha`.

Prepared expression operands now execute against one expression-wide stack and
exact-key named map, following `ReversePolishInterpretor`. Function body
expressions receive independent state. Parameter matching keeps the dollar in
its original spelling; only the existing internal memory index removes it.
Duplicate keys and nested calls retain Java's distinct-key count and shared-stack
behavior, including fatal underflow. They do not receive fabricated values.
Plain builtin calls parse named RHS expressions for effects without counting them
as positional operands. Signature coverage precedes return/procedure kind checks.

Multiline legacy calls preserve the raw caller prefix and intentionally discard
the suffix; procedures retain their evaluated prefix behavior. Local subblocks
reject nested starts during collection, before any buffered effects. External
raw selection and TIM comment processing remain separate. IntVal parses complete
signed integer text instead of truncating decimals. HslColor converts typed
integer operands, clamps and computes the cited Java float equations, and includes
alpha in Java's transparent/RGB/ARGB serialization. No builtin breadth was added.

Production changes only `preprocess/mod.rs`. Date widths, reader policy, parser
handoffs, renderers, fonts, registry, comparators, frozen fixtures/baselines,
allowlists and targets are untouched. Fifteen Java source files were compared
byte-for-byte to revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`;
`java-source-provenance.json` contains their hashes.

## Measured results

All SVG numbers below use the shipped CLI and unchanged strict comparator.
Negative references are excluded from positive matches, not from membership.

| Preserved membership | Valid references | Before CLI matches | Final CLI matches | Exact valid sources matched |
| --- | ---: | ---: | ---: | ---: |
| Restored 1,516 | 1,459 | 567 | 567 | 59/59 available |
| Previous checker 31 | 21 | 21 | 21 | 21/21 |
| Checker5 fresh 29 | 14 | 11 | 14 | 14/14 |
| Previous maker 15 | 9 | 9 | 9 | 9/9 |
| Prior maker 17 | 14 | 14 | 14 | 14/14 |
| New maker 18 | 8 | 2 | 8 | 8/8 |

There are **no lost valid passes, new valid CLI errors or valid model-count drops**
in these measurements. Checker5's key-spelling lost pass is restored. Its other
two valid mismatches, raw macro prefix and HslColor alpha, now match source/SVG.
The 955 and 1,044 historical subsets remain 431 and 495 matches. The frozen 1,449
subset remains 520. All six raw-file controls and five class-checker TIM controls
retain their measured passes; this does not claim combined class integration.

The listed memberships total 1,626 entries. The 1,516 contains 1,437 unique
source/reference byte pairs (79 duplicated entries); the other cohorts have no
pair overlap with it or each other. Their union is 1,547 distinct byte pairs.
The unchanged standing harness sees 1,576 stored perturbation entries, excluding
the maker-only 15/17/18 directories. These are this checkout's denominators,
not the parent's larger integration cohort. `analysis.json` records the exact
pair intersections and membership counts; failed or duplicate entries were not
dropped from any replay.

The 1,516 preserves 32 panic entries: 24 variants of one sequence underflow
signature and eight of the other. The existing valid class `body_ownership`
parse error also remains. No new panic was observed. All 1,516 received final
CLI YAML/model capture as well as SVG; exact-source comparison is claimed only
where a preserved source oracle exists. Known-negative inputs that now correctly
fail naturally lose their previously invented model; those four model drops are
explicitly listed separately from valid regressions.

Final gates ran once on the committed production (exact commands, environments,
statuses and logs are in `gates.json` and the archive):

- Parser: **605 passed**; CLI pipe: **15 passed**.
- Product: **11,179/11,251**, exit 101; assisted: **10,497 passed,
  754 failed, 1,299 skipped**, exit 101. The historical GET versus owner-approved
  raw-reader conflict is unchanged; no reference or assertion was normalized.
- Standing perturbations: **575/1,576**, exit 101. Matching-environment isolated
  CLI: **602/1,576**. The 27 cwd/timezone/environment differences are enumerated,
  not waived. The registry cascade is absent; the two real sequence panics remain.
- Guards: **4/6**, exit 101 (existing decimal ratchet and review-evidence failures).
- Touched rustfmt and diff check: exit 0. Clippy: exit 101 at unchanged
  `crates/rustuml-parser/src/parse/component.rs:72` dead code.
- Final staged evidence diff check: exit 2 solely for five exact comparator
  stdout captures ending in blank lines. `final-staged-diff-check.txt` preserves
  those diagnostics; raw reference/comparison bytes were not cleaned up.

No repeated main gates were used to seek better counts. Focused replay was
repeated after the final plain-call diagnostic wording correction. Intermediate
failures remain archived: the initial parser run had 593 passes/four failures
(two inadvertently rejected existing float-extension tests, one stale arity
message assertion, and one duplicate-key assertion encoding the rejected
per-call stack model). The original duplicate-key test source is retained, now
asserting fatal underflow; no old fixture or review was edited. The float
extension was preserved. The next parser run passed 605 before final gates.

## Negative evidence and boundaries

The new 18 are **8 valid and 10 negative**; intended labels remain unchanged.
`keys_stack` was intended as a valid exploration but Java exits 1 with empty
non-XML output: it consumes the enclosing operand and later underflows. Its
original source and both failed results are preserved. Java's other empty
underflow is `keys_duplicate`. Eight other negatives produce exit-200 error SVGs.
Rust rejects all ten. XML validity, process status, error-page classification,
probe status and exact raw bytes remain separate in the records. Probes exit 0
for 16 cases and 1 for the two underflows; probe success alone is not validity.

Checker5 negatives reject 12/15; prior31 rejects 8/10, maker15 6/6, maker17 3/3.
Remaining accepted negative checker5 cases are isolated-surrogate display,
function-in-index grammar, and `sub_suffix_continuation`. The last retains
`!endsub0` in evaluated source; the downstream class parser accepts that otherwise
invalid line. Changing that grammar is outside this packet's owned preprocessing
contracts. No parser handoff was changed. For nested-sub errors Java CLI reports
line 0 while Rust reports the located outer declaration (line 1 in the fresh
control); this diagnostic-number difference remains explicit. Underflow uses the
existing fatal preprocessing error channel rather than reproducing a JVM stack
trace. Neither behavior is represented as exact negative-output parity.

General isolated-surrogate projection, wider Unicode/locale conversion (including
Java Character.digit versus Rust ASCII IntVal text), arbitrary JVM property
configuration, function-in-index grammar, wider macro-body execution, rendering
source-line-zero fallback, class/style/palette, handwritten/activity geometry,
and sequence continued-line geometry remain outside any established closure.
The successful date-width model and noncanonical environment controls were
preserved; they do not establish host-runtime equivalence. This packet makes no
whole-TIM, main-product, or combined class/sequence acceptance claim.

## Remote provenance and reproduction

No local Java/javac, SSH, network change, new agent or shipping action occurred.
The parent relayed the immutable 18-case/19-resource request to
`/Users/marcelo/rustuml-reference/scratch/worker.VQo5emLT` on Monkey Man.
All 36 remote SVG/probe calls have actual wrapper/Java argv, cwd, environment,
stdout/stderr hashes, statuses and no timeout. The wrapper uses headless AWT,
UTF-8, en_AU, GMT+11:00 and pinned JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
OpenJDK 21.0.5 / Graphviz 12.2.0 differ from historical Java 21.0.13 / Graphviz
16.1. This is scoped new-reference evidence, not global historical equivalence.

- Request SHA256: `7be8f4bac187e38dee21d8812613dc1ac93f2e6f379da57b738496693d5629a5`.
- Manifest SHA256: `ebed72f9cb2872e86be271b76f8affaaf376d86ae5a522e50aeee156f2175890`.
- Input compressed transport SHA256:
  `1ea2ab3b52b91deb9596ae135ba45480ccadc514ea1f24b61bc3c2b9af5c7fe8`.
- Returned gzip SHA256:
  `b204b17dee9a3d4ee6ec60b0c67ac835621d9ee8947f04da3db8bdd3a106e943`.

The raw return bundle/transport and decoded streams are under `relay18/`.
`inputs/` preserves all 18 sources and same-stem Java SVG bytes, including empty
and error files. `measurements.tar.gz` retains all initial/final captures,
commands, scripts, logs and failed iterations from `/tmp/rustuml-tim6`.
`files.json` hashes the packet files. `integrity.json` verifies all **7,866**
previous evidence files against the imported Git blobs and the unchanged golden
gitlink `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`.

The before binary is the separately built 1703 candidate from the existing
`/tmp/rustuml-tim5-target` (SHA256
`be7949988c2a1212f587deb7b80362c0f40d21556c6138c925d0e73ae34594b5`).
The final build uses only `/tmp/rustuml-tim6-target`, `CARGO_BUILD_JOBS=4`, with
no source-mtime checkout switch (binary SHA256
`d12f8b9b79daa5d1b6ca85a16a7553d2a32bed6680a15643c9ce476caaf0b584`).
Before measurements for the preserved cohorts reuse checker5's exact-candidate
`after` records; the new 18 also execute the actual before CLI. The comparator
binary is unchanged (`43ce27167f6ad8697a8043edbb4f580ddbf9ab68b4e0ea4db15f2a69e3fc97b5`),
using the preserved checker3 probe and unmodified comparator/invalid-reference
classification.

To reproduce, restore the archive to `/tmp/rustuml-tim6`, restore referenced
checker5/cache archives if absent, build each revision into its separate target,
and copy the final executable to `final-rustuml`. Then run from this checkout:

```text
CARGO_TARGET_DIR=/tmp/rustuml-tim6-target CARGO_BUILD_JOBS=4 cargo build -p rustuml --release
python3 /tmp/rustuml-tim6/replay.py final collateral
python3 /tmp/rustuml-tim6/replay.py final prior31
python3 /tmp/rustuml-tim6/replay.py final fresh29
python3 /tmp/rustuml-tim6/replay.py final maker15
python3 /tmp/rustuml-tim6/replay.py final maker17
python3 /tmp/rustuml-tim6/relay_eval.py final
```

These scripts only consume preserved references and launch local Rust binaries.
Do not rerun the immutable generator into its existing input directory. The final
main-tier commands are recorded in `gates.json`; their existing results stand.
