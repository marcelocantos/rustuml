# Own-cmap advances (grok-g2, 🎯T14.55)

`tools/CmapAdvanceExtract.swift` (sha256 `954cc456f4eaf527d383c4eb0f0fe3199eac35749f5237a5a0c7b50cf9cacc6c`)
was run on the reference host by `docs/parity-reviews/face-cmap-advances/maker-grok/mm-cmap/run.sh`
(sha256 `d05f35e00dac5aa0c06e93525e4ce26a14f98fb50f633d381b55dc285f9b7844`). The parent relayed that
command as `bash run.sh <work>`. It exited 0. `cae-stderr.txt` is empty.

Host: macOS 27.2 build 26B123, the same build as `../monkeyman-round9/`. The nine face files in
`font-hashes.txt` match that directory's hashes. Swift on the host was swiftlang-6.3.0.123.5.

`SHA256SUMS` (sha256 `1b1e9a8eeca7aeb326e825c8c583129de2c4eb846aba3c62e15aacea8e4d20fe`) lists the
files byte for byte as the run wrote them. This README is not listed. Each `*.cmap.tsv` line is
`M`, a scalar, its glyph ids, and the `CTFontGetAdvancesForGlyphs` advance at 2048 units per em.
The scalars equal that face's `../monkeyman-round9/*.own.tsv` `P` runs. The printable-ASCII column
equals `../ascii-advances-round11/`.

The immutable relay copy is `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g2-mm-cmap/out/`.
