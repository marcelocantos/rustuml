# Simple-symbol painted bounds and note URL ownership: maker packet5

This is maker evidence, not an acceptance attestation. Production commit `96a4bde220eddbc3faf135cad6d13f338b7d7ed5` changes only `crates/rustuml-render/src/class.rs`. The two earlier causal accounts were committed separately:

- Painted bounds: `3790f114c79f2d19fdf91e46b84f1f79d85f6677`, `class-simple-symbol-painted-maximum/account.json`.
- Note URL ownership: `41d9a803623d00f1263eaaaa08ac56ab51a5eac0`, `class-note-url-group-ownership/account.json`.

The production commit names both exact account hashes. All older accounts, reports, reviews, captures, tolerances and baselines are unchanged. No parser, shared metrics/font, layout/native, sequence or TIM production file changed. The interrupted native review was neither resumed nor replaced; no fault-injection tooling was run or edited.

## Model changes

`entity_painted_extent` now computes the non-degenerated simple-symbol maxima from the same drawn components as Java `USymbolSimpleAbstract`: the actor/boundary paths, their ellipses, label baselines and stereotype baselines. `LimitFinder.drawUPath` measures path points; `drawEllipse` includes its shadow endpoint; `drawText` has maximum `baseline + 1.5`. These primitive maxima feed both body/chrome and final canvas unions, per `SvekResult.calculateDimension`. The preferred `calc_entity_dims` sizes and all Graphviz inputs remain unchanged. `EntityImageDegenerated` retains its preferred-size path. No per-font/fixture subtraction, glyph substitution, output postprocessing or Graphviz tuning was introduced.

Both `render_attached_note` and `render_floating_note_entity` now emit entity `<g>`, optional URL `<a>`, image, `</a>`, `</g>` in Java `EntityImageNote.drawU` ownership order. URL creation, entity metadata, source command consumption, note content and drawing coordinates are unchanged.

Two new renderer tests exercise whole rendering: simple-symbol canvas height against the independently reconstructed final text baseline over actor/boundary and three sizes; URL/group ownership over leaf/group targets and brace/end-note bodies. The first test's initial failure was a test attribute-lookup error, not a product mismatch: `y=` matched `font-family`. Its failed log is archived and the corrected lookup passes. No test-specific product branch was added.

## Exact scopes and outcomes

The preserved 1559 baseline is recovery4's 930 matches, not the earlier rejected 908-match candidate. The separate 36 baseline is the remote follow-up's 28 matches. The fresh20 baseline was rendered with the preserved pre-packet 7bd5 executable against these new references.

| Cohort | Entries | Matches before → after | Remaining mismatches | Process errors | Invalid refs |
|---|---:|---:|---:|---:|---:|
| original810 | 810 | 553 → 553 | 250 | 0 | 7 |
| class23 | 23 | 23 → 23 | 0 | 0 | 0 |
| current955 | 955 | 610 → 610 | 329 | 7 | 9 |
| maker62 | 62 | 62 → 62 | 0 | 0 | 0 |
| checker139 | 139 | 124 → 124 | 11 | 0 | 4 |
| frozen1192 | 1192 | 821 → 821 | 341 | 17 | 13 |
| combined1449 | 1449 | 838 → 838 | 518 | 60 | 33 |
| combined1559 | 1559 | 930 → 930 | 534 | 60 | 35 |
| maker36 | 36 | 28 → 32 | 4 | 0 | 0 |
| fresh20 | 20 | 6 → 16 | 4 | 0 | 0 |

The rows overlap: original810/class23/current955/earlier maker62/checker139 are subsets of the preserved larger cohorts. Do not add them. The 1559 entry inventory contains 1485 distinct source hashes and retains its explicit duplicates. The 36 maker inputs and 20 new inputs are distinct from it and from each other by source hash. The disjoint 1595 preserved entries therefore have 962 matches, 538 mismatches, 60 process errors and 35 invalid references after this packet; this arithmetic joins two separately recorded replays, not an additional combined run. Fresh20 remains a separately reported new cohort.

There are no lost passes or new process errors in any measured cohort. All 1559 AST debug captures are byte-identical before/after. All 20 fresh YAML models are also byte-identical, and explicit collection checks retain their expected entities, edges, groups, notes and URL/tag/stereotype/body metadata. `fresh-model-retention.json` records those counts; this is model retention, not full Java AST equivalence.

The prior size 12 boundary controls were tolerated 1px canvas discrepancies, not exact matches. Their before captures remain immutable. Both size 12 and both size 18 boundary cases now have exactly equal Java/Rust viewBox dimensions, recorded in `residuals-and-invariants.json`. The original four custom-Q cases remain failures; no shared glyph serializer change was attempted.

## Fresh20 reference provenance and counterexamples

The 20 new source controls vary font 10/22, one/two-line labels, stereotype, TB/LR topology, boundary fill/stroke, actor stroke/shadow, note top/right/bottom/left positions, brace/end-note delimiters, leaf/group targets, URL tooltip and link target. Parent ran their exact bytes through the existing Monkey Man relay. Worker: `/Users/marcelo/rustuml-reference/scratch/worker.OAeKKbU4`; explicit timezone `GMT+11:00`.

Input gzip SHA256: `73030d7e7e58583614562a6a02b3d674bbab4e007886ed086ffa39efa9f4e586`.
Returned gzip SHA256: `8ce0ccae44313a8209408f46baa615f0482c719209bfe368e2c4d8ef7344fc56`.

Exact input manifest/envelope, raw returned JSON/gzip/transport, decoded stderr and source/reference pairs are preserved. Every membership, source hash, stream hash, gzip/transport correspondence, explicit headless Java argv, pinned JAR, timezone and zero-timeout status was checked. All 20 outputs are well-formed SVGs without the frozen error markers; all remote exits are 0. Exit zero was not used as a parity verdict. All reference stderr is empty. Java source 71806a23780b04a5ccde2f8ceb5121edad5eb711 and JAR SHA256 aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053 remain pinned. Remote OpenJDK 21.0.5 / Graphviz 12.2.0 calibration is scoped, not global equivalence.

The fresh matrix improves 6→16 matches. These four valid cases still fail and remain ordinary executable source/reference pairs under `fresh-pairs/`:

- `note_top_False.puml`: group outline right edge 155 versus 152; note/edge x 145 versus 144, plus existing spline differences.
- `note_right_True.puml`: group right edge 155 versus 152; note x 181.93 versus 178.93; canvas width 375 versus 372 and connecting spline differences.
- `note_bottom_False.puml`: same group width residual; note/edge x 145 versus 144 and spline differences.
- `note_left_True.puml`: group right edge 357.07 versus 354.0713; canvas width 372 versus 369 and connecting spline differences.

All four target a group. Every parsed non-group/non-anchor XML tag, attribute and text record (including root, paths and text) is identical between the old and new Rust outputs; the only candidate change is the corrected URL/entity nesting. The anchor parentage and attributes now match Java in all eight note controls. These observations isolate an already-present group outline/layout/edge dependency; they do not waive its failure or establish a single cause. The earlier remote point-node/environment discrepancy remains unresolved and was not used to tune geometry. No production expansion followed these failures.

All 12 new simple-symbol controls and all 4 leaf-note controls match. Four group-note controls still prevent any whole-packet acceptance claim. The new controls and all residual outputs are preserved regardless of status.

## Stabilized checks and binary chronology

Final gates ran at production commit 96a4bde220eddbc3faf135cad6d13f338b7d7ed5, with the existing isolated Cargo target `/Users/marcelo/Library/Caches/rustuml-class-cff4/target` and `CARGO_BUILD_JOBS=4`. Exact commands, cwd, environment, durations and revisions are in `corrected-gates.json`:

- `cargo fmt --check`: pass.
- `cargo test --workspace --release --lib --no-fail-fast`: 1439 passed (42 layout, 10 math, 31 oracle, 579 parser, 777 render); two renderer tests added.
- `cargo test --workspace --release --test pipe_mode --test error_paths --test doc_claims --no-fail-fast`: 13 CLI, 19 error-path, 2 documentation tests passed.
- `cargo test --release --test golden_no_oracle -- --nocapture`: 11188/11251, same 63 failed pairs; exit 101 on frozen ratchets.
- `cargo test --release --test golden_pairs -- --nocapture`: 10523/11251, same 728 failed pairs; exit 101. Existing exclusions 1299 of 12550 unchanged.
- `cargo test --release --test harness_guards -- --nocapture`: 4/6; decimal and independent-evidence guards remain red.
- `cargo clippy --workspace --release -- -D warnings`: same 20 baseline diagnostic messages/sites after ignoring line shifts; exit 101.

No main failure was exchanged for another; `gate-delta.json` compares complete failure-name lists. No baseline/guard/skip edit was made and no red process is called a green gate.

The 1559 and 36 CLI replays used the v1 release executable built after all production edits. Only the two cfg(test) additions and a test attribute lookup changed afterward. The final release rebuild has a different binary SHA; it is **not claimed byte-identical** to v1. The final binary rendered the 20 fresh cases, and final release tests/main tiers used the committed source. `binary-identity.json` and per-run records preserve both hashes. Per the instruction against duplicate broad runs when production is unchanged, the 1559/main suites were not rerun merely for returned references. A fresh checker should test the final whole revision independently.

The copied final binary initially lacked execute permission; the first fresh comparison stopped before launching it. Completed baseline output and the failure chronology are archived. Restoring executable permission allowed the matrix to finish; no source/reference or production change was involved.

## Residue and delivery

The previous 1559 inventory still has 534 SVG mismatches, 60 process errors and 35 invalid references. The 60 errors are the pre-existing 33 sequence panics plus 27 parse errors, not newly introduced failures. All exact errors and every per-case status remain in the report/archive. Prior cloud/chrome, nested-group/frontier, native/custom header, font, quoted identity and TIM/sequence dependencies remain. Four Q-contour failures are explicitly unowned shared-metrics work. Nothing in this packet closes those mechanisms or updates a rejected record.

`inventory-v1.json.gz`, `focused-v1.json.gz` and `fresh-comparison.json` contain actual CLI commands, output/reference hashes, exits and unchanged comparator results. `preserved-cli-and-logs.tar.gz` holds all 1595 preserved output sets, final gate logs and intermediate instrument failures. Fresh20 has exact sources, references, old/new SVGs, structured models and full diff/error streams directly under `fresh-pairs/`. Scripts are retained with private-cache paths for reproducibility; copy them to that cache before running. No local JVM/SSH, children, native work, pushes, PRs or releases occurred. This local maker packet stops for a fresh independent checker; it does not claim acceptance.

Raw comparator stdout retains its original trailing blank lines; these evidence files are intentionally not whitespace-normalized. Production formatting was checked separately.
