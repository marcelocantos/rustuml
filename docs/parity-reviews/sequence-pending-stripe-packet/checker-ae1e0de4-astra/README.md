# Independent sequence pending CREATE / stripe review

**Packet rejected.** Tested production `653b1ed7a49784e6e6205f3aebf69a16278f8d78`, evidence tip `ae1e0de4d39840dd58074d89d40aa97304c6ca9f`; exact before packet `711dbca7c5e29592395f0927881aabfc2517b217` / production `a4c3c35794ae1d7117d709134ed22c7191db9db8`. New independent gpt-6-astra checker; evidence-only branch. No production fix or shared packet import.

## Disproof

Four fresh Java-valid outgoing external pending-CREATE sources were whole CLI matches before, and now panic at `crates/rustuml-render/src/sequence.rs:14287:81` (`Option::unwrap` on missing border target). Both border directions and both engines fail. Path: parser `validate_return_destinations` -> lowered `sequence_create_attachments` -> renderer `sequence_create_message_indices` -> `create_msg_idx` -> `created_inline` / `is_create_msg` -> `id_to_idx[msg.to].unwrap()`. The sole real participant is the source on an outgoing MessageExo. Java's `Step1MessageExo` / `CommunicationExoTile` do not use the ordinary creation-head path. Simply accepting ownership is not a complete render-model port.

All four sources and exact raw Java SVGs remain executable under `test-diagrams/perturbations/sequence-single-pending-create/`. The stable 2169 cohort did not expose these failures.

## Exact shipped CLI replay

| Cohort | Entries | Before match / valid-fail / invalid | After |
|---|---:|---:|---:|
| Preserved | 2083 | 1448 / 576 / 59 | 1448 / 576 / 59 |
| Checker | 64 | 42 / 16 / 6 | 50 / 8 / 6 |
| Maker | 20 | 0 / 16 / 4 | 16 / 0 / 4 |
| Shared handoff | 2 | 0 / 2 / 0 | 0 / 2 / 0 |
| Total | 2169 | 1490 / 610 / 69 | 1514 / 586 / 69 |

All maker CLI SVG hashes and all CLI/product-library/assisted-library status codes reproduce exactly. There are 2121 distinct source byte strings and 48 retained duplicates. The frozen cohort has 24 recoveries, zero lost passes in every mode, zero new valid parse errors and zero renderer panics. Invalid references remain in the denominator: Rust reject/accept changes 45/24 to 49/20. Two empty-reference `diff_one` comparison-helper panics are preserved separately, not relabeled renderer panics.

Parent1449 is not additive: 1123 overlapping paths and 326 unreplayed paths. The overlap has1097 distinct source byte strings; the326 paths contain286 distinct strings, of which278 are exclusive (8 duplicate overlapping bytes). No unreplayed path is credited.

## Fresh evidence and verdicts

The first86 and followup10 are distinct private relay batches:96 fresh entries,88 Java-valid and8 command-error references. All streams were hash-checked and XML/metadata/diagnostic/comparator-classified independently; zero timeouts or intention disagreements. Before:45 matches,43 valid failures,8 invalid. After:67 matches,21 valid failures (17 SVG differences plus4 renderer panics),8 invalid. There are26 recoveries and4 lost passes. No new Java-valid parse rejection; the four new runtime errors are renderer panics. No case is removed for failing.

| Mechanism | Fresh whole matches / valid entries | Verdict |
|---|---:|---|
| Single pending CREATE | 19/30 (+8 invalid retained) | Rejected |
| Queue stripe origin | 8/14 | Rejected |
| Message stripe baselines | 12/16 | Rejected |
| Self half-polygon order | 24/24 | Accepted, bounded to this matrix |
| Collections citation | 4/4 | Accepted, citation-only |

Queue failure is causal: the AST retains `dispatch`, while the unboxed/created queue paints zero stereotype labels instead of Java's two. Rust gates `stereo_ref` and height on `in_box`; Java `Participant.getDisplay` and `ComponentRoseQueue` pass the complete Display. All six missing-stereotype cases already failed before. Heading stripe controls likewise already fail: Java paints `Crest`/`Ridge`; Rust paints literal heading markers. Notes and dividers use separate Rust consumers, determined from source, and remain control failures. Neither old failure is attributed as a new regression, but neither is waived.

Both shared handoffs remain whole failures. All586 frozen valid failures remain listed in `remaining-failures.json`: reverse-created queue legacy; unclosed groups; mixed-kind Teoz boxes; multiline/nested boxes and englobers; ancestor-sum envelopes; tabs, URL Display, fonts, themes, shadows, notes, delays, chrome and factory cases. The TIM welcome handoff is separately replayed from exact historical commit `0f00130a26b2f300996d0bd427f9e70b4ef10e30`: two welcome-truncation references, both comparator-invalid/nonpositive and both revisions rendering Class. They are neither added to2169 nor called positive whole-output failures. No shared style/WBS combined correctness is claimed.

## Counts beyond whole matches

Real `--ast -pipe`, `-ttxt -pipe` and full SVG captures cover every2169 retained entry and every96 fresh entry. The maker148 subset reproduces4 ASCII parse rejections and144 unchanged. Across all2169, exactly4 AST and4 ASCII outcomes change (those invalids); across96 fresh,8 invalid rejections change, with88 valid AST/ASCII outputs unchanged. Four of those88 SVG renders panic, demonstrating why AST/ASCII success is not SVG evidence.

`semantic-inventory.json.gz` preserves Java/before/after tag counts, exact label multiplicities, available participant groups, text positions, polygon vertices and raw AST event counts. Inline created heads are not always grouped, so group counts alone are explicitly incomplete. The double-pending witness changes Roe labels3 to2; maker actor witness changes4 to3 while retaining3 CREATE events. All frozen valid increased-excess tag/label checks are empty. The full raw outputs are the evidence, not that diagnostic alone.

## Gates and provenance

Private release targets were built separately with jobs4. The golden submodule content was hydrated from the local clone at exact `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`; no local JVM ran. No setup/build failure occurred. Stable required gates ran ONCE per revision before evidence registration:

- Before parser574, renderer793; after parser575, renderer795 (12 ASCII), CLI13 in both.
- Both main library product tiers11181/11251, assisted10497/11251, sequence1411/1411. These are library diagnostics, not11251 CLI processes.
- Before standing744/1352; after794/1416,622 failures. Both guards4/6. All remain red; frozen inputs were not changed.
- Existing class-file workspace formatting, parser component dead-code Clippy and19 renderer Clippy findings reproduce. Exact failure-name snapshots were copied immediately per revision and are byte-identical.

All8462 baseline evidence paths and183 imported paths are byte-identical. All five accounts first appear in4aaab169 before their implementation commits, whose messages reference that account. Only three allowed sequence production files differ from baseline. No serialized field, parse/mod, ASCII, shared text/style/font, class, TIM, native, comparator, skip, baseline, allow-list or target changed. The collections function is unchanged after removing comments. No new fixture-keyed production branch or unexplained layout constant was found. All nine imported claims stay rejected.

Java source is pinned to71806a23780b04a5ccde2f8ceb5121edad5eb711 and hashed against the read-only clone; JAR SHAaa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053. All new Java references came through the parent's EXISTING relay, workersYD4ftmgc andrWaHIBFO, GMT+11:00. Java argv is wrapper-derived, not process-traced. Remote OpenJDK21.0.5 and Graphviz12.2 do not prove historical font/Graphviz equivalence.

## Reproduction and bounded obligations

`build_gates.py`, `replay.py`, `fresh_inputs.py`, `followup_inputs.py`, relay manifests, classification scripts, raw streams/status/provenance, exact commands and binary bindings are retained. `execution-captures.tar.gz` contains full before/after SVGs, ASTs, ASCII, comparator/library diagnostics and immediate gate snapshots; `archive-files.json` hashes every member. Absolute paths record the actual commands; remap paths to a new private cache for reuse. Do not replay generation through a new JVM/runner. The standing perturbation test discovers all new pairs recursively; no comparator or gate edit is required. Broad gates were not repeated after evidence-only registration.

Next work must separately model ordinary versus external creation consumers, complete queue Display/stereotype composition, and resolve heading/shared Display semantics while retaining the counterexamples. The two bounded acceptances do not accept the packet or transfer historical acceptance. All old reviews and accounts remain intact.
