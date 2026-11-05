#!/bin/bash
# Round-7 maker request (run on Monkey Man, read-only): SHA256 of every font file a RustUML ChainFace's
# primary face reads, so the drift gate can confirm colossus's own-cmap extraction applies to the
# reference host (font_chain.rs PENDING_REFERENCE_HASH). Output: stdout; no files are written.
set -u
echo "## host"; hostname; sw_vers; date -u
echo "## font files"
for f in "/System/Library/Fonts/LucidaGrande.ttc" "/System/Library/Fonts/Helvetica.ttc" "/System/Library/Fonts/Menlo.ttc" \
         "/System/Library/Fonts/Supplemental/Arial.ttf" "/System/Library/Fonts/Supplemental/Arial Bold.ttf" \
         "/System/Library/Fonts/Supplemental/Verdana.ttf" "/System/Library/Fonts/Supplemental/Verdana Bold.ttf" \
         "/System/Library/Fonts/Supplemental/Courier New.ttf" "/System/Library/Fonts/Supplemental/Courier New Bold.ttf"; do
  shasum -a 256 "$f"
done
