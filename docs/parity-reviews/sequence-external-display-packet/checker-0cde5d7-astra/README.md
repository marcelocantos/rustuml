# Independent external CREATE and queue Display review

Both mechanisms are **rejected** at tested production `d714bb55d40ca18590d15a5ffd57c7620b91b456`, evidence tip `0cde5d7e55139b63cb8e91f8e42acd473441f80a`. Baseline production is `653b1ed7a49784e6e6205f3aebf69a16278f8d78`, evidence `ae1e0de4d39840dd58074d89d40aa97304c6ca9f`. This is a new independent gpt-6-astra checker task in the T3-prepared `t3-5fc3056e` checkout. No production, old evidence, account, target, baseline, comparator, skip, allowance or shipping change was made.

## Introduced regression

All four `ind5f_hide_widename_{bare,created}_{legacy,teoz}` cases match before and fail after, in the shipped CLI and both library modes. `hide stereotype` is supported by pinned Java. Java and baseline Rust paint zero stereotype labels; new Rust paints **two `«x»` labels**, reserving extra head/tail height. Participant names and messages remain present, with two lifelines. The unchanged AST retains the raw stereotype.

Java `Participant#getDisplay` filters hidden stereotypes and respects `stereotypePositionTop`; `ComponentRoseQueue` consumes that already-filtered Display. The changed Rust queue consumer instead passes raw `p.stereotype` unconditionally and unconditionally adds its height. The old bare-queue omission happened to honor hiding for paint; the new code exposes the ignored hide directive. The follow-up names are wider than the stereotype, removing the pre-existing width mismatch that masked whole-output losses in the first hiding probes. This is an introduced regression, not an inherited failure relabeled as new.

## Exact shipped-CLI replay

Columns are whole match / SVG mismatch / runtime failure / invalid reference. Runtime failure includes parse rejection, not only panics. Invalid references remain nonpositive regardless of Rust exit status.

| Cohort | Entries | Before | After |
|---|---:|---|---|
| Frozen | 2169 | 1514 / 586 / 0 / 69 | 1514 / 586 / 0 / 69 |
| Prior checker | 96 | 67 / 17 / 4 / 8 | 81 / 7 / 0 / 8 |
| Maker | 16 | 2 / 10 / 4 / 0 | 16 / 0 / 0 / 0 |
| **Reported packet** | **2281** | **1583 / 613 / 8 / 77** | **1611 / 593 / 0 / 77** |
| Independent first batch | 60 | 6 / 38 / 14 / 2 | 33 / 23 / 2 / 2 |
| Independent hiding follow-up | 4 | 4 / 0 / 0 / 0 | 0 / 4 / 0 / 0 |
| **Independent combined** | **64** | **10 / 38 / 14 / 2** | **33 / 27 / 2 / 2** |

The reported packet has 2233 distinct source byte strings and 48 duplicate entries. Every source/reference hash, actual CLI SVG hash, status and comparison-mode status agrees with the maker's recorded before/after measurements. The 28 recoveries split into 20 SVG failures and 8 panics. All 2169 frozen CLI hashes and statuses are unchanged. Invalid references reject 57 and accept 20 in both revisions. No reported-cohort lost passes, new valid errors or final panics were found.

All 64 independent sources are distinct and byte-disjoint from all 2281 retained entries. They add 27 recoveries and four lost passes. No new valid-input rejection or panic was introduced. Two valid short incoming `?` arrows are rejected by Rust in both revisions. Twelve baseline fresh-case panics disappear. The two outgoing unactivated returns are intended-valid/observed-invalid disagreements: Java error SVGs and stderr explicitly report “Nowhere to return to.” Their intent was not rewritten, and both Rust revisions reject them.

Actual `--ast -pipe` and `-ttxt -pipe` stdout hashes, stderr and status are unchanged for all 2281 entries, and for the independent64. This is an observation, not an ASCII parity claim. Full raw outputs are retained. The two empty-reference `diff_one` comparison panics occur in both comparison modes (four diagnostic invocations); `--print-rust` succeeds for them. They are comparator-side failures, not final CLI or library-render panics.

Parent1449 is **not additive**: independently reconstructed overlap is 1123 paths, with 326 unreplayed paths. Those have 286 distinct strings, 278 exclusive to the unreplayed portion and eight duplicating overlapping bytes. No claim covers those 326 paths.

## Mechanism verdicts and witnesses

- **External CREATE: rejected.** Fresh32 contains 30 valid cases: 19 match, nine SVG mismatches, two pre-existing valid-input parse failures; two Java-invalid references remain separate. External note labels disappear because `try_external_message` does not update `last_message`, so bare-note parsing emits no Note event. Short `?` arrows are absent from Rust's external regexes. Legacy activation/activated-return bars start 10 units early; fresh group and nesting cases also fail. These residuals do not become maker regressions merely because the packet remains rejected.
- **Complete queue Display: rejected.** Fresh32 (28 initial plus four follow-ups) has 14 matches and 18 SVG mismatches. Four are introduced lost passes. Multiple stereotypes lose the second label; spotted syntax is painted literally and omits spot shapes; stereotype font size/color and bottom order remain wrong. Nested boxes retain ancestor/collections failures. Single-box mixed-kind and no-queue controls pass unchanged, and the surcharge for non-queue kinds is algebraically unchanged.

The external attachment/head separation is causally supported within its narrow scope. `TeozHistory` still derives raw CREATE owners from the full attachment helper; only the ordinary-head map filters external border topology. The passing ownership unit is retained. Fresh repeated external creates preserve three Create/four Message events, two labels per participant and two lifelines. External creation followed by ordinary creation of a different participant preserves two Create/three Message events, two labels per each of three participants and three lifelines. Both engines match; ordinary creation/destroy controls also match. `semantic-witnesses.json.gz` records event, head-label, label, shape and lifeline counts and geometry explicitly. Group metadata alone is insufficient for ungrouped/Teoz heads and is not used as the only head census.

The old self half-polygon 24/24 and collections 4/4 cases still match, without transferring their historical acceptance. The prior pending-CREATE matrix remains 27/30 valid matches, retaining both group failures and the Teoz box failure. Both heading and both note/divider stripe failures remain under the shared-stripe handoff. All 586 frozen failures and the nonpositive TIM welcome remain unresolved. No combined correctness with the concurrent `creole.rs` packet is claimed.

## Provenance and stable gates

All 8782 baseline evidence paths and 245 imported paths are byte-identical. The original rejection patch and `cd4a5ab71d68aaaa8915bb89a2a5450151609b68` import are identical. All three accounts precede the drafts; implementation messages cite `6857e26f9b0e1fc4c96d34faca06f7fe3ce539f2`. Only `crates/rustuml-render/src/sequence.rs` changes production. No fixture-shaped branch, missing-target guard, dropped CREATE owner or new coordinate compensation was found in this diff. Pinned Java files were compared byte-for-byte to revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`.

Private exact release builds used separate before/after targets with jobs4. The golden archive was hydrated from the existing local clone at exactly `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`, then every archived blob was verified. No local Java/Javac, new SSH connection, runner, child or extra worktree was used. The comparator adapter is byte-identical to the existing adapter and comparator production sources are unchanged; separately linked binary hashes are recorded rather than falsely claimed identical.

Stable required broad gates ran once per exact production revision. Both main library runs yielded product **11181/11251**, assisted **10497/11251** (12550 total minus 1299 skipped), and sequence **1411/1411**. Failure-name files were copied immediately and are byte-identical across revisions. These are library results, not CLI subprocess counts.

Final parser575, renderer797 including ASCII12, and CLI13 pass. The focused module filter runs62, while the maker's broader `sequence` filter names64; its two extra skinparam tests passed in the full797 run. All64 maker test names are covered green, without a redundant rerun. Standing is **875/1512** after versus **794/1416** before; membership grew by the imported96, so these are not a fixed-denominator improvement claim. Guards remain **4/6**. Main unlocked ratchets, assisted failures, standing failures, class-file workspace fmt, parser component dead-code Clippy and19 renderer Clippy findings remain red. The sequence file formatting passes. Raw logs and exact commands are preserved.

Fresh pairs were run through the exact no-oracle library helper and CLI, and are registered under the standing perturbation corpus. Broad gates were not repeated after evidence-only registration. Their failures explicitly block acceptance.

## Reference and reproduction limits

The parent used its existing Monkey Man relay for both batches, pinned JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, GMT+11:00, headless en_AU/UTF-8. Independent local validation checks immutable manifests, names, source/stdin/stream hashes, JSON-array argv with only PRIVATE_CWD substitution, cwd/environment and XML/error evidence. Exact original manifests, bundles and returned envelopes remain intact. Java argv is derived from the wrapper, not process-traced. OpenJDK21.0.5 / Graphviz12.2 historical equivalence remains unproved.

`review-index.json` points to the new per-mechanism records. `fresh-pairs.json` is relocatable input membership. `captures.tar.gz` contains all raw before/after outputs and gate logs; summaries, scripts, binary bindings and reference records accompany it. Copy reproduction scripts into private scratch, remap only checkout/cache paths and select a new output directory. The runner accepts `MANIFEST OUTPUT_NAME` and uses the bound exact binaries; it preserves duplicates and nonpositives.

Next work is bounded: recover all four hiding regressions through filtered/ordered Display semantics; resolve the retained external note/short-arrow/lifecycle/envelope and queue stereotype/style failures with appropriate ownership; retain all old rejects; obtain a new independent review. No acceptance, target mutation or production fix is part of this evidence commit.
