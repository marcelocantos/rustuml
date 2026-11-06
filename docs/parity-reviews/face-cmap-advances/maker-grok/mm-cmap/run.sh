#!/bin/bash
# Face-cmap advance extraction for the reference host (Monkey Man).
# Read-only outside this script's directory. Writes only under <work dir>/out/.
# Usage: bash run.sh <work dir>
set -u
H=$(cd "$(dirname "$0")" && pwd)
W=$1
mkdir -p "$W/out"
cd "$W"
{ hostname; sw_vers; date -u; } > out/host.txt
swiftc --version > out/swiftc-version.txt 2>&1
swiftc -O "$H/CmapAdvanceExtract.swift" -o "$W/out/cmap-advance-extract" > out/swiftc-cae.txt 2>&1 || exit 2
shasum -a 256 "$H/CmapAdvanceExtract.swift" > out/extractor.sha256
names=(plain bold italic bolditalic)
for slot in \
  Helvetica:0 Helvetica-Bold:1 Helvetica-Oblique:2 Helvetica-BoldOblique:3 \
  ArialMT:0 Arial-BoldMT:1 Arial-ItalicMT:2 Arial-BoldItalicMT:3 \
  Verdana:0 Verdana-Bold:1 Verdana-Italic:2 Verdana-BoldItalic:3
do
  n=${slot%%:*}
  s=${slot#*:}
  "$W/out/cmap-advance-extract" "$n" "$s" > "out/$n.${names[$s]}.cmap.tsv" 2>> out/cae-stderr.txt
done
{
  echo "## font files"
  for f in /System/Library/Fonts/Helvetica.ttc \
           "/System/Library/Fonts/Supplemental/Arial.ttf" \
           "/System/Library/Fonts/Supplemental/Arial Bold.ttf" \
           "/System/Library/Fonts/Supplemental/Arial Italic.ttf" \
           "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf" \
           "/System/Library/Fonts/Supplemental/Verdana.ttf" \
           "/System/Library/Fonts/Supplemental/Verdana Bold.ttf" \
           "/System/Library/Fonts/Supplemental/Verdana Italic.ttf" \
           "/System/Library/Fonts/Supplemental/Verdana Bold Italic.ttf"
  do
    shasum -a 256 "$f"
  done
} > out/font-hashes.txt
# The extractor binary is regenerable. Hash the extraction products only.
rm -f out/cmap-advance-extract
(cd out && shasum -a 256 * > ../SHA256SUMS.out && mv ../SHA256SUMS.out SHA256SUMS)
echo done
