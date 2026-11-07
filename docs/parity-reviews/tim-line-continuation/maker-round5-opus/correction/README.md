# Round-5 correction after checker review-5 (`\startuml` main blocks)

Maker: Claude Opus 5.5. This answers review `7a610890c` (`../../review-5.json`, verdict: rejected), which named one blocker: DF `cr5o_scope_uml_backslash_start`. It is not a completion claim, and it awaits a checker.

## Commits

| Commit | Role |
|---|---|
| `1ad869dae` | Merge of parity-recovery `28af44ab3` (new commit, no rebase). `bullseye.yaml` is identical to `28af44ab3`. **New merged base.** |
| `5956d5915` | Addendum 3 (before any code): Java trace of `Pipe`, `BlockUmlBuilder`, `StartUtils` and `DiagramType`, and the planned start test |
| `b24935a2d` | Start test: the preprocessor's block start, `split_blocks` and `detect_type` use `StartUtils#isStartDirective` (`[@\\]start<type>`, case-insensitive); `split_blocks` ends at `isEndDirective` |
| `55a14b0f1` | Addendum 4 (post-measurement, before the change): line numbering when Pipe finds no `@start` mark |
| `ea5375293` | **Tip code.** Block lines are numbered from Pipe's text (Pipe.java:217) |
| `102291307` | Pre-registration of held-out batch o5s |
| `32c61d7f0` | Pins: 11,183/11,251 |

## Java trace

- **`Pipe#readSingleDiagram`** (Pipe.java:180-225) looks only for a literal `startsWith("@start")`. When it finds no such line, it returns `"@startuml\n" + input + "@enduml\n"` (line 217).
- **`BlockUmlBuilder#<init>`** (BlockUmlBuilder.java:100-133) restarts the block at every `StartUtils#isStartDirective` line and closes it at the first `isEndDirective` line.
- **`isStartDirective`** (StartUtils.java:52) is `DiagramType#findStartTypes` (DiagramType.java:69-92). It skips whitespace, requires `@` or `\`, and matches `start` plus a type with `check`, which is `regionMatches(true, …)` (line 218): **case-insensitive**.
- **`isEndDirective`** (StartUtils.java:110, via `startsWithDirectiveKeyword` at 86) matches `end` **case-sensitively**.
- **Effects:**
  - `\startuml`…`\enduml` is a SEQUENCE block (PSystemBuilder.java:210-212).
  - In `@STARTUML`…`@ENDUML`, the start opens the block, but `@ENDUML` is a content line. That is why Java rejects `cr5o_scope_uml_upper_start`, and the tip still rejects it.
- **`data-source-line`** is the position in Pipe's text. A start line Pipe does not recognise (`\`, indented, or upper-case) is numbered from Pipe's prepended line.

## Measurement

**Method.**
- Builds: fresh git-archive release builds of `ea5375293`, `1ad869dae` and `28af44ab3`. The archive includes `third_party/`, which parity-recovery now patches `svg2pdf` from. The `9ed52f435` binary is reused; its hash is in `binaries.sha256`.
- Comparator and scripts: `compare_cli`, `replay.py`, `classify.py` and `rejection_contract.py`, all unchanged.
- References: DD and DF were re-verified against the parent-validated `reference-results.json`: 13 and 72 cases, 0 mismatches.

| Cohort | tip match (+Welcome) | vs `1ad869dae` worse/gains | vs `9ed52f435` | vs `28af44ab3` |
|---|---:|---|---|---|
| DD | 10 | 0/0 | 0/9 | 0/9 |
| DF | 45 | 0/2 | 0/11 | 1/28 |
| CC | 18 (+8) | 0/0 | 0/4 | 8 (+1 lost Welcome)/19 |
| CB | 24 (+6) | 0/0 | 0/0 | 0/36 |
| CA | 49 (+17) | 0/1 | 0/1 | 3/71 |
| BZ | 13 (+5) | 0/0 | 0/0 | 0/18 |
| BY | 27 (+5) | 0/1 | 0/1 | 1/37 |
| BW | 15 | 0/0 | 0/0 | 1/18 |
| BU | 32 | 0/0 | 0/1 | 0/31 |
| BD | 25 | 0/0 | 0/0 | 1/22 |
| BA | 52 | 0/0 | 0/2 | 1/12 |
| AY | 36 | 0/0 | 0/0 | 0/1 |
| AZ | 5 | 0/0 | 0/0 | 1/0 |
| AX | 27 | 0/0 | 0/0 | 2/6 |
| AV | 20 | 0/0 | 0/0 | 2/10 |
| AT | 20 | 0/0 | 0/0 | 1/8 |
| 915 | 249 | 0/0 | 0/0 | 2/334 |
| retained | 2,744 | 0/0 | 0/3 | 13/331 |
| golden | 11,183 (+17) | 0/0 | 0/1 | 0/126 |

**Against the merged base `1ad869dae`:** 0 worse. There are 4 gains:
- `cr5o_scope_uml_backslash_start` (valid-error → match);
- `cr5o_scope_uml_leading_ws_start` (mismatch → match);
- CA `cr3_end_bs_in_block_comment` (→ Welcome match);
- BY `cr2_seq_startUML_case_end_merged` (valid-error → match).

One more change, BY `cr2_ditaa_case_start_continued_row` (`@startDitaa`), goes from valid-error to rendering. Java's reference for it is a PNG, so `compare_cli` exits 101 on reading it.

**Against `9ed52f435`:** 0 worse.

**Against `28af44ab3`:** 38 worse, every one on review-5's approved list (`checker-opus-round5/merge/worse-vs-32684f81e.json`):
- the 37 cases of round 4's `cb550c0d6` list, including the lost Welcome match `cr4_cnt_exit_alone`;
- `cr5o_scope_uml_usecase_theme`, under the approved usecase mechanism.

The list's 39th entry, `cr5o_scope_uml_backslash_start`, now matches. Nothing else is worse.

**Other results:**
- Panics: only the unchanged `--yaml` set (retained 63, golden 148). Timeouts: 0.
- Welcome matches: CC 8/9, CB 6/6, CA 17/19, BZ 5/5, BY 5/5, golden 17/17.
- `rejection_contract.py`: 0 introduced losses in all 51 runs (17 cohorts × 3 builds). DF rejections: 9/15 at the tip, the base and `9ed52f435`; 3 at `28af44ab3`.

## Gates

Gates ran at worktree `102291307` (code `ea5375293`) and at a `1ad869dae` export.
- **Workspace lib:** 1,679 passed, 0 failed.
- **`golden_no_oracle`:** 11,183/11,251 at the tip and the base, with nothing unlocked.
  - The 68 failure names are identical to the base's.
  - Against `28af44ab3`, the only difference is `edge_pragma_useVerticalIf`, newly passing.
- **`pipe_mode` and `doc_claims`:** failed on the 11,182 pins; after the pin commit, 17/17 and 2/2.
- **`golden_pairs`:** 10,498/753/1,299, with a summary identical to the base's.
- **`parity_perturbations`:** 3,793 failing pairs, against 3,794 at the base. 0 new; 1 fixed (`cr2_seq_startUML_case_end_merged`).
- **`harness_guards`:** 5 pass. `decimal_literal_ratchet` and `parity_review_records_are_complete` fail with normalised output identical to the base's.

## Pending relay: batch o5s

- Directory: `~/Library/Caches/rustuml-opus-tim-r5/o5s/`, read-only. It holds 28 cases (26 predicted Java-valid) and 3 resources.
- `reference-manifest.json`: sha256 `839f67fcedc46e7abd62f85dae67c053e527080af8cd7f4cd7af509bedec3551`.
- `reference-input-bundle.json.gz`: sha256 `3d28c209935d22ae21455fe27049e1a6dc46b46593e31fb9a5e2ec80c85a674b`.
- Rust outputs: `../pending-request-o5s/o5s-rust-predictions.json` (sha256 `e687df07c7f282321ba49cbf07519f87c6708e1cf0b1a017b22bcfb1aa6bdeae`).

**Known prediction risk:** `o5s_nest_at_then_bs` and `o5s_nest_bs_then_at`. Java restarts the block at a later start, so I predict it renders only `two`. Rust keeps the first start and renders `one` and `two`, and parity-recovery does the same for `@`-in-`@`. This is pre-existing.
