# Post-TIM sprite command ownership, round 4: maker results

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/sprite-round4-claude`. This is a maker record, not a review; an independent checker must judge it. No push, no PR, no local Java.

- **Rejection answered:** `6fd931656` (`../review-2.json`, checker-claude round 3). Participant residue was accepted there and is not touched.
- **Account of record:** `account.json` here, commit `b62315702`, before the draft. The guard projection `../account.json` was updated in the same commit.
- **Numbers:** `summary.json`. Runs in `runs/`, classifications in `classified/`, gate logs and lists in `gates/`, scripts in `support/`. The replay, classify, prepare and rejection scripts are the round-3 checker's, unchanged (`support/unchanged-checker-scripts.sha256`).

## Commits

| Commit | Role |
|---|---|
| `044f73657` | merge of parity-recovery `a0ed69740` (only `bullseye.yaml`; `crates/` identical to `607890d0d`). This is the **before** tree. |
| `b62315702` | round-4 account (Java trace, cross-family scope, precommitted measurement) |
| `df1c3f096` | `remove_inner_comment`: port of `StringLocated#removeInnerComment` at TIM it40 in `process_one_line` |
| `d322dec66` | the theme seed source (`UmlSource`) carries the same text. This is the **after** code. |
| this commit | evidence and the pending relay requests |

## Java trace

- `tim.TContext#buildCodeIterator` builds the same chain for the main source, every `!include` and every procedure, function or definelong body:
  it10 `CodeIteratorImpl` → it20 `CodeIteratorLongComment` → it30 `CodeIteratorShortComment` → it40 `CodeIteratorInnerComment` → it50 `CodeIteratorSub` → it60/61 functions and procedures → it70 `!if` → it80 legacy define → it90/100 `!while`/`!foreach` → it110 assignments.
- `TLineType#getFromLineInternal`:
  - `COMMENT_SIMPLE` is `^\s*'` or `^\s*/'.*'/\s*$`; `CodeIteratorShortComment` drops the line.
  - `COMMENT_LONG_START` is `^\s*/'` with no `'/` in the line; `CodeIteratorLongComment` skips through the line whose trimmed text ends with `'/`.
  - Any other line is PLAIN.
- `CodeIteratorInnerComment#peek` returns `removeInnerComment()` of every line that survives. `StringLocated#removeInnerComment`:
  - `trim = s.replace('\t',' ').trim()` (Java `String#trim`: characters ≤ U+0020);
  - **leading**: `trim` starts with `/'` → `s.substring(s.indexOf("'/") + 2)`;
  - **trailing**: `trim` ends with `'/` → `s.substring(0, s.lastIndexOf("/'"))`;
  - **special**: both `/'''` and `'''/` present → `replaceAll("/'''[-\\w]*'''/", "")`, where Java `\w` is `[A-Za-z0-9_]`;
  - the special replacement also runs on the leading and trailing results.
  - **Quotes:** none. A span inside double quotes is removed, and an apostrophe that is not part of `/'` or `'/` is never touched.
- `BlockUml#<init>` sets `data` to `TimLoader#getResultList`, so `UmlSource` and the SVG id seed hash the post-it40 text. Evidence: the pre-existing pair `tim-theme-post-expansion-source-identity/checker_inner_comment` has seed `g1hf47bqthl8fl`, the hash of `class BirchLedger ` (Rust gave the same id once the line was stripped).
- Out of scope: `preproc2.ReadFilterQuoteComment`, which also calls `removeInnerComment`, is used only by `ReadFilterMergeLines` (backslash continuation) and `ReadFilterAddConfig`.

## Rust change

- `preprocess/mod.rs::remove_inner_comment` and `remove_special_inner_comment` are a literal port: Java trim, `find`/`rfind` for `indexOf`/`lastIndexOf`, regex `/'''[-A-Za-z0-9_]*'''/`.
- In `process_one_line`, after the existing long-comment and whole-line `'` checks, the line is replaced by the result before every later stage: the sprite role and exemption, `strip_inline_comment`, `@start`/`@end`, definelong and sub collection, function definitions, `!while`/`!foreach`, directives and output.
- A `/'` line that contains `'/`, does not end with `'/`, and is not Rust's long-comment start (`ends_with("/'")`) is no longer dropped whole. Java classes it PLAIN, and its tail survives.
- `expanded_theme_seed_source` applies `remove_inner_comment` to the lines the comment stages pass. Comment lines keep their raw text, as before.
- Unchanged: block-comment termination, the whole-line `'` rule, the EBNF exemption, `strip_inline_comment`, and the round-3 sprite exemption.

## Results (before `044f73657`, after `d322dec66`, four modes, unchanged `compare_cli`)

| Cohort | n | Lost matches | Lost rejections | New valid-input errors | New panics | Gains | SVG byte changes |
|---|---:|---:|---:|---:|---:|---|---:|
| AX (ck3) | 77 | 0 | 0 | 0 | 0 | 5 valid-error→match, 3 valid-error→mismatch | 8 |
| AV (mk3) | 63 | 0 | 0 | 0 | 0 | 0 | 0 |
| AT | 45 | 0 | 0 | 0 | 0 | 0 | 0 |
| AS/AQ/790/28 | 915 | 0 | 0 | 0 | 0 | 0 | 0 |
| retained | 4,174 | 0 | 0 | 0 | 0 | 1 mismatch→match | 2 |
| goldens | 12,550 | 0 | 0 | 0 | 0 | 0 | 0 |

Panics and timeouts are 0 in `-tsvg`, `--ast` and `-ttxt`. `--yaml` panics on the same 148 golden and 63 retained sources in both builds.

### The seven AX acceptance cases

| Case | After | Explanation |
|---|---|---|
| `ck3_pixel_end_inner_comment_class` | **match** | `} /' end '/` → `} ` (trailing); END `^\}$` after Trim.BOTH |
| `ck3_svg_end_inner_comment_nospace_component` | **match** | `</svg>/'end'/` → `</svg>` |
| `ck3_svg_inline_inner_comment_object` | **match** | single-line SVG sprite: trailing span removed |
| `ck3_pixel_start_inner_comment_deployment` | **match** | `{ /' start '/` → `{ ` |
| `ck3_svg_end_inner_comment_state` | mismatch (was exit 1) | comment fully removed; the remaining diff is not comment handling (below) |
| `ck3_pixel_end_inner_comment_seq` | mismatch (was exit 1) | same |
| `ck3_svg_inline_quoted_apostrophe_seq` | still accepted | not a comment input (below) |

Also gained: `ck3_svg_end_inner_comment_usecase` now matches. `ck3_svg_end_inner_comment_teoz` goes from exit 1 to a mismatch for the same reason as `_seq`.

**`_state` and `_seq`.** With the comment, the after binary's SVG is byte-identical to its output for the same source without the comment, and that output is identical in both builds. Rust warns:
- `state sprite command CommandSpriteSvgMultiline is recognized; sprite materialization remains unsupported`;
- `sequence command CommandFactorySprite$1 is recognized but unimplemented`.

The `<$box>`/`<$dot>` reference is therefore not drawn. That is sprite materialization in state (SVG) and sequence (pixel), a separate mechanism. The relay requests the comment-free controls `mk4_ctl_svg_end_state` and `mk4_ctl_pixel_end_seq`, so Java can confirm that the comment does not change its output.

**`ck3_svg_inline_quoted_apostrophe_seq`.** Neither line contains `/'`, `'/` or `/'''`, so `removeInnerComment` does not change it.
- Java's page is a `NullPointerException`: `HColorSet#parseColor` (HColorSet.java:76), called from `HColorSet#getColorOrWhite` and `SvgNanoParser#drawText` (SvgNanoParser.java:302). The sprite's `<text>` has no `fill`, so `getFillString` returns null.
- `93bf53e18` rejected the input only because the Rust-only `strip_inline_comment` cut the SVG at ` ' s` and the sprite never completed. Restoring that would move away from Java.
- The faithful fix is porting the fill-less `<text>` crash in the render crate. That is outside `removeInnerComment`, so it is not done here; this acceptance item is **not met**.
- Requested to confirm the mechanism:
  - `mk4_svg_text_fill_apostrophe_seq`: the same input with `fill`, expected valid;
  - `mk4_svg_text_nofill_plain_seq` and `mk4_svg_text_nofill_usecase`: no apostrophe, no `fill`, expected invalid.

### Retained SVG byte changes (both Java-traced)

- `tim-theme-post-expansion-source-identity/checker_inner_comment`: mismatch→**match**. The seed source now holds `class BirchLedger `.
- `tim-evaluated-event-stream/checker_comments_and_origins`: mismatch→mismatch, 19→12 differing elements.
  - `/' before '/ !theme cerulean` is now PLAIN and the theme applies, as in Java. The element structure now matches Java.
  - The after output equals the output for a comment-free equivalent source, except for gradient ids. That comment-free source renders identically in both builds.
  - What remains is `data-source-line` after `!theme` + `!if` and the gradient seed. Both are the same in the comment-free control in both builds, so they predate this change.

Goldens: 0 SVG byte changes. The AST/YAML differences are run-to-run `HashMap` ordering on DOT goldens (`order-only`). AX/retained model changes are the `UmlSource` text losing the comment span, as in Java.

### `rejection_contract.py` (unchanged)

| Set | Entries | Rejected before | Rejected after | Introduced losses |
|---|---:|---:|---:|---:|
| fresh | 313 | 304 | 304 | 0 |
| AT | 3 | 3 | 3 | 0 |
| AV | 3 | 3 | 3 | 0 |
| AX | 8 | 1 | 1 | 0 |

The false accepts are the same as before: 9 fresh and 7 AX, including `ck3_svg_inline_quoted_apostrophe_seq`.

### Gates (after = the worktree at `d322dec66`)

- `golden_no_oracle`: 70 failure names, byte-identical to `9834a7194`. The "progress not locked in" message is identical to round 3.
- `golden_pairs`: 754 failure names, byte-identical to `9834a7194`.
- `parity_perturbations`: 2,507 → 2,499. **0 new failures**, 8 fixed (`summary.json`). The before list was run on the before tree with its own target dir, and it equals the round-3 checker's post list.
- `harness_guards`: the same 2 failing guards. The decimal ratchet is identical (no render file touched). The 2,471 normalised review-record violation lines are identical to round 3, and none is in this mechanism.
- Workspace lib: 1,501/1,501 (3 new preprocess tests). `pipe_mode`: 13/13.
- Clippy: the only `-D warnings` error is the pre-existing dead `looks_like_description_bracket_command` in `parse/component.rs`, which is untouched.

Frozen items unchanged: only `crates/rustuml-parser/src/preprocess/mod.rs` and `docs/` changed.

## Residue (declared)

- **Sprite materialization.** State SVG sprites and sequence pixel sprites are not drawn (`_state`, `_seq`, `_teoz`). Proposed as a target.
- **Fill-less SVG `<text>`.** `SvgNanoParser#drawText` crashes in Java on such a sprite (`ck3_svg_inline_quoted_apostrophe_seq`). Proposed as a target.
- **Rust long-comment rules.** These differ from `CodeIteratorLongComment`: any line containing `'/` ends a block, and a `/'` line ending with `/'` opens one. `mk4_leading_comment_ends_with_open` documents the second.
- **Theme seed source.** It keeps raw whole-line comment and directive lines (`mk4b_theme_whole_line_comment_class`, `mk4b_theme_block_comment_class`).
- **`ReadFilterQuoteComment`.** Not ported here; it belongs to the backslash-continuation reader.
- **`strip_inline_comment`.** The Rust-only trailing-apostrophe stripping on non-sprite lines is unchanged; Java has no such form.

## Pending relay requests (not counted above)

| Batch | Cases | `reference-manifest.json` sha256 | `reference-input-bundle.json.gz` sha256 |
|---|---:|---|---|
| `~/Library/Caches/rustuml-sprite-round4-claude-maker/mk4/` | 48 (45 intended valid) | `3b75bc0bcbba234a39371e29112bbef33879ceaf5538a6354458601791661766` | `0287d308512c9dc02b0e0fd0eb2274bbab704b9bd1120abab8f73f24261cc0ea` |
| `~/Library/Caches/rustuml-sprite-round4-claude-maker/mk4b/` | 8 (8 intended valid) | `d13369506982d52b849d8874c582f886ba9d55efc9ab7456b49aa4590dd175ce` | `52d4851f7cd3f669d9d22bad2b5121d45f32a1940bd934c2796806666999ceb1` |

Copies are in `pending-request/` and `pending-request-mk4b/`.

`runs/mk4-rust-only.json.gz` and `runs/mk4b-rust-only.json.gz` record the Rust before/after outputs. In mk4, 25 of 48 SVG outputs change and 9 go from exit 1 to exit 0; in mk4b, 5 of 8 change and 1 goes from exit 1 to exit 0. There are no panics.

mk4 covers:
- SVG body lines (leading, trailing, two spans, `/'''w-1'''/`, spaced `/''' '''/`);
- `/'''word'''/` in pixel rows and END lines;
- leading comments before sprite starts;
- non-sprite lines in sequence, class, object, component, state, usecase, activity3, mindmap, gantt, json, salt and skinparam;
- quote interaction;
- TIM directives and bodies;
- the controls above.

mk4b covers the theme seed source.
