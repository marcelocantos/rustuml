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
