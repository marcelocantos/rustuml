# TIM line continuation (🎯T14.22) round 5: recheck after the `\startuml` rejection (Opus)

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/opus-tim-round5-recheck`. It reviews maker tip `6fd071169`:
- code `b24935a2d` (start test) and `ea5375293` (line numbering);
- addenda `5956d5915` (3) and `55a14b0f1` (4);
- pre-registration `102291307`;
- pins `32c61d7f0`;
- record `6fd071169`.

The new merged base is `1ad869dae` (parity-recovery `28af44ab3` merged in). The parents are round-4 code `9ed52f435` and parity-recovery `28af44ab3`. The record is `../review-6.json`. Java was never run locally.

**Verdict: rejected. The packet is not accept-ready for local merge.**

The tip clears review-5's blocker: `cr5o_scope_uml_backslash_start` now matches. The fix is incomplete, though, and it causes new losses:
- **Against the merged base `1ad869dae` and against `9ed52f435`: 8 worse each.** These are the same 8 cases: 7 from my held-out DJ batch and 1 from the maker's own held-out DH batch.
- **Against `28af44ab3`: 65 worse.**
  - 38 are exactly review-5's approved list minus the fixed blocker.
  - 27 are outside it: 20 are caused by the packet and 7 are inherited.

The maker's claims of "0 worse vs the merged base" and "38 worse vs `28af44ab3`, all approved" are false.

## Blocker 1: line numbering when Pipe finds no column-0 `@start` (caused by `ea5375293`)

**Java trace.**
1. `Pipe#readSingleDiagram` (Pipe.java:180-225) looks only for a line that `startsWith("@start")`, literal and at column 0.
2. If none exists, it returns `"@startuml\n" + sb + "@enduml\n"` (Pipe.java:217).
   - `sb` begins at the first non-blank line, because `NO_CONTENT` skips blank lines (Pipe.java:191).
3. `BlockUmlBuilder` (100-135) then restarts the block at the `\startuml`, `  @startuml` or `@StartUml` line.
4. Every `data-source-line` and `codeLine` is the 0-based `LineLocation` position in Pipe's text. With no leading blank lines, input line n (1-based) is therefore position **n**.

**Rust divergence.**
- `ea5375293` changes only `PreprocessContext::current_diagram_source_line`. That number reaches only lines that carry a source-line marker: `!theme` bodies, includes and expression or buffered lines.
- Plain lines keep the parser's own numbering, which counts from the start line (n−1), as though Pipe had found its mark.
- So `\startuml` + `!theme` + message matches (DF `cr5o_scope_uml_backslash_start`, DH `o5s_bs_uml_theme_continued`), but a plain `\startuml` block does not.
- The fault reaches every family that emits `data-source-line`: sequence, class, object, state, deployment.
  - `1ad869dae` matched the plain non-sequence cases (class, object, state, deployment).
  - `28af44ab3` matched the sequence cases.

**Evidence** (`results/plus1-line-shift.json`): adding 1 to every `data-source-line` and `codeLine` in the tip's SVG turns 23 of the 27 no-mark tip mismatches into exact matches, scored by the unchanged `compare_cli`. The other 4 also need the `NO_CONTENT` blank-line skip, or are pre-existing; see residue.

**Pairs the maker must pass** (source, Java SVG), all from the immutable relays:
- **Losses against the merged base and `9ed52f435`:**
  - DJ: `r5b_num_bs_state`, `r5b_num_bs_object`, `r5b_num_bs_deployment`;
  - DH: `o5s_bs_uml_class`.
- **Losses against `28af44ab3`:**
  - DJ: `r5b_num_bs_seq_three`, `r5b_num_upper_at_seq`, `r5b_num_us_bs_seq`, `r5b_num_bs_blank_inside`, `r5b_start_bs_name`, `r5b_end_bs_bare`, `r5b_multi_bs_then_bs_mindmap`, `r5b_crlf_bs_seq`;
  - DH: `o5s_mix_bs_start_at_end`, `o5s_ws_tab_bs`, `o5s_ws_bs_end_indented`, `o5s_case_bs_upper_start`, `o5s_case_at_upper_start_lower_end`.
- **Also fixed by the same rule** (mismatch at every build):
  - DJ: `r5b_num_ws_at_seq`, `r5b_num_emsp_seq`, `r5b_num_tab_at_state`, `r5b_num_comment_bs_seq`;
  - DH: `o5s_two_blocks_bs`, `o5s_bs_seq_continuation`.

Directories: DJ is `~/Library/Caches/rustuml-relay-parent/opus-tim-r5-recheck-heldout/`; DH is `.../opus-tim-r5-dh/`.

**Minimal fix.** When `pipe_single_diagram` reports no start mark, number the block's lines from Pipe's text for every line, not only marker-carrying ones. Plain-line positions must be n, not n−1, for both `data-source-line` and `codeLine`. For example:
- emit one extra placeholder for Pipe's prepended `@startuml` line; or
- route all lines through the source-line marker while `!pipe_start_mark_found`.

Inputs whose Pipe mark is found must keep their numbering. Every golden is such an input, and the census finds none without a mark.

## Blocker 2: the start-line type word (caused by `b24935a2d`)

Java's `DiagramType#findStartTypes` and `getTypes` (69-216) decide the type at the character immediately after `start`, by a case-insensitive prefix test (`check`, 218):
- `project` is GANTT and `chen` is CHEN_EER;
- `umlish` is UML;
- a space after `start` is UNKNOWN, which gives `PSystemUnsupported`.

`detect_type` instead takes the first whitespace-separated word after `start` and looks it up by name. Three cases are worse than the merged base:
- **`r5b_start_bs_space_uml`** (`\start uml`): a **lost rejection**. Java exits 200 with "Diagram not supported". `1ad869dae` rejects it; the tip reads `uml` and renders.
- **`r5b_start_bs_chen` and `r5b_start_bs_project`:** **new valid-input errors**. The tip now reports "unsupported diagram type: @startchen/@startproject". The base rendered them, as mismatches, and Java renders both.
  - Rust has no `chen` or `project` dispatch even for `@`, so this is pre-existing for `@startchen`/`@startproject`. The packet extends it to `\start…`.

**Fix:** port `getTypes`: the character after `start`, then the prefix keys, with `project` mapped to gantt. Then either dispatch `chen`, `files` and `creole`, or keep the pre-packet result for types Rust cannot dispatch. That choice is the owner's.

## Blocker 3: an NBSP before `@startuml` (caused by `b24935a2d`)

`r5b_start_nbsp` (` @startuml` … `@enduml`):
- Java exits 0 and renders the message; my prediction was invalid, and Java disagrees.
- `1ad869dae` renders it, as a mismatch, because its old test used Rust's `trim`.
- The tip's `java_is_whitespace` start test correctly treats NBSP as not whitespace, so the line becomes content. Sequence admission then rejects it, a new valid-input error.

Java accepts the line as content somewhere downstream. I did not trace which command consumes it; the maker must, and port it.

## Relays

| Batch | Directory | `reference-results.json` | Cases | Pre-registered |
|---|---|---|---|---|
| DJ (checker R5B) | `rustuml-relay-parent/opus-tim-r5-recheck-heldout/` | `ab87498a…` | 57 (3 resources) | `b31b0168d` (14:42), before the relay |
| DH (maker O5S) | `rustuml-relay-parent/opus-tim-r5-dh/` | `1a36016c…` | 28 | `102291307` (13:51), before the results (14:17) |

- **Re-rendering:** all 228 DJ outputs and all 112 DH outputs re-render byte-identically at the four builds.
- **Held-out check:** no R5B file appears among 314,224 corpus files, by sha256 or by normalised text. The corpus covered the goldens, the perturbations, every relay-parent bundle and every other `rustuml-*` cache (`pending-request-r5b/heldout-check-r5b.json`).
- **Java validity in DJ:** 49 exit 0, 7 exit 200 and 1 exit 100.
  - Under the declared overlay, 47 are valid.
  - The two exit-0 Welcome pages, `r5b_empty_bs_block` and `r5b_end_first_then_content`, carry `prepare.py`'s Welcome marker. They are scored as Welcome matches under T14.63, as I predicted.
  - My validity prediction was wrong on 1 case: `r5b_start_nbsp`, which I predicted invalid and Java renders (see `results/r5b-percase.json`).

**Declared validity overlay** (`support/summarize_r5b.py` with `OVERLAY=1`): a reference with Java exit 100 or 50, or with a "Diagram not supported by this release" page, counts as Java-invalid. Without the overlay the rows are classified exactly as round 5 did (`results/three-way-table-round5-classification.json`). The two classifications differ only on such pages.

## Three-way comparison (T14.63 Welcome-match overlay plus the validity overlay; `results/three-way-table.json`)

"Worse" means a lost match, lost rejection, lost Welcome match, new valid-input error, panic or timeout. Each cell gives worse/other changes. The tip is `6fd071169`.

| Cohort | n | Java-valid | tip match | Welcome match | vs `1ad869dae` | vs `9ed52f435` | vs `28af44ab3` |
|---|---:|---:|---:|---:|---|---|---|
| DJ (held out) | 57 | 47 | 17 | 1 | **7**/22 | **7**/22 | **20**/17 |
| DH (maker held out) | 28 | 26 | 14 | 0 | **1**/21 | **1**/21 | **6**/12 |
| DF | 72 | 57 | 45 | 0 | 0/2 | 0/12 | 1/28 |
| DD | 13 | 12 | 10 | 0 | 0/0 | 0/9 | 0/9 |
| CC | 74 | 35 | 18 | 8 | 0/0 | 0/4 | 9/19 |
| CB | 39 | 25 | 24 | 6 | 0/0 | 0/0 | 0/36 |
| CA | 88 | 58 | 49 | 17 | 0/1 | 0/1 | 3/71 |
| BZ | 23 | 14 | 13 | 5 | 0/0 | 0/0 | 0/18 |
| BY | 53 | 37 | 27 | 5 | 0/2 | 0/2 | 2/37 |
| BW | 23 | 20 | 15 | 0 | 0/0 | 0/0 | 1/18 |
| BU | 40 | 37 | 32 | 0 | 0/0 | 0/1 | 0/31 |
| BD | 33 | 30 | 25 | 0 | 0/0 | 0/0 | 1/22 |
| BA | 64 | 57 | 52 | 0 | 0/0 | 0/2 | 1/12 |
| AY | 48 | 44 | 36 | 0 | 0/0 | 0/0 | 0/1 |
| AZ | 8 | 8 | 5 | 0 | 0/0 | 0/0 | 1/0 |
| AX | 77 | 69 | 27 | 0 | 0/0 | 0/0 | 2/7 |
| AV | 63 | 60 | 20 | 0 | 0/0 | 0/0 | 2/10 |
| AT | 45 | 42 | 20 | 0 | 0/0 | 0/0 | 1/8 |
| 915 (AS, AQ, 790, 28) | 915 | 602 | 249 | 0 | 0/0 | 0/0 | 2/334 |
| retained | 4,174 | – | 2,744 | 0 | 0/0 | 0/3 | 13/331 |
| golden | 12,550 | – | 11,183 | 17 | 0/0 | 0/1 | 0/126 |
| **Total** | **18,487** | | **14,625** | **59** | **8** | **8** | **65** |

**Worse lists:** `merge/worse-vs-1ad869dae.json`, `merge/worse-vs-9ed52f435.json` and `merge/worse-vs-28af44ab3.json`. Each row carries its attribution.

**Against `28af44ab3`, the 65 worse are:**
- **38** on review-5's approved list. Its 39th entry, `cr5o_scope_uml_backslash_start`, is fixed.
- **20 caused by the packet:**
  - 17 numbering cases (blocker 1);
  - `r5b_start_bs_chen` and `r5b_start_bs_project` (blocker 2);
  - `r5b_start_nbsp` (blocker 3).
- **7 inherited**, where the tip's outcome equals `1ad869dae` and `9ed52f435`:
  - `r5b_num_unmarked_bs_end_only`, `r5b_start_bs_umlish`, `r5b_start_bom`, `r5b_start_bs_files`, `r5b_start_bs_creole` and `r5b_nest_uml_content_bs_json`, all from the sprite/participant line's strict admission;
  - BY `cr2_seq_two_blocks_first_swallowed`. Java gives an exit-0 "No valid @start/@end found" page. `28af44ab3` now produces output the comparator cannot score (exit 3); every TIM-line build errors.

**Gains against the merged base:**
- DJ: 10 valid-error→match gains, 10 valid-error→mismatch, 1 newly rejected (`r5b_start_in_note`) and 1 Welcome-match gain (`r5b_empty_bs_block`);
- DH: 21 changes, all better or neutral, except `o5s_bs_uml_class`.

**Panics and timeouts:**
- Panics occur only in the unchanged `--yaml` set: retained 63 and golden 148 at the tip, the base and `9ed52f435` (72 and 160 at `28af44ab3`).
- There are 0 timeouts.

**`cr2_ditaa_case_start_continued_row`:** Java's reference is a PNG, so `compare_cli` exits 101 at every build. The tip renders (exit 0) where the three other builds report "unsupported diagram type: @startDitaa". That is not a loss; it is not scorable as a match either.

## Challenge findings

1. **Start/end recognition is not an exact port.**
   - Exact:
     - `directive_tail` and `java_is_whitespace` (tab, U+2003, U+001F and U+3000 are starts; NBSP is not);
     - the case-insensitive `start` and case-sensitive `end`;
     - `@`/`\` mixing;
     - `\end` and `@endfoo` closing a block;
     - `\startyaml`, `\startnwdiag`, `\startregex` and `\StartJSON`, which now match;
     - `!startsub` in a `\start` main block.
   - Not exact:
     - the type word (blocker 2);
     - a nested start, where Java restarts the block and Rust keeps the first (pre-existing; DH `o5s_nest_*` and DJ `r5b_nest_uml_content_bs_json`);
     - a BOM line: `ReadLineReader` strips U+FEFF from every line after Pipe has looked, and both the tip and the base reject it (inherited);
     - a note line `\started …`, which Java restarts into UNKNOWN; the tip now rejects it too, a gain.
   - `!include f!N` and `!include f!ID` over `\start` blocks render at the tip and the base but mismatch Java.
2. **Line numbering is incomplete** (blocker 1).
   - Java's error pages number from Pipe's text: `\startuml` with the bad line at input 3 is "line 4"; `@startuml` with `@Enduml` at input 3 is "line 3".
   - Rust prints a block-relative line on stderr (2) and no error page.
   - Error pages are not scored, so this is residue.
3. **Three-way:** see above.
4. **The 11,183 lock and the doc pins are genuine.**
   - Baselines: `32684f81e` 11,181; `28af44ab3` 11,182 (G4's rendering 96, edge-cases 759); `1ad869dae` and the tip 11,183 (edge-cases 760). Nothing is unlocked.
   - My replay finds the same 68 eligible golden failure names at the tip and the base (`gates/no-oracle-failures-tip.txt`, ditaa PNG references excluded).
     - Against review-5's 69, only `splines_multilevel_inheritance` (G4) has left the list.
     - Against `28af44ab3`, only `edge_pragma_useVerticalIf` (the round-5 lock) differs.
   - README, STABILITY, `--help-agent` and `doc_claims` all carry 11,183/11,251. The two remaining "11,182" strings are a historical attestation in `bullseye.yaml` and a coincidental SVG coordinate.
5. **Process:**
   - Addendum 3 (13:36:05) precedes `b24935a2d` (13:40:22); addendum 4 (13:40:13) precedes `ea5375293` (13:44:14). Both addenda touch docs only.
   - The non-test code adds no fixture-keyed literal: only `"ebnf"`, `"uml"` and the comment `"@start"`.
   - Against `28af44ab3`, `crates/rustuml-oracle` differs only in the `doc_claims` pin; `scripts` and `.gitmodules` do not differ; the non-perturbation `test-diagrams` differ only in `no_oracle_baseline.txt`.
   - Frozen items are unchanged: `GEOM_EPS`, markers, the park list, the error census, the allow-lists, the strict-XML list and the 15-source approved valid-error set.
   - `rejection_contract.py` reports 0 introduced losses in 18 of 19 cohorts against all three trees.
     - In DJ it flags `r5b_empty_bs_block` against `1ad869dae` and `9ed52f435`. Java gives a Welcome page there and the tip reproduces it, so under T14.63 it is a Welcome-match gain, not a loss.
     - Overlay classification additionally finds the lost rejection `r5b_start_bs_space_uml`, which the contract misses because `prepare.py` marks Java's "Diagram not supported" page valid.

## Gates (`gates/`; tip and base in `git archive` exports with the golden submodule symlinked)

| Gate | Result |
|---|---|
| Workspace lib | 1,679 passed, 0 failed |
| `pipe_mode` | 17/17 |
| `doc_claims` | 2/2 |
| `golden_no_oracle` | Pass, 11,183/11,251 at the tip and at `1ad869dae` |
| `golden_pairs` | 10,498/753/1,299 at the tip and the base, identical per family |
| `parity_perturbations` | 3,793 failing at the tip and 3,794 at the base. 0 tip-only failures; base-only `tim-line-continuation/cr2_seq_startUML_case_end_merged` is a gain. Of 156 TIM perturbations, 125 pass |
| `harness_guards` | 5 pass. `decimal_literal_ratchet` and `parity_review_records_are_complete` fail identically at the tip and the base (logs identical modulo the export path) |
| clippy `-D warnings` | `rustuml-layout` and `rustuml-math` pass. `rustuml`, `rustuml-parser`, `rustuml-render` and `rustuml-oracle` fail on the unused `looks_like_description_bracket_command`, which is also unused at `28af44ab3` and `1ad869dae` |
| `rejection_contract.py` | See finding 5 |

## Files

- `../review-6.json`
- `results/`:
  - `three-way-table.json` and `three-way-table-round5-classification.json`;
  - `summary.json.gz` and `summary-round5-classification.json.gz`;
  - `r5b-percase.json` and `dh-percase.json`;
  - `plus1-line-shift.json` and `binaries.sha256`.
- `merge/worse-vs-{1ad869dae,9ed52f435,28af44ab3}.json`
- `rejection/`, `gates/` and `manifests/`
- `pending-request-r5b/` (committed in `b31b0168d`, before the relay) and `request-dj/SHA256SUMS`
- `support/`. `unchanged-checker-scripts.sha256` records `replay.py`, `classify.py`, `rejection_contract.py` and `prepare.py`, identical to review-5's.
