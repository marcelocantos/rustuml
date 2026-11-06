# Held-out Menlo Bold Italic, second batch (maker-grok)

The first batch (`../heldout/`) asked Java for `ClassFontStyle bolditalic`.
`ValueImpl.asFontFace` maps that token to the normal face, so Java measured
a Menlo cell. This batch uses the path both sides already share:
`ClassAttributeFontStyle bold` plus an `{abstract}` member.
`MethodsOrFieldsArea` calls `config.italic()` after the bold attribute face,
and `mutateFont` keeps weight 700. The class name inherits the attribute
weight and stays upright.

Each bold case has a `_plain` control with the same family, size, and
member text, and neither the attribute style nor `{abstract}`. Those
controls should stay the plain Menlo cell.

Rendered by `rustuml` SHA256
`6ec2e5e37e34684554f9ee0685cd4042d744c620d020d6dc7ea042be4d4d5af4`
with `TZ=GMT+11 rustuml -pipe -tsvg`, before any Java run. Do not
regenerate the SVGs after the Java run.

| Case | Member | Face | Size | Predicted member textLength | Plain control |
|---|---|---|---|---|---|
| g3_bi2_symbols | U+2604 | AppleSymbols | 14 | 9.2217 | 8.4287 |
| g3_bi2_dingbat | U+27B2 | ZapfDingbatsITC | 14 | 10.6367 | 8.4287 |
| g3_bi2_cjk | U+2601 | HiraginoSans-W6 | 14 | 14 | 8.4287 |
| g3_bi2_emoji | U+2614 | AppleColorEmoji | 14 | 19 | 8.4287 |
| g3_bi2_mixed | A, U+27B2, Z | Menlo cell, ZapfDingbatsITC, Menlo cell | 14 | 27.4941 | 25.2861 |
| g3_bi2_emoji_12 | U+2614 | AppleColorEmoji | 12 | 16.2857 | 7.2246 |
| g3_bi2_dingbat_12 | U+27B2 | ZapfDingbatsITC | 12 | 9.1172 | 7.2246 |

The bold class name is the Menlo cell at `font-weight="700"` with no
`font-style`. The bold member adds `font-style="italic"` and
`font-weight="700"`. Plain controls have neither attribute. The 12 pt
rows scale the 14 pt font-matrix extraction (`units / units_den / upem * size`).

Sources also live at
`~/Library/Caches/rustuml-grok-g3/menlo-bi-heldout2/` (same bytes).
`reference-manifest.json` SHA256
`b84c81d7c95f362ed703c25a2fc3f5433fecff1ee0f69bb7a6cc66f362419459`.
`reference-input-bundle.json.gz` SHA256
`2a7af5e4daed20f16af7e8e0c20eeeafae770c8dfe15a7d69b05fc51198012e6`.
Java has not been run. Stage the manifest and the bundle. Run each case
with the invocation in the manifest, piping the bundled source. Return
each Java SVG and its SHA256. Score with the unchanged
`compare_svg_strict` against `rust-predictions/`.
