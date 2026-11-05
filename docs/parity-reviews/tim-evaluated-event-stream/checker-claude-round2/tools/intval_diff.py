"""Differential: Rust CLI %intval vs a Python port of JDK 21 Integer.parseInt(String) (radix 10) over UTF-16 units,
with Character.digit(char,10) = BMP Nd digit value (Unicode 15.x BMP Nd == 37 runs). Usage: intval_diff.py <bin> <n>"""
import random, re, subprocess, sys, unicodedata, html
BIN, N = sys.argv[1], int(sys.argv[2])
def units(s): b = s.encode("utf-16-le"); return [int.from_bytes(b[i:i+2], "little") for i in range(0, len(b), 2)]
def digit(u):
    if 0xD800 <= u <= 0xDFFF: return -1
    c = chr(u); return unicodedata.digit(c) if unicodedata.category(c) == "Nd" else -1
def parse_int(s):
    us = units(s); MAXV, MINV = 2**31 - 1, -2**31
    if not us: return None
    i, neg, limit = 0, False, -MAXV
    if us[0] < ord("0"):
        if us[0] == ord("-"): neg, limit = True, MINV
        elif us[0] != ord("+"): return None
        if len(us) == 1: return None
        i = 1
    multmin = int(limit / 10)  # Java int division truncates toward zero
    r = 0
    for u in us[i:]:
        d = digit(u)
        if d < 0 or r < multmin: return None
        r *= 10
        if r < limit + d: return None
        r -= d
    return r if neg else -r
zeros = [0x30, 0x660, 0x6F0, 0x966, 0x9E6, 0xE50, 0xFF10, 0xA9F0, 0x1C50, 0xABF0]
def rnd():
    k = random.random()
    if k < 0.3:  # near bounds in random scripts
        v = random.choice([2**31 - 1, 2**31, -2**31, -2**31 - 1, 0, 7, 10**9, 2147483639, 2147483657])
        s = str(v)
    else:
        s = "".join(random.choice("0123456789") for _ in range(random.randint(0, 11)))
        if random.random() < 0.4: s = random.choice(["-", "+", "--", "+-", "-+", ""]) + s
    out = []
    for ch in s:
        if ch.isdigit() and random.random() < 0.5:
            out.append(chr(random.choice(zeros) + int(ch)))
        else: out.append(ch)
    if random.random() < 0.15:
        out.insert(random.randint(0, len(out)), random.choice(["²", "Ⅷ", "\U0001D7CF", "x", " ", "༪", "፩", "Ａ"]))
    return "".join(out)
random.seed(20261105)
bad = []; seen = 0
for _ in range(N):
    s = rnd()
    src = f'@startuml\nAlice -> Bob : <%intval("{s}")>\n@enduml\n'
    p = subprocess.run([BIN, "-pipe", "-tsvg"], input=src.encode(), capture_output=True)
    exp = parse_int(s)
    if p.returncode != 0:
        got = None
    else:
        m = re.search(r"&lt;(-?\d+)&gt;", p.stdout.decode()) or re.search(r"<(-?\d+)>", html.unescape(p.stdout.decode()))
        got = int(m.group(1)) if m else "??"
    seen += 1
    if got != exp: bad.append((s, exp, got, p.stderr.decode()[:80]))
print(seen, "cases;", len(bad), "mismatches")
for b in bad[:20]: print(" ", [hex(ord(c)) for c in b[0]], b[1:])
