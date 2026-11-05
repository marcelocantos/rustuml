#!/bin/bash
# SHA256 of every external source file the round-7 accounts cite (untrusted downloads, read as data).
U=~/Library/Caches/rustuml-shared-svg-round9-claude-maker/untrusted
cd $U && for f in harfbuzz/15.0.0/* harfbuzz/10.0.1/* jdk21u/* ucd18/UnicodeData.txt ucd16/UnicodeData.txt ucd32/Scripts-3.2.0.txt; do shasum -a 256 "$f"; done
echo "# harfbuzz tags: 15.0.0 = $(git -C harfbuzz/repo rev-parse '15.0.0^{commit}'), 10.0.1 = $(git -C harfbuzz/repo rev-parse '10.0.1^{commit}')"
echo "# hangul fallback_position commit: $(git -C harfbuzz/repo log -1 --format='%H %s' 63e8c13e1); first tags: $(git -C harfbuzz/repo tag --contains 63e8c13e1 | sort -V | head -1)"
