# Class recovery continuation: maker results, not acceptance

Tested production revision: `7bd5c9f5d2f4dee23472ddc8e3ac4b96a3e196e3`. The imported independent evidence rejected all 15 claims. Those records, all earlier records, and all supplied reference bytes remain unchanged. No checker verdict is authored here. A fresh independent checker is still required, and the failures below prohibit treating the packet as accepted.

## Changes and causal closure

Production files are exactly:

- `vendor/graphviz/lib/rustuml_helpers.c`: allocate each cell before growing/publishing the NULL-terminated ownership array; failure cleanup can never traverse an uninitialized sentinel. WIDTH/HEIGHT now follow Graphviz `htmllex.doInt` integer-prefix `strtol` semantics: `1e2` is 1, `65535.9` is 65535, and blank input is invalid. Grid geometry is unchanged.
- `crates/rustuml-parser/src/diagram/class.rs`: retain note stereotype, tag, URL and tooltip metadata in the ordinary model.
- `crates/rustuml-parser/src/parse/class.rs`: one consuming attached-note grammar owns modifiers and whole braced/end-note bodies, resolves named-note quarks, and rejects invalid targets rather than falling back. Ancestor placeholders have no UID until actual materialization. `net/atmp/CucaDiagram.reallyCreateLeaf` materializes parents only for LIKE_CLASS leaves; `gotoGroup` and `getTextBlock` own later phases. Explicitly reopening an already materialized group preserves its earlier metadata. Header/footer line ownership retains blank lines. Six new parser tests cover metadata, whole bodies, named peers, allocation order, and negative inputs.
- `crates/rustuml-render/src/class.rs`: selected-symbol stereotype merge alignment, actor Fashion shadow preferred/painted dimensions, boundary stereotype painted minimum, alternate package visibility composition, large/tiny member icon placement, blank ribbon fallback and line metrics, named/empty peer note links, connector construction/serialization order, per-primitive shadow frontier, and measured header circle centering.

The member placement model deliberately keeps `MethodsOrFieldsArea.calculateDimensionOnlyMembers` preferred height distinct from `PlacementStrategyVisibility.getPositions` drawing advances. The corresponding corrective account was committed before that correction. Likewise, the first final main-tier run exposed a singleton shadow canvas regression: `EntityImageDegenerated` uses preferred size, whereas `SvekResult` measures painted bounds. The final correction confines new shadow frontier expansion to the latter. The failed run and corrective chronology are preserved, not overwritten.

All mechanism plans preceded their production changes. Full chronological hashes are in `commits.txt`; the production commits are:

- `7fa39737be47590a9186d48066358ea1bbaa6b1f` (native cleanup/numeric contract; account `3e94010e384baac280d282f256becae65ddcbdab`).
- `aba7e2a86753e1ae19765868d54d86150263e910` (parser and dependent image composition; exact earlier account hashes are in its commit message).
- `7bd5c9f5d2f4dee23472ddc8e3ac4b96a3e196e3` (degenerated shadow boundary; account `197b020094784f6a825bfebc47c920b13e420056`).

The parent 257-entry addition and two missing include dependencies were imported byte-for-byte from the committed checker-tested parent tree. Existing paired files were verified unchanged; `inventory-import.json` records the import. The root-cwd include difference is an input dependency, not a geometry improvement. No TIM, sequence, shared font/style, comparator, guard, baseline or target changes were made.

## Exact preserved-reference CLI scope

Baseline is the independent candidate in `fc698b9b42abf2870c3ea27e44d07984211cc8c9`, not an inferred previous headline. Counts include explicit duplicate entries. Invalid references remain in the denominator and never count as passes. These are overlapping cohorts, not additive rows.

| Cohort | Entries | Matches before → after | Remaining mismatch | Process errors | Invalid reference |
|---|---:|---:|---:|---:|---:|
| original810 | 810 | 546 → 553 | 250 | 0 | 7 |
| current955 | 955 | 603 → 610 | 329 | 7 | 9 |
| class23 | 23 | 23 → 23 | 0 | 0 | 0 |
| maker62 | 62 | 62 → 62 | 0 | 0 | 0 |
| checker139 | 139 | 124 → 124 | 11 | 0 | 4 |
| frozen1192 | 1192 | 814 → 821 | 341 | 17 | 13 |
| combined1449 | 1449 | 831 → 838 | 518 | 60 | 33 |
| checker62 | 62 | 32 → 47 | 13 | 0 | 2 |
| maker48 | 48 | 45 → 45 | 3 | 0 | 0 |
| combined1559 | 1559 | 908 → 930 | 534 | 60 | 35 |

All 22 repaired paths, all 60 remaining process errors with exact stderr, all 35 invalid references, and all 16 remaining mismatches in the latest 110-input class cluster are enumerated in `corrected-deltas.json`. There are **zero lost passes and zero new process errors** in the complete scope. The one repaired process error is named-note attachment (`note_peer`). The remaining 60 errors are the same 33 sequence panics and 27 parse errors already present in the independent baseline; they are visible dependencies, not 60 newly introduced bugs.

The original 23 class inputs remain 23/23; earlier maker62 remains 62/62; checker139 remains 124 matches, 11 mismatches and 4 invalid references. The current scope is 1559 = parent1449 + latest62 checker + latest48 retained maker copies. It does not claim coverage of subsequent parent additions. `corrected-cohort-membership.json` preserves membership derived from the original independent manifests and earlier recorded cohorts.

## Tests and product path

The real release CLI rendered every paired source and compared it using the unchanged repository strict SVG comparator. Source/reference SHA256 values match the independent baseline for every entry. CLI and AST commands, stdout, stderr, exits, comparator output and binary hash are retained in `corrected.json.gz` and `cli-models-logs.tar.gz`. `--ast` produces debug text despite the historical `.ast.json` artifact extension; structured model checking uses `--yaml` separately.

Final checks at the final production revision:

- `cargo fmt --check`: pass.
- `cargo test --workspace --release --lib --no-fail-fast`: 1437 passed (42 layout, 10 math, 31 oracle, 579 parser, 775 render), up from 1431 through six parser tests.
- `cargo test --workspace --release --test pipe_mode --test error_paths --test doc_claims --no-fail-fast`: 13 CLI, 19 error-path and 2 documentation tests passed.
- `cargo test --release --test golden_no_oracle -- --nocapture`: **11188/11251**, unchanged from the independent baseline, with the same 63 failures. Process exit 101 because frozen family ratchets are not updated; this is not a green gate.
- `cargo test --release --test golden_pairs -- --nocapture`: **10523/11251**, unchanged, with the same 728 failures; exit 101. Of the complete 12550 corpus, 1299 are excluded by existing rules.
- `cargo test --release --test harness_guards -- --nocapture`: **4/6**, decimal and independent-evidence guards still fail. No allowlist, ratchet, baseline or rejected record was modified.
- `cargo clippy --workspace --release -- -D warnings`: exit 101, the same 20 diagnostic messages/source sites as the checker baseline after ignoring shifted line numbers. `gate-comparison.json` records both lists.

`corrected-gates.json` records exact commands, cwd, environment, durations and revisions. Each command uses the dedicated cache target and `CARGO_BUILD_JOBS=4`. The first final-tree product run at aba7e2a had 11187 passes due to `class/class_skinparam_shadowing`; its log and actual SVG are retained. The final correction restores that exact failure without exchanging another pass. Both complete failure-name lists are unchanged from the independent baseline.

Native safety: **242 ASan/UBSan runs** passed the helper-specific failure/numeric matrix already committed under `class-static-grid-lifetime-integer-contract/maker-evidence`. This includes every reachable allocation failure for four shapes, retained native controls, and 24 WIDTH/HEIGHT cases. An additional **11 runs of the original checker injector** all finish safely; its formerly crashing allocation index 3 now returns controlled unsupported/error 10 without sanitizer diagnostics. The exact checker source/input/results are preserved here. Instrumentation covers the included helper; the linked native archive is not wholly ASan-instrumented, so this is not a claim of whole-engine memory safety.

The 1559-entry YAML model census shows no dropped class collection elements or entity identities compared with the independent baseline.50 model errors remain. Restoring the parent selected-subroutine include recovered `Selected`; only that case was rechecked after the full census because the parser had not changed. `corrected-model-retention-summary.json` explains this incremental provenance. This census is not full Java AST equivalence.

There are 36new source-guided controls (32image/note controls plus 4single/linked shadow controls). All 36 render and retain expected model object counts in Rust at the final revision; the 32 earlier controls were replayed once because the shadow correction changed the renderer. Their earlier captures remain in the private cache. Four custom-header center invariants measured during implementation also hold. **Their Java validity and parity remain unverified** while remote transport is unavailable. They are additional pending evidence, not paired successes. Existing references were reused without Java regeneration.

## Remaining counterexamples and integration dependencies

- Nested group notes and final group frontier/coordinate ownership: `nested_brace`, `nested_group`, `nested_lr`, `nested_wide_lr`, `reversed_tb` remain mismatches. Source quark/UID and link ownership repairs are not sufficient to close their SVG geometry. The exact remote DOT point-node .05 versus local native .01 discrepancy remains unresolved; no geometry was tuned around that environment difference.
- Cloud/chrome: retained `chrome_center`, `chrome_left`, `chrome_right`, and `cloud_nested` retain cloud model/coordinate residuals. They are not waived by correct command recognition.
- Ribbon fonts: `header_mono`, `footer_serif`, `title_italic` still fail. A measured mono header expected 217.5166 width versus 251.6572 actual; the preserved Java glyph advances appear proportional despite the monospace family label. The shared font resolution/environment path needs characterization, not a per-fixture width correction. No shared-font rewrite is included.
- `root_note` and `uml1_note` retain frontier/source-location differences. Leading-blank source lines differ by one, and group bounds remain different (for example 162 versus 158). Source identity and point-node/environment explanations remain dependencies, not established resolutions.
- `bare_quoted` and `order_stereo_entity` are sequence-owned shared-entity stereotype failures. The parent is notified through this handoff; no sequence file was edited.
- Existing theme/native-header cases remain beyond the latest 110: current custom-spot glyph/color and measured circle centering do not close legacy stereotype font/color resolution. `EntityImageClassHeader` uses CLASS_STEREOTYPE font configuration while Rust still inherits some name/root font choices; preserved custom-class measurements include expected #343A40 versus #FFF and 61.2366 versus 58.2764 text width. Native group header theme metrics and quoted qualified identity/note geometry also retain failures. All remain executable in the 1559 scope.
- TIM residual commands are not ignored. The parent's earlier independent evidence showed five class-parser TIM regressions resolve on a separate TIM candidate, but that candidate was not accepted/integrated here and the combined strict parser remains unverified. Existing sequence panics similarly remain the sequence worker's dependency. No cross-packet success is claimed.

## Reference and replay provenance

No local Java or javac was launched. Source inspection found the dormant unguarded `Command::new("java")` in `crates/rustuml-oracle/src/runner.rs:105`; it was not invoked or changed. Gate PATH rejects java/javac, and JAVA_TOOL_OPTIONS preserves existing options plus headless. The logs contain no attempted-launch marker.192.168.1.112 transport is unavailable; no new endpoint or connection polling was used. Fresh source paths are under this directory's `fresh-controls/` and `shadow-controls/`, ready to stage in the worker-private remote directory when supplied.

Pinned Java source71806a23780b04a5ccde2f8ceb5121edad5eb711, JAR SHA256aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053, stdlib747d1eae809cde20675976d67e3eb84dc0021a0c, and golden submodule9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a are recorded in `provenance.json`. Existing remote OpenJDK 21.0.5 / Graphviz 12.2.0 calibration is scoped, not proof of global equivalence; historical fonts/Graphviz byte-equivalence remains unproven.

Reproduction: build the tested revision with `CARGO_TARGET_DIR=/Users/marcelo/Library/Caches/rustuml-class-cff4/target CARGO_BUILD_JOBS=4 cargo build --release --workspace`; copy the release binary to the cache's `corrected-rustuml`. Compile the unchanged comparator with the command in `comparator-build.log` (select the current matching rlib if its build hash changed). Copy the replay script to the cache, write the `combined1559` membership list from `corrected-cohort-membership.json` as `all1559.json`, then run `python3 replay.py corrected RELEASERUSTUML CACHE/all1559.json REVISION` from the repository root. Use a new output name to preserve these captures. Set the exact stdlib/debug date environment recorded in the script. Copy `corrected_gates.py` to the private cache to rerun the final batch; it installs the rejecting JVM wrappers before any Cargo command. These scripts intentionally target the maker's private cache, not repository scratch.

`cli-models-logs.tar.gz` holds the final CLI captures, structured models, final gate logs and preserved failed singleton run. Earlier iteration result records are compressed separately so regressions discovered and repaired remain visible. Private complete iteration captures remain in the cache; no outputs were overwritten or references regenerated. All delivery is local; no push, PR, release, target edit or acceptance attestation.
