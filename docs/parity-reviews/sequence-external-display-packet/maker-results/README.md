# External creation and queue Display — maker evidence

**New independent checker required; no self-acceptance.** Tested production `d714bb55d40ca18590d15a5ffd57c7620b91b456`; baseline evidence `ae1e0de4d39840dd58074d89d40aa97304c6ca9f`, production `653b1ed7a49784e6e6205f3aebf69a16278f8d78`. Rejection `82020a168c622ea109f1a61074c942072f37327e` imported unchanged as `cd4a5ab71d68aaaa8915bb89a2a5450151609b68`. All8,782 baseline evidence paths and245 imported paths remain byte-identical. Self half-polygon order and collections citation acceptances remain bound to their original tested revision and scope. No old claim was rewritten.

## Shipped-path results

Every source ran through the real release CLI and unchanged comparator. There are2,281 retained entries,2,233 distinct source byte strings and48 duplicate entries. Frozen2169, checker96 and maker16 memberships remain intact; checker96 retains first86 plus followup10. New16 are source-byte-disjoint. Parent1449 is not additive: historical overlap is1123 paths/1097 distinct bytes;326 paths are not replayed here (286 distinct strings,278 exclusive and8 duplicating overlapping bytes).

| Cohort | Entries | Before match / SVG mismatch / runtime failure / invalid | After |
|---|---:|---:|---:|
| frozen2169 | 2169 | 1514 / 586 / 0 / 69 | 1514 / 586 / 0 / 69 |
| checker96 | 96 | 67 / 17 / 4 / 8 | 81 / 7 / 0 / 8 |
| maker16 | 16 | 2 / 10 / 4 / 0 | 16 / 0 / 0 / 0 |
| **Total** | **2281** | **1583 / 613 / 8 / 77** | **1611 / 593 / 0 / 77** |

**28 recoveries:20 SVG mismatches and8 renderer panics. No lost passes in any comparison mode, no new valid-input errors and no final CLI/library-render panics.** The four independent outgoing pending-CREATE regressions are reproduced before and match after. Four new outgoing controls also panic before and match after. All2169 frozen status codes and CLI SVG hashes are unchanged. Checker96 improves67→81 valid matches of88, retaining7 valid failures and8 invalids. All16 new controls match whole output.

The new references were independently classified from XML, Sequence metadata and stderr as16 valid outputs, not from exit0 alone; every result is preserved. All77 invalid references stay nonpositive: Rust rejects57 and accepts20 in both revisions. Original intention/observation records remain immutable. The two empty invalid references still panic in `diff_one` comparison diagnostics; this is separate from library rendering, which has no final panic. `remaining-failures.json.gz` retains every593 valid whole failure plus77 invalid references; runtime statuses and diagnostics are preserved in both measurement sets.

## Model correction

Three schema-complete accounts were precommitted in `6857e26f9b0e1fc4c96d34faca06f7fe3ce539f2` before drafts. Only `crates/rustuml-render/src/sequence.rs` changes production. No model field, parser dispatch, ASCII, class, shared text/style/font/Creole, TIM, target or native edit occurs.

- **External CREATE consumers**, implementation `f3d8bf0cce9caecfcaee445eaa69226f68378047`, account `sequence-external-create-consumer`. Java `SequenceDiagram#addMessage` attaches and clears pending CREATE for compatible `MessageExo`; that ownership remains in the existing sequence attachment helper and Teoz raw history. `DrawableSetInitializer` dispatches external messages to `Step1MessageExo`, and Teoz uses `CommunicationExoTile`. These consumers draw an external arrow and issue its life callback, without `Step1Message#createArrowCreate` or `CommunicationTile`'s `goCreate`/head branch. The renderer now derives a separately named ordinary creation-message map for inline heads, spacing and creation origins. External attachment is retained; it does not enter ordinary participant-target geometry. No missing-target guard, dropped owner or coordinate compensation was added. Unit tests check both external directions and engines, preserved owners, and a subsequent ordinary creation.
- **Complete queue Display**, implementation `d714bb55d40ca18590d15a5ffd57c7620b91b456`, account `sequence-queue-complete-stereotype-display`. `Participant#getDisplay` includes the stereotype. `ComponentRoseQueue` passes that complete text block as its label to `USymbolQueue`, with an empty separate symbol stereotype. Queue preferred height and paint now include the same stereotype/name block metrics regardless of box membership. The queue branch accounts for its complete Display once; the boxed-figure surcharge no longer double-reserves it. Initial, created and tail consumers use this contract, retaining the existing first-stripe ascent and underline behavior. All14 checker queue controls match, including the six formerly missing stereotypes. All8 new queue controls match across bare/boxed/created/repeated heads, blank/rich stripes and both engines.
- **Heading handoff**, account `sequence-heading-shared-stripe-handoff`; no production implementation. Parent accepted the precise ownership handoff as **🎯T14.13**. Java `CreoleStripeSimpleParser` recognizes horizontal rules before equals headings, consumes the prefix, trims the text and records heading order. `StripeSimple#fontConfigurationForHeading` chooses +4bold/+2bold/+1bold/italic before atom measurement and paint. This is shared stripe semantics, not sequence-owned preprocessing. No marker stripping or font tuning was added to sequence. Both heading and both note/divider controls remain explicit failures, together with the two old shared-maker handoffs. No shared production or combined correctness is claimed.

The pending-CREATE assigned matrix is now27/30 valid whole matches, with3 group/box failures retained. It is not accepted wholesale. The prior accepted24 self cases and4 collections controls continue to match; their historical attestations are unchanged and are not transferred to this packet.

## Counts and complete CLI diagnostics

`semantic-inventory.json.gz` records Java/before/after tags, labels, available head groups and geometry for all2281 entries. The raw created heads are not always grouped, so a head-group count alone is incomplete. All28 recovered outputs change geometry,24 change structural counts, and no valid case develops larger excess tag or label counts. These are bounded diagnostics, not a replacement for whole comparisons.

`creation-witnesses.json.gz` includes every outgoing/incoming case and queue stereotype witnesses. The outgoing checker cases retain one CREATE and one Message in their AST, now paint each participant label twice like Java, and match all output elements. Queue cases retain stereotype in the AST and now paint its two head/tail labels. New repeated queue creations retain each attached event and the tail. The unit additionally verifies that an external message remains the CREATE owner in `TeozHistory`, while the following ordinary message alone appears in the head-consumer map.

Actual `--ast -pipe` and `-ttxt -pipe` outputs were captured for **all2281 entries in both revisions**. Their stdout hashes, stderr and exit statuses are unchanged; recorded binary paths naturally differ. This is a measured observation, not a new ASCII behavior/parity claim. Before, eight of those successful AST/ASCII paths still panicked during SVG rendering. Full raw outputs remain in the archive and `final-model-and-ascii.json.gz`.

## Stable gates

Required broad gates ran **once on stable final code**. Main library product **11181/11251**, assisted **10497/11251**, sequence **1411/1411**; both failure-name sets are byte-identical to the prior exact maker snapshot. Assisted total12550 minus1299 skipped yields11251 eligible. These are library calls, not11251 CLI subprocesses. Failure-name files were copied immediately after their run and packaged from the immutable private copies.

Parser575, renderer797 (including12 ASCII), CLI13 and focused sequence64 pass. Owned-file formatting passes. Existing red gates remain: main product unlocked ratchets,754 assisted failures, guards4/6, standing875/1512 with637 failures, class-file workspace formatting, parser component dead-code Clippy and19 renderer Clippy findings. Standing membership includes all96 imported pairs; fresh maker inputs live in the maker packet. Standing library cwd remains the Cargo package; CLI replay uses repository cwd, retaining T14.6.2 distinctions. No comparator, baseline, skip, error census, allowance or frozen contract changed.

## Provenance and reproduction

`all-inputs.json` is relocatable membership. `binaries.json` binds the before653b1ed7 and finald714bb55 code/binaries, including unchanged comparator bytes. The first dirty draft has an exact source snapshot/hash and all112 focused outputs. The initial unit compile error called unwrap on a String return; its log remains preserved. No failed control or earlier output was discarded.

`captures.tar.gz` contains all private before/after SVGs, AST/ASCII, comparison diagnostics, source snapshots, intermediate logs and raw relay results. `checks.json`, scripts and logs preserve actual commands, cwd and environment. Copy scripts/manifests to nonsynced scratch before reproduction, remapping only paths and choosing a fresh output directory:

```text
python3 docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py PRIVATE_CACHE/all2281.json PRIVATE_CACHE/final-bin PRIVATE_CACHE/final2281
```

The private persistent target uses Cargo jobs4; no local JVM/Javac, SSH, child or new runner was used. All16 fresh references came through the parent's existing Monkey Man relay, worker.Vj1RPcfr/new, GMT+11:00, headless en_AU/UTF-8 and pinned JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Manifest SHA `6f038fa8177116750c9ac6858c8c05ca4d0e89a2e4094eac2250baf134809b57`; original bundle SHA `f8b48bf7c7908dacb5eb13f32028983a2de9a9d6d5368c56e6a975d85a6b4013`; returned gzip SHA `9dfcc45130464b349d53a7827ed203885408163653a9cf5557360ef5412acc6b`; transport SHA `f8003e2887f94980b31ef25a5162dbc16ff48ee3f6e00e92e66a6887c4462319`. Local verification checks membership, exact source/stdin/stream hashes, JSON-array argv, only PRIVATE_CWD substitution, cwd/TZ/env and XML/type/status. Derived Java argv is not process-traced. Remote OpenJDK21.0.5 and Graphviz12.2 differ from origin; historical equivalence remains unproved.

## Residuals

All593 valid failures remain active. The new checker residuals are `ind78_group_scope_legacy`, `ind78_group_scope_teoz`, `ind78_box_scope_teoz`, both `ind78_stripe_heading` and both `ind78_stripe_note_divider` cases. T14.13 covers shared heading semantics; it does not waive whole-output failures. The complete586 frozen failures are unchanged, including shared blank-stripe/markup handoffs, nested/multiline englober, Dolls ancestor sums, reverse-created queue and other envelopes, logical fonts T14.11, tabs, URL Display, theme Real/padder, shadows, notes, delays, chrome and factory dependencies. TIM welcome remains separately nonpositive. See `dependencies.json` and exact residual comparisons.

No self-review, target mutation or shipping. Commit evidence and stop for a NEW independent checker.
