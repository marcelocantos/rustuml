# Round-6 correction after re-check review-6

Maker: Claude Opus 5.5. This answers review `78cc4caf8` (`../../review-6.json`, verdict: rejected). It is not a completion claim, and it awaits a checker.

**Correction of record.** My round-5 correction report claimed "0 worse vs the merged base" and "38 worse vs `28af44ab3`, all approved". I measured that without DH and DJ, and both claims are false over the full cohort set. Review-6's counts, 8 and 65, stand. `account-addendum-5.json` records the correction.

## Commits

| Commit | Role |
|---|---|
| `c25ccbf3c` | Merge of parity-recovery `9fc567332` (G5), as a new commit. `bullseye.yaml` is identical to the head. The `parse/state.rs` conflict keeps both sides: this line's command spans and sprite skip, plus G5's fallible predeclaration and `first_content_line`. **Merged base.** |
| `c23d43b04` | Addendum 5, before any code: Java traces for all three obligations and the declared `chen` exception |
| `59d78989f` | **Tip code.** `UncommentReadLine` on the main block; Pipe-text numbering when Pipe finds no mark; `DiagramType#getTypes` |
| `c7d511ff0` | Pre-registration of held-out batch o6n |

## Java traces

1. **Numbering.**
   - `Pipe#readSingleDiagram` (Pipe.java:180-225) does not append `@@@format ` lines (188), nor blank lines while it is still in NO_CONTENT (191, 207).
   - When no line `startsWith("@start")`, it returns `"@startuml\n" + sb` (217).
   - `data-source-line` is the 0-based position in that text.
   - Every content line of such an input is now marked with that position.
2. **Type.**
   - `DiagramType#getTypes` (DiagramType.java:94-216) switches on the lower-cased first character after `start`, then runs case-insensitive prefix checks in Java's order (`project` maps to GANTT).
   - Anything else is UNKNOWN, which becomes PSystemUnsupported (PSystemBuilder.java:225-227).
3. **NBSP.**
   - `BlockUmlBuilder` reads through `UncommentReadLine` (BlockUmlBuilder.java:91; UncommentReadLine.java:57-81).
   - For ` @startuml`, `StartUtils#beforeStartUml` returns the header ` `, because NBSP is neither `Character.isWhitespace` nor a word character. `UncommentReadLine` strips that header, so the line read is `@startuml`.

## Per-pair results (tip `59d78989f`, unchanged `compare_cli`; `per-pair-tip.txt`)

- **The 17 required numbering pairs match:**
  - DJ: `r5b_num_bs_state`, `r5b_num_bs_object`, `r5b_num_bs_deployment`, `r5b_num_bs_seq_three`, `r5b_num_upper_at_seq`, `r5b_num_us_bs_seq`, `r5b_num_bs_blank_inside`, `r5b_start_bs_name`, `r5b_end_bs_bare`, `r5b_multi_bs_then_bs_mindmap`, `r5b_crlf_bs_seq`.
  - DH: `o5s_bs_uml_class`, `o5s_mix_bs_start_at_end`, `o5s_ws_tab_bs`, `o5s_ws_bs_end_indented`, `o5s_case_bs_upper_start`, `o5s_case_at_upper_start_lower_end`.
- **The 6 further pairs the same rule fixes also match:** `r5b_num_ws_at_seq`, `r5b_num_emsp_seq`, `r5b_num_tab_at_state`, `r5b_num_comment_bs_seq`, `o5s_two_blocks_bs`, `o5s_bs_seq_continuation`.
- **Blank-line skip (review-6 residue), now matching:** `r5b_num_two_blank_bs_seq`, `r5b_num_pipe_nostart_blank_ws`, `o5s_ws_leading_blank_line`.
- **Start type and NBSP:**
  - `r5b_start_bs_space_uml` is rejected, as Java rejects it.
  - `r5b_start_bs_project`, `r5b_start_nbsp` and `r5b_start_bs_umlish` match.
- **Declared exception, `r5b_start_bs_chen`:** Java draws CHEN_EER, and Rust has no Chen parser (`@startchen` is unsupported at every build). The tip reports "unsupported diagram type", the same as the merged base. That is worse than `9ed52f435`, which drew an accidental CLASS diagram only because `\startchen` was not a start line there. No fallback reproduces that output; this is left to the owner.
- **Marked-input residue, kept and declared:** `r5b_num_comment_at_seq` and `o5s_ws_leading_blank_at` still mismatch. Pipe's discard of lines before a column-0 mark is not ported, because the 8 goldens with lines before `@start` carry file-relative numbers (picoweb).

## Three-way check

**Method.**
- Builds: fresh git-archive builds of `59d78989f`, `c25ccbf3c` and `9fc567332`. The `9ed52f435` binary is reused; its hash is in `binaries.sha256`.
- Coverage: 21 cohorts, DD, DF, DH and DJ included, in 4 modes, with per-replay timeouts.
- Classification:
  - the T14.63 Welcome overlay;
  - review-6's declared validity overlay: Java exit 100/50 or a "Diagram not supported by this release" page counts as Java-invalid;
  - a lost Welcome match counts as worse.

| Against | Worse | Notes |
|---|---:|---|
| merged base `c25ccbf3c` | **0** | 35 gains: DJ 25 (including the `@start uml` and `\start uml` rejections), DH 9, BW `r2_seq_indented_start_and_end` |
| `9ed52f435` | **1** | the declared `r5b_start_bs_chen` |
| parity-recovery `9fc567332` | 43 | 38 on review-5's approved list; 4 of review-6's 7 inherited (`r5b_num_unmarked_bs_end_only`, `r5b_start_bs_files`, `r5b_start_bs_creole`, `r5b_nest_uml_content_bs_json`); `r5b_start_bs_chen` |

Review-6's other three inherited cases are no longer worse: `r5b_start_bs_umlish` and `r5b_start_bom` now match, and BY `cr2_seq_two_blocks_first_swallowed` equals the head.

**Other results:**
- Panics: only the unchanged `--yaml` set (retained 63, golden 148). Timeouts: 0.
- Welcome matches: DJ 1/2, CC 8/9, CB 6/6, CA 17/19, BZ 5/5, BY 5/5, golden 17/17.
- Rejection contract: 0 introduced losses in 56 of 57 runs. The 57th, DJ against `9ed52f435`, flags `r5b_empty_bs_block`, which is the Welcome-match gain review-6 identified (Java and the tip both show the Welcome page).

## Gates

Gates ran at worktree `59d78989f` and at a `c25ccbf3c` export.
- **Workspace lib:** 1,708 passed, 0 failed.
- **`pipe_mode`:** 17/17. **`doc_claims`:** 2/2.
- **`golden_no_oracle`:** 11,183/11,251 at the tip and the base. The 68 failure names are identical to the base's. The lock and pins are unchanged.
- **`golden_pairs`:** 10,498/753/1,299, with a summary identical to the base's.
- **`parity_perturbations`:** 3,727 failing pairs, against 3,728 at the base. 0 new; 1 fixed (`r2_seq_indented_start_and_end`).
- **`harness_guards`:** 5 pass. `decimal_literal_ratchet` and `parity_review_records_are_complete` fail with normalised output identical to the base's.

## Pending relay: o6n

- Directory: `~/Library/Caches/rustuml-opus-tim-r5/o6n/`, read-only. It holds 36 cases (33 predicted Java-valid) and 2 resources.
- `reference-manifest.json`: sha256 `ad64ec9d13a85ea434218294419a3d6bdfb008c6cad49423cc12e9107c4758d1`.
- `reference-input-bundle.json.gz`: sha256 `c782c87487d0ae2629beda3b2a2ab872611537e95d3dd38529b459b6084c3fc3`.
- Rust outputs: `../pending-request-o6n/o6n-rust-predictions.json` (sha256 `37f02451baaaf165a3063aff359d8b6ea9190a91eaf749cdda43be9b195623df`).
