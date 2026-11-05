# Pinned shaping sources

Upstream files the `rustuml-render` drift gate (`font_chain` and `java_bidi`
tests) re-derives the generated shaping tables from. `SHA256SUMS` covers every
file here; its own SHA256 is pinned in `font_chain.rs`
(`SHAPING_SOURCES_SHA256SUMS_SHA256`).

| File | Source | License |
| --- | --- | --- |
| `harfbuzz-15.0.0/hb-ot-shaper.hh`, `hb-script-list.h`, `hb-ot-layout.cc` | https://github.com/harfbuzz/harfbuzz tag 15.0.0 (35f656a1a2bab548ea7f54a6dd3fba2b09680d14), `src/` | HarfBuzz "Old MIT" (header of each file) |
| `jdk-21.0.5-ga/ubidi.icu` | https://github.com/openjdk/jdk21u tag jdk-21.0.5-ga, `src/java.base/share/classes/jdk/internal/icu/impl/data/icudt72b/ubidi.icu` | Unicode License (ICU data) |
| `unicode-18.0.0/UnicodeData.txt.gz` | https://www.unicode.org/Public/18.0.0/ucd/UnicodeData.txt, gzipped (`gzip -9 -n`); decompressed SHA256 0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950 | Unicode License |
| `unicode-15.0.0/BidiCharacterTest-sample.txt` | every 97th data line of https://www.unicode.org/Public/15.0.0/ucd/BidiCharacterTest.txt (header names its SHA256) | Unicode License |

Generators: `tools/font_chain_unicode.py` (UnicodeData, HarfBuzz files),
`tools/java_bidi_data.py` (ubidi.icu, checked against Unicode 15.0.0
DerivedBidiClass.txt and BidiBrackets.txt), `tools/font_chain_coverage.py`.
