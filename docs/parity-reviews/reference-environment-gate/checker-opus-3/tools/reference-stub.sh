#!/bin/sh
# Stub reference JVM: never runs Java. Prints DYLD-style lines for real local files.
cat >/dev/null
printf '<svg/>'
echo "dyld[1]: <00000000-0000-0000-0000-000000000001> /opt/homebrew/Cellar/openjdk@21/21.0.13.0.2/libexec/openjdk.jdk/Contents/Home/lib/libfontmanager.dylib" >&2
echo "dyld[1]: <00000000-0000-0000-0000-000000000002> /opt/homebrew/Cellar/harfbuzz/15.0.0_1/lib/libharfbuzz.0.dylib" >&2
echo "dyld[1]: <00000000-0000-0000-0000-000000000003> /opt/homebrew/Cellar/freetype/2.15.0/lib/libfreetype.6.dylib" >&2
exit ${STUB_EXIT:-0}
