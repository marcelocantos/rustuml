# Round-11 maker (claude) request: Monkey Man per-face printable-ASCII advances (read-only, no Java)

Purpose: obligation 1 (Verdana Italic / Bold Italic ASCII advances) uses advances extracted on colossus from font files
whose SHA256 equals the round-9 Monkey Man record. This run re-extracts them on the reference host itself.

Stage `run.sh` and `AsciiAdvanceExtract.swift` (identical to the maker branch's `tools/AsciiAdvanceExtract.swift`,
sha256 c1db59acea0d7ffc390476fdc2c5daac9c4125fde1c518840c2ff4f38a747518) in a fresh worker directory WORKER_PATH on
Monkey Man, then:

    ssh -T -o BatchMode=yes marcelo@192.168.1.112 'bash WORKER_PATH/run.sh WORKER_PATH/work' > run.stdout 2> run.stderr; echo $? > run.exit

Return `WORKER_PATH/work/out/` (28 `<face>.<style>.ascii.tsv`, `font-hashes.txt`, `host.txt`, `swiftc-*.txt`,
`extractor.sha256`, `aae-stderr.txt`, `SHA256SUMS`) with run.stdout, run.stderr, run.exit and their SHA256s.
Nothing outside WORKER_PATH is written.

Pre-registered expectation (before the run): every `*.ascii.tsv` is byte-identical to the colossus extraction
(`expected-ascii-tsv.sha256`, also committed as tools/font-chain-reference/ascii-advances-round11/SHA256SUMS
entries); `aae-stderr.txt` is empty; font-hashes.txt agrees with the round-9 Monkey Man record on its 15 files.
