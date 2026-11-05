# Round-9 maker (claude) request: Java face-resolution probe (read-only)

`FaceProbe.java` prints, for SansSerif, Lucida Grande, Arial, Helvetica, Verdana, Monospaced, Menlo and Courier New
in each of the four styles: `Font.getFontName()`, `Font.getPSName()`, `Font.getFamily()`, and the
`getStringBounds` width (antialiasing and fractional metrics on, PlantUML's FontRenderContext) of `Pier x` plus
U+1D167 and of `Pier x` alone. It ties ChainFace::for_style to the faces the reference JVM measures and shows the
mark's advance per style directly (Courier New plain/bold zeroed by the GDEF blocklist; italic/bold italic kept).

Stage `FaceProbe.java` in a fresh worker directory WORKER_PATH on Monkey Man, then:

    ssh -T -o BatchMode=yes marcelo@192.168.1.112 \
      '/Users/marcelo/rustuml-reference/bin/reference --probe WORKER_PATH/FaceProbe.java' \
      > probe.stdout 2> probe.stderr; echo $? > probe.exit

Return probe.stdout, probe.stderr and probe.exit with their SHA256s. No PlantUML classpath is needed.

Maker predictions (registered before any Java run), one line per family x style, face resolved by
tools/PrimaryFaceExtract.swift on colossus (PostScript name; the Lucida Grande italics are derived variants of
the upright faces) and Courier New's mark kept or zeroed:

    SansSerif, Lucida Grande: plain LucidaGrande, bold LucidaGrande-Bold, italic and bold italic the same files (derived)
    Arial: ArialMT, Arial-BoldMT, Arial-ItalicMT, Arial-BoldItalicMT
    Helvetica: Helvetica, Helvetica-Bold, Helvetica-Oblique, Helvetica-BoldOblique
    Verdana: Verdana, Verdana-Bold, Verdana-Italic, Verdana-BoldItalic
    Monospaced, Menlo: Menlo-Regular, Menlo-Bold, Menlo-Italic, Menlo-BoldItalic
    Courier New: CourierNewPSMT, CourierNewPS-BoldMT, CourierNewPS-ItalicMT, CourierNewPS-BoldItalicMT;
      with-mark minus without: plain 0 and bold 0 (blocklisted, zeroed), italic and bold italic 8.4013671875 (1229/2048 x 14)
