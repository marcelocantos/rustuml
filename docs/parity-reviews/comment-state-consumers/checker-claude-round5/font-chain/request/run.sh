#!/bin/bash
# Font-chain environment extraction on the reference host (NOT a Java run). Run from this directory.
# Usage: bash run.sh <outdir>   (writes TSVs, census and SHA256SUMS into <outdir>)
set -eu
O=$1; mkdir -p "$O"
swiftc -O FontChainExtract.swift -o "$O/fce"; swiftc -O whofb.swift -o "$O/whofb"; swiftc -O notdef.swift -o "$O/notdef"
sw_vers > "$O/sw_vers.txt"; hostname > "$O/hostname.txt"
for f in LucidaGrande LucidaGrande-Bold ArialMT Arial-BoldMT Helvetica Helvetica-Bold Verdana Verdana-Bold Menlo-Regular Menlo-Bold CourierNewPSMT CourierNewPS-BoldMT TimesNewRomanPSMT; do
  "$O/fce" "$f" > "$O/$f.tsv"
done
"$O/notdef" LucidaGrande LucidaGrande-Bold ArialMT Arial-BoldMT Helvetica Helvetica-Bold Verdana Verdana-Bold Menlo-Regular CourierNewPSMT TimesNewRomanPSMT > "$O/notdef.txt"
"$O/whofb" LucidaGrande 0528 052D 20B7 A698 A69D A69E F880 F8B7 F8E5 F8FE 211A 22AD A8FB FE0E FE0F E123 F000 0378 0870 1C80 2E5E 1AB0 0D80 2065 FFF0 180F A500 1E900 > "$O/whofb-lucida.txt"
"$O/whofb" TimesNewRomanPSMT 0528 052D 20B7 A698 A69D A69E F880 F8B7 F8E5 F8FE 01CC 211A 22AD A8FB FE0E FE0F 11FD2 > "$O/whofb-times.txt"
for d in "$HOME/Library/Fonts" /Library/Fonts; do echo "## $d"; ls -la "$d" 2>/dev/null || true; done > "$O/user-font-census.txt"
(cd "$O" && shasum -a 256 *.tsv *.txt > SHA256SUMS)
cat "$O/SHA256SUMS"
