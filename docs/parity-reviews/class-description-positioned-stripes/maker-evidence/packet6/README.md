# Class description drawing and named-quark URL consumption: maker packet6

Maker evidence only. Independent acceptance is pending. Both imported review verdicts and every earlier counterexample remain unchanged. This packet does not claim that either rejected mechanism is accepted.

Production: `d84b89cd8ee2ddd64ac71863004ec6ee84b1c4cd`. Baseline: `b074d2082cb9ed2b9460f79a32b170e31cc1ab5e`. Imported original evidence `5416610690a5b507b6fd2e95a0cd30f780bd1192` is preserved by local commit `06dedeb975ba3b0ef70bb3b161de1f22d9b991a1`, including all34 pairs. The implementation commit names every exact earlier causal account commit.

## Drawing and command changes

Only three production paths changed:

- `crates/rustuml-render/src/text_render.rs`: additive `PositionedStripe` API. `StripeSimple.getAtoms` supplies a space when a stripe is empty. `Sea.doAlign` bottom-aligns effective-font boxes with their font-position altitude and translates the minimum; `AtomText.drawU` adds that font's ascent. Stored runs drive emission and `LimitFinder.drawText` extrema, including blank, reduced sub/sup and mixed-size atoms. Existing `emit_text` and its callers are unchanged. No font metrics or glyph serializer changed.
- `crates/rustuml-render/src/class.rs`: description preferred sizes, drawing and extrema consume those positioned stripes. `TextBlockMarged.drawU` contributes its real `UEmpty(width+2,height)` around stereotype text, as well as translated text. `USymbolBoundary.getDrawing` normalizes positive/nonzero nonnegative Fashion shadow to4; Boundary path and ellipse both carry the shadow filter, while actor retains its separate Fashion. `EntityImageDescription` stereotype font resolves its own style signature, with inherited/overridden bold/italic/size/family/color and default italic. Native class generic-header overhang is excluded from description images because they do not paint `EntityImageClassHeader`. `SvekResult` uses the simple-image painted union rather than a supplemental Graphviz preferred box. `EntityImageDegenerated` retains preferred dimensions and its7px origin: the old actor-only painted-origin correction had incorrectly shifted it, which explains and repairs the preserved degenerate case.
- `crates/rustuml-parser/src/parse/class.rs`: the shared consuming trial executes anchored Java `CommandUrl` grammar against an existing quark. `CucaDiagram.quarkInContextSafe(true)` reuses a unique short name before current-context lookup. Strict `UrlBuilder` alternatives retain quoted URLs, tooltip-only URLs and topurl semantics. Existing note/class/group data receives the URL; no new entity or UID is allocated. Missing/forward targets and invalid inline named-note URLs still fail.

Preferred geometry and painted bounds remain distinct; no per-font subtraction, SVG parsing for measurement, fixture branch, Graphviz tuning, or output surgery was introduced. Native/layout C, shared fonts/metrics, sequence, TIM, comparator, guard inputs, target records and all previous accounts/reviews/references were untouched. Native cleanup review/fault tooling was not resumed or replaced.

## Exact outcomes and denominators

| Scope | Entries | Matches before → after | Remaining mismatches | Process errors | Invalid Java refs |
|---|---:|---:|---:|---:|---:|
| original810 | 810 | 553 → 553 | 250 | 0 | 7 |
| class23 | 23 | 23 → 23 | 0 | 0 | 0 |
| current955 | 955 | 610 → 610 | 329 | 7 | 9 |
| maker62 | 62 | 62 → 62 | 0 | 0 | 0 |
| checker139 | 139 | 124 → 124 | 11 | 0 | 4 |
| frozen1192 | 1192 | 821 → 821 | 341 | 17 | 13 |
| combined1449 | 1449 | 838 → 838 | 518 | 60 | 33 |
| combined1559 | 1559 | 930 → 930 | 534 | 60 | 35 |
| maker36 | 36 | 32 → 32 | 4 | 0 | 0 |
| maker20 | 20 | 16 → 16 | 4 | 0 | 0 |
| checker34 | 34 | 12 → 26 | 4 | 0 | 4 |
| all1649 | 1649 | 990 → 1004 | 546 | 60 | 39 |
| fresh26 | 26 | 2 → 20 | 6 | 0 | 0 |

The first smaller rows are overlapping subsets; do not add them. The final preserved1649 consists of1559+36+20+34 disjoint cohorts by source hash. The1559 retains74 explicit duplicate entries (1485 unique sources). Fresh26 is disjoint from all four cohorts. The complete1675 entries therefore represent1601 unique source hashes. Combining the two final replays arithmetically gives1024 matches,552 mismatches,60 process errors and39 invalid refs; this is not an additional independent run. The frozen1192/1449 scopes remain labeled, with no claim about a newer parent inventory.

There are **zero lost passes and zero new valid-input process errors**. The14 newly matching preserved cases are enumerated in `deltas-and-residuals.json`: twelve symbol cases plus `note_url_command_False` and `note_url_command_True`. The independent34 cohort changes from12pass/16mismatch/2errors/4invalid to26pass/4mismatch/0errors/4invalid. Both URL-command cases now retain their named note, class, optional relationship, URL/tooltip and original UID order.

All1595 previous debug AST captures (1559+36) are byte-identical before/after, including1560 successful AST outputs; failed/empty outputs are not presented as model proof. The20 previous YAML models are byte-identical. The34+26 model matrix has50 unchanged models and10 parse recoveries (the two preserved plus eight fresh CommandUrl controls). Explicit recovered-model checks retain one class, one note and the declared relationship count, with exactly one note UID. `model-retention.json` retains all objects and commands. The four invalid reference cases retain their CLI outcomes, recorded separately in `deltas-and-residuals.json`; an invalid Java SVG is never counted as parity.

The60 preserved errors remain33 sequence panics plus27 parse errors. They are not newly introduced. Every residual case, status, exit and stderr is enumerated. Previous cloud/nested-group/frontier, native/custom-header, qualified identity, TIM/sequence and font dependencies remain visible. Historical size12 one-pixel canvas discrepancies remain in immutable prior captures and are not called exact matches; a frozen-comparator pass is not byte-exact SVG equality.

## Every fresh failure and causal scope

The26 new controls are all valid XML/reference diagrams, rawexit0, no timeouts. Final20/26 match; all six remaining failures are preserved under `fresh-pairs/`:

- `actor_blank`, `boundary_blank`: logical `FontName Serif` falls through to Sans in Rust `text_render::metric_family`. Java `EntityImageDescription` passes the resolved family to its `FontConfiguration`/AWT string bounder. For example, Cedar at18 has Java width42.9785 vs Rust51.1172. Empty stripes now emit real space atoms, but their inherited Serif metrics still differ. Shared metrics are explicitly outside this packet.
- `actor_stereo`, `boundary_stereo`: independently resolved stereotype attributes now match (Serif13 italic DarkGreen versus Serif19 bold label), but the same existing logical Serif metric mapping changes widths and baselines. The whole cases remain failures; correct style attributes do not waive geometry.
- `group_left`, `group_top`: **two independent defects**. (1) Java `CommandFactoryNoteOnEntity.createSingleLine.executeArg` passes null URL to `executeInternal`, whereas its multiline path extracts/stores the URL. Rust `ClassParser.try_note` currently stores it for both, so these single-line controls emit an extra anchor. Parser ownership for this packet was limited to CommandUrl; no rendering suppression was used. (2) Group/edge/note geometry remains different, independently of the anchor. Java `ClusterDotString.printInternal` emits `shape=point,width=.01` for linked group quarks; Rust `class.rs` creates `GraphLayout.add_svek_cluster_endpoint` and includes it in the cluster. `GraphvizImageBuilder` requires a leaf `SvekNode` peer for Opale, so a group target correctly keeps a normal note plus compound-clipped `SvekEdge`. Existing exact-DOT evidence in `class-command-quark-note-ownership/checker-support/REPORT.md` documents .05-inch remote12.2 versus .01-inch vendor/local16.1 point behavior on preserved inputs. The solved cluster width differs before rendering (fresh controls4px; preserved review cases about3px), propagating to note and connector positions. This is an explicit graph/native/environment dependency, not a proven global environment explanation or permission to compensate coordinates.

The preserved four `note_group_left/top/right/bottom` cases and four prior20 group cases still fail whole SVG geometry. The four prior36 Q-contour cases remain unowned glyph failures. No group case is declared solved from DOM alone.

## Verification and intermediate failures

All final checks ran on production revision `d84b89cd8ee2ddd64ac71863004ec6ee84b1c4cd` with own `CARGO_TARGET_DIR=/Users/marcelo/Library/Caches/rustuml-class-cff4/target`, `CARGO_BUILD_JOBS=4`, pinned stdlib, and `RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`. Exact commands/environment/exit codes are in `corrected-gates.json` and the replay reports. Final CLI binary SHA256: `f999036ac976c7a6dda44d4bbd3d252e9382eaddb947d6ce9a0fb16c1551a713`. Frozen comparator SHA256: `0e368102f36bfbc59efe141286df5935b785535db6c0cf537bdbadc9350f21cd`.

- `cargo fmt --check`: pass.
- `cargo test --workspace --release --lib --no-fail-fast`:1444 pass (42layout,10math,31oracle,582parser,779render).
- `cargo test --workspace --release --test pipe_mode --test error_paths --test doc_claims --no-fail-fast`:13CLI,19error-path,2documentation tests pass.
- `cargo test --release --test golden_no_oracle -- --nocapture`:11188/11251 product matches; process remains red on frozen ratchet. All63 failure names unchanged.
- `cargo test --release --test golden_pairs -- --nocapture`:10523/11251 assisted matches,728 failures,1299 excluded of12550 total; process red. All728 failure names unchanged.
- `cargo test --release --test harness_guards -- --nocapture`:4/6; decimal and review-evidence guards remain red.
- `cargo clippy --workspace --release -- -D warnings`:red with the same20 pre-existing diagnostic sites; no new diagnostics.
- Final real CLI1649 replay and separate fresh26 replay use unchanged `compare_svg_strict` and frozen Java-error classification. Earlier focused253 description cases, old56 and34 comparisons are retained. Existing non-class `emit_text` callers were not modified; old sequence/non-class CLI outputs and all existing text unit tests remain covered by final regression checks. Two additive positioned-stripe tests exercise stored baseline/blank behavior and legacy emission compatibility; three parser tests cover named-quark URL retention, unique/ambiguous identities, strict URL forms and negatives.

No broad suite was repeated for evidence-only changes. Intermediate failures remain archived: v1 lost stereotype italic defaults, then corrected with source-owned default resolution; v3 omitted stereotype margin UEmpty and lost8 old56 plus2 preserved1559 passes, all restored by v4/final; the styled-label width trace exposed native generic overhang metadata after fixing runs. The earlier Graphviz-extent hypothesis did not by itself repair those cases. The first model-evidence script incorrectly expected a relationship in the intentionally unlinked `note_url_command_False` control; that assertion failure is retained and the source-derived expected count corrected. No product change followed final gates.

## Reference transport and immutable evidence

New sources were staged only in the private cache and sent by the authorized parent MonkeyMan relay. Input gzip SHA256 `a12e6b126779f08d38026834c3994d684332e1fd1aa08b70158b05828df91870`; returned gzip SHA256 `afc25b5c5958bd8643b43ab180e5d6bca96be0ddb4d25562469787dab740e4b4`. Both raw returned JSON and gzip, transport, source manifest, exact source/SVG pairs and stdout/stderr hashes are preserved. Private remote worker: `/Users/marcelo/rustuml-reference/scratch/worker.gFo3QrSD`; explicitGMT+11:00; pinned headless JAR SHA256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, source71806a23780b04a5ccde2f8ceb5121edad5eb711, remote OpenJDK21.0.5/Graphviz12.2.0. Earlier seven-case calibration is scoped, not global equivalence. There were no local Java/Javac/SSH launches, children or remote shipping actions.

`cli-iterations-models-and-gates.tar.gz` retains all final1649 CLI/AST/diff/stderr outputs, failed focused iterations, model captures, traces and full gate logs. Replay reports carry original source/reference hashes and actual commands. Scripts retain private-cache paths for reproducibility; copy them to that cache when replaying. Existing reference files remain in their original repository paths. `manifest.json` lists production files, full commits and checksums. This packet stops for a fresh independent checker; it authors no checker attestation.

## Full commit chronology

- `06dedeb975ba3b0ef70bb3b161de1f22d9b991a1` — test: reject incomplete symbol bounds and preserve note ownership review
- `5dae08ccf450e2e7a958fc2076b98e8d39062d3e` — docs: account for shared positioned description stripes before implementation
- `98f3e5ce8b40c208079897334d189d7172e68a6c` — docs: account for boundary factory shadow normalization
- `51b3754fa8a2b799cb06c6071c81cca32df46d71` — docs: account for independent description stereotype font
- `2b567395ed5fdb9103b315fd84f01b78cc1e971f` — docs: account for consuming URL command on existing note quarks
- `12b72619e304cbb5bb0259aefd243676094fb904` — docs: account for simple description painted envelope ownership
- `3a4453e17a51b3a57c1d4bcef8aa1792ebf220a4` — docs: account for native header envelope leaking into description images
- `00e00750b42f448ab45441c9d2cdd84b1ee5eb78` — docs: correct URL quark reuse and strict builder grammar account
- `838a47104cc3d220087507dacae91b64ccc644fc` — docs: account for stereotype margin wrapper drawing envelope
- `d84b89cd8ee2ddd64ac71863004ec6ee84b1c4cd` — fix(class): share positioned text drawing and consume named-quark URLs
