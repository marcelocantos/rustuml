# Pending CREATE and sequence stripe packet — maker evidence

**New independent checker required. No acceptance claim.** Tested production: `653b1ed7a49784e6e6205f3aebf69a16278f8d78`. Baseline: `711dbca7c5e29592395f0927881aabfc2517b217` (production `a4c3c35794ae1d7117d709134ed22c7191db9db8`). Rejection `96e2561122e70ffe53dfa3f102cf18d199cc464f` was imported unchanged as `2766d2c5425786df05cf62fed538003a6d2a52d3`. Its nine rejected claims remain rejected. All 8,462 baseline evidence paths, including the previously protected 8,324, and all 183 imported paths remain byte-identical. Earlier bounded acceptances stay tied to their exact tested revisions.

## Measured shipped path

The unchanged comparator measured real CLI outputs for every retained source. The 2,169 entries contain 2,121 distinct source-byte strings and 48 retained duplicates. Checker64, maker20 and handoff2 are byte-disjoint from the preserved 2,083 and each other. Parent1449 is not additive: its historical overlap with the preserved cohort is 1,123 paths / 1,097 distinct bytes, with 326 paths / 278 distinct bytes not replayed here. No unrelated parent growth was imported.

| Cohort | Entries | Before match / valid failure / invalid | After match / valid failure / invalid |
|---|---:|---:|---:|
| preserved2083 | 2083 | 1448 / 576 / 59 | 1448 / 576 / 59 |
| checker64 | 64 | 42 / 16 / 6 | 50 / 8 / 6 |
| maker20 | 20 | 0 / 16 / 4 | 16 / 0 / 4 |
| handoff2 | 2 | 0 / 2 / 0 | 0 / 2 / 0 |
| **Total** | **2169** | **1490 / 610 / 69** | **1514 / 586 / 69** |

There are **24 whole-output recoveries, no lost passes in any comparison mode, no new valid-input errors and no CLI or library-render panics**. All 2,083 preserved statuses remain unchanged. The 20 new references classify as 16 valid Sequence outputs and four command-error XML diagrams, not merely from raw exit codes: each negative contains the incompatible pending CREATE diagnostic and has exit200; all valid outputs have exit0 and Sequence metadata. No timeout or new intention disagreement occurred. The checker's six intention/observation disagreements and every negative remain immutable.

The 69 invalid references receive no SVG credit. Rust rejects49/accepts20, versus45/24 before; the four new rejections are pending incompatible-receiver/self cases in both engines. Two empty historical references still panic inside `diff_one`'s comparison helper in each library diagnostic mode. These are not renderer panics. `negative-status.json` retains all statuses. Product-library diagnostics match 1514; assisted diagnostics match1472, with all differences and stderr preserved independently of CLI results.

## Semantic regression checks

Whole-pass counts alone previously missed an extra created head. `semantic-inventory.json.gz` now retains Java/before/after tag counts, labels, available head groups and geometry for all 2,169 entries. Eighteen outputs change geometry; eleven change structural counts (including invalid rejection output). No valid case acquires a larger excess tag or label count against Java. This bounded diagnostic is not proof of complete semantic equivalence.

The two checker `review711_create_double_pending` witnesses paint `Roe` three times before versus Java twice; final paints twice in each engine. New actor `pending_same` witnesses have four actor ellipses/labels before versus Java three; final has three, preserving the two attached creations plus the tail. `pending_different` has three actor ellipses before versus Java/final two. `creation-witnesses.json.gz` preserves exact shapes and labels. Raw ASTs keep all three CREATE events in the new valid witnesses, including overwritten unowned events. Already-attached repeats are not removed. Unit coverage checks that the first pending event is unowned and the later two own their exact messages, including same/different participants and incoming/outgoing external compatibility.

Actual `--ast -pipe` and `-ttxt -pipe` captures cover the prior62 controls, all64 checker,20 maker and two handoff inputs (148 entries). Four ASCII outputs change from successful invalid render to parse rejection; the other144 measured ASCII outputs are unchanged. There is no ASCII file edit or global behavior-invariance claim. Raw model/status/output and event counts are preserved.

## Causal models and chronology

All five accounts were committed before drafts in `4aaab169442b1e8a1d9e896d0cd3f019de6803d4`. Only `crates/rustuml-parser/src/diagram/sequence.rs`, `crates/rustuml-parser/src/parse/sequence.rs`, and `crates/rustuml-render/src/sequence.rs` changed. No serialized field was added. No parse/mod, ASCII, class, shared text/style/font, TIM, target or native change was made.

1. **Single pending CREATE identity.** Java `SequenceDiagram#activate` overwrites one pending event; `addMessage` validates compatibility, attaches that event and clears pending. `Message#compatibleForCreate` excludes source/self; `MessageExo` accepts its sole real participant. A shared sequence event-index attachment derivation now drives legacy head drawing, spacing and Teoz history. Raw events and prior attachments survive; `LifeLine#setCreate`/`LivingSpace#goCreate` still independently overwrite participant life origin. Parser validation runs after return lowering so the actual message recipient controls pending ownership. Incompatible input yields a real parse error, not a fake positive SVG.
2. **Queue component origin.** Java `USymbolQueue#asSmall` merges stereotype and label text blocks and translates the block by its Margin(5,15,5,5). Rust now passes the actual text top and uses the complete first stripe's own ascent. A rich-size first stripe is no longer anchored using base-font ascent and then moved by subtracting its actual ascent. Label, stereotype, creation and tail consume the same component contract. Fresh normal/created queues and first-large/first-small multiline labels pass in both engines.
3. **Message stripe ascent.** `SheetBlock1#initMap` aligns each Sea, places its minimum Y at cumulative preceding height, then advances by that Sea's height. `emit_message_label` keeps its first-baseline contract and adds the current-minus-prior ascent when advancing to each following stripe. Ordinary/self and rising/falling font sizes pass in both engines. The separately handed-off shared-maker cases remain whole failures because this branch lacks its shared markup/blank-stripe changes; no combined correctness is claimed.
4. **Self TOP polygon order.** `ComponentRoseSelfArrow#getPolygon` emits TOP vertices as outer-upper, inner-tip, outer-baseline; BOTTOM emits outer-baseline, inner-tip, outer-lower. Rust now preserves this order in its self consumer for both directions. Independent ordinary endpoint dressing ownership is unchanged. Fresh full/half, forward/reverse, circle and nested-live controls pass without comparator changes.
5. **Collections provenance clarification.** The adjacent comment now names `Rose#createComponentParticipant` selecting `ComponentRoseParticipant(collections=true)`, including its marginY7 and collection delta4. No numeric behavior changes, and the original account/history remains untouched.

- `sequence-single-pending-create`: account `4aaab169442b1e8a1d9e896d0cd3f019de6803d4`; implementation `d5478061ffbd2b1ec31a8ac513ff23b4489e3fa6`.
- `sequence-queue-stripe-origin`: account `4aaab169442b1e8a1d9e896d0cd3f019de6803d4`; implementation `653b1ed7a49784e6e6205f3aebf69a16278f8d78`.
- `sequence-message-stripe-baselines`: account `4aaab169442b1e8a1d9e896d0cd3f019de6803d4`; implementation `653b1ed7a49784e6e6205f3aebf69a16278f8d78`.
- `sequence-self-half-polygon-order`: account `4aaab169442b1e8a1d9e896d0cd3f019de6803d4`; implementation `653b1ed7a49784e6e6205f3aebf69a16278f8d78`.
- `sequence-collections-source-clarification`: account `4aaab169442b1e8a1d9e896d0cd3f019de6803d4`; implementation `653b1ed7a49784e6e6205f3aebf69a16278f8d78`.

## Stable validation

Final main **library** tiers: product **11,181/11,251**, assisted **10,497/11,251**, sequence **1,411/1,411**. Assisted total12,550 minus1,299 skipped gives11,251 eligible. These are not11,251 CLI processes. Both main failure-name sets match the prior maker's exact final snapshot; new snapshots were copied immediately after the command into revision-bound scratch. The independent checker's overwritten intermediate name files are not used as before/final evidence.

Parser575, renderer795 (including12 ASCII tests), and CLI13 pass. Focused sequence tests and owned-file formatting pass. Required broad gates ran once on the final code. Existing red gates remain: product unlocked ratchets,754 assisted failures, guards4/6, standing794/1416 with622 failures, workspace formatting in class files, parser component dead-code Clippy and19 renderer Clippy findings. The standing membership includes the64 imported checker cases; its cwd is the Cargo oracle package, unlike repository-cwd CLI replay. No baseline, skip, comparator, error census or allow-list was altered.

## Captures and reproduction

`all-inputs.json` provides relocatable source/reference membership. `before-measurements.json.gz`, `measurements.json.gz`, `before-after.json.gz` and `remaining-failures.json.gz` preserve all comparison outputs and residuals. `captures.tar.gz` retains every private intermediate/final SVG, model, raw relay stream, script, exact source snapshot and log. The initial compile failure (error-line type u32 instead of usize) is retained; it was corrected before focused tests and final commits. No completed references were regenerated. Binaries remain private, with clean revision/source/SHA bindings in `binaries.json`. The first dirty build also has an exact source snapshot; it is not represented as a clean committed revision.

```text
python3 docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py PRIVATE_CACHE/all2169.json PRIVATE_CACHE/final-bin PRIVATE_CACHE/final2169
```

Copy the manifest/scripts to private nonsynced scratch before reuse; adapt only paths and create a new output directory. `checks.json` and `final_checks.py` preserve exact argv/cwd/environment, owned persistent target and Cargo jobs4. Frozen `compare_cli` bytes are identical before/after. `artifact-sha256.json` inventories packaged files. Every failed intermediate/control remains available; no failed source was edited to manufacture a pass.

All20 new references came through the parent's existing Monkey Man relay, worker.zkWjIjt9/new, GMT+11:00, pinned headless JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Gzip SHA `fe3e044a7f09e30bb04b2cd19c4879e924cc0d9d82e205f2a5ebe59fad465f54`; transport SHA `4f3025eb604cb5d9fe079b9575911bb76cc85011c46f89dbc87085dcd799e8d4`. Local validation checks exact source/stream hashes, immutable manifest, cwd substitution and recorded argv/environment. Python-repr argv fields were parsed only for comparison; raw bytes remain unchanged. Java argv is derived from the pinned wrapper, not process-traced. Remote OpenJDK21.0.5/Graphviz12.2 differ from origin; scoped conformance is not global equivalence. No local Java/Javac, SSH, child or new remote runner was used.

## Remaining failures and handoffs

Every586 valid whole-output failure remains active. Latest checker residuals are reverse-created queue legacy canvas, both unclosed-group canvases, mixed actor/queue Teoz boxes, both multiline box-title cases and nested Teoz box. The two shared handoff sources also still fail whole-output comparison. `dependencies.json` enumerates these and links inherited obligations rather than waiving them.

In the separate shared-maker tree, `present_bold_empty` reportedly had only the baseline93.6777 versus Java106.2461 difference. Baseline711 here also has canvas/text differences (Java173x169 versus Rust145x153); `style_parameter_contrast` likewise differs (Java183x162 versus Rust169x130). Final stripe-origin correction does not import or certify shared markup/blank-line fixes. Both whole failures, raw references, exact sources and CLI/AST/ASCII outcomes are preserved.

Nested/multiline englober, Dolls ancestor sums, other box/envelopes/factory failures, logical-font T14.11, tabs, URL second Display, themed Real/padder, shared sprite guillemets/omitted note line, page chrome, per-kind shadow/hexnote/multiline delay and the TIM welcome dependency remain unresolved. No broad canvas rewrite or fitted offset was attempted. No review.json, acceptance attestation, target mutation, push, PR or release was created. Stop for a new independent checker.
