"""Checker-written: BMP Lu/Ll/Lt/Lm/Lo (and Nd) from UnicodeData.txt, including <..., First>/<..., Last> ranges,
compared with the BMP_LETTER_RANGES table parsed out of the Rust source. Usage: python3 -I ucd_letters_check.py UCD15 UCD17 MOD_RS"""
import re, sys
def cats(path):
    out = {}; first = None
    for line in open(path, encoding="utf-8"):
        f = line.split(";"); cp = int(f[0], 16); name, gc = f[1], f[2]
        if name.endswith(", First>"): first = cp; continue
        if name.endswith(", Last>"):
            for c in range(first, cp + 1): out[c] = gc
            first = None; continue
        out[cp] = gc
    return out
L = {"Lu", "Ll", "Lt", "Lm", "Lo"}
def bmp(c, want): return {cp for cp, g in c.items() if cp <= 0xFFFF and g in want}
c15, c17 = cats(sys.argv[1]), cats(sys.argv[2])
l15, l17 = bmp(c15, L), bmp(c17, L)
src = open(sys.argv[3], encoding="utf-8").read()
body = src[src.index("const BMP_LETTER_RANGES"):]; body = body[:body.index("];")]
ranges = [(int(a, 16), int(b, 16)) for a, b in re.findall(r"\(0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+)\)", body)]
rust = set()
for a, b in ranges: rust.update(range(a, b + 1))
runs = 0; prev = None
for cp in sorted(l15):
    if prev is None or cp != prev + 1: runs += 1
    prev = cp
print("ucd15 bmp letters", len(l15), "runs", runs, "rust", len(rust), "rust runs", len(ranges))
print("rust - ucd15", sorted(hex(x) for x in rust - l15)); print("ucd15 - rust", sorted(hex(x) for x in l15 - rust))
print("ucd17 - ucd15", sorted(hex(x) for x in l17 - l15)); print("ucd15 - ucd17", sorted(hex(x) for x in l15 - l17))
print("Nd bmp 15/17", len(bmp(c15, {"Nd"})), len(bmp(c17, {"Nd"})), "Nd diff", sorted(hex(x) for x in bmp(c15, {"Nd"}) ^ bmp(c17, {"Nd"})))
print("sorted-disjoint", all(ranges[i][1] + 1 < ranges[i + 1][0] for i in range(len(ranges) - 1)))
