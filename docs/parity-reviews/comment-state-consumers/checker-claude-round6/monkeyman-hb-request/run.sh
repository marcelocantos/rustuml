#!/bin/bash
# Round-6 checker diagnostic for Monkey Man (the Java reference host). Read-only: it writes only under $1.
# Purpose: which libharfbuzz the reference JDK actually LOADS (path, install version, dylib current_version, sha256),
# plus the identity of the font files whose plan facts (kern cross-stream bit, cmap) were measured on colossus.
# Usage (from this directory, on Monkey Man):  bash run.sh <empty output dir>
set -u
OUT=${1:?usage: run.sh <empty output dir>}; mkdir -p "$OUT"; HERE=$(cd "$(dirname "$0")" && pwd)
JHOME=/opt/homebrew/Cellar/openjdk@21/21.0.5/libexec/openjdk.jdk/Contents/Home
FM=$JHOME/lib/libfontmanager.dylib
HB=/opt/homebrew/opt/harfbuzz/lib/libharfbuzz.0.dylib
REF=/Users/marcelo/rustuml-reference/bin/reference
{
  echo "## host"; hostname; sw_vers; date -u
  echo "## libfontmanager link-time dependencies"; otool -L "$FM"
  echo "## opt/harfbuzz symlink and kegs"; ls -la /opt/homebrew/opt/harfbuzz; ls -1 /opt/homebrew/Cellar/harfbuzz/
  echo "## resolved libharfbuzz"; R=$(python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$HB"); echo "$R"
  echo "## resolved dylib id and current_version"; otool -L "$R" | head -2; otool -l "$R" | grep -A4 LC_ID_DYLIB
  echo "## receipt version"; python3 -c 'import json,sys;d=json.load(open(sys.argv[1]));print(d.get("source",{}).get("versions"), d.get("time"))' /opt/homebrew/opt/harfbuzz/INSTALL_RECEIPT.json
  echo "## sha256"; shasum -a 256 "$R" "$FM" "$JHOME/bin/java" "$REF"
  echo "## font files (compare with colossus hashes in the checker record)"
  shasum -a 256 /System/Library/Fonts/Helvetica.ttc /System/Library/Fonts/LucidaGrande.ttc /System/Library/Fonts/Menlo.ttc \
    /System/Library/Fonts/GeezaPro.ttc "/System/Library/Fonts/Supplemental/Times New Roman.ttf" /System/Library/Fonts/Supplemental/Verdana.ttf \
    /System/Library/Fonts/Supplemental/Arial.ttf "/System/Library/Fonts/Supplemental/Courier New.ttf" 2>&1
} > "$OUT/static.txt" 2>&1
# Method A: dyld prints every image the reference JVM loads (including dlopen of libfontmanager -> libharfbuzz).
DYLD_PRINT_LIBRARIES=1 "$REF" --cwd "$OUT" --timezone GMT+11:00 -- -pipe -tsvg < "$HERE/input/probe.puml" > "$OUT/methodA.svg" 2> "$OUT/methodA.stderr"
echo "exit $?" > "$OUT/methodA.exit"
grep -i -E 'harfbuzz|fontmanager|freetype' "$OUT/methodA.stderr" > "$OUT/methodA.loaded.txt"
# Method B: a long complex-text render; while it runs, list mapped images of the live JVM.
"$REF" --cwd "$OUT" --timezone GMT+11:00 -- -pipe -tsvg < "$HERE/input/long.puml" > /dev/null 2> "$OUT/methodB.stderr" &
for i in $(seq 1 120); do
  P=$(pgrep -n -f 'reference.jar' || true)
  if [ -n "$P" ] && lsof -p "$P" 2>/dev/null | grep -q -i harfbuzz; then
    lsof -p "$P" | grep -i -E 'harfbuzz|fontmanager' > "$OUT/methodB.lsof.txt"
    vmmap -w "$P" 2>/dev/null | grep -i -E 'harfbuzz|fontmanager' > "$OUT/methodB.vmmap.txt"
    break
  fi
  sleep 0.5
done
wait
( cd "$OUT" && shasum -a 256 static.txt methodA.* methodB.* > SHA256SUMS )
echo done
