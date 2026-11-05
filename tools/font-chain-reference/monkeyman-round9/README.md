# Monkey Man per-style extraction (shared-SVG round 9)

`tools/FontChainExtract.swift` and `tools/PrimaryFaceExtract.swift` (sha256 `c240a99b…` and `7920e8dc…`,
`extractors.sha256`) run on Monkey Man, the reference host, by the parent relay from the round-9 maker's
request (`run.sh` 6a63a29b…). Files are the relay's `out/` byte for byte; `SHA256SUMS` (sha256
`ad8e6566043819726c0146b76261b2aea3710eb83372d59547b46c3132f3ed55`) lists them. This README is not listed.

Relay record: `/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round9-mm-styles`
(run.exit 0, run.stderr empty, run.stdout d117fa00…).

* `<PostScript name>.tsv`: chain coverage (simple-path missing BMP runs, composite-cascade mapped runs),
  `.notdef` advance and HarfBuzz plan facts of the 22 faces of the six modelled families.
* `<native name>.<plain|bold|italic|bolditalic>.own.tsv`: the face JDK measures for each of the 24
  (family, style) slots (AWTFont.awtFontForName), its own cmap and planner facts.
* `font-hashes.txt`: SHA256 of every font file involved.

`tools/font_chain_coverage.py` generates `crates/rustuml-render/src/font_chain_coverage.rs` from this
directory (chain and own-cmap data alike); `font_chain`'s drift gate re-checks every file.

## Regenerating `font_chain_coverage.rs`

Run from the repository root. This is the exact command; it reproduces the committed
`crates/rustuml-render/src/font_chain_coverage.rs` byte for byte (sha256 `eef8a552…3479`,
re-run in shared-SVG round 10):

```
D=tools/font-chain-reference/monkeyman-round9
python3 -I tools/font_chain_coverage.py crates/rustuml-render/src/font_chain_coverage.rs $D --primary $D \
 LUCIDA_GRANDE=LucidaGrande.tsv:LucidaGrande.plain.own.tsv LUCIDA_GRANDE_BOLD=LucidaGrande-Bold.tsv:LucidaGrande-Bold.bold.own.tsv \
 ARIAL=ArialMT.tsv:ArialMT.plain.own.tsv ARIAL_BOLD=Arial-BoldMT.tsv:Arial-BoldMT.bold.own.tsv ARIAL_ITALIC=Arial-ItalicMT.tsv:Arial-ItalicMT.italic.own.tsv ARIAL_BOLD_ITALIC=Arial-BoldItalicMT.tsv:Arial-BoldItalicMT.bolditalic.own.tsv \
 HELVETICA=Helvetica.tsv:Helvetica.plain.own.tsv HELVETICA_BOLD=Helvetica-Bold.tsv:Helvetica-Bold.bold.own.tsv HELVETICA_OBLIQUE=Helvetica-Oblique.tsv:Helvetica-Oblique.italic.own.tsv HELVETICA_BOLD_OBLIQUE=Helvetica-BoldOblique.tsv:Helvetica-BoldOblique.bolditalic.own.tsv \
 VERDANA=Verdana.tsv:Verdana.plain.own.tsv VERDANA_BOLD=Verdana-Bold.tsv:Verdana-Bold.bold.own.tsv VERDANA_ITALIC=Verdana-Italic.tsv:Verdana-Italic.italic.own.tsv VERDANA_BOLD_ITALIC=Verdana-BoldItalic.tsv:Verdana-BoldItalic.bolditalic.own.tsv \
 MENLO=Menlo-Regular.tsv:Menlo-Regular.plain.own.tsv MENLO_BOLD=Menlo-Bold.tsv:Menlo-Bold.bold.own.tsv MENLO_ITALIC=Menlo-Italic.tsv:Menlo-Italic.italic.own.tsv MENLO_BOLD_ITALIC=Menlo-BoldItalic.tsv:Menlo-BoldItalic.bolditalic.own.tsv \
 COURIER_NEW=CourierNewPSMT.tsv:CourierNewPSMT.plain.own.tsv COURIER_NEW_BOLD=CourierNewPS-BoldMT.tsv:CourierNewPS-BoldMT.bold.own.tsv COURIER_NEW_ITALIC=CourierNewPS-ItalicMT.tsv:CourierNewPS-ItalicMT.italic.own.tsv COURIER_NEW_BOLD_ITALIC=CourierNewPS-BoldItalicMT.tsv:CourierNewPS-BoldItalicMT.bolditalic.own.tsv
rustfmt --edition 2024 crates/rustuml-render/src/font_chain_coverage.rs
```

The generator prints `22 faces, 26 tables, 40965 runs; SHA256SUMS ad8e6566…` (the source and primary
directories are this one; there is no provisional directory).
