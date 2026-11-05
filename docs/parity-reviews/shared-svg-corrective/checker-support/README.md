# Independent corrective review — rejected

Checker: gpt-6-astra/high through T3 Code/Codex, no prior maker involvement. Evidence tip `3875fee3b164d63d4b39df4bc6c13ca8ace89169`; production `346f6b3f680895d3eb457041ed88ae224c583d95`; production base `a33dccc8e94dc44b5bafdc756fc1b88c42823a7d`. Exact Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`, JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, OpenJDK21.0.5. Private checkout branch; no production changes, children, additional worktrees, targets, shipping or sequence integration.

## Verdicts

- **Comments: rejected** for whole-output parity. The shared serialization algorithm has bounded source/runtime support, including one `?` per supplementary scalar. Consumer names, cluster names, missing consumers, metrics and four valid-input parser panics remain wrong.
- **State fills: rejected** for whole-output parity. Both ordinary rectangles correctly serialize tested RGB alpha endpoints and short hex. Transparent ordinary fill still becomes white; named-color acceptance and empty-description geometry differ. Composite/concurrent and other direct shape writers still bypass the serializer.
- **DOCTYPE oracle: accepted within its policy scope.** All25 scratch mutations behave as intended; all26 real DOT outputs pass; both malformed-ratchet directions reject injected drift. This does not accept the production packet.

New per-mechanism records are under `../comments/`, `../state-fill/`, `../doctype/`; `../review.json` rejects the packet. All old records remain untouched. The54 new same-stem pairs are executable under `test-diagrams/perturbations/shared-svg-corrective/checker-independent-astra/`. The existing standing no-oracle test scoped to these54 was executed and failed:16 pass,38 fail, including all eight Java-negative references. A passing projection never substitutes for a whole-output result.

## Reproduced actual CLI measurements

| Corpus | Entries | Valid | Before passes | After passes |
|---|---:|---:|---:|---:|
| Full goldens |12550|11251|11179|11179|
| Twenty retained cohorts |2191|2042|1027|1029|
| Retained plus corrective16 |2207|2057|1028|1040|
| checker87 |87|83|34|36|
| corrective16 |16|15|1|11|
| Six monospace controls |6|6|6|6|
| Fresh checker comments |30|22|4|8|
| Fresh checker state fills |24|24|1|8|
| All fresh checker cases |54|46|5|16|

Goldens emit12470 SVGs;80 process errors, zero panics/malformed outputs. Exactly39 SVG byte changes, all semantic comment bodies:37 retain passing status, two Hindi/Thai Java-negative references remain nonpositive. Decoding all39 verifies attributes, painted text and geometry unchanged; zero XML-equivalent re-encodings. Thirty comment lists match Java exactly; seven keep existing name-selection differences; two lack positive Java references. `changed-outputs.json` preserves all decoded comment lists and comparisons.

Every retained/full membership preserves duplicates and nonpositives; no lost passes, new errors or new panics. YAML independently measured14757 entries:14751 byte-identical and **six** map-order-only changes, zero semantic changes. The immutable maker archives independently verify their different run:14750 identical and **seven** order-only changes. This is the existing T14.6.6 nondeterminism, not an exact-count reproduction claim.

Strict registered census before adding checker cases:2069 pairs,1958 emitted,111 process errors including32 panics in both phases. Malformed outputs16→7; all nine malformed comments repaired. The seven approved C0 cases remain failures owned by T14.20. The CLI gate passes17/17. Newly discovered checker cases are reported separately, not folded into that frozen denominator.

Fresh54:50 emitted in each phase; four process errors, all parser panics, in each phase; malformed17→7. These seven remaining malformed outputs arise in the initial intended-valid/observed-invalid Unicode probes. No new losses/errors/panics. Comments are30 total/22 valid; states24/24 valid. All eight initial Java-negative sources remain executable and nonpositive; all eight safe Unicode follow-ups are valid but fail whole-output comparison.

## Causal findings and bounded obligations

The four valid panic cases are `class "--"`, `class "----"`, `class "-->"`, `class "<!--"`. Rust indexes an empty identity path at `parse/class.rs:285`; these existed before the packet. `cluster_comment` and description/deployment Unicode link cases expose existing name normalization, independently of correct dash/ASCII serialization. Object and legacy activity cases still omit expected consumers. These defects need model-level repairs, not comment output tuning.

Ordinary alpha00/01/7f/fe/ff and #9b4 paints agree with Java. Empty state geometry still fails. Ordinary `transparent` is normalized upstream through `resolve_state_paint`→`sequence::resolve_color` to white before the shared serializer receives it. RebeccaPurple differs from Java's accepted/default color behavior. Composite/concurrent headers and nested ordinary rectangles retain raw eight-digit colors without SVG opacity. The direct-fill census and full paint projections identify each bypass. Choice/history/start/end dispatch/name behavior also remains wrong. Fork/join fixed bars agree in these probes but are not serializer coverage.

The original Unicode probes contain **no literal invalid C0 bytes**. Their U+E000 reaches pinned Java `AtomText:80`→`CharHidder.unhideChar:87–88`, which maps reserved U+E000..E0FF to U+0000..00FF. Five Java outputs crash with the resulting NUL; three other consumers reject the source syntax. This is a source-traced Java private-use collision, not source NUL and not positive comment evidence. The safe follow-up has usable BMP, NBSP, combining and supplementary-character controls. A UTF-8 source cannot contain lone surrogate code points. Unsupported categories were not silently claimed as covered.

No waivers: seven approved C0 crashes, automatic colors/logical fonts/alias-metric coupling(T14.15), gradients(T14.18), base style(T14.19), comment label selection, warnings(T14.12), stripes(T14.13), plain/code(T14.14), unconsumed lines(T14.17), sequence escaping/handoff, salt/json process errors. No combined correctness claim with concurrent sequence work.

## Source, history and library diagnostics

All three accounts precede their implementation and are unchanged. Every cited Java source matches its revision; all eight supplied OpenJDK source hashes match independently fetched pinned files. Additional CharHidder/AtomText and JDK surrogate/encoder sources substantiate the new findings. Exactly nine production files changed. The45-site independent audit preserves all outside-comment templates and non-comment arguments, including five mixed macros; no comment removal or fixture-shaped logic. Only two ordinary state rectangle fills changed. All11325 prior review/account/perturbation files are byte-identical.

Broad gates ran once on the stable candidate before registering fresh cases: parser624/624, renderer797/799 (same two failures), other library suites green; CLI17/17; no-oracle library diagnostic11179/11251, assisted10497/11251; standing898/2069; guards4/6. Failure-name sets72/754/1171 are exactly the maker's. Clippy fails on existing parser component.rs:72 dead code. Fmt reports existing parser/class and renderer/class residue. Diff check passes. These are library/quality diagnostics, separate from shipped CLI truth. No gate was relabeled green.

## Reproduction and provenance

Private cache: `/Users/marcelo/Library/Caches/rustuml-shared-svg-corrective-checker/`. `builds.json` records separate exact before/after release snapshots/targets with jobs4; binaries were privately copied and hashed. `archives.json` names raw captures, scripts, metadata, logs, reference requests/results and entry SHA256 manifests. Extract each archive into one private directory and relocate the retained absolute paths consistently. Existing maker archives are hash-verified and referenced rather than duplicated. Do not write capture output into a working repository.

Fresh Java used only the parent's existing Monkey Man runner, workers `worker.lTnmIo28` and `worker.I3Ig0G77`, cwd `<worker>/new`, TZ GMT+11:00, env{}. Sources, membership, stream hashes, pins and wrapper argv were independently checked. Java argv is derived, not process-traced. Original request envelopes and lossless parent mappings are preserved. No local JVM, javac, SSH or new runner was used. All mutation documents and scratch test fixtures were removed. Initial setup/analysis failures and their corrections are recorded; none caused a production change or broadened a frozen allowance.
