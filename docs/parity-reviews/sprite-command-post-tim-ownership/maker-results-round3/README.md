# Post-TIM sprite command ownership, round 3: maker results

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/sprite-participant-round3-claude`. This is a maker record, not a review; an independent checker must judge it. No push, no PR, no local Java.

- **Rejection answered:** `f803c8e33` (checker-claude on round-2 tip `5cf1eac12`).
- **Account of record:** `account.json` here (commit `9b2e4758f`, before every draft). It also amends the round-2 account (`../maker-results/account.json`, key `amendment_round3`): description and activity sprites are offered first, which is the Java order.
- **Numbers:** `summary.json`. Per-row outputs are in `runs/`, classifications in `classified/`, gate logs and failure lists in `gates/`, and scripts in `support/`.

## Commits

| Commit | Role |
|---|---|
| `dea00b295` | merge of parity-recovery `a20ae0f6d` (code unchanged since `9834a7194`) |
| `9b2e4758f` | round-3 accounts (both mechanisms), guard-schema `account.json` updates, round-2 amendment |
| `43067aa7c` | obligation 1: `parse/command_spans.rs`, a per-factory `getCandidate` replay that gates the sprite dispatch |
| `1dbdd02e8` | obligation 2: no trailing-apostrophe stripping on lines a sprite command owns |
| `f7270990e` | obligation 3: `!startsub` bodies replayed in place |
| `cde7b092c`, `b461817d4` | participant obligations 4 and 5 (see `../../sequence-participant-silent-residue/maker-results-round3/`) |
| `cd82c28dd` | clippy-only refactor; its SVG output is byte-identical to `b461817d4` on every cohort |

## Java trace, by obligation

1. **Body ownership.**
   - `PSystemCommandFactory#getCandidate` / `#isMultilineCommandOk` / `#addOneSingleLineManageEmbedded2`: a line inside a completed multiline block is never a command start.
   - `command_spans::walk` replays this rule over each factory's `initCommandsList`: `DescriptionDiagramFactory`, `StateDiagramFactory`, `ActivityDiagramFactory` and `ClassDiagramFactory` (class and object).
   - The registry holds, in order:
     - every multiline command (the `addTitleCommands` multilines, `CommandSkinParamMultilines`, the sprite group, `CommandStyleMultilinesCSS`, the note factories, `CommandCreateElementMultilines` TYPE0/TYPE1, map, json (with `#finalVerification`), archimate, `CommandCreateClassMultilines` with `#syntaxWithFinalBracket`, `CommandCreateEntityObjectMultilines`, the tip factory and `CommandLinkLongActivity`);
     - the single-line commands that can take one of their start lines first (the `addTitleCommands` singles, `CommandCreateElementFull` and `CommandCreateElementFull2`).
   - The other single-line commands are argued disjoint in the module.
2. **Comment exemption.**
   - Java has no trailing-apostrophe comment: `TContext#buildCodeIterator` uses `CodeIteratorShortComment` (whole line) and `StringLocated#removeInnerComment` (`/' '/` only).
   - The Rust-only `strip_inline_comment` now spares every line a sprite start grammar accepts, and every line of an open `CommandFactorySprite` or `CommandSpriteSvgMultiline` through its END. The END test is `Trim.BOTH` with `Matcher#matches`, tracked on active TIM lines.
3. **`!startsub`.**
   - `CodeIteratorSub#peek` stores the block, then replays it in place (`readingInProgress`) through the rest of TIM.
   - The Rust preprocessor now processes the collected lines in place. A `!theme` body still replays its subs into `theme_tail` (counter `theme_body_depth`).
   - This is bounded, so no separate target is needed.

## Results (after = `cd82c28dd`)

Losses are counted against both befores: lost matches, lost rejections, new valid-input errors, and panics or timeouts.

| Cohort | n | vs `93bf53e18` (pre-packet) | vs `dea00b29` (round 2) |
|---|---:|---|---|
| AT (checker, held out) | 45 | **0 losses**. 7 mismatch→match, 1 newly rejected | **0 losses**. 17 valid-error→mismatch, 4 valid-error→match, 5 mismatch→match, 3 newly rejected |
| AS | 54 | **0**. 9 gains, 15 newly rejected | **0**. Identical |
| AQ | 43 | **0**. 27 gains, 4 newly rejected | **0**. Identical |
| 790 | 790 | **0**. 15 gains, 12 newly rejected | **0**. Identical |
| 28 | 28 | **0**. 8 gains, 4 newly rejected | **0**. Identical |
| retained | 4,174 | **0**. 19 gains, 2 newly rejected; 27 SVG byte changes | **0**. 6 gains; 10 SVG byte changes |
| goldens | 12,550 | **0**. 1 newly rejected (`edge_render_sprite`, round 2); 2 SVG byte changes | **0**. 1 SVG byte change |

Panics and timeouts are 0 in `-tsvg`, `--ast` and `-ttxt`. `--yaml` panics on the same 148 golden and 63 retained sources in all three builds.

Remaining valid-input errors after the change were already errors in both befores:
- AT `ck_usecase_header_body`, AS `mk2_proc_uninvoked_svg_usecase` and AS `mk2_usecase_note_upper`: usecase sources routed to Sequence (🎯T14.21).
- AT `ck_ditaa_sprite_text` is compare-101 everywhere.

**Changes against round 2, all explained:**
- golden `preproc_include_startsub`: a Java error page, still accepted by Rust. The sub bodies now appear in place, as Java's echoed source shows.
- retained `checker_dynamic_sub_sprite` and `checker_subblock_state`: `!startsub` in place; they mismatch before and after.
- retained `sequence-local-attribute-serialization/bd087_*`: 8 changes, 6 of them new matches. The participant URL comes from `UrlBuilder#getUrl`, with `Url#<init>` quote stripping and the tooltip read correctly.
- golden DOT `--ast`/`--yaml` diffs are run-to-run `HashMap` ordering: the unchanged `dea00b29` binary gives three different AST hashes for `dot_edge_labels` in three runs.

**`rejection_contract.py` (unchanged):**
- 313 Java-invalid fresh inputs: 269 rejected before (`93bf53e18`), 304 after, 0 lost.
- Exit 1 on the same 9 known false accepts as round 2.
- AT Java-invalid inputs: 3/3 rejected after, against 2 before `93bf53e18` and 0 before `dea00b29`.

**Gates** (`gates/`; after = the worktree at `cd82c28dd`, before = `dea00b29` export):
- `golden_no_oracle`: 70 failure names, byte-identical to `9834a7194`.
- `golden_pairs`: 754 failure names, byte-identical to `9834a7194`.
- `parity_perturbations`: 2,452 → 2,437, 0 new, 15 fixed.
- `harness_guards`: the same 2 failing guards. The decimal ratchet is identical and the 2,757 review-record violation lines are identical.
- Workspace lib: 1,498/1,498. `pipe_mode`: 13/13.

**Frozen items:** only `crates/rustuml-parser` and `docs/` changed. The approved new-valid-errors set is unchanged (sha256 `c3076b2c…`).

## Residue (declared, not hidden)

- The walk models only the single-line commands that can take a multiline start line. `ColorParser` PART2's lookahead is written lookahead-free; it is equivalent except for TYPE0's colour glued to `as`. FoxSignature prefilters are skipped (vertical-tab separators only). JSON validity uses `serde_json`.
- The comment exemption is tracked on the streaming preprocessor, so a sprite opened outside a procedure body and closed inside it is stripped at definition. An unclosed sprite keeps the exemption to end of file; Java rejects that input anyway.
- Activity3 keeps its round-2 `at_command_start` gate. It is not in the AT failures and was not re-audited.

## Pending relay request

The held-out batch **mk3** has 63 cases (60 intended valid). It covers renamed and held-out variants of every AT failure shape: other families, owners, body texts, nesting, apostrophe and `!startsub` shapes, tooltip alternatives and re-declaration orders.

It is staged immutable at `/Users/marcelo/Library/Caches/rustuml-sprite-round3-claude-maker/mk3/`, with a copy in `pending-request/`:
- `reference-manifest.json`: sha256 `fdc4ec7562061a983befb650faf48e2f798c14ade95d0bde021539a911dbdd56`
- `reference-input-bundle.json.gz`: sha256 `15439c3385f1cfafcf65f220625a3f3d0778205940297d5818bcb219d5e84819`

Its results are not counted above.
