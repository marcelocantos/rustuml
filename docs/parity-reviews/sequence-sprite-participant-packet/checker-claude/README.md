# Independent sprite/participant packet review: rejected

Reviewer: Claude Opus 5.5 (`claude-opus-5-5`), Claude Code in T3 Code, isolated branch `review/sprite-participant-claude` at evidence tip `f4ff1cb4e0678fee8f099c14c92df07529b0afbb`. No prior involvement, no child agents, no production changes, no local Java, no shipping. Java references were the ones staged by the previous checker, whose provider quota ran out. Validity comes from the raw Java SVG and stderr, not from exit codes.

**The packet introduces new valid-input errors outside the approved exception set, including lost matches.** Across the 818 fresh Java references, 10 Java-valid programs that ran without error at baseline now exit 1. Four of them were whole matches at baseline.

| Source (legacy and teoz each) | Baseline → final | Java |
|---|---|---|
| `c75s_inactive_unclosed_svg_*` | match → error | valid |
| `c75s_macro_uninvoked_unclosed_svg_*` | match → error | valid |
| `c75s_inactive_unclosed_pixel_*` | mismatch → error | valid |
| `c75s_macro_uninvoked_unclosed_pixel_*` | mismatch → error | valid |
| `c75_include_pixel_into_include_*` | mismatch → error | valid |

Cause: `PreprocessContext::process_one_line` runs the sprite body branch and the declaration block before `!if`/`is_active`, `!procedure` collection and `!include` expansion. Java's TIM preprocessor runs before `CommandMultilines2#isValid`. The packet's deferred blanking pushes every pending raw line (`output.push(line.to_owned())`) and bypasses `is_active`. As a result, inactive or uninvoked sprite headers and unexpanded `!include` lines reach strict sequence admission.

## Revisions and scope

- Production: `ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7`. Baseline: `c525fd32ae97b642b440a161140e9457f29bcfc8` (evidence `c6295ccde`). Java: `71806a23`. JAR: `aa5262a2…`. Golden submodule: `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`.
- Only `parse/sequence.rs` and the authorized parts of `preprocess/mod.rs` changed. Those parts are the enum, the field, the initializer, the body block and the declaration block, including the pending-suffix change. The diagram model and the renderer `sequence.rs` are unchanged. No TIM, continuation, include or conditional code changed (`scope-audit.json`).
- Every account precedes its implementation in ancestry. The 1,267 imported rejection paths and all prior evidence are byte-identical.
- The 21 maker-cited Java files match the pinned hashes. Two citations name the wrong package: `regex.FoxSignature` should be `text.FoxSignature`, which was not clarified, and `FactorySequenceNoteCommand`, which was clarified. The cited participant early return omits `SequenceDiagram#putParticipantInLast` (`source-audit.json`).
- No fixture-shaped logic was found. The structural defect is context-free matching of the declaration grammar on every preprocessed line.

## Actual CLI results

Private release builds (jobs 4), the unchanged `compare_cli` and four modes per source (SVG, AST, ASCII, YAML).

| Cohort | Entries | Before match / mismatch / valid error / nonpositive | After |
|---|---:|---|---|
| Maker retained (4,122 distinct) | 4,174 | 2,625 / 1,091 / 22 / 436 | 2,645 / 1,076 / 17 / 436 |
| Goldens | 12,550 | 11,181 / 70 / 0 / 1,199, plus 100 reference-read failures | identical; SVG bytes 0 changed |
| Fresh staged (790 + 28) | 818 | 140 / 365 / 30 / 283 | 176 / 345 / 14 / 283 |

- **Retained:** these results reproduce the maker exactly. There are 20 gains, 0 lost matches, 0 lost rejections and 0 new valid errors. Nine more invalid inputs are rejected. `bd087_reg_27_plain` recovers. The 17 errors are the 15 approved entries plus two pre-existing short external CREATE entries.
- **Fresh:** 40 gains (net +36 matches), **4 lost matches and 10 new valid errors**, 15 newly rejected Java-invalid inputs and 0 lost rejections. The approved exception set (15 entries) would have to grow to 25 entries; that change is not approved.
- No SVG panics or timeouts. `--yaml` panics on 63 retained and 148 golden sources in both builds (`serde_yml` nested enums). This contradicts the maker's "zero CLI panics" claim but is not a regression.
- Model changes are categorised in `*-model-changes.json`. The goldens have 6 AST and 6 YAML changes, all order-only (HashMap iteration; parsed YAML values are equal). ASCII has none. The retained and fresh model changes follow admission or SVG changes, plus 13 retained and 37 fresh AST/YAML-only changes with unchanged SVG. All 13 retained ones are in sprite-bearing sources (the sprite map in the AST). The fresh ones are sprite-map entries and complete participant fields, such as `Cedar.ring` instead of `Cedar` and `order 7` instead of the index.

Unchanged `rejection_contract.py` on the fresh Java-invalid inputs: 283 entries, 248 rejected before and 263 after, 0 lost rejections and 20 false successes remaining. Ten of those are unclosed sprites in class, state, activity, usecase and component diagrams, so the claim that every family is covered fails.

## Per-mechanism verdicts (all rejected)

| Mechanism | Fresh valid: before → after (match/mismatch/error) | Invalid rejected | Key finding |
|---|---|---|---|
| sprite-block-consumption | 24/20/16 → 35/17/8 | 0 → 4 of 14 | 4 lost matches, 8 new errors (TIM ordering) |
| sprite-completion-admission | n/a | 2 → 8 of 18 | 10 non-sequence unclosed blocks still accepted |
| sprite-pending-span | 0/2/10 → 6/0/6 | n/a | 2 new errors (`!include` inside body), 4 persisting |
| participant-complete-consumption | 54/148/2 → 72/132/0 | 72 → 72 | 42 silent mismatches (multi-stereotype, URL tooltip) |
| participant-residue-preservation | 0/42/0 → 0/42/0 | n/a | re-declaration reorder omitted silently |
| command-specific-color-paths | 62/153/2 → 63/154/0 | 174 → 179 | `-` warning fixed; 24 silent colour mismatches |

All 818 pairs remain executable under `test-diagrams/perturbations/<mechanism>/`. The include resources are in `sequence-sprite-pending-span/`. Intended validity is unchanged; 63 intended-versus-observed disagreements are retained.

## Broad gates (once, on `f4ff1cb4e`, before checker files were added)

Parser 594/594, renderer 801/801, CLI 13/13. Product library 11,181/11,251, assisted 10,497/11,251, sequence 1,411/1,411. Both tiers exit 101 under the unchanged ratchets. The failure-name files are byte-identical to the maker's and the prior checker's. Perturbations: 3,259 entries, 1,434 failures. Guards 4/6: the decimal-literal ratchet and the review-record check fail as before. Owned-file formatting passes. Logs are in `logs/` and `gate-failure-names/`.

## Pending Java request (batch1, 43 cases)

`pending-request/` contains the immutable request. Local before/after CLI observations are in `local-observations.json`; none of these cases has a Java verdict yet.

- The case-widened single-line sprite grammar blanks text that starts with `Sprite`/`SPRITE` inside notes, legends, refs and class bodies. It also blanks class links such as `Sprite -- Player`; Java's ClassDiagramFactory registers link commands before CommonCommands.
- A note line `Sprite frame {` opens a pixel block and now exits 1.
- Inline SVG containing ` '` is cut by Rust-only inline-comment stripping and is then treated as a multiline header, so it now exits 1.

## Bounded next obligations

1. Run sprite ownership on active, TIM-expanded lines only: `!if`, `!procedure` and `!include` come first.
2. Limit declaration matching to the command context: lines inside multiline command bodies, and lines a family factory dispatches before CommonCommands, are not sprite commands.
3. Admit incomplete blocks in every family that registers sprite commands.
4. Execute or explicitly warn for multi-stereotype lists, URL tooltips, `putParticipantInLast` and englober changes on re-declaration, and every colour path that is not painted.

## Reproduction

`support/` holds the scripts and `bindings.json` the exact hashes. `runs/*.results.json.gz` holds per-row exits, output hashes, stderr and comparator results. `captures-fresh.tar.gz` holds every fresh output. `captures-retained.tar.gz` and `captures-golden.tar.gz` hold the rows where any mode changed. Copy the scripts into a private directory before running them; there is no Java dependency.
