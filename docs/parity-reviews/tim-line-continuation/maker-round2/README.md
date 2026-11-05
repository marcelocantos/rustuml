# TIM backslash line continuation (🎯T14.22): maker record, round 2

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/tim-continuation-round2-claude`. This round answers the v2 review (`../review.json`, `../checker-claude/`, `../../sprite-command-post-tim-ownership/review-5.json`) and carries the whole sprite/participant line. An independent checker must review it; nothing here is a completion claim.

- Branch start: `fdc8df1fd` (v2 review, on maker tip `0a52663ac`, code `594c2b3c5`). Merge with `parity-recovery-2026-11-03` `4623c5494`: `b0fb43c3e` (`bullseye.yaml` merged cleanly; code unchanged). This merge commit is the **merged base**, and its code is identical to `594c2b3c5`.
- Account, committed before any code change: `account.json`, commit `3573444dc`.
- Implementation: `f2d12d0ef` (`crates/rustuml-parser/src/preprocess/mod.rs`, `crates/rustuml-parser/src/parse/mod.rs`). No code changed after measuring, so there is no addendum.
- Java: `71806a23780b04a5ccde2f8ceb5121edad5eb711`, read only and never run locally.

## Obligation 1: column-0 end rule

**Java.** `Pipe#readSingleDiagram` (Pipe.java:180-224) feeds `-pipe` input to `SourceStringReader`, and it looks only at column 0:
- A line that `startsWith("@start")` opens the diagram.
- `expectedEnd` is `@end` plus the type when the whole start line matches `@start([A-Za-z]*)`, and `@end` otherwise (lines 194-199).
- The diagram is complete at the first later line that `startsWith(expectedEnd)` (line 203). Reading stops after that line.
- With no such line, Pipe appends `expectedEnd` (line 219). With no column-0 start, it wraps the input in `@startuml` … `@enduml` (line 217).

`BlockUmlBuilder#<init>` (lines 88-135) then tests `StartUtils#isStartDirective` and `#isEndDirective` on the lines `ReadFilterMergeLines` returns, so only on merged lines.

**Rust change.**
- `pipe_single_diagram` ports that start and end recognition.
- `merge_ending_backslash_lines` runs `ReadFilterMergeLines` (now `read_filter_merge_lines`) over Pipe's text: cut after Pipe's end, or with Pipe's end appended. The appended end stays in the list, as it does in Java's TIM input; round 1 popped it.
- `split_blocks` no longer treats a line that `ReadFilterMergeLines` merges into a continued line as a start or an end (`lines_merged_into_previous`).

**Result.** `tc1_seq_indented_end_after_backslash` changes from valid-error to **match**. Held-out variants (batch r2: tab, `\enduml`, class, mindmap, state with a skipped comment, CRLF, named start, content after the indented end) now render at the tip; the merged base errors on every one of them. Their Java references are pending.

## Obligation 2: included tail (`UmlSource#loadInternal`)

**Java.** `PSystemBuilder#createPSystem` (lines 210-212) builds `UmlSource` with `checkEndingBackslash` for `@startuml`. `UmlSource#loadInternal` (UmlSource.java:118-134) runs over the TIM result list:
- The list holds the start line, plain lines (blank lines included), included lines and the end line.
- Comment and directive lines produce no entry.
- A line that ends with a backslash joins the next entry, and the joined line takes that entry's location.
- `PSystemCommandFactory#createSystem` (lines 97-109) stops at an end directive or when the source runs out. So `tail @enduml` is executed as a message.

**Rust change.**
- **Residue 2 (placeholders).** Rust's directive placeholders (`push_directive_placeholder` and the post-procedure padding) are now an internal value, `DIRECTIVE_PLACEHOLDER`. The post-TIM join passes over it, and it becomes an empty line afterwards. Real blank lines are still lines.
- **Residue 3 (pending tail).** `PreprocessContext::block_end` records the top-level end line: its diagram source line and its TIM text. A tail still pending at the end of the parser stream joins with it.

**Result.** All three Java-backed include cases now **match**:

| Case | Rendered message | Location |
|---|---|---|
| `tc1_seq_include_tail_before_end` | `tail @enduml` | 3 |
| `tc1_seq_include_then_define` | `tail Tui -> Kea : two` | 3 |
| `tc1_seq_include_then_directive` | `tail Tui -> Kea : two 2` | 3 |

So both residues now match Java. Neither depends on 🎯T14.53.

The include cases still failing in BU are no-join controls or depend on included-line numbering (🎯T14.53): `tc1_ctl_*_include_plain`, `tc1_class_include_three` and `tc1_seq_include_directive_continued`. They mismatch at every build.

## Obligation 3: ditaa exemption probe

Ditaa output is a PNG, which `compare_cli` cannot compare (outcome compare-101 for every ditaa golden). So the probe is a Java-backed equivalence test (batch r2). If Java exempts `@startditaa`, these hold:

1. `r2_ditaa_continued_row` (`| Ash \` + `| Elm    |`) differs in bytes from `r2_ditaa_continued_row_joined`.
2. `r2_uml_ditaa_continued_row` (`@startuml` + `ditaa`, which is not exempt) is byte-identical to its joined control.
3. `r2_ditaa_backslash_before_end` renders.

Rust's `--ast` keeps the `| Ash \` row for `@startditaa`. Rust has no `ditaa` command inside `@startuml`, so it errors on the `@startuml` pair at every build (pre-existing). The non-exemption on that start is unit-tested in `escaped_backslash_and_ditaa_blocks_are_not_continued`.

The goldens `wire/ditaa_mixed` and `wire/ditaa_rounded` contain `/---------\` rows. They are Java PNGs whose heights match the unjoined art, but they come from picoweb rather than `-pipe`, so they are supporting evidence only.

## Measurements (unchanged `compare_cli`, `prepare.py`, `replay.py`, `classify.py`, `rejection_contract.py`; 4 modes; each case from its recorded cwd)

Builds are private `git archive` trees with one `CARGO_TARGET_DIR` each (`support/build.sh`; hashes in `results/binaries.sha256`): merged base `b0fb43c3e`, tip `f2d12d0ef`, `93bf53e18` and `9834a7194`. The fresh cohorts were re-prepared from the immutable relay references. `strict_validity.py` marks exit-100 no-block pages and crash pages invalid. Data: `results/summary.json`, `results/runs/`, `results/manifests/`.

| Cohort | n (Java-valid) | tip match | vs merged base | vs `93bf53e18` | vs `9834a7194` |
|---|---:|---:|---|---|---|
| BU | 40 (37) | 29 | **+4 gains**, 0 losses | +27, 2 newly rejected | +27, 2 newly rejected, 0 new valid errors |
| BD | 33 (29) | 23 | no change | +18, 2 newly rejected, 1 lost rejection (`mc1_seq_foreach_body`, T14.52) | +18, 1 new valid error (`mc1_usecase_continued`, T14.21) |
| BA | 64 (57) | 41 | no change | +37, 4 newly rejected, 1 lost match (T14.33) | +36, 2 new valid errors |
| AY | 48 (44) | 34 | no change | +21, 1 lost match (T14.33) | +20 |
| AZ | 8 (8) | 3 | no change | +3 | +3, 1 new valid error |
| AX | 77 (69) | 26 | no change | +18, 1 lost rejection (T14.32) | +14, 2 new valid errors |
| AV | 63 (60) | 20 | no change | +10, 3 newly rejected | +8, 2 new valid errors |
| AT | 45 (42) | 19 | no change | +7, 1 newly rejected | +4, 1 new valid error |
| AS/AQ/790/28 | 915 (602) | 247 | no change (0 bytes) | +59, 35 newly rejected | +53, 2 new valid errors |
| retained | 4,174 | 2,667 | 1 newly rejected (below) | +22, 3 newly rejected | +78, 13 new valid errors (approved set) |
| golden | 12,550 | 11,181 | 0 outcome changes, 0 SVG bytes | 1 newly rejected | 126 newly rejected |

- **Against the merged base:** 0 lost matches, 0 lost rejections, 0 new valid-input errors, 0 panics and 0 timeouts in every cohort. Panics are the unchanged `--yaml` sets in every build: 148 golden and 63 retained.
- **Model-only changes:** 7 golden DOT/`sprite_multiple` AST/YAML outputs differ in order only. This is HashMap order, as in round 1.
- **Retained newly rejected:** `sequence-target-circle-decoration/checker_092_empty.puml`, a zero-byte input.
  - Java: Pipe returns null in state `NO_CONTENT`, so no output and exit 50 (that review's `empty_output` class).
  - Merged base: rendered an empty diagram with exit 0, because round 1 appended `@enduml` to any input without an end.
  - Tip: no Pipe end is appended to an input with no content, so no block exists and the CLI exits 1. That is Java's outcome class (no diagram); neither output is comparable.
- **Against `9834a7194`:** the 11 fresh new valid-input errors and the 13 approved retained ones are the pre-existing T14.21/T14.31 set. They are unchanged and out of scope. BU's round-1 valid error is gone.
- **Against `93bf53e18`:** the 4 losses are the known residues: T14.33 ×2, T14.32 ×1 and T14.52 ×1.

**`rejection_contract.py`** (unchanged; tip against each before; `results/rejection/`):

| Set | Entries | vs merged base | vs `93bf53e18` | vs `9834a7194` | Introduced losses |
|---|---:|---|---|---|---|
| BU | 3 | 2 → 2 | 0 → 2 | 0 → 2 | none |
| BD | 4 | 2 → 2 | 1 → 2 | 0 → 2 | `mc1_seq_foreach_body` vs `93bf53e18` (T14.52) |
| BA | 7 | 4 → 4 | 0 → 4 | 0 → 4 | none |
| AY | 4 | 1 → 1 | 0 → 1 | 0 → 1 | none |
| AZ | 0 | none | none | none | none |
| AX | 8 | 1 → 1 | 2 → 1 | 0 → 1 | `ck3_svg_inline_quoted_apostrophe_seq` vs `93bf53e18` (T14.32) |
| AV | 3 | 3 → 3 | 0 → 3 | 0 → 3 | none |
| AT | 3 | 3 → 3 | 2 → 3 | 0 → 3 | none |
| AS/AQ/790/28 | 313 | 304 → 304 | 269 → 304 | 15 → 304 | none |

### Gates (`support/gates.sh`; logs in `results/gates/`)

- **Workspace lib:** 1,510 passed, 0 failed. Three new unit tests: `pipe_ends_the_diagram_only_at_a_column_zero_end`, `post_tim_join_passes_placeholders_and_reaches_the_end_line` and `split_blocks_tests_directives_on_merged_lines`. Two round-1 tests were adjusted for the kept appended end and Pipe's cut.
- **`pipe_mode`:** 13 passed.
- **`golden_no_oracle`:** 11,181/11,251. The 70 failure names are byte-identical to a `9834a7194` export run. As before, it fails only on the unlocked class, preprocessing and rendering baseline lines.
- **`golden_pairs`:** 10,497 passed, 754 failed. The failure names are byte-identical to `9834a7194`.
- **`parity_perturbations`:** 2,531 failures at the tip against 2,532 at the merged-base export. **0 new; 1 fixed:** `tim-line-continuation/tc1_seq_indented_end_after_backslash`.
- **`harness_guards`:** the four frozen-item guards pass (`comparator_constants_frozen`, `error_golden_census_is_frozen`, `no_golden_data_embedded_in_shipping_crates`, `parity_review_paths_reject_traversal`).
  - `decimal_literal_ratchet` fails on the same 5 `rustuml-render` files as the base. Render is untouched.
  - `parity_review_records_are_complete` has 2,712 normalised violations, identical to the merged-base export (`gv-base.txt`, `gv-tip.txt`).
- **Frozen items:** the comparator slack, syntax markers, park list, error census, guard allow-lists, strict-XML list and the 15-source approved set are unchanged. Nothing under `crates/rustuml-oracle`, `crates/rustuml/tests`, `test-diagrams` or `scripts` changed between `b0fb43c3e` and the tip.

## Pending Java request: batch r2 (23 cases, 22 intended valid, 5 include resources)

| File | sha256 |
|---|---|
| `~/Library/Caches/rustuml-tim-continuation-round2-claude-maker/r2/reference-manifest.json` | `80c79057f73b8471ae91b13b6463cc8e02b636129dd3ceed7eb5580b5b8ffa9a` |
| `~/Library/Caches/rustuml-tim-continuation-round2-claude-maker/r2/reference-input-bundle.json.gz` | `c77fd9358e39d8a46510cb1ac89ce65650bc28023bc91eef71a7985d0e26c5a3` |

- Copies are in `pending-request-r2/`, with `cases-with-java-predictions.json`, the Java predictions registered from the account's trace.
- Rust predictions (exit, `data-source-line`, texts, ditaa AST rows, for all four builds) are in `results/r2-rust-predictions.json`.
- The obligation-1 held-out acceptance, the ditaa probe and the extra include variants are **not met until these references return**.

Risks registered before Java:

| Probe | Prediction |
|---|---|
| `r2_seq_indented_start_and_end` | `data-source-line` probably differs by one, because Pipe prepends `@startuml` and Rust does not. Declared residue. |
| `r2_seq_include_tail_then_include` | The joined line takes the included line's location, which depends on T14.53. |
| `r2_component_include_tail_then_variable` | The tip errors. The joined line `[Alder] --> [Rowan] : wire [Rowan] --> [Ash] : n3` is routed to sequence (T14.21). The merged base rendered two links without the join, which is the wrong shape. If Java renders one component link, this is a **new valid-input error vs the merged base on a probe**, caused by T14.21 dispatch rather than by the join. The same line written directly in the main source errors at `93bf53e18`, the merged base and the tip, and renders at `9834a7194`, which has no strict sequence admission. |

## Declared residue

1. **Pipe pieces not ported:** the `@startuml` prepend for input without a column-0 start (it shifts Java locations by one), the discard of lines before that start, and `@@@format` lines. These lines are only skipped when recognising Pipe's end.
2. **A tail before `!theme`:** it would join Rust's theme metadata line, while Java joins it with the first theme line. Not probed.
3. **Out of scope and unchanged:** T14.52 (`mc1_seq_foreach_body`), T14.53, T14.54 (`tc1_seq_theme_local_postjoin`), T14.32 (sprites: `tc1_seq_sprite_*`), T14.33, and T14.21/T14.31 (the 11 fresh errors vs `9834a7194`).
4. **`tc1_activity_legacy_label`:** the join is right (`First line`), but legacy-activity geometry and `data-source-line` differ at every build. No target owns this; one is proposed.
