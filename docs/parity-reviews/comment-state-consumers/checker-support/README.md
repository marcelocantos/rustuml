# Independent comment/state checker: rejected

Reviewer: gpt-6-astra, Codex harness in T3 Code, task `76d88857-1ec0-408c-8c06-0b37eda8395a`. New independent checker, no maker involvement. Production `523a2d89226d764b818b7af2472593d2690e9c4b`, evidence tip `2a791353c998e4c34bbf0f16699c7ffd21d50cea`, before evidence `3875fee3b164d63d4b39df4bc6c13ca8ace89169` (production `346f6b3f680895d3eb457041ed88ae224c583d95`). All three mechanisms are **rejected**. Bounded improvements below are not acceptance.

The new records are `../review.json`, `../comment-identity/review.json`, `../missing-comments/review.json`, and `../state-paint/review.json`. The 100 fresh source/Java pairs are under `test-diagrams/perturbations/comment-state-consumers/checker-independent-astra-20261105/`. Every failure and both intended-positive Java syntax rejections remain executable. No production, account, baseline, comparator, skip, allow-list, target, or sequence files were changed. No shipping occurred.

## Decisive findings

1. **Two new retained comment-list regressions.** `collateral/1162-review_root_quark_no_reparent` and `collateral/1164-root_quark_nested_reuse_control` had exact Java comment lists before. They now gain spurious cluster comments. Both already failed the whole-output comparator; zero lost comparator passes therefore does not mean zero comment regressions. See `comment-list-regressions.json` for exact sources, hashes, and before/after/Java lists. Fresh `state_history_nested_identity` independently adds unwanted `cluster Top` and `cluster Lower` comments.
2. **Identity is still inferred.** Explicit aliases equal to `label_to_id(display)` are mistaken for unaliased names. Fresh `deploy_alias_collision` and `group_identity_collision` render `Copper Bay`/`Metal Case` instead of Java `Copper_Bay`/`Metal_Case`; link comments lose previously correct underscore identities. Nested default/custom namespaces still emit joined names. Three such fresh cases pass the strict comparator because it ignores comments. T14.23 is recorded, not waived. Nested history markers are shortened before the new history-comment helper can recognize `[H]`/`[H*]`.
3. **The paint packet changes strokes.** `resolve_pseudo_state_color` now delegates to the fill resolver while still serving `start_stroke`/`end_stroke`. `circle_root_rebecca` changes circle strokes from `#663399` to `#FFFFFF`; Java uses `#FFFFFF`, so this is an improvement outside the predraft “Stroke unchanged” boundary, not a new regression. Ordinary state/arrow strokes still fail. `circle_root_alpha` also changes circle stroke hex casing. `noncomment-boundary-validation.json` confirms these two fresh cases are the only measured non-fill changes; retained non-comment changes are exclusively fill/opacity.
4. **Whole failures remain.** Composite transparent/alpha-00 cases emit an extra transparent path; concurrent cases retain incorrect UIDs. All 45 paired choice/fork/join/history/deep-history cases are valid Java STATE diagrams but route to SEQUENCE in the shipped Rust CLI. These are T14.21 failures, not state-paint evidence. Activity aliases/directions/partitions and quoted objects expose missing model/consumer admission. No combined correctness is claimed for the concurrent sequence packet.

Object comment omission is supported by the pinned `GeneralImageBuilder:180–181` selection of `EntityImageObject`, which has no `UComment`. Both the relevant golden and the fresh mixed-kind case have exact Java producer lists. The fresh mixed-kind whole output still fails geometry.

## Exact measurements

All denominators, duplicates, and nonpositives are retained. Independent private before/after release builds used jobs 4. The comparator executable is unchanged and hash-verified; actual CLI output, not library rendering, supplies the headline counts.

| Measurement | Before | After | Denominator |
|---|---:|---:|---:|
| Full actual-CLI golden passes | 11,179 | 11,179 | 11,251 valid / 12,550 entries |
| Immutable retained + maker cohorts | 1,065 | 1,072 | 2,119 valid / 2,277 memberships |
| Fresh checker94 | 18 | 23 | 93 valid / 94 entries |
| Fresh supplement6 | 2 | 2 | 5 valid / 6 entries |

There are **zero lost strict-comparator passes, zero new process errors, and zero new panics** in these measurements. The **two new comment-list regressions** are reported separately. All 100 fresh CLI calls exit 0, with zero malformed XML or panics; Rust silently accepts the two Java syntax rejections. Combined fresh comparator passes are **20 → 25/98**; requiring exact comment lists too leaves **22/98** after. Thus 76 valid fresh outputs fail at least one whole-output/comment check, plus the two nonpositive references.

Exactly **194** golden SVG byte changes reproduce. Every one is comment-only at the byte level, and independently decoded XML confirms no element, attribute, painted text, geometry, or dimension changes. **188** match both Java comment lists and following-sibling producer positions. The six unchanged exceptions are exactly those recorded by the maker. Of the 194, 184 remain comparator-passing and ten remain nonpassing/nonpositive. All 456 existing element macros are unchanged after isolating comments and fill serialization. The source diff is confined to the five accounted renderer files.

Full goldens: 12,470 emitted SVGs, 80 existing process errors, zero panics, zero malformed. Retained/maker cohorts: 131 process errors including 36 panics, 14 malformed memberships. Stable registered corpus: **2,139 inputs, 2,024 emitted, 115 process errors including 36 panics, exactly 14 approved malformed outputs**. Those 14 remain correctness failures under T14.20/T14.21, with T14.17 admission co-dependencies; a green ratchet does not make them well-formed. The existing four valid class parser panic cases are unchanged.

YAML: **14,827 memberships, 14,818 byte-identical, nine map-order-only differences, zero semantic differences**. The maker had eight order-only differences; this run has seven DOT memberships and two memberships of `sprites/sprite_multiple`. This is T14.6.6 nondeterminism, not a semantic equality failure.

Broad gates ran **once** on the unchanged candidate, before registering reviewer pairs:

- Parser library 624/624; renderer 799/801, with the same two failed tests.
- CLI gate 17/17; strict XML reports the exact counts above.
- Library product diagnostic 11,179/11,251 and assisted 10,497/11,251; both gates remain red. Failure-name files are byte-identical to the maker (72 and 754 names).
- Standing corpus 930/2,139, 1,209 failures; guards 4/6, with the existing decimal and review-record failures.
- Clippy remains red on `parse/component.rs:72` dead code. Format remains red on baseline formatting. Diff check passes.

Fresh pairs were run individually through both actual CLIs, the unchanged strict comparator, strict XML, and comment/producer-position checks. There was no second broad run after registration. Fresh assisted rendering is a separate diagnostic: **32/98**, with raw outputs and comment positions archived. It is not substituted for CLI truth. No acceptance is claimed from any red gate.

## State fill census

`state-direct-fill-census.json` enumerates all remaining direct dynamic-looking fill templates at the exact candidate revision: **19 sites**, comprising 15 fixed `NOTE_FILL`/`PSEUDO_COLOR`/`BAR_COLOR` shape sites and four `TEXT_COLOR` text sites. The text sites are in `render_with_oracle`, at macro lines **6833** (handwritten warning), **7083** (scoped history marker), **7278** (history), and **7316** (deep history). Their upstream text color can vary. No additional user-variable shape fill bypass was found. Fixed literal/`none` writers and gradient stop colors are outside that count.

This routing inventory is not proof of correct role selection, dispatch, or geometry. Simple empty/body pairs pass all nine color probes; composites pass seven of nine. Concurrent alpha bytes agree but whole outputs fail. Pseudostate dispatch failures cannot establish serializer behavior for those state paths. Automatic colors, font names and alias/metric coupling (T14.15), gradients (T14.18), base style (T14.19), warning chrome, stripes, plain/code, empty-state geometry, object admission, and sequence escaping remain explicit failures.

## Process and provenance

All four causal predrafts are byte-identical to their original commits and precede production by ancestry. The canonical `account.json` is post-production and is only an index: it adds no causal claim. The descriptive predraft filenames and the two object-shaped Rust-location arrays (`missing-comments-account.json`, `follow-on-producers-account.json`) are recorded format deviations. The predraft source-boundary addendum corrects two wrong original Java class names without rewriting history.

Java source pin: `71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Twenty-nine cited/adjacent Java source files match the pin and read-only checkout; all supplied source hashes match. All **11,553** historical review/account/perturbation files from the imported rejection remain byte-exact. Every entry of all nine maker archives was independently hash-verified. Every frozen golden source/reference byte matches the hydrated submodule at **9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a**.

Target context was read from integration revision `9874a537e3bf18628377f525692103815f429982`, as confirmed by the parent; the evidence-tip ledger predates those target additions. No ledger was edited.

Fresh Java used only the parent's existing Monkey Man runner. The two immutable requests are separate (94 and 6); worker directories were `worker.zka9Xjzq/new` and `worker.gnzehg5s/new`. All sources, resources, membership, stream hashes, pins, wrapper argv, GMT+11:00, and empty environment overrides were independently checked; zero timeouts. Java argv is **derived from the pinned wrapper, not process-traced**. No local JVM/javac, SSH, new runner, worktree, or child was created. All fresh sources avoid U+E000–E0FF and XML-invalid C0; none duplicates a registered prior source hash.

Initial local-file submodule hydration failure and later setup diagnostics are preserved. The packaging path probe also initially mishandled a dotted golden stem (`arrowThickness_0.5`); its log is retained, and suffix appending corrected the read-only validation. No incomplete setup run supplies a headline value.

## Evidence and bounded obligations

`archives.json` names the lossless metadata, log, fresh Java/assisted, and before/after CLI archives. Each has a SHA256 entry manifest and was read back and verified. Sources/references for the old memberships remain frozen in the maker archives (independently validated here); `replay-membership.json` in metadata maps every replay to exact hashes. Cache paths are provenance, not portable installation paths: relocate consistently when replaying. All scripts and per-call commands/statuses/streams are retained.

Next work must address the stroke boundary with a predraft account, retain lossless Quark identity, preserve history endpoints until projection, and select actual Java cluster consumers before emitting comments. Admission/dispatch repairs must precede fresh positive coverage of the currently bypassed activity and pseudostate consumers. Whole-output geometry/UID failures and all 14 malformed cases remain obligations. No production fix or target closure is included in this review.
