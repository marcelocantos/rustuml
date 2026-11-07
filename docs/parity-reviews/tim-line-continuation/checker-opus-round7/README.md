# TIM line continuation (🎯T14.22) round 7: re-check of the Pipe-drop / regionMatches / ReadLineReader correction (Opus)

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/opus-tim-round7`. It reviews maker tip `df078f7d7`:
- code `16b1c4dcf` (with the draft `50adb43e2` before it);
- addenda 6 `7d9694cdb` and 7 `466e6d883`;
- pre-registration `4ef8260d2` (batch o7p, relayed as DM).

The merged base is `6d195c178` (parity-recovery `f6a738af1` merged in; code unchanged from round 6). The parents are round-4 code `9ed52f435` and parity-recovery `f6a738af1`. The record is `../review-8.json`. Java was never run locally.

**Verdict: rejected. The packet is not accept-ready for local merge as the bar is written.**

The round-7 code itself causes no loss. Against the merged base it is 0 worse over 18,660 cases, and every one of its three claims holds on the clean held-out probes. All four of review-7's must-pass pairs now match. The rejection rests on 3 held-out cases that are worse than `f6a738af1` and fall outside the named allowed set. All 3 are inherited from earlier rounds of this packet: they fail identically at the merged base and at `9ed52f435`. If the parent adds them to the inherited set (rulings below), the packet meets the stated bar.

| Against | Worse | Composition |
|---|---:|---|
| merged base `6d195c178` | **0** | 47 other changes: 39 gains, 6 error→mismatch, 2 error→unscoreable PNG |
| `9ed52f435` | **2** | `r5b_start_bs_chen` (review-7 ruling) and DN `q8_lead_blank_assert_bs` (ruled below: not a loss) |
| `f6a738af1` | **44** | 35 review-5 approved, 4 named inherited, BY `cr2_seq_two_blocks_first_swallowed`, `chen`, **3 outside** |

## The 3 cases outside the allowed set (worse than `f6a738af1`; tip = merged base = `9ed52f435` on each)

1. **DM `o7p_fmt_no_space` and `o7p_fmt_indented` (the maker's expected mismatches): inherited, not caused by round 7.**
   - Each has a content line (`@@@formatsvg`, `  @@@format svg`) before `\startuml` and no column-0 mark.
   - Pipe prepends `@startuml`, so Java's BlockUmlBuilder opens a block, then restarts it at `\startuml` (`isStartDirective`). The content line is discarded, and Java draws the message at line 3.
   - Every TIM-line build (`9ed52f435`, `6d195c178`, the tip) keeps the line in the block and errors: "no registered sequence command accepts". `f6a738af1` matches.
   - The same happens with a plain `hello` line, so this is not about `@@@format` at all. The mechanism is 🎯T14.111's nested restart, whose named witness `r5b_nest_uml_content_bs_json` is already inherited.
2. **DN `q8_cr_inside_marked` (`one\rVane -> Quern : two`): inherited, not caused by round 7.**
   - Pipe's `BufferedReader#readLine` ends a line at a lone CR, so Java draws two messages.
   - No Rust build splits on a lone CR. `f6a738af1` renders one message with a CR in its label (mismatch). The strict sequence admission that the TIM-line branch carries rejects that line (error), exactly as at the merged base and `9ed52f435`.
   - Same mechanism as the approved CC `cr4_inc_cr_only_lines` (CR-only lines in an include).

**Proposed disposition:** add these 3 to the inherited set (2 under 🎯T14.111 nested restart, 1 under the line-terminator target proposed below). No round-7 code change is needed for them.

## Ruling on DN `q8_lead_blank_assert_bs` (worse than `9ed52f435`): not a loss

- Java rejects the failing `!assert 1 == 2 : boom` (exit 200, "Assertion error : boom").
- `9ed52f435` "rejects" only because round-4 code cannot read `\startuml` at all.
- The tip, the merged base and `f6a738af1` all render. The same body under `@startuml` renders at every build, `9ed52f435` included.
- A failing `!assert` is ignored by Rust's TIM everywhere, so this is pre-existing TIM residue, not a packet loss (proposed target 4). The reasoning is the same as the `chen` ruling: getting the old outcome back would need `\startuml` to stop working.

## Claim checks

**Claim 1: `@@@format` and leading trim-empty lines never reach TIM or the parser.**

Confirmed for content. Java removes `@@@format ` lines anywhere (start, middle, before the end, between continuation pieces, twice in a row, prefix-only `@@@format `, CRLF), and so does the tip. Matches: DN `q8_fmt_mid_bs_seq_svg`, `q8_fmt_in_continuation_bs` ("one tail"), `q8_fmt_crlf_mid_bs`, `q8_fmt_after_blank_lead_bs`, `q8_fmt_include_parts_bs` and `q8_mix_crlf_fmt_bom_cont`; DM `o7p_fmt_before_end_bs`, `o7p_fmt_two_lines_bs` and `o7p_blank_lead_ctrl_mix`.

The tip correctly rejects, as Java does, the lines Pipe does not remove:
- `@@@format<TAB>svg`;
- `@@@FORMAT svg`;
- a BOM-prefixed `@@@format` (Pipe sees the BOM; ReadLineReader strips it later);
- a `@@@format` line inside an `!include` (only Pipe reads stdin);
- a U+001F line after content.

Leading trim-empty lines match in every unmarked probe: blank, tab, CRLF-mixed, U+001F and U+0001 leads; a no-start body; a no-start continuation; a TIM definition after the leads. The "empty placeholder" leading lines are equivalent downstream:
- PlantUML has no line builtin (`tim/builtin` has none).
- Error lines agree with Java's stderr: `q8_lead_blank_error_bs` and `q8_fmt_mid_error_line_bs` are Java `ERROR 3` and Rust `line 3`.
- The Welcome rule counts the same: `q8_welcome_lead_ws_bs` and `q8_welcome_lead_ctrl_bs` are Welcome at both.
- A NO_CONTENT stdin (Java exit 50) is rejected.

**Part false: the numbering after a removed `@@@format` line in a MARKED input.** Java numbers every line from Pipe's text, so lines after a removed `@@@format` line move up. The tip's "explicit numbers" are file lines.

| Case | Java lines | Tip lines | Tip outcome |
|---|---|---|---|
| DN `q8_fmt_mid_marked_seq` | 1,2 | 1,3 | mismatch |
| DN `q8_fmt_two_mid_marked` | 1,2 | 1,4 | mismatch |
| DN `q8_fmt_space_only_marked` | 1,2 | 1,3 | mismatch |
| DL `q7_num_format_mid_marked` | 1,2,3 | 1,3,4 | mismatch |

None of these is a loss: `f6a738af1` gives the same numbers, and the base errors. No golden contains a `@@@format` line, so subtracting the removed lines is golden-safe (proposed target 1).

A `@@@format` line *before* a column-0 mark (DN `q8_fmt_lead_marked_seq_two`, DM `o7p_fmt_first_at_seq`) falls under 🎯T14.114, the owner decision on lines before the mark.

**Claim 2: regionMatches(true) case fold.** Confirmed.
- **Independent regeneration:** I downloaded UnicodeData 15.0.0 (`806e9aed…`) and 13.0.0 (`bdbffbbf…`) myself and ran `support/fold_check.py`. It ports `StringUTF16.regionMatchesCI_Latin1` per UTF-16 unit, comparing in both directions and with a broader U/L variant. All give exactly U+0130 and U+0131 → i, U+017F → s and U+212A → k, equal to the Rust table.
- **The type switch:** `Character.toLowerCase` lands on ASCII only for U+0130 (i) and U+212A (k), and `getTypes` has a case for neither. So any fold char in the switch position is UNKNOWN, which the Rust switch reproduces (DM `o7p_type_longs_first_salt` and `o7p_type_kelvin_first` are rejected).
- **Matching probes:**
  - U+017F in `start` (indented `@ſTARTUML`, `\ſTARTMINDMAP`, DM `\ſtartuml` and `@ſtartmindmap`);
  - U+0130 in an upper-case type (`\startMİNDMAP`, DM `\startmİndmap`);
  - U+0131 (DM nwdiag);
  - U+017F at the end of the type (DM wbs);
  - upper-case `@STARTUML` at column 0, which is not a Pipe mark and restarts the block.
- **Rejected as in Java:** `I`+U+0307, a fullwidth `ｕ`, `ß` in `start` or in the type, and an upper-case `@ENDUML` (end matching is case-sensitive).

**Claim 3: ReadLineReader on the main block.** Confirmed for what `ReadLineReader#readLine` does after `br.readLine()`: it drops one leading U+FEFF and replaces U+2013 with `-` (ReadLineReader.java:96-100; the U+00A0 and quote replacements are commented out).
- **Matching probes:**
  - a BOM before a column-0 `@startuml` (which is not a Pipe mark: Java line 2, tip line 2);
  - BOMs on a continued line, before a comment, before a TIM line, on every line, and on the end line;
  - U+2013 in a class arrow, a label and an include path (`!include q8–dash.iuml` finds `q8-dash.iuml`);
  - a U+2013 header that UncommentReadLine strips from `- ` lines, which shows the reader runs before UncommentReadLine.
- **Nothing extra was added:** U+2014, U+2212 and U+2012 arrows are rejected as in Java. U+00A0, curly quotes and an inner U+FEFF are kept. The quote and inner-FEFF probes mismatch at every build on text advance only (residue).
- **CC:** the maker's "5 more CC" is confirmed: `cr4_main_bom_line2`, `cr4_main_bom_before_directive`, `cr4_main_endash_arrow` (all three approved), `cr4_main_endash_label` and `cr4_main_endash_include_path` are gains against the base.
- **Not ported (pre-existing at every build): `BufferedReader`'s line terminators.** A lone `\r` ends a line in Java's Pipe and in every ReadLineReader, but not in Rust. DN `q8_lone_cr_bs` gives Java a diagram but every Rust build a Welcome page (mismatch); see also `q8_cr_inside_marked` above (proposed target 2).

**`@@@format png` (DM `o7p_fmt_mid_at_seq`, `o7p_fmt_mid_bs_seq`).** `Pipe#manageFormat` switches Java's output to PNG. The comparator cannot read a PNG reference (`compare-101` at every build), and Rust ignores the switch (proposed target 3).

## Relays

| Batch | Directory | `reference-results.json` | Cases | Pre-registered |
|---|---|---|---|---|
| DN (checker Q8) | `rustuml-relay-parent/opus-tim-r7-recheck-heldout/` | `ac31f1a3…` | 51 (3 resources) | `c9423fbb9`, before the relay |
| DM (maker o7p) | `rustuml-relay-parent/opus-tim-r7-dm/` | `a44f0ac0…` | 28 | `4ef8260d2`, before the relay |

- **Re-rendering:**
  - DM: the maker's `o7p-rust-predictions.json` (`9b3d9650…`) re-renders 112/112 (28 cases × 4 builds) byte-identically with my fresh builds.
  - DN: my `q8-rust-predictions.json` (`952a42b6…`) re-renders 204/204.
  - The relay did not copy the prediction file alongside the results; it is committed in `pending-request-q8/`.
- **Held-out check:** 0 DN hits among 301,502 corpus files.
- **DN Java results:** 36 exit 0, 14 exit 200, 1 exit 50.
  - Validity under the declared overlay: 33 valid. Invalid: 14 exit-200 error pages; 3 Welcome pages (exit 0); exit 50; 2 "not supported" pages, which are exit 200 but carry no error marker, so only the overlay marks them invalid.
  - The three "intent disagreements" are exit-0 Welcome pages. I predicted them Welcome/invalid, so all 51 validity predictions are correct, and every predicted message `data-source-line` matched.
- **DN at the tip:**
  - 25 of 33 valid cases match. Of the 8 that do not, 7 are mismatches at `f6a738af1` too, and 1 (`q8_cr_inside_marked`) is ruled above.
  - All 3 Welcome cases are Welcome matches.
  - 14 of 15 invalid cases are rejected. The exception is `q8_lead_blank_assert_bs`.
- **DN against the merged base:** 20 changes, all gains or neutral, 0 worse.

## Three-way comparison (T14.63 Welcome-match overlay plus the declared validity overlay; `results/three-way-table.json`)

The overlay is review-7's, unchanged. A Java reference that exits 100 or 50, or a "Diagram not supported by this release" page, is Java-invalid even with no error marker. A Java Welcome page that Rust reproduces is a match.

"Worse" means a lost match, lost rejection, lost Welcome match, new valid-input error, panic or timeout. Each cell gives worse/other changes. The tip is `df078f7d7`.

| Cohort | n | Java-valid | tip match | Welcome match | vs `6d195c178` | vs `9ed52f435` | vs `f6a738af1` |
|---|---:|---:|---:|---:|---|---|---|
| DN (checker held out) | 51 | 33 | 25 | 3 | 0/20 | **1**/31 | **1**/31 |
| DM (maker held out) | 28 | 23 | 17 | 0 | 0/17 | 0/20 | **2**/15 |
| DL | 58 | 50 | 46 | 0 | 0/3 | 0/38 | 0/40 |
| DK | 36 | 33 | 31 | 0 | 0/1 | 0/24 | 0/18 |
| DJ | 57 | 47 | 40 | 1 | 0/0 | **1**/33 | **5**/29 |
| DH | 28 | 26 | 23 | 0 | 0/0 | 0/21 | 0/15 |
| DF | 72 | 57 | 45 | 0 | 0/0 | 0/12 | **1**/28 |
| DD | 13 | 12 | 10 | 0 | 0/0 | 0/9 | 0/9 |
| CC | 74 | 35 | 23 | 8 | 0/5 | 0/9 | **6**/24 |
| CB | 39 | 25 | 24 | 6 | 0/0 | 0/0 | 0/36 |
| CA | 88 | 58 | 49 | 17 | 0/0 | 0/1 | **3**/71 |
| BZ | 23 | 14 | 13 | 5 | 0/0 | 0/0 | 0/18 |
| BY | 53 | 37 | 27 | 5 | 0/0 | 0/2 | **2**/37 |
| BW | 23 | 20 | 16 | 0 | 0/0 | 0/1 | **1**/19 |
| BU | 40 | 37 | 32 | 0 | 0/0 | 0/1 | 0/31 |
| BD | 33 | 30 | 25 | 0 | 0/0 | 0/0 | **1**/22 |
| BA | 64 | 57 | 52 | 0 | 0/0 | 0/2 | **1**/12 |
| AY | 48 | 44 | 36 | 0 | 0/0 | 0/0 | 0/1 |
| AZ | 8 | 8 | 5 | 0 | 0/0 | 0/0 | **1**/0 |
| AX | 77 | 69 | 27 | 0 | 0/0 | 0/0 | **2**/7 |
| AV | 63 | 60 | 20 | 0 | 0/0 | 0/0 | **2**/10 |
| AT | 45 | 42 | 20 | 0 | 0/0 | 0/0 | **1**/8 |
| 915 (AS, AQ, 790, 28) | 915 | 602 | 249 | 0 | 0/0 | 0/0 | **2**/334 |
| retained | 4,174 | – | 2,747 | 0 | 0/1 | 0/6 | **13**/332 |
| golden | 12,550 | – | 11,183 | 17 | 0/0 | 0/1 | 0/126 |
| **Total** | 18,660 | 1,419 | 14,785 | 62 | **0**/47 | **2**/211 | **44**/1,273 |

**Worse lists:** `merge/worse-vs-{6d195c178,9ed52f435,f6a738af1}.json`. Each row is flagged `in_review5_approved`, `named_inherited`, `by_inherited` or `chen`, and each file carries an `outside_allowed` list.
- Of review-5's 38 approved cases, 35 remain worse. The 3 fixed are `cr4_main_bom_line2`, `cr4_main_bom_before_directive` and `cr4_main_endash_arrow`.

**Maker counts:**
- "0 worse vs the merged base" and "1 worse vs `9ed52f435`" are confirmed on the maker's cohorts. DN adds `q8_lead_blank_assert_bs`, ruled above.
- "40 worse vs the head" is 41 by the unchanged classifier on the same cohorts. The maker scores BY `cr2_seq_two_blocks_first_swallowed` as a correct rejection; the classifier counts it as a new valid-input error (`f6a738af1` is unscoreable, `compare-3`). BY is in the allowed set either way.
- With DM's 2 and DN's 1 the total is 44.

**Panics and timeouts:**
- 0 timeouts.
- Panics occur only in `--yaml` mode: retained 63 and golden 148 at the tip, the base and `9ed52f435` (72 and 160 at `f6a738af1`), plus DL `q7_type_bs_ebnfx` (review-7 residue). None is new against the base.

## Process

**The `50adb43e2` slip:**
- I reproduced it: `50adb43e2`'s `rustuml-parser` lib run fails exactly `parse::tests::parse_block_preserves_absolute_source_lines` (747 passed, 1 failed; `gates/slip-50adb43e2-parser-lib.log.gz`), as addendum 7 declares.
- **Ordering:** addendum 6 (16:32:47) precedes the draft (16:35:46). Addendum 7 (16:36:33) precedes the fix `16b1c4dcf` (16:38:00). The o7p pre-registration (16:53:49) precedes the DM relay (16:55-16:56).
- **No effect on the evidence:** no measurement, prediction or relay used `50adb43e2`. The maker's binaries are `rustuml-16b1c4dcf`, `-59d78989f`, `-9ed52f435` and `-9fc567332`, its o7p predictions re-render byte-identically with my fresh tip build, and the tip's lib run is clean.
- It is still a process defect: the maker's command chain did not stop on a failing test. That is a tooling note, not a rejection ground.

**Diffs:**
- **Round-7 code** touches only `preprocess/mod.rs`. The new non-test literals are the 4-entry fold table (with its UCD provenance), `@@@format `, U+FEFF and U+2013. There is no fixture-keyed logic.
- **Against `f6a738af1`:**
  - `crates/rustuml-oracle` differs only in the `doc_claims` pin (11,182 → 11,183);
  - `scripts` and `.gitmodules` do not differ;
  - the non-perturbation `test-diagrams` differ only in `no_oracle_baseline.txt`;
  - perturbations are additions only.
- **Against the merged base:** `test-diagrams`, `scripts`, the oracle crate, README and STABILITY do not differ.

**Frozen items unchanged:** `GEOM_EPS`, markers, the park list, the error census, the allow-lists.

**`rejection_contract.py`:** 0 introduced losses in 67 of 69 runs (23 cohorts × 3 parents). The 2 exceptions are both against `9ed52f435`:
- DJ `r5b_empty_bs_block` (a Welcome match, as in review-7);
- DN `q8_welcome_lead_ws_bs` and `q8_welcome_lead_ctrl_bs` (Welcome matches) and `q8_lead_blank_assert_bs` (ruled above).

## Gates (`gates/`; the tip in the worktree, the base in a `git archive` export with the golden submodule symlinked)

| Gate | Result |
|---|---|
| Workspace lib | 1,711 passed, 0 failed (1,708 + the 3 new round-7 tests) |
| `pipe_mode` | 17/17 |
| `doc_claims` | 2/2 |
| `golden_no_oracle` | Pass, 11,183/11,251 at the tip and at `6d195c178`. The 68 eligible failure names (from the golden replay) are identical to the base's and to the maker's list. Against `f6a738af1` only the round-5 lock `edge_pragma_useVerticalIf` differs (`f6a738af1` fails it). Lock and pins (README, STABILITY, `--help-agent`, `doc_claims`) are 11,183/11,251. The baseline file is unchanged from the merged base; nothing is unlocked |
| `golden_pairs` | 10,498/753/1,299, output identical to the base |
| `parity_perturbations` | 3,722 failing at the tip, 3,727 at the base. 0 tip-only failures. The 5 base-only failures are gains: `cr4_main_bom_before_directive`, `cr4_main_bom_line2`, `cr4_main_endash_arrow`, `cr4_main_endash_label` and `times-new-roman-unicode-advances/checker_punctuation_four_faces_tb_nested`. Of 156 TIM perturbations, 130 pass (126 at round 6). The tip list equals the maker's |
| `harness_guards` | 5 pass. `decimal_literal_ratchet` (render-crate literals) and `parity_review_records_are_complete` fail with identical messages at the tip and the base (3,530 lines, path-normalised diff empty). Neither involves round-7 code |
| clippy `-D warnings` | `rustuml-layout` and `rustuml-math` pass. `rustuml`, `rustuml-parser`, `rustuml-render` and `rustuml-oracle` fail only on the unused `looks_like_description_bracket_command`, as at round 6 (also unused at parity-recovery) |
| `rejection_contract.py` | See Process |

## Passing executable pairs named by this review

Each is a source and its Java SVG in the immutable relay.

**`rustuml-relay-parent/opus-tim-r7-recheck-heldout/` (DN), matching at the tip:**
- `q8_fmt_mid_bs_seq_svg`, `q8_fmt_in_continuation_bs`, `q8_fmt_crlf_mid_bs`, `q8_fmt_after_blank_lead_bs`, `q8_fmt_include_parts_bs`, `q8_inc_parts_bs_control`;
- `q8_lead_ws_nostart_seq`, `q8_lead_crlf_mixed_bs`, `q8_blank_after_start_bs`, `q8_lead_blank_cont_nostart_class`;
- `q8_cf_ws_at_longs_upper`, `q8_cf_bs_longs_upper_mindmap`, `q8_cf_upper_dotted_mindmap`, `q8_cf_upper_at_col0_restart`;
- `q8_rlr_bom_marked_first`, `q8_rlr_bom_continuation_bs`, `q8_rlr_bom_comment_marked`, `q8_rlr_bom_tim_var_marked`, `q8_rlr_endash_class_arrow`, `q8_rlr_endash_header_mixed_bs`, `q8_rlr_endash_include_path`, `q8_rlr_nbsp_kept_label`;
- `q8_mix_crlf_fmt_bom_cont`, `q8_mix_lead_bom_blank_ws`, `q8_tim_def_after_blank_lead_bs`.

**Review-7's must-pass pairs, now matching:**
- DK `o6n_format_line_bs_seq`;
- DL `q7_num_us_blank_bs_seq`, `q7_type_bs_dotless_mindmap` and `q7_hdr_bom_content`.

## Proposed targets (not filed; `bullseye.yaml` untouched)

1. **Marked inputs number lines after a removed `@@@format ` line from Pipe's text.**
   - Every later line moves up one per removed line, as Java's `-pipe` does.
   - This is golden-safe: no golden contains `@@@format`.
   - Witnesses: DN `q8_fmt_mid_marked_seq`, `q8_fmt_two_mid_marked` and `q8_fmt_space_only_marked`; DL `q7_num_format_mid_marked`.
   - Lines before the mark stay under 🎯T14.114.
2. **Main-block and include input splits lines as `BufferedReader#readLine` does** (`\n`, `\r`, `\r\n`).
   - Witnesses: DN `q8_lone_cr_bs` and `q8_cr_inside_marked`; CC `cr4_inc_cr_only_lines`.
3. **`-pipe` honours `Pipe#manageFormat`:** `@@@format png`, `svg`, `atxt` and `utxt` switch the output format (Pipe.java:227-237), or the gap is documented.
   - Witnesses: DM `o7p_fmt_mid_at_seq` and `o7p_fmt_mid_bs_seq` (Java PNG).
4. **A failing `!assert` is a TIM error** (Java "Assertion error : <msg>", exit 200).
   - Rust renders at every build.
   - Witness: DN `q8_lead_blank_assert_bs`.
5. **New witnesses for 🎯T14.111's nested restart** (a content line before a nested start in an unmarked input): DM `o7p_fmt_no_space` and `o7p_fmt_indented`.
6. **Text advances for U+201C/U+201D/U+2018/U+2019 and an inner U+FEFF** match Java. These mismatch at every build.
   - Witnesses: DN `q8_rlr_smart_quotes_label` and `q8_rlr_bom_mid_line_kept`.

## Files

- `../review-8.json`
- `results/`:
  - `three-way-table.json` and `summary.json.gz`;
  - `dn-percase.json`, `dm-percase.json` and `binaries.sha256`.
- `merge/worse-vs-*.json`
- `rejection/`, `gates/` and `manifests/`
- `pending-request-q8/` (committed in `c9423fbb9`, before the relay) and `request-dn/SHA256SUMS`
- `support/`:
  - `build.sh`, `run_chain.sh`, `gates.sh`, `summarize_q8.py` and `table_q8.py`;
  - `gen_probes_q8.py`, `stage_q8.py`, `predict_rust_q8.py`, `heldout_check_q8.py`, `verify_predictions.py`, `fold_check.py` and `try.py`;
  - `unchanged-checker-scripts.sha256`, which records `prepare.py`, `replay.py`, `classify.py`, `fresh_classify.py` and `rejection_contract.py`, identical to review-7's.
