# Sequence decoration and complete child allocation — maker packet

This is implementation evidence, **not an acceptance or checker record**. All historical verdicts remain attached to their exact tested revisions. The imported independent rejection and its pairs are unchanged.

The final real-CLI cohort has **1,350 matches, 569 valid mismatches and 47 invalid Java references out of 1,966 entries**. The baseline has 1,319 matches and 600 valid mismatches: **31 recoveries, no lost passes, no new valid-input CLI errors or panics**. Invalid sources are separate: 35 rejected and 12 still accepted, versus 31/16 before; rejection does not count as SVG parity. Both library comparison modes also have no lost passes. Each retains the existing `diff_one` comparison panic on the empty invalid TIM reference `checker_invalid_overflow_seconds`; it is not a CLI or renderer panic.

| Cohort | Entries | Before match / valid mismatch / invalid | After |
|---|---:|---:|---:|
| Preserved maker/checker inventory | 1,901 | 1,299 / 559 / 43 | 1,301 / 557 / 43 |
| Fresh independent checker | 37 | 17 / 18 / 2 | 25 / 10 / 2 |
| New decoration/height/style controls | 24 | 3 / 19 / 2 | 20 / 2 / 2 |
| New underline/rule controls | 4 | 0 / 4 / 0 | 4 / 0 / 0 |

There are 1,966 distinct source paths and 1,918 distinct source byte strings; all 48 duplicate entries remain measured. The checker registered 39 pairs, including two copies of prior maker failures; only its 37 fresh sources are added to the preserved 1,901. The frozen parent 1,449 overlaps the original cohort in 1,123 sources, with 326 parent-only sources not rerun here. Do not sum these overlapping populations. No unrelated parent production was imported.

## Changes and chronology

`implementation-map.json` records full hashes and file ownership. Evidence import: `eaedee419da43b38fb91d8848cea39d325c55d1c`, cherry-picked from `fdf1154eb3a17ebb041fffb0c18710a38210f56b`.

* Preimplementation account `0b3295821b0aeef0d5fc8b190cd57aadba8ca0e0` covers three separate account files: target circles, ordinary-note child allocation and component style consumption. Implementation `350e776a2cd376dd2171dc79b61010da8f01fc2e` separates destination decoration from head dressing, carries circle geometry through ordinary/self/external components, rejects interior circle ordering, uses complete child heights for legacy ordinary notes, and resolves inherited component colors and plain-participant numeric shadow through the existing shared cascade. The activationBox fallback stays independent of its lifeLine sibling. The two ASCII changes are **only** the authorized `target_circle: false` constructor fields.
* The first broad run found one real regression, `creole_underline_in_sequence`, while sequence remained 1,411/1,411. That failed run is preserved under `validation350e776a` in `captures.tar.gz`, along with before/intermediate/final real CLI outputs in `main-regression`.
* Preimplementation account `54fb67fce0c99cab0152e748b033099feb8462ca` identifies synthetic underline-message flow plus duplicate folded-note tail reservation. Account `6e9c711d5984185796e2f9c0ae0c8986d0668649` refines the note path to `BodyEnhanced2` title wrappers. Implementation `026e5eeb2f3521ec3a2f299b30d52a4cac51f05c` removes duplicate flow and derives rule height/paint from title metrics. All four new controls initially failed; their intermediate measurements are retained. All four now match in both engines with differing arrow/note fonts.
* The final consumer audit also applies the same plain-participant shadow height to Teoz creation, under the original component account. Its implementation hash and final source binding are recorded in `implementation-map.json`. This small production follow-up justified the final validation repetition; it does not establish complete per-kind style coverage.

No shared font, text, style, layout, class, preprocessing, harness, comparator, baseline, target or review-attestation file was edited. `protected-evidence.json` verifies imported evidence byte-for-byte and lists the four modified existing files.

## Verification

* Main **library** tiers: product 11,181/11,251; assisted 10,497/11,251 eligible (12,550 total, 1,299 skipped); sequence 1,411/1,411. Both failure-name sets equal the baseline. These are not 11,251 CLI processes.
* Parser 570, renderer 790 (including 12 ASCII tests), CLI pipe tests 13: all pass. New tests cover independent decoration/head ownership, serde compatibility, grammar negatives, component-style siblings and underline-height invariance.
* Built-in perturbations: 686/1,279 match, 593 fail at Cargo package cwd. The unchanged suite preserves invalid-reference failures. Isolated CLI captures use repo cwd; the established T14.6.2 theme include discrepancy remains documented.
* Owned-file formatting passes. Workspace formatting fails in the pre-existing class parser/renderer areas. Clippy remains red on parser component dead code and 20 existing renderer findings. Guards remain 4/6; frozen decimal/evidence ratchets and rejected/unreviewed records were not changed.

`checks.json` contains exact commands, cwd and exits; compressed logs are in `logs/`. `binaries.json` binds the preserved baseline executables and final binaries to source hashes. Build metadata retains its actual checkout revision, including dirty-source builds; committed production equality is established by file SHA256, not invented build chronology.

## References and replay

No local Java/Javac or SSH was launched. Parent relayed immutable inputs to the pinned Monkey Man headless wrapper. `reference-provenance.json` retains the exact environments, hashes and worker paths. The JAR is `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`; remote OpenJDK 21.0.5 and Graphviz 12.2.0 differ from the origin. Earlier seven-case transport/conformance agreement is scoped, not global equivalence.

The 24-case return gzip is `3730fb26722f9766bd410b986c5813fc3c9b46f6c70f6d4f78c1b55feda0dcf2`; the four-case return gzip is `3b9b1d1b5905d1779bbaabab0c466bd3b7b95ff61d8f92fef5646f24e4a9c63b`. Both are verified against the returned transport bytes. All 28 memberships, current source hashes, stdout/stderr hashes and XML/status classifications are preserved. The 24 comprise 22 valid and two intended exit-200 negatives; all four additional inputs are valid. Source bytes were never rewritten after reference capture.

`all-inputs.json` is the relocatable replay manifest; new exact source/SVG pairs are in `inputs/`. Raw measurements keep original capture paths and hashes. Run the unchanged `docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py MANIFEST BIN_DIR PRIVATE_OUTPUT [CWD]`; output must be a new nonsynced private directory. Packaged maker scripts describe this run and its private cache; copy them to private scratch before adapting them. They do not authorize reference regeneration or alter the comparator.

## Remaining dependencies

Every nonzero input is enumerated in `remaining-failures.json.gz`; `negative-status.json` separates invalid evidence. `dependencies.json` supplies source-level handoffs. In particular:

* Both `circle_both` sources remain Java-valid CLI failures. `parse/mod.rs:642` awards class10/object5 for `o--` even when `sequence::looks_like_message` recognizes `o-->>o`; CLI `--ast` is Class and the SVG is blank. Both ASTs, source bytes and Java references are retained. Parent acknowledged the unowned dispatch dependency. There is no shipped-path credit from a direct sequence-only diagnostic.
* Per-atom tab origins, the second Display conversion inside URL labels, natural `\r`/`\l` alignment and both logical-font controls remain unresolved. No geometry compensation or host-font import was used.
* Themed self-note padder/Real/envelope paths, per-kind shadow/style consumers, page chrome/legend/styled captions, hexnote bounds, right-note canvas, multiline delay and boxPadding/nested englobers remain next-packet concerns. The root/participant/lifeLine/arrow/note/activation controls do not prove all styles, stereotypes or event histories.

The packet stops at local commits for a fresh independent review. No whole-sequence or extracted acceptance is claimed.
