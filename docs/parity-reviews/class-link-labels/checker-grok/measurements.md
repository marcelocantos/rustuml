# Checker measurements for class link labels

Verdict: rejected. Not accept-ready for a local merge.

The xlabel path reads `ED_xlabel` after Graphviz `centerPt` (`vendor/graphviz/lib/postproc.c` center conversion, `graph.rs` label fallback to kind 3). That is the same center the existing `ED_label` paint path already uses. It is not a post-hoc coordinate rewrite. `ORTHO_XLABEL_VERTICAL_INSET` stays `5.0`, the value already on base, and the class.rs diff adds no 3-decimal literal.

## Counts

| Cohort | Denominator | Tip match | Base match | Lost matches | Gains | Panics |
|---|---:|---:|---:|---:|---:|---:|
| Held-out g4k, `compare_svg_strict` | 100 | 66 | 13 | 0 | 53 | 0 |
| Held-out Java exit 0 | 94 | 66 | 13 | 0 | 53 | 0 |
| Held-out Java exit 200 | 6 | 0 (all `JAVA_ERROR_PAGE`) | 0 | 0 | 0 | 0 |
| Maker CK `g4_*` | 13 | 13 | 1 | 0 | 12 | 0 |
| `c9a_af_*` | 38 | 34 | 21 | 0 | 13 | 0 |
| `c11a_fc_kcls_*` | 8 | 8 | 2 | 0 | 6 | 0 |
| Golden byte vs golden file | 12550 | — | — | 0 byte-losses | 0 byte-gains | 0 |
| No-oracle failure names | 11251 eligible | 11182 pass | 11181 pass | 0 names added | 1 name removed | 0 |
| Perturbation failure names | 6328 | 3620 not listed | 3601 not listed | 0 | 19 | 4 pre-existing on both |

No-oracle name removed: `rendering/splines/splines_multilevel_inheritance`. The tip CLI strict-matches that golden. Base is DIFF 4.

Golden bytes differ on exactly 3 of 12550 files. `er/er_ecommerce_schema` is DIFF 298 on both and the strict diff text is identical. `rendering/splines/splines_self_reference_ortho` improves from DIFF 31 to DIFF 4; the tip canvas is 147x99 against Java 147x105. `class/class_entity_er_ecommerce` is not in the byte delta.

`golden_pairs` (oracle layout, not the product metric) is 10497 passed, 754 failed, 1299 skipped, of 12550, on both trees, with the same per-directory failure counts. Both runs fail the test. Panics 0.

`parity_perturbations` fails on both trees (pre-existing red corpus). Tip lists 2708 failures, base 2727. The 19 names present only on base are the c9a/c11a files the tip now matches. No name is present only on the tip.

`pipe_mode` 17/17 on both. `doc_claims` 2/2 on both. Workspace `--lib` passed on both; one package went from 665 to 666 passing tests and `rustuml-render` stayed 862 passed, 0 failed, 1 ignored.

`decimal_literal_ratchet` fails the same way on base and tip: `class.rs` 383 to 578, `deployment.rs` 43 to 44, `plantuml_metrics.rs` 1053 to 1055, `state.rs` 53 to 62. Clippy on the touched crates is 23 diagnostics on both; the four line-number shifts are the same warnings.

`harness_guards` fails on base and on the tip before this review. The tip failure list includes `class-link-labels: account has no independent checker review`. This review is `rejected`, so once it is the latest file the guard also reports that the latest review must be accepted. That conflict is recorded, not patched.

## Valid-input losses caused by the packet

- `g4k_ortho_dl_cc_lr`. `left to right direction` and `Kiln ||--o{ Glaze : firing\nshelf`. Java exit 0. The skip predicate does not check orientation, so the tip uses the synthesizer on a horizontal edge. Classes move from y 28.291 to y 35.291. Label lines move to y 19.5684 and 34.8789; Java has them at y 44.5684 and 59.8789.
- `g4k_nl_escaped`. Source label `a\\nb`. Java draws one line `a\nb`. Tip splits into `a\` and `b` because `split("\\n")` has no escape state.
- `g4k_nl_url`. Source label `pre\n[[http://example.com/a\nb]]\npost`. Java draws `pre`, `http://example.com/a`, `b`, `post`. Tip draws `pre`, `]`, `post`.

## Java exit 200, intended valid

All six are `JAVA_ERROR_PAGE` on both sides. Rust does not emit Java's error page.

| Case | Java stderr | Packet effect |
|---|---|---|
| `g4k_st_two` | style bad definition, line 10, assumed sequence | Bytes change. Base 184x70, tip 125x195. Style block is illegal, so this is not a two-stereotype parse. |
| `g4k_st_two_tight` | style bad definition, line 6 | Same pair of viewBoxes. Same confound. |
| `g4k_st_space` | style bad definition, line 6, `arrow <<foo bar>>` | Base 184x70, tip 141x195. Space stereotype is confounded with the illegal style. |
| `g4k_st_style` | style bad definition, line 7 | Base 188x195, tip 197x195. Both already diagrams. |
| `g4k_q_card_stereo` | style bad definition, line 19 | Base 188x193, tip 191x193. |
| `g4k_q_empty` | `Syntax Error?` assumed class, line 3 | Base and tip identical, 184x70. Pre-existing. Empty quotes are not a quantifier. |

`g4k_nl_pct_short` (`%n()`) is Java exit 0 and Rust exit 1 on both sides (`Unknown built-in function %n`). Pre-existing `COMPARE_ERR`.

## Residuals that improved and still do not match

Magic-arrow sizes 7 and 9 are DIFF 4 on the arrowhead polygon (34 on base). `FontWeight bolder` is weight 700 against Java 800, with the canvas width now matching. `FontWeight 100` gains italic and still omits weight 100. Orthogonal `\r` and `\l` stay one unsplit string; the tip only nudges that string. Vertical `||--o{` remains DIFF 31.

Out-of-lane object, usecase, component, deployment, and state probes are byte-identical to base and still DIFF. `c9a_af_object_vd`, `c9a_af_usecase_vd`, `c9a_af_deploy_size16`, and `c9a_af_component_vd` are the four c9a misses and are byte-identical.
