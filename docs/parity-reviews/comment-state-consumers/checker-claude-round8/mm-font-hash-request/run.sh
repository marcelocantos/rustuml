#!/bin/bash
# Checker round-8 (claude) request, run on Monkey Man, read-only: SHA256 of the italic/bold-italic Courier New,
# Times New Roman and Tahoma files and the Helvetica collection, so the C8A italic-face and mirroring findings can be
# tied to the reference host's files (colossus hashes are recorded beside this script). Output: stdout only.
set -u
echo "## host"; hostname; sw_vers; date -u
echo "## font files"
for f in "/System/Library/Fonts/Supplemental/Courier New.ttf" "/System/Library/Fonts/Supplemental/Courier New Bold.ttf" \
         "/System/Library/Fonts/Supplemental/Courier New Italic.ttf" "/System/Library/Fonts/Supplemental/Courier New Bold Italic.ttf" \
         "/System/Library/Fonts/Supplemental/Times New Roman.ttf" "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf" \
         "/System/Library/Fonts/Supplemental/Times New Roman Bold Italic.ttf" "/System/Library/Fonts/Supplemental/Tahoma.ttf" \
         "/System/Library/Fonts/Supplemental/Tahoma Bold.ttf" "/System/Library/Fonts/Helvetica.ttc"; do
  shasum -a 256 "$f"
done
echo "## user/local copies that could shadow them"
ls -la ~/Library/Fonts /Library/Fonts 2>/dev/null | grep -i -E "courier|times|tahoma|helvetica" || echo "(none)"
