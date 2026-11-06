# TIM line continuation (🎯T14.22) round 4: independent checker review

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-round4-claude`. It reviews maker tip `fa3f95010`: account `9ee6c154c`, addenda `874a3b3f0` and `96954ed40`, draft `3ae71ad99`, code `9ed52f435`, merged base `1f6ffa9f4`.

Records:
- `../review-4.json` (this mechanism).
- `../../sprite-command-post-tim-ownership/review-8.json` (whole-line addendum).

Earlier reviews are preserved. Java was never run locally.

**Verdict: rejected.**
- The round-4 code loses nothing against the merged base, and its four obligations are accepted within their scope.
- The parity-recovery merge `1f6ffa9f4` does not keep everything from both sides. It has three merge losses. Two are registered pairs; the third is a new valid-input error that both parents avoid.

## Pre-registration and relay

Batch CR4 (74 cases, 21 resources) was pre-registered in `a01ab9853`, before any Java result existed (`pending-request-cr4/`). It was relayed as **CC**; the hashes are in `request-cc/SHA256SUMS`, and `reference-results.json` is `2f6b7b68…`.
- **Validity** comes from the raw Java SVG and stderr (`prepare.py`), plus the brief's rule that Java exit 100/50 is invalid. That rule applies to one case, `cr4_cnt_pragma_continued_end`. `manifests/fresh-cc-manifest.prepare.json.gz` is the unpatched file.
- **Counts:** 35 Java-valid, 9 Java Welcome pages, 30 other invalid.
- **My Java predictions were wrong on 12 of the 74:**
  - `!PRAGMA` renders.
  - The continued `!pragma` line swallows the end line in the raw merge (exit 100).
  - An empty include is a Java fatal error.
  - `!assume` alone and an unshipped theme alone render.
  - A two-line mindmap whose end line a definition swallows crashes Java.
  - NBSP before `!$x =` is a Java error (a001).
  - `!1!2` and both quoted include paths give "cannot include".
  - An unterminated `!startsub` gives "Compile Error null STARTSUB".
  - Unterminated `!foreach`, `!while` and active `!if` render `one`. Every Rust build matches these three.

## Causes of rejection: merge losses

Method: a three-way comparison per case and mode (`merge/merge3.json`, `merge/merge3-outcomes.json`).
- Ancestor: `9834a7194`, which has the same code as the merge base `adf56ce53`.
- Sprite side: `d28dd64a9`, from the round-3 checker's runs.
- parity-recovery side: `cb550c0d6`.

Of all registered pairs that existed at the round-3 tip, exactly two fail now (`gates/registered-at-round3-now-failing.txt`).

| Case | `d28dd64a9` | `cb550c0d6` | `1f6ffa9f4` / tip | Cause |
|---|---|---|---|---|
| `ck4_ebnf_trailing` (registered, sprite dir) | match | mismatch | mismatch | Java's `CodeIteratorInnerComment` (it40) runs `removeInnerComment` on every line. The merge kept parity-recovery's EBNF exemption. |
| `ck4_tim_definelong_body` (registered, sprite dir) | match | mismatch | mismatch | The definelong body is buffered from the raw line, not from the `removeInnerComment` text. |
| `tc1_seq_theme_local_postjoin` (BU, Java-valid) | mismatch | mismatch | **valid-input error** | Local-theme body lines go to `theme_tail`, which the post-TIM `UmlSource#loadInternal` join never sees. `#DD0000` reaches the parser. |

Every other three-way difference takes one parent's outcome. The sprite-extraction drop is clean: no sprite case is worse than both parents.

## Per-mechanism verdicts (round-4 code)

**1. Line count over Java's TIM list: accepted.**
- CB: the 5 pragma cases and the shipped theme gain. All 6 Java Welcome pages are reproduced.
- CC:
  - The include-pragma tail gains.
  - The Welcome pages for `!option`, `!log`, `!assert`, `!undef`, the empty sub and the comment-only local theme match.
  - `!PRAGMA` (Java renders) mismatches at every build.

**2. One-line sources: accepted for `@startuml`.** Java's `UmlSource#isEmpty` is wider than this rule:
- `@startuml` bodies of blank, whitespace-only or TIM-empty lines, and an include of one blank line, are Java exit-200 Empty description. Every build shows Welcome with exit 0.
- Mindmap, gantt and wbs one-line sources are Java Empty description. Every build renders them.

**3. Whitespace at the `@end`/`\end` tests: accepted.** Other sites still differ:
- TIM directive and comment detection uses `trim()`, where Java's `TLineType` uses `[\s ]*`. Four U+3000 cases are false accepts at every build.
- `Sub#fromFile`'s marker test.
- NBSP before an assignment (a001).
- `!exit` (`StartUtils#isExit`) is unported.

**4. Include reader, header test and `!N`: accepted.**
- Gains: CB 8, CC 4 (`!`, `!0`, `_once` twice, and the en dash in an `!includesub` body).
- Unicode residue: `unicode-properties` 0.1.4 is Unicode 17, and JDK 21 is Unicode 15.0. Sixteen BMP code points differ. All four probes (U+1C89, U+A7CB, U+A7CE, U+088F) are Java-valid and Rust valid-input errors. `Cargo.toml`'s `0.1` lets `cargo update` move to 0.1.5 (Unicode 18).

## Numbers (tip `9ed52f435`; T14.63 welcome-match overlay; `summary.json`)

| Cohort | n (valid; Welcome) | tip match + Welcome match | vs `1f6ffa9f4` | vs `93bf53e18` | vs `9834a7194` | vs `cb550c0d6` |
|---|---:|---:|---|---|---|---|
| CC | 74 (35; 9) | 14 + 8 | +5, +2 Welcome | +8, 2 newly rejected | 12 new valid errors, 1 lost Welcome match | same, plus 1 lost match (accidental) |
| CB | 39 (25; 6) | 24 + 6 | +19, +2 Welcome, 5 newly rejected | +24 | +23 | +23 |
| CA | 88 (58; 19) | 49 + 16 | +11, +2 Welcome, 6 newly rejected | +48 | 3 new valid errors | 3 new valid errors |
| BZ, BY | 23, 53 | 13+5, 26+5 | 0 bytes | gains | BY 1 new valid error | same |
| BW, BU, BD, BA, AY, AZ, AX, AV, AT | 401 | — | 0 bytes | `mc1_seq_foreach_body` (lost rejection); `ck3_svg_inline_quoted_apostrophe_seq` (lost rejection); BW and BU 1 new valid error each | known set | known set |
| 915 (AS, AQ, 790, 28) | 915 (602) | 249 | 0 SVG bytes | +61 | 2 new valid errors (known) | same |
| retained | 4,174 | 2,741 | 0 SVG bytes | +96 | the approved 13 | the approved 13 |
| golden | 12,550 (Welcome 17) | 11,182 + 17 | +1 (`edge_pragma_useVerticalIf`) | 3 lost matches (inherited, `cb550c0d6` mismatches), 1 accidental lost rejection | 3 inherited lost matches | +1 |

Panics: only the unchanged `--yaml` set (retained 63, golden 148); `9834a7194` has 72/160. Timeouts: 0.

**Rejection contract** (`rejection/`): 0 introduced losses against `1f6ffa9f4`, `9834a7194` or `cb550c0d6` in any cohort. Against `93bf53e18` it lists:
- `mc1_seq_foreach_body` (T14.52);
- `ck3_svg_inline_quoted_apostrophe_seq` (T14.32);
- five reproduced Java Welcome pages, which count as matches.

## Gates (`gates/`)

- **Workspace lib:** 1,660 passed, 0 failed.
- **`pipe_mode`:** 16/17. `help_agent_discloses_no_oracle_product_metric` fails because the tip has `f70f107b8`'s 11,175. `cb550c0d6` passes 17/17, so `cb550c0d6` needs merging.
- **`golden_no_oracle`:** 11,182/11,251 at the tip, 11,181 at `1f6ffa9f4`. The only difference is edge-cases 759 → 760, which is unlocked.
- **`golden_pairs`:** 10,498/753/1,299 at the tip, 10,497/754/1,299 at the base.
- **`parity_perturbations`:**
  - 3,890 failures at the tip and 3,895 at the base: 0 new, 5 fixed.
  - After registration: 3,903, the 13 registered pairs that fail. The harness agrees with `compare_cli` on all 34.
  - Against the round-3 tip, the only new failures among pairs registered then are the 2 merge losses.
- **`harness_guards`:** 4 pass. `decimal_literal_ratchet` and `parity_review_records_are_complete` fail identically at the tip and the base (normalised records sha `72fc42ed…`).

## Process and frozen items

- The account precedes the draft, and addendum 2 precedes the final code. The account commits touch docs only.
- Probe strings appear only in `#[cfg(test)]`.
- `crates/rustuml-oracle`, `crates/rustuml`, `scripts`, `.gitmodules` and the non-perturbation `test-diagrams` are identical to parity-recovery `f70f107b8`. Every frozen item is unchanged.

## Files

`summary.json`, `merge/`, `rejection/`, `gates/`, `manifests/`, `probes/cc-percase.txt`, `registered-pairs.json` (34 CB/CC pairs), `evidence-only-cases.json` (79), `verify-fresh.json`, `binaries.sha256`, `pending-request-cr4/`, `request-cc/`, `support/`. `support/unchanged-checker-scripts.sha256` records the unchanged scripts.
