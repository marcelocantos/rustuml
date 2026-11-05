# Independent checker6: six TIM corrections are not accepted

Reviewed exact candidate `4a288dd47fe925a792f7cb672ca294f6ef6fdcf3`, production
`f663c083df56b736b7d8fef693a9b9a3c106c7a5`, against independently built before
`1703a2727fa364ee8004ba59a22b4c6934ee5d3d`. Reviewer: gpt-6-astra/high through
Codex/T3, separate task and checkout `t3-0d7aeef2`; same model family as maker.
No children, production edits, targets, baselines, comparator/skip/policy changes,
local JVM, SSH, new remote agent, push or PR. This is evidence only.

The six account blobs at `c91103f92e9f7e4e79fd1d1c735a68af0359934a` precede
implementation. The implementation changes only `preprocess/mod.rs` and cites
that account commit. Candidate production equals implementation production.
Nineteen relevant read-only Java source files match pinned revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711` byte for byte. No new fixture-shaped
selection or unsupported layout constant was found. HSL constants and operation
order trace to the named Java equations.

## New counterexamples and bounded conclusions

- **RPN exact spelling is incomplete.** Java keeps `b` and `$b` distinct through
  `TFunctionImpl.getNewMemory` and `TMemoryLocal`. Rust preserves names during
  matching, then strips the dollar and aliases slots. `rpn_dual_parameters`
  produces Java `plain/dollar / cedar/larch`, Rust `/dollar / /larch`.
  `rpn_bare_read` removes the builtin-lookup confound: Java `plain/dollar`, Rust
  `b/dollar`. Expression-wide stack/map, duplicate-key underflow, nested-call
  consumption and independent function-body expression controls otherwise survive.
- **Call resolution lacks the available signature set.** Java `FunctionsSet`
  stores `TFunctionSignature` keys; Rust's name-keyed map overwrites an earlier
  overload. Two valid controls vary arity, declaration order and function kind:
  Java renders `value 7` or `alder/beech`; Rust reports unknown function in both.
  Plain named RHS effects, duplicate named effects, exact-arity shortcut and
  coverage-before-kind first-error controls pass their bounded observations.
- **Sub collection happens too late inside function definitions.** Java
  `buildCodeIterator` places `CodeIteratorSub` before function/procedure
  collectors. Rust's `try_function_def` consumes new markers first. Java rejects
  nesting in an uncalled procedure; Rust renders `Survivor`. A valid subblock
  defined inside an uncalled procedure is available for later inclusion in Java,
  but Rust reports `cannot include stored`. Top-level nested validation before
  effects and comment/replay controls pass.
- **IntVal ASCII scope survives; general conversion does not.** Signed bounds,
  JSON-displayed integers, invalid signs and overflow behave as claimed for ASCII.
  Two valid Unicode controls convert fullwidth/Arabic-Indic/mixed Devanagari digits
  in Java, but Rust rejects them. These are new valid CLI errors relative to before
  (which rendered wrong zero values), not lost correct SVG passes. The precommitted
  planned change explicitly excluded Unicode `Character.digit` equivalence; that
  qualification is real, but the unqualified displayed-TValue conversion invariant
  is not established. Keep this boundary explicit and its failing pairs executable.
- **Raw legacy prefix and typed HSL/alpha observations survive.** Repeated
  variable/function prefixes, indentation, suffix dropping versus procedure suffix
  retention, signed extreme hue, typed string/JSON channels, clamping and alpha
  rounding all match exact evaluated source and source-only CLI SVG. These bounded
  observations are not accepted records while mandatory gates remain red.

All six `review.json` verdicts are **rejected**. Three mechanisms have fresh
in-scope causal counterexamples; the other records distinguish surviving bounded
behavior from acceptance. No whole-TIM or combined class/sequence verdict follows.
The repair guidance is in each mechanism's review; repairs need new preimplementation
accounts and independent review, preserving these inputs and all earlier failures.

## Independent measurements

Both release CLIs were rebuilt with jobs=4 and distinct private targets. The before
source is an exact `git archive` export, not another worktree. All replay phases
execute those binaries anew; no maker/checker5 measurement is reused as the before
result. The unchanged comparator binary is hash-verified and its source/library
Git blobs are unchanged. Full YAML model and SVG streams are captured for both
phases of every cohort member, including duplicate and negative entries.

| Preserved membership | Valid | Before CLI matches | Candidate CLI matches | Candidate exact valid sources |
| --- | ---: | ---: | ---: | ---: |
| Frozen/restored 1,516 | 1,459 | 567 | 567 | 59/59 available |
| Prior checker31 | 21 | 21 | 21 | 21/21 |
| Checker5 fresh29 | 14 | 11 | 14 | 14/14 |
| Maker15 | 9 | 9 | 9 | 9/9 |
| Prior maker17 | 14 | 14 | 14 | 14/14 |
| Maker18 | 8 | 2 | 8 | 8/8 |
| Independent fresh25 + supplement4 | 19 | 5 | 12 | 12/19 |

The preserved cohorts have no lost valid matches, new valid CLI errors or valid
entity-count drops. The historical 955/1,044/1,449 subsets retain 431/495/520.
All six raw-file CLI controls and five class/TIM controls retain their passes.
The 32 preserved panic entries are 24 instances of the sequence len2 signature and
eight of len1, both at `sequence.rs:8022`; they are not 32 independent new bugs.
The valid class `body_ownership` parse error remains. Known-negative model drops
are recorded separately, not reported as valid regressions.

Preserved memberships total **1,626 entries / 1,547 distinct source+reference
byte pairs**. The 1,516 contains 1,437 unique pairs (79 duplicate entries). Other
cohorts have no pair overlap. The 29 fresh inputs add 29 distinct pairs: total
**1,655 entries / 1,576 unique pairs**. This last number is a pair union, not the
standing-harness denominator. Exact intersections are in `analysis.json`.

The fresh 29 are **19 valid and 10 negative**. Twelve valid pairs match; seven do
not. Nine negatives reject; nested markers in the uncalled procedure remain
accepted. All intended classifications remain in the immutable manifests; observed
Java validity, XML validity, raw process/probe status, comparator result and Rust
status are separate. The underflow has empty non-XML output and exit 1; nine other
negatives have exit 200 error SVGs. Successful SourceProbe exit is not validity.
No negative-output parity is claimed. First diagnostics and all raw streams remain.

The previous negative residuals remain: isolated-surrogate projection,
function-in-index grammar, downstream class acceptance of raw `!endsub0`, nested-sub
line numbering, and fatal error versus JVM underflow stacktrace. The prior31
negatives reject 8/10, checker5 12/15, maker15 6/6, maker17 3/3, maker18 10/10.
These residuals limit broader claims and are not silently waived.

## Gates, run once on exact candidate

- Parser 605 and CLI pipe 15 pass.
- Main no-oracle **library tier** 11,179/11,251, exit 101. Assisted 10,497/11,251
  eligible, 754 failed, 1,299 skipped, exit 101. These are not source-only CLI
  headline results. Historical GET versus approved raw-reader differences remain.
- Standing perturbations 575/1,576, exit 101. Matching-environment CLI 602/1,576;
  all 27 cwd/timezone/environment differences are individually recorded.
- Guards 4/6, exit 101: decimal ratchet and historical review-evidence failures.
- Owned rustfmt and initial diff check pass. Clippy exits101 at unchanged
  `parse/component.rs:72` dead code. No broad gate was rerun after reference arrival.

Those standing/guard results measure the exact candidate before new review
artifacts. Adding 29 pairs gives 1,605 stored perturbation inputs; this review does
not invent a new full-suite count. Every fresh pair was independently executed
through the real CLI and unchanged comparator. Error/empty references remain
failing standing-oracle cases even where correct Rust rejection is observed.
The final staged diff check exits 2 for exact raw comparator/test logs ending
in blank lines and raw Git history/diff whitespace. Its full diagnostics are
recorded separately; these bytes are not normalized to make the check green.

## Provenance and reproduction

Parent relay used the existing Monkey Man runner, unchanged `SourceProbe.java`,
headless AWT/UTF-8/en_AU/GMT+11:00, pinned JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
All 31 resources, 58 calls, source/stream hashes, actual wrapper and Java argv,
cwd/env/timezone and no-timeout statuses were independently checked. Raw bundles
and transports are preserved in `relay25/` and `relay4/`; same-stem source/SVG
pairs are under each mechanism's `test-diagrams/perturbations/.../checker6/`.

- TIM25 request 414e7b6d4ec350e33f2d9211c5ac8fbaed7d41c0e85b628ac55d1b10989a57a4;
  manifest 5d5f22068b58b81ce39209d21990ce6fa934d1020786144d5318f5662a42bf80;
  returned gzip 4526badf765755ebfef45aaa59f6eabf7f647500c694ba9c8a0c9e2c06e8f29e.
- Supplement request 746656ea7c641ff2de1073fd9caace504d045666a3a3c821076538bcc724f9f9;
  manifest 6c7ac32d7268ebb3898dd66504c286f8b9231f0a112f2e7bbf098eae3a05f46e;
  returned gzip 2828c26d2923df37f0e59f329be7cbaa69ba6f7141e63b53b5ef557a0e1f3f6c.

Remote OpenJDK 21.0.5/dot 12.2.0 differ from historical 21.0.13/dot 16.1. No universal
font/runtime/Graphviz equivalence is claimed. Historical GET files were untouched.
All 7,866 imported old evidence files and all 8,295 candidate evidence files match
Git blobs. The pinned golden gitlink remains 9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a.

`audit.json`, `builds.json`, `gates.json`, per-case records and `measurements.tar.gz`
retain revisions, hashes, commands, all before/after captures and all failures.
The initial attempt to use the checker5 cache comparator found a hash mismatch;
that unused binary was rejected. The exact maker-recorded comparator was located
and verified before measurements. This setup failure remains in `setup-failure.json`.

Working paths are `/tmp/rustuml-checker6`; a private persistent copy, including
both independently built CLIs and targets, is in
`/Users/marcelo/Library/Caches/rustuml-review-4a288dd-checker6`.
Reproduction scripts are preserved here. Restore the measurements archive to its
recorded scratch path and use the immutable requests and recorded command manifests.
Do not rerun generators into existing immutable input directories. Historical
external references used by cohort manifests remain available from their archived
checker5 packets; no new remote reference is needed for replay.

The review contributes no acceptance, release or shipping authorization.
