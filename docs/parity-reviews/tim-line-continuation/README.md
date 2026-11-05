# TIM backslash line continuation (🎯T14.22): maker record, round 1

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/tim-continuation-claude`. This packet sits on the sprite packet line (round-4 tip `c9e11493c`, review addendum `725814981`), so it is also sprite round 5. An independent checker must review it; nothing here is a completion claim.

- Account (committed before the draft): `account.json`, commit `ef52d5b32`; schema fields completed in `8bd933bc3` (no change of substance).
- Implementation: `594c2b3c5` (`crates/rustuml-parser/src/preprocess/mod.rs`, `crates/rustuml-parser/src/parse/mod.rs`).
- Before trees: merged base `2a9373aca` (code identical to `c9e11493c`) and pre-packet `93bf53e18`.
- Java: `71806a23780b04a5ccde2f8ceb5121edad5eb711` (read-only source; never run locally).

## Java trace

Java joins continued lines in two places.

**1. Before TIM, on the main source** (`BlockUmlBuilder#<init>` → `preproc2.Preprocessor` → `ReadFilterMergeLines`):
- A line continues when `StringUtils#endsWithBackslash` holds: it ends with `\` but not `\\`. An escaped `\\` is passed on unchanged, and so is `\` followed by a space.
- The next line is read through `ReadFilterQuoteComment`, which skips `'` lines, whole `/' … '/` lines and long `/' … '/` blocks, and returns the first other line with `StringLocated#removeInnerComment` applied.
- `StringLocated#mergeEndBackslash` drops the backslash and appends that line. The merged line keeps the **first** line's `LineLocation`. Continued lines chain.
- The continued (first) line is not comment-filtered. `/' c '/ \` + `A -> B : hi` becomes `/' c '/ A -> B : hi`, and TIM's it40 `removeInnerComment` later makes it ` A -> B : hi`.
- A start directive whose types contain DITAA turns the join off until the next end directive. Start and end directives are tested only on lines read as a new result, never on merged-in text.
- If no next line exists, the line keeps its backslash. If the next line is the end directive it is merged like any other line. `BlockUmlBuilder` then finds no end and builds no block. In `-pipe` mode `Pipe#readSingleDiagram` appends `@end<type>` when the input has none, so a continued last line always swallows the end, and `SourceStringReader#noValidStartFound` prints "No valid @start/@end found, please check the version".
- This join runs before every TIM stage, so before round 4's `removeInnerComment` stage, and it covers `!procedure`, `!define`, `!definelong`, `!foreach` and multiline command bodies written in the main source.
- `!include` files (`TContext#executeInclude`), `!includesub` and themes (`ThemeUtils`) are read without this filter.
- CRLF: `BufferedReader#readLine` ends lines at CRLF, so `\` before CRLF continues.

**2. After TIM, for `@startuml` only** (`PSystemBuilder#createPSystem` → `UmlSource.createWithRaw(source, findStartTypes(first).contains(SEQUENCE))` → `UmlSource#loadInternal`):
- Every TIM output line that `endsWithBackslash` is joined with the next one.
- The joined line takes the **last** piece's location.
- A pending tail at the end of the list is dropped.
- `UmlSource#seed` hashes the joined list, so SVG ids follow it.
- In practice this join catches continued lines that come from `!include` files.

## Rust change (`594c2b3c5`)

- **`merge_ending_backslash_lines`** is a port of `ReadFilterMergeLines`, `ReadFilterQuoteComment`, `endsWithBackslash`, the start/end directive tests and the ditaa switch.
  - It runs on the top-level block input, after `source_identity_input_lines`, for the parse pass, the source-identity pass and `expanded_theme_seed_source`.
  - Each merged line keeps its first piece's number. Once a merge absorbs lines, later output lines carry explicit source-line markers. This is the existing include mechanism.
- **`collapse_post_tim_ending_backslashes`** (`UmlSource#loadInternal`) runs on the parser stream for `@startuml` blocks, and the joined line takes the last piece's origin.
  - The existing seed collapse in `uml_source_string` is now keyed to the `@startuml` start. Before, it was keyed to the sequence-parse flag.
- **Swallowed end:** when the merge swallows the block's end directive, `parse_with_base` returns an error with Java's message, because Java builds no block. See residue item 1.
- **Plain lines for marker-blind parsers:** json, yaml, salt, nwdiag, regex, ditaa, dot, board, ebnf, git, math and latex parsers read no markers, so they get plain lines. Their parsers have no marker support; before this change they only received markers after an include or loop.
- The activity parser's own backslash join is unchanged. It now sees lines that are already merged.

## Results (after `594c2b3c5`; unchanged `compare_cli` and checker scripts, 4 modes, each case from its recorded cwd)

| Cohort | n | vs merged base `2a9373aca` | vs pre-packet `93bf53e18` |
|---|---:|---|---|
| BA (ck4) | 64 | **6 gains**, 0 losses | 37 gains, 4 newly rejected; 1 lost match (`ck4_procedure_sprite_end_comment_two`, T14.33) |
| AY (mk4) | 48 | no change | 21 gains; 1 lost match (`mk4_tim_sprite_in_procedure`, T14.33) |
| AZ (mk4b) | 8 | no change | 3 gains |
| AX (ck3) | 77 | no change | 18 gains; 1 lost rejection (`ck3_svg_inline_quoted_apostrophe_seq`, T14.32) |
| AV | 63 | no change | 10 gains, 3 newly rejected |
| AT | 45 | no change | 7 gains, 1 newly rejected |
| AS/AQ/790/28 | 915 | no change (0 SVG byte changes) | 59 gains, 35 newly rejected |
| retained | 4,174 | **2 gains** (the two exception-set sources), 2 SVG byte changes | 22 gains, 2 newly rejected |
| goldens | 12,550 | 0 outcome changes; 12 SVG byte changes (below) | 1 newly rejected; 14 SVG byte changes |

- **Against the merged base**, every cohort has zero lost matches, lost rejections, new valid-input errors, panics and timeouts.
- **The three losses against `93bf53e18`** are the round-4 sprite residues, already present at `c9e11493c` and unchanged here: two T14.33 cases and one T14.32 case. This packet does not touch them.
- **`--yaml` panics** are the same 148 golden and 63 retained sources in all three builds.
- **BA gains:** `ck4_seq_continuation_comment_only_then_msg` and `ck4_seq_continuation_next_leading` (valid-error → match), plus `ck4_class_comment_then_backslash`, `ck4_component_comment_then_backslash`, `ck4_state_comment_then_backslash` and `ck4_class_continuation_next_leading` (mismatch → match).

**Golden SVG byte changes** (all 12 against the merged base; all `nonpositive-accepted` before and after): `activity/act_color_{blue,cyan,green,lime,magenta,navy,orange,pink,purple,red,teal,yellow}_ending_output`.
- Source: `#red:Action with output\` + `stop`.
- Java's golden is a syntax error on the merged line, `#red:Action with outputstop` at line 4, which is the `ReadFilterMergeLines` text and the first line's location.
- Rust now receives exactly that line. It still drops it silently instead of rejecting it, which is the T14.17 silent-drop residue, so the diagram loses its `stop` node. That is the byte change. Before, Rust silently dropped the unjoined `#red:…\` line instead.

**Other model-only changes** (AST/YAML; SVG unchanged):
- `meta.source` is now Java's merged `UmlSource` text in `act_action_backslash_valid_multiline`, `act_ending_*`, `edge_preproc_macro_multiline_body` (`class Foo {   +method(): void }`, the text Java's error page shows).
- DOT and `sprite_multiple` AST/YAML differ in order only (HashMap order).

**Exception-set sources:** `tim-evaluated-event-stream/retained_checker_line_continuation.puml` and `tim-theme-post-expansion-source-identity/checker_line_continuation.puml` now **match** (valid-error → match). Both are `!theme cerulean` + `first\` / `second`, and both render `firstsecond`. The frozen exception-set file is not edited.

**rejection_contract.py** (unchanged), before → after rejections:

| Set | Entries | vs `2a9373aca` | vs `93bf53e18` | Introduced losses |
|---|---:|---|---|---|
| fresh | 313 | 304 → 304 | 269 → 304 | none |
| BA | 7 | 4 → 4 | 0 → 4 | none |
| AY | 4 | 1 → 1 | 0 → 1 | none |
| AX | 8 | 1 → 1 | 2 → 1 | `ck3_svg_inline_quoted_apostrophe_seq` vs `93bf53e18` only (T14.32, round 3) |
| AV | 3 | 3 → 3 | 0 → 3 | none |
| AT | 3 | 3 → 3 | 2 → 3 | none |

## Gates (worktree at `594c2b3c5`; `maker-results/gates/`)

- Workspace lib: 1,507 passed, 0 failed.
- `pipe_mode`: 13 passed.
- `golden_no_oracle`: 11,181/11,251.
  - The 70 failure names are identical to `9834a7194`.
  - The test fails, as at round 4, on the unlocked class, preprocessing and rendering baseline lines. The sprite packet's improvements are not ratcheted.
- `golden_pairs`: the 754 failure names are identical to `9834a7194`.
- `parity_perturbations`: 2,525 failures, against 2,533 at the round-4 addendum (post-BA). **0 new, 8 fixed.** The 8 are the six ck4 continuation pairs and the two exception-set sources.
- `harness_guards`: the same 2 failing guards (`decimal_literal_ratchet` with the same 5 render files, and `parity_review_records_are_complete`). In this environment the merged base gives 2,731 normalised violations. After `594c2b3c5` the only additions were this directory's 4 account-schema lines. After `8bd933bc3` the rerun (`harness_guards-final.log.gz`) gives 2,732: the base set plus "account has no independent checker review", which remains until a checker reviews.
- The comparator slack, syntax markers, park list, error census, guard allow-lists, strict-XML list and the 15-source approved valid-error set are unchanged. Nothing under `crates/rustuml-oracle`, `tests` or `test-diagrams` changed.

## Held-out probes: batch mc1, Java pending

There are 33 cases, 31 intended valid, plus 3 include resources. They cover:
- continuation in `!procedure`, `!define`, `!definelong` and `!foreach` bodies;
- `!theme plain`, `sketchy-outline` and `bluegray`;
- `!include` files, which exercise the post-TIM join, including a procedure defined in an include;
- comment and long-comment lines skipped before the continuation;
- three-piece joins and the following `data-source-line`;
- note and multiline action bodies, state, usecase and object;
- escaped `\\` and backslash followed by a space;
- CRLF (two cases);
- `\` before `@enduml` and at EOF (both intended invalid);
- TIM assignments and `!if` conditions;
- mindmap and json.

Rust-only results (`maker-results/probes/mc1-rust-only.json`): `594c2b3c5` changes 16 cases from error to accepted. It changes the two swallowed-end cases from accepted to error. There are 0 panics. `mc1_usecase_continued` errors in all builds, because of the pre-existing usecase detection.

| Request file | sha256 |
|---|---|
| `~/Library/Caches/rustuml-tim-continuation-claude-maker/mc1/reference-manifest.json` | `b2b8832aadd5132d264d492ae9c32947eaef49d63d4c98c4bd94f7a8bfc26dde` |
| `~/Library/Caches/rustuml-tim-continuation-claude-maker/mc1/reference-input-bundle.json.gz` | `4354dc3d669cd80492bcfff796f4dc3f499327245e20e20966f0f8ef1aa5f058` |

Copies are in `maker-results/pending-request-mc1/`. The held-out acceptance (at least 4 inputs, including a macro and a theme) is **not met until these references return**.

## Declared residue

1. **Swallowed end directive** (`mc1_seq_before_enduml`, `mc1_seq_at_eof`).
   - Java prints the "No valid @start/@end found" page, which contains no frozen syntax marker.
   - Rust now exits with an error. The harness may therefore classify these as Java-valid, which would make them new valid-input errors (`93bf53e18` and `2a9373aca` rendered a `last \` message).
   - Rust has no Java error-page renderer, so neither outcome can match. I chose the faithful outcome, no diagram, and flag it for the checker or owner.
2. **Rust placeholder lines.** The post-TIM join treats Rust's blank directive placeholders as lines. Java has no such lines, so an included line ending in `\` followed by a consumed directive joins with the blank line in Rust.
3. **Pending tail.** An included last line ending in `\` directly before `@enduml` is dropped from the parser stream in Rust. In Java it is joined with `@enduml`. Not probed.
4. **Theme seed numbering.** `expanded_theme_seed_source` still numbers raw lines, while theme expansions use post-filter numbers. This mismatch predates the packet and is untouched.
5. **Unchanged and out of scope:** T14.33 (post-procedure line numbers), T14.32 (sprite drawing and `ck3_svg_inline_quoted_apostrophe_seq`), T14.31 (block-comment classification) and the accepted participant mechanism.
