"""Per case: textLength of the first <text> containing a probe word, Java vs each tree; with the control's.
Usage: python3 -I widths.py <work> <label> <word> [out.json]"""
import json, re, sys, pathlib, html
W = pathlib.Path(sys.argv[1]); L = sys.argv[2]; WORD = sys.argv[3]
rows = json.load(open(W / f"batch-{L}.json")); out = {}
T = re.compile(r'<text\b[^>]*?textLength="([^"]*)"[^>]*>(.*?)</text>', re.S)
def tl(path):
    t = path.read_text(errors="replace")
    for m in T.finditer(t):
        if WORD in html.unescape(m.group(2)): return float(m.group(1))
    return None
for n, r in rows.items():
    ext = r["format"][2:]
    if ext != "svg": continue
    d = {"java": tl(W / f"out-{L}" / f"{n}.java.svg")}
    for t in ["base", "merge", "tip"]: d[t] = tl(W / f"out-{L}" / f"{n}.{t}.svg")
    out[n] = d
for n, d in out.items():
    c = out.get(n + "_ctrl")
    f = lambda v: "None" if v is None else f"{v:.4f}"
    line = f"{n:40s} J {f(d['java'])} b {f(d['base'])} m {f(d['merge'])} t {f(d['tip'])}"
    if c and None not in (d["java"], c["java"], d["tip"], c["tip"]):
        line += f" | dJ {d['java']-c['java']:+.4f} dm {d['merge']-c['merge']:+.4f} dt {d['tip']-c['tip']:+.4f} {'OK' if abs((d['java']-c['java'])-(d['tip']-c['tip']))<1e-3 else 'MISMATCH'}"
    print(line)
if len(sys.argv) > 4: json.dump(out, open(sys.argv[4], "w"), indent=1)
