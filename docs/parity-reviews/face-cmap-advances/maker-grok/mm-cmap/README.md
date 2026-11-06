# Monkey Man request: own-cmap advances

Account: `docs/parity-reviews/face-cmap-advances/maker-grok/account.json` (committed before this draft).

The extractor is `CmapAdvanceExtract.swift` in this directory, the same bytes as `tools/CmapAdvanceExtract.swift`. It resolves each of the twelve Helvetica, Arial and Verdana style slots the way `tools/PrimaryFaceExtract.swift` does, and prints `CTFontGetAdvancesForGlyphs` at 2048 units per em for every scalar that face's cmap maps.

This machine does not run it. Monkey Man runs:

```
bash run.sh <work dir>
```

`<work dir>` is an empty directory the host may create. The script writes only `<work dir>/out/`. It compiles the extractor into that output directory and deletes the binary before `SHA256SUMS`, so the returned directory is the twelve `*.cmap.tsv` files, `font-hashes.txt`, `host.txt`, `extractor.sha256`, the swiftc logs, `cae-stderr.txt` and `SHA256SUMS`.

Compare `font-hashes.txt` with `tools/font-chain-reference/monkeyman-round9/font-hashes.txt` before any table is consumed. A scalar present here and absent from that face's `*.own.tsv` `P` run, or the reverse, is a failed extraction.
