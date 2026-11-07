# TIM line continuation (🎯T14.22) round 4: maker record

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/tim-continuation-round4-claude`. This round carries the whole sprite/participant line. It is not self-certified: an independent checker decides. Java was never run locally.

## Commits

| Commit | Role |
|---|---|
| `1f6ffa9f4` | Merge of `parity-recovery-2026-11-03` (`f70f107b8`) into the round-3 review `6ff3ce77e`. This is the merged base ("before") for every number below. |
| `9ee6c154c` | Pre-implementation account (`account.json`). |
| `874a3b3f0` | Addendum 1: an unshipped Java-bundled theme counts as a non-empty span (written during the draft, before any measurement). |
| `3ae71ad99` | Draft: obligations 1-4. Measured in full (`results/summary-draft-3ae71ad99.json`). |
| `96954ed40` | Addendum 2, written after measuring the draft: the block's `@end` test uses `StartUtils#isEndDirective`. |
| `9ed52f435` | Final code: the addendum-2 change. Every number below is for this tip unless marked as the draft's. |

### The merge

parity-recovery replaced the two-pass TIM with one evaluated-event pass (`Plain`, `Boundary`, `ParserOnly`, `Theme`), projected once into Java's `UmlSource` text (identity) and once into the parser stream. The merge keeps that pass as the base and ports the round-3 mechanisms onto it:
- `ReadFilterMergeLines` and the Pipe end rule run before TIM (`process_merged_lines`).
- `\end…` block ends and `block_end`.
- The post-TIM `UmlSource#loadInternal` join runs over the parser projection. `ParserOnly` lines and the theme marker are `TimLine::RustOnly`, which the join passes over, as the round-3 stand-ins were.
- `DiagramDetector` extraction for local includes, with numbered lines through the include-origin path.
- Sprite declarations stay diagram commands. parity-recovery's post-TIM `extract_sprites` lowering and `PreprocessOutput::sprites` are dropped. parity-recovery also removed the Rust-only trailing-apostrophe stripping, so the sprite comment exemption is dead code and is removed.
- The round-2 `removeInnerComment` port replaces parity-recovery's shorter one. parity-recovery's `CodeIteratorSub` port is kept.
- In `parse_with_base`, the TIM fatal error comes before the Welcome rule (`BlockUml` builds `PSystemErrorPreprocessor` first).

`bullseye.yaml` merged cleanly (a union; 161 targets).

## Obligations: Java trace and status

Full traces are in `account.json` and the addenda.

1. **The two-line count follows Java's list.**
   - **Java:** `TLineType` has no pragma type (`text/TLineType.java:259` returns PLAIN). `TContext#addPlain` (`tim/TContext.java:456-470`) therefore puts `!pragma` in the result list. `UmlSource#loadInternal` (`core/UmlSource.java:118-134`) does the join; `getTotalLineCount` (198-200) is the count, and `PSystemWelcomeFactory.java:52` fires on 2.
   - **Rust:**
     - An active `!pragma` becomes `EvaluatedEvent::Executed`. The identity projection and the join carry its substituted text; the parser sees a stand-in unless the join folds the pragma into a line.
     - The count is taken from the events, from the start `Boundary` through the join (`java_list_lines`, `uml_source_line_count`), so Rust-only `ParserOnly` lines are never counted.
     - An unshipped Java-bundled theme (`JavaOnlyLines`, addendum 1) counts as a non-empty span.
     - Count 2 gives the Welcome page whether or not there is a continuation.
   - **Acceptance:** CA `cr3_dir_pragma_start` and `cr3_dir_teoz_pragma_start` match again; `_middle` and `_end` are also gains. Java Welcome pages are reproduced everywhere they were: BZ 5/5, BY 5/5, CA 16/19. CA gains `cr3_two_nojoin_option_handwritten` and `cr3_end_at_in_block_comment`. The 3 CA Welcome pages still not reproduced (`cr3_end_bs_in_block_comment`, `cr3_end_bs_in_if0`, `cr3_end_at_in_if0`) were not reproduced at the merged base either; they belong to T14.66.

2. **One-line sources.**
   - **Java:** `UmlSource#isEmpty` (`core/UmlSource.java:214-229`) holds for the start line alone, so `PSystemCommandFactory.java:85-89` builds `Empty description` (`PSystemAbstractFactory.java:53, 60-63`) and Java exits 200. `CodeIteratorProcedure#peek` and `CodeIteratorReturnFunction#peek` (71-80 in each) take an end line inside an unterminated definition as body text.
   - **Rust:**
     - Count 1 returns ParseError `Empty description` at the start line.
     - An end directive inside an unterminated `!procedure`, `!function` or `!definelong` ends the block but yields no `Boundary`.
   - **Acceptance:** `cr3_end_bs_in_procdef` is rejected again. `cr3_two_one_line_after_join` and `cr3_two_zero_content_end_continued` are newly rejected, as Java does (exit 200). `cr3_dir_startsub_start` and `cr3_dir_startsub_middle` are also newly rejected (Java exit 200), because parity-recovery's empty-sub iterator stop leaves a one-line list.

3. **Java whitespace.**
   - **Java:** JDK 21 `Character#isWhitespace` is used by `StartUtils#startsWithDirectiveKeyword` (`utils/StartUtils.java:86-108`).
   - **Rust:** `java_is_whitespace` is the explicit set U+0009–U+000D, U+001C–U+0020, U+1680, U+2000–U+2006, U+2008–U+200A, U+2028, U+2029, U+205F, U+3000. It no longer includes U+0085 and now includes U+001C–U+001F. Addendum 2: inside the block, the `@end` test is `is_end_directive` on the block line too. Rust's `trim` stripped U+0085 and the no-break spaces, and it ran after `removeInnerComment`. A corpus scan found no input affected other than my probe.
   - **Acceptance:** `cr3_end_nel_before_bs_end` is rejected again, and `cr3_end_us_before_bs_end` matches.

4. **Wrapped-include fidelity.**
   - **Java:**
     - `ReadLineReader#readLine` (`preproc/ReadLineReader.java:89-103`) removes one leading U+FEFF and replaces U+2013 with `-`. It reads every local include, both through `DiagramDetector` and raw (`tim/TContext.java:843, 850`), and every `!includesub` file (660).
     - `StartUtils#isWordOrTilde` (82-84) uses `Character#isLetterOrDigit` on UTF-16 units.
     - `TContext#executeInclude` (793-802) cuts the text after the last `!`.
   - **Rust:** `read_line_reader_lines`; `java_is_letter_or_digit` (L* from `unicode-properties`, Nd from the existing JDK 21 table, BMP units only); and the suffix cut on local include paths.
   - **Acceptance:** `cr3_wrap_bom_hash_header`, `cr3_wrap_bom_inner_line`, `cr3_raw_bom_include`, `cr3_wrap_superscript_header`, and the `!N` cases `cr3_wrap_three_blocks_bang2` and `cr3_wrap_ids_bang_beta` all match.

## Measurement

**Method.** Unchanged `replay.py`, `classify.py`, `prepare.py`, `rejection_contract.py` and `verify_fresh.py`; their hashes equal the round-3 checker's (`results/unchanged-scripts.sha256`). `compare_cli` comes from unchanged source, linked against `93bf53e18`'s `rustuml_oracle`. Every build is a fresh `git archive` release build with its own `CARGO_TARGET_DIR` (`results/binaries.sha256`). Comparison is whole-output in 4 modes, each case run from its recorded cwd. Every prepared cohort was re-verified against the relay results: 0 bad (`results/verify-fresh.json`).

**Outcomes.** `results/summary.json` uses `summarize_r4.py`: the unchanged `classify.outcome`, the validity adjustment, and `welcome-match` when Java's Welcome page and Rust's Welcome page coincide (T14.63).

### Tip `9ed52f435` against merged base `1f6ffa9f4`

| Cohort | n | Lost matches | Lost rejections | New valid-input errors | New panics | Gains | Newly rejected | SVG byte changes |
|---|---|---|---|---|---|---|---|---|
| CA | 88 | 0 | 0 | 0 | 0 | 13 | 6 | 20 |
| BZ | 23 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| BY | 53 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| BW, BU, BD, BA, AY, AZ, AX, AV, AT | 401 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| AS+AQ+790+28 | 915 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| retained | 4,174 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| golden | 12,550 | 0 | 0 | 0 | 0 | 1 | 0 | 1 |

**Gains.**
- **CA (13):** `cr3_dir_pragma_start`, `_middle`, `_end`, `cr3_dir_teoz_pragma_start`, `cr3_two_nojoin_option_handwritten` (Welcome), `cr3_end_at_in_block_comment` (Welcome), `cr3_end_us_before_bs_end`, `cr3_wrap_three_blocks_bang2`, `cr3_wrap_ids_bang_beta`, `cr3_wrap_superscript_header`, `cr3_wrap_bom_hash_header`, `cr3_wrap_bom_inner_line`, `cr3_raw_bom_include`.
- **CA newly rejected (6, all Java exit 200):** `cr3_dir_startsub_start`, `cr3_dir_startsub_middle`, `cr3_two_one_line_after_join`, `cr3_two_zero_content_end_continued`, `cr3_end_bs_in_procdef`, `cr3_end_nel_before_bs_end`.
- **Golden:** `edge-cases/edge_pragma_useVerticalIf`.

**Panics.** Only the unchanged `--yaml` set panics: 63 in retained and 148 in golden, the same at base and tip. There are 0 timeouts.

**Model-mode differences.** All of them are AST or YAML; ASCII has none outside CA.
- **Expected:** `meta.source` now holds the `!pragma` line, as Java's `UmlSource` does. This covers the six `c75_*_combined_teoz` cases (fresh), `title_e0_q0`/`q1` (retained) and seven `preprocessing/preproc_pragma_*`/`edge_pragma_*` goldens.
- **Nondeterministic:** the six `dot/*` goldens, which give 4–5 distinct AST hashes in 6 runs at both builds.

**CA at the tip.**
- 49 matches and 16 Welcome matches; 58 cases are Java-valid and 19 are Java Welcome.
- **6 mismatches:**
  - `cr3_dir_theme_*` ×3 (T14.62).
  - `cr3_two_nojoin_pragma`: Java renders an empty CLASS diagram; Rust's empty-source fallback gives Welcome at every build. See the proposed target.
  - `cr3_wrap_vars_and_procedure` (T14.53).
  - `cr3_raw_include_mid_end` (T14.66).
- **3 valid-input errors:** `cr3_end_tim_chr64_enduml` and `cr3_end_tim_chr92_enduml` (T14.66), and `cr3_wrap_first_is_mindmap` (T14.21). All three existed at the base.

The draft `3ae71ad99` gave identical outcome counts in every cohort (`results/summary-draft-3ae71ad99.json`).

### Rejection contract (`rejection_contract.py`, base `1f6ffa9f4` → tip)

There are 0 introduced losses in every cohort. Rejection counts:

| Cohort | Before | After |
|---|---|---|
| CA | 4/30 | 10/30 |
| BZ | 4/9 | 4/9 |
| BY | 11/17 | 11/17 |
| BW | 3/3 | 3/3 |
| BU | 2/3 | 2/3 |
| BD | 2/4 | 2/4 |
| BA | 4/7 | 4/7 |
| AY | 1/4 | 1/4 |
| AZ | 0/0 | 0/0 |
| AX | 2/8 | 2/8 |
| AV | 3/3 | 3/3 |
| AT | 3/3 | 3/3 |
| 915 | 304/313 | 304/313 |

The contract still lists after-build false accepts. Separated (`results/rejection/summary.json`):
- **Java Welcome pages reproduced:** CA 16, BZ 5 and BY 5. These are matches under the T14.63 rule.
- **Other Java-invalid inputs Rust accepts:** CA 4, BY 1, BU 1, BD 2, BA 3, AY 3, AX 6 and the 915 9. None of them is new against the base.

### Gates (`results/gates/summary.json`)

- **Workspace lib:** 1,660 passed, 0 failed.
- **`pipe_mode`:** 16/17. `help_agent_discloses_no_oracle_product_metric` fails because `main.rs` prints 11,175 against the locked 11,181. Both files are identical to parity-recovery, so this failure is inherited.
- **`golden_no_oracle`:** 11,181/11,251 at the base, where the locked baseline passes; 11,182/11,251 at the tip. The tip's failure-name list is the base's minus `edge-cases/edge_pragma_useVerticalIf`. The suite fails only for that unlocked +1. The baseline is frozen, so it is left to the parent.
- **`golden_pairs`:** 10,497/754/1,299 at the base, 10,498/753/1,299 at the tip. The only name difference is the same golden.
- **`parity_perturbations`:** 3,895 failures at the base, 3,890 at the tip. 0 are new; 5 are fixed (the registered CA pragma ×4 and `us_before_bs_end` pairs).
- **`harness_guards`:**
  - 4 guards pass: comparator constants, error census, no golden data, path traversal.
  - `decimal_literal_ratchet` fails identically at the base, the tip and parity-recovery `f70f107b8` (no render file changed).
  - `parity_review_records_are_complete` has identical violation lists at base and tip (2,967 normalised lines, same sha256).

**Frozen items are unchanged:** comparator slack, syntax markers, park list, error census, guard allow-lists, the strict-XML list, the 15-source approved set, and the no-oracle and decimal baselines. Nothing under `crates/rustuml-oracle`, `crates/rustuml/tests`, `scripts` or `.gitmodules` differs from parity-recovery.

## Pending relay request: batch MR4 (39 cases, 10 resources)

The batch is staged, immutable, at `~/Library/Caches/rustuml-tim-continuation-round4-claude-maker/relay-mr4/`, and copied here in `pending-request-mr4/`. Rust predictions were registered at all four builds before any Java result existed (`mr4-rust-predictions.json`). My Java predictions are in `cases-with-java-predictions.json`: 25 valid, 6 Welcome, 4 Empty description, 4 errors.

| File | sha256 |
|---|---|
| `reference-manifest.json` | `aae9db814ccd39b26135617f129c8e27a414f882332af0044b716caad00537ea` |
| `reference-input-bundle.json.gz` | `3e3ba0dec5e5b0bcd86e60d48a5046c57c89322e071ae31f2ede53dc1b1f6fc8` |
| `mr4-rust-predictions.json` | `338a8a57132bce2a7f836fc3e3fb191779e8108d9f944e366e5daab496efb9a8` |
| `cases-with-java-predictions.json` | `4ce08d5c39fc3b99ce44a120635f1c791d1e42f61f7a8cc9ba31aed6d89575a4` |

The probes cover:
- **Pragma:** start; two pragmas; middle join into pragma text; inactive branch; from a procedure; with a variable.
- **Rust-only lines:** `!option handwritten` and `!dump_memory`, with and without a tail.
- **Themes:** an unshipped and a shipped theme before a tail.
- **End lines in definitions:** `@enduml`/`\enduml` in `!procedure`, `!function` and `!definelong`; content before a definition-swallowed end (Java Welcome); a continued end after a directive; a terminated-definition control.
- **Whitespace:** U+001C, U+001D, U+001E, U+3000, U+2028, U+2001, U+200B, U+2007 and U+180E before `\enduml`; U+001D and U+0085 before `@enduml`.
- **Includes:** BOM on every line; BOM before a directive; en dash raw and wrapped; header chars U+00BD, U+2160 and U+1D400; `!1` and `!x` suffixes; BOM before `!startsub`.

## Residue and proposed targets (not added to the ledger)

- **Empty-source fallback (new).** Rust's Welcome fallback for an empty `@startuml` source should follow `PSystemWelcomeFactory`'s count and `UmlSource#isEmpty`. A count-3 source such as `@startuml` / `!pragma layout smetana` / `@enduml` is an empty CLASS diagram in Java; Rust's empty fallback shows Welcome (CA `cr3_two_nojoin_pragma`, at every build).
- **Main source reader (new).** The main source should be read as `ReadLineReader#readLine` reads it: `BlockUmlBuilder` (`BlockUmlBuilder.java:91`) strips a leading U+FEFF from every line and maps U+2013 to `-`. Rust now does this for local includes only.
- **Extend T14.66.** An end line swallowed by an open `!startsub` collection or a `!foreach`/`!while` body: the same `CodeIterator` family as the definitions handled here.
- **Extend T14.61.** Stdlib and URL includes (and themes) use `ReadLineReader#readLine` and the `!N` suffix cut as well.
- **Declared residue.** `unicode-properties` carries Unicode 17 general categories, while JDK 21 uses Unicode 15.0. Letters added after 15.0 count as word characters in Rust's header check only.
- **Declared residue.** Quoted include paths. Java's `PathSystem` keeps the quotes. Rust strips the suffix inside the quotes and opens the file.

## Disk

Cache `~/Library/Caches/rustuml-tim-continuation-round4-claude-maker`. Every target and export was deleted once its binary was saved. Free space is reported in the maker's final message.
