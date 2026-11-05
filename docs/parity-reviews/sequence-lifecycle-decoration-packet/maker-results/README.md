# Narrow sequence lifecycle and decoration maker packet

Measured production revision: `41e2ce88aec0c6856d57f0c2322ca097ecd55de4`.
Starting candidate: `0fb4e14b7227eb768d268128d42fa5f6d5a5b1af`.
Independent rejection `7ab58bbc0be8210cedc7654fa7f0bc52fc6425eb` was imported as `9af2148f95db1ace898bc0942fbf32a59fb30e38`. All its reviews, sources and references are unchanged. This packet is maker evidence, **not acceptance**. The prior eleven rejected claims remain rejected pending a new independent checker. The box-title and external-arrow bounded acceptances apply only as recorded at their full tested revision; this packet does not claim extracted acceptance.

## Measured results

All comparisons use the unchanged checker comparator and real `rustuml -tsvg -pipe`, with exact preserved input bytes. Invalid Java references are excluded from positive parity and retained separately.

| Cohort | Entries | Before matches | After matches | After valid mismatches | Invalid references |
|---|---:|---:|---:|---:|---:|
| Preserved prior maker/checker inventory | 1,728 | 1,160 | 1,160 | 542 | 26 |
| Independent checker fresh cases | 68 | 39 | 46 | 15 | 7 |
| New compact maker controls | 32 | 9 | 26 | 0 | 6 |
| Exact combined replay | 1,828 | 1,208 | 1,232 | 557 | 39 |

There are **24 recovered valid entries, zero lost passes, zero valid CLI errors, zero CLI panics and zero timeouts**. The product-library adapter agrees with CLI on all 1,789 valid inputs. Assisted-library matches improve from 1,165 to 1,189 with zero lost passes; its other 600 valid mismatches remain failures. The pre-existing empty invalid-date reference still causes the diagnostic `diff_one` adapter to exit 101. That is not a renderer panic or a valid-reference result.

Nineteen previously accepted invalid inputs now reject: eight from the old 1,728, five from the fresh checker and six new maker controls. Overall invalid status changes from eight rejected/31 accepted to 27 rejected/12 accepted. Every negative retains source, Java output/status and Rust output/status. A corrected rejection is not counted as an SVG match.

Seven fresh checker failures recover: legacy repeated returns, parallel return identity, nested return after destruction, Teoz parallel events, Teoz creation followed by destruction, standalone destroy callback allocation, and the active `x--` self-cross case. Every other checker case stays in the replay. The four true reverse controls use `<-x`, across both engines and depths zero/two; all match. The original `x--` example is **forward starting CROSSX**, not reverseDefine. The immutable source refinement at `42ec7126` corrects the maker's initial hypothesis; no checker wording was rewritten.

The frozen 955 portion stays at 403 matches/542 valid mismatches/10 invalid references. All 144 valid prior checker cases, 22 retained failures, 535 prior maker cases and 56 previous new valid cases remain matching. Original grouped-note 41 remains 39 matching/two failed font controls. The 1,828 entries have 1,828 paths but 1,780 distinct source byte strings (48 duplicate entries retained). Parent's 1,449 inventory overlaps the old maker inventory at 1,123 paths; its 326 parent-only entries were not rerun. Do not sum these overlapping cohorts. Parent owns combined integration.

## Causal implementation and chronology

Every account below was committed before its corresponding production mechanism. Full file hashes and commit mapping are in `implementation-map.json`.

| Mechanism | Preimplementation account commit | Implementation commit(s) |
|---|---|---|
| Command/parallel/return identity and validation | `ea313c6203844a0087bc40261366f5bace617d43` | `e6a4d41b3a2210cc5c0a8f6047d25edd39bbb07d` |
| Teoz lifecycle event scheduling and painting | `ea313c6203844a0087bc40261366f5bace617d43` | `19074587f391b3b0bd5306cdce116fb564d06f97` |
| Self-cross decoration and endpoints | `ea313c6203844a0087bc40261366f5bace617d43` | `a8d717ca3bc8733c9b1deb2e14dbb569e6e547bc` |
| Legacy standalone/attached destroy component | `bc18300d13f13f31b50018abe57bfff4bec637db` | `19074587f391b3b0bd5306cdce116fb564d06f97` |
| Legacy ordinary message-note envelope | `bc18300d13f13f31b50018abe57bfff4bec637db` | `07aa161142fa8631629f6d94bfa776d11812820d`, `41e2ce88aec0c6856d57f0c2322ca097ecd55de4` |
| Styled composite integration correction | `76972e36bd9abbeb264934bc112dea72314a54c5` | `41e2ce88aec0c6856d57f0c2322ca097ecd55de4` |

Additional preimplementation source refinements: `42ec7126f6559d832d7ba5bbb9c293d152ca8afa` (cross direction), `09eec925d59a83a461606ca26b7138f903ee999e` (event-envelope dominance), `14228d60539c365b92f96d3051c99cdf05f22e2c` (already included composite shadow).

`CommandReturn`, `SequenceDiagram` and `AbstractMessage#addLifeEvent` distinguish command ownership from painted activation depth. The shared sequence resolver preserves parallel identity and the activation-command return stack, treats DESTROY separately from DEACTIVATE, lowers return arrow configuration before layout, and rejects prohibited same-message reopening and unknown return colors. `Message.parallel` and `ReturnMessage.parallel` are serde-defaulted. Four negative transition combinations and unknown color controls are exercised through real CLI across both engines; valid named colors remain valid.

Teoz raw history preserves CREATE without adding activation depth and resolves parallel ancestry directionally. Its staircase callback traversal retains the existing measurement/background/draw passes; attached destruction no longer masquerades as deactivation. Standalone destroy consumes the cross component's preferred height and participates in callback scheduling. Creation uses the participant component and arrow tile dimensions, including creation-time live endpoints and paint ordering. Legacy standalone and attached self-destruction use their actual component/event ordinates. This does not rewrite the retained folded-note Real phase or claim closure of all Teoz models.

`ComponentRoseSelfArrow` decorations adjust the body endpoints and paint both cross strokes. Forward starting `x--` and true reverse `<-x` are modeled separately. Ordinary message/right-note envelopes consume `MessageArrow` live endpoints and `ArrowAndNoteBox` preferred dimensions, including the note's component-start live shift. Final validation caught and repaired integration regressions in alignment-selected margins, existing theme-envelope dominance and duplicate shadow reservation. These corrections preserve existing themed unit expectations and restore all eight temporarily lost main-tier cases. Intermediate failing outputs/logs remain archived.

Changed code files only:

- `crates/rustuml-parser/src/diagram/sequence.rs`
- `crates/rustuml-parser/src/parse/sequence.rs`
- `crates/rustuml-render/src/sequence.rs`
- `crates/rustuml-render/src/ascii.rs`: exactly two authorized test-constructor `parallel: false` additions; no ASCII behavior change.

No class, TIM, shared font/style/layout/API, registry cleanup, comparator, frozen fixture, baseline, target or checker attestation was changed. `protected-evidence.json` records the immutable-evidence diff check.

## Validation and reproduction

Final release units: parser **567/567**, renderer **783/783** (includes ASCII), CLI pipe **13/13**. The old raw-history unit's invalid deactivate/reactivate branch now asserts parser rejection; its valid lifecycle-mode coverage remains. No unit was disabled or removed.

Final main release library tiers: product **11,181/11,251**, assisted **10,497/11,251**, sequence **1,411/1,411**. Both counts and exact failing-name sets match the previous maker packet. These are library tests, not 11,251 CLI processes. The product suite still exits 101 because unrelated existing improvements are not locked into the frozen baseline; assisted retains 754 valid mismatches. No ratchet was edited.

The standing 1,191 perturbation suite has **614 matches/577 failures** at Cargo-package cwd. The isolated root-cwd replay has **615 matches/557 valid mismatches/19 invalid references** on those same 1,191 paths. The single additional package-cwd failure is the previously documented include-context T14.6.2 dependency. Root cwd is the explicit base for all isolated CLI measurements, not an implicit reference rewrite.

Guards remain **4/6**, with decimal ratchet and review completeness red. Owned formatting passes. Workspace formatting remains red in pre-existing class files. Strict Clippy is blocked by parser/component dead code; renderer-only Clippy retains its previous 22 findings. All logs and nonzero exits are preserved. No known failure was converted into success by exclusions or a comparator change.

`commands.json` and `checks.json` contain exact commands, cwd and statuses. `binaries.json` binds the measured binaries and four source files to the production revision. Final checks were rerun after code changes exposed by the first validation; the first failed run is retained under `logs/revision19074587-final-logs/`. The final 1,828 replay was run once on the final production source.

```text
python3 docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py PRIVATE_CACHE/all1828.json PRIVATE_CACHE/bin PRIVATE_CACHE/final1828
cargo test --release --lib -p rustuml-parser -p rustuml-render
cargo test --release -p rustuml --test pipe_mode
cargo test --release --no-fail-fast -p rustuml-oracle --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards -- --nocapture
```

Cargo uses the existing private `/Users/marcelo/Library/Caches/rustuml-sequence-repair/target`, jobs capped at four. Captures first land in `/Users/marcelo/Library/Caches/rustuml-sequence-lifecycle-packet`, then are deliberately packaged here. `cli-captures.tar.gz` retains final and intermediate raw outputs plus before measurements. `measurements.json.gz`, `before-new32.json.gz`, and the immutable checker `replay1728.json.gz`/`fresh.json.gz` support the before/after comparison. `package_evidence.py` packages existing captures only; it launches no JVM.

## Reference provenance

No local Java or javac was launched. Parent relayed the three exact source manifests to Monkey Man and verified returned membership, all input hashes, command arguments (only private cwd substituted), stdout/stderr base64 integrity and environment. Thirty-two captures completed without timeout: 26 exit zero, six exit 200. Exact source bytes, Java SVG/error output, stderr and per-case argv/cwd/status are retained under `inputs/`; full returned payloads are retained as gzip JSON.

Pinned JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`; Java source `71806a23780b04a5ccde2f8ceb5121edad5eb711`. Wrapper forces headless AWT and pins Java/JAR/dot. Per-case timezone is GMT+11:00. Remote Homebrew OpenJDK 21.0.5 and Graphviz 12.2.0 differ from the origin environment. The seven calibration pairs establish scoped agreement, not universal environment equivalence.

| Batch | Worker | Returned gzip SHA256 |
|---|---|---|
| Lifecycle 24 | `worker.u8h2B0IO` | `158ce6a8ae5af77bc1b1f0e42f20d203d313cf9575c91e13c78390c59c88f0c3` |
| Reverse controls 4 | `worker.HELFXbN7` | `0d57ea2467bb0b97652f30ea7c6ea9e0969ca3b00c1c4b5366075cdabed743be` |
| Legacy note controls 4 | `worker.FJsbADAm` | `485b3a2e2e96872995759068fb8b66fd0d2b042b82766eb4405e4d682dc5bf11` |

Original request compressed SHAs respectively: `952a45d01b84e3842abba33e52e34706be3a558032ef69dc6bd9b08ec547cf84`, `17ac9d5557ecc3a48df1af4b5e4c37d2d57fa50a2961686294d58a357cd470e1`, `4b658e02e03cb98a2af806b73c7f5be911dcea0f8aa88c06e9c4d97b3175c056`. Full environment objects accompany each batch.

## Remaining dependencies

`dependencies.json` names every one of the 15 remaining fresh valid failures: page chrome/multiline caption/top legend/styled caption (six), Teoz right-note canvas (two), boxPadding/nested englober (four), multiline delay (one), actor shadow preferred geometry (one), and hex-note/group shared bounds (one). These were deliberately deferred by the narrowed assignment and are not waived or claimed repaired. `remaining-failures.json.gz` retains all 557 valid mismatches and 39 invalid references with complete diagnostics; `negative-input-status.json` identifies the 12 still-accepted invalid inputs, including the two spaced right-border negatives.

Both logical font controls remain failed under unowned T14.11. Physical-font and shared family/hyperlink serialization dependencies remain active. The shared entity command review `fc698b9b` has two independent sequence stereotype failures; exact original source/reference pairs and hashes are preserved in `dependencies/shared-entity/`. Their downstream declaration/stereotype cause does not justify expanding this lifecycle packet into class/shared parsing.

The next step is independent adversarial checking of this narrow packet. No maker-authored `review.json`, target achievement or whole-sequence closure is asserted.
