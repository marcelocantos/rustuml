"""Independent regeneration of the units String#regionMatches(true, ...) matches against an ASCII key letter.
JDK 21 StringUTF16.regionMatchesCI_Latin1 per UTF-16 unit c1 (text) vs c2 (ASCII key letter):
  c1 == c2, or u1 = Character.toUpperCase(c1) == u2 = Character.toUpperCase(c2), or Character.toLowerCase(u1) == Character.toLowerCase(u2).
Character.toUpperCase/toLowerCase(char) = UnicodeData simple mappings (fields 12/13), else itself.
Computed both ways (text unit vs key and key vs text unit) and with a broader variant (any of U, L, L.U, U.L equal) for robustness.
Also: the getTypes switch unit, Character.toLowerCase(c) for c a non-ASCII unit, landing on an ASCII letter.
Usage: python3 -I fold_check.py UnicodeData.txt"""
import hashlib, sys
from pathlib import Path
p = Path(sys.argv[1]); up, lo = {}, {}
for line in p.read_text().splitlines():
    f = line.split(";"); cp = int(f[0], 16)
    if f[12]: up[cp] = int(f[12], 16)
    if f[13]: lo[cp] = int(f[13], 16)
U = lambda c: up.get(c, c); L = lambda c: lo.get(c, c)
rule = lambda a, b: a == b or U(a) == U(b) or L(U(a)) == L(U(b))
broad = lambda a, b: len({U(a), L(a), L(U(a)), U(L(a))} & {U(b), L(b), L(U(b)), U(L(b))}) > 0
units = [c for c in range(0x80, 0x10000) if not 0xD800 <= c <= 0xDFFF]
keys = range(ord("a"), ord("z") + 1)
fwd = sorted((c, chr(k)) for c in units for k in keys if rule(c, k))
rev = sorted((c, chr(k)) for c in units for k in keys if rule(k, c))
br = sorted((c, chr(k)) for c in units for k in keys if broad(c, k))
sw = sorted((c, chr(L(c))) for c in units if L(c) < 0x80 and chr(L(c)).isalpha())
print("ucd", hashlib.sha256(p.read_bytes()).hexdigest())
print("rule text-vs-key", [(f"U+{c:04X}", k) for c, k in fwd])
print("rule key-vs-text", [(f"U+{c:04X}", k) for c, k in rev], "same" if fwd == rev else "DIFFERENT")
print("broad variant   ", [(f"U+{c:04X}", k) for c, k in br])
print("switch toLowerCase lands on ASCII", [(f"U+{c:04X}", k) for c, k in sw])
