# Independent sequence consumer-contract review: rejected

Independent checker: gpt-6-astra (high), Codex harness in T3 Code, task ending `aa41bca4-1418-4a0f-8f84-c25a34ef449d`. No prior involvement, children, production changes, account edits, target changes, baseline/comparator/skip/allow-list changes, or shipping. This checkout was clean-switched to unique branch `review/sequence-consumer-independent-b4b6` at evidence tip `9784e412146e3ece634d88ea0a1bddd6b42a82b6`.

**Introduced regression:** ten Java-valid `of`-alias note forms now fail with Rust exit 1. The optional `of` prefix greedily consumes the participant and the new empty-capture check errors. Java requires a participant and backtracks to capture `of`. All sixteen follow-up references are valid sequence SVGs. The ten affected cases previously produced mismatching SVGs, so this is ten new valid-input errors despite zero lost whole-output matches. Their library `diff_one` processes panic on the ParseError unwrap; the shipped CLI reports the errors without panicking.

Minimal case: declare `participant "Reserved Alias" as of`, send `of -> Kestrel: seed`, then `note over of : reserved identity`. The preserved follow-up matrix also covers colour, multiline, rnote/hnote and both engines, with tight-colon, bare multiline and pair controls.

## Revisions and integrity

- Tested before production: `61e6aeed6eaf686b3497fbc696aadc03a64e69a2`; source-identical to baseline evidence `bce6bea405d8b5efd55136f4768117115e7f634f`.
- Tested after production: `d491d4bda47faf90d4751484d45aa1387fb1b290`; source-identical to the evidence tip.
- Accounts `0b512262aaf4bcf0c0ab144b9f28c0719f329e4c` precede all four implementations, whose commit messages reference that account commit.
- Rejection `f3913f3cb7094d8aedd5618f1ba27830a2804713` was imported unchanged as `b286fe630c92ee33bfea4bd1b6aa7307ce6e1a6a`. All 9,499 prior evidence paths and 487 imported paths are byte-exact.
- Only `parse/sequence.rs` and `render/sequence.rs` changed among existing paths. No model, ASCII, shared, factory-dispatch or target change.
- Every regex source-map hash and account-cited Java file matches pinned revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`. Two cited methods **do not exist**: `CommandParticipant#getRegexConcat` and `DrawableSetInitializer#createParticipant`. The actual declaration grammar is in subclasses such as `CommandParticipantA`; the initializer method is `prepareParticipant`. These account provenance defects are recorded without rewriting the accounts.
- No fixture-shaped logic, coordinate compensation, golden embedding, or unexplained new numeric calibration was found in the production diff.
- Measurement-label recovery was metadata-only: the original 2,519 rows, private manifest input fields, and all SVG captures are identical after removing only cohort labels. The original summary-print KeyError remains preserved. The recovery script combines existing captures; it does not render.

## Actual CLI replay

Each row is **match / SVG mismatch / valid CLI error / nonpositive reference**. Duplicates and nonpositives remain in the denominator.

| Cohort | Entries | Before | After |
|---|---:|---|---|
| Frozen | 2,371 | 1674 / 616 / 2 / 79 | 1674 / 616 / 2 / 79 |
| Imported checker | 148 | 94 / 42 / 0 / 12 | 115 / 21 / 0 / 12 |
| Maker fresh | 30 | 4 / 18 / 0 / 8 | 20 / 2 / 0 / 8 |
| Total | 2,549 | 1772 / 676 / 2 / 99 | 1809 / 639 / 2 / 99 |

These are independent actual-CLI processes using private exact release builds and the unchanged comparator. All output SHA256s and comparator/library mode exits reproduce the maker's measurements. There are 2,501 unique source byte strings and 48 duplicate entries. All three comparison modes recover 37 positive matches and lose none. No new valid CLI error or CLI SVG panic appears in this frozen replay.

The unchanged `rejection_contract.py` independently reproduces 65 → 77 rejections among 99 nonpositive entries, with zero lost rejection. Twenty-two false accepts remain. They include the two `message_gap` inputs: regex refusal, then `unconsumed=true`, then `parse_sequence` returns `finish()` without testing that flag. These remain T14.17, not SVG passes. The twenty frozen nonpositive successes remain separate.

All 2,371 frozen AST/ASCII/YAML stdout captures are byte-identical. Fourteen non-frozen diagnostic changes remain: twelve new rejections and two omitted malformed whitespace messages. The nineteen nested-enum CLI YAML panics pre-exist. The main short-question helper panics are diagnostic unwraps; the CLI reports its two pre-existing valid-input errors cleanly. Other frozen helper failures are also retained in the raw evidence rather than hidden.

## New held-out evidence and verdicts

The checker generated 534 entries (530 unique source byte strings; four duplicate entries retained), disjoint from all 2,549 prior input bytes. Every requested source and same-stem Java SVG is preserved under its mechanism's `test-diagrams/perturbations/` directory, including every failure and nonpositive. Both relay batches preserve exact Unicode/control bytes and original intended validity.

| Mechanism | Entries | Before match / mismatch / valid error / nonpositive | After | Verdict |
|---|---:|---|---|---|
| Pattern2 whitespace | 294 | 179 / 63 / 0 / 52 | 179 / 63 / 0 / 52 | Rejected |
| All-participant Display | 128 | 47 / 81 / 0 / 0 | 119 / 9 / 0 / 0 | Rejected |
| Teoz self-note component | 48 | 10 / 38 / 0 / 0 | 26 / 22 / 0 / 0 | Rejected |
| Note-over domain | 64 | 24 / 32 / 0 / 8 | 24 / 22 / 10 / 8 | Rejected |
| Total | 534 | 260 / 214 / 0 / 60 | 348 / 116 / 10 / 60 | Rejected |

All three modes gain 88 whole-output matches and lose none. There are ten introduced valid-input errors and zero CLI SVG panics. No lost rejection occurs. Of 52 fresh Java-invalid inputs, Rust rejections improve 2 → 36; sixteen still falsely succeed. Eight additional nonpositive references are Java exporter failures: exit 0, error SVG and XML-invalid-character exceptions for VT/FF divider/delay text. Rust exits 0 for those eight before/after; they are not syntax-rejection evidence or SVG passes.

The first relay contains sixteen intended-valid inputs that Java rejects. Original intent remains unchanged. Eight are only-VT command separators: `RegexLeaf`/`RegexConcat` apply a required whitespace FoxSignature bit before the regex, but `FoxSignature` omits VT. Six are checker colour/order probes with the invalid option order (`order` follows terminal colour). Two are deliberate `accross` probes taken from the regex; the factory's `executeInternal` explicitly rejects that spelling. Eight further intended-valid cases hit the exporter failures above. `classification-refinement.json` records the metadata-only distinction between syntax/semantic errors and exporter errors; no process was rerun for this labeling.

The whitespace class transformation itself is source-supported, including negation, but whole-command parity is not established: literal-space entry checks for box/across/autonumber resume and Java's VT prefilter remain. Other Unicode label metric failures, short-arrow behavior, and frozen failures remain explicit pre-existing residue.

Each participant kind preserves its prior positive matches. The nine remaining Display failures are nested Teoz boxes for all eight kinds plus created control in legacy. Legacy/nonself/named-over self-note controls are byte-identical before/after, as are all 48 self-note AST/YAML/ASCII captures. Twenty-two self/nonself envelope/ordinate failures remain. Whole-output failure rejects each mechanism; bounded support does not waive these failures or transfer historical acceptance.

## Broad gates and setup

The stable broad commands ran **once per exact production revision**, with independent private release targets and jobs 4. Full commands, environments, statuses and immediately copied failure-name files are retained.

After: parser 581, renderer 800 (including 12 ASCII), CLI 13 pass. Main library product 11181/11251; assisted 10497/11251 (12550 total, 1299 skipped); sequence 1411/1411. Both failure-name files are byte-identical before/after and match the maker. These are library diagnostics, not 11,251 CLI processes. Standing after: 1031/1724, red. Guards: 4/6, red. Owned formatting passes; class workspace formatting, parser component dead-code clippy and all nineteen renderer clippy findings are pre-existing. Clippy error messages are identical before/after.

The exact before production contains 1,576 standing inputs and scores 916/1576; the after snapshot includes the 148 imported checker inputs and scores 1031/1724. Those standing denominators must not be presented as like-for-like. The 2,549-entry replay is the frozen comparison. Fresh inputs were compared separately; the stable broad gates were not rerun after adding review evidence.

Golden submodule is exactly `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Initial setup failed because the configured local submodule transport was disallowed; a command-scoped file-transport retry hydrated the pinned commit. One replay setup attempt reached the unavailable maker-30 paths while the checkout was detached at production; that failure and partial captures are retained. The completed replay binds those unchanged source/reference bytes to their immutable maker checkout paths. Sawmill's documented single-file parse failed with a not-a-directory error; exact Git/source inspection was used instead. None of these setup failures is claimed as a product result.

## Reference provenance and bounded next obligations

New Java outputs came only through the parent's existing Monkey Man relay. Batch T: 518 cases, response gzip SHA256 `36104c4215d5f28ac816f2f0298a9f80cf5fb985881f2ca4b93db941ebefe527`. Batch U: 16 cases, response gzip SHA256 `3b15a1191e5655f298111dc846fb58774dffc3ead2600804b71e2ddf4ade1724`. All source/stdin/stream hashes, manifest bytes, original bundle fields, transport envelopes, JSON-array argv, private cwd, GMT+11:00, env {}, and pinned JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053` were independently checked. No timeouts. Java argv is derived, not process-traced. OpenJDK 21.0.5 / Graphviz 12.2 historical equivalence remains unproved.

The next bounded obligations are to repair the optional-`of` grammar without losing bare-over rejections, correct provenance through new append-only evidence, and address or explicitly retain the demonstrated command-pipeline, box/creation, stereotype and note-envelope failures before a new independent acceptance review. The existing T14.17/T14.13/font/TIM and other named dependencies remain failures. No combined correctness with the concurrent creole fallback packet is claimed.

See the four new per-mechanism `review.json` records, `fresh-pairs.json`, `source-audit.json`, `packet-audit.json`, the summaries and `captures.tar.gz`. `capture-inventory.json` binds every raw archived file. `support/` retains the exact replay/comparator/rejection adapters and the checker's generation, validation, audit and packaging code.
