# gen_non_ascii_widths.java

Regenerates `src/non_ascii_widths.rs` — the exact AWT advances for every
non-ASCII codepoint used in the golden corpus.

PlantUML measures SVG text with `java.awt.FontMetrics.getStringBounds` on the
JVM SansSerif logical font. On the macOS JVM that generated the goldens this
reproduces our ASCII table byte-for-byte, so the same call gives exact non-Latin
advances. Advances are linear in point size, so values are stored as
advance-per-unit-size (advance@14 / 14) and scaled by font size at lookup.

To regenerate (codepoint list is collected from the golden SVGs):
```
# 1. collect codepoints → /tmp/cps.txt (python, see commit message)
# 2. javac gen_non_ascii_widths.java && java -Djava.awt.headless=true GenW
# 3. cp /tmp/non_ascii_widths.rs ../src/non_ascii_widths.rs
```
