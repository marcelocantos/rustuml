# Independent review of sequence component/display packet 711dbca7

**Rejected.** Independent checker: gpt-6-astra through Codex/T3, prepared isolated checkout `t3-5c023380`, branch `review/sequence-711dbca7-5c023380`. No implementation, account, old evidence, target, baseline, comparator, skip, allow-list or shipping edits. No child, local Java/Javac, SSH, new remote runner or native/fault work.

Exact packet: `711dbca7c5e29592395f0927881aabfc2517b217`; production: `a4c3c35794ae1d7117d709134ed22c7191db9db8`; before: `672cd8f26e1d7c6725d8971e01ad7575defd8567`; intermediate: `cf3eafb293de2f01699399321d2220c4de9eb28c`. Packet and final production have identical crates/Cargo inputs. Nine new mechanism-specific `review.json` records are indexed in `review-index.json`. The checker did not implement these changes. Same model family as maker; independent task, source audit and new inputs, not a claim of model diversity.

## Causal rejection findings

1. **Pending CREATE is overwritten, not queued.** Java `SequenceDiagram.activate` assigns one `pendingCreate`; `addMessage` consumes and clears it. Rust's `sequence_create_message_indices` queues IDs. Both same-participant and different-participant consecutive CREATE controls are Java-valid and fail in both engines. In the same-participant case Java and before Rust paint the `Roe` label twice, while final Rust paints it three times: one additional created head. This is an introduced semantic regression inside an already failing whole-output case. Zero lost whole-case passes must not be reported as zero regressions.
2. **The first queue stripe uses the wrong baseline.** `ParticipantTextBlock` measures a size 25 first stripe correctly, but `render_participant_shape` anchors it using the base font's ascent. Its `draw` subtracts the actual first-stripe ascent, shifting both queue label lines upward by 10.6348. `USymbolQueue.asSmall` translates the complete block by marginY1=5. Both engines fail the new first-stripe cases.
3. **Whole reverse-self outputs still fail.** The filled TOP_PART triangle's point order differs from `ComponentRoseSelfArrow.getPolygon`. The vertex set is identical, but the frozen strict comparator rejects it. This does not justify changing comparator slack or undoing independent ordinary dressing ownership.
4. **Whole box/creation envelopes remain incomplete.** Reverse created queue legacy width is 213 versus Java 223. Mixed Teoz boxes have horizontal-envelope failures. Multiline englober titles still use a fixed single-line height, and Teoz nested Dolls do not sum ancestor title heights. Java-valid unclosed groups also fail in both engines. These are explicit failures, not scope waivers.
5. **One new source citation names a nonexistent Java class.** The `ComponentRoseCollections` comment should name `ComponentRoseParticipant(collections=true)` reached through `Rose.createComponentParticipant`. Its margins 7 and collection delta 4 are source-derived, but the adjacent class name is inaccurate.

`source-audit.json` gives the nine causal findings and constant provenance. No newly added fixture-label branch, golden read/embed, branch-count ladder or SVG-coordinate surgery was found in the three-file diff. This is bounded inspection, not certification of untouched renderer history.

Whitespace-title space atoms and kind-specific font-size consumers each pass all four fresh assigned cases. These are bounded positive observations; their records withhold acceptance because the required standing perturbation and guard gates are red. They do not transfer historical acceptance or waive any failing whole case.

## Real CLI evidence

All runs use independently built release binaries, private per-revision Cargo targets, and at most four Cargo jobs. The unchanged comparator adapter links the exact revision's oracle library. Full SVGs, statuses, stderr and library diagnostics are preserved in `execution-captures.tar.gz`. CLI results are distinguished from library-only tiers.

The frozen membership is **2,083 entries, 2,035 distinct source-byte strings, 48 duplicate entries**:

| Cohort | Entries | Before match / valid failure / invalid | Final match / valid failure / invalid |
|---|---:|---:|---:|
| Preserved | 2021 | 1401 / 569 / 51 | 1401 / 569 / 51 |
| Checker | 42 | 17 / 19 / 6 | 30 / 6 / 6 |
| Maker | 20 | 5 / 13 / 2 | 17 / 1 / 2 |
| Total | 2083 | 1423 / 601 / 59 | 1448 / 576 / 59 |

The frozen replay reproduces 25 gains and zero lost whole passes. No new valid-input errors, CLI panics or library-render panics were observed. All 576 remaining valid failures are enumerated with full comparison output in `remaining-failures.json.gz`. Of 59 invalid references, Rust rejects 43/accepts 16, versus 39/20 before; four newly rejected inputs are unclosed notes. `diff_one` still panics comparing two empty invalid references in each diagnostic mode; those are comparison-helper panics, not renderer panics or valid SVG credit.

The parent 1,449 corpus is **not additive**: 1,123 source paths overlap, 326 parent paths were not replayed. Its 1,375 distinct source bytes overlap 1,097; 278 distinct byte strings were not replayed. `parent-overlap.json` computes this from immutable historical `original_source` and source SHA256 records and current source hashes. No fresh execution of those 326 is claimed.

### Fresh matrix

The checker authored **64 new distinct byte strings**, disjoint from the 2,083 membership, across normal/reverse/self/external arrows, head/open/circle/half combinations, repeated/overwritten CREATEs, live scope/direction, multiline/blank/stereotype/rich-size participant/queue/figure labels, aliases, kind font overrides, closure contrasts and nested/mixed boxes, in both engines.

- Original 52: 46 Java-valid,6 invalid;8→34 valid matches,12 valid whole failures.
- Supplemental 12: 12 Java-valid;2→8 valid matches,4 valid whole failures.
- Combined 64: 58 valid,6 invalid;10→42 matches,32 gains,16 valid failures and zero lost whole passes. The extra-head regression above remains explicit despite the latter metric.

Six intention/observation disagreements are retained: two ordinary combination inputs and two literal-empty queue-label inputs were intended valid but Java rejected them; two double-pending CREATE inputs were intended invalid but Java accepted them. The two unclosed-note negatives behaved as intended. Four invalid sources are accepted by Rust; two notes are rejected. All original invalid inputs and raw Java error SVGs remain alongside the 12 new supplemental sources. No source was repaired in place to manufacture a pass.

All64 source/Java SVG pairs live under `test-diagrams/perturbations/sequence-component-display-review711/`, which the existing recursive perturbation harness executes automatically. `fresh-summary.json` enumerates every valid failure; the mechanism records refer to the exact pairs. Fresh inputs use no includes, resources or timestamps. Registration did not trigger another broad run; the exact fresh whole CLI/library comparisons were already executed.

Actual `--ast -pipe` and `-ttxt -pipe` captures are retained for the 62 prior controls and 64 fresh inputs. The prior 62 reproduce the maker exactly, including 14 changed ASCII outputs. Untouched ASCII code is not behavioral invariance, and matching SVG does not establish ASCII parity.

## Stable gates and intermediate regression

The exact golden revision `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` was absent initially, then hydrated from the existing local clone's exact git archive. Every extracted blob was checked. No suite was allowed to pass on an empty corpus. Setup/tooling limitations are preserved in `setup-notes.json`.

| Exact revision | Product library | Assisted library | Sequence library |
|---|---:|---:|---:|
| Before 672cd8f | 11181/11251 | 10497/11251 | 1411/1411 |
| Intermediate cf3eafb | 11180/11251 | 10496/11251 | 1410/1411 |
| Final 711dbca7 (production a4c3) | 11181/11251 | 10497/11251 | 1411/1411 |

The assisted total is 12,550 with 1,299 skipped, leaving 11,251 eligible. These are library executions, not 11,251 CLI subprocesses. Exact before/intermediate/after private targets prevent stale-binary attribution. `seq_box_with_queue` independently reproduces pass/fail/pass; all 68 valid members of the frozen 70 main-queue selection pass at final, with 2 invalids separate. The frozen 70 selection is not an exhaustive inventory of every source containing a queue.

Final stable required gates were run once before new evidence registration: parser 574, renderer 793 (including 12 ASCII) and CLI 13 pass. Product library still fails unlocked ratchets; assisted retains 754 failures. Standing perturbations are 744/1352 with 608 failures at Cargo-package cwd, while the CLI replay uses repository cwd. Guards 4/6 remain red. Owned-file formatting passes; workspace formatting and Clippy remain red (parser dead code, 19 renderer findings). No baseline, threshold, exclusion or comparator was changed. Before/intermediate main tiers ran once per distinct production revision to attribute the named regression, not to repeat final evidence registration.

Historical maker first 62 self regressions remain two losses in their draft captures. The old `display-bin` capture has no simultaneous full source binding and remains unbound historical evidence. This review neither recertifies that draft nor treats it as an exact-revision run.

## Provenance and remaining obligations

All 8,221 prior and 103 imported evidence paths are byte-identical. The nine exact account commits precede their mapped implementations and their account blobs are unchanged. Only parser diagram/sequence, parse/sequence and renderer sequence changed in production; there is no new serialized field or parse/mod/ASCII edit. `preservation-and-chronology.json` retains the blob inventory and ancestor checks.

New Java was produced solely through the parent's existing Monkey Man relay, with source revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`, JAR SHA256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, GMT+11:00, private cwd and resources[]. Immutable manifests preserve exact source bytes/base64/hash, absolute and repository path identities, and intention. The two result archives preserve every actual raw stdout/stderr, status and timeout. The checker independently verified these against the manifests; no timeouts occurred. Wrapper argv is observed; Java argv is derived from the pinned wrapper construction, **not process-traced**. Historical font/Graphviz equivalence is unproved.

All existing whole failures remain obligations: factory quoted header/quote/UID order, component/bare-circle, accepted invalid factory commands, unclosed-group/centered-note/boxed-queue canvases, tabs, URL second Display, logical fonts, themes, style signatures, hexnote, multiline delay and nested boxes. The sprite handoff's message guillemet conversion is still not fixed. Its raw source/reference remain in the maker's `dependency-handoff`, excluded from 2,083; no shared sprite production was imported and no combined correctness is claimed. Shared blank-stripe/absent-Display and note resolver work remain pending.

For reproduction, extract `execution-captures.tar.gz` into nonsynced private scratch, adapt only `measure.py`'s repository root, build each exact revision as documented in `builds.py`, and use `all2083.json.gz` or `fresh64.json` with the matching private binaries. Exact build/test argv, cwd, environment overrides, compiler version and binary/source hashes are preserved separately. No shipping occurred.

The tests overwrite ignored failure-name files between revisions. The retained name files are bound only to the last intermediate run; independent before/final equality of those exact name sets is not claimed. Console counts and full CLI replay failures are preserved. See `diagnostic-capture-limitations.json`.
