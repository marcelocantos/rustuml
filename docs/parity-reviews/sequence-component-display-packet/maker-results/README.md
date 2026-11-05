# Sequence component/display packet — maker evidence

**New independent review required. No acceptance claim.** Final tested production is `a4c3c35794ae1d7117d709134ed22c7191db9db8`; baseline is `672cd8f26e1d7c6725d8971e01ad7575defd8567`. Independent rejection `4dd330b62f6497365de55856853b2bdafa145fbb` was imported as `16b28fc6de23d3920150efe50cfad7d065985c1a` before accounts and production. All 8,221 prior evidence paths and 103 imported paths remain byte-identical. Earlier accepted/bounded records remain attached to their exact tested revisions.

## Measured output

The unchanged comparator measured exact source through the real release CLI. The 2,083 entries comprise 2,035 distinct source byte strings and 48 retained duplicate entries. New 42 checker and 20 maker sources are disjoint from each other and the preserved 2,021. The parent 1,449 overlaps:1,123 paths overlap and 326 parent-only paths were not replayed; unique-byte overlap is 1,097, with 278 parent-only distinct bytes. These denominators are not additive.

| Cohort | Entries | Before match / valid failure / invalid | After match / valid failure / invalid |
|---|---:|---:|---:|
| preserved2021 | 2021 | 1401 / 569 / 51 | 1401 / 569 / 51 |
| checker42 | 42 | 17 / 19 / 6 | 30 / 6 / 6 |
| maker20 | 20 | 5 / 13 / 2 | 17 / 1 / 2 |
| **Total** | **2083** | **1423 / 601 / 59** | **1448 / 576 / 59** |

There are **25 recovered valid cases, no lost passes in any comparison mode, no new valid-input errors, and no CLI or library-render panics**. The preserved 2,021 cohort is unchanged. Latest checker valid outputs improve 17→30/36; new maker valid outputs improve 5→17/18. These gains are execution evidence, not checker acceptance. Every 576 valid whole-output failure remains in `remaining-failures.json.gz` with full comparator output. The new maker failure `queue_stereotype_legacy` is a canvas-width mismatch; it is not waived. The checker retains six valid failures detailed below and in dependencies.

The 59 invalid references stay separate: Rust rejects 43 and accepts 16, versus 39/20 before. Four additional rejections are the two checker and two maker unclosed notes. Original invalid arrows, mixed/unknown factory negatives, and zero-byte Java outputs remain immutable; none receives SVG credit. The unchanged `diff_one` comparison helper still panics on two empty invalid references in each diagnostic mode. Actual CLI and library rendering do not panic. `negative-status.json` preserves every exact status.

Final main **library** tiers are product **11,181/11,251**, assisted **10,497/11,251**, sequence **1,411/1,411**. These are not 11,251 CLI subprocesses. Both failure-name sets match baseline exactly. All 574 parser tests, 793 renderer tests (including 12 ASCII tests), and 13 CLI tests pass. Serde-default compatibility tests remain green; this packet adds no serialized field. Owned-file fmt passes. Main no-oracle still reports unchanged unlocked ratchets; assisted has 754 failures. Guards 4/6 and standing perturbations 744/1352 (608 failures) remain red. Workspace fmt and Clippy remain red for preserved class formatting, parser component dead code and 19 renderer findings. No red gate was altered or waived.

## Models and chronology

Each account below was committed before its mechanism draft. `implementation-map.json` gives exact paths and full hashes. Only sequence diagram/model, parse/sequence and render/sequence changed; parse/mod, ASCII, shared text/style/fonts, class, TIM and targets were untouched.

1. **Dressing-owned parts.** `ArrowConfiguration#withPart` chooses dressing2 when present, otherwise dressing1. Ordinary endpoint composition keeps source/target heads/circles separate and only applies the chosen part to its owning dressing, including normalized direction. `ComponentRoseSelfArrow` is a distinct consumer: its start/end painting explicitly uses aggregate `getPart`. The first draft incorrectly forced self source FULL; two fresh self regressions are preserved in `first62`, and the separately preaccounted correction restores the Java consumer. No new model field or ASCII constructor edit was needed.
2. **Created images and event identity.** `SequenceDiagram#addMessage`, `Step1Message#createArrowCreate` and `CommunicationTile` attach every CREATE to its compatible message. Each event owns its drawn head; `LifeLine#setCreate`/`LivingSpace#goCreate` separately overwrite participant creation origin. The renderer now retains every repeated creation and translates box, actor center and text as one image. Fresh repeated/live/reverse actor controls pass in both engines.
3. **Complete participant label component.** `CommandParticipant` evaluates Display before storage; identity remains separate. `ParticipantTextBlock` measures each existing shared-text stripe and uses the same component layout for painting. Preferred sizes propagate through plain heads, queues, figures, creation and tails. Queue geometry uses `USymbolQueue` text plus Margin(5,15,5,5), rather than one/two-line expressions or mixed-kind offsets. `Participant#getUsedStyles` supplies kind-specific font size through the existing style API; participantFontSize no longer leaks into queue/actor siblings. Symbol heights are source-derived, and underline allocation stays intact. This is bounded component coverage, not complete shared Display/style parity.
4. **Whitespace rule title and closure.** `BodyEnhanced2` title stripes use `StripeSimple#getAtoms`'s space atom for whitespace-only titles. Body/rule composition remains unchanged. `PSystemCommandFactory` requires the complete multiline note command; an unclosed note now yields a real parse failure while Java-valid unclosed group/box behavior remains active.
5. **Head/englober composition.** ComponentRoseEnglober reserves its own text+two marginY1+trailing3. The first broad cf3eafb2 run exposed `sequence/seq_box_with_queue`:11180product/10496assisted/1410sequence. A new account precedes the member-composition repair: legacy `DrawableSet#getHeadHeight` chooses max(head(member)+its englober), while Teoz `LivingSpaces` and `Dolls` compose separate maxima. All 68 valid main queue controls then pass; two invalid queue references remain separate. Changed production justified the final full rerun. No queue-kind gap exception or final diagram offset was added.

- `sequence-dressing-owned-parts`: account `bf400d8455757a31123b51fd67b64e497514e223`; implementation `83a496134c763817ce5898758f917fe5c754dadc`.
- `sequence-created-image-event-ownership`: account `bf400d8455757a31123b51fd67b64e497514e223`; implementation `cf3eafb293de2f01699399321d2220c4de9eb28c`.
- `sequence-complete-participant-display`: account `bf400d8455757a31123b51fd67b64e497514e223`; implementation `cf3eafb293de2f01699399321d2220c4de9eb28c`.
- `sequence-empty-rule-title-atom`: account `bf400d8455757a31123b51fd67b64e497514e223`; implementation `b653a5f8adb43f4a6cb2f0ab3a4aad50c05a2bcf`.
- `sequence-unclosed-note-command`: account `bf400d8455757a31123b51fd67b64e497514e223`; implementation `71422f1b957c15ca5d45850c5f260db6b4b15862`.
- `sequence-self-part-consumer`: account `c31c4216327952ef019a64d8d9de86a20418f3c8`; implementation `cf3eafb293de2f01699399321d2220c4de9eb28c`.
- `sequence-participant-kind-font-consumer`: account `9e6a899c153c4816f71f2c93b717f8e1e6a2b05b`; implementation `cf3eafb293de2f01699399321d2220c4de9eb28c`.
- `sequence-head-englober-component-height`: account `9e6a899c153c4816f71f2c93b717f8e1e6a2b05b`; implementation `cf3eafb293de2f01699399321d2220c4de9eb28c`.
- `sequence-englober-member-composition`: account `c0ae326fe0ec171784f51a85b6da7afb88d7073a`; implementation `a4c3c35794ae1d7117d709134ed22c7191db9db8`.

## Captures and reproduction

`captures.tar.gz` preserves every intermediate output, source snapshot, comparison, log and private script, including first self failures, Display/font/title failures, initial creation borrow-check failure, and the first broad queue regression. The initial Display unit failed because it expected a literal XML space; the shared serializer emits `&#160;`. The corrected unit checks that encoding and passes. No reference/input was regenerated. The intermediate `display-bin` capture lacks a full simultaneous source snapshot and is explicitly not represented as an exact committed-source run. Final binaries are bound to clean committed production; `binaries.json` verifies their source/binary hashes. Executables remain in the private cache.

`all-inputs.json` is a relocatable 2,083-entry manifest. `model-and-ascii.json.gz` preserves 62 actual before/after CLI models and `-ttxt` outputs; 14 ASCII outputs change from Display normalization/negative rejection. An untouched ASCII renderer does not imply unchanged behavior or both-endpoint correctness. Parser/model ownership and every raw status are inspectable.

```text
uv run --no-project python docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py PRIVATE_CACHE/all2083.json PRIVATE_CACHE/final-bin PRIVATE_CACHE/final2083
```

`checks.json` records exact final commands/cwd/status. Private `final_checks.py` records environment/jobs and clean binary binding. All compiled commands use the owned target and at most 4 Cargo jobs. Real CLI captures run at repository cwd; the standing library test runs at Cargo-package cwd, preserving known T14.6.2 behavior. Helpers are evidence: copy/adapt them into nonsynced private scratch before reuse. They do not replace or modify the frozen harness.

All 20 new references came from the existing parent Monkey Man relay, worker.P1SfmUju, pinned JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, explicit GMT+11:00 and headless wrapper. Gzip SHA `fea5c30e9ab500514ac22273e1ca2b4e70f3b2548c03c805943c7c068e664b8e`; transport SHA `fef84f97afdb1eca075b69124b9eb66b38d9e22b51fde08ff5101cb2a8e77180`. Local validation checks exact source/stream hashes, recorded argv/cwd/environment, XML/type/error/status and immutable manifest. Eighteen are valid Sequence exit0, two are syntax errors exit200, zero timeouts; intended/observed validity agrees. Recorded Java argv derives from the pinned wrapper construction. Remote OpenJDK21.0.5 and Graphviz12.2.0 differ from origin; seven-case conformance was scoped, not global equivalence. No local Java/Javac, SSH, new runner or child was used.

## Residual dependencies

All whole-output nonzeros are preserved. `dependencies.json` names factory quoted-header/UID ordering, Component aggregation/bare-circle Class, accepted invalid factory commands, unclosed-group canvas, centered-note canvas and boxed multiline queue canvas; shared per-atom tabs, URL second Display, natural alignment and logical fonts T14.11; themed self-note Real/padder/envelope, page chrome, per-kind shadow/style, hexnote, multiline delay and nested englober/boxPadding gaps. The unrelated frozen 542/deferred 15 remain failures, not scope waivers.

The new sprite handoff is recorded without importing shared production or rerunning gates for it. Java `CreoleParser#createSheet` converts ordinary message Display guillemets before shared stripe parsing; this branch only converts participant labels, so `<<$枝>>` in the supplied message remains unresolved. The exact handed-off source/raw Java pair and provenance are in `dependency-handoff/`, excluded from the 2,083 denominator. Shared sprite-only note omission and blank-stripe/absent-Display integration remain pending; nested_unicode fonts/unsupported plain are not repaired. The separate TIM8 welcome factory dependency stays explicit and unchanged.

No review.json was authored, no target mutated, and nothing shipped. Stop for a new independent checker.
