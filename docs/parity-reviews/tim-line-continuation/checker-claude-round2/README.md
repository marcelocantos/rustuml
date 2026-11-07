# TIM line continuation (🎯T14.22) round 2: independent checker review

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-round2-claude`. Reviewed maker tip `e3aa1c6b4` (account `3573444dc`, code `f2d12d0ef`).

Records:
- `../review-2.json` (this mechanism).
- `../../sprite-command-post-tim-ownership/review-6.json` (whole-line addendum).

Earlier reviews are preserved. Java was never run locally.

**Verdict: rejected.**

## Pre-registration (commit `d0e5108a5`, before any Java result)

The held-out batch CR2 has 53 cases and 10 include resources. It was relayed as **BY**.

| File | sha256 |
|---|---|
| `reference-manifest.json` | `eadfd605b47bf8f1eb556f117654d983a9752b90264fa74864c643923df97b77` |
| `reference-input-bundle.json.gz` | `d7717be2ff88503dc15151586625a263c123a86735fe011232983fd2e3f35e84` |
| `pending-request-cr2/cr2-rust-predictions.json` | `8ffb84f505f4e4d08e9d81227f2bf90f27e283b7f9610d416e86eb0fb1da3b02` |
| `pending-request-cr2/cases-with-java-predictions.json` | `e0a83502c6963474d218516e1446a5c5fef809ad53caefadb198bf9f04d890f2` |

The Java results are in `request-by/` (relay `tim-continuation-cr2`; `reference-results.json` is `f00f906d…`).

Validity was decided from the raw Java SVG and stderr:
- `prepare.py` markers.
- `strict_validity.py`: "No valid @start/@end" pages and crash pages are invalid.
- `no_diagram_validity.py`: Java exit 50, which is `ExitStatus.ERROR_50_NO_FILE_FOUND` (empty stdout, no diagram), is invalid. This is the parent's rule.

That leaves 36 of the 53 Java-valid. My Java predictions were wrong on 7 cases:
- 5 Welcome pages (see cause 2).
- `cr2_nbsp_only`: Java renders an empty CLASS diagram.
- `cr2_seq_note_chr30_rsp`: Java exit-0 XML crash page.

## Causes of rejection

### 1. Placeholder collision: lost rejections caused by the packet

`f2d12d0ef` represents TIM-consumed directive lines in-band as the string `"\x1ERSP"` (`DIRECTIVE_PLACEHOLDER`, `preprocess/mod.rs:3833`). Every line equal to it is cleared (`:109`), and the post-TIM join passes over it (`:3859`). A user line can produce exactly that string:
- `%chr(30)RSP`
- a raw U+001E followed by `RSP`

At the tip such a line silently disappears.

| Case (BY) | Java | `b0fb43c3e` | `f2d12d0ef` |
|---|---|---|---|
| `cr2_seq_chr30_rsp_line` | exit 200, Syntax Error + XML `SAXException` page (invalid) | rejected | **accepted** (line dropped) |
| `cr2_seq_raw_rs_rsp_line` | exit 200, same | rejected | **accepted** |
| `cr2_seq_note_chr30_rsp` | exit 0 crash page (invalid) | note `RSP` drawn | note text silently dropped |

`rejection_contract.py` (unchanged) reports both as `introduced_losses` against the merged base. The fix belongs in this packet: the placeholder must be out of band, for example a typed line or a side table, not a reachable string value.

### 2. Residue 3 is only half ported: Java's two-line empty-diagram rule is missing

`UmlSource#loadInternal` joins a pending tail with the end line, and the tip ports that. Java then applies `PSystemCommandFactory#finalizeDiagram`: when `source.getTotalLineCount() == 2`, it returns `buildEmptyError` (PSystemCommandFactory.java:123). That is the Welcome page. So when the join absorbs the end and only `@startuml` plus one joined line remain, Java draws no message. Batch BY shows this five times:

| Case | Java | `b0fb43c3e` | `f2d12d0ef` |
|---|---|---|---|
| `cr2_seq_depth2_tail_then_end` | Welcome | Welcome | message `deep @enduml` |
| `cr2_seq_tail_then_indented_end` | Welcome | Welcome | message `tail   @enduml` |
| `cr2_seq_chr92_tail_then_end` | Welcome | Welcome | message `tail @enduml` |
| `cr2_seq_tail_then_backslash_end` | Welcome | message `tail \enduml` | message `tail \enduml` (`93bf53e18` rejected it) |
| `cr2_seq_end_line_continued_then_text` | Welcome (the `@enduml \` tail is dropped) | message `one` | message `one` (every build) |

For three cases the merged base gave the Java-equivalent outcome (its Welcome page). The packet replaces that with a rendered diagram.

The comparator classes every Welcome page as nonpositive, and Rust's Welcome page exits 0. So these three are not counted as lost rejections, but they are packet-caused losses of Java-equivalent behaviour. `cr2_seq_tail_then_backslash_end` is a lost rejection against `93bf53e18`; the rejection there was accidental (`\enduml` unsupported).

The tc1 include cases matched only because their joined sources have three or more lines.

### 3. New valid-input error vs the merged base (exposed, root cause T14.21)

`r2_component_include_tail_then_variable` (BW): Java renders one DESCRIPTION link with the label `wire [Rowan] --> [Ash] : n3`.
- **Merged base:** renders two links (mismatch). The tail was joined to the `!$n = 3` placeholder.
- **Tip:** correctly passes over the placeholder, as Java's TIM list has no line for `!$n`. The joined line is then misrouted to sequence and errors.

Control: the same joined line written directly in the source errors at `93bf53e18`, `b0fb43c3e` and the tip, and renders at `9834a7194`. Labels without an arrow route correctly (`cr2_component_include_tail_link` matches).

Under the whole-output rule, this is a new valid-input error against the merged base. The mechanism change behind it is right, so it needs either T14.21 or an owner exception.

## Per-mechanism verdicts

**`pipe_single_diagram`: faithful to Pipe.java 180–224 for end recognition.**
- `expectedEnd` comes from `matches()` on the whole line:
  - a named start (`cr2_seq_named_start_col0_endother_swallow`) or a trailing space ends at any column-0 `@end…`, matching Java's "No valid" (2 rejections, correct);
  - an exact start merges `@endmindmap` (`…exact…`, and its CRLF twin: 2 gains);
  - `@endumlx` prefix-matches (correct rejection).
- A nested `@start` merged into a continued line matches, and so does content after the end.
- BW gains: 8 of 9 Pipe probes now match.
- Unported pieces, all pre-existing T14.26:
  - Pipe's prepend: `r2_seq_indented_start_and_end`.
  - The discard of lines before the start: `cr2_seq_pre_start_backslash_text` errors at `93bf53e18`, the merged base and the tip; `cr2_seq_pre_start_blank_then_continued` is shifted by two lines per blank line at every build.
  - `@@@format` lines.
- **Empty or whitespace-only stdin** (`cr2_empty_zero_bytes`, newlines, spaces/tab, CRLF, VT/FF, and the retained `checker_092_empty`): Java exits 50 with empty stdout and stderr, because `readFirstDiagram` returns null and the loop never runs. The tip exits 1 with empty stdout and stderr `No valid @start/@end found`. Every earlier build exits 0 with a 4,554-byte SVG.
  - The rejection is correct: no diagram, nonzero exit, identical (empty) stdout.
  - Exit code (1 vs 50) and stderr differ; this is not whole-output identity.
  - There are 6 such newly correct rejections.
- **NBSP-only input:** Java renders an empty 16×16 CLASS diagram. Rust gives the Welcome page at every build (pre-existing whitespace classification).

**`split_blocks`: no regression.**
- 0 SVG byte changes on all 12,550 goldens and on the retained 4,174 (apart from the empty-input case above).
- The ast/yaml differences on 12 `dot/*` goldens and `sprites/sprite_multiple` are run-to-run nondeterminism at both builds (same set of output hashes).
- A nested `@start` in a continued line matches Java.
- A plain nested `@start` mismatches at every build: Java's `BlockUmlBuilder` resets `current` at a nested start, while Rust counts depth. Pre-existing; proposed as a target.

**Include-tail join: rejected.**
- Residue 2 (placeholders) is fixed: BW `then_comment`, `then_if` and `two_tails` now match; BY `tail_then_block_comment`, `chr92_tail_then_directive`, `tail_then_procedure` and `tail_then_comment_include` match.
- Residue 3 is only partly fixed (cause 2), and the placeholder design is unsound (cause 1).
- Pre-existing gaps the probes exposed, all builds:
  - **Wrapped include files:** `DiagramDetector#extractFromFile` runs `ReadFilterMergeLines` on the included file (DiagramDetector.java:71–99), so Java joins the tail with the included `@enduml` (`wrapped @enduml` then `two`). Rust does not.
  - **Tail before `!theme`** (maker residue 2): Java joins `tail` with the theme's first TIM line.
  - **Locations** of included and procedure-shifted lines: T14.53, T14.33 (`cr2_seq_procedure_emits_tail`, line 7 vs 6).

**Ditaa:** Java's exemption is case-insensitive and does not depend on the end's indentation. `cr2_ditaa_indented_end_continued_row`, `cr2_ditaa_case_start_continued_row` and the reference give byte-identical Java PNGs. Rust cannot compare PNGs (compare-101). `@startDitaa` is a valid-input error at every Rust build (pre-existing).

## Numbers

Unchanged `compare_cli` and scripts, 4 modes, each case run from its recorded cwd. Builds are fresh `git archive` trees, each with its own `CARGO_TARGET_DIR`.

| Cohort | n (Java-valid) | tip match | vs `b0fb43c3e` (merged base) | vs `93bf53e18` | vs `9834a7194` |
|---|---:|---:|---|---|---|
| BY (CR2) | 53 (36) | 22 | +6 gains, 5 newly rejected, **2 lost rejections**, 1 valid-error→mismatch | +16, 9 newly rejected, **3 lost rejections** | +16, 9 newly rejected, 1 new valid error |
| BW (r2) | 23 (20) | 14 | +13, **1 new valid error**, 1 valid-error→mismatch | +14, 1 newly rejected, 1 new valid error | +14, 3 newly rejected, 1 new valid error |
| BU | 40 (37) | 29 | +4 | +27, 2 newly rejected | +27, 2 newly rejected |
| BD | 33 (29) | 23 | none | +18, 2 newly rejected, 1 lost rejection | +18, 2 newly rejected, 1 new valid error |
| BA | 64 (57) | 41 | none | +37, 4 newly rejected, 1 lost match | +36, 4 newly rejected, 2 new valid errors |
| AY | 48 (44) | 34 | none | +21, 1 newly rejected, 1 lost match | +20, 1 newly rejected |
| AZ | 8 (8) | 3 | none | +3 | +3, 1 new valid error |
| AX | 77 (69) | 26 | none | +18, 1 lost rejection | +14, 1 newly rejected, 2 new valid errors |
| AV | 63 (60) | 20 | none | +10, 3 newly rejected | +8, 3 newly rejected, 2 new valid errors |
| AT | 45 (42) | 19 | none | +7, 1 newly rejected | +4, 3 newly rejected, 1 new valid error |
| AS/AQ/790/28 | 915 (602) | 247 | none (0 bytes) | +59, 35 newly rejected | +53, 289 newly rejected, 2 new valid errors |
| retained | 4,174 | 2,667 | 1 newly rejected (`checker_092_empty`, correct) | +22, 3 newly rejected | +78, 253 newly rejected, 13 new valid errors (the approved set, unchanged from round 1) |
| golden | 12,550 | 11,181 | 0 outcome or SVG-byte changes | 1 newly rejected | 126 newly rejected |

Panics: only the unchanged `--yaml` set, identical at `93bf53e18`, the merged base and the tip (retained 63, golden 148; `9834a7194` has 72/160). There were no timeouts.

## Every loss, classified

**vs `b0fb43c3e` (merged base):**
- `cr2_seq_chr30_rsp_line`, `cr2_seq_raw_rs_rsp_line`: lost rejections. **Packet** (placeholder collision).
- `r2_component_include_tail_then_variable`: new valid-input error. Exposed by the correct placeholder pass-over; root cause **T14.21**.
- Uncounted semantic losses: `cr2_seq_depth2_tail_then_end`, `cr2_seq_tail_then_indented_end`, `cr2_seq_chr92_tail_then_end` lose the Java-equivalent Welcome page. **Packet** (missing `getTotalLineCount()==2`).

**vs `93bf53e18`, the above plus:**
- `cr2_seq_tail_then_backslash_end`: lost rejection. Accidental rejection at `93bf53e18`; Java's Welcome page comes from the same missing rule (packet, since round 1).
- `ck4_procedure_sprite_end_comment_two`, `mk4_tim_sprite_in_procedure`: lost matches, **T14.33**.
- `ck3_svg_inline_quoted_apostrophe_seq`: lost rejection, **T14.32**.
- `mc1_seq_foreach_body`: lost rejection, **T14.52**.

**vs `9834a7194`:**
- No lost matches or rejections.
- New valid errors in fresh cohorts:
  - The 11 known: `mc1_usecase_continued`, `ck4_seq_divider_trailing`, `ck4_usecase_leading_actor_line`, `mk4b_theme_trailing_comment_usecase`, `ck3_usecase_note_top_of_paren`, `ck3_class_enum_upper_body`, `mk3_usecase_right_header_body`, `mk3_deployment_archimate_body`, `ck_usecase_header_body`, `mk2_proc_uninvoked_svg_usecase`, `mk2_usecase_note_upper`. These are T14.21, plus `ck4_seq_divider_trailing` under T14.31.
  - 2 new held-out ones:
    - `r2_component_include_tail_then_variable` (T14.21).
    - `cr2_seq_pre_start_backslash_text`: Java discards text before the start (T14.26); Rust errors since `93bf53e18`.
- The retained 13 are the approved set.

## Gates (once each; `gates/`)

- **Workspace lib:** 1,510 passed, 0 failed.
- **`pipe_mode`:** 13 passed.
- **`golden_no_oracle`:** 11,181 of 11,251 eligible pass. The 70 failure names are byte-identical to a `9834a7194` run. It fails only on the unlocked class/preprocessing/rendering baseline lines.
- **`golden_pairs`:** 10,497 passed, 754 failed. The names are byte-identical to `9834a7194`.
- **`parity_perturbations`:** 2,531 failures at the tip against 2,532 at the merged base. 0 new; 1 fixed (`tc1_seq_indented_end_after_backslash`).
  - The preprocessor panic `byte index … is not a char boundary` (`preprocess/mod.rs:2672`, `:2659` at the base) occurs at both. It is pre-existing; a target is proposed.
  - Measured before this review registered new pairs.
- **`harness_guards`:**
  - The four frozen-item guards pass.
  - `decimal_literal_ratchet` fails on the same 5 render files as the merged base.
  - `parity_review_records_are_complete`: 2,707 normalised violations, identical to the merged base. There are no T14.6.7 path violations in either mechanism directory.
- **`rejection_contract.py`** (unchanged; `rejection/`):
  - BY against the merged base introduces 2 losses (the placeholder pair); against `93bf53e18` it introduces 3.
  - BD against `93bf53e18`: `mc1_seq_foreach_body`.
  - AX against `93bf53e18`: `ck3_svg_inline_quoted_apostrophe_seq`.
  - Every other cohort: none.

## Process and frozen items

- The account (`3573444dc`, 09:38) precedes the code (`f2d12d0ef`, 09:42) and touches only docs.
- No fixture-keyed logic: fixture-like strings appear only in `#[cfg(test)]` unit tests.
- Nothing under `crates/rustuml-oracle`, `crates/rustuml/tests`, `test-diagrams`, `scripts` or `.gitmodules` changed between `b0fb43c3e` and `e3aa1c6b4`. Comparator slack, syntax markers, park list, error census, guard allow-lists, the strict-XML list and the 15-source approved set are unchanged.
- The maker's records follow the T14.6.7 layout.

## Registered pairs

`registered-pairs.json` lists 29 Java-valid, resource-free SVG pairs (9 BW, 20 BY) under `test-diagrams/perturbations/tim-line-continuation/`. `evidence-only-cases.json` holds 47: Java-invalid cases, cases that need an include resource (T14.6.2), and ditaa PNGs.

## Files

- `summary.json`, `runs/` (fresh cohorts, 4 builds), `manifests/`, `rejection/`, `gates/`, `binaries.sha256`, `verify-fresh.json`.
- `probes/` (per-case BY/BW tables), `request-by/` (relay results and SHA256SUMS), `pending-request-cr2/` (pre-registration).
- `support/`. The unchanged scripts' hashes are in `support/unchanged-checker-scripts.sha256`.
