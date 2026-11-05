# Java bidi oracle (java.text.Bidi on the reference JVM)

`java-bidi-levels.txt.gz` holds 93,786 lines of the form `<input>|<Java output>`, selected by
`select_oracle.py` from a Java differential the shared-SVG round-8 checker ran on Monkey Man
(the reference host) through the parent relay:

| File | SHA256 |
| --- | --- |
| `bidi-input.txt` (242,508 inputs; 91,707 Unicode 15.0.0 BidiCharacterTest lines, 150,000 seeded random strings, 801 stack/overflow strings) | `fb7cd937973956246a1b34bdb9784ee7cf36422d1b8dd398fe47e97c1254d2e2` |
| `BidiProbe.java` | `7e0632e7bddb82e99b7708c122fe8b7bd58b70ccbe6fa747d708a2d3ca83af33` |
| `bidi-java-levels.txt` (first line: `OpenJDK 64-Bit Server VM 21.0.5`, Homebrew openjdk@21) | `e09425f49fb9f02fe2647c8bcd554ca7f6108d33586f0ed5b04cd7d1494bbf26` |

Relay record: `/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round8-bidi-probe`
(review 8089ef150, `relays_answered`, "Java bidi probe").

Input: `<dir>;<UTF-16 units in hex>`, dir `0` = `DIRECTION_LEFT_TO_RIGHT`, `1` =
`DIRECTION_RIGHT_TO_LEFT`, `D` = `DIRECTION_DEFAULT_LEFT_TO_RIGHT` (the flag `TextLine.fastCreateTextLine` uses).
Java output: `requiresBidi;isLeftToRight;getBaseLevel;getLevelAt(0..n-1);run levels (BidiUtils.getLevels)`.

Selection (fixed in `docs/parity-reviews/*/maker-round9-claude/bidi-oracle-gate-account.json` before any
mutation run): all part-C lines, every DEFAULT_LTR line with a paragraph separator before its end, every
line whose only RTL-triggering class is AN, every line ending in WS/S/B/BN or an explicit or isolate
control, and every 8th remaining line. Regenerate with:

    python3 -I select_oracle.py bidi-input.txt bidi-java-levels.txt java-bidi-levels.txt.gz

`java_bidi::tests::java_bidi_oracle` checks every line against `java_bidi` and pins `SHA256SUMS`.
