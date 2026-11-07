# TIM line continuation (🎯T14.22) round 3: maker record

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/tim-continuation-round3-claude`. This round answers the round-2 review (`../review-2.json`, `../checker-claude-round2/`, `../../sprite-command-post-tim-ownership/review-6.json`) and carries the whole sprite/participant line. An independent checker must review it. Nothing here is a completion claim.

**Branch history.**
- Start: `c07cad778` (round-2 review, on maker tip `e3aa1c6b4`, code `f2d12d0ef`).
- Merge with `parity-recovery-2026-11-03` `adf56ce53`: `a5720e095`. `bullseye.yaml` merged cleanly and the code did not change. This commit is the **merged base**; its code is identical to `f2d12d0ef`.
- Account, committed before any code change: `account.json`, commit `60bee23b8` (10:39).
- Implementation: `d28dd64a9` (10:44). It touches `crates/rustuml-parser/src/preprocess/mod.rs` and `crates/rustuml-parser/src/parse/mod.rs`.
- Pre-registration of batch MR3: `e141389dc` (10:47), before any Java result.

No code changed after measuring, so there is no addendum. Java `71806a23780b04a5ccde2f8ceb5121edad5eb711` was read only and never run locally.

## Obligation 1: out-of-band stand-in (blocking): done

**Java.** The TIM result list has no entry for a directive or comment line. Rust keeps a stand-in for each such top-level line, only to keep its parser stream aligned with source lines.

**Defect.** Round 2 made that stand-in the string `"\x1ERSP"`, so any user line with that text was deleted.

**Rust change.**
- The TIM output stream is `Vec<TimLine>`, where `enum TimLine { Text(String), Consumed }`.
- `push_directive_placeholder` and the post-procedure padding push `Consumed`. So do the `!option` (non-handwritten) and empty or inactive `!theme` directives, which round 2 represented as a real blank line; Java's TIM gives them no entry either. Every other push is `Text`.
- Nested outputs (includes, `!includesub`, themes) carry the type through.
- `collapse_post_tim_ending_backslashes` passes over `Consumed`. Only there, or in `tim_texts` when no join runs, does the stream become `Vec<String>` again. `Consumed` becomes the empty line `b0fb43c3e` used.
- No string value is special any more.

**Result (BY).**

| Case | Java | `b0fb43c3e` | merged base | tip |
|---|---|---|---|---|
| `cr2_seq_chr30_rsp_line` | exit 200 | rejected | accepted (line dropped) | **rejected** |
| `cr2_seq_raw_rs_rsp_line` | exit 200 | rejected | accepted (line dropped) | **rejected** |
| `cr2_seq_note_chr30_rsp` | crash page (invalid) | note `RSP` drawn | note text dropped | **note `RSP` drawn** |

The control `cr2_seq_note_plain_rsp_control` still matches. The unit test is `consumed_lines_are_out_of_band`.

## Obligation 2: Java's two-line source after the join (blocking): done

**Java.** `PSystemBuilder#createPSystem` tries `PSystemWelcomeFactory` first (PSystemBuilder.java:129; its diagram type is SEQUENCE, "a hack for @startuml"). It returns `PSystemWelcome` when `UmlSource#getTotalLineCount() == 2`, before any family factory or command runs. The two lines are the start line and the joined list `UmlSource#loadInternal` built (UmlSource.java:118-134, 198-200).

The join produces two lines in two ways:
- a pending tail absorbs the end line;
- the end line itself ends with a backslash, stays pending and is never added.

Separately, `BlockUmlBuilder` ends the block at the first `StartUtils#isEndDirective` line, which is `@` or `\` followed by `end`. So `\enduml` is the block's end line.

**Rust change.**
- `collapse_post_tim_ending_backslashes` now runs over Java's whole list: stand-ins are excluded, and the end line is the last entry, dropped when it ends with a backslash. It counts the result together with the start line.
- When the join changed the list and the count is 2, `PreprocessOutput::two_line_source` is set.
- `parse_with_base` then returns the empty `@startuml` diagram (Rust's Welcome page). This happens before the allowmixing check and every family parser, and after the existing no-block (swallowed-end) case.
- At the top level, outside procedure bodies, the block also ends at `is_end_directive`, the existing port of `StartUtils#isEndDirective`.

**Result (BY).** Every Java Welcome case in BY is now a Rust Welcome page:

| Case | Java | `b0fb43c3e` | merged base | tip |
|---|---|---|---|---|
| `cr2_seq_depth2_tail_then_end` | Welcome | Welcome | message | **Welcome** |
| `cr2_seq_tail_then_indented_end` | Welcome | Welcome | message | **Welcome** |
| `cr2_seq_chr92_tail_then_end` | Welcome | Welcome | message | **Welcome** |
| `cr2_seq_tail_then_backslash_end` | Welcome | message `tail \enduml` | same | **Welcome** |
| `cr2_seq_end_line_continued_then_text` | Welcome | message `one` | same | **Welcome** |

The unit tests are `two_line_source_after_the_join` and `two_line_join_gives_the_welcome_diagram_first`. The second checks that a joined line no command accepts still gives Welcome, because the factory runs first.

**Declared residue.**
- Two-line sources that arise without a join are left to the existing empty-diagram path. Rust's stream carries Rust-only text lines (the `!theme` metadata line, `skinparam __optionHandwritten`) that would miscount them. A `!option handwritten` line before a two-line join is miscounted for the same reason.
- `split_blocks` (the multi-block splitter) still recognises only `@end` as a block end, not `\end`.

## Obligation 3: wrapped include files: done for local files

**Java.**
- `TContext#executeInclude` (TContext.java:794-870) reads a local file through `DiagramDetector#extractFromFile` (DiagramDetector.java:71-113): `ReadFilterMergeLines`, then `UncommentReadLine`.
- When one of those lines is a start directive, `DiagramExtractor` returns the lines after the first start and before the first end.
- Otherwise the file is read raw, with no merge.

**Rust change.**
- In `try_include`'s local-file branch, `wrapped_diagram_lines` ports this: `read_filter_merge_lines`, then `UncommentReadLine`'s header removal (`before_start_uml` ports `StartUtils#beforeStartUml`), then the start/end window.
- Each line is processed with its first piece's file line (`process_numbered_lines`).
- Unwrapped files are processed as before.
- Unit test: `wrapped_include_is_extracted_from_merged_lines`.

**Result.** `cr2_seq_wrapped_include_tail_then_end` and `cr2_seq_wrapped_include_tail_then_msg` (BY) change from mismatch at every earlier build to **match**.

**Residual (bounded out).** The stdlib branch (`PreprocessorUtils#getReaderStdlibInclude` → `extractFromBytes`) and the URL branch use the same extractor, and Rust does not port it there. The stdlib is a host install, and a change there needs a stdlib-wide measurement. A target is proposed.

## Measurements

**Setup.**
- Comparator and scripts: unchanged `compare_cli` (linked against `93bf53e18`'s `rustuml_oracle`), `prepare.py`, `replay.py`, `classify.py`, `fresh_classify.py` and `rejection_contract.py`. Their hashes, in `results/unchanged-scripts.sha256`, equal `checker-claude-round2/support/unchanged-checker-scripts.sha256`.
- Modes: 4. Each case runs from its recorded cwd.
- Builds: five private `git archive` builds, one `CARGO_TARGET_DIR` each (`support/build.sh`; hashes in `results/binaries.sha256`): tip `d28dd64a9`, merged base `a5720e095`, `b0fb43c3e` (the round-2 merged base), `93bf53e18` and `9834a7194`.
- Prepared cohorts: re-verified against the relay reference results (`support/verify_fresh.py`, `results/verify-fresh.json`). There are 0 source or SVG hash mismatches in all 11 manifests, and the BY/BW/BU/BD include resources match the relay bundles.
- BY validity: the 5 BY validity differences are the Java exit-50 empty inputs (the no-diagram rule).
- Welcome pages: `support/summarize.py` classifies them explicitly (Java `Welcome to PlantUML`, Rust `Welcome to RustUML`), because the unchanged outcome folds them into nonpositive-accepted.
- Data: `results/summary.json`, `results/runs/`, `results/manifests/`, `results/probes/`.

### Against the merged base `a5720e095` (the bar)

There are no losses in any cohort:

| | Count |
|---|---:|
| Lost matches | 0 |
| Lost rejections | 0 |
| New valid-input errors | 0 |
| New panics | 0 |
| Timeouts | 0 |
| Lost Welcome pages | 0 |

| Cohort | n (Java-valid) | tip match | Change vs merged base |
|---|---:|---:|---|
| BY | 53 (36) | 24 | **+2 gains** (both wrapped includes); 2 newly rejected (the RSP pair); 5 Welcome pages gained, all Java Welcome. 10 SVG changes: these 9 plus `cr2_seq_note_chr30_rsp` (note text restored) |
| BW | 23 (20) | 14 | none (0 bytes) |
| BU | 40 (37) | 29 | none (0 bytes) |
| BD | 33 (29) | 23 | none (0 bytes) |
| BA | 64 (57) | 41 | none (0 bytes) |
| AY | 48 (44) | 34 | none (0 bytes) |
| AZ | 8 (8) | 3 | none (0 bytes) |
| AX | 77 (69) | 26 | none (0 bytes) |
| AV | 63 (60) | 20 | none (0 bytes) |
| AT | 45 (42) | 19 | none (0 bytes) |
| AS/AQ/790/28 | 915 (602) | 247 | none (0 bytes) |
| retained | 4,174 | 2,667 | none (0 SVG bytes, 0 model changes) |
| golden | 12,550 | 11,181 | **0 outcome changes, 0 SVG bytes** |

The 2 golden model-mode differences are run-to-run nondeterminism (the `dot/*` ast/yaml set the round-2 checker named). `dot_edge_labels --ast` gives the same two hashes at the tip and at the merged base.

Panics: only the unchanged `--yaml` set (retained 63, golden 148), identical at the tip, `a5720e095`, `b0fb43c3e` and `93bf53e18`. `9834a7194` has 72/160.

### Against `b0fb43c3e` (the round-2 merged base)

**BY:**
- 8 gains;
- 5 newly rejected: the 5 empty inputs, as in round 2;
- 2 Welcome pages gained (Java Welcome): `tail_then_backslash_end` and `end_line_continued_then_text`;
- 6 Welcome pages lost where Java is not Welcome: the 5 empty inputs, now rejected, and `wrapped_include_tail_then_end`, now a match;
- 1 valid-error → mismatch;
- 0 lost matches, 0 lost rejections.

**BW:**
- 13 gains;
- 1 new valid-input error, `r2_component_include_tail_then_variable` (T14.21, unchanged from round 2, for owner sign-off);
- 1 valid-error → mismatch.

**Retained:** 1 newly rejected (`checker_092_empty`, as in round 2).

**Every other cohort:** no change.

### Against `93bf53e18` and `9834a7194`

These are the round-2 lists, unchanged, with one exception.

**vs `93bf53e18`:**
- `cr2_seq_tail_then_backslash_end` was a lost rejection. Its rejection at `93bf53e18` was accidental (`\enduml` unsupported). The tip now gives Java's own outcome, the Welcome page.
- Lost matches `ck4_procedure_sprite_end_comment_two` and `mk4_tim_sprite_in_procedure` (T14.33).
- Lost rejections `ck3_svg_inline_quoted_apostrophe_seq` (T14.32) and `mc1_seq_foreach_body` (T14.52).

**vs `9834a7194`:**
- No lost matches or rejections.
- New valid-input errors: the 11 known fresh cases (T14.21/T14.31) and `cr2_seq_pre_start_backslash_text` (T14.26).
- The retained 13 are the approved set.

### Welcome classification (explicit)

**Java Welcome pages:** BY 5, BA 5, AX 3, AS/AQ/790/28 7, retained 14, golden 126.

**Java Welcome but tip not Welcome, all unchanged against the merged base and `b0fb43c3e`:**
- BA `ck4_seq_double_leading_top`, `ck4_inv_nbsp_leading_comment_seq` and `ck4_inv_class_after_leading_two` have been rejected since `b0fb43c3e`: a leading `/' … '/ /' …` comment run.
- AX `ck3_nested_startsub_seq`.
- Golden `preprocessing/preproc_include_startsub`.

These come from comment and `!startsub` handling, not from the join. A target is proposed.

### `rejection_contract.py` (unchanged; `results/rejection/`)

| Before build | Cohorts | `introduced_losses` |
|---|---|---|
| `a5720e095` | every fresh cohort | **0** |
| `b0fb43c3e` | every fresh cohort | **0** |
| `93bf53e18` | BY | `cr2_seq_tail_then_backslash_end` (now Java's Welcome, above) |
| `93bf53e18` | AX | `ck3_svg_inline_quoted_apostrophe_seq` |
| `93bf53e18` | BD | `mc1_seq_foreach_body` |

### Gates (`support/gates.sh`; `results/gates/`)

- **Branch:** HEAD `e141389dc` (tip code `d28dd64a9`). `git status` before and after showed only the three untracked support scripts written before the run.
- **Workspace lib:** 1,514 passed, 0 failed. Round 2 had 1,510; this round adds 4 unit tests.
- **`pipe_mode`:** 13 passed.
- **`golden_no_oracle`:** 11,181 of 11,251 eligible pass. The 70 failure names are byte-identical to `9834a7194` (and to round 2). It fails only on the same unlocked class/preprocessing/rendering baseline lines as round 2.
- **`golden_pairs`:** 10,497 passed, 754 failed, 1,299 skipped. The 754 names are byte-identical to `9834a7194`.
- **`parity_perturbations`:** 2,539 failures at both the tip and the merged base: **0 new, 0 fixed** (`pert-new.txt`, `pert-fixed.txt` are empty).
  - The T14.59 char-boundary panic occurs at both builds. Its line is `preprocess/mod.rs:2711` at the tip and `:2672` at the base; it moved only because of lines inserted above it.
  - This round does not touch `eval_builtins_once`.
- **`harness_guards`:**
  - `comparator_constants_frozen`, `parity_review_paths_reject_traversal`, `no_golden_data_embedded_in_shipping_crates` and `error_golden_census_is_frozen` pass.
  - `decimal_literal_ratchet` fails on the same 5 render files with the same counts as the merged base.
  - `parity_review_records_are_complete` gives 2,708 violations, identical to the merged base after normalising the tree root (`harness_guards-records-*.txt`). None is a T14.6.7 violation in this mechanism or in `sprite-command-post-tim-ownership`.

### Process and frozen items

- Nothing under `crates/rustuml-oracle`, `crates/rustuml/tests`, `test-diagrams`, `scripts` or `.gitmodules` changed between `a5720e095` and the tip. Comparator slack, syntax markers, the park list, the error census, guard allow-lists, the strict-XML list and the 15-source approved set are unchanged.
- No fixture-keyed logic: fixture-like strings appear only in `#[cfg(test)]` tests.
- The account precedes the code.

## Pending relay request: batch MR3

The batch was staged immutably at `~/Library/Caches/rustuml-tim-continuation-round3-claude-maker/relay-mr3/`, and a copy is in `pending-request-mr3/`. It has 23 cases (14 predicted Java-valid) and 12 resources:
- stand-in collisions;
- two-line variants, including a joined line no command accepts and a class link tail;
- `\enduml` without a join;
- wrapped includes: preamble, trailer, continuation, indented, comment header, two blocks, class;
- controls.

| File | sha256 |
|---|---|
| `reference-manifest.json` | `f84459521717326e95590560252eda6ae96e5318e3fcd9e4f8470f41c5d7938c` |
| `reference-input-bundle.json.gz` | `fbaf2bcfe4e534c40287f2d58d2b520605e72dba2940dec1cfc3d83e0b530289` |
| `mr3-rust-predictions.json` (Rust at the five builds) | `c0cd5a9277c130a1e3c8eae681bf1382ea4ea4b3cdf13e4739bdf7805b67f4e9` |
| `cases-with-java-predictions.json` | `35d0c1502a956a32cb5e93b669a3d154f82273461e8c9d3ef34789e0ff253f43` |

## Proposed targets (not filed; for the parent)

1. **The source-line marker is in band.** `SOURCE_LINE_PREFIX` is `"\x1ERSL:"`, with the `\x1F` separator. A user line `%chr(30)RSL:5%chr(31)x` would be read as a marker. It should be carried out of band, like `TimLine`.
2. **Stdlib and URL includes use `DiagramDetector` extraction** (merged, between the first start and end) as local files now do.
3. **Rust-only metadata lines** (`skinparam __theme …`, `skinparam __optionHandwritten true`) are not lines of Java's TIM result list, but the post-TIM join and the two-line count treat them as lines. This includes the tail-before-`!theme` residue (`cr2_seq_tail_then_theme`).
4. **Java-Welcome outcomes Rust does not reproduce**, unchanged this round: BA `ck4_seq_double_leading_top`, `ck4_inv_nbsp_leading_comment_seq` and `ck4_inv_class_after_leading_two` (rejected); AX `ck3_nested_startsub_seq`; golden `preprocessing/preproc_include_startsub`.
5. **Extend T14.58:** `split_blocks` should recognise `\end…` (`StartUtils#isEndDirective`) as well as `@end…`. Also, Java ends a block at a line-initial `\end{…}`, even in `@startlatex`/`@startmath`; there is no corpus coverage for this.
