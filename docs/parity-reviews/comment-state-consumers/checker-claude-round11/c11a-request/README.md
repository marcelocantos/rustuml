# Checker round-11 (claude) relay request: held-out batch C11A (515 cases, -tsvg)

Immutable private staging: `/Users/marcelo/Library/Caches/rustuml-shared-svg-round11-claude-checker/c11a/`
- `reference-manifest.json` sha256 `97cfd7ec1a4ab4a4c0d6445699c1739c61234202cddce1735af0b126da1c4a9a` (absolute source paths under `/Users/marcelo/Library/Caches/rustuml-shared-svg-round11-claude-checker/c11a/sources/`)
- `reference-input-bundle.json.gz` sha256 `a26d1aef3d715162408c4b70426c34235156e08918673901e3aa343ea3f7daba` (no resources)
- `rust-predictions.json` sha256 `c6c9ca87908eb93f51b5689efa09310bdaf2da8c7bcbcb66704cf2c694d40698` (base 5ddf7733c, merge 879d166f4, tip 97830bd1e; registered before any Java run; compact copy here without stdout bodies)
- `checker-expectations.json` sha256 `58887ad7a34a7b9aa95d3202a356d3f54f387edf42a376d29f2a7b67cc58f43c` (hypotheses H1-H7, one per case)
- `cases.json` sha256 `a2f81611ceed9074c57a65c7c790a58838efb4eee4a6d5a8ee558c3d6582a115`

Binaries: private release builds from git archive exports, private CARGO_TARGET_DIR, -j 6 (../binaries.sha256).
Java revision 71806a23780b04a5ccde2f8ceb5121edad5eb711, JAR aa5262a2...; every case: cwd PRIVATE_CWD, timezone GMT+11:00,
invocation `/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg`, env {}.
All cases intended valid; all exit 0 with empty stderr at base, merge and tip. Groups: tools/c11a_sources.py docstring
(fc face x 30 consumers, sz sizes 9/23, cr creole runs, of other families, st stereotypes, sx stateFontStyle forms (C5),
qu queue pill (C3), na non-ASCII residue). Pre-Java facts: merge != tip on 117 cases, all Verdana; Arial, other-family,
Times/SansSerif state and Times queue cases are byte-identical merge -> tip.
Return reference-results.json (+ .gz) with the usual transport/validation records.

Face-data provenance (no Java needed): provenance/hmtx-check.json — fontTools hmtx advances of the 8 Arial/Verdana face
files (SHA256 equal to monkeyman-round9/font-hashes.txt) equal the tip's u16 tables and the maker's TSVs on all 95 code
points; the merge tree's upright f64 tables x 2048 equal hmtx exactly.
