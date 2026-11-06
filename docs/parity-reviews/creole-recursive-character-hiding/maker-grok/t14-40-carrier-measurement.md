# T14.40 carrier-range measurement

Zero-loss is against the merge base `8d294442f4d88aed3f1c353a9766045fc90f2a7c`
(parity-recovery merged, G1 included, carrier bug still present). Tip is
`3f6bf7a7332da0ad85dbc514d4af665f82920632`. Comparator is
`compare_svg_strict`. CLI is `-pipe -tsvg` with `TZ=GMT+11:00`.

## Required pairs

`ckg6_amp_last_scalar` strict-matches the CM Java SVG (0 diffs). The text
node is size 14, weight 700, textLength 78.3057. Merge-base exits 1
(`Unicode: 0xdfff`).

`cr_ent_dec_10fffd` merge-base exits 1 (`Unicode: 0xdffd`). Tip exits 0.
The label textLength is 79.5156, the same as the committed golden. One
strict diff remains: the class rectangle fill is `#1114109` versus the
golden `#F1F1F1`. That is the out-of-lane class colour parse of `#` inside
`&#N;`, not the exporter. The pre-packet base also exited 0.

## Cohorts

| Cohort | Result |
|---|---|
| CM (48) | Merge-base 14 strict matches. Tip 15. Lost 0. Gain: `ckg6_amp_last_scalar`. Tip exits 0 on all 48. The other misses are the same diff counts on both trees, including the NUL comparator error on `ckg6_amp_nul`. |
| CH (20) | 13 strict matches on both trees. Lost 0. The seven misses are unchanged, including `ref_invalid_uplus_nul`. |
| Goldens (12,550) | Stdout and exit identical. Panics 0. Timeouts 0. |
| Perturbations (6,365) | 6,364 identical. The one change is `cr_ent_dec_10fffd` (exit 1 to exit 0). New exit-1 errors: 0. Panics 4 on both trees. Timeouts 0. |
| Retained paths (173 names containing `retained`) | 173/173 identical. Panics 0. |

## Held-out batch CN

Staged, not sent to Java. 25 probes. Decimal scalars in
`U+10F7FF..=U+10FFFF` exit 1 on the merge-base and 0 on the tip. Decimals
below that, and 4- or 5-digit `<U+…>`, are byte-identical. Six-digit
`<U+10F800>`, `<U+10FFFD>` and `<U+10FFFF>` stay literal on both.
