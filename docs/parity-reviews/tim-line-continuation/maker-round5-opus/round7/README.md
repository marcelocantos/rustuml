# Round-7 correction after re-check review-7

Maker: Claude Opus 5.5. This answers review `a80bb440f` (`../../review-7.json`, verdict: rejected). It is not a completion claim, and it awaits a checker.

## Corrections of record (`account-addendum-6.json`)

- **Count against the head.** Round 6's "43 worse than `9fc567332`" was wrong. Review-7 counts 48: 45 on my own cohorts once DK's results existed, plus 3 from DL.
- **BY `cr2_seq_two_blocks_first_swallowed`.** Round 6 said it "equals the head", which is wrong. The tip, the merged base and `9ed52f435` report "No valid @start/@end found" (exit 1). `9fc567332` produces output the comparator cannot score (exit 3). Java exits 0 with the "No valid @start/@end found" page. It is inherited by parent decision and untouched.
  - The two summarisers disagree on it. The manifest marks that Java page invalid, so my unchanged `classify.outcome` plus the validity adjustment scores the tip as a matched rejection and the head as an accepted invalid input, and this record does not count it as worse. Review-6/7 count it as worse, and inherited.
- **Golden marks.** Addendum 5 said every golden has a Pipe mark. False: the ten `type-detection/noheader_*` goldens have none. They match at every build because PicoWebServer (lines 278-279) wraps them as Pipe does.

## Commits

| Commit | Role |
|---|---|
| `6d195c178` | Merge of parity-recovery `f6a738af1` (bullseye only). `bullseye.yaml` is identical to the head; the code is identical to `59d78989f`. **Merged base.** |
| `7d9694cdb` | Addendum 6, before any code; plus `support/gen_ascii_fold.py` |
| `50adb43e2` | Code draft. It was committed while one lib test failed (`parse_block_preserves_absolute_source_lines`), because my command chain did not stop on the failure. |
| `466e6d883` | Addendum 7: cause of that failure and the plan |
| `16b1c4dcf` | **Tip code.** Leading Pipe-dropped lines become empty placeholders; `@@@format ` lines are removed |
| `4ef8260d2` | Pre-registration of held-out batch o7p |

## Java traces and changes

1. **Lines Pipe never appends.**
   - Java: `@@@format ` lines are dropped anywhere (Pipe.java:188-189), and trim-empty lines are dropped while NO_CONTENT (191-192, 207-208).
   - Rust: a removed `@@@format ` line turns on explicit numbering for the lines after it. A leading trim-empty line stays as an empty placeholder: it precedes all content, so it is no TIM or parser input, and the stream positions that `parse_block`'s preamble padding relies on are kept.
2. **`String#regionMatches(true, …)` per UTF-16 unit** (DiagramType.java:218-220), for both `start` and the type keys.
   - The non-ASCII units that match an ASCII letter are U+0130→i, U+0131→i, U+017F→s and U+212A→k. `gen_ascii_fold.py` derives them from UnicodeData 15.0.0, and the Rust table equals its output.
   - U+00DF (sharp s) does not match `s`.
   - The `getTypes` switch keeps `Character.toLowerCase` semantics.
3. **`ReadLineReader` on the main block** (BlockUmlBuilder.java:91; ReadLineReader.java:89-103). One leading U+FEFF is dropped and U+2013 becomes `-`, before `UncommentReadLine`. Pipe's own mark test still sees the raw line.

## Per-pair results (tip `16b1c4dcf`; `per-pair-tip.txt`)

- **Required, all matching:** DK `o6n_format_line_bs_seq`, DL `q7_num_us_blank_bs_seq`, DL `q7_type_bs_dotless_mindmap`, DL `q7_hdr_bom_content`.
- **Round-6 list:** unchanged at 29 matches, 2 marked-input residue (T14.114), and 5 expected rejections or unsupported types (`chen` among them, T14.113).
- **New gains against the merged base (10):**
  - the 4 required cases;
  - CC `cr4_main_bom_line2`, `cr4_main_bom_before_directive`, `cr4_main_endash_arrow`, `cr4_main_endash_label` and `cr4_main_endash_include_path` (three of these were on review-5's approved list);
  - retained `times-new-roman-unicode-advances/checker_punctuation_four_faces_tb_nested` (U+2013 in the source).

## Three-way check

**Method.**
- Builds: a fresh git-archive build of the tip. The merged base and the head have code identical to the fresh round-6 builds `59d78989f` and `9fc567332` (the `git diff` is empty), so those binaries stand in for them. `9ed52f435` is reused.
- Coverage: 23 cohorts, DK and DL included, in 4 modes, with replay timeouts.
- Classification: the T14.63 Welcome overlay and review-6's validity overlay; a lost Welcome match counts as worse.

| Against | Worse | Notes |
|---|---:|---|
| merged base `6d195c178` | **0** | 10 gains; SVG bytes change only in those 10 cases |
| `9ed52f435` | **1** | `r5b_start_bs_chen` |
| head `f6a738af1` | 40 | 35 of review-5's 38 (`cr4_main_bom_line2`, `cr4_main_bom_before_directive` and `cr4_main_endash_arrow` now match); the 4 named inherited cases; `chen`. BY is not counted, as explained above. Nothing else. |

**Other results:**
- Panics: only the existing `--yaml` set (retained 63, golden 148, DL 1 = `q7_type_bs_ebnfx`, the same as the base). No new mode panics. Timeouts: 0.
- Welcome matches: DJ 1/2, CC 8/9, CB 6/6, CA 17/19, BZ 5/5, BY 5/5, golden 17/17.
- Rejection contract: 0 introduced losses in 62 of 63 runs. The 63rd, DJ against `9ed52f435`, flags `r5b_empty_bs_block`, the Welcome-match gain review-6 identified.

## Gates

Gates ran at worktree `16b1c4dcf` and at a `6d195c178` export.
- **Workspace lib:** 1,711 passed, 0 failed.
- **`pipe_mode`:** 17/17. **`doc_claims`:** 2/2.
- **`golden_no_oracle`:** 11,183/11,251 at the tip and the base. The 68 failure names are identical, and the lock and pins are unchanged.
- **`golden_pairs`:** 10,498/753/1,299, identical to the base.
- **`parity_perturbations`:** 3,722 failing pairs, against 3,727 at the base. 0 new; 5 fixed (the four `cr4_main_*` pairs and the Times New Roman pair).
- **`harness_guards`:** normalised output identical to the base's (5 pass; the 2 known failures).

## Pending relay: o7p

- Directory: `~/Library/Caches/rustuml-opus-tim-r5/o7p/`, read-only. It holds 28 cases (23 predicted Java-valid) and no resources.
- `reference-manifest.json`: sha256 `267dfd99dbfc557d06dbeead777e836d825d17b8f659b7e66ff28ce504c3f3c2`.
- `reference-input-bundle.json.gz`: sha256 `4efac2f3893c24118740b33d6cfcbd68897eead6a1796a4220f44dc0683ccaf9`.
- Rust outputs: `../pending-request-o7p/o7p-rust-predictions.json` (sha256 `9b3d9650583d78d57f53b86732b30942f1ba80b2d359744338e8ceedc3c63869`).

**Known prediction risks:**
- `o7p_fmt_no_space` and `o7p_fmt_indented`. Each puts an appended line before `\startuml`; Java's block restarts at the start line, so that line lies outside it. Rust passes the pre-start line through, and every TIM-line build errors on it.
- `o7p_fmt_mid_at_seq`. Its numbering after the dropped line in a marked block is the marked-input residue (T14.114).
