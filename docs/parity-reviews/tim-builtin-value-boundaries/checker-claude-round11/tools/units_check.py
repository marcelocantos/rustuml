"""Every Java <text> in Verdana or Arial whose content is printable ASCII: does sum(face units)/2048*size reproduce Java's
textLength (4-dp rounding), and with which (bold, italic) slot? Tables are read from the tip source (text_render.rs).
Usage: python3 -I units_check.py <tip text_render.rs> <out.json> <svg-dir-or-glob>..."""
import html, json, re, sys, pathlib, collections, glob
src = open(sys.argv[1]).read()
def u16(name): return [int(x) for x in re.findall(r"\d+", re.search(rf"const {name}: \[u16; 95\] = \[(.*?)\];", src, re.S).group(1))]
SLOTS = {(b, i): {} for b in (0, 1) for i in (0, 1)}
for fam in ["ARIAL", "VERDANA"]:
    for (b, i), suf in {(0, 0): "", (1, 0): "_BOLD", (0, 1): "_ITALIC", (1, 1): "_BOLD_ITALIC"}.items():
        SLOTS[(b, i)][fam.capitalize()] = u16(fam + suf + "_ASCII")
T = re.compile(r"<text ([^>]*)>([^<]*)</text>")
A = lambda attrs, k: (re.search(rf'{k}="([^"]*)"', attrs) or [None, None])[1]
stat = collections.Counter(); bad = []; files = []
for g in sys.argv[3:]: files += glob.glob(g, recursive=True)
for f in sorted(files):
    s = open(f, errors="replace").read()
    for attrs, txt in T.findall(s):
        fam = A(attrs, "font-family"); tl = A(attrs, "textLength"); fs = A(attrs, "font-size")
        if fam not in ("Verdana", "Arial") or not tl or not fs: continue
        refs = [int(x[1:], 16) if x.startswith('x') else int(x) for x in re.findall(r'&#(x[0-9a-fA-F]+|\d+);', txt)]
        if any(not (0x20 <= r <= 0x7E) for r in refs): continue  # html.unescape drops invalid code points (e.g. U+10FFFF)
        t = html.unescape(txt)
        if not t or any(not (0x20 <= ord(c) <= 0x7E) for c in t): continue
        b = int(A(attrs, "font-weight") in ("700", "bold")); i = int(A(attrs, "font-style") == "italic")
        size = float(fs); jl = float(tl)
        w = {k: sum(v[fam][ord(c) - 32] for c in t) / 2048 * size for k, v in SLOTS.items()}
        ok = abs(w[(b, i)] - jl) < 6e-5
        others = [k for k in w if k != (b, i) and abs(w[k] - jl) < 6e-5]
        key = (fam, b, i)
        stat[key + ("exact",) if ok else key + ("miss",)] += 1
        if ok and not others and w[(b, i)] != w[(b, 0)]: stat[key + ("only_own_slot_and_differs_from_upright",)] += 1
        if not ok and len(bad) < 200: bad.append({"file": f, "text": t, "family": fam, "bold": b, "italic": i, "size": size, "java": jl, "own_slot": round(w[(b, i)], 5), "other_slots_matching": [list(k) for k in others]})
json.dump({"stats": {"/".join(map(str, k)): v for k, v in sorted(stat.items())}, "misses": bad}, open(sys.argv[2], "w"), indent=1)
for k, v in sorted(stat.items()): print(k, v)
for x in bad[:15]: print(x)
