# TIM checker6 corrective maker packet — acceptance pending

This is implementation evidence, not an independent review. The three new claims
need a new independent checker. No previous review, source, reference, comparator,
baseline, target, reader policy or acceptance record was changed. The parent has
not integrated this TIM production.

- Starting candidate: `4a288dd47fe925a792f7cb672ca294f6ef6fdcf3`.
- Checker6 evidence: `a3b48251354497c68eaa0b93a168d3ba5c455c0f`, imported as
  `ba57d3c7275a688b1f1ac565551cdff08fc52d32`.
- Three corrective causal accounts committed before production:
  `4652860936464f02d80584ef158a29a4a1c13cd1`.
- Main implementation: `329c39f48eccddfbb4896f1710dd97ddd1c1f2e8`.
- Final corrective implementation: `55de7dd6cb090d86febc1e5e3f57804134429a19`.
- Maker: inherited gpt-6-astra/high through Codex/T3, isolated checkout
  `/Users/marcelo/.t3/worktrees/rustuml/t3-2c674563`, no children.

## Changed models

`tim-exact-memory-identities` replaces the separate bare-name map and dollar-stripped
memory with exact keys throughout assignment, captured defaults, parameter storage,
global/local/override lookup, direct expression reads, text expansion, builtins,
existence and removal. `b` and `$b` remain separate. Existing scope precedence,
sibling frames, typed values, loop return unwind and filtered environment behavior
remain covered by preserved controls. Plain text scanning may expand a bare name
after a literal dollar; that does not make `$NAME` an alias in expression memory.

`tim-function-signature-availability` retains completed declarations by exact name
and declared arity, with pending collection separate from the available registry.
Redefinition replaces the same signature, not every overload. Resolution prefers
an exact positional signature before compatible named/default coverage and kind
checking. Bare function names are available to the text scanner with Java's token
boundary. Fallback order derives from `TFunctionSignature.hashCode` and ordinary
OpenJDK HashMap bucket traversal, including capacity growth. The 74 initial builtin
signatures and their source hashes are enumerated in
`signature-capacity-derivation.json`; these are registration inputs, not 74 newly
implemented bodies. Java/JDK constants and data flow are cited in the precommitted
account and code. Pathological tree-bin ordering and unsupported legacy-overload
syntax remain unproved; this packet does not claim complete registry equivalence.

`tim-sub-before-function-buffering` puts subblock collection ahead of procedure,
return-function and legacy body buffering, as in `TContext.buildCodeIterator`.
It validates and registers subs inside uncalled definitions, then sends located
body lines through the downstream collector exactly once. It does not execute a
stored function body early. External raw selection and TIM comment processing
remain distinct. No parser handoff or renderer was edited.

Production changes are confined to `preprocess/mod.rs`. Pinned Java files were
verified against revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`;
`java-source-provenance.json` records their hashes. The derivation also records the
read-only JDK HashMap source hash. No runtime Java dependency was introduced.

## Actual CLI measurements

The SVG columns use shipped CLI output and the unchanged strict comparator.
Negative references remain members but never count as positive matches.

| Preserved membership | Valid refs | Before SVG matches | Final SVG matches | Exact valid sources |
| --- | ---: | ---: | ---: | ---: |
| Restored 1,516 | 1,459 | 567 | 567 | 59/59 available |
| Previous checker 31 | 21 | 21 | 21 | 21/21 |
| Checker5 fresh 29 | 14 | 14 | 14 | 14/14 |
| Previous maker 15 | 9 | 9 | 9 | 9/9 |
| Prior maker 17 | 14 | 14 | 14 | 14/14 |
| Previous maker 18 | 8 | 8 | 8 | 8/8 |
| Checker6 fresh 29 | 19 | 12 | 17 | 17/19 |
| New maker 15 | 14 | 2 | 14 | 14/14 |
| New supplement 2 | 2 | 0 | 1 | 2/2 |

There are no final lost valid passes, new valid CLI errors or valid model-count
drops relative to the starting candidate in these cohorts. Checker6's five
in-scope valid failures now match source/SVG. Its two Unicode IntVal failures
remain actual CLI errors, explicitly outside this bounded packet. Its uncalled
nested-sub negative now rejects before emitting the formerly invented Survivor
model; that negative model drop is recorded separately.

The memberships total 1,672 entries and 1,593 unique source/reference byte pairs.
The 1,516 contains 79 duplicate entries; all are still measured. There is no
pair overlap between the other cohorts or with that cohort. `analysis.json`
records every intersection. Historical subsets remain 431/955, 495/1,044 and
520/1,449. These are this checkout's scoped cohorts, not the parent's integration
inventory. All six raw-file CLI controls and all five class-checker TIM controls
retain their measured passes; this is not combined class integration evidence.

All 1,516 also received actual CLI YAML/model capture. The 32 existing panic
entries remain: 24 variants of the sequence len-2 underflow and eight of len-1,
both at `sequence.rs:8022`. The existing valid class `body_ownership` parse error
remains. Exact-source equality is claimed only where a preserved source oracle
exists, not inferred from SVG matches or model counts.

## Supplement meaning and ordinary unit expectations

The owner's explicit authorization permits correcting five ordinary unit
expectations once the captures establish exact-key behavior. Supplement2 does:
`define_spelling` evaluates `NAME / $NAME / MSG / $MSG` to
`Alice / $Alice / hello / $hello`. `define_condition` chooses `class Right` when
`MODE` exists but `$MODE` does not, and preserves literal dollars in note text.
Both raw Java source probes equal the final CLI model source byte-for-byte.

`unit-expectation-corrections.json` preserves each complete old block, old
assertion, unchanged input expression, new block, Java methods, capture linkage
and what it still verifies. `parser1.log` in the archive preserves the five
original failing assertions (600 passed, five failed). The changes keep negative
NAME-versus-$NAME distinctions, include-loaded definitions, multiple substitutions,
foreach preservation and conditional selection. No corpus assertion or baseline
was changed. The independent checker must assess these expectation changes.

Supplement `define_condition` still fails SVG comparison: disconnected class and
notes are packed differently (Java 236x155, Rust 354x70). Its exact evaluated
source matches, and both processes succeed. This is a retained downstream class
layout dependency, not counted as a preprocessing or SVG pass.

## Failed iteration and final gates

The first final-gate run on `329c39f4` exposed an accidental deletion of the
existing `%filename()` dispatch branch during the memory edit: product 11,178,
assisted 10,496 and a new valid CLI error. The entire failed run, source snapshot,
commands and raw outputs remain archived. The unrelated branch was restored
verbatim in `55de7dd6`, with a focused expression/plain-substitution error-status
unit. `filename-regression.json` records frozen-source actual CLI before pass,
intermediate failure, final pass. Four unrelated include diagnostics from the
investigation were run from repository cwd and fail identically before/after;
they are not newly invented geometry defects. No frozen source was regenerated.

The code correction justified one final rerun. `gates2.json` and `logs2/` in the
archive contain exact commands, environments and nonzero statuses:

- Parser units: **615 passed**; CLI pipe: **15 passed**.
- Main product library tier: **11,179/11,251**, exit 101.
- Assisted library tier: **10,497 passed, 754 failed, 1,299 skipped**, exit 101.
- Standing perturbations: **592/1,605**, exit 101. Matching-environment isolated
  CLI: **619/1,605**. The 27 cwd/timezone/environment differences are enumerated,
  not waived. The registry cascade is absent; real sequence panics remain.
- Guards: **4/6**, exit 101 (decimal ratchet and review-evidence failures).
- Touched rustfmt and production diff check: exit 0.
- Clippy: exit 101 at unchanged `parse/component.rs:72` dead code.

The main library tiers are not presented as actual CLI measurements. Their
historical GET-versus-approved-raw reader conflict is unchanged. No date-width,
raw-origin assertion or gate input was modified. The archive retains earlier
focused iterations and their failures rather than replacing them with final logs.

## Negative evidence and remaining scope

New maker15 has 14 valid cases and one exit-200 error SVG (`sub_uncalled_nested`);
all 15 source probes exit 0. Rust rejects the negative. Supplement2 has two valid
XML SVGs, both SVG/probe exit 0. XML well-formedness, process status, comparator
error-page classification and probe status are separate fields, not substitutes
for each other. All stdout/stderr bytes, including empty streams, are retained.

Checker6 negatives reject 10/10. Earlier negatives retain their outcomes:
37/57 in the restored cohort, 8/10 prior31, 12/15 checker5, 6/6 maker15,
3/3 maker17 and 10/10 maker18. Accepted negative controls remain failures, not
positive evidence. General isolated-surrogate projection, function-in-index
Rust grammar, raw `!endsub0` downstream class acceptance, and Java-versus-Rust
negative error-page/line-number output remain outside the claim. Unicode IntVal
text, arbitrary JVM property configuration, wider macro-body behavior, renderer
source-line-zero fallback, class/style/palette, handwritten/activity geometry,
and sequence continued-line geometry remain explicit dependencies. Successful
date-width controls do not establish global host-runtime equivalence.

## Provenance and reproduction

No local Java/javac, SSH, new agent, network change or shipping action occurred.
The parent relayed immutable resources to Monkey Man's pinned headless wrapper.
All 34 SVG/source-probe calls have actual wrapper/Java argv, cwd, timezone,
environment, stream hashes and statuses; none timed out. The wrapper uses UTF-8,
en_AU, GMT+11:00 and JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Remote OpenJDK 21.0.5 / Graphviz 12.2.0 differ from historical Java 21.0.13 /
Graphviz 16.1. These new captures are scoped evidence, not global equivalence.

| Batch | Remote worker | Returned gzip SHA256 |
| --- | --- | --- |
| Maker15 | `/Users/marcelo/rustuml-reference/scratch/worker.cGSCHgP6` | `a9af5ebfeece6d43a4dbfdc8df22b14d2d0db1b7de86d0e3186cf27e49855905` |
| Supplement2 | `/Users/marcelo/rustuml-reference/scratch/worker.4deTDGRy` | `47ef58c2c974f58f6a89032a7d05fe076af2e4a5fba752b8fd0957d3a78b9d0c` |

Request, manifest, input-transport and resource hashes are preserved in the
requests and each relay's environment/summary/transport records. `inputs/` and
`supplement/` contain all 17 unchanged sources and same-stem Java SVG bytes.
`relay15/` and `relay2/` contain the complete returned bundles and decoded streams.
`measurements.tar.gz` contains all local captures, input bytes, replay metadata,
failed/final logs and scripts from the persistent private cache, excluding build
outputs and executables. `files.json` hashes the durable packet.

`integrity2.json` verifies **8,987** prior evidence files against the imported Git
blobs, with zero modifications. The golden gitlink remains
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Before and current builds use separate
targets: the unchanged 4a288dd executable from `/tmp/rustuml-tim6-target` versus
`/Users/marcelo/Library/Caches/rustuml-tim7-maker/target-after`, always jobs <=4.
The before binary hash is `d12f8b9b79daa5d1b6ca85a16a7553d2a32bed6680a15643c9ce476caaf0b584`;
final is `6f85256ff443f2a6f44d572fc7b46df5c22e1c0d7fef19f574de098d5ca038a2`.
The unchanged comparator binary hash is
`43ce27167f6ad8697a8043edbb4f580ddbf9ab68b4e0ea4db15f2a69e3fc97b5`.

For reproduction, restore the archive to the named private cache. Replay-input
JSONs preserve the exact checker6 before measurements at 4a288dd; restore them to
`/Users/marcelo/Library/Caches/rustuml-review-4a288dd-checker6` if absent.
`replay-input-paths.json` maps original paths to content-addressed bytes under
`replay-input-bytes/` for reconstructing absent ephemeral sources/references.
Keep include-relative cwd and the pinned external stdlib hierarchy intact. Build
the final revision in its own target and copy it to `final2-rustuml`; restore or
separately build the before executable. Rebuild the unchanged comparator from
`tim-typed-values-and-binding/checker3-support/probe.rs` as in prior evidence.

```text
CARGO_TARGET_DIR=/Users/marcelo/Library/Caches/rustuml-tim7-maker/target-after CARGO_BUILD_JOBS=4 cargo build -p rustuml --release
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 collateral
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 prior31
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 fresh29
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 maker15
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 maker17
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 maker18
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/replay.py final2 fresh6
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/relay_eval.py final2
python3 /Users/marcelo/Library/Caches/rustuml-tim7-maker/relay2_eval.py final2
```

These commands consume preserved references and execute only local Rust. Do not
rerun generators into immutable input directories. Final gate commands and their
real statuses are recorded; this packet makes no acceptance or whole-TIM claim.
