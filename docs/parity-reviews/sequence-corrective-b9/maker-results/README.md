# Bounded sequence correction maker packet

**This is maker evidence, not acceptance. Fresh independent review is required.** The implementation starts at `5358314a0180cf760584c79bb5efd4ec1ae6320b`; final tested production is `1b29d8c78d671649b0db6efcdc09e01391f6ce98`. Independent rejection/evidence `68c8b95ef1087b5daa83d16b7e8e83670d5ca10f` was imported as `37642c05b4cbd8cf84fca9f32091f3848676d58c`; all 468 paths changed by that evidence commit are byte-for-byte unchanged. Its bounded self-cross and legacy-destroy acceptances remain attached only to the full tested `5358314a`, not this candidate or extracted commits.

Only these existing files changed in this correction:

- `crates/rustuml-parser/src/parse/sequence.rs`
- `crates/rustuml-render/src/sequence.rs`

No shared color/style/font, class, TIM, ASCII, harness, comparator, baseline, fixture, target, old account or checker-record edits were made. No child, new checkout, local JVM/Javac, SSH, push, PR or release was used.

## Commits and causal mechanisms

| Mechanism | Preimplementation account commit | Implementation commit |
| --- | --- | --- |
| Inline note Display before measure/paint | `8e1440fbfd454d49959c6a25a827072c02694a72` | `c534957672b4a8e2c1721f4646a507110b1a4a84` |
| Sequence arrow color domain and primitive alpha/no-paint | `8e1440fbfd454d49959c6a25a827072c02694a72` | `ecd996a303ec94c7f180548328afe88967982c59` |
| Legacy raw life variations, separate from return ownership | `8e1440fbfd454d49959c6a25a827072c02694a72` | `f77e328cd9b278bd23eee7d2ad166d7468732dc6` |
| Teoz parallel contacts/predecessors and per-level paint order | `8e1440fbfd454d49959c6a25a827072c02694a72` | `7e074830c99a898316a5b0448b0c0810d3df8fde` |
| Effective note alignment/shadow and activationBox border | `8e1440fbfd454d49959c6a25a827072c02694a72` | `8d668d3fbc67101bb715662442d75aec3773dae9` |
| Text-block full precision, complete height, shared preferred width | `2dab3648e4574d3246dfbe683566c84a38f52ec5` | `8d668d3fbc67101bb715662442d75aec3773dae9` |
| Scoped shadow numeric extraction/default semantics | `2d17c23b96bee08ac0e61947b4ffe0fc91b969b9` | `1b29d8c78d671649b0db6efcdc09e01391f6ce98` |

Color and ordered-paint units are included in `8d668d3f`. See [implementation-map.json](implementation-map.json) for exact file hashes and [source-audit.md](source-audit.md) for named Java methods/data flow, derivations and source-locator refinements. Intermediate commits have not been independently accepted.

## Measured results

Real CLI `rustuml -tsvg -pipe` outputs were compared with the unchanged `compare_cli` built from the existing checker source. Each input also ran the unchanged product and assisted library comparators. No failures or duplicates were removed.

| Cohort | Entries | Before CLI matches | After CLI matches | Remaining valid mismatches | Invalid references |
| --- | ---: | ---: | ---: | ---: | ---: |
| Retained packet | 1,828 | 1,232 | 1,232 | 557 | 39 |
| New checker | 49 | 23 | 45 | 0 | 4 |
| First new maker controls | 20 | 4 | 18 | 2 | 0 |
| Shadow-value controls | 4 | 0 | 4 | 0 | 0 |
| Total | 1,901 | 1,259 | 1,299 | 559 | 43 |

There are **40 gains, zero lost passes in all three comparison modes, zero valid-input CLI errors, and zero CLI panics**. All 45 valid new checker controls match; all four checker negatives still reject. All eight bounded self-cross and four legacy-destroy checker controls still match. All 542 prior valid mismatches and 15 deferred checker mismatches remain active.

The 43 invalid references are separate: 31 CLI rejections and 12 continuing invalid acceptances. Neither is reported as an SVG match. One pre-existing `diff_one` panic on the empty invalid TIM overflow reference remains unchanged; see [library-invalid-reference-status.json](library-invalid-reference-status.json). It is not a valid-input renderer panic, and the CLI acceptance remains a failure.

The final 1,897-entry replay is byte-identical to the earlier `8d668d3f` replay. The four new shadow cases fail both the original baseline and `8d668d3f`, and pass `1b29d8c7`. All intermediate outputs are retained. The first focused 69 replay had 53 matches/12 valid mismatches/4 invalid. A first main run lost three lifeline-border controls because activationBox incorrectly inherited its sibling lifeLine border; source-derived fallback correction restored all three. Before/intermediate/final real CLI captures of those controls are included.

The main tiers are **library measurements**, not 11,251 CLI processes:

- Product: **11,181/11,251**, unchanged exact failure names; sequence **1,411/1,411**.
- Assisted: **10,497/11,251** eligible (754 failures; 1,299 excluded of 12,550 total), unchanged failure names.
- Parser units: **568/568**. Renderer units, including ASCII: **787/787**. CLI pipe tests: **13/13**.
- Guards: **4/6**; decimal and review-completeness guards remain red. No frozen rules were changed.
- Owned-file format passes. Workspace format remains red on existing class formatting. Combined Clippy remains blocked by parser/component dead code; renderer-only retains 20 existing findings (previously 22), with no new finding.
- Built-in perturbations: **659/1,240** at Cargo package cwd; isolated repo-root CLI subset **660/1,240** (557 valid mismatches, 23 invalid). The additional package-cwd failure is the existing theme include control, T14.6.2.

[summary.json](summary.json), [before-after.json.gz](before-after.json.gz), [remaining-failures.json.gz](remaining-failures.json.gz), [negative-input-status.json](negative-input-status.json) and [checks.json](checks.json) retain exact statuses. The 1,901 paths contain 1,853 unique source byte strings and 48 duplicate entries, all retained. The parent 1,449 cohort overlaps old1728 in 1,123 entries, with 326 parent-only entries not replayed. Do not sum overlapping cohorts; the parent owns combined integration. See [cohort-overlap.json](cohort-overlap.json).

## Reference and execution provenance

All 24 new references were captured by the parent's existing Monkey Man relay, not by a local JVM or SSH launch. Every exact source, SVG stdout, stderr, exit, argv, cwd, timezone and per-stream hash is retained in `inputs/`, the raw result gzip files and environment records. All 24 raw exits were 0, with zero timeouts, and the unchanged comparator classified all 24 references as valid.

- First20 manifest SHA256: `758da5f6cf52580a2ae83d3ef4f8cb33f01a6e4fba865f4585ff8228cb497912`; result gzip `084b60166689371184df41d99f39e7340efc8cb42c3363f3c12ed92e0f80a5ef`; input transport `f3a9ffa25d1e9e30b872ed9738e586fb51d4251b44f10ff346a9e77acf9fd2a1`.
- Shadow4 manifest SHA256: `f074dc38629881c9d540ccbd0432e14b42f1188c2608c1685d9164169bffc9ce`; result gzip `92f4fa57a19633895c66ce8135f3530625489db2714371c23ad5482f3fbca15d`; input transport `8a08dd1baa4c0f8dc3afc0293acff72b2beaf07eee9c86c836b1cc075c03b04d`.
- Workers: `worker.xfHmIo4F` and `worker.glQAQqNl`, each a private directory under `/Users/marcelo/rustuml-reference/scratch/`; timezone `GMT+11:00`; forced headless AWT; JAR SHA256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
- Remote OpenJDK **21.0.5** and Graphviz **12.2.0** differ from origin. The parent's seven-case transport calibration is scoped, not global font/renderer equivalence.

Local runs used repository cwd `/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af`, isolated captures in `/Users/marcelo/Library/Caches/rustuml-sequence-corrective-b9`, existing owned Cargo target `/Users/marcelo/Library/Caches/rustuml-sequence-repair/target`, and `CARGO_BUILD_JOBS=4`. Baseline binaries were reused from the authenticated prior packet, copied into private `before-bin`; no fictitious fresh before build is claimed. See [binaries.json](binaries.json).

Replay entry point: `python3 docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py MANIFEST BIN_DIR PRIVATE_OUTPUT_DIR`. `all1897-inputs.json` and `shadow-manifest.json` identify the executed batches; `all-inputs.json` combines them without deduplication. `final_checks.py` records the exact build, units, CLI, two main tiers, built-in perturbations, guards, fmt and Clippy commands. `commands.json` and compressed `logs/` retain cwd/status. `cli-captures.tar.gz` preserves all before/intermediate/final CLI output and per-input diagnostics. Sources and references are verified again during packaging; old evidence and comparator hashes are recorded in [protected-evidence.json](protected-evidence.json).

## Remaining dependencies

[dependencies.json](dependencies.json) enumerates every deferred group and exact source paths. The two new valid failures are cerulean self-message attached-note preferred allocation: legacy note/canvas 10px short, and Teoz participant/note/canvas differences. Activation borders now agree. These belong to the existing theme-padding/component-allocation path; no geometry offsets were added to conceal them.

Page wrapper/legend/styled-caption composition, per-kind shadow geometry, hex-note shared bounds, right-note canvas extent, multiline delay, boxPadding/nested englober, both logical-font controls (unowned T14.11), shared entity stereotypes, and the retained invalid acceptances are unresolved. Broader Display natural-alignment/Jaws escapes and automatic arrow color have no positive coverage claim here. No whole-sequence or whole styled-component acceptance is requested; the candidate requires a fresh independent checker, with every remaining failure preserved.
