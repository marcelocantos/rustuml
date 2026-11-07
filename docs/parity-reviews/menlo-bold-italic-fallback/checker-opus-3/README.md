# Checker review 3: Menlo Bold Italic fallback (🎯T14.50), integration re-check

Verdict: **accepted**, confirming review 2. The JRS fallback advances survive the merge unchanged. The merge loses nothing that either parent had.

See `../review-3.json` for the findings and `scoreboard.json` for the numbers.

## Identities

- **Merge `6484bb7e030828eb2ac3913aea27b5a48add2374`.** A single-parent squash
  onto parity-recovery `32684f81e4df2cf10bec9a74d935ea8ee944b5f0`. It carries
  `review/opus-g3-recheck` `4761d217bbcd35add29e0393ba559c87e7b87d6d`, but
  `4761d217b` is not an ancestor.
- **Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`.** Relayed results only;
  no Java was run locally.

## Merge reconstruction

`git merge-tree 32684f81e 4761d217b` conflicts only in `text_render.rs`. The
committed tree differs from that automatic result only in `text_render.rs`:

- The three conflict hunks keep G3's `style_face` call.
- `style_face` takes bold and italic from `effective_bold` and
  `effective_italic`, so an atom with `owns_font_configuration` keeps its own
  face bits.
- The test fix binds `dialog_base` once.

Every other changed file equals one parent byte for byte, or carries every line
from both parents (`creole.rs`).

## Ratchet

The guard's own counting rule gives these `text_render.rs` counts:

| Tree | Count |
|------|-------|
| 505999cac | 51 |
| 32684f81e | 68 |
| 4761d217b | 52 |
| 6484bb7e0 | 68 |

Every file count at the merge equals `32684f81e`, and the
`decimal_literal_ratchet` listing is identical.

## No-loss evidence

The CLI was run with `compare_svg_strict` on the whole SVG.

| Cohort | Valid / cases | 32684f81e | G3 tip | Merge | Bytes |
|---|---|---|---|---|---|
| CZ | 68/68 | 24 | 51 | 51 | = G3 tip, 68/68 registered predictions |
| CN | 46/46 | 8 | 38 | 38 | = G3 tip |
| CG | 6/6 | 1 | 6 | 6 | = G3 tip |
| CQ | 7/7 | 0 | 7 | 7 | = G3 tip |
| CR | 14/14 | 0 | 13 | 13 | = G3 tip |
| CX | 14/14 | 3 | 11 | 11 | = G3 tip |
| CH | 17/20 | 13 | 1 | 13 | = 32684f81e |
| CM | 40/48 | 15 | 4 | 15 | = 32684f81e |
| CS | 25/25 | 25 | 3 | 25 | = 32684f81e |
| CU | 13/18 | 7 | 2 | 7 | = 32684f81e |
| G2 checker | 28/28 | 0 | 0 | 0 | no element loss vs 32684f81e |
| Goldens | 12,550 | 11,179 | 11,179 | 11,179 | 1 change (a `%date()` timestamp) |
| Perturbations | 6,405 | 3,726 | 3,708 | 3,771 | 0 losses vs either parent |

The cases marked invalid are Java exit-0 crash pages, plus two exit-200 cases
in CM.

## Gates at the merge

- `golden_no_oracle` passes at 11181/11251.
- `golden_pairs` gives 10497/754/1299, identical to `32684f81e`.
- `parity_perturbations` fails as at `32684f81e`:
  - 2,662 failing paths at `32684f81e` and 2,640 at the merge.
  - 25 are fixed.
  - 3 newly fail. They are the font-name residue pairs, which do not exist at
    `32684f81e`.
- `pipe_mode` 17/17.
- `doc_claims` 2/2.
- Workspace lib tests: 0 failed, with 1 ignored in render.
- Clippy warning sets are identical per crate at all three trees.
- `harness_guards`:
  - Five categories pass.
  - `decimal_literal_ratchet` and `parity_review_records_are_complete` fail
    exactly as at `32684f81e`. This record adds no lines.

## Pairs

- All 7 pairs under `test-diagrams/perturbations/menlo-bold-italic-fallback/` pass at the merge and are named in `../review-3.json`.
- The 5 failing residue cases stay under `../checker-opus-2/residue/`, outside the executable walk.
