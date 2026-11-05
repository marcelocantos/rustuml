# Post-TIM sprite command ownership, round 4: checker review

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/sprite-round4-claude`. The record is `../review-3.json`. Earlier reviews (`../review.json`, `../review-2.json`, `../checker-claude/`, `../checker-claude-round3/`) are preserved.

**Verdict: rejected.**

- Packet tip: `c9e11493c`. Before trees: `93bf53e18` (pre-packet) and `044f73657` (round-3 code).
- Rejection answered: `6fd931656`. Its accepted participant mechanism is untouched: `044f73657..c9e11493c` changes only `crates/rustuml-parser/src/preprocess/mod.rs` and this mechanism's docs. The participant AX cases are unchanged.

## Cause of the rejection

**Held-out AY case `mk4_tim_sprite_in_procedure`** (Java-valid; the maker had not seen its result) is a lost match against `93bf53e18`:
- `93bf53e18` matched.
- `c9e11493c` mismatches on one attribute: `class Oak` gets `data-source-line="11"`; Java has `8`.
- Against `044f73657` it is a gain (valid-error to mismatch).

Attribution (`controls.json`):
- **The defect predates the packet.** After a multi-line `!procedure` expansion, Rust numbers the following lines by their position in the expanded stream, not by Java's `StringLocated` location.
  - A procedure that emits four classes puts `Oak` at line 11 in all three builds.
  - Java confirms the defect on pairs that already existed: `sequence-registered-command-rejection/bd087_macro_valid` (Java 8, Rust 9; mismatch in all three builds) and `sequence-sprite-block-consumption/c75_sprite_procedure_legacy` (Java 10, Rust 12).
  - No target tracks it.
- **The `93bf53e18` match was accidental.**
  - Its UmlSource contains `} /' e '/`, `!endprocedure` and `$mk()` as diagram text, so the procedure was never expanded and the raw numbering survived.
  - With the comment removed, `93bf53e18` gives `Oak` line 9: also wrong, and not a match.
- **The packet's role:** the round-4 `removeInnerComment` port makes the procedure-wrapped sprite parse as in Java. That exposes the defect.

Classification: a lost match on a held-out case, caused by accidental-match residue. It is not a rejection loss. Under the whole-output rule it rejects the packet.

## The round-3 acceptance items

| Item | Verified result |
|---|---|
| 4 of 6 AX inner-comment pairs | **match** in `c9e11493c` (`ck3_pixel_end_inner_comment_class`, `ck3_svg_end_inner_comment_nospace_component`, `ck3_svg_inline_inner_comment_object`, `ck3_pixel_start_inner_comment_deployment`). `ck3_svg_end_inner_comment_usecase` also matches. |
| `ck3_svg_end_inner_comment_state`, `ck3_pixel_end_inner_comment_seq` | Confirmed, see below. |
| `ck3_svg_inline_quoted_apostrophe_seq` | **Lost rejection against `93bf53e18`.** Accidental-rejection residue; see below. |

**`_state` and `_seq`:**
- Both go from exit 1 to a mismatch.
- `c9e11493c` produces the same SVG bytes as for the comment-free source, which renders identically in `044f73657`.
- Java's output with the comment is structurally equivalent to its output for the AY controls `mk4_ctl_svg_end_state` and `mk4_ctl_pixel_end_seq`. Only `<?plantuml-src?>` differs.
- The remaining difference is sprite drawing (🎯T14.32).

**`ck3_svg_inline_quoted_apostrophe_seq`:**
- It was introduced at round 3 (`044f73657` already accepts it) and round 4 leaves it unchanged.
- AY confirms the Java mechanism. `mk4_svg_text_nofill_plain_seq` and `mk4_svg_text_nofill_usecase` (no apostrophe, no `fill`) are Java crash pages: exit 0, `An error has occured : java.lang.NullPointerException`, `HColorSet#parseColor` ← `SvgNanoParser#drawText`. `mk4_svg_text_fill_apostrophe_seq` (with `fill`) is valid.
- `93bf53e18` rejected the input only because Rust-only `strip_inline_comment` cut the SVG.
- All three builds accept both no-fill cases.

**Correction for 🎯T14.32:** its context says Java renders some no-fill probes. By raw SVG plus stderr, it renders neither; both are crash pages with exit 0.

**Other lost rejections:** none in any cohort, against either before. The unchanged `rejection_contract.py` gives:

| Set | Entries | Rejected (`93bf53e18` → after) | Rejected (`044f73657` → after) | Introduced losses (vs `93bf53e18` / vs `044f73657`) |
|---|---:|---|---|---|
| fresh | 313 | 269 → 304 | 304 → 304 | none / none |
| AX | 8 | 2 → 1 | 1 → 1 | `ck3_svg_inline_quoted_apostrophe_seq` / none |
| AV | 3 | 0 → 3 | 3 → 3 | none / none |
| AT | 3 | 2 → 3 | 3 → 3 | none / none |
| AY | 4 | 0 → 1 | 1 → 1 | none / none |

In AY, Rust still accepts `mk4_pixel_row_triple_spaced_component` and both no-fill probes in all builds. Java crashes on all three: `/''' row '''/` is not removed, so the pixel row ends with `level=-1`.

## Counts (after = `c9e11493c`; 4 modes; unchanged `compare_cli`, sha256 `5f6e6002…`, rebuilt from `d1424469…`)

| Cohort | n | vs `93bf53e18` | vs `044f73657` |
|---|---:|---|---|
| AY (mk4, held out) | 48 | **1 lost match** (`mk4_tim_sprite_in_procedure`); 21 gains, 1 newly rejected, 2 valid-error→mismatch | 0 losses; 22 gains, 2 valid-error→mismatch |
| AZ (mk4b) | 8 | 0 losses; 3 gains | 0 losses; 3 gains, 1 valid-error→mismatch |
| AX (ck3) | 77 | **1 lost rejection** (round 3, above); 18 gains, 1 valid-error→mismatch | 0 losses; 5 gains, 3 valid-error→mismatch |
| AV | 63 | 0; 10 gains, 3 newly rejected | no outcome or SVG change |
| AT | 45 | 0; 7 gains, 1 newly rejected | no change |
| AS/AQ/790/28 | 915 | 0; 59 gains, 35 newly rejected | no change |
| retained | 4,174 | 0; 20 gains, 2 newly rejected (29 SVG byte changes) | 0; 1 gain (`checker_inner_comment`), 2 SVG byte changes |
| goldens | 12,550 | 0; 1 newly rejected (2 SVG byte changes) | 0; **0 SVG byte changes**; 7 DOT AST/YAML changes, all `order-only` |
| ck4 probes (checker) | 64 | Java pending | Rust-only, below |

- Panics: `--yaml` panics on the same 148 golden and 63 retained sources in all three builds, and on nothing else.
- Timeouts: 0.
- New valid-input errors among Java-referenced cases: 0 against either before.

## Port exactness (challenge 1)

`remove_inner_comment` and `remove_special_inner_comment` match `StringLocated#removeInnerComment` and `#removeSpecialInnerComment` (StringLocated.java:255–282):
- Java trim (characters ≤ U+0020);
- leading: `indexOf("'/")` on the untrimmed string; trailing: `lastIndexOf("/'")`;
- the special regex with ASCII `\w`, also applied to the leading and trailing results;
- no quote handling.

The removal sits after the long-comment and `'` stages and before every later stage:
- `!procedure`, `!function` and `!definelong` bodies are replayed through `process_one_line`, so their lines are stripped again, as Java's `executeLines` → `buildCodeIterator` does.
- An in-place `!startsub` body is stripped once.
- `!includesub` is stripped twice.

Residue that predates the packet, belonging to 🎯T14.31 (the classification code is unchanged):
- a block comment ends on any line containing `'/`; Java ends it at a line whose trimmed text ends with `'/`;
- a `/'…/'` line opens a block;
- classification uses Rust's Unicode `trim`, where Java's `TLineType` uses `[\s ]` (`Pattern2` `%s`).

**Backslash continuation (🎯T14.22).** Java merges `\` lines before TIM: `preproc2.Preprocessor` → `ReadFilterMergeLines`, which reads the next line through `ReadFilterQuoteComment` and so calls `removeInnerComment`. Rust has no such merge in any build (`A -> B : one \` / `two` errors in all three). The packet changes two sequence probes from accepted to error:
- `ck4_seq_continuation_comment_only_then_msg` (`/' c '/ \` + `A -> B : hi`): Java should merge this to ` A -> B : hi`. The line is now a lone `\` that no command accepts.
- `ck4_seq_continuation_next_leading`.

The source trace predicts both are Java-valid. **They are not counted until the ck4 references return.**

## Cross-family probes (challenge 2): batch ck4, Java pending

`request-ck4/` holds 64 cases (60 intended valid) and 3 resources:
- sequence, class, usecase, component, state, activity (legacy and 3), object, YAML, JSON, mindmap, WBS, timing, nwdiag, EBNF, gantt and salt;
- quoted labels, URLs, creole, notes, CRLF, `'/` inside strings, comment-only lines, `\` continuation;
- `!define`, `!procedure`, `!function`, `!definelong`, `!while`, `!foreach`, `!include`, `!startsub` and `!includesub` bodies, including double-pass spans;
- procedure source-line controls.

Rust-only (`probes/ck4-rust-only.json`), `044f73657` to `c9e11493c`:
- 44 outputs change.
- 6 go from accepted to error: the two continuation cases above, plus `ck4_seq_span_in_quotes_trailing`, `ck4_seq_double_leading_top`, `ck4_inv_nbsp_leading_comment_seq` and `ck4_inv_class_after_leading_two`. The last three are predicted Java-invalid.
- 5 go from error to accepted.
- 0 panics.

| File | sha256 |
|---|---|
| `~/Library/Caches/rustuml-sprite-round4-claude-checker/ck4/reference-manifest.json` | `cc33567828a33e804c7d4c273d46ba65058bb9e095f72b0343a9ad240ad0f2c0` |
| `~/Library/Caches/rustuml-sprite-round4-claude-checker/ck4/reference-input-bundle.json.gz` | `461cfd5f1fce04a5f01c0cfa3406690be474e15ee9a7cf7f1f6ac5cde3dc2d7d` |

## Theme seed source (challenge 3)

On AZ, the three cases with a comment on a plain line now match: trailing, `/'''w'''/`, and a comment before `!theme`. The seed hashes the stripped text, as Java's `BlockUml#data` (`TimLoader#getResultList`) does.

Residue present in all builds:
- `mk4b_theme_whole_line_comment_class` and `mk4b_theme_block_comment_class` have the same Java seed (`g1x50mafwu8zhk`), because Java's data omits comment lines. Rust's seed source keeps them.
- `mk4b_theme_leading_comment_component` mismatches because Rust draws no cerulean component gradient even without a comment.
- `mk4b_theme_trailing_comment_usecase` hits the usecase-detection gap (`:Clerk: --> (Pay)` errors without a comment too).

## `checker_comments_and_origins` (challenge 4)

`/' before '/ !theme cerulean` is now PLAIN, and the theme applies, as in the Java golden (cerulean paint). Differing elements go from 19 to 12.

The after SVG equals the output for the comment-free source, modulo gradient ids. That control renders identically in all three builds. So the remaining `data-source-line` offset after `!theme` + `!if` and the gradient seed predate the packet.

## Process (challenge 5)

- **Account first.** `b62315702` (18:25:15) precedes `df1c3f096` (18:26:32).
- **No fixture-keyed logic** in the diff.
- **Frozen items unchanged.** `93bf53e18..c9e11493c` modifies no file under `crates/rustuml-oracle`, `tests` or `test-diagrams` outside `perturbations/`, and `perturbations/` only gains files. That covers comparator slack, syntax markers, park list, error census, guard allow-lists, the strict-XML list and the approved valid-error set.
- **Evidence layout.** The maker's evidence follows the T14.6.7 layout.
- **Maker numbers.** Every round-4 number the maker reports reproduces here.

## Gates (worktree at `c9e11493c`, `CARGO_TARGET_DIR` private)

- **`golden_no_oracle`:** 70 failure names, identical to `9834a7194`. The test still fails with the same "progress not locked in" message as rounds 1–3.
- **`golden_pairs`:** 754 names, identical to `9834a7194`.
- **`parity_perturbations`:**
  - before registering: 2,499 failures; 0 new against the round-3 post list, 8 fixed; identical to the maker's after list;
  - after registering the 52 AY/AZ pairs (`gates/parity_perturbations-post.log.gz`): 2,514; the 15 new failures are exactly the pairs the CLI fails (harness = CLI), and no earlier pair changed.
- **`harness_guards`:** the same 2 failing guards (`decimal_literal_ratchet`, identical to rounds 3 and 4; `parity_review_records_are_complete`). Before this review, the 2,471 normalised violation lines were identical to round 3 and the maker. `gates/guard-violations-post.txt` is the run with `review-3.json` in place.
- **Workspace lib:** 1,501 / 1,501. **`pipe_mode`:** 13 / 13.

## Next obligations (bounded)

1. **Procedure source lines.** A line after a top-level procedure call keeps its own Java location. Port this from `StringLocated` locations through `TContext#executeProcedure` / `executeLines`. Acceptance: `mk4_tim_sprite_in_procedure`, `bd087_macro_valid`, `c75_sprite_procedure_legacy` and the ck4 controls match, with no pass lost. No target exists; the parent should file one.
2. **The lost rejection.** `ck3_svg_inline_quoted_apostrophe_seq` (round 3) belongs to 🎯T14.32's no-fill `<text>` crash. Whether it can stand until then is the parent's or owner's call.
3. **ck4.** The parent relays the ck4 references. Then classify them with `fresh_classify.py`, and decide whether the continuation accept→error transitions are new valid-input errors (🎯T14.22 interaction) or acceptable.
4. **🎯T14.31.** Add the Unicode-`trim` versus `[\s ]` classification gap.

## Files

- `summary.json`: per-cohort outcomes, change lists, panics and timeouts.
- `runs/`: all replays.
- `classified/`, `rejection/`: classifications and rejection-contract output.
- `controls.json`: attribution controls.
- `gates/`: failure lists and logs.
- `registered-pairs.json`, `evidence-only-cases.json`.
- `fresh-*-manifest*`: per-case Java validity.
- `request-ck4/`, `probes/`.
- `support/`: scripts. `replay.py`, `classify.py`, `fresh_classify.py`, `prepare.py`, `rejection_contract.py` and `model_changes.py` are the round-3 copies, byte-identical (sha256 in `../review-3.json`).
