# Menlo Bold Italic fallback probe (maker-grok, read-only)

`MenloFallbackProbe.java` prints two groups.

Name rows use PlantUML's face construction (`FontStack.getFont` 109-121,
`UFontFace.deriveFont` 219-224): `Font.decode`, `deriveFont(PLAIN, 14)`, then
`TextAttribute.WEIGHT` / `POSTURE`. The width is `getStringBounds` with
antialiasing and fractional metrics, the same `FontRenderContext` as
`shared-svg-round9-face-probe`.

`CONTROL` is U+2580, present in both Menlo-Bold and Menlo-BoldItalic own cmaps.
`FALLBACK` is the 267 BMP scalars in `Menlo-Bold.bold.own.tsv` and not in
`Menlo-BoldItalic.bolditalic.own.tsv` (monkeyman-round9). None of them is in
the Bold Italic `S` (simple-path missing) runs, so the simple path does not
draw `.notdef`. 164 are also in the complex-cascade `C` runs; 103 are in
neither. The cited review range U+2580–U+27AF includes U+2580–U+2600, which
Bold Italic already maps; those scalars are not in `FALLBACK`.

Stage this directory as WORKER_PATH on the reference host, then:

    ssh -T -o BatchMode=yes marcelo@192.168.1.112 \
      '/Users/marcelo/rustuml-reference/bin/reference --probe WORKER_PATH/MenloFallbackProbe.java' \
      > probe.stdout 2> probe.stderr; echo $? > probe.exit

Return `probe.stdout`, `probe.stderr` and `probe.exit` with their SHA256s.
No PlantUML classpath.

Rust predictions, registered before any Java run, are `rust-predictions.tsv`
in this directory. Every `FALLBACK` and `CONTROL` width is one Menlo cell,
`1233/2048 * 14 = 8.4287109375`, for both bold and bold italic. That is the
current `cell_count` model (`plantuml_metrics.rs`), not a guess of the
fallback advance. Java is expected to disagree wherever JRS substitutes
another font.
