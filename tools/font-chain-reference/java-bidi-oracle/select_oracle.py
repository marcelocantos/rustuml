"""Select the committed java.text.Bidi oracle subset (round-9 maker, bidi-oracle-gate-account.json).

Usage: python3 -I select_oracle.py <bidi-input.txt> <bidi-java-levels.txt> <out.txt.gz>

Inputs are the checker round-8 relay files (input sha256 fb7cd937..., Java output sha256 e09425f4...).
Each kept line is "<input line>|<Java output line>". Selection (fixed before any mutation run):
  * every line of part C (index >= 241707: stack/overflow strings);
  * every DIRECTION_DEFAULT_LEFT_TO_RIGHT line holding a paragraph separator (bidi class B) before its end;
  * every line with an AN code point and no R, AL, RLE or RLO code point;
  * every line whose last code point is WS, S, B, BN or an explicit/isolate control;
  * every 8th remaining line.
Bidi classes here come from Python's unicodedata (selection only; the test compares Java's answers).
"""
import gzip, sys, unicodedata

inp = open(sys.argv[1], encoding="ascii").read().splitlines()
java = open(sys.argv[2], encoding="ascii").read().splitlines()
header, java = java[0], java[1:]
assert len(inp) == len(java) == 242508, (len(inp), len(java))
PART_C = 241707
TRAILING = {"WS", "S", "B", "BN", "LRE", "RLE", "LRO", "RLO", "PDF", "LRI", "RLI", "FSI", "PDI"}


def codes(hexes):
    units = [int(h, 16) for h in hexes.split()]
    out, i = [], 0
    while i < len(units):
        u = units[i]
        if 0xD800 <= u <= 0xDBFF and i + 1 < len(units) and 0xDC00 <= units[i + 1] <= 0xDFFF:
            out.append(0x10000 + ((u - 0xD800) << 10) + (units[i + 1] - 0xDC00)); i += 2
        else:
            out.append(u); i += 1
    return out


def bidi(code):
    return "" if 0xD800 <= code <= 0xDFFF else unicodedata.bidirectional(chr(code))


kept, reasons = [], {"part_c": 0, "multi_paragraph": 0, "an_only": 0, "trailing_ws": 0, "every_8th": 0}
rest = 0
for i, (line, out) in enumerate(zip(inp, java)):
    direction, hexes = line.split(";", 1)
    classes = [bidi(c) for c in codes(hexes)]
    if i >= PART_C:
        reason = "part_c"
    elif direction == "D" and "B" in classes[:-1]:
        reason = "multi_paragraph"
    elif "AN" in classes and not {"R", "AL", "RLE", "RLO"} & set(classes):
        reason = "an_only"
    elif classes and classes[-1] in TRAILING:
        reason = "trailing_ws"
    else:
        rest += 1
        if rest % 8:
            continue
        reason = "every_8th"
    reasons[reason] += 1
    kept.append(f"{line}|{out}")
with gzip.GzipFile(sys.argv[3], "wb", mtime=0) as f:
    f.write(("# " + header.lstrip("# ") + "\n" + "\n".join(kept) + "\n").encode("ascii"))
print(len(kept), reasons)
