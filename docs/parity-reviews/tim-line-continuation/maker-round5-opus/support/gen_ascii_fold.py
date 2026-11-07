"""Derive the non-ASCII BMP UTF-16 units that String#regionMatches(ignoreCase=true)
matches against an ASCII letter, from UnicodeData.txt (Java's simple case mappings:
Character#toUpperCase(char) = field 12, #toLowerCase(char) = field 13; no mapping = itself).

regionMatches per unit (JDK 21): equal; or u1 = toUpperCase(c1), u2 = toUpperCase(c2) equal;
or toLowerCase(u1) == toLowerCase(u2).
Usage: python3 -I gen_ascii_fold.py UnicodeData.txt [mod.rs]
Prints the sha256, each matching unit with its ASCII letter, and checks the Rust table.
"""
import hashlib, re, sys
from pathlib import Path

path = sys.argv[1]
upper, lower = {}, {}
for line in Path(path).read_text().splitlines():
    f = line.split(";")
    cp = int(f[0], 16)
    if cp > 0xFFFF:
        continue
    if f[12]:
        upper[cp] = int(f[12], 16)
    if f[13]:
        lower[cp] = int(f[13], 16)
U = lambda c: upper.get(c, c)
L = lambda c: lower.get(c, c)
print("ucd", hashlib.sha256(Path(path).read_bytes()).hexdigest())
found = []
for c in range(0x80, 0x10000):
    if 0xD800 <= c <= 0xDFFF:
        continue
    for k in range(ord("a"), ord("z") + 1):
        if c == k or U(c) == U(k) or L(U(c)) == L(U(k)):
            found.append((c, chr(k)))
for c, k in found:
    print(f"U+{c:04X} -> {k}")
if len(sys.argv) > 2:
    body = Path(sys.argv[2]).read_text().split("const JAVA_ASCII_CASE_FOLDS", 1)[1].split("];", 1)[0]
    table = [(int(a, 16), b) for a, b in re.findall(r"\('\\u\{([0-9A-Fa-f]+)\}', '([a-z])'\)", body)]
    ok = table == found
    print("rust_table", table, "equal" if ok else "DIFFERENT")
    sys.exit(0 if ok else 1)
