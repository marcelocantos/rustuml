# Class link labels — CT score

Scored batch CT after the pipe-URL fix. Validity comes from the Java
exit code and stderr, not from `intended_valid`. `compare_svg_strict`
uses `GEOM_EPS` 0.02. This is a measurement, not a checker review.

## Binaries

Tip commit `50218c805`. Tip binary
`b2008a28be42efbe34965aa26519a5daeac792d3a7a6b9d9add04867030f459a`.
Merge base `5c035c88fb07be7a023c7b4e3e8ec5d82bbf1e7bcf787f77b3b240a27fca30f6`.
Java results
`b3ce050b1c6f06ec87b5118b831e4f6255c3112395ed23d5fa921fdaf53eb7d0`.
The on-disk Java SVGs match those stdout hashes.

## Counts

70 cases. Java exited 0 on 46 and 200 on 24. The 24 are syntax errors
(`Syntax Error? (Assumed diagram type: class)` at line 5).

| Set | n | Tip | Merge |
|---|---:|---|---|
| Valid, both MATCH | 19 | MATCH | MATCH |
| Valid, tip gained | 26 | MATCH | DIFF |
| Valid, both DIFF | 1 | DIFF 10 | DIFF 10 |
| Invalid, both render | 24 | exit 0, DIFF 16 | exit 0, DIFF 16 |

Lost matches: 0. Lost rejections: 0. New errors on valid input: 0.
Tip panics: 0.

`g4f_url_pipe` is MATCH on the tip and DIFF 15 on the merge base.
The other 44 valid matches from before the fix stayed MATCH.

## Per case

| Case | Java | Tip | Merge |
|---|---|---|---|
| `g4f_creole_bold` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_creole_italic` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_creole_under` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_creole_url` | valid 0 | 0 MATCH | 0 DIFF 15 |
| `g4f_esc_bsbsn` | valid 0 | 0 MATCH | 0 DIFF 12 |
| `g4f_esc_bsn` | valid 0 | 0 MATCH | 0 DIFF 12 |
| `g4f_esc_mixed` | valid 0 | 0 MATCH | 0 DIFF 14 |
| `g4f_esc_tilde` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_lr_bl_dl` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bl_lb` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bl_ob` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bl_ol` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bo_dl` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bo_lb` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bo_ob` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_bo_ol` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_dl_dl` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_dl_lb` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_dl_ob` | valid 0 | 0 MATCH | 0 DIFF 23 |
| `g4f_ortho_lr_dl_ol` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_lb_dl` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_lb_lb` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_lb_ob` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_lb_ol` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_lo_dl` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_lo_lb` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_lo_ob` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_lo_ol` | valid 0 | 0 MATCH | 0 DIFF 1 |
| `g4f_ortho_lr_ob_dl` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ob_lb` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ob_ob` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ob_ol` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ol_dl` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ol_lb` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ol_ob` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_lr_ol_ol` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_bl_dl` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bl_lb` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bl_ob` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bl_ol` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bo_dl` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bo_lb` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bo_ob` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_bo_ol` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_dl_dl` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_dl_lb` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_dl_ob` | valid 0 | 0 DIFF 10 | 0 DIFF 10 |
| `g4f_ortho_tb_dl_ol` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_lb_dl` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_lb_lb` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_lb_ob` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_lb_ol` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_lo_dl` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_lo_lb` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_lo_ob` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_lo_ol` | valid 0 | 0 MATCH | 0 MATCH |
| `g4f_ortho_tb_ob_dl` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ob_lb` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ob_ob` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ob_ol` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ol_dl` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ol_lb` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ol_ob` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_ortho_tb_ol_ol` | syntax 200 | 0 DIFF 16 | 0 DIFF 16 |
| `g4f_roles_esc` | valid 0 | 0 MATCH | 0 DIFF 24 |
| `g4f_url_inline` | valid 0 | 0 MATCH | 0 DIFF 12 |
| `g4f_url_neighbors` | valid 0 | 0 MATCH | 0 DIFF 15 |
| `g4f_url_pipe` | valid 0 | 0 MATCH | 0 DIFF 15 |
| `g4f_url_tip_label` | valid 0 | 0 MATCH | 0 DIFF 11 |
| `g4f_url_tooltip` | valid 0 | 0 MATCH | 0 DIFF 2 |

## Residue

The 24 syntax errors are every ortho pair whose left token is `|{`,
`o|`, or `o{`, in both directions. The merge binary also exits 0 on
all 24, so the under-rejection is older than this packet. It is not
fixed here. Proposed target, not filed: class link decoration-token
admission follows Java `CommandLinkClass`. The left head is
`LinkDecor.getRegexDecors1()` (`}`, `}o`, `|o`, `||`, `}|`). The right
head is `getRegexDecors2()`. The left rejects `|{`, `o|`, and `o{`.

`g4f_ortho_tb_dl_ob` (`||--o{`, top to bottom) is DIFF 10 on both
binaries. The gap is the vertical route. This packet does not refit it.

## Zero loss on the earlier tip

The cohort scorer ran on tip
`0f3c2199607543d8752f86a5f35aa45faba3f95a6f3baf6592ed4d6b2d11e15e`,
which already kept a bar inside the URL and did not yet keep a bar
that sits only in a stacked label. Counts from
`~/Library/Caches/rustuml-grok-g4/fix-score/summary.txt`:

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

The four perturbation panics are the pre-existing empty-`parts` panic
at `parse/class.rs:285` and also fire on the merge binary. They are
not this packet.

`golden_no_oracle` on that earlier tip was 11,182/11,251, rendering
96/98. It was not re-run after the stacked-label bar. No file under
`test-diagrams/`, the CO, CK, c9a, or c11a sources contains `|` inside
`[[ ]]` except the probes named below and `g4f_url_pipe`. That case
already took the direct path because the URL itself contains `|`.

## Next relay

Four probes, all registered valid. Directory
`~/Library/Caches/rustuml-grok-g4/class-link-pipe-heldout/`.

| File | SHA256 |
|---|---|
| `reference-manifest.json` | `8b897447698cee5cca51b90bd08ae9786d6c3b21de4de6f748677463b4980433` |
| `reference-input-bundle.json.gz` | `f233920694139728a3c7bafe11aacd6a1e43b1b90a6cb441f1cac29dfdd2a42e` |
| `rust-predictions.json` | `23faf6b39ab072959d163bd22054ae1f9556a174a395f91baeb5716a4f0efb46` |

`g4p_label_pipe` draws `x|y` and `z` at `http://example.com/a` with
title `hot`. `g4p_pipe_neighbors`, `g4p_pipe_two`, and
`g4p_pipe_inline` keep the bar inside the URL and split on the newline.
Predictions were hashed after those SVGs were written.
