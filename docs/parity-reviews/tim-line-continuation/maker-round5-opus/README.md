# TIM line continuation (🎯T14.22) round 5: maker record (completion of the Grok draft)

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/opus-tim-continuation-round5`. This completes the round-5 draft by Grok (account `10c9fe37b`, draft `790726291`, `../maker-round5-grok/account.json`). An independent checker must review it; nothing here is a completion claim. Java was never run locally.

## Commits

| Commit | Role |
|---|---|
| `10c9fe37b` | Grok round-5 account (before the draft) |
| `790726291` | Grok draft: it40 on every line, definelong buffers it40 text, per-body theme join, Unicode 15.0 letter table, `unicode-properties` removed from `rustuml-parser` |
| `9dc1af5f9` | Merge of parity-recovery `32684f81e` (G1, G6, G2). No conflicts. `bullseye.yaml` is parity-recovery's text; the draft line made no bullseye edit. **Merged base for measurement.** |
| `5f74f24fa` | Addendum 1 (before any code change or measurement): draft review, UCD generator, plan for the `checkEndingBackslash` gate |
| `29c930a5f` | Gate: a theme body joins only when `checkEndingBackslash` is set; names the table generator |
| `6e2034c6f` | Addendum 2 (post-measurement, before the change): held-out probes show the per-body join leaves an open theme tail and a nested-theme body unjoined |
| `c4b5a8436` | **Tip code.** One post-TIM join over Java's result-list order (`PostTimJoin`, `JoinStream`), replacing the per-body join |
| `f9a6a6086` | Pre-registration of held-out batch or5 (13 cases, 8 resources) with Rust outputs from six builds |
| `8bdfff0e3` | Lock no-oracle 11,182/11,251 and move the help text, README, STABILITY and `doc_claims` pins |

## Obligations

1. **Inner-comment stripping on every line: done (draft code; verified).** `TContext#buildCodeIterator` (tim/TContext.java:325-330) wraps it30 (`CodeIteratorShortComment`) in it40 (`CodeIteratorInnerComment`). `CodeIteratorInnerComment#peek` (tim/iterator/CodeIteratorInnerComment.java:46-53) returns `removeInnerComment()` with no diagram-type test, and the procedure, legacy-define, while and foreach iterators read it40's lines. `execute_one_line` now always computes `line_no_comment = remove_inner_comment(line)`; the definelong, function, sub, while and foreach collectors all buffer that text. `ck4_ebnf_trailing` and `ck4_tim_definelong_body` match at the tip, and pass as registered perturbations.
2. **Local-theme bodies get Java's join: done.** `TContext#executeTheme` (TContext.java:725-745) runs the theme body through `executeLines` into the same result list. `UmlSource#loadInternal` (core/UmlSource.java:118-134) joins that one list in order, only when `checkEndingBackslash` is set (`PSystemBuilder#createPSystem`, PSystemBuilder.java:210-212: `findStartTypes` contains SEQUENCE). The tip carries one pending join across the top-level stream and every theme body in list order, gated on that flag. `tc1_seq_theme_local_postjoin` **matches** at the tip. It was a mismatch at `cb550c0d6` and `32684f81e`, and a valid-input error at `9ed52f435`.
3. **Unicode 15.0 letter table: done (draft table; verified).** `support/gen_bmp_letters.py` regenerates `BMP_LETTER_RANGES` from UnicodeData.txt 15.0.0 (sha256 `806e9aed…6a73`). It finds 48,965 BMP letters in 380 runs, equal to the Rust table. Against 17.0.0 (sha256 `2e1efc1d…470c`) it lists exactly the 16 added letters, and none removed. BMP Nd is 370 code points in both versions. The unit test guards the 16 code points. `unicode-properties` remains in Cargo.lock only as rustybuzz's dependency. The four probes `cr4_hdr_u16_cyrillic_tje`, `cr4_hdr_u16_latin_a7cb`, `cr4_hdr_u17_latin_a7ce` and `cr4_hdr_u17_arabic_088f` match.
4. **Lock: done.** `edge-cases 759 → 760` (`edge_pragma_useVerticalIf`); the total is 11,182/11,251 (99.4%).

## Measurement

**Method.**
- Builds: fresh git-archive release builds, one `CARGO_TARGET_DIR` each, deleted once the binary was saved (`results/binaries.sha256`). `cb550c0d6` is the round-4 checker's binary; its hash equals that checker's record, and it was replayed on BU only.
- Comparator: `compare_cli` from the unchanged source, linked against `32684f81e`'s `rustuml_oracle`.
- Scripts: `replay.py`, `classify.py` and `rejection_contract.py` are unchanged; their hashes equal the round-4 checker's copies (`support/unchanged-checker-scripts.sha256`).
- Manifests: the round-4 checker's prepared manifests. Every Java SVG and source hash was re-verified against the parent-validated `reference-results.json`: all 14 fresh batches and the 790 match, with 0 mismatches.
- Outcomes: 4 modes, each case run from its recorded cwd, scored with the T14.63 welcome-match overlay (`support/summarize_r5.py`).

The table shows worse/gains for each comparison. "Worse" means a lost match, lost rejection, lost Welcome match, new valid-input error, panic or timeout. The tip is `c4b5a8436`.

| Cohort | n | tip match | Welcome match (Java Welcome) | vs `9dc1af5f9` | vs `9ed52f435` | vs `32684f81e` | vs `d28dd64a9` |
|---|---:|---:|---:|---|---|---|---|
| CC | 74 | 18 | 8 (9) | 0/0 | 0/4 | 8/19 | 0/15 |
| CB | 39 | 24 | 6 (6) | 0/0 | 0/0 | 0/36 | 0/28 |
| CA | 88 | 49 | 16 (19) | 0/0 | 0/0 | 3/70 | 0/25 |
| BZ | 23 | 13 | 5 (5) | 0/0 | 0/0 | 0/18 | 0/1 |
| BY | 53 | 26 | 5 (5) | 0/0 | 0/0 | 1/36 | 0/2 |
| BW | 23 | 15 | 0 | 0/0 | 0/0 | 1/18 | 0/1 |
| BU | 40 | 32 | 0 | 0/0 | 0/1 | 0/31 | 0/3 |
| BD | 33 | 25 | 0 | 0/0 | 0/0 | 1/22 | 0/2 |
| BA | 64 | 52 | 0 | 0/0 | 0/2 | 1/12 | 0/11 |
| AY | 48 | 36 | 0 | 0/0 | 0/0 | 0/1 | 0/2 |
| AZ | 8 | 5 | 0 | 0/0 | 0/0 | 1/0 | 0/2 |
| AX | 77 | 27 | 0 | 0/0 | 0/0 | 2/6 | 0/2 |
| AV | 63 | 20 | 0 | 0/0 | 0/0 | 2/10 | 0/0 |
| AT | 45 | 20 | 0 | 0/0 | 0/0 | 1/8 | 0/1 |
| 915 (AS, AQ, 790, 28) | 915 | 249 | 0 | 0/0 | 0/0 | 2/334 | 0/2 |
| retained | 4,174 | 2,742 | 0 | 0/0 | 0/1 | 13/331 | 0/77 |
| golden | 12,550 | 11,182 | 17 (17) | 0/0 | 0/0 | 0/126 | 4/8 |

- **Against the merged base `9dc1af5f9`:** 0 worse and 0 SVG byte changes in every cohort. The base already contains the draft, so its fixes appear against the parents. The only model changes are 8 golden DOT AST/YAML key-order differences (HashMap order), plus `sprite_multiple` YAML.
- **Against `9ed52f435` (round-4 code):** 0 worse. 8 gains:
  - the four `cr4_hdr_*` probes;
  - `tc1_seq_theme_local_postjoin`;
  - `ck4_ebnf_trailing` and `ck4_tim_definelong_body`;
  - retained `bd087_s_attribute_quoted_identifier_49` (from parity-recovery).
- **Against `32684f81e` (parity-recovery):** 36 worse, all inherited.
  - `9ed52f435` has the same 36 plus 5 more: the four Unicode probes and `tc1`. The tip adds none (`results/summary-9ed52f435-as-tip.json.gz`).
  - Every one of the 36 is in the round-4 review's list against `cb550c0d6`:
    - the retained 13, identical to that list;
    - CC `!exit`, BOM/en dash in the main source, lone CR, YAML header and `!N` suffix (8);
    - CA's T14.66 pair and T14.21;
    - T14.26 (BY) and T14.21 (BW);
    - the usecase/class strict-admission cases in BD, BA, AZ, AX, AV, AT and the 915.
- **Against `d28dd64a9`:** 4 worse, all golden and all inherited from parity-recovery; `9ed52f435` and the base have them too.
  - `act_skinparam2_shadowing_true`, `act_skinparam_shadowing` and `edge_whitespace_multiple_blank_lines` also mismatch at `32684f81e`.
  - `preproc_foreach_nested` is the accidental rejection the round-4 review noted.
- **Panics:** only the `--yaml` set: retained 63 and golden 148 at the tip, the base and `9ed52f435`; 72 and 160 at `32684f81e`. **Timeouts:** 0. One `compare_cli` timeout under CPU load killed the first `d28dd64a9` 915 replay; it was re-run alone and completed.
- **Welcome matches:** CC 8/9 (not `cr4_cnt_exit_alone`), CB 6/6, CA 16/19 (the T14.66 trio), BZ 5/5, BY 5/5, golden 17/17.
- **`rejection_contract.py`** (`results/rejection/`): 0 introduced losses in all 15 sets against `9dc1af5f9`, `9ed52f435` and `32684f81e`. The 915: 304 rejections at the base and at `9ed52f435`, against 15 at `32684f81e`; 304 at the tip.

## Gates

Gates ran at worktree `f9a6a6086` (code `c4b5a8436`) and at a `9dc1af5f9` export.
- **Workspace lib:** 1,668 passed, 0 failed.
- **`pipe_mode`:** 17/17 at the tip, and again 17/17 after the lock; 17/17 at the base.
- **`doc_claims`:** 2/2 before and after the lock.
- **`golden_no_oracle`:** 11,182/11,251 at the tip and the base. Before the lock it failed only on the unlocked edge-cases line (759 → 760); after the lock it passes.
  - The 69 failure names are identical at the tip, the base and `9ed52f435`.
  - Against `32684f81e`, the only difference is `edge_pragma_useVerticalIf`, newly passing.
- **`golden_pairs`:** 10,498/753/1,299 at the tip and the base, with an identical per-directory summary.
- **`parity_perturbations`:** 3,835 failing pairs at the tip and the base, the identical name set.
- **`harness_guards`:** 5 pass. `decimal_literal_ratchet` (class, deployment, plantuml_metrics, state, text_render) and `parity_review_records_are_complete` (3,095 lines, including `review-4.json` not accepted) fail identically at the tip and the base. The normalised outputs are equal.
- **Frozen items:** not touched. No changes under `crates/rustuml-oracle` apart from the `doc_claims` total pin, and none under `scripts`.

## Pending relay: batch or5

- Directory: `~/Library/Caches/rustuml-opus-tim-r5/or5/`, read-only.
- `reference-manifest.json`: sha256 `fd00e4dc8429ab6399b05c4030a58553a0b11ee87340d06443d394c4c1811ce0`.
- `reference-input-bundle.json.gz`: sha256 `e6114a322edd01c6fa2e7808dc754e43cc8902b14ecaa2170554eaca83da812a`.
- Rust outputs: `pending-request-or5/or5-rust-predictions.json` (sha256 `07d1389c…c41d`).
- Java predictions (`intended_valid`): 11 valid; `or5_theme_mindmap_continued` and `or5_theme_wbs_continued` predicted Java syntax errors, because there is no join without SEQUENCE.

## Residue

- `JavaOnlyLines` (an unshipped Java theme) has no text in Rust, so the join passes over it.
- The EBNF short-comment (`'`) exemption is parity-recovery's and is unchanged. Java's `CodeIteratorShortComment` has no EBNF test (proposed target).
