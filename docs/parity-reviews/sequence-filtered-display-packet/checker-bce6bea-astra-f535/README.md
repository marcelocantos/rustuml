# Independent filtered Display / note-consumer review

**All four mechanisms are rejected.** The original maker replay is reproducible, but fresh whole-output counterexamples remain and the terminator implementation introduces four losses of the rejection contract.

Reviewer: new independent `gpt-6-astra` checker through T3 Code / Codex, branch `review/sequence-packet-independent-f535ce04`, isolated checkout `t3-f535ce04`. No children, other worktrees, production edits, old-evidence edits, target changes, baseline/comparator changes, shipping or combined Creole correctness claim.

Exact before evidence: `0cde5d7e55139b63cb8e91f8e42acd473441f80a`, source-identical to production `d714bb55d40ca18590d15a5ffd57c7620b91b456`. Exact after evidence: `bce6bea405d8b5efd55136f4768117115e7f634f`, source-identical to production `61e6aeed6eaf686b3497fbc696aadc03a64e69a2`. Both were built independently in private release target directories with jobs 4. Binding files contain binary hashes; commands and raw build logs are retained.

The pinned Java revision is `71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256 is `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. All new references came through the parent's existing Monkey Man relay, in three immutable batches of 130, 6 and 12. Names, source/stdin bytes, hashes, argv arrays, streams, private cwd, timezone and environment were independently revalidated. XML type, error markers, stderr and status were checked together to classify validity. Java argv is derived, not process-traced; OpenJDK 21.0.5 / Graphviz 12.2 historical equivalence remains unproved.

## Introduced regression

`indf535_terminator_em_space_{legacy,teoz}` and `indf535_terminator_narrow_nbsp_{legacy,teoz}` are intended-invalid and observed-invalid Java inputs. Each Java run returns an error SVG, exit 200 and nonempty stderr. Before Rust rejects with exit 1; after Rust accepts with exit 0. **These are four introduced lost rejection passes.** The unchanged comparator reports invalid-reference status 2 for both Rust revisions, so that column alone cannot expose this regression.

Causal source: Java `Pattern2` expands `%s` to ASCII `\s` plus NBSP and compiles with `CASE_INSENSITIVE`, without `UNICODE_CHARACTER_CLASS`. The new Rust `NOTE_END` uses Unicode `\s`, accepting U+2003 and U+202F. The NBSP controls are valid in Java and now match in Rust. All six sources, references and error streams are preserved.

## Exact maker replay

Columns are match / SVG mismatch / valid non-SVG error / nonpositive reference. Denominators include all duplicates and negatives.

| Cohort | Entries | Before | After |
| --- | ---: | --- | --- |
| Frozen | 2281 | 1611 / 593 / 0 / 77 | 1611 / 593 / 0 / 77 |
| Imported checker | 64 | 33 / 27 / 2 / 2 | 41 / 19 / 2 / 2 |
| Maker fresh relay | 24 | 4 / 16 / 4 / 0 | 22 / 2 / 0 / 0 |
| Creole handoffs | 2 | 0 / 2 / 0 / 0 | 0 / 2 / 0 / 0 |
| Total | 2371 | 1648 / 638 / 6 / 79 | 1674 / 616 / 2 / 79 |

There are 2323 unique source hashes and 48 duplicate entries. Every CLI SVG/stdout byte hash and comparator outcome reproduces the maker's captures in both revisions. There are 26 recoveries and zero lost positive passes in CLI, product-library and assisted-library comparisons. There are zero new valid CLI errors and zero SVG renderer panics. The 79 nonpositive references retain 59 Rust failures and 20 successes. All four imported hidden-wide-name losses recover. These bounded replay facts do not cover the new rejection-contract losses above.

All 2281 frozen YAML stdout captures are identical. Of the additional 90 entries, 30 change: 16 nondefault Display fields, 10 Note ownership/event cases and 4 parse recoveries. Nineteen YAML serialization panics remain unchanged. All AST, ASCII and YAML stdout/status captures match the maker's recordings. Ten ASCII stdout/status changes are confined to parser ownership and terminator recovery; this is not an ASCII parity claim.

Independent old-model round-trip check: 2286 of 2287 successfully serialized before-models deserialize and reserialize byte-exactly through the after parser. One state minty-skinparam model panics in the libyml scanner in both revisions. This separate pre-existing deserialization residue must not be folded into the 19 CLI YAML serialization panics or described as a new SVG panic.

## Fresh checker evidence

All 148 source hashes are unique and disjoint from the full 2371 packet, including the maker's 24 and imported checker's 64. There are 136 positive references and 12 negatives. Before: 34 matches, 86 SVG mismatches, 16 valid parse errors, 12 negatives. After: 94 matches, 42 SVG mismatches, no valid parse errors, 12 negatives. There are 60 positive recoveries, zero positive-match losses, four introduced rejection losses, and no new valid error, SVG panic or fresh YAML panic.

| Mechanism | Positive / all entries | After positive matches | Verdict |
| --- | ---: | ---: | --- |
| Filtered ordered participant Display | 42 / 42 | 21 / 42 | Rejected |
| Note aggregate paint height | 60 / 60 | 50 / 60 | Rejected |
| External note owner | 16 / 18 | 9 / 16 | Rejected |
| Note shape terminator | 18 / 28 | 14 / 18 | Rejected; four introduced rejection losses |

The new per-mechanism `review.json` files contain every pair, exact identities, revisions, commands, source findings and bounded next obligations. All valid whole-output failures remain failures, even when component geometry matches. Negative inputs retain intent, Java observation, Rust statuses and rejection-contract results without positive parity credit.

Participant visibility filtering precedes metrics for all kinds, and all eight final-hide controls pass in both engines. Ordered Display paint is queue-only; the other components retain baseline bottom-order or bare-stereotype defects. Java retains bare figure stereotypes where Rust's existing boxed-only heuristic drops them. Created/box/nesting residuals remain explicit.

Note paint height agrees directly in 58/60 cases. The 12 exact-boundary controls all match fully: total font-size 512 gives text height 603 exactly, and total 557 gives 655.998046875. Box/hex heights are 611/663, folded heights 613/665. The earlier exploratory boundary probes remain preserved; they were not exact-boundary tests under the actual font metrics. The two remaining height mismatches are Teoz self-message rnote/hnote consumers: Java deliberately selects folded `ComponentType.NOTE` and paints height 72, while Rust selects requested shape and paints height 70. This was already wrong before. Other whole failures involve note horizontal envelopes, self-message ordinates and canvas bounds.

External-note positive controls contain 20 Note events after versus 16 before; every final note label has the same multiplicity as Java. Normalized owners and ordinary-message replacement are retained. Seven whole mismatches confirm the maker's explicit unrepaired external-note envelopes. Bare `note over:` is Java-invalid but accepted in both Rust revisions, a pre-existing negative failure.

## Stable broad gates

Each command ran once per stable revision. The golden submodule is exactly `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Initial hydration failed because local file transport was blocked; the command-scoped retry succeeded. Both logs are preserved. There was no code-change or gate rerun.

Final units: parser 579, renderer 798 (12 ASCII), CLI 13. Main **library** product tier: 11181/11251; assisted tier: 10497/11251; sequence: 1411/1411. Failure-name files were copied immediately and are byte-identical before/after and to maker evidence. These library diagnostics are separate from the shipped-CLI replay.

Final standing suite is red at 916/1576; guards are red at 4/6. Before standing evidence had 1512 entries and 875 matches; its smaller denominator predates the 64 imported cases. Workspace formatting remains red in class files, parser clippy stops on the existing component dead function, renderer-only clippy retains 19 findings. Owned formatting passes. Broad gates were not rerun after evidence-only additions; the fresh pairs were executed through the actual CLI and unchanged library comparators and are discoverable by the standing perturbation harness.

## Preservation and next obligations

All 9145 prior account/review/pair paths and 205 imported paths are byte-exact. The rejection import's tree delta matches its original commit. Accounts precede implementations and are unchanged since precommit. Only the three sequence production files changed, plus exactly the three authorized ASCII constructor defaults. All 16 cited Java files match the pin; checker-added provenance files do too. No fixture-shaped logic, queue-only visibility check, coordinate compensation or unexplained new constant was found.

Restore exact Java whitespace grammar first. Preserve the valid terminator, visibility, note-ownership and aggregate-cast gains. Subsequent source-derived work must close nonqueue ordered Display, Teoz self-note component selection, note envelopes/ordinates and external-note horizontal envelopes, with fresh independent checks. Short `?` arrows, legacy external callbacks, spotted/multiple/styled stereotypes, group/box/nesting bounds, heading/note/divider Creole handoffs, logical fonts, frozen failures and TIM welcome remain explicit dependencies or nonpositive evidence. The two outgoing unactivated returns remain Java-invalid. No claim includes the concurrent `creole.rs` packet.

`captures.tar.gz` retains all checker raw stdout/stderr, before/after SVGs, diagnostics, broad logs, failure-name snapshots, Java relay manifests/results/transport envelopes and sources. It excludes build binaries; their hashes and exact build commands are retained. `support/` contains the checker scripts, unchanged comparator adapter and independent round-trip helper. `artifact-sha256.json` inventories evidence hashes.

The executable `support/rejection_contract.py` checks every retained observed-invalid input against exact before/after CLI binaries. Its fresh run exits 1 as required: 10/12 rejections before, 6/12 after; four introduced false accepts and two pre-existing bare-over false accepts. Results and exact argv are in `rejection-contract.json`.
