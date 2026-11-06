# Printable-ASCII advances per face (shared-SVG round 11)

`tools/AsciiAdvanceExtract.swift` (sha256 `c1db59ac…`, `extractor.sha256`) run by
`~/Library/Caches/rustuml-shared-svg-round11-claude-maker/relay/mm-ascii/run.sh` for the 28 (family, style)
slots of the seven table-measured families. It resolves each slot's face exactly as
`tools/PrimaryFaceExtract.swift` does (AWTFont.m `awtFontForName`) and prints, for U+0020..U+007E, the face's own
glyph id and its `CTFontGetAdvancesForGlyphs` advance at size 2048 (font units; every face here has 2048
units per em).

Host: extracted on colossus (macOS 27.2 build 26B123, the reference host's build). The 15 font files that the
round-9 Monkey Man extraction hashed (`../monkeyman-round9/font-hashes.txt`) have identical SHA256s here
(`font-hashes.txt`, which also hashes the four Times New Roman files). The same `run.sh` is requested on Monkey
Man through the relay; the expectation registered before that run is that every `*.ascii.tsv` is byte-identical.

`SHA256SUMS` (sha256 `1fa07b8c…`) lists the files byte for byte as `run.sh` wrote them; this README is not listed.
`text_render`'s ASCII-table gate reads the Verdana and Arial files and pins that hash.
