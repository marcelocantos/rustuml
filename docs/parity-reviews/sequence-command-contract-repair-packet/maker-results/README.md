# Sequence command-contract repair — maker evidence

Final production: `b4849a5008ff28c360fefe0176ee6ad073c02fd7`. Baseline evidence: `9784e412146e3ece634d88ea0a1bddd6b42a82b6` (production `d491d4bda47faf90d4751484d45aa1387fb1b290`). Independent rejection `e654acb9ba31aec53329c739e8151d5bf46659f7` is preserved unchanged as `1cb5fa66d5855c412a44167ae74ae1a69ceae889`. No acceptance or self-review is issued. A new independent checker is required.

The mandatory note identity is now part of the matching grammar. Single-identity, pair and attached-note consumers are separate; an optional prefix cannot consume the sole participant. All 16 alias follow-up outputs now match Java, restoring 10 valid inputs that had exited1. The whitespace prefilter checks the whole command, including a create prefix retained across declaration parsing. Optional/nested Java regex whitespace and direct Pattern2 consumers remain separate. Rust literal-space entry gates for box/across/resume now follow their Java whitespace composition. Recognized `accross` produces an execution error.

New append-only provenance corrects the nonexistent CommandParticipant#getRegexConcat and DrawableSetInitializer#createParticipant citations: the actual methods are declaration subclasses such as CommandParticipantA#getRegexConcat and DrawableSetInitializer#prepareParticipant. Historical accounts remain untouched.

## Commits

| Mechanism | Preimplementation account | Implementation |
|---|---|---|
| sequence-required-note-identity | `4d84995c7b52ad5e3a08835ef9e59a6c7480868e` | `8cb61e5dce9006a570549de0fd11f2bc9f1dfcc3` |
| sequence-command-fox-space | `4d84995c7b52ad5e3a08835ef9e59a6c7480868e` | `fc3019e363c1e2b8b2f4822261255dc954037d7e` |
| sequence-command-entry-grammar | `4d84995c7b52ad5e3a08835ef9e59a6c7480868e` | `f48bafbc94af6d0fd3adc777708666407952a585` |
| sequence-command-provenance-correction | `4d84995c7b52ad5e3a08835ef9e59a6c7480868e` | `5dccddca0c94fa2d436afd4a8c6ff0f09f028170` |
| sequence-pattern2-class-composition | `2a0441642b1d84315cb6bfef1d21c9fb4e5d3e08` | `70bc20cf1efe456fa5dd83e3a726c2796717ce9b` |
| sequence-create-command-signature-scope | `d4fc86b240d2ea0cde6780c0910de2fff01b3c23` | `b4849a5008ff28c360fefe0176ee6ad073c02fd7` |

Only `crates/rustuml-parser/src/parse/sequence.rs` has production behavior changes. `crates/rustuml-render/src/sequence.rs` changes one source-citation comment. The model, ASCII implementation/fixtures, factory dispatch, shared serialization/text/style/font modules, targets, harnesses and baselines are unchanged.

## Actual CLI replay

Rows are match / SVG mismatch / valid CLI error / nonpositive reference. Duplicates and nonpositives are retained.

| Cohort | Entries | Before | After |
|---|---:|---|---|
| frozen 2549 | 2549 | 1809 / 639 / 2 / 99 | 1809 / 639 / 2 / 99 |
| checker534 | 534 | 348 / 116 / 10 / 60 | 370 / 104 / 0 / 60 |
| maker24 | 24 | 4 / 12 / 2 / 6 | 12 / 6 / 0 / 6 |
| signature4 | 4 | 4 / 0 / 0 / 0 | 4 / 0 / 0 / 0 |
| Total | 3111 | 2165 / 767 / 14 / 165 | 2195 / 749 / 2 / 165 |

Thirty whole-output recoveries, zero lost matches in all three comparator modes, zero lost rejections, zero new valid SVG CLI errors or panics. Two pre-existing short-arrow CLI errors remain. The library diff_one helper panics on those ParseError unwraps; these are not CLI render panics. The cohorts contain 3059 unique source byte strings and 52 duplicate entries. The 534 checker bytes and 28 maker bytes are disjoint from the frozen 2549; parent 1449 is not an additive denominator.

All 2549 frozen AST, ASCII and YAML stdout streams remain byte-identical. Across 3111 inputs,58 AST, 36 ASCII and 52 YAML stdout streams change; complete streams/statuses are retained. Four new exit1 outcomes in diagnostic modes are corrected Java-invalid accross rejections. Thirty-five existing nested-enum YAML serialization panics remain, with zero newly introduced panics. No ASCII parity or behavior-invariance claim is made. Element/text/head-group inventories detect no increased excess counts on valid references.

Negative outcomes are explicit: 99 historical nonpositive entries retain 77 rejections; the 52 checker syntax-invalid plus 6 maker syntax-invalid cases improve 36 to 40 rejections, leaving 18 false successes. Eight Java exporter failures remain separate, exit 0/error SVG with invalid-character exceptions; they are not syntax negatives or SVG passes. Thus 165 nonpositive entries yield 117 Rust rejections and 48 successes; 40 successes remain rejection-contract failures (22 historical plus18 fresh), and 8 are exporter outcomes. Both T14.17 message_gap cases remain failed. All intended/observed disagreements remain preserved.

## Intermediate failures and final gates

The first broad run at5dccddca introduced 29 colored-note losses: the old helper expanded complete `[%s]`/`[^%s]` spellings but left new `[^%s:]` literal, admitting spaces into color tokens. Account2a0441642 preceded production70bc20cf1, which expands the token compositionally. First-run logs and immediate failure-name snapshots are retained. A subsequent audit found create-tail signature context loss; four supplementary Java cases expose 2 intermediate lost matches, repaired by accountd4fc86b24/productionb4849a500. All 4 supplement cases now match and preserve actor/creation ownership. These are measured intermediate regressions, not erased history.

Final exact production was broadly gated once. Three broad runs total are justified by the two actual corrective production changes; there were no unchanged-code repeat broad runs. Final parser 586, renderer 800 (including 12 ASCII), CLI 13 pass. Main source-only library 11181/11251, assisted library 10497/11251, sequence 1411/1411. Both final main failure-name files are byte-identical to baseline. These main counts are library diagnostics, not 11251 CLI processes.

Standing perturbations: 1401/2258, 857 failures, package cwd `crates/rustuml-oracle`; guard 4/6, with decimal/review-record gates red. Owned formatting passes; workspace class formatting and parser component dead-code clippy remain red. Renderer clippy retains 19 existing findings. No baselines, skips, allowlists or targets were edited.

## Captures and residuals

The 28 new reference captures use the parent existing Monkey Man relay only, pinned JAR SHA256 aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053, GMT+11:00, headless AWT, en_AU and UTF-8. Twenty-two are valid sequence SVGs and 6 syntax-invalid; all statuses, raw streams, inputs, hashes and parent transport validations are retained. Java argv is derived, not process-traced. Remote OpenJDK21.0.5/Graphviz12.2 differ from origin; no global equivalence is claimed. No local Java/javac, SSH, new runners or children were used.

Whole-output failures remain active: all 8 nested Teoz Display cases need participant englober genealogy; created control legacy retains an extra polygon whose complete cause is not established. These are separate model paths, not a bounded single Display fix. Four maker across-shape cases still fail envelopes, and 2 quoted-life-event controls expose an existing word-only WHO parser despite the correct global space bit. Existing 22 self/nonself note geometry failures, note options/URL/stereotype handling, short arrows, Unicode/logical fonts, T14.13 headings, T14.17 consumption, TIM welcome, group/box envelopes and frozen failures remain unresolved. See `remaining-dependencies.json` and the exhaustive `remaining-failures.json.gz`; none is waived.

`captures.tar.gz` preserves raw CLI SVGs, all model/ASCII/YAML streams, logs, private request envelopes and every intermediate result. `all-inputs.json` is an executable relocated manifest; source/reference bytes are immutable. `binaries.json`, `implementation-map.json`, `protected-evidence.json`, `source-trace.json`, `rejection-contract.json` and `cohort-overlap.json` bind revisions, hashes, scope and denominators. All 10159 prior evidence paths plus 1641 imported paths are unchanged. No shared Creole/serializer integration correctness is claimed.
