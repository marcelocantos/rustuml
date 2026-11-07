# TIM line continuation (🎯T14.22) round 3: independent checker review

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-round3-claude`. The review covers maker tip `f2accc662`:
- account `60bee23b8`;
- code `d28dd64a9`;
- merged base `a5720e095`, whose code is the same as `f2d12d0ef`.

Records:
- `../review-3.json` (this mechanism).
- `../../sprite-command-post-tim-ownership/review-7.json` (whole-line addendum).

Earlier reviews are preserved. Java was never run locally.

**Verdict: rejected.** Obligation 1 is accepted. Obligations 2 and 3 are right in mechanism, but the held-out batch exposes four packet-caused losses against the merged base.

## Pre-registration (commit `745c941ac`, before any Java result)

The held-out batch CR3 has 88 cases and 22 resources. It was relayed as **CA** (`tim-continuation-cr3`; `reference-results.json` is `b7930a53…`, and `request-ca/SHA256SUMS` lists every file).

| File | sha256 |
|---|---|
| `reference-manifest.json` | `443cea19516ac0801b6eb6136304a7480f13a0e67551cb9b06131b962d0ba1c4` |
| `reference-input-bundle.json.gz` | `8baa2d7f502dbee6689d91884bc1b04b6250fbc2520d1e6380e04a10a8fe5737` |
| `pending-request-cr3/cr3-rust-predictions.json` | `6e88c3003f2b24087b5a94297a0107be25354acfc3801de209becae8e5891899` |
| `pending-request-cr3/cases-with-java-predictions.json` | `2b7d4e57f570da5dc9def26a7de86ab73620d344edf3141aa1e4db4e4d3826a5` |

**Validity.** It was decided from the raw Java SVG and stderr: `prepare.py` markers, `strict_validity.py` and `no_diagram_validity.py`. 58 of the 88 cases are Java-valid.

**Welcome pages are classified separately.** A Java Welcome page is exit 0, `Welcome to PlantUML`, and no error text. CA has 19. The exit-200 "Empty description" pages also draw the Welcome text, but they are errors and so invalid.

**Where my Java predictions were wrong (3):**
- `cr3_dir_startsub_start` and `cr3_dir_startsub_middle`: Java gives an "Empty description" error. At the top level, `CodeIteratorSub#peek` never clears `readingInProgress`, so every TIM line after `!endsub` is lost, including the end line.
- `cr3_wrap_first_is_mindmap`: Java renders `* mm` as an ACTIVITY diagram.

## Causes of rejection (all against the merged base `a5720e095`)

### 1. `!pragma` miscounted by the two-line rule: two lost matches

| Case | Java | `a5720e095` | tip |
|---|---|---|---|
| `cr3_dir_pragma_start` (`!pragma layout smetana`, then a `%chr(92)` tail, then `@enduml`) | message `x @enduml` | match | **Welcome** |
| `cr3_dir_teoz_pragma_start` (`!pragma teoz true`) | teoz message `x @enduml` | match | **Welcome** |

- **Java:** `TLineType` has no `!pragma` type, so a `!pragma` line is PLAIN. It is a line of the TIM result list and therefore of `UmlSource`, so the count is 3 and `PSystemWelcomeFactory` does not fire.
- **Rust:** `try_silent_directive` consumes `!pragma` as a stand-in, so `collapse_post_tim_ending_backslashes` counts 2.
- **Same root cause, every build:**
  - `cr3_dir_pragma_middle` and `cr3_dir_pragma_end`: Java joins the tail with the pragma text, giving `x !pragma layout smetana`.
  - `cr3_two_nojoin_pragma`: Java draws an empty CLASS diagram; Rust draws Welcome.
- `!assume` is PLAIN in both, and all three of its positions match.

### 2. U+0085 is not Java whitespace: lost rejection

`cr3_end_nel_before_bs_end` is U+0085 followed by `\enduml`.
- **Java:** `Character#isWhitespace(U+0085)` is false, so the line is not an end directive. Java reports a syntax error and exits 200.
- **Merged base:** rejects.
- **Tip:** ends the block and renders `one`. `is_end_directive` → `directive_tail` → `java_is_whitespace` is defined as `char::is_whitespace` minus three no-break spaces. That set includes U+0085, and it omits U+001C–U+001F, which Java does count as whitespace. `cr3_end_us_before_bs_end` (U+001F) is therefore a valid-input error at every build since `93bf53e18`.

### 3. Count-1 sources are errors in Java, Welcome in Rust: lost rejection

`cr3_end_bs_in_procdef` has `\enduml` inside a `!procedure` definition.
- **Java:** the block ends there. TIM swallows the end line into the unterminated definition, so `UmlSource` has one line. `PSystemCommandFactory` builds an "Empty description" error; Java exits 200 (invalid).
- **Merged base:** rejected, but only because `\enduml` was unsupported.
- **Tip:** returns the exit-0 Welcome page, because Rust has no count-1 rule.
- **Same shape, with no base rejection to lose:** `cr3_two_one_line_after_join` and `cr3_two_zero_content_end_continued`.

## Per-mechanism verdicts

### Obligation 1: out-of-band stand-ins (`TimLine { Text, Consumed }`): accepted

- BY `cr2_seq_chr30_rsp_line` and `cr2_seq_raw_rs_rsp_line` are rejected again. BZ's 4 RS cases are rejected (Java exit 200).
- The change is byte-neutral wherever no join or stand-in is involved, against `a5720e095`:
  - 0 SVG bytes on the 12,550 goldens, the retained 4,174, the 915, and BW, BU, BD, BA, AY, AZ, AX, AV and AT.
  - The 13 golden model-mode differences are run-to-run nondeterminism. `dot/*` and `sprites/sprite_multiple` give 2–6 distinct hashes in 6 runs at both builds.
- **Per directive (CA):** the tip matches Java at the start, middle and end for `!define`, `!procedure`, `!if`, `!include`, `!option`, `!log` and `!assume`. The `!option` middle and end cases are gains, because `!option` is now `Consumed`.
- **Pre-existing mismatches:**
  - `!theme`: Java joins the tail with the theme's first TIM line.
  - `!dump_memory`: Rust emits a text line `' [dump] …`; Java has no entry.
  - `!startsub`: the top-level truncation described above.

### Obligation 2: the two-line rule: right mechanism, wrong where Rust's stand-ins differ from Java's list

- **BZ:** all 5 Java Welcome pages are reproduced. None of the 5 intended-invalid exit-0 cases is an error page; each is a `PSystemWelcome` page with no error text:
  - `mr3_seq_end_continued_one_line`;
  - `mr3_seq_bad_tail_then_end` (the joined line is no command; the factory runs first);
  - `mr3_class_only_link_tail_then_end`;
  - `mr3_seq_chr92_tail_then_tab_end`;
  - `mr3_seq_tail_then_backslash_end_then_text`.
- **CA:** 11 Java Welcome pages are gained: the directive-start cases, `end_continued_after_two`, `comments_only`, `directives_only`, `double_tail` and `activity_tail`. The count-3 controls match.
- **The rule fires where Java's does not:** the `!pragma` pair (cause 1) and the count-1 sources (cause 3).
- **Java Welcome not reproduced, the same at every build:**
  - `cr3_dir_dump_start` and `cr3_two_include_tail_then_dump`: the dump line.
  - `cr3_two_nojoin_option_handwritten`: the Rust-only skinparam line.
  - `cr3_dir_startsub_end`.
  - An end line inside a block comment or an inactive `!if` (`cr3_end_bs_in_block_comment`, `cr3_end_at_in_block_comment`, `cr3_end_bs_in_if0`, `cr3_end_at_in_if0`). `BlockUmlBuilder` knows neither, so the block ends there and TIM then drops the end line.

### `\end…` forms (`StartUtils#isEndDirective`): faithful except for the whitespace set

- **Matches or rejects as Java does:**
  - Matches: bare `\end`; indented `\endless story`; `\endmindmap` in `@startuml`; trailing text; a second diagram in the Pipe text; no `@end` line at all; a mindmap block (a gain).
  - Rejects: `\Enduml`, a double backslash, and an NBSP before the end.
  - A note body is newly rejected, as Java rejects it.
- **Gaps:**
  - U+0085 and U+001F (cause 2).
  - A TIM-generated `@enduml` or `\enduml` (`cr3_end_tim_chr64_enduml`, `cr3_end_tim_chr92_enduml`). `PSystemCommandFactory#createSystem` stops at the first end-directive line; Rust gives a valid-input error at every build since `93bf53e18`.
  - An included raw `@enduml` (`cr3_raw_include_mid_end`): Java draws `a` only; Rust draws `a|b|two` at every build.

### Obligation 3: `DiagramDetector#extractFromFile`: faithful in structure, with three fidelity defects

- **Gains:**
  - CA: `three_blocks`, `hash_header`, `angle_header`, `bom_start`, `crlf_continuation_comments`, `backslash_end`, `end_trailing_text`, `backslash_start`, `mindmap_family`.
  - BZ: `trailer_dropped`, `continued_line`, `comment_header`, `two_blocks`, `class_wrapped_continued`.
- **Defects.** None loses anything against the merged base: each case is an error at `93bf53e18`, the merged base and the tip.
  1. `before_start_uml` uses `char::is_alphanumeric`; Java uses `Character#isLetterOrDigit`. Example: `cr3_wrap_superscript_header` (U+00B2).
  2. `ReadLineReader#readLine` strips a leading U+FEFF from every line; Rust does not. Examples: `cr3_wrap_bom_hash_header`, `cr3_wrap_bom_inner_line`, and the raw `cr3_raw_bom_include`.
  3. `TContext#executeInclude` strips `!N` or `!id` from a local path and ignores it. Rust opens `file!2` and includes nothing. Examples: `cr3_wrap_three_blocks_bang2`, `cr3_wrap_ids_bang_beta`.
- **Included-line locations** (`data-source-line`, 🎯T14.53): `cr3_wrap_nested_wrapped`, `cr3_wrap_vars_and_procedure` and BZ `mr3_seq_wrapped_preamble_dropped`.

## Numbers

Unchanged `compare_cli` and scripts, 4 modes, each case run from its recorded cwd. The builds are fresh `git archive` trees, each with its own `CARGO_TARGET_DIR`.

| Cohort | n (Java-valid; Java Welcome) | tip match | vs `a5720e095` | vs `93bf53e18` | vs `9834a7194` |
|---|---:|---:|---|---|---|
| CA (CR3) | 88 (58; 19) | 35 | +18 gains; **2 lost matches**; **2 lost rejections** (the contract also lists `cr3_two_activity_tail`, a reproduced Java Welcome page); 1 newly rejected; 11 Welcome pages gained (Java Welcome) | +34; 4 contract-lost (2 real) | +33; 4 newly rejected; 8 new valid errors |
| BZ (MR3) | 23 (14; 5) | 12 | +7; 3 newly rejected; 5 Welcome pages gained (Java Welcome); `mr3_seq_bad_tail_then_end` contract-listed (Java Welcome, reproduced) | +10 | +9; 4 newly rejected |
| BY | 53 (36; 5) | 24 | +2; 2 newly rejected; 5 Welcome pages gained (Java Welcome) | +18; `cr2_seq_tail_then_backslash_end` contract-listed (Java Welcome) | +18; 1 new valid error |
| BW | 23 (20) | 14 | none (0 bytes) | +14; 1 new valid error | +14; 1 new valid error |
| BU, BD, BA, AY, AZ, AX, AV, AT | 378 (346) | 195 | none (0 bytes) | round-2 lists unchanged | round-2 lists unchanged |
| AS/AQ/790/28 | 915 (602) | 247 | none (0 bytes) | +59 | +53; 2 new valid errors |
| retained | 4,174 | 2,667 | 0 SVG bytes, 0 model changes | +22 | 13 new valid errors (the approved set, identical) |
| golden | 12,550 (Java Welcome 17) | 11,181 | 0 outcome or SVG-byte changes | 1 newly rejected | 126 newly rejected |

Panics: only the unchanged `--yaml` set (retained 63, golden 148) at the tip and the merged base; `9834a7194` has 72/160. There were 0 timeouts.

## Every loss, classified

**vs `a5720e095` (merged base):**
- `cr3_dir_pragma_start`, `cr3_dir_teoz_pragma_start`: lost matches. **Packet** (cause 1).
- `cr3_end_nel_before_bs_end`: lost rejection. **Packet** (cause 2).
- `cr3_end_bs_in_procdef`: lost rejection. **Packet-exposed** (cause 3); the base rejection was accidental.
- Contract-listed but Java-equivalent (Java exit-0 Welcome, reproduced): `cr3_two_activity_tail`, `mr3_seq_bad_tail_then_end`.

**vs `93bf53e18`, the above plus:**
- `ck4_procedure_sprite_end_comment_two`, `mk4_tim_sprite_in_procedure`: lost matches, **🎯T14.33**.
- `ck3_svg_inline_quoted_apostrophe_seq`: lost rejection, **🎯T14.32**.
- `mc1_seq_foreach_body`: lost rejection, **🎯T14.52**.
- Java Welcome reproduced: `cr2_seq_tail_then_backslash_end`, `mr3_seq_tail_then_backslash_end_then_text`, `cr3_two_double_tail`.

**vs `9834a7194`:**
- No lost matches and no lost rejections.
- New valid-input errors:
  - **The 13 approved retained sources** (unchanged).
  - **The 11 known fresh:** `mc1_usecase_continued`, `ck4_seq_divider_trailing`, `ck4_usecase_leading_actor_line`, `mk4b_theme_trailing_comment_usecase`, `ck3_usecase_note_top_of_paren`, `ck3_class_enum_upper_body`, `mk3_usecase_right_header_body`, `mk3_deployment_archimate_body`, `ck_usecase_header_body`, `mk2_proc_uninvoked_svg_usecase`, `mk2_usecase_note_upper`. These are 🎯T14.21, plus 🎯T14.31 for `ck4_seq_divider_trailing`.
  - `r2_component_include_tail_then_variable` (🎯T14.21).
  - `cr2_seq_pre_start_backslash_text` (🎯T14.26).
  - **8 new from CA.** Each is an error since `93bf53e18`, where `9834a7194` rendered a mismatch:
    - `cr3_end_us_before_bs_end`: the Java whitespace set.
    - `cr3_end_tim_chr64_enduml`, `cr3_end_tim_chr92_enduml`: the command-loop end peek.
    - `cr3_wrap_first_is_mindmap`: `* mm` is ACTIVITY in Java (🎯T14.21-type dispatch).
    - `cr3_wrap_superscript_header`: `isLetterOrDigit`.
    - `cr3_wrap_bom_hash_header`, `cr3_wrap_bom_inner_line`, `cr3_raw_bom_include`: per-line BOM stripping.

## Gates (once each; `gates/`)

- **Workspace lib:** 1,514 passed, 0 failed.
- **`pipe_mode`:** 13 passed.
- **`golden_no_oracle`:** 11,181 of 11,251 eligible pass. The 70 failure names are byte-identical to a `9834a7194` run. It fails only on the unlocked class/preprocessing/rendering baseline lines.
- **`golden_pairs`:** 10,497 passed, 754 failed, 1,299 skipped. The names are byte-identical to `9834a7194`.
- **`parity_perturbations`:** 2,539 failures at both the tip and `a5720e095`; 0 new, 0 fixed. The 🎯T14.59 char-boundary panic occurs at both. This was measured before registration. After registration there are 2,552: the 13 registered pairs that fail at the tip.
- **`harness_guards`:**
  - `comparator_constants_frozen`, `parity_review_paths_reject_traversal`, `no_golden_data_embedded_in_shipping_crates` and `error_golden_census_is_frozen` pass.
  - `decimal_literal_ratchet` fails on the same 5 render files with the same counts as `a5720e095`.
  - `parity_review_records_are_complete` gives 684 normalised violations, identical to `a5720e095`. None is a 🎯T14.6.7 layout violation.
- **`rejection_contract.py`** (unchanged; `rejection/`): the introduced losses are exactly those classified above.

## Process and frozen items

- The account (`60bee23b8`, 10:39) precedes the code (`d28dd64a9`, 10:44) and touches only docs.
- No fixture-keyed logic: fixture-like strings appear only in `#[cfg(test)]` code.
- Between `9834a7194` and `f2accc662`, nothing changed under `crates/rustuml-oracle`, `crates/rustuml/tests`, `scripts` or `.gitmodules`, and `test-diagrams` only gained perturbation files.
- Comparator slack, syntax markers, the park list, the error census, guard allow-lists, the strict-XML list and the 15-source approved set are unchanged.

## Registered pairs

`registered-pairs.json` lists 40 Java-valid, resource-free SVG pairs (3 BZ, 37 CA) under `test-diagrams/perturbations/tim-line-continuation/`. At the tip, 27 pass and 13 fail in `parity_perturbations`, and the harness agrees with `compare_cli` on every pair.

`evidence-only-cases.json` holds 71 cases:
- Java Welcome pages;
- Java-invalid pages;
- cases that need an include resource (🎯T14.6.2).

## Files

- `summary.json`, `runs/` (fresh cohorts, 4 builds), `manifests/`, `rejection/`, `gates/`, `binaries.sha256`, `verify-fresh.json`.
- `probes/`: per-case tables for CA, BZ and BY.
- `request-ca/`: relay results and SHA256SUMS.
- `pending-request-cr3/`: the pre-registration.
- `support/`. The unchanged scripts' hashes are in `support/unchanged-checker-scripts.sha256`; the first five equal round 2's.
