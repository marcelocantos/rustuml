# Round-9 maker (claude) request: Monkey Man per-style font extraction (read-only)

Purpose: the per-face model (round-9 obligation 1) needs the reference host's own chain coverage, own cmaps and
planner facts for every style of the six modelled families. Ten italic faces currently come from a provisional
colossus extraction (`tools/font-chain-reference/colossus-styled-chain`), and Verdana Italic / Bold Italic have no
reference-host file hash.

Stage `run.sh`, `FontChainExtract.swift` and `PrimaryFaceExtract.swift` (both identical to the maker branch's
`tools/`) in a fresh worker directory WORKER_PATH on Monkey Man, then:

    ssh -T -o BatchMode=yes marcelo@192.168.1.112 'bash WORKER_PATH/run.sh WORKER_PATH/work' > run.stdout 2> run.stderr; echo $? > run.exit

Return `WORKER_PATH/work/out/` (22 `<face>.tsv`, 24 `<face>.<style>.own.tsv`, `font-hashes.txt`, `host.txt`,
`swiftc-*.txt`, `extractors.sha256`, `fce-stderr.txt`, `pfe-stderr.txt`, `SHA256SUMS`) with run.stdout,
run.stderr, run.exit and their SHA256s. Nothing outside WORKER_PATH is written; no Java is run.
