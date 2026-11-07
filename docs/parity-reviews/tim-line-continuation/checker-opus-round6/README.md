# TIM line continuation (🎯T14.22) round 6: re-check of the Pipe/getTypes/UncommentReadLine correction (Opus)

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/opus-tim-round6`. It reviews maker tip `7e13632cd`:
- code `59d78989f`;
- addendum 5 `c23d43b04`;
- pre-registration `c7d511ff0` (batch o6n, relayed as DK).

The merged base is `c25ccbf3c` (parity-recovery `9fc567332` merged in). The parents are round-4 code `9ed52f435` and parity-recovery `9fc567332`. The record is `../review-7.json`. Java was never run locally.

**Verdict: rejected. The packet is not accept-ready for local merge.**

The round-6 code causes no loss against the merged base, and its three obligations hold on every clean probe. It is still worse than parity-recovery `9fc567332` on 5 cases outside the approved set, the 4 named inherited cases and `chen`. Two of those 5 sit inside obligation 1's own mechanism, and one inside obligation 2's.

| Against | Worse | Composition |
|---|---:|---|
| merged base `c25ccbf3c` | **0** | 87 other changes, all gains or neutral |
| `9ed52f435` | **1** | `r5b_start_bs_chen` (ruled below: not a loss) |
| `9fc567332` | **48** | 38 approved (review-5), 4 named inherited, `chen`, **5 outside** |

## The 5 cases outside the allowed set (all worse than `9fc567332`; tip = merged base = `9ed52f435` on each)

1. **DK `o6n_format_line_bs_seq` (the maker's own held-out case) and DL `q7_num_us_blank_bs_seq`: obligation 1 is only half ported.**
   - `Pipe#readSingleDiagram` never appends a `@@@format ` line (Pipe.java:188-189), nor a leading line whose `String#trim` is empty while it is in NO_CONTENT (191, 207).
   - Java renders both messages at `data-source-line` 2, which proves the line is gone from Pipe's text.
   - `59d78989f` skips such lines only when it numbers lines (`pipe_text_positions`). `merge_ending_backslash_lines` still keeps `input[..pipe.kept]` whole, so the `@@@format svg` or U+001F line reaches strict sequence admission, and the tip errors with "no registered sequence command accepts".
   - `9fc567332`'s lenient admission rendered a mismatch.
   - No golden contains a `@@@format ` line or a trim-empty control-character lead line, so dropping these lines is golden-safe.
2. **DL `q7_type_bs_dotless_mindmap` (`\startmındmap`): obligation 2's case fold.**
   - `DiagramType#check` is `String#regionMatches(true, …)`. It compares `Character.toUpperCase` per character, so U+0131 matches `i` and Java draws MINDMAP.
   - Rust's `start_type_is` uses `eq_ignore_ascii_case` on a byte prefix, so the tip reports an unsupported type.
3. **DL `q7_hdr_bom_content`: inherited, 🎯T14.111.** `ReadLineReader` strips U+FEFF from every line, so Java renders. Every TIM-line build errors on the BOM-prefixed content line.
4. **BY `cr2_seq_two_blocks_first_swallowed`: inherited, review-6's 7th inherited case.** Java gives an exit-0 "No valid @start/@end found" page. Every TIM-line build reports the same text as an error (exit 1). `9fc567332` produces output the comparator cannot score (exit 3). The maker's statement that it now "equals the head" is false.

**Pairs the maker must pass** (source and Java SVG in the immutable relays):
- `rustuml-relay-parent/opus-tim-r6-dk/`: `o6n_format_line_bs_seq`;
- `rustuml-relay-parent/opus-tim-r6-recheck-heldout/`: `q7_num_us_blank_bs_seq` and `q7_type_bs_dotless_mindmap`;
- unless the owner adds them to the inherited set:
  - `q7_hdr_bom_content` (same relay);
  - BY `cr2_seq_two_blocks_first_swallowed` (`tim-continuation-cr2/`).

## Ruling on `r5b_start_bs_chen`: not a loss

- Java draws CHEN_EER.
- `9ed52f435` and `9fc567332` draw a diagram with `data-diagram-type="CLASS"`: `\startchen` was not a start line there, so the entity block was read as an accidental class diagram. That is a wrong-type mismatch.
- The tip, like the merged base, recognises the CHEN start and reports "unsupported diagram type: @startchen". That is exactly what `@startchen` gives at every build.
- Neither tree matches Java. Re-gaining the old output would need a fallback that re-creates a wrong diagram type.
- I accept the declared exception. The case stays residue under 🎯T14.111, where Chen dispatch is an owner decision.

## Claim checks

- **Obligation 1 (numbering)** holds on every clean no-mark probe. DL gains against the merged base:
  - sequence, class and state continuations;
  - `@@@format` between class lines;
  - a CRLF tab and blank lead line;
  - usecase and component;
  - a TIM function and `!if`.
  
  U+3000 and NBSP lead lines count as content, as `String#trim` counts them. Raw and NBSP-wrapped includes match. The marked controls are unchanged.
- **Obligation 2 (`getTypes`)** holds on every ASCII probe: `regexp`, `ebnfx`, `nwdiagram`, `yamlfile`, `UMLdiagram`, `PROJECT`, mixed-case salt, a named wbs, and an inner `@startjsonx`. `@start<TAB>uml` and `\startumm` are rejected, as Java rejects them.
- **Obligation 3 (`UncommentReadLine`)** holds on every header probe:
  - headers of U+2007, U+202F, ZWSP, NBSP+tab and NBSP (state);
  - a `'` header, alone and with a continuation;
  - a `<q>` header with a continuation;
  - a `-` header, two `<…>` groups, and an unclosed `<a ` before the directive;
  - a `'` header that turns a comment into content, rejected as in Java;
  - `~@startuml`, rejected;
  - a commented inner `'@startmindmap`, which becomes MINDMAP as in Java.
- **Maker counts:**
  - "0 worse than `c25ccbf3c`" and "1 worse than `9ed52f435`" are confirmed, DK and DL included.
  - "43 worse than `9fc567332`" is false. It is 45 on the maker's own cohorts (DK `o6n_format_line_bs_seq` and BY `cr2_seq_two_blocks_first_swallowed`), and 48 with DL.
- **Addendum 5's "every golden" has a Pipe mark: false.**
  - The 10 `type-detection/noheader_*` goldens have no mark.
  - They match at all four builds, because `PicoWebServer` (278-279) wraps any source that does not start with `@start` exactly as Pipe does.

## Is not porting the discard before a column-0 mark sound?

**Yes, for numbering.**
- The goldens come from picoweb (`scripts/generate-golden.sh` POSTs to `/render`). It prepends `@startuml` to the 8 `multi-diagram/preproc_*` sources, so they are numbered by file line: `preproc_01_define_both_blocks` has `data-source-line` 5 for file line 5.
- `-pipe` instead discards those lines (`sb.setLength(0)`).
- One Rust library path serves both entry points. Porting the discard needs a `-pipe`-only mode (proposed target 3).

**The oracles also disagree on content.**
- The goldens show the definitions before `@startuml` applied.
- `-pipe` Java drops a variable or include placed before `\start`. In DL `q7_num_def_before_bs` the label is literally `$who`, and `q7_num_inc_before_bs` shows no included participant.
- Rust's preamble keeps both. These are mismatches at all four builds: residue, not losses.

## Relays

| Batch | Directory | `reference-results.json` | Cases | Pre-registered |
|---|---|---|---|---|
| DL (checker Q7) | `rustuml-relay-parent/opus-tim-r6-recheck-heldout/` | `96404d71…` | 58 (3 resources) | `a13482ed6`, before the relay |
| DK (maker o6n) | `rustuml-relay-parent/opus-tim-r6-dk/` | `5da36b54…` | 36 | `c7d511ff0`, before the relay |

- **Re-rendering:**
  - DK: the maker's `o6n-rust-predictions.json` (`37f02451…`) re-renders byte-identically, 144/144 (36 cases × 4 builds).
  - DL: my predictions re-render 232/232.
- **Held-out check:** 0 DL hits among 298,854 corpus files (goldens, perturbations, `docs/`, every other `rustuml-*` cache).
- **DL Java results:** 50 exit 0 and 8 exit 200. All 58 validity predictions were correct under the declared overlay.
- **DL at the tip:**
  - 43 of 50 valid cases match.
  - 7 of 8 invalid cases are rejected. The exception, `q7_err_bs_class_line`, is a false accept at every build.
- **DL against the merged base:** 26 gains, 2 newly rejected, 0 worse.

## Three-way comparison (T14.63 Welcome-match overlay plus review-6's validity overlay; `results/three-way-table.json`)

"Worse" means a lost match, lost rejection, lost Welcome match, new valid-input error, panic or timeout. Each cell gives worse/other changes. The tip is `7e13632cd`.

| Cohort | n | Java-valid | tip match | Welcome match | vs `c25ccbf3c` | vs `9ed52f435` | vs `9fc567332` |
|---|---:|---:|---:|---:|---|---|---|
| DL (checker held out) | 58 | 50 | 43 | 0 | 0/28 | 0/35 | **3**/37 |
| DK (maker held out) | 36 | 33 | 30 | 0 | 0/24 | 0/23 | **1**/17 |
| DJ | 57 | 47 | 40 | 1 | 0/25 | **1**/33 | **5**/29 |
| DH | 28 | 26 | 23 | 0 | 0/9 | 0/21 | 0/15 |
| DF | 72 | 57 | 45 | 0 | 0/0 | 0/12 | **1**/28 |
| DD | 13 | 12 | 10 | 0 | 0/0 | 0/9 | 0/9 |
| CC | 74 | 35 | 18 | 8 | 0/0 | 0/4 | **9**/19 |
| CB | 39 | 25 | 24 | 6 | 0/0 | 0/0 | 0/36 |
| CA | 88 | 58 | 49 | 17 | 0/0 | 0/1 | **3**/71 |
| BZ | 23 | 14 | 13 | 5 | 0/0 | 0/0 | 0/18 |
| BY | 53 | 37 | 27 | 5 | 0/0 | 0/2 | **2**/37 |
| BW | 23 | 20 | 16 | 0 | 0/1 | 0/1 | **1**/19 |
| BU | 40 | 37 | 32 | 0 | 0/0 | 0/1 | 0/31 |
| BD | 33 | 30 | 25 | 0 | 0/0 | 0/0 | **1**/22 |
| BA | 64 | 57 | 52 | 0 | 0/0 | 0/2 | **1**/12 |
| AY | 48 | 44 | 36 | 0 | 0/0 | 0/0 | 0/1 |
| AZ | 8 | 8 | 5 | 0 | 0/0 | 0/0 | **1**/0 |
| AX | 77 | 69 | 27 | 0 | 0/0 | 0/0 | **2**/7 |
| AV | 63 | 60 | 20 | 0 | 0/0 | 0/0 | **2**/10 |
| AT | 45 | 42 | 20 | 0 | 0/0 | 0/0 | **1**/8 |
| 915 (AS, AQ, 790, 28) | 915 | 602 | 249 | 0 | 0/0 | 0/0 | **2**/334 |
| retained | 4,174 | – | 2,746 | 0 | 0/0 | 0/5 | **13**/331 |
| golden | 12,550 | – | 11,183 | 17 | 0/0 | 0/1 | 0/126 |
| **Total** | 18,581 | 1363 | 14,733 | 59 | 0/87 | **1**/150 | **48**/1217 |

**Worse lists:** `merge/worse-vs-{c25ccbf3c,9ed52f435,9fc567332}.json`. Each row is flagged `in_review5_approved` or `named_inherited`, and each file carries an `outside_approved_named` list.

**Welcome:** 59 Welcome matches. Java-Welcome cases that the tip does not reproduce are unchanged from review-6 (DJ 1, CC 1, CA 2).

**Panics and timeouts:**
- There are 0 timeouts.
- Panics occur only in `--yaml` mode: retained 63 and golden 148 at the tip, the base and `9ed52f435` (72 and 160 at `9fc567332`).
- One panic is new at the tip: DL `q7_type_bs_ebnfx --yaml` ("serializing nested enums in YAML is not supported yet").
  - The input now dispatches to EBNF and matches in SVG.
  - 21 of the 25 EBNF goldens hit the same panic at the merged base.
  - It is existing behaviour that a newly dispatched input reaches, so I report it as residue.

## Process

- **Order:** addendum 5 (15:18:05) precedes code `59d78989f` (15:21:31) and touches docs only. The o6n pre-registration precedes the DK relay.
- **No fixture-keyed logic:** the added non-test literals are the `DiagramType` key table, `@@@format ` and comments.
- **Diff against `9fc567332`:**
  - `crates/rustuml-oracle` differs only in the `doc_claims` pin;
  - `scripts` and `.gitmodules` do not differ;
  - the non-perturbation `test-diagrams` differ only in `no_oracle_baseline.txt`;
  - perturbations are additions only.
- **Frozen items unchanged:** `GEOM_EPS`, markers, the park list, the error census, the allow-lists, the strict-XML list and the 15-source approved valid-error set.
- **`rejection_contract.py`:** 0 introduced losses in 62 of 63 runs (21 cohorts × 3 parents). The 63rd is DJ against `9ed52f435`, which flags `r5b_empty_bs_block`: the Welcome-match gain, where Java and the tip both show Welcome.

## Gates (`gates/`; the tip in the worktree, the base in a `git archive` export with the golden submodule symlinked)

| Gate | Result |
|---|---|
| Workspace lib | 1,708 passed, 0 failed |
| `pipe_mode` | 17/17 |
| `doc_claims` | 2/2 |
| `golden_no_oracle` | Pass, 11,183/11,251 at the tip and at `c25ccbf3c`. The 68 eligible failure names are identical; against `9fc567332`, only the round-5 lock `edge_pragma_useVerticalIf` differs. Lock and pins (README, STABILITY, `--help-agent`, `doc_claims`) are 11,183/11,251, the baseline file is unchanged from the merged base, and nothing is unlocked |
| `golden_pairs` | 10,498/753/1,299, output identical to the base |
| `parity_perturbations` | 3,727 failing at the tip and 3,728 at the base. 0 tip-only failures; the base-only `tim-line-continuation/r2_seq_indented_start_and_end` is a gain. Of 156 TIM perturbations, 126 pass |
| `harness_guards` | 5 pass. `decimal_literal_ratchet` (render-crate literals) and `parity_review_records_are_complete` (`sequence-page-wrapper-hierarchy`) fail identically at the tip and the base |
| clippy `-D warnings` | `rustuml-layout` and `rustuml-math` pass. `rustuml`, `rustuml-parser`, `rustuml-render` and `rustuml-oracle` fail only on the unused `looks_like_description_bracket_command`, which is also unused at `9fc567332` |
| `rejection_contract.py` | See Process |

## Proposed targets (not filed; `bullseye.yaml` untouched)

1. Lines that `Pipe#readSingleDiagram` never appends never reach TIM or a parser, whether or not the input is marked. These are `@@@format ` lines anywhere and leading `String#trim`-empty lines.
   - Witnesses: DK `o6n_format_line_bs_seq`; DL `q7_num_us_blank_bs_seq` and `q7_num_format_mid_marked`.
   - This belongs to T14.22's round-6 correction.
2. `DiagramType#check` folds case as `regionMatches(true)` does, beyond ASCII. Witness: DL `q7_type_bs_dotless_mindmap`.
3. `-pipe` mode reads Pipe's text, while picoweb and file mode keep file numbering, behind a CLI mode flag (owner decision). In `-pipe` mode, lines before a column-0 mark are discarded, and lines before a restarting start reach no block.
   - Witnesses: `r5b_num_comment_at_seq`, `o5s_ws_leading_blank_at`, DL `q7_num_def_before_bs` and `q7_num_inc_before_bs`.
   - Constraint: the `multi-diagram/preproc_*` goldens.
4. `--yaml` serialises nested enums without panicking. Witnesses: DL `q7_type_bs_ebnfx` and 21 EBNF goldens.
5. New witnesses for 🎯T14.111: DL `q7_hdr_bom_content` (per-line BOM), plus the `chen` ruling above.

## Files

- `../review-7.json`
- `results/`:
  - `three-way-table.json` and `summary.json.gz`;
  - `dl-percase.json` and `binaries.sha256`.
- `merge/worse-vs-*.json`
- `rejection/`, `gates/` and `manifests/`
- `pending-request-q7/` (committed in `a13482ed6`, before the relay) and `request-dl/SHA256SUMS`
- `support/`. `unchanged-checker-scripts.sha256` records `prepare.py`, `replay.py`, `classify.py`, `fresh_classify.py` and `rejection_contract.py`, identical to review-6's.
