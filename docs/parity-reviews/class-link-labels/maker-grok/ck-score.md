# Class link labels — held-out batch CK

Scored after the parent relay. The pre-registered tip files were not edited.
This is a measurement, not a checker review.

## Relay

Batch CK, worker `worker.0N9ToFlO`. Java results
`reference-results.json` SHA256
`793d8f12ee948ecd62158ab2983a3105b8da0e983aed8f68fcc621e872df9f05`.
Manifest `3534780d2179f461d02c67f6f43231f3870d18f423197f70513a5cbff74c0b87`.
Bundle `c02d69ce2ee2a9cb75e6a03fcb352b14b5cffb272956650ffe6a48c9894314a1`.
Tip predictions `ffecba6a159cbe8b7f5da3dd24cb4be889aea3dfba3881431749e984b831ea5e`,
binary `88050537a0cdf5fd5a38f22be2483724d4689b390dafe17c7b6cd0508821b6bd`.

Each registered prediction file's SHA256 matches that record. Java stdout
was decoded and its SHA256 matches the result record. All 13 cases exited 0
with empty stderr. Each stdout is a class SVG (`data-diagram-type="CLASS"`).
They are valid. There is no intent disagreement.

`rust-predictions.json` registered the tip only. The base column is a fresh
render of frozen base `cb550c0d6`, binary
`3e07b64d8ddcdf83b161b90923b7b609a5a9fc696bed32546333acb71c217281`,
on the same source bytes. Those files are not the prediction record.

## Score

`compare_svg_strict` on the whole SVG stdout. Expected is Java. Actual is
the named side. Tip is 13 MATCH, 0 DIFF. Base is 1 MATCH, 12 DIFF.

| Case | Tip | Base |
|---|---|---|
| `g4_nl_vd15` | MATCH | DIFF 12 |
| `g4_style_bold` | MATCH | DIFF 2 |
| `g4_style_italic` | MATCH | DIFF 1 |
| `g4_cn_italic_mark` | MATCH | DIFF 2 |
| `g4_cn_italic_mark_ctrl` | MATCH | DIFF 1 |
| `g4_stereo_deploy` | MATCH | DIFF 8 |
| `g4_stereo_global_vd` | MATCH | DIFF 8 |
| `g4_roles_size15` | MATCH | DIFF 2 |
| `g4_size8` | MATCH | DIFF 9 |
| `g4_ortho_cn15` | MATCH | DIFF 1 |
| `g4_bold_italic_plain` | MATCH | MATCH |
| `g4_self_nl` | MATCH | DIFF 4 |
| `g4_stereo_nl_italic` | MATCH | DIFF 8 |

The base diffs are the pre-port path: `\n` left literal, `ArrowFontStyle`
absent, a `<<stereotype>>` before the label drops the link, quantifiers
and the size-8 table use the old box, and the ortho label sits 2 px low.
`g4_bold_italic_plain` matches on base because `bold italic` stays plain
on both sides. No tip case failed, so this batch adds no code change and
no new residue.
