# JRS fallback face probe (maker-grok, read-only)

`MenloFallbackProbe.java` measured bold and bold-italic advances and did not
name the font Java substituted. JDK 21.0.5 `CoreTextSupport.m`
`CTS_GetGlyphsAsIntsForCharacters` (lines 99–135) asks
`JRSFontCreateFallbackFontForCharacters` and, when that font has a glyph,
returns the Unicode scalar as a negative glyph id. `AWTStrike` then measures
that CTFont (`CTS_CopyCTFallbackFontAndGlyphForJavaGlyphCode`). A Java
`CharToGlyphMapper` cannot see the CTFont's PostScript name.

`MenloJrsFaceProbe.java` prints `getStringBounds` for the same 267 scalars
and for the U+2580 control, on Menlo Bold and Menlo Bold Italic, with the
same `Font.decode` / `deriveFont(PLAIN, 14)` / weight / posture construction
as the previous probe. It then writes a C helper under `java.io.tmpdir`,
compiles it with `cc`, `clang`, or `xcrun --find clang`, and runs it. The
C source is a Java text block, so each tab and newline in a C string is
written with a doubled backslash. The helper calls the same JRS entry
`tools/FontChainExtract.swift` calls and
prints, per scalar, `OWN`, `FALLBACK`, or `MISSING`, the PostScript name,
family, glyph id, point advance, units per em, advance in font units, and
font path. It deletes the temporary source and binary before exit. It does
not write into the worker directory.

The helper is required because monkeyman-round9 has cmaps, `.notdef`
advances, and ASCII advances only. It has no Monaco or Apple Symbols data,
and JRS order is not the `Menlo-BoldItalic.tsv` cascade. Do not treat a
width that equals a modelled cell as the name of the face.

No Rust predictions are registered for the new columns. The previous
`probe/rust-predictions.tsv` stays as it was.

Stage this directory as `WORKER_PATH` and run:

```
ssh -T -o BatchMode=yes marcelo@192.168.1.112 \
  '/Users/marcelo/rustuml-reference/bin/reference --probe WORKER_PATH/MenloJrsFaceProbe.java' \
  > probe.stdout 2> probe.stderr; echo $? > probe.exit
```

Return `probe.stdout`, `probe.stderr`, `probe.exit`, and their SHA256s.
No PlantUML classpath. Do not run it on the origin Mac.
