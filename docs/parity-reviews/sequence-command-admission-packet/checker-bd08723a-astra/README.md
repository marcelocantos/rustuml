# Independent sequence admission review: rejected

Reviewer: GPT-6 Astra, high reasoning effort, Codex harness in T3 Code, isolated branch `review/sequence-admission-bd08723a`. No prior involvement, child agents, production changes, parent/maker edits, local Java execution, shipping, or comparator/baseline/target changes.

**The packet loses a valid baseline pass.** `bd087_reg_27_plain.puml` declares an unused multiline SVG sprite. Java renders it successfully and the baseline CLI matches Java. The candidate CLI errors on the remaining `<rect>` body. Unchanged `preprocess/mod.rs:873–904` removes only the lowercase `sprite` header; the new sequence admission rejects the exposed body. This is a packet regression despite the upstream interaction. Its source SHA256 is `8785094fa9474b31ca2be3e0b3d83c9d8d41f5ff1815f8a63e9d523771aa9409`.

## Exact revisions and scope

- Evidence tested: `c6295ccdef39719b60f0e88dfd9555919523c961`; production: `c525fd32ae97b642b440a161140e9457f29bcfc8`.
- Baseline evidence: `dc438fd0cd0f5edd44816db81f0d765bf48ce4e6`; production: `b4849a5008ff28c360fefe0176ee6ad073c02fd7`.
- Java: `71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
- Golden submodule: `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Initial local-file transport setup failure and successful hydration are preserved in logs.

Private before/after release targets were built with jobs 4. Binary, compiler, source and comparator bindings are in `before-binding.json`, `after-binding.json` and `build-commands.json`. Only `parse/sequence.rs` and renderer `sequence.rs` changed in production; the allowed diagram model file did not change. Dispatch, TIM, other families, shared SVG/text rendering and all protected gate inputs are unchanged. The size revert restores production/build/test sources byte-for-byte to `268fa9f8b`. All accounts precede their drafts, and 896 imported old evidence files remain byte-identical. No new fixture-shaped logic or family heuristic was found.

All 81 Java registration source hashes and signatures match the pinned source. The independently reconstructed handler mapping is in `source-audit.json` and `handler-review.json`. Eighty regexes reconstruct identically; grouping replaces fixed negative lookbehinds with equivalent disjoint alternatives. The correction account's literal `PSystemBuilder#initFactories` citation does not exist; its separate immutable citation correction correctly names the constructor. The default size-revert message links the reverted implementation but omits the earlier correction account.

## Actual CLI results

Counts retain duplicates and nonpositive references. A CLI panic is distinct from the unchanged comparator's inability to read a reference.

| Corpus | Entries | Before matches / mismatches / valid errors / nonpositive | After |
|---|---:|---|---|
| Frozen retained | 3,569 | 2,431 / 895 / 2 / 241 | 2,445 / 866 / 17 / 241 |
| All goldens | 12,550 | 11,181 / 70 / 0 / 1,199; 100 reference-read failures | Identical |
| Fresh checker | 571 | 146 / 236 / 0 / 189 | 170 / 211 / 1 / 189 |

Retained: 14 gains, zero lost matches or rejections. Goldens: zero lost passes or new valid errors. Fresh: 25 gains, **one lost match and one new unapproved valid-input error**, zero lost rejections. There were zero CLI panics or timeouts in these replays.

The frozen exception multiset is exactly the approved 15 entries: 13 dispatch cases and two TIM cases. Those two TIM paths contain identical bytes, so the 15 entries have **14 distinct source hashes**. Causes and hashes are preserved in `exception-source-audit.json`. Across the complete review, the sprite adds a sixteenth entry and fifteenth distinct hash. The global exception claim therefore fails; no exception was silently added to the approved set.

The 571 fresh inputs are byte-unique and disjoint from the prior inventoried source sets. Six explicitly recorded aliases let two ancillary mechanism records reference the same fresh inputs: 577 new standing pairs, not 577 independent cases. All sources and same-stem Java SVGs remain executable under `test-diagrams/perturbations/`. Intended validity is immutable. The 49 intended-versus-observed disagreements, including all supplied relay disagreements, are classified with source causes in `intention-disagreements.json`.

## Per-mechanism verdicts

All seven new `review.json` records reject whole-output/rejection-contract acceptance. Correct local behavior and preexisting failures are distinguished from regressions.

- **Registered admission:** rejected. The sprite is a new regression. Partial participant handlers also silently truncate dotted/at-sign identifiers, discard negative order and suffixes, and claim success without an unsupported declaration diagnostic.
- **Colour execution:** rejected. Participant execution passes the complete colour capture to `HColorSet`; generic `Colors` validation wrongly accepts five composite-colour command failures. Valid hyphen gradients mismatch silently while slash/bar/backslash forms warn. These false successes and mismatches predate this packet; they disprove completeness without being lost rejections.
- **Footbox:** rejected under the full contract. All 28 valid controls match. Bare `footbox` encounters a Java null-type execution failure, and four embedded-CR controls expose reader differences; Rust still succeeds on all five. These are preexisting false successes.
- **Attribute serialization:** rejected for whole-output parity. The local decimal-reference/entity mapping follows the pinned trace, including one reference per non-BMP scalar. Upstream tooltip/link extraction still produces 22 valid mismatches and three CR false successes. Every nonempty captured CLI SVG strictly parses: 31,616 frozen/golden outputs plus 992 fresh outputs. All 571 new Java SVGs also parse. No malformed XML or combined shared-SVG correctness is claimed.
- **Size correction:** rejected for parity, with the revert verified. The original eight references do fall through to CLASS at 2,001 UTF-16 units. Fresh arrow-only controls reproduce this. Declaration-bearing documents cannot fall through because the class factory rejects their sequence declarations: four fresh over-limit documents return Java 200 while Rust succeeds. Two arrow-only over-limit documents remain wrong-family mismatches. Source shape resolves the apparent reference contradiction.
- **Withdrawn size guard:** rejected/withdrawn, with no production residue; four fresh alias pairs preserve the counterexamples.
- **Resume citation correction:** the citation is correct, but whole-output acceptance is rejected by the unusual-format resume counterexample. Two explicit aliases preserve that evidence.

The divider invalid-colour case is accepted because Creole colour handling ignores the failed colour change. A spaced group token is comment text rather than the primary colour slot; multiline reference BACKCOLOR is unused. Legal `@` identifiers and optional `@newpage` account for accepted near misses. Supplement `plain`/`norank` arrow styles reach colour execution; bare footbox throws; skinparam requires the opening brace on its command line; the inline colour sprite fails palette decoding. See the source-linked disagreement records for all classifications.

## Rejection and broad gates

The unchanged `rejection_contract.py` reports:

| Cohort | Entries | Before rejected | After rejected | After false successes | Lost rejections |
|---|---:|---:|---:|---:|---:|
| Historical nonpositive | 99 | 77 | 87 | 12 | 0 |
| Prior fresh Java invalid | 122 | 40 | 122 | 0 | 0 |
| Fresh checker Java 200 | 174 | 1 | 148 | 26 | 0 |

Exporter cohorts remain separate: historical 20 and fresh 15, all zero Rust rejections. Java exit 0 alone is not validity: the fresh 15 have exporter error output and stderr. They are T14.20 residue, not positive parity or packet regressions.

The stable broad gates ran once before registering the new evidence: parser 589, renderer 801 and CLI 13 pass. Library product is 11,181/11,251, assisted 10,497/11,251, sequence 1,411/1,411. These library diagnostics are separate from actual CLI truth. Failure-name files were copied immediately and have the maker's exact hashes. Standing corpus: 2,682 entries, 1,037 failures; guards: 4/6 pass, with recorded existing provenance/review failures. Owned formatting passes; existing workspace format and clippy failures are preserved.

A separate evidence-registration check followed the addition of the pairs and records; it did not rerun the stable broad suite. The enlarged standing corpus has 3,259 entries and 1,441 failures. Guards remain red as required for seven rejected reviews and 256 failing perturbation entries. No new review schema defect was reported. The standing library gate treats some negative contracts differently from the explicit rejection replay, so its failures are not the fresh CLI mismatch denominator.

Complete AST/ASCII/YAML captures are retained. Retained stdout changes are 129/103/121, matching the maker. Golden changes are 131/111/119; the maker's AST 132 differs by one nondeterministic map-order change. Our 131 comprise 125 newly rejected nonpositive sources plus six order-only changes. `model-changes.json.gz` explains each change; no normalization was introduced. Existing note geometry/envelopes, nested Teoz boxes, quoted activation, shaped-across envelopes, short question-mark arrows, headings, fonts, frozen failures and TIM welcome remain failures, not new regressions absent causal evidence.

## Evidence and reproduction

`artifact-integrity.json` hashes every packaged artifact. `captures.tar.gz` contains full before/after SVG, AST, ASCII, YAML, diagnostics, command/exit records and comparator results. Extract it into an unsynced private directory. `support/` contains the review scripts; copy them into that private directory before replay because their cache root is their own directory. Run from the isolated repository checkout. Exact executed commands are in the binding files, `commands.json`, per-mechanism records and capture rows. The comparator and rejection-contract scripts remain at their existing repository paths.

`relay-main/` and `relay-supplement/` retain separate immutable requests and response transports. The parent's existing Monkey Man relay alone generated Java references: batches AH (509) and AI (62), GMT+11:00, empty environment overrides, no assets, zero timeouts. Parent-validated provenance includes exact source/stdin/stream hashes, names, cwd and argv. The argv is wrapper-derived, not process-traced. No new runner or connection was opened by this checker.

Bounded next obligations are to restore SVG-sprite preprocessing/admission interoperability without widening the exception set; make partial-handler support and command-specific colour execution honest; expose unsupported hyphen gradients; and resolve the retained reader, link/tooltip and factory-fall-through contracts through separately scoped source-backed work. This evidence does not authorize production fixes or shipping.
