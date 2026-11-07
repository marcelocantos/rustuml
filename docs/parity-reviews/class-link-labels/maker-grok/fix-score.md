# Class link labels — correction score

Scored tip `50cec1dd1` against the post-merge binary, then staged a new
held-out batch. Predictions were not edited after hashing. This is a
measurement, not a checker review.

## Binaries

Tip `b7bc4bddd540ba0cda31963c35260673ede64dea6b5ac57f31d9a4eb93097ab0`.
Merge base `5c035c88fb07be7a023c7b4e3e8ec5d82bbf1e7bcf787f77b3b240a27fca30f6`
(addendum `77461b8f2`, the parity-recovery merge). `golden_no_oracle` on the
tip is 11,182/11,251. Rendering stays 96/98.

## Zero loss

`compare_svg_strict` only when tip and merge stdout differ and Java exited 0.
Identical output cannot lose a match.

| Cohort | n | changed | lost | lost rejection | new error | tip panic |
|---|---:|---:|---:|---:|---:|---:|
| CO | 100 | 10 | 0 | 0 | 0 | 0 |
| CK | 13 | 0 | 0 | 0 | 0 | 0 |
| c9a | 288 | 0 | 0 | 0 | 0 | 0 |
| c11a | 515 | 0 | 0 | 0 | 0 | 0 |
| golden | 12550 | 4 | 0 | 0 | 0 | 0 |
| perturbations | 6365 | 0 | 0 | 0 | 0 | 4 |
| checker pairs | 8 | 0 | 0 | 0 | 0 | 0 |
| retained `c9a_af_*` | 38 | | 0 | | 0 | 0 |

The four perturbation panics (`dash_01`, `dash_03`, `dash_06`, `dash_07`)
also panic on the merge binary, exit 101, same stdout. They are not new.

CO pairs whose bytes moved:

| Case | Tip | Merge |
|---|---|---|
| `g4k_ortho_dl_cc_lr` | MATCH | DIFF |
| `g4k_nl_escaped` | MATCH | DIFF |
| `g4k_nl_url` | MATCH | DIFF |
| `g4k_nl_l` | MATCH | DIFF |
| `g4k_nl_r` | MATCH | DIFF |
| `g4k_ortho_lr_nl2` | MATCH | DIFF |
| `g4k_ortho_nl_l` | MATCH | DIFF |
| `g4k_ortho_nl_r` | MATCH | DIFF |
| `g4k_st_self` | MATCH | DIFF |
| `g4k_ortho_dl_cc_tb` | DIFF | DIFF |

`g4k_ortho_dl_cc_tb` was already a diff. The vertical route deltas and the
5.0 inset were not refit.

Four ER goldens changed bytes and still fail strict match on both sides:
`er_ecommerce_schema`, `er_healthcare_schema`, `er_financial_schema`,
`er_hr_schema`. On `er_hr_schema` both sides are DIFF 158. The `manages`
label moved from (344.82, 258.52) to (400.69, 282.23); Java is
(390.03, 308.82).

## Held-out batch

`~/Library/Caches/rustuml-grok-g4/class-link-fix-heldout/`. 70 cases, all
`intended_valid`, tip exit 0. 56 are ortho decoration pairs (7 from-tokens
by 4 to-tokens, LR and TB) with label `firing\nshelf`. The other 14 cover
`\\n`, `\\\\n`, `~\n`, a mixed escape, URL neighbors, an inline URL, a
tooltip newline, a tooltip plus a split label, a pipe URL, bold, italic,
underline, creole beside a URL, and escaped role labels.

Manifest `c95d31a62fe849340a4ebbe887ec9a4443b4ab4d1c49fc851366f6120f9d7eed`.
Bundle `520f40582e66de04160a6921d50a8830036a8fbe1cbf474abfec682389d12058`.
Predictions `9b727e01c288f099bec22a52e8371b8ba68a06dd669fd727a5a7e821751d1ebd`.
