"""Generate BMP_LETTER_RANGES (preprocess/mod.rs) from UnicodeData.txt and check it.

Character#isLetterOrDigit(char) on JDK 21 (Unicode 15.0) is true for general
category Lu, Ll, Lt, Lm, Lo or Nd. This script emits the inclusive BMP runs of
the L* categories. Nd stays on BMP_DECIMAL_DIGIT_ZEROS.

Usage:
  python3 -I gen_bmp_letters.py UCD15/UnicodeData.txt [UCD17/UnicodeData.txt] [mod.rs]
Prints the sha256 of each input, the letter and run counts, the BMP letters
Unicode 17 adds or removes, and whether the Rust table equals the generated one.
Exit 1 on any table mismatch.
"""
import hashlib, re, sys
from pathlib import Path

LETTER = {"Lu", "Ll", "Lt", "Lm", "Lo"}


def bmp_letters(path):
    letters, first = set(), None
    for line in Path(path).read_text().splitlines():
        f = line.split(";")
        cp, name, gc = int(f[0], 16), f[1], f[2]
        if name.endswith(", First>"):
            first = cp
            continue
        lo = first if name.endswith(", Last>") else cp
        first = None
        if gc in LETTER:
            letters.update(c for c in range(lo, cp + 1) if c <= 0xFFFF)
    return letters


def runs(cps):
    out = []
    for c in sorted(cps):
        if out and out[-1][1] == c - 1:
            out[-1][1] = c
        else:
            out.append([c, c])
    return [tuple(r) for r in out]


args = sys.argv[1:]
u15 = args[0]
print("ucd15", hashlib.sha256(Path(u15).read_bytes()).hexdigest())
l15 = bmp_letters(u15)
r15 = runs(l15)
print("bmp_letters", len(l15), "runs", len(r15))
if len(args) > 1:
    print("ucd17", hashlib.sha256(Path(args[1]).read_bytes()).hexdigest())
    l17 = bmp_letters(args[1])
    print("added_in_17", " ".join(f"U+{c:04X}" for c in sorted(l17 - l15)))
    print("removed_in_17", " ".join(f"U+{c:04X}" for c in sorted(l15 - l17)))
if len(args) > 2:
    src = Path(args[2]).read_text()
    body = src.split("const BMP_LETTER_RANGES", 1)[1].split("];", 1)[0]
    table = [(int(a, 16), int(b, 16)) for a, b in re.findall(r"\(0x([0-9A-F]{4}), 0x([0-9A-F]{4})\)", body)]
    ok = table == r15
    print("rust_table_runs", len(table), "equal" if ok else "DIFFERENT")
    sys.exit(0 if ok else 1)
for a, b in r15:
    print(f"    (0x{a:04X}, 0x{b:04X}),")
