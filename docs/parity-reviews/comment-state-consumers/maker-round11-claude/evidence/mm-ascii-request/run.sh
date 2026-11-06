#!/bin/bash
# Round-11 maker (claude) request, run on Monkey Man, read-only outside its own directory: the printable-ASCII
# advances (font units) of the face JDK 21 measures for each (family, style) slot of the seven table-measured
# families, plus the font-file hashes. Usage: bash run.sh <work dir>  (writes <work dir>/out/ only).
set -u
H=$(cd "$(dirname "$0")" && pwd)
W=$1; mkdir -p "$W/out"; cd "$W"
{ hostname; sw_vers; date -u; } > out/host.txt
swiftc --version > out/swiftc-version.txt 2>&1
swiftc -O "$H/AsciiAdvanceExtract.swift" -o "$W/ascii-advance-extract" > out/swiftc-aae.txt 2>&1 || exit 2
shasum -a 256 "$H/AsciiAdvanceExtract.swift" > out/extractor.sha256
names=(plain bold italic bolditalic)
for slot in LucidaGrande:0 LucidaGrande-Bold:1 LucidaGrande:2 LucidaGrande-Bold:3 ArialMT:0 Arial-BoldMT:1 Arial-ItalicMT:2 Arial-BoldItalicMT:3 \
            Helvetica:0 Helvetica-Bold:1 Helvetica-Oblique:2 Helvetica-BoldOblique:3 Verdana:0 Verdana-Bold:1 Verdana-Italic:2 Verdana-BoldItalic:3 \
            Menlo-Regular:0 Menlo-Bold:1 Menlo-Italic:2 Menlo-BoldItalic:3 CourierNewPSMT:0 CourierNewPS-BoldMT:1 CourierNewPS-ItalicMT:2 CourierNewPS-BoldItalicMT:3 \
            TimesNewRomanPSMT:0 TimesNewRomanPS-BoldMT:1 TimesNewRomanPS-ItalicMT:2 TimesNewRomanPS-BoldItalicMT:3; do
  n=${slot%%:*}; s=${slot#*:}
  "$W/ascii-advance-extract" "$n" "$s" > "out/$n.${names[$s]}.ascii.tsv" 2>> out/aae-stderr.txt
done
{
  echo "## font files"
  for f in /System/Library/Fonts/LucidaGrande.ttc /System/Library/Fonts/Helvetica.ttc /System/Library/Fonts/Menlo.ttc \
           "/System/Library/Fonts/Supplemental/Arial.ttf" "/System/Library/Fonts/Supplemental/Arial Bold.ttf" \
           "/System/Library/Fonts/Supplemental/Arial Italic.ttf" "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf" \
           "/System/Library/Fonts/Supplemental/Verdana.ttf" "/System/Library/Fonts/Supplemental/Verdana Bold.ttf" \
           "/System/Library/Fonts/Supplemental/Verdana Italic.ttf" "/System/Library/Fonts/Supplemental/Verdana Bold Italic.ttf" \
           "/System/Library/Fonts/Supplemental/Courier New.ttf" "/System/Library/Fonts/Supplemental/Courier New Bold.ttf" \
           "/System/Library/Fonts/Supplemental/Courier New Italic.ttf" "/System/Library/Fonts/Supplemental/Courier New Bold Italic.ttf" \
           "/System/Library/Fonts/Supplemental/Times New Roman.ttf" "/System/Library/Fonts/Supplemental/Times New Roman Bold.ttf" \
           "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf" "/System/Library/Fonts/Supplemental/Times New Roman Bold Italic.ttf"; do
    shasum -a 256 "$f"
  done
  echo "## user/local copies that could shadow them"
  ls -la ~/Library/Fonts /Library/Fonts 2>/dev/null | grep -i -E "lucida|helvetica|menlo|arial|verdana|courier|times" || echo "(none)"
} > out/font-hashes.txt
(cd out && shasum -a 256 * > ../SHA256SUMS.out && mv ../SHA256SUMS.out SHA256SUMS)
echo done
