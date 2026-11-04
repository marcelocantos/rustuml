# Independent review of 154d4c7

All three new mechanisms are **rejected**. This is evidence-only review of exact candidate `154d4c7fd58bc7a1b91df7d01f530b3ea911a5fc`, whose production files equal `17a3705e17cc79640a1bcfea241b61f9b762d957`, against before `eb09cecbc90f5690c4a09bf49118f2feff8c1c12`.

The separate checker task used Codex `gpt-6-astra` with high reasoning effort. It worked alone, implemented none of the candidate, and changed no old account, review, production file, baseline, comparator, skip rule or target. Read [audit.md](audit.md) for source-level causal findings and explicit dependencies. The three new schema-shaped records are:

- [Checked quark context](../review.json): 5 matches, 3 mismatches, 1 valid-input process error, 4 invalid references in 13 fresh controls.
- [Note URL lifecycle](../../class-note-url-command-lifecycle/review.json): 4 matches, 3 mismatches, 2 invalid references in 9 fresh controls.
- [Fashion conversion](../../class-fashion-value-digit-semantics/review.json): 9 matches and 2 mismatches in 11 fresh controls.

All six new invalid references are correctly rejected, but are not parity successes. All 33 exact source/reference pairs are under their respective `test-diagrams/perturbations/<mechanism>/checker-0423fc37/` directories. They remain executable through the unchanged standing perturbation suite. Negative error references remain explicit failures under its existing acceptance rules; no waiver is applied.

## Findings that prevent acceptance

Root/local notes retain identities, UIDs and URL owners but have the wrong paint order. Null-separator lookup into a root note preserves an empty package in YAML while the lone-note rendering path omits its painted image. A smart-quoted attached-note target is accepted by Java but rejected by Rust. A multiline note URL containing spaces is truncated to `“docs` rather than `docs with spaces`. Group-note geometry still differs after anchor ownership is corrected.

The Fashion account's blanket legacy yes/no rule is false for `actorShadowing yes`: Java's mapped-key conversion bypasses the yes/no fallback and yields 1.5; Rust yields 3. The actor label and following class move by 1.5, and the viewport differs. A separate fresh negative-number control still lacks the native class's shadow filter. These are observed whole-case failures, including preexisting defects, not claims of new regressions.

The six packet7 failures remain failures: `root_class_local_note`, `root_note_local_class`, `null_separator_lookup`, `group_single`, `group_multi`, and `actor_legacy_negative`. The full retained CLI replay preserves their exact source/reference and captures in the archive. Model/DOM agreement cannot waive a whole-case failure.

## Independent CLI ledger

The comparator is unchanged, SHA256 `0e368102f36bfbc59efe141286df5935b785535db6c0cf537bdbadc9350f21cd`. Each cell below is matches / mismatches / valid process errors / invalid Java references.

| Frozen cohort | Entries | Before | Candidate |
|---|---:|---:|---:|
| Frozen1761 | 1761 | 1060 / 569 / 62 / 70 | 1065 / 566 / 60 / 70 |
| Maker fresh32 | 32 | 7 / 16 / 3 / 6 | 20 / 6 / 0 / 6 |
| Independent fresh33 | 33 | 6 / 18 / 3 / 6 | 18 / 8 / 1 / 6 |

Every maker-ledger before/after status and CLI exit is reproduced by independently built private release binaries. No passing comparison is lost and no new valid process error appears. Of the retained 1,793 YAML/status pairs, 1,772 are byte-unchanged. Full YAML and parsed inventories are retained; counted retention is diagnostic evidence, not proof of rendering correctness. Fresh current-group controls intentionally correct the old colliding entity identity `Cove` to `Cove.Cove`; these are explicitly recorded identity changes.

The retained 1,793 entries have 1,719 unique source hashes. There are 48 duplicate-source groups accounting for 74 extra entries, all with identical references within each group. Constituent cohorts are disjoint; older overlapping inventories and six earlier `retained_maker_` copies are not additional denominators. The 33 fresh sources are disjoint, so this review has 1,826 entries and 1,752 unique sources. [summary.json](summary.json) and the archive's `denominator-provenance.json` preserve the full calculations and membership.

## Gates and provenance

Each required broad gate ran once after stable builds, with `CARGO_BUILD_JOBS=4` and separate private before/after release targets. Source exports are immutable Git archives, not new shell worktrees. Logs and real exits are in [gates.json](gates.json) and the archive.

- Workspace units: 1,451 passed. CLI/error/docs: 34 passed. Formatting: passed.
- Library no-oracle diagnostic: 11,188/11,251, exit 101. Assisted: 10,523/11,251 eligible, exit 101; 728 failures and 1,299 skips out of 12,550 total. Both failure-name files are byte-identical to packet7.
- Guards: 4/6, exit 101. Clippy: 20 diagnostics, exit 101. Standing perturbations: 1 passed / 1 failed, exit 101, including all 33 new pairs.

The library tier numbers are not real-CLI headline measurements; the CLI ledger above is separate. The guard run preceded authoring the new rejected review records and is not described as a post-record run. No broad check was repeated for evidence-only packaging.

[integrity.json](integrity.json) validates 365 packet files, 33,591 archive members, 8,802 historical Git blobs and all frozen input/reference content hashes. [supplemental-integrity.json](supplemental-integrity.json) checks the original rejection import, additional Java sources and gate summaries. Git proves that the three new account commits precede committed production and contain no production changes; it cannot independently prove absence of uncommitted drafts. Earlier admitted chronology defects remain unchanged and are not retroactively excused.

Only the parent's existing Monkey Man relay generated Java references. The immutable manifest uses the literal `PRIVATE_CWD` placeholder. The returned bundle SHA256 is `827d1abc7eed85dfc23c62bb6d1f06d8ce7c0f1a4b9da09a0673c8bf1bdd5993`; [reference-validation.json](reference-validation.json) records independent membership/source/stream/argv/cwd/environment checks. All 33 captures have real exits and no timeouts: 27 valid diagrams, 6 invalid references, no intended/observed classification disagreements. The earlier maker `quote_invisible` intended-positive disagreement remains preserved in packet7.

Pinned Java revision: `71806a23780b04a5ccde2f8ceb5121edad5eb711`. Pinned JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Wrapper SHA256: `6115a3503877999796cb98b9efb6872d9bb8090c2f7b986706919e475dee069e`. Timezone: `GMT+11:00`. Historical fonts and Graphviz equivalence are not claimed.

## Reproduction and limitations

[evidence.tar.gz](evidence.tar.gz) contains every new capture, both complete CLI/YAML replays, source/reference snapshots, manifests, raw relay bundle, scripts, build logs and final gate logs. [archive-integrity.json](archive-integrity.json) records its SHA256 and verifies every member. Extract into a new private cache directory. The scripts preserve exact original commands and paths; choose new capture/build destinations before reproducing, and retain the unchanged comparator with its recorded hash. Use the same pinned stdlib and historical process cwd for the old cases. Do not invoke the relay or local JVM from the replay scripts; references are immutable files.

The 33 new pairs are also exercised by `cargo test --release --test parity_perturbations -- --nocapture`; its current failure is intentional evidence of unresolved defects and invalid references, not a passing acceptance gate.

Logical fonts, rooted `CommandLinkClass` multipass behavior, the TIM nested-quote handoff, `o-->>o` factory dependencies and solved group extents remain explicit residue. The separate provider-blocked native review was not replaced or retried. No graph/native/fault-injection work, local Java/javac, SSH, child agent, shipping, shared metric fix or scope extension occurred. No acceptance is granted by excluding those dependencies.
