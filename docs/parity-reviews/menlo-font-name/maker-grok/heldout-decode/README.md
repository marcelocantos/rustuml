# Held-out font decode (maker-grok)

Pre-registered after the creole `Font.decode` correction. Java has been
run (relay `grok-g3-cx`). Do not regenerate these SVGs. The prediction
bytes stay the pre-Java render.

Rendered by release `rustuml` SHA256
`0333eb3e4da55cf4f07a9c2a30c6552beeb6edba6ab5542484970f1c65b9e506`
with `TZ=GMT+11 rustuml -pipe -tsvg`.

Sources live at `~/Library/Caches/rustuml-grok-g3/menlo-name-heldout3/`
(same bytes as the bundle). `reference-manifest.json` SHA256
`8f99d08ee579fc15bc9d5f2a40812b0d985a9a3e521d8447ac481377c8e5eb37`.
`reference-input-bundle.json.gz` SHA256
`21093eb68b5ce78817e26809afbaffbabb54c3f22b7d764cf863596638f75c57`.

Stage the manifest and the bundle. Run each case with the invocation in
the manifest, piping the bundled source. Return each Java SVG and its
SHA256. Score with the unchanged `compare_svg_strict` against
`rust-predictions/`.

The port claims, and these predictions record:

- Creole `<font:monospace>` measures Dialog. `<font:monospaced>`,
  `<font:Monospaced>`, and `<font:MONOSPACED>` measure the Menlo cell
  and the SVG family is `monospace`. `<font:MONOSPACE>` is not that
  logical name: the family stays `MONOSPACE` and the width is Dialog.
  `<font:Courier>` keeps the family `Courier` and the Menlo cell.
  `<font:Times>` keeps the family `Times` and the Dialog width.
  `<font:serif>` and `<font:timesroman>` use the serif width.
- `defaultFontName monospace` measures Dialog and does not select Menlo.
  `defaultFontName Monospaced` measures the Menlo cell and is written
  `monospace`.
- `sequenceFontName` does not change metrics. `sequenceParticipantFontName`,
  `arrowFontName`, `noteFontName`, `classFontName`, and
  `classAttributeFontName` do.
- `hn_skin_quoted_pad` is the current port, which trims the inner spaces
  of `" Menlo "`. Keeping those spaces is a pre-existing residue, not a
  special case added here. The prediction is what this binary emits.

| Case | viewBox | Notable text |
|---|---|---|
| hn_creole_faces | 0 0 85 294 | Kelp Menlo 33.7148; Nimbus monospace 52.0967; three `monospace` 50.5723; MONOSPACE 52.0967; Courier 50.5723; Dialog 52.0967; SansSerif 52.0967; serif 44.3379; timesroman 44.3379; Times 52.0967; Arial-BoldMT 51.3379; Helvetica-Oblique 47.4551; Menlo-Regular 50.5723 |
| hn_skin_monospace_word | 0 0 88 84 | Pylon monospace 36.3877; Kelp NBSP wy monospace 52.3291 |
| hn_skin_monospaced_word | 0 0 94 84 | Pylon monospace 42.1436; Kelp NBSP wy monospace 59.001 |
| hn_skin_mixed_menlo | 0 0 94 84 | family `mEnLo`, Pylon 42.1436, Kelp wy 59.001 |
| hn_skin_quoted_menlo | 0 0 94 84 | family `'Menlo'`, Pylon 42.1436, Kelp wy 59.001 |
| hn_skin_quoted_pad | 0 0 91 84 | family `'Menlo'`, Quay 33.7148, Kelp wy 59.001 |
| hn_skin_quoted_single | 0 0 91 84 | family `'Courier'`, Quay 33.7148, Kelp NBSP wy 59.001 |
| hn_seq_ignores_sequencefontname | 0 0 119 123 | Cara Times 30.8779; Drew Times 34.8018; ping Times 28.1138 |
| hn_seq_honours_participant | 0 0 121 122 | Cara and Drew Courier 33.7148; ping Times 28.1138 |
| hn_seq_honours_arrow | 0 0 119 123 | Cara Times 30.8779; Drew Times 34.8018; ping Menlo 31.3066 |
| hn_seq_honours_note | 0 0 181 129 | ping Times 28.1138; note Kelp wy Menlo 54.7866 |
| hn_class_attribute_font | 0 0 91 84 | Quay Courier 33.7148; Kelp NBSP wy Courier 59.001 |
| hn_class_fontname_helvetica | 0 0 85 68 | Quay Helvetica 33.4619 |
| hn_skin_courier_new_ps | 0 0 85 83 | Quay and Kelp CourierNewPS-ItalicMT 33.6055 |

Scored with `compare_svg_strict` on the whole SVG. Tip `5ca278f72` is
11/14. Merge base `0cd7743aa` is 7/14. The four gains are
`hn_seq_honours_note`, `hn_seq_ignores_sequencefontname`,
`hn_skin_monospace_word`, and `hn_skin_monospaced_word`. The three tip
misses are the same misses on the base:

- `hn_creole_faces` — family `SansSerif` versus Java `sans-serif`
  (`UFontImpl.getFamily`). Width matches. 🎯T29.
- `hn_skin_quoted_pad` — family `'Menlo'` versus `' Menlo '`. Widths
  match. Already 🎯T14.94.
- `hn_skin_quoted_single` — family `'Courier'` matches. Java measures
  the default face (viewBox `0 0 86 84`). Rust measures the Menlo cell
  and a NBSP. 🎯T30.

`reference-results.json` SHA256
`b333ba69cb4b54d4c214da93b6ee4a81b322e216dfb8d5075c80ce5a64930c94`.
Recorded in `../addendum-heldout-decode.json`. No production change.
