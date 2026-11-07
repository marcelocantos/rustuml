# Sequence registered-command admission maker packet

This is implementation evidence, not an acceptance record. A **new independent checker** is required. All earlier rejected accounts, reviews and reference pairs remain byte-exact; no verdict is replaced.

Final production is `c525fd32ae97b642b440a161140e9457f29bcfc8`. Its production, build and test sources are byte-identical to the tested revision `268fa9f8b700ba08579efc603e4d0e92b4313e5f`; see `final-source-identity.json`. The original stable broad run is retained at its actual revision, not relabelled as a new execution. No redundant broad rerun followed the exact reversal of the size guard.

Baseline evidence: `dc438fd0cd0f5edd44816db81f0d765bf48ce4e6`, production `b4849a5008ff28c360fefe0176ee6ad073c02fd7`. Independent rejection `0fb379a26ab64ead0744278935a0b6c64082ef35` was imported unchanged as `193480c26fc5885d7b8e60f0452c9c755ca6d68e`.

## What changed

- The sequence parser now admits commands from the pinned factory's **81 registrations in their original order**. The source probe, raw patterns, Fox signatures, class names and Rust handler/support mapping are preserved in `inventory.json` and `command-inventory-raw.json`. Matching includes Java Pattern2 whitespace/classes, FoxSignature, multiline command ends and ColorParser token boundaries. No registered command match produces a syntax error. Recognized but unsupported commands/variants produce explicit stderr warnings, rather than invented message events or silent dropping. This does not claim those commands render correctly.
- Selected command color captures now propagate the Java `SingleLineCommand2#execute` badColor outcome. Composite/gradient validity is checked without claiming complete style materialization. The four valid composite/gradient controls still fail whole-output comparison and report the unsupported style residue.
- `CommandFootbox` optional whitespace and legacy on/off forms reach the visibility handler before permissive message parsing. This removes bogus participants/messages and fixes all 14 new whole-output matches.
- The sequence-local attribute escaper emits Java-compatible decimal references for LF/CR/TAB and non-ASCII scalars, including non-BMP characters. Existing text-node consumers retain their distinct serialization path. The shared branch was not imported.
- The resume citation now names `CommandAutonumberResume#getConcat`. `citation-corrections.json` also corrects the new size-account's factory-order method name: the code is the **PSystemBuilder constructor**, not an `initFactories` method. Old accounts are untouched.

Only `crates/rustuml-parser/src/parse/sequence.rs` and `crates/rustuml-render/src/sequence.rs` changed in production. Model, ASCII, parse/mod, other-family, shared text/style/font, preprocessor, native, registry, harness, comparator, baselines and targets are untouched. Actual ASCII output changes are captured; leaving its renderer untouched is not a behavior-invariance claim.

## Size-limit hypothesis and reversal

Account `8efd66082c108fb0b46439665363fe9a8ce827d4` and implementation `ba3d0196bcb11829a3e114871976faaa695c7025` were committed **before the remote results arrived**. The hypothesis wrongly treated a sequence-command rejection as a global rejection. All eight raw SVGs are well-formed, have complete labels and exit 0. Four 2,000-UTF-16-unit inputs render SEQUENCE; four 2,001-unit inputs render CLASS. Four intended-invalid labels were therefore wrong and remain unchanged in the request manifest.

`CommandArrow#getRegexConcat` sets `protectSize(2000)`; `RegexConcat#match` returns false beyond it. `SingleLineCommand2#isValid` and `PSystemCommandFactory#executeFewLines` produce a sequence syntax error. **PSystemBuilder#createPSystem:230–255 then continues to ClassDiagramFactory**, whose CommandLinkClass accepts the relation and preserves the complete label. Nothing is truncated.

Rust lacks faithful cross-factory rejection/fall-through. The guard introduced four Java-valid CLI errors. New correction account `297c7a1eacfff66445700496190630b891eb3ee7` preceded revert `c525fd32ae97b642b440a161140e9457f29bcfc8`. The failed guard binary's actual SVG/AST/ASCII/YAML outcomes, source and hashes are preserved in `size-diagnostics.json.gz` and `captures.tar.gz`. No dispatch edit or new exception was made. Final size controls are **2 matches / 6 mismatches / 0 errors**, unchanged from baseline: the four CLASS cases remain an executable T14.21 dependency, and the two ASCII SEQUENCE width cases still mismatch.

## Real CLI measurements

The unchanged comparator evaluated exact source bytes through `rustuml -tsvg -pipe`, from the repository root. Counts are not library substitutes.

| Cohort | Entries | Before: match / mismatch / valid error / nonpositive | After: match / mismatch / valid error / nonpositive |
|---|---:|---:|---:|
| Frozen | 3,111 | 2,195 / 749 / 2 / 165 | 2,195 / 734 / 17 / 165 |
| Independent checker | 422 | 232 / 124 / 0 / 66 | 242 / 114 / 0 / 66 |
| New render controls | 28 | 2 / 16 / 0 / 10 | 6 / 12 / 0 / 10 |
| Size controls | 8 | 2 / 6 / 0 / 0 | 2 / 6 / 0 / 0 |
| **Total** | **3,569** | **2,431 / 895 / 2 / 241** | **2,445 / 866 / 17 / 241** |

There are **14 gains, zero lost matches, zero lost rejections, zero CLI panics and zero timeouts**. The 15 new valid-input errors are exactly the parent-approved exceptions in `parent-approved-exceptions.json`, with original paths, source SHA256, before family, after diagnostic and cause:

- 13: **new valid-input CLI error caused by upstream misdispatch, T14.21, parent-approved**. These were wrong-family whole-output failures before. Dispatch ownership was withdrawn; no parse/mod edit was made.
- 2: **new valid-input CLI error caused by upstream T14.22, parent-approved**. These are the exact retained TIM line-continuation sources named by the parent.

The unchanged `diff_one` library diagnostic wrapper exits 101 on 19 cases (4 before): its `Result::unwrap()` at line 73 panics on parse errors. These are separately preserved in `library-wrapper-errors.json` and the library status counts, not hidden as CLI/render panics. The 15 new wrapper failures correspond to the exact approved upstream errors.

The two pre-existing valid errors are `ind5f_exo_short_in_legacy` and `ind5f_exo_short_in_teoz`. No other new valid error is excused. `remaining-failures.json` retains **every** non-match; no mismatch or invalid reference was removed.

The cohort has 3,569 distinct paths, 3,517 unique source hashes and 52 duplicate-source entries, all retained. One source hash overlaps the golden corpus. Do not sum this cohort with historical or current parent cohorts; integration is the parent's separate measurement.

## Rejection contracts

Comparator nonpositive status cannot prove rejection. `rejection-contract.json` records Rust exit/diagnostic for all 241 nonpositive entries, independently of positive comparison:

| Reference classification | Entries | Rejected before | Rejected after |
|---|---:|---:|---:|
| Explicitly classified Java-invalid | 122 | 40 | 122 |
| Historical nonpositive, preserving unknown original validity/exit where unavailable | 99 | 77 | 87 |
| Java exporter failures | 20 | 0 | 0 |

The 54 fresh checker command/color rejections, 18 prior false successes and 10 new invalid controls all reject. Ten historical nonpositive false successes also become errors. Twelve historical false successes remain explicitly listed. The 20 exporter failures are T14.20 nonpositives, **not positive matches and not a requirement to reproduce Java's crash**.

## All 12,550 goldens and broad gates

Every golden source was captured through actual CLI before and after, including all 1,521 sequence-family sources. Results are unchanged: **11,181 matches / 70 mismatches / 1,199 comparator nonpositives / 100 non-UTF8 reference-read failures**. The 100 are comparator panics while reading non-UTF8 references (including PNG-bearing `.svg` files), not Rust CLI panics. No valid golden newly errors and no pass is lost. Exactly 125 previously accepted nonpositive golden inputs now reject; their output bytes and diagnostics are preserved separately.

The single stable broad run reports:

- Library product: **11,181/11,251**; sequence **1,411/1,411**.
- Oracle-assisted library: **10,497/11,251** (754 failures, 1,299 exclusions).
- Parser **589**, renderer **801** (including **12 ASCII**), CLI pipe **13** tests pass.
- Guards **4/6**. Decimal ratchet and rejected/unreviewed-account gates remain red.
- Standing perturbations: **2,682 entries, 1,037 failures**, package cwd; separate from the repository-cwd CLI replay.
- Owned formatting passes. Workspace formatting remains red in existing class code. Clippy remains red in existing component dead code and renderer lint findings; full logs are retained.
- The product command remains nonzero because frozen baseline ratchets are not edited; the assisted command remains nonzero for its 754 existing whole-output failures. Their failure-name files were copied immediately and are **byte-identical to baseline** (`main-failure-name-comparison.json`).

`final-checks.json` contains exact commands, cwd and exit status. `logs/` contains compressed original logs. No baseline, comparator, exclusion, error census or allow-list was changed to make a gate green.

## AST, ASCII and YAML evidence

Full before/after stdout, stderr and exit statuses for all 3,569 retained and 12,550 golden inputs are in `captures.tar.gz`. Changes are counted separately from SVG comparison:

| Cohort | AST stdout changes | ASCII stdout changes | YAML stdout changes |
|---|---:|---:|---:|
| Retained 3,569 | 129 | 103 | 121 |
| Golden 12,550 | 132 | 111 | 119 |

See `model-summary.json`, `model-changes.json`, `successful-model-diffs.json` and `family-changes.json`. Seven successful golden AST differences are map iteration order only (DOT attributes/sprite map); the complete diffs are retained, not called byte-identical. Retained successful model changes include footbox state/bogus event removal, unsupported participant blocks no longer fabricated as messages, and legend/participant source-line metadata. YAML's existing process/error behavior differs from SVG and is reported on its own terms. No serialized model field was added.

There are 122 observed warning instances across retained plus golden CLI runs, over 22 command classes. `unimplemented-command-residue.json` records every class, source and diagnostic. The 81-row inventory lists all registered commands, including unexercised unsupported commands. Warnings are residue, not proof of support or whole-output correctness.

## Provenance and reproduction

All new references used the parent's existing Monkey Man relay; no local Java/javac, own SSH or new runner was used. Original source bytes, manifests, input bundles, raw streams/statuses, transport records and parent validations are preserved. The reference JAR SHA256 is `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, source revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`. Remote OpenJDK 21.0.5 and Graphviz 12.2 differ from origin; no global environment-equivalence claim is made. GMT+11:00, headless AWT, en_AU and UTF-8 are recorded per batch. Java argv is wrapper-derived, not process-traced. Probe source-file compilation has no separate javac invocation and is not independently traced.

- Probe gzip: `ce1cfbfba5d8bb77827ba043233f0324b552ab8249d2824ccb3992827f1cd476`.
- Render28 gzip: `0240e3dbdb0ff6139c1afb08885c9e3235ce4e94bd02003ab15350279ec6abb9`.
- Size8 gzip: `2a38010f72b553b3f45779b0a4b6949e8fd588e66a20aa883832500636da0483`.

`all-inputs.json` is an executable 3,569-entry manifest with new pairs relocated losslessly into `inputs/`. `capture-path-mapping.json` maps original private paths to those durable paths; raw requests/captures are never rewritten. `before.json.gz` and `after.json.gz` relocate only source/golden path fields, retaining all output diagnostics. The archive contains original manifests and raw captures, including candidate1's exact 15-case file, candidate2, failed unit runs and the failed size guard. Executable binaries are excluded; exact binary/source hashes remain in binding records.

Reproduce using the unchanged `docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py` with this manifest, separately built private binaries and a private output directory. Use `CARGO_BUILD_JOBS=4`; preserve original repository cwd and pinned stdlib/debug environment from scripts. Do not regenerate Java references or alter the comparator. `verify-package.py` checks archive hashes, durable source/reference hashes and original relay gzip identity without launching Java.

`implementation-map.json` contains every full account and implementation hash. `protected-evidence.json` verifies prior and imported evidence unchanged. `dependencies.json` preserves T14.21/T14.22, T14.20, T14.13, logical fonts, unsupported commands/style materialization, short external arrows, note/box/page envelopes and all other whole-output failures. This packet makes no sequence-closure or acceptance claim.
