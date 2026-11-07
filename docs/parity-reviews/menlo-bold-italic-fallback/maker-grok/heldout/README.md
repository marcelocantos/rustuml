# Held-out Menlo Bold Italic widths (maker-grok)

Pre-registered Rust SVGs for the fallback advances in
`probe-jrs-face/fallback-advances.tsv`. Rendered by
`rustuml` SHA256
`b7c68efb448db3f26a27ab925f572263c5dbbf80ad3d462458d45f09a7f67fb4`
(commit `dc6cc0799`) with `TZ=GMT+11 rustuml -pipe -tsvg`. Do not
regenerate them after the Java run.

Each source sets `defaultFontName Menlo`, `ClassFontStyle bolditalic`,
and `ClassAttributeFontStyle bolditalic`, so the member and the class
name are Menlo Bold Italic. The class name is ASCII and stays one Menlo
cell per character. The member is the fallback.

| Case | Member | Face | Size | Predicted member textLength |
|---|---|---|---|---|
| g3_bi_symbols | U+2604 | AppleSymbols | 14 | 9.2217 |
| g3_bi_dingbat | U+27B2 | ZapfDingbatsITC | 14 | 10.6367 |
| g3_bi_cjk | U+2601 | HiraginoSans-W6 | 14 | 14 |
| g3_bi_emoji | U+2614 | AppleColorEmoji | 14 | 19 |
| g3_bi_mixed | A, U+27B2, Z | Menlo cell, ZapfDingbatsITC, Menlo cell | 14 | 27.4941 |
| g3_bi_emoji_12 | U+2614 | AppleColorEmoji | 12 | 16.2857 |
| g3_bi_dingbat_12 | U+27B2 | ZapfDingbatsITC | 12 | 9.1172 |

The 12 pt rows are the font-matrix scaling of the 14 pt extraction
(`units / units_den / upem * size`). They were not measured at 12 pt.

Stage `reference-manifest.json` and `reference-input-bundle.json.gz`.
Run each case with the invocation in the manifest, piping the bundled
source. Return each Java SVG and its SHA256. Score with the unchanged
`compare_svg_strict` against `rust-predictions/`.
